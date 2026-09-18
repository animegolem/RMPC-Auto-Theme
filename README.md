# rmpc-auto-theme

rmpc-auto-theme derives a readable [rmpc](https://github.com/mierak/rmpc)
color theme from the current album art. It keeps your existing layout and
formatting, writes the generated theme to its own file, and applies it to a
running rmpc process over IPC.

The generator, hook, configuration, installer, and `notan-palette/1` publisher
are implemented. Automated tests cover generation and failure behavior; the
owner-run macOS playback smoke test remains a separate acceptance gate.

## Requirements

- macOS or Linux with a POSIX shell
- Rust and Cargo to build the generator
- rmpc 0.11 or later
- MPD album art available through `readpicture` or `albumart`

The color engine is included at `vendor/color-core`; no sibling checkout is
needed to build this project.

## Install rmpc-auto-theme

Run the installer without consent to change your live rmpc configuration:

```sh
./install.sh
```

This command builds and installs:

- `~/.local/bin/rmpc-theme-gen`
- `~/.local/libexec/rmpc-auto-theme/on_song_change.sh`
- `~/.config/rmpc-auto-theme/config.toml`
- `~/.config/rmpc-auto-theme/sample-config.ron` (a four-tab example; your live config is not replaced)
- `~/.config/rmpc/themes/auto-theme.ron`

It then prints the `theme` and `on_song_change` fields to add to
`~/.config/rmpc/config.ron`.
The sample config is derived from the shipped v3 theme, with the hook disabled
until you explicitly enable it. It includes Now Playing, Library, Lyrics, and
Search tabs with keys 1–4. Edit paths and keybinds for your setup; there are no
owner-specific paths in the sample.

### Enable the optional Cava pane

The sample's Cava pane and input stanza are commented out. On macOS, rmpc's
default PulseAudio input is unavailable; Cava needs a PCM FIFO from MPD.

1. Choose an absolute FIFO path, create its parent directory, and run
   `mkfifo /absolute/path/to/mpd.fifo` once. The FIFO path must match both MPD
   and the sample config.
2. Add an MPD FIFO output to your MPD configuration and restart MPD:

   ```text
   audio_output {
       type   "fifo"
       name   "Cava"
       path   "/absolute/path/to/mpd.fifo"
       format "44100:16:2"
   }
   ```

3. In `sample-config.ron` (or your copied live config), uncomment the `cava`
   input and the `Pane(Cava)` line in the Lyrics tab. Set the same FIFO path;
   `sample_rate: 44100` and `sample_bits: 16` must match MPD's format. Keep
   the pane commented if you do not want a visualizer.

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

The palette artifact's destination can be overridden with `palette_path` in
the same TOML file; by default it is
`~/.config/rmpc-auto-theme/palette/palette.json` (under `XDG_CONFIG_HOME` when
set). Query its effective path with `--print-config palette_path`. A consumer
should read each complete JSON snapshot at that path; it must not treat the
presence of a file as evidence that its `status` is `ok`. To publish a
one-shot snapshot for the current MPD song without waiting for a song-change
event, run `rmpc-theme-gen --publish-current`.

## Runtime behavior

On each song change, the hook admits the event's `FILE` before taking a
blocking single-flight lock. The publisher fetches art by that admitted MPD
file identity, so a later track cannot relabel the art. It emits a fresh
`notan-palette/1` envelope even when a new song shares the preceding cover and
the analysis cache is reused; the hook may skip redundant theme application.
Snapshots are written by atomic rename under `palette_path`, with a final
admission-token check so an older overlapping event cannot overwrite a newer
one. A no-art track restores the unmodified stock base. Because that base uses
terminal-dependent `reset` colors, its no-art envelope has no claimed role or
verified-pair bundle. Failures leave the running theme alone and emit a
`failed` snapshot only with identity actually acquired. Hook diagnostics are
buffered and prefixed per invocation in the size-capped
`~/.config/rmpc-auto-theme/hook.log`; the hook returns status 0 to protect
playback. Multiple live rmpc instances are supported.

## Design and license

[SPEC.org](SPEC.org) is the normative design. The vendored color engine is
derived from [color-tool-kmeans](https://github.com/animegolem/color-tool-kmeans)
and uses OKLab clustering with hue-preserving gamut mapping. Project code is
MIT; the engine's provenance and CC BY 3.0 obligations are documented in
[VENDORED.md](vendor/color-core/VENDORED.md) and
[ATTRIBUTIONS.md](vendor/color-core/ATTRIBUTIONS.md).
