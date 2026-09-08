# rmpc-auto-theme

rmpc-auto-theme derives a readable [rmpc](https://github.com/mierak/rmpc)
color theme from the current album art. It keeps your existing layout and
formatting, writes the generated theme to its own file, and applies it to a
running rmpc process over IPC.

The generator, hook, configuration, and installer are implemented. Automated
tests cover generation and failure behavior; the owner-run macOS playback smoke
test is still pending.

## Requirements

- macOS or Linux with a POSIX shell
- Rust and Cargo to build the generator
- rmpc 0.11 or later
- MPD album art available through `readpicture` or `albumart`

The build also expects the `color-tool-kmeans` repository beside this repository
because `color-core` is currently a path dependency.

## Install rmpc-auto-theme

Run the installer without consent to change your live rmpc configuration:

```sh
./install.sh
```

This command builds and installs:

- `~/.local/bin/rmpc-theme-gen`
- `~/.local/libexec/rmpc-auto-theme/on_song_change.sh`
- `~/.config/rmpc-auto-theme/config.toml`
- `~/.config/rmpc/themes/auto-theme.ron`

It then prints the `theme` and `on_song_change` fields to add to
`~/.config/rmpc/config.ron`.

To let the installer wire those fields, run:

```sh
./install.sh --wire-hook
```

The consented form creates a timestamped backup of `config.ron`. It only edits a
configuration with exactly one ordinary `theme` field and exactly one
`on_song_change: None` field. It refuses an existing custom hook instead of
overwriting it. Repeating either install form is safe.

If your current named theme exists under `~/.config/rmpc/themes`, the consented
installer snapshots it as the immutable base before switching rmpc to
`auto-theme`. Otherwise, set `base_theme` explicitly before you wire the hook.

The installer respects `XDG_CONFIG_HOME` and `RMPC_AUTO_THEME_PREFIX`.

## Configure theme generation

Edit `~/.config/rmpc-auto-theme/config.toml`. The installed file lists every
tunable value and its default. The most common settings are:

```toml
base_theme = "/Users/me/.config/rmpc/themes/my-theme.ron"
quality = 2
osc11 = false
```

Use an absolute path or a `~/` path for `base_theme`. Quality accepts values from
0 through 4. OSC 11 terminal-background synchronization is disabled by default
and only writes when `/dev/tty` is writable.

You can also customize the lightness search band for each role and the final
contrast, color-distance, and hue-separation floors. For example:

```toml
[bands.accent]
target = 0.74
min = 0.68
max = 0.80

[bands.selection]
target = 0.75
min = 0.72
max = 0.82

[floors]
accent_background = 4.5
highlight_selection = 4.5
selection_active_delta_l = 0.08
peer_delta_e = 25.0
hue_separation = 30.0
```

Malformed files and unknown keys make direct generator invocations fail with an
error. The song-change hook records the error and exits successfully so theming
cannot interrupt playback.

## Generate a theme directly

Pass encoded artwork on standard input. The complete theme is written to
standard output:

```sh
rmpc-theme-gen --base-theme "$HOME/.config/rmpc/themes/base.ron" \
  < cover.jpg > preview.ron
```

Add `--debug` to print role provenance and measured guardrails to standard
error. Command-line flags override the TOML file. Use `--set` for any documented
scalar:

```sh
rmpc-theme-gen --set quality=4 --set bands.accent.target=0.76 < cover.jpg
```

Query one effective value without reading artwork:

```sh
rmpc-theme-gen --print-config osc11
```

## Runtime behavior

On each song change, the hook:

1. Acquires album art with `rmpc albumart` and honors its distinct no-art and
   no-song exit codes.
2. Skips generation when the SHA-256 matches the previously applied artwork.
3. Generates into a temporary file and atomically renames it to
   `auto-theme.ron`.
4. Applies those same bytes with `rmpc remote --pid "$PID" set theme -`.
5. Commits the art hash only after a successful IPC apply.

A no-art track restores the unmodified base theme and clears the saved hash. A
newer overlapping song-change event supersedes an older one. Failures leave the
running theme alone, are written to the size-capped
`~/.config/rmpc-auto-theme/hook.log`, and always return exit status 0.

## Design and license

[SPEC.org](SPEC.org) is the normative design. The color engine comes from
[color-tool-kmeans](https://github.com/animegolem/color-tool-kmeans) and uses
OKLab clustering with hue-preserving gamut mapping. Project code is MIT;
attribution details for the color conversion code are retained in project
history.
