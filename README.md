# rmpc-auto-theme

rmpc-auto-theme generates a color theme for the
[rmpc](https://github.com/mierak/rmpc) music player from the album
art of the song that's playing, and applies it to the running player
without a restart. Every generated theme meets WCAG contrast minimums,
so the interface stays readable no matter what the album art looks
like.

**Status: pre-implementation.** The design is complete
([SPEC.org](SPEC.org)); implementation hasn't started. Version 1,
which required a patched rmpc fork, is archived under `.archive/` and
retained in git history.

## How it works

When rmpc changes songs, it runs a small hook script. The hook:

1. Reads the current album art with `rmpc albumart -o -`.
2. Compares the art's SHA-256 hash to the previous song's hash, and
   exits early if they match. Tracks from the same album don't
   regenerate the theme or cause flicker.
3. Pipes the art to `rmpc-theme-gen`, which extracts a palette,
   assigns colors to theme roles, and patches them into your base
   theme.
4. Applies the result with `rmpc remote set theme -`, which swaps
   the theme in the running player over IPC.

If any step fails, the hook exits cleanly and playback continues
with the current theme. Theming never interrupts music.

## Design principles

**Values are truth; colors are mapped onto values.** Each theme role
(background, text, accent, and so on) owns a fixed lightness band,
chosen so that the required contrast ratios are achievable before
any album is considered. Album art contributes only hue and chroma,
which the generator projects into each role's band. Contrast checks
verify the result instead of rescuing it.

Other guarantees:

- **Deterministic.** The same album art always produces the same
  theme.
- **Yours stays yours.** The generator patches color tokens into
  your own base theme and never touches your layout, tabs, or
  formats.
- **Auditable.** The `--debug` flag reports where every color came
  from and the contrast it achieves.

## Requirements

- rmpc 0.11 or later. The project relies on `rmpc remote set theme`
  and the `on_song_change` hook; no fork or patch is required.
- MPD with album art available through `readpicture` or `albumart`.
- Optional: the `cava` binary, if your layout uses the Cava pane.

## Color engine

Color extraction comes from
[color-tool-kmeans](https://github.com/animegolem/color-tool-kmeans):
OKLab k-means clustering with hue-preserving gamut mapping,
consumed as a library rather than copied. The color conversion code
credits Laurent Jégou's Color-tool (CC BY 3.0); see
[ATTRIBUTIONS](.archive/ATTRIBUTIONS.md) pending the crate split.

## Project documents

- [SPEC.org](SPEC.org) — the normative specification. Design
  decisions land there first.
- `RAG/` — work tracking: epics, implementation tickets, session
  logs, and a generated kanban (`RAG/INDEX.md`).
- `~/git/personal-theming/RAG/design/` — the four-view UI redesign
  this theming serves (local; static configuration, out of scope
  for the generator).

## License

MIT for project code. Portions of the color engine carry CC BY 3.0
attribution requirements as noted above.
