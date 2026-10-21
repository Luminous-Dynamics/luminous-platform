#!/usr/bin/env python3
"""Check endpoint approval-record shape and exact binding to one frozen profile.

This tool does not authenticate a signer, verify a signature, or authorize a connection.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from datetime import datetime
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker

from validate_hardware_portfolio import load_json
from validate_vendor_management_endpoint_profile import profile_errors

_READ_ONLY_OPERATIONS = {
    "inventory_read",
    "health_read",
    "version_read",
    "entitlement_read",
    "lifecycle_read",
}
MAX_SAFE_JSON_INTEGER = 9_007_199_254_740_991
MAX_JSON_NESTING = 128


def _ensure_safe_json_integer_domain(value: Any) -> None:
    """Enforce numeric and nesting limits without recursive traversal."""
    pending: list[tuple[Any, int]] = [(value, 0)]
    while pending:
        current, depth = pending.pop()
        if type(current) is int:
            if abs(current) > MAX_SAFE_JSON_INTEGER:
                raise ValueError("integer outside safe interoperable JSON integer range")
        elif isinstance(current, dict):
            if depth >= MAX_JSON_NESTING:
                raise ValueError("JSON nesting exceeds configured limit")
            pending.extend((nested, depth + 1) for nested in current.values())
        elif isinstance(current, list):
            if depth >= MAX_JSON_NESTING:
                raise ValueError("JSON nesting exceeds configured limit")
            pending.extend((nested, depth + 1) for nested in current)


def parse_time(value: str, label: str, errors: list[str]) -> datetime | None:
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except (TypeError, ValueError):
        errors.append(f"{label} must be an ISO-8601 date-time")
        return None
    if parsed.tzinfo is None:
        errors.append(f"{label} must include an explicit timezone offset")
        return None
    return parsed


def canonical_profile_digest(profile: dict[str, Any]) -> str:
    """Compute the profile digest for this schema's string/integer-only value domain.

    The endpoint profile schema excludes arbitrary properties and floating-point values;
    its keys are fixed ASCII schema keys and its values are strings, integers, booleans or null.
    Keep this routine version-pinned and covered by fixtures before using it as part of a
    cryptographic verifier.
    """
    _ensure_safe_json_integer_domain(profile)
    try:
        canonical = json.dumps(
            profile,
            sort_keys=True,
            separators=(",", ":"),
            ensure_ascii=False,
            allow_nan=False,
        ).encode("utf-8", errors="strict")
    except RecursionError:
        raise ValueError("JSON nesting exceeds canonicalization limits") from None
    return hashlib.sha256(canonical).hexdigest()


def _safe_schema_error_code(error: Any) -> str:
    """Describe a schema failure without echoing untrusted values into logs."""
    rule = str(getattr(error, "validator", "schema")).upper()
    return f"SCHEMA_{rule}_VIOLATION"


def approval_errors(
    approval: dict[str, Any],
    approval_schema: dict[str, Any],
    profile: dict[str, Any],
    profile_schema: dict[str, Any],
) -> list[str]:
    """Check record schema and exact target/profile bindings, not cryptographic authority."""
    errors: list[str] = []
    Draft202012Validator.check_schema(approval_schema)
    validator = Draft202012Validator(approval_schema, format_checker=FormatChecker())
    for error in sorted(validator.iter_errors(approval), key=lambda item: list(map(str, item.path))):
        where = "$" + "".join(
            f"[{part}]" if isinstance(part, int) else f".{part}"
            for part in error.absolute_path
        )
        errors.append(f"{where}: {_safe_schema_error_code(error)}")
    if errors:
        return errors

    profile_violations = profile_errors(profile, profile_schema)
    if profile_violations:
        errors.extend(f"$.profile{item[1:] if item.startswith('$') else '.' + item}" for item in profile_violations)
        return errors

    try:
        digest = canonical_profile_digest(profile)
    except (UnicodeEncodeError, ValueError, TypeError) as exc:
        return [f"$.profile cannot be canonicalized for digest verification: {type(exc).__name__}"]

    if approval["profile_digest"] != digest:
        errors.append("$.profile_digest must match the canonical SHA-256 digest of the supplied frozen profile")
    if approval["profile_id"] != profile["profile_id"]:
        errors.append("$.profile_id must match the supplied profile")
    if approval["profile_revision"] != profile["profile_revision"]:
        errors.append("$.profile_revision must match the supplied profile revision")

    adapter = approval["adapter"]
    profile_adapter = profile["adapter"]
    for key in ("adapter_id", "adapter_revision", "vendor", "product_family"):
        if adapter[key] != profile_adapter[key]:
            errors.append(f"$.adapter.{key} must exactly match the supplied endpoint profile")

    target = approval["target"]
    scope = profile["scope"]
    for key in ("model", "hardware_revision", "software_version"):
        if target[key] != scope[key]:
            errors.append(f"$.target.{key} must exactly match profile.scope.{key}")

    if approval["expected_principal_ref"] != scope["expected_principal_ref"]:
        errors.append("$.expected_principal_ref must match profile.scope.expected_principal_ref")
    if approval["expected_tenant_or_org_ref"] != scope["expected_tenant_or_org_ref"]:
        errors.append("$.expected_tenant_or_org_ref must match profile.scope.expected_tenant_or_org_ref")

    created = parse_time(approval["created_at"], "$.created_at", errors)
    not_before = parse_time(approval["not_before"], "$.not_before", errors)
    expires = parse_time(approval["expires_at"], "$.expires_at", errors)
    if not_before and expires and expires <= not_before:
        errors.append("$.expires_at must be later than $.not_before")
    if created and expires and expires <= created:
        errors.append("$.expires_at must be later than $.created_at")

    if approval["state"] in {"approved", "rejected", "revoked"}:
        if approval["requested_by_ref"] == approval["approver_ref"]:
            errors.append("$.requested_by_ref and $.approver_ref must be distinct principals")

    if approval["state"] == "approved":
        if not approval["allowed_operations"]:
            errors.append("$.allowed_operations must name at least one explicit read-only operation")
        unexpected = set(approval["allowed_operations"]) - _READ_ONLY_OPERATIONS
        if unexpected:
            errors.append("$.allowed_operations contains an operation outside the read-only allowlist")
        for dimension_name in (
            "deployment_jurisdiction",
            "hardware_or_commercial_region",
            "support_jurisdiction",
        ):
            dimension = scope[dimension_name]
            if dimension["state"] == "unresolved":
                errors.append(f"$.profile.scope.{dimension_name} remains unresolved for an approved decision")

    return errors


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--approval",
        type=Path,
        default=Path("tests/fixtures/vendor-management-endpoint-approval-v1.json"),
    )
    parser.add_argument(
        "--approval-schema",
        type=Path,
        default=Path("schemas/vendor-management-endpoint-approval-v1.schema.json"),
    )
    parser.add_argument(
        "--profile",
        type=Path,
        default=Path("tests/fixtures/vendor-management-endpoint-profile-v1.json"),
    )
    parser.add_argument(
        "--profile-schema",
        type=Path,
        default=Path("schemas/vendor-management-endpoint-profile-v1.schema.json"),
    )
    args = parser.parse_args(argv)

    try:
        approval = load_json(args.approval)
        approval_schema = load_json(args.approval_schema)
        profile = load_json(args.profile)
        profile_schema = load_json(args.profile_schema)
        errors = approval_errors(approval, approval_schema, profile, profile_schema)
    except (OSError, ValueError) as exc:
        print(f"FAIL: unable to load approval/profile/schema: {type(exc).__name__}", file=sys.stderr)
        return 2

    if errors:
        print("FAIL: endpoint approval record/profile binding contract")
        for error in errors:
            print(f"- {error}")
        return 1

    print(
        "VALID RECORD STRUCTURE AND PROFILE BINDING ONLY; "
        "signature authentication, signer trust, revocation, freshness at use, "
        "and actual authorization remain external"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
