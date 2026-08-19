# AI-EPIC-001-dynamic-album-art-theme-generation
---
node_id: AI-EPIC-001
tags:
  - EPIC
  - AI
  - theme-generation
  - rmpc
  - color-extraction
  - kmeans
date_created: 2025-09-29
date_completed: 2025-09-30
kanban-status: completed
AI_IMP_spawned: []
---

# AI-EPIC-001-dynamic-album-art-theme-generation

## Problem Statement/Feature Scope

Users of rmpc music player currently experience a static, unchanging UI theme regardless of what music
is playing. This creates a disconnect between the visual experience and the album artwork, which often
contains carefully chosen colors that reflect the mood and aesthetic of the music. Users cannot easily
create themes that match their album art, and manually creating themes for each album is impractical.
The current workflow requires manual theme creation and switching, providing no integration with the
music playback experience.

## Proposed Solution(s)

Build an automated theme generation system that extracts dominant colors from album artwork and generates
rmpc theme files dynamically. The system will leverage existing K-means color clustering algorithms from
the color-abstract-via-multidim-KMeans project to analyze album art and extract 6-8 representative colors.

**User Workflow:**
1. User plays a new track in rmpc
2. System automatically extracts album art via `rmpc albumart` command
3. Color extraction algorithm analyzes the image using K-means clustering in perceptually uniform color space
4. Theme generator maps extracted colors to UI elements using intelligent heuristics (dominant → background,
   high saturation → accents, high contrast → text/highlights)
5. Generated theme file is written to `~/.config/rmpc/themes/current-song.ron`
6. rmpc's hot-reload feature automatically applies the new theme

**Technical Approach:**
- Rust CLI tool for theme generation (reuses existing kmeans/color modules)
- Shell wrapper script executed via rmpc's `on_song_change` hook
- Role-based color mapping: analyze HSV/Lab properties to assign colors intelligently
- Fallback to default theme when album art is unavailable

This integrates seamlessly with rmpc's existing configuration system and requires no modifications to rmpc itself.

## Path(s) Not Taken

Manual theme switching: Rejected due to impracticality of creating themes for thousands of albums.
Browser extension approach: Out of scope - requires GUI, not applicable to terminal-based player.
Real-time color tracking during playback: Unnecessarily complex, song-change trigger is sufficient.
Modifying rmpc source code: Avoided to maintain compatibility and reduce maintenance burden.

## Success Metrics

**Primary Metrics:**
- Theme generation completes within 500ms of song change (measured from album art extraction to theme file write)
- Generated themes are visually coherent with album artwork (subjective evaluation on 20+ diverse albums)
- Zero crashes or errors during theme generation across 100+ song changes
- Hot-reload successfully applies new theme within 1 second of file write

**Secondary Metrics:**
- User can optionally preview and adjust color mapping strategies
- System gracefully handles edge cases (missing album art, monochrome covers, all-black covers)

## Requirements

### Functional Requirements

- [ ] FR-1: System shall extract album art using `rmpc albumart --output` command on song change
- [x] FR-2: System shall analyze album art using K-means clustering to extract 6-8 dominant colors (AI-IMP-001)
- [x] FR-3: System shall compute HSV and CIELAB values for each extracted color cluster (AI-IMP-001)
- [x] FR-4: System shall map extracted colors to theme elements using brightness, saturation, and contrast heuristics (AI-IMP-002)
- [x] FR-5: System shall assign most dominant color to background or primary UI elements (AI-IMP-002)
- [x] FR-6: System shall assign high-saturation colors to accent elements (borders, highlights, current item) (AI-IMP-002)
- [x] FR-7: System shall assign high-contrast colors to text and active selections (AI-IMP-002)
- [x] FR-8: System shall generate valid RON-formatted theme file compatible with rmpc 0.9.0+ (AI-IMP-003)
- [x] FR-9: System shall write generated theme to `~/.config/rmpc/themes/current-song.ron` (AI-IMP-003)
- [ ] FR-10: System shall fall back to default theme when album art extraction fails
- [ ] FR-11: System shall be invoked via `on_song_change` hook in rmpc configuration
- [ ] FR-12: System shall pass album art metadata via environment variables ($FILE, $ARTIST, $ALBUM)
- [x] FR-13: System shall support configurable color extraction parameters (k-value, color space, seed) (AI-IMP-001)
- [ ] FR-14: System shall log errors to a persistent log file for debugging

### Non-Functional Requirements

**Performance:** Theme generation shall complete in under 500ms for typical album art (1000x1000px or smaller).
**Reliability:** System shall handle missing album art, corrupted images, and edge cases without crashing rmpc.
**Maintainability:** Code shall reuse existing color/kmeans modules from color-abstract-via-multidim-KMeans.
**Compatibility:** Generated themes shall be valid RON syntax compatible with rmpc 0.9.0 theme schema.
**Usability:** System shall require minimal user configuration beyond adding script to `on_song_change`.

## Implementation Breakdown

This section will track AI-IMP tickets as they are created:

- [x] AI-IMP-001: Create Rust CLI theme generator tool structure and argument parsing (Completed 2025-09-29)
- [x] AI-IMP-002: Implement color-to-theme-element mapping algorithm with HSV/Lab analysis (Completed 2025-09-29)
- [x] AI-IMP-003: Implement RON theme file generation with proper formatting (Completed 2025-09-29)
- [x] AI-IMP-004: Create shell wrapper script for on_song_change hook (Completed 2025-09-29)
- [x] AI-IMP-005: Integration testing with rmpc and multiple diverse album artworks (Completed 2025-09-29)
- [x] AI-IMP-006: Error handling and fallback logic for edge cases (Completed 2025-09-30)