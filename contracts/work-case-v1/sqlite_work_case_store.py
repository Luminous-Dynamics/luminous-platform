"""SQLite-backed local persistence reference for Work Case Contract V1.

This is a local conformance model, not a production API or guarantee for every
filesystem or deployment. Local mutations atomically write case, history,
idempotency result, external mapping, and a minimal pending outbox event.
"""
from __future__ import annotations

from collections.abc import Callable
from contextlib import closing
import json
from pathlib import Path
import re
import sqlite3
import time
from typing import Any

from work_case_model import (
    ALLOWED_TRANSITIONS, CaseActivity, CaseError, CaseKind, CaseState,
    EvidenceReference, ExternalReference, Forbidden, IdempotencyConflict,
    InvalidTransition, MappingConflict, NotFound, Principal, RevisionConflict,
    ValidationError, WorkCase, _case_id, _digest, _mutator, _nonempty,
    _principal, _timestamp,
)


class SQLiteWorkCaseStore:
    """Per-operation SQLite transactions; fail_at exists for fault-injection tests."""

    SCHEMA_VERSION = 1

    def __init__(self, database_path: str | Path) -> None:
        self.path = str(database_path)
        with closing(self._connect()) as db:
            db.execute("BEGIN IMMEDIATE")
            try:
                version = int(db.execute("PRAGMA user_version").fetchone()[0])
                if version > self.SCHEMA_VERSION:
                    raise RuntimeError("database schema is newer than this reference model")
                statements = (
                    """CREATE TABLE IF NOT EXISTS work_cases (
                        tenant_id TEXT NOT NULL, case_id TEXT NOT NULL,
                        revision INTEGER NOT NULL CHECK(revision >= 1), payload_json TEXT NOT NULL,
                        PRIMARY KEY(tenant_id, case_id))""",
                    """CREATE TABLE IF NOT EXISTS case_activity (
                        tenant_id TEXT NOT NULL, case_id TEXT NOT NULL, sequence INTEGER NOT NULL CHECK(sequence >= 1),
                        activity_id TEXT NOT NULL, command_id TEXT NOT NULL, actor_id TEXT NOT NULL,
                        actor_role TEXT NOT NULL, activity_type TEXT NOT NULL, occurred_at TEXT NOT NULL,
                        reason TEXT NOT NULL, prior_revision INTEGER NOT NULL, new_revision INTEGER NOT NULL,
                        details_json TEXT NOT NULL, PRIMARY KEY(tenant_id,case_id,sequence),
                        UNIQUE(tenant_id,activity_id),
                        FOREIGN KEY(tenant_id,case_id) REFERENCES work_cases(tenant_id,case_id))""",
                    """CREATE TABLE IF NOT EXISTS command_idempotency (
                        tenant_id TEXT NOT NULL, idempotency_key TEXT NOT NULL, command_digest TEXT NOT NULL,
                        result_json TEXT NOT NULL, recorded_at INTEGER NOT NULL,
                        PRIMARY KEY(tenant_id,idempotency_key))""",
                    """CREATE TABLE IF NOT EXISTS external_case_mappings (
                        tenant_id TEXT NOT NULL, connection_id TEXT NOT NULL, provider TEXT NOT NULL,
                        external_id TEXT NOT NULL, case_id TEXT NOT NULL, linked_at TEXT NOT NULL,
                        PRIMARY KEY(tenant_id,connection_id,provider,external_id),
                        FOREIGN KEY(tenant_id,case_id) REFERENCES work_cases(tenant_id,case_id))""",
                    """CREATE TABLE IF NOT EXISTS case_outbox (
                        outbox_seq INTEGER PRIMARY KEY AUTOINCREMENT, outbox_id TEXT NOT NULL UNIQUE,
                        tenant_id TEXT NOT NULL, case_id TEXT NOT NULL, revision INTEGER NOT NULL,
                        event_type TEXT NOT NULL, payload_json TEXT NOT NULL,
                        status TEXT NOT NULL CHECK(status IN ('PENDING','LEASED','DELIVERED')),
                        attempts INTEGER NOT NULL DEFAULT 0, lease_owner TEXT, lease_until INTEGER,
                        created_at TEXT NOT NULL, delivered_at TEXT,
                        FOREIGN KEY(tenant_id,case_id) REFERENCES work_cases(tenant_id,case_id))""",
                    """CREATE INDEX IF NOT EXISTS case_outbox_ready_idx ON case_outbox(status,lease_until,outbox_seq)""",
                    """CREATE INDEX IF NOT EXISTS case_activity_timeline_idx ON case_activity(tenant_id,case_id,sequence)""",
                )
                for statement in statements:
                    db.execute(statement)
                if version == 0:
                    db.execute(f"PRAGMA user_version = {self.SCHEMA_VERSION}")
                db.commit()
            except Exception:
                if db.in_transaction:
                    db.rollback()
                raise

    def _connect(self) -> sqlite3.Connection:
        db = sqlite3.connect(self.path, timeout=10, isolation_level=None)
        db.row_factory = sqlite3.Row
        db.execute("PRAGMA foreign_keys=ON")
        db.execute("PRAGMA busy_timeout=10000")
        db.execute("PRAGMA journal_mode=WAL")
        db.execute("PRAGMA synchronous=FULL")
        return db

    @staticmethod
    def _json(value: object) -> str:
        return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False, allow_nan=False)

    @staticmethod
    def _case_from_json(raw: str) -> WorkCase:
        value = json.loads(raw)
        return WorkCase(
            case_id=value["case_id"], tenant_id=value["tenant_id"], kind=CaseKind(value["kind"]),
            title=value["title"], summary=value["summary"], customer_id=value["customer_id"],
            site_id=value["site_id"], asset_ids=tuple(value["asset_ids"]), priority=value["priority"],
            state=CaseState(value["state"]), revision=value["revision"], assignee_id=value["assignee_id"],
            created_at=value["created_at"], updated_at=value["updated_at"],
            external_refs=tuple(ExternalReference(
                item["provider"], item["connection_id"], item["external_id"], item["linked_at"]
            ) for item in value["external_refs"]),
            evidence_refs=tuple(EvidenceReference(
                item["evidence_id"], item["digest_sha256"], item["classification"],
                item["evidence_kind"], item["issuer"], item["created_at"]
            ) for item in value["evidence_refs"]),
        )

    @staticmethod
    def _activity(row: sqlite3.Row) -> CaseActivity:
        return CaseActivity(
            sequence=row["sequence"], activity_id=row["activity_id"], command_id=row["command_id"],
            tenant_id=row["tenant_id"], case_id=row["case_id"], actor_id=row["actor_id"],
            actor_role=row["actor_role"], activity_type=row["activity_type"],
            occurred_at=row["occurred_at"], reason=row["reason"], prior_revision=row["prior_revision"],
            new_revision=row["new_revision"], details=tuple(sorted(json.loads(row["details_json"]).items())),
        )

    def _load(self, db: sqlite3.Connection, principal: Principal, case_id: str) -> WorkCase:
        row = db.execute(
            "SELECT revision,payload_json FROM work_cases WHERE tenant_id=? AND case_id=?",
            (principal.tenant_id, case_id),
        ).fetchone()
        if row is None:
            raise NotFound("work case not found")
        case = self._case_from_json(row["payload_json"])
        if case.revision != row["revision"]:
            raise RuntimeError("stored revision disagrees with serialized snapshot")
        return case

    def _mutate(self, principal: Principal, *, idempotency_key: str, operation: str,
                payload: dict[str, Any], command_id: str, occurred_at: str,
                action: Callable[[sqlite3.Connection], tuple[
                    WorkCase | None, WorkCase, str, str, dict[str, str],
                    Callable[[sqlite3.Connection], None] | None
                ]], fail_at: str | None = None) -> WorkCase:
        idem_key = _nonempty(idempotency_key, "idempotency_key", 200)
        cmd = _nonempty(command_id, "command_id", 128)
        timestamp = _timestamp(occurred_at)
        command_digest = _digest({
            "tenant_id": principal.tenant_id, "actor_id": principal.actor_id,
            "actor_role": principal.role, "operation": operation, "payload": payload,
        })
        with closing(self._connect()) as db:
            db.execute("BEGIN IMMEDIATE")
            try:
                prior_row = db.execute(
                    "SELECT command_digest,result_json FROM command_idempotency WHERE tenant_id=? AND idempotency_key=?",
                    (principal.tenant_id, idem_key),
                ).fetchone()
                if prior_row is not None:
                    if prior_row["command_digest"] != command_digest:
                        raise IdempotencyConflict("idempotency key represents different command semantics")
                    result = self._case_from_json(prior_row["result_json"])
                    db.commit()
                    return result

                previous, updated, activity_type, reason, details, side_effect = action(db)
                encoded = self._json(updated.to_dict())
                # A same-state retry under a new key is a no-op: it receives an idempotent
                # response but does not create a fake revision, history row, or outbox event.
                if previous is not None and updated == previous and side_effect is None:
                    db.execute(
                        "INSERT INTO command_idempotency VALUES(?,?,?,?,?)",
                        (principal.tenant_id, idem_key, command_digest, encoded, int(time.time())),
                    )
                    db.commit()
                    return updated

                if previous is None:
                    db.execute(
                        "INSERT INTO work_cases(tenant_id,case_id,revision,payload_json) VALUES(?,?,?,?)",
                        (updated.tenant_id, updated.case_id, updated.revision, encoded),
                    )
                else:
                    change = db.execute(
                        """UPDATE work_cases SET revision=?,payload_json=?
                           WHERE tenant_id=? AND case_id=? AND revision=?""",
                        (updated.revision, encoded, updated.tenant_id, updated.case_id, previous.revision),
                    )
                    if change.rowcount != 1:
                        raise RevisionConflict("case changed during the operation")
                if fail_at == "after_case":
                    raise RuntimeError("injected failure after case snapshot write")

                if side_effect is not None:
                    side_effect(db)
                if fail_at == "after_side_effect":
                    raise RuntimeError("injected failure after secondary mapping write")

                sequence = int(db.execute(
                    "SELECT COALESCE(MAX(sequence),0)+1 FROM case_activity WHERE tenant_id=? AND case_id=?",
                    (updated.tenant_id, updated.case_id),
                ).fetchone()[0])
                activity_id = f"{updated.case_id}:activity:{sequence}"
                db.execute(
                    """INSERT INTO case_activity(
                       tenant_id,case_id,sequence,activity_id,command_id,actor_id,actor_role,
                       activity_type,occurred_at,reason,prior_revision,new_revision,details_json)
                       VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?)""",
                    (updated.tenant_id, updated.case_id, sequence, activity_id, cmd,
                     principal.actor_id, principal.role, activity_type, timestamp, reason,
                     previous.revision if previous is not None else 0, updated.revision, self._json(details)),
                )
                if fail_at == "after_history":
                    raise RuntimeError("injected failure after activity write")

                event_material = {
                    "tenant_id": updated.tenant_id, "case_id": updated.case_id,
                    "revision": updated.revision, "activity_type": activity_type,
                    "command_id": cmd, "occurred_at": timestamp,
                }
                outbox_id = _digest({"domain": "luminous.work-case.outbox.v1", **event_material})
                db.execute(
                    """INSERT INTO case_outbox(
                       outbox_id,tenant_id,case_id,revision,event_type,payload_json,status,created_at)
                       VALUES(?,?,?,?,?,?,'PENDING',?)""",
                    (outbox_id, updated.tenant_id, updated.case_id, updated.revision,
                     f"io.luminousdynamics.workcase.{activity_type}", self._json(event_material), timestamp),
                )
                if fail_at == "after_outbox":
                    raise RuntimeError("injected failure after outbox write")

                db.execute(
                    "INSERT INTO command_idempotency VALUES(?,?,?,?,?)",
                    (principal.tenant_id, idem_key, command_digest, encoded, int(time.time())),
                )
                if fail_at == "before_commit":
                    raise RuntimeError("injected failure before commit")
                db.commit()
                return updated
            except Exception:
                if db.in_transaction:
                    db.rollback()
                raise

    def create_case(self, principal: Principal, *, idempotency_key: str, command_id: str,
                    occurred_at: str, case_id: str, kind: str, title: str, summary: str,
                    customer_id: str, site_id: str | None = None,
                    asset_ids: tuple[str, ...] | list[str] = (), priority: int = 3,
                    assignee_id: str | None = None, fail_at: str | None = None) -> WorkCase:
        _principal(principal)
        local_id, timestamp = _case_id(case_id), _timestamp(occurred_at)
        try:
            case_kind = CaseKind(kind)
        except ValueError as exc:
            raise ValidationError("unknown case kind") from exc
        clean_title, clean_summary = _nonempty(title, "title", 240), _nonempty(summary, "summary", 4000)
        customer = _nonempty(customer_id, "customer_id", 128)
        site = None if site_id is None else _nonempty(site_id, "site_id", 128)
        assets = tuple(_nonempty(item, "asset_id", 128) for item in asset_ids)
        if len(set(assets)) != len(assets):
            raise ValidationError("asset_ids must be unique")
        if isinstance(priority, bool) or not isinstance(priority, int) or not 1 <= priority <= 5:
            raise ValidationError("priority must be an integer from 1 to 5")
        assignee = None if assignee_id is None else _nonempty(assignee_id, "assignee_id", 128)
        if principal.role == "requester" and assignee is not None:
            raise Forbidden("requesters cannot assign work")
        payload = {"case_id": local_id, "kind": case_kind.value, "title": clean_title,
                   "summary": clean_summary, "customer_id": customer, "site_id": site,
                   "asset_ids": list(assets), "priority": priority, "assignee_id": assignee}

        def action(db):
            exists = db.execute(
                "SELECT 1 FROM work_cases WHERE tenant_id=? AND case_id=?",
                (principal.tenant_id, local_id),
            ).fetchone()
            if exists is not None:
                raise CaseError("local case_id already exists in this tenant")
            result = WorkCase(
                case_id=local_id, tenant_id=principal.tenant_id, kind=case_kind, title=clean_title,
                summary=clean_summary, customer_id=customer, site_id=site, asset_ids=assets, priority=priority,
                state=CaseState.OPEN, revision=1, assignee_id=assignee, created_at=timestamp, updated_at=timestamp,
            )
            return None, result, "case.created", "case created", {"kind": case_kind.value, "customer_id": customer}, None

        return self._mutate(principal, idempotency_key=idempotency_key, operation="case.create", payload=payload,
                            command_id=command_id, occurred_at=timestamp, action=action, fail_at=fail_at)

    def get_case(self, principal: Principal, case_id: str) -> WorkCase:
        _principal(principal)
        with closing(self._connect()) as db:
            return self._load(db, principal, _case_id(case_id))

    def history(self, principal: Principal, case_id: str) -> tuple[CaseActivity, ...]:
        _principal(principal)
        local_id = _case_id(case_id)
        with closing(self._connect()) as db:
            self._load(db, principal, local_id)
            rows = db.execute(
                "SELECT * FROM case_activity WHERE tenant_id=? AND case_id=? ORDER BY sequence",
                (principal.tenant_id, local_id),
            ).fetchall()
            return tuple(self._activity(row) for row in rows)

    def transition(self, principal: Principal, case_id: str, *, target_state: str,
                   expected_revision: int, reason: str, idempotency_key: str,
                   command_id: str, occurred_at: str, fail_at: str | None = None) -> WorkCase:
        _mutator(principal)
        local_id, explanation, timestamp = _case_id(case_id), _nonempty(reason, "reason", 1000), _timestamp(occurred_at)
        try:
            target = CaseState(target_state)
        except ValueError as exc:
            raise ValidationError("unknown target state") from exc
        payload = {"case_id": local_id, "target_state": target.value,
                   "expected_revision": expected_revision, "reason": explanation}

        def action(db):
            current = self._load(db, principal, local_id)
            if isinstance(expected_revision, bool) or not isinstance(expected_revision, int) or expected_revision < 1:
                raise ValidationError("expected_revision must be a positive integer")
            if current.revision != expected_revision:
                raise RevisionConflict(f"stale revision: expected {expected_revision}, current {current.revision}")
            if target not in ALLOWED_TRANSITIONS[current.state]:
                raise InvalidTransition(f"{current.state.value} -> {target.value} is not allowed")
            updated = WorkCase(**{**current.__dict__, "state": target,
                                  "revision": current.revision + 1, "updated_at": timestamp})
            return current, updated, "case.state_changed", explanation, {
                "from": current.state.value, "to": target.value,
            }, None

        return self._mutate(principal, idempotency_key=idempotency_key, operation="case.transition", payload=payload,
                            command_id=command_id, occurred_at=timestamp, action=action, fail_at=fail_at)

    def assign(self, principal: Principal, case_id: str, *, assignee_id: str | None,
               expected_revision: int, reason: str, idempotency_key: str,
               command_id: str, occurred_at: str, fail_at: str | None = None) -> WorkCase:
        _mutator(principal)
        local_id, explanation, timestamp = _case_id(case_id), _nonempty(reason, "reason", 1000), _timestamp(occurred_at)
        assignee = None if assignee_id is None else _nonempty(assignee_id, "assignee_id", 128)
        payload = {"case_id": local_id, "assignee_id": assignee,
                   "expected_revision": expected_revision, "reason": explanation}

        def action(db):
            current = self._load(db, principal, local_id)
            if isinstance(expected_revision, bool) or not isinstance(expected_revision, int) or expected_revision < 1:
                raise ValidationError("expected_revision must be a positive integer")
            if current.revision != expected_revision:
                raise RevisionConflict(f"stale revision: expected {expected_revision}, current {current.revision}")
            if current.state in {CaseState.CLOSED, CaseState.CANCELLED}:
                raise InvalidTransition("closed or cancelled cases cannot be reassigned")
            if current.assignee_id == assignee:
                return current, current, "case.assignment_unchanged", explanation, {"assignee_id": assignee or "unassigned"}, None
            updated = WorkCase(**{**current.__dict__, "assignee_id": assignee,
                                  "revision": current.revision + 1, "updated_at": timestamp})
            return current, updated, "case.assigned", explanation, {"assignee_id": assignee or "unassigned"}, None

        return self._mutate(principal, idempotency_key=idempotency_key, operation="case.assign", payload=payload,
                            command_id=command_id, occurred_at=timestamp, action=action, fail_at=fail_at)

    def add_evidence_reference(self, principal: Principal, case_id: str, *,
                               evidence_id: str, digest_sha256: str, classification: str,
                               evidence_kind: str, issuer: str, created_at: str,
                               expected_revision: int, reason: str, idempotency_key: str,
                               command_id: str, occurred_at: str, fail_at: str | None = None) -> WorkCase:
        _mutator(principal)
        local_id, ref_id = _case_id(case_id), _nonempty(evidence_id, "evidence_id", 128)
        digest = _nonempty(digest_sha256, "digest_sha256", 64)
        if not re.fullmatch(r"[0-9a-f]{64}", digest):
            raise ValidationError("digest must be 64 lowercase hexadecimal characters")
        if classification not in {"public", "internal", "confidential", "restricted"}:
            raise ValidationError("unsupported evidence classification")
        if evidence_kind not in {"observation", "customer_assertion", "simulation", "interpretation", "unverified"}:
            raise ValidationError("unsupported evidence kind")
        reference = EvidenceReference(ref_id, digest, classification, evidence_kind,
                                      _nonempty(issuer, "issuer", 128), _timestamp(created_at))
        explanation, timestamp = _nonempty(reason, "reason", 1000), _timestamp(occurred_at)
        payload = {"case_id": local_id, "evidence": reference.to_dict(),
                   "expected_revision": expected_revision, "reason": explanation}

        def action(db):
            current = self._load(db, principal, local_id)
            if isinstance(expected_revision, bool) or not isinstance(expected_revision, int) or current.revision != expected_revision:
                raise RevisionConflict("stale or invalid expected revision")
            existing = next((item for item in current.evidence_refs if item.evidence_id == ref_id), None)
            if existing is not None:
                if existing != reference:
                    raise CaseError("evidence ID already refers to different content")
                return current, current, "case.evidence_unchanged", explanation, {"evidence_id": ref_id}, None
            updated = WorkCase(**{**current.__dict__, "evidence_refs": current.evidence_refs + (reference,),
                                  "revision": current.revision + 1, "updated_at": timestamp})
            return current, updated, "case.evidence_linked", explanation, {
                "evidence_id": ref_id, "evidence_kind": evidence_kind, "classification": classification,
            }, None

        return self._mutate(principal, idempotency_key=idempotency_key, operation="case.add_evidence", payload=payload,
                            command_id=command_id, occurred_at=timestamp, action=action, fail_at=fail_at)

    def link_external_reference(self, principal: Principal, case_id: str, *,
                                provider: str, connection_id: str, external_id: str,
                                expected_revision: int, reason: str, idempotency_key: str,
                                command_id: str, occurred_at: str, fail_at: str | None = None) -> WorkCase:
        _mutator(principal)
        local_id, provider_key = _case_id(case_id), _nonempty(provider, "provider", 64).lower()
        if not re.fullmatch(r"[a-z0-9][a-z0-9.-]{0,63}", provider_key):
            raise ValidationError("invalid provider key")
        connection, foreign_id = _nonempty(connection_id, "connection_id", 128), _nonempty(external_id, "external_id", 256)
        explanation, timestamp = _nonempty(reason, "reason", 1000), _timestamp(occurred_at)
        payload = {"case_id": local_id, "provider": provider_key, "connection_id": connection,
                   "external_id": foreign_id, "expected_revision": expected_revision, "reason": explanation}

        def action(db):
            current = self._load(db, principal, local_id)
            if isinstance(expected_revision, bool) or not isinstance(expected_revision, int) or current.revision != expected_revision:
                raise RevisionConflict("stale or invalid expected revision")
            bound = db.execute(
                """SELECT case_id FROM external_case_mappings
                   WHERE tenant_id=? AND connection_id=? AND provider=? AND external_id=?""",
                (principal.tenant_id, connection, provider_key, foreign_id),
            ).fetchone()
            if bound is not None:
                if bound["case_id"] != local_id:
                    raise MappingConflict("external identifier already maps to another local case")
                existing = next((item for item in current.external_refs
                                 if (item.provider, item.connection_id, item.external_id) == (provider_key, connection, foreign_id)), None)
                if existing is None:
                    raise RuntimeError("external mapping table and case snapshot disagree")
                return current, current, "case.external_reference_unchanged", explanation, {
                    "provider": provider_key, "connection_id": connection, "external_id": foreign_id,
                }, None
            reference = ExternalReference(provider_key, connection, foreign_id, timestamp)
            updated = WorkCase(**{**current.__dict__, "external_refs": current.external_refs + (reference,),
                                  "revision": current.revision + 1, "updated_at": timestamp})
            def insert_mapping(connection_db):
                connection_db.execute(
                    """INSERT INTO external_case_mappings(tenant_id,connection_id,provider,external_id,case_id,linked_at)
                       VALUES(?,?,?,?,?,?)""",
                    (principal.tenant_id, connection, provider_key, foreign_id, local_id, timestamp),
                )
            return current, updated, "case.external_reference_linked", explanation, {
                "provider": provider_key, "connection_id": connection, "external_id": foreign_id,
            }, insert_mapping

        return self._mutate(principal, idempotency_key=idempotency_key, operation="case.link_external_reference",
                            payload=payload, command_id=command_id, occurred_at=timestamp, action=action, fail_at=fail_at)

    def find_by_external_reference(self, principal: Principal, provider: str,
                                   connection_id: str, external_id: str) -> WorkCase:
        _principal(principal)
        provider_key, connection = _nonempty(provider, "provider", 64).lower(), _nonempty(connection_id, "connection_id", 128)
        foreign_id = _nonempty(external_id, "external_id", 256)
        with closing(self._connect()) as db:
            row = db.execute(
                """SELECT case_id FROM external_case_mappings
                   WHERE tenant_id=? AND connection_id=? AND provider=? AND external_id=?""",
                (principal.tenant_id, connection, provider_key, foreign_id),
            ).fetchone()
            if row is None:
                raise NotFound("external reference not found")
            return self._load(db, principal, row["case_id"])

    def counts(self) -> dict[str, int]:
        tables = ("work_cases", "case_activity", "command_idempotency", "external_case_mappings", "case_outbox")
        with closing(self._connect()) as db:
            return {name: int(db.execute(f"SELECT COUNT(*) FROM {name}").fetchone()[0]) for name in tables}

    def integrity_check(self) -> tuple[str, ...]:
        with closing(self._connect()) as db:
            return tuple(row[0] for row in db.execute("PRAGMA integrity_check").fetchall())
