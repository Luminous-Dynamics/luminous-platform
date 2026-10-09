"""SQLite-backed conformance model for the Operations Event V1 seam.

This is a local reference harness, not a production receiver or provider client.
It demonstrates durable uniqueness, atomic local inbox/state/outbox writes,
conservative revision handling, quarantine minimization, and retryable leases.
"""
from __future__ import annotations

from collections.abc import Callable, Mapping
import hashlib
import json
from pathlib import Path
import re
from contextlib import closing
import sqlite3
import time
from typing import Any

from adapter_model import (
    ENVELOPE_VALIDATOR, INCIDENT_VALIDATOR, IdempotencyConflict,
    IngestionBoundary, NormalizationError, TenantBindingError,
)

RevisionComparator = Callable[[str, str], int | None]


def _json(value: object) -> bytes:
    return json.dumps(value, ensure_ascii=False, sort_keys=True,
                      separators=(",", ":"), allow_nan=False).encode("utf-8")


def _digest(domain: str, value: object) -> str:
    return hashlib.sha256(domain.encode("ascii") + b"\0" + _json(value)).hexdigest()


def _event_digest(event: Mapping[str, Any]) -> str:
    content = dict(event)
    content.pop("observedat", None)  # delivery-local, not source event meaning
    return _digest("ops.sqlite.event.v1", content)


def _effect_digest(event: Mapping[str, Any]) -> str:
    return _digest("ops.sqlite.effect.v1", {
        key: event.get(key) for key in (
            "tenantid", "source", "type", "subject", "dataschema",
            "producersystem", "actorkind", "actorid", "authorityref", "data"
        )
    })


def numeric_revision_compare(incoming: str, current: str) -> int | None:
    """Example only: configure only after the provider guarantees numeric ordering."""
    def ordinal(value: str) -> int | None:
        match = re.fullmatch(r"(?:rev(?:ision)?-)?([0-9]+)", value)
        return int(match.group(1)) if match else None
    a, b = ordinal(incoming), ordinal(current)
    return None if a is None or b is None else (a > b) - (a < b)


class SQLiteInbox:
    """SQLite transaction model; unknown source-revision order fails closed."""
    def __init__(self, database_path: str | Path, boundary: IngestionBoundary,
                 *, revision_compare: RevisionComparator | None = None) -> None:
        self.path = str(database_path)
        self.boundary = boundary
        self.revision_compare = revision_compare
        with closing(self._connection()) as db:
            db.executescript("""
              CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY, applied_at INTEGER NOT NULL);
              CREATE TABLE IF NOT EXISTS inbox_events (
                source TEXT NOT NULL, event_id TEXT NOT NULL, event_digest TEXT NOT NULL,
                connection_id TEXT NOT NULL, tenant_id TEXT NOT NULL, incident_id TEXT NOT NULL,
                revision TEXT NOT NULL, outcome TEXT NOT NULL
                  CHECK(outcome IN ('APPLIED','STALE','DUPLICATE')),
                received_at INTEGER NOT NULL, PRIMARY KEY(source,event_id));
              CREATE TABLE IF NOT EXISTS business_effects (
                connection_id TEXT NOT NULL, tenant_id TEXT NOT NULL, idempotency_key TEXT NOT NULL,
                effect_digest TEXT NOT NULL, source TEXT NOT NULL, event_id TEXT NOT NULL,
                PRIMARY KEY(connection_id,tenant_id,idempotency_key));
              CREATE TABLE IF NOT EXISTS incident_state (
                tenant_id TEXT NOT NULL, incident_id TEXT NOT NULL, revision TEXT NOT NULL,
                state_digest TEXT NOT NULL, payload_json TEXT NOT NULL, updated_at INTEGER NOT NULL,
                PRIMARY KEY(tenant_id,incident_id));
              CREATE TABLE IF NOT EXISTS outbox_events (
                outbox_id TEXT PRIMARY KEY, tenant_id TEXT NOT NULL, incident_id TEXT NOT NULL,
                source TEXT NOT NULL, source_event_id TEXT NOT NULL, payload_json TEXT NOT NULL,
                status TEXT NOT NULL CHECK(status IN ('PENDING','LEASED','DELIVERED')),
                attempts INTEGER NOT NULL DEFAULT 0, lease_owner TEXT, lease_until INTEGER,
                created_at INTEGER NOT NULL, delivered_at INTEGER);
              CREATE INDEX IF NOT EXISTS outbox_ready_idx
                ON outbox_events(status,lease_until,created_at);
              CREATE TABLE IF NOT EXISTS quarantined_events (
                quarantine_id INTEGER PRIMARY KEY AUTOINCREMENT, event_digest TEXT NOT NULL,
                reason_code TEXT NOT NULL, quarantined_at INTEGER NOT NULL);
            """)
            version = db.execute("SELECT COALESCE(MAX(version),0) FROM schema_migrations").fetchone()[0]
            if version > 1:
                raise RuntimeError("database schema is newer than this reference model")
            db.execute("INSERT OR IGNORE INTO schema_migrations VALUES (1,?)", (int(time.time()),))

    def _connection(self):
        db = sqlite3.connect(self.path, timeout=10, isolation_level=None)
        db.row_factory = sqlite3.Row
        db.execute("PRAGMA foreign_keys=ON")
        db.execute("PRAGMA busy_timeout=10000")
        return db

    def _validate(self, event: Mapping[str, Any]) -> None:
        if list(ENVELOPE_VALIDATOR.iter_errors(event)):
            raise NormalizationError("event envelope failed schema validation")
        if list(INCIDENT_VALIDATOR.iter_errors(event.get("data"))):
            raise NormalizationError("incident payload failed schema validation")

    def _bind(self, event: Mapping[str, Any]) -> tuple[str, str]:
        b = self.boundary
        if (event.get("source") != b.source_uri or event.get("actorid") != b.connection_id
                or event.get("producersystem") != b.producer_system):
            raise TenantBindingError("source, actor, or producer differs from trusted connection")
        data = event.get("data")
        ref = data.get("source_reference") if isinstance(data, Mapping) else None
        if not isinstance(ref, Mapping):
            raise NormalizationError("source reference must be an object")
        if ref.get("system") != b.producer_system or ref.get("resource_type") != "service_ticket":
            raise TenantBindingError("unsupported source system or resource type")
        company, ticket = ref.get("company_id"), ref.get("resource_id")
        tenant = b.company_tenant_map.get(company)
        incident = b.incident_id_map.get((company, ticket))
        if not tenant or event.get("tenantid") != tenant:
            raise TenantBindingError("company and tenant are not bound by trusted configuration")
        if not incident or event.get("subject") != f"incident/{incident}":
            raise TenantBindingError("ticket and local incident are not explicitly mapped")
        return tenant, incident

    def _quarantine(self, event: Mapping[str, Any], reason: str, now: int) -> str:
        try:
            item_hash = _digest("ops.sqlite.quarantine.v1", event)
        except (TypeError, ValueError):
            item_hash = hashlib.sha256(b"unencodable-event").hexdigest()
        # Deliberately persist only digest, reason code, and time—not the raw event.
        with closing(self._connection()) as db:
            db.execute("BEGIN IMMEDIATE")
            db.execute("INSERT INTO quarantined_events(event_digest,reason_code,quarantined_at) VALUES(?,?,?)",
                       (item_hash, reason, int(now)))
            db.commit()
        return "QUARANTINED"

    def accept(self, event: Mapping[str, Any], *, now: int | None = None,
               fail_at: str | None = None) -> str:
        """Atomically persist inbox, effect dedupe, state and outbox or roll all back.

        fail_at is a test-only crash seam. Schema-invalid input raises; binding and
        unorderable-revision observations are quarantined without business mutation.
        """
        timestamp = int(time.time()) if now is None else int(now)
        self._validate(event)
        try:
            tenant, incident = self._bind(event)
        except TenantBindingError:
            return self._quarantine(event, "SOURCE_TENANT_OR_RESOURCE_BINDING", timestamp)

        source, event_id = str(event["source"]), str(event["id"])
        event_hash, effect_hash = _event_digest(event), _effect_digest(event)
        connection_id, idem = self.boundary.connection_id, str(event["idempotencykey"])
        ref = event["data"]["source_reference"]
        revision = str(ref["revision"])
        payload = {
            "status": event["data"]["status"], "summary": event["data"]["summary"],
            "priority": event["data"].get("priority"), "source_reference": dict(ref),
        }
        state_hash = _digest("ops.sqlite.incident-state.v1", payload)
        db = self._connection()
        try:
            db.execute("BEGIN IMMEDIATE")
            prior = db.execute("SELECT event_digest FROM inbox_events WHERE source=? AND event_id=?",
                               (source, event_id)).fetchone()
            if prior:
                if prior["event_digest"] != event_hash:
                    raise IdempotencyConflict("source + event ID reused with changed content")
                db.rollback()
                return "DUPLICATE"

            prior_effect = db.execute(
                "SELECT effect_digest FROM business_effects WHERE connection_id=? AND tenant_id=? AND idempotency_key=?",
                (connection_id, tenant, idem)).fetchone()
            if prior_effect:
                if prior_effect["effect_digest"] != effect_hash:
                    raise IdempotencyConflict("business idempotency key reused with changed effect")
                db.execute("INSERT INTO inbox_events VALUES(?,?,?,?,?,?,?,?,?)",
                           (source,event_id,event_hash,connection_id,tenant,incident,revision,"DUPLICATE",timestamp))
                db.commit()
                return "DUPLICATE"

            current = db.execute("SELECT revision,state_digest FROM incident_state WHERE tenant_id=? AND incident_id=?",
                                 (tenant,incident)).fetchone()
            outcome = "APPLIED"
            if current:
                cmp = None if self.revision_compare is None else self.revision_compare(revision,current["revision"])
                if cmp not in (-1,0,1):
                    db.rollback()
                    return self._quarantine(event,"REVISION_ORDER_UNPROVEN",timestamp)
                if cmp < 0:
                    outcome = "STALE"
                elif cmp == 0:
                    if current["state_digest"] != state_hash:
                        raise IdempotencyConflict("same source revision describes conflicting state")
                    outcome = "DUPLICATE"

            db.execute("INSERT INTO inbox_events VALUES(?,?,?,?,?,?,?,?,?)",
                       (source,event_id,event_hash,connection_id,tenant,incident,revision,outcome,timestamp))
            if fail_at == "after_inbox":
                raise RuntimeError("injected crash after inbox write")
            db.execute("INSERT INTO business_effects VALUES(?,?,?,?,?,?)",
                       (connection_id,tenant,idem,effect_hash,source,event_id))
            if outcome == "STALE" or outcome == "DUPLICATE":
                db.commit()
                return outcome

            db.execute("""INSERT INTO incident_state VALUES(?,?,?,?,?,?)
              ON CONFLICT(tenant_id,incident_id) DO UPDATE SET revision=excluded.revision,
              state_digest=excluded.state_digest,payload_json=excluded.payload_json,updated_at=excluded.updated_at""",
              (tenant,incident,revision,state_hash,_json(payload).decode("utf-8"),timestamp))
            if fail_at == "after_state":
                raise RuntimeError("injected crash after state write")
            outbox_id = _digest("ops.sqlite.outbox-id.v1", {"source":source,"event_id":event_id})
            out_payload = {
                "type":"io.luminousdynamics.ops.incident.snapshot.applied.v1",
                "tenant_id":tenant,"incident_id":incident,
                "source_event":{"source":source,"id":event_id,"digest":event_hash},
                "state_digest":state_hash,"revision":revision,
            }
            db.execute("""INSERT INTO outbox_events
              (outbox_id,tenant_id,incident_id,source,source_event_id,payload_json,status,created_at)
              VALUES(?,?,?,?,?,?,'PENDING',?)""",
              (outbox_id,tenant,incident,source,event_id,_json(out_payload).decode("utf-8"),timestamp))
            if fail_at in ("after_outbox","before_commit"):
                raise RuntimeError("injected crash before commit")
            db.commit()
            return "ACCEPTED"
        except Exception:
            if db.in_transaction:
                db.rollback()
            raise
        finally:
            db.close()

    def claim_outbox(self, worker_id: str, *, now: int, lease_seconds: int = 30) -> dict[str, Any] | None:
        if not worker_id or lease_seconds <= 0:
            raise ValueError("worker ID and positive lease are required")
        db = self._connection()
        try:
            db.execute("BEGIN IMMEDIATE")
            row = db.execute("""SELECT o.* FROM outbox_events AS o
              WHERE (o.status='PENDING' OR (o.status='LEASED' AND o.lease_until<=?))
                AND NOT EXISTS (
                  SELECT 1 FROM outbox_events AS prior
                  WHERE prior.tenant_id=o.tenant_id AND prior.incident_id=o.incident_id
                    AND prior.status!='DELIVERED'
                    AND (prior.created_at<o.created_at OR
                      (prior.created_at=o.created_at AND prior.outbox_id<o.outbox_id))
                )
              ORDER BY o.created_at,o.outbox_id LIMIT 1""", (int(now),)).fetchone()
            if row is None:
                db.commit()
                return None
            db.execute("""UPDATE outbox_events SET status='LEASED',attempts=attempts+1,
              lease_owner=?,lease_until=? WHERE outbox_id=?""",
              (worker_id,int(now)+lease_seconds,row["outbox_id"]))
            claimed = dict(db.execute("SELECT * FROM outbox_events WHERE outbox_id=?",
                                      (row["outbox_id"],)).fetchone())
            db.commit()
            return claimed
        except Exception:
            if db.in_transaction:
                db.rollback()
            raise
        finally:
            db.close()

    def acknowledge_outbox(self, outbox_id: str, worker_id: str, *, now: int) -> bool:
        db = self._connection()
        try:
            db.execute("BEGIN IMMEDIATE")
            cur = db.execute("""UPDATE outbox_events SET status='DELIVERED',delivered_at=?,
              lease_owner=NULL,lease_until=NULL WHERE outbox_id=? AND status='LEASED'
              AND lease_owner=? AND lease_until>?""", (int(now),outbox_id,worker_id,int(now)))
            db.commit()
            return cur.rowcount == 1
        finally:
            db.close()

    def counts(self) -> dict[str, int]:
        with closing(self._connection()) as db:
            tables = ("inbox_events","business_effects","incident_state","outbox_events","quarantined_events")
            return {table:int(db.execute(f"SELECT COUNT(*) FROM {table}").fetchone()[0]) for table in tables}

    def get_incident(self, tenant_id: str, incident_id: str) -> dict[str, Any] | None:
        with closing(self._connection()) as db:
            row = db.execute("SELECT * FROM incident_state WHERE tenant_id=? AND incident_id=?",
                             (tenant_id,incident_id)).fetchone()
            if row is None:
                return None
            result = dict(row)
            result["payload"] = json.loads(result.pop("payload_json"))
            return result

    def list_outbox(self) -> list[dict[str, Any]]:
        with closing(self._connection()) as db:
            return [dict(row) for row in db.execute("SELECT * FROM outbox_events ORDER BY created_at,outbox_id")]
