use color_core::color::{oklab_to_oklch, rgb8_to_hsv, rgb8_to_oklab};
use serde::Deserialize;

const OKLAB_TOL: f32 = 2e-3;
const OKLCH_TOL: f32 = 2e-3;
const HUE_TOL: f32 = 0.5;
const HSV_TOL: f32 = 2e-3;
const CHROMA_EPS: f32 = 1e-3;

#[derive(Debug, Deserialize)]
struct FixtureFile {
    samples: Vec<FixtureSample>,
}

#[derive(Debug, Deserialize)]
struct FixtureSample {
    rgb: [u8; 3],
    oklab: [f32; 3],
    oklch: [f32; 3],
    hsv: [f32; 3],
}

#[test]
fn matches_gold_standard_fixtures() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("color_golden.json");
    let contents = std::fs::read_to_string(path).expect("fixture file missing");
    let fixtures: FixtureFile = serde_json::from_str(&contents).expect("fixture parse failed");

    for sample in fixtures.samples {
        let rgb = sample.rgb;
        let oklab = rgb8_to_oklab(rgb);
        let oklch = oklab_to_oklch(oklab);
        let hsv = rgb8_to_hsv(rgb);

        assert_close_vec("oklab", oklab, sample.oklab, OKLAB_TOL);
        assert_close("oklch.l", oklch[0], sample.oklch[0], OKLCH_TOL);
        assert_close("oklch.c", oklch[1], sample.oklch[1], OKLCH_TOL);
        if sample.oklch[1] > CHROMA_EPS && oklch[1] > CHROMA_EPS {
            assert_hue_close("oklch.h", oklch[2], sample.oklch[2], HUE_TOL);
        }

        if sample.hsv[1] > CHROMA_EPS && hsv[1] > CHROMA_EPS {
            assert_hue_close("hsv.h", hsv[0], sample.hsv[0], HUE_TOL);
        }
        assert_close("hsv.s", hsv[1], sample.hsv[1], HSV_TOL);
        assert_close("hsv.v", hsv[2], sample.hsv[2], HSV_TOL);
    }
}

fn assert_close_vec(label: &str, actual: [f32; 3], expected: [f32; 3], tol: f32) {
    for (idx, (a, b)) in actual.iter().zip(expected.iter()).enumerate() {
        assert_close(&format!("{label}[{idx}]"), *a, *b, tol);
    }
}

fn assert_close(label: &str, actual: f32, expected: f32, tol: f32) {
    let diff = (actual - expected).abs();
    assert!(
        diff <= tol,
        "{label} diff {diff} exceeds tol {tol} (actual={actual}, expected={expected})"
    );
}

fn assert_hue_close(label: &str, actual: f32, expected: f32, tol: f32) {
    let mut diff = (actual - expected).abs();
    if diff > 180.0 {
        diff = 360.0 - diff;
    }
    assert!(
        diff <= tol,
        "{label} diff {diff} exceeds tol {tol} (actual={actual}, expected={expected})"
    );
}
