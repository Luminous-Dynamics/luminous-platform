"""Pure OMSI/Nixward identity-binding prototype.

This module binds an already validated OMSI operation to identities exported
from one versioned Nixward SystemTransaction envelope. It does not verify
signatures, create ChangePlan/ChangeAuthorization objects, invoke the
NixOSExecutor, or attest host state. The caller remains responsible for
obtaining and validating this envelope through the authoritative Nixward API.
"""
from __future__ import annotations

import re
from datetime import datetime
from typing import Any

import omsi_validate as validator

ADAPTER_SCHEMA = "org.omsi.nixward-adapter-binding/0.1"
ADAPTER_VERSION = 1
NIXWARD_TRANSACTION_SCHEMA = "luminous-nixward-system-transaction-v1"
_NIXWARD_HEX = re.compile(r"^[0-9a-f]{64}$")


def _tag_nixward_digest(field: str, transaction: dict[str, Any]) -> str:
    if not isinstance(transaction, dict):
        raise validator.ConformanceError("Nixward transaction envelope must be an object")
    if transaction.get("schema") != NIXWARD_TRANSACTION_SCHEMA:
        raise validator.ConformanceError("unsupported Nixward transaction schema")
    version = transaction.get("version")
    if type(version) is not int or version != 1:
        raise validator.ConformanceError("unsupported Nixward transaction version")
    value = transaction.get(field)
    if not isinstance(value, str) or not _NIXWARD_HEX.fullmatch(value):
        raise validator.ConformanceError(
            f"Nixward transaction {field} must be 64 lowercase hexadecimal characters"
        )
    return f"blake3:{value}"


def _validate_transaction_semantics(transaction: dict[str, Any]) -> None:
    """Check cross-field invariants from the currently inspected Rust lifecycle."""
    phase = transaction["phase"]
    validation = transaction["validation"]
    authorization = transaction["authorization"]
    snapshot = transaction["snapshot"]
    application = transaction["application"]
    verification = transaction["verification"]
    outcome = transaction["outcome"]

    for label, receipt, field in (
        ("authorization evidence", authorization, "evidence_digest"),
        ("validation evidence", validation, "evidence_digest"),
        ("snapshot pre-state", snapshot, "pre_state_digest"),
        ("snapshot recovery binding", snapshot, "recovery_binding_digest"),
        ("application command", application, "command_digest"),
        ("verification post-state", verification, "post_state_digest"),
        ("verification evidence", verification, "evidence_digest"),
    ):
        if receipt is not None and receipt[field] == "0" * 64:
            raise validator.ConformanceError(f"Nixward {label} digest must be non-zero")

    # Nixward applies byte-size limits to UTF-8 strings. JSON Schema maxLength
    # counts Unicode characters, so check byte lengths explicitly as well.
    if authorization is not None:
        issuer_bytes = len(authorization["issuer"].encode("utf-8"))
        if issuer_bytes == 0 or issuer_bytes > 256:
            raise validator.ConformanceError("Nixward authorization issuer has invalid byte length")
        signer_key_id = authorization["signer_key_id"]
        if signer_key_id is not None:
            signer_bytes = len(signer_key_id.encode("utf-8"))
            if signer_bytes == 0 or signer_bytes > 256:
                raise validator.ConformanceError("Nixward signer key ID has invalid byte length")
    if validation is not None and len(validation["summary"].encode("utf-8")) > 4096:
        raise validator.ConformanceError("Nixward validation summary exceeds byte limit")
    if outcome is not None and outcome["reason"] is not None:
        if len(outcome["reason"].encode("utf-8")) > 4096:
            raise validator.ConformanceError("Nixward outcome reason exceeds byte limit")

    def require_present(field: str, value: Any) -> None:
        if value is None:
            raise validator.ConformanceError(
                f"Nixward phase {phase} requires {field} evidence"
            )

    def require_absent(field: str, value: Any) -> None:
        if value is not None:
            raise validator.ConformanceError(
                f"Nixward phase {phase} must not contain {field} evidence"
            )

    if phase == "planned":
        for field, value in (
            ("validation", validation), ("authorization", authorization),
            ("snapshot", snapshot), ("application", application),
            ("verification", verification), ("outcome", outcome),
        ):
            require_absent(field, value)
        return

    require_present("validation", validation)

    if phase == "validated":
        for field, value in (
            ("authorization", authorization), ("snapshot", snapshot),
            ("application", application), ("verification", verification),
            ("outcome", outcome),
        ):
            require_absent(field, value)
        return

    require_present("authorization", authorization)
    if phase == "authorized":
        for field, value in (
            ("snapshot", snapshot), ("application", application),
            ("verification", verification), ("outcome", outcome),
        ):
            require_absent(field, value)
        return

    require_present("snapshot", snapshot)
    if phase == "snapshotted":
        for field, value in (
            ("application", application), ("verification", verification),
            ("outcome", outcome),
        ):
            require_absent(field, value)
        return

    require_present("application", application)
    if application["finished_at_ms"] < application["started_at_ms"]:
        raise validator.ConformanceError("Nixward application timestamp interval is invalid")

    exit_status = application["exit_status"]
    if phase == "applied":
        if exit_status != 0:
            raise validator.ConformanceError("Nixward applied phase requires exit status zero")
        require_absent("verification", verification)
        require_absent("outcome", outcome)
        return

    if phase == "verified":
        if exit_status != 0:
            raise validator.ConformanceError("Nixward verified phase requires exit status zero")
        require_present("verification", verification)
        require_absent("outcome", outcome)
        return

    if phase == "promoted":
        if exit_status != 0:
            raise validator.ConformanceError("Nixward promoted phase requires exit status zero")
        require_present("verification", verification)
        require_present("outcome", outcome)
        if outcome["phase"] != "promoted" or outcome["reason"] is not None:
            raise validator.ConformanceError("Nixward promoted outcome is inconsistent with phase")
        if verification["boot_healthy"] is False:
            raise validator.ConformanceError("Nixward unhealthy boot cannot be promoted")
        return

    if phase == "failed":
        if exit_status == 0:
            raise validator.ConformanceError("Nixward failed phase requires nonzero exit status")
        require_absent("verification", verification)
        require_present("outcome", outcome)
        if outcome["phase"] != "failed" or outcome["reason"] is None:
            raise validator.ConformanceError("Nixward failed outcome is inconsistent with phase")
        return

    if phase == "recovered":
        require_present("outcome", outcome)
        if outcome["phase"] != "recovered" or outcome["reason"] is None:
            raise validator.ConformanceError("Nixward recovered outcome is inconsistent with phase")
        if exit_status == 0:
            require_present("verification", verification)
        else:
            require_absent("verification", verification)
        return

    # The allowed phase set and current schema should make this unreachable.
    raise validator.ConformanceError("unsupported Nixward transaction phase")


def _binding_inputs(transaction: dict[str, Any]) -> dict[str, str]:
    """Extract namespaced IDs together from one versioned transaction envelope."""
    if not isinstance(transaction, dict):
        raise validator.ConformanceError("Nixward transaction envelope must be an object")
    if transaction.get("schema") != NIXWARD_TRANSACTION_SCHEMA:
        raise validator.ConformanceError("unsupported Nixward transaction schema")
    version = transaction.get("version")
    if type(version) is not int or version != 1:
        raise validator.ConformanceError("unsupported Nixward transaction version")
    if transaction.get("phase") not in {
        "planned", "authorized", "validated", "snapshotted", "applied",
        "verified", "promoted", "recovered", "failed",
    }:
        raise validator.ConformanceError("unsupported or missing Nixward transaction phase")
    validator.validate_schema("nixward_transaction", transaction)
    _validate_transaction_semantics(transaction)
    return {
        "nixward_plan_digest": _tag_nixward_digest("plan_digest", transaction),
        "nixward_target_machine_digest": _tag_nixward_digest(
            "target_machine_digest", transaction
        ),
        "nixward_transaction_id": _tag_nixward_digest("transaction_id", transaction),
    }

def build_adapter_binding(
    operation: dict[str, Any],
    *,
    nixward_transaction: dict[str, Any],
    now: datetime | None = None,
) -> dict[str, Any]:
    """Build a cross-reference from an operation and one Nixward transaction.

    The Nixward envelope fields plan_digest, target_machine_digest and
    transaction_id are expected to come from the same SystemTransaction.
    Nixward's raw lowercase hex values are tagged blake3:; the OMSI manifest
    digest retains its sha256: namespace. This does not prove provenance of the
    envelope or verify its contents using Nixward's Rust implementation.
    """
    validator.validate_operation(operation, now=now)
    ids = _binding_inputs(nixward_transaction)
    binding = {
        "schema": ADAPTER_SCHEMA,
        "version": ADAPTER_VERSION,
        "operation_id": operation["id"],
        "tenant_id": operation["tenant_id"],
        "resource_id": operation["target"]["resource_id"],
        "correlation_id": operation["correlation_id"],
        "omsi_manifest_digest": validator.digest(
            operation, validator.OPERATION_MANIFEST_DOMAIN
        ),
        **ids,
    }
    validator.validate_schema("nixward_adapter_binding", binding)
    return binding


def validate_adapter_binding(
    binding: dict[str, Any],
    operation: dict[str, Any],
    *,
    nixward_transaction: dict[str, Any],
    now: datetime | None = None,
) -> None:
    """Fail closed unless every operation and transaction identity still matches.

    The transaction argument must be the actual versioned Nixward envelope
    being qualified, not a reconstructed dict copied from the adapter binding.
    This Python prototype checks structure and cross-binding only; it cannot
    authenticate the envelope or guarantee that a caller used the Rust API.
    """
    validator.validate_schema("nixward_adapter_binding", binding)
    expected = build_adapter_binding(
        operation, nixward_transaction=nixward_transaction, now=now
    )
    for key in expected:
        if binding.get(key) != expected[key]:
            raise validator.ConformanceError(
                f"adapter binding {key} does not match authoritative operation/transaction identity"
            )
