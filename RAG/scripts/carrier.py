#!/usr/bin/env python3
"""carrier — durable notification core (Phase A, shadow).

Constitution (notification-architecture consultation, settled
2026-07-17): files are the auditable mailbox; the ledger is
delivery state; a wake is only a hint; the matching application
ACK is truth.

Laws enforced here:
- Payload artifact is written atomically BEFORE the index row;
  a row is only created after stat + digest match on disk.
- Receipts are atomic, digest-keyed, timestamps IN the body;
  a receipt may only record a fact NOT derivable from the
  artifact tree (application-ACK, surface-notified, attempts).
- A DB-only ACK is forbidden: ack writes the receipt first,
  then advances the index.
- The index is a MATERIALIZED VIEW: `rebuild` reconstructs every
  semantic state from artifacts + receipts alone (failure proof
  #11). In-flight leases are transient and expire on rebuild.

Artifacts live repo-local:   <repo>/<channel>/carrier/
Receipts live repo-local:    <repo>/<channel>/carrier/receipts/
Index lives user-global:     $CARRIER_STATE_DIR or ~/.local/state/carrier

Env: CARRIER_CHANNEL names the channel dir (default ".codex");
CARRIER_STATE_DIR relocates the index. Both optional.
"""

import argparse, hashlib, json, os, secrets, sqlite3, sys, tempfile
from datetime import datetime, timedelta, timezone
from pathlib import Path

DB_DIR = Path(os.environ.get("CARRIER_STATE_DIR",
                             str(Path.home() / ".local/state/carrier")))
DB = DB_DIR / "index.db"
CHANNEL = os.environ.get("CARRIER_CHANNEL", ".codex")

SCHEMA = """
CREATE TABLE IF NOT EXISTS events (
  event_id TEXT PRIMARY KEY,
  namespace TEXT NOT NULL,
  topic TEXT NOT NULL,
  direction TEXT NOT NULL,
  round INTEGER NOT NULL,
  artifact_path TEXT NOT NULL,
  digest TEXT NOT NULL,
  created_ts TEXT NOT NULL,
  state TEXT NOT NULL,           -- pending | notified | acked
  attempts INTEGER NOT NULL DEFAULT 0,
  lease_ts TEXT,                 -- transient; expired on rebuild
  last_error TEXT
);
CREATE INDEX IF NOT EXISTS idx_ns_topic ON events(namespace, topic);
CREATE TABLE IF NOT EXISTS delivery_attempts (
  attempt_id TEXT PRIMARY KEY,
  namespace TEXT NOT NULL,
  recipient TEXT NOT NULL,
  topic TEXT NOT NULL,
  direction TEXT NOT NULL,
  delivery_key TEXT NOT NULL,
  event_ids_json TEXT NOT NULL,
  state TEXT NOT NULL,
  claimed_ts TEXT NOT NULL,
  accepted_ts TEXT,
  completed_ts TEXT,
  failed_ts TEXT,
  target TEXT,
  target_version TEXT,
  turn_id TEXT,
  retry_at TEXT,
  last_error TEXT,
  claim_token TEXT
);
CREATE INDEX IF NOT EXISTS idx_delivery_ns
  ON delivery_attempts(namespace, recipient, claimed_ts);
"""

EVENT_MIGRATIONS = {
    "lease_token": "TEXT",
    "lease_owner": "TEXT",
    "lease_expires_ts": "TEXT",
    "retry_at": "TEXT",
    "last_attempt_id": "TEXT",
    "last_accept_ts": "TEXT",
    "last_complete_ts": "TEXT",
}

# A claim is one topic only. Five consecutive events is large enough to
# coalesce ordinary review churn while keeping poison isolation and recovery
# payloads bounded. It is deliberately named and finite (Phase-C r1 ruling 3).
MAX_CLAIM_EVENTS = 5
DEFAULT_LEASE_SECONDS = 120


def now() -> str:
    return datetime.now(timezone.utc).isoformat()


def die(msg: str, code: int = 2):
    print(f"carrier: {msg}", file=sys.stderr)
    sys.exit(code)


def db() -> sqlite3.Connection:
    DB_DIR.mkdir(parents=True, exist_ok=True)
    c = sqlite3.connect(DB)
    c.execute("PRAGMA busy_timeout=5000")  # two leads may ack in the same instant
    c.executescript(SCHEMA)
    c.execute("BEGIN IMMEDIATE")
    columns = {r[1] for r in c.execute("PRAGMA table_info(events)")}
    for name, sql_type in EVENT_MIGRATIONS.items():
        if name not in columns:
            c.execute(f"ALTER TABLE events ADD COLUMN {name} {sql_type}")
    c.commit()
    return c


def digest_of(p: Path) -> str:
    return hashlib.sha256(p.read_bytes()).hexdigest()


def atomic_write(dest: Path, data: bytes):
    dest.parent.mkdir(parents=True, exist_ok=True)
    fd, tmp = tempfile.mkstemp(dir=dest.parent, suffix=".tmp")
    with os.fdopen(fd, "wb") as f:
        f.write(data)
    os.replace(tmp, dest)


def carrier_dir(repo: Path) -> Path:
    return repo / CHANNEL / "carrier"


def receipt_path(repo: Path, event_id: str, kind: str) -> Path:
    return carrier_dir(repo) / "receipts" / f"{event_id}.{kind}.json"


def attempt_receipt_path(repo: Path, attempt_id: str, kind: str) -> Path:
    return (carrier_dir(repo) / "receipts" / "attempts" /
            f"{attempt_id}.{kind}.json")


def namespace_of(repo: Path) -> str:
    return str(repo.resolve())


def parse_ts(value: str) -> datetime:
    return datetime.fromisoformat(value)


def delivery_key(recipient: str, events: list[dict]) -> str:
    members = sorted(f"{e['event_id']}:{e['digest']}" for e in events)
    digest = hashlib.sha256(
        f"{recipient}|{'|'.join(members)}".encode()).hexdigest()
    return f"{recipient}:{digest}"


def _attempt_body(attempt_id: str, kind: str, delivery: dict,
                  **facts) -> dict:
    body = {
        "attempt_id": attempt_id,
        "kind": kind,
        "delivery_key": delivery["delivery_key"],
        "recipient": delivery["recipient"],
        "topic": delivery["topic"],
        "direction": delivery["direction"],
        "events": delivery["events"],
        "ts": facts.pop("ts", None) or now(),
    }
    body.update({k: v for k, v in facts.items() if v is not None})
    return body


def _write_attempt_receipt(repo: Path, attempt_id: str, kind: str,
                           delivery: dict, **facts) -> dict:
    dest = attempt_receipt_path(repo, attempt_id, kind)
    proposed = _attempt_body(attempt_id, kind, delivery, **facts)
    if dest.exists():
        try:
            prior = json.loads(dest.read_text())
        except ValueError:
            die(f"corrupt attempt receipt {dest} — refusing")
        immutable = ("attempt_id", "kind", "delivery_key", "recipient",
                     "topic", "direction", "events")
        if all(prior.get(k) == proposed.get(k) for k in immutable):
            for key, value in facts.items():
                if value is not None and prior.get(key) != value:
                    die(f"conflicting attempt receipt at {dest} — refusing")
            return prior
        die(f"conflicting attempt receipt at {dest} — refusing")
    atomic_write(dest, json.dumps(proposed, indent=1, sort_keys=True).encode())
    return proposed


def _attempt_delivery(row) -> dict:
    return {
        "attempt_id": row[0],
        "recipient": row[1],
        "topic": row[2],
        "direction": row[3],
        "delivery_key": row[4],
        "events": json.loads(row[5]),
    }


def _load_claim(c, repo: Path, token: str):
    row = c.execute(
        "SELECT attempt_id,recipient,topic,direction,delivery_key,"
        "event_ids_json,state,claim_token FROM delivery_attempts "
        "WHERE namespace=? AND claim_token=?",
        (namespace_of(repo), token)).fetchone()
    if not row:
        die("unknown or expired claim token")
    return row


def _event_payload(c, repo: Path, event_id: str):
    row = c.execute(
        "SELECT event_id,topic,direction,round,artifact_path,digest,state,"
        "created_ts,lease_token,lease_expires_ts,retry_at "
        "FROM events WHERE namespace=? AND event_id=?",
        (namespace_of(repo), event_id)).fetchone()
    if not row:
        die(f"unknown event {event_id}")
    return {
        "event_id": row[0], "topic": row[1], "direction": row[2],
        "round": row[3], "artifact_path": row[4], "digest": row[5],
        "state": row[6], "created_ts": row[7], "lease_token": row[8],
        "lease_expires_ts": row[9], "retry_at": row[10],
    }


def event_identity(ns: str, topic: str, direction: str, rnd: int,
                   digest: str) -> str:
    """Deterministic event id (post-drill review r2): producers are
    structurally idempotent — the same message can never mint two
    events, regardless of DB state or retry."""
    key = f"{ns}|{topic}|{direction}|{rnd}|{digest}"
    return hashlib.sha256(key.encode()).hexdigest()[:16]


def _register_existing(c, repo: Path, ns: str, eid: str, topic: str,
                       direction: str, rnd: int, artifact: Path,
                       digest: str):
    """Index an already-published artifact (recovered-DB case):
    state and birth come from receipts, exactly as rebuild derives
    them — never a fresh pending over an acked history."""
    born = receipt_path(repo, eid, "born")
    if not born.exists():
        die(f"artifact without born receipt: {artifact.name}")
    b = json.loads(born.read_text())
    if b.get("digest") != digest or b.get("event_id") != eid:
        die(f"born receipt mismatch for {eid}")
    state = "pending"
    for kind, st in (("notified", "notified"), ("ack", "acked")):
        if receipt_path(repo, eid, kind).exists():
            state = st
    c.execute(
        "INSERT OR IGNORE INTO events (event_id,namespace,topic,direction,"
        "round,artifact_path,digest,created_ts,state) VALUES (?,?,?,?,?,?,?,?,?)",
        (eid, ns, topic, direction, rnd, str(artifact), digest, b["ts"], state))


def cmd_submit(a):
    repo = Path(a.repo)
    payload = Path(a.file).read_bytes()
    digest = hashlib.sha256(payload).hexdigest()
    ns = namespace_of(repo)
    event_id = event_identity(ns, a.topic, a.direction, a.round, digest)
    born_ts = now()
    name = f"{a.topic}-{a.direction}-r{a.round}-{event_id}.md"
    artifact = carrier_dir(repo) / name
    if artifact.exists():
        if digest_of(artifact) != digest:
            die("identity collision with different content — refusing")
        with db() as c:
            _register_existing(c, repo, ns, event_id, a.topic, a.direction,
                               a.round, artifact, digest)
        print(event_id)
        return
    # Publication order (post-drill review P1): birth truth first —
    # a born receipt without an artifact is harmless and ignored by
    # rebuild; an artifact must never exist without its birth.
    _write_receipt(repo, event_id, "born", digest, ts=born_ts)
    atomic_write(artifact, payload)
    if digest_of(artifact) != digest:
        die("artifact digest mismatch after write — refusing to index")
    with db() as c:
        c.execute(
            "INSERT INTO events (event_id,namespace,topic,direction,round,"
            "artifact_path,digest,created_ts,state) VALUES (?,?,?,?,?,?,?,?,?)",
            (event_id, namespace_of(repo), a.topic, a.direction, a.round,
             str(artifact), digest, born_ts, "pending"),
        )
    print(event_id)


def _write_receipt(repo: Path, event_id: str, kind: str, digest: str,
                   ts: str | None = None):
    dest = receipt_path(repo, event_id, kind)
    if dest.exists():
        try:
            prior = json.loads(dest.read_text())
        except ValueError:
            die(f"corrupt receipt {dest} — refusing")
        if (prior.get("event_id") == event_id and prior.get("kind") == kind
                and prior.get("digest") == digest):
            return  # idempotent: existing valid receipt stands byte-for-byte
        die(f"conflicting receipt at {dest} — refusing to overwrite")
    body = json.dumps(
        {"event_id": event_id, "kind": kind, "digest": digest,
         "ts": ts or now()}, indent=1).encode()
    atomic_write(dest, body)


def _receipt_first_transition(a, kind: str, new_state: str):
    repo = Path(a.repo)
    with db() as c:
        row = c.execute(
            "SELECT artifact_path, digest, state FROM events WHERE event_id=? "
            "AND namespace=?", (a.event_id, namespace_of(repo))).fetchone()
    if not row:
        die(f"unknown event {a.event_id} (run rebuild if artifacts exist)")
    artifact, digest, state = Path(row[0]), row[1], row[2]
    if not artifact.exists() or digest_of(artifact) != digest:
        die("artifact missing or digest mismatch — refusing receipt")
    # Law: receipt before index; DB-only transitions are forbidden.
    _write_receipt(repo, a.event_id, kind, digest)
    # acked outranks notified; never downgrade.
    if not (state == "acked" and new_state == "notified"):
        with db() as c:
            c.execute(
                "UPDATE events SET state=?,lease_ts=NULL,lease_token=NULL,"
                "lease_owner=NULL,lease_expires_ts=NULL WHERE event_id=?",
                (new_state, a.event_id))
    print(f"{new_state}: {a.event_id}")


def cmd_ack(a):
    _receipt_first_transition(a, "ack", "acked")


def cmd_notified(a):
    _receipt_first_transition(a, "notified", "notified")


def _eligible_at(value: str | None, instant: datetime) -> bool:
    return value is None or parse_ts(value) <= instant


def cmd_claim(a):
    repo = Path(a.repo)
    ns = namespace_of(repo)
    instant = datetime.now(timezone.utc)
    lease_seconds = a.lease_seconds
    if lease_seconds < 10 or lease_seconds > 3600:
        die("lease-seconds must be between 10 and 3600")
    limit = min(a.limit, MAX_CLAIM_EVENTS)
    if limit < 1:
        die("limit must be positive")

    with db() as c:
        c.execute("BEGIN IMMEDIATE")
        rows = c.execute(
            "SELECT event_id,topic,direction,round,artifact_path,digest,state,"
            "created_ts,lease_token,lease_expires_ts,retry_at "
            "FROM events WHERE namespace=? AND direction=? AND state!='acked' "
            "ORDER BY created_ts,event_id", (ns, a.direction)).fetchall()

        # Receipt truth outranks a lagging materialized index.
        for row in rows:
            rp = receipt_path(repo, row[0], "ack")
            if rp.exists():
                body = json.loads(rp.read_text())
                if (body.get("event_id") != row[0]
                        or body.get("kind") != "ack"
                        or body.get("digest") != row[5]):
                    die(f"ACK receipt mismatch for {row[0]}")
                c.execute(
                    "UPDATE events SET state='acked',lease_token=NULL,"
                    "lease_owner=NULL,lease_expires_ts=NULL WHERE event_id=?",
                    (row[0],))
        rows = [r for r in rows if not receipt_path(repo, r[0], "ack").exists()]

        topic_rows: dict[str, list] = {}
        topic_order: list[str] = []
        for row in rows:
            if row[1] not in topic_rows:
                topic_order.append(row[1])
                topic_rows[row[1]] = []
            topic_rows[row[1]].append(row)

        chosen = None
        for topic in topic_order:
            head = topic_rows[topic][0]
            lease_active = (
                head[8] is not None and head[9] is not None
                and parse_ts(head[9]) > instant)
            if lease_active or not _eligible_at(head[10], instant):
                continue
            artifact = Path(head[4])
            if not artifact.exists() or digest_of(artifact) != head[5]:
                c.execute(
                    "UPDATE events SET last_error=? WHERE event_id=?",
                    ("artifact missing or digest mismatch at claim", head[0]))
                continue
            chosen = topic
            break

        if chosen is None:
            print(json.dumps({"claimed": False, "reason": "no eligible event"}))
            return

        events = []
        for row in topic_rows[chosen]:
            if len(events) >= limit:
                break
            lease_active = (
                row[8] is not None and row[9] is not None
                and parse_ts(row[9]) > instant)
            if lease_active or not _eligible_at(row[10], instant):
                break
            artifact = Path(row[4])
            if (not artifact.exists() or digest_of(artifact) != row[5]
                    or receipt_path(repo, row[0], "ack").exists()):
                break
            events.append({
                "event_id": row[0], "digest": row[5], "round": row[3],
                "artifact_path": row[4], "created_ts": row[7],
            })
        if not events:
            print(json.dumps({"claimed": False, "reason": "no eligible event"}))
            return

        attempt_id = secrets.token_hex(16)
        # Hex cannot begin with '-' and is therefore safe as a CLI option value.
        claim_token = secrets.token_hex(24)
        expires = (instant + timedelta(seconds=lease_seconds)).isoformat()
        delivery = {
            "recipient": a.recipient,
            "topic": chosen,
            "direction": a.direction,
            "events": events,
        }
        delivery["delivery_key"] = delivery_key(a.recipient, events)
        started = _write_attempt_receipt(
            repo, attempt_id, "started", delivery, ts=instant.isoformat(),
            lease_owner=a.owner)
        c.execute(
            "INSERT INTO delivery_attempts "
            "(attempt_id,namespace,recipient,topic,direction,delivery_key,"
            "event_ids_json,state,claimed_ts,claim_token) "
            "VALUES (?,?,?,?,?,?,?,?,?,?)",
            (attempt_id, ns, a.recipient, chosen, a.direction,
             delivery["delivery_key"], json.dumps(events, sort_keys=True),
             "claimed", started["ts"], claim_token))
        for event in events:
            c.execute(
                "UPDATE events SET attempts=attempts+1,lease_ts=?,"
                "lease_token=?,lease_owner=?,lease_expires_ts=?,"
                "last_attempt_id=?,last_error=NULL WHERE event_id=?",
                (instant.isoformat(), claim_token, a.owner, expires,
                 attempt_id, event["event_id"]))
        result = dict(delivery)
        result.update({
            "claimed": True, "attempt_id": attempt_id,
            "claim_token": claim_token, "lease_expires_ts": expires,
            "max_claim_events": MAX_CLAIM_EVENTS,
        })
        print(json.dumps(result, indent=2, sort_keys=True))


def cmd_confirm(a):
    repo = Path(a.repo)
    instant = datetime.now(timezone.utc)
    with db() as c:
        c.execute("BEGIN IMMEDIATE")
        row = _load_claim(c, repo, a.claim_token)
        delivery = _attempt_delivery(row)
        if row[6] not in ("claimed", "accepted"):
            die(f"claim is not active (state={row[6]})")
        confirmed = []
        for expected in delivery["events"]:
            event = _event_payload(c, repo, expected["event_id"])
            ack = receipt_path(repo, event["event_id"], "ack")
            if ack.exists():
                body = json.loads(ack.read_text())
                if (body.get("event_id") != event["event_id"]
                        or body.get("kind") != "ack"
                        or body.get("digest") != event["digest"]):
                    die(f"ACK receipt mismatch for {event['event_id']}")
                c.execute(
                    "UPDATE events SET state='acked',lease_token=NULL,"
                    "lease_owner=NULL,lease_expires_ts=NULL WHERE event_id=?",
                    (event["event_id"],))
                die(f"event already application-ACKed: {event['event_id']}")
            if event["lease_token"] != a.claim_token:
                die(f"lease ownership changed for {event['event_id']}")
            if (not event["lease_expires_ts"]
                    or parse_ts(event["lease_expires_ts"]) <= instant):
                die(f"lease expired for {event['event_id']}")
            if event["digest"] != expected["digest"]:
                die(f"digest changed in index for {event['event_id']}")
            artifact = Path(event["artifact_path"])
            if not artifact.exists() or digest_of(artifact) != event["digest"]:
                die(f"artifact missing or digest mismatch: {event['event_id']}")

            live = repo / CHANNEL / event["direction"] / f"{event['topic']}.md"
            live_topic, live_round = _parse_live(live) if live.exists() else (
                None, None)
            current = (
                live.exists() and live_topic == event["topic"]
                and live_round == event["round"]
                and digest_of(live) == event["digest"])
            confirmed.append({
                **expected,
                "classification": "CURRENT" if current else "HISTORY",
                "payload": artifact.read_text(),
            })
        result = dict(delivery)
        result["confirmed_at"] = instant.isoformat()
        result["events"] = confirmed
        print(json.dumps(result, indent=2, sort_keys=True))


def cmd_renew(a):
    repo = Path(a.repo)
    instant = datetime.now(timezone.utc)
    if a.lease_seconds < 10 or a.lease_seconds > 3600:
        die("lease-seconds must be between 10 and 3600")
    expires = (instant + timedelta(seconds=a.lease_seconds)).isoformat()
    with db() as c:
        c.execute("BEGIN IMMEDIATE")
        row = _load_claim(c, repo, a.claim_token)
        if row[6] not in ("claimed", "accepted"):
            die(f"claim is not renewable (state={row[6]})")
        lease_rows = c.execute(
            "SELECT lease_expires_ts FROM events "
            "WHERE namespace=? AND lease_token=?",
            (namespace_of(repo), a.claim_token)).fetchall()
        if not lease_rows:
            die("claim owns no events")
        if any(not item[0] or parse_ts(item[0]) <= instant
               for item in lease_rows):
            die("claim lease already expired")
        changed = c.execute(
            "UPDATE events SET lease_ts=?,lease_expires_ts=? "
            "WHERE namespace=? AND lease_token=?",
            (instant.isoformat(), expires, namespace_of(repo),
             a.claim_token)).rowcount
        if not changed:
            die("claim owns no events")
    print(json.dumps({"renewed": changed, "lease_expires_ts": expires}))


def _attempt_transition(a, kind: str, state: str):
    repo = Path(a.repo)
    with db() as c:
        c.execute("BEGIN IMMEDIATE")
        row = _load_claim(c, repo, a.claim_token)
        delivery = _attempt_delivery(row)
        facts = {}
        if hasattr(a, "reason"):
            facts["reason"] = a.reason
        receipt = _write_attempt_receipt(
            repo, delivery["attempt_id"], kind, delivery, **facts)
        c.execute(
            "UPDATE delivery_attempts SET state=?,claim_token=NULL,"
            "last_error=? WHERE attempt_id=?",
            (state, facts.get("reason"), delivery["attempt_id"]))
        c.execute(
            "UPDATE events SET lease_ts=NULL,lease_token=NULL,lease_owner=NULL,"
            "lease_expires_ts=NULL WHERE namespace=? AND lease_token=?",
            (namespace_of(repo), a.claim_token))
    print(json.dumps({"state": state, "attempt_id": delivery["attempt_id"],
                      "ts": receipt["ts"]}))


def cmd_release(a):
    _attempt_transition(a, "released", "released")


def cmd_fail(a):
    repo = Path(a.repo)
    instant = datetime.now(timezone.utc)
    retry_at = (instant + timedelta(seconds=max(0, a.retry_seconds))).isoformat()
    with db() as c:
        c.execute("BEGIN IMMEDIATE")
        row = _load_claim(c, repo, a.claim_token)
        delivery = _attempt_delivery(row)
        receipt = _write_attempt_receipt(
            repo, delivery["attempt_id"], "failed", delivery,
            error=a.error, retry_at=retry_at)
        c.execute(
            "UPDATE delivery_attempts SET state='failed',failed_ts=?,"
            "retry_at=?,last_error=?,claim_token=NULL WHERE attempt_id=?",
            (receipt["ts"], retry_at, a.error, delivery["attempt_id"]))
        c.execute(
            "UPDATE events SET lease_ts=NULL,lease_token=NULL,lease_owner=NULL,"
            "lease_expires_ts=NULL,retry_at=?,last_error=? "
            "WHERE namespace=? AND lease_token=?",
            (retry_at, a.error, namespace_of(repo), a.claim_token))
    print(json.dumps({"state": "failed", "attempt_id": delivery["attempt_id"],
                      "retry_at": retry_at, "error": a.error}))


def cmd_accepted(a):
    repo = Path(a.repo)
    with db() as c:
        c.execute("BEGIN IMMEDIATE")
        row = _load_claim(c, repo, a.claim_token)
        delivery = _attempt_delivery(row)
        receipt = _write_attempt_receipt(
            repo, delivery["attempt_id"], "accepted", delivery,
            target=a.target, target_version=a.target_version,
            turn_id=a.turn_id)
        c.execute(
            "UPDATE delivery_attempts SET state='accepted',accepted_ts=?,"
            "target=?,target_version=?,turn_id=? WHERE attempt_id=?",
            (receipt["ts"], a.target, a.target_version, a.turn_id,
             delivery["attempt_id"]))
        c.execute(
            "UPDATE events SET last_accept_ts=? "
            "WHERE namespace=? AND lease_token=?",
            (receipt["ts"], namespace_of(repo), a.claim_token))
    print(json.dumps({"state": "accepted", "attempt_id": delivery["attempt_id"],
                      "accepted_ts": receipt["ts"]}))


def cmd_completed(a):
    repo = Path(a.repo)
    with db() as c:
        c.execute("BEGIN IMMEDIATE")
        row = _load_claim(c, repo, a.claim_token)
        delivery = _attempt_delivery(row)
        receipt = _write_attempt_receipt(
            repo, delivery["attempt_id"], "completed", delivery,
            target=a.target, target_version=a.target_version,
            turn_id=a.turn_id)
        for event in delivery["events"]:
            artifact = Path(event["artifact_path"])
            if not artifact.exists() or digest_of(artifact) != event["digest"]:
                die(f"artifact missing or digest mismatch: {event['event_id']}")
            _write_receipt(repo, event["event_id"], "notified",
                           event["digest"], ts=receipt["ts"])
            c.execute(
                "UPDATE events SET state=CASE WHEN state='acked' THEN state "
                "ELSE 'notified' END,last_complete_ts=?,retry_at=NULL,"
                "last_error=NULL,lease_ts=NULL,lease_token=NULL,"
                "lease_owner=NULL,lease_expires_ts=NULL WHERE event_id=?",
                (receipt["ts"], event["event_id"]))
        c.execute(
            "UPDATE delivery_attempts SET state='completed',completed_ts=?,"
            "target=?,target_version=?,turn_id=?,claim_token=NULL "
            "WHERE attempt_id=?",
            (receipt["ts"], a.target, a.target_version, a.turn_id,
             delivery["attempt_id"]))
    print(json.dumps({"state": "completed",
                      "attempt_id": delivery["attempt_id"],
                      "completed_ts": receipt["ts"]}))


def delivery_status(repo: Path, recipient: str | None = None,
                    direction: str | None = None) -> dict:
    ns = namespace_of(repo)
    with db() as c:
        where = "namespace=?"
        params: list = [ns]
        if direction:
            where += " AND direction=?"
            params.append(direction)
        rows = c.execute(
            "SELECT topic,direction,round,state,created_ts,event_id,digest,"
            "lease_ts,lease_owner,lease_expires_ts,retry_at,last_error,"
            "last_accept_ts,last_complete_ts FROM events WHERE " + where +
            " ORDER BY (state='acked'),created_ts", params).fetchall()
        attempt_where = "namespace=?"
        attempt_params: list = [ns]
        if recipient:
            attempt_where += " AND recipient=?"
            attempt_params.append(recipient)
        if direction:
            attempt_where += " AND direction=?"
            attempt_params.append(direction)
        attempts = c.execute(
            "SELECT attempt_id,recipient,topic,direction,delivery_key,state,"
            "claimed_ts,accepted_ts,completed_ts,failed_ts,target,"
            "target_version,turn_id,retry_at,last_error FROM delivery_attempts "
            f"WHERE {attempt_where} ORDER BY claimed_ts",
            attempt_params).fetchall()
    pending = [r for r in rows if r[3] != "acked"]
    ack_facts = []
    for r in rows:
        rp = receipt_path(repo, r[5], "ack")
        if rp.exists():
            try:
                body = json.loads(rp.read_text())
                ack_facts.append({"event_id": r[5], "ts": body["ts"]})
            except (ValueError, KeyError):
                pass
    instant = datetime.now(timezone.utc)
    active_leases = [
        {"event_id": r[5], "topic": r[0], "lease_ts": r[7],
         "lease_owner": r[8], "lease_expires_ts": r[9]}
        for r in rows
        if r[9] is not None and parse_ts(r[9]) > instant
    ]
    errors = [
        {"event_id": r[5], "topic": r[0], "retry_at": r[10],
         "error": r[11]}
        for r in rows if r[11] is not None
    ]
    acceptances = [r[7] for r in attempts if r[7]]
    completions = [r[8] for r in attempts if r[8]]
    return {
        "namespace": ns,
        "event_count": len(rows),
        "pending_count": len(pending),
        "oldest_pending_ts": pending[0][4] if pending else None,
        "events": [
            {"topic": r[0], "direction": r[1], "round": r[2],
             "state": r[3], "created_ts": r[4], "event_id": r[5],
             "digest": r[6], "lease_ts": r[7], "lease_owner": r[8],
             "lease_expires_ts": r[9], "retry_at": r[10],
             "last_error": r[11], "last_accept_ts": r[12],
             "last_complete_ts": r[13]} for r in rows
        ],
        "active_leases": active_leases,
        "last_transport_acceptance": max(acceptances) if acceptances else None,
        "last_history_completion": max(completions) if completions else None,
        "last_application_ack": (
            max(ack_facts, key=lambda fact: fact["ts"]) if ack_facts else None),
        "retry_errors": errors,
        "attempts": [
            {"attempt_id": r[0], "recipient": r[1], "topic": r[2],
             "direction": r[3], "delivery_key": r[4], "state": r[5],
             "claimed_ts": r[6], "accepted_ts": r[7],
             "completed_ts": r[8], "failed_ts": r[9], "target": r[10],
             "target_version": r[11], "turn_id": r[12],
             "retry_at": r[13], "last_error": r[14]} for r in attempts
        ],
    }


def cmd_status(a):
    repo = Path(a.repo)
    snapshot = delivery_status(
        repo, getattr(a, "recipient", None), getattr(a, "direction", None))
    if getattr(a, "json", False):
        print(json.dumps(snapshot, indent=2, sort_keys=True))
        sys.exit(0 if not snapshot["pending_count"] else 1)
    rows = snapshot["events"]
    if not rows:
        print("no events in namespace")
        return
    print(f"events {len(rows)} · unacked {snapshot['pending_count']}")
    print("delivery truth:")
    print(f"  last acceptance: {snapshot['last_transport_acceptance'] or 'never'}")
    print(f"  last completion: {snapshot['last_history_completion'] or 'never'}")
    ack = snapshot["last_application_ack"]
    print(f"  last application ACK: {ack['ts'] if ack else 'never'}")
    print(f"  active leases: {len(snapshot['active_leases'])}")
    for event in rows:
        t, d, rnd, st, ts, eid = (
            event["topic"], event["direction"], event["round"],
            event["state"], event["created_ts"], event["event_id"])
        age = ""
        if st != "acked":
            dt = datetime.now(timezone.utc) - datetime.fromisoformat(ts)
            age = f"  (unacked {int(dt.total_seconds()//60)}m)"
        print(f"  {t} {d} r{rnd} [{st}] {eid}{age}")
    sys.exit(0 if not snapshot["pending_count"] else 1)


def _attempt_receipts(repo: Path) -> dict[str, dict[str, dict]]:
    root = carrier_dir(repo) / "receipts" / "attempts"
    grouped: dict[str, dict[str, dict]] = {}
    if not root.is_dir():
        return grouped
    for path in sorted(root.glob("*.json")):
        try:
            attempt_id, kind, suffix = path.name.rsplit(".", 2)
            if suffix != "json":
                continue
            body = json.loads(path.read_text())
        except (ValueError, OSError):
            die(f"rebuild refused: invalid attempt receipt {path.name}")
        if (body.get("attempt_id") != attempt_id or body.get("kind") != kind
                or not isinstance(body.get("events"), list)):
            die(f"rebuild refused: attempt receipt mismatch {path.name}")
        grouped.setdefault(attempt_id, {})[kind] = body
    return grouped


def cmd_rebuild(a):
    repo = Path(a.repo)
    ns = namespace_of(repo)
    cdir = carrier_dir(repo)
    rows = []
    for art in sorted(cdir.glob("*.md")):
        stem = art.stem  # <topic>-<direction>-r<N>-<eid>
        try:
            head, eid = stem.rsplit("-", 1)
            rest, rnd = head.rsplit("-r", 1)
            topic, direction = rest.rsplit("-", 1)
        except ValueError:
            print(f"skip unparseable artifact: {art.name}", file=sys.stderr)
            continue
        digest = digest_of(art)
        state = "pending"
        born = receipt_path(repo, eid, "born")
        if not born.exists():
            die(f"rebuild refused: no born receipt for {eid}")
        try:
            b = json.loads(born.read_text())
            datetime.fromisoformat(b["ts"])
        except (ValueError, KeyError):
            die(f"rebuild refused: unparseable born receipt for {eid}")
        if (b.get("event_id") != eid or b.get("kind") != "born"
                or b.get("digest") != digest):
            die(f"rebuild refused: born receipt mismatch for {eid}")
        ts = b["ts"]
        for kind, st in (("notified", "notified"), ("ack", "acked")):
            rp = receipt_path(repo, eid, kind)
            if rp.exists():
                body = json.loads(rp.read_text())
                if body.get("digest") != digest:
                    print(f"receipt/digest mismatch on {eid} — kept pending",
                          file=sys.stderr)
                    break
                state = st
        rows.append((eid, ns, topic, direction, int(rnd), str(art), digest,
                     ts, state))
    with db() as c:
        c.execute("DELETE FROM events WHERE namespace=?", (ns,))
        c.executemany(
            "INSERT INTO events (event_id,namespace,topic,direction,round,"
            "artifact_path,digest,created_ts,state) VALUES (?,?,?,?,?,?,?,?,?)",
            rows)
        c.execute("DELETE FROM delivery_attempts WHERE namespace=?", (ns,))
        attempts = _attempt_receipts(repo)
        ordered_attempts = sorted(
            attempts.items(),
            key=lambda item: item[1].get("started", {}).get("ts", ""))
        for attempt_id, facts in ordered_attempts:
            started = facts.get("started")
            if not started:
                die(f"rebuild refused: attempt {attempt_id} has no start")
            expected_key = delivery_key(started["recipient"], started["events"])
            if started.get("delivery_key") != expected_key:
                die(f"rebuild refused: delivery key mismatch {attempt_id}")
            immutable = ("delivery_key", "recipient", "topic", "direction",
                         "events")
            for kind, body in facts.items():
                if any(body.get(k) != started.get(k) for k in immutable):
                    die(f"rebuild refused: attempt fact mismatch "
                        f"{attempt_id}.{kind}")
                try:
                    parse_ts(body["ts"])
                except (ValueError, KeyError):
                    die(f"rebuild refused: invalid attempt time "
                        f"{attempt_id}.{kind}")
            accepted = facts.get("accepted")
            completed = facts.get("completed")
            failed = facts.get("failed")
            released = facts.get("released")
            if completed:
                state = "completed"
            elif failed:
                state = "failed"
            elif released:
                state = "released"
            else:
                # Claims and accepted transports are runtime coordination.
                # Rebuild deliberately expires them; history is the retry gate.
                state = "expired"
            final = completed or failed or released or accepted or started
            c.execute(
                "INSERT INTO delivery_attempts "
                "(attempt_id,namespace,recipient,topic,direction,delivery_key,"
                "event_ids_json,state,claimed_ts,accepted_ts,completed_ts,"
                "failed_ts,target,target_version,turn_id,retry_at,last_error,"
                "claim_token) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,NULL)",
                (attempt_id, ns, started["recipient"], started["topic"],
                 started["direction"], started["delivery_key"],
                 json.dumps(started["events"], sort_keys=True), state,
                 started["ts"], accepted["ts"] if accepted else None,
                 completed["ts"] if completed else None,
                 failed["ts"] if failed else None, final.get("target"),
                 final.get("target_version"), final.get("turn_id"),
                 failed.get("retry_at") if failed else None,
                 failed.get("error") if failed else None))
            for event in started["events"]:
                values = {
                    "accept": accepted["ts"] if accepted else None,
                    "complete": completed["ts"] if completed else None,
                    "retry": failed.get("retry_at") if failed else None,
                    "error": failed.get("error") if failed else None,
                }
                c.execute(
                    "UPDATE events SET attempts=attempts+1,"
                    "last_accept_ts=CASE WHEN ? IS NULL THEN last_accept_ts "
                    "ELSE ? END,"
                    "last_complete_ts=CASE WHEN ? IS NULL THEN "
                    "last_complete_ts ELSE ? END,"
                    "retry_at=?,last_error=?,lease_ts=NULL,lease_token=NULL,"
                    "lease_owner=NULL,lease_expires_ts=NULL WHERE event_id=?",
                    (values["accept"], values["accept"], values["complete"],
                     values["complete"], values["retry"], values["error"],
                     event["event_id"]))
    print(f"rebuilt {len(rows)} event(s) from artifacts+receipts")


def _parse_live(path: Path):
    """Frontmatter (submission, round) from a live channel file."""
    topic = rnd = None
    try:
        for line in path.read_text().splitlines()[:12]:
            if line.startswith("submission:"):
                topic = line.split(":", 1)[1].strip()
            elif line.startswith("round:"):
                rnd = int(line.split(":", 1)[1].strip())
    except (ValueError, OSError):
        return None, None
    return topic, rnd


def cmd_sync(a):
    """Register live channel files as durable events, idempotent by
    (namespace, topic, direction, round, digest). Same round with a
    NEW digest is a NEW event (invariant 2: identity is immutable;
    changed content is a later message, never an overwrite)."""
    repo = Path(a.repo)
    ns = namespace_of(repo)
    chan = repo / CHANNEL
    registered = 0
    with db() as c:
        for direction in ("inbox", "outbox"):
            d = chan / direction
            if not d.is_dir():
                continue
            for f in sorted(d.glob("*.md")):
                topic, rnd = _parse_live(f)
                if topic is None or rnd is None:
                    continue
                digest = digest_of(f)
                event_id = event_identity(ns, topic, direction, rnd, digest)
                dup = c.execute("SELECT 1 FROM events WHERE event_id=?",
                                (event_id,)).fetchone()
                if dup:
                    continue
                artifact = carrier_dir(repo) / \
                    f"{topic}-{direction}-r{rnd}-{event_id}.md"
                if artifact.exists():
                    # DB-loss race (sync before rebuild): the canonical
                    # artifact already exists — re-index, never duplicate.
                    _register_existing(c, repo, ns, event_id, topic,
                                       direction, rnd, artifact, digest)
                    continue
                born_ts = now()
                _write_receipt(repo, event_id, "born", digest, ts=born_ts)
                atomic_write(artifact, f.read_bytes())
                if digest_of(artifact) != digest:
                    die("artifact digest mismatch during sync")
                c.execute(
                    "INSERT INTO events (event_id,namespace,topic,direction,"
                    "round,artifact_path,digest,created_ts,state) "
                    "VALUES (?,?,?,?,?,?,?,?,?)",
                    (event_id, ns, topic, direction, rnd, str(artifact),
                     digest, born_ts, "pending"))
                registered += 1
    print(f"sync: {registered} new event(s)")


def cmd_ack_file(a):
    """ACK the event whose digest matches this live file's CURRENT
    content — acks exactly what was read, nothing newer."""
    repo = Path(a.repo)
    f = Path(a.file)
    if not f.exists():
        die(f"no such file {f}")
    digest = digest_of(f)
    topic, rnd = _parse_live(f)
    direction = f.parent.name
    if topic is None or rnd is None or direction not in ("inbox", "outbox"):
        die("cannot derive (topic, direction, round) from this file")
    with db() as c:
        row = c.execute(
            "SELECT event_id, state FROM events WHERE namespace=? AND "
            "topic=? AND direction=? AND round=? AND digest=? "
            "ORDER BY created_ts DESC LIMIT 1",
            (namespace_of(repo), topic, direction, rnd, digest)).fetchone()
    if not row:
        die("no event matches this file's digest (run sync first)")
    if row[1] == "acked":
        print(f"acked: {row[0]} (already)")
        return
    a.event_id = row[0]
    _receipt_first_transition(a, "ack", "acked")


def main():
    p = argparse.ArgumentParser(prog="carrier")
    sub = p.add_subparsers(dest="cmd", required=True)
    for name, fn in (("submit", cmd_submit), ("ack", cmd_ack),
                     ("notified", cmd_notified), ("status", cmd_status),
                     ("rebuild", cmd_rebuild), ("sync", cmd_sync),
                     ("ack-file", cmd_ack_file), ("claim", cmd_claim),
                     ("confirm", cmd_confirm), ("renew", cmd_renew),
                     ("release", cmd_release), ("fail", cmd_fail),
                     ("accepted", cmd_accepted),
                     ("completed", cmd_completed)):
        s = sub.add_parser(name)
        s.add_argument("--repo", required=True)
        if name == "submit":
            s.add_argument("--topic", required=True)
            s.add_argument("--direction", required=True,
                           choices=["inbox", "outbox"])
            s.add_argument("--round", type=int, required=True)
            s.add_argument("--file", required=True)
        elif name in ("ack", "notified"):
            s.add_argument("--event-id", required=True)
        elif name == "ack-file":
            s.add_argument("--file", required=True)
        elif name == "claim":
            s.add_argument("--recipient", required=True)
            s.add_argument("--direction", required=True,
                           choices=["inbox", "outbox"])
            s.add_argument("--owner", required=True)
            s.add_argument("--limit", type=int, default=MAX_CLAIM_EVENTS)
            s.add_argument("--lease-seconds", type=int,
                           default=DEFAULT_LEASE_SECONDS)
        elif name in ("confirm", "release"):
            s.add_argument("--claim-token", required=True)
            if name == "release":
                s.add_argument("--reason", required=True)
        elif name == "renew":
            s.add_argument("--claim-token", required=True)
            s.add_argument("--lease-seconds", type=int,
                           default=DEFAULT_LEASE_SECONDS)
        elif name == "fail":
            s.add_argument("--claim-token", required=True)
            s.add_argument("--error", required=True)
            s.add_argument("--retry-seconds", type=int, default=15)
        elif name in ("accepted", "completed"):
            s.add_argument("--claim-token", required=True)
            s.add_argument("--target", required=True)
            s.add_argument("--target-version", required=True)
            s.add_argument("--turn-id", required=True)
        elif name == "status":
            s.add_argument("--recipient")
            s.add_argument("--direction", choices=["inbox", "outbox"])
            s.add_argument("--json", action="store_true")
        s.set_defaults(fn=fn)
    a = p.parse_args()
    a.fn(a)


if __name__ == "__main__":
    main()
