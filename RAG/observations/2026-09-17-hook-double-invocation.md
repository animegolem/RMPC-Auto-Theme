# Observation: on_song_change fires twice concurrently per change

Reported by the termviz seat (users-bb) 2026-09-17 while tracing
consumer surfaces; reproduced by RL same day.

## Evidence

`~/.config/rmpc-auto-theme/hook.log`: 6 of 143 analyze_image lines
show intra-line interleaving, e.g.
`[analyze_image] merge_threshold=[analyze_image] merge_threshold=0.0.008080` —
two concurrent `rmpc-theme-gen` processes writing unbuffered stderr
to the shared `2>>` log.

## Analysis (hook/on_song_change.sh)

- **No torn theme file is possible:** writes are mktemp + `mv`
  (atomic, same filesystem), and the `latest-event` token gates
  application at :42/:44. Consumer-observable state is old-or-new,
  never partial. The reporter's torn-file worry is answered.
- **Extraction runs twice per change:** both invocations read the
  stale `art.sha256` before either writes it, so both run the full
  kmeans. Loser exits at the token check having done all the work.
- Root cause of the double fire not yet traced: candidates are two
  live rmpc instances each running the hook, or double event
  delivery from one instance. Needs a trace before the fix is
  designed (evidence first; no restart-shaped guesses).

## Disposition

Fix rides AI-IMP-006 (publisher work already touches this hook for
publish-per-event): single-flight guard (flock or mkdir lock) around
extraction, per-invocation log prefix or line buffering for stderr.
Also note the same-art early-exit at :31 is the known contract gap
(same-art/different-file) already bound into IMP-006 acceptance
criteria per theme-engine-consumption r7 — one hook revision covers
all three.
