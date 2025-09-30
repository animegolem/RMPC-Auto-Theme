# Repository Guidelines

## Project Structure & Module Organization
- Core crate lives in `src/`, with `rmpc_theme_gen.rs` as the CLI entry point and helper modules (`color.rs`, `image_pipeline.rs`, `kmeans.rs`, `lib.rs`) covering color science, image prep, clustering, and shared exports.
- Automation scripts sit at the repository root: `build.sh` drives release builds and installs, while `on_song_change.sh` wires the binary into rmpc’s hook system.
- Documentation, requirements, and historical notes are tracked under `RAG/`; integration artefacts and sample outputs land in `test-results/` (keep large fixtures out of git and reference them via paths under `~/.config/rmpc`).

## Build, Test, and Development Commands
- `./build.sh` — wraps `cargo build --release` and reports binary size; append `install` to copy into `~/.local/bin` (respects `INSTALL_DIR`).
- `cargo check` — fast validation before opening a PR; mirrors the release feature set.
- `cargo fmt` and `cargo clippy -- -D warnings` — enforce formatting and lint hygiene before pushing.
- `~/.config/rmpc/theme-switcher/test-results/run-tests.sh` — runs integration smoke tests against sample album art and captures JSON + RON outputs.

## Coding Style & Naming Conventions
- Use Rust 2021 defaults: 4-space indentation, trailing commas where `rustfmt` inserts them, `snake_case` for files/functions, `PascalCase` for types, and `SCREAMING_SNAKE_CASE` for consts.
- Prefer expressive helper functions in the relevant module (`color::`, `kmeans::`, etc.) instead of adding utilities to the binary file; surface shared APIs through `lib.rs`.
- Keep CLI arguments defined via `clap` derive macros together near `main()` and document non-obvious flags inline.

## Testing Guidelines
- Extend unit coverage with `cargo test` (place module tests alongside implementation) and add regression fixtures under `test-results/` when bugs involve image inputs.
- When adding new integration samples, note the source path in `TEST-RESULTS.md` and update `run-tests.sh` if bespoke arguments are required.
- Capture before/after theme screenshots or contrast ratios for visual changes and attach them in the PR description.

## Commit & Pull Request Guidelines
- Follow imperative, lower-case commit subjects (`add kmeans centroid caching`); group related changes into a single commit when practical.
- Reference tracked work items from `RAG/` in the body (e.g., `Refs AI-IMP-007`) and link external tickets if applicable.
- PRs should state intent, testing performed, and any configuration steps; include logs from `run-tests.sh` when behaviour changes theme outputs.
