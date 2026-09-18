---
node_id: AI-LOG-2026-09-18-imp-006-publisher-and-v3
tags:
  - AI-log
  - theming
  - publisher
kanban_status: in-progress
date_created: 2026-09-18
closed_tickets: []
related_files:
  - RAG/AI-IMP/AI-IMP-006-ship-v3-default-theme.md
  - fixtures/chromatic-palette-capture.json
confidence_score: 0.95
---

# IMP-006: v3 default, single-flight hook, and palette publisher

## Work Completed

- Carried `imp/006` forward to RMPC main `68bbeec`, including the
  owner-ruled `vendor/color-core` promotion at `92d88c4`. The
  vendored accessor has Astra's reviewed source digest
  `f396bbac37e5ee10db35e6e58668b88755ea88caa7b4232afbcb5c14f4d4d8a3`.
- Shipped the genericized v3 base and companion four-tab sample,
  complete mapped chrome, deliberate theme golden, and installer
  messaging. The hook now gates extraction with single-flight and
  emits attributed, bounded stderr while preserving per-event
  publication and latest-event ordering.
- The publisher consumes the mapped candidate from vendored
  `color-core`, checks paired RGB bytes, and serializes extraction
  OKLCH via f64 chart change with C < 1e-4 setting H to zero. Its
  `producer.spec_revision` is 0.13. The full synthetic-art `ok`
  capture is `fixtures/chromatic-palette-capture.json` with SHA-256
  `4cbdfbcede60f99b215affc81c88c93f97e3158c34721bf7b4a61174aa2efe7b`.
- A real current-track MPD capture into scratch returned `ok`
  (`/private/tmp/rmpc-imp006-live.wjEVOw/palette.json`): six
  clusters, nine roles, seven passing verified pairs, share sum 1.0.
  This did not install a hook or apply a live theme.

## Session Commits

- The single `imp/006` ticket commit carries this log; its SHA is
  reported in the review submission.

## Issues Encountered

- Ruling r12 superseded the original sibling path dependency. The
  parent color-tool-kmeans app is out of bounds; no canonical edit
  or commit was made by this sitting. A recoverable pre-vendor stash
  remains in the isolated clone under the destructive-op fence.
- Astra's f32 OKLab→OKLCH replay vectors can shift a paired RGB byte.
  Ruling r13 sets coordinate fidelity, not extraction-to-hex byte
  equality, as the publisher invariant. Those vectors are retained
  as tests; no color math was changed to conceal the difference.
- An initial assertion that the chromatic swatch fixture must show
  a large inverse-hex gap failed: its highly chromatic clusters are
  nearly exact original RGB swatches. The assertion was removed and
  the fixture's evidence limit documented. A genuine live capture
  and a fixture capture are separate evidence classes.

## Tests Added

- Publisher JSON round-trip reproduces mapped OKLab within
  1e-6/component; achromatic hue serialization and Astra's two
  byte-boundary vectors are covered. Mock-MPD artwork acquisition
  exercises a full `ok` artifact and checks each emitted cluster
  against the mapped candidate.
- Branch-tip gate: `cargo build`, `cargo fmt --check`,
  `cargo clippy -- -D warnings`, `cargo test` — 57 passed / 0 failed.
  Vendored color-core: 27 passed / 0 failed. Explicit ticket
  validation: 1 checked, 0 errors, 0 warnings.

## Next Steps

1. Review the one-commit `imp/006` submission and the committed
   publisher capture against BRIEF-004 r12/r13 and contract A5-A7
   pending fold.
2. Keep owner macOS live theme/IPC/listening acceptance separate;
   this sitting only made scratch captures and automated checks.
