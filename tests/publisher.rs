use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::ops::Deref;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use rmpc_theme_gen::config::EffectiveConfig;
use rmpc_theme_gen::publisher::{
    admit_event, commit_theme, is_current, publish_current, publish_event, PublishResult,
};
use serde_json::Value;

const CHROMATIC: &str = include_str!("../fixtures/chromatic.png.b64");

static MPD_ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
static SCRATCH_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct Scratch(PathBuf);

impl Deref for Scratch {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn scratch() -> Scratch {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let sequence = SCRATCH_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "rmpc-publisher-test-{}-{nanos}-{sequence}",
        std::process::id(),
    ));
    fs::create_dir(&dir).unwrap();
    Scratch(dir)
}

fn config(dir: &std::path::Path, base: &str) -> EffectiveConfig {
    let path = dir.join("base.ron");
    fs::write(&path, base).unwrap();
    EffectiveConfig {
        base_theme: Some(path),
        palette_path: dir.join("palette/palette.json"),
        ..EffectiveConfig::default()
    }
}

fn hex_base(colors: [&str; 8]) -> String {
    format!("(background_color: \"{}\", text_color: \"{}\", level_styles: (info: (fg: \"{}\")), highlighted_item_style: (fg: \"{}\", bg: \"{}\"), current_item_style: (bg: \"{}\"), borders_style: (fg: \"{}\"), preview_label_style: (fg: \"{}\"))",
        colors[0], colors[1], colors[2], colors[3], colors[4], colors[5], colors[6], colors[7])
}

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

fn serve_art(art: Vec<u8>) -> (u16, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream.write_all(b"OK MPD 0.24.0\n").unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        loop {
            let mut command = String::new();
            if reader.read_line(&mut command).unwrap() == 0 {
                break;
            }
            if command.starts_with("lsinfo ") {
                stream
                    .write_all(
                        b"file: A/B.flac\nArtist: Test\nAlbum: Chromatic\nTitle: Cover\nOK\n",
                    )
                    .unwrap();
            } else if command.starts_with("albumart ") {
                stream
                    .write_all(format!("size: {}\nbinary: {}\n", art.len(), art.len()).as_bytes())
                    .unwrap();
                stream.write_all(&art).unwrap();
                stream.write_all(b"\nOK\n").unwrap();
            } else {
                panic!("unexpected MPD command: {command}");
            }
        }
    });
    (port, handle)
}

fn serve_no_art(connections: usize) -> (u16, thread::JoinHandle<()>) {
    serve_no_art_with_metadata(connections, "Artist: Test\nAlbum: Empty\nTitle: No Cover\n")
}

fn serve_no_art_with_metadata(
    connections: usize,
    metadata: &'static str,
) -> (u16, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = thread::spawn(move || {
        for _ in 0..connections {
            let (mut stream, _) = listener.accept().unwrap();
            stream.write_all(b"OK MPD 0.24.0\n").unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            loop {
                let mut command = String::new();
                if reader.read_line(&mut command).unwrap() == 0 {
                    break;
                }
                if command.starts_with("lsinfo ") {
                    let file = command
                        .trim_end()
                        .strip_prefix("lsinfo \"")
                        .unwrap()
                        .strip_suffix('"')
                        .unwrap();
                    stream
                        .write_all(format!("file: {file}\n{metadata}OK\n").as_bytes())
                        .unwrap();
                } else if command.starts_with("albumart ") || command.starts_with("readpicture ") {
                    stream.write_all(b"OK\n").unwrap();
                } else {
                    panic!("unexpected MPD command: {command}");
                }
            }
        }
    });
    (port, handle)
}

fn serve_albumart_ack(reply: &'static str) -> (u16, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream.write_all(b"OK MPD 0.24.0\n").unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        loop {
            let mut command = String::new();
            if reader.read_line(&mut command).unwrap() == 0 {
                break;
            }
            if command.starts_with("lsinfo ") {
                stream.write_all(b"file: A/B.flac\nOK\n").unwrap();
            } else if command.starts_with("albumart ") {
                stream.write_all(reply.as_bytes()).unwrap();
            } else if command.starts_with("readpicture ") {
                stream.write_all(b"OK\n").unwrap();
            } else {
                panic!("unexpected MPD command: {command}");
            }
        }
    });
    (port, handle)
}

fn run_no_art(base: &str) -> (Value, String) {
    run_no_art_with_metadata(base, "Artist: Test\nAlbum: Empty\nTitle: No Cover\n")
}

fn run_no_art_with_metadata(base: &str, metadata: &'static str) -> (Value, String) {
    let _guard = MPD_ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let dir = scratch();
    let config = config(&dir, base);
    let (port, server) = serve_no_art_with_metadata(1, metadata);
    std::env::set_var("MPD_HOST", "127.0.0.1");
    std::env::set_var("MPD_PORT", port.to_string());
    let admitted = admit_event(&config, Some("Artist/Album/01.flac".to_owned())).unwrap();
    let staged = dir.join("staged.ron");
    let result = publish_event(&config, &admitted, Some(&staged)).unwrap();
    assert_eq!(result, PublishResult::NoArt);
    let artifact: Value = serde_json::from_slice(&fs::read(&config.palette_path).unwrap()).unwrap();
    let theme = fs::read_to_string(staged).unwrap();
    server.join().unwrap();
    std::env::remove_var("MPD_PORT");
    std::env::remove_var("MPD_HOST");
    (artifact, theme)
}

#[test]
fn artwork_publish_emits_mapped_extraction_coordinates() {
    let _guard = MPD_ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let dir = scratch();
    let base = include_str!("../fixtures/default_theme.ron");
    let config = config(&dir, base);
    let art = decode_base64(CHROMATIC);
    let (port, server) = serve_art(art.clone());
    std::env::set_var("MPD_HOST", "127.0.0.1");
    std::env::set_var("MPD_PORT", port.to_string());
    let admitted = admit_event(&config, Some("A/B.flac".to_owned())).unwrap();
    let staged = dir.join("staged.ron");
    let result = publish_event(&config, &admitted, Some(&staged)).unwrap();
    server.join().unwrap();
    std::env::remove_var("MPD_PORT");
    std::env::remove_var("MPD_HOST");
    assert!(matches!(result, PublishResult::Ok(_)), "{result:?}");

    let artifact: Value = serde_json::from_slice(&fs::read(&config.palette_path).unwrap()).unwrap();
    assert_eq!(artifact["status"], "ok");
    assert_eq!(artifact["track"]["artist"], "Test");
    let clusters = artifact["clusters"].as_array().unwrap();
    let generated = rmpc_theme_gen::generate_with_config(&art, base, &config).unwrap();
    assert!(!clusters.is_empty());
    assert_eq!(clusters.len(), generated.clusters.len());
    for (record, cluster) in clusters.iter().zip(&generated.clusters) {
        let (_, mapped) = color_core::color::oklab_to_srgb8_gamut_mapped_with_oklab(cluster.oklab);
        let lch = record["oklch_extraction"].as_array().unwrap();
        let [l, c, h] = std::array::from_fn(|i| lch[i].as_f64().unwrap());
        let back = [l, c * h.to_radians().cos(), c * h.to_radians().sin()];
        for (actual, expected) in back.into_iter().zip(mapped.map(f64::from)) {
            assert!((actual - expected).abs() < 1e-6);
        }
        assert_eq!(
            record["hex"],
            format!(
                "#{:02x}{:02x}{:02x}",
                cluster.rgb.r, cluster.rgb.g, cluster.rgb.b
            )
        );
    }
    assert!(staged.exists());
    if let Some(path) = std::env::var_os("RMPC_CAPTURE_FIXTURE") {
        fs::copy(&config.palette_path, path).unwrap();
    }
}

#[test]
fn acquisition_failure_has_only_error_and_admitted_identity() {
    let _guard = MPD_ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let dir = scratch();
    let config = config(&dir, rmpc_theme_gen::DEFAULT_THEME);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream.write_all(b"OK MPD 0.24.0\n").unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        loop {
            let mut command = String::new();
            if reader.read_line(&mut command).unwrap() == 0 {
                break;
            }
            if command.starts_with("lsinfo ") {
                stream
                    .write_all(b"file: Artist/Album/01.flac\nOK\n")
                    .unwrap();
            } else if command.starts_with("albumart ") {
                stream
                    .write_all(b"ACK [5@0] {albumart} permission denied\n")
                    .unwrap();
                break;
            }
        }
    });
    std::env::set_var("MPD_HOST", "127.0.0.1");
    std::env::set_var("MPD_PORT", port.to_string());
    let admitted = admit_event(&config, Some("Artist/Album/01.flac".to_owned())).unwrap();
    assert_eq!(
        publish_event(&config, &admitted, None).unwrap(),
        PublishResult::Failed
    );
    server.join().unwrap();
    let first_bytes = fs::read(&config.palette_path).unwrap();
    let newer = admit_event(&config, Some("Artist/Album/02.flac".to_owned())).unwrap();
    assert!(newer.sequence > admitted.sequence);
    assert_eq!(
        publish_event(&config, &admitted, None).unwrap(),
        PublishResult::Superseded
    );
    assert_eq!(fs::read(&config.palette_path).unwrap(), first_bytes);
    std::env::remove_var("MPD_PORT");
    std::env::remove_var("MPD_HOST");
    let artifact: Value = serde_json::from_slice(&fs::read(config.palette_path).unwrap()).unwrap();
    assert_eq!(artifact["status"], "failed");
    assert_eq!(artifact["track"]["file"], "Artist/Album/01.flac");
    assert!(artifact["track"]["art_sha256"].is_null());
    for field in ["artist", "album", "title"] {
        assert!(
            artifact["track"].get(field).is_none(),
            "{field} must be absent"
        );
    }
    assert!(artifact.get("error").is_some());
    for field in ["roles", "verified_pairs", "clusters"] {
        assert!(artifact.get(field).is_none(), "{field} was not absent");
    }
}

#[test]
fn publish_current_query_failure_never_invents_track_identity() {
    let _guard = MPD_ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let dir = scratch();
    let config = config(&dir, rmpc_theme_gen::DEFAULT_THEME);
    let fake_rmpc = dir.join("rmpc-fake");
    fs::write(&fake_rmpc, "#!/bin/sh\nprintf 'not-json\\n'\n").unwrap();
    fs::set_permissions(&fake_rmpc, fs::Permissions::from_mode(0o755)).unwrap();
    std::env::set_var("RMPC_BIN", &fake_rmpc);
    assert_eq!(
        publish_current(&config, None).unwrap(),
        PublishResult::Failed
    );
    std::env::remove_var("RMPC_BIN");
    let artifact: Value = serde_json::from_slice(&fs::read(config.palette_path).unwrap()).unwrap();
    assert_eq!(artifact["status"], "failed");
    assert!(artifact["track"]["file"].is_null());
    assert!(artifact.get("roles").is_none());
    assert!(artifact.get("clusters").is_none());
}

#[test]
fn empty_file_admission_publishes_failed_with_null_identity() {
    let dir = scratch();
    let config = config(&dir, rmpc_theme_gen::DEFAULT_THEME);
    let admission = admit_event(&config, Some(String::new())).unwrap();
    assert!(admission.file.is_none());
    assert_eq!(
        publish_event(&config, &admission, None).unwrap(),
        PublishResult::Failed
    );
    let artifact: Value = serde_json::from_slice(&fs::read(config.palette_path).unwrap()).unwrap();
    assert!(artifact["track"]["file"].is_null());
    assert!(artifact["track"]["art_sha256"].is_null());
    for field in ["artist", "album", "title"] {
        assert!(
            artifact["track"].get(field).is_none(),
            "{field} must be absent"
        );
    }
}

#[test]
fn no_art_omits_one_missing_metadata_tag_without_nulling_the_others() {
    let (artifact, _) = run_no_art_with_metadata(
        rmpc_theme_gen::DEFAULT_THEME,
        "Artist: Test\nTitle: No Cover\n",
    );
    assert_eq!(artifact["status"], "no-art");
    assert_eq!(artifact["track"]["artist"], "Test");
    assert!(artifact["track"].get("album").is_none());
    assert_eq!(artifact["track"]["title"], "No Cover");
    assert_eq!(artifact["track"]["file"], "Artist/Album/01.flac");
    assert!(artifact["track"]["art_sha256"].is_null());
}

#[test]
fn no_art_stock_base_has_no_unverifiable_bundle() {
    let base = rmpc_theme_gen::DEFAULT_THEME;
    let (artifact, theme) = run_no_art(base);
    assert_eq!(theme, base);
    assert_eq!(artifact["status"], "no-art");
    let revision = artifact["producer"]["revision"].as_str().unwrap();
    assert!(revision.starts_with("src-") && revision.len() == 68);
    assert_eq!(artifact["producer"]["spec_revision"], "0.13");
    assert!(artifact.get("roles").is_none());
    assert!(artifact.get("verified_pairs").is_none());
    assert_eq!(artifact["features"], serde_json::json!([]));
    assert_eq!(artifact["clusters"], serde_json::json!([]));
    assert!(artifact["track"]["art_sha256"].is_null());
}

#[test]
fn no_art_all_black_fails_floors_without_altering_theme() {
    let base = hex_base(["#000000"; 8]);
    let (artifact, theme) = run_no_art(&base);
    assert_eq!(theme, base);
    assert_eq!(artifact["status"], "no-art");
    assert!(artifact.get("roles").is_none());
    assert!(artifact.get("verified_pairs").is_none());
    assert_eq!(artifact["features"], serde_json::json!([]));
}

#[test]
fn all_black_cli_diagnostic_names_failed_floor() {
    let dir = scratch();
    let base = hex_base(["#000000"; 8]);
    let base_path = dir.join("base.ron");
    fs::write(&base_path, base).unwrap();
    let palette = dir.join("palette/palette.json");
    let binary = env!("CARGO_BIN_EXE_rmpc-theme-gen");
    let base_set = format!("base_theme={}", base_path.display());
    let palette_set = format!("palette_path={}", palette.display());
    let admit = Command::new(binary)
        .args([
            "--set",
            &base_set,
            "--set",
            &palette_set,
            "--admit-event",
            "--file",
            "A/B.flac",
        ])
        .env("XDG_CONFIG_HOME", &*dir)
        .output()
        .unwrap();
    assert!(
        admit.status.success(),
        "{}",
        String::from_utf8_lossy(&admit.stderr)
    );
    let admitted = String::from_utf8(admit.stdout).unwrap();
    let mut parts = admitted.split_whitespace();
    let token = parts.next().unwrap();
    let sequence = parts.next().unwrap();
    let (port, server) = serve_no_art(1);
    let published = Command::new(binary)
        .args([
            "--set",
            &base_set,
            "--set",
            &palette_set,
            "--publish-event",
            "--file",
            "A/B.flac",
            "--token",
            token,
            "--sequence",
            sequence,
        ])
        .env("XDG_CONFIG_HOME", &*dir)
        .env("MPD_HOST", "127.0.0.1")
        .env("MPD_PORT", port.to_string())
        .output()
        .unwrap();
    server.join().unwrap();
    assert!(
        published.status.success(),
        "{}",
        String::from_utf8_lossy(&published.stderr)
    );
    assert_eq!(published.stdout, b"no-art\n");
    assert!(
        String::from_utf8_lossy(&published.stderr).contains("below floor"),
        "missing floor diagnostic: {}",
        String::from_utf8_lossy(&published.stderr)
    );
    let artifact: Value = serde_json::from_slice(&fs::read(palette).unwrap()).unwrap();
    assert!(artifact.get("roles").is_none());
    assert!(artifact.get("verified_pairs").is_none());
}

#[test]
fn palette_path_query_uses_producer_owned_default() {
    let dir = scratch();
    let output = Command::new(env!("CARGO_BIN_EXE_rmpc-theme-gen"))
        .args(["--print-config", "palette_path"])
        .env("XDG_CONFIG_HOME", &*dir)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim_end(),
        dir.join("rmpc-auto-theme/palette/palette.json")
            .display()
            .to_string()
    );
}

#[test]
fn no_art_passing_hex_base_emits_full_bundle_and_pairs() {
    let base = hex_base([
        "#140200", "#f8d700", "#00ca80", "#00160a", "#a58f00", "#dfa039", "#7a6900", "#ccb100",
    ]);
    let (artifact, theme) = run_no_art(&base);
    assert_eq!(theme, base);
    assert_eq!(artifact["features"], serde_json::json!(["selection"]));
    assert_eq!(artifact["roles"].as_object().unwrap().len(), 9);
    assert_eq!(artifact["verified_pairs"].as_array().unwrap().len(), 7);
    for (_, role) in artifact["roles"].as_object().unwrap() {
        if role.get("alias_of").is_none() {
            assert_eq!(role["origin"], "stock-base");
        }
        assert!(
            role.get("tier").is_none(),
            "stock-base role cannot claim solver tier"
        );
    }
    assert_eq!(artifact["roles"]["selection"]["kind"], "derived");
    assert_eq!(artifact["roles"]["selection"]["derived_from"], "active");
    assert_eq!(artifact["roles"]["inactive"]["alias_of"], "frame");
}

#[test]
fn no_art_with_invalid_base_is_failed_not_success() {
    let _guard = MPD_ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let dir = scratch();
    let config = config(&dir, "not a RON theme");
    let (port, server) = serve_no_art(1);
    std::env::set_var("MPD_HOST", "127.0.0.1");
    std::env::set_var("MPD_PORT", port.to_string());
    let admitted = admit_event(&config, Some("A/B.flac".to_owned())).unwrap();
    let staged = dir.join("staged.ron");
    assert_eq!(
        publish_event(&config, &admitted, Some(&staged)).unwrap(),
        PublishResult::Failed
    );
    server.join().unwrap();
    std::env::remove_var("MPD_PORT");
    std::env::remove_var("MPD_HOST");
    assert!(!staged.exists());
    let artifact: Value = serde_json::from_slice(&fs::read(config.palette_path).unwrap()).unwrap();
    assert_eq!(artifact["status"], "failed");
    assert!(artifact.get("roles").is_none());
}

#[test]
fn only_documented_albumart_absence_ack_falls_back_to_readpicture() {
    let _guard = MPD_ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    for (reply, expected) in [
        (
            "ACK [50@0] {albumart} No file exists\n",
            PublishResult::NoArt,
        ),
        (
            "ACK [50@0] {albumart} No art file exists\n",
            PublishResult::NoArt,
        ),
        ("ACK [50@0] {albumart} No database\n", PublishResult::Failed),
        (
            "ACK [50@0] {albumart} Cannot get size for stream\n",
            PublishResult::Failed,
        ),
    ] {
        let dir = scratch();
        let config = config(&dir, rmpc_theme_gen::DEFAULT_THEME);
        let (port, server) = serve_albumart_ack(reply);
        std::env::set_var("MPD_HOST", "127.0.0.1");
        std::env::set_var("MPD_PORT", port.to_string());
        let admitted = admit_event(&config, Some("A/B.flac".to_owned())).unwrap();
        assert_eq!(
            publish_event(&config, &admitted, None).unwrap(),
            expected,
            "{reply}"
        );
        server.join().unwrap();
        let artifact: Value =
            serde_json::from_slice(&fs::read(config.palette_path).unwrap()).unwrap();
        assert_eq!(artifact["status"], expected.line(), "{reply}");
    }
    std::env::remove_var("MPD_PORT");
    std::env::remove_var("MPD_HOST");
}

#[test]
fn newer_admission_suppresses_stale_failure_and_commit() {
    let dir = scratch();
    let config = config(&dir, rmpc_theme_gen::DEFAULT_THEME);
    let first = admit_event(&config, Some("A/1.flac".to_owned())).unwrap();
    let second = admit_event(&config, Some("B/2.flac".to_owned())).unwrap();
    assert!(second.sequence > first.sequence);
    assert!(!is_current(&config, &first).unwrap());
    assert!(is_current(&config, &second).unwrap());
    let staged = dir.join("staged.ron");
    fs::write(&staged, "staged").unwrap();
    let committed = dir.join("committed.ron");
    assert!(!commit_theme(&config, &first, &staged, &committed).unwrap());
    assert!(!committed.exists());
    assert!(commit_theme(&config, &second, &staged, &committed).unwrap());
    assert_eq!(fs::read_to_string(committed).unwrap(), "staged");
}
