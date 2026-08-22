#!/usr/bin/env bash
# relay-doorbell.sh — launchd WatchPaths handler: when new mail lands in
# the channel, ring every live Claude Code seat whose session name starts
# with SEAT_PREFIX via the documented cross-session path (headless
# `claude -p` child → ListAgents → SendMessage). The ear that survives a
# session death: launchd-run, fate-independent.
#
# This is a HINT layer. A failed ring costs nothing — the carrier still
# holds the letter (truth) and the channel watchdog notices aging unacked
# mail (backstop). Session sockets ring; they never store.
#
# Parameters (env, with defaults):
#   SEAT_PREFIX   session-name prefix to ring (e.g. "mediaplayer")   [required]
#   STATE_DIR     where rings.log / last-ring live                    [required]
#   CLAUDE_BIN    absolute path to claude (launchd PATH lacks ~/.local/bin)
#   DEBOUNCE_MIN  minutes between rings                               [3]
#   $1            the message to deliver
set -uo pipefail
MSG="${1:-relay doorbell: new mail in the channel inbox — scan when you can.}"
: "${SEAT_PREFIX:?set SEAT_PREFIX}"; : "${STATE_DIR:?set STATE_DIR}"
CLAUDE_BIN="${CLAUDE_BIN:-$HOME/.local/bin/claude}"
DEBOUNCE_MIN="${DEBOUNCE_MIN:-3}"
LOG="$STATE_DIR/rings.log"
mkdir -p "$STATE_DIR"
now=$(date +%s); last=$(cat "$STATE_DIR/last-ring" 2>/dev/null || echo 0)
if (( now - last < DEBOUNCE_MIN * 60 )); then echo "$(date -u +%FT%TZ) debounced" >> "$LOG"; exit 0; fi
echo "$now" > "$STATE_DIR/last-ring"
if [[ ! -x "$CLAUDE_BIN" ]]; then echo "$(date -u +%FT%TZ) ring FAILED → missing claude binary: $CLAUDE_BIN" >> "$LOG"; exit 1; fi
out=$("$CLAUDE_BIN" -p --model haiku \
  "Use ListAgents. For EVERY peer session whose name starts with '${SEAT_PREFIX}' (there may be more than one; send to each, do not pick one), use SendMessage to send exactly: '${MSG}' Then reply DONE. If no such peer exists, reply NO-PEER and do nothing else." \
  2>&1 | tail -1); status=$?
if (( status != 0 )); then echo "$(date -u +%FT%TZ) ring FAILED (exit $status) → ${out}" >> "$LOG"; exit 1; fi
echo "$(date -u +%FT%TZ) ring → ${out}" >> "$LOG"
