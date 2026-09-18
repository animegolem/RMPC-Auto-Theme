//! Color space conversions for the Rust compute core.
//! All transforms assume sRGB primaries with a D65 white point (IEC 61966-2-1).
//!
//! Color space algorithms based on:
//! - Color-tool by L. Jégou (CC BY 3.0): https://github.com/ljegou/Color-tool
//! - CIE 15:2018 (Colorimetry, 4th Edition) for LAB/LUV
//! - IEC 61966-2-1:1999 for sRGB gamma and XYZ transforms

const EPSILON: f32 = 1e-6;
const XYZ_WHITE: [f32; 3] = [0.95047, 1.0, 1.08883]; // D65

#[inline]
fn clamp01(v: f32) -> f32 {
    v.clamp(0.0, 1.0)
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

#[allow(clippy::excessive_precision)]
pub fn rgb_to_xyz(linear_rgb: [f32; 3]) -> [f32; 3] {
    let r = linear_rgb[0];
    let g = linear_rgb[1];
    let b = linear_rgb[2];
    [
        0.4124564 * r + 0.3575761 * g + 0.1804375 * b,
        0.2126729 * r + 0.7151522 * g + 0.0721750 * b,
        0.0193339 * r + 0.119_192 * g + 0.9503041 * b,
    ]
}

#[allow(clippy::excessive_precision)]
pub fn xyz_to_rgb(xyz: [f32; 3]) -> [f32; 3] {
    let x = xyz[0];
    let y = xyz[1];
    let z = xyz[2];
    [
        3.2404542 * x - 1.5371385 * y - 0.4985314 * z,
        -0.969_266 * x + 1.8760108 * y + 0.0415560 * z,
        0.0556434 * x - 0.2040259 * y + 1.0572252 * z,
    ]
}

fn normalize_degrees(deg: f32) -> f32 {
    let mut value = deg % 360.0;
    if value < 0.0 {
        value += 360.0;
    }
    value
}

#[allow(clippy::excessive_precision)]
fn linear_rgb_to_oklab(rgb: [f32; 3]) -> [f32; 3] {
    let r = rgb[0];
    let g = rgb[1];
    let b = rgb[2];
    let l = 0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b;
    let m = 0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b;
    let s = 0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b;
    let l_ = l.cbrt();
    let m_ = m.cbrt();
    let s_ = s.cbrt();
    [
        0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_,
        1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_,
        0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_,
    ]
}

#[allow(clippy::excessive_precision)]
fn oklab_to_linear_rgb(lab: [f32; 3]) -> [f32; 3] {
    let l = lab[0];
    let a = lab[1];
    let b = lab[2];
    let l_ = l + 0.3963377774 * a + 0.2158037573 * b;
    let m_ = l - 0.1055613458 * a - 0.0638541728 * b;
    let s_ = l - 0.0894841775 * a - 1.2914855480 * b;
    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;
    [
        4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
        -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
        -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s,
    ]
}

fn is_in_gamut_rgb(rgb: [f32; 3]) -> bool {
    rgb.iter().all(|&c| (-EPSILON..=1.0 + EPSILON).contains(&c))
}

/// Convert sRGB bytes to OKLab (L in 0..1, a/b in OKLab units).
pub fn rgb8_to_oklab(rgb: [u8; 3]) -> [f32; 3] {
    let linear = srgb8_to_linear(rgb);
    linear_rgb_to_oklab(linear)
}

/// Convert OKLab (L in 0..1) to sRGB bytes with simple RGB clamping.
pub fn oklab_to_rgb8(lab: [f32; 3]) -> [u8; 3] {
    let linear = oklab_to_linear_rgb(lab).map(clamp01);
    linear_to_srgb8(linear)
}

/// Convert OKLab to OKLCH (h in degrees, C in OKLab units).
pub fn oklab_to_oklch(lab: [f32; 3]) -> [f32; 3] {
    let l = lab[0];
    let a = lab[1];
    let b = lab[2];
    let c = (a * a + b * b).sqrt();
    let h = if c < EPSILON {
        0.0
    } else {
        normalize_degrees(b.atan2(a).to_degrees())
    };
    [l, c, h]
}

/// Convert OKLCH (h in degrees, C in OKLab units) to OKLab.
pub fn oklch_to_oklab(lch: [f32; 3]) -> [f32; 3] {
    let l = lch[0];
    let c = lch[1].max(0.0);
    let h = normalize_degrees(lch[2]).to_radians();
    let a = c * h.cos();
    let b = c * h.sin();
    [l, a, b]
}

/// Convert OKLab to sRGB bytes using chroma compression to keep L/h stable.
pub fn oklab_to_srgb8_gamut_mapped(lab: [f32; 3]) -> [u8; 3] {
    oklab_to_srgb8_gamut_mapped_with_oklab(lab).0
}

/// Return mapped sRGB bytes and the same post-map OKLab coordinate used to
/// produce them, before transfer encoding and 8-bit quantization.
pub fn oklab_to_srgb8_gamut_mapped_with_oklab(lab: [f32; 3]) -> ([u8; 3], [f32; 3]) {
    let linear = oklab_to_linear_rgb(lab);
    if is_in_gamut_rgb(linear) {
        return (linear_to_srgb8(linear), lab);
    }
    let lch = oklab_to_oklch(lab);
    let l = lch[0];
    let c = lch[1];
    let h = lch[2];
    if c <= 0.0 {
        let mapped = [l, 0.0, 0.0];
        let neutral = oklab_to_linear_rgb(mapped).map(clamp01);
        return (linear_to_srgb8(neutral), mapped);
    }
    let mut low = 0.0;
    let mut high = c;
    let mut best = [l, 0.0, 0.0];
    for _ in 0..24 {
        let mid = (low + high) * 0.5;
        let candidate = oklch_to_oklab([l, mid, h]);
        let linear = oklab_to_linear_rgb(candidate);
        if is_in_gamut_rgb(linear) {
            best = candidate;
            low = mid;
        } else {
            high = mid;
        }
    }
    (
        linear_to_srgb8(oklab_to_linear_rgb(best).map(clamp01)),
        best,
    )
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
    let r = rgb[0] as f32;
    let g = rgb[1] as f32;
    let b = rgb[2] as f32;
    // BT.601 coefficients (matching CC BY 3.0 Color-tool reference)
    let y = r * 0.299 + g * 0.587 + b * 0.114;
    let u = r * -0.168736 + g * -0.331264 + b * 0.5 + 128.0;
    let v = r * 0.5 + g * -0.418688 + b * -0.081312 + 128.0;
    [y, u, v]
}

pub fn yuv_to_rgb8(yuv: [f32; 3]) -> [u8; 3] {
    let y = yuv[0];
    let u = yuv[1];
    let v = yuv[2];
    // BT.601 inverse (matching CC BY 3.0 Color-tool reference)
    let r = y + 1.4075 * (v - 128.0);
    let g = y - 0.3455 * (u - 128.0) - 0.7169 * (v - 128.0);
    let b = y + 1.779 * (u - 128.0);
    [to_u8(r / 255.0), to_u8(g / 255.0), to_u8(b / 255.0)]
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

    fn assert_lab_close(actual: [f32; 3], expected: [f64; 3], tol: f64) {
        for i in 0..3 {
            let diff = (actual[i] as f64 - expected[i]).abs();
            assert!(diff <= tol, "channel {i} diff {diff} exceeds {tol}");
        }
    }

    fn srgb_to_linear_ref(c: f64) -> f64 {
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }

    fn linear_to_srgb_ref(c: f64) -> f64 {
        if c <= 0.0031308 {
            c * 12.92
        } else {
            1.055 * c.powf(1.0 / 2.4) - 0.055
        }
    }

    fn clamp01_ref(v: f64) -> f64 {
        v.clamp(0.0, 1.0)
    }

    fn to_u8_ref(v: f64) -> u8 {
        (clamp01_ref(v) * 255.0 + 0.5).floor() as u8
    }

    fn srgb8_to_linear_ref(rgb: [u8; 3]) -> [f64; 3] {
        rgb.map(|c| srgb_to_linear_ref((c as f64) / 255.0))
    }

    fn linear_to_srgb8_ref(rgb: [f64; 3]) -> [u8; 3] {
        rgb.map(|c| to_u8_ref(linear_to_srgb_ref(c)))
    }

    fn linear_rgb_to_oklab_ref(rgb: [f64; 3]) -> [f64; 3] {
        let r = rgb[0];
        let g = rgb[1];
        let b = rgb[2];
        let l = 0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b;
        let m = 0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b;
        let s = 0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b;
        let l_ = l.cbrt();
        let m_ = m.cbrt();
        let s_ = s.cbrt();
        [
            0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_,
            1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_,
            0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_,
        ]
    }

    fn oklab_to_linear_rgb_ref(lab: [f64; 3]) -> [f64; 3] {
        let l = lab[0];
        let a = lab[1];
        let b = lab[2];
        let l_ = l + 0.3963377774 * a + 0.2158037573 * b;
        let m_ = l - 0.1055613458 * a - 0.0638541728 * b;
        let s_ = l - 0.0894841775 * a - 1.2914855480 * b;
        let l = l_ * l_ * l_;
        let m = m_ * m_ * m_;
        let s = s_ * s_ * s_;
        [
            4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
            -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
            -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s,
        ]
    }

    fn oklab_to_oklch_ref(lab: [f64; 3]) -> [f64; 3] {
        let l = lab[0];
        let a = lab[1];
        let b = lab[2];
        let c = (a * a + b * b).sqrt();
        let h = if c < 1e-12 {
            0.0
        } else {
            let mut deg = b.atan2(a).to_degrees();
            if deg < 0.0 {
                deg += 360.0;
            }
            deg
        };
        [l, c, h]
    }

    fn oklch_to_oklab_ref(lch: [f64; 3]) -> [f64; 3] {
        let l = lch[0];
        let c = lch[1].max(0.0);
        let h = lch[2].to_radians();
        let a = c * h.cos();
        let b = c * h.sin();
        [l, a, b]
    }

    fn is_in_gamut_rgb_ref(rgb: [f64; 3]) -> bool {
        rgb.iter().all(|&c| (-1e-9..=1.0 + 1e-9).contains(&c))
    }

    fn oklab_to_srgb8_gamut_mapped_ref(lab: [f64; 3]) -> [u8; 3] {
        let linear = oklab_to_linear_rgb_ref(lab);
        if is_in_gamut_rgb_ref(linear) {
            return linear_to_srgb8_ref(linear);
        }
        let lch = oklab_to_oklch_ref(lab);
        let l = lch[0];
        let c = lch[1];
        let h = lch[2];
        if c <= 0.0 {
            let neutral = oklab_to_linear_rgb_ref([l, 0.0, 0.0]);
            return linear_to_srgb8_ref(neutral.map(clamp01_ref));
        }
        let mut low = 0.0;
        let mut high = c;
        let mut best = [l, 0.0, 0.0];
        for _ in 0..32 {
            let mid = (low + high) * 0.5;
            let candidate = oklch_to_oklab_ref([l, mid, h]);
            let linear = oklab_to_linear_rgb_ref(candidate);
            if is_in_gamut_rgb_ref(linear) {
                best = candidate;
                low = mid;
            } else {
                high = mid;
            }
        }
        linear_to_srgb8_ref(oklab_to_linear_rgb_ref(best).map(clamp01_ref))
    }

    fn srgb8_to_oklab_ref(rgb: [u8; 3]) -> [f64; 3] {
        linear_rgb_to_oklab_ref(srgb8_to_linear_ref(rgb))
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
    fn oklab_round_trip() {
        let samples = [[18, 42, 200], [250, 128, 114], [240, 240, 240]];
        for rgb in samples {
            let lab = rgb8_to_oklab(rgb);
            let back = oklab_to_rgb8(lab);
            assert_rgb_close(rgb, back, 2);
        }
    }

    #[test]
    fn oklch_round_trip() {
        let lab = rgb8_to_oklab([120, 200, 32]);
        let lch = oklab_to_oklch(lab);
        let back = oklch_to_oklab(lch);
        assert!((lab[0] - back[0]).abs() < 1e-4);
        assert!((lab[1] - back[1]).abs() < 1e-4);
        assert!((lab[2] - back[2]).abs() < 1e-4);
    }

    #[test]
    fn oklab_gamut_mapping_keeps_hue_stable() {
        let lab = [0.65, 0.4, 0.2];
        let lch = oklab_to_oklch(lab);
        let mapped = oklab_to_srgb8_gamut_mapped(lab);
        let mapped_lch = oklab_to_oklch(rgb8_to_oklab(mapped));
        let hue_delta = (lch[2] - mapped_lch[2]).abs();
        let hue_delta = hue_delta.min(360.0 - hue_delta);
        let linear = srgb8_to_linear(mapped);
        assert!(is_in_gamut_rgb(linear));
        assert!(hue_delta < 5.0, "hue drift too large: {hue_delta}");
    }

    #[test]
    fn gamut_mapped_coordinate_is_the_quantizer_input() {
        let in_gamut = rgb8_to_oklab([80, 130, 190]);
        let neutral = [1.2, 0.0, 0.0];
        let compressed = [0.65, 0.4, 0.2];
        let mut boundary = None;
        for other in [0.05, 0.2, 0.4, 0.6, 0.8] {
            for step in 1..10 {
                let lab = linear_rgb_to_oklab([1.0 + EPSILON * (step as f32 / 10.0), other, 0.3]);
                let linear = oklab_to_linear_rgb(lab);
                if is_in_gamut_rgb(linear) && linear.iter().any(|&c| !(0.0..=1.0).contains(&c)) {
                    boundary = Some(lab);
                    break;
                }
            }
        }
        let boundary = boundary.expect("find a channel within the gamut epsilon, beyond the cube");
        assert!(is_in_gamut_rgb(oklab_to_linear_rgb(in_gamut)));
        assert!(!is_in_gamut_rgb(oklab_to_linear_rgb(neutral)));
        assert!(!is_in_gamut_rgb(oklab_to_linear_rgb(compressed)));
        let boundary_linear = oklab_to_linear_rgb(boundary);
        assert!(is_in_gamut_rgb(boundary_linear));
        assert!(boundary_linear.iter().any(|&c| !(0.0..=1.0).contains(&c)));

        for lab in [in_gamut, neutral, compressed, boundary] {
            let (bytes, mapped) = oklab_to_srgb8_gamut_mapped_with_oklab(lab);
            let quantized = linear_to_srgb8(oklab_to_linear_rgb(mapped).map(clamp01));
            assert_eq!(bytes, quantized, "mapped coordinate diverged for {lab:?}");
            assert_eq!(bytes, oklab_to_srgb8_gamut_mapped(lab));
        }
    }

    #[test]
    fn oklab_reference_matches_high_precision() {
        let samples = [
            [255, 0, 0],
            [0, 255, 0],
            [0, 0, 255],
            [128, 128, 128],
            [255, 255, 255],
        ];
        for rgb in samples {
            let expected = srgb8_to_oklab_ref(rgb);
            let actual = rgb8_to_oklab(rgb);
            assert_lab_close(actual, expected, 6e-4);

            let back_expected = oklab_to_srgb8_gamut_mapped_ref(expected);
            let back_actual = oklab_to_srgb8_gamut_mapped([
                expected[0] as f32,
                expected[1] as f32,
                expected[2] as f32,
            ]);
            assert_rgb_close(back_actual, back_expected, 1);
        }
    }

    #[test]
    fn known_lab_value_for_red() {
        let lab = rgb8_to_lab([255, 0, 0]);
        assert!((lab[0] - 53.24).abs() < 0.2);
        assert!((lab[1] - 80.09).abs() < 0.5);
        assert!((lab[2] - 67.20).abs() < 0.5);
    }
}
