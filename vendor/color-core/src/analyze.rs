//! Full analysis pipeline: sample an image, cluster in OKLab, merge, convert.
//!
//! This is the orchestration that the Tauri app's `analyze_image` command
//! exposes over IPC, lifted into a plain function so library consumers can
//! call it directly. The request/response types keep their serde shapes; the
//! Tauri command serializes them unchanged, so the IPC contract is defined
//! here.

use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::color;
use crate::image_pipeline::{
    prepare_samples, prepare_samples_from_bytes, quality_preset, SampleParams, SamplingError,
};
use crate::kmeans::{run_kmeans, KMeansConfig};
use crate::merge::{merge_clusters_by_threshold, RawCluster};

pub fn default_tol() -> f32 {
    1e-3
}
pub fn default_max_iters() -> u32 {
    40
}
pub fn default_seed() -> u64 {
    1
}
pub fn default_max_samples() -> usize {
    300_000
}

/// Analysis parameters. Serde attributes match the Tauri IPC contract.
///
/// `path` is only consulted by callers that analyze by path (the Tauri
/// command); [`analyze`] itself reads the image from its [`ImageSource`]
/// argument.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzeRequest {
    #[serde(default)]
    pub path: String,
    #[serde(default, alias = "K", alias = "k", alias = "clusters")]
    pub k: usize,
    #[serde(default)]
    pub stride: u32,
    #[serde(default)]
    pub quality: Option<i8>,
    #[serde(default, alias = "ignoreTopN", alias = "ignore_top_n")]
    pub ignore_top_n: usize,
    #[serde(default, alias = "mergeThreshold", alias = "merge_threshold")]
    pub merge_threshold: f32,
    #[serde(default, alias = "min_lum")]
    pub min_lum: u8,
    #[serde(default = "default_tol")]
    pub tol: f32,
    #[serde(default = "default_max_iters", alias = "max_iters")]
    pub max_iter: u32,
    #[serde(default = "default_seed")]
    pub seed: u64,
    #[serde(default = "default_max_samples")]
    pub max_samples: usize,
    #[serde(default, alias = "snap_to_real")]
    pub snap_to_real: bool,
}

#[derive(Debug, Serialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct RgbValue {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClusterOut {
    pub count: usize,
    pub share: f64,
    pub centroid_space: [f32; 3],
    pub oklab: [f32; 3],
    pub oklch: [f32; 3],
    pub rgb: RgbValue,
    pub hsv: [f32; 3],
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzeResponse {
    pub clusters: Vec<ClusterOut>,
    pub iterations: usize,
    pub duration_ms: f64,
    pub total_samples: usize,
    pub variant: String,
}

/// Where [`analyze`] reads the image from.
pub enum ImageSource<'a> {
    /// Decode the image from a file on disk.
    Path(&'a Path),
    /// Decode the image from an in-memory encoded buffer (format guessed
    /// from the magic bytes).
    Bytes(&'a [u8]),
}

/// Analysis failure. `Display` output is stable: the Tauri command maps
/// these errors to IPC error strings with `to_string()`.
#[derive(Debug, Error)]
pub enum AnalyzeError {
    #[error("Sampling failed: {0}")]
    Sampling(#[from] SamplingError),
    #[error("No pixels met sampling criteria (check stride/minLum)")]
    NoSamples,
}

fn snap_centroids_to_nearest(raw_clusters: &mut [RawCluster], samples_oklab: &[[f32; 3]]) {
    for cluster in raw_clusters.iter_mut() {
        let mut best_dist = f32::MAX;
        let mut best = cluster.centroid;
        for sample in samples_oklab {
            let dx = cluster.centroid[0] - sample[0];
            let dy = cluster.centroid[1] - sample[1];
            let dz = cluster.centroid[2] - sample[2];
            let d = dx * dx + dy * dy + dz * dz;
            if d < best_dist {
                best_dist = d;
                best = *sample;
            }
        }
        cluster.centroid = best;
    }
}

/// Runs the full analysis pipeline on an image and returns the palette,
/// dominance-ordered. Deterministic for a given image, parameters, and seed.
pub fn analyze(
    req: &AnalyzeRequest,
    source: ImageSource<'_>,
) -> Result<AnalyzeResponse, AnalyzeError> {
    let k = if req.k == 0 { 16 } else { req.k };

    // 1) Sampling
    let (stride, max_samples, max_dimension) = if let Some(quality) = req.quality {
        let preset = quality_preset(quality);
        (preset.stride, preset.max_samples, preset.max_dimension)
    } else {
        (req.stride.max(1), req.max_samples.max(1), Some(3200))
    };
    let sample_params = SampleParams {
        path: match &source {
            ImageSource::Path(path) => path.to_path_buf(),
            ImageSource::Bytes(_) => PathBuf::new(),
        },
        stride,
        min_lum: req.min_lum,
        max_samples,
        max_dimension,
        seed: req.seed,
    };
    let samples = match source {
        ImageSource::Path(_) => prepare_samples(&sample_params)?,
        ImageSource::Bytes(bytes) => prepare_samples_from_bytes(bytes, &sample_params)?,
    };
    if samples.sampled_pixels == 0 {
        return Err(AnalyzeError::NoSamples);
    }

    // 2) Build working dataset in OKLab (perceptual)
    let dataset: Vec<[f32; 3]> = if let Some(lab) = &samples.samples_oklab {
        lab.clone()
    } else {
        samples
            .samples
            .iter()
            .map(|&rgb| color::rgb8_to_oklab(rgb))
            .collect()
    };

    let effective_k = k.min(dataset.len().max(1));

    // 3) Run k-means
    let cfg = KMeansConfig {
        k: effective_k,
        max_iters: req.max_iter as usize,
        tol: req.tol,
        seed: req.seed,
        warm_start: None,
        mini_batch: None,
    };
    let start = Instant::now();
    let result = run_kmeans(&dataset, &cfg);
    let duration_ms = start.elapsed().as_secs_f64() * 1000.0;

    // 4) Build clusters; apply merge threshold; convert centroid to RGB and HSV
    let mut raw_clusters: Vec<RawCluster> = result
        .centroids
        .iter()
        .zip(result.counts.iter())
        .filter_map(|(centroid, &count)| {
            if count == 0 {
                return None;
            }
            Some(RawCluster {
                centroid: *centroid,
                count,
            })
        })
        .collect();
    if req.snap_to_real {
        let oklab = samples.samples_oklab.as_ref().unwrap();
        snap_centroids_to_nearest(&mut raw_clusters, oklab);
    }
    let merge_threshold = req.merge_threshold.clamp(0.0, 0.2);
    if merge_threshold > 0.0 {
        let before_count = raw_clusters.len();
        raw_clusters = merge_clusters_by_threshold(raw_clusters, merge_threshold);
        let after_count = raw_clusters.len();
        eprintln!(
            "[analyze_image] merge_threshold={:.3} clusters={} -> {}",
            merge_threshold, before_count, after_count
        );
    } else {
        raw_clusters.sort_by_key(|c| std::cmp::Reverse(c.count));
    }

    let mut clusters: Vec<ClusterOut> = raw_clusters
        .into_iter()
        .map(|cluster| {
            let rgb_u8 = color::oklab_to_srgb8_gamut_mapped(cluster.centroid);
            let rgb = RgbValue {
                r: rgb_u8[0],
                g: rgb_u8[1],
                b: rgb_u8[2],
            };
            let oklab = cluster.centroid;
            let oklch = color::oklab_to_oklch(cluster.centroid);
            let hsv = color::rgb8_to_hsv(rgb_u8);
            ClusterOut {
                count: cluster.count,
                share: (cluster.count as f64) / (samples.sampled_pixels as f64),
                centroid_space: cluster.centroid,
                oklab,
                oklch,
                rgb,
                hsv,
            }
        })
        .collect();
    clusters.sort_by_key(|c| std::cmp::Reverse(c.count));
    let ignore_top_n = req.ignore_top_n.min(clusters.len().saturating_sub(1));
    if ignore_top_n > 0 {
        clusters = clusters.into_iter().skip(ignore_top_n).collect();
    }

    let variant = if merge_threshold > 0.0 {
        "inhouse+merge"
    } else {
        "inhouse"
    };

    Ok(AnalyzeResponse {
        clusters,
        iterations: result.iterations,
        duration_ms,
        total_samples: samples.sampled_pixels,
        variant: variant.into(),
    })
}
