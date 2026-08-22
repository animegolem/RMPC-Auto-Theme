# Assignment: {{wave name}} — {{TICKET-A}} · {{TICKET-B}} · {{...}}

Review Lead → Code Lead, {{YYYY-MM-DD}}. One sitting, {{N}} tickets,
one domain: {{one sentence naming the theme that unifies the
range}}. Manage your own subagents as you see fit; the protocol in
`{{channel-dir}}/PROTOCOL.md` governs (isolated clone, branch
`{{prefix}}/{{sitting-id}}`, inbox submission, atomic commit per
ticket, destructive-op fence: you never delete
worktrees/branches/refs — the Review Lead owns cleanup).

<!--
A brief is SELF-CONTAINED: the Code Lead must be able to work
the whole range from this file plus the constitution, without the
design conversations. Assign EXTENDED ranges (whole epics where
possible) so a healthy sitting never runs dry of authorized work.
-->

## Round 1 is a PRE-IMPLEMENTATION REVIEW — no code

{{State which rulings post-date the tickets.}} Verify every ticket
claim against CURRENT source, report corrections with citations,
and propose the repair scope. The Review Lead answers with rulings in the
outbox; you code against the verdict where ticket text differs.
Known staleness to check explicitly: {{line numbers, moved seams,
anything merged since the tickets were cut}}. Trivial tickets in
the range may skip to build — name them and say so.

## The tickets (in build order)

1. **{{path/to/TICKET-A.md}}** — {{one-paragraph diagnosis;
   what "done" means; why it builds FIRST if order matters}}.
2. **{{path/to/TICKET-B.md}}** — {{...}}

## Normative supplement (post-dates the tickets — binding)

<!-- Rulings folded into the constitution AFTER the tickets were
cut. The brief outranks stale ticket text; tickets sync after the
round settles. Cite constitution sections. -->

- **{{§ref}}**: {{ruling, stated operationally — what to build,
  what NOT to "fix", what copy/behavior is ratified verbatim}}.
- **{{§ref}}**: {{...}} If {{a seam}} makes this expensive, say so
  in round 1 rather than building it degraded.

## Fences

- Domain fence: {{directories/surfaces in scope}} and their tests.
- Do NOT touch: {{explicit list — always include whatever the
  lead is building in parallel (collision risk), the ticket index
  and human-testing/design queues, and any freshly-pinned tests}}.
- Reserved identifiers: {{migration/schema numbers assigned to
  this range, or}} none assigned — if you believe you need one,
  STOP and say so in a submission; do not take a number.
- Validation: `set -o pipefail` on every chain;
  {{exact full-gate command}} at the tip — read counts, not exit
  codes; report the failed line if any exists;
  {{build command}} before any integration run;
  `scripts/validate-tickets.sh --changed` clean before submitting.

## Report contents (each round)

Per PROTOCOL.md, plus: counts for every suite you ran
({{suite list}}), {{any range-specific artifact — policy-matrix
diffs, before/after measurements}}, and candid friction notes —
they feed the wave retro.
