# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository Overview

This is a configuration directory for **rmpc** (Rust Music Player Client for MPD), containing:
- rmpc configuration files (`config.ron`, `config.debug.ron`)
- A dynamic theme generator (`theme-switcher/`) that automatically generates rmpc themes from album artwork using K-means color extraction
- Generated theme files in `themes/`
- Integration scripts (`on_song_change.sh`)

## Build Commands

### Theme Generator
```bash
# Build and install the theme generator binary
cd theme-switcher
./build.sh install           # Builds release binary and installs to ~/.local/bin/rmpc-theme-gen

# Standard Rust development
cargo check                  # Fast validation
cargo clippy -- -D warnings  # Lint checks
cargo fmt                    # Format code

# Integration tests
./test-results/run-tests.sh  # Run theme generation tests on sample album art
```

### Manual Theme Generation
```bash
rmpc-theme-gen \
  --image /path/to/album-art.jpg \
  --k 8 \
  --space CIELAB \
  --theme-output ~/.config/rmpc/themes/my-theme.ron
```

## Architecture

### Dynamic Theme System Flow
```
rmpc plays song → on_song_change.sh → rmpc albumart extracts cover →
rmpc-theme-gen analyzes colors → themes/current-song.ron generated →
rmpc hot-reloads theme
```

### Color Science Pipeline
1. **Image Sampling**: Album art loaded and downsampled (image_pipeline.rs)
2. **K-means Clustering**: Extract 8 dominant colors in CIELAB space (kmeans.rs)
3. **Color Mapping**: Intelligent assignment to UI roles (rmpc_theme_gen.rs):
   - **Background**: Most dominant, low saturation (S < 0.4)
   - **Text**: Highest contrast vs background (≥ 4.5:1 WCAG AA)
   - **Accent**: High saturation + good contrast (≥ 3.0:1)
   - **Border**: Mid-saturation, perceptually distinct (ΔE > 20)
   - **ActiveItem**: Bright + saturated (V > 0.5, S > 0.3)
4. **RON Generation**: Valid rmpc theme file written (~5.4KB)

### Key Configuration
- `config.ron`:
  - `theme: "current-song"` - Uses dynamically generated theme
  - `enable_config_hot_reload: true` - Required for automatic updates
  - `on_song_change: ["~/.config/rmpc/on_song_change.sh"]` - Triggers generation
- `on_song_change.sh`: Bash script handling album art extraction, format detection, and theme generation invocation

## Project Structure

```
.
├── config.ron                    # rmpc configuration
├── on_song_change.sh             # Song change hook (calls theme generator)
├── themes/
│   └── current-song.ron          # Auto-generated theme (hot-reloaded by rmpc)
└── theme-switcher/               # Theme generator Rust project
    ├── src/
    │   ├── rmpc_theme_gen.rs     # Main CLI binary
    │   ├── color.rs              # Color space conversions, contrast calculations
    │   ├── kmeans.rs             # K-means clustering algorithm
    │   ├── image_pipeline.rs     # Image loading and sampling
    │   └── lib.rs                # Library exports
    ├── RAG/                      # Documentation and implementation tracking
    │   ├── AI-EPIC/              # Epic-level requirements
    │   ├── AI-IMP/               # Implementation tickets
    │   └── AI-LOG/               # Session logs
    ├── test-results/             # Test data and validation outputs
    ├── Cargo.toml                # Rust dependencies
    ├── build.sh                  # Build and install script
    ├── AGENTS.md                 # Development guidelines
    └── README.md                 # Detailed project documentation
```

## Coding Standards

### Rust Conventions (from AGENTS.md)
- **Edition**: Rust 2021, 4-space indentation
- **Naming**: `snake_case` for files/functions, `PascalCase` for types, `SCREAMING_SNAKE_CASE` for consts
- **Module Organization**: Helper functions belong in relevant modules (`color::`, `kmeans::`, etc.), not the binary file
- **CLI Args**: Use `clap` derive macros, document non-obvious flags inline
- **Formatting**: `cargo fmt` enforces style, `cargo clippy` for lints

### Testing Approach
- Unit tests in module files
- Integration tests via `test-results/run-tests.sh` (captures JSON + RON outputs)
- Add regression fixtures under `test-results/` for image-related bugs
- Document test sources in `TEST-RESULTS.md`

### Commit Guidelines
- Imperative, lower-case subjects: `add kmeans centroid caching`
- Reference work items: `Refs AI-IMP-007`
- Include test outputs when behavior affects theme generation

## Key Implementation Details

### Color Utilities (color.rs)
- RGB ↔ HSV ↔ CIELAB conversions
- WCAG contrast ratio calculation
- CIE76 Delta E perceptual distance
- Synthetic color generation for fallbacks

### K-means (kmeans.rs)
- Multi-dimensional clustering (supports RGB, HSL, HSV, CIELAB, etc.)
- Configurable iterations and convergence
- Returns centroids with pixel counts

### Theme Generation (rmpc_theme_gen.rs)
- Color role assignment based on HSV/Lab properties
- RON format output matching rmpc theme schema
- Scrollbar configuration (can be disabled with `--disable-scrollbar`)
- Performance target: <500ms (typically ~10ms)

## Known Issues

### Intermittent Background Color Persistence
- **Status**: Minor visual artifact (cosmetic only)
- **Behavior**: Previous theme colors may briefly show during transitions
- **Cause**: Suspected rmpc hot-reload timing/rendering behavior
- **Impact**: Low - colors correct themselves within 1-2 seconds
- **Details**: See `theme-switcher/issue-scope-and-statement.md`

## Logging and Debugging

```bash
# View theme generation logs
tail -f ~/.config/rmpc/theme-switcher.log

# Log format
[2025-09-29 22:13:59] ========== Song Change Detected ==========
[2025-09-29 22:13:59] File: /path/to/song.mp3
[2025-09-29 22:13:59] Artist: Artist Name
[2025-09-29 22:13:59] Generating theme...
[2025-09-29 22:13:59] Theme generated successfully
```

## Dependencies

- **Rust toolchain**: cargo 1.89.0+ (from rustup.rs)
- **rmpc**: 0.9.0+ (Rust Music Player Client)
- **MPD**: Music Player Daemon with configured music library
- **Libraries**: serde, image (PNG/JPEG/WebP), clap, rayon, rand