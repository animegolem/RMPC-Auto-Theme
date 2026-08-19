---
node_id: AI-IMP-002
tags:
  - IMP-LIST
  - Implementation
  - rust
  - solver
  - emission
kanban_status: planned
depends_on: ["AI-IMP-001"]
parent_epic: [[AI-EPIC-001-album-art-driven-live-theming]]
confidence_score: 0.7
date_created: 2026-08-19
date_completed:
---

# AI-IMP-002-theme-generator-core

## Summary of Issue #1

There is no v2 binary. Done state: `rmpc-theme-gen` reads art
bytes on stdin, extracts via color-core, assigns the eight SPEC §5
roles under the §2 tone-band doctrine, patches tokens into the base
theme with the `ron` crate, and writes a complete theme document to
stdout deterministically, with `--debug` provenance output; fixture
corpus (incl. low-contrast/mono/neutral art) passes guardrail
verification tests.

### Out of Scope 

No hook script, no config-file loading (flags only; config file is
AI-IMP-003), no OSC 11, no application to rmpc (stdout only).

### Design/Approach  

New Rust crate in this repo. Port from .archive/src/rmpc_theme_gen.rs
ONLY: guardrail consts, WCAG contrast, ΔE, hue-separation w/ chroma
gate, pairwise solver shape, synthesize_color_between, provenance
structs — restructured into modules (roles.rs, solver.rs, emit.rs).
NEW: tone-band projection (L replaced per role, hue held, chroma
gamut-compressed via color-core) ahead of the solver; neutral-ramp
fallback. Emission: parse base theme RON → patch ~9 tokens → emit
complete document with pragmas, 7-byte hex only (SPEC §6). Golden
tests: fixture art → byte-identical theme.

### Files to Touch

- Cargo.toml (workspace root here), src/main.rs, src/roles.rs,
  src/solver.rs, src/emit.rs, tests/, fixtures/.

### Implementation Checklist

<CRITICAL_RULE>
Before marking an item complete on the checklist MUST **stop** and **think**. Have you validated all aspects are **implemented** and **tested**? 
</CRITICAL_RULE>

- [ ] color-core consumed by path dependency; no engine code copied.
- [ ] Tone-band projection implemented; property test: every role's
      emitted L within its configured band.
- [ ] Guardrail verification tests across fixture corpus (incl. a
      monochrome and a near-black cover) — all floors met or
      documented synthetic fallback with provenance.
- [ ] ron-crate base-theme patching emits a COMPLETE document; test:
      patching the default theme changes only color tokens (AST diff).
- [ ] Determinism golden test (identical bytes → identical output).
- [ ] --debug prints per-role origin/confidence/contrast matrix.
- [ ] Gates green: fmt, clippy -D warnings, test.

### Acceptance Criteria

GIVEN a base theme with a custom layout, WHEN the generator patches
it, THEN the emitted document differs only in color tokens.
GIVEN low-chroma album art, WHEN roles are assigned, THEN neutrals
come from the ramp and every pair still meets its floor.

### Issues Encountered 

(to be filled during implementation)
