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

The four-view redesign existed only as a design handoff. Done
state: the owner's live config.ron and themes/base.ron implement it
(tabs Library/Now Playing/Lyrics/Search on keys 1-4, hidden tab
bar, np_header component, two-column queue table), verified visually
by the owner, and the applied pair is archived in
~/git/personal-theming.

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

- [x] base.ron and config.ron written per handoff, corrected for
      0.11 semantics (tabs in config; Pane(Tabs) omission hides bar).
- [x] cava installed; Cava panes in Now Playing and Lyrics views.
- [x] Applied to the running instance over IPC without error.
- [ ] Owner visual verification: all four views render as designed
      (keys 1-4), queue table two-column, no tab bar, np_header
      strip present in Now Playing and Lyrics.
- [ ] Applied pair archived in personal-theming with a commit.
- [ ] Handoff corrections written back to
      personal-theming/RAG/design/handoff.md.

### Acceptance Criteria

GIVEN the owner's terminal, WHEN keys 1-4 are pressed, THEN the
four designed views render with no tab bar and readable defaults.

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
