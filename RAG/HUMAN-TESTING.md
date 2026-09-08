# Human testing queue

Items that need the owner at the keyboard (live rmpc, real album art, perceptual checks). One entry per item with the ticket it gates.

## 2026-09-06 — Wave 2 owner sitting (gates AI-EPIC-001 close)

1. RL applies base-theme materialization (16 None paths, SPEC §7
   rev 0.7) live + archive; owner eyeballs the static look.
2. Run `./install.sh --wire-hook` (backup is automatic); confirm
   wiring output.
3. Live smoke: song change rethemes < 1 s; same-album track = no-op
   (no flicker); kill the generator mid-run — theme intact; no-art
   track = clean base look.
4. Optional: flip osc11=true in config.toml and watch Ghostty tint.

**2026-09-07 result**: PASSED by owner in live use — retheme <1s
across multiple albums (Float, Addict Rhythms), same-album no-op
confirmed, materialized static look confirmed equivalent. No-art
case WAIVED at the keyboard (no convenient artless record in
rotation); covered by the exit-2 shell-integration test in temp
HOME. Mid-run kill considered covered by the same suite. One live
finding filed: DQ-1 / AI-IMP-005 (selection vs playing row).
