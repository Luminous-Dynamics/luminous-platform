#!/usr/bin/env python3
# Copyright (C) 2026 Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Fail-closed declaration checks for cooperative purchase intents.

This validates a local declaration and its temporal/financial relationships. It
never authenticates the actors, verifies consent or mandate evidence, checks
official code-list membership/local law, checks uniqueness in a durable store,
or authorizes/dispatches an order.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys

import rfc8785
from datetime import datetime, timezone
from decimal import Decimal, InvalidOperation
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_SCHEMA_PATH = ROOT / "schemas" / "cooperative-purchase-intent-v1.schema.json"
ACTIVE_STATES = {"draft", "non_binding_interest", "quote_request", "binding_order"}


class DuplicateJsonKey(ValueError):
    """Raised when an input JSON object contains a repeated key."""


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise DuplicateJsonKey(f"duplicate JSON object key: {key}")
        result[key] = value
    return result


def _reject_nonfinite_constant(value: str) -> None:
    raise ValueError(f"non-standard JSON numeric constant is not allowed: {value}")


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


def _amount(value: Any) -> Decimal | None:
    if not isinstance(value, str):
        return None
    try:
        amount = Decimal(value)
    except (InvalidOperation, ValueError):
        return None
    return amount if amount.is_finite() else None


def _add(errors: list[str], path: str, message: str) -> None:
    errors.append(f"{path}: {message}")


def consent_scope_payload(document: dict[str, Any]) -> dict[str, Any]:
    """Build the exact party/recipient/field scope approved by the recorded consent."""
    buyer = document["buyer"]
    demand = document["demand"]
    sharing = document["sharing"]
    consent = sharing["consent"]
    fields = set(sharing["shareable_fields"])
    shared_data: dict[str, Any] = {}

    if "product_reference" in fields:
        shared_data["product_reference"] = demand["product_reference"]
    if "aggregate_quantity" in fields:
        shared_data["aggregate_quantity"] = {
            "requested_quantity": demand["requested_quantity"],
            "requested_unit_code": demand["requested_unit_code"],
            "requested_unit_code_system": demand["requested_unit_code_system"],
        }
    if "destination_country" in fields:
        shared_data["destination_country"] = demand["destination_country_code"]
    if "destination_region" in fields:
        shared_data["destination_region"] = demand.get("destination_region")
    if "delivery_window" in fields:
        shared_data["delivery_window"] = {
            "start": demand.get("delivery_window_start"),
            "end": demand.get("delivery_window_end"),
        }
    if "buyer_contact_via_platform" in fields:
        shared_data["buyer_contact_via_platform"] = {"buyer_party_id": buyer["buyer_party_id"]}

    return {
        "schema": "luminous.sharing-consent-scope/v1",
        "consent_id": consent["consent_id"],
        "granted_by": consent["granted_by"],
        "granted_at": consent["granted_at"],
        "expires_at": consent["expires_at"],
        "revocable": consent["revocable"],
        "buyer_party_id": buyer["buyer_party_id"],
        "buyer_legal_entity_id": buyer["buyer_legal_entity_id"],
        "purpose": sharing["purpose"],
        "recipient_scope": sharing["recipient_scope"],
        "recipients": sorted(sharing["recipients"]),
        "shareable_fields": sorted(fields),
        "shared_data": shared_data,
    }


def consent_scope_sha256(document: dict[str, Any]) -> str:
    """Hash the exact consent scope using RFC 8785 canonical JSON bytes."""
    return hashlib.sha256(rfc8785.dumps(consent_scope_payload(document))).hexdigest()


def purchase_scope_payload(document: dict[str, Any]) -> dict[str, Any]:
    """Return the fixed v1 projection that a single-use mandate must bind."""
    metadata = document["metadata"]
    buyer = document["buyer"]
    demand = document["demand"]
    product = demand["product_reference"]
    offer = document["accepted_offer"]
    accepted_total = offer["accepted_total"]
    return {
        "schema": "luminous.purchase-scope/v1",
        "intent_id": metadata["intent_id"],
        "intent_revision": metadata["revision"],
        "idempotency_key": metadata["idempotency_key"],
        "buyer_party_id": buyer["buyer_party_id"],
        "buyer_legal_entity_id": buyer["buyer_legal_entity_id"],
        "buyer_site_id": buyer.get("buyer_site_id"),
        "local_cooperative_id": buyer.get("local_cooperative_id"),
        "offer_id": offer["offer_id"],
        "offer_revision": offer["revision"],
        "offer_sha256": offer["offer_sha256"].lower(),
        "product_reference": {
            "scheme": product["scheme"],
            "value": product["value"],
            "offer_specification_sha256": product.get("offer_specification_sha256"),
        },
        "sharing_consent": {
            "consent_id": document["sharing"]["consent"]["consent_id"],
            "scope_sha256": document["sharing"]["consent"]["scope_sha256"].lower(),
        },
        "requested_quantity": demand["requested_quantity"],
        "requested_unit_code": demand["requested_unit_code"],
        "requested_unit_code_system": demand["requested_unit_code_system"],
        "destination_country_code": demand["destination_country_code"],
        "destination_region": demand.get("destination_region"),
        "delivery_window_start": demand.get("delivery_window_start"),
        "delivery_window_end": demand.get("delivery_window_end"),
        "substitution_policy": demand["substitution_policy"],
        "accepted_total": {
            "amount": accepted_total["amount"],
            "currency": accepted_total["currency"],
        },
        "authorized_maximum_total": {
            "amount": document["authorization"]["maximum_total"]["amount"],
            "currency": document["authorization"]["maximum_total"]["currency"],
        },
    }


def purchase_scope_sha256(document: dict[str, Any]) -> str:
    """Hash the canonical JSON projection used by the v1 single-use mandate."""
    # RFC 8785 JCS defines cross-language key ordering and string/primitive encoding.
    # Monetary amounts and quantities remain decimal strings, never JSON floats.
    canonical = rfc8785.dumps(purchase_scope_payload(document))
    return hashlib.sha256(canonical).hexdigest()


def validate_intent(
    document: Any,
    *,
    schema: dict[str, Any],
    now: datetime | None = None,
) -> dict[str, Any]:
    supplied_now = now if now is not None else datetime.now(timezone.utc)
    if supplied_now.tzinfo is None or supplied_now.utcoffset() is None:
        raise ValueError("now must be timezone-aware")
    current = supplied_now.astimezone(timezone.utc)

    validator = Draft202012Validator(schema, format_checker=FormatChecker())
    structural_errors = sorted(
        validator.iter_errors(document),
        key=lambda item: (list(map(str, item.absolute_path)), item.message),
    )
    errors: list[str] = []
    warnings: list[str] = []

    if structural_errors:
        for item in structural_errors:
            path = "/".join(str(part) for part in item.absolute_path) or "$"
            _add(errors, path, item.message)
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

    metadata = document["metadata"]
    state = metadata["state"]
    buyer = document["buyer"]
    demand = document["demand"]
    sharing = document["sharing"]
    consent = sharing["consent"]

    timestamps: dict[str, datetime] = {}
    raw_timestamps = {
        "metadata.created_at": metadata["created_at"],
        "metadata.updated_at": metadata["updated_at"],
        "metadata.expires_at": metadata["expires_at"],
        "sharing.consent.granted_at": consent["granted_at"],
        "sharing.consent.expires_at": consent["expires_at"],
    }
    if demand.get("delivery_window_start") is not None:
        raw_timestamps["demand.delivery_window_start"] = demand["delivery_window_start"]
    if demand.get("delivery_window_end") is not None:
        raw_timestamps["demand.delivery_window_end"] = demand["delivery_window_end"]

    if "accepted_offer" in document:
        raw_timestamps["accepted_offer.accepted_at"] = document["accepted_offer"]["accepted_at"]
        raw_timestamps["accepted_offer.offer_expires_at"] = document["accepted_offer"]["offer_expires_at"]
    if "authorization" in document:
        raw_timestamps["authorization.authorized_at"] = document["authorization"]["authorized_at"]
        raw_timestamps["authorization.expires_at"] = document["authorization"]["expires_at"]

    for path, raw in raw_timestamps.items():
        parsed = _timestamp(raw)
        if parsed is None:
            _add(errors, path, "timestamp must be parseable and include an explicit timezone")
        else:
            timestamps[path] = parsed

    created = timestamps.get("metadata.created_at")
    updated = timestamps.get("metadata.updated_at")
    expires = timestamps.get("metadata.expires_at")
    if created and updated and updated < created:
        _add(errors, "metadata.updated_at", "must be at or after created_at")
    if created and expires and expires <= created:
        _add(errors, "metadata.expires_at", "must be strictly after created_at")
    if updated and created and updated > current and state in ACTIVE_STATES:
        _add(errors, "metadata.updated_at", "active intent must not claim a future update")
    if created and created > current:
        _add(errors, "metadata.created_at", "must not be in the future")

    try:
        expected_consent_scope_digest = consent_scope_sha256(document)
    except (ValueError, UnicodeError, OverflowError):
        _add(
            errors,
            "sharing.consent.scope_sha256",
            "consent scope contains a value that cannot be represented by RFC 8785 JCS; no digest or authorization may be accepted",
        )
    else:
        if document["sharing"]["consent"]["scope_sha256"].lower() != expected_consent_scope_digest:
            _add(
                errors,
                "sharing.consent.scope_sha256",
                "does not match the RFC 8785 canonical consent scope; changing recipients, purpose, fields, buyer entity, or any approved field value requires fresh consent",
            )

    consent_granted = timestamps.get("sharing.consent.granted_at")
    consent_expires = timestamps.get("sharing.consent.expires_at")
    if consent_granted and consent_expires and consent_granted >= consent_expires:
        _add(errors, "sharing.consent", "granted_at must be strictly before expires_at")
    if state in ACTIVE_STATES:
        if expires and current >= expires:
            _add(errors, "metadata.expires_at", "active intent is expired; transition it to expired before reuse")
        if consent_granted and consent_granted > current:
            _add(errors, "sharing.consent.granted_at", "consent cannot be used before its granted_at time")
        if consent_expires and current >= consent_expires:
            _add(errors, "sharing.consent.expires_at", "active intent cannot rely on expired sharing consent")

    delivery_start = timestamps.get("demand.delivery_window_start")
    delivery_end = timestamps.get("demand.delivery_window_end")
    if delivery_start and delivery_end and delivery_start >= delivery_end:
        _add(errors, "demand", "delivery_window_start must be strictly before delivery_window_end")

    if state == "expired" and expires and current < expires:
        _add(errors, "metadata.state", "expired state is inconsistent with a future intent expiry")

    if state == "binding_order":
        offer = document["accepted_offer"]
        authorization = document["authorization"]
        accepted_at = timestamps.get("accepted_offer.accepted_at")
        offer_expires = timestamps.get("accepted_offer.offer_expires_at")
        authorized_at = timestamps.get("authorization.authorized_at")
        authorization_expires = timestamps.get("authorization.expires_at")

        if accepted_at and offer_expires and accepted_at >= offer_expires:
            _add(errors, "accepted_offer", "accepted_at must be strictly before offer_expires_at")
        if accepted_at and accepted_at > current:
            _add(errors, "accepted_offer.accepted_at", "cannot be in the future")
        if offer_expires and current >= offer_expires:
            _add(errors, "accepted_offer.offer_expires_at", "offer has expired; obtain and accept a current revision")
        if authorized_at and authorization_expires and authorized_at >= authorization_expires:
            _add(errors, "authorization", "authorized_at must be strictly before authorization expiry")
        if authorized_at and accepted_at and authorized_at > accepted_at:
            _add(errors, "authorization.authorized_at", "authorization must precede or coincide with order acceptance")
        if authorized_at and authorized_at > current:
            _add(errors, "authorization.authorized_at", "cannot be in the future")
        if authorization_expires and current >= authorization_expires:
            _add(errors, "authorization.expires_at", "authorization has expired; a fresh mandate is required")

        if authorization["buyer_legal_entity_id"] != buyer["buyer_legal_entity_id"]:
            _add(errors, "authorization.buyer_legal_entity_id", "must match the buyer legal entity on this intent")
        accepted_total = _amount(offer["accepted_total"]["amount"])
        maximum_total = _amount(authorization["maximum_total"]["amount"])
        accepted_currency = offer["accepted_total"]["currency"]
        maximum_currency = authorization["maximum_total"]["currency"]
        if accepted_currency != maximum_currency:
            _add(errors, "authorization.maximum_total.currency", "must match accepted offer currency; no implicit FX conversion is permitted")
        if accepted_total is None or maximum_total is None:
            _add(errors, "accepted_offer.accepted_total", "amounts must be exact finite decimal strings")
        elif accepted_total > maximum_total:
            _add(errors, "accepted_offer.accepted_total", "accepted total exceeds the explicitly authorized maximum")
        if authorization["action"] != "place_order":
            _add(errors, "authorization.action", "only place_order is valid for a binding purchase intent")
        if not authorization["single_use"]:
            _add(errors, "authorization.single_use", "purchase authorization must be single-use")
        try:
            expected_scope_digest = purchase_scope_sha256(document)
        except (ValueError, UnicodeError, OverflowError):
            _add(
                errors,
                "authorization.scope_sha256",
                "purchase scope contains a value that cannot be represented by RFC 8785 JCS; no purchase authorization may be accepted",
            )
        else:
            if authorization["scope_sha256"].lower() != expected_scope_digest:
                _add(errors, "authorization.scope_sha256", "does not match the canonical purchase-scope projection; a changed quantity, unit, destination, buyer, consent scope, offer revision, accepted total, intent revision, or idempotency key requires fresh authorization")
        if accepted_at and consent_granted and accepted_at < consent_granted:
            warnings.append("order acceptance predates the current sharing-consent grant; verify which data was disclosed under which consent")

        if offer["offer_sha256"] == "":
            _add(errors, "accepted_offer.offer_sha256", "offer revision digest must be present")

    if state in {"draft", "non_binding_interest", "quote_request"}:
        if "accepted_offer" in document or "authorization" in document:
            _add(errors, "metadata.state", "non-binding states cannot carry a binding accepted offer or purchase authorization")
        if state == "non_binding_interest":
            warnings.append("non-binding interest only: this declaration does not create a purchase obligation")
        if state == "quote_request":
            warnings.append("quote request only: no purchase authority is implied")

    if state in {"cancelled", "expired"}:
        warnings.append(f"{state} intent is retained as a record and must not be reused as a live purchase request")

    result = "DECLARATION_VALID_NO_TRANSACTION_AUTHORIZED" if not errors else "INVALID_DECLARATION"
    return {
        "schema_id": schema.get("$id"),
        "intent_id": metadata["intent_id"],
        "revision": metadata["revision"],
        "state": state,
        "result": result,
        "structural_valid": True,
        "semantic_valid": not errors,
        "transaction_authorized": False,
        "errors": errors,
        "warnings": warnings,
        "notice": "This is a declaration check, not authentication or transaction authorization. It does not verify supplier/buyer identity, signatures, consent evidence, mandate validity in the source system, official code-list membership, local law, inventory, or idempotency against a durable order ledger.",
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("intent", type=Path, help="JSON purchase-intent declaration")
    parser.add_argument("--schema", type=Path, default=DEFAULT_SCHEMA_PATH, help="JSON Schema path")
    parser.add_argument("--as-of", help="timezone-aware ISO-8601 check time (default: current UTC)")
    args = parser.parse_args(argv)

    try:
        schema = load_json(args.schema)
        Draft202012Validator.check_schema(schema)
        document = load_json(args.intent)
        if args.as_of:
            now = _timestamp(args.as_of)
            if now is None:
                parser.error("--as-of must be a timezone-aware ISO-8601 timestamp")
        else:
            now = datetime.now(timezone.utc)
        report = validate_intent(document, schema=schema, now=now)
    except (OSError, UnicodeError, json.JSONDecodeError, DuplicateJsonKey, ValueError, rfc8785.CanonicalizationError) as exc:
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
