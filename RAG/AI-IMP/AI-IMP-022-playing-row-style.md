---
node_id: AI-IMP-022
tags:
  - IMP-LIST
  - Implementation
  - generator
  - contrast
  - rmpc
kanban_status: backlog
depends_on:
  - AI-IMP-013
  - AI-IMP-021
confidence_score: 0.7
created_date: 2025-10-01
close_date:
---

# AI-IMP-022-playing-row-style

## Give the “playing but not selected” row a distinct style
Even after tightening contrast guardrails, the queue row that is *playing* but not currently highlighted still blends into the page. rmpc renders that state as `accent` text on the plain background, so when accent drifts toward neutral hues the row becomes nearly invisible. We need a generator-level fix that keeps the playing row readable without sacrificing palette fidelity.

### Out of Scope
- Full OKLCH optimisation (handled in AI-IMP-021).
- rmpc fork changes; we stay within the theme generator.
- Non-queue components (tabs, modals) already covered by AI-IMP-020/021.

### Design/Approach
- Introduce a dedicated `PlayingRow` role (or reuse existing Active/Highlight roles) that rmpc can map to the “now playing” state:
  - Option A: set `highlighted_item_style` to accent, `current_item_style` to highlight, and map `playing_row_style` (new) to `active` background with `highlight_text`. This needs rmpc support.
  - Option B (no rmpc changes): reconfigure the theme so the playing row uses the same background/text combination as the current row—e.g., set `highlighted_item_style.bg` to the active background and keep accent for foreground. This effectively makes the playing row share the highlight background even when not selected.
- Enforce hue separation when selecting accent vs background so accent remains chromatically distinct.
- Update the theme exporter to ensure `<playing>` states (status headers) remain accent-coloured while the queue row gets the high-contrast pair.
- Update debug output to record the chosen playing-row colours.

### Files to Touch
- `src/rmpc_theme_gen.rs`: add/select the new playing-row colour (likely reusing Active background + Highlight text) and adjust the RON template mappings.
- `README.md`: describe the new behavior so users know the playing row inherits the highlight background.
- `bench-results/` artefacts for regression.

### Implementation Checklist

<CRITICAL_RULE>
Before marking an item complete on the checklist MUST **stop** and **think**. Have you validated all aspects are **implemented** and **tested**? 
</CRITICAL_RULE>

- [ ] Decide on the playing-row mapping (shared highlight background vs new role) and implement it in the RON template.
- [ ] Ensure accent text remains readable after the change (adjust guardrails if needed).
- [ ] Update JSON debug output with playing-row assignments.
- [ ] Regenerate bench themes to confirm the playing-but-not-selected row is visually distinct across covers.
- [ ] Refresh README to call out the new behaviour.

### Acceptance Criteria
**GIVEN** the existing benchmark covers,
**WHEN** the generator emits a theme,
**THEN** the queue row corresponding to the currently playing track (but not selected) shows the high-contrast highlight background and readable text,
**AND** accent is still usable elsewhere (status, borders),
**AND** JSON debug includes the playing-row colour mapping so we can audit it.

### Issues Encountered
TBD

