---
id: AI-IMP-012
title: Skip theme generation when album art is unchanged
status: completed
owner: ai
created: 2025-09-30
tags: [performance, ux]
---

Problem
- On every track change, the generator reloads the image and writes a theme, causing visual noise and unnecessary work when the art is identical.

Proposal
- Compute a fast hash (e.g., SHA-256) of the album art in `on_song_change.sh`, cache the last hash in a dotfile under `~/.config/rmpc/theme-switcher/.last_art.sha256`, and no-op if the hash matches. If different, proceed with generation and update the cache.

Acceptance Criteria
- If album art file content is unchanged, the script exits without re-running the generator or touching the theme file.
- If changed, generator runs as normal and cache updates.
- Add a `--force` env/flag to bypass the hash check for debugging.

Notes
- Do the check in the shell wrapper to avoid adding I/O paths to the binary.
