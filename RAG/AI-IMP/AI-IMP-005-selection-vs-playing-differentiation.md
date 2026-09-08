---
node_id: AI-IMP-005
tags:
  - IMP-LIST
  - Implementation
  - rust
  - solver
kanban_status: planned
depends_on: ["AI-IMP-003"]
parent_epic: [[AI-EPIC-001-album-art-driven-live-theming]]
confidence_score: 0.8
date_created: 2026-09-07
date_completed:
---

# AI-IMP-005-selection-vs-playing-differentiation

## Summary of Issue #1

First-day live use (owner, 2026-09-07, DQ-1): the cursor row and
the playing row render identically — the §6 binding map routes both
`current_item_style` and `highlighted_item_style` to highlight-text
on active. Done state: a derived `selection` pair distinguishable
from the playing pair by lightness (owner ruling: same hue family,
selection LIGHTER), guardrail-verified (highlight-text vs selection
≥ 4.5:1 after gamut mapping; selection vs active ΔL floor so the
two bands never collapse), binding map split accordingly, golden
fixtures regenerated deliberately, SPEC §5/§6 amended.

### Out of Scope

New art-harvested roles (selection derives from active); UI/layout;
hook/installer changes.

### Design/Approach

`selection` = active projected to a lighter tone band (e.g. target
L = active.L + 0.10, band-clamped; exact numbers proposed round 1
and ruled into SPEC). Emitted as `current_item_style` bg;
`highlighted_item_style` keeps active. Verify: WCAG for its text
pair on final sRGB; minimum ΔL vs active; neutral ramp variant for
the both-neutral case. Provenance records `derived:selection`.

### Files to Touch

src/roles.rs, src/solver.rs, src/emit.rs (binding map),
tests/, fixtures/ (regenerated goldens), SPEC.org (via RL).

### Implementation Checklist

<CRITICAL_RULE>
Before marking an item complete on the checklist MUST **stop** and **think**. Have you validated all aspects are **implemented** and **tested**? 
</CRITICAL_RULE>

- [ ] Round-1 numeric proposal: selection band + ΔL floor, with
      counterexample search like Wave 1's.
- [ ] Binding map split; AST/golden tests updated deliberately.
- [ ] Guardrail verification incl. both-neutral case.
- [ ] Gates green; owner visual pass on 2-3 real albums.

### Acceptance Criteria

GIVEN any album art, WHEN the queue renders with cursor on a
non-playing row, THEN the cursor band and playing band are visibly
distinct (lighter selection, same palette family) and every floor
still holds.

### Issues Encountered

(to be filled during implementation)
