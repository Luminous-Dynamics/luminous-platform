"""In-memory reference model for Native Work Case Contract V1.

This is an executable state-machine contract, not a production service or
durable store. Tenant scope comes from Principal, never from a command body.
"""
from __future__ import annotations

from dataclasses import dataclass, replace
from datetime import datetime
from enum import Enum
import hashlib
import json
import re
from threading import RLock
from typing import Any


class CaseError(Exception):
    """Base class for expected domain failures."""


class NotFound(CaseError):
    """Resource is absent or outside the caller's tenant."""


class Forbidden(CaseError):
    """The current principal is not allowed to mutate this resource."""


class ValidationError(CaseError):
    """A command is malformed or outside the contract."""


class InvalidTransition(CaseError):
    """The requested lifecycle transition is not allowed."""


class RevisionConflict(CaseError):
    """The command was based on a stale revision."""


class IdempotencyConflict(CaseError):
    """An idempotency key was reused for different command semantics."""


class MappingConflict(CaseError):
    """An external identifier is already bound to another local case."""


class CaseKind(str, Enum):
    INCIDENT = "incident"
    REQUEST = "request"
    PROBLEM = "problem"
    CHANGE = "change"


class CaseState(str, Enum):
    OPEN = "open"
    IN_PROGRESS = "in_progress"
    WAITING = "waiting"
    RESOLVED = "resolved"
    CLOSED = "closed"
    CANCELLED = "cancelled"


ALLOWED_TRANSITIONS = {
    CaseState.OPEN: frozenset({CaseState.IN_PROGRESS, CaseState.WAITING, CaseState.CANCELLED}),
    CaseState.IN_PROGRESS: frozenset({CaseState.WAITING, CaseState.RESOLVED, CaseState.CANCELLED}),
    CaseState.WAITING: frozenset({CaseState.IN_PROGRESS, CaseState.RESOLVED, CaseState.CANCELLED}),
    CaseState.RESOLVED: frozenset({CaseState.OPEN, CaseState.CLOSED}),
    CaseState.CLOSED: frozenset(),
    CaseState.CANCELLED: frozenset(),
}
MUTATION_ROLES = frozenset({"technician", "admin", "integration"})
KNOWN_ROLES = MUTATION_ROLES | frozenset({"requester"})


@dataclass(frozen=True)
class Principal:
    """Identity and tenant context established by the authenticated boundary."""
    tenant_id: str
    actor_id: str
    role: str


@dataclass(frozen=True)
class ExternalReference:
    provider: str
    external_id: str
    linked_at: str

    def to_dict(self) -> dict[str, str]:
        return {"provider": self.provider, "external_id": self.external_id, "linked_at": self.linked_at}


@dataclass(frozen=True)
class EvidenceReference:
    evidence_id: str
    digest_sha256: str
    classification: str
    evidence_kind: str
    issuer: str
    created_at: str

    def to_dict(self) -> dict[str, str]:
        return {
            "evidence_id": self.evidence_id,
            "digest_sha256": self.digest_sha256,
            "classification": self.classification,
            "evidence_kind": self.evidence_kind,
            "issuer": self.issuer,
            "created_at": self.created_at,
        }


@dataclass(frozen=True)
class WorkCase:
    case_id: str
    tenant_id: str
    kind: CaseKind
    title: str
    summary: str
    customer_id: str
    site_id: str | None
    asset_ids: tuple[str, ...]
    priority: int
    state: CaseState
    revision: int
    assignee_id: str | None
    created_at: str
    updated_at: str
    external_refs: tuple[ExternalReference, ...] = ()
    evidence_refs: tuple[EvidenceReference, ...] = ()

    def to_dict(self) -> dict[str, Any]:
        return {
            "schema_version": "work-case-v1",
            "case_id": self.case_id,
            "tenant_id": self.tenant_id,
            "kind": self.kind.value,
            "title": self.title,
            "summary": self.summary,
            "customer_id": self.customer_id,
            "site_id": self.site_id,
            "asset_ids": list(self.asset_ids),
            "priority": self.priority,
            "state": self.state.value,
            "revision": self.revision,
            "assignee_id": self.assignee_id,
            "created_at": self.created_at,
            "updated_at": self.updated_at,
            "external_refs": [item.to_dict() for item in self.external_refs],
            "evidence_refs": [item.to_dict() for item in self.evidence_refs],
        }


@dataclass(frozen=True)
class CaseActivity:
    sequence: int
    activity_id: str
    command_id: str
    tenant_id: str
    case_id: str
    actor_id: str
    actor_role: str
    activity_type: str
    occurred_at: str
    reason: str
    prior_revision: int
    new_revision: int
    details: tuple[tuple[str, str], ...] = ()

    def to_dict(self) -> dict[str, Any]:
        return {
            "sequence": self.sequence,
            "activity_id": self.activity_id,
            "command_id": self.command_id,
            "tenant_id": self.tenant_id,
            "case_id": self.case_id,
            "actor_id": self.actor_id,
            "actor_role": self.actor_role,
            "activity_type": self.activity_type,
            "occurred_at": self.occurred_at,
            "reason": self.reason,
            "prior_revision": self.prior_revision,
            "new_revision": self.new_revision,
            "details": dict(self.details),
        }


def _nonempty(value: object, field: str, max_len: int = 256) -> str:
    if not isinstance(value, str) or not value.strip() or len(value) > max_len:
        raise ValidationError(f"{field} must be a non-empty string no longer than {max_len} characters")
    return value.strip()


def _timestamp(value: object) -> str:
    text = _nonempty(value, "timestamp", 64)
    try:
        parsed = datetime.fromisoformat(text.replace("Z", "+00:00"))
    except ValueError as exc:
        raise ValidationError("timestamp must be ISO-8601 with a timezone") from exc
    if parsed.tzinfo is None or parsed.utcoffset() is None:
        raise ValidationError("timestamp must include a timezone")
    return text


def _digest(value: object) -> str:
    encoded = json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False, allow_nan=False)
    return hashlib.sha256(encoded.encode("utf-8")).hexdigest()


def _case_id(value: object) -> str:
    result = _nonempty(value, "case_id", 128)
    if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._:-]{0,127}", result):
        raise ValidationError("case_id must be an opaque local identifier")
    return result


def _principal(principal: Principal) -> None:
    _nonempty(principal.tenant_id, "principal.tenant_id", 128)
    _nonempty(principal.actor_id, "principal.actor_id", 128)
    if principal.role not in KNOWN_ROLES:
        raise Forbidden("unknown principal role")


def _mutator(principal: Principal) -> None:
    _principal(principal)
    if principal.role not in MUTATION_ROLES:
        raise Forbidden("current role cannot mutate work cases")


class WorkCaseStore:
    """Thread-safe in-memory reference only; restart and multi-process guarantees do not apply."""

    def __init__(self) -> None:
        self._cases: dict[tuple[str, str], WorkCase] = {}
        self._history: dict[tuple[str, str], list[CaseActivity]] = {}
        self._external_map: dict[tuple[str, str, str], str] = {}
        self._idempotency: dict[tuple[str, str], tuple[str, WorkCase]] = {}
        self._lock = RLock()

    def _execute(self, principal: Principal, key: str, operation: str,
                 payload: dict[str, Any], action) -> WorkCase:
        idempotency_key = _nonempty(key, "idempotency_key", 200)
        command_digest = _digest({
            "tenant_id": principal.tenant_id, "actor_id": principal.actor_id,
            "actor_role": principal.role, "operation": operation, "payload": payload,
        })
        cache_key = (principal.tenant_id, idempotency_key)
        with self._lock:
            previous = self._idempotency.get(cache_key)
            if previous is not None:
                prior_digest, prior_result = previous
                if prior_digest != command_digest:
                    raise IdempotencyConflict("idempotency key already represents a different command")
                return prior_result
            result = action()
            self._idempotency[cache_key] = (command_digest, result)
            return result

    def _owned(self, principal: Principal, case_id: str) -> WorkCase:
        result = self._cases.get((principal.tenant_id, case_id))
        if result is None:
            raise NotFound("work case not found")
        return result

    def _append(self, principal: Principal, prior: WorkCase | None, updated: WorkCase, *,
                command_id: str, occurred_at: str, activity_type: str, reason: str,
                details: dict[str, str] | None = None) -> WorkCase:
        key = (principal.tenant_id, updated.case_id)
        timeline = self._history.setdefault(key, [])
        old_revision = prior.revision if prior is not None else 0
        event = CaseActivity(
            sequence=len(timeline) + 1,
            activity_id=f"{updated.case_id}:activity:{len(timeline) + 1}",
            command_id=command_id, tenant_id=principal.tenant_id, case_id=updated.case_id,
            actor_id=principal.actor_id, actor_role=principal.role, activity_type=activity_type,
            occurred_at=occurred_at, reason=reason, prior_revision=old_revision,
            new_revision=updated.revision, details=tuple(sorted((details or {}).items())),
        )
        self._cases[key] = updated
        timeline.append(event)
        return updated

    def create_case(self, principal: Principal, *, idempotency_key: str, command_id: str,
                    occurred_at: str, case_id: str, kind: str, title: str, summary: str,
                    customer_id: str, site_id: str | None = None,
                    asset_ids: tuple[str, ...] | list[str] = (), priority: int = 3,
                    assignee_id: str | None = None) -> WorkCase:
        _principal(principal)
        cmd = _nonempty(command_id, "command_id", 128)
        timestamp = _timestamp(occurred_at)
        local_id = _case_id(case_id)
        try:
            case_kind = CaseKind(kind)
        except ValueError as exc:
            raise ValidationError("kind must be incident, request, problem, or change") from exc
        clean_title = _nonempty(title, "title", 240)
        clean_summary = _nonempty(summary, "summary", 4000)
        customer = _nonempty(customer_id, "customer_id", 128)
        site = None if site_id is None else _nonempty(site_id, "site_id", 128)
        assets = tuple(_nonempty(item, "asset_id", 128) for item in asset_ids)
        if len(set(assets)) != len(assets):
            raise ValidationError("asset_ids must not contain duplicates")
        if isinstance(priority, bool) or not isinstance(priority, int) or not 1 <= priority <= 5:
            raise ValidationError("priority must be an integer from 1 (highest) to 5 (lowest)")
        assignee = None if assignee_id is None else _nonempty(assignee_id, "assignee_id", 128)
        if principal.role == "requester" and assignee is not None:
            raise Forbidden("requesters cannot assign work")
        payload = {
            "case_id": local_id, "kind": case_kind.value, "title": clean_title, "summary": clean_summary,
            "customer_id": customer, "site_id": site, "asset_ids": list(assets),
            "priority": priority, "assignee_id": assignee,
        }

        def action() -> WorkCase:
            if (principal.tenant_id, local_id) in self._cases:
                raise CaseError("local case_id already exists in this tenant")
            record = WorkCase(
                case_id=local_id, tenant_id=principal.tenant_id, kind=case_kind,
                title=clean_title, summary=clean_summary, customer_id=customer, site_id=site,
                asset_ids=assets, priority=priority, state=CaseState.OPEN, revision=1,
                assignee_id=assignee, created_at=timestamp, updated_at=timestamp,
            )
            return self._append(principal, None, record, command_id=cmd, occurred_at=timestamp,
                                activity_type="case.created", reason="case created",
                                details={"kind": case_kind.value, "customer_id": customer})
        return self._execute(principal, idempotency_key, "case.create", payload, action)

    def get_case(self, principal: Principal, case_id: str) -> WorkCase:
        _principal(principal)
        with self._lock:
            return self._owned(principal, _case_id(case_id))

    def history(self, principal: Principal, case_id: str) -> tuple[CaseActivity, ...]:
        _principal(principal)
        local_id = _case_id(case_id)
        with self._lock:
            self._owned(principal, local_id)
            return tuple(self._history[(principal.tenant_id, local_id)])

    def _check_revision(self, current: WorkCase, expected: int) -> None:
        if isinstance(expected, bool) or not isinstance(expected, int) or expected < 1:
            raise ValidationError("expected_revision must be a positive integer")
        if current.revision != expected:
            raise RevisionConflict(f"stale revision: expected {expected}, current {current.revision}")

    def transition(self, principal: Principal, case_id: str, *, target_state: str,
                   expected_revision: int, reason: str, idempotency_key: str,
                   command_id: str, occurred_at: str) -> WorkCase:
        _mutator(principal)
        local_id = _case_id(case_id)
        cmd = _nonempty(command_id, "command_id", 128)
        explanation = _nonempty(reason, "reason", 1000)
        timestamp = _timestamp(occurred_at)
        try:
            target = CaseState(target_state)
        except ValueError as exc:
            raise ValidationError("unknown target_state") from exc
        payload = {"case_id": local_id, "target_state": target.value,
                   "expected_revision": expected_revision, "reason": explanation}

        def action() -> WorkCase:
            current = self._owned(principal, local_id)
            self._check_revision(current, expected_revision)
            if target not in ALLOWED_TRANSITIONS[current.state]:
                raise InvalidTransition(f"{current.state.value} -> {target.value} is not allowed")
            changed = replace(current, state=target, revision=current.revision + 1, updated_at=timestamp)
            return self._append(principal, current, changed, command_id=cmd, occurred_at=timestamp,
                                activity_type="case.state_changed", reason=explanation,
                                details={"from": current.state.value, "to": target.value})
        return self._execute(principal, idempotency_key, "case.transition", payload, action)

    def assign(self, principal: Principal, case_id: str, *, assignee_id: str | None,
               expected_revision: int, reason: str, idempotency_key: str,
               command_id: str, occurred_at: str) -> WorkCase:
        _mutator(principal)
        local_id = _case_id(case_id)
        assignee = None if assignee_id is None else _nonempty(assignee_id, "assignee_id", 128)
        explanation = _nonempty(reason, "reason", 1000)
        cmd = _nonempty(command_id, "command_id", 128)
        timestamp = _timestamp(occurred_at)
        payload = {"case_id": local_id, "assignee_id": assignee,
                   "expected_revision": expected_revision, "reason": explanation}

        def action() -> WorkCase:
            current = self._owned(principal, local_id)
            self._check_revision(current, expected_revision)
            if current.state in {CaseState.CLOSED, CaseState.CANCELLED}:
                raise InvalidTransition("closed or cancelled cases cannot be reassigned")
            if current.assignee_id == assignee:
                return current
            changed = replace(current, assignee_id=assignee, revision=current.revision + 1, updated_at=timestamp)
            return self._append(principal, current, changed, command_id=cmd, occurred_at=timestamp,
                                activity_type="case.assigned", reason=explanation,
                                details={"assignee_id": assignee or "unassigned"})
        return self._execute(principal, idempotency_key, "case.assign", payload, action)

    def add_evidence_reference(self, principal: Principal, case_id: str, *,
                               evidence_id: str, digest_sha256: str, classification: str,
                               evidence_kind: str, issuer: str, created_at: str,
                               expected_revision: int, reason: str, idempotency_key: str,
                               command_id: str, occurred_at: str) -> WorkCase:
        _mutator(principal)
        local_id = _case_id(case_id)
        ref_id = _nonempty(evidence_id, "evidence_id", 128)
        digest = _nonempty(digest_sha256, "digest_sha256", 64)
        if not re.fullmatch(r"[0-9a-f]{64}", digest):
            raise ValidationError("digest_sha256 must be 64 lowercase hexadecimal characters")
        if classification not in {"public", "internal", "confidential", "restricted"}:
            raise ValidationError("unsupported classification")
        if evidence_kind not in {"observation", "customer_assertion", "simulation", "interpretation", "unverified"}:
            raise ValidationError("unsupported evidence_kind")
        evidence_issuer = _nonempty(issuer, "issuer", 128)
        evidence_time = _timestamp(created_at)
        explanation = _nonempty(reason, "reason", 1000)
        cmd = _nonempty(command_id, "command_id", 128)
        timestamp = _timestamp(occurred_at)
        reference = EvidenceReference(ref_id, digest, classification, evidence_kind, evidence_issuer, evidence_time)
        payload = {"case_id": local_id, "evidence": reference.to_dict(),
                   "expected_revision": expected_revision, "reason": explanation}

        def action() -> WorkCase:
            current = self._owned(principal, local_id)
            self._check_revision(current, expected_revision)
            previous = next((item for item in current.evidence_refs if item.evidence_id == ref_id), None)
            if previous is not None:
                if previous != reference:
                    raise CaseError("evidence_id already identifies different content")
                return current
            changed = replace(current, evidence_refs=current.evidence_refs + (reference,),
                              revision=current.revision + 1, updated_at=timestamp)
            return self._append(principal, current, changed, command_id=cmd, occurred_at=timestamp,
                                activity_type="case.evidence_linked", reason=explanation,
                                details={"evidence_id": ref_id, "evidence_kind": evidence_kind,
                                         "classification": classification})
        return self._execute(principal, idempotency_key, "case.add_evidence", payload, action)

    def link_external_reference(self, principal: Principal, case_id: str, *,
                                provider: str, external_id: str, expected_revision: int,
                                reason: str, idempotency_key: str, command_id: str,
                                occurred_at: str) -> WorkCase:
        _mutator(principal)
        local_id = _case_id(case_id)
        provider_key = _nonempty(provider, "provider", 64).lower()
        if not re.fullmatch(r"[a-z0-9][a-z0-9.-]{0,63}", provider_key):
            raise ValidationError("provider must be a stable provider key")
        foreign_id = _nonempty(external_id, "external_id", 256)
        explanation = _nonempty(reason, "reason", 1000)
        cmd = _nonempty(command_id, "command_id", 128)
        timestamp = _timestamp(occurred_at)
        payload = {"case_id": local_id, "provider": provider_key, "external_id": foreign_id,
                   "expected_revision": expected_revision, "reason": explanation}

        def action() -> WorkCase:
            current = self._owned(principal, local_id)
            self._check_revision(current, expected_revision)
            map_key = (principal.tenant_id, provider_key, foreign_id)
            already_bound = self._external_map.get(map_key)
            if already_bound is not None:
                if already_bound != local_id:
                    raise MappingConflict("external identifier is already mapped to another local case")
                return current
            ref = ExternalReference(provider_key, foreign_id, timestamp)
            changed = replace(current, external_refs=current.external_refs + (ref,),
                              revision=current.revision + 1, updated_at=timestamp)
            self._external_map[map_key] = local_id
            return self._append(principal, current, changed, command_id=cmd, occurred_at=timestamp,
                                activity_type="case.external_reference_linked", reason=explanation,
                                details={"provider": provider_key, "external_id": foreign_id})
        return self._execute(principal, idempotency_key, "case.link_external_reference", payload, action)

    def find_by_external_reference(self, principal: Principal, provider: str, external_id: str) -> WorkCase:
        _principal(principal)
        provider_key = _nonempty(provider, "provider", 64).lower()
        foreign_id = _nonempty(external_id, "external_id", 256)
        with self._lock:
            local_id = self._external_map.get((principal.tenant_id, provider_key, foreign_id))
            if local_id is None:
                raise NotFound("external reference not found")
            return self._owned(principal, local_id)
