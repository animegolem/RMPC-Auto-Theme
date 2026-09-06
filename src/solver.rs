//! Deterministic role assignment and final-output guardrail verification.

use std::collections::HashSet;

use color_core::ClusterOut;

use crate::roles::{AssignedColor, Role, RolePalette, SolverTier, ToneBand, ToneBands};

const NEUTRAL_THRESHOLD: f32 = 0.04;
const NEUTRAL_RAMP_CHROMA: f32 = 0.02;
const RELAXED_PEER_DELTA_E: f32 = 20.0;
const RELAXED_HUE_SEPARATION: f32 = 20.0;

/// Effective final-output guardrail thresholds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GuardrailFloors {
    pub accent_background: f32,
    pub text_background: f32,
    pub active_background: f32,
    pub highlight_active: f32,
    pub frame_background: f32,
    pub peer_delta_e: f32,
    pub hue_separation: f32,
}

impl Default for GuardrailFloors {
    fn default() -> Self {
        Self {
            accent_background: 4.5,
            text_background: 4.5,
            active_background: 3.5,
            highlight_active: 4.5,
            frame_background: 3.0,
            peer_delta_e: 25.0,
            hue_separation: 30.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GuardrailReport {
    pub accent_background: f32,
    pub text_background: f32,
    pub active_background: f32,
    pub highlight_active: f32,
    pub frame_background: f32,
    pub accent_active_delta_e: f32,
    pub accent_active_hue_distance: f32,
    pub accent_active_both_neutral: bool,
    pub accent_active_hue_exempt: bool,
}

/// Assign extracted, dominance-ordered clusters to all eight semantic roles.
pub fn assign_roles(clusters: &[ClusterOut]) -> Result<RolePalette, String> {
    assign_roles_with_config(clusters, &ToneBands::default(), &GuardrailFloors::default())
}

/// Assign roles using caller-supplied search bands and hard floors.
pub fn assign_roles_with_config(
    clusters: &[ClusterOut],
    bands: &ToneBands,
    floors: &GuardrailFloors,
) -> Result<RolePalette, String> {
    if clusters.is_empty() {
        return Err("cannot assign roles from an empty cluster set".to_owned());
    }

    let neutral_hue = inherited_neutral_hue(clusters);
    let background = choose_single(Role::Background, clusters, neutral_hue, bands, |_| true)?;
    let text = choose_single(Role::Text, clusters, neutral_hue, bands, |candidate| {
        wcag_contrast(candidate.rgb, background.rgb) >= floors.text_background
    })?;
    let frame = choose_single(Role::Frame, clusters, neutral_hue, bands, |candidate| {
        wcag_contrast(candidate.rgb, background.rgb) >= floors.frame_background
    })?;
    let header = choose_single(Role::HeaderAccent, clusters, neutral_hue, bands, |_| true)?;

    let (accent, active, highlight) =
        choose_accent_active(clusters, neutral_hue, &background, bands, floors)?;

    let inactive = AssignedColor {
        role: Role::Inactive,
        origin: format!("alias:{}", frame.origin),
        ..frame.clone()
    };
    let palette = RolePalette::new([
        background, text, accent, active, highlight, frame, header, inactive,
    ]);
    validate_palette_with_config(&palette, bands, floors)?;
    Ok(palette)
}

fn choose_single<F>(
    role: Role,
    clusters: &[ClusterOut],
    neutral_hue: f32,
    bands: &ToneBands,
    predicate: F,
) -> Result<AssignedColor, String>
where
    F: Fn(&AssignedColor) -> bool,
{
    for candidate in harvested_candidates(role, clusters, bands) {
        if predicate(&candidate) {
            return Ok(candidate);
        }
    }
    for candidate in synthesized_candidates(role, clusters, bands) {
        if predicate(&candidate) {
            return Ok(candidate);
        }
    }
    for candidate in neutral_candidates(role, neutral_hue, bands) {
        if predicate(&candidate) {
            return Ok(candidate);
        }
    }
    Err(format!("no guardrail-compliant candidate for {role:?}"))
}

fn choose_accent_active(
    clusters: &[ClusterOut],
    neutral_hue: f32,
    background: &AssignedColor,
    bands: &ToneBands,
    floors: &GuardrailFloors,
) -> Result<(AssignedColor, AssignedColor, AssignedColor), String> {
    let accents = harvested_candidates(Role::Accent, clusters, bands);
    let actives = harvested_candidates(Role::Active, clusters, bands);
    let highlights = harvested_candidates(Role::HighlightText, clusters, bands);

    for (tier, delta_floor, hue_floor) in [
        (
            SolverTier::Strict,
            floors.peer_delta_e,
            floors.hue_separation,
        ),
        (
            SolverTier::Relaxed,
            RELAXED_PEER_DELTA_E,
            RELAXED_HUE_SEPARATION,
        ),
    ] {
        if let Some(result) = find_accent_active(
            &accents,
            &actives,
            &highlights,
            background,
            tier,
            delta_floor,
            hue_floor,
            false,
            floors,
        ) {
            return Ok(result);
        }
    }

    // Exhaust every all-harvested pair at both ruled tiers before synthesis.
    let mut synthesized_accents = accents;
    synthesized_accents.extend(synthesized_candidates(Role::Accent, clusters, bands));
    let mut synthesized_actives = actives;
    synthesized_actives.extend(synthesized_candidates(Role::Active, clusters, bands));
    let mut synthesized_highlights = highlights;
    synthesized_highlights.extend(synthesized_candidates(Role::HighlightText, clusters, bands));
    if let Some(result) = find_accent_active(
        &synthesized_accents,
        &synthesized_actives,
        &synthesized_highlights,
        background,
        SolverTier::Relaxed,
        RELAXED_PEER_DELTA_E,
        RELAXED_HUE_SEPARATION,
        true,
        floors,
    ) {
        return Ok(result);
    }

    // The terminal ramp intentionally makes both peers neutral. Rev 0.4
    // exempts that pair from ΔE and hue while retaining every WCAG floor.
    let accents = neutral_candidates(Role::Accent, neutral_hue, bands);
    let actives = neutral_candidates(Role::Active, neutral_hue, bands);
    let highlights = neutral_candidates(Role::HighlightText, neutral_hue, bands);
    for active in actives {
        if wcag_contrast(active.rgb, background.rgb) < floors.active_background {
            continue;
        }
        let Some(highlight) = highlights
            .iter()
            .find(|candidate| wcag_contrast(candidate.rgb, active.rgb) >= floors.highlight_active)
        else {
            continue;
        };
        if let Some(accent) = accents.iter().find(|candidate| {
            wcag_contrast(candidate.rgb, background.rgb) >= floors.accent_background
                && peer_passes(
                    candidate,
                    &active,
                    RELAXED_PEER_DELTA_E,
                    RELAXED_HUE_SEPARATION,
                )
        }) {
            return Ok((accent.clone(), active, highlight.clone()));
        }
    }
    Err("no WCAG-compliant accent/active/highlight assignment".to_owned())
}

#[allow(clippy::too_many_arguments)]
fn find_accent_active(
    accents: &[AssignedColor],
    actives: &[AssignedColor],
    highlights: &[AssignedColor],
    background: &AssignedColor,
    pass_tier: SolverTier,
    delta_floor: f32,
    hue_floor: f32,
    require_synthesis: bool,
    floors: &GuardrailFloors,
) -> Option<(AssignedColor, AssignedColor, AssignedColor)> {
    for active in actives {
        if wcag_contrast(active.rgb, background.rgb) < floors.active_background {
            continue;
        }
        for highlight in highlights {
            if wcag_contrast(highlight.rgb, active.rgb) < floors.highlight_active {
                continue;
            }
            for accent in accents {
                if require_synthesis
                    && accent.tier != SolverTier::Synthesized
                    && active.tier != SolverTier::Synthesized
                    && highlight.tier != SolverTier::Synthesized
                {
                    continue;
                }
                if wcag_contrast(accent.rgb, background.rgb) < floors.accent_background
                    || !peer_passes(accent, active, delta_floor, hue_floor)
                {
                    continue;
                }
                let mut accent = accent.clone();
                let mut active = active.clone();
                let mut highlight = highlight.clone();
                if accent.tier != SolverTier::Synthesized {
                    accent.tier = pass_tier;
                }
                if active.tier != SolverTier::Synthesized {
                    active.tier = pass_tier;
                }
                if highlight.tier != SolverTier::Synthesized {
                    highlight.tier = SolverTier::Strict;
                }
                return Some((accent, active, highlight));
            }
        }
    }
    None
}

fn harvested_candidates(
    role: Role,
    clusters: &[ClusterOut],
    bands: &ToneBands,
) -> Vec<AssignedColor> {
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();
    for (index, cluster) in clusters.iter().enumerate() {
        let source_chroma = cluster.oklch[1];
        let source_hue = cluster.oklch[2];
        if !source_chroma.is_finite()
            || !source_hue.is_finite()
            || source_chroma < NEUTRAL_THRESHOLD
        {
            continue;
        }
        let confidence = cluster.share.clamp(0.0, 1.0) as f32;
        add_projected_candidates(
            &mut candidates,
            &mut seen,
            role,
            source_hue,
            source_chroma,
            confidence,
            format!("cluster[{index}]:projected"),
            SolverTier::Strict,
            true,
            bands,
        );
    }

    candidates
}

fn synthesized_candidates(
    role: Role,
    clusters: &[ClusterOut],
    bands: &ToneBands,
) -> Vec<AssignedColor> {
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();

    // Rev 0.4 counterpart of v1's synthesize_color_between: only art a/b is
    // interpolated; destination-role lightness remains independently owned.
    for left_index in 0..clusters.len().min(4) {
        for right_index in (left_index + 1)..clusters.len().min(4) {
            for candidate in synthesize_color_between(
                role,
                left_index,
                &clusters[left_index],
                right_index,
                &clusters[right_index],
                bands,
            ) {
                if candidate.oklch[1] > NEUTRAL_THRESHOLD && seen.insert(candidate.rgb) {
                    candidates.push(candidate);
                }
            }
        }
    }

    // Rotated hues follow interpolation and only supply peer separation when
    // the artwork has insufficient hue diversity.
    if let Some((index, cluster)) = clusters
        .iter()
        .enumerate()
        .filter(|(_, cluster)| cluster.oklch[1].is_finite())
        .max_by(|(_, left), (_, right)| left.oklch[1].total_cmp(&right.oklch[1]))
    {
        if cluster.oklch[1] >= NEUTRAL_THRESHOLD && cluster.oklch[2].is_finite() {
            for offset in [60.0, -60.0, 120.0, -120.0, 180.0] {
                add_projected_candidates(
                    &mut candidates,
                    &mut seen,
                    role,
                    normalize_hue(cluster.oklch[2] + offset),
                    cluster.oklch[1].max(0.08),
                    (cluster.share.clamp(0.0, 1.0) as f32) * 0.6,
                    format!("cluster[{index}]:synthesized:{offset:+.0}deg"),
                    SolverTier::Synthesized,
                    true,
                    bands,
                );
            }
        }
    }
    candidates
}

fn synthesize_color_between(
    role: Role,
    left_index: usize,
    left: &ClusterOut,
    right_index: usize,
    right: &ClusterOut,
    bands: &ToneBands,
) -> Vec<AssignedColor> {
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();
    if !left.oklab[1].is_finite()
        || !left.oklab[2].is_finite()
        || !right.oklab[1].is_finite()
        || !right.oklab[2].is_finite()
    {
        return candidates;
    }
    for fraction in [0.25_f32, 0.5, 0.75] {
        let a = left.oklab[1] + (right.oklab[1] - left.oklab[1]) * fraction;
        let b = left.oklab[2] + (right.oklab[2] - left.oklab[2]) * fraction;
        let chroma = a.hypot(b);
        if chroma <= NEUTRAL_THRESHOLD {
            continue;
        }
        let hue = normalize_hue(b.atan2(a).to_degrees());
        add_projected_candidates(
            &mut candidates,
            &mut seen,
            role,
            hue,
            chroma,
            ((left.share + right.share) * 0.35).clamp(0.0, 1.0) as f32,
            format!(
                "synthesized-between:cluster[{left_index}]:cluster[{right_index}]:t{fraction:.2}"
            ),
            SolverTier::Synthesized,
            true,
            bands,
        );
    }
    candidates
}

#[allow(clippy::too_many_arguments)]
fn add_projected_candidates(
    candidates: &mut Vec<AssignedColor>,
    seen: &mut HashSet<[u8; 3]>,
    role: Role,
    hue: f32,
    chroma: f32,
    confidence: f32,
    origin: String,
    tier: SolverTier,
    require_chromatic: bool,
    bands: &ToneBands,
) {
    let band = role.tone_band_in(bands);
    for lightness in lightness_samples(band) {
        for factor in [1.0, 0.75, 0.5, 0.3] {
            if let Some(candidate) = projected_color(
                role,
                lightness,
                chroma * factor,
                hue,
                origin.clone(),
                confidence,
                tier,
                band,
            ) {
                if require_chromatic && candidate.oklch[1] <= NEUTRAL_THRESHOLD {
                    continue;
                }
                if seen.insert(candidate.rgb) {
                    candidates.push(candidate);
                }
            }
        }
    }
}

fn neutral_candidates(role: Role, hue: f32, bands: &ToneBands) -> Vec<AssignedColor> {
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();
    let band = role.tone_band_in(bands);
    for lightness in lightness_samples(band) {
        for chroma in [NEUTRAL_RAMP_CHROMA, 0.01, 0.0] {
            if let Some(candidate) = projected_color(
                role,
                lightness,
                chroma,
                hue,
                format!("neutral-ramp:h{hue:.1}"),
                0.25,
                SolverTier::Neutral,
                band,
            ) {
                if candidate.oklch[1] <= NEUTRAL_THRESHOLD && seen.insert(candidate.rgb) {
                    candidates.push(candidate);
                }
            }
        }
    }
    candidates
}

#[allow(clippy::too_many_arguments)]
fn projected_color(
    role: Role,
    lightness: f32,
    chroma: f32,
    hue: f32,
    origin: String,
    confidence: f32,
    tier: SolverTier,
    band: ToneBand,
) -> Option<AssignedColor> {
    let requested = color_core::color::oklch_to_oklab([lightness, chroma.max(0.0), hue]);
    let rgb = color_core::color::oklab_to_srgb8_gamut_mapped(requested);
    // Measurements and stored coordinates describe the bytes we actually emit,
    // not the pre-gamut-map request.
    let oklab = color_core::color::rgb8_to_oklab(rgb);
    if !band.contains(oklab[0]) {
        return None;
    }
    let oklch = color_core::color::oklab_to_oklch(oklab);
    Some(AssignedColor {
        role,
        rgb,
        oklab,
        oklch,
        origin,
        confidence,
        tier,
    })
}

fn lightness_samples(band: ToneBand) -> Vec<f32> {
    let mut samples = vec![band.target, band.min + 0.004, band.max - 0.004];
    for step in 1..=11 {
        samples.push(band.min + (band.max - band.min) * (step as f32 / 12.0));
    }
    samples
}

fn inherited_neutral_hue(clusters: &[ClusterOut]) -> f32 {
    clusters
        .iter()
        .filter(|cluster| cluster.oklch[1].is_finite() && cluster.oklch[2].is_finite())
        .max_by(|left, right| left.oklch[1].total_cmp(&right.oklch[1]))
        .filter(|cluster| cluster.oklch[1] >= 0.02)
        .map_or(250.0, |cluster| normalize_hue(cluster.oklch[2]))
}

fn peer_passes(
    accent: &AssignedColor,
    active: &AssignedColor,
    delta_floor: f32,
    hue_floor: f32,
) -> bool {
    let accent_neutral = accent.oklch[1] <= NEUTRAL_THRESHOLD;
    let active_neutral = active.oklch[1] <= NEUTRAL_THRESHOLD;
    if accent_neutral && active_neutral {
        return true;
    }
    if cie76_delta_e(accent.rgb, active.rgb) < delta_floor {
        return false;
    }
    accent_neutral || active_neutral || hue_distance(accent.oklch[2], active.oklch[2]) >= hue_floor
}

/// Verify all normative floors against the final emitted sRGB bytes.
pub fn validate_palette(palette: &RolePalette) -> Result<GuardrailReport, String> {
    validate_palette_with_config(palette, &ToneBands::default(), &GuardrailFloors::default())
}

/// Verify a palette against the effective bands and floors.
pub fn validate_palette_with_config(
    palette: &RolePalette,
    bands: &ToneBands,
    floors: &GuardrailFloors,
) -> Result<GuardrailReport, String> {
    for role in Role::ALL {
        let color = palette.get(role);
        let band = role.tone_band_in(bands);
        if !band.contains(color.oklab[0]) {
            return Err(format!(
                "{role:?} lightness {} lies outside {:?}",
                color.oklab[0], band
            ));
        }
    }

    let background = palette.get(Role::Background);
    let accent = palette.get(Role::Accent);
    let text = palette.get(Role::Text);
    let active = palette.get(Role::Active);
    let highlight = palette.get(Role::HighlightText);
    let frame = palette.get(Role::Frame);
    let accent_neutral = accent.oklch[1] <= NEUTRAL_THRESHOLD;
    let active_neutral = active.oklch[1] <= NEUTRAL_THRESHOLD;
    let report = GuardrailReport {
        accent_background: wcag_contrast(accent.rgb, background.rgb),
        text_background: wcag_contrast(text.rgb, background.rgb),
        active_background: wcag_contrast(active.rgb, background.rgb),
        highlight_active: wcag_contrast(highlight.rgb, active.rgb),
        frame_background: wcag_contrast(frame.rgb, background.rgb),
        accent_active_delta_e: cie76_delta_e(accent.rgb, active.rgb),
        accent_active_hue_distance: hue_distance(accent.oklch[2], active.oklch[2]),
        accent_active_both_neutral: accent_neutral && active_neutral,
        accent_active_hue_exempt: accent_neutral || active_neutral,
    };

    for (name, actual, floor) in [
        (
            "accent/background",
            report.accent_background,
            floors.accent_background,
        ),
        (
            "text/background",
            report.text_background,
            floors.text_background,
        ),
        (
            "active/background",
            report.active_background,
            floors.active_background,
        ),
        (
            "highlight/active",
            report.highlight_active,
            floors.highlight_active,
        ),
        (
            "frame/background",
            report.frame_background,
            floors.frame_background,
        ),
    ] {
        if actual < floor {
            return Err(format!("{name} contrast {actual:.3} is below {floor:.1}"));
        }
    }

    let (delta_floor, hue_floor) =
        if accent.tier == SolverTier::Strict && active.tier == SolverTier::Strict {
            (floors.peer_delta_e, floors.hue_separation)
        } else {
            (RELAXED_PEER_DELTA_E, RELAXED_HUE_SEPARATION)
        };
    if !peer_passes(accent, active, delta_floor, hue_floor) {
        return Err(format!(
            "accent/active peer floor failed: deltaE {:.3}, hue {:.3}",
            report.accent_active_delta_e, report.accent_active_hue_distance
        ));
    }
    if palette.get(Role::Inactive).rgb != frame.rgb {
        return Err("inactive is not an exact frame alias".to_owned());
    }
    Ok(report)
}

/// WCAG 2.x contrast ratio calculated from final sRGB bytes.
pub fn wcag_contrast(left: [u8; 3], right: [u8; 3]) -> f32 {
    fn channel(byte: u8) -> f64 {
        let value = f64::from(byte) / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    }
    fn luminance(rgb: [u8; 3]) -> f64 {
        0.2126 * channel(rgb[0]) + 0.7152 * channel(rgb[1]) + 0.0722 * channel(rgb[2])
    }
    let left = luminance(left);
    let right = luminance(right);
    ((left.max(right) + 0.05) / (left.min(right) + 0.05)) as f32
}

/// CIE76 distance calculated by independently converting final sRGB bytes to
/// CIELAB (D65). This intentionally does not use color-core's measurement code.
pub fn cie76_delta_e(left: [u8; 3], right: [u8; 3]) -> f32 {
    fn lab(rgb: [u8; 3]) -> [f64; 3] {
        fn linear(byte: u8) -> f64 {
            let value = f64::from(byte) / 255.0;
            if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        }
        fn pivot(value: f64) -> f64 {
            const EPSILON: f64 = 216.0 / 24389.0;
            const KAPPA: f64 = 24389.0 / 27.0;
            if value > EPSILON {
                value.cbrt()
            } else {
                (KAPPA * value + 16.0) / 116.0
            }
        }
        let r = linear(rgb[0]);
        let g = linear(rgb[1]);
        let b = linear(rgb[2]);
        let x = (0.4124 * r + 0.3576 * g + 0.1805 * b) / 0.95047;
        let y = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        let z = (0.0193 * r + 0.1192 * g + 0.9505 * b) / 1.08883;
        let fx = pivot(x);
        let fy = pivot(y);
        let fz = pivot(z);
        [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
    }
    let left = lab(left);
    let right = lab(right);
    let sum = left
        .iter()
        .zip(right)
        .map(|(left, right)| (left - right).powi(2))
        .sum::<f64>();
    sum.sqrt() as f32
}

/// The shortest angular distance between two hues, in degrees.
pub fn hue_distance(left: f32, right: f32) -> f32 {
    let distance = (normalize_hue(left) - normalize_hue(right)).abs();
    distance.min(360.0 - distance)
}

fn normalize_hue(hue: f32) -> f32 {
    hue.rem_euclid(360.0)
}

#[cfg(test)]
mod tests {
    use color_core::RgbValue;

    use super::*;

    fn cluster(l: f32, c: f32, h: f32, share: f64) -> ClusterOut {
        let oklab = color_core::color::oklch_to_oklab([l, c, h]);
        let rgb = color_core::color::oklab_to_srgb8_gamut_mapped(oklab);
        ClusterOut {
            count: (share * 1000.0) as usize,
            share,
            centroid_space: oklab,
            oklab,
            oklch: color_core::color::oklab_to_oklch(oklab),
            rgb: RgbValue {
                r: rgb[0],
                g: rgb[1],
                b: rgb[2],
            },
            hsv: [0.0; 3],
        }
    }

    #[test]
    fn known_measurement_values() {
        assert!((wcag_contrast([0, 0, 0], [255, 255, 255]) - 21.0).abs() < 0.001);
        assert!((cie76_delta_e([0, 0, 0], [255, 255, 255]) - 100.0).abs() < 0.05);
        assert_eq!(hue_distance(350.0, 10.0), 20.0);
    }

    #[test]
    fn monochrome_art_uses_valid_neutral_peer_exemption() {
        let palette = assign_roles(&[cluster(0.03, 0.0, 0.0, 1.0)]).unwrap();
        let report = validate_palette(&palette).unwrap();
        assert!(report.accent_active_both_neutral);
        assert!(report.accent_active_delta_e < 25.0);
        assert_eq!(
            palette.get(Role::Inactive).rgb,
            palette.get(Role::Frame).rgb
        );
    }

    #[test]
    fn exactly_one_neutral_keeps_relaxed_delta_floor() {
        let neutral = AssignedColor {
            role: Role::Accent,
            rgb: [190; 3],
            oklab: [0.80, 0.0, 0.0],
            oklch: [0.80, 0.0, 250.0],
            origin: "test".into(),
            confidence: 1.0,
            tier: SolverTier::Relaxed,
        };
        let close_colored = AssignedColor {
            role: Role::Active,
            rgb: [180; 3],
            oklab: [0.70, 0.05, 0.0],
            oklch: [0.70, 0.05, 250.0],
            origin: "test".into(),
            confidence: 1.0,
            tier: SolverTier::Relaxed,
        };
        assert!(!peer_passes(&neutral, &close_colored, 20.0, 20.0));
    }

    #[test]
    fn rev04_counterexamples_are_repaired_on_final_rgb() {
        let palette = assign_roles(&[
            cluster(0.60, 0.28, 142.0, 0.55),
            cluster(0.48, 0.25, 28.0, 0.30),
            cluster(0.18, 0.20, 300.0, 0.15),
        ])
        .unwrap();
        let report = validate_palette(&palette).unwrap();
        let floors = GuardrailFloors::default();
        assert!(report.highlight_active >= floors.highlight_active);
        assert!(report.frame_background >= floors.frame_background);
    }

    #[test]
    fn between_synthesis_interpolates_chroma_but_role_owns_lightness() {
        let left = cluster(0.02, 0.20, 20.0, 0.6);
        let right = cluster(0.96, 0.18, 110.0, 0.4);
        let candidates =
            synthesize_color_between(Role::Frame, 0, &left, 1, &right, &ToneBands::default());
        assert!(!candidates.is_empty());
        assert!(candidates.iter().all(|candidate| {
            candidate.tier == SolverTier::Synthesized
                && candidate.origin.starts_with("synthesized-between:")
                && Role::Frame.tone_band().contains(candidate.oklab[0])
        }));
    }

    #[test]
    fn synthesis_is_used_only_after_single_hue_harvest_is_exhausted() {
        let palette = assign_roles(&[cluster(0.5, 0.18, 35.0, 1.0)]).unwrap();
        let accent = palette.get(Role::Accent);
        let active = palette.get(Role::Active);
        assert!(
            accent.tier == SolverTier::Synthesized || active.tier == SolverTier::Synthesized,
            "one harvested hue cannot satisfy the peer hue floor"
        );
        validate_palette(&palette).unwrap();
    }

    #[test]
    fn randomized_art_always_stays_in_bands_and_meets_floors() {
        let mut state = 0x9e37_79b9_7f4a_7c15_u64;
        let mut next = || {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            ((state >> 32) as u32) as f32 / u32::MAX as f32
        };
        for _case in 0..96 {
            let mut clusters = Vec::new();
            for index in 0..6 {
                clusters.push(cluster(
                    next(),
                    next() * 0.38,
                    next() * 360.0,
                    f64::from(6 - index) / 21.0,
                ));
            }
            let first = assign_roles(&clusters).unwrap();
            let second = assign_roles(&clusters).unwrap();
            assert_eq!(first, second);
            validate_palette(&first).unwrap();
            for role in Role::ALL {
                assert!(role.tone_band().contains(first.get(role).oklab[0]));
            }
        }
    }
}
