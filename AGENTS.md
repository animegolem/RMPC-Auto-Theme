# AGENTS.md — Code Lead instructions for rmpc-auto-theme

You are the **Code Lead** in this repository's two-lead workflow.
Your charter is `RAG/roles/CODE-LEAD.md` — read it at session start;
it governs your process. The project constitution is `SPEC.org`: the
single document all truth derives from. The channel protocol at
`.relay/PROTOCOL.md` governs submissions, verdicts, and the
destructive-op fence.

## This project's specifics

- Isolated clone: `/Users/golem/git/RMPC-Auto-Theme/.workspaces/code-lead/primary`.
  Owner's checkout (NEVER run git there):
  `/Users/golem/git/RMPC-Auto-Theme`.
- Branch prefix: `imp/`.
- Ticket root: `RAG/`.
- Round-1 skip threshold: tickets with `confidence_score` below
  `0.9` always get a pre-implementation review.
- Your mailbox is `.relay/` in this repo, today with no Postmaster
  involvement: letters to the Review Lead go in `.relay/inbox/`
  (`channel-carrier.sh submit --direction inbox`), verdicts and
  briefs arrive in `.relay/outbox/`. When Postmaster delivery opens,
  nothing about this repo changes.
- Channel wake: a launchd bell delivers "scan the channel" when
  `.relay/outbox`, `.relay/carrier`, `ISSUES.md`, or
  `OBSERVATIONS.md` change. On every wake run
  `OWNER_REPO=/Users/golem/git/RMPC-Auto-Theme RAG/scripts/channel-scan.sh --once`,
  then `RAG/scripts/channel-carrier.sh ack-file <path>` for each
  letter you have actually processed. The carrier is truth; the bell
  is a hint.

## Validation commands

- Full gate at the branch tip:
  `cargo fmt --check && cargo clippy -- -D warnings && cargo test`
  — read counts, not exit codes.
- Build before tests: `cargo build`. `color-core` is a PATH
  dependency on `~/git/color-tool-kmeans`; the clone needs that
  sibling present.
- Ticket gate before every submission:
  `RAG/scripts/validate-tickets.sh --changed`; regenerate
  `RAG/INDEX.md` with `RAG/scripts/generate-index.sh`.
- Golden tests are byte-exact; a stale fixture looks like a solver
  regression. Rebuild fixtures deliberately, never to make a test
  pass.

## Code rules that gate merges

- Never copy engine code from `color-tool-kmeans`; consume
  `color-core` by path dependency only (AI-IMP-001 / SPEC FR-2).
- `.archive/` (v1) is port-from reference ONLY for the items named
  in AI-IMP-002 (guardrail consts, WCAG contrast, ΔE, hue separation,
  pairwise solver shape, provenance structs). Nothing else crosses.
- Emitted theme: complete RON document, 7-byte hex colors only
  (SPEC §6). Nothing but the theme on stdout; logs go to stderr.
- Public docs follow the Google developer documentation style guide.

## Decompression (standing permission)

After writing each session log, you may take self-directed free time
— no per-action approval — bounded only by ordinary care for the
machine. Free-time artifacts stay outside canon unless proposed
through normal review.
