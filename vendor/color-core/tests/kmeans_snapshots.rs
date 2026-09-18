use color_core::image_pipeline::{prepare_samples, SampleParams};
use color_core::kmeans::{run_kmeans, KMeansConfig};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const SNAPSHOT_DIR: &str = "tests/snapshots";
const ROUND_DIGITS: f32 = 1_000_000.0;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct SnapshotCluster {
    centroid: [f32; 3],
    count: usize,
    share: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct SnapshotFile {
    name: String,
    image: String,
    params: SnapshotParams,
    clusters: Vec<SnapshotCluster>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct SnapshotParams {
    k: usize,
    seed: u64,
    stride: u32,
    max_samples: usize,
    max_dimension: Option<u32>,
}

#[derive(Debug, Clone)]
struct SnapshotCase<'a> {
    name: &'a str,
    image: &'a str,
    k: usize,
    seed: u64,
    stride: u32,
    max_samples: usize,
    max_dimension: Option<u32>,
}

#[test]
fn kmeans_snapshots_match() {
    let force = std::env::var("SNAPSHOT_FORCE").is_ok();
    if !force && !cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        eprintln!("kmeans snapshots skipped: non-linux target");
        return;
    }
    if cfg!(feature = "simd") {
        eprintln!("kmeans snapshots skipped: simd enabled (use --no-default-features)");
        return;
    }

    let cases = [SnapshotCase {
        name: "hsl_lightness",
        image: "hsl_ligthness.png",
        k: 16,
        seed: 1,
        stride: 2,
        max_samples: 120_000,
        max_dimension: None,
    }];

    for case in cases {
        let snapshot = build_snapshot(&case);
        let snapshot_path = snapshot_path(case.name);
        if std::env::var("UPDATE_SNAPSHOTS").is_ok() {
            write_snapshot(&snapshot_path, &snapshot);
        } else {
            let expected = read_snapshot(&snapshot_path);
            assert_eq!(expected, snapshot, "snapshot mismatch: {}", case.name);
        }
    }
}

fn build_snapshot(case: &SnapshotCase<'_>) -> SnapshotFile {
    let image_path = test_patterns_dir().join(case.image);
    let params = SampleParams {
        path: image_path.clone(),
        stride: case.stride,
        min_lum: 0,
        max_samples: case.max_samples,
        max_dimension: case.max_dimension,
        seed: case.seed,
    };
    let samples = prepare_samples(&params).expect("prepare samples");
    let dataset = samples.samples_oklab.expect("oklab samples missing");
    let config = KMeansConfig {
        k: case.k.min(dataset.len().max(1)),
        max_iters: 40,
        tol: 1e-3,
        seed: case.seed,
        warm_start: None,
        mini_batch: None,
    };

    let result = run_kmeans(&dataset, &config);
    let total_samples = samples.sampled_pixels.max(1) as f32;
    let mut clusters: Vec<SnapshotCluster> = result
        .centroids
        .iter()
        .zip(result.counts.iter())
        .filter_map(|(centroid, &count)| {
            if count == 0 {
                return None;
            }
            let share = count as f32 / total_samples;
            Some(SnapshotCluster {
                centroid: normalize_triplet(*centroid),
                count,
                share: normalize(share),
            })
        })
        .collect();
    clusters.sort_by(|a, b| b.count.cmp(&a.count));

    SnapshotFile {
        name: case.name.to_string(),
        image: case.image.to_string(),
        params: SnapshotParams {
            k: config.k,
            seed: case.seed,
            stride: case.stride,
            max_samples: case.max_samples,
            max_dimension: case.max_dimension,
        },
        clusters,
    }
}

fn snapshot_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(SNAPSHOT_DIR)
        .join(format!("{name}.json"))
}

fn test_patterns_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-patterns")
}

fn write_snapshot(path: &Path, snapshot: &SnapshotFile) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create snapshot dir");
    }
    let data = serde_json::to_string_pretty(snapshot).expect("serialize snapshot");
    std::fs::write(path, data).expect("write snapshot");
}

fn read_snapshot(path: &Path) -> SnapshotFile {
    let data = std::fs::read_to_string(path)
        .unwrap_or_else(|_| panic!("snapshot missing: {}", path.display()));
    serde_json::from_str(&data).expect("parse snapshot")
}

fn normalize(value: f32) -> f32 {
    (value * ROUND_DIGITS).round() / ROUND_DIGITS
}

fn normalize_triplet(mut value: [f32; 3]) -> [f32; 3] {
    for channel in &mut value {
        *channel = normalize(*channel);
    }
    value
}
