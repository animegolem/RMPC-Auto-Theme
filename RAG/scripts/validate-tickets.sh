#!/usr/bin/env bash
# Validate AI-EPIC / AI-IMP / AI-LOG files against the template
# contracts. Loud gate for what generate-index.sh's
# normalize_frontmatter silently patches.
#
# Usage:
#   <ticket-root>/scripts/validate-tickets.sh [FILE...]      # given files
#   <ticket-root>/scripts/validate-tickets.sh                # whole tree
#   <ticket-root>/scripts/validate-tickets.sh --changed [REF]
#       # only files changed vs REF (default origin/main) — the
#       # mode sub-agents run before submitting.
#
# Output: path: [ERROR|WARN] message, then a summary with COUNTS.
# Exit codes (cli-rag lineage): 0 clean, 2 validation errors.

set -o pipefail

TICKETS_DIR="$(cd "$(dirname "$0")/.." && pwd)"
ROOT_DIR="$(cd "$TICKETS_DIR/.." && pwd)"
LEGAL_STATUS="backlog planned in-progress completed cancelled deferred"
ERRORS=0
WARNS=0
CHECKED=0

err()  { echo "$1: ERROR $2"; ERRORS=$((ERRORS + 1)); }
warn() { echo "$1: WARN  $2"; WARNS=$((WARNS + 1)); }

# ---------------------------------------------------------------------------
# collect_files: resolve arguments into the list to validate
# ---------------------------------------------------------------------------
collect_files() {
  if [[ "${1:-}" == "--changed" ]]; then
    local ref="${2:-origin/main}"
    # Fail loudly if the ref doesn't resolve — isolated clones are
    # exactly where origin/main may not be fetched yet, and git's own
    # error scrolls past. (Message to stderr so the file list on
    # stdout stays clean.)
    if ! git -C "$ROOT_DIR" rev-parse --verify --quiet "$ref" >/dev/null; then
      echo "validate-tickets: ref '$ref' does not resolve — fetch it or pass one that does (--changed [REF])" >&2
      exit 2
    fi
    # Derive pathspecs from where this script actually lives, so a
    # renamed ticket root keeps working; and union in untracked
    # files — `git diff` alone misses brand-new tickets, which are
    # exactly the ones most likely to carry unfilled templates.
    local tickets_rel="${TICKETS_DIR#"$ROOT_DIR"/}"
    {
      git -C "$ROOT_DIR" diff --name-only --diff-filter=ACMR "$ref"... -- \
          "$tickets_rel/AI-EPIC/*.md" "$tickets_rel/AI-IMP/*.md" "$tickets_rel/AI-LOG/*.md"
      git -C "$ROOT_DIR" ls-files --others --exclude-standard -- \
          "$tickets_rel/AI-EPIC/*.md" "$tickets_rel/AI-IMP/*.md" "$tickets_rel/AI-LOG/*.md"
    } | sort -u | while read -r rel; do echo "$ROOT_DIR/$rel"; done
  elif [[ $# -gt 0 ]]; then
    # Normalize to absolute paths — the duplicate-node_id check
    # excludes "self" by exact path match, so a relative argument
    # would report a file as its own duplicate.
    local f
    for f in "$@"; do
      if [[ "$f" = /* ]]; then printf '%s\n' "$f"
      else printf '%s\n' "$(cd "$(dirname "$f")" && pwd)/$(basename "$f")"
      fi
    done
  else
    find "$TICKETS_DIR/AI-EPIC" "$TICKETS_DIR/AI-IMP" "$TICKETS_DIR/AI-LOG" \
         -maxdepth 1 -name '*.md' 2>/dev/null | sort
  fi
}

# ---------------------------------------------------------------------------
# fm_field: extract a frontmatter field (first block only)
# ---------------------------------------------------------------------------
fm_field() {
  awk -v field="$2" '
    NR == 1 && $0 != "---" { exit }
    /^---$/ { fm++; if (fm == 2) exit; next }
    fm == 1 && $0 ~ "^" field ":" {
      sub("^" field ":[[:space:]]*", "")
      gsub(/[[:space:]]*$/, "")
      gsub(/^["'\'']|["'\'']$/, "")
      print
      exit
    }
  ' "$1"
}

# ---------------------------------------------------------------------------
# validate_file
# ---------------------------------------------------------------------------
ALL_IDS_FILE=$(mktemp)
trap 'rm -f "$ALL_IDS_FILE"' EXIT
grep -H '^node_id:' "$TICKETS_DIR"/AI-EPIC/*.md "$TICKETS_DIR"/AI-IMP/*.md 2>/dev/null \
  | sed 's/:node_id:[[:space:]]*/\t/' | awk -F'\t' '$2 != ""' > "$ALL_IDS_FILE"

ref_resolves() {
  local ref="$1"
  grep -q "	$ref\$" "$ALL_IDS_FILE" && return 0
  compgen -G "$TICKETS_DIR/AI-IMP/${ref}*.md" > /dev/null 2>&1 && return 0
  compgen -G "$TICKETS_DIR/AI-EPIC/${ref}*.md" > /dev/null 2>&1 && return 0
  return 1
}

validate_file() {
  local file="$1"
  local base kind
  base=$(basename "$file" .md)
  CHECKED=$((CHECKED + 1))

  case "$base" in
    AI-EPIC-*) kind=epic ;;
    AI-IMP-*)  kind=imp ;;
    *LOG*)     kind=log ;;
    *) warn "$file" "unrecognized filename shape (expected AI-EPIC-*, AI-IMP-*, or *LOG*)"; return ;;
  esac

  # -- frontmatter must be the FIRST thing in the file ----------------------
  if [[ "$(head -1 "$file")" != "---" ]]; then
    err "$file" "frontmatter is not first in the file (line 1 must be ---)"
    return
  fi
  if [[ $(grep -c '^---$' "$file") -lt 2 ]]; then
    err "$file" "frontmatter block never closes (second --- missing)"
    return
  fi

  # -- deprecated field names: loud, not silent -----------------------------
  local fm
  fm=$(awk '/^---$/{fm++; next} fm==1{print} fm==2{exit}' "$file")
  if [[ "$kind" != "log" ]]; then
    grep -q '^created_date:' <<<"$fm" \
      && err "$file" "deprecated field 'created_date' (canonical: date_created)"
  else
    # Logs shipped with created_date historically; the template is
    # unified on date_created now. WARN (not ERROR) so the existing
    # corpus keeps passing while new logs converge.
    grep -q '^created_date:' <<<"$fm" \
      && warn "$file" "legacy field 'created_date' (template now uses date_created)"
  fi
  grep -q '^kanban-status:' <<<"$fm" \
    && err "$file" "deprecated field 'kanban-status' (canonical: kanban_status)"
  grep -q '^close_date:' <<<"$fm" \
    && err "$file" "deprecated field 'close_date' (canonical: date_completed)"

  # -- node_id present and agreeing with the filename -----------------------
  local node_id
  node_id=$(fm_field "$file" node_id)
  if [[ -z "$node_id" ]]; then
    err "$file" "missing node_id"
  elif [[ "$kind" != "log" && "$base" != "$node_id"* ]]; then
    if [[ "$node_id" == *XXX* ]]; then
      warn "$file" "node_id '$node_id' is unassigned (XXX) — lead assigns at merge"
    else
      err "$file" "node_id '$node_id' does not prefix filename '$base'"
    fi
  fi

  # -- duplicate node_id across the tree (masterplan-validate lineage) ------
  if [[ -n "$node_id" && "$node_id" != *XXX* && "$kind" != "log" ]]; then
    local dupes
    dupes=$(awk -F'\t' -v id="$node_id" -v self="$file" \
      '$2 == id && $1 != self { print $1 }' "$ALL_IDS_FILE")
    [[ -n "$dupes" ]] \
      && err "$file" "duplicate node_id '$node_id' (also in $(basename "$(head -1 <<<"$dupes")"))"
  fi

  # -- depends_on references must resolve (masterplan-validate lineage) -----
  if [[ "$kind" != "log" ]]; then
    local deps ref
    deps=$(awk '
      /^---$/ { fm++; if (fm == 2) exit; next }
      fm == 1 && /^depends_on:/ {
        line = $0; sub(/^depends_on:[[:space:]]*/, "", line)
        if (line ~ /^\[/) { gsub(/[][]/, "", line); print line; exit }
        if (line != "") { print line; exit }
        indep = 1; next
      }
      fm == 1 && indep && /^[[:space:]]+-[[:space:]]/ {
        sub(/^[[:space:]]+-[[:space:]]*/, ""); print; next
      }
      fm == 1 && indep && /^[^[:space:]]/ { exit }
    ' "$file" | tr ',' '\n' | sed 's/^[[:space:]]*//;s/[[:space:]]*$//;s/[][]//g' \
      | grep -v '^$' | grep -v '^{')
    for ref in $deps; do
      ref="${ref%\"}"; ref="${ref#\"}"
      [[ "$ref" =~ ^(AI-IMP|AI-EPIC|AI-ADR|ADR)- ]] || continue
      ref_resolves "$ref" \
        || err "$file" "depends_on '$ref' does not resolve to any ticket"
    done
  fi

  # -- status / date discipline (epics and imps) -----------------------------
  if [[ "$kind" != "log" ]]; then
    local status dc dd
    status=$(fm_field "$file" kanban_status)
    dc=$(fm_field "$file" date_created)
    dd=$(fm_field "$file" date_completed)
    if [[ -z "$status" ]]; then
      err "$file" "missing kanban_status"
    else
      # Exact-token match. grep -w treats '-' as a word boundary, so
      # 'progress' (or even 'in') would match inside 'in-progress'.
      local legal_hit="" s
      for s in $LEGAL_STATUS; do [[ "$status" == "$s" ]] && legal_hit=1; done
      [[ -n "$legal_hit" ]] \
        || err "$file" "illegal kanban_status '$status' (legal: $LEGAL_STATUS)"
    fi
    [[ -n "$dc" && ! "$dc" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}$ ]] \
      && err "$file" "date_created '$dc' is not YYYY-MM-DD"
    [[ -n "$dd" && ! "$dd" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}$ ]] \
      && err "$file" "date_completed '$dd' is not YYYY-MM-DD"
    [[ "$status" == "completed" && -z "$dd" ]] \
      && err "$file" "status completed but date_completed is empty"
    [[ -n "$dd" && "$status" != "completed" && "$status" != "cancelled" ]] \
      && warn "$file" "date_completed set but status is '$status'"
  fi

  # -- confidence_score sanity ----------------------------------------------
  local conf
  conf=$(fm_field "$file" confidence_score)
  if [[ -n "$conf" ]]; then
    awk -v c="$conf" 'BEGIN { exit !(c ~ /^[0-9.]+$/ && c >= 0 && c <= 1) }' \
      || err "$file" "confidence_score '$conf' not in 0.0-1.0"
  fi

  # -- body contracts ---------------------------------------------------------
  if [[ "$kind" == "imp" ]]; then
    grep -q '<CRITICAL_RULE>' "$file" \
      || err "$file" "CRITICAL_RULE removed (template: the checklist gate must remain)"
    grep -q '^###* *Issues Encountered' "$file" \
      || err "$file" "Issues Encountered section missing"
  fi

  # -- unfilled template placeholders ----------------------------------------
  grep -q '{LOC|' "$file" \
    && err "$file" "unfilled {LOC|N} placeholder left in body"
  # Any double-brace content is template text — digits, slashes,
  # and spaces included ({{TICKET-A}}, {{path/to/...}}, {{YYYY-MM-DD}}).
  grep -qE '\{\{[^}]+\}\}' "$file" \
    && err "$file" "unfilled {{placeholder}} left in body"
  # The AI-IMP frontmatter ships single-brace placeholders
  # ({more tags as needed}, {0.0-1.0}, {YYYY-MM-DD}, {Legal
  # Values: ...}). Frontmatter values are never legitimately
  # brace-wrapped, so scope the single-brace check there — the
  # body stays free for code snippets.
  awk 'NR == 1 && $0 != "---" { exit }
       /^---$/ { fm++; if (fm == 2) exit; next }
       fm == 1' "$file" \
    | grep -qE '(:[[:space:]]*|^[[:space:]]*-[[:space:]]*)\{' \
    && err "$file" "unfilled {placeholder} left in frontmatter"
}

# ---------------------------------------------------------------------------
# main
# ---------------------------------------------------------------------------
# collect_files runs in a subshell here; its exit 2 (bad --changed
# ref) must be propagated or the gate would fail open.
FILES=$(collect_files "$@") || exit 2
if [[ -z "$FILES" ]]; then
  echo "validate-tickets: nothing to validate"
  exit 0
fi
while IFS= read -r f; do
  [[ -f "$f" ]] && validate_file "$f"
done <<<"$FILES"

echo "----"
echo "validate-tickets: checked $CHECKED file(s): $ERRORS error(s), $WARNS warning(s)"
[[ $ERRORS -gt 0 ]] && exit 2
exit 0
