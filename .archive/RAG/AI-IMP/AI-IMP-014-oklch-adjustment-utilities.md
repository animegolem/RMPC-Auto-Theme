---
node_id: AI-IMP-014
tags:
  - IMP-LIST
  - Implementation
  - color-science
kanban_status: planned
depends_on: [AI-EPIC-003]
confidence_score: 0.72
created_date: 2025-10-01
close_date:
---

# AI-IMP-014-oklch-adjustment-utilities

## Summary of Issue
Lab lightness tweaks can look muddy on light backgrounds. Implement OKLab/OKLCH conversions and lightness/chroma adjustments to produce cleaner darken/lighten while keeping contrast checks in sRGB.

### Out of Scope
- Replacing WCAG contrast with APCA as a hard gate (tie-breaker only if added).

### Design/Approach
- Add OKLab/OKLCH converters (float32) with sRGB D65 pipeline.
- Provide helpers: `oklch_darken`, `oklch_lighten`, `oklch_clamp_chroma` with gamut clipping back to sRGB.
- Integrate with adjustment paths used by the pairwise solver.

### Files to Touch
- `src/color.rs`: conversions and helpers.
- `src/rmpc_theme_gen.rs`: swap adjustment utility calls.

### Implementation Checklist

<CRITICAL_RULE>
Before marking an item complete on the checklist MUST **stop** and **think**. Have you validated all aspects are **implemented** and **tested**? 
</CRITICAL_RULE>

- [ ] Add OKLab/OKLCH conversion functions with tests (round-trip within tolerance).
- [ ] Add helpers for L/C adjustments and gamut clamp.
- [ ] Replace Lab-based tweak calls in generator with OKLCH equivalents.
- [ ] Validate on light-tan and neon-pastel covers for cleaner results.

### Acceptance Criteria
**GIVEN** adjustment is required,
**WHEN** colors are darkened/lightened,
**THEN** resulting RGB remains in gamut and looks less muddy subjectively on light palettes,
**AND** contrast measurements remain equal or better vs prior implementation.

### Issues Encountered
TBD.

