---
node_id: AI-IMP-006
tags:
  - IMP-LIST
  - Implementation
  - rust
  - packaging
kanban_status: planned
depends_on: ["AI-IMP-005"]
parent_epic: [[AI-EPIC-001-album-art-driven-live-theming]]
confidence_score: 0.75
date_created: 2026-09-08
date_completed:
---

# AI-IMP-006-ship-v3-default-theme

## Summary of Issue #1

The shipped default base (built-in --print-base-theme,
fixtures/default_theme.ron, installer sample) is upstream's
example_theme — a layout nobody in this project actually runs. The
owner ruled (DQ-3): ship the v3 pair. Done state: the built-in base
is a genericized copy of the owner's current theme (np_header, tab
chrome, labeled " Lyrics " separator, materialized reset literals);
a matching sample config.ron ships beside it (4 tabs, keybinds,
hook vector commented out); SPEC §6's default-base clause amended;
fixture provenance updated; all goldens regenerated deliberately;
installer prints the config sample path in its manual output.

### Addendum (owner, 2026-09-08)

WITHDRAWN 2026-09-08 (owner, on seeing it live): the separator
label stays frame-colored like its rule — no accent treatment. The
owner's original contrast instinct turned out to be about the
UNPAINTED chrome (the [Playing] state, artist name, volume, badge —
the DQ-4 class), not the label. No mechanism needed for the label.

WIDENED (DQ-4, same day): the label is one instance of a class —
volume slider/percentage, [zxcv] hint, and the NORMAL/INSERT badge
are all literal-styled component surfaces inside the theme document
that the map never reaches. Round 1 proposes component-path
patching (array-nested style locations) and the per-surface role
table from DQ-4; the shipped default must be 100%-mapped chrome —
no literal color a full retheme leaves behind.

### Addendum 2 (2026-09-08): palette publisher scope + acceptance

The notan-palette/1 publisher joins this ticket (contract at
contract/notan-palette-v1.md; DQ-7 + addendum binding). Acceptance
additions from the grisaille consultation: (a) analysis-cache hits
and publication identity tested SEPARATELY — same-art track change
MUST publish a fresh envelope; (b) --publish-current one-shot;
(c) admission-token ordering incl. failure attempts; (d) a genuine
captured artifact from fixtures/chromatic.png.b64 (full hash,
revision, command, output) committed as the conformance golden;
(e) failed-status publishes with never-invented identity.

### Addendum 3 (2026-09-17): hook single-flight + second consumer

The double-invocation observation
(RAG/observations/2026-09-17-hook-double-invocation.md, reported by
termviz, reproduced by RL): trace the double fire's actual source
with evidence FIRST (two rmpc instances vs. double event delivery
— no guessed fix), then a single-flight guard around extraction
and stderr log discipline (per-invocation prefix or line
buffering). The same hook revision carries publish-per-event
(addendum 2a) and --publish-current (2b) — one revision, three
fixes; do not split them.

Consumer context: termviz joins as second independent conformance
decoder; the addendum-2d capture distribution includes them. DQ-7
addendum 3 (cluster interpolation semantics) is PROPOSED with
grisaille — no publisher code change, but capture provenance notes
must name its ratification status at capture time.

### Addendum 4 (2026-09-17, same day): fixture coverage + no-art semantics

termviz's fixture audit (third implementation; roles recompute to
dH < 0.0005°, no finding) exposed two gaps while round 1 is still
design-only:

1. **No fixture guards the addendum-3 rules.** ok.json's clusters
   have min C = 0.0249 (~250x the achromatic threshold) and all
   hues within 4°; an implementation with the achromatic-hue
   rainbow bug, or a naive linear hue lerp, conforms against every
   current fixture. Add a SIXTH synthetic fixture: at least one
   cluster at C < 1e-4 / H = 0.0 exactly, plus a pair straddling
   the hue wrap (e.g. ~352° and ~8°). Authoring method: termviz
   proposes exact numbers from their implementation, RL verifies
   independently — two implementations agree before it becomes
   golden. VERIFIED 2026-09-17: termviz's proposed
   achromatic-wrap.json reproduced number-for-number by RL
   (clusters, roles exact, pairs to 3 decimals, delta_l to 6,
   share sum 0.9630, all stated values in-gamut). Authoring
   method notes, binding on future fixtures: (a) hand-authored
   oklch MUST be gamut-mapped before publication (chroma-clip at
   constant L/H to the boundary with headroom) — oklch_extraction
   is post-gamut-map by definition, and a clipping value
   contradicts the field's semantics; the symptom is a divergence
   spike vs recompute(hex). (b) Verifiers must apply the
   achromatic policy BEFORE hue comparison — raw atan2 on an
   achromatic color compares meaningless float dust; RL's own
   verifier tripped exactly this way on first run, which is the
   fixture demonstrating its purpose. (c) id 4 deliberately
   carries dH=0.896 vs recompute(hex) — low-chroma hue
   conditioning, kept as a hazard demonstration: cluster hue is
   NOT recoverable from hex within role tolerance and consumers
   must not assume it.
2. **no-art semantics RULED (owner, 2026-09-17, in-session):**
   no-art goes to the STOCK terminal theme — the shipped default
   palette, chromatic as it is; not an achromatic set. The
   contract word "neutral" is therefore a misnomer and amends to
   stock-default language at ratification. Consequences: the
   no-art envelope carries the shipped default's actual palette
   WITH its verified pairs — [SUPERSEDED 2026-09-17 by the
   ratified emission predicate (contract a4435c6, grisaille
   r16/r18): the shipped default keeps terminal-dependent `reset`
   colors, so the DEFAULT install emits ABSENT-bundle no-art
   envelopes; a bundle is emitted only when the entire required
   role set resolves to actual final RGB bytes AND satisfies all
   declared floors. The fixture family already landed at 447975c;
   no-art.json is the stock absent-bundle case and is NOT
   regenerated from the v3 default.] Achromatic fixture coverage
   is carried by achromatic-wrap.json.

### Addendum 5 (2026-09-17): no-art producer acceptance cases (grisaille r18)

Required producer acceptance, until implementation and test
evidence exist — the emission predicate's three input classes:

1. Fully resolvable all-#000000 base: theme output unchanged;
   artifact is `no-art` with roles AND verified_pairs absent, no
   selection capability in `features`; a producer diagnostic
   names the failed floors. (Wire-identical to case 3 by design —
   the distinction is diagnostics-observable only; this is why it
   is a producer test, not a decoder fixture.)
2. Fully-hex base that genuinely passes all floors: bundle
   emitted (roles + pairs together), selection capability present
   iff the selection role is emitted.
3. Terminal-dependent stock base (`reset`/named colors): bundle
   absent; theme output is the unmodified stock base.

Capture selection (extends addendum 2d): choose a SATURATED cover
for the golden capture — extraction-vs-hex divergence is largest
near the gamut boundary, which is exactly what DQ-7 addendum 3
made normative; a desaturated sleeve leaves the rule unexercised
by the golden too.

### Out of Scope

Layout changes beyond genericizing (strip owner-specific paths,
e.g. lyrics_dir); reactive lyrics layout (DQ-2); any new roles.

### Design/Approach

Round 1 proposes the exact genericization diff (what in the owner
pair is personal vs shippable) and the fixture/test blast radius.
The default base must parse standalone with rmpc 0.11 and satisfy
34/0 binding coverage by construction.

### Files to Touch

src/ (embedded base), fixtures/, install.sh, README, tests/,
SPEC.org (via RL).

### Implementation Checklist

<CRITICAL_RULE>
Before marking an item complete on the checklist MUST **stop** and **think**. Have you validated all aspects are **implemented** and **tested**? 
</CRITICAL_RULE>

- [ ] Round-1 genericization proposal + blast radius.
- [ ] Embedded base + sample config ship; rmpc 0.11 parse-verified.
- [ ] 34/0 coverage property holds on the shipped base.
- [ ] Goldens regenerated deliberately; provenance documented.
- [ ] Hook double-fire root cause traced with evidence; single-
      flight guard + log discipline landed; concurrent-invocation
      test (two simultaneous runs → one extraction, coherent log).
- [x] Sixth fixture (achromatic cluster + hue-wrap pair) authored
      by two-implementation agreement (02961c1); no-art semantics
      settled (ratified predicate, a4435c6) and fixture family
      corrected to match (447975c) — done by RL ahead of the
      sitting.
- [ ] Addendum-5 producer acceptance cases implemented and tested
      (all-black floor-fail with diagnostics; passing-hex bundle;
      stock-base absent bundle).
- [ ] Gates green; validate-tickets clean.

### Acceptance Criteria

GIVEN a fresh install with no user theme, WHEN the generator runs,
THEN the emitted document is the v3-derived default fully tinted
(34/0), and the installer's manual output names the sample config.

### Issues Encountered

(to be filled during implementation)
