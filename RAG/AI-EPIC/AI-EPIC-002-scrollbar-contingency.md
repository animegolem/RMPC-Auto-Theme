# AI-EPIC-002 Scrollbar Contingency Plan

## Summary
Dynamic themes still reveal stale "gutter" pixels when rmpc panes reuse the same scrollbar column without repainting it. The generator can hide most cases, but rapid tab switches (or panes without scroll thumb) still inherit the previous color. This contingency documents the minimal upstream fix: clear the scrollbar column before re-rendering in every component that draws a vertical scrollbar.

## Target Files
- `src/ui/widgets/browser.rs`: queue/library browser columns.
- `src/ui/panes/queue.rs`: queue table pane.
- `src/ui/panes/search/mod.rs`: search pane song column.
- `src/ui/panes/logs.rs`: logs reader.
- `src/ui/modals/{select_modal, outputs, keybinds, decoders, info_list_modal}.rs`: modal lists that render scrollbars.

## Implementation Notes
1. Import `ratatui::widgets::Clear` where missing.
2. Before calling `frame.render_stateful_widget(scrollbar, area, ...)` (or the widget equivalent with buffers), insert `frame.render_widget(Clear, area);` to wipe the column. For widget contexts (no `Frame`), use `Clear.render(area, buf);`.
3. Reuse existing area calculations by storing them in a local variable (`let scrollbar_area = ...;`).
4. No behaviour changes occur when scrollbars are disabled; the clear simply blanks the column before the optional draw.

## Verification
- `cargo fmt`
- `cargo check`
- Manual smoke test: `cargo run --features image-preview` (optional) and switch rapidly between Queue/Library/Search tabs to confirm the column no longer shows stale colors.

## Open Questions / Risks
- `Clear` requires the area width > 0. Existing code guards for zero-width scrollbars (e.g., queue). Ensure any new panes respect that before calling the clear.
- Ratatui `Clear` erases with default background color; upstream theme values must still paint the scrollbar track so the column blends with the active palette.

## Next Steps
- Once validated locally, create a PR against the upstream rmpc repository with these changes, referencing this document and screenshots showing the issue before/after.
- Link the PR to AI-IMP-010 for traceability.
