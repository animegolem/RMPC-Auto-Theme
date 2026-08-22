#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
OWNER_REPO="${OWNER_REPO:?set OWNER_REPO to the canonical checkout}"
CHANNEL_DIR="$OWNER_REPO/.relay"
MODE="${1:---once}"

case "$MODE" in
  --once|--wait) ;;
  *) echo "usage: $0 [--once|--wait]" >&2; exit 2 ;;
esac

snapshot="$(mktemp)"
candidate="$(mktemp)"
quiet="$(mktemp)"
trap 'rm -f "$snapshot" "$candidate" "$quiet"' EXIT

fingerprint() {
  {
    find "$CHANNEL_DIR/inbox" "$CHANNEL_DIR/outbox" \
      -type f -name '*.md' ! -name '*.tmp' -print 2>/dev/null
    for extra in "$CHANNEL_DIR/triage-report.md" \
                 "$CHANNEL_DIR/PROCESS-LAB.md" \
                 "$CHANNEL_DIR/ISSUES.md" \
                 "$CHANNEL_DIR/OBSERVATIONS.md"; do
      if [[ -f "$extra" ]]; then
        printf '%s\n' "$extra"
      fi
    done
  } | LC_ALL=C sort | while IFS= read -r path; do
    [[ -n "$path" ]] || continue
    rel="${path#"$CHANNEL_DIR"/}"
    digest="$(shasum -a 256 "$path" | awk '{print $1}')"
    printf '%s\t%s\n' "$rel" "$digest"
  done
}

stable_sample() {
  while true; do
    fingerprint > "$candidate"
    sleep 1
    fingerprint > "$quiet"
    if cmp -s "$candidate" "$quiet"; then
      cp "$quiet" "$snapshot"
      return
    fi
  done
}

sync_and_report() {
  "$SCRIPT_DIR/channel-carrier.sh" sync
  digest="$(shasum -a 256 "$snapshot" | awk '{print $1}')"
  echo "channel fingerprint: $digest"
  if [[ -s "$snapshot" ]]; then
    cat "$snapshot"
  else
    echo "(no live channel files)"
  fi
  "$SCRIPT_DIR/channel-carrier.sh" status || true
}

stable_sample
if [[ "$MODE" == "--once" ]]; then
  sync_and_report
  exit 0
fi

baseline="$(shasum -a 256 "$snapshot" | awk '{print $1}')"
while true; do
  sleep 2
  fingerprint > "$candidate"
  current="$(shasum -a 256 "$candidate" | awk '{print $1}')"
  [[ "$current" == "$baseline" ]] && continue

  sleep 1
  fingerprint > "$quiet"
  quiet_digest="$(shasum -a 256 "$quiet" | awk '{print $1}')"
  if [[ "$current" == "$quiet_digest" ]]; then
    cp "$quiet" "$snapshot"
    sync_and_report
    exit 0
  fi

  cp "$quiet" "$snapshot"
  baseline="$quiet_digest"
done
