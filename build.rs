use std::fs;
use std::path::PathBuf;

use sha2::{Digest, Sha256};

fn main() {
    let mut inputs = vec![
        PathBuf::from("Cargo.toml"),
        PathBuf::from("Cargo.lock"),
        PathBuf::from("build.rs"),
        PathBuf::from("fixtures/default_theme.ron"),
        PathBuf::from("fixtures/sample_config.ron"),
        PathBuf::from("hook/on_song_change.sh"),
        PathBuf::from("../color-tool-kmeans/color-core/Cargo.toml"),
    ];
    rust_sources(&PathBuf::from("src"), &mut inputs);
    rust_sources(
        &PathBuf::from("../color-tool-kmeans/color-core/src"),
        &mut inputs,
    );
    inputs.sort();
    let mut digest = Sha256::new();
    for path in inputs {
        println!("cargo:rerun-if-changed={}", path.display());
        digest.update(path.to_string_lossy().as_bytes());
        digest.update([0]);
        match fs::read(&path) {
            Ok(content) => digest.update(content),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                digest.update(b"<absent>")
            }
            Err(error) => panic!("read {}: {error}", path.display()),
        }
        digest.update([0]);
    }
    println!(
        "cargo:rustc-env=RMPC_THEME_REVISION=src-{:x}",
        digest.finalize()
    );
}

fn rust_sources(root: &std::path::Path, inputs: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                rust_sources(&path, inputs);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                inputs.push(path);
            }
        }
    }
}
