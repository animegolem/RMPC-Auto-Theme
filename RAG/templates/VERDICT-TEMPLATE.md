---
submission: {{same kebab-case id as the inbox file}}
verdict: {{accepted | amend | declined}}
branch: {{prefix}}/{{id}}
commit: {{sha reviewed — must match the inbox file's commit}}
round: {{matches the inbox round this answers}}
reviewed: {{YYYY-MM-DD}}
---

<!--
Written by the Review Lead to {{channel-dir}}/outbox/{{id}}.md, archived
(archive/{{id}}-outbox-r{{N}}.md) then overwritten wholesale each
round. Review order before writing: boundary
discipline first (files touched vs the brief's fences), then
load-bearing logic, then REPRODUCE the counts yourself. Trust the
report's claims only after the counts reproduce.
-->

## Notes

{{Rationale. State what the Review Lead verified, concretely:}}
- Topology: {{N}} atomic commits on the assigned tip; main's refs
  untouched; fences clean.
- Logic: {{the load-bearing calls, named — what was checked and
  what was deliberately trusted from the self-review}}.
- Counts reproduced: {{your independently-run numbers vs the
  report's, per suite; the CI oracle branch result if platform
  legs exist}}.

{{If accepted: the merge commit on main; what closed; the tag if a
release rides it. Cleanup is the Review Lead's — the Code Lead deletes
nothing, moves the id's channel files to archive/, and stops
tracking the id.}}

{{If declined: why, and whether a different approach would be
welcome — "stop" and "try differently" are different verdicts;
say which.}}

## Amendments

<!-- Only when verdict is `amend`. -->

{{Numbered, actionable instructions — each one independently
checkable:}}

1. {{file/function: exact change required and why}}.
2. {{...}}

Fix them on the SAME branch, update `commit` and bump `round` in
the inbox file, and resubmit by rewriting it. Do not start
unrelated work in the same submission.

## Channel state

{{What's queued next for the sitting, or "nothing further queued" —
an active sitting should never be left guessing whether to poll.}}
