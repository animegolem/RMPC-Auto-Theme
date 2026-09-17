---
submission: brief-004-imp-006
type: verdict
branch: imp/006
commit: none
round: 2
---

# Round-1 verdict: all rulings issued; trace may start now

Sol — strong round 1. Numbered rulings; SPEC rev 0.11 follows from
me before implementation starts, but your round-2 trace needs none
of it and may begin immediately.

## V-1: v3 pair / genericization — APPROVED as proposed

Your seam list stands. Cava pane ruling: RETAIN `Pane(Cava)`,
commented out beside the commented hook vector, with a numbered
README setup procedure (fifo input required; note upstream's
default Pulse input does not exist on macOS). SPEC §6 default-base
amendment is mine in rev 0.11.

## V-2: DQ-4 — role table APPROVED; one mapping, one exemption

Your role table is ratified exactly as listed. Materialize slider
styles and cava `bar_color` before mapping — approved. Walker
extensions (quoted map keys, array indices, variant context,
custom-component preservation) — approved; test every surface.
The two literals you asked about: album-table white is CHROME —
map it (album style as you proposed). Level severity colors are an
EXPLICIT EXEMPTION — they are semantic (the color IS the signal;
an error-red rethemed to album-amber stops meaning error), so they
stay literal, recorded in SPEC rev 0.11 with that rationale.
"No literal color survives" becomes "no literal CHROME color
survives; semantic severity colors are exempt by ruling."

## V-3: publisher — architecture APPROVED; both rulings issued

Typed Rust snapshot/JSON emission, shell admission, blocking
single-flight with no lock/cache skip, sibling-temp atomic rename
under producer-owned `palette_path`, `--print-config palette_path`,
`--publish-current` via `rmpc song` through the same path,
short-lock token/sequence at admission with recheck before either
rename — all approved. Failed envelopes: identity only from
acquired data, and palette fields ABSENT, never null (grisaille
r12 finding R4 corrected my fold on exactly this — the contract's
absent-shape is law; your invalid-case tests should reject a
`roles: null` failed envelope).

Ruling (1): APPROVED — art acquisition must be addressed by the
ADMITTED file identity (MPD `albumart`/`readpicture` by URI), not
current-song. The mid-fetch mislabel you found is the
never-invent-identity class and gets the strong fence; the weaker
fence is rejected. Narrow SPEC §7 amendment (and the §3 diagram
note) in rev 0.11.

Ruling (2): CONFIRMED as you stated it — superseded attempts are
suppressed; every remaining admitted event publishes, including on
an analysis-cache hit.

## V-4: no-art STOP — ruled by splitting the two surfaces

Your refusal to regenerate `no-art.json` with unmeasurable pair
claims was correct and is the reason this goes back to the
contract table. The conflict resolves by separating what the
terminal shows from what the artifact claims:

- THEME surface: the owner's ruling stands unchanged — no-art
  applies the UNMODIFIED stock base (`reset`, terminal's own
  colors). SPEC §7 keeps rev 0.7's path.
- ENVELOPE surface: an artifact never claims colors it cannot
  measure. PROPOSED contract amendment (sent to Astra as r14
  today; not folded silently): no-art roles/verified_pairs are
  PRESENT iff the effective base theme is fully hex-resolvable;
  with `reset`/named colors they are ABSENT. Status stays
  `no-art` — distinct from `failed`: nothing failed, there is no
  art; consumer retention rules apply. The shipped default keeps
  `reset`, so default installs emit absent-role no-art envelopes.

HOLD the `no-art.json` regeneration until Astra rules on the
amendment. Everything else in your scope is unblocked.

## V-5: double fire — trace plan APPROVED

Two-instance delivery is credible (both rmpc processes
owner-confirmed as manually launched) and not yet proven — your
controlled-change trace with hook PID/PPID, env PID/FILE and
token/time is the right next step, then single-flight plus
per-invocation prefixed/line-buffered stderr. One constraint on
the fix: two live rmpc instances are a LEGITIMATE user pattern —
make concurrency safe, never forbid it.

## Sequencing

Round-2 trace: start now. Implementation: after SPEC rev 0.11
lands (mine, next). `no-art.json`: after Astra's amendment ruling.
Capture: saturated cover, all addendum-2d criteria, absent-shape
law included.

— notan, RL
