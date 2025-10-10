# rmpc Dynamic Theme Generator

Automatically generate rmpc themes from album artwork using K-means color extraction.

**Currently this addon is in BETA.**

- The core theme switcher works but some combinations still present illegal/unreadable contrast.
- Currently non-default themes are not supported as the swticher _overwrites_ the entire theme on album change. Custom configuration is planned. 
## Features

- Extracts dominant colors from album art using K-means clustering
- Intelligently maps colors to UI elements (background, text, accents, borders)
- Deterministic pairwise accent/active solver with contrast matrix guardrails
- WCAG AA contrast compliance (4.5:1 ratio)
- Fast generation (~10ms per image)
- Automatic theme switching on song change
- Robust error handling
- Detailed logging for debugging

## Quick Start

### Building from Source

```bash
# Clone or download this repository
cd ~/.config/rmpc/theme-switcher

# Build the binary (requires Rust/Cargo)
./build.sh install

# This will:
# - Build the release binary (~2.2MB)
# - Install to ~/.local/bin/rmpc-theme-gen
# - Make it available in your PATH
```

### Prerequisites

- Rust toolchain (rustc, cargo) - Install from https://rustup.rs
- rmpc music player (0.9.0+) - https://github.com/mierak/rmpc
- MPD (Music Player Daemon) configured with music library

### Configuration

In the RMPC config.ron: 

```ron
# ~/.config/rmpc/config.ron
(
    theme: "current-song",
    enable_config_hot_reload: true,
    on_song_change: ["~/.config/rmpc/on_song_change.sh"],
    ...
)
```

### Usage

Just play music in rmpc! The theme will automatically update when songs change.

The on_song_change hook computes a SHA-256 of the extracted cover and skips generation when the image is unchanged from the last run. To bypass this optimization for debugging, set `RMPC_THEME_FORCE=1` in the environment before launching rmpc or invoking the hook.

Check the generator version anytime with:

```bash
./rmpc-theme-gen --version
```

## How It Works

1. **Song Changes** → rmpc triggers `on_song_change.sh`
2. **Extract Album Art** → `rmpc albumart` saves cover to `/tmp/rmpc/current_cover`
3. **Analyze Colors** → `rmpc-theme-gen` runs K-means clustering in CIELAB space
4. **Map to Roles** → Colors assigned to UI elements using HSV/Lab properties
5. **Generate Theme** → RON file written to `~/.config/rmpc/themes/current-song.ron`
6. **Hot Reload** → rmpc automatically applies the new theme

## Color Mapping Algorithm

- **Background**: Most dominant color with low saturation (S < 0.4)
- **Text**: Highest contrast against background (≥ 4.5:1 WCAG AA)
- **Accent & Active**: Evaluated together via a deterministic pairwise solver that maximizes the minimum contrast across (accent↔bg, accent↔text, accent↔active, active↔bg, active↔text) while enforcing ΔE ≥ 25 and ≥25 L* separation. Fallbacks relax peer contrast slightly before resorting to neutral synthetics.
- **Highlight Text**: Derived from the body text (or accent/neutral fallbacks) to guarantee ≥ 4.5:1 contrast against the active highlight background while staying distinct from the page background.
- **Frame**: A shared stroke color (≥3.0:1 vs background) used for borders, dividers, progress rails, and scrollbar ends/thumb so we avoid inventing extra palette colors.
- **Guardrails**: Base text and highlight text stay ≥4.5:1 against their backgrounds; accent↔background ≥3.0:1; accent↔active ≥3.0:1 with ΔE ≥ 25. Debug mode (`--debug`) emits pairwise matrices plus chosen highlight/frame colors.

## Project Structure

```
theme-switcher/
├── src/
│   ├── rmpc_theme_gen.rs    # Main binary source
│   ├── color.rs              # Color conversion and utilities
│   ├── image_pipeline.rs     # Image loading and sampling
│   ├── kmeans.rs             # K-means clustering algorithm
│   └── lib.rs                # Library exports
├── RAG/                      # Documentation and tracking
│   ├── AI-EPIC/              # Epic-level requirements
│   ├── AI-IMP/               # Implementation tickets
│   └── AI-LOG/               # Session logs
├── test-results/             # Test data and results
├── Cargo.toml                # Rust project manifest
├── build.sh                  # Build and install script
├── on_song_change.sh         # rmpc integration script
└── README.md                 # This file

Installed files:
~/.local/bin/rmpc-theme-gen              # Binary (2.2MB)
~/.config/rmpc/on_song_change.sh         # Integration script
~/.config/rmpc/themes/current-song.ron   # Generated theme
~/.config/rmpc/theme-switcher.log        # Debug logs
```

## Logs

View theme generation activity:

```bash
tail -f ~/.config/rmpc/theme-switcher.log
```

Log format:
```
[2025-09-29 22:13:59] ========== Song Change Detected ==========
[2025-09-29 22:13:59] File: /path/to/song.mp3
[2025-09-29 22:13:59] Artist: Artist Name
[2025-09-29 22:13:59] Title: Song Title
[2025-09-29 22:13:59] Extracting album art...
[2025-09-29 22:13:59] Album art extracted successfully
[2025-09-29 22:13:59] Generating theme...
[2025-09-29 22:13:59] Theme generated successfully
[2025-09-29 22:13:59] ========== Theme Update Complete ==========
```

## Manual Usage

Generate theme from any image:

```bash
rmpc-theme-gen \
  --image /path/to/album-art.jpg \
  --k 12 \
  --space CIELAB \
  --theme-output ~/.config/rmpc/themes/my-theme.ron
```

Options:
- `--image` (required): Path to album art image
- `--k` (default: 12): Number of color clusters to extract
- `--space` (default: CIELAB): Color space (CIELAB, RGB, HSL, HSV, YUV, CIELUV)
- `--theme-output`: Path to output theme file (generates RON format)
- `--output`: Path to output JSON analysis (optional)
- `--disable-scrollbar`: Omit the scrollbar block (helpful if panes never scroll or you want to hide the gutter)
- `--debug`: Emit pairwise contrast diagnostics (also available via `RMPC_THEME_DEBUG=1`)

### Debug Diagnostics

Set `--debug` or `RMPC_THEME_DEBUG=1` to embed a `debug.pairwise` block in the JSON output. It captures the evaluated accent/active matrix, top-scoring pairs, and candidate provenance so you can diagnose outliers quickly.

## Performance

- **Generation time**: ~10ms (tested, target: <500ms)
- **Theme file size**: ~5.4KB
- **Binary size**: 2.2MB (release build)
- **Memory usage**: Minimal, processes images in-memory
- **Contrast ratios**: WCAG AA 4.5:1 achieved in all tests

## Testing

Test results available at:
```
~/.config/rmpc/theme-switcher/test-results/TEST-RESULTS.md
```

Run tests:
```bash
~/.config/rmpc/theme-switcher/test-results/run-tests.sh
```

## Troubleshooting

**Theme not changing:**
1. Check logs: `tail ~/.config/rmpc/theme-switcher.log`
2. Verify script is executable: `ls -l ~/.config/rmpc/on_song_change.sh`
3. Check config: `grep on_song_change ~/.config/rmpc/config.ron`
4. Test manually: `FILE=/tmp/test.mp3 ARTIST=Test TITLE=Song ~/.config/rmpc/on_song_change.sh`

**Album art not found:**
- Script will log "ERROR: Album art extraction failed"
- Theme remains unchanged
- Playback continues normally

**Binary not found:**
- Set environment variable: `export RMPC_THEME_GEN_PATH=/path/to/rmpc-theme-gen`
- Or add to PATH: `export PATH="$HOME/.local/bin:$PATH"`

**Scrollbar strip shows stale color:**
- Default themes now paint scrollbar tracks and ends with the active background; regenerate themes after updating.
- To suppress the scrollbar entirely, set `RMPC_THEME_GEN_DISABLE_SCROLLBAR=1` before invoking `on_song_change.sh` (the script passes `--disable-scrollbar`).

## Architecture

```
┌──────────────┐
│  rmpc plays  │
│   new song   │
└──────┬───────┘
       │
       ▼
┌──────────────────────┐
│  on_song_change.sh   │
│  - Check state       │
│  - Extract art       │
│  - Call generator    │
└──────┬───────────────┘
       │
       ▼
┌──────────────────────┐
│  rmpc-theme-gen      │
│  - K-means cluster   │
│  - Color mapping     │
│  - RON generation    │
└──────┬───────────────┘
       │
       ▼
┌──────────────────────┐
│  current-song.ron    │
│  - Valid RON theme   │
│  - rmpc hot-reloads  │
└──────────────────────┘
```

## Credits

This project is built using K-means color extraction algorithms extracted from the [color-abstract-via-multidim-KMeans](https://github.com/yourusername/color-abstract-via-multidim-KMeans) project.

**Source modules** (`color.rs`, `kmeans.rs`, `image_pipeline.rs`) were extracted and adapted to create a standalone theme generator for rmpc.

**Color Science:**
- Color spaces: CIELAB (perceptually uniform), HSV (hue/saturation), RGB
- Contrast: WCAG 2.1 guidelines (4.5:1 minimum ratio)
- Perceptual distance: CIE76 Delta E

**Original K-means Implementation Credits:**
The K-means clustering with SIMD optimizations and multi-dimensional color space support is based on the color-abstract-via-multidim-KMeans project.

## License & Attributions

- Licensed under the MIT License (`LICENSE`).
- Third-party algorithms and references are documented in `ATTRIBUTIONS.md` and must accompany redistributions.
