---
node_id: AI-IMP-007
tags:
  - IMP-LIST
  - Implementation
  - theme-generation
  - ui-polish
kanban_status: cancelled
depends_on: AI-EPIC-002
confidence_score: 0.6
created_date: 2025-09-30
close_date:
--- 


# AI-IMP-007-scrollbar-gutter-mitigation

## Summary of Issue
Scrollbar gutters retain stale colors because the generator leaves `track_style` and `ends_style` unset. We will make the generator write both styles using the chosen background color and offer a `--no-scrollbar` opt-out so users can omit the gutter when they know panes will not scroll. Completion means regenerated themes always paint scrollbars to match the active palette or omit them entirely when requested.

### Out of Scope 
- Modifying rmpc rendering internals.
- Automatically detecting per-pane scroll necessity inside rmpc.

### Design/Approach  
Update `generate_theme_ron` to map background-derived styles into scrollbar tracks/ends. Introduce a CLI/config flag (persisted in JSON output) that toggles scrollbar emission; when disabled, emit `scrollbar: None`. Default behaviour keeps scrollbars but now harmonised with the background color. Extend shell wrapper to pass through the new option when configured.

### Files to Touch
- `src/rmpc_theme_gen.rs`: set scrollbar colors, add CLI flag/plumbing.
- `src/lib.rs` (if config structs exposed) or relevant module for option wiring.
- `README.md` / docs: describe new flag.
- `on_song_change.sh`: pass opt-out environment/config if needed.
- `test-results/run-tests.sh`: ensure coverage of new option.

### Implementation Checklist
<CRITICAL_RULE>
Before marking an item complete on the checklist MUST **stop** and **think**. Have you validated all aspects are **implemented** and **tested**? 
</CRITICAL_RULE> 
- [ ] Adjust `generate_theme_ron` so scrollbar `track_style`/`ends_style` use the computed background color.
- [ ] Add CLI flag/config entry (e.g., `--disable-scrollbar`) and thread through to theme emission.
- [ ] Update JSON output schema/tests to reflect the new configuration knob if exposed.
- [ ] Document the new behaviour and flag in `README.md` and shell script comments.
- [ ] Update integration test script to exercise both default and disabled-scrollbar paths.
 
### Acceptance Criteria
**Scenario:** Generating theme with defaults.
**GIVEN** the generator runs with existing options,
**WHEN** a theme is produced,
**THEN** the `scrollbar.track_style` and `scrollbar.ends_style` entries equal the background hex value.

**Scenario:** Disabling scrollbars.
**GIVEN** the generator runs with the new opt-out flag,
**WHEN** the theme is produced,
**THEN** the `scrollbar` section is omitted or set to `None` in the RON output,
**AND** the JSON metadata records the opt-out.

### Issues Encountered 

**RESOLUTION REQUIRED CHANGES TO RMPC directly as an upstream PR**
<!-- Repeat the Issue pattern above as needed based on the needs of the users request.  --> 
