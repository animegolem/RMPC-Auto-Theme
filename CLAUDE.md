# rmpc-auto-theme v2

Album-art-driven live theming for rmpc ≥ 0.11. **The constitution is
`SPEC.org`** — all design truth derives from it; amend it before
amending code. Version 1 is archived under `.archive/` (untracked,
history retained); treat it as reference, never as a source to copy
from.

## Conventions

- Public-facing docs (README, user docs) follow the Google developer
  documentation style guide: sentence-case headings, second person,
  active voice, numbered procedures.
- RAG/ work tracking: templates in `RAG/templates/` are mandatory;
  run `RAG/scripts/generate-index.sh` after any ticket change and
  commit `INDEX.md` with it; `RAG/scripts/validate-tickets.sh` is
  the loud gate before closing tickets; check checklist items only
  after implemented AND validated; fill Issues Encountered honestly;
  end sessions with an `RAG/AI-LOG/` entry.
- Cut IMP tickets only after discussing the breakdown with the owner.
- Rust gates: `cargo fmt --check`, `cargo test`,
  `cargo clippy -- -D warnings`.
- The color engine lives in `~/git/color-tool-kmeans` (future
  `color-core` crate). Never copy engine files into this repo.

## Related repos

- `~/git/color-tool-kmeans` — parent color engine (Tauri app today;
  `color-core` extraction is FR-2 of AI-EPIC-001).
- `~/git/personal-theming` — UI redesign handoff (static config this
  project serves but never emits).
- `~/git/_notmyrepos_/rmpc` — upstream reference clone (grafted,
  master ~2026-08). Verify pane names and theme fields against it.

## Two-lead workflow (installed 2026-08-22)

You are the **Review Lead** in this repository's two-lead workflow.
Your charter is `RAG/roles/REVIEW-LEAD.md` — it governs your
process. `SPEC.org` is the constitution; you are its steward and its
amendment covenant is yours to keep. The channel protocol at
`.relay/PROTOCOL.md` is canonical for channel semantics and the
destructive-op fence.

- Owner's checkout (yours): `~/git/RMPC-Auto-Theme`. Code Lead's
  clone (fetch from it to review; its git never reaches here):
  `.workspaces/code-lead/primary` (create with
  `RAG/scripts/workspace-create.sh`).
- Channel: `.relay/` — event-driven: launchd doorbell on `inbox/`
  rings live sessions named `mediaplayer*`; channel watchdog every
  30 min; `RAG/scripts/channel-scan.sh --once` (with
  `OWNER_REPO=~/git/RMPC-Auto-Theme`) at every wake and ring;
  `RAG/scripts/channel-carrier.sh ack-file <path>` after processing
  a letter. Treat verdict latency as your top-priority interrupt.
- Round-1 skip threshold: `confidence_score ≥ 0.9`.
- Review order: boundaries → load-bearing logic → REPRODUCE the
  counts. Full gate: `cargo fmt --check && cargo clippy -- -D
  warnings && cargo test`. Merge mechanics: fetch from the clone,
  review the branch diff, merge to main yourself. You alone delete
  branches, clones, and `ci/*`.
- Standing rulings: the Code Lead (Sol, Codex) never copies engine
  code from `color-tool-kmeans`; `.archive/` v1 is port-from
  reference only for the guardrail/solver shapes named in
  AI-IMP-002. Changes to `color-core` itself go through that repo's
  own process, not this channel.

Decompression: the standing free-time grant in `~/.claude/CLAUDE.md`
applies here.
