#!/usr/bin/env python3
# Copyright (C) 2026 Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Fail-closed structural and semantic checks for proposed cooperative offers.

This tool validates a declaration only. It does not verify supplier identity,
source authenticity, official ISO code assignment, legal/tax compliance,
product equivalence, deliverability, savings, or transaction authorization.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from datetime import datetime, timedelta, timezone
from decimal import Decimal, InvalidOperation
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_SCHEMA_PATH = ROOT / "schemas" / "cooperative-offer-v1.schema.json"
DEFAULT_MAX_AVAILABILITY_AGE_HOURS = 24


class DuplicateJsonKey(ValueError):
    """Raised when JSON repeats an object key."""


def _reject_duplicate_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise DuplicateJsonKey(f"duplicate JSON object key: {key}")
        result[key] = value
    return result


def _reject_nonfinite_constant(value: str) -> None:
    raise ValueError(f"non-standard JSON numeric constant is not allowed: {value}")


def _parse_timestamp(value: Any) -> datetime | None:
    if not isinstance(value, str):
        return None
    try:
        parsed = datetime.fromisoformat(value[:-1] + "+00:00" if value.endswith("Z") else value)
    except ValueError:
        return None
    if parsed.tzinfo is None or parsed.utcoffset() is None:
        return None
    return parsed.astimezone(timezone.utc)


def _decimal(value: Any) -> Decimal | None:
    if not isinstance(value, str):
        return None
    try:
        result = Decimal(value)
    except (InvalidOperation, ValueError):
        return None
    if not result.is_finite():
        return None
    return result


def _error(errors: list[str], path: str, message: str) -> None:
    errors.append(f"{path}: {message}")


def validate_offer(
    document: Any,
    *,
    schema: dict[str, Any],
    now: datetime | None = None,
    max_availability_age: timedelta = timedelta(hours=DEFAULT_MAX_AVAILABILITY_AGE_HOURS),
) -> dict[str, Any]:
    """Validate structure and cross-field invariants without authorizing commerce."""
    supplied_now = now if now is not None else datetime.now(timezone.utc)
    if supplied_now.tzinfo is None or supplied_now.utcoffset() is None:
        raise ValueError("now must be timezone-aware")
    current = supplied_now.astimezone(timezone.utc)

    structural_validator = Draft202012Validator(schema, format_checker=FormatChecker())
    structural_errors = sorted(
        structural_validator.iter_errors(document),
        key=lambda item: (list(map(str, item.absolute_path)), item.message),
    )
    errors: list[str] = []
    warnings: list[str] = []

    if structural_errors:
        for item in structural_errors:
            path = "/".join(str(part) for part in item.absolute_path) or "$"
            _error(errors, path, item.message)
        return {
            "schema_id": schema.get("$id"),
            "result": "INVALID_DECLARATION",
            "structural_valid": False,
            "semantic_valid": False,
            "transaction_authorized": False,
            "errors": errors,
            "warnings": warnings,
            "notice": "Schema failure; semantic evaluation was not performed.",
        }

    # From here the schema guarantees the nested structures and string shapes.
    metadata = document["metadata"]
    product = document["product"]
    pack = product["pack"]
    terms = document["terms"]
    price = terms["unit_price"]
    availability = document["availability"]
    fulfillment = document["fulfillment"]
    jurisdiction = document["jurisdiction"]
    validity = document["validity"]
    evidence = document["evidence"]

    parsed_times: dict[str, datetime] = {}
    time_paths = {
        "created_at": validity["created_at"],
        "valid_from": validity["valid_from"],
        "expires_at": validity["expires_at"],
        "availability.observed_at": availability["observed_at"],
    }
    for path, raw in time_paths.items():
        parsed = _parse_timestamp(raw)
        if parsed is None:
            _error(errors, path, "timestamp must contain an explicit timezone and be parseable")
        else:
            parsed_times[path] = parsed

    created = parsed_times.get("created_at")
    valid_from = parsed_times.get("valid_from")
    expires = parsed_times.get("expires_at")
    observed = parsed_times.get("availability.observed_at")

    if created and valid_from and created > valid_from:
        _error(errors, "validity", "created_at must be at or before valid_from")
    if valid_from and expires and valid_from >= expires:
        _error(errors, "validity", "valid_from must be strictly before expires_at")
    if created and created > current:
        _error(errors, "validity.created_at", "created_at must not be in the future")
    if observed and observed > current:
        _error(errors, "availability.observed_at", "availability observation must not be in the future")

    minimum_order = _decimal(terms["minimum_order_quantity"])
    units_per_pack = _decimal(pack["units_per_pack"])
    each_quantity = _decimal(pack.get("each_quantity", "1"))
    price_amount = _decimal(price["amount"])
    price_base_quantity = _decimal(price["per_quantity"])

    for path, value in (
        ("terms.minimum_order_quantity", minimum_order),
        ("product.pack.units_per_pack", units_per_pack),
        ("product.pack.each_quantity", each_quantity),
        ("terms.unit_price.per_quantity", price_base_quantity),
    ):
        if value is None or value <= 0:
            _error(errors, path, "quantity/base quantity must be a finite decimal greater than zero")

    if price_amount is None or price_amount < 0:
        _error(errors, "terms.unit_price.amount", "amount must be a finite non-negative decimal string")
    if price["per_unit_code"] != pack["order_unit_code"]:
        _error(errors, "terms.unit_price.per_unit_code", "must match product.pack.order_unit_code unless a separately reviewed conversion is defined")

    base_currency = price["currency"]
    previous_break_quantity: Decimal | None = None
    for index, price_break in enumerate(terms.get("price_breaks", [])):
        path = f"terms.price_breaks[{index}]"
        break_quantity = _decimal(price_break["minimum_quantity"])
        break_amount = _decimal(price_break["unit_price_amount"])
        if break_quantity is None or break_quantity <= 0:
            _error(errors, path + ".minimum_quantity", "must be a finite decimal greater than zero")
        elif minimum_order is not None and break_quantity < minimum_order:
            _error(errors, path + ".minimum_quantity", "must not be below minimum_order_quantity")
        if break_amount is None or break_amount < 0:
            _error(errors, path + ".unit_price_amount", "must be a finite non-negative decimal string")
        if price_break["currency"] != base_currency:
            _error(errors, path + ".currency", "must match the base unit-price currency; use a separately evidenced FX conversion for comparisons")
        if break_quantity is not None and previous_break_quantity is not None and break_quantity <= previous_break_quantity:
            _error(errors, path + ".minimum_quantity", "price-break thresholds must be strictly increasing")
        if break_quantity is not None:
            previous_break_quantity = break_quantity

    lead_min = availability.get("lead_time_min")
    lead_max = availability.get("lead_time_max")
    if lead_min is not None and lead_max is not None and lead_min > lead_max:
        _error(errors, "availability", "lead_time_min must not exceed lead_time_max")

    delivery_min = fulfillment.get("estimated_delivery_days_min")
    delivery_max = fulfillment.get("estimated_delivery_days_max")
    if delivery_min is not None and delivery_max is not None and delivery_min > delivery_max:
        _error(errors, "fulfillment", "estimated_delivery_days_min must not exceed estimated_delivery_days_max")

    available_quantity = availability.get("quantity")
    stock_status = availability["status"]
    order_unit = pack["order_unit_code"]
    available_unit = availability.get("quantity_unit_code")
    if stock_status in {"in_stock", "limited"}:
        quantity = _decimal(available_quantity) if available_quantity != "unknown" else None
        if quantity is None or quantity <= 0:
            _error(errors, "availability.quantity", "in_stock/limited offers require a known positive quantity")
        if not available_unit:
            _error(errors, "availability.quantity_unit_code", "in_stock/limited offers require an explicit quantity unit")
        elif available_unit != order_unit:
            _error(errors, "availability.quantity_unit_code", "must match product.pack.order_unit_code unless a separately reviewed unit conversion is applied")
        if quantity is not None and quantity > 0 and minimum_order is not None and available_unit == order_unit and quantity < minimum_order:
            _error(errors, "availability.quantity", "known available quantity is below the minimum order quantity")

    transaction_mode = jurisdiction["transaction_mode"]
    if transaction_mode == "cross_border":
        if not fulfillment["cross_border_supported"]:
            _error(errors, "fulfillment.cross_border_supported", "must be true for cross-border transaction mode")
        origin = fulfillment.get("origin_country_code")
        if not origin:
            _error(errors, "fulfillment.origin_country_code", "required for cross-border offers")
        elif not any(code != origin for code in fulfillment["available_country_codes"]):
            _error(errors, "fulfillment.available_country_codes", "cross-border mode must identify at least one destination country different from origin")

    status = metadata["status"]
    fixture_only = metadata["fixture_only"]
    verification = evidence["verification_level"]
    source_reference = evidence.get("source_reference", "").strip()
    content_digest = evidence.get("content_sha256", "").strip()
    tax_treatment = terms["tax_treatment"]

    if status == "published":
        if fixture_only:
            _error(errors, "metadata.fixture_only", "synthetic fixtures cannot be published")
        if verification == "unverified":
            _error(errors, "evidence.verification_level", "published offers require at least supplier-attested evidence")
        if not source_reference:
            _error(errors, "evidence.source_reference", "published offers require a source reference")
        if not re.fullmatch(r"[a-fA-F0-9]{64}", content_digest):
            _error(errors, "evidence.content_sha256", "published offers require a SHA-256 digest of retained source evidence")
        if tax_treatment == "unknown":
            _error(errors, "terms.tax_treatment", "published offers require an explicit tax-treatment assertion; local tax correctness still needs jurisdiction review")
        if valid_from and expires and not (valid_from <= current < expires):
            _error(errors, "validity", "published offer must be currently within its validity window")
        if observed:
            age = current - observed
            if age > max_availability_age:
                _error(errors, "availability.observed_at", f"published offer availability is stale (>{max_availability_age.total_seconds() / 3600:g} hours)")
        if stock_status == "unknown":
            warnings.append("availability is unknown; buyers must not interpret this offer as in-stock")
    else:
        if fixture_only:
            warnings.append("synthetic fixture: not a purchasable supplier offer")
        if tax_treatment == "unknown":
            warnings.append("tax treatment is unknown; do not use the offer to commit a transaction")

    for index, charge in enumerate(terms.get("charges", [])):
        charge_value = charge["value"]
        if charge_value == "unknown":
            warnings.append(f"terms.charges[{index}]: {charge['kind']} cost is unknown, not zero")
        if charge["currency"] != base_currency:
            warnings.append(f"terms.charges[{index}]: charge currency differs from product currency; comparable landed cost requires an evidenced FX conversion")

    result = "DECLARATION_VALID_NO_TRANSACTION_AUTHORIZED" if not errors else "INVALID_DECLARATION"
    return {
        "schema_id": schema.get("$id"),
        "offer_id": metadata["offer_id"],
        "revision": metadata["revision"],
        "result": result,
        "structural_valid": True,
        "semantic_valid": not errors,
        "transaction_authorized": False,
        "errors": errors,
        "warnings": warnings,
        "notice": "Validation checks the supplied declaration only. It does not authenticate a supplier, validate official code-list membership or local law, prove product equivalence or availability, calculate realized savings, or authorize a purchase.",
    }


def _load_json(path: Path) -> Any:
    return json.loads(
        path.read_text(encoding="utf-8"),
        object_pairs_hook=_reject_duplicate_pairs,
        parse_constant=_reject_nonfinite_constant,
    )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("offer", type=Path, help="JSON offer declaration to validate")
    parser.add_argument("--schema", type=Path, default=DEFAULT_SCHEMA_PATH, help="JSON Schema path")
    parser.add_argument(
        "--as-of",
        help="timezone-aware ISO-8601 validation time (default: current UTC); primarily for deterministic tests",
    )
    parser.add_argument(
        "--max-availability-age-hours",
        type=int,
        default=DEFAULT_MAX_AVAILABILITY_AGE_HOURS,
        help=f"maximum allowed stock observation age for published offers (default: {DEFAULT_MAX_AVAILABILITY_AGE_HOURS})",
    )
    args = parser.parse_args(argv)

    if args.max_availability_age_hours < 0:
        parser.error("--max-availability-age-hours must be non-negative")

    try:
        schema = _load_json(args.schema)
        Draft202012Validator.check_schema(schema)
        document = _load_json(args.offer)
        if args.as_of:
            now = _parse_timestamp(args.as_of)
            if now is None:
                parser.error("--as-of must be a timezone-aware ISO-8601 timestamp")
        else:
            now = datetime.now(timezone.utc)
        report = validate_offer(
            document,
            schema=schema,
            now=now,
            max_availability_age=timedelta(hours=args.max_availability_age_hours),
        )
    except (OSError, UnicodeError, json.JSONDecodeError, DuplicateJsonKey, ValueError) as exc:
        report = {
            "result": "INVALID_DECLARATION",
            "structural_valid": False,
            "semantic_valid": False,
            "transaction_authorized": False,
            "errors": [str(exc)],
            "warnings": [],
            "notice": "Input or schema could not be safely parsed/validated.",
        }
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0 if report["semantic_valid"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
