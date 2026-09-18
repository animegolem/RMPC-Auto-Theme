# Test fixtures

The image fixtures are base64-encoded PNG files so they remain reviewable text
while exercising the same encoded-byte path used for album artwork. Tests must
decode them before passing their bytes to `color-core`.

- `chromatic.png.b64`: Code Lead synthetic 16 x 16 RGB swatch with red, blue,
  orange, green, and violet clusters for chromatic role assignment.
- `monochrome.png.b64`: Code Lead synthetic 16 x 16 uniform RGB(96, 96, 96)
  image for neutral-ramp selection.
- `near-black.png.b64`: Code Lead synthetic 16 x 16 RGB image whose channels span
  0 through 8 for low-contrast terminal-fallback coverage.
- `default_theme.ron`: genericized copy of the owner's v3
  `~/.config/rmpc/themes/base.ron` as read 2026-09-18. It preserves the
  `np_header`/tab chrome and materialized stock `reset` colors; unused personal
  paths do not appear. The album-table chrome, volume slider styles, and Cava
  color are materialized for the DQ-4 mapped surfaces. Level severity
  foregrounds intentionally remain semantic literals per BRIEF-004 V-2.
- `sample_config.ron`: companion four-tab rmpc 0.11 config derived from the
  owner's live config as read 2026-09-18. Owner paths are removed, while the
  hook and Cava FIFO/pane are commented until explicitly configured.
- `chromatic-theme.ron`: byte-exact generated output for `chromatic.png.b64`
  against the shipped v3 base. Regenerate only after deliberate review of the
  emitter and solver, and record the generating command and source revision.

IMP-006 golden regeneration (2026-09-18 UTC): repository baseline
`7ff2cee74eb676fa4ab2b4c03e0f16784e498f7c` plus this ticket's uncommitted
v3/emitter worktree; `color-core` checkout
`5baa20e021855fbc57aebf48fa0f9b3374ded281`. Command:

```sh
set -o pipefail
base64 -D -i fixtures/chromatic.png.b64 | cargo run --quiet -- --config /dev/null > fixtures/chromatic-theme.ron
```

SHA-256 of encoded input `203f0a98b323632d677575a77e8d4892e6eae830ffaff4dde59791fea9115a10`,
v3 base `5d8413dae4e244f234f05081a36779839b2a2e1f82abdaf90eee56c65cdbf1e9`,
and resulting golden `8424620c074d7001b423d2283904292835bbb90bebbc7a97785ee1e759ef2f1b`.
The generator test asserts byte equality, and both source and golden were loaded
by installed rmpc 0.11.0 with `fixtures/sample_config.ron` using `debuginfo`.

IMP-006 publisher capture (2026-09-18 UTC):
`chromatic-palette-capture.json` is the full `notan-palette/1` `ok` artifact
produced by `artwork_publish_emits_mapped_extraction_coordinates` through the
real MPD-binary acquisition and publication code against a loopback MPD
fixture server. The art is decoded from `chromatic.png.b64`; its decoded-byte
SHA-256 is `517adccc702a83ff23028bd5488c0d28dfd0edcb44326c80cfe32ca099b42fba`.
The capture uses the v3 base above and the vendored `color-core` at RMPC main
`92d88c4` (mapped accessor file SHA-256
`f396bbac37e5ee10db35e6e58668b88755ea88caa7b4232afbcb5c14f4d4d8a3`).
Its embedded producer revision is
`src-dc6702270d19e9658b89c929bb0bd3587b59dfa7de9feaca9b58002c01e50155`.
Regeneration is deliberate, not part of the ordinary test run:

```sh
RMPC_CAPTURE_FIXTURE=fixtures/chromatic-palette-capture.json \
  cargo test artwork_publish_emits_mapped_extraction_coordinates -- --nocapture
```

Capture SHA-256: `4cbdfbcede60f99b215affc81c88c93f97e3158c34721bf7b4a61174aa2efe7b`.
It has 5 clusters (C range 0.1614–0.2187), 9 roles, 7 passing verified
pairs, and shares summing to 1. The test deserializes the emitted coordinates
and checks f64 inverse-chart fidelity against the mapped OKLab candidate to
within 1e-6/component. These exact swatches do **not** demonstrate a large
extraction-versus-hex inversion gap; Astra's retained two-vector tests cover
that permitted distinction. The capture is a fixture-path producer artifact,
not proof of a live rmpc theme application.
