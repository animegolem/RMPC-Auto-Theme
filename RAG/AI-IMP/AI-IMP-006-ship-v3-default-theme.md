---
node_id: AI-IMP-006
tags:
  - IMP-LIST
  - Implementation
  - rust
  - packaging
kanban_status: planned
depends_on: ["AI-IMP-005"]
parent_epic: [[AI-EPIC-001-album-art-driven-live-theming]]
confidence_score: 0.75
date_created: 2026-09-08
date_completed:
---

# AI-IMP-006-ship-v3-default-theme

## Summary of Issue #1

The shipped default base (built-in --print-base-theme,
fixtures/default_theme.ron, installer sample) is upstream's
example_theme — a layout nobody in this project actually runs. The
owner ruled (DQ-3): ship the v3 pair. Done state: the built-in base
is a genericized copy of the owner's current theme (np_header, tab
chrome, labeled " Lyrics " separator, materialized reset literals);
a matching sample config.ron ships beside it (4 tabs, keybinds,
hook vector commented out); SPEC §6's default-base clause amended;
fixture provenance updated; all goldens regenerated deliberately;
installer prints the config sample path in its manual output.

### Addendum (owner, 2026-09-08)

The lyrics separator's LABEL should render in a themed accent
(contrast against the frame-toned rule), per the owner's sketch.
Because border_title colors are config-side literals the generator
cannot patch, round 1 must propose the mechanism — likely moving
the separator's styled label into a theme-owned surface (component
or a new mapped path) so the accent role reaches it; the offset
rule itself stays frame-colored. This rides this ticket, not 005.

### Out of Scope

Layout changes beyond genericizing (strip owner-specific paths,
e.g. lyrics_dir); reactive lyrics layout (DQ-2); any new roles.

### Design/Approach

Round 1 proposes the exact genericization diff (what in the owner
pair is personal vs shippable) and the fixture/test blast radius.
The default base must parse standalone with rmpc 0.11 and satisfy
34/0 binding coverage by construction.

### Files to Touch

src/ (embedded base), fixtures/, install.sh, README, tests/,
SPEC.org (via RL).

### Implementation Checklist

<CRITICAL_RULE>
Before marking an item complete on the checklist MUST **stop** and **think**. Have you validated all aspects are **implemented** and **tested**? 
</CRITICAL_RULE>

- [ ] Round-1 genericization proposal + blast radius.
- [ ] Embedded base + sample config ship; rmpc 0.11 parse-verified.
- [ ] 34/0 coverage property holds on the shipped base.
- [ ] Goldens regenerated deliberately; provenance documented.
- [ ] Gates green; validate-tickets clean.

### Acceptance Criteria

GIVEN a fresh install with no user theme, WHEN the generator runs,
THEN the emitted document is the v3-derived default fully tinted
(34/0), and the installer's manual output names the sample config.

### Issues Encountered

(to be filled during implementation)
