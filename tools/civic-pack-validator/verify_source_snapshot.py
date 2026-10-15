#!/usr/bin/env python3
"""Verify retained snapshot bytes against a source-capture manifest, without network access."""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker, SchemaError

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_SCHEMA = ROOT / "schemas" / "source-capture-manifest-v0.1.0.schema.json"


def sha256_hex(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def _check(check_id: str, passed: bool, details: str) -> dict[str, str]:
    return {
        "check_id": check_id,
        "status": "PASS" if passed else "FAIL",
        "details": details,
    }


def verify_source_snapshot(
    manifest_bytes: bytes,
    snapshot_bytes: bytes,
    schema_bytes: bytes,
) -> dict[str, Any]:
    """Validate manifest structure and match retained bytes to its declared digest and size."""
    checks: list[dict[str, str]] = []
    manifest_value: Any = None
    schema_value: Any = None
    manifest_parse_error: str | None = None
    schema_parse_error: str | None = None
    try:
        manifest_value = json.loads(manifest_bytes)
        checks.append(_check(
            "manifest.json",
            True,
            "Manifest bytes decode as JSON.",
        ))
    except (json.JSONDecodeError, UnicodeDecodeError) as exc:
        manifest_parse_error = f"Manifest is not valid UTF-8 JSON: {exc}"
        checks.append(_check("manifest.json", False, manifest_parse_error))

    if not isinstance(manifest_value, dict):
        if manifest_parse_error is None:
            manifest_parse_error = "Manifest JSON root must be an object."
            checks.append(_check("manifest.root_type", False, manifest_parse_error))
        manifest_object: dict[str, Any] | None = None
    else:
        manifest_object = manifest_value
        checks.append(_check("manifest.root_type", True, "Manifest root is an object."))

    try:
        schema_value = json.loads(schema_bytes)
        if not isinstance(schema_value, dict):
            raise ValueError("schema JSON root must be an object")
        Draft202012Validator.check_schema(schema_value)
        checks.append(_check(
            "schema.meta_schema",
            True,
            "Manifest schema is valid JSON Schema Draft 2020-12.",
        ))
    except (json.JSONDecodeError, UnicodeDecodeError, SchemaError, ValueError) as exc:
        schema_parse_error = f"Schema is invalid: {exc}"
        checks.append(_check("schema.meta_schema", False, schema_parse_error))

    if manifest_object is not None and schema_value is not None and schema_parse_error is None:
        validator = Draft202012Validator(schema_value, format_checker=FormatChecker())
        errors = sorted(
            validator.iter_errors(manifest_object),
            key=lambda error: (
                list(map(str, error.absolute_path)),
                error.message,
            ),
        )
        if errors:
            details = " | ".join(
                "$" + "".join(
                    f"[{part}]" if isinstance(part, int) else f".{part}"
                    for part in error.absolute_path
                ) + f": {error.message}"
                for error in errors[:25]
            )
            checks.append(_check("manifest.schema_validation", False, details))
        else:
            checks.append(_check(
                "manifest.schema_validation",
                True,
                "Manifest conforms to the capture-manifest schema.",
            ))
    else:
        checks.append(_check(
            "manifest.schema_validation",
            False,
            "Cannot validate manifest because the manifest or schema could not be parsed.",
        ))

    snapshot_sha256 = sha256_hex(snapshot_bytes)
    snapshot_size = len(snapshot_bytes)
    snapshot_decl = manifest_object.get("snapshot") if manifest_object is not None else None
    expected_sha = snapshot_decl.get("sha256") if isinstance(snapshot_decl, dict) else None
    expected_size = snapshot_decl.get("size_bytes") if isinstance(snapshot_decl, dict) else None
    digest_matches = isinstance(expected_sha, str) and expected_sha == snapshot_sha256
    size_matches = type(expected_size) is int and expected_size == snapshot_size

    checks.append(_check(
        "snapshot.sha256",
        digest_matches,
        (
            "Retained snapshot SHA-256 matches the manifest."
            if digest_matches
            else "Retained snapshot SHA-256 does not match the manifest or the manifest has no usable digest."
        ),
    ))
    checks.append(_check(
        "snapshot.size_bytes",
        size_matches,
        (
            "Retained snapshot byte length matches the manifest."
            if size_matches
            else "Retained snapshot byte length does not match the manifest or the manifest has no usable size."
        ),
    ))

    metadata = manifest_object or {}
    integrity_passed = all(item["status"] == "PASS" for item in checks)
    return {
        "verifier_version": "0.1.0",
        "integrity_passed": integrity_passed,
        "source_id": metadata.get("source_id"),
        "canonical_url": metadata.get("canonical_url"),
        "final_url": metadata.get("final_url"),
        "captured_at": metadata.get("captured_at"),
        "manifest_sha256": sha256_hex(manifest_bytes),
        "schema_sha256": sha256_hex(schema_bytes),
        "snapshot_sha256": snapshot_sha256,
        "snapshot_size_bytes": snapshot_size,
        "manifest_snapshot_sha256": expected_sha,
        "manifest_snapshot_size_bytes": expected_size,
        "network_access_performed": False,
        "provenance_authenticated": False,
        "legal_accuracy_verified": False,
        "human_review_required": True,
        "checks": checks,
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", required=True, type=Path)
    parser.add_argument("--snapshot", required=True, type=Path)
    parser.add_argument("--schema", type=Path, default=DEFAULT_SCHEMA)
    args = parser.parse_args(argv)

    try:
        manifest_bytes = args.manifest.read_bytes()
        snapshot_bytes = args.snapshot.read_bytes()
        schema_bytes = args.schema.read_bytes()
    except OSError as exc:
        print(json.dumps({
            "verifier_version": "0.1.0",
            "integrity_passed": False,
            "network_access_performed": False,
            "provenance_authenticated": False,
            "legal_accuracy_verified": False,
            "human_review_required": True,
            "errors": [f"Cannot read verification input: {exc}"],
        }, indent=2, sort_keys=True))
        return 1

    report = verify_source_snapshot(manifest_bytes, snapshot_bytes, schema_bytes)
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0 if report["integrity_passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
