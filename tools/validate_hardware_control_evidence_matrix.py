#!/usr/bin/env python3
"""Validate the control evidence matrix and fail closed on unsupported pass claims."""

from __future__ import annotations

import argparse
import re
import sys
from datetime import date
from pathlib import Path
from urllib.parse import urlparse
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker

from validate_hardware_portfolio import load_json


_SHA40 = re.compile(r"^[0-9a-f]{40}$")


def matrix_errors(matrix: dict[str, Any], schema: dict[str, Any]) -> list[str]:
    """Return schema and cross-reference/evidence-contract violations."""
    errors: list[str] = []
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema, format_checker=FormatChecker())
    for error in sorted(validator.iter_errors(matrix), key=lambda item: list(map(str, item.path))):
        where = "$" + "".join(
            f"[{part}]" if isinstance(part, int) else f".{part}"
            for part in error.absolute_path
        )
        errors.append(f"{where}: {error.message}")
    if errors:
        return errors

    for path, raw_date in (
        ("$.as_of", matrix["as_of"]),
    ):
        try:
            date.fromisoformat(raw_date)
        except (TypeError, ValueError):
            errors.append(f"{path} must be an ISO calendar date")

    references = {item["id"]: item for item in matrix["references"]}
    if len(references) != len(matrix["references"]):
        errors.append("$.references contains duplicate reference IDs")
    for index, reference in enumerate(matrix["references"]):
        prefix = f"$.references[{index}]"
        parsed = urlparse(reference["url"])
        if parsed.scheme != "https" or not parsed.netloc:
            errors.append(f"{prefix}.url must be an HTTPS URL")
        try:
            date.fromisoformat(reference["last_reviewed"])
        except (TypeError, ValueError):
            errors.append(f"{prefix}.last_reviewed must be an ISO calendar date")

    seen_control_ids: set[str] = set()
    for index, control in enumerate(matrix["controls"]):
        prefix = f"$.controls[{index}]"
        control_id = control["id"]
        if control_id in seen_control_ids:
            errors.append(f"{prefix}.id duplicates {control_id!r}")
        seen_control_ids.add(control_id)

        for mapping in control["mapped_controls"]:
            reference_id = mapping.split(":", 1)[0]
            if reference_id not in references:
                errors.append(f"{prefix}.mapped_controls refers to unknown source {reference_id!r}")

        status = control["status"]
        record = control["assessment_record"]
        if status == "NOT_EVALUATED":
            if record is not None:
                errors.append(f"{prefix}: NOT_EVALUATED must not carry an assessment_record")
            continue

        if record is None:
            errors.append(f"{prefix}: {status} requires an assessment_record")
            continue
        if record["result"] != status:
            errors.append(f"{prefix}: status and assessment_record.result must match")
        try:
            date.fromisoformat(record["assessed_on"])
        except (TypeError, ValueError):
            errors.append(f"{prefix}.assessment_record.assessed_on must be an ISO calendar date")
        if not _SHA40.fullmatch(record["tested_revision"]):
            errors.append(f"{prefix}.assessment_record.tested_revision must be a full 40-character lowercase commit SHA")
        if not record["evidence_refs"]:
            errors.append(f"{prefix}: evaluated status requires evidence references")
        if status == "PASS":
            if control["open_gaps"]:
                errors.append(f"{prefix}: PASS cannot have unresolved open_gaps")
            if record["limitations"]:
                errors.append(f"{prefix}: PASS cannot have unaccepted limitations; use PARTIAL until resolved")
        if status == "NOT_APPLICABLE" and not record["limitations"]:
            errors.append(f"{prefix}: NOT_APPLICABLE requires a documented justification in limitations")

    return errors


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--matrix", type=Path, default=Path("security/control-evidence-matrix-v1.json"))
    parser.add_argument("--schema", type=Path, default=Path("schemas/hardware-control-evidence-matrix-v1.schema.json"))
    args = parser.parse_args(argv)

    try:
        matrix = load_json(args.matrix)
        schema = load_json(args.schema)
    except ValueError as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        return 1

    if not isinstance(matrix, dict) or not isinstance(schema, dict):
        print("FAIL: matrix and schema roots must be JSON objects", file=sys.stderr)
        return 1

    failures = matrix_errors(matrix, schema)
    if failures:
        for failure in failures:
            print(f"FAIL: {failure}", file=sys.stderr)
        return 1

    print(
        "PASS: control evidence matrix is structurally valid; "
        "all control statuses remain unassessed or require traceable evidence. "
        "This is not a compliance determination or deployment authorization."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
