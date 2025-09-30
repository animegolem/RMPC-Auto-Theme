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
    source_cluster_index: usize,
    confidence: f32,
}

#[derive(Parser, Debug)]
#[command(name = "rmpc-theme-gen")]
#[command(about = "Generate rmpc theme from album art", long_about = None)]
struct Args {
    /// Path to album art image
    #[arg(short, long)]
    image: PathBuf,

    /// Number of color clusters to extract
    #[arg(short, long, default_value = "8")]
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

/// Select accent color: high saturation with good contrast
fn select_accent_color(
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
        let contrast = color::calculate_contrast_ratio(bg_lab, cluster.lab);

        // Score: favor high saturation and good contrast
        let score = s * 2.0 + (contrast / 21.0) * 3.0;

        if score > best_score && contrast > 3.0 {
            best_score = score;
            best_idx = idx;
        }
    }

    let confidence = if best_score > 2.0 { 0.85 } else { 0.6 };
    (best_idx, confidence)
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

/// Select active item color: bright and saturated
fn select_active_item_color(
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
        let v = cluster.hsv[2];
        let contrast = color::calculate_contrast_ratio(bg_lab, cluster.lab);

        // Prefer bright, saturated colors with good contrast
        let score = v + s + (contrast / 21.0);

        if score > best_score && v > 0.5 && s > 0.3 {
            best_score = score;
            best_idx = idx;
        }
    }

    let confidence = if best_score > 1.5 { 0.85 } else { 0.6 };
    (best_idx, confidence)
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
            "    scrollbar: (\n        symbols: [\"│\", \"█\", \"▲\", \"▼\"],\n        track_style: (fg: \"{}\"),\n        ends_style: (fg: \"{}\"),\n        thumb_style: (fg: \"{}\"),\n    ),\n",
            bg.hex, bg.hex, accent.hex
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
    preview_label_style: (fg: "{}"),
    preview_metadata_group_style: (fg: "{}", modifiers: "Bold"),
    tab_bar: (
        enabled: true,
        active_style: (fg: "{}", bg: "{}", modifiers: "Bold"),
        inactive_style: (fg: "{}", bg: "{}"),
    ),
    highlighted_item_style: (fg: "{}", modifiers: "Bold"),
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
        track_style: (fg: "{}"),
        elapsed_style: (fg: "{}"),
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
            prop: (kind: Property(Album), style: (fg: "{}"),
                default: (kind: Text("Unknown Album"), style: (fg: "{}"))
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
        accent.hex,      // preview_label_style
        accent.hex,      // preview_metadata_group_style
        // Tab bar
        text.hex,        // tab_bar.active_style.fg
        active.hex,      // tab_bar.active_style.bg
        inactive.hex,    // tab_bar.inactive_style.fg
        bg.hex,          // tab_bar.inactive_style.bg
        // Item styles
        accent.hex,      // highlighted_item_style.fg
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
        inactive.hex,    // progress_bar.track_style.fg
        active.hex,      // progress_bar.elapsed_style.fg
        active.hex,      // progress_bar.thumb_style.fg
        bg.hex,          // progress_bar.thumb_style.bg
        // Song table format
        text.hex,        // album style fg
        text.hex,        // album default style fg
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

    // 1. Select background (most dominant, reasonable properties)
    let (bg_idx, bg_conf) = select_background(clusters);
    let bg_cluster = &clusters[bg_idx];
    let bg_lab = bg_cluster.lab;
    used_indices.push(bg_idx);

    assignments.push(RoleAssignment {
        role: ColorRole::Background,
        rgb: bg_cluster.rgb,
        hsv: bg_cluster.hsv,
        lab: bg_cluster.lab,
        hex: color::rgb_to_hex([bg_cluster.rgb.r, bg_cluster.rgb.g, bg_cluster.rgb.b]),
        source_cluster_index: bg_idx,
        confidence: bg_conf,
    });

    // 2. Select text color (highest contrast)
    let (text_idx, mut text_conf) = select_text_color(clusters, bg_lab);
    let text_cluster = &clusters[text_idx];

    // Fallback: if contrast is too low, generate synthetic text color
    let (text_rgb, text_hsv, text_lab, text_hex) =
        if color::calculate_contrast_ratio(bg_lab, text_cluster.lab) < 4.5 {
            text_conf = 0.4;
            let (rgb, hsv, lab) = if bg_cluster.lab[0] < 50.0 {
                generate_light_text()
            } else {
                generate_dark_text()
            };
            (
                RgbValue {
                    r: rgb[0],
                    g: rgb[1],
                    b: rgb[2],
                },
                hsv,
                lab,
                color::rgb_to_hex(rgb),
            )
        } else {
            used_indices.push(text_idx);
            (
                text_cluster.rgb,
                text_cluster.hsv,
                text_cluster.lab,
                color::rgb_to_hex([text_cluster.rgb.r, text_cluster.rgb.g, text_cluster.rgb.b]),
            )
        };

    assignments.push(RoleAssignment {
        role: ColorRole::Text,
        rgb: text_rgb,
        hsv: text_hsv,
        lab: text_lab,
        hex: text_hex,
        source_cluster_index: text_idx,
        confidence: text_conf,
    });

    // 3. Select accent color (high saturation, good contrast)
    let (accent_idx, accent_conf) = select_accent_color(clusters, bg_lab, &used_indices);
    let accent_cluster = &clusters[accent_idx];
    used_indices.push(accent_idx);

    assignments.push(RoleAssignment {
        role: ColorRole::Accent,
        rgb: accent_cluster.rgb,
        hsv: accent_cluster.hsv,
        lab: accent_cluster.lab,
        hex: color::rgb_to_hex([
            accent_cluster.rgb.r,
            accent_cluster.rgb.g,
            accent_cluster.rgb.b,
        ]),
        source_cluster_index: accent_idx,
        confidence: accent_conf,
    });

    // 4. Select border color (distinct from background)
    let (border_idx, border_conf) = select_border_color(clusters, bg_lab, &used_indices);
    let border_cluster = &clusters[border_idx];
    used_indices.push(border_idx);

    assignments.push(RoleAssignment {
        role: ColorRole::Border,
        rgb: border_cluster.rgb,
        hsv: border_cluster.hsv,
        lab: border_cluster.lab,
        hex: color::rgb_to_hex([
            border_cluster.rgb.r,
            border_cluster.rgb.g,
            border_cluster.rgb.b,
        ]),
        source_cluster_index: border_idx,
        confidence: border_conf,
    });

    // 5. Select active item color (bright, saturated)
    let (active_idx, active_conf) = select_active_item_color(clusters, bg_lab, &used_indices);
    let active_cluster = &clusters[active_idx];
    used_indices.push(active_idx);

    assignments.push(RoleAssignment {
        role: ColorRole::ActiveItem,
        rgb: active_cluster.rgb,
        hsv: active_cluster.hsv,
        lab: active_cluster.lab,
        hex: color::rgb_to_hex([
            active_cluster.rgb.r,
            active_cluster.rgb.g,
            active_cluster.rgb.b,
        ]),
        source_cluster_index: active_idx,
        confidence: active_conf,
    });

    // 6. Inactive/muted - reuse border or background
    let inactive_cluster = &clusters[border_idx];
    assignments.push(RoleAssignment {
        role: ColorRole::InactiveItem,
        rgb: inactive_cluster.rgb,
        hsv: inactive_cluster.hsv,
        lab: inactive_cluster.lab,
        hex: color::rgb_to_hex([
            inactive_cluster.rgb.r,
            inactive_cluster.rgb.g,
            inactive_cluster.rgb.b,
        ]),
        source_cluster_index: border_idx,
        confidence: 0.7,
    });

    // 7. Progress bar - reuse accent
    assignments.push(RoleAssignment {
        role: ColorRole::ProgressBar,
        rgb: accent_cluster.rgb,
        hsv: accent_cluster.hsv,
        lab: accent_cluster.lab,
        hex: color::rgb_to_hex([
            accent_cluster.rgb.r,
            accent_cluster.rgb.g,
            accent_cluster.rgb.b,
        ]),
        source_cluster_index: accent_idx,
        confidence: accent_conf,
    });

    // 8. Scrollbar - reuse accent or active
    assignments.push(RoleAssignment {
        role: ColorRole::Scrollbar,
        rgb: active_cluster.rgb,
        hsv: active_cluster.hsv,
        lab: active_cluster.lab,
        hex: color::rgb_to_hex([
            active_cluster.rgb.r,
            active_cluster.rgb.g,
            active_cluster.rgb.b,
        ]),
        source_cluster_index: active_idx,
        confidence: active_conf,
    });

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
