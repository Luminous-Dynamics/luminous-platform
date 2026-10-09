# Copyright (C) 2026 Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Semantic regression tests for purchase-intent authority and time bounds."""

from __future__ import annotations

import copy
import hashlib
import json
import sys
import unittest
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))

import rfc8785  # noqa: E402

from validate_cooperative_purchase_intent import (  # noqa: E402
    purchase_scope_payload,
    purchase_scope_sha256,
    validate_intent,
)

SCHEMA = json.loads(
    (ROOT / "schemas" / "cooperative-purchase-intent-v1.schema.json").read_text(encoding="utf-8")
)
FIXTURE = json.loads(
    (ROOT / "business-packs" / "cooperative-buying" / "fixtures" / "example-nonbinding-interest.json").read_text(encoding="utf-8")
)
NOW = datetime(2026, 10, 9, 0, 45, tzinfo=timezone.utc)


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
    document["authorization"]["scope_sha256"] = purchase_scope_sha256(document)
    return document


class CooperativePurchaseIntentSemanticTests(unittest.TestCase):
    def validate(self, document=None, *, now=NOW):
        return validate_intent(copy.deepcopy(FIXTURE if document is None else document), schema=SCHEMA, now=now)

    def test_nonbinding_interest_passes_without_transaction_authority(self):
        report = self.validate()
        self.assertTrue(report["structural_valid"])
        self.assertTrue(report["semantic_valid"], report["errors"])
        self.assertFalse(report["transaction_authorized"])
        self.assertEqual(report["result"], "DECLARATION_VALID_NO_TRANSACTION_AUTHORIZED")

    def test_exact_binding_order_with_unexpired_scoped_mandate_passes_declaration_checks(self):
        report = self.validate(binding_order_candidate())
        self.assertTrue(report["semantic_valid"], report["errors"])
        self.assertFalse(report["transaction_authorized"])

    def test_purchase_scope_uses_rfc8785_canonicalization(self):
        document = binding_order_candidate()
        payload = purchase_scope_payload(document)
        reordered_payload = dict(reversed(list(payload.items())))
        self.assertEqual(rfc8785.dumps(payload), rfc8785.dumps(reordered_payload))
        self.assertEqual(rfc8785.dumps({"b": 2, "a": 1}), b'{"a":1,"b":2}')
        self.assertEqual(purchase_scope_sha256(document), hashlib.sha256(rfc8785.dumps(payload)).hexdigest())

    def test_changed_quantity_invalidates_previously_authorized_scope_digest(self):
        document = binding_order_candidate()
        document["demand"]["requested_quantity"] = "11"
        report = self.validate(document)
        self.assertFalse(report["semantic_valid"])
        self.assertTrue(any("canonical purchase-scope projection" in error for error in report["errors"]))

    def test_changed_authorized_maximum_invalidates_scope_digest(self):
        document = binding_order_candidate()
        document["authorization"]["maximum_total"]["amount"] = "9000.00"
        report = self.validate(document)
        self.assertFalse(report["semantic_valid"])
        self.assertTrue(any("canonical purchase-scope projection" in error for error in report["errors"]))

    def test_accepted_total_cannot_exceed_authorized_maximum(self):
        document = binding_order_candidate()
        document["accepted_offer"]["accepted_total"]["amount"] = "3000.01"
        report = self.validate(document)
        self.assertFalse(report["semantic_valid"])
        self.assertTrue(any("exceeds the explicitly authorized maximum" in error for error in report["errors"]))

    def test_authorization_currency_must_match_accepted_offer(self):
        document = binding_order_candidate()
        document["authorization"]["maximum_total"]["currency"] = "EUR"
        report = self.validate(document)
        self.assertFalse(report["semantic_valid"])
        self.assertTrue(any("must match accepted offer currency" in error for error in report["errors"]))

    def test_authorized_legal_entity_must_match_buyer(self):
        document = binding_order_candidate()
        document["authorization"]["buyer_legal_entity_id"] = "fixture:different-entity"
        report = self.validate(document)
        self.assertFalse(report["semantic_valid"])
        self.assertTrue(any("must match the buyer legal entity" in error for error in report["errors"]))

    def test_expired_authorization_is_rejected(self):
        report = self.validate(binding_order_candidate(), now=datetime(2026, 10, 9, 2, 0, tzinfo=timezone.utc))
        self.assertFalse(report["semantic_valid"])
        self.assertTrue(any("authorization has expired" in error for error in report["errors"]))

    def test_expired_offer_revision_is_rejected(self):
        report = self.validate(binding_order_candidate(), now=datetime(2026, 10, 11, 0, 0, tzinfo=timezone.utc))
        self.assertFalse(report["semantic_valid"])
        self.assertTrue(any("offer has expired" in error for error in report["errors"]))

    def test_reversed_delivery_window_is_rejected(self):
        document = copy.deepcopy(FIXTURE)
        document["demand"]["delivery_window_start"] = "2026-10-22T00:00:00Z"
        document["demand"]["delivery_window_end"] = "2026-10-15T00:00:00Z"
        report = self.validate(document)
        self.assertFalse(report["semantic_valid"])
        self.assertTrue(any("delivery_window_start" in error for error in report["errors"]))

    def test_expired_consent_is_rejected_for_active_intent(self):
        document = copy.deepcopy(FIXTURE)
        document["sharing"]["consent"]["expires_at"] = "2026-10-09T00:30:00Z"
        report = self.validate(document)
        self.assertFalse(report["semantic_valid"])
        self.assertTrue(any("expired sharing consent" in error for error in report["errors"]))

    def test_active_intent_cannot_be_reused_after_expiry(self):
        report = self.validate(FIXTURE, now=datetime(2027, 1, 1, tzinfo=timezone.utc))
        self.assertFalse(report["semantic_valid"])
        self.assertTrue(any("active intent is expired" in error for error in report["errors"]))

    def test_naive_as_of_timestamp_is_rejected(self):
        with self.assertRaises(ValueError):
            validate_intent(FIXTURE, schema=SCHEMA, now=datetime(2026, 10, 9, 0, 45))


if __name__ == "__main__":
    unittest.main()
