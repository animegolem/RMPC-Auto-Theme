//! Album-art driven rmpc theme generation.

use color_core::analyze::{default_max_samples, AnalyzeRequest, ImageSource};

pub mod config;
pub mod emit;
pub mod roles;
pub mod solver;

use roles::{Role, RolePalette};

/// Complete rmpc 0.11 theme used when no user base theme is supplied.
pub const DEFAULT_THEME: &str = include_str!("../fixtures/default_theme.ron");

#[derive(Debug)]
pub struct Generation {
    pub theme: String,
    pub debug: String,
    pub palette: RolePalette,
    pub patched_paths: Vec<String>,
    pub untouched_paths: Vec<String>,
}

/// Extract colors, assign roles, and patch a complete base theme.
pub fn generate(art_bytes: &[u8], base_theme: &str) -> Result<Generation, String> {
    generate_with_config(art_bytes, base_theme, &config::EffectiveConfig::default())
}

/// Generate a theme using the effective user configuration.
pub fn generate_with_config(
    art_bytes: &[u8],
    base_theme: &str,
    config: &config::EffectiveConfig,
) -> Result<Generation, String> {
    let request = AnalyzeRequest {
        path: String::new(),
        k: 8,
        stride: 4,
        quality: Some(config.quality),
        ignore_top_n: 0,
        merge_threshold: 0.08,
        min_lum: 0,
        tol: 1e-3,
        max_iter: 40,
        seed: 1,
        max_samples: default_max_samples(),
        snap_to_real: false,
    };
    let analysis = color_core::analyze(&request, ImageSource::Bytes(art_bytes))
        .map_err(|error| error.to_string())?;
    let palette =
        solver::assign_roles_with_config(&analysis.clusters, &config.bands, &config.floors)?;
    let patch = emit::patch_theme(base_theme, &palette)?;
    let debug = debug_report(
        &palette,
        &patch.patched_paths,
        &patch.untouched_paths,
        &config.floors,
    );

    Ok(Generation {
        theme: patch.output,
        debug,
        palette,
        patched_paths: patch.patched_paths,
        untouched_paths: patch.untouched_paths,
    })
}

fn debug_report(
    palette: &RolePalette,
    patched: &[String],
    untouched: &[String],
    floors: &solver::GuardrailFloors,
) -> String {
    let mut output = String::from("roles:\n");
    for role in Role::ALL {
        let color = palette.get(role);
        output.push_str(&format!(
            "  {:<15} #{:02x}{:02x}{:02x} L={:.3} C={:.3} h={:.1} origin={} confidence={:.3} tier={:?}\n",
            role.as_str(),
            color.rgb[0],
            color.rgb[1],
            color.rgb[2],
            color.oklch[0],
            color.oklch[1],
            color.oklch[2],
            color.origin,
            color.confidence,
            color.tier,
        ));
    }

    output.push_str("contrast matrix:\n");
    for (left, right, floor) in [
        (Role::Accent, Role::Background, floors.accent_background),
        (Role::Text, Role::Background, floors.text_background),
        (Role::Active, Role::Background, floors.active_background),
        (Role::HighlightText, Role::Active, floors.highlight_active),
        (
            Role::HighlightText,
            Role::Selection,
            floors.highlight_selection,
        ),
        (Role::Frame, Role::Background, floors.frame_background),
    ] {
        let ratio = solver::wcag_contrast(palette.get(left).rgb, palette.get(right).rgb);
        output.push_str(&format!(
            "  {:<15} / {:<15} {:.3}:1 (floor {:.1})\n",
            left.as_str(),
            right.as_str(),
            ratio,
            floor,
        ));
    }
    let accent = palette.get(Role::Accent);
    let active = palette.get(Role::Active);
    output.push_str(&format!(
        "  accent / active deltaE={:.3} hue={:.1}°\n",
        solver::cie76_delta_e(accent.rgb, active.rgb),
        solver::hue_distance(accent.oklch[2], active.oklch[2]),
    ));
    output.push_str(&format!(
        "  selection / active deltaL={:.3} (floor {:.3})\n",
        palette.get(Role::Selection).oklab[0] - active.oklab[0],
        floors.selection_active_delta_l,
    ));

    output.push_str(&format!("patched paths ({}):\n", patched.len()));
    for path in patched {
        output.push_str(&format!("  {path}\n"));
    }
    output.push_str(&format!("untouched mapped paths ({}):\n", untouched.len()));
    for path in untouched {
        output.push_str(&format!("  {path}\n"));
    }
    output
}
