# rmpc-auto-theme v2

Album-art-driven live theming for [rmpc](https://github.com/mierak/rmpc),
rebuilt for rmpc ≥ 0.11. v1 lives in `.archive/` (untracked; full git
history retained) — it worked, but it required a patched rmpc fork and
overwrote the user's entire theme from a frozen template. Neither is
true anymore.

## Why v2 is a different program

rmpc 0.11 removed every constraint v1 was built around:

| v1 pain | rmpc 0.11 answer |
|---|---|
| Fork + repaint patches to apply themes | `rmpc remote set theme <path|->` — live IPC swap, no restart, broadcasts to every running instance; or in-place theme-file write + built-in hot reload (default on; note: close-write only, **no** atomic-rename) |
| Temp-file + MIME-sniff dance for art | `rmpc albumart -o -` streams current art to stdout; exit codes 0/1/2 (no art)/3 (no song) |
| Frozen 230-line RON blob owning layout/header/tabs | Theme = colors only. Layout/tabs are static user config (tabs live in `config.ron`, chrome `layout` in the base theme); the generator patches **color tokens into the user's base theme** with the `ron` crate and emits a complete document |
| Polling/guessing song changes | `on_song_change` hook with env payload (`$FILE`, `$PID`, tags); `$PID` lets the hook target the exact instance: `gen … \| rmpc remote --pid $PID set theme -` |

Watchlist: the new `rmpcd` Lua-plugin daemon (song_change/idle hooks,
`mpd.read_picture`, `process.spawn`) is the eventual natural host, but
upstream calls it not-ready ("expect changes to basically everything").
Also `album_art.custom_loader` — a two-way art hook. Revisit both later.

### Hard-won gotchas (from the 0.11 source survey)

- **Partial themes are dangerous.** `UiConfigFile` is `#[serde(default)]`:
  omitted fields reset to *built-in defaults*, not the loaded theme.
  Always emit a complete theme document (base-theme patching gives us
  this for free). `assets/example_theme.ron` == `UiConfigFile::default()`
  (upstream test-guaranteed) — the canonical template.
- Hex colors must be exactly 7 bytes (`#rrggbb`); no shorthand, no alpha.
- Keep the three RON pragmas (`implicit_some`, `unwrap_newtypes`,
  `unwrap_variant_newtypes`) at the top of emitted themes.
- `tab_bar.enabled` is deprecated — hide the bar by omitting `Pane(Tabs)`
  from the theme `layout`.
- 0.11 changelog: "some color styling options changed, mainly
  `text_color`" — don't trust v1's template fields.
- Cava pane needs the `cava` binary (not installed yet: `brew install cava`).

## Architecture

```
on_song_change.sh (thin)                 rmpc-theme-gen (Rust)
  art-hash short-circuit          ┌──────────────────────────────────┐
  rmpc albumart -o - ──────────►  │ extract: color-core (OKLab k-means│
  exit 0 on every failure         │   from ~/git/color-tool-kmeans)   │
                                  │ assign: role solver + guardrails  │
                                  │ emit: ron-patch base theme tokens │
                                  └──────────────┬───────────────────┘
                                                 ▼
                                   rmpc remote --pid $PID set theme -
                                   (+ optional OSC 11 terminal bg)
```

**Extraction** comes from `~/git/color-tool-kmeans` — the parent project
v1's copy forked from, now far ahead: OKLab-only golden path, chroma-
compressing gamut mapping (hue-stable), alpha-aware sampling, quality
presets, radius-guarded cluster merge (dedup), snap-to-real centroids,
deterministic SIMD k-means. Plan: carve `color.rs + kmeans.rs +
image_pipeline.rs + merge.rs + analyze()` into a tauri-free `color-core`
crate in that repo's workspace and consume it by path dependency; never
copy the files again (that's how v1 rotted). Carry the CC BY 3.0
attribution (Laurent Jégou's Color-tool) with `color.rs`.

**Role assignment** ports v1's genuinely valuable layer (see
`.archive/src/rmpc_theme_gen.rs`, the const block at :53 and the solver
at :1366): real WCAG contrast (relative luminance, not L*), ΔE for
"visibly different", OKLCH hue separation with a neutral-chroma
exemption, the pairwise accent×active solver with escalating
strict→relaxed→best-effort passes, ladder search with clamps,
`synthesize_color_between` overshooting t∈[-0.3,1.3], and
provenance/confidence on every role for `--debug` audits.

**New doctrine — own lightness, harvest hue** (the Material-You/HCT
lesson; what pywal never solved): the readability failures in v1 came
from accepting art lightness and patching contrast afterward. v2 fixes a
tone (OKLab L) range per role up front — bg, text, accent, active each
get an L band chosen so WCAG minimums are satisfiable *by construction*
— and projects art colors into those bands, taking only hue+chroma from
the album. Guardrails then verify instead of rescue. Prior art studied:
pywal/pywal16, wpgtk, wallust (Rust pywal successor), matugen (Material
You HCT). None do WCAG-verified role assignment; matugen's tonal-palette
idea is the one worth stealing.

**Config** (v1's never-built AI-IMP-015, now load-bearing): TOML or RON
at `~/.config/rmpc-auto-theme/`: base theme path, k / quality preset,
per-role L bands and contrast minimums (defaults = v1's proven consts:
accent/text 4.5:1, active 3.5:1, frame 3.0:1, peer ΔE 25, hue delta 30°),
OSC 11 on/off.

### Kept from v1 (proven in daily use)

Art-content hash short-circuit (same album ⇒ no regeneration, no
flicker) with graceful no-hash-tool degradation · never-disrupt-playback
(`exit 0` on every hook failure path) · deterministic seeds (same art ⇒
same theme) · OSC 11 terminal-background sync · `rmpc remote status`
toast · role aliasing (inactive=frame, progress=accent, scrollbar=active)
so the template stays ~9 tokens.

### Dropped from v1

The fork and both repaint patches (retest against 0.11 before believing
any repaint bug still exists — upstream issue #328 context) · the
scrollbar-gutter saga and its flags · MIME-sniff art dance ·
whole-theme overwrite · the 2,440-line monolith · the stale k-means copy
(upstream fixed mini-batch count accounting after the fork).

## The UI redesign this serves

`~/git/personal-theming/RAG/design/` — four views on keybinds 1-4
(Library / Now Playing / Lyrics / Search), tab bar hidden, queue inside
Now Playing. That design is **static config**, not generator output:
`tabs:` + keybinds in `config.ron`, chrome in the base theme's `layout`.
The generator only ever touches color tokens, so the design and the
theming evolve independently. One handoff correction: tabs belong in
`config.ron` in 0.11, not theme.ron as the handoff assumed.

## Status

Pre-implementation. Next: cut the RAG epic (color-core extraction ·
generator core · hook + config · design-config rollout).
