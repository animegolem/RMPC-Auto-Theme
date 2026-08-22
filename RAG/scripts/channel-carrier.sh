#!/usr/bin/env bash
# Carrier wrapper: fixes channel + state dir for this repo.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
OWNER_REPO="${RMPC_AUTO_THEME_OWNER_REPO:-/Users/golem/git/RMPC-Auto-Theme}"
[[ $# -ge 1 ]] || { echo "usage: $0 <carrier-command> [arguments]" >&2; exit 2; }
cmd="$1"; shift
export CARRIER_CHANNEL=".relay"
export CARRIER_STATE_DIR="$OWNER_REPO/.relay/state"
exec python3 "$SCRIPT_DIR/carrier.py" "$cmd" --repo "$OWNER_REPO" "$@"
