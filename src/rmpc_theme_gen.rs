use std::cmp::Ordering;
use std::path::PathBuf;
use std::time::Instant;

use anyhow::{Context, Result};
use clap::Parser;
use serde::Serialize;

mod color;
mod image_pipeline;
mod kmeans;

use crate::image_pipeline::{prepare_samples, SampleParams};
use crate::kmeans::{run_kmeans, KMeansConfig};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
enum ColorRole {
    Background,
    Text,
    Accent,
    Border,
    ActiveItem,
    InactiveItem,
    ProgressBar,
    Scrollbar,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RoleAssignment {
    role: ColorRole,
    rgb: RgbValue,
    hsv: [f32; 3],
    lab: [f32; 3],
    hex: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_cluster_index: Option<usize>,
    confidence: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    contrast_against_background: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    contrast_against_text: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    origin: Option<String>,
}

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser, Debug)]
#[command(name = "rmpc-theme-gen", version = APP_VERSION)]
#[command(about = "Generate rmpc theme from album art", long_about = None)]
struct Args {
    /// Path to album art image
    #[arg(short, long)]
    image: PathBuf,

    /// Number of color clusters to extract
    #[arg(short, long, default_value = "12")]
    k: usize,

    /// Color space for clustering (CIELAB, RGB, HSL, HSV, YUV, CIELUV)
    #[arg(short, long, default_value = "CIELAB")]
    space: String,

    /// Output file path (stdout if not specified)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Generate and write theme file (RON format) to specified path
    #[arg(long)]
    theme_output: Option<PathBuf>,

    /// Disable scrollbar block in generated theme
    #[arg(long)]
    disable_scrollbar: bool,
}

#[derive(Debug, Serialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
struct RgbValue {
    r: u8,
    g: u8,
    b: u8,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ColorCluster {
    rgb: RgbValue,
    hsv: [f32; 3],
    lab: [f32; 3],
    count: usize,
    share: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ThemeGenOutput {
    version: String,
    clusters: Vec<ColorCluster>,
    role_assignments: Vec<RoleAssignment>,
    total_samples: usize,
    iterations: usize,
    duration_ms: f64,
    color_space: String,
    scrollbar_enabled: bool,
}

/// Select background color: prefer most dominant with reasonable saturation/lightness
fn select_background(clusters: &[ColorCluster]) -> (usize, f32) {
    // Prefer dominant colors with moderate properties
    for (idx, cluster) in clusters.iter().enumerate() {
        let s = cluster.hsv[1];
        let l = cluster.lab[0];

        // Good background: low-mid saturation, reasonable lightness
        if s < 0.4 && l > 15.0 && l < 85.0 {
            return (idx, 0.9);
        }
    }

    // Fallback: most dominant color regardless of properties
    (0, 0.5)
}

/// Select text color: highest contrast against background
fn select_text_color(clusters: &[ColorCluster], bg_lab: [f32; 3]) -> (usize, f32) {
    let mut best_idx = 0;
    let mut best_contrast = 0.0;

    for (idx, cluster) in clusters.iter().enumerate() {
        let contrast = color::calculate_contrast_ratio(bg_lab, cluster.lab);
        if contrast > best_contrast {
            best_contrast = contrast;
            best_idx = idx;
        }
    }

    // Check if we meet WCAG AA standard (4.5:1)
    let confidence = if best_contrast >= 4.5 { 0.9 } else { 0.6 };
    (best_idx, confidence)
}

#[derive(Clone, Debug)]
struct Candidate {
    index: usize,
    score: f32,
}

fn sort_candidates_desc(candidates: &mut Vec<Candidate>) {
    candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
}

/// Rank accent color candidates: high saturation with good contrast
fn rank_accent_candidates(
    clusters: &[ColorCluster],
    bg_lab: [f32; 3],
    used_indices: &[usize],
) -> Vec<Candidate> {
    let mut candidates = Vec::new();

    for (idx, cluster) in clusters.iter().enumerate() {
        if used_indices.contains(&idx) {
            continue;
        }

        let s = cluster.hsv[1];
        let contrast = color::calculate_contrast_ratio(bg_lab, cluster.lab);
        let score = s * 2.0 + (contrast / 21.0) * 3.0;

        if contrast > 1.5 {
            candidates.push(Candidate { index: idx, score });
        }
    }

    sort_candidates_desc(&mut candidates);
    candidates
}

/// Select border color: mid-saturation, distinct from background
fn select_border_color(
    clusters: &[ColorCluster],
    bg_lab: [f32; 3],
    used_indices: &[usize],
) -> (usize, f32) {
    let mut best_idx = 0;
    let mut best_score = 0.0;

    for (idx, cluster) in clusters.iter().enumerate() {
        if used_indices.contains(&idx) {
            continue;
        }

        let s = cluster.hsv[1];
        let delta_e = color::delta_e_cie76(bg_lab, cluster.lab);

        // Prefer mid-saturation with good perceptual distance
        let score = if s >= 0.2 && s <= 0.6 {
            delta_e / 100.0 + s
        } else {
            delta_e / 100.0
        };

        if score > best_score && delta_e > 20.0 {
            best_score = score;
            best_idx = idx;
        }
    }

    let confidence = if best_score > 0.5 { 0.8 } else { 0.5 };
    (best_idx, confidence)
}

/// Rank active item color candidates: bright and saturated
fn rank_active_item_candidates(
    clusters: &[ColorCluster],
    bg_lab: [f32; 3],
    used_indices: &[usize],
) -> Vec<Candidate> {
    let mut candidates = Vec::new();

    for (idx, cluster) in clusters.iter().enumerate() {
        if used_indices.contains(&idx) {
            continue;
        }

        let s = cluster.hsv[1];
        let v = cluster.hsv[2];
        let contrast = color::calculate_contrast_ratio(bg_lab, cluster.lab);
        let score = v + s + (contrast / 21.0);

        if v > 0.4 {
            candidates.push(Candidate { index: idx, score });
        }
    }

    sort_candidates_desc(&mut candidates);
    candidates
}

#[derive(Clone, Copy)]
struct GuardrailConfig {
    min_contrast_bg: f32,
    min_contrast_text: f32,
    min_contrast_peer: Option<f32>,
    min_delta_e_peer: Option<f32>,
    adjust_step: f32,
    max_adjust_steps: usize,
}

fn contrast_metrics(lab: [f32; 3], bg_lab: [f32; 3], text_lab: [f32; 3]) -> (f32, f32) {
    let contrast_bg = color::calculate_contrast_ratio(lab, bg_lab);
    let contrast_text = color::calculate_contrast_ratio(lab, text_lab);
    (contrast_bg, contrast_text)
}

fn meets_guardrails(
    contrast_bg: f32,
    contrast_text: f32,
    config: &GuardrailConfig,
    peer_contrast: Option<f32>,
    peer_delta_e: Option<f32>,
) -> bool {
    if contrast_bg < config.min_contrast_bg || contrast_text < config.min_contrast_text {
        return false;
    }

    if let Some(min_peer_contrast) = config.min_contrast_peer {
        if peer_contrast.map_or(true, |actual| actual < min_peer_contrast) {
            return false;
        }
    }

    if let Some(min_peer_delta_e) = config.min_delta_e_peer {
        if peer_delta_e.map_or(true, |actual| actual < min_peer_delta_e) {
            return false;
        }
    }

    true
}

fn adjust_lightness_for_contrast(
    base_lab: [f32; 3],
    bg_lab: [f32; 3],
    text_lab: [f32; 3],
    config: &GuardrailConfig,
    peer_lab: Option<[f32; 3]>,
) -> Option<[f32; 3]> {
    for direction in [-1.0f32, 1.0f32] {
        let mut candidate = base_lab;
        for _ in 0..config.max_adjust_steps {
            candidate[0] = (candidate[0] + direction * config.adjust_step).clamp(0.0, 100.0);
            let (contrast_bg, contrast_text) = contrast_metrics(candidate, bg_lab, text_lab);
            let peer_metrics = peer_lab.map(|peer| {
                (
                    color::calculate_contrast_ratio(candidate, peer),
                    color::delta_e_cie76(candidate, peer),
                )
            });
            let (peer_contrast, peer_delta_e) = peer_metrics.unwrap_or((f32::NAN, f32::NAN));
            if meets_guardrails(
                contrast_bg,
                contrast_text,
                config,
                peer_lab.map(|_| peer_contrast),
                peer_lab.map(|_| peer_delta_e),
            ) {
                return Some(candidate);
            }
        }
    }
    None
}

fn synthesize_color_between(
    bg_lab: [f32; 3],
    text_lab: [f32; 3],
    config: &GuardrailConfig,
    peer_lab: Option<[f32; 3]>,
) -> Option<[f32; 3]> {
    let mut t = -0.3f32;
    while t <= 1.3 {
        let candidate = [
            bg_lab[0] + (text_lab[0] - bg_lab[0]) * t,
            bg_lab[1] + (text_lab[1] - bg_lab[1]) * t,
            bg_lab[2] + (text_lab[2] - bg_lab[2]) * t,
        ];
        let (contrast_bg, contrast_text) = contrast_metrics(candidate, bg_lab, text_lab);
        let peer_metrics = peer_lab.map(|peer| {
            (
                color::calculate_contrast_ratio(candidate, peer),
                color::delta_e_cie76(candidate, peer),
            )
        });
        let (peer_contrast, peer_delta_e) = peer_metrics.unwrap_or((f32::NAN, f32::NAN));
        if meets_guardrails(
            contrast_bg,
            contrast_text,
            config,
            peer_lab.map(|_| peer_contrast),
            peer_lab.map(|_| peer_delta_e),
        ) {
            return Some(candidate);
        }
        t += 0.05;
    }
    None
}

fn role_assignment_from_lab(
    role: ColorRole,
    lab: [f32; 3],
    source_cluster_index: Option<usize>,
    confidence: f32,
    origin: Option<&str>,
    bg_lab: Option<[f32; 3]>,
    text_lab: Option<[f32; 3]>,
) -> RoleAssignment {
    let rgb = color::lab_to_rgb8(lab);
    let hsv = color::rgb8_to_hsv(rgb);
    let canonical_lab = color::rgb8_to_lab(rgb);
    let hex = color::rgb_to_hex(rgb);

    let contrast_against_background =
        bg_lab.map(|bg| color::calculate_contrast_ratio(canonical_lab, bg));
    let contrast_against_text =
        text_lab.map(|text| color::calculate_contrast_ratio(canonical_lab, text));

    RoleAssignment {
        role,
        rgb: RgbValue {
            r: rgb[0],
            g: rgb[1],
            b: rgb[2],
        },
        hsv,
        lab: canonical_lab,
        hex,
        source_cluster_index,
        confidence,
        contrast_against_background,
        contrast_against_text,
        origin: origin.map(|s| s.to_string()),
    }
}

fn clone_for_role(role: ColorRole, base: &RoleAssignment, confidence: f32) -> RoleAssignment {
    RoleAssignment {
        role,
        rgb: base.rgb,
        hsv: base.hsv,
        lab: base.lab,
        hex: base.hex.clone(),
        source_cluster_index: base.source_cluster_index,
        confidence,
        contrast_against_background: base.contrast_against_background,
        contrast_against_text: base.contrast_against_text,
        origin: base.origin.clone(),
    }
}

fn resolve_guarded_color(
    role: ColorRole,
    clusters: &[ColorCluster],
    candidates: &[Candidate],
    used_indices: &mut Vec<usize>,
    bg_lab: [f32; 3],
    text_lab: [f32; 3],
    config: GuardrailConfig,
    confidence_from_score: impl Fn(f32) -> f32,
    peer_lab: Option<[f32; 3]>,
) -> RoleAssignment {
    for candidate in candidates {
        let cluster = &clusters[candidate.index];
        let base_lab = cluster.lab;
        let (contrast_bg, contrast_text) = contrast_metrics(base_lab, bg_lab, text_lab);
        let peer_metrics = peer_lab.map(|peer| {
            (
                color::calculate_contrast_ratio(base_lab, peer),
                color::delta_e_cie76(base_lab, peer),
            )
        });
        let (peer_contrast, peer_delta_e) = peer_metrics.unwrap_or((f32::NAN, f32::NAN));
        if meets_guardrails(
            contrast_bg,
            contrast_text,
            &config,
            peer_lab.map(|_| peer_contrast),
            peer_lab.map(|_| peer_delta_e),
        ) {
            used_indices.push(candidate.index);
            return role_assignment_from_lab(
                role,
                base_lab,
                Some(candidate.index),
                confidence_from_score(candidate.score),
                Some("cluster"),
                Some(bg_lab),
                Some(text_lab),
            );
        }

        if let Some(adjusted_lab) =
            adjust_lightness_for_contrast(base_lab, bg_lab, text_lab, &config, peer_lab)
        {
            used_indices.push(candidate.index);
            return role_assignment_from_lab(
                role,
                adjusted_lab,
                Some(candidate.index),
                confidence_from_score(candidate.score) * 0.8,
                Some("adjusted"),
                Some(bg_lab),
                Some(text_lab),
            );
        }
    }

    if let Some(lab) = synthesize_color_between(bg_lab, text_lab, &config, peer_lab) {
        return role_assignment_from_lab(
            role,
            lab,
            None,
            0.45,
            Some("synthetic"),
            Some(bg_lab),
            Some(text_lab),
        );
    }

    if let Some(lab) = adjust_lightness_for_contrast(bg_lab, bg_lab, text_lab, &config, peer_lab) {
        return role_assignment_from_lab(
            role,
            lab,
            None,
            0.4,
            Some("synthetic"),
            Some(bg_lab),
            Some(text_lab),
        );
    }

    if let Some(lab) = adjust_lightness_for_contrast(text_lab, bg_lab, text_lab, &config, peer_lab)
    {
        return role_assignment_from_lab(
            role,
            lab,
            None,
            0.35,
            Some("synthetic"),
            Some(bg_lab),
            Some(text_lab),
        );
    }

    for rgb in [[0u8, 0, 0], [255, 255, 255]] {
        let lab_candidate = color::rgb8_to_lab(rgb);
        let (contrast_bg, contrast_text) = contrast_metrics(lab_candidate, bg_lab, text_lab);
        let peer_metrics = peer_lab.map(|peer| {
            (
                color::calculate_contrast_ratio(lab_candidate, peer),
                color::delta_e_cie76(lab_candidate, peer),
            )
        });
        let (peer_contrast, peer_delta_e) = peer_metrics.unwrap_or((f32::NAN, f32::NAN));
        if meets_guardrails(
            contrast_bg,
            contrast_text,
            &config,
            peer_lab.map(|_| peer_contrast),
            peer_lab.map(|_| peer_delta_e),
        ) {
            return role_assignment_from_lab(
                role,
                lab_candidate,
                None,
                0.3,
                Some("synthetic"),
                Some(bg_lab),
                Some(text_lab),
            );
        }
    }

    role_assignment_from_lab(
        role,
        text_lab,
        None,
        0.25,
        Some("fallback"),
        Some(bg_lab),
        Some(text_lab),
    )
}

/// Generate synthetic light text color as fallback
fn generate_light_text() -> ([u8; 3], [f32; 3], [f32; 3]) {
    let rgb = [220, 220, 220];
    let hsv = color::rgb8_to_hsv(rgb);
    let lab = color::rgb8_to_lab(rgb);
    (rgb, hsv, lab)
}

/// Generate synthetic dark text color as fallback
fn generate_dark_text() -> ([u8; 3], [f32; 3], [f32; 3]) {
    let rgb = [30, 30, 30];
    let hsv = color::rgb8_to_hsv(rgb);
    let lab = color::rgb8_to_lab(rgb);
    (rgb, hsv, lab)
}

/// Generate RON theme file content from role assignments
fn generate_theme_ron(assignments: &[RoleAssignment], scrollbar_enabled: bool) -> String {
    // Find role assignments
    let bg = assignments
        .iter()
        .find(|a| a.role == ColorRole::Background)
        .unwrap();
    let text = assignments
        .iter()
        .find(|a| a.role == ColorRole::Text)
        .unwrap();
    let accent = assignments
        .iter()
        .find(|a| a.role == ColorRole::Accent)
        .unwrap();
    let border = assignments
        .iter()
        .find(|a| a.role == ColorRole::Border)
        .unwrap();
    let active = assignments
        .iter()
        .find(|a| a.role == ColorRole::ActiveItem)
        .unwrap();
    let inactive = assignments
        .iter()
        .find(|a| a.role == ColorRole::InactiveItem)
        .unwrap();

    let scrollbar_block = if scrollbar_enabled {
        format!(
            "    scrollbar: (\n        symbols: [\"│\", \"█\", \"▲\", \"▼\"],\n        track_style: (fg: \"{}\", bg: \"{}\"),\n        ends_style: (fg: \"{}\", bg: \"{}\"),\n        thumb_style: (fg: \"{}\", bg: \"{}\"),\n    ),\n",
            bg.hex, bg.hex, bg.hex, bg.hex, accent.hex, bg.hex
        )
    } else {
        "    scrollbar: None,\n".to_string()
    };

    format!(
        r#"#![enable(implicit_some)]
#![enable(unwrap_newtypes)]
#![enable(unwrap_variant_newtypes)]
(
    default_album_art_path: None,
    show_song_table_header: true,
    draw_borders: true,
    format_tag_separator: " | ",
    browser_column_widths: [20, 38, 42],
    background_color: "{}",
    text_color: "{}",
    header_background_color: "{}",
    modal_background_color: "{}",
    modal_backdrop: false,
    preview_label_style: (fg: "{}", bg: "{}"),
    preview_metadata_group_style: (fg: "{}", bg: "{}", modifiers: "Bold"),
    tab_bar: (
        enabled: true,
        active_style: (fg: "{}", bg: "{}", modifiers: "Bold"),
        inactive_style: (fg: "{}", bg: "{}"),
    ),
    highlighted_item_style: (fg: "{}", bg: "{}", modifiers: "Bold"),
    current_item_style: (fg: "{}", bg: "{}", modifiers: "Bold"),
    borders_style: (fg: "{}"),
    highlight_border_style: (fg: "{}"),
    symbols: (
        song: "",
        dir: "",
        playlist: "P",
        marker: "M",
        ellipsis: "...",
        song_style: None,
        dir_style: None,
        playlist_style: None,
    ),
    level_styles: (
        info: (fg: "{}", bg: "{}"),
        warn: (fg: "{}", bg: "{}"),
        error: (fg: "{}", bg: "{}"),
        debug: (fg: "{}", bg: "{}"),
        trace: (fg: "{}", bg: "{}"),
    ),
    progress_bar: (
        symbols: ["[", "=", ">", " ", "]"],
        track_style: (fg: "{}", bg: "{}"),
        elapsed_style: (fg: "{}", bg: "{}"),
        thumb_style: (fg: "{}", bg: "{}"),
    ),
__SCROLLBAR_BLOCK__
    song_table_format: [
        (
            prop: (kind: Property(Artist),
                default: (kind: Text("Unknown"))
            ),
            width: "20%",
        ),
        (
            prop: (kind: Property(Title),
                default: (kind: Text("Unknown"))
            ),
            width: "35%",
        ),
        (
            prop: (kind: Property(Album), style: (fg: "{}", bg: "{}"),
                default: (kind: Text("Unknown Album"), style: (fg: "{}", bg: "{}"))
            ),
            width: "30%",
        ),
        (
            prop: (kind: Property(Duration),
                default: (kind: Text("-"))
            ),
            width: "15%",
            alignment: Right,
        ),
    ],
    components: {{}},
    layout: Split(
        direction: Vertical,
        panes: [
            (
                pane: Pane(Header),
                size: "2",
            ),
            (
                pane: Pane(Tabs),
                size: "3",
            ),
            (
                pane: Pane(TabContent),
                size: "100%",
            ),
            (
                pane: Pane(ProgressBar),
                size: "1",
            ),
        ],
    ),
    header: (
        rows: [
            (
                left: [
                    (kind: Text("["), style: (fg: "{}", modifiers: "Bold")),
                    (kind: Property(Status(StateV2(playing_label: "Playing", paused_label: "Paused", stopped_label: "Stopped"))), style: (fg: "{}", modifiers: "Bold")),
                    (kind: Text("]"), style: (fg: "{}", modifiers: "Bold"))
                ],
                center: [
                    (kind: Property(Song(Title)), style: (modifiers: "Bold"),
                        default: (kind: Text("No Song"), style: (modifiers: "Bold"))
                    )
                ],
                right: [
                    (kind: Property(Widget(ScanStatus)), style: (fg: "{}")),
                    (kind: Property(Widget(Volume)), style: (fg: "{}"))
                ]
            ),
            (
                left: [
                    (kind: Property(Status(Elapsed))),
                    (kind: Text(" / ")),
                    (kind: Property(Status(Duration))),
                    (kind: Text(" (")),
                    (kind: Property(Status(Bitrate))),
                    (kind: Text(" kbps)"))
                ],
                center: [
                    (kind: Property(Song(Artist)), style: (fg: "{}", modifiers: "Bold"),
                        default: (kind: Text("Unknown"), style: (fg: "{}", modifiers: "Bold"))
                    ),
                    (kind: Text(" - ")),
                    (kind: Property(Song(Album)),
                        default: (kind: Text("Unknown Album"))
                    )
                ],
                right: [
                    (
                        kind: Property(Widget(States(
                            active_style: (fg: "{}", modifiers: "Bold"),
                            separator_style: (fg: "{}")))
                        ),
                        style: (fg: "{}")
                    ),
                ]
            ),
        ],
    ),
    browser_song_format: [
        (
            kind: Group([
                (kind: Property(Track)),
                (kind: Text(" ")),
            ])
        ),
        (
            kind: Group([
                (kind: Property(Artist)),
                (kind: Text(" - ")),
                (kind: Property(Title)),
            ]),
            default: (kind: Property(Filename))
        ),
    ],
    lyrics: (
        timestamp: false
    )
)
"#,
        // Global colors
        bg.hex,          // background_color
        text.hex,        // text_color
        bg.hex,          // header_background_color
        bg.hex,          // modal_background_color
        accent.hex, bg.hex,      // preview_label_style
        accent.hex, bg.hex,      // preview_metadata_group_style
        // Tab bar
        text.hex,        // tab_bar.active_style.fg
        active.hex,      // tab_bar.active_style.bg
        inactive.hex,    // tab_bar.inactive_style.fg
        bg.hex,          // tab_bar.inactive_style.bg
        // Item styles
        accent.hex, bg.hex,      // highlighted_item_style (fg, bg)
        text.hex,        // current_item_style.fg
        active.hex,      // current_item_style.bg
        border.hex,      // borders_style.fg
        accent.hex,      // highlight_border_style.fg
        // Level styles (info, warn, error, debug, trace)
        accent.hex, bg.hex,      // info
        "#f0c674", bg.hex,       // warn (yellowish)
        "#cc6666", bg.hex,       // error (reddish)
        "#b5bd68", bg.hex,       // debug (greenish)
        "#b294bb", bg.hex,       // trace (purplish)
        // Progress bar
        inactive.hex, bg.hex,    // progress_bar.track_style (fg, bg)
        active.hex,   bg.hex,    // progress_bar.elapsed_style (fg, bg)
        active.hex,             // progress_bar.thumb_style.fg
        bg.hex,                 // progress_bar.thumb_style.bg
        // Song table format
        text.hex, bg.hex,        // album style (fg, bg)
        text.hex, bg.hex,        // album default style (fg, bg)
        // Header row 1
        accent.hex,      // status bracket [
        accent.hex,      // status text
        accent.hex,      // status bracket ]
        accent.hex,      // scan status
        accent.hex,      // volume
        // Header row 2
        accent.hex,      // artist style fg
        accent.hex,      // artist default style fg
        // States widget
        text.hex,        // active_style.fg
        text.hex,        // separator_style.fg
        inactive.hex,    // style.fg
    )
    .replace("__SCROLLBAR_BLOCK__", &scrollbar_block)
}

/// Map color clusters to UI element roles
fn map_colors_to_roles(clusters: &[ColorCluster]) -> Vec<RoleAssignment> {
    let mut assignments = Vec::new();
    let mut used_indices = Vec::new();

    // 1. Background (most dominant, reasonable properties)
    let (bg_idx, bg_conf) = select_background(clusters);
    let bg_cluster = &clusters[bg_idx];
    let bg_assignment = role_assignment_from_lab(
        ColorRole::Background,
        bg_cluster.lab,
        Some(bg_idx),
        bg_conf,
        Some("cluster"),
        None,
        None,
    );
    let bg_lab = bg_assignment.lab;
    assignments.push(bg_assignment);
    used_indices.push(bg_idx);

    // 2. Text color with fallback to light/dark synthetic values if needed
    let (text_idx, mut text_conf) = select_text_color(clusters, bg_lab);
    let text_cluster = &clusters[text_idx];
    let mut text_lab = text_cluster.lab;
    let mut text_source = Some(text_idx);
    let mut text_origin = "cluster";
    if color::calculate_contrast_ratio(bg_lab, text_lab) < 4.5 {
        text_conf = 0.45;
        text_source = None;
        text_origin = "synthetic";
        let (_, _, lab) = if bg_lab[0] < 50.0 {
            generate_light_text()
        } else {
            generate_dark_text()
        };
        text_lab = lab;
    } else {
        used_indices.push(text_idx);
    }

    let text_assignment = role_assignment_from_lab(
        ColorRole::Text,
        text_lab,
        text_source,
        text_conf,
        Some(text_origin),
        Some(bg_lab),
        None,
    );
    let text_lab = text_assignment.lab;
    assignments.push(text_assignment);

    // 3. Accent color with contrast guardrails
    let accent_candidates = rank_accent_candidates(clusters, bg_lab, &used_indices);
    let accent_assignment = resolve_guarded_color(
        ColorRole::Accent,
        clusters,
        &accent_candidates,
        &mut used_indices,
        bg_lab,
        text_lab,
        GuardrailConfig {
            min_contrast_bg: 3.0,
            min_contrast_text: 4.5,
            min_contrast_peer: None,
            min_delta_e_peer: None,
            adjust_step: 4.0,
            max_adjust_steps: 12,
        },
        |score| if score > 2.0 { 0.85 } else { 0.6 },
        None,
    );
    assignments.push(accent_assignment.clone());

    // 4. Border color (distinct from background)
    let (border_idx, border_conf) = select_border_color(clusters, bg_lab, &used_indices);
    let border_cluster = &clusters[border_idx];
    used_indices.push(border_idx);
    let border_assignment = role_assignment_from_lab(
        ColorRole::Border,
        border_cluster.lab,
        Some(border_idx),
        border_conf,
        Some("cluster"),
        None,
        None,
    );
    assignments.push(border_assignment.clone());

    // 5. Active item color with guardrails (used as background behind text)
    let active_candidates = rank_active_item_candidates(clusters, bg_lab, &used_indices);
    let active_assignment = resolve_guarded_color(
        ColorRole::ActiveItem,
        clusters,
        &active_candidates,
        &mut used_indices,
        bg_lab,
        text_lab,
        GuardrailConfig {
            min_contrast_bg: 2.0,
            min_contrast_text: 4.5,
            min_contrast_peer: Some(4.5),
            min_delta_e_peer: Some(25.0),
            adjust_step: 4.0,
            max_adjust_steps: 12,
        },
        |score| if score > 1.5 { 0.85 } else { 0.6 },
        Some(accent_assignment.lab),
    );
    assignments.push(active_assignment.clone());

    // 6. Inactive/muted - reuse border color
    assignments.push(clone_for_role(
        ColorRole::InactiveItem,
        &border_assignment,
        0.7,
    ));

    // 7. Progress bar - reuse accent color
    assignments.push(clone_for_role(
        ColorRole::ProgressBar,
        &accent_assignment,
        accent_assignment.confidence,
    ));

    // 8. Scrollbar - reuse active color
    assignments.push(clone_for_role(
        ColorRole::Scrollbar,
        &active_assignment,
        active_assignment.confidence,
    ));

    assignments
}

fn main() -> Result<()> {
    let args = Args::parse();
    let start = Instant::now();

    // Validate image path exists
    if !args.image.exists() {
        anyhow::bail!("Image file not found: {}", args.image.display());
    }

    // Prepare sampling parameters
    let sample_params = SampleParams {
        path: args.image.clone(),
        stride: 4,
        min_lum: 0,
        max_samples: 300_000,
        max_dimension: Some(3200),
        seed: 1,
    };

    // Sample pixels from image
    let sample_result =
        prepare_samples(&sample_params).context("Failed to load and sample image")?;

    if sample_result.samples.is_empty() {
        anyhow::bail!("No pixels sampled from image");
    }

    // Convert samples to chosen color space
    let space_upper = args.space.to_ascii_uppercase();
    let dataset: Vec<[f32; 3]> = match space_upper.as_str() {
        "CIELAB" | "LAB" => sample_result
            .samples
            .iter()
            .map(|&rgb| color::rgb8_to_lab(rgb))
            .collect(),
        "RGB" => sample_result
            .samples
            .iter()
            .map(|&rgb| [rgb[0] as f32, rgb[1] as f32, rgb[2] as f32])
            .collect(),
        "HSL" => sample_result
            .samples
            .iter()
            .map(|&rgb| color::rgb8_to_hsl(rgb))
            .collect(),
        "HSV" => sample_result
            .samples
            .iter()
            .map(|&rgb| color::rgb8_to_hsv(rgb))
            .collect(),
        "YUV" => sample_result
            .samples
            .iter()
            .map(|&rgb| color::rgb8_to_yuv(rgb))
            .collect(),
        "CIELUV" | "LUV" => sample_result
            .samples
            .iter()
            .map(|&rgb| color::rgb8_to_luv(rgb))
            .collect(),
        _ => anyhow::bail!("Unsupported color space: {}", args.space),
    };

    // Run K-means clustering
    let k = args.k.min(dataset.len().max(1));
    let kmeans_config = KMeansConfig {
        k,
        max_iters: 40,
        tol: 1e-3,
        seed: 1,
        warm_start: None,
        mini_batch: None,
    };

    let kmeans_result = run_kmeans(&dataset, &kmeans_config);

    // Convert centroids to all color spaces
    let mut clusters: Vec<ColorCluster> = Vec::with_capacity(kmeans_result.centroids.len());
    for (centroid, &count) in kmeans_result
        .centroids
        .iter()
        .zip(kmeans_result.counts.iter())
    {
        if count == 0 {
            continue;
        }

        // Convert centroid from clustering space to RGB
        let rgb_u8 = match space_upper.as_str() {
            "RGB" => [
                centroid[0].round().clamp(0.0, 255.0) as u8,
                centroid[1].round().clamp(0.0, 255.0) as u8,
                centroid[2].round().clamp(0.0, 255.0) as u8,
            ],
            "HSL" => color::hsl_to_rgb8(*centroid),
            "HSV" => color::hsv_to_rgb8(*centroid),
            "YUV" => color::yuv_to_rgb8(*centroid),
            "CIELAB" | "LAB" => color::lab_to_rgb8(*centroid),
            "CIELUV" | "LUV" => color::luv_to_rgb8(*centroid),
            _ => unreachable!(),
        };

        let hsv = color::rgb8_to_hsv(rgb_u8);
        let lab = color::rgb8_to_lab(rgb_u8);

        clusters.push(ColorCluster {
            rgb: RgbValue {
                r: rgb_u8[0],
                g: rgb_u8[1],
                b: rgb_u8[2],
            },
            hsv,
            lab,
            count,
            share: (count as f64) / (sample_result.sampled_pixels as f64),
        });
    }

    // Sort clusters by count (descending) for consistency
    clusters.sort_by(|a, b| b.count.cmp(&a.count));

    // Map colors to theme element roles
    let role_assignments = map_colors_to_roles(&clusters);
    let scrollbar_enabled = !args.disable_scrollbar;

    // Generate theme file if requested (before moving role_assignments)
    if let Some(theme_path) = &args.theme_output {
        let theme_ron = generate_theme_ron(&role_assignments, scrollbar_enabled);

        // Ensure parent directory exists
        if let Some(parent) = theme_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {}", parent.display()))?;
        }

        std::fs::write(&theme_path, theme_ron)
            .with_context(|| format!("Failed to write theme to {}", theme_path.display()))?;

        eprintln!("Theme written to: {}", theme_path.display());
    }

    let duration_ms = start.elapsed().as_secs_f64() * 1000.0;

    let output = ThemeGenOutput {
        version: APP_VERSION.to_string(),
        clusters,
        role_assignments,
        total_samples: sample_result.sampled_pixels,
        iterations: kmeans_result.iterations,
        duration_ms,
        color_space: args.space.clone(),
        scrollbar_enabled,
    };

    // Serialize to JSON
    let json_output =
        serde_json::to_string_pretty(&output).context("Failed to serialize output to JSON")?;

    // Write to stdout or file
    if let Some(output_path) = args.output {
        std::fs::write(&output_path, json_output)
            .with_context(|| format!("Failed to write output to {}", output_path.display()))?;
    } else {
        println!("{}", json_output);
    }

    Ok(())
}
