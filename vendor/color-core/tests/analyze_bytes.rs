use std::io::Cursor;
use std::path::Path;

use color_core::analyze::{analyze, AnalyzeRequest, ImageSource};
use image::{ImageFormat, Rgb, RgbImage};

fn request(seed: u64) -> AnalyzeRequest {
    serde_json::from_value(serde_json::json!({
        "path": "",
        "k": 8,
        "quality": 2,
        "mergeThreshold": 0.0,
        "seed": seed,
    }))
    .expect("request from json")
}

fn jpeg_bytes() -> Vec<u8> {
    let mut img = RgbImage::new(96, 64);
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        *pixel = Rgb([(x * 2) as u8, (y * 3) as u8, ((x + y) % 255) as u8]);
    }
    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Jpeg)
        .expect("encode jpeg");
    buf
}

fn palette_json(bytes: &[u8], seed: u64) -> String {
    let response = analyze(&request(seed), ImageSource::Bytes(bytes)).expect("analyze bytes");
    serde_json::to_string(&response.clusters).expect("serialize clusters")
}

#[test]
fn jpeg_bytes_analysis_is_deterministic() {
    let bytes = jpeg_bytes();
    let first = palette_json(&bytes, 1);
    let second = palette_json(&bytes, 1);
    assert_eq!(first, second, "same seed must produce identical palettes");
}

#[test]
fn bytes_and_path_sources_produce_identical_palettes() {
    let pattern = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-patterns/hsl_ligthness.png");
    assert!(pattern.exists(), "missing test pattern: {pattern:?}");
    let bytes = std::fs::read(&pattern).expect("read pattern bytes");

    let via_path = analyze(&request(1), ImageSource::Path(&pattern)).expect("analyze path source");
    let via_bytes = palette_json(&bytes, 1);
    assert_eq!(
        serde_json::to_string(&via_path.clusters).expect("serialize clusters"),
        via_bytes,
        "path and bytes sources must agree on the same image"
    );
}
