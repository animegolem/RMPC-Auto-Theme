---
node_id: AI-IMP-004
tags:
  - IMP-LIST
  - Implementation
  - config
  - rollout
kanban_status: planned
depends_on: []
parent_epic: [[AI-EPIC-001-album-art-driven-live-theming]]
confidence_score: 0.9
date_created: 2026-08-19
date_completed:
---

# AI-IMP-004-four-view-ui-rollout

## Summary of Issue #1

RE-CUT 2026-09-05 (owner-ruled): the original four-view rollout was
applied 2026-08-19 and then SUPERSEDED by the v3 Linux-recreation
layout (2026-08-20, archived personal-theming 245bc4e) plus the
two-column Lyrics tab (2026-08-27). This ticket is now
verify-and-close: confirm the CURRENT live pair is the canonical
base for the generator and the archive is in sync. Done state: every
SPEC §6 binding-map path is present (or deliberately None) in the
live base theme; the live config.ron/theme pair is re-archived in
~/git/personal-theming with the lyrics amendment; §8 boundary holds
(no layout fields owned by the generator); findings folded into
SPEC if any.

### Out of Scope 

Generated coloring (that's the generator's job); fallback-art
PNG export (deferred until wanted); lyrics-view art thumbnail
(no native equivalent, accepted loss).

### Design/Approach  

Applied 2026-08-19 in-session: base.ron derived from `rmpc theme`
default with layout stripped of global header + Pane(Tabs),
two-column song_table_format (Group(artist — title) 100% +
Duration 6 Right), np_header component (2-row stats, bitrate cut);
config.ron: 4 tabs (Artists browser; art+cava | np_header+queue;
np_header+lyrics+cava; search), keybinds 1-4, theme: "base";
cava installed via brew. Applied over IPC (set theme, then set
config — macOS hot reload non-functional, SPEC §7). Findings folded
into SPEC rev 0.2 including the 0.11.0 debuginfo false positive.

### Files to Touch

- ~/.config/rmpc/config.ron, ~/.config/rmpc/themes/base.ron
  (live), ~/git/personal-theming (archive copies).

### Implementation Checklist

<CRITICAL_RULE>
Before marking an item complete on the checklist MUST **stop** and **think**. Have you validated all aspects are **implemented** and **tested**? 
</CRITICAL_RULE>

- [ ] Binding-map coverage audit of the live theme (18+ mapped
      paths): each present or documented-None; report table in
      submission.
- [ ] Live pair (config.ron + theme) re-archived in
      ~/git/personal-theming, including the 2026-08-27 two-column
      Lyrics tab; commit referenced in submission.
- [ ] §8 boundary check: generator patches colors only; no layout
      drift between archive and live.
- [ ] Ticket text/SPEC discrepancies found during audit reported,
      not silently fixed.
### Acceptance Criteria

GIVEN the owner's terminal, WHEN keys 1-4 are pressed, THEN the
canonical v3 views (SPEC §8 rev 0.7: visible tab bar, np_header,
two-column Lyrics) render with readable defaults, AND the archive
pair in ~/git/personal-theming matches the live pair.

### Issues Encountered 

- First render review (owner, 2026-08-19): queue block flush against
  the window edge, art size/placement drifting with resizes, and the
  hidden tab bar judged a mistake. v2.1 amendments applied same day:
  spacer-pane insets, fixed 64-col art column with top alignment,
  tab bar restored, tab order now 1 Now Playing / 2 Library /
  3 Lyrics / 4 Search. Handoff updated in personal-theming.
- macOS: rmpc hot reload inert; debuginfo false-positives on tab
  components (SPEC §7). Applied over IPC both times.
- Cava caveat: on macOS cava needs a loopback input (e.g. BlackHole)
  to see MPD output; with the default mic input the pane may stay
  empty or react to room noise.
