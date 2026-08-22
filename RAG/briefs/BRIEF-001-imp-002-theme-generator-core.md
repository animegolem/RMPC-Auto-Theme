# Assignment: Wave 1 — AI-IMP-002 theme generator core

Review Lead → Code Lead, 2026-08-22. One sitting, one ticket, one
domain: the `rmpc-theme-gen` binary — album-art bytes on stdin to a
complete, guardrail-verified rmpc theme on stdout. Manage your own
subagents as you see fit; the protocol in `.relay/PROTOCOL.md`
governs (isolated clone `.workspaces/code-lead/primary`, branch
`imp/002`, inbox submission, atomic commit per ticket,
destructive-op fence: you never delete clones/branches/refs — the
Review Lead owns cleanup).

## Round 1 is a PRE-IMPLEMENTATION REVIEW — no code

The ticket was cut 2026-08-19 against SPEC rev 0.2, before
AI-IMP-001 landed and before the scaffold commit (7081f49). Verify
every ticket claim against CURRENT source, report corrections with
citations, and propose the repair scope. The Review Lead answers with
rulings in the outbox; you code against the verdict where ticket
text differs. Known staleness to check explicitly:

- `src/main.rs` (45 lines) is a seam proof only: stdin → `color_core::analyze`
  → clusters on stderr. Confirm the `AnalyzeRequest` fields it uses
  still match `color-core` at `~/git/color-tool-kmeans` main (2cc2000+).
- `color-core/src/analyze.rs:220` `println!`s a merge log to STDOUT.
  The generator owns stdout (SPEC §6). Propose the route: a change in
  color-tool-kmeans (stderr or a log facade) — which goes through
  THAT repo's own process, not this channel — versus a generator-side
  workaround. State which you recommend and why; do not build either
  in round 1.
- `.archive/src/rmpc_theme_gen.rs` is the v1 port source. Inventory
  exactly which items the ticket names (guardrail consts, WCAG
  contrast, ΔE, hue separation w/ chroma gate, pairwise solver shape,
  `synthesize_color_between`, provenance structs) and cite line
  ranges. Anything else in that file is NOT ported.
- `ron = "0.12"` is pinned in Cargo.toml. Confirm it can round-trip
  rmpc 0.11's `example_theme.ron` (`~/git/_notmyrepos_/rmpc`) with the
  three pragmas preserved, or report what breaks.
- Verify the §5 floor table against the v1 consts; report any
  numeric drift.

## The ticket

1. **RAG/AI-IMP/AI-IMP-002-theme-generator-core.md** — there is no
   v2 binary. Done: `rmpc-theme-gen` reads art bytes on stdin,
   extracts via color-core, assigns the eight §5 roles under the §2
   tone-band doctrine (own lightness, harvest hue: L replaced per
   role band, hue held, chroma gamut-compressed), verifies the §5
   floors pairwise, patches ~9 color tokens into the base theme via
   the `ron` crate, and emits a COMPLETE document deterministically;
   `--debug` prints per-role origin/confidence/contrast matrix to
   stderr. Fixture corpus includes monochrome and near-black covers.
   Modules: `roles.rs`, `solver.rs`, `emit.rs`.

## Normative supplement (binding)

- **§2**: projection BEFORE the solver. Guardrails verify; they do
  not rescue. Ladder search is last-resort inside synthesis only.
  Neutral ramp fallback when art chroma is below threshold — do not
  invent chroma.
- **§5**: contrast is WCAG relative luminance on sRGB, never an L*
  proxy. Derived roles are aliases (inactive=frame, progress=accent,
  scrollbar=active), not independently solved.
- **§6**: partial documents are FORBIDDEN (`UiConfigFile` is
  `#[serde(default)]`; omitted fields reset to built-ins). Emit the
  whole document, 7-byte hex only, pragmas kept. Base theme defaults
  to the `example_theme.ron` equivalent.
- **Stdout discipline**: nothing but the theme on stdout, ever.
- **Config file loading is AI-IMP-003** — flags only here. If a seam
  makes the flags-only shape expensive, say so in round 1 rather
  than building it degraded.

## Fences

- Domain fence: `src/`, `tests/`, `fixtures/`, `Cargo.toml`/`Cargo.lock`
  of this repo.
- Do NOT touch: `~/git/color-tool-kmeans` (propose, don't edit),
  `SPEC.org`, `RAG/INDEX.md`, `RAG/DESIGN-QUEUE.md`,
  `RAG/HUMAN-TESTING.md`, `.archive/`, `README.md`.
- Reserved identifiers: none assigned — if you believe you need one,
  STOP and say so in a submission.
- Validation: `set -o pipefail` on every chain;
  `cargo fmt --check && cargo clippy -- -D warnings && cargo test`
  at the tip — read counts, not exit codes; `cargo build` before any
  test run (color-core is a path dep; the sibling repo must be
  present); `RAG/scripts/validate-tickets.sh --changed` clean before
  submitting.

## Report contents (each round)

Per PROTOCOL.md, plus: test counts per suite (unit, golden,
property, guardrail-corpus), the fixture corpus list with one-line
provenance each, the `--debug` output for one real cover, and candid
friction notes — they feed the wave retro.
