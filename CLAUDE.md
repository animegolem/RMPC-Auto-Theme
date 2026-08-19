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
