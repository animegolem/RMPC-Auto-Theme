use std::io::Write;
use std::process::{Command, Output, Stdio};

use rmpc_theme_gen::config::EffectiveConfig;
use rmpc_theme_gen::emit::patch_theme;
use rmpc_theme_gen::roles::{Role, SolverTier};
use rmpc_theme_gen::solver::{validate_palette, validate_palette_with_config};
use rmpc_theme_gen::{generate, generate_with_config, DEFAULT_THEME};

const CHROMATIC: &str = include_str!("../fixtures/chromatic.png.b64");
const MONOCHROME: &str = include_str!("../fixtures/monochrome.png.b64");
const NEAR_BLACK: &str = include_str!("../fixtures/near-black.png.b64");
const CHROMATIC_GOLDEN: &str = include_str!("../fixtures/chromatic-theme.ron");

fn decode_base64(input: &str) -> Vec<u8> {
    let mut output = Vec::new();
    let mut accumulator = 0_u32;
    let mut bits = 0_u8;

    for byte in input.bytes().filter(|byte| !byte.is_ascii_whitespace()) {
        if byte == b'=' {
            break;
        }
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => panic!("invalid base64 fixture byte: {byte}"),
        };
        accumulator = (accumulator << 6) | u32::from(value);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            output.push((accumulator >> bits) as u8);
            accumulator &= (1_u32 << bits) - 1;
        }
    }
    output
}

fn fixtures() -> [(&'static str, Vec<u8>); 3] {
    [
        ("chromatic", decode_base64(CHROMATIC)),
        ("monochrome", decode_base64(MONOCHROME)),
        ("near-black", decode_base64(NEAR_BLACK)),
    ]
}

fn hex(rgb: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2])
}

fn collect_value_diffs(path: &str, left: &ron::Value, right: &ron::Value, out: &mut Vec<String>) {
    match (left, right) {
        (ron::Value::Map(left), ron::Value::Map(right)) => {
            for (key, left_value) in left.iter() {
                let Some(right_value) = right.get(key) else {
                    out.push(path.to_owned());
                    continue;
                };
                let segment = match key {
                    ron::Value::String(value) => value.clone(),
                    _ => format!("{key:?}"),
                };
                let child = if path.is_empty() {
                    segment
                } else {
                    format!("{path}.{segment}")
                };
                collect_value_diffs(&child, left_value, right_value, out);
            }
            if left.len() != right.len() {
                out.push(path.to_owned());
            }
        }
        (ron::Value::Option(left), ron::Value::Option(right)) => match (left, right) {
            (Some(left), Some(right)) => collect_value_diffs(path, left, right, out),
            (None, None) => {}
            _ => out.push(path.to_owned()),
        },
        (ron::Value::Seq(left), ron::Value::Seq(right)) if left.len() == right.len() => {
            for (index, (left, right)) in left.iter().zip(right).enumerate() {
                collect_value_diffs(&format!("{path}[{index}]"), left, right, out);
            }
        }
        _ if left != right => out.push(path.to_owned()),
        _ => {}
    }
}

#[test]
fn fixture_corpus_is_deterministic_and_meets_bands_and_guardrails() {
    for (name, artwork) in fixtures() {
        let first = generate(&artwork, DEFAULT_THEME)
            .unwrap_or_else(|error| panic!("{name} generation failed: {error}"));
        let second = generate(&artwork, DEFAULT_THEME)
            .unwrap_or_else(|error| panic!("{name} repeat generation failed: {error}"));

        assert_eq!(
            first.theme, second.theme,
            "{name} theme changed between runs"
        );
        assert_eq!(
            first.debug, second.debug,
            "{name} debug output changed between runs"
        );
        assert_eq!(
            first.palette, second.palette,
            "{name} palette changed between runs"
        );
        assert_eq!(
            first.patched_paths, second.patched_paths,
            "{name} patch set changed"
        );

        for role in Role::ALL {
            let color = first.palette.get(role);
            assert!(
                role.tone_band().contains(color.oklab[0]),
                "{name} {role:?} L={} escaped {:?}",
                color.oklab[0],
                role.tone_band(),
            );
        }

        let report = validate_palette(&first.palette)
            .unwrap_or_else(|error| panic!("{name} guardrails failed: {error}"));
        assert!(report.accent_background >= 4.5, "{name}: {report:?}");
        assert!(report.text_background >= 4.5, "{name}: {report:?}");
        assert!(report.active_background >= 3.5, "{name}: {report:?}");
        assert!(report.highlight_active >= 4.5, "{name}: {report:?}");
        assert!(report.highlight_selection >= 4.5, "{name}: {report:?}");
        assert!(
            report.selection_active_delta_l >= 0.08,
            "{name}: {report:?}"
        );
        assert!(report.frame_background >= 3.0, "{name}: {report:?}");
    }
}

#[test]
fn configured_bands_and_floors_reach_the_solver() {
    let mut config = EffectiveConfig::default();
    config.bands.background.target = 0.12;
    config.bands.background.min = 0.11;
    config.bands.background.max = 0.14;
    config.floors.accent_background = 5.0;
    config.floors.highlight_selection = 5.0;
    config.validate().unwrap();
    let generated =
        generate_with_config(&decode_base64(CHROMATIC), DEFAULT_THEME, &config).unwrap();
    let background = generated.palette.get(Role::Background);
    assert!(config.bands.background.contains(background.oklab[0]));
    let report =
        validate_palette_with_config(&generated.palette, &config.bands, &config.floors).unwrap();
    assert!(report.accent_background >= 5.0);
    assert!(report.highlight_selection >= 5.0);
}

#[test]
fn chromatic_fixture_matches_byte_exact_golden() {
    let generated = generate(&decode_base64(CHROMATIC), DEFAULT_THEME).unwrap();
    assert_eq!(generated.theme, CHROMATIC_GOLDEN);
}

#[test]
fn default_theme_ast_diff_contains_only_governed_color_paths() {
    let generated = generate(&decode_base64(CHROMATIC), DEFAULT_THEME).unwrap();
    let before: ron::Value = ron::de::from_str(DEFAULT_THEME).unwrap();
    let after: ron::Value = ron::de::from_str(&generated.theme).unwrap();
    let mut differences = Vec::new();
    collect_value_diffs("", &before, &after, &mut differences);
    differences.sort();

    assert_eq!(differences, generated.patched_paths);
}

#[test]
fn monochrome_art_records_neutral_provenance() {
    let generated = generate(&decode_base64(MONOCHROME), DEFAULT_THEME).unwrap();

    for role in Role::ALL {
        let color = generated.palette.get(role);
        assert_eq!(
            color.tier,
            SolverTier::Neutral,
            "{role:?} did not use neutral tier"
        );
        if role == Role::Selection {
            assert_eq!(color.origin, "derived:selection(active)");
        } else {
            assert!(
                color.origin.contains("neutral"),
                "{role:?} has non-neutral origin {:?}",
                color.origin,
            );
        }
        assert!(
            color.oklch[1] <= 0.04,
            "{role:?} emitted C={}",
            color.oklch[1]
        );
    }

    let report = validate_palette(&generated.palette).unwrap();
    assert!(report.accent_active_both_neutral);
    assert!(report.accent_active_hue_exempt);
    assert!(report.highlight_selection >= 4.5);
    assert!(report.selection_active_delta_l >= 0.08);
}

#[test]
fn cursor_and_playing_backgrounds_use_distinct_ruled_roles() {
    let generated = generate(&decode_base64(CHROMATIC), DEFAULT_THEME).unwrap();
    let selection = hex(generated.palette.get(Role::Selection).rgb);
    let active = hex(generated.palette.get(Role::Active).rgb);

    assert_ne!(selection, active);
    assert!(generated.theme.contains(&format!(
        "current_item_style: (fg: \"#001608\", bg: \"{selection}\""
    )));
    assert!(generated
        .theme
        .contains("highlighted_item_style: (fg: \"#001608\""));
    assert!(generated.theme.contains(&format!(
        "tab_bar: (\n        active_style: (fg: \"#001608\", bg: \"{active}\""
    )));
}

#[test]
fn emitter_preserves_every_byte_outside_mapped_string_tokens() {
    const BASE: &str = r##"#![enable(implicit_some)]
#![enable(unwrap_newtypes)]
#![enable(unwrap_variant_newtypes)]
(
    // This comment, spacing, and unrelated value must survive byte-for-byte.
    background_color: "#010203",
    text_color: "#040506",
    format_tag_separator: " blue | red ",
)
"##;
    let generated = generate(&decode_base64(CHROMATIC), DEFAULT_THEME).unwrap();
    let patched = patch_theme(BASE, &generated.palette).unwrap();

    let expected = BASE
        .replacen(
            "\"#010203\"",
            &format!("\"{}\"", hex(generated.palette.get(Role::Background).rgb)),
            1,
        )
        .replacen(
            "\"#040506\"",
            &format!("\"{}\"", hex(generated.palette.get(Role::Text).rgb)),
            1,
        );

    assert_eq!(patched.output, expected);
    assert_eq!(patched.patched_paths, ["background_color", "text_color"]);
    assert!(patched
        .output
        .contains("format_tag_separator: \" blue | red \""));
}

fn run_cli(artwork: &[u8], debug: bool) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_rmpc-theme-gen"));
    command.args(["--config", "/dev/null"]);
    if debug {
        command.arg("--debug");
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(artwork).unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn cli_stdout_contains_only_the_complete_theme() {
    let artwork = decode_base64(CHROMATIC);
    let expected = generate(&artwork, DEFAULT_THEME).unwrap();

    let ordinary = run_cli(&artwork, false);
    assert!(
        ordinary.status.success(),
        "{}",
        String::from_utf8_lossy(&ordinary.stderr)
    );
    assert_eq!(ordinary.stdout, expected.theme.as_bytes());

    let debug = run_cli(&artwork, true);
    assert!(
        debug.status.success(),
        "{}",
        String::from_utf8_lossy(&debug.stderr)
    );
    assert_eq!(debug.stdout, expected.theme.as_bytes());
    let debug_stderr = String::from_utf8(debug.stderr).unwrap();
    assert!(debug_stderr.contains("roles:"));
    assert!(debug_stderr.contains("contrast matrix:"));
}

#[test]
fn cli_help_and_errors_keep_stdout_empty() {
    let help = Command::new(env!("CARGO_BIN_EXE_rmpc-theme-gen"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(help.status.success());
    assert!(help.stdout.is_empty());
    assert!(String::from_utf8_lossy(&help.stderr).contains("Usage:"));

    let invalid = run_cli(b"not an image", false);
    assert!(!invalid.status.success());
    assert!(invalid.stdout.is_empty());
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("failed to decode image"));
}

#[test]
fn cli_queries_the_effective_config_and_rejects_malformed_toml() {
    let root = std::env::temp_dir().join(format!("rmpc-config-cli-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let config = root.join("config.toml");
    std::fs::write(&config, "quality = 3\nosc11 = false\n").unwrap();

    let query = Command::new(env!("CARGO_BIN_EXE_rmpc-theme-gen"))
        .args(["--config", config.to_str().unwrap(), "--quality", "4"])
        .args(["--print-config", "quality"])
        .output()
        .unwrap();
    assert!(query.status.success());
    assert_eq!(query.stdout, b"4\n");
    assert!(query.stderr.is_empty());

    std::fs::write(&config, "quality = \"loud\"\n").unwrap();
    let malformed = Command::new(env!("CARGO_BIN_EXE_rmpc-theme-gen"))
        .args([
            "--config",
            config.to_str().unwrap(),
            "--print-config",
            "quality",
        ])
        .output()
        .unwrap();
    assert!(!malformed.status.success());
    assert!(malformed.stdout.is_empty());
    assert!(String::from_utf8_lossy(&malformed.stderr).contains("config"));
    std::fs::remove_dir_all(root).unwrap();
}
