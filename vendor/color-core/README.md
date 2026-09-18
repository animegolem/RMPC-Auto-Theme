# color-core

`color-core` is the color analysis engine behind the color-tool desktop app,
packaged as a plain Rust crate with no Tauri dependency. It extracts a
dominance-ordered color palette from an image by sampling pixels, clustering
them with k-means++ in OKLab, merging near-duplicate clusters, and gamut
mapping the results back to sRGB.

## Modules

- `analyze` — full pipeline orchestration and the serde request/response
  types that define the palette contract.
- `image_pipeline` — image decode, downscale, and alpha-aware pixel
  sampling with quality presets.
- `kmeans` — deterministic k-means++ with SIMD, warm-start, and mini-batch
  support.
- `merge` — radius-guarded agglomerative cluster merging.
- `color` — conversions between sRGB, OKLab, OKLch, HSV, and friends, plus
  chroma-compressing gamut mapping.

## Analyze an image

Call `analyze` with a request and an image source. You can pass a file path
or an in-memory encoded image (the format is guessed from the magic bytes):

```rust
use color_core::analyze::{analyze, AnalyzeRequest, ImageSource};

let req: AnalyzeRequest = serde_json::from_value(serde_json::json!({
    "k": 8,
    "quality": 2,
    "seed": 1,
}))?;

// From a file on disk:
let response = analyze(&req, ImageSource::Path("photo.jpg".as_ref()))?;

// From bytes you already hold in memory:
let bytes = std::fs::read("photo.jpg")?;
let response = analyze(&req, ImageSource::Bytes(&bytes))?;

for cluster in &response.clusters {
    println!("{:?} share={:.3}", cluster.rgb, cluster.share);
}
```

Results are deterministic for a given image, parameters, and seed.

## Features

- `simd` (default) — SIMD k-means kernels via the `wide` crate. Disable
  with `default-features = false` to build the scalar path.

## Tests

Run the crate's test suite from the workspace root:

```sh
cargo test -p color-core
```

The integration tests include golden fixtures for the color conversions and
snapshot tests for k-means determinism; they read reference images from the
repository's `test-patterns/` directory.

## License and attribution

MIT, like the rest of the repository. The color space conversions are based
on Color-tool by Laurent Jégou (CC BY 3.0); see
[ATTRIBUTIONS.md](ATTRIBUTIONS.md), which must travel with any distribution
that embeds this crate.
