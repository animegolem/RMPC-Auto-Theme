---
node_id: AI-IMP-015
tags:
  - IMP-LIST
  - Implementation
  - theme
  - config
kanban_status: planned
depends_on: [AI-EPIC-003]
confidence_score: 0.8
created_date: 2025-10-01
close_date:
---

# AI-IMP-015-base-theme-overlay-and-config

## Summary of Issue
Theme layout/symbols are hardcoded. Users can’t customize format without code edits. Add a user-provided base theme and a small config to make the generator a color overlay tool.

### Out of Scope
- Large layout semantics changes in rmpc itself.

### Design/Approach
- If present, read `~/.config/rmpc/theme-switcher/base.ron`, parse to a minimal struct, and patch documented color tokens only.
- If absent, fallback to the current internal template.
- Load optional `~/.config/rmpc/theme-switcher/config.ron` (k, thresholds, sampling); allow CLI overrides.
- Provide `scripts/setup-config.sh` to copy samples into place (doom-like workflow).

### Files to Touch
- `src/rmpc_theme_gen.rs`: base overlay loader, token patcher, config loader.
- `scripts/setup-config.sh`: new script.
- `README.md`: usage docs for base/config.

### Implementation Checklist

<CRITICAL_RULE>
Before marking an item complete on the checklist MUST **stop** and **think**. Have you validated all aspects are **implemented** and **tested**? 
</CRITICAL_RULE>

- [ ] Add RON loader for base theme; define list of color keys to patch.
- [ ] Implement overlay merge and write final RON to output path.
- [ ] Add config loader (RON) and wiring to CLI defaults.
- [ ] Create sample `base.ron` and `config.ron`; add setup script.
- [ ] Document overlay + config workflow in README.

### Acceptance Criteria
**GIVEN** a user supplies a base theme,
**WHEN** the generator runs,
**THEN** only color tokens are overwritten while layout/symbols remain unchanged,
**AND** thresholds/k can be configured via config file or CLI.

### Issues Encountered
TBD.

