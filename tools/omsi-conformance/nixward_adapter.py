"""Pure OMSI/Nixward identity-binding prototype.

This module only binds an already validated OMSI manifest to externally
obtained Nixward digest values. It does not verify signatures, create
ChangePlan/ChangeAuthorization objects, invoke NixOSExecutor, or attest host
state. Callers must obtain the three Nixward identities from the authoritative
Nixward transaction/plan objects, not from caller-supplied claims alone.
"""
from __future__ import annotations

import re
from datetime import datetime
from typing import Any

import omsi_validate as validator

ADAPTER_SCHEMA = "org.omsi.nixward-adapter-binding/0.1"
ADAPTER_VERSION = 1
_NIXWARD_HEX = re.compile(r"^[0-9a-f]{64}$")


def _tag_nixward_digest(field: str, raw_hex: str) -> str:
    if not isinstance(raw_hex, str) or not _NIXWARD_HEX.fullmatch(raw_hex):
        raise validator.ConformanceError(
            f"{field} must be 64 lowercase hexadecimal characters from Nixward"
        )
    return f"blake3:{raw_hex}"


def build_adapter_binding(
    operation: dict[str, Any],
    *,
    nixward_plan_digest: str,
    nixward_target_machine_digest: str,
    nixward_transaction_id: str,
    now: datetime | None = None,
) -> dict[str, Any]:
    """Build an explicit binding record from a validated OMSI operation.

    Nixward API digest values are passed as 64-character lowercase hex; the
    binding adds the blake3: namespace tag. The OMSI manifest digest keeps its
    sha256: tag. Digest strings are intentionally never interchangeable.
    """
    validator.validate_operation(operation, now=now)
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
        "nixward_plan_digest": _tag_nixward_digest(
            "nixward_plan_digest", nixward_plan_digest
        ),
        "nixward_target_machine_digest": _tag_nixward_digest(
            "nixward_target_machine_digest", nixward_target_machine_digest
        ),
        "nixward_transaction_id": _tag_nixward_digest(
            "nixward_transaction_id", nixward_transaction_id
        ),
    }
    validator.validate_schema("nixward_adapter_binding", binding)
    return binding


def validate_adapter_binding(
    binding: dict[str, Any],
    operation: dict[str, Any],
    *,
    nixward_plan_digest: str,
    nixward_target_machine_digest: str,
    nixward_transaction_id: str,
    now: datetime | None = None,
) -> None:
    """Fail closed unless every OMSI and Nixward identity still matches.

    Nixward identity values here must be read from the authoritative plan,
    machine binding and transaction that the caller is about to use. Passing
    the binding's own stored values back into this function would not
    independently verify the Nixward side of the relationship.
    """
    validator.validate_schema("nixward_adapter_binding", binding)
    expected = build_adapter_binding(
        operation,
        nixward_plan_digest=nixward_plan_digest,
        nixward_target_machine_digest=nixward_target_machine_digest,
        nixward_transaction_id=nixward_transaction_id,
        now=now,
    )
    for key in expected:
        if binding.get(key) != expected[key]:
            raise validator.ConformanceError(
                f"adapter binding {key} does not match authoritative operation/plan identity"
            )
