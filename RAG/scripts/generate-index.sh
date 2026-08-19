#!/usr/bin/env bash
# Generate RAG/INDEX.md from AI-EPIC and AI-IMP front matter.
# Also normalizes field names and adds parent_epic backlinks.
#
# Usage: ./RAG/scripts/generate-index.sh

set -euo pipefail

RAG_DIR="$(cd "$(dirname "$0")/.." && pwd)"
INDEX_FILE="$RAG_DIR/INDEX.md"
TODAY=$(date +%Y-%m-%d)

# Use TAB as delimiter to avoid conflicts with | in wikilinks
TAB=$'\t'

# Temporary files for collecting data
EPICS_IN_PROGRESS=$(mktemp)
EPICS_PLANNED=$(mktemp)
EPICS_DEFERRED=$(mktemp)
EPICS_CANCELLED=$(mktemp)
EPICS_COMPLETED=$(mktemp)
IMPS_BY_EPIC=$(mktemp)
ORPHAN_IMPS=$(mktemp)
STATUS_MISMATCHES=$(mktemp)
LARGE_FILES=$(mktemp)
ILLEGAL_STATUS=$(mktemp)

trap 'rm -f "$EPICS_IN_PROGRESS" "$EPICS_PLANNED" "$EPICS_DEFERRED" "$EPICS_CANCELLED" "$EPICS_COMPLETED" "$IMPS_BY_EPIC" "$ORPHAN_IMPS" "$STATUS_MISMATCHES" "$LARGE_FILES" "$ILLEGAL_STATUS"' EXIT

# -----------------------------------------------------------------------------
# normalize_frontmatter: Fix field name variations in-place
# -----------------------------------------------------------------------------
normalize_frontmatter() {
  local file="$1"
  if grep -qE '^(kanban-status|close_date|created_date):' "$file" 2>/dev/null; then
    local tmp="${file}.tmp"
    sed -e 's/^kanban-status:/kanban_status:/' \
        -e 's/^close_date:/date_completed:/' \
        -e 's/^created_date:/date_created:/' "$file" > "$tmp" && mv "$tmp" "$file"
  fi
}

# -----------------------------------------------------------------------------
# extract_frontmatter_field: Get a field value from YAML front matter
# -----------------------------------------------------------------------------
extract_frontmatter_field() {
  local file="$1"
  local field="$2"

  awk -v field="$field" '
    /^---$/ { in_fm = !in_fm; next }
    in_fm && $0 ~ "^" field ":" {
      sub("^" field ":[[:space:]]*", "")
      gsub(/[[:space:]]*$/, "")
      gsub(/^["'\''"]|["'\''"]$/, "")
      print
      exit
    }
  ' "$file"
}

# -----------------------------------------------------------------------------
# extract_problem_statement: Get first paragraph from Problem Statement section
# -----------------------------------------------------------------------------
extract_problem_statement() {
  local file="$1"

  awk '
    /^## Problem Statement/ { capture = 1; next }
    /^##[^#]/ && capture { exit }
    capture && /^[^#<\[][[:alnum:]]/ {
      gsub(/[[:space:]]+$/, "")
      print
      exit
    }
  ' "$file"
}

# -----------------------------------------------------------------------------
# extract_imp_title: Get a display title from an IMP file
# Falls back to filename-derived title if first ## heading is generic
# -----------------------------------------------------------------------------
extract_imp_title() {
  local file="$1"
  local filename
  filename=$(basename "$file" .md)

  # Try first ## heading after frontmatter
  local heading
  heading=$(awk '
    BEGIN { fm = 0 }
    /^---$/ { fm++; next }
    fm >= 2 && /^## / {
      sub(/^## */, "")
      gsub(/[[:space:]]*$/, "")
      print
      exit
    }
  ' "$file")

  # Use heading if it's descriptive (not generic "Summary" variants)
  if [[ -n "$heading" ]] && ! echo "$heading" | grep -qiE '^summary( |$)'; then
    echo "$heading"
    return
  fi

  # Fallback: derive from filename (strip AI-IMP-NNN- prefix, dashes to spaces, capitalize)
  local slug
  slug=$(echo "$filename" | sed 's/^AI-IMP-[0-9]*-*//')
  slug=$(echo "$slug" | tr '-' ' ')
  echo "$slug" | awk '{print toupper(substr($0,1,1)) substr($0,2)}'
}

# -----------------------------------------------------------------------------
# get_epic_from_depends_on: Extract first AI-EPIC reference from depends_on
# -----------------------------------------------------------------------------
get_epic_from_depends_on() {
  local depends_on="$1"
  echo "$depends_on" | grep -oE 'AI-EPIC-[0-9]+' 2>/dev/null | head -1 || true
}

# -----------------------------------------------------------------------------
# add_parent_epic_backlink: Add/update parent_epic field in IMP file
# -----------------------------------------------------------------------------
add_parent_epic_backlink() {
  local file="$1"
  local epic_id="$2"
  local tmp="${file}.tmp"

  if [[ -z "$epic_id" ]]; then
    return
  fi

  # Find the full epic filename to create proper wikilink
  local epic_file
  epic_file=$(find "$RAG_DIR/AI-EPIC" -maxdepth 1 -name "${epic_id}*.md" 2>/dev/null | head -1)
  local epic_basename
  if [[ -n "$epic_file" ]]; then
    epic_basename=$(basename "$epic_file" .md)
  else
    epic_basename="$epic_id"
  fi

  local wikilink="[[${epic_basename}]]"

  # Check if parent_epic already exists
  if grep -q '^parent_epic:' "$file" 2>/dev/null; then
    # Update existing field
    sed "s|^parent_epic:.*|parent_epic: ${wikilink}|" "$file" > "$tmp" && mv "$tmp" "$file"
  else
    # Add new field after depends_on (or after kanban_status if no depends_on)
    if grep -q '^depends_on:' "$file" 2>/dev/null; then
      awk -v link="parent_epic: ${wikilink}" '
        /^depends_on:/ { print; print link; next }
        { print }
      ' "$file" > "$tmp" && mv "$tmp" "$file"
    elif grep -q '^kanban_status:' "$file" 2>/dev/null; then
      awk -v link="parent_epic: ${wikilink}" '
        /^kanban_status:/ { print; print link; next }
        { print }
      ' "$file" > "$tmp" && mv "$tmp" "$file"
    fi
  fi
}

# -----------------------------------------------------------------------------
# Process EPICs
# -----------------------------------------------------------------------------
echo "[generate-index] Scanning AI-EPIC files..."

if [[ ! -d "$RAG_DIR/AI-EPIC" ]]; then
  echo "[generate-index] WARNING: AI-EPIC/ directory not found — skipping EPICs"
else
  for file in "$RAG_DIR/AI-EPIC"/*.md; do
    [[ -f "$file" ]] || continue

    filename=$(basename "$file" .md)
    epic_num=$(echo "$filename" | grep -oE '[0-9]+' | head -1)

    # Normalize field names
    normalize_frontmatter "$file"

    # Extract fields
    status=$(extract_frontmatter_field "$file" "kanban_status")
    status=$(echo "$status" | tr '[:upper:]' '[:lower:]')
    date_completed=$(extract_frontmatter_field "$file" "date_completed")
    problem=$(extract_problem_statement "$file")

    # Create title from filename (capitalize first letter using awk for portability)
    title=$(echo "$filename" | sed 's/AI-EPIC-[0-9]*-//' | tr '-' ' ')
    title=$(echo "$title" | awk '{print toupper(substr($0,1,1)) substr($0,2)}')

    # Truncate problem statement if too long
    if [[ ${#problem} -gt 150 ]]; then
      problem="${problem:0:147}..."
    fi

    # Store: filename TAB epic_num TAB title TAB problem TAB date_completed
    case "$status" in
      completed|complete)
        printf '%s\t%s\t%s\t%s\t%s\n' "$filename" "$epic_num" "$title" "$problem" "$date_completed" >> "$EPICS_COMPLETED"
        ;;
      in-progress|in_progress)
        printf '%s\t%s\t%s\t%s\t%s\n' "$filename" "$epic_num" "$title" "$problem" "" >> "$EPICS_IN_PROGRESS"
        ;;
      deferred)
        printf '%s\t%s\t%s\t%s\t%s\n' "$filename" "$epic_num" "$title" "$problem" "" >> "$EPICS_DEFERRED"
        ;;
      cancelled)
        printf '%s\t%s\t%s\t%s\t%s\n' "$filename" "$epic_num" "$title" "$problem" "" >> "$EPICS_CANCELLED"
        ;;
      planned|backlog|"")
        printf '%s\t%s\t%s\t%s\t%s\n' "$filename" "$epic_num" "$title" "$problem" "" >> "$EPICS_PLANNED"
        ;;
      *)
        printf '%s\t%s\t%s\n' "$filename" "$epic_num" "$status" >> "$ILLEGAL_STATUS"
        ;;
    esac
  done
fi

# -----------------------------------------------------------------------------
# Process IMPs
# -----------------------------------------------------------------------------
echo "[generate-index] Scanning AI-IMP files..."

if [[ ! -d "$RAG_DIR/AI-IMP" ]]; then
  echo "[generate-index] WARNING: AI-IMP/ directory not found — skipping IMPs"
else
  for file in "$RAG_DIR/AI-IMP"/*.md; do
    [[ -f "$file" ]] || continue

    filename=$(basename "$file" .md)
    imp_num=$(echo "$filename" | sed 's/^AI-IMP-//' | grep -oE '^[0-9]+(-[0-9]+)?')

    # Normalize field names
    normalize_frontmatter "$file"

    # Extract fields
    status=$(extract_frontmatter_field "$file" "kanban_status")
    status=$(echo "$status" | tr '[:upper:]' '[:lower:]')

    # Extract display title
    imp_title=$(extract_imp_title "$file")

    # Validate IMP status
    case "$status" in
      completed|complete|in-progress|in_progress|deferred|planned|backlog|cancelled|"") ;;
      *)
        printf '%s\t%s\t%s\n' "$filename" "$imp_num" "$status" >> "$ILLEGAL_STATUS"
        ;;
    esac

    depends_on=$(extract_frontmatter_field "$file" "depends_on")

    # Find parent epic
    parent_epic=$(get_epic_from_depends_on "$depends_on")

    # Fallback to parent_epic frontmatter field (handles sub-tickets whose depends_on points to parent IMP)
    if [[ -z "$parent_epic" ]]; then
      parent_epic_field=$(extract_frontmatter_field "$file" "parent_epic")
      parent_epic=$(echo "$parent_epic_field" | grep -oE 'AI-EPIC-[0-9]+' | head -1 || true)
    fi

    # Add backlink if we found an epic
    if [[ -n "$parent_epic" ]]; then
      add_parent_epic_backlink "$file" "$parent_epic"
      epic_num=$(echo "$parent_epic" | grep -oE '[0-9]+')

      # Find epic status
      epic_file=$(find "$RAG_DIR/AI-EPIC" -maxdepth 1 -name "${parent_epic}*.md" 2>/dev/null | head -1)
      if [[ -n "$epic_file" ]]; then
        epic_status=$(extract_frontmatter_field "$epic_file" "kanban_status")
        epic_status=$(echo "$epic_status" | tr '[:upper:]' '[:lower:]')

        # Check for status mismatches
        if [[ "$epic_status" == "completed" || "$epic_status" == "complete" ]] && \
           [[ "$status" != "completed" && "$status" != "complete" ]]; then
          printf '%s\t%s\t%s\topen but parent epic %s is completed\n' "$filename" "$imp_num" "$status" "$parent_epic" >> "$STATUS_MISMATCHES"
        fi
      fi

      # Store: epic_num TAB imp_filename TAB imp_num TAB status TAB imp_title
      printf '%s\t%s\t%s\t%s\t%s\n' "$epic_num" "$filename" "$imp_num" "$status" "$imp_title" >> "$IMPS_BY_EPIC"
    else
      # Orphaned IMP - report non-completed in Anomalies; count all for Summary totals
      if [[ "$status" != "completed" && "$status" != "complete" ]]; then
        printf '%s\t%s\t%s\tno epic dependency found\n' "$filename" "$imp_num" "$status" >> "$ORPHAN_IMPS"
      fi
      # Track all orphan statuses for Summary counts (including completed)
      orphan_completed_count=${orphan_completed_count:-0}
      if [[ "$status" == "completed" || "$status" == "complete" ]]; then
        orphan_completed_count=$((orphan_completed_count + 1))
      fi
    fi
  done
fi

# -----------------------------------------------------------------------------
# Collect large files (report only)
# -----------------------------------------------------------------------------
echo "[generate-index] Scanning large files..."

ROOT_DIR="$(cd "$RAG_DIR/.." && pwd)"

while IFS= read -r -d '' file; do
  path="$ROOT_DIR/$file"
  [[ -f "$path" ]] || continue

  case "$file" in
    RAG/INDEX.md|*/package-lock.json|package-lock.json|tauri-app/src-tauri/tests/fixtures/color_golden.json|*.png|*.jpg|*.jpeg|*.gif|*.svg|*.ico|*.bin|*.exe|*.dll|*.so|*.dylib|*.woff*|*.ttf|*.otf|*.pdf|*.mp4|*.mov|*.zip|*.tar*|*.gz|*.xz)
      continue
      ;;
  esac

  lines=$(wc -l < "$path" | tr -d ' ')
  if [[ "$lines" -gt 300 ]]; then
    printf '%s\t%s\n' "$lines" "$file" >> "$LARGE_FILES"
  fi
done < <(git -C "$ROOT_DIR" ls-files -z)

# -----------------------------------------------------------------------------
# Generate INDEX.md
# -----------------------------------------------------------------------------
echo "[generate-index] Generating INDEX.md..."

{
  cat <<EOF
# Project Index
> Auto-generated by \`RAG/scripts/generate-index.sh\`. Do not edit manually.
> Last updated: ${TODAY}

EOF

  # ---------------------------------------------------------------------------
  # Summary table
  # ---------------------------------------------------------------------------
  # Count EPICs by status
  epics_in_progress=$(wc -l < "$EPICS_IN_PROGRESS" 2>/dev/null | tr -d ' ')
  epics_planned=$(wc -l < "$EPICS_PLANNED" 2>/dev/null | tr -d ' ')
  epics_deferred=$(wc -l < "$EPICS_DEFERRED" 2>/dev/null | tr -d ' ')
  epics_cancelled=$(wc -l < "$EPICS_CANCELLED" 2>/dev/null | tr -d ' ')
  epics_completed=$(wc -l < "$EPICS_COMPLETED" 2>/dev/null | tr -d ' ')
  epics_total=$((epics_in_progress + epics_planned + epics_deferred + epics_cancelled + epics_completed))

  # Count IMPs by individual status (from IMPS_BY_EPIC column 4)
  imps_in_progress=$(awk -F'\t' '$4 ~ /^in[-_]progress/' "$IMPS_BY_EPIC" 2>/dev/null | wc -l | tr -d ' ')
  imps_planned=$(awk -F'\t' '$4 ~ /^(planned|backlog)$/' "$IMPS_BY_EPIC" 2>/dev/null | wc -l | tr -d ' ')
  imps_deferred=$(awk -F'\t' '$4 == "deferred"' "$IMPS_BY_EPIC" 2>/dev/null | wc -l | tr -d ' ')
  imps_cancelled=$(awk -F'\t' '$4 == "cancelled"' "$IMPS_BY_EPIC" 2>/dev/null | wc -l | tr -d ' ')
  imps_completed=$(awk -F'\t' '$4 ~ /^complete/' "$IMPS_BY_EPIC" 2>/dev/null | wc -l | tr -d ' ')
  # Include orphan IMPs in totals (non-completed orphans shown in Anomalies; completed orphans counted silently)
  orphan_count=$(wc -l < "$ORPHAN_IMPS" 2>/dev/null | tr -d ' ')
  orphan_completed=${orphan_completed_count:-0}

  [[ -z "$imps_in_progress" ]] && imps_in_progress=0
  [[ -z "$imps_planned" ]] && imps_planned=0
  [[ -z "$imps_deferred" ]] && imps_deferred=0
  [[ -z "$imps_cancelled" ]] && imps_cancelled=0
  [[ -z "$imps_completed" ]] && imps_completed=0

  # Add orphan completed IMPs to the completed count
  imps_completed=$((imps_completed + orphan_completed))
  imps_total=$((imps_in_progress + imps_planned + imps_deferred + imps_cancelled + imps_completed + orphan_count))

  cat <<EOF
## Summary

| Status | EPICs | IMPs |
|--------|-------|------|
| In Progress | ${epics_in_progress} | ${imps_in_progress} |
| Planned | ${epics_planned} | ${imps_planned} |
| Deferred | ${epics_deferred} | ${imps_deferred} |
| Cancelled | ${epics_cancelled} | ${imps_cancelled} |
| Completed | ${epics_completed} | ${imps_completed} |
| **Total** | **${epics_total}** | **${imps_total}** |

EOF

  # ---------------------------------------------------------------------------
  # In Progress section
  # ---------------------------------------------------------------------------
  if [[ -s "$EPICS_IN_PROGRESS" ]]; then
    echo "## In Progress"
    echo ""

    while IFS=$'\t' read -r filename epic_num title problem _; do
      echo "### [[${filename}|EPIC-${epic_num}: ${title}]]"
      if [[ -n "$problem" ]]; then
        echo "> ${problem}"
      fi
      echo ""

      # Find IMPs for this epic
      if grep -q "^${epic_num}${TAB}" "$IMPS_BY_EPIC" 2>/dev/null; then
        echo "**IMPs:**"
        grep "^${epic_num}${TAB}" "$IMPS_BY_EPIC" | while IFS=$'\t' read -r _ imp_filename imp_num imp_status imp_title; do
          echo "- [[${imp_filename}|IMP-${imp_num}]] ${imp_title} — ${imp_status}"
        done
        echo ""
      fi

      echo "---"
      echo ""
    done < "$EPICS_IN_PROGRESS"
  fi

  # ---------------------------------------------------------------------------
  # Planned section
  # ---------------------------------------------------------------------------
  if [[ -s "$EPICS_PLANNED" ]]; then
    echo "## Planned"
    echo ""

    while IFS=$'\t' read -r filename epic_num title problem _; do
      echo "### [[${filename}|EPIC-${epic_num}: ${title}]]"
      if [[ -n "$problem" ]]; then
        echo "> ${problem}"
      fi
      echo ""

      # Find IMPs for this epic
      if grep -q "^${epic_num}${TAB}" "$IMPS_BY_EPIC" 2>/dev/null; then
        echo "**IMPs:**"
        grep "^${epic_num}${TAB}" "$IMPS_BY_EPIC" | while IFS=$'\t' read -r _ imp_filename imp_num imp_status imp_title; do
          echo "- [[${imp_filename}|IMP-${imp_num}]] ${imp_title} — ${imp_status}"
        done
        echo ""
      fi

      echo "---"
      echo ""
    done < "$EPICS_PLANNED"
  fi

  # ---------------------------------------------------------------------------
  # Anomalies section
  # ---------------------------------------------------------------------------
  if [[ -s "$ORPHAN_IMPS" ]] || [[ -s "$STATUS_MISMATCHES" ]] || [[ -s "$ILLEGAL_STATUS" ]]; then
    echo "## Anomalies"
    echo ""

    if [[ -s "$ORPHAN_IMPS" ]]; then
      echo "### Orphaned IMPs (no epic dependency)"
      while IFS=$'\t' read -r imp_filename imp_num status reason; do
        echo "- [[${imp_filename}|IMP-${imp_num}]] — ${status}, ${reason}"
      done < "$ORPHAN_IMPS"
      echo ""
    fi

    if [[ -s "$STATUS_MISMATCHES" ]]; then
      echo "### Status Mismatches"
      while IFS=$'\t' read -r imp_filename imp_num status reason; do
        echo "- [[${imp_filename}|IMP-${imp_num}]] — ${reason}"
      done < "$STATUS_MISMATCHES"
      echo ""
    fi

    if [[ -s "$ILLEGAL_STATUS" ]]; then
      echo "### Illegal Status Values"
      while IFS=$'\t' read -r fname fnum fstatus _; do
        echo "- [[${fname}|${fnum}]] — unrecognized status: \`${fstatus}\`"
      done < "$ILLEGAL_STATUS"
      echo ""
    fi

    echo "---"
    echo ""
  fi

  # ---------------------------------------------------------------------------
  # Cancelled / Deferred section (merged)
  # ---------------------------------------------------------------------------
  if [[ -s "$EPICS_CANCELLED" ]] || [[ -s "$EPICS_DEFERRED" ]]; then
    echo "## Cancelled / Deferred"
    echo ""

    # Render cancelled EPICs first
    if [[ -s "$EPICS_CANCELLED" ]]; then
      while IFS=$'\t' read -r filename epic_num title problem _; do
        echo "- [[${filename}|EPIC-${epic_num}]] ${title} — cancelled — \"${problem}\""
        # Find IMPs for this epic
        if grep -q "^${epic_num}${TAB}" "$IMPS_BY_EPIC" 2>/dev/null; then
          grep "^${epic_num}${TAB}" "$IMPS_BY_EPIC" | while IFS=$'\t' read -r _ imp_filename imp_num imp_status imp_title; do
            echo "  - [[${imp_filename}|IMP-${imp_num}]] ${imp_title} — ${imp_status}"
          done
        else
          echo "  - (no IMPs)"
        fi
      done < "$EPICS_CANCELLED"
    fi

    # Render deferred EPICs
    if [[ -s "$EPICS_DEFERRED" ]]; then
      while IFS=$'\t' read -r filename epic_num title problem _; do
        echo "- [[${filename}|EPIC-${epic_num}]] ${title} — deferred — \"${problem}\""
        # Find IMPs for this epic
        if grep -q "^${epic_num}${TAB}" "$IMPS_BY_EPIC" 2>/dev/null; then
          grep "^${epic_num}${TAB}" "$IMPS_BY_EPIC" | while IFS=$'\t' read -r _ imp_filename imp_num imp_status imp_title; do
            echo "  - [[${imp_filename}|IMP-${imp_num}]] ${imp_title} — ${imp_status}"
          done
        else
          echo "  - (no IMPs)"
        fi
      done < "$EPICS_DEFERRED"
    fi

    echo ""
    echo "---"
    echo ""
  fi

  # ---------------------------------------------------------------------------
  # Size Watch section
  # ---------------------------------------------------------------------------
  if [[ -s "$LARGE_FILES" ]]; then
    echo "## Size Watch"
    echo ""
    echo "Generated from tracked files; binary assets excluded."
    echo ""

    # Check if any files over 600
    if sort -rn "$LARGE_FILES" | awk -F '\t' '$1 > 600 { found=1 } END { exit !found }'; then
      echo "### > 600 LOC"
      echo ""
      sort -rn "$LARGE_FILES" | awk -F '\t' '$1 > 600 { print "- " $2 " (" $1 " LOC)" }'
      echo ""
    fi

    # Check if any files 300-600
    if sort -rn "$LARGE_FILES" | awk -F '\t' '$1 > 300 && $1 <= 600 { found=1 } END { exit !found }'; then
      echo "### > 300 LOC"
      echo ""
      sort -rn "$LARGE_FILES" | awk -F '\t' '$1 > 300 && $1 <= 600 { print "- " $2 " (" $1 " LOC)" }'
      echo ""
    fi

    echo "---"
    echo ""
  fi

  # ---------------------------------------------------------------------------
  # Completed section (with IMP children)
  # ---------------------------------------------------------------------------
  if [[ -s "$EPICS_COMPLETED" ]]; then
    completed_epics=$(wc -l < "$EPICS_COMPLETED" | tr -d ' ')
    completed_imps=$(awk -F'\t' '$4 ~ /^complete/' "$IMPS_BY_EPIC" 2>/dev/null | wc -l | tr -d ' ')
    [[ -z "$completed_imps" ]] && completed_imps=0

    echo "## Completed"
    echo "<details>"
    echo "<summary>${completed_epics} EPICs, ${completed_imps} IMPs completed</summary>"
    echo ""

    while IFS=$'\t' read -r filename epic_num title _ date_completed; do
      if [[ -n "$date_completed" ]]; then
        echo "- [[${filename}|EPIC-${epic_num}]] ${title} — ${date_completed}"
      else
        echo "- [[${filename}|EPIC-${epic_num}]] ${title}"
      fi
      # Show IMP children for this epic
      if grep -q "^${epic_num}${TAB}" "$IMPS_BY_EPIC" 2>/dev/null; then
        grep "^${epic_num}${TAB}" "$IMPS_BY_EPIC" | while IFS=$'\t' read -r _ imp_filename imp_num imp_status imp_title; do
          echo "  - [[${imp_filename}|IMP-${imp_num}]] ${imp_title} — ${imp_status}"
        done
      fi
    done < "$EPICS_COMPLETED"

    echo ""
    echo "</details>"
  fi

} > "$INDEX_FILE"

echo "[generate-index] Done. Generated $INDEX_FILE"
