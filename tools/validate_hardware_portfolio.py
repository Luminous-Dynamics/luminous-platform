#!/usr/bin/env python3
"""Validate the hardware portfolio schema and fail-closed maturity invariants."""

from __future__ import annotations

import argparse
import json
import sys
from datetime import date
from pathlib import Path
from urllib.parse import urlparse
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker


class DuplicateJSONKeyError(ValueError):
    """Raised for a duplicated JSON object key."""


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise DuplicateJSONKeyError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_json(path: Path) -> Any:
    """Parse strict UTF-8 JSON and reject duplicate object keys."""
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as exc:
        raise ValueError(f"cannot read {path}: {exc}") from exc
    try:
        return json.loads(text, object_pairs_hook=_unique_object)
    except (json.JSONDecodeError, DuplicateJSONKeyError) as exc:
        raise ValueError(f"invalid JSON in {path}: {exc}") from exc


def catalog_errors(payload: dict[str, Any], schema: dict[str, Any]) -> list[str]:
    """Return schema and cross-field contract violations."""
    errors: list[str] = []
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema, format_checker=FormatChecker())
    for error in sorted(validator.iter_errors(payload), key=lambda item: list(map(str, item.path))):
        where = "$" + "".join(
            f"[{part}]" if isinstance(part, int) else f".{part}"
            for part in error.absolute_path
        )
        errors.append(f"{where}: {error.message}")

    if errors:
        return errors

    try:
        date.fromisoformat(payload["as_of"])
    except (KeyError, TypeError, ValueError):
        errors.append("$.as_of must be an ISO calendar date")

    seen: set[str] = set()
    for index, item in enumerate(payload["items"]):
        prefix = f"$.items[{index}]"
        item_id = item["id"]
        if item_id in seen:
            errors.append(f"{prefix}.id duplicates {item_id!r}")
        seen.add(item_id)

        try:
            date.fromisoformat(item["last_checked"])
        except (TypeError, ValueError):
            errors.append(f"{prefix}.last_checked must be an ISO calendar date")

        for source in item["source_urls"]:
            parsed = urlparse(source)
            if parsed.scheme != "https" or not parsed.netloc:
                errors.append(f"{prefix}.source_urls must contain HTTPS URLs: {source!r}")

        maturity = item["maturity"]
        if maturity in {
            "H1_SOURCE_REVIEWED",
            "H2_BUILDABLE",
            "H3_PLATFORM_INTEGRATED",
            "H4_OPERATIONALLY_QUALIFIED",
            "H5_DEPLOYMENT_AUTHORIZED",
        }:
            if not item["source_revision"]:
                errors.append(f"{prefix}: {maturity} requires an immutable source/model revision")
            if item["license_status"] not in {
                "compatible_open_license_reviewed",
                "vendor_terms_reviewed",
            }:
                errors.append(f"{prefix}: {maturity} requires reviewed license/vendor terms")
            if item["source_completeness"] not in {
                "native_design_and_bom_reviewed",
                "product_documentation_reviewed",
            }:
                errors.append(f"{prefix}: {maturity} requires reviewed source/product collateral")

        if maturity in {"H2_BUILDABLE", "H3_PLATFORM_INTEGRATED", "H4_OPERATIONALLY_QUALIFIED", "H5_DEPLOYMENT_AUTHORIZED"}:
            if item["availability_status"] not in {"verified_stock", "manufacturing_path_verified"}:
                errors.append(f"{prefix}: {maturity} requires a verified procurement/manufacturing path")
            if item["bill_of_materials_status"] != "complete_and_reviewed":
                errors.append(f"{prefix}: {maturity} requires a reviewed complete BOM or configuration BOM")

        if maturity in {"H3_PLATFORM_INTEGRATED", "H4_OPERATIONALLY_QUALIFIED", "H5_DEPLOYMENT_AUTHORIZED"}:
            if not item["qualification_evidence_refs"]:
                errors.append(f"{prefix}: {maturity} requires qualification evidence references")

        if maturity == "H5_DEPLOYMENT_AUTHORIZED":
            if item["deployment_authorization"] != "approved_for_named_profile":
                errors.append(f"{prefix}: H5 requires named-profile deployment authorization")
            if not item["deployment_profile"]:
                errors.append(f"{prefix}: H5 requires deployment_profile")
            if not item["open_blockers"]:
                pass
            else:
                errors.append(f"{prefix}: H5 cannot have open_blockers")
        elif item["deployment_authorization"] == "approved_for_named_profile":
            errors.append(f"{prefix}: authorization cannot be claimed below H5")

        if item["maturity"] == "GAP_UNSELECTED" and item["item_type"] != "selection_gap":
            errors.append(f"{prefix}: GAP_UNSELECTED requires item_type=selection_gap")
        if item["item_type"] == "selection_gap" and item["maturity"] != "GAP_UNSELECTED":
            errors.append(f"{prefix}: selection_gap items cannot be promoted to product maturity")

        if item["maturity"] in {"H0_CANDIDATE", "GAP_UNSELECTED", "REFERENCE_ONLY"} and not item["open_blockers"]:
            errors.append(f"{prefix}: unresolved entries must retain at least one explicit blocker")

    return errors


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--catalog", type=Path, default=Path("hardware/portfolio-v1.json"))
    parser.add_argument("--schema", type=Path, default=Path("schemas/hardware-portfolio-v1.schema.json"))
    args = parser.parse_args(argv)

    try:
        payload = load_json(args.catalog)
        schema = load_json(args.schema)
    except ValueError as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        return 1

    if not isinstance(payload, dict) or not isinstance(schema, dict):
        print("FAIL: catalog and schema roots must be JSON objects", file=sys.stderr)
        return 1

    errors = catalog_errors(payload, schema)
    if errors:
        for error in errors:
            print(f"FAIL: {error}", file=sys.stderr)
        return 1

    print(
        "PASS: hardware catalog contract is structurally valid; "
        "this does not qualify hardware or authorize deployment."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
