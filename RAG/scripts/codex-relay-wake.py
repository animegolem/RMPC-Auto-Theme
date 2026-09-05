#!/usr/bin/env python3
"""Bounded carrier-to-Codex wake for a launchd filesystem hint.

The carrier ledger is delivery truth.  This helper persists only health and
wake intent, never channel bodies or authority, and retires an intent only
after carrier status proves that the watched direction has no unACKed event.
"""

from __future__ import annotations

import argparse
from dataclasses import dataclass
from datetime import datetime, timedelta, timezone
import hashlib
import json
import os
from pathlib import Path
import shlex
import signal
import subprocess
import sys
import tempfile
import time


RETRY_EXIT = 75
CONFIG_EXIT = 78


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat()


def atomic_json(path: Path, value: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    data = json.dumps(value, indent=2, sort_keys=True) + "\n"
    fd, temp = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as handle:
            handle.write(data)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temp, path)
    finally:
        try:
            os.unlink(temp)
        except FileNotFoundError:
            pass


def collect(repo: Path, watched: list[str]) -> list[Path]:
    found: set[Path] = set()
    for raw in watched:
        candidate = (
            (repo / raw).resolve()
            if not Path(raw).is_absolute()
            else Path(raw).resolve()
        )
        if candidate.is_dir():
            found.update(path for path in candidate.rglob("*.md") if path.is_file())
        elif candidate.is_file():
            found.add(candidate)
    return sorted(found, key=lambda path: str(path))


def fingerprint(repo: Path, watched: list[str]) -> tuple[str, list[dict]]:
    rows: list[dict] = []
    for path in collect(repo, watched):
        try:
            relative = str(path.relative_to(repo))
        except ValueError:
            relative = str(path)
        rows.append(
            {
                "path": relative,
                "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
            }
        )
    encoded = json.dumps(rows, separators=(",", ":"), sort_keys=True).encode()
    return hashlib.sha256(encoded).hexdigest(), rows


def quiet_fingerprint(
    repo: Path, watched: list[str], quiet_seconds: float, attempts: int
) -> tuple[str, list[dict]]:
    prior = fingerprint(repo, watched)
    for _ in range(attempts):
        if quiet_seconds:
            time.sleep(quiet_seconds)
        current = fingerprint(repo, watched)
        if current == prior:
            return current
        prior = current
    raise RuntimeError("watched files did not reach a quiet second sample")


def load_json(path: Path) -> dict | None:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
        return value if isinstance(value, dict) else None
    except (FileNotFoundError, ValueError, OSError):
        return None


@dataclass(frozen=True)
class CommandResult:
    returncode: int | None
    stdout: str
    stderr: str
    timed_out: bool
    launch_error: bool = False


def _terminate_owned_process(process: subprocess.Popen, grace_seconds: float) -> None:
    if process.poll() is not None:
        return
    try:
        if os.name == "posix":
            os.killpg(process.pid, signal.SIGTERM)
        else:
            process.terminate()
    except ProcessLookupError:
        return
    try:
        process.wait(timeout=grace_seconds)
        return
    except subprocess.TimeoutExpired:
        pass
    try:
        if os.name == "posix":
            os.killpg(process.pid, signal.SIGKILL)
        else:
            process.kill()
    except ProcessLookupError:
        return
    try:
        process.wait(timeout=grace_seconds)
    except subprocess.TimeoutExpired:
        # The caller still owns an outer communicate bound. Never turn the
        # kill escalation itself into an unbounded wait.
        return


def run_bounded(
    command: list[str],
    repo: Path,
    timeout_seconds: float,
    grace_seconds: float,
    env: dict[str, str] | None = None,
    capture_output: bool = True,
) -> CommandResult:
    stream = subprocess.PIPE if capture_output else subprocess.DEVNULL
    try:
        process = subprocess.Popen(
            command,
            cwd=repo,
            env=env,
            text=True,
            stdout=stream,
            stderr=stream,
            start_new_session=(os.name == "posix"),
        )
    except OSError:
        return CommandResult(None, "", "", False, True)
    try:
        stdout, stderr = process.communicate(timeout=timeout_seconds)
        return CommandResult(process.returncode, stdout or "", stderr or "", False)
    except subprocess.TimeoutExpired:
        _terminate_owned_process(process, grace_seconds)
        try:
            stdout, stderr = process.communicate(timeout=grace_seconds)
        except subprocess.TimeoutExpired:
            # A descendant that deliberately escaped the child-owned process
            # group may still hold an inherited pipe. Do not let that defeat
            # the outer delivery bound.
            if process.stdout:
                process.stdout.close()
            if process.stderr:
                process.stderr.close()
            stdout, stderr = "", ""
        return CommandResult(process.returncode, stdout or "", stderr or "", True)


def update_health(path: Path, **sections: dict) -> dict:
    health = load_json(path) or {"schema_version": 1}
    health["schema_version"] = 1
    for name, values in sections.items():
        current = health.get(name)
        if not isinstance(current, dict):
            current = {}
        current.update(values)
        health[name] = current
    atomic_json(path, health)
    return health


def carrier_environment(args: argparse.Namespace) -> dict[str, str]:
    env = os.environ.copy()
    env["CARRIER_STATE_DIR"] = str(args.carrier_state_dir.expanduser().resolve())
    return env


def carrier_truth(
    args: argparse.Namespace, repo: Path
) -> tuple[dict | None, str | None]:
    carrier = str(args.carrier_script.expanduser().resolve())
    common = [sys.executable, carrier]
    env = carrier_environment(args)
    synced = run_bounded(
        [*common, "sync", "--repo", str(repo)],
        repo,
        args.carrier_timeout_seconds,
        args.kill_grace_seconds,
        env,
    )
    if synced.timed_out:
        return None, "carrier_timeout"
    if synced.returncode != 0:
        return None, "carrier_error"

    status = run_bounded(
        [
            *common,
            "status",
            "--repo",
            str(repo),
            "--direction",
            args.direction,
            "--json",
        ],
        repo,
        args.carrier_timeout_seconds,
        args.kill_grace_seconds,
        env,
    )
    if status.timed_out:
        return None, "carrier_timeout"
    # Carrier status deliberately exits 1 while unACKed events exist.
    if status.returncode not in (0, 1):
        return None, "carrier_error"
    try:
        snapshot = json.loads(status.stdout)
        pending_count = snapshot["pending_count"]
    except (ValueError, KeyError, TypeError):
        return None, "carrier_status_invalid"
    if isinstance(pending_count, bool) or not isinstance(pending_count, int):
        return None, "carrier_status_invalid"
    if pending_count < 0:
        return None, "carrier_status_invalid"
    if (pending_count == 0) != (status.returncode == 0):
        return None, "carrier_status_invalid"
    return snapshot, None


def retry_at(failures: int, base_seconds: float, maximum_seconds: float) -> str:
    exponent = min(max(failures - 1, 0), 30)
    delay = min(maximum_seconds, base_seconds * (2**exponent))
    return (datetime.now(timezone.utc) + timedelta(seconds=delay)).isoformat()


def retry_due(target: dict) -> bool:
    raw = target.get("next_retry_at")
    if not isinstance(raw, str):
        return True
    try:
        return datetime.now(timezone.utc) >= datetime.fromisoformat(raw)
    except ValueError:
        return True


def record_failure(
    health_path: Path,
    pending_path: Path,
    target: dict,
    args: argparse.Namespace,
    outcome: str,
    started_at: str,
    returncode: int | None = None,
) -> int:
    failures = int(target.get("consecutive_failures", 0)) + 1
    target.update(
        {
            "attempts": int(target.get("attempts", 0)) + 1,
            "attempt_started_at": started_at,
            "attempt_finished_at": utc_now(),
            "consecutive_failures": failures,
            "next_retry_at": retry_at(
                failures, args.retry_seconds, args.max_retry_seconds
            ),
            "reason": outcome,
        }
    )
    atomic_json(pending_path, target)
    update_health(
        health_path,
        delivery={
            "attempt_started_at": started_at,
            "attempt_finished_at": target["attempt_finished_at"],
            "outcome": outcome,
            "consecutive_failures": failures,
            "next_retry_at": target["next_retry_at"],
            "returncode": returncode,
        },
    )
    print(f"codex-relay-wake: {outcome}", file=sys.stderr)
    return RETRY_EXIT


def run_once(args: argparse.Namespace) -> int:
    repo = args.repo.expanduser().resolve()
    state_dir = args.state_dir.expanduser().resolve()
    delivered_path = state_dir / "delivered.json"
    pending_path = state_dir / "pending.json"
    health_path = state_dir / "health.json"
    current_hash, rows = quiet_fingerprint(
        repo, args.watch, args.quiet_seconds, args.quiet_attempts
    )
    sampled_at = utc_now()
    update_health(
        health_path,
        detector={
            "sampled_at": sampled_at,
            "fingerprint": current_hash,
            "row_count": len(rows),
        },
    )

    delivered = load_json(delivered_path) or {}
    pending = load_json(pending_path)
    snapshot, carrier_error = carrier_truth(args, repo)
    if carrier_error:
        target = pending or {
            "fingerprint": current_hash,
            "first_seen_at": sampled_at,
            "watched": rows,
        }
        return record_failure(
            health_path,
            pending_path,
            target,
            args,
            carrier_error,
            sampled_at,
        )

    pending_count = snapshot["pending_count"]
    update_health(
        health_path,
        carrier={"checked_at": utc_now(), "pending_count": pending_count},
    )

    if args.probe:
        update_health(
            health_path,
            delivery={"outcome": "probe", "attempt_finished_at": utc_now()},
        )
        print(
            json.dumps(
                {
                    "carrier_pending_count": pending_count,
                    "current_fingerprint": current_hash,
                    "delivered_fingerprint": delivered.get("fingerprint"),
                    "pending": pending,
                    "watched": rows,
                },
                indent=2,
                sort_keys=True,
            )
        )
        return 0

    if args.prime:
        if args.expect_fingerprint != current_hash:
            update_health(
                health_path,
                delivery={"outcome": "prime_fingerprint_mismatch"},
            )
            print("codex-relay-wake: prime fingerprint mismatch", file=sys.stderr)
            return CONFIG_EXIT
        if pending_count != 0:
            update_health(
                health_path,
                delivery={"outcome": "prime_carrier_pending"},
            )
            print("codex-relay-wake: carrier has unACKed work", file=sys.stderr)
            return CONFIG_EXIT
        atomic_json(
            delivered_path,
            {
                "fingerprint": current_hash,
                "reconciled_at": utc_now(),
                "reason": "explicit_prime",
                "watched": rows,
            },
        )
        pending_path.unlink(missing_ok=True)
        update_health(
            health_path,
            delivery={
                "outcome": "primed",
                "consecutive_failures": 0,
                "next_retry_at": None,
            },
        )
        return 0

    if not delivered and not pending and pending_count == 0:
        update_health(health_path, delivery={"outcome": "baseline_required"})
        print(
            "codex-relay-wake: run --probe, then --prime with --expect-fingerprint",
            file=sys.stderr,
        )
        return CONFIG_EXIT

    if pending_count == 0 and delivered.get("fingerprint") == current_hash:
        atomic_json(
            delivered_path,
            {
                "fingerprint": current_hash,
                "reconciled_at": utc_now(),
                "reason": "carrier_application_ack",
                "watched": rows,
            },
        )
        pending_path.unlink(missing_ok=True)
        update_health(
            health_path,
            delivery={
                "outcome": "application_reconciled",
                "consecutive_failures": 0,
                "next_retry_at": None,
            },
        )
        return 0

    if pending and pending.get("fingerprint") == current_hash:
        target = pending
    else:
        target = {
            "fingerprint": current_hash,
            "first_seen_at": sampled_at,
            "watched": rows,
            "attempts": 0,
            "consecutive_failures": 0,
        }
    if not retry_due(target):
        update_health(
            health_path,
            delivery={
                "outcome": "backoff",
                "consecutive_failures": target.get("consecutive_failures", 0),
                "next_retry_at": target.get("next_retry_at"),
            },
        )
        atomic_json(pending_path, target)
        return RETRY_EXIT

    started_at = utc_now()
    target.update(
        {
            "attempt_started_at": started_at,
            "reason": "delivery_in_progress",
        }
    )
    atomic_json(pending_path, target)
    update_health(
        health_path,
        delivery={
            "attempt_started_at": started_at,
            "outcome": "delivery_in_progress",
        },
    )

    wake_command = [
        *shlex.split(args.codex_command),
        "exec",
        "resume",
        args.thread_id,
        args.prompt,
    ]
    woke = run_bounded(
        wake_command,
        repo,
        args.delivery_timeout_seconds,
        args.kill_grace_seconds,
        capture_output=False,
    )
    if woke.timed_out:
        return record_failure(
            health_path,
            pending_path,
            target,
            args,
            "delivery_timeout",
            started_at,
            woke.returncode,
        )
    if woke.returncode != 0:
        return record_failure(
            health_path,
            pending_path,
            target,
            args,
            "delivery_error",
            started_at,
            woke.returncode,
        )

    after, after_error = carrier_truth(args, repo)
    after_hash, after_rows = quiet_fingerprint(
        repo, args.watch, args.quiet_seconds, args.quiet_attempts
    )
    if after_hash != current_hash:
        changed = {
            "fingerprint": after_hash,
            "first_seen_at": utc_now(),
            "watched": after_rows,
            "attempts": 0,
            "consecutive_failures": 0,
            "reason": "changed_during_delivery",
        }
        atomic_json(pending_path, changed)
        update_health(
            health_path,
            detector={
                "sampled_at": utc_now(),
                "fingerprint": after_hash,
                "row_count": len(after_rows),
            },
            delivery={
                "attempt_finished_at": utc_now(),
                "outcome": "changed_during_delivery",
                "consecutive_failures": 0,
                "next_retry_at": None,
            },
        )
        return RETRY_EXIT
    if after_error:
        return record_failure(
            health_path,
            pending_path,
            target,
            args,
            after_error,
            started_at,
        )

    after_count = after["pending_count"]
    update_health(
        health_path,
        carrier={"checked_at": utc_now(), "pending_count": after_count},
    )
    if after_count != 0:
        return record_failure(
            health_path,
            pending_path,
            target,
            args,
            "application_ack_pending",
            started_at,
            0,
        )

    atomic_json(
        delivered_path,
        {
            "fingerprint": current_hash,
            "reconciled_at": utc_now(),
            "reason": "carrier_application_ack",
            "watched": rows,
        },
    )
    pending_path.unlink(missing_ok=True)
    update_health(
        health_path,
        delivery={
            "attempt_finished_at": utc_now(),
            "outcome": "application_reconciled",
            "consecutive_failures": 0,
            "next_retry_at": None,
            "returncode": 0,
        },
    )
    return 0


def positive(parser: argparse.ArgumentParser, name: str, value: float) -> None:
    if value <= 0:
        parser.error(f"{name} must be greater than zero")


def main() -> int:
    parser = argparse.ArgumentParser(prog="codex-relay-wake")
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--thread-id", required=True)
    parser.add_argument("--carrier-script", type=Path, required=True)
    parser.add_argument("--carrier-state-dir", type=Path, required=True)
    parser.add_argument("--state-dir", type=Path, required=True)
    parser.add_argument("--codex-command", default="codex")
    parser.add_argument("--prompt", default="scan the channel")
    parser.add_argument("--direction", choices=("inbox", "outbox"), default="outbox")
    parser.add_argument(
        "--watch",
        action="append",
        help="relative file or directory; repeatable (default: .codex/outbox)",
    )
    parser.add_argument("--quiet-seconds", type=float, default=0.35)
    parser.add_argument("--quiet-attempts", type=int, default=8)
    parser.add_argument("--carrier-timeout-seconds", type=float, default=30.0)
    parser.add_argument("--delivery-timeout-seconds", type=float, default=120.0)
    parser.add_argument("--kill-grace-seconds", type=float, default=2.0)
    parser.add_argument("--retry-seconds", type=float, default=15.0)
    parser.add_argument("--max-retry-seconds", type=float, default=300.0)
    modes = parser.add_mutually_exclusive_group()
    modes.add_argument("--probe", action="store_true")
    modes.add_argument("--prime", action="store_true")
    parser.add_argument("--expect-fingerprint")
    args = parser.parse_args()
    args.watch = args.watch or [".codex/outbox"]
    if args.prime and not args.expect_fingerprint:
        parser.error("--prime requires --expect-fingerprint")
    if args.expect_fingerprint and not args.prime:
        parser.error("--expect-fingerprint requires --prime")
    if args.quiet_seconds < 0:
        parser.error("--quiet-seconds must not be negative")
    if args.quiet_attempts < 1:
        parser.error("--quiet-attempts must be at least one")
    for name in (
        "carrier_timeout_seconds",
        "delivery_timeout_seconds",
        "kill_grace_seconds",
        "retry_seconds",
        "max_retry_seconds",
    ):
        positive(parser, f"--{name.replace('_', '-')}", getattr(args, name))
    if args.max_retry_seconds < args.retry_seconds:
        parser.error("--max-retry-seconds must be >= --retry-seconds")
    try:
        return run_once(args)
    except Exception as exc:
        print(f"codex-relay-wake: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
