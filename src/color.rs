//! Color space conversions for the Rust compute core.
//! All transforms assume sRGB primaries with a D65 white point (IEC 61966-2-1).

const EPSILON: f32 = 1e-6;
const XYZ_WHITE: [f32; 3] = [0.95047, 1.0, 1.08883]; // D65

#[inline]
fn clamp01(v: f32) -> f32 {
    v.max(0.0).min(1.0)
}

#[inline]
fn to_u8(v: f32) -> u8 {
    (clamp01(v) * 255.0 + 0.5).floor() as u8
}

#[inline]
pub fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

#[inline]
pub fn linear_to_srgb(c: f32) -> f32 {
    if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

pub fn srgb8_to_linear(rgb: [u8; 3]) -> [f32; 3] {
    rgb.map(|c| srgb_to_linear((c as f32) / 255.0))
}

pub fn linear_to_srgb8(rgb: [f32; 3]) -> [u8; 3] {
    rgb.map(|c| {
        let srgb = linear_to_srgb(c);
        to_u8(srgb)
    })
}

pub fn rgb_to_xyz(linear_rgb: [f32; 3]) -> [f32; 3] {
    let r = linear_rgb[0];
    let g = linear_rgb[1];
    let b = linear_rgb[2];
    [
        0.4124564 * r + 0.3575761 * g + 0.1804375 * b,
        0.2126729 * r + 0.7151522 * g + 0.0721750 * b,
        0.0193339 * r + 0.1191920 * g + 0.9503041 * b,
    ]
}

pub fn xyz_to_rgb(xyz: [f32; 3]) -> [f32; 3] {
    let x = xyz[0];
    let y = xyz[1];
    let z = xyz[2];
    [
        3.2404542 * x - 1.5371385 * y - 0.4985314 * z,
        -0.9692660 * x + 1.8760108 * y + 0.0415560 * z,
        0.0556434 * x - 0.2040259 * y + 1.0572252 * z,
    ]
}

#[inline]
fn f_lab(t: f32) -> f32 {
    const DELTA: f32 = 6.0 / 29.0;
    if t > DELTA.powi(3) {
        t.cbrt()
    } else {
        t / (3.0 * DELTA * DELTA) + 4.0 / 29.0
    }
}

#[inline]
fn f_lab_inv(t: f32) -> f32 {
    const DELTA: f32 = 6.0 / 29.0;
    if t > DELTA {
        t.powi(3)
    } else {
        3.0 * DELTA * DELTA * (t - 4.0 / 29.0)
    }
}

pub fn rgb8_to_lab(rgb: [u8; 3]) -> [f32; 3] {
    let linear = srgb8_to_linear(rgb);
    let xyz = rgb_to_xyz(linear);
    let xr = xyz[0] / XYZ_WHITE[0];
    let yr = xyz[1] / XYZ_WHITE[1];
    let zr = xyz[2] / XYZ_WHITE[2];
    let fx = f_lab(xr);
    let fy = f_lab(yr);
    let fz = f_lab(zr);
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

pub fn lab_to_rgb8(lab: [f32; 3]) -> [u8; 3] {
    let l = lab[0];
    let a = lab[1];
    let b = lab[2];
    let fy = (l + 16.0) / 116.0;
    let fx = fy + a / 500.0;
    let fz = fy - b / 200.0;
    let xr = f_lab_inv(fx);
    let yr = f_lab_inv(fy);
    let zr = f_lab_inv(fz);
    let xyz = [xr * XYZ_WHITE[0], yr * XYZ_WHITE[1], zr * XYZ_WHITE[2]];
    let linear = xyz_to_rgb(xyz).map(clamp01);
    linear_to_srgb8(linear)
}

pub fn rgb8_to_luv(rgb: [u8; 3]) -> [f32; 3] {
    let linear = srgb8_to_linear(rgb);
    let xyz = rgb_to_xyz(linear);
    let denom = xyz[0] + 15.0 * xyz[1] + 3.0 * xyz[2];
    let (u_prime, v_prime) = if denom.abs() < EPSILON {
        (0.0, 0.0)
    } else {
        (4.0 * xyz[0] / denom, 9.0 * xyz[1] / denom)
    };

    let denom_ref = XYZ_WHITE[0] + 15.0 * XYZ_WHITE[1] + 3.0 * XYZ_WHITE[2];
    let u_ref = 4.0 * XYZ_WHITE[0] / denom_ref;
    let v_ref = 9.0 * XYZ_WHITE[1] / denom_ref;

    let y_ratio = xyz[1] / XYZ_WHITE[1];
    let l = if y_ratio > (6.0f32 / 29.0).powi(3) {
        116.0 * y_ratio.cbrt() - 16.0
    } else {
        (29.0f32 / 3.0).powi(3) * y_ratio
    };

    let u = 13.0 * l * (u_prime - u_ref);
    let v = 13.0 * l * (v_prime - v_ref);
    [l, u, v]
}

pub fn luv_to_rgb8(luv: [f32; 3]) -> [u8; 3] {
    let l = luv[0];
    if l.abs() < EPSILON {
        return [0, 0, 0];
    }
    let denom_ref = XYZ_WHITE[0] + 15.0 * XYZ_WHITE[1] + 3.0 * XYZ_WHITE[2];
    let u_ref = 4.0 * XYZ_WHITE[0] / denom_ref;
    let v_ref = 9.0 * XYZ_WHITE[1] / denom_ref;
    let u_prime = luv[1] / (13.0 * l) + u_ref;
    let v_prime = luv[2] / (13.0 * l) + v_ref;

    let y = if l > 8.0 {
        ((l + 16.0) / 116.0).powi(3)
    } else {
        l / (29.0f32 / 3.0).powi(3)
    } * XYZ_WHITE[1];

    let denom = 4.0 * v_prime;
    let (x, z) = if denom.abs() < EPSILON {
        (0.0, 0.0)
    } else {
        let x = 9.0 * y * u_prime / denom;
        let z = y * (12.0 - 3.0 * u_prime - 20.0 * v_prime) / denom;
        (x, z)
    };
    let xyz = [x, y, z];
    let linear = xyz_to_rgb(xyz).map(clamp01);
    linear_to_srgb8(linear)
}

pub fn rgb8_to_yuv(rgb: [u8; 3]) -> [f32; 3] {
    let r = rgb[0] as f32 / 255.0;
    let g = rgb[1] as f32 / 255.0;
    let b = rgb[2] as f32 / 255.0;
    [
        0.299 * r + 0.587 * g + 0.114 * b,
        -0.14713 * r - 0.28886 * g + 0.436 * b,
        0.615 * r - 0.51499 * g - 0.10001 * b,
    ]
}

pub fn yuv_to_rgb8(yuv: [f32; 3]) -> [u8; 3] {
    let y = yuv[0];
    let u = yuv[1];
    let v = yuv[2];
    let r = y + 1.13983 * v;
    let g = y - 0.39465 * u - 0.58060 * v;
    let b = y + 2.03211 * u;
    [to_u8(r), to_u8(g), to_u8(b)]
}

pub fn rgb8_to_hsl(rgb: [u8; 3]) -> [f32; 3] {
    let r = rgb[0] as f32 / 255.0;
    let g = rgb[1] as f32 / 255.0;
    let b = rgb[2] as f32 / 255.0;
    let max = r.max(g.max(b));
    let min = r.min(g.min(b));
    let delta = max - min;
    let l = (max + min) * 0.5;

    let s = if delta.abs() < EPSILON {
        0.0
    } else {
        delta / (1.0 - (2.0 * l - 1.0).abs())
    };

    let h = if delta.abs() < EPSILON {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta) % 6.0)
    } else if max == g {
        60.0 * (((b - r) / delta) + 2.0)
    } else {
        60.0 * (((r - g) / delta) + 4.0)
    };
    let h_normalized = if h < 0.0 { h + 360.0 } else { h };
    [h_normalized, s, l]
}

pub fn hsl_to_rgb8(hsl: [f32; 3]) -> [u8; 3] {
    let h = (hsl[0] % 360.0 + 360.0) % 360.0;
    let s = hsl[1].clamp(0.0, 1.0);
    let l = hsl[2].clamp(0.0, 1.0);

    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;

    let (r1, g1, b1) = match h {
        h if h < 60.0 => (c, x, 0.0),
        h if h < 120.0 => (x, c, 0.0),
        h if h < 180.0 => (0.0, c, x),
        h if h < 240.0 => (0.0, x, c),
        h if h < 300.0 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    [to_u8(r1 + m), to_u8(g1 + m), to_u8(b1 + m)]
}

pub fn rgb8_to_hsv(rgb: [u8; 3]) -> [f32; 3] {
    let r = rgb[0] as f32 / 255.0;
    let g = rgb[1] as f32 / 255.0;
    let b = rgb[2] as f32 / 255.0;
    let max = r.max(g.max(b));
    let min = r.min(g.min(b));
    let delta = max - min;
    let v = max;
    let s = if max.abs() < EPSILON {
        0.0
    } else {
        delta / max
    };
    let h = if delta.abs() < EPSILON {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta) % 6.0)
    } else if max == g {
        60.0 * (((b - r) / delta) + 2.0)
    } else {
        60.0 * (((r - g) / delta) + 4.0)
    };
    let h_norm = if h < 0.0 { h + 360.0 } else { h };
    [h_norm, s, v]
}

pub fn hsv_to_rgb8(hsv: [f32; 3]) -> [u8; 3] {
    let h = (hsv[0] % 360.0 + 360.0) % 360.0;
    let s = hsv[1].clamp(0.0, 1.0);
    let v = hsv[2].clamp(0.0, 1.0);

    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r1, g1, b1) = match h {
        h if h < 60.0 => (c, x, 0.0),
        h if h < 120.0 => (x, c, 0.0),
        h if h < 180.0 => (0.0, c, x),
        h if h < 240.0 => (0.0, x, c),
        h if h < 300.0 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    [to_u8(r1 + m), to_u8(g1 + m), to_u8(b1 + m)]
}

pub fn hue_to_radians(h: f32) -> f32 {
    h.to_radians()
}

/// Calculate WCAG contrast ratio between two colors using their Lab L* values
/// Returns ratio in range [1.0, 21.0] where higher is more contrast
pub fn calculate_contrast_ratio(lab1: [f32; 3], lab2: [f32; 3]) -> f32 {
    // WCAG formula uses relative luminance (not Lab L*)
    // Convert Lab back to linear RGB, then calculate relative luminance
    let rgb1 = lab_to_rgb8(lab1);
    let rgb2 = lab_to_rgb8(lab2);

    let linear1 = srgb8_to_linear(rgb1);
    let linear2 = srgb8_to_linear(rgb2);

    // Calculate relative luminance using sRGB coefficients
    let lum1 = 0.2126 * linear1[0] + 0.7152 * linear1[1] + 0.0722 * linear1[2];
    let lum2 = 0.2126 * linear2[0] + 0.7152 * linear2[1] + 0.0722 * linear2[2];

    // WCAG contrast ratio formula: (lighter + 0.05) / (darker + 0.05)
    let lighter = lum1.max(lum2);
    let darker = lum1.min(lum2);
    (lighter + 0.05) / (darker + 0.05)
}

/// Calculate CIE76 Delta E color difference
/// Returns perceptual distance where 0 = identical, >100 = very different
pub fn delta_e_cie76(lab1: [f32; 3], lab2: [f32; 3]) -> f32 {
    let dl = lab1[0] - lab2[0];
    let da = lab1[1] - lab2[1];
    let db = lab1[2] - lab2[2];
    (dl * dl + da * da + db * db).sqrt()
}

/// Convert RGB to hex string format (#RRGGBB)
pub fn rgb_to_hex(rgb: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_rgb_close(a: [u8; 3], b: [u8; 3], tol: u8) {
        for i in 0..3 {
            let diff = a[i].abs_diff(b[i]);
            assert!(diff <= tol, "channel {i} diff {diff} exceeds {tol}");
        }
    }

    #[test]
    fn lab_round_trip() {
        let samples = [[255, 0, 0], [12, 200, 64], [240, 240, 240]];
        for rgb in samples {
            let lab = rgb8_to_lab(rgb);
            let back = lab_to_rgb8(lab);
            assert_rgb_close(rgb, back, 2);
        }
    }

    #[test]
    fn luv_round_trip() {
        let samples = [[18, 42, 200], [250, 128, 114], [0, 0, 0]];
        for rgb in samples {
            let luv = rgb8_to_luv(rgb);
            let back = luv_to_rgb8(luv);
            assert_rgb_close(rgb, back, 3);
        }
    }

    #[test]
    fn yuv_round_trip() {
        let rgb = [128, 200, 32];
        let yuv = rgb8_to_yuv(rgb);
        let back = yuv_to_rgb8(yuv);
        assert_rgb_close(rgb, back, 2);
    }

    #[test]
    fn hsl_round_trip() {
        let rgb = [12, 180, 90];
        let hsl = rgb8_to_hsl(rgb);
        let back = hsl_to_rgb8(hsl);
        assert_rgb_close(rgb, back, 2);
    }

    #[test]
    fn hsv_round_trip() {
        let rgb = [200, 32, 240];
        let hsv = rgb8_to_hsv(rgb);
        let back = hsv_to_rgb8(hsv);
        assert_rgb_close(rgb, back, 2);
    }

    #[test]
    fn known_lab_value_for_red() {
        let lab = rgb8_to_lab([255, 0, 0]);
        assert!((lab[0] - 53.24).abs() < 0.2);
        assert!((lab[1] - 80.09).abs() < 0.5);
        assert!((lab[2] - 67.20).abs() < 0.5);
    }
}
