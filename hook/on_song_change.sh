#!/bin/sh
# rmpc on_song_change adapter. Playback must never depend on the theme hook.
umask 077
case ${HOME:-} in /*) ;; *) exit 0 ;; esac
cfg_root=${XDG_CONFIG_HOME:-"$HOME/.config"}
state_dir=${RMPC_AUTO_THEME_STATE:-"$cfg_root/rmpc-auto-theme"}
theme=${RMPC_AUTO_THEME_OUTPUT:-"$cfg_root/rmpc/themes/auto-theme.ron"}
# Generator lookup: an explicit override is honored as given; otherwise the
# user install, then PATH (package installs put the binary in /usr/bin).
if [ -n "${RMPC_THEME_GEN:-}" ]; then
  gen=$RMPC_THEME_GEN
else
  gen="$HOME/.local/bin/rmpc-theme-gen"
  [ -x "$gen" ] || gen=$(command -v rmpc-theme-gen 2>/dev/null) || exit 0
fi
rmpc=${RMPC_BIN:-rmpc}
log_file=${RMPC_AUTO_THEME_LOG:-"$state_dir/hook.log"}
tty=${RMPC_AUTO_THEME_TTY:-/dev/tty}
mkdir -p "$state_dir" "$(dirname "$theme")" 2>/dev/null || exit 0

# Keep one log buffer per invocation. Flush while holding the extraction lock,
# so two rmpc instances never interleave unlabelled stderr fragments.
inv_log=${RMPC_INVOCATION_LOG:-}
if [ -z "$inv_log" ]; then
  inv_log=$(mktemp "$state_dir/invocation.XXXXXX") || exit 0
  export RMPC_INVOCATION_LOG="$inv_log"
fi
note() { printf '%s\n' "$*" >>"$inv_log" 2>/dev/null; }
flush_log() {
  [ -f "$inv_log" ] || return
  if [ -f "$log_file" ] && [ "$(wc -c <"$log_file" 2>/dev/null)" -gt 262144 ]; then
    tail -c 131072 "$log_file" >"$log_file.tmp.$$" 2>/dev/null && mv "$log_file.tmp.$$" "$log_file"
  fi
  while IFS= read -r line || [ -n "$line" ]; do
    printf '%s [hook=%s rmpc=%s] %s\n' "$(date +%s)" "$$" "${PID:-unknown}" "$line" >>"$log_file" 2>/dev/null
  done <"$inv_log"
  rm -f "$inv_log"
}

# Admission precedes the blocking extraction lock. The producer owns a short
# lock around token/sequence assignment and around final artifact rename.
if [ "${1:-}" != --under-extraction-lock ]; then
  admission=$("$gen" --admit-event --file "${FILE:-}" 2>>"$inv_log") || {
    note 'event admission failed'; flush_log; exit 0;
  }
  token=${admission%% *}
  sequence=${admission#* }
  case $admission in
    *' '*) ;;
    *) note 'event admission returned no sequence'; flush_log; exit 0 ;;
  esac
  case $sequence in
    ''|*[!0-9]*) note 'event admission returned invalid sequence'; flush_log; exit 0 ;;
  esac
  [ -n "$token" ] || { note 'event admission returned no token'; flush_log; exit 0; }
  export RMPC_EVENT_TOKEN="$token" RMPC_EVENT_SEQUENCE="$sequence"
  if command -v lockf >/dev/null 2>&1; then
    lockf "$state_dir/extraction.lock" /bin/sh "$0" --under-extraction-lock || {
      note 'extraction lock failed'; flush_log;
    }
  elif command -v flock >/dev/null 2>&1; then
    flock "$state_dir/extraction.lock" /bin/sh "$0" --under-extraction-lock || {
      note 'extraction lock failed'; flush_log;
    }
  else
    note 'neither lockf nor flock is available; cannot safely extract'; flush_log; exit 0
  fi
  exit 0
fi

out=$(mktemp "$(dirname "$theme")/.auto-theme.XXXXXX") || {
  note 'cannot create staged theme'; flush_log; exit 0;
}
cleanup() { rm -f "$out"; flush_log; }
trap cleanup EXIT
trap 'exit 0' HUP INT TERM

# The publisher fetches art by the admitted MPD URI, caches analysis, emits a
# fresh envelope even for same-art events, and suppresses superseded results.
result=$("$gen" --publish-event --file "${FILE:-}" \
  --token "$RMPC_EVENT_TOKEN" --sequence "$RMPC_EVENT_SEQUENCE" \
  --theme-output "$out" 2>>"$inv_log") || {
  note 'publisher invocation failed'; exit 0;
}
case $result in
  'ok '*) hash=${result#ok } ;;
  no-art) hash= ;;
  failed|superseded) exit 0 ;;
  *) note "publisher returned unexpected result: $result"; exit 0 ;;
esac
[ -s "$out" ] || { note 'publisher returned no staged theme'; exit 0; }

# A cache hit still publishes a new envelope, but does not re-apply identical
# art. The token-checked commit handles the race with later admissions.
if [ -n "$hash" ] && [ -s "$theme" ] && [ "$(cat "$state_dir/art.sha256" 2>/dev/null)" = "$hash" ] && cmp -s "$out" "$theme"; then
  note 'same-art envelope published; theme application skipped'
  exit 0
fi
commit=$("$gen" --commit-theme --file "${FILE:-}" --token "$RMPC_EVENT_TOKEN" \
  --sequence "$RMPC_EVENT_SEQUENCE" --staged-theme "$out" --theme "$theme" \
  2>>"$inv_log") || { note 'theme commit failed'; exit 0; }
case $commit in
  committed) ;;
  superseded) exit 0 ;;
  *) note "theme commit returned unexpected result: $commit"; exit 0 ;;
esac
"$gen" --is-current --file "${FILE:-}" --token "$RMPC_EVENT_TOKEN" \
  --sequence "$RMPC_EVENT_SEQUENCE" 2>>"$inv_log" || exit 0
"$rmpc" remote --pid "${PID:-}" set theme - <"$theme" >>"$inv_log" 2>&1 || {
  note 'IPC apply failed'; exit 0;
}
if "$gen" --is-current --file "${FILE:-}" --token "$RMPC_EVENT_TOKEN" \
  --sequence "$RMPC_EVENT_SEQUENCE" 2>>"$inv_log"; then
  if [ -n "$hash" ]; then
    printf '%s\n' "$hash" >"$state_dir/hash.tmp.$$" && mv "$state_dir/hash.tmp.$$" "$state_dir/art.sha256"
  else
    rm -f "$state_dir/art.sha256"
  fi
fi
if [ "$("$gen" --print-config osc11 2>>"$inv_log")" = true ] && [ -w "$tty" ]; then
  bg=$(sed -n 's/^[[:space:](]*background_color:[[:space:]]*"\(#[0-9a-fA-F]\{6\}\)".*/\1/p' "$theme" | head -1)
  [ -n "$bg" ] && printf '\033]11;%s\007' "$bg" >"$tty" 2>/dev/null
fi
exit 0
