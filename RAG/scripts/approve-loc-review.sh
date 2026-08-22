#!/usr/bin/env bash
# Record one or more tracked files as manually reviewed for the Size Watch.
#
# Usage:
#   <ticket-root>/scripts/approve-loc-review.sh [--review-ref REF] [--note NOTE] FILE...
#
# The manifest records the exact reviewed content blob. generate-index.sh can
# therefore distinguish an unchanged file from one that has grown or changed
# since review without relying on checkout-sensitive filesystem mtimes.

set -euo pipefail

TICKETS_DIR="$(cd "$(dirname "$0")/.." && pwd)"
ROOT_DIR="$(cd "$TICKETS_DIR/.." && pwd)"
MANIFEST="$TICKETS_DIR/.loc-reviews.tsv"
REVIEW_REF=""
REVIEW_NOTE=""

usage() {
  cat <<'EOF'
Usage: <ticket-root>/scripts/approve-loc-review.sh [--review-ref REF] [--note NOTE] FILE...

Record the current content and LOC of tracked repository files as manually
reviewed for the generated INDEX.md's Size Watch. Paths may be absolute, relative to the
current directory, or relative to the repository root. Existing records are
replaced; other records are kept.

Options:
  --review-ref REF  Audit, ticket, or note that explains the approval
  --note NOTE       One-line conclusion from the manual review
  -h, --help        Show this help
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --review-ref)
      [[ $# -ge 2 ]] || { echo "approve-loc-review: --review-ref needs a value" >&2; exit 2; }
      REVIEW_REF="$2"
      shift 2
      ;;
    --note)
      [[ $# -ge 2 ]] || { echo "approve-loc-review: --note needs a value" >&2; exit 2; }
      REVIEW_NOTE="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    --)
      shift
      break
      ;;
    -*)
      echo "approve-loc-review: unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
    *)
      break
      ;;
  esac
done

[[ $# -gt 0 ]] || { usage >&2; exit 2; }

for value in "$REVIEW_REF" "$REVIEW_NOTE" "$@"; do
  if [[ "$value" == *$'\t'* || "$value" == *$'\n'* ]]; then
    echo "approve-loc-review: tabs and newlines are not supported: $value" >&2
    exit 2
  fi
done

tmp=$(mktemp)
updates=$(mktemp)
trap 'rm -f "$tmp" "$updates"' EXIT

reviewed_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)
reviewed_commit=$(git -C "$ROOT_DIR" rev-parse HEAD)
stored_ref=${REVIEW_REF:--}
stored_note=${REVIEW_NOTE:--}

for input in "$@"; do
  if [[ "$input" = /* ]]; then
    candidate="$input"
  elif [[ -e "$input" ]]; then
    candidate="$(cd "$(dirname "$input")" && pwd -P)/$(basename "$input")"
  else
    candidate="$ROOT_DIR/${input#./}"
  fi

  case "$candidate" in
    "$ROOT_DIR"/*) file=${candidate#"$ROOT_DIR"/} ;;
    *) echo "approve-loc-review: path is outside the repository: $input" >&2; exit 2 ;;
  esac

  # Reject unresolved root-relative traversal. A path that existed relative to
  # the current directory was canonicalized above.
  if [[ "$file" == ../* || "$file" == */../* || "$file" == */.. ]]; then
    echo "approve-loc-review: unresolved parent traversal: $input" >&2
    exit 2
  fi

  if ! git -C "$ROOT_DIR" ls-files --error-unmatch -- "$file" >/dev/null 2>&1; then
    echo "approve-loc-review: not a tracked file: $file" >&2
    exit 2
  fi
  if [[ ! -f "$ROOT_DIR/$file" ]]; then
    echo "approve-loc-review: not a regular file: $file" >&2
    exit 2
  fi

  reviewed_blob=$(git -C "$ROOT_DIR" hash-object -- "$file")
  reviewed_loc=$(wc -l < "$ROOT_DIR/$file" | tr -d ' ')
  printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
    "$file" "$reviewed_at" "$reviewed_commit" "$reviewed_blob" \
    "$reviewed_loc" "$stored_ref" "$stored_note" >> "$updates"
done

# Repeated input paths are harmless and produce one manifest record.
LC_ALL=C sort -t $'\t' -k1,1 -u "$updates" -o "$updates"

# Preserve comments and records not replaced by this invocation.
if [[ -f "$MANIFEST" ]]; then
  awk -F '\t' '
    NR == FNR { replacing[$1] = 1; next }
    /^#/ || NF == 0 { print; next }
    !($1 in replacing) { print }
  ' "$updates" "$MANIFEST" > "$tmp"
else
  printf '# path\treviewed_at\treviewed_commit\treviewed_blob\treviewed_loc\treview_ref\treview_note\n' > "$tmp"
fi

cat "$updates" >> "$tmp"
LC_ALL=C sort -t $'\t' -k1,1 "$tmp" -o "$tmp"
mv "$tmp" "$MANIFEST"

count=$(wc -l < "$updates" | tr -d ' ')
echo "[loc-review] Recorded $count file(s) in ${MANIFEST#"$ROOT_DIR"/} at $reviewed_at"
