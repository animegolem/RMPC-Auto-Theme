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
