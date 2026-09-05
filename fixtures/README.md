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
- `default_theme.ron`: byte-exact copy of rmpc 0.11
  `assets/example_theme.ron` at upstream commit `056cfab`; used to verify
  complete-document, span-preserving emission.
- `chromatic-theme.ron`: byte-exact expected output for the chromatic fixture;
  update only after deliberate solver and emitter review.
