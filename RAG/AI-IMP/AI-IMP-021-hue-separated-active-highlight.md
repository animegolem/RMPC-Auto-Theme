---
node_id: AI-IMP-021
tags:
  - IMP-LIST
  - Implementation
  - generator
  - contrast
  - color-science
kanban_status: backlog
depends_on:
  - AI-IMP-013
  - AI-IMP-014
confidence_score: 0.7
created_date: 2025-10-01
close_date:
---

# AI-IMP-021-hue-separated-active-highlight

## Enforce hue separation between active highlight and page palette
Active rows that are “playing but not selected” still blend into the page because the active background, body text, and accent share similar hues even when luminance contrast is high. We need to incorporate OKLCH hue/chroma separation so the active highlight picks a colour family distinct from the page/background/text without sacrificing readability.

### Out of Scope
- Full OKLCH adjustment utilities beyond what’s required for hue/chroma checks (handled in AI-IMP-014).
- Non-highlight colour roles (handled by existing guardrails).
- rmpc UI changes; this remains generator-only.

### Design/Approach
- Convert candidate colours to OKLCH (use helpers from AI-IMP-014) and add thresholds such as `min_hue_delta` (~25°) and `min_chroma_delta` to keep hues distinct when chroma is non-zero.
- During pairwise solving and fallback, ensure Active vs Background, Active vs Text, and Active vs Accent satisfy both contrast and hue separation. If the palette lacks a suitable hue, synthesize one by rotating the hue (e.g. ±30° / 180°) and adjusting lightness to keep ≥3.5:1 contrast.
- Reuse the derived highlight text for tab labels (already in place) and verify the playing-but-not-selected row remains legible.
- Increase default cluster count to 30 to widen the available hue options (configurable later).
- Regenerate the benchmark covers to confirm the playing row stands out perceptually.

### Files to Touch
- `src/rmpc_theme_gen.rs`: add OKLCH hue/chroma checks, hue-rotated fallback for Active, bump default `k` to 30, and extend debug output with hue info.
- `src/color.rs`: helper for RGB ↔ OKLCH (if not already added in AI-IMP-014).
- `README.md`: document hue guardrails, 30-cluster default, and the rationale.

### Implementation Checklist

<CRITICAL_RULE>
Before marking an item complete on the checklist MUST **stop** and **think**. Have you validated all aspects are **implemented** and **tested**? 
</CRITICAL_RULE>

- [ ] Introduce OKLCH conversions (if missing) and helper to compute hue/chroma differences.
- [ ] Enforce `min_hue_delta` / `min_chroma_delta` across Active vs Background/Text/Accent during solver and fallback.
- [ ] When no palette colour satisfies both contrast and hue separation, synthesize a hue-rotated candidate (preserve contrast floors).
- [ ] Set default `k` to 30 and allow `--k`/config to override.
- [ ] Update JSON debug with hue/chroma metrics for the chosen active colour.
- [ ] Regenerate `bench-assets/problem-covers` outputs and verify playing rows stand out.
- [ ] Update README with the new guardrails and cluster default.

### Acceptance Criteria
**GIVEN** the problem covers in `bench-assets/problem-covers`,
**WHEN** generating themes with hue-separated logic,
**THEN** the playing-but-not-selected row (Active background) differs in hue or chroma sufficiently to remain visually distinct while keeping ≥3.5:1 contrast against the page,
**AND** active tab labels use the same highlight text colour (≥4.5:1 vs tab background),
**AND** JSON debug reports hue/chroma deltas so we can audit the solver,
**AND** generation remains ≤50 ms under the higher `k` default.

### Issues Encountered
TBD

