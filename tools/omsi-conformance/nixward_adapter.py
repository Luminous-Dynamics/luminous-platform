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


def _binding_inputs(transaction: dict[str, Any]) -> dict[str, str]:
    """Extract namespaced IDs together from one versioned transaction envelope."""
    if not isinstance(transaction, dict):
        raise validator.ConformanceError("Nixward transaction envelope must be an object")
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
