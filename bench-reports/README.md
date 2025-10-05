# Benchmark Reports

Generated performance reports live here. Outputs are gitignored except for this README.

## Workflow
- Place benchmark source images under `bench-assets/` (gitignored).
- Run the harness via `npm run bench:compare` from the repository root.
  - `-- --variant crate` evaluates the optional `kmeans_colors` backend (requires the `bench-crate` feature).
  - `-- --delta de2000` switches the ΔE metric to CIEDE2000 (default `de76`).
  - `-- --weighted[=alpha]` applies share-aware weighted matching for Hungarian assignment (default α = 1.0).
  - `-- --explain` writes per-image diagnostics under `bench-reports/explain/` (confusion matrices, Jaccard, nearest neighbors).
  - `-- --tails-gate` enforces the tail fidelity gate (≤6 clusters per image with ΔE > 20) and fails the run if exceeded.
- Inspect generated JSON/Markdown summaries inside `bench-reports/`.
  - Table columns now include P95 ΔE, count of clusters with ΔE > 20, and the matching strategy label.
  - `comparison.json` also exposes ΔE quantiles, counts above 10/15/20, and share-weighted mean ΔE.
  - `parity.json` appears when `--parity` is used and summarizes LAB conversion parity (per image + aggregate stats).
- Run `node scripts/bench/wasm-bench.mjs` to compare the wasm build against the native results on the latest sample set.
  - Outputs console timings and writes `wasm-bench.json` under `bench-reports/`.
  - The script assumes `compute-wasm/pkg-node` exists (run `node scripts/wasm/build.mjs --node` beforehand).

Nothing in this directory besides this file should be committed.
