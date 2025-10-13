# rmpc tab redraw mitigation

## What changed
- Detects whether a tab layout contains multi-pane list widgets (directories, albums, artists, album artists, playlists, search, browser) by walking the `SizedPaneOrSplit` tree.
- Only clears the terminal on tab changes when the target tab requires a full redraw; queue-only tabs (album art + queue) skip the expensive `terminal.clear()`.
- Guards all tab-switch pathways (keyboard, mouse, UI event, remote tab switches) with the new predicate so repaint is consistent regardless of input.

## Why
- Previously we called `terminal.clear()` on every tab switch to prevent stale scrollbar gutters, but this caused album art to blink and reload because the image pane was wiped each time.
- Queue-centric tabs never suffered the stale-color bug, so they can avoid clearing entirely.
- Library/Search/Playlist tabs include the multi-pane widgets where ratatui’s diffing misbehaves; they still clear, preserving the visual fix without the album art penalty.

## Testing
- Built from source (`cargo build --release`) and ran the patched binary.
- Switched among Queue/Library/Search multiple times: queue keeps album art stable; library/search force a clean repaint with no stale gutters.
- Verified OSC 11/skip-hash changes remain unaffected.

## Next steps
- Offer upstream PR referencing https://github.com/mierak/rmpc/issues/328, noting selective clears prevent gutter artefacts while avoiding superfluous redraws.
