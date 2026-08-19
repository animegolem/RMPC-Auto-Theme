---
node_id: AI-EPIC-003
tags:
  - EPIC
  - AI
  - generator
  - theme
  - contrast
date_created: 2025-10-01
date_completed:
kanban-status: proposed
AI_IMP_spawned:
  - AI-IMP-013
  - AI-IMP-014
  - AI-IMP-015
  - AI-IMP-016
  - AI-IMP-017
  - AI-IMP-018
---

# AI-EPIC-003-deterministic-pairwise-and-theme-overlay

## Problem Statement/Feature Scope
Sequential color selection (bg → text → accent → active) creates unreadable overlays on light/tan palettes and yields inconsistent outcomes across similar inputs. Tuning thresholds helps some cases but regresses others, and our hardcoded theme template forces code edits for non-color changes. We need a deterministic color solver that optimizes state combinations and a user-editable base theme the generator overlays with computed colors.

## Proposed Solution(s)
1) Deterministic pairwise solver: choose Accent and Active together by evaluating a small matrix of contrasts (accent↔bg, accent↔text, accent↔active, text↔active, active↔bg) for candidate pairs, maximizing the minimum contrast and enforcing brightness separation. Emit a debug matrix explaining the choice.

2) OKLCH-based adjustments: perform lightness/chroma tweaks in OKLCH for perceptually stable darken/lighten operations, while keeping contrast checks in sRGB (WCAG 2) and optionally APCA as a tie-breaker.

3) Base theme overlay + user config: load a user-provided base RON (e.g., `~/.config/rmpc/theme-switcher/base.ron`) and patch named color tokens only. Add a small config (`config.ron`) for k/thresholds/sampling with CLI overrides. Provide a setup script that installs sample base/config (doom-style workflow).

## Path(s) Not Taken
- Increasing k or further threshold tweaks alone (insufficiently robust).
- Large rmpc UI refactors (out of scope for this epic).
- Forcing complementary hues universally (risk of artificial palettes) except as fallback heuristics.

## Success Metrics
- ≥95% of a 50-cover regression set meet: text↔bg ≥4.5, accent↔bg ≥3.0 (pref 4.5), text↔active ≥4.5, accent↔active ≥4.5 and ΔE ≥25; measured within 2 weeks of merge.
- Determinism: repeated runs on identical input produce identical role hexes and matrices (hash match) across Linux/macOS.
- Performance: generation ≤50 ms on 1024px-capped image; binary ≤2.0 MB.
- Adoption: user can change layout/symbols via `base.ron` without code edits.

## Requirements

### Functional Requirements
- [ ] FR-1: Build candidate sets from clusters + adjusted + synthetic colors.
- [ ] FR-2: Compute contrast matrix for each (accent, active) pair.
- [ ] FR-3: Optimize lexicographically: maximize minimum contrast, then brightness separation, then palette fidelity.
- [ ] FR-4: Enforce guardrails: text↔bg ≥4.5; text↔active ≥4.5; accent↔bg ≥3.0 (pref 4.5); accent↔active ≥4.5 and ΔE ≥25.
- [ ] FR-5: Implement OKLCH conversions; adjust L/C in OKLCH with sRGB gamut clipping.
- [ ] FR-6: Emit debug matrix + origins (cluster/adjusted/synthetic) in JSON output.
- [ ] FR-7: Load optional `~/.config/rmpc/theme-switcher/base.ron`; overlay color tokens only.
- [ ] FR-8: Load optional `~/.config/rmpc/theme-switcher/config.ron`; CLI overrides config.
- [ ] FR-9: Provide `scripts/setup-config.sh` to install sample base/config.
- [ ] FR-10: Add regression fixtures and a comparison harness for success metrics.

### Non-Functional Requirements
- NFR-1: Deterministic outputs given fixed inputs and config.
- NFR-2: No additional native deps; pure Rust implementation.
- NFR-3: Backward compatible when no base/config present.
- NFR-4: Keep runtime ≤50 ms and binary ≤2.0 MB (release).

## Implementation Breakdown
- AI-IMP-013: Pairwise (accent, active) solver + contrast matrix + scorer.
- AI-IMP-014: OKLCH conversions and adjustment utilities (with sRGB clamp).
- AI-IMP-015: Base theme overlay (load `base.ron`, patch tokens, write final RON).
- AI-IMP-016: Debug matrix and origin annotations in JSON; hash determinism check.
- AI-IMP-017: Config loader (`config.ron`) + CLI override plumbing; sample config.
- AI-IMP-018: Setup script, docs, and regression test harness with sample covers.
