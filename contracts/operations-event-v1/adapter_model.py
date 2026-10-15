"""Deterministic reference model for a tenant-bound PSA event adapter.

This is a synthetic test harness, not a live ConnectWise client and not a
durable inbox implementation. Production consumers need authenticated provider
parsing, durable transactional idempotency, and an atomic database/outbox seam.
"""

from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime
import hashlib
import json
from pathlib import Path
from typing import Mapping

from jsonschema import Draft202012Validator, FormatChecker

ROOT = Path(__file__).resolve().parent
ENVELOPE_SCHEMA = json.loads((ROOT / "schema.json").read_text(encoding="utf-8"))
INCIDENT_SCHEMA = json.loads(
    (ROOT / "payloads" / "incident-snapshot-v1.schema.json").read_text(
        encoding="utf-8"
    )
)
ENVELOPE_VALIDATOR = Draft202012Validator(
    ENVELOPE_SCHEMA, format_checker=FormatChecker()
)
INCIDENT_VALIDATOR = Draft202012Validator(
    INCIDENT_SCHEMA, format_checker=FormatChecker()
)
INCIDENT_SCHEMA_URI = INCIDENT_SCHEMA["$id"]
VALID_STATUSES = {
    "new",
    "open",
    "in_progress",
    "waiting_on_customer",
    "resolved",
    "closed",
    "cancelled",
}


class NormalizationError(ValueError):
    """The connector could not safely normalize or map the source observation."""


class TenantBindingError(PermissionError):
    """The event does not match the trusted connection and tenant boundary."""


class IdempotencyConflict(ValueError):
    """A previously used identity/key was reused with different semantics."""


@dataclass(frozen=True)
class PsaIncidentSnapshot:
    """Vendor-neutral record produced only after adapter-specific parsing."""

    company_id: str
    ticket_id: str
    revision: str
    status: str
    redacted_summary: str
    occurred_at: str | None = None
    priority: int | None = None


@dataclass(frozen=True)
class TrustedPsaConnection:
    """Configuration resolved outside the untrusted ticket/webhook payload."""

    connection_id: str
    source_uri: str
    company_tenant_map: Mapping[str, str]
    incident_id_map: Mapping[tuple[str, str], str]


@dataclass(frozen=True)
class IngestionBoundary:
    """Authenticated connector identity plus its trusted resource mappings."""

    connection_id: str
    source_uri: str
    company_tenant_map: Mapping[str, str]
    incident_id_map: Mapping[tuple[str, str], str]
    producer_system: str = "connectwise-psa"


def _canonical_json(value: object) -> bytes:
    """Deterministic local digest encoding; not a signed canonical JSON standard."""
    return json.dumps(
        value,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
        allow_nan=False,
    ).encode("utf-8")


def _sha256(domain: str, value: object) -> str:
    return hashlib.sha256(
        domain.encode("ascii") + b"\x00" + _canonical_json(value)
    ).hexdigest()


def _validate_timestamp(value: str, label: str) -> None:
    if not isinstance(value, str) or not value:
        raise NormalizationError(f"{label} must be an RFC 3339 timestamp")
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError as exc:
        raise NormalizationError(f"{label} must be an RFC 3339 timestamp") from exc
    if parsed.tzinfo is None or parsed.utcoffset() is None:
        raise NormalizationError(f"{label} must include a timezone offset")


def normalize_incident_snapshot(
    snapshot: PsaIncidentSnapshot,
    connection: TrustedPsaConnection,
    *,
    observed_at: str,
) -> dict[str, object]:
    """Normalize one parsed provider snapshot into a CloudEvents 1.0 snapshot event.

    Tenant/source/actor values come from trusted connection configuration.
    Provider IDs must resolve through explicit company and incident mappings;
    we never infer an internal tenant or incident ID from arbitrary ticket text.
    """
    _validate_timestamp(observed_at, "observed_at")
    if snapshot.occurred_at is not None:
        _validate_timestamp(snapshot.occurred_at, "occurred_at")

    # Python annotations do not validate decoded/runtime values. Reject malformed
    # adapter records explicitly before mapping lookups or string operations.
    for label, value in (
        ("company_id", snapshot.company_id),
        ("ticket_id", snapshot.ticket_id),
        ("revision", snapshot.revision),
        ("status", snapshot.status),
    ):
        if not isinstance(value, str) or not value:
            raise NormalizationError(f"{label} must be a non-empty string")
    if snapshot.status not in VALID_STATUSES:
        raise NormalizationError("status has no approved normalized mapping")
    if not isinstance(snapshot.redacted_summary, str):
        raise NormalizationError("redacted summary must be a string")

    if not connection.connection_id or not connection.source_uri:
        raise NormalizationError("trusted connection identity is incomplete")
    if snapshot.company_id not in connection.company_tenant_map:
        raise NormalizationError("external company has no explicit tenant mapping")
    tenant_id = connection.company_tenant_map[snapshot.company_id]
    if not tenant_id:
        raise NormalizationError("mapped tenant ID is empty")

    local_incident_id = connection.incident_id_map.get(
        (snapshot.company_id, snapshot.ticket_id)
    )
    if not local_incident_id:
        raise NormalizationError("ticket has no explicit local incident mapping")

    summary = snapshot.redacted_summary.strip()
    if not summary or len(summary) > 300:
        raise NormalizationError("redacted summary must contain 1..300 characters")
    if snapshot.priority is not None and (
        isinstance(snapshot.priority, bool)
        or not isinstance(snapshot.priority, int)
        or not 1 <= snapshot.priority <= 5
    ):
        raise NormalizationError("priority must be an integer in the normalized range 1..5")

    identity_basis = {
        "source": connection.source_uri,
        "company_id": snapshot.company_id,
        "ticket_id": snapshot.ticket_id,
        "revision": snapshot.revision,
    }
    event_id = _sha256("luminous.cloudevent.id.v1", identity_basis)
    idempotency_basis = {
        "connection_id": connection.connection_id,
        "tenant_id": tenant_id,
        **identity_basis,
    }
    idempotency_key = _sha256("luminous.operations.idempotency.v1", idempotency_basis)
    correlation_id = _sha256(
        "luminous.operations.correlation.v1",
        {
            "source": connection.source_uri,
            "company_id": snapshot.company_id,
            "ticket_id": snapshot.ticket_id,
        },
    )

    data: dict[str, object] = {
        "source_reference": {
            "system": "connectwise-psa",
            "company_id": snapshot.company_id,
            "resource_type": "service_ticket",
            "resource_id": snapshot.ticket_id,
            "revision": snapshot.revision,
        },
        "status": snapshot.status,
        "summary": summary,
        "evidence_refs": [],
    }
    if snapshot.priority is not None:
        data["priority"] = snapshot.priority

    event: dict[str, object] = {
        "specversion": "1.0",
        "id": event_id,
        "source": connection.source_uri,
        "type": "io.luminousdynamics.ops.incident.snapshot.v1",
        "subject": f"incident/{local_incident_id}",
        "datacontenttype": "application/json",
        "dataschema": INCIDENT_SCHEMA_URI,
        "tenantid": tenant_id,
        "idempotencykey": idempotency_key,
        "observedat": observed_at,
        "correlationid": correlation_id,
        "producersystem": "connectwise-psa",
        "producercomponent": "normalization-reference-model",
        "producerversion": "0.0.1-test",
        "actorkind": "external_system",
        "actorid": connection.connection_id,
        "data": data,
    }
    if snapshot.occurred_at is not None:
        event["time"] = snapshot.occurred_at

    errors = list(ENVELOPE_VALIDATOR.iter_errors(event))
    if errors:
        raise NormalizationError(
            "normalized envelope failed schema validation: "
            + "; ".join(error.message for error in errors)
        )
    errors = list(INCIDENT_VALIDATOR.iter_errors(data))
    if errors:
        raise NormalizationError(
            "normalized payload failed schema validation: "
            + "; ".join(error.message for error in errors)
        )
    return event


def _business_effect_fingerprint(event: Mapping[str, object]) -> str:
    """Hash stable intended business effect, excluding delivery metadata."""
    data = event.get("data")
    semantic = {
        "tenantid": event.get("tenantid"),
        "source": event.get("source"),
        "type": event.get("type"),
        "subject": event.get("subject"),
        "dataschema": event.get("dataschema"),
        "producersystem": event.get("producersystem"),
        "actorkind": event.get("actorkind"),
        "actorid": event.get("actorid"),
        "authorityref": event.get("authorityref"),
        "data": data,
    }
    return _sha256("luminous.operations.business-effect.v1", semantic)


class MemoryInbox:
    """Small executable state-machine model; deliberately not durable storage.

    Tests the difference between exact event replay and reuse of a business
    idempotency key with changed semantics. Production code must implement
    equivalent uniqueness/atomicity in a durable database transaction.
    """

    def __init__(self) -> None:
        self._event_digests: dict[tuple[str, str], str] = {}
        self._effect_digests: dict[tuple[str, str, str], str] = {}

    def accept(
        self,
        event: Mapping[str, object],
        boundary: IngestionBoundary,
    ) -> str:
        errors = list(ENVELOPE_VALIDATOR.iter_errors(event))
        if errors:
            raise NormalizationError("event envelope failed schema validation")
        errors = list(INCIDENT_VALIDATOR.iter_errors(event.get("data")))
        if errors:
            raise NormalizationError("incident payload failed schema validation")

        if event.get("source") != boundary.source_uri:
            raise TenantBindingError("event source does not match authenticated connection")
        if event.get("actorid") != boundary.connection_id:
            raise TenantBindingError("event actor does not match authenticated connection")
        if event.get("producersystem") != boundary.producer_system:
            raise TenantBindingError("event producer does not match configured adapter")

        data = event.get("data")
        if not isinstance(data, Mapping):
            raise NormalizationError("incident payload is not an object")
        source_ref = data.get("source_reference")
        if not isinstance(source_ref, Mapping):
            raise NormalizationError("source reference is not an object")
        if source_ref.get("system") != boundary.producer_system:
            raise TenantBindingError("source reference system does not match authenticated adapter")
        if source_ref.get("resource_type") != "service_ticket":
            raise TenantBindingError("source resource type is not a supported service ticket")
        company_id = str(source_ref.get("company_id", ""))
        ticket_id = str(source_ref.get("resource_id", ""))
        expected_tenant = boundary.company_tenant_map.get(company_id)
        if not expected_tenant:
            raise TenantBindingError("source company is not mapped by this connector")
        if event.get("tenantid") != expected_tenant:
            raise TenantBindingError("event tenant conflicts with trusted company-to-tenant mapping")
        expected_incident = boundary.incident_id_map.get((company_id, ticket_id))
        if not expected_incident:
            raise TenantBindingError("source ticket is not mapped to a local incident")
        if event.get("subject") != f"incident/{expected_incident}":
            raise TenantBindingError("event subject conflicts with trusted ticket-to-incident mapping")

        identity = (str(event["source"]), str(event["id"]))
        # observedat is the first trusted ingestion time, not provider event
        # identity. A redelivery may be observed later; compare semantic event
        # bytes without that receiver-local timestamp so replay is a duplicate.
        # Durable consumers should retain the first observedat from their inbox.
        stable_event = {
            key: value for key, value in event.items() if key != "observedat"
        }
        event_digest = hashlib.sha256(_canonical_json(stable_event)).hexdigest()
        old_event_digest = self._event_digests.get(identity)
        if old_event_digest is not None:
            if old_event_digest != event_digest:
                raise IdempotencyConflict(
                    "CloudEvents source+id identity was reused with different content"
                )
            return "DUPLICATE"

        effect_identity = (
            boundary.connection_id,
            expected_tenant,
            str(event["idempotencykey"]),
        )
        effect_digest = _business_effect_fingerprint(event)
        old_effect_digest = self._effect_digests.get(effect_identity)
        if old_effect_digest is not None:
            if old_effect_digest != effect_digest:
                raise IdempotencyConflict(
                    "business idempotency key was reused with different semantics"
                )
            # Record the alternate delivery identity so it is stable on replay.
            self._event_digests[identity] = event_digest
            return "DUPLICATE"

        self._event_digests[identity] = event_digest
        self._effect_digests[effect_identity] = effect_digest
        return "ACCEPTED"
