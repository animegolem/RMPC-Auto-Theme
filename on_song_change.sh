#!/usr/bin/env bash
#
# rmpc Dynamic Theme Generator - on_song_change hook
#
# This script is triggered by rmpc when a song changes. It extracts album art,
# analyzes dominant colors using K-means clustering, and generates a theme file
# that rmpc automatically reloads.
#
# Requirements:
#   - rmpc-theme-gen binary in PATH
#   - rmpc with enable_config_hot_reload: true
#
# Environment variables provided by rmpc:
#   $FILE, $ARTIST, $TITLE, $ALBUM, $PID, etc.
# Optional overrides:
#   RMPC_THEME_GEN_DISABLE_SCROLLBAR=1 to omit scrollbar block from generated themes
#

set -euo pipefail

# Configuration
TMP_DIR="/tmp/rmpc"
THEME_DIR="$HOME/.config/rmpc/themes"
LOG_FILE="$HOME/.config/rmpc/theme-switcher.log"
BINARY_PATH="${RMPC_THEME_GEN_PATH:-rmpc-theme-gen}"

# Optional toggles
SCROLLBAR_ARGS=()
case "${RMPC_THEME_GEN_DISABLE_SCROLLBAR:-0}" in
    1|true|TRUE|True|yes|YES)
        SCROLLBAR_ARGS+=("--disable-scrollbar")
        ;;
esac

# Ensure directories exist
mkdir -p "$TMP_DIR" "$THEME_DIR"

# Hash cache for album art
HASH_FILE="$HOME/.config/rmpc/theme-switcher/.last_art.sha256"
SKIP_COUNT_FILE="$HOME/.config/rmpc/theme-switcher/.skip_count"

# Helper: sha256 of a file (portable)
sha256_of() {
    local f="$1"
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$f" | awk '{print $1}'
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$f" | awk '{print $1}'
    else
        # No hashing tool available; return empty to disable short-circuit
        echo ""
    fi
}

# Logging helper
log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*" >> "$LOG_FILE"
}

log "========== Song Change Detected =========="
log "File: ${FILE:-unknown}"
log "Artist: ${ARTIST:-unknown}"
log "Title: ${TITLE:-unknown}"

# State management: skip if same song
LAST_FILE=$(cat "$TMP_DIR/last_song_file" 2>/dev/null || echo "")
if [ "$FILE" = "$LAST_FILE" ]; then
    log "Same song as previous, skipping theme generation"
    exit 0
fi

# Update state file
echo "$FILE" > "$TMP_DIR/last_song_file"

# Extract album art
log "Extracting album art..."
if ! rmpc albumart --output "$TMP_DIR/current_cover" >> "$LOG_FILE" 2>&1; then
    log "ERROR: Album art extraction failed"
    exit 0  # Exit silently, don't disrupt playback
fi

# Verify album art file exists and is non-empty
if [ ! -s "$TMP_DIR/current_cover" ]; then
    log "ERROR: Album art file is empty or does not exist"
    exit 0
fi

log "Album art extracted successfully"

# Detect image format and add proper extension
# The image crate needs file extensions for format detection
COVER_PATH="$TMP_DIR/current_cover"
MIME_TYPE=$(file --mime-type -b "$COVER_PATH" 2>/dev/null || echo "unknown")
case "$MIME_TYPE" in
    image/jpeg)
        COVER_WITH_EXT="$TMP_DIR/current_cover.jpg"
        ;;
    image/png)
        COVER_WITH_EXT="$TMP_DIR/current_cover.png"
        ;;
    image/webp)
        COVER_WITH_EXT="$TMP_DIR/current_cover.webp"
        ;;
    *)
        # Default to .jpg if format unknown
        log "WARNING: Unknown image format '$MIME_TYPE', defaulting to .jpg"
        COVER_WITH_EXT="$TMP_DIR/current_cover.jpg"
        ;;
esac

# Copy to path with extension if different
if [ "$COVER_PATH" != "$COVER_WITH_EXT" ]; then
    cp "$COVER_PATH" "$COVER_WITH_EXT"
fi

# Short-circuit: skip generation if album art is unchanged
if [ "${RMPC_THEME_FORCE:-0}" != "1" ] && [ "${RMPC_THEME_FORCE:-0}" != "true" ]; then
    CUR_HASH=$(sha256_of "$COVER_WITH_EXT")
    if [ -n "$CUR_HASH" ]; then
        PREV_HASH=$(cat "$HASH_FILE" 2>/dev/null || echo "")
        if [ "$CUR_HASH" = "$PREV_HASH" ]; then
            # Increment skip counter for observability
            SKIPS=$(cat "$SKIP_COUNT_FILE" 2>/dev/null || echo 0)
            SKIPS=$((SKIPS + 1))
            echo "$SKIPS" > "$SKIP_COUNT_FILE" || true
            log "Album art hash unchanged; skipping theme generation (skip_count=$SKIPS)"
            # Still update last_song_file to avoid repeated work on next event
            echo "$FILE" > "$TMP_DIR/last_song_file"
            exit 0
        fi
        echo "$CUR_HASH" > "$HASH_FILE" || true
    else
        log "WARNING: No sha256 tool found; proceeding without hash short-circuit"
    fi
else
    log "Force flag set (RMPC_THEME_FORCE); bypassing hash short-circuit"
fi

# Generate theme
log "Generating theme (format: $MIME_TYPE)..."
GENERATOR_CMD=(
    "$BINARY_PATH"
    --image "$COVER_WITH_EXT"
    --k 30
    --space CIELAB
    --theme-output "$THEME_DIR/current-song.ron"
)

if [ ${#SCROLLBAR_ARGS[@]} -gt 0 ]; then
    GENERATOR_CMD+=("${SCROLLBAR_ARGS[@]}")
fi

if ! "${GENERATOR_CMD[@]}" >> "$LOG_FILE" 2>&1; then
    log "ERROR: Theme generation failed"
    exit 0
fi

log "Theme generated successfully: $THEME_DIR/current-song.ron"

# Optional: Set terminal default background (OSC 11) to match theme background
case "${RMPC_THEME_SET_TERM_BG:-0}" in
    1|true|TRUE|True|yes|YES)
        BG_COLOR=$(grep -m1 'background_color:' "$THEME_DIR/current-song.ron" | sed -E 's/.*"(#[0-9a-fA-F]{6,8})".*/\1/')
        if [ -n "$BG_COLOR" ]; then
            if [ -w /dev/tty ]; then
                printf "\e]11;%s\e\\" "$BG_COLOR" > /dev/tty 2>>"$LOG_FILE" || true
                log "Set terminal default background via OSC 11 to $BG_COLOR"
            else
                log "WARNING: /dev/tty not writable; skipped OSC 11 background update"
            fi
        else
            log "WARNING: Could not parse background_color from theme; skipped OSC 11 update"
        fi
        ;;
esac

# Optional: Send status notification to rmpc
if [ -n "${PID:-}" ]; then
    rmpc remote --pid "$PID" status "Theme updated: ${ARTIST:-Unknown} - ${TITLE:-Unknown}" --level info >> "$LOG_FILE" 2>&1 || true
fi

log "========== Theme Update Complete ==========
"
