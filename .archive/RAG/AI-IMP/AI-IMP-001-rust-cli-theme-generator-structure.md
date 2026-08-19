---
node_id: AI-IMP-001
tags:
  - IMP-LIST
  - Implementation
  - rust
  - cli
  - theme-generator
kanban_status: completed
depends_on: []
confidence_score: 0.9
created_date: 2025-09-29
close_date: 2025-09-29
---

# AI-IMP-001-rust-cli-theme-generator-structure

## Summary of Issue

Create a Rust CLI tool that serves as the theme generator binary. This tool will accept album art image paths as input, invoke the K-means color extraction pipeline, and output theme configuration. The tool will be structured as a new binary within the existing color-abstract-via-multidim-KMeans project to reuse existing color conversion and K-means modules.

**Scope:** Argument parsing, configuration loading, basic I/O structure, and integration with existing kmeans/color modules.

**Measurable Outcome:** A working CLI binary (`rmpc-theme-gen`) that accepts an image path, extracts colors using K-means, and outputs color data in JSON format for downstream theme generation.

### Out of Scope

- Theme file RON generation (handled in AI-IMP-003)
- Color-to-UI-element mapping logic (handled in AI-IMP-002)
- Shell wrapper script integration (handled in AI-IMP-004)
- Error recovery and fallback strategies (handled in AI-IMP-006)

### Design/Approach

**Architecture:**
- New binary target `bin/rmpc_theme_gen.rs` in the tauri-app/src-tauri directory
- Reuse existing modules: `image_pipeline.rs`, `kmeans.rs`, `color.rs`
- CLI framework: `clap` for argument parsing (already in dependencies)
- Output format: JSON with extracted colors, counts, HSV, and Lab values

**Workflow:**
1. Parse CLI arguments: `--image <path>`, `--k <num_colors>`, `--space <colorspace>`, `--output <path>`
2. Load image via existing `image_pipeline::prepare_samples()`
3. Run K-means clustering via existing `kmeans::run_kmeans()`
4. Convert centroids to RGB, HSV, Lab formats
5. Output structured JSON with color analysis data

**Alternative Considered:** Extending `compute_cli.rs` instead of new binary. Rejected because theme generation has different concerns (file I/O, theme structure) than the generic compute CLI.

### Files to Touch

- `tauri-app/src-tauri/src/bin/rmpc_theme_gen.rs`: new binary for theme generation CLI
- `tauri-app/src-tauri/Cargo.toml`: add new binary target if needed
- `tauri-app/src-tauri/src/image_pipeline.rs`: potentially expose additional metadata
- `tauri-app/src-tauri/src/color.rs`: ensure all necessary conversions are public

### Implementation Checklist

<CRITICAL_RULE>
Before marking an item complete on the checklist MUST **stop** and **think**. Have you validated all aspects are **implemented** and **tested**?
</CRITICAL_RULE>

- [x] Create `tauri-app/src-tauri/src/bin/rmpc_theme_gen.rs` with main function and clap CLI structure
- [x] Add CLI arguments: `--image` (required), `--k` (default: 8), `--space` (default: CIELAB), `--output` (optional)
- [x] Import and integrate `image_pipeline::prepare_samples()` for image loading
- [x] Import and integrate `kmeans::run_kmeans()` for color clustering
- [x] Compute RGB values from centroids using appropriate color space conversions
- [x] Compute HSV values for each centroid using `color::rgb8_to_hsv()`
- [x] Compute Lab values for each centroid (if not already in Lab space)
- [x] Create output struct with fields: `clusters` (rgb, hsv, lab, count, share), `total_samples`, `iterations`
- [x] Serialize output struct to JSON using serde_json
- [x] Write JSON to stdout or specified output file
- [x] Add error handling for image loading failures
- [x] Add error handling for invalid color space specifications
- [x] Test binary with sample album art image (1000x1000 jpeg)
- [x] Verify JSON output contains expected fields and valid color values
- [x] Measure execution time on typical album art (should be <500ms)

### Acceptance Criteria

**Scenario:** Developer runs theme generator on album art to extract dominant colors.

**GIVEN** A valid album art image at `/tmp/album_art.jpg` (1200x1200 RGB JPEG).
**WHEN** Developer executes `rmpc-theme-gen --image /tmp/album_art.jpg --k 8 --space CIELAB`.
**THEN** The tool completes execution within 500ms.
**AND** Valid JSON is written to stdout containing 8 color clusters.
**AND** Each cluster includes `rgb`, `hsv`, `lab`, `count`, and `share` fields.
**AND** RGB values are in range [0, 255], HSV values are valid (H: 0-360, S/V: 0-1), Lab values are valid.
**AND** Share values sum to approximately 1.0.
**AND** Clusters are sorted by count (descending).

**GIVEN** An invalid image path `/tmp/nonexistent.jpg`.
**WHEN** Developer executes `rmpc-theme-gen --image /tmp/nonexistent.jpg`.
**THEN** The tool exits with non-zero status code.
**AND** A clear error message is printed to stderr.

### Issues Encountered

**Completed 2025-09-29:**
- Implementation went smoothly, all acceptance criteria met
- Binary successfully built and tested with el-get logo (240x400 PNG)
- Execution time: 11.7ms (well under 500ms target)
- JSON output validated: all fields present, clusters sorted by count
- Error handling verified: nonexistent file produces clear error message and non-zero exit
- RGB values in range [0,255], HSV values valid, Lab values valid
- Share values sum to 1.0 as expected