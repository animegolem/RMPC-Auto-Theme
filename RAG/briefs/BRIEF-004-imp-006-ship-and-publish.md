# Assignment: Wave 4 — AI-IMP-006 (ship v3 default + chrome completeness + palette publisher + hook single-flight)

Review Lead → Code Lead, 2026-09-17. Owner authorized this sitting
today. Three things converged on this one ticket this week: termviz
arrived as the palette's second live consumer (the DQ-6 trigger),
grisaille's genuine-capture gate (their F2) sits behind the
publisher, and termviz's tracing surfaced the hook double-invocation
(observation banked, reproduced with counts). IMP-005 is closed:
merged at 303d036, owner visual ruling recorded as SPEC rev 0.10,
gates green 2026-09-17 (fmt/clippy clean, 28 tests).

## Scope, one sitting

AI-IMP-006 as ticketed, all three addenda binding:

1. **Ship v3 default** (original scope + DQ-4 widening): genericized
   owner pair as the embedded base + sample config; 100%-mapped
   chrome — component-path patching so no literal color survives a
   full retheme (volume slider/percentage, [zxcv] hint, NORMAL/
   INSERT badge, separator label stays frame-colored per the
   withdrawn addendum-1 instinct).
2. **notan-palette/1 publisher** (addendum 2, all five criteria):
   publish EVERY accepted event with fresh identity + sequence
   (same-art track change publishes — the r6 hole stays closed),
   atomic rename into the producer-owned dir, --publish-current
   one-shot, admission-token ordering incl. failures, failed status
   with never-invented identity, and the genuine capture from
   fixtures/chromatic.png.b64 committed as conformance golden (full
   hash, revision, command, unedited output).
3. **Hook single-flight** (addendum 3): trace the double fire's
   source with EVIDENCE FIRST — two rmpc instances vs. double event
   delivery; no guessed fix — then single-flight guard around
   extraction + stderr log discipline. One hook revision carries
   this plus publish-per-event plus --publish-current; do not split.

## Round 1 (no code)

Propose: (a) the exact genericization diff — personal vs shippable
in the owner pair — and fixture/golden blast radius; (b) the
component-path patching design for the DQ-4 chrome class;
(c) publisher emission design: where the envelope writer lives
relative to the generator, atomicity mechanics, config surface
(palette_path via --print-config), and how the single-flight lock
composes with publish-per-event so a lock skip can never eat a
publication; (d) your double-fire trace findings. I rule, SPEC
amends where needed, round 2 implements.

## Fences

src/, tests/, fixtures/, install.sh, hook/ for this ticket;
Cargo.toml only with a stated need. SPEC.org via RL only.
contract/notan-palette-v1.md is FROZEN pending grisaille
ratification — no edits; DQ-7 addendum 3 (cluster interpolation) is
PROPOSED, so capture provenance notes name its status at capture
time, nothing more. Owner live surfaces read-only, as ever. Engine
code never copied from color-tool-kmeans.

## Gates

Full gate (cargo fmt --check && cargo clippy -- -D warnings &&
cargo test) + validate-tickets --changed + the concurrent-invocation
test from the ticket checklist. Branch `imp/006`. Carrier-submit
every letter. Reproduce the counts before any claim — the capture
especially: it is the artifact two consumer shops will decode.
