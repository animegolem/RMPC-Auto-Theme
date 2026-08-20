//! rmpc-theme-gen: album art on stdin, complete rmpc theme on stdout.
//! Pipeline per SPEC.org §3: extract (color-core) → assign → emit.
//! This is the IMP-002 seam proof: stdin bytes through color_core::analyze.

use std::io::Read;

use color_core::analyze::{default_max_samples, AnalyzeRequest, ImageSource};

fn main() -> Result<(), String> {
    let mut bytes = Vec::new();
    std::io::stdin()
        .read_to_end(&mut bytes)
        .map_err(|e| format!("read stdin: {e}"))?;

    let req = AnalyzeRequest {
        path: String::new(),
        k: 8,
        stride: 4,
        quality: Some(2),
        ignore_top_n: 0,
        merge_threshold: 0.08,
        min_lum: 0,
        tol: 1e-3,
        max_iter: 40,
        seed: 1,
        max_samples: default_max_samples(),
        snap_to_real: false,
    };
    let analysis =
        color_core::analyze(&req, ImageSource::Bytes(&bytes)).map_err(|e| e.to_string())?;

    for c in &analysis.clusters {
        eprintln!(
            "share {:>5.1}%  L {:>5.3}  C {:>5.3}  h {:>6.1}  #{:02x}{:02x}{:02x}",
            c.share * 100.0,
            c.oklch[0],
            c.oklch[1],
            c.oklch[2],
            c.rgb.r,
            c.rgb.g,
            c.rgb.b
        );
    }
    Ok(())
}
