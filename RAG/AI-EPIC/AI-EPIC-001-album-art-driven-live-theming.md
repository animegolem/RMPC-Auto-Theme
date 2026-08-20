---
node_id: AI-EPIC-001
tags:
  - EPIC
  - AI
  - theming
  - rmpc
  - color
date_created: 2026-08-19
date_completed:
kanban_status: in-progress
AI_IMP_spawned: true
---

# AI-EPIC-001-album-art-driven-live-theming

## Problem Statement/Feature Scope

rmpc runs with a static theme that ignores the album being played.
The v1 tool solved this but is unusable today: it depends on a
patched rmpc fork, overwrites the user's entire theme from a frozen
template, and carries a stale copy of its color engine. Readability
was patched after the fact, so low-contrast art still produced
themes that were hard to read. The owner wants live, art-driven
theming on stock rmpc 0.11 with readability guaranteed up front.

## Proposed Solution(s)

Build `rmpc-theme-gen`, a short-lived Rust binary that rmpc's
`on_song_change` hook runs on each track change. The binary reads
the current album art from stdin, extracts a palette with the
`color-core` engine (carved from `~/git/color-tool-kmeans` as a
tauri-free crate), assigns colors to a fixed set of theme roles
under the spec's color doctrine (SPEC.org §2: roles own lightness;
art contributes hue and chroma), patches the tokens into the user's
base theme with the `ron` crate, and applies the result to the
running rmpc instance through `rmpc remote set theme -`.

From the user's point of view: change songs and the interface
recolors to match the album within the same second, with text that
always meets WCAG contrast minimums. Same-album tracks don't
retheme or flicker. Theming failures never interrupt playback.

Normative detail lives in SPEC.org rev 0.1; design provenance is
the three-codebase survey recorded in README.md and the v1 archive.

## Path(s) Not Taken

- No rmpc fork or patches; v1's patches addressed repaint bugs, not
  theme application, and must be re-verified against 0.11 before
  anyone assumes they're still needed.
- No daemon; the hook-spawned process model is sufficient and
  simpler. The rmpcd Lua plugin host is deferred until upstream
  declares it stable (SPEC.org §9).
- No layout or tab emission; the four-view UI redesign is static
  user configuration (SPEC.org §8).
- No engine file copies; that's how v1 rotted.

## Success Metrics

- On song change with new art, the running rmpc instance shows the
  new theme in under 1 second on the owner's machine, without
  restart and with queue/selection state intact.
- Every generated theme passes the SPEC §5 guardrail floors,
  machine-verified by tests across a fixture corpus that includes
  low-contrast, monochrome, and neutral album art.
- Same-album track changes produce zero regeneration (hash
  short-circuit observed in the log).
- A theming failure (no art, corrupt art, generator crash) leaves
  playback and the current theme untouched; the hook exits 0.

## Requirements

### Functional Requirements

- [x] FR-1: Extract a dominance-ordered OKLab palette from album art
      bytes on stdin using the `color-core` crate.
- [x] FR-2: Carve `color-core` (color, kmeans, image_pipeline, merge,
      analyze orchestration) out of color-tool-kmeans as a tauri-free
      workspace crate consumed here by path dependency, carrying the
      CC BY 3.0 attribution.
- [ ] FR-3: Assign the eight primary roles under the tone-band
      doctrine: project art hue/chroma into per-role OKLab lightness
      bands; verify with WCAG guardrails; fall back to synthesis only
      when projection cannot satisfy the floors.
- [ ] FR-4: Solve accent and active jointly (pairwise solver with
      strict, relaxed, and best-effort passes) with per-role
      provenance and confidence, surfaced by a `--debug` report.
- [ ] FR-5: Emit a complete rmpc theme document by patching color
      tokens into the user's base theme with the `ron` crate,
      preserving the RON pragmas and emitting 7-byte hex colors only.
- [ ] FR-6: Apply the theme through `rmpc remote --pid $PID set
      theme -`, with in-place file write plus hot reload as the
      documented fallback.
- [ ] FR-7: Ship an `on_song_change` hook that short-circuits on the
      art's SHA-256, honors `rmpc albumart` exit codes, exits 0 on
      every failure path, and logs to a capped file.
- [ ] FR-8: Provide a configuration file for base-theme path,
      quality preset, per-role tone bands, guardrail floors, and the
      OSC 11 terminal-background toggle, with documented defaults.
- [ ] FR-9: Prove determinism: identical art bytes produce an
      identical theme document (golden-file test).
- [ ] FR-10: Roll out the four-view UI configuration (tabs,
      keybinds, base-theme layout) from the personal-theming design
      handoff as static config, corrected for 0.11 semantics.

### Non-Functional Requirements

- Generator wall-clock under 150 ms for a 1200 px cover at the
  default quality preset on the owner's machine.
- Rust gates: `cargo fmt --check`, `cargo test`, `cargo clippy --
  -D warnings` green at every ticket close.
- Public-facing docs follow the Google developer documentation
  style guide.
- No unbounded logs; no state outside the config and cache paths
  named in the spec.

## Implementation Breakdown

Cut 2026-08-19 (owner delegated the breakdown this session):
- [[AI-IMP-001-color-core-extraction]] — engine crate split in
  color-tool-kmeans (FR-1, FR-2).
- [[AI-IMP-002-theme-generator-core]] — roles, solver, tone-band
  projection, RON emission (FR-3, FR-4, FR-5, FR-9).
- [[AI-IMP-003-hook-config-packaging]] — trigger, config file,
  installer, live smoke (FR-6, FR-7, FR-8).
- [[AI-IMP-004-four-view-ui-rollout]] — static UI config, applied
  live 2026-08-19; owner visual verification pending (FR-10).
Order: 004 (mostly done) ∥ 001 → 002 → 003.
