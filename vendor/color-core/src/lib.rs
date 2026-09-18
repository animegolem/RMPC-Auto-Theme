//! Tauri-free color analysis engine.
//!
//! Extracted from the color-tool Tauri app so external projects can consume
//! the engine as a library. The pipeline: sample pixels from an image
//! ([`image_pipeline`]), cluster them with k-means++ in OKLab ([`kmeans`]),
//! merge near-duplicate clusters ([`merge`]), and convert centroids across
//! color spaces ([`color`]). [`analyze`] orchestrates the full pipeline.
//!
//! Color space algorithms in [`color`] are based in part on Color-tool by
//! L. Jégou (CC BY 3.0); see ATTRIBUTIONS.md in this crate.

pub mod analyze;
pub mod color;
pub mod image_pipeline;
pub mod kmeans;
pub mod merge;

pub use analyze::{
    analyze, AnalyzeError, AnalyzeRequest, AnalyzeResponse, ClusterOut, ImageSource, RgbValue,
};
