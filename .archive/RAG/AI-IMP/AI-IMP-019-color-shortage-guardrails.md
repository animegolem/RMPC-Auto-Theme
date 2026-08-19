---
node_id: AI-IMP-019
tags:
  - IMP-LIST
  - Implementation
  - generator
  - contrast
  - palette
kanban_status: backlog
depends_on:
  - AI-IMP-013
confidence_score: 0.68
created_date: 2025-10-01
close_date:
---

# AI-IMP-019-color-shortage-guardrails

## Rein in synthetic colors when palette is sparse
Shallow palettes are forcing the solver into fallback mode, where we invent neutral accents/actives. Active backgrounds end up within ~2:1 of the base (see `bench-results/problem-baseline/*.json`), making the queue hard to read. We need stronger guardrails for active↔bg contrast, a structured way to reuse existing roles before inventing new colors, and optional extra clusters to widen the candidate pool.

### Out of Scope
- Switching adjustments to OKLCH (AI-IMP-014).
- Base theme/config loader work (AI-IMP-015).
- Debug matrix enhancements already covered by AI-IMP-016.

### Design/Approach
- Raise `ACTIVE_BG_MIN` (and relaxed floor) to ≥3.5 and retune solver scoring so higher active↔bg contrast wins ties.
- Stage fallback order: prefer palette-derived colors and deterministic transformations before synthetic neutrals; cap synthetic use as last resort.
- Introduce legal role groups (e.g., header/progress/scrollbar) so shortages reuse an existing assignment instead of minting new colors.
- Optionally lift default `k` to 16 when not overridden, provided timing remains <50 ms (validated against `bench-assets/problem-covers`).
- Capture before/after metrics in `bench-results/problem-baseline/` to verify improved contrast without excessive color invention.

### Files to Touch
- `src/rmpc_theme_gen.rs`: adjust guardrail constants, solver scoring, fallback ordering, and legal role reuse.
- `README.md`: document revised guardrails and optional higher default `k`.
- `bench-results/` notes (if needed) for comparison.

### Implementation Checklist

<CRITICAL_RULE>
Before marking an item complete on the checklist MUST **stop** and **think**. Have you validated all aspects are **implemented** and **tested**? 
</CRITICAL_RULE>

- [ ] Raise `ACTIVE_BG_MIN` (and relaxed peer floor) and rerun `bench-assets/problem-covers`.
- [ ] Update solver to reorder fallback preference: palette-derived → transformed → synthetic; log provenance in debug output.
- [ ] Add legal role group mapping so shortage roles reuse existing assignments instead of inventing new colors.
- [ ] Optionally bump default `k` to 16 (or make conditional) and measure generation time vs 50 ms target.
- [ ] Update README guardrail/usage docs and capture before/after metrics in `bench-results`.

### Acceptance Criteria
**GIVEN** fidlar/golgo/ohio covers under `bench-assets/problem-covers`,  
**WHEN** generating themes with the updated solver (default settings),  
**THEN** active↔background contrast is ≥3.5, accent↔background ≥3.0,  
**AND** `debug.pairwise.winningPair.origin` stays within cluster/adjusted where feasible (synthetic only when no legal pair exists),  
**AND** header/progress/scrollbar reuse the same color without inventing extras,  
**AND** generation time stays ≤50 ms on 1024px-limited images.

### Issues Encountered
TBD

