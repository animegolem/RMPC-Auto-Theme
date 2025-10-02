---
node_id: AI-IMP-013
tags:
  - IMP-LIST
  - Implementation
  - generator
  - contrast
kanban_status: planned
depends_on: [AI-EPIC-003]
confidence_score: 0.78
created_date: 2025-10-01
close_date:
---

# AI-IMP-013-pairwise-accent-active-solver

## Summary of Issue
Current sequential selection can choose an accent that conflicts with the later-picked active background, producing unreadable overlays. Build a deterministic solver that evaluates (accent, active) pairs and selects the pair that maximizes minimum contrast while honoring guardrails and brightness separation. Outcome: ≥95% of a 50-cover regression set passes all overlay contrast checks.

### Out of Scope
- Theme template redesign and base overlay (handled in AI-IMP-015).
- OKLCH math (AI-IMP-014) beyond minimal stubs if needed.

### Design/Approach
- Construct candidate lists for accent and active from: clusters, adjusted (lightness tweaks), and synthetic.
- For each pair, compute a contrast matrix: accent↔bg, accent↔text, accent↔active, text↔active, active↔bg.
- Score lexicographically: (1) maximize min contrast; (2) enforce |L*|≥25 separation; (3) prefer cluster>adjusted>synthetic; (4) prefer higher APCA (optional tie-break).
- Emit the winning pair plus full matrix into JSON (debug block).

### Files to Touch
- `src/rmpc_theme_gen.rs`: add pairwise solver, scoring, and JSON debug export.
- `src/color.rs`: helper for ΔE/contrast utils reuse (if needed).
- `README.md`: brief note about deterministic solver and debug output.

### Implementation Checklist

<CRITICAL_RULE>
Before marking an item complete on the checklist MUST **stop** and **think**. Have you validated all aspects are **implemented** and **tested**? 
</CRITICAL_RULE>

- [ ] Add `PairCandidate` structs and sources (cluster/adjusted/synthetic) with provenance.
- [ ] Implement matrix builder computing all required pairwise contrasts.
- [ ] Implement lexicographic scorer (min-contrast first, then separation, then provenance).
- [ ] Replace sequential accent/active selection with solver output.
- [ ] Add JSON `debug.pairwise` block with matrix, chosen pair, and rationale.
- [ ] Guard with existing thresholds (text↔bg 4.5, text↔active 4.5, accent↔bg ≥3.0 pref 4.5, accent↔active ≥4.5 & ΔE ≥25).
- [ ] Add feature flag/env to toggle verbose debug (default off in release JSON to keep size reasonable).
- [ ] Smoke test on 5–10 covers; verify overlays improve and outputs are deterministic.

### Acceptance Criteria
**GIVEN** the solver is enabled,
**WHEN** themes are generated for the regression covers,
**THEN** the chosen accent/active pair yields all overlay contrasts passing thresholds,
**AND** JSON contains a `debug.pairwise` block with a single winner and rationale,
**AND** repeated runs with identical inputs produce identical hex values.

### Issues Encountered
TBD (to be filled post-implementation).

