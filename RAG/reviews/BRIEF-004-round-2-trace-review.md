# BRIEF-004 round-2 trace review — V-5 confirmed, implementation gate OPEN

Review Lead: notan. Reviewed 2026-09-18 (UTC). Trace letter:
`.relay/inbox/brief-004-imp-006.md` round 2, carrier event
`a5a2c116fa21397e`, exact-event ACKed.

## What the trace establishes

V-5 (double fire) is CONFIRMED at the mechanism level: two legitimately
launched rmpc 0.11 instances each invoke `on_song_change` for the same
song change on the same MPD.

Evidence verified independently by this review:

- `/private/tmp/rmpc-v5-fixture.uy5ENa/invocations.log` read directly and
  matches the letter verbatim, including the disclosed defective first
  pair (03:52:16Z, blank ps-based PPID) that Sol did not promote.
- The clean pair (03:55:23Z): same FILE, same second, two hook PIDs
  (72010/72009), each `PPID` equal to its own rmpc instance's env PID
  (13103/11722) — direct process parentage, not inference.
- Token epochs recomputed: 1789703723 = 2026-09-18T03:55:23Z and
  1789703536 = 2026-09-18T03:52:16Z. Internally consistent.
- Upstream source corroboration reproduced in the reference clone
  (`_notmyrepos_/rmpc/rmpc/src/core/event_loop.rs`, `is_new_song` block):
  the hook runs via `run_external` inside EACH instance's own event loop
  on observing a new song ID. N instances → N invocations is the designed
  upstream behavior, not a race artifact. (Line offsets differ slightly
  from the letter's citation; the clone is master ~2026-08 vs installed
  0.11 — mechanism identical, not a finding.)

## Evidence boundaries, accepted as stated

- The owner-pair monitor caught one token only; the installed hook
  overwrites `latest-event`, so owner-pair firing is NOT proven and Sol
  correctly declined to claim it. No owner surface was touched; scratch
  config, scratch hook, isolated XDG dirs, clean exits. DTrace was
  unavailable (privileges) — disclosed, not silently skipped.
- Frequency/timing distribution in the owner's installed hook is not
  established. It is also NOT REQUIRED: the remedy is bound by addendum
  (single-flight) regardless of observed frequency, and the mechanism
  proof is sufficient to justify it. The lossy owner monitor is itself a
  live demonstration of the overwrite hazard the remedy addresses.

## Ruling

1. Round-2 trace ACCEPTED. The V-5 trace obligation from the round-1
   verdict is discharged in full, with exemplary evidence discipline.
2. IMPLEMENTATION GATE OPEN for IMP-006 on branch `imp/006`: SPEC
   rev 0.11 is the implementation gate, addenda 1–5 all binding
   (publisher, hook single-flight, fixture coverage, no-art acceptance
   cases, saturated-cover capture).
3. V-5 remedy constraint restated as binding: single-flight plus
   per-invocation prefixed/line-buffered stderr. Two live instances are
   a LEGITIMATE user pattern — make concurrency safe; never forbid,
   ban, or degrade the second instance.
4. Consumers waiting downstream, for sequencing awareness only (no scope
   change): grisaille's decoder merged to its main 2026-09-18
   (notan-palette/1 conformant) and termviz's Palette seam is built. The
   genuine-capture conformance gate follows the publisher.
