# notan-palette/1 — palette artifact contract (draft for ratification)

Status: synthetic-fixture stage. The producer (JSON publisher) exists
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
- `no-art`: + `roles` (neutral path, all floors still verified),
  `verified_pairs`; `clusters: []`; `art_sha256: null`.
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
`{alias_of: "frame"}` + its color data. `selection` carries
`kind: "derived", derived_from: "active"`. Role `oklch` is COMPUTED
FROM the final emitted sRGB bytes (this is the only meaning);
tolerance vs any conforming implementation ±0.002 (L, C), ±0.5°
(H). Achromatic policy: C < 1e-4 → H = 0.0 exactly.

Reference conversion: sRGB (IEC 61966-2-1 EOTF, D65) → OKLab per
Björn Ottosson's published matrices (the same conversion the
fixtures were generated with); OKLCH is its cylindrical form,
H in degrees [0, 360).

## Clusters

`oklch_extraction`: extraction-stage coordinates — POST gamut
mapping, PRE 8-bit quantization (named stage; this is why they may
differ from converting the paired hex). `hex` is the display
projection. Shares in (0,1], sum ≤ 1.0 + 0.001; ordered
share-descending, ties by ascending id; ids unique within one
artifact only.

## verified_pairs

Discriminated by `kind`:
- `{"kind":"contrast","fg","bg","floor","measured"}` — WCAG 2.x
  relative-luminance ratio on final sRGB bytes, 3 decimals.
- `{"kind":"delta_l","roles":[a,b],"floor","measured"}` — OKLab L
  difference (a minus b) on final bytes, 6 decimals.
The list is the COMPLETE guarantee; nothing outside it is claimed,
including compositions of listed pairs. Required in `ok` AND
`no-art` (neutral palettes are verified palettes): the six contrast
pairs + the selection/active delta_l, whenever `selection` is in
`features`.

## Non-goals (v1)

No `gradient` (absent, not null). No playback-event identity. No
retention inside the artifact. No PCM/audio data.
