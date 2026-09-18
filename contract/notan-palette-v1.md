# notan-palette/1 — palette artifact contract

Status: RATIFIED 2026-09-17 — grisaille r18, full ratification of
this folded revision at fixture head 447975c, document digest
pinned in that letter (sha256 fa0d0696…96bd). Amendment record: interpolation semantics +
no-art predicate + stock-default wording folded 2026-09-17 from
DQ-7 addendum 3 and letters r14-r16; every change letter-traced,
none silent. Synthetic-fixture stage: The producer (JSON publisher) exists
at no revision; it is AI-IMP-006 scope. Fixtures in `fixtures/` are
SYNTHETIC and labeled so; they are numerically self-consistent
(oklch computed from hex via the reference conversion below,
contrasts measured from final bytes). The first producer capture
ships with the publisher and becomes the conformance golden.

## Identity and correlation

The engine coalesces ANALYSIS by artwork; it publishes an ENVELOPE
per accepted player event. Analysis cache key: (art_sha256,
config_digest, producer_revision). Publication is NEVER skipped for
an accepted event: same-art track changes reuse the cached palette
under a fresh envelope carrying the new `track.file` and a new
`sequence`. Consumers match `track.file` against their own observed
MPD current song. File equality establishes association with a
library URI at `generated_at` — it is NOT verified current-art
equivalence; artwork bytes are re-fetched and re-hashed per event,
so changed embedded art invalidates the cache naturally on the next
event.

Ordering (producer-owned): a last-event-wins token is captured at
event admission; identity fields are bound to the SAME admission
(a mid-fetch track switch can never label A's bytes as B), and a
superseded attempt — success or failure — must not replace a newer
admitted attempt's artifact. `sequence` is monotonic per producer
process, debug-only, no cross-restart meaning.

Startup/reconciliation: the publisher provides a one-shot
`--publish-current` mode (query MPD for the current song, run the
normal path, publish). Invoked by the producer at its own startup;
also invocable by anything else as an explicit refresh — it is a
bounded CLI call, not a service. Consumers needing liveness after
their own restart may invoke it or wait for the next event; both
are legitimate.

## Publication

Default `~/.config/rmpc-auto-theme/palette/palette.json`; dir
producer-owned; path via `--print-config palette_path`. Write
`.tmp` sibling → atomic rename, for EVERY attempt incl. failures.
Consumers watch the parent directory and own retention/recovery.

## Statuses and required fields

Required in every artifact: `schema`, `schema_version`, `producer
{name, revision, spec_revision}`, `features`, `sequence`,
`generated_at`, `status`, `track`.

- `ok`: + `roles`, `verified_pairs`, `clusters`;
  `track.art_sha256` full 64-hex.
- `no-art`: `clusters: []`; `track.art_sha256: null`. The theme
  surface applies the UNMODIFIED stock base (owner ruling
  2026-09-17); the artifact never invents RGB or contrast claims
  for terminal-dependent `reset`/named colors. Emission predicate
  (grisaille r16, adopted verbatim): emit `roles` and
  `verified_pairs` TOGETHER only when the entire required role
  bundle resolves to actual final RGB bytes AND satisfies all
  applicable declared pair floors; otherwise omit BOTH, claiming
  no portable verified palette. Hex resolvability is necessary,
  not sufficient (an all-#000000 base resolves and fails every
  floor). A floor-failing resolvable base reports its reason in
  producer diagnostics, never in the artifact. Roles/pairs are
  both-present or both-absent — never null, never a partial
  bundle. `no-art` remains distinct from `failed`: nothing
  failed; there is no art; consumer retention rules apply, and
  retained colors are never relabeled as a verified current-track
  palette. The absent-clusters rule here is no-art's own
  (`clusters: []`), not `failed`'s absent-field shorthand.
- `failed`: + `error` (string). `roles`/`verified_pairs`/`clusters`
  ABSENT. `track.file` MAY be null when identity itself was
  unobtainable (e.g. MPD query failure) — never invented.

Unknown handling, separately specified: unknown top-level fields →
ignore. Unknown `schema` major → reject artifact, keep last usable.
Unknown `status` → treat as `failed` semantics (keep last usable),
log. Unknown `verified_pairs[].kind` → ignore that entry only.
`features` is the only capability authority.

## Roles

Canonical keys: background, text, accent, active, highlight_text,
frame, header_accent, inactive, selection. A role is PRESENT with
full data or ABSENT; never null. `inactive` always present, always
`{alias_of: "frame"}` + its color data. Alias exactness (addendum A3,
ratified 2026-09-18 on the RGB-bytes reading): an alias role's `hex`
MUST resolve to final RGB bytes identical to its source role's, and
its numeric OKLCH values MUST be identical to the source's. Hex
comparison is case-insensitive; JSON string spelling is not normative.
Per-role numeric tolerance governs derived values and NEVER applies to
alias copies — alias divergence at any magnitude is a producer defect. `selection` carries
`kind: "derived", derived_from: "active"`. "Always present" for
`inactive` is scoped to the EMITTED role bundle — an absent-bundle
`no-art` artifact carries no roles at all. Role `oklch` is COMPUTED
FROM the final emitted sRGB bytes (this is the only meaning);
tolerance vs any conforming implementation ±0.002 (L, C), ±0.5°
(H). Achromatic policy: C < 1e-4 → H = 0.0 exactly — and the
policy binds ALL operations on hue (emission, comparison,
interpolation, display), not only emission: below the threshold,
hue is not a quantity; unclamped hue operations on achromatic
values are nondeterministic between conforming implementations
and are spec violations.

Reference conversion: sRGB (IEC 61966-2-1 EOTF, D65) → OKLab per
Björn Ottosson's published matrices (the same conversion the
fixtures were generated with); OKLCH is its cylindrical form,
H in degrees [0, 360).

## Provenance and structure (addendum A4, ratified 2026-09-18)

`kind`, `derived_from` and `alias_of` are STRUCTURAL role-model
constants — they state what a role IS in the model, never how a
particular run manufactured its bytes. They are emitted verbatim and
are not provenance claims.

Provenance lives in two role-level fields:
- `origin` — REQUIRED on every non-alias role; a truthful,
  informative string with open vocabulary (`synthetic-fixture`,
  `stock-base`, solver-run forms such as
  `derived:selection(active)`). Consumers MUST ignore unknown values.
- `tier` — OPTIONAL on non-alias roles; present IF AND ONLY IF a
  solver tier actually applied, omitted otherwise (omission is the
  honest shape; a producer MUST NOT invent a tier). When present it
  is a string; consumers MUST accept its absence and MUST ignore
  unknown values.

Alias roles (`inactive`) carry NEITHER `origin` NOR `tier`: an alias
is a copy, and its provenance is its source role's. Nothing
load-bearing may hang on `origin` or `tier` — they are audit truth,
not behavior switches.

## Clusters

`oklch_extraction`: extraction-stage coordinates — POST gamut
mapping, PRE 8-bit quantization (named stage; this is why they may
differ from converting the paired hex). `hex` is the display
projection. Shares in (0,1], sum ≤ 1.0 + 0.001; ordered
share-descending, ties by ascending id; ids unique within one
artifact only.

## Cluster interpolation (normative)

`hex` is normative for at-rest swatch display and everything
contrast-bearing. `oklch_extraction` is normative for CONTINUOUS
INTERPOLATION between clusters. The one conforming algorithm is
POLAR OKLCH: L and C linear; H by shortest arc; at exact 180.0°
opposition the path is the semicircle ascending from the
lower-valued hue endpoint — a property of the pair, giving the
invariant lerp(a,b,t) == lerp(b,a,1-t) everywhere. An achromatic
endpoint contributes no hue (adopt the chromatic endpoint's hue;
two achromatic endpoints make hue irrelevant). Cartesian OKLab
interpolation is NOT conforming for cluster ramps. Interpolated
in-between colors inherit NO guarantees. Implementation note:
plain shortest-arc normalization suffices; do not special-case
opposition (the signed-delta clamps already encode the pair rule).
Out-of-gamut intermediates: chroma-reduce at constant L and H for
display — non-normative guidance; no conformance claim attaches
to out-of-gamut display bytes. Expected vectors (each requires
BOTH argument orders to agree; all values in gamut):
- General (three-implementation verified): achromatic-wrap ids
  0×3, t=0.5 → [0.465, 0.112779, 281.0] → #4f5096.
- Exact opposition (two-implementation verified):
  (0.50, 0.07, 100.0) × (0.50, 0.07, 280.0), t=0.5, both orders →
  [0.500000, 0.070000, 190.0] → #29706c.

## verified_pairs

Discriminated by `kind`:
- `{"kind":"contrast","fg","bg","floor","measured"}` — WCAG 2.x
  relative-luminance ratio on final sRGB bytes, 3 decimals.
- `{"kind":"delta_l","roles":[a,b],"floor","measured"}` — OKLab L
  difference (a minus b) on final bytes, 6 decimals.
The list is the COMPLETE guarantee; nothing outside it is claimed,
including compositions of listed pairs. This holds when the list is
EMPTY: an empty list means no pair guarantees, and consumers MUST NOT
infer unlisted pairs or floors (addendum A2, ratified 2026-09-18). Required in `ok`; in
`no-art`, present exactly when the role bundle is emitted (see the
emission predicate — when a no-art palette is expressible in
bytes AND meets its floors, it is verified; otherwise the
artifact stays silent instead of inventing). The six contrast
pairs + the selection/active delta_l, whenever `selection` is in
`features`; `features` omits the selection capability whenever no
selection role is emitted. Scope of that rule (addendum A1, ratified
2026-09-18): it binds EMITTED bundles through the no-art emission
predicate. On `failed` envelopes, `features` advertises the attempted
producer's capability set and carries no emission guarantee. No
reverse implication is imposed on `ok` bundles: a selection role not
advertised in `features` remains acceptable.

## Non-goals (v1)

No `gradient` (absent, not null). No playback-event identity. No
retention inside the artifact. No PCM/audio data.
