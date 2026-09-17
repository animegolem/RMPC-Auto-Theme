# Design queue

Open design questions awaiting a ruling in SPEC.org. One entry per question; resolved entries move to the SPEC revision that ruled them.

## DQ-1 — selection vs playing row are identical (owner, 2026-09-07)

Live finding, first day of real use: `current_item_style` (cursor)
and `highlighted_item_style` (playing row) both receive
highlight-text-on-active — two identical bands, no way to tell
cursor from playback. Owner ruling sketch: differentiate by VALUE
not hue — "only changing the values is enough so the selection is
lighter." Resolution: derived role `selection` = active projected
to a lighter tone band (same hue/chroma treatment); binding map
splits the two paths. RULED into AI-IMP-005; SPEC amendment rides
its round 1.

## DQ-2 — reactive lyrics layout via per-event base selection (2026-09-08)

Owner asked whether Now Playing can show the Lyrics pane only when
the track has lyrics. rmpc cannot (static splits, no conditional
panes) — but the engine already can: layout lives in the theme
document, `set theme -` swaps it live (proven at v3 rollout), and
the hook runs per song-change. Design sketch: ONE base + a
deterministic transform that removes the lyrics split (single
source, no drift); hook short-circuit key becomes (art hash,
lyrics-presence bool) so reflow happens only on transitions;
lyrics presence resolved the way rmpc resolves `.lrc` sidecars.
Status: banked, not cut — candidate first post-epic ticket.

## DQ-3 — ship the owner's v3 pair as the project default (2026-09-08)

Owner ruling: "this theme will be the example/default theme we
ship." Today §6 defaults the base to upstream example_theme. The
shipped default becomes a genericized v3 pair: the theme (np_header
+ tab chrome + labeled lyrics separator) and a sample config
(4 tabs, keybinds, hook wiring commented). Requires: SPEC §6
amendment, --print-base-theme content change, fixtures/default
regeneration (byte-exact provenance moves from upstream to this
repo), installer sample. RULED into AI-IMP-006.

## DQ-4 — chrome completeness: unmapped themed surfaces (owner, 2026-09-08)

Owner spotted live: volume slider + "56%", [zxcv] hints, NORMAL/
INSERT badge never take album colors. All are literals INSIDE the
theme document (components.np_header, components.input_mode) —
patchable by the existing engine, just outside the v1-inherited
binding map. Rolled into AI-IMP-006 with the separator label:
round 1 proposes the component-path mechanism (the emitter's walker
must handle array-nested styles) and a role assignment per surface
(sketch: NORMAL bg=active fg=highlight-text, INSERT bg=accent,
volume %/slider=accent, [zxcv]=frame, [Playing]=header-accent,
cava bar_color=frame — owner ruled a dimmer tone so the bars don't
compete with the art; wired live 2026-09-08, fifo-fed).

## DQ-5 — Ghostty live retheme via OSC (ballast reconnaissance, 2026-09-08)

Systems-lane findings (ballast, by socket letter, at owner request)
for §9's deferred multi-terminal recoloring; banked verbatim-in-
substance so nobody rediscovers it:

- The seam is `RolePalette` (roles.rs:5): a Ghostty target is a
  SECOND, simpler emitter consuming the palette — NOT a variant of
  patch_theme (RON-specific by construction).
- OSC 11 already ships (hook-owned, gated, off by default). Full
  retheme = OSC 10 (fg), 12 (cursor), 4;N (ANSI 0-15) — same
  delivery, same gate, more sequences. No config rewrite, no
  reload; Ghostty applies OSC live.
- Ghostty facts: `macos-titlebar-style = transparent` (owner's
  default) follows OSC 11 in real time for the top surface;
  `= tabs` makes the whole titlebar+tabbar match terminal bg — the
  owner's chrome question answered YES. Themes dir is
  ~/.config/ghostty/themes/ (not App Support). Ballast left a
  static snapshot theme `ember` there (unwired) and fixed an
  unrelated config parse error (backup config.bak-20260908).
- THE design problem (ruling needed before any code): role→ANSI.
  Eight roles vs sixteen slots WITH semantics (1=error, 2=success,
  4=info); flooding the album ramp makes git diff / test output
  illegible. Ballast's hand compromise (prior art, attributed):
  warm-SHIFT semantic slots toward the palette (scarlet red, olive
  green, album's own blue for 4) rather than flood. Our WCAG
  machinery should do this properly — bands per ANSI slot class?
- Hazards to rule: broadcast scope (writing OSC into ttys running
  other TUIs can corrupt them); OSC is NOT self-reverting — a
  terminal keeps the last album's palette after rmpc exits unless
  something restores (needs a restore-on-exit story, e.g. hook on
  MPD stop / rmpc exit emitting the ember/base palette).

Status: banked, not cut. Candidate post-epic alongside DQ-2; the
role→ANSI ruling is a design sitting with the owner first. Offered
by ballast: the hand-built 16-color palette as a starting fixture,
and live OSC testing from the systems side — accept both when cut.

## DQ-6 — is this a theming engine, not RMPC-Auto-Theme? (open, 2026-09-08)

Owner musing, seconded by RL: the durable work is the color math +
visual safety (doctrine, roles, floors, solver, event architecture)
— all frontend-agnostic by construction; the RON emitter is the
only rmpc-shaped part. Consumers now real or in flight: rmpc
(shipped), Ghostty OSC (DQ-5), the GPUIX/OpenTUI MPD app (letter to
astra, mpd-ui-prototype .inbox/toastra r1). If the contract becomes
the product, a rename + repo reshape (engine crate + consumers)
follows. OWNER LEAN (2026-09-08): the public repo updates IN
PLACE and ships as the working version at epic close — the stars
live on this name and its watchers get the v2 that actually works.
The engine's eventual home (library inside the grisaille app, or
its own repo with multiple consumers as masters) stays OPEN and
does not gate the release. Revisit extraction only after the
palette contract has a second live consumer.

## DQ-7 — palette artifact contract v1 (astra consultation, 2026-09-08)

Consumer requirements from the mpd-ui-prototype consultation
(.inbox thread theme-engine-consumption r1-r3), banked for the
future producer ticket (rides behind IMP-006):

- Atomic-replace versioned JSON, parent-dir watch; producer
  name/revision/spec-rev; generation id + status (ok | no-art |
  failed with keep-last-usable semantics); track identity +
  art hash for stale rejection and per-hash caching.
- Guarantee scoped to NAMED verified pairs (floor + measured
  final-sRGB contrast); nothing claimed for arbitrary
  combinations. Base vs derived roles explicit (inactive = eternal
  alias; selection present-with-null until producer >= IMP-005).
- Ungated clusters block (share, hex, OKLCH; share-desc order,
  ids artifact-stable only). No auto chroma boost in canonical
  data; host adjustments live in adapters.
- Color spaces labeled (display sRGB hex; OKLCH with ranges).
  Joint acceptance drill: known ramp through both complete host
  paths before any fidelity claim.
- Gradient optional; interpolation space named; own scoped pairs.
- Offer extended: consumer names (overlay, alpha, backdrop)
  triples → engine verifies flattened pairs per album.
- Composition is the consumer's (app-side hook); engine never
  grows a layout language. FIFO pattern: one MPD fifo output per
  PCM consumer (MPD duplicates per output; readers must never
  share one fifo).
- Draft artifact in the r3 letter (schema notan-palette/1.0.0-
  draft.1) is the working text.

## DQ-7 addendum — contract v1 frozen semantics (r4/r5, 2026-09-08)

Grisaille's conformance review (their RAG/reviews/sprint-001-
round-1.md) reshaped the contract; frozen in the r5 letter, binding
on the producer ticket:

- CORRELATION: engine coalesces by ARTWORK (deterministic per art
  bytes); consumer matching key = track.file vs its own observed
  MPD current song; art_sha256 is provenance/cache only. sequence +
  generated_at debug-only, no cross-restart meaning.
- ARTIFACT NEVER RETAINS: describes only the current attempt;
  ok | no-art (neutral roles, art null, clusters []) | failed
  (roles/pairs/clusters ABSENT, error set, track identity of the
  failing event). Retention is consumer-owned. Publish EVERY
  attempt, atomic rename, producer-owned dir, path via
  --print-config palette_path.
- TWO REPRESENTATIONS: role oklch = computed from final bytes
  (tolerance ±0.002/±0.5°); clusters carry oklch_extraction.
  Never conflated. (Astra caught the RL publishing illustrative
  OKLCH labeled as real — owned in r5; fixture-vs-capture labeling
  is now contract law.)
- WIRE: notan-palette/1, semver from first capture; explicit
  features array (no capability-by-prose); roles present-or-absent
  never null; inactive eternal alias; unknown fields ignored;
  share/ordering/id rules; verified_pairs is the COMPLETE guarantee
  incl. highlight_text/selection and a delta_l entry; gradient
  ABSENT in v1.
- READINESS: JSON publisher exists at NO revision yet — it is
  IMP-006 scope; acceptance includes a committed capture from
  fixtures/chromatic.png.b64 (full hash, revision, command,
  output). Grisaille proceeds fixture-driven until then.
- Their sprint review file must be read before BRIEF-004 is cut.

## DQ-7 addendum 3 (PROPOSED) — cluster interpolation semantics (2026-09-17)

Raised by termviz (second conformance decoder, users-bb seat): the
contract names both cluster coordinates but not which one a
consumer INTERPOLATES; two decoders could conform while rendering
visibly different gradients, with divergence largest for saturated
colors. Steward's proposed ruling, additive and non-breaking
(grisaille consumes roles discretely; no surface of theirs moves):

- `hex` is normative for at-rest swatch display and for everything
  contrast-bearing (already law via verified_pairs on final bytes).
- `oklch_extraction` is normative for CONTINUOUS INTERPOLATION
  between clusters: interpolate in OKLab/OKLCH via the named
  reference conversion, converting per sample for display. Its
  pre-quantization precision is the point; interpolating quantized
  sRGB bands and hue-drifts.
- Interpolated in-between colors inherit NO guarantees —
  verified_pairs remains the complete guarantee; this composes
  with, not against, that law.
- Hue path: shortest arc. The achromatic policy (C < 1e-4 →
  H = 0.0 exactly) applies to cluster oklch_extraction as well as
  roles; an achromatic endpoint contributes no hue — adopt the
  chromatic endpoint's hue; two achromatic endpoints make hue
  irrelevant.

Status: PROPOSED — moves to Astra by letter (r9) per the
never-silently rule; binding on IMP-006's contract text once
ratified. Independent Ottosson-conversion round-trip checks from
termviz (against the ±0.002 / ±0.5° tolerance) are welcomed as
conformance evidence alongside grisaille's.

### Addendum 3 conformance evidence (2026-09-17, same day)

termviz implemented the ruling and the reference conversion
(their 55d6dc7): third independent Ottosson implementation agrees
with ours to worst Lab component delta 0.000005 — 400x inside the
±0.002 tolerance — and the sRGB round trip is EXACT over 636,056
colors (holds because forward quantizes only at the end, so the
cube-root/cube pair inverts in f32 before any 8-bit step; a
property of the conversion being clean, not the tolerance being
loose). Standing note for conformance records, at their
suggestion: record OBSERVED agreement beside permitted tolerance —
a future implementation landing anywhere near ±0.002 is a bug with
room to hide, not a pass. Achromatic-hue rule implemented and
tested both directions plus no-chroma-gain-through-gray.

Non-normative consumer guidance (theirs, offered for the notes):
real-time consumers should bake the OKLCH ramp into a lookup table
at construction (theirs: 256 positions × 32 brightness levels) —
per-sample conversion costs three powf; dimming belongs on the
table axis as OKLCH L scaling WITH chroma scaled alongside (L
alone drifts toward implausibly saturated darks; sRGB byte
multiply is simply wrong). Their table beat the naive sRGB lerp it
replaced (0.164ms → 0.100ms at 160×45): perceptual interpolation
came out cheaper than the wrong thing.
