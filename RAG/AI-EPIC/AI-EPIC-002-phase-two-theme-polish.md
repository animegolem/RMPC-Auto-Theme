# AI-EPIC-002-phase-two-theme-polish
---
node_id: AI-EPIC-002
tags:
  - EPIC
  - AI
  - theme-generation
  - contrast
  - ui-polish
date_created: 2025-09-30
date_completed:
kanban-status: proposed
AI_IMP_spawned: []
---

# AI-EPIC-002-phase-two-theme-polish

## Problem Statement/Feature Scope
Dynamic themes sometimes inherit visible gutter artifacts and unreadable highlight colors. Scrollbar tracks keep the first boot palette, leaving mismatched vertical strips, and accent selections occasionally fail contrast expectations, making text hard to read. We need a second phase that keeps the generator-only workflow viable until upstream rmpc adjustments are considered.

## Proposed Solution(s)
Deliver generator-side mitigation that hides residual gutters and enforces minimum contrast for focus, accent, and tab styles. The theme generator will explicitly paint scrollbar track/ends with the current background color and optionally disable the gutter when no scrollbar is needed. For highlight colors, expand the role-mapping heuristics to measure WCAG ratios against both background and text, adjust lightness as needed, and fall back to synthetic color blends when album art clusters cannot meet the target. The epic also documents contingency work: if residual artifacts persist, draft a minimal rmpc patch that clears reserved columns before rendering, ready for submission as a follow-up PR.

## Path(s) Not Taken
- Postponing fixes until an upstream rmpc release; rejected to keep standalone users unblocked.
- Replacing dynamic colors with fixed palettes; out of scope because it removes album-art responsiveness.
- Heavy rmpc theming rewrite; deferred unless generator-side adjustments fail.

## Success Metrics
- Within 1 week of release, verified gutter strip no longer shows on 30 album switches without modifying rmpc (QA checklist).
- All generated highlight/active colors maintain ≥4.5:1 contrast against text and ≥3.0:1 against background in automated regression tests.
- Zero regressions in theme generation time (remains ≤500 ms median on tested fixtures).

## Requirements

### Functional Requirements
- [ ] FR-1: Generator shall set scrollbar `track_style` and `ends_style` to the selected background color when writing themes.
- [ ] FR-2: Generator shall detect when the scrollbar is effectively unused and allow opting out of rendering via configuration.
- [ ] FR-3: Generator shall enforce contrast thresholds for accent and active roles by adjusting lightness or selecting alternate clusters.
- [ ] FR-4: Generator shall fall back to synthetic accent colors when no sampled cluster can satisfy contrast rules while preserving hue affinity.
- [ ] FR-5: Regression tests shall cover gutter mitigation and contrast adjustments using representative album art fixtures.
- [ ] FR-6: Documentation (`README` and `TEST-RESULTS.md`) shall describe the new contrast logic and testing process.
- [ ] FR-7: Contingency: prepare a scoped upstream patch plan that clears scrollbar gutters inside rmpc renderers, to be executed only if generator fixes prove insufficient.

### Non-Functional Requirements
- Contrast adjustments must keep generation under 650 ms on 1000×1000 album art (measured on reference hardware).
- Theme outputs must remain valid RON and backwards compatible with rmpc ≥0.9.0.
- New logic must be covered by automated tests that fail on contrast regressions.

## Implementation Breakdown
- [ ] AI-IMP-007: Scrollbar gutter mitigation via background-aligned styles and opt-out flag.
- [ ] AI-IMP-008: Highlight contrast guardrails with adjustment/fallback logic.
- [ ] AI-IMP-009: Regression testing and documentation updates for phase-two behaviour.
- [ ] AI-IMP-010: Contingency plan for upstream rmpc scrollbar clearing patch (`AI-EPIC-002-scrollbar-contingency.md`).
