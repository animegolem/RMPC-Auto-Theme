---
node_id: AI-IMP-016
tags:
  - IMP-LIST
  - Implementation
  - Performance
  - Profiling
  - Observability
kanban_status: in-progress
depends_on: []
confidence_score: 0.95
created_date: 2025-10-05
close_date:
---

# AI-IMP-016-cargo-flamegraph-profiling

## Summary of Issue

The theme generator currently achieves ~10ms performance for typical workloads, but we lack empirical data showing where time is spent (image decoding, k-means clustering, color space conversions, or I/O).

This ticket establishes performance profiling infrastructure using `cargo-flamegraph` to:
1. Generate baseline flamegraph visualizations for representative workloads
2. Identify hot paths and allocation patterns
3. Document findings in a reusable profiling report under `RAG/AI-LOG/`

**Success criteria**: Flamegraph generated, top 3 time-consuming operations identified with percentages, documented in session log.

### Out of Scope

- Implementing performance optimizations based on findings (separate follow-up ticket if needed)
- Continuous profiling or CI integration
- Alternative profilers (perf, valgrind, samply) - flamegraph only for initial pass
- Profiling edge cases (malformed images, extreme `--k` values)

### Design/Approach

**Approach**: Install `cargo-flamegraph`, run against 4 existing benchmark images from `bench-assets/`, capture SVG outputs, analyze call stacks.

**Workload selection** (from existing bench-assets):
- 1.5m-example.png (1.6MB) - baseline case
- 4mb-example.png (4.1MB) - moderate size
- 11mb-example.png (11MB) - large image stress test
- 24mb-example.PNG (23MB) - maximum size stress test

**Alternatives considered**:
- `perf` + FlameGraph scripts: More setup, Linux-only
- `cargo instruments` (macOS): Platform-specific
- **Chosen**: `cargo-flamegraph` for ease of use, cross-platform Rust focus

### Files to Touch

- `RAG/AI-LOG/2025-10-05-flamegraph-profiling.md`: New profiling session log with findings
- `bench-assets/*.svg`: Generated flamegraph outputs (gitignored, reference only)
- No source code changes required

### Implementation Checklist

<CRITICAL_RULE>
Before marking an item complete on the checklist MUST **stop** and **think**. Have you validated all aspects are **implemented** and **tested**?
</CRITICAL_RULE>

- [ ] Install `cargo-flamegraph`: Run `cargo install flamegraph` and verify installation succeeds
- [ ] Run baseline profiling on 1.5m-example.png: `cargo flamegraph --bin rmpc-theme-gen -- --image bench-assets/1.5m-example.png --k 8 --space CIELAB --theme-output /tmp/test.ron`
- [ ] Capture flamegraph SVG output, rename to `bench-assets/flamegraph-1.5m.svg`
- [ ] Run profiling on 4mb-example.png, save to `bench-assets/flamegraph-4mb.svg`
- [ ] Run profiling on 11mb-example.png, save to `bench-assets/flamegraph-11mb.svg`
- [ ] Run profiling on 24mb-example.PNG, save to `bench-assets/flamegraph-24mb.svg`
- [ ] Analyze flamegraphs: Identify top 3 functions/modules by time percentage for baseline case
- [ ] Compare across sizes: Note if bottlenecks shift between small vs large images
- [ ] Document findings in `RAG/AI-LOG/2025-10-05-flamegraph-profiling.md` with:
  - Benchmark images used (paths, dimensions, formats)
  - Flamegraph SVG paths
  - Top 3 hotspots with estimated time percentages
  - Performance scaling observations (linear with image size? plateaus?)
  - Any surprising observations (e.g., "PNG decode is 60% of runtime")
  - Recommendations for future optimization (if any low-hanging fruit identified)
- [ ] Verify session log is committed to repo

### Acceptance Criteria

**Scenario**: Developer wants to understand where theme generation spends CPU time.

**GIVEN** `cargo-flamegraph` is installed and benchmark images are available in `bench-assets/`.

**WHEN** Profiling is run against the 1.5MB baseline image with `--k 8 --space CIELAB`.

**THEN** A flamegraph SVG is generated showing call stack breakdown.
**AND** The top 3 time-consuming operations are identified with percentage estimates (e.g., "image::load: 45%, kmeans::cluster: 30%, color::convert_lab: 15%").
**AND** Findings are documented in `RAG/AI-LOG/2025-10-05-flamegraph-profiling.md` with reproduction commands.

**WHEN** Profiling is repeated across all 4 benchmark sizes (1.5MB, 4MB, 11MB, 24MB).

**THEN** Comparative flamegraphs are generated showing performance characteristics across workloads.
**AND** Scaling behavior is documented (e.g., "Image decode time scales linearly with file size, k-means time constant").
**AND** Any size-specific bottlenecks are noted.

### Issues Encountered
<!--
The comments under the 'Issues Encountered' heading are the only comments you MUST not remove
This section is filled out post work as you fill out the checklists.
You SHOULD document any issues encountered and resolved during the sprint.
You MUST document any failed implementations, blockers or missing tests.
-->

(To be filled during implementation)
