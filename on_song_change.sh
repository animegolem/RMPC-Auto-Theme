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
#

set -euo pipefail

# Configuration
TMP_DIR="/tmp/rmpc"
THEME_DIR="$HOME/.config/rmpc/themes"
LOG_FILE="$HOME/.config/rmpc/theme-switcher.log"
BINARY_PATH="${RMPC_THEME_GEN_PATH:-rmpc-theme-gen}"

# Ensure directories exist
mkdir -p "$TMP_DIR" "$THEME_DIR"

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

# Generate theme
log "Generating theme (format: $MIME_TYPE)..."
if ! "$BINARY_PATH" \
    --image "$COVER_WITH_EXT" \
    --k 8 \
    --space CIELAB \
    --theme-output "$THEME_DIR/current-song.ron" \
    >> "$LOG_FILE" 2>&1; then
    log "ERROR: Theme generation failed"
    exit 0
fi

log "Theme generated successfully: $THEME_DIR/current-song.ron"

# Optional: Send status notification to rmpc
if [ -n "${PID:-}" ]; then
    rmpc remote --pid "$PID" status "Theme updated: ${ARTIST:-Unknown} - ${TITLE:-Unknown}" --level info >> "$LOG_FILE" 2>&1 || true
fi

log "========== Theme Update Complete ==========
"