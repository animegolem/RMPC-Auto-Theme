---
node_id: AI-IMP-020
tags:
  - IMP-LIST
  - Implementation
  - generator
  - contrast
  - theming
kanban_status: backlog
depends_on: [AI-IMP-013]
confidence_score: 0.72
created_date: 2025-10-01
close_date:
---

# AI-IMP-020-highlight-text-and-frame-role

## Summary of Issue
Current highlights (active rows) can be unreadable because one global text color is reused everywhere and the “frame” strokes (rules, separators, rails) pull independent colors. We need: (1) a dedicated `highlight_text` that always contrasts with the chosen Active background, and (2) a single Frame color applied to the thin rules/separators so we avoid inventing extra colors without impacting pane backgrounds.

### Out of Scope
- OKLCH conversion utilities (AI-IMP-014) unless needed for small L* shifts.
- Base theme overlay/config (AI-IMP-015).
- rmpc code changes; we stick to generator output.

### Design/Approach
- Introduce `highlight_text` role selected to satisfy ≥ 4.5:1 vs Active (highlight background). Candidate order: body text → adjusted text (±L) → white/black → accent (only if ≥4.5:1). Persist choice in JSON debug.
- Add `Frame` role for UI strokes; apply to:
  - `borders_style.fg`
  - `highlight_border_style.fg`
  - header states `separator_style.fg`
  - `progress_bar.track_style.fg`, `progress_bar.ends_style.fg`
  - `scrollbar.ends_style.fg` (keep track bg = background for repaint safety)
  - thumb default: Accent (better visibility); allow config to switch to Frame
- Hard rules (enforced):
  - Text↔Background ≥ 4.5:1; HighlightText↔Active ≥ 4.5:1
  - Active↔Background ≥ 3.5:1 (relaxed 3.0)
  - Accent↔Active ≥ 3.0:1 and ΔE ≥ 25; ΔE ≥ 10 w.r.t Text to avoid “same color”
- Legal collisions: all listed strokes reuse Frame; pane backgrounds stay independent.
- Add debug: emit selected `highlightText` and `frame` with rationale.

### Files to Touch
- `src/rmpc_theme_gen.rs`: add roles, selection helpers, guardrails; update RON writer mappings.
- `README.md`: document new roles, legal-collision policy, and defaults (thumb=Accent).
- `test-results/` (optional): add before/after snippets for the three problem covers.

### Implementation Checklist

<CRITICAL_RULE>
Before marking an item complete on the checklist MUST **stop** and **think**. Have you validated all aspects are **implemented** and **tested**? 
</CRITICAL_RULE>

- [ ] Add `highlight_text` selection ensuring ≥ 4.5:1 vs Active; fallbacks per design.
- [ ] Add `frame` role selection (prefer Accent; small ±L adjustments if needed; never equal to Text/Active).
- [ ] Apply `frame` to borders, highlight border, header separator, progress track/ends, scrollbar ends (keep scrollbar track bg = background).
- [ ] Keep scrollbar thumb = Accent by default; gate a config/env to switch to Frame.
- [ ] Extend JSON debug with `roles.highlightText` and `roles.frame` (hex + rationale).
- [ ] Re-generate themes for `bench-assets/problem-covers` and capture results.
- [ ] Update README to describe the new roles and legal-collision mapping.

### Acceptance Criteria
**GIVEN** the three problem covers in `bench-assets/problem-covers`,
**WHEN** generating themes with the updated roles,
**THEN** current-row text (highlightText) contrasts with Active ≥ 4.5:1,
**AND** Active↔Background ≥ 3.5:1 (≥ 3.0 in relaxed),
**AND** borders/separators/rails share the Frame color, with no additional invented tones,
**AND** scrollbar track remains background-colored (to avoid stale repaint), ends use Frame, thumb uses Accent by default,
**AND** outputs are deterministic with `--debug` showing role rationales.

### Issues Encountered
TBD (fill out during implementation and testing).

