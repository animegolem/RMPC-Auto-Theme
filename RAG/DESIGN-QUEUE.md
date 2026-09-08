# Design queue

Open design questions awaiting a ruling in SPEC.org. One entry per question; resolved entries move to the SPEC revision that ruled them.

## DQ-1 — selection vs playing row are identical (owner, 2026-09-07)

Live finding, first day of real use: `current_item_style` (cursor)
and `highlighted_item_style` (playing row) both receive
highlight-text-on-active — two identical bands, no way to tell
cursor from playback. Owner ruling sketch: differentiate by VALUE
not hue — "only changing the values is enough so the selection is
lighter." Resolution: derived role `selection` = active projected
to a lighter tone band (same hue/chroma treatment); binding map
splits the two paths. RULED into AI-IMP-005; SPEC amendment rides
its round 1.

## DQ-2 — reactive lyrics layout via per-event base selection (2026-09-08)

Owner asked whether Now Playing can show the Lyrics pane only when
the track has lyrics. rmpc cannot (static splits, no conditional
panes) — but the engine already can: layout lives in the theme
document, `set theme -` swaps it live (proven at v3 rollout), and
the hook runs per song-change. Design sketch: ONE base + a
deterministic transform that removes the lyrics split (single
source, no drift); hook short-circuit key becomes (art hash,
lyrics-presence bool) so reflow happens only on transitions;
lyrics presence resolved the way rmpc resolves `.lrc` sidecars.
Status: banked, not cut — candidate first post-epic ticket.

## DQ-3 — ship the owner's v3 pair as the project default (2026-09-08)

Owner ruling: "this theme will be the example/default theme we
ship." Today §6 defaults the base to upstream example_theme. The
shipped default becomes a genericized v3 pair: the theme (np_header
+ tab chrome + labeled lyrics separator) and a sample config
(4 tabs, keybinds, hook wiring commented). Requires: SPEC §6
amendment, --print-base-theme content change, fixtures/default
regeneration (byte-exact provenance moves from upstream to this
repo), installer sample. RULED into AI-IMP-006.

## DQ-4 — chrome completeness: unmapped themed surfaces (owner, 2026-09-08)

Owner spotted live: volume slider + "56%", [zxcv] hints, NORMAL/
INSERT badge never take album colors. All are literals INSIDE the
theme document (components.np_header, components.input_mode) —
patchable by the existing engine, just outside the v1-inherited
binding map. Rolled into AI-IMP-006 with the separator label:
round 1 proposes the component-path mechanism (the emitter's walker
must handle array-nested styles) and a role assignment per surface
(sketch: NORMAL bg=active fg=highlight-text, INSERT bg=accent,
volume %/slider=accent, [zxcv]=frame, [Playing]=header-accent,
cava bar_color=frame — owner ruled a dimmer tone so the bars don't
compete with the art; wired live 2026-09-08, fifo-fed).

## DQ-5 — Ghostty live retheme via OSC (ballast reconnaissance, 2026-09-08)

Systems-lane findings (ballast, by socket letter, at owner request)
for §9's deferred multi-terminal recoloring; banked verbatim-in-
substance so nobody rediscovers it:

- The seam is `RolePalette` (roles.rs:5): a Ghostty target is a
  SECOND, simpler emitter consuming the palette — NOT a variant of
  patch_theme (RON-specific by construction).
- OSC 11 already ships (hook-owned, gated, off by default). Full
  retheme = OSC 10 (fg), 12 (cursor), 4;N (ANSI 0-15) — same
  delivery, same gate, more sequences. No config rewrite, no
  reload; Ghostty applies OSC live.
- Ghostty facts: `macos-titlebar-style = transparent` (owner's
  default) follows OSC 11 in real time for the top surface;
  `= tabs` makes the whole titlebar+tabbar match terminal bg — the
  owner's chrome question answered YES. Themes dir is
  ~/.config/ghostty/themes/ (not App Support). Ballast left a
  static snapshot theme `ember` there (unwired) and fixed an
  unrelated config parse error (backup config.bak-20260908).
- THE design problem (ruling needed before any code): role→ANSI.
  Eight roles vs sixteen slots WITH semantics (1=error, 2=success,
  4=info); flooding the album ramp makes git diff / test output
  illegible. Ballast's hand compromise (prior art, attributed):
  warm-SHIFT semantic slots toward the palette (scarlet red, olive
  green, album's own blue for 4) rather than flood. Our WCAG
  machinery should do this properly — bands per ANSI slot class?
- Hazards to rule: broadcast scope (writing OSC into ttys running
  other TUIs can corrupt them); OSC is NOT self-reverting — a
  terminal keeps the last album's palette after rmpc exits unless
  something restores (needs a restore-on-exit story, e.g. hook on
  MPD stop / rmpc exit emitting the ember/base palette).

Status: banked, not cut. Candidate post-epic alongside DQ-2; the
role→ANSI ruling is a design sitting with the owner first. Offered
by ballast: the hand-built 16-color palette as a starting fixture,
and live OSC testing from the systems side — accept both when cut.
