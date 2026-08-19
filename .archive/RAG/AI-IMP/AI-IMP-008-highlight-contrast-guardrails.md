---
node_id: AI-IMP-008
tags:
  - IMP-LIST
  - Implementation
  - contrast
  - theme-generation
kanban_status: backlog
depends_on: AI-EPIC-002
confidence_score: 0.5
created_date: 2025-09-30
close_date:
--- 


# AI-IMP-008-highlight-contrast-guardrails

## Summary of Issue
Current accent and active colors can fall below readable contrast ratios, especially on dark palettes, leading to illegible text over highlights. We need to analyse contrast against both background and text, adjust lightness or select alternate clusters, and only emit colors that meet ≥4.5:1 vs text and ≥3.0:1 vs background. Completion means every generated highlight/active/tab color satisfies these thresholds or uses a controlled fallback.

### Out of Scope 
- Changes to primary text/background selection heuristics.
- UI rendering changes in rmpc itself.

### Design/Approach  
Enhance `select_accent_color` / `select_active_item_color` with scoring that rejects clusters below threshold, tries alternate clusters, and, if none qualify, performs lightness adjustments in Lab space while keeping hue/saturation within bounds. Increase the default cluster count (e.g. k=12) so more candidate colors are available before adjustments kick in. Require the highlight foreground to achieve ≥4.5:1 against background and enforce ≥4.5:1 contrast + ΔE ≥ 25 between accent and active selections so stacked states stay legible. Fallback to blending accent with text/background when adjustments hit gamut limits. Record contrast metrics in JSON output for validation.

### Files to Touch
- `src/rmpc_theme_gen.rs`: update selection logic, add adjustment helpers.
- `src/color.rs`: add utilities for Lab lightness tweaks if needed.
- `test-results/` fixtures or new tests verifying contrast ratios.
- `README.md` / docs: describe contrast guarantees.

### Implementation Checklist
<CRITICAL_RULE>
Before marking an item complete on the checklist MUST **stop** and **think**. Have you validated all aspects are **implemented** and **tested**? 
</CRITICAL_RULE> 
- [x] Update accent/active selection algorithms to enforce contrast thresholds.
- [x] Implement Lab lightness adjustment helper with bounds checking.
- [x] Add fallback path blending toward text/background when clusters fail.
- [x] Emit contrast metrics in JSON output for verification.
- [x] Document the new guarantees and configuration in README/docs.
 
### Acceptance Criteria
**Scenario:** Theme generated from dark album art.
**GIVEN** input art yields low-light palettes,
**WHEN** the theme is generated,
**THEN** the accent and active colors achieve ≥4.5:1 contrast with text and ≥3.0:1 with background (validated via automated test output) without dropping below the new default k value.

**Scenario:** No cluster meets thresholds.
**GIVEN** all clusters fail the thresholds,
**WHEN** the generator runs,
**THEN** it produces a blended fallback color that still meets the thresholds,
**AND** JSON output marks the color as synthetic.

### Issues Encountered 
_None yet._

<!-- Repeat the Issue pattern above as needed based on the needs of the users request.  --> 
