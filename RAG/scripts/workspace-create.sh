#!/usr/bin/env bash
# workspace-create.sh — create an independent-clone workspace entry.
#
# Usage: workspace-create.sh <canonical-repo-root> <role>/<id>
#   e.g.: workspace-create.sh /path/to/repo code-lead/primary
#
# Deterministic, low-freedom recipe (2026-08-08 two-lead consultation):
# resolve and validate the canonical root and the target descendant,
# REFUSE an existing target, clone --no-hardlinks to a temporary
# sibling, verify the entry's git common-dir is independent, then
# rename into place. There is deliberately NO cleanup subcommand:
# workspace cleanup is Review Lead-only, one resolved descendant at a
# time, never the root, never a glob — and never `git clean -x` in the
# canonical root.
set -euo pipefail

die() { echo "workspace-create: ERROR $*" >&2; exit 2; }

[[ $# -eq 2 ]] || die "usage: workspace-create.sh <canonical-repo-root> <role>/<id>"

canonical="$(cd "$1" 2>/dev/null && pwd -P)" || die "canonical root does not resolve: $1"
[[ -d "$canonical/.git" ]] || die "not a repository root (no .git): $canonical"
case "$canonical" in
  */.workspaces/*) die "canonical root may not itself be a workspace entry: $canonical" ;;
esac

entry="$2"
case "$entry" in
  */*) ;;
  *) die "target must be <role>/<id>, got: $entry" ;;
esac
case "$entry" in
  *..*|/*|*/) die "target must be a relative <role>/<id> with no traversal: $entry" ;;
esac

target="$canonical/.workspaces/$entry"
[[ -e "$target" ]] && die "target exists; this tool never overwrites: $target"

grep -qxF '.workspaces/' "$canonical/.gitignore" 2>/dev/null \
  || die ".workspaces/ is not in $canonical/.gitignore — add it first (bootstrap step)"

mkdir -p "$(dirname "$target")"
tmp="$(dirname "$target")/.$(basename "$target").tmp.$$"

git clone --no-hardlinks "$canonical" "$tmp" >/dev/null 2>&1 \
  || { rm -rf "$tmp"; die "clone failed"; }

common_dir="$(git -C "$tmp" rev-parse --git-common-dir)"
case "$common_dir" in
  .git|"$tmp"/.git) ;;
  *) rm -rf "$tmp"; die "clone common-dir escapes the entry ($common_dir) — refusing" ;;
esac

mv "$tmp" "$target"
echo "workspace-create: OK $target"
echo "  branch: $(git -C "$target" branch --show-current)"
echo "  origin: $(git -C "$target" remote get-url origin)"
