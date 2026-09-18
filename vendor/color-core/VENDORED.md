# Vendored: color-core — deliberate fork with provenance

Owner-ruled 2026-09-18 ("let's properly pull the color math into both
repos"): this repository owns its color engine outright. The
path-dependency reach into `~/git/color-tool-kmeans` (rev 0.1 wiring)
is retired; the parent app is no longer a build dependency of this
project and is never modified by this project's work.

## Provenance

- Base: color-tool-kmeans commit `5baa20e021855fbc57aebf48fa0f9b3374ded281`.
- One divergence from base: the additive mapped-coordinate seam in
  `src/color.rs` — `oklab_to_srgb8_gamut_mapped_with_oklab` exposes
  the post-gamut-mapping, pre-quantization OKLab candidate; the legacy
  byte function delegates to it (single source by construction).
  File SHA256 `f396bbac37e5ee10db35e6e58668b88755ea88caa7b4232afbcb5c14f4d4d8a3`.
- Evidence for "math unaltered": two independent exhaustive
  16,777,216-triple before/after sweeps (Sol; Astra evaluating both
  source versions per input), zero changed bytes on all three routes,
  plus 500k continuous/centroid probes; crate tests 27/0.
- Reviews: independent — grisaille
  `.inbox/tofable/mpd-estate-color-core-review-r2.md` (Astra, no
  blocking findings, evidence retained beside it); final — notan (RL),
  gates reproduced.
- Authorization trail: RMPC carrier brief-004 r11 (event
  3be310afcc562bc4) + RAG/DESIGN-QUEUE.md entries of 2026-09-18.

## Rules of the fork

- The v1 anti-rot law survives in new form: NO ad-hoc copying of
  engine files into `src/` — the vendored crate is the one copy, and
  any future re-sync with the parent project is a CONSCIOUS re-vendor
  recorded in this file, never silent drift.
- `src/color.rs` carries CC BY 3.0 attribution (Laurent Jégou,
  Color-tool; see ATTRIBUTIONS.md) — the obligation travels with the
  crate wherever it goes.
- `vendor/test-patterns/` is the crate's test-fixture sibling
  (`CARGO_MANIFEST_DIR/../test-patterns`); it moves with the crate.
