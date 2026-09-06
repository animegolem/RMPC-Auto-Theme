#!/bin/sh
# Thin rmpc on_song_change adapter. Every path deliberately exits zero.
umask 077
case ${HOME:-} in /*) ;; *) exit 0 ;; esac
cfg_root=${XDG_CONFIG_HOME:-"$HOME/.config"}
state_dir=${RMPC_AUTO_THEME_STATE:-"$cfg_root/rmpc-auto-theme"}
theme=${RMPC_AUTO_THEME_OUTPUT:-"$cfg_root/rmpc/themes/auto-theme.ron"}
gen=${RMPC_THEME_GEN:-"$HOME/.local/bin/rmpc-theme-gen"}
rmpc=${RMPC_BIN:-rmpc}
log_file=${RMPC_AUTO_THEME_LOG:-"$state_dir/hook.log"}
tty=${RMPC_AUTO_THEME_TTY:-/dev/tty}
mkdir -p "$state_dir" "$(dirname "$theme")" 2>/dev/null || exit 0
if [ -f "$log_file" ] && [ "$(wc -c <"$log_file" 2>/dev/null)" -gt 262144 ]; then
  tail -c 131072 "$log_file" >"$log_file.tmp" 2>/dev/null && mv "$log_file.tmp" "$log_file"
fi
log() { printf '%s %s\n' "$(date +%s)" "$*" >>"$log_file" 2>/dev/null; }
token="$$:${FILE:-}:${PID:-}:$(date +%s)"
printf '%s\n' "$token" >"$state_dir/event.tmp.$$" 2>/dev/null || exit 0
mv "$state_dir/event.tmp.$$" "$state_dir/latest-event" 2>/dev/null || exit 0
latest() { [ "$(cat "$state_dir/latest-event" 2>/dev/null)" = "$token" ]; }
art=$(mktemp "$state_dir/art.XXXXXX") || exit 0
out=$(mktemp "$(dirname "$theme")/.auto-theme.XXXXXX") || { rm -f "$art"; exit 0; }
trap 'rm -f "$art" "$out"' EXIT HUP INT TERM
"$rmpc" albumart -o "$art" >/dev/null 2>>"$log_file"
art_rc=$?
case $art_rc in
  0)
    hash=
    if command -v shasum >/dev/null 2>&1; then hash=$(shasum -a 256 "$art" | awk '{print $1}')
    elif command -v sha256sum >/dev/null 2>&1; then hash=$(sha256sum "$art" | awk '{print $1}'); fi
    [ -n "$hash" ] && [ "$(cat "$state_dir/art.sha256" 2>/dev/null)" = "$hash" ] && exit 0
    "$gen" <"$art" >"$out" 2>>"$log_file" || { log "generator failed"; exit 0; }
    ;;
  2)
    rm -f "$state_dir/art.sha256"
    "$gen" --print-base-theme >"$out" 2>>"$log_file" || { log "base reset failed"; exit 0; }
    hash=
    ;;
  3) exit 0 ;;
  *) log "albumart failed: exit $art_rc"; exit 0 ;;
esac
latest || exit 0
mv "$out" "$theme" 2>>"$log_file" || { log "theme write failed"; exit 0; }
latest || exit 0
"$rmpc" remote --pid "${PID:-}" set theme - <"$theme" >>"$log_file" 2>&1 || { log "IPC apply failed"; exit 0; }
if [ -n "${hash:-}" ]; then
  printf '%s\n' "$hash" >"$state_dir/hash.tmp.$$" && mv "$state_dir/hash.tmp.$$" "$state_dir/art.sha256"
fi
if [ "$("$gen" --print-config osc11 2>/dev/null)" = true ] && [ -w "$tty" ]; then
  bg=$(sed -n 's/^[[:space:](]*background_color:[[:space:]]*"\(#[0-9a-fA-F]\{6\}\)".*/\1/p' "$theme" | head -1)
  [ -n "$bg" ] && printf '\033]11;%s\007' "$bg" >"$tty" 2>/dev/null
fi
exit 0
