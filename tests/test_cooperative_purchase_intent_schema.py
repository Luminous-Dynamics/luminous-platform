# Copyright (C) 2026 Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Structural tests for non-binding demand and authority-bound purchase intent."""

from __future__ import annotations

import copy
import json
import unittest
from pathlib import Path

from jsonschema import Draft202012Validator, FormatChecker

ROOT = Path(__file__).resolve().parents[1]
SCHEMA = json.loads(
    (ROOT / "schemas" / "cooperative-purchase-intent-v1.schema.json").read_text(encoding="utf-8")
)
FIXTURE = json.loads(
    (ROOT / "business-packs" / "cooperative-buying" / "fixtures" / "example-nonbinding-interest.json").read_text(encoding="utf-8")
)
VALIDATOR = Draft202012Validator(SCHEMA, format_checker=FormatChecker())


def binding_order_candidate() -> dict:
    document = copy.deepcopy(FIXTURE)
    document["metadata"]["state"] = "binding_order"
    document["metadata"]["idempotency_key"] = "fixture-once-only-key-000001"
    document["accepted_offer"] = {
        "offer_id": "fixture:offer:001",
        "revision": 1,
        "offer_sha256": "b" * 64,
        "accepted_total": {"amount": "2500.00", "currency": "ZAR"},
        "accepted_at": "2026-10-09T00:30:00Z",
        "offer_expires_at": "2026-10-10T00:00:00Z",
        "commercial_terms_reference": "fixture:commercial-terms:001"
    }
    document["authorization"] = {
        "authority_id": "fixture:mandate:001",
        "actor_party_id": "fixture:buyer-actor:001",
        "buyer_legal_entity_id": "fixture:buyer-entity:001",
        "action": "place_order",
        "scope_sha256": "c" * 64,
        "maximum_total": {"amount": "3000.00", "currency": "ZAR"},
        "authorized_at": "2026-10-09T00:20:00Z",
        "expires_at": "2026-10-09T01:00:00Z",
        "evidence_reference": "fixture:authorization-evidence:001",
        "evidence_sha256": "d" * 64,
        "signature_reference": "fixture:signature:001",
        "single_use": True
    }
    return document


class CooperativePurchaseIntentSchemaTests(unittest.TestCase):
    def errors(self, document: dict) -> list:
        return sorted(
            VALIDATOR.iter_errors(document),
            key=lambda err: (list(map(str, err.absolute_path)), err.message),
        )

    def test_schema_is_valid_draft_2020_12(self):
        Draft202012Validator.check_schema(SCHEMA)
        self.assertEqual(SCHEMA["$id"], "urn:luminous:cooperative-purchase-intent:v1")

    def test_synthetic_nonbinding_interest_fixture_is_valid(self):
        errors = self.errors(FIXTURE)
        self.assertEqual(errors, [ ] if not errors else errors)
        self.assertEqual(FIXTURE["metadata"]["state"], "non_binding_interest")
        self.assertNotIn("accepted_offer", FIXTURE)
        self.assertNotIn("authorization", FIXTURE)

    def test_binding_order_requires_accepted_offer_authority_and_idempotency(self):
        document = copy.deepcopy(FIXTURE)
        document["metadata"]["state"] = "binding_order"
        errors = self.errors(document)
        self.assertTrue(errors)
        self.assertTrue(any("accepted_offer" in error.message for error in errors))
        self.assertTrue(any("authorization" in error.message for error in errors))
        self.assertTrue(any("idempotency_key" in error.message for error in errors))

    def test_complete_binding_declaration_has_exact_order_authority_fields(self):
        document = binding_order_candidate()
        errors = self.errors(document)
        self.assertEqual(errors, [], [error.message for error in errors])

    def test_nonbinding_interest_cannot_carry_an_accepted_offer(self):
        document = copy.deepcopy(FIXTURE)
        document["accepted_offer"] = binding_order_candidate()["accepted_offer"]
        errors = self.errors(document)
        self.assertTrue(errors)
        self.assertTrue(any("not" in error.message.lower() or "accepted_offer" in error.message for error in errors))

    def test_quote_request_cannot_carry_purchase_authority(self):
        document = copy.deepcopy(FIXTURE)
        document["metadata"]["state"] = "quote_request"
        document["authorization"] = binding_order_candidate()["authorization"]
        errors = self.errors(document)
        self.assertTrue(errors)
        self.assertTrue(any("not" in error.message.lower() or "authorization" in error.message for error in errors))

    def test_purchase_authorization_cannot_authorize_an_unrelated_action(self):
        document = binding_order_candidate()
        document["authorization"]["action"] = "approve_price_change"
        errors = self.errors(document)
        self.assertTrue(errors)
        self.assertTrue(any("place_order" in error.message or "const" in error.message for error in errors))

    def test_consent_evidence_and_revocability_are_required(self):
        document = copy.deepcopy(FIXTURE)
        del document["sharing"]["consent"]["evidence_sha256"]
        document["sharing"]["consent"]["revocable"] = False
        errors = self.errors(document)
        self.assertTrue(errors)
        self.assertTrue(any("evidence_sha256" in error.message for error in errors))
        self.assertTrue(any("true" in error.message for error in errors))

    def test_money_and_quantity_use_decimal_strings(self):
        document = binding_order_candidate()
        document["demand"]["requested_quantity"] = 10.0
        document["accepted_offer"]["accepted_total"]["amount"] = 2500.0
        errors = self.errors(document)
        self.assertTrue(errors)
        self.assertTrue(any("requested_quantity" in error.message or "does not match" in error.message for error in errors))


if __name__ == "__main__":
    unittest.main()
