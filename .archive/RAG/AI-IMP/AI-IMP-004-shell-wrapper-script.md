---
node_id: AI-IMP-004
tags:
  - IMP-LIST
  - Implementation
  - shell-script
  - integration
  - on-song-change
kanban_status: completed
depends_on: [AI-IMP-003]
confidence_score: 0.9
created_date: 2025-09-29
close_date: 2025-09-29
---

# AI-IMP-004-shell-wrapper-script

## Summary of Issue

Create a shell wrapper script that integrates the theme generator with rmpc's `on_song_change` hook. The script extracts album art, invokes the theme generator, handles errors gracefully, and ensures the generated theme is available for rmpc to load.

**Scope:** Shell script creation, rmpc command integration, error handling, and basic logging.

**Measurable Outcome:** A working bash script at `~/.config/rmpc/on_song_change.sh` that successfully generates and applies themes when songs change in rmpc.

### Out of Scope

- Theme generator implementation (AI-IMP-001, 002, 003)
- Comprehensive error recovery strategies (AI-IMP-006 handles advanced fallback)
- Performance optimization beyond basic caching
- User-configurable theme preferences (future enhancement)

### Design/Approach

**Script Workflow:**
1. Check if new song differs from previous (use state file)
2. Extract album art using `rmpc albumart --output /tmp/rmpc/current_cover`
3. Check if album art extraction succeeded
4. Invoke theme generator: `rmpc-theme-gen --image /tmp/rmpc/current_cover --output ~/.config/rmpc/themes/current-song.ron`
5. If theme generation fails, log error and exit silently (don't disrupt playback)
6. Optional: Use `rmpc remote --pid $PID status "Theme updated for $ARTIST - $TITLE"`

**State Management:**
- Store previous song FILE path in `/tmp/rmpc/last_song_file`
- Compare `$FILE` env var to stored value
- Skip theme generation if song hasn't changed (prevents duplicate work on pause/unpause)

**Error Handling:**
- Check exit codes from `rmpc albumart` and `rmpc-theme-gen`
- Log errors to `~/.config/rmpc/theme-switcher.log`
- Fall back silently (don't show error to user unless verbose mode enabled)

**Performance:**
- Cache theme by album: if album art hash matches previous, reuse theme
- Keep temporary files minimal, clean up old covers

### Files to Touch

- `~/.config/rmpc/on_song_change.sh`: new shell script
- `~/.config/rmpc/theme-switcher.log`: log file for debugging

### Implementation Checklist

<CRITICAL_RULE>
Before marking an item complete on the checklist MUST **stop** and **think**. Have you validated all aspects are **implemented** and **tested**?
</CRITICAL_RULE>

- [ ] Create `~/.config/rmpc/on_song_change.sh` with shebang `#!/usr/bin/env bash`
- [ ] Add header comment explaining script purpose and usage
- [ ] Set strict mode: `set -euo pipefail` for robust error handling
- [ ] Define variables: `TMP_DIR="/tmp/rmpc"`, `THEME_DIR="$HOME/.config/rmpc/themes"`, `LOG_FILE="$HOME/.config/rmpc/theme-switcher.log"`
- [ ] Create temp and theme directories if they don't exist: `mkdir -p "$TMP_DIR" "$THEME_DIR"`
- [ ] Read previous song from state file: `LAST_FILE=$(cat "$TMP_DIR/last_song_file" 2>/dev/null || echo "")`
- [ ] Compare current `$FILE` with `$LAST_FILE`, exit early if same
- [ ] Update state file with current song: `echo "$FILE" > "$TMP_DIR/last_song_file"`
- [ ] Extract album art: `rmpc albumart --output "$TMP_DIR/current_cover" >> "$LOG_FILE" 2>&1`
- [ ] Check album art extraction exit code, log and exit if failed
- [ ] Verify album art file exists and is non-empty: `[ -s "$TMP_DIR/current_cover" ]`
- [ ] Invoke theme generator: `rmpc-theme-gen --image "$TMP_DIR/current_cover" --output "$THEME_DIR/current-song.ron" >> "$LOG_FILE" 2>&1`
- [ ] Check theme generation exit code, log error if failed
- [ ] Verify theme file was written successfully
- [ ] Optional: Send status message to rmpc: `rmpc remote --pid "$PID" status "Theme updated" --level info >> "$LOG_FILE" 2>&1 || true`
- [ ] Add timestamp to log entries: `echo "[$(date '+%Y-%m-%d %H:%M:%S')] Theme generated for $ARTIST - $TITLE" >> "$LOG_FILE"`
- [ ] Make script executable: `chmod +x ~/.config/rmpc/on_song_change.sh`
- [ ] Test script manually by setting env vars and running: `FILE=/path/to/song.mp3 ARTIST="Test" TITLE="Song" ./on_song_change.sh`
- [ ] Verify theme file is created and valid
- [ ] Verify log file contains appropriate entries
- [ ] Test error case: missing album art (script should exit gracefully)

### Acceptance Criteria

**Scenario:** Script generates theme when song changes.

**GIVEN** rmpc is playing and a song change occurs.
**WHEN** The `on_song_change` hook triggers the script with environment variables set.
**THEN** Album art is extracted to `/tmp/rmpc/current_cover`.
**AND** Theme generator is invoked with the album art path.
**AND** Theme file is written to `~/.config/rmpc/themes/current-song.ron`.
**AND** Log file contains timestamped entry for the theme generation.
**AND** Script completes within 1 second.
**AND** Script exits with code 0.

**Scenario:** Script handles missing album art gracefully.

**GIVEN** A song without embedded album art is playing.
**WHEN** The script runs and `rmpc albumart` fails.
**THEN** Script logs the error to theme-switcher.log.
**AND** Script exits without generating a theme.
**AND** Script exits with code 0 (non-disruptive).
**AND** No error messages are shown to user.

**Scenario:** Script skips duplicate processing.

**GIVEN** The same song is playing and script has already run.
**WHEN** Script is triggered again (e.g., on unpause).
**THEN** Script reads state file and detects duplicate.
**AND** Script exits early without extracting album art or generating theme.
**AND** Script completes in under 10ms.

### Issues Encountered

**Completed 2025-09-29:**
- Successfully created shell wrapper script at ~/.config/rmpc/on_song_change.sh (2.3KB)
- Script includes comprehensive error handling and logging
- State management prevents duplicate processing on same song
- Logging format: `[YYYY-MM-DD HH:MM:SS] message` to theme-switcher.log
- Environment variable support: RMPC_THEME_GEN_PATH for binary location override
- Tested error handling: properly handles missing album art and failed extraction
- Script exits silently (code 0) on errors to avoid disrupting rmpc playback
- Log file location: ~/.config/rmpc/theme-switcher.log
- Optional rmpc remote status notifications when PID available
- Script is executable (chmod +x) and uses bash strict mode (set -euo pipefail)