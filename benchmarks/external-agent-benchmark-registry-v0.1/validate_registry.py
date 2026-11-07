#!/usr/bin/env python3
"""Validation-only guard for the external benchmark evidence registry.

The benchmark runtime remains Rust. This script checks the public JSON Schema
and cross-field constraints that a JSON Schema cannot conveniently express.
"""
from __future__ import annotations

import copy
import json
import sys
from datetime import date
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker

ROOT = Path(__file__).resolve().parent
SCHEMA_PATH = ROOT / "registry.schema.json"
SNAPSHOT_PATH = ROOT / "registry.snapshot-2026-11-07.json"
FORBIDDEN_METRICS = {
    "overall_sre_score",
    "overall_agent_score",
    "overall_governance_score",
    "composite_sre_score",
    "translated_sre_score",
}


def read_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise SystemExit(f"cannot load {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise SystemExit(f"{path} must contain a JSON object")
    return value


def semantic_errors(snapshot: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    snapshot_date = date.fromisoformat(snapshot["snapshot_date"])
    record_ids: set[str] = set()

    for record in snapshot["records"]:
        rid = record["id"]
        if rid in record_ids:
            errors.append(f"{rid}: duplicate record id")
        record_ids.add(rid)

        publication = record["publication"]
        observed = date.fromisoformat(publication["observed_at"])
        published_at = publication["published_at"]
        if observed > snapshot_date:
            errors.append(f"{rid}: observation date after snapshot date")
        if published_at is not None:
            published = date.fromisoformat(published_at)
            if published > observed or published > snapshot_date:
                errors.append(f"{rid}: publication date after observation/snapshot date")

        source = record["source"]
        capture = source["capture"]
        state = capture["status"]
        content_hash = capture["content_sha256"]
        blob_sha = source["raw_git_blob_sha"]
        if not source["reference_id"].strip():
            errors.append(f"{rid}: source reference identity is empty")
        if state == "metadata_only_no_content_hash" and content_hash is not None:
            errors.append(f"{rid}: metadata-only source claims a content hash")
        if state == "raw_git_blob_sha_only":
            if not blob_sha or content_hash is not None:
                errors.append(f"{rid}: Git-blob-only status needs blob SHA and no content SHA")
        if state == "content_sha256_verified" and content_hash is None:
            errors.append(f"{rid}: verified content capture has no SHA-256")

        metric_ids: set[str] = set()
        for metric in record["metrics"]:
            mid = metric["metric_id"]
            if mid in metric_ids:
                errors.append(f"{rid}: duplicate metric id {mid}")
            metric_ids.add(mid)
            if mid in FORBIDDEN_METRICS or "composite" in mid or mid.startswith("overall-"):
                errors.append(f"{rid}/{mid}: composite or translated aggregate is forbidden")

            if metric["value_semantics"] != "derived_from_reported_counts":
                if metric["value_semantics"] == "owner_claim" and metric["value"] is None:
                    errors.append(f"{rid}/{mid}: owner claim has no value")
                continue

            numerator = metric["numerator"]
            denominator = metric["denominator"]
            if numerator is None or denominator is None or denominator <= 0:
                errors.append(f"{rid}/{mid}: derived count metric requires a positive denominator")
                continue
            if numerator > denominator:
                errors.append(f"{rid}/{mid}: numerator exceeds denominator")
                continue

            unit = metric["unit"]
            if unit == "percent":
                expected = numerator / denominator * 100.0
                tolerance = 0.11
            elif unit == "fraction":
                expected = numerator / denominator
                tolerance = 0.0011
            else:
                continue
            value = metric["value"]
            if not isinstance(value, (int, float)) or abs(float(value) - expected) > tolerance:
                errors.append(
                    f"{rid}/{mid}: value {value!r} does not match "
                    f"{numerator}/{denominator} ({expected:.5f})"
                )

    return errors


def validate(snapshot: dict[str, Any], schema: dict[str, Any]) -> list[str]:
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema, format_checker=FormatChecker())
    errors = [
        f"{'/'.join(str(p) for p in err.absolute_path) or '$'}: {err.message}"
        for err in sorted(
            validator.iter_errors(snapshot),
            key=lambda item: list(map(str, item.absolute_path)),
        )
    ]
    if errors:
        return errors
    return semantic_errors(snapshot)


def expect_rejected(name: str, candidate: dict[str, Any], schema: dict[str, Any]) -> None:
    if not validate(candidate, schema):
        raise SystemExit(f"negative control unexpectedly passed: {name}")
    print(f"rejected as expected: {name}")


def main() -> int:
    schema = read_json(SCHEMA_PATH)
    snapshot = read_json(SNAPSHOT_PATH)

    errors = validate(snapshot, schema)
    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 1

    print(
        f"valid registry: {len(snapshot['records'])} records; "
        f"snapshot {snapshot['snapshot_date']}"
    )

    missing_source = copy.deepcopy(snapshot)
    del missing_source["records"][0]["source"]["reference_id"]
    expect_rejected("missing source identity", missing_source, schema)

    missing_denominator = copy.deepcopy(snapshot)
    del missing_denominator["records"][0]["metrics"][0]["denominator"]
    expect_rejected("missing denominator", missing_denominator, schema)

    unknown_semantics = copy.deepcopy(snapshot)
    unknown_semantics["records"][0]["metrics"][0]["value_semantics"] = "unknown_metric"
    expect_rejected("unknown metric semantics", unknown_semantics, schema)

    bad_date_order = copy.deepcopy(snapshot)
    bad_date_order["records"][0]["publication"]["published_at"] = "2026-11-08"
    expect_rejected("publication date after observation/snapshot", bad_date_order, schema)

    composite = copy.deepcopy(snapshot)
    composite["records"][0]["metrics"][0]["metric_id"] = "overall_sre_score"
    expect_rejected("composite or translated score injection", composite, schema)

    print("all registry validation and negative controls passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
