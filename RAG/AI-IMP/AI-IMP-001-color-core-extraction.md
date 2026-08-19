---
node_id: AI-IMP-001
tags:
  - IMP-LIST
  - Implementation
  - color
  - upstream
kanban_status: planned
depends_on: []
parent_epic: [[AI-EPIC-001-album-art-driven-live-theming]]
confidence_score: 0.8
date_created: 2026-08-19
date_completed:
---

# AI-IMP-001-color-core-extraction

## Summary of Issue #1

color-tool-kmeans locks its engine inside the Tauri binary crate:
`merge.rs` and the `analyze_image` orchestration are `mod`s of
`main.rs`, and `lib.rs` drags in the full Tauri stack. v2 needs the
engine as a dependency, not a copy (SPEC §4). Done state: a
`color-core` workspace member in ~/git/color-tool-kmeans with no
tauri dependency, exporting `analyze(bytes/params) -> AnalyzeResponse`
plus the color/kmeans/pipeline/merge modules, all existing Rust
tests passing in both crates.

### Out of Scope 

No behavior changes to the engine; no new features; no changes to
the Tauri app's IPC surface beyond re-exporting from the new crate.
No publication to crates.io.

### Design/Approach  

Work happens in ~/git/color-tool-kmeans (owner's repo, separate
sitting). Create workspace member `color-core`; move color.rs,
kmeans.rs, image_pipeline.rs, merge.rs and lift `analyze_image`'s
body into a plain function with a bytes-input variant (v2 feeds
stdin bytes, not a path). Tauri app re-imports from color-core.
Carry ATTRIBUTIONS (CC BY 3.0, Laurent Jégou) into the crate. Keep
the existing snapshot/golden/roundtrip tests green; they are the
regression net for the move.

### Files to Touch

- ~/git/color-tool-kmeans: Cargo.toml (workspace), new
  color-core/ crate, tauri-app/src-tauri (imports), ATTRIBUTIONS.
- This repo: none (consumes by path dep in AI-IMP-002).

### Implementation Checklist

<CRITICAL_RULE>
Before marking an item complete on the checklist MUST **stop** and **think**. Have you validated all aspects are **implemented** and **tested**? 
</CRITICAL_RULE>

- [ ] Workspace member `color-core` builds with no tauri deps
      (`cargo tree` shows none).
- [ ] `analyze` accepts in-memory bytes as well as a path.
- [ ] All existing Rust tests pass in both crates; CI config updated.
- [ ] Attribution files travel with the crate.
- [ ] Upstream repo gates green: fmt, clippy -D warnings, tests.

### Acceptance Criteria

GIVEN the color-tool-kmeans workspace after the split, WHEN
`cargo test --workspace` runs, THEN all pre-split tests pass and
`color-core` has no tauri dependency in its tree.
GIVEN a JPEG as bytes, WHEN `color_core::analyze` runs with seed 1
twice, THEN both palettes are identical.

### Issues Encountered 

(to be filled during implementation)
