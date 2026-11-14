---
node_id: AI-IMP-007
tags:
  - IMP-LIST
  - Implementation
  - publisher
  - config
  - userspace-primitive
kanban_status: in-progress
depends_on:
parent_epic:
confidence_score: 0.9
date_created: 2026-11-14
date_completed:
---

# AI-IMP-007-art-fallback-command

## Summary of Issue #1

GitHub issue #3 asked for retheming on yt-dlp-added songs. Live
reproduction (2026-11-14, rmpc 0.11.0 + MPD 0.24.16 sandbox) showed
the ask already works: rmpc's `addyt` embeds thumbnails and MPD
`readpicture` serves embedded art for out-of-database absolute
paths, which our existing fallback chain consumes. The remaining gap
is art sources that do NOT embed (user-scripted downloads, other
second spaces). Per SPEC rev 0.14 (§7), we ship a userspace
primitive instead of any format-specific integration: a new
`art_fallback_command` config key the publisher runs only when both
MPD verbs return no art. Done means: the key is plumbed through
file config, `--set`, and `--print-config`; the command runs
bounded (10 s, MAX_ART_BYTES); every failure degrades to the
existing no-art path; gates pass.

### Out of Scope

- Any yt-dlp, cache-dir, or format-specific detection in code.
- A push-shaped "apply this art now" external entry point.
- Changes to admission/sequencing/commit mechanics.
- Hook (`on_song_change.sh`) changes — the seam is publisher-side.

### Design/Approach

Pull shape, owner-ruled: the publisher already decides the no-art
moment inside the admitted, token-bound event, so the fallback runs
there and its bytes ride the unchanged publish/commit path — no new
races. The command is a single string run via `/bin/sh -c` with
`FILE` in the environment (the admitted identity), stdout captured
as art bytes. Bounds per the limits law: 10-second wall clock
enforced by poll-and-kill (std has no wait_timeout), stdout capped
at MAX_ART_BYTES during read, stdin null. Nonzero exit, empty
stdout, timeout, or cap breach log one stderr note and return None
— indistinguishable downstream from MPD having no art. Alternative
considered and rejected: a push-shaped CLI entry (user reimplements
"when" and races the pipeline); documenting only the raw
stdin→stdout mode (leaves the timing gap that motivated the issue).

### Files to Touch

`SPEC.org`: rev 0.14 amendment (done, rides this ticket).
`src/config.rs`: `art_fallback_command` on EffectiveConfig,
FileConfig, apply_override, print_value, validate (non-empty).
`src/publisher.rs`: run fallback when `mpd.art()` returns None;
bounded child-process helper.
`tests/generator.rs` or `tests/publisher.rs`: config plumbing cases.
`tests/shell.rs`: end-to-end fallback behavior if the harness shape
fits; otherwise unit coverage of the helper.
`fixtures/config.toml`: document the key (commented example).
`README.md`: user-facing docs for the primitive.

### Implementation Checklist

<CRITICAL_RULE>
Before marking an item complete on the checklist MUST **stop** and **think**. Have you validated all aspects are **implemented** and **tested**?
</CRITICAL_RULE>

- [ ] `src/config.rs`: add `art_fallback_command: Option<String>` to EffectiveConfig (default None), FileConfig, merge, `apply_override`, `print_value`; `validate` rejects empty/whitespace-only values.
- [ ] `src/publisher.rs`: add `run_art_fallback(command, file) -> Option<Vec<u8>>` — `/bin/sh -c`, env FILE, stdin null, stdout piped, 10 s poll-and-kill timeout, MAX_ART_BYTES cap, stderr note on every degrade path.
- [ ] `src/publisher.rs`: call the fallback in `publish_event` when `mpd.art()` yields None and the key is set; Some(bytes) proceeds through the existing generate path unchanged.
- [ ] Tests: config plumbing (file key, --set, --print-config, empty-value rejection).
- [ ] Tests: fallback helper — success bytes, nonzero exit → None, empty stdout → None, timeout kill → None, oversize → None.
- [ ] `fixtures/config.toml` + `README.md`: document the key with a yt-dlp-flavored example script.
- [ ] Gates: `cargo fmt --check && cargo clippy -- -D warnings && cargo test` all green.
- [ ] `RAG/scripts/generate-index.sh` run; INDEX.md committed with the ticket.

### Acceptance Criteria

**Scenario:** A song outside MPD's database with no embedded art
plays while `art_fallback_command` points at a script that prints
a JPEG.
**GIVEN** the sandbox MPD/rmpc rig from the 2026-11-14 reproduction.
**WHEN** the publisher handles the admitted event and both MPD verbs
return no art.
**THEN** the configured command runs with FILE set to the admitted
path, its stdout is hashed and themed exactly as MPD art would be,
and the theme applies over IPC.
**AND** with the command unset, failing, printing nothing, hanging
past 10 s, or exceeding the cap, the run publishes the unchanged
no-art envelope and base theme, exit 0.

### Issues Encountered

<!-- Fill honestly as work proceeds. -->
