---
node_id: 2026-11-14-LOG-AI-issue-triage-art-fallback-primitive
tags:
  - AI-log
  - development-summary
  - issue-triage
  - publisher
  - userspace-primitive
closed_tickets: [AI-IMP-007]
created_date: 2026-11-14
related_files:
  - SPEC.org
  - src/publisher.rs
  - src/config.rs
  - README.md
  - fixtures/config.toml
  - RAG/AI-IMP/AI-IMP-007-art-fallback-command.md
confidence_score: 0.95
---

# 2026-11-14-LOG-AI-issue-triage-art-fallback-primitive

## Work Completed

Triaged both open GitHub issues with live reproduction before any
code (owner-directed). Built an isolated sandbox rig (throwaway MPD
0.24.16 on a unix socket + rmpc 0.11.0 in tmux; owner's live MPD
untouched). Findings: issue #1 (header truncation) no longer
reproduces — the v2 base theme's `scroll_speed: 1` marquee-scrolls a
150-char album name (rmpc 0.9.0 added property-pane scrolling).
Issue #3 (yt-dlp retheme) already works end to end: rmpc's `addyt`
passes `--embed-thumbnail`, MPD `readpicture` serves embedded art
for out-of-database absolute paths, and the publisher's existing
fallback consumes it — verified with a real yt-dlp download played
from the cache dir. Owner then ruled the generic primitive worth
shipping anyway: SPEC rev 0.14 adds `art_fallback_command` (AI-IMP-
007), a pull-shaped userspace hook run only when both MPD verbs
return no art. Implemented, tested, acceptance-run on the rig
(sidecar-thumbnail success and degrade paths), cross-model review
loop with GPT-6.1-Sol (xhigh) — one blocker found and fixed — then
merged to main as PR #4. Both issues answered with approved
on-behalf comments and closed; issue tracker is now empty.

## Session Commits

- `9ba2364` SPEC rev 0.14 + AI-IMP-007 cut (reproduction findings
  recorded in the revision log).
- `5ef4315` AI-IMP-007 implementation: config plumbing (TOML,
  `--set`, `--print-config`, empty-value validation), publisher
  fallback at the no-art seam, bounds (10 s, MAX_ART_BYTES), docs
  in README and fixtures/config.toml, unit tests.
- `19d8284` Review-loop S1 fix: fallback shell now leads its own
  process group; deadline kills the group; stdout EOF awaited via
  mpsc `recv_timeout` under the same deadline instead of an
  unbounded `join`.
- `673686e` (merge) PR #4 into main; remote feature branch deleted.

## Issues Encountered

- Sol's round-1 blocker was real and reproduced: killing only the
  shell left grandchildren holding the stdout pipe (`sleep 12; :`
  ran 12 s; a backgrounded orphan after shell exit blocked forever).
  Fix as above; two regression tests pin both shapes.
- CLAUDE.md's upstream reference clone `~/git/_notmyrepos_/rmpc`
  does not exist on disk; verified pane/scroll facts via GitHub API
  instead. Filed in the house friction ledger
  (`interchange/recommendations/rmpc-auto-theme.frictions.2026-11-14.jsonl`).
  CLAUDE.md pointer still needs fixing or the clone restoring.
- `validate-tickets.sh` reports one pre-existing error in
  `2026-09-20-LOG-AI-rl-brief-004-close-and-vendoring.md`
  (frontmatter not first); untouched this session.
- Process deviation, owner-directed: the Review Lead seat
  implemented directly in this bb worktree thread instead of the
  two-lead brief cycle; independence was restored via the
  review-loop (Sol as cross-family reviewer) before merge.

## Tests Added

- `config::tests::art_fallback_command_plumbs_through_file_override_and_query`
  — TOML key, override, query, whitespace rejection.
- `publisher::tests::art_fallback_returns_stdout_bytes_with_file_in_env`
  — success path with `$FILE` substitution.
- `publisher::tests::art_fallback_degrades_on_failure_and_empty_output`
  — nonzero exit, empty stdout, stderr-only output.
- `publisher::tests::art_fallback_kills_a_hung_command_at_the_deadline`
  plus the two S1 regressions (compound command grandchild; exited
  shell with orphan holding stdout) — all assert elapsed < 5 s with
  a 200 ms bound via the parameterized `run_art_fallback_bounded`.
- Suite now 64/64 green; fmt and clippy clean.

## Next Steps

- Watch #3/#1 for reporter replies (reopen path is in both
  comments); a `hook.log` from Yogchan would confirm or refute the
  works-already finding on their setup.
- The AUR package now serves a main with the new key; no release
  tag was cut this session — consider one if the AUR PKGBUILD
  tracks tags.
- Fix the stale `~/git/_notmyrepos_/rmpc` pointer in CLAUDE.md
  (restore the clone or amend the doc).
- Pre-existing AI-LOG frontmatter error (2026-09-20 log) still
  trips validate-tickets; cheap cleanup.
- Reviewer thread for reuse if findings return: `thr_ckxanuww8r`
  (Codex gpt-6.1-sol, xhigh).
