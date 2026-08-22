#!/usr/bin/env python3
"""channel-watchdog.py — "Is the mailbox up?" (owner-proposed, 2026-08-08).

Fate-independent liveness check for the two-lead channel. Runs from
launchd on an interval, NEVER from inside an agent session — the
2026-08-08 incident showed both leads' ears dying with their sessions
while the carrier held every letter; a watchdog that shares fate with
the watchers it guards inherits exactly that failure.

The check is OUTCOME-based, not process-based, per house doctrine (the
ledger is truth; a wake is only a hint): a dead ear — whichever
component died — manifests as carrier events that stay unacked too
long. So the single health question is: does any born receipt lack its
ack beyond the threshold age?

Healthy → silent (launchd log line only).
Stale   → macOS notification to the owner + log line.

Usage:
  channel-watchdog.py [--receipts DIR] [--threshold-minutes N] [--verbose]
"""

import argparse
import datetime
import json
import pathlib
import subprocess
import sys

DEFAULT_RECEIPTS = None  # pass --receipts; no default checkout is assumed


def notify(message: str, title: str = "VampireOTP — channel watchdog") -> None:
    msg = message.replace('"', '\\"')
    ttl = title.replace('"', '\\"')
    subprocess.run(
        ["osascript", "-e",
         f'display notification "{msg}" with title "{ttl}" sound name "Sosumi"'],
        check=False,
    )


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--receipts", default=DEFAULT_RECEIPTS, required=True)
    ap.add_argument("--threshold-minutes", type=float, default=30.0)
    ap.add_argument("--verbose", action="store_true")
    args = ap.parse_args()

    receipts = pathlib.Path(args.receipts)
    if not receipts.is_dir():
        # The channel itself being absent is the loudest possible failure.
        notify(f"receipts directory missing: {receipts}")
        print(f"watchdog: ERROR receipts directory missing: {receipts}")
        return 2

    now = datetime.datetime.now(datetime.timezone.utc)
    stale = []
    pending = 0
    outbox_pending = 0
    carrier = receipts.parent
    for born in sorted(receipts.glob("*.born.json")):
        event_id = born.name.split(".")[0]
        if (receipts / f"{event_id}.ack.json").exists():
            continue
        # Direction-aware (2026-08-19 correction): the consultation lane has
        # no response SLA, so an outbox event awaiting the peer's ack is
        # ordinary correspondence, not a dead ear. Only INBOX events — mail
        # addressed to this seat — measure this mailbox's health. Outbox
        # pendings are logged, never notified.
        if not list(carrier.glob(f"*-inbox-r*-{event_id}.md")):
            outbox_pending += 1
            continue
        pending += 1
        try:
            ts = json.loads(born.read_text()).get("ts", "")
            born_at = datetime.datetime.fromisoformat(ts)
        except (ValueError, json.JSONDecodeError):
            # A born receipt that cannot be parsed is itself an alert.
            stale.append((event_id, float("inf")))
            continue
        age_min = (now - born_at).total_seconds() / 60.0
        if age_min > args.threshold_minutes:
            stale.append((event_id, age_min))

    if stale:
        oldest = max(a for _, a in stale)
        oldest_txt = "unparseable" if oldest == float("inf") else f"{oldest:.0f} min"
        msg = (f"mailbox may be down: {len(stale)} unacked INBOX event(s) past "
               f"{args.threshold_minutes:.0f} min (oldest {oldest_txt})")
        notify(msg)
        print(f"watchdog: STALE {msg}: " + ", ".join(e for e, _ in stale))
        return 1

    if args.verbose or pending or outbox_pending:
        print(f"watchdog: healthy ({pending} inbox in-flight within threshold; "
              f"{outbox_pending} outbox awaiting peer ack — not alerted)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
