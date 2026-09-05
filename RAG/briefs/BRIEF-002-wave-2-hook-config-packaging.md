# Assignment: Wave 2 — AI-IMP-003 · AI-IMP-004

Review Lead → Code Lead, 2026-09-05. One sitting, two tickets, one
domain: making the accepted generator LIVE — trigger, config,
install, and verification that the owner's real setup is the
canonical base. Owner rulings are settled and in SPEC rev 0.6; this
sitting is AUTHORIZED. Protocol as before: isolated clone, branch
`imp/003` (one atomic commit per ticket, 004 may ride the same
branch after 003's commit), inbox submission with carrier submit,
destructive-op fence.

## Round 1 is a PRE-IMPLEMENTATION REVIEW — no code

Both tickets predate SPEC 0.3–0.6 and the merged generator. Verify
claims against current source and the owner's LIVE setup
(read-only!), report corrections, propose repair scope. Check
explicitly:
- The hook pipeline `rmpc albumart -o - | rmpc-theme-gen | rmpc
  remote --pid $PID set theme -` against rev 0.6 persistence
  (write-then-apply: the pipe shape changes — generated doc goes to
  a file first, then applies via IPC).
- v1's hook skeleton (.archive) for what actually ports; MIME dance
  is dropped.
- `rmpc albumart` exit codes 0/1/2/3 against installed rmpc 0.11.
- Config surface (TOML, ~/.config/rmpc-auto-theme/): confirm which
  §5/§7 knobs are genuinely per-user vs SPEC-fixed; propose the
  minimal schema. Flags override file; malformed file = loud at
  generator start, silent skip in hook.
- IMP-004 (re-cut 2026-09-05, verify-and-close): read the new
  ticket text; the audit table format is yours to propose.

## Normative supplement (binding, all in SPEC rev 0.6)

- §7 persistence: generated theme persists to its OWN file
  referenced by config.ron; base theme never overwritten; write
  first, apply second; hash short-circuit valid across restarts.
- OSC 11: build it, ship it DISABLED by default.
- install.sh touches the owner's live config.ron ONLY behind an
  explicit consent flag (e.g. --wire-hook) with a timestamped
  backup; default run builds/installs and prints the manual line.
- §7 failure policy: exit 0 on every failure path; capped log;
  debuginfo is NOT a validator on 0.11.0.
- Owner's live pair is read-only to you; IMP-004 archives go to
  ~/git/personal-theming via submission, applied by the RL.

## Fences

- Domain: `hook/`, `src/config.rs` (+ wiring in main), `install.sh`,
  `README.md`, `tests/`, `fixtures/` as needed.
- Do NOT touch: `~/.config/rmpc/**` (read-only), `SPEC.org`,
  `RAG/INDEX.md`, queues, `.archive/`, the solver/emit modules
  except where config plumbs in.
- Owner-run items (live smoke: <1s retheme, same-album no-op,
  mid-run kill) go to `RAG/HUMAN-TESTING.md` via your submission
  text, not executed by you.
- Validation: `set -o pipefail`; full gate at tip (fmt, clippy -D
  warnings, test — read counts); shell tests in temp HOME;
  `validate-tickets.sh --changed` clean before submitting.

## Report contents

Per PROTOCOL.md plus: test counts per suite, the IMP-004 audit
table, exact install.sh invocation matrix (default / --wire-hook),
and candid friction notes. Carrier-submit every letter — it is part
of the send ritual.
