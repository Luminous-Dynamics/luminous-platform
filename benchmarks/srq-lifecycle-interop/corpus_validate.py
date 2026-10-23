#!/usr/bin/env python3
"""Strict semantic conformance checker for the frozen SRQ lifecycle corpus.

This is test tooling only. It validates the protocol as data, cross-checks
immutable upstream pins, and never evaluates release qualification.
"""
from __future__ import annotations

import argparse
import hashlib
import importlib.metadata
import json
import sys
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker, SchemaError
from schema_validate import (
    EXPECTED_JSONSCHEMA_VERSION,
    find_remote_refs,
    load_strict_json,
    pointer,
)

PROTOCOL_SCHEMA = "srq-lifecycle-adversarial-protocol-v1"
PROTOCOL_ID = "srq-lifecycle-adversarial-v1"
REPORT_SCHEMA = "srq-corpus-validation-report-v1"
DRAFT_URI = "https://json-schema.org/draft/2020-12/schema"

EXPECTED_CASE_IDS = {
    "s0-domain-validated-status",
    "s0-independently-checked-status",
    "planned-gate-inflation",
    "planned-subject-commit-injection",
    "planned-artifact-hash-injection",
    "planned-result-count-injection",
    "s3-without-independent-verification",
    "s2-source-reproducible-without-source",
    "unknown-tier",
    "validated-without-validation-record",
    "authorized-without-prior-records",
    "snapshotted-without-prior-records",
    "applied-without-prior-records",
    "verified-without-prior-records",
    "promoted-without-any-evidence-records",
    "failed-without-outcome-record",
    "recovered-without-snapshot-outcome",
}
EXPECTED_POSITIVE_CONTROLS = {
    "valid-s2-source-reproducible-shape": ("mycelix_receipt", "S2", "source_reproducible", None),
    "valid-s4-domain-validated-shape": ("mycelix_receipt", "S4", "domain_validated", None),
    "valid-validated-phase-with-validation-record": ("nixward_transaction", None, None, "validated"),
    "valid-failed-phase-with-outcome": ("nixward_transaction", None, None, "failed"),
    "valid-recovered-phase-with-snapshot-outcome": ("nixward_transaction", None, None, "recovered"),
}


def sha256(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def semantic_errors(manifest: Any, lock: Any) -> list[dict[str, str]]:
    errors: list[dict[str, str]] = []

    def reject(code: str, message: str) -> None:
        errors.append({"code": code, "message": message})

    if not isinstance(manifest, dict):
        return [{"code": "manifest-root", "message": "manifest root must be an object"}]
    if not isinstance(lock, dict) or not isinstance(lock.get("pins"), list):
        return [{"code": "lock-shape", "message": "upstream lock must contain a pins array"}]

    if manifest.get("schema") != PROTOCOL_SCHEMA or manifest.get("protocol_id") != PROTOCOL_ID or manifest.get("version") != 1:
        reject("protocol-identity", "protocol schema, protocol_id, or version does not match frozen v1")
    validator = manifest.get("schema_validator", {})
    if not isinstance(validator, dict) or (
        validator.get("draft") != DRAFT_URI
        or validator.get("library") != "python-jsonschema"
        or validator.get("version") != EXPECTED_JSONSCHEMA_VERSION
        or validator.get("format_assertion_required") is not True
    ):
        reject("schema-validator-identity", "schema validator identity or format-assertion requirement changed")

    invariant = manifest.get("release_invariant", {})
    if not isinstance(invariant, dict) or (
        invariant.get("accepted_structural_control_must_leave_release_status") != "BLOCKED"
        or invariant.get("accepted_structural_control_must_leave_claim_ceiling") != "none"
        or invariant.get("release_pass_allowed") is not False
    ):
        reject("release-invariant", "structural positive controls must never qualify release")

    cases = manifest.get("cases")
    controls = manifest.get("positive_controls")
    metrics = manifest.get("target_metrics")
    if not isinstance(cases, list):
        reject("cases-type", "cases must be an array")
        cases = []
    if not isinstance(controls, list):
        reject("positive-controls-type", "positive_controls must be an array")
        controls = []
    if not isinstance(metrics, dict):
        reject("target-metrics-type", "target_metrics must be an object")
        metrics = {}

    case_ids = [item.get("id") for item in cases if isinstance(item, dict)]
    control_ids = [item.get("id") for item in controls if isinstance(item, dict)]
    if len(cases) != 17 or set(case_ids) != EXPECTED_CASE_IDS or len(case_ids) != len(set(case_ids)):
        reject("case-inventory", "v1 must contain exactly the 17 frozen, unique negative case IDs")
    if len(controls) != 5 or set(control_ids) != set(EXPECTED_POSITIVE_CONTROLS) or len(control_ids) != len(set(control_ids)):
        reject("positive-control-inventory", "v1 must contain exactly the 5 frozen, unique positive-control IDs")
    if set(case_ids) & set(control_ids):
        reject("cross-inventory-id-collision", "negative and positive-control IDs must be disjoint")

    for item in cases:
        if not isinstance(item, dict):
            reject("case-shape", "each negative case must be an object")
            continue
        expected_schema = "INVALID" if item.get("input_kind") == "mycelix_receipt" else "NOT_APPLICABLE"
        if item.get("input_kind") not in {"mycelix_receipt", "nixward_transaction"}:
            reject("case-input-kind", f"negative case {item.get('id')!r} has an unknown input kind")
        if item.get("expected_schema_result") != expected_schema:
            reject("case-schema-disposition", f"negative case {item.get('id')!r} has a wrong schema disposition")
        if item.get("expected_lifecycle_result") != "REJECT":
            reject("case-lifecycle-disposition", f"negative case {item.get('id')!r} must be rejected")

    for item in controls:
        if not isinstance(item, dict):
            reject("positive-control-shape", "each positive control must be an object")
            continue
        ident = item.get("id")
        expected = EXPECTED_POSITIVE_CONTROLS.get(ident)
        if expected is None:
            reject("positive-control-id", f"unknown positive-control ID {ident!r}")
            continue
        kind, tier, ceiling, phase = expected
        if item.get("input_kind") != kind:
            reject("positive-control-kind", f"positive control {ident!r} has the wrong input kind")
        if item.get("expected_lifecycle_result") != "ACCEPT_RECORD_KEEP_RELEASE_BLOCKED":
            reject("positive-control-lifecycle", f"positive control {ident!r} must be accepted only while release remains blocked")
        if kind == "mycelix_receipt":
            if item.get("expected_schema_result") != "VALID" or item.get("expected_evidence_tier") != tier or item.get("expected_evidence_claim_ceiling") != ceiling:
                reject("positive-control-evidence-ceiling", f"positive control {ident!r} has the wrong schema result/tier/claim ceiling")
        elif item.get("expected_schema_result") != "NOT_APPLICABLE" or item.get("expected_transaction_phase") != phase:
            reject("positive-control-transaction-phase", f"positive control {ident!r} has the wrong schema result or phase")

    expected_metrics = {
        "negative_case_count": 17,
        "negative_false_accept_count": 0,
        "positive_control_count": 5,
        "positive_false_reject_count": 0,
        "release_false_pass_count": 0,
    }
    for key, expected in expected_metrics.items():
        if metrics.get(key) != expected:
            reject("target-metric", f"target metric {key!r} must remain {expected}")

    upstream = manifest.get("upstream_inputs")
    if not isinstance(upstream, list) or len(upstream) != 4:
        reject("upstream-inventory", "exactly four upstream inputs are required")
        upstream = []
    lock_pins = lock.get("pins", [])
    used: set[tuple[str, str, str, str]] = set()
    for item in upstream:
        if not isinstance(item, dict):
            reject("upstream-item-shape", "each upstream input must be an object")
            continue
        identity = (
            item.get("repository"),
            item.get("commit_sha"),
            item.get("path"),
            item.get("git_blob_sha1"),
        )
        matching = [
            pin for pin in lock_pins
            if isinstance(pin, dict)
            and (
                pin.get("repository"),
                pin.get("commit_sha"),
                pin.get("path"),
                pin.get("git_blob_sha1"),
            ) == identity
        ]
        if len(matching) != 1:
            reject("upstream-pin-mismatch", f"upstream input {item.get('name')!r} does not match exactly one immutable lock pin")
        if identity in used:
            reject("duplicate-upstream-pin", f"upstream input {item.get('name')!r} duplicates an existing pin identity")
        used.add(identity)

    return errors


def validate_files(schema_path: Path, manifest_path: Path, lock_path: Path) -> tuple[str, list[dict[str, str]], dict[str, Any]]:
    report: dict[str, Any] = {
        "schema": REPORT_SCHEMA,
        "result": "ERROR",
        "validator": {
            "library": "python-jsonschema",
            "required_version": EXPECTED_JSONSCHEMA_VERSION,
            "installed_version": None,
            "draft": DRAFT_URI,
            "format_assertion": True,
        },
        "schema_input": {"path_label": schema_path.name, "sha256": None, "bytes": None},
        "manifest_input": {"path_label": manifest_path.name, "sha256": None, "bytes": None},
        "lock_input": {"path_label": lock_path.name, "sha256": None, "bytes": None},
        "errors": [],
        "limitations": [
            "Protocol validation proves corpus shape, inventory and pin consistency only.",
            "The manifest is not signed or independently timestamped; digest binding is not proof of pre-registration.",
            "This utility never establishes source truth, authorization, runtime authenticity or release qualification.",
        ],
    }
    errors: list[dict[str, str]] = []
    installed = importlib.metadata.version("jsonschema")
    report["validator"]["installed_version"] = installed
    if installed != EXPECTED_JSONSCHEMA_VERSION:
        raise RuntimeError(f"validator version mismatch: expected {EXPECTED_JSONSCHEMA_VERSION}, found {installed}")

    schema, schema_raw = load_strict_json(schema_path, 2 * 1024 * 1024)
    manifest, manifest_raw = load_strict_json(manifest_path, 2 * 1024 * 1024)
    lock, lock_raw = load_strict_json(lock_path, 2 * 1024 * 1024)
    report["schema_input"].update(sha256=sha256(schema_raw), bytes=len(schema_raw))
    report["manifest_input"].update(sha256=sha256(manifest_raw), bytes=len(manifest_raw))
    report["lock_input"].update(sha256=sha256(lock_raw), bytes=len(lock_raw))

    if not isinstance(schema, dict) or schema.get("$schema") != DRAFT_URI:
        raise ValueError("corpus schema must be an object declaring Draft 2020-12")
    remote_refs = find_remote_refs(schema)
    if remote_refs:
        raise ValueError("remote $ref values are forbidden: " + "; ".join(remote_refs))
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema, format_checker=FormatChecker())
    errors.extend({
        "code": "json-schema",
        "message": f"{pointer(error.absolute_path) or '$'}: {error.message}",
    } for error in sorted(validator.iter_errors(manifest), key=lambda e: (list(map(str, e.absolute_path)), e.message)))
    errors.extend(semantic_errors(manifest, lock))
    if errors:
        report["result"] = "INVALID"
    else:
        report["result"] = "VALID"
    report["protocol_id"] = manifest.get("protocol_id") if isinstance(manifest, dict) else None
    report["semantic_error_count"] = len(errors)
    report["errors"] = errors
    return report["result"], errors, report


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--schema", required=True, type=Path)
    parser.add_argument("--manifest", required=True, type=Path)
    parser.add_argument("--lock", required=True, type=Path)
    parser.add_argument("--expect", choices=("valid", "invalid"), required=True)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    try:
        result, errors, report = validate_files(args.schema, args.manifest, args.lock)
    except Exception as exc:
        report = {
            "schema": REPORT_SCHEMA,
            "result": "ERROR",
            "error": f"{type(exc).__name__}: {exc}",
            "errors": [],
            "limitations": ["No protocol conformance result was produced."],
        }
        result = "ERROR"
        errors = []

    encoded = json.dumps(report, sort_keys=True, separators=(",", ":"), ensure_ascii=False) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(encoded, encoding="utf-8")
    print(encoded, end="")
    if result == "ERROR":
        return 2
    matches = result == args.expect.upper()
    return 0 if matches else 1


if __name__ == "__main__":
    raise SystemExit(main())
