---
node_id: AI-LOG-2025-09-30-scrollbar-investigation
tags:
  - AI-log
  - development-summary
  - theme-generation
  - rmpc
closed_tickets: []
created_date: 2025-09-30
related_files:
  - AGENTS.md
  - src/rmpc_theme_gen.rs
  - on_song_change.sh
  - test-results/run-tests.sh
  - README.md
  - pkgs/rmpc_PR_prep/src/ui/widgets/browser.rs
  - pkgs/rmpc_PR_prep/src/ui/panes/queue.rs
  - pkgs/rmpc_PR_prep/src/ui/panes/search/mod.rs
  - pkgs/rmpc_PR_prep/src/ui/panes/logs.rs
  - pkgs/rmpc_PR_prep/src/ui/modals/select_modal.rs
  - pkgs/rmpc_PR_prep/src/ui/modals/outputs.rs
  - pkgs/rmpc_PR_prep/src/ui/modals/keybinds.rs
  - pkgs/rmpc_PR_prep/src/ui/modals/decoders.rs
  - pkgs/rmpc_PR_prep/src/ui/modals/info_list_modal.rs
confidence_score: 0.6
---

## Summary
- Enhanced theme generator (AI-IMP-007) to write background-aligned scrollbar styles, surface `--disable-scrollbar`, and extend docs/tests, but visual artefact persisted.
- Forked upstream rmpc (AI-IMP-010) and iterated on multiple repaint strategies—`Clear`, background `Block`, and direct `buffer.set_style`—across all panes/modals with scrollbars; behaviour unchanged in Ghostty and GNOME Terminal during rapid tab switches.
- Captured detailed contingency plan (`AI-EPIC-002-scrollbar-contingency.md`) and documented terminal observations showing correct escape codes but stale gutter pixels still visible until window refresh.

## Details
- Updated generator CLI (`src/rmpc_theme_gen.rs`, `on_song_change.sh`, `test-results/run-tests.sh`, `README.md`) to paint scrollbar track/ends with background colour and expose opt-out flag.
- Repeatedly rebuilt local fork (`pkgs/rmpc_PR_prep`) after touching `widgets/browser.rs`, queue/search/log panes, and multiple modals to clear and repaint scrollbar columns before drawing the thumb; also filled columns even when thumb absent.
- Terminal inspector confirmed no stale colour escapes; suspect repaint timing/double buffering in ratatui causes pixels to linger until a full redraw (e.g., resize) occurs.

## Tickets
- AI-IMP-007 (partial progress, not closed)
- AI-IMP-010 (documentation + prototype changes, PR pending)

## Next Steps
- Break to reassess approach; potential avenues include forcing full pane redraw on theme swap or adjusting ratatui widget behaviour rather than per-pane fixes.
- Compact workspace before further exploration.
