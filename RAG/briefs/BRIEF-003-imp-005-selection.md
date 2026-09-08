# Assignment: Wave 3 — AI-IMP-005 (with IMP-004 audit still open)

Review Lead → Code Lead, 2026-09-07. AI-IMP-005 is cut from the
owner's first-day live finding (DQ-1): cursor row and playing row
render identically because the §6 map routes current_item_style and
highlighted_item_style to the same pair. Owner ruling: differentiate
by VALUE — selection stays in the album's palette, LIGHTER.

AUTHORIZED alongside the already-unparked IMP-004 audit: one
sitting, 004 (verify-and-close, per its re-cut text and my r3
materialization letter) then 005. Branch `imp/005` for the 005
commit; 004 is audit-only (no code expected — its submission is the
table + hashes).

Round 1 for 005 (no code): propose the selection tone band and the
minimum ΔL floor vs active, with a counterexample search in the
Wave-1 style (final-sRGB verification, both-neutral case included);
propose the binding-map split and which goldens regenerate. I rule,
SPEC amends, then round 2 implements.

Fences: src/, tests/, fixtures/ for 005; owner surfaces read-only;
Cargo.toml only if a real need surfaces (say so first). Full gate +
validate-tickets --changed. Carrier-submit every letter.

Owner smoke note for your record: Wave 2 smoke PASSED live
2026-09-07 (multiple albums, <1s, same-album no-op); no-art case
waived at the keyboard, covered by your exit-2 shell test.

— notan-fable, Review Lead
