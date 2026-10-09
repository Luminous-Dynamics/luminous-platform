#!/usr/bin/env python3
# Copyright (C) 2026 Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Fail-closed semantic checks for code-list snapshot manifests.

This is reference/CI validation only. It never downloads a list, authenticates
the authority, verifies the retained bytes against a trusted signature, or
proves that a code is valid for a particular transaction.
"""

from __future__ import annotations

import argparse
import json
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_SCHEMA_PATH = ROOT / "schemas" / "code-list-snapshot-manifest-v1.schema.json"
REQUIRED_ACTIVE_CHECKS = {
    "source_digest_checked",
    "schema_validated",
    "duplicate_codes_checked",
    "effective_dates_checked",
}


class DuplicateJsonKey(ValueError):
    """Raised when JSON repeats an object key."""


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise DuplicateJsonKey(f"duplicate JSON object key: {key}")
        result[key] = value
    return result


def _reject_nonfinite_constant(value: str) -> None:
    raise ValueError(f"non-standard JSON constant is not allowed: {value}")


def load_json(path: Path) -> Any:
    return json.loads(
        path.read_text(encoding="utf-8"),
        object_pairs_hook=_unique_object,
        parse_constant=_reject_nonfinite_constant,
    )


def _timestamp(value: Any) -> datetime | None:
    if not isinstance(value, str):
        return None
    try:
        parsed = datetime.fromisoformat(value[:-1] + "+00:00" if value.endswith("Z") else value)
    except ValueError:
        return None
    if parsed.tzinfo is None or parsed.utcoffset() is None:
        return None
    return parsed.astimezone(timezone.utc)


def validate_manifest(
    document: Any,
    *,
    schema: dict[str, Any],
    now: datetime | None = None,
) -> dict[str, Any]:
    supplied_now = now if now is not None else datetime.now(timezone.utc)
    if supplied_now.tzinfo is None or supplied_now.utcoffset() is None:
        raise ValueError("now must be timezone-aware")
    current = supplied_now.astimezone(timezone.utc)

    structural_errors = sorted(
        Draft202012Validator(schema, format_checker=FormatChecker()).iter_errors(document),
        key=lambda error: (list(map(str, error.absolute_path)), error.message),
    )
    if structural_errors:
        errors = [
            f"{'/'.join(str(part) for part in error.absolute_path) or '$'}: {error.message}"
            for error in structural_errors
        ]
        return {
            "schema_id": schema.get("$id"),
            "result": "INVALID_DECLARATION",
            "structural_valid": False,
            "semantic_valid": False,
            "activation_eligible": False,
            "errors": errors,
            "warnings": [],
            "notice": "Schema failure; semantic evaluation was not performed.",
        }

    errors: list[str] = []
    warnings: list[str] = []
    metadata = document["metadata"]
    source = document["source"]
    snapshot = document["snapshot"]
    validation = document["validation"]
    code_list = document["code_list"]
    status = metadata["status"]

    timestamp_fields = {
        "metadata.created_at": metadata["created_at"],
        "metadata.review_due_at": metadata["review_due_at"],
        "validation.reviewed_at": validation["reviewed_at"],
    }
    for optional_path, container, field in (
        ("source.retrieved_at", source, "retrieved_at"),
        ("source.published_at", source, "published_at"),
        ("snapshot.effective_from", snapshot, "effective_from"),
        ("snapshot.effective_until", snapshot, "effective_until"),
    ):
        if field in container:
            timestamp_fields[optional_path] = container[field]

    parsed: dict[str, datetime] = {}
    for path, value in timestamp_fields.items():
        instant = _timestamp(value)
        if instant is None:
            errors.append(f"{path}: timestamp must parse and include an explicit timezone")
        else:
            parsed[path] = instant

    created = parsed.get("metadata.created_at")
    review_due = parsed.get("metadata.review_due_at")
    reviewed = parsed.get("validation.reviewed_at")
    retrieved = parsed.get("source.retrieved_at")
    effective_from = parsed.get("snapshot.effective_from")
    effective_until = parsed.get("snapshot.effective_until")

    if created and created > current:
        errors.append("metadata.created_at: must not be in the future")
    if reviewed and reviewed > current:
        errors.append("validation.reviewed_at: review time must not be in the future")
    if retrieved and retrieved > current:
        errors.append("source.retrieved_at: retrieval time must not be in the future")
    if created and review_due and review_due <= created:
        errors.append("metadata.review_due_at: must be after created_at")
    if effective_from and effective_until and effective_until <= effective_from:
        errors.append("snapshot.effective_until: must be after effective_from")
    if status == "active":
        if code_list["version_label"].strip().lower() in {"", "not-retrieved", "unknown", "unversioned"}:
            errors.append("code_list.version_label: active registries require a publisher version or an explicitly assigned immutable snapshot label")
        if review_due and current >= review_due:
            errors.append("metadata.review_due_at: active manifest is stale because its review deadline has passed")
        if effective_from and current < effective_from:
            errors.append("snapshot.effective_from: registry is not yet effective")
        if effective_until and current >= effective_until:
            errors.append("snapshot.effective_until: registry is no longer effective")
        if retrieved is None:
            errors.append("source.retrieved_at: active registry requires a recorded retrieval time")
        if reviewed is None:
            errors.append("validation.reviewed_at: active registry requires a recorded review time")
        if source["retrieval_status"] != "verified_against_authority":
            errors.append("source.retrieval_status: active registry requires source-authority verification")
        missing_checks = sorted(REQUIRED_ACTIVE_CHECKS.difference(validation["checks"]))
        if missing_checks:
            errors.append(f"validation.checks: active registry is missing required checks: {', '.join(missing_checks)}")
        if snapshot["duplicate_code_count"] != 0:
            errors.append("snapshot.duplicate_code_count: active registry cannot contain duplicate codes")
        if snapshot["record_count"] <= 0:
            errors.append("snapshot.record_count: active registry must contain at least one code")
        if code_list["record_effectivity"] == "not_provided":
            warnings.append("code_list.record_effectivity: active registry has no declared effectivity model")

    if status in {"stale", "deprecated"}:
        warnings.append(f"metadata.status={status}: this snapshot must not be used for new transaction decisions")
    elif status != "active":
        warnings.append("registry is not active; syntax validation must not be mistaken for code-list membership validation")

    active = status == "active" and not errors and not metadata["fixture_only"]
    return {
        "schema_id": schema.get("$id"),
        "manifest_id": metadata["manifest_id"],
        "revision": metadata["revision"],
        "code_list_id": code_list["code_list_id"],
        "status": status,
        "result": "ACTIVE_DECLARATION_ELIGIBLE_FOR_RUNTIME_VERIFICATION" if active else (
            "INVALID_DECLARATION" if errors else "DECLARATION_VALID_NOT_ACTIVE"
        ),
        "structural_valid": True,
        "semantic_valid": not errors,
        "activation_eligible": active,
        "errors": errors,
        "warnings": warnings,
        "notice": "Eligibility means the declared metadata passed these checks only. A runtime must still verify retained payload digests, source authenticity, code uniqueness/effectivity from actual records, review signatures, and the current clock before using membership results.",
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("manifest", type=Path, help="code-list snapshot manifest JSON")
    parser.add_argument("--schema", type=Path, default=DEFAULT_SCHEMA_PATH, help="manifest JSON Schema")
    parser.add_argument("--as-of", help="timezone-aware ISO-8601 evaluation time")
    args = parser.parse_args(argv)

    try:
        schema = load_json(args.schema)
        Draft202012Validator.check_schema(schema)
        document = load_json(args.manifest)
        if args.as_of:
            now = _timestamp(args.as_of)
            if now is None:
                parser.error("--as-of must be a timezone-aware ISO-8601 timestamp")
        else:
            now = datetime.now(timezone.utc)
        report = validate_manifest(document, schema=schema, now=now)
    except (OSError, UnicodeError, json.JSONDecodeError, DuplicateJsonKey, ValueError) as exc:
        report = {
            "result": "INVALID_DECLARATION",
            "structural_valid": False,
            "semantic_valid": False,
            "activation_eligible": False,
            "errors": [str(exc)],
            "warnings": [],
            "notice": "Input or schema could not be safely parsed/validated.",
        }
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0 if report["semantic_valid"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
