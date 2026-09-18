use color_core::color::{hsv_to_rgb8, oklab_to_rgb8, rgb8_to_hsv, rgb8_to_oklab};
use proptest::prelude::*;

const RGB_TOLERANCE: u8 = 2;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2000))]

    #[test]
    fn rgb_oklab_roundtrip(r in any::<u8>(), g in any::<u8>(), b in any::<u8>()) {
        let rgb = [r, g, b];
        let lab = rgb8_to_oklab(rgb);
        let back = oklab_to_rgb8(lab);
        prop_assert!(max_channel_delta(rgb, back) <= RGB_TOLERANCE);
    }

    #[test]
    fn rgb_hsv_roundtrip(r in any::<u8>(), g in any::<u8>(), b in any::<u8>()) {
        let rgb = [r, g, b];
        let hsv = rgb8_to_hsv(rgb);
        let back = hsv_to_rgb8(hsv);
        prop_assert!(max_channel_delta(rgb, back) <= RGB_TOLERANCE);
    }
}

#[test]
fn rgb_roundtrip_boundaries() {
    let samples = [
        [0, 0, 0],
        [255, 255, 255],
        [255, 0, 0],
        [0, 255, 0],
        [0, 0, 255],
        [255, 255, 0],
        [0, 255, 255],
        [255, 0, 255],
        [128, 128, 128],
        [1, 1, 1],
        [254, 254, 254],
        [250, 128, 114],
    ];
    for rgb in samples {
        let lab = rgb8_to_oklab(rgb);
        let back = oklab_to_rgb8(lab);
        assert!(max_channel_delta(rgb, back) <= RGB_TOLERANCE);

        let hsv = rgb8_to_hsv(rgb);
        let back = hsv_to_rgb8(hsv);
        assert!(max_channel_delta(rgb, back) <= RGB_TOLERANCE);
    }
}

fn max_channel_delta(a: [u8; 3], b: [u8; 3]) -> u8 {
    let dr = a[0].abs_diff(b[0]);
    let dg = a[1].abs_diff(b[1]);
    let db = a[2].abs_diff(b[2]);
    dr.max(dg).max(db)
}
