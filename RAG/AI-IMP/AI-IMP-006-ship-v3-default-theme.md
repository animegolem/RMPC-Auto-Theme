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

WITHDRAWN 2026-09-08 (owner, on seeing it live): the separator
label stays frame-colored like its rule — no accent treatment. The
owner's original contrast instinct turned out to be about the
UNPAINTED chrome (the [Playing] state, artist name, volume, badge —
the DQ-4 class), not the label. No mechanism needed for the label.

WIDENED (DQ-4, same day): the label is one instance of a class —
volume slider/percentage, [zxcv] hint, and the NORMAL/INSERT badge
are all literal-styled component surfaces inside the theme document
that the map never reaches. Round 1 proposes component-path
patching (array-nested style locations) and the per-surface role
table from DQ-4; the shipped default must be 100%-mapped chrome —
no literal color a full retheme leaves behind.

### Addendum 2 (2026-09-08): palette publisher scope + acceptance

The notan-palette/1 publisher joins this ticket (contract at
contract/notan-palette-v1.md; DQ-7 + addendum binding). Acceptance
additions from the grisaille consultation: (a) analysis-cache hits
and publication identity tested SEPARATELY — same-art track change
MUST publish a fresh envelope; (b) --publish-current one-shot;
(c) admission-token ordering incl. failure attempts; (d) a genuine
captured artifact from fixtures/chromatic.png.b64 (full hash,
revision, command, output) committed as the conformance golden;
(e) failed-status publishes with never-invented identity.

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
