---
node_id: AI-IMP-002
tags:
  - IMP-LIST
  - Implementation
  - rust
  - solver
  - emission
kanban_status: completed
depends_on: ["AI-IMP-001"]
parent_epic: [[AI-EPIC-001-album-art-driven-live-theming]]
confidence_score: 0.7
date_created: 2026-08-19
date_completed: 2026-09-04
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

- [x] color-core consumed by path dependency; no engine code copied.
- [x] Tone-band projection implemented; property test: every role's
      emitted L within its configured band.
- [x] Guardrail verification tests across fixture corpus (incl. a
      monochrome and a near-black cover) — all floors met or
      documented synthetic fallback with provenance.
- [x] ron-crate base-theme patching emits a COMPLETE document; test:
      patching the default theme changes only color tokens (AST diff).
- [x] Determinism golden test (identical bytes → identical output).
- [x] --debug prints per-role origin/confidence/contrast matrix.
- [x] Gates green: fmt, clippy -D warnings, test.

### Acceptance Criteria

GIVEN a base theme with a custom layout, WHEN the generator patches
it, THEN the emitted document differs only in color tokens.
GIVEN low-chroma album art, WHEN roles are assigned, THEN neutrals
come from the ramp and every pair still meets its floor.

### Issues Encountered 

Round 1 (pre-implementation) found six real defects in ticket/spec:
broken path dep in the sitting clone (fixed: pinned sibling clone),
color-core stdout corruption (fixed upstream, 5baa20e), serde RON
round-trip destroying base themes (ruled: span-preserving rewriter,
SPEC 0.3), missing binding map + solver numerics (ruled, SPEC 0.3),
port-inventory line corrections. Round 2 STOP: rev 0.3's
"satisfiable by construction" band claim was numerically false
(witnesses 3.984/2.506/10.575, RL-reproduced); SPEC 0.4 demoted
bands to search bounds with floors verified on final outputs and
ruled the neutral peer exemption. Implementation then landed in one
commit (35ce2f2), 21 tests, merged da7af6c. §5 alias prose divergence
from the §6 map found at submission; reconciled in SPEC 0.5.
