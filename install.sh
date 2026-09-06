#!/bin/sh
set -eu

usage() { printf 'Usage: %s [--wire-hook]\n' "$0"; }
wire=false
case ${1:-} in "") ;; --wire-hook) wire=true ;; -h|--help) usage; exit 0 ;; *) usage >&2; exit 2 ;; esac
[ "$#" -le 1 ] || { usage >&2; exit 2; }
case ${HOME:-} in ""|/|.|..) printf 'install.sh: unsafe HOME\n' >&2; exit 1 ;; esac
case $HOME in /*) ;; *) printf 'install.sh: HOME must be absolute\n' >&2; exit 1 ;; esac
case $HOME in *\"*) printf 'install.sh: HOME may not contain a quote\n' >&2; exit 1 ;; esac

repo=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
prefix=${RMPC_AUTO_THEME_PREFIX:-"$HOME/.local"}
cfg_root=${XDG_CONFIG_HOME:-"$HOME/.config"}
app_cfg="$cfg_root/rmpc-auto-theme/config.toml"
live_cfg="$cfg_root/rmpc/config.ron"
theme_dir="$cfg_root/rmpc/themes"
theme_out="$theme_dir/auto-theme.ron"
bin="$prefix/bin/rmpc-theme-gen"
hook="$prefix/libexec/rmpc-auto-theme/on_song_change.sh"

if [ "${RMPC_AUTO_THEME_SKIP_BUILD:-0}" != 1 ]; then
  cargo build --release --manifest-path "$repo/Cargo.toml"
fi
source_bin=${RMPC_THEME_GEN_SOURCE:-"$repo/target/release/rmpc-theme-gen"}
[ -f "$source_bin" ] || { printf 'install.sh: missing binary %s\n' "$source_bin" >&2; exit 1; }
mkdir -p "$(dirname "$bin")" "$(dirname "$hook")" "$(dirname "$app_cfg")" "$theme_dir"
cp "$source_bin" "$bin"
cp "$repo/hook/on_song_change.sh" "$hook"
chmod 755 "$bin" "$hook"
if [ ! -e "$app_cfg" ]; then cp "$repo/fixtures/config.toml" "$app_cfg"; fi
if $wire && [ ! -f "$live_cfg" ]; then
  printf 'install.sh: missing %s\n' "$live_cfg" >&2
  exit 1
fi

base=$("$bin" --print-config base_theme)
if $wire && [ -z "$base" ]; then
  current=$(sed -n 's/^[[:space:]]*theme:[[:space:]]*"\([^"]*\)",.*/\1/p' "$live_cfg")
  [ "$(printf '%s\n' "$current" | wc -l)" -eq 1 ] || current=
  candidate="$theme_dir/$current.ron"
  if [ -n "$current" ] && [ -f "$candidate" ]; then
    base="$cfg_root/rmpc-auto-theme/base.ron"
    cp "$candidate" "$base"
  fi
fi
if [ -n "$base" ] && ! grep -Eq '^[[:space:]]*base_theme[[:space:]]*=' "$app_cfg"; then
  awk -v path="$base" 'BEGIN { print "base_theme = \"" path "\"" } { print }' "$app_cfg" >"$app_cfg.tmp"
  mv "$app_cfg.tmp" "$app_cfg"
fi
if $wire && [ -n "$base" ]; then
  cp "$base" "$theme_out"
elif [ ! -e "$theme_out" ]; then
  "$bin" --print-base-theme >"$theme_out.tmp"
  mv "$theme_out.tmp" "$theme_out"
fi

manual() {
  printf 'Add these fields to %s:\n  theme: "auto-theme",\n  on_song_change: ["%s"],\n' "$live_cfg" "$hook"
}
$wire || { manual; exit 0; }
hook_none=$(grep -Ec '^[[:space:]]*on_song_change:[[:space:]]*None,' "$live_cfg" || true)
hook_ours=$(grep -Fc "on_song_change: [\"$hook\"]," "$live_cfg" || true)
theme_field=$(grep -Ec '^[[:space:]]*theme:[[:space:]]*"[^"]+",' "$live_cfg" || true)
theme_ours=$(grep -Ec '^[[:space:]]*theme:[[:space:]]*"auto-theme",' "$live_cfg" || true)
if [ "$hook_ours" -eq 1 ] && [ "$theme_ours" -eq 1 ]; then
  printf 'install.sh: hook already wired\n'
  exit 0
fi
[ "$hook_none" -eq 1 ] || { printf 'install.sh: refusing non-None or ambiguous on_song_change\n' >&2; exit 1; }
[ "$theme_field" -eq 1 ] || { printf 'install.sh: refusing missing or ambiguous theme field\n' >&2; exit 1; }
backup="$live_cfg.backup.$(date +%Y%m%d%H%M%S)"
[ ! -e "$backup" ] || backup="$backup.$$"
cp "$live_cfg" "$backup"
cp "$live_cfg" "$live_cfg.tmp"
awk -v hook="$hook" '
  /^[[:space:]]*theme:[[:space:]]*"[^"]+",/ { sub(/"[^"]+",/, "\"auto-theme\",") }
  /^[[:space:]]*on_song_change:[[:space:]]*None,/ { sub(/None,/, "[\"" hook "\"],") }
  { print }
' "$live_cfg" >"$live_cfg.tmp"
grep -Fq 'theme: "auto-theme",' "$live_cfg.tmp" || { printf 'install.sh: generated theme wiring failed\n' >&2; exit 1; }
grep -Fq "on_song_change: [\"$hook\"]," "$live_cfg.tmp" || { printf 'install.sh: generated hook wiring failed\n' >&2; exit 1; }
mv "$live_cfg.tmp" "$live_cfg"
printf 'install.sh: wired hook; backup %s\n' "$backup"
