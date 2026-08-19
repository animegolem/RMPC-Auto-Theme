---
id: AI-IMP-011
title: Add explicit background to all theme styles
status: completed
owner: ai
created: 2025-09-30
tags: [theme, styling, repaint]
---

Problem
- Intermittent stale vertical strips appear after switching tabs, likely due to partial redraws and style diffs skipping unchanged cells.

Goal
- Reduce artefacts without modifying rmpc by emitting explicit background values for as many style fields as the schema allows.

Scope
- Update generator to include bg: background_color in style objects: preview_label_style, preview_metadata_group_style, highlighted_item_style, progress_bar (track/elapsed/thumb), scrollbar (track/ends/thumb), and album cell styles.

Acceptance Criteria
- Generated RON sets bg for listed style blocks.
- No schema parse errors in rmpc.
- Visual artefacts decrease in frequency during Queue ↔ Library tab switches.

Notes
- Upstream already clears on config/theme reload; this targets tab navigation.
