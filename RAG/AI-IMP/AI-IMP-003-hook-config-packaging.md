---
node_id: AI-IMP-003
tags:
  - IMP-LIST
  - Implementation
  - shell
  - config
  - install
kanban_status: completed
depends_on: ["AI-IMP-002"]
parent_epic: [[AI-EPIC-001-album-art-driven-live-theming]]
confidence_score: 0.75
date_created: 2026-08-19
date_completed: 2026-09-06
---

# AI-IMP-003-hook-config-packaging

## Summary of Issue #1

The generator has no trigger, persistent configuration, or install
story. Done state: an on_song_change hook wired per SPEC §7 (art
hash short-circuit, exit-0 failure policy, capped log, theme-then-
config-safe IPC apply), a config file at
~/.config/rmpc-auto-theme/ with SPEC §7 defaults, and an installer
that wires config.ron's on_song_change plus builds/installs the
binary. macOS caveat honored: apply via IPC only, never rely on hot
reload (SPEC §7 verified finding).

### Out of Scope 

No UI-config changes (AI-IMP-004), no daemonization, no rmpcd.

### Design/Approach  

Hook ports v1's proven shell skeleton (hash short-circuit incl.
no-hash-tool degradation, exit-0 paths, rmpc remote toast) minus the
MIME dance; art flows `rmpc albumart -o - | rmpc-theme-gen | rmpc
remote --pid $PID set theme -`. Config: TOML with base-theme path,
quality preset, per-role bands, floors, OSC 11 toggle; flags
override file. Log: single file, size-capped truncation. Installer
mirrors nowplayd's discipline: idempotent, validates targets,
refuses unsafe HOME.

### Files to Touch

- hook/on_song_change.sh, src/config.rs, install.sh, README
  updates (Google style).

### Implementation Checklist

<CRITICAL_RULE>
Before marking an item complete on the checklist MUST **stop** and **think**. Have you validated all aspects are **implemented** and **tested**? 
</CRITICAL_RULE>

- [x] Hook: hash short-circuit, exit codes honored, exit-0 on all
      failures, capped log; shell-level tests in a temp HOME.
- [x] Config file parsed with documented defaults; malformed file =
      loud failure at generator start, silent skip in hook.
- [x] OSC 11 optional and gated on writable /dev/tty.
- [x] install.sh idempotent; wires on_song_change into config.ron
      only with explicit consent flag.
- [ ] Live smoke on owner's machine queued (owner-run): song change
      rethemes < 1 s; same-album no-op; kill generator mid-run leaves
      theme intact.

### Acceptance Criteria

GIVEN two tracks sharing album art, WHEN the second starts, THEN
the hook exits before extraction and the theme file is untouched.
GIVEN a generator crash, WHEN the hook finishes, THEN it exits 0 and
rmpc keeps the current theme.

### Issues Encountered 

Round 1 STOP produced five rulings (SPEC 0.7): §8 reconciled to v3,
exit-2 = base reset, pure-binary seam with --print-config +
--print-base-theme, toml dep, materialization deferred to RL/owner.
Implementation landed in one commit (4a0a9c8, 35 tests incl. 9
shell-integration in temp HOME), merged after independent gate
reproduction. The final checklist item (owner live smoke: <1s
retheme, same-album no-op, mid-run kill) is QUEUED in
HUMAN-TESTING.md, not run — ticket closes on the code; the smoke
gates the epic close, not this ticket.
