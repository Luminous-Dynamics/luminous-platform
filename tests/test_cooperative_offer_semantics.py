# Copyright (C) 2026 Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Fail-closed semantic tests for the proposed cooperative-offer validator."""

from __future__ import annotations

import copy
import json
import sys
import unittest
from datetime import datetime, timedelta, timezone
from pathlib import Path
from tempfile import TemporaryDirectory

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))

from validate_cooperative_offer import (  # noqa: E402
    DEFAULT_SCHEMA_PATH,
    DuplicateJsonKey,
    _load_json,
    validate_offer,
)

SCHEMA = json.loads(DEFAULT_SCHEMA_PATH.read_text(encoding="utf-8"))
FIXTURE_DIR = ROOT / "business-packs" / "cooperative-buying" / "fixtures"
ZAR = json.loads((FIXTURE_DIR / "example-offer-zar.json").read_text(encoding="utf-8"))
EUR = json.loads((FIXTURE_DIR / "example-offer-eur.json").read_text(encoding="utf-8"))
NOW = datetime(2026, 10, 9, 1, 0, 0, tzinfo=timezone.utc)


class CooperativeOfferSemanticTests(unittest.TestCase):
    def validate(self, document=None, *, now=NOW, max_hours=24):
        return validate_offer(
            copy.deepcopy(ZAR if document is None else document),
            schema=SCHEMA,
            now=now,
            max_availability_age=timedelta(hours=max_hours),
        )

    def make_publishable_candidate(self):
        document = copy.deepcopy(ZAR)
        document["metadata"]["fixture_only"] = False
        document["metadata"]["status"] = "published"
        document["evidence"]["verification_level"] = "supplier_attested"
        document["evidence"]["source_reference"] = "source:customer-approved-supplier-quote"
        document["evidence"]["content_sha256"] = "a" * 64
        document["terms"]["tax_treatment"] = "excluded"
        document["availability"].update({
            "status": "in_stock",
            "quantity": "100",
            "quantity_unit_code": "case",
            "observed_at": "2026-10-09T00:00:00Z",
        })
        return document

    def test_draft_fixtures_have_valid_declarations_but_no_authority(self):
        for document in (ZAR, EUR):
            with self.subTest(offer=document["metadata"]["offer_id"]):
                report = self.validate(document)
                self.assertTrue(report["structural_valid"])
                self.assertTrue(report["semantic_valid"], report["errors"])
                self.assertFalse(report["transaction_authorized"])
                self.assertEqual(report["result"], "DECLARATION_VALID_NO_TRANSACTION_AUTHORIZED")

    def test_complete_published_candidate_passes_declared_gates_without_authorizing_purchase(self):
        report = self.validate(self.make_publishable_candidate())
        self.assertTrue(report["semantic_valid"], report["errors"])
        self.assertTrue(report["structural_valid"])
        self.assertFalse(report["transaction_authorized"])

    def test_published_offer_with_unverified_source_is_rejected(self):
        document = self.make_publishable_candidate()
        document["evidence"]["verification_level"] = "unverified"
        report = self.validate(document)
        self.assertFalse(report["semantic_valid"])
        self.assertTrue(any("supplier-attested" in error for error in report["errors"]))

    def test_published_offer_without_retained_source_digest_is_rejected(self):
        document = self.make_publishable_candidate()
        del document["evidence"]["content_sha256"]
        # Missing required field is a structural error and therefore cannot reach semantic pass.
        report = self.validate(document)
        self.assertFalse(report["semantic_valid"])
        self.assertFalse(report["structural_valid"])

    def test_published_offer_with_unknown_tax_treatment_is_rejected(self):
        document = self.make_publishable_candidate()
        document["terms"]["tax_treatment"] = "unknown"
        report = self.validate(document)
        self.assertFalse(report["semantic_valid"])
        self.assertTrue(any("tax-treatment" in error for error in report["errors"]))

    def test_expired_published_offer_is_rejected(self):
        document = self.make_publishable_candidate()
        report = self.validate(document, now=datetime(2027, 1, 1, tzinfo=timezone.utc))
        self.assertFalse(report["semantic_valid"])
        self.assertTrue(any("validity window" in error for error in report["errors"]))

    def test_stale_availability_is_rejected_for_published_offer(self):
        document = self.make_publishable_candidate()
        report = self.validate(document, now=datetime(2026, 10, 11, tzinfo=timezone.utc), max_hours=24)
        self.assertFalse(report["semantic_valid"])
        self.assertTrue(any("stale" in error for error in report["errors"]))

    def test_future_availability_observation_is_rejected(self):
        document = copy.deepcopy(ZAR)
        document["availability"]["observed_at"] = "2026-10-10T00:00:00Z"
        report = self.validate(document, now=NOW)
        self.assertFalse(report["semantic_valid"])
        self.assertTrue(any("must not be in the future" in error for error in report["errors"]))

    def test_created_at_must_not_follow_valid_from(self):
        document = copy.deepcopy(ZAR)
        document["validity"]["created_at"] = "2026-10-10T00:00:00Z"
        report = self.validate(document)
        self.assertFalse(report["semantic_valid"])
        self.assertTrue(any("created_at must be at or before valid_from" in error for error in report["errors"]))

    def test_expiry_must_follow_valid_from(self):
        document = copy.deepcopy(ZAR)
        document["validity"]["expires_at"] = "2026-10-09T00:00:00Z"
        report = self.validate(document)
        self.assertFalse(report["semantic_valid"])
        self.assertTrue(any("strictly before expires_at" in error for error in report["errors"]))

    def test_price_break_currency_must_match_base_unit_price_currency(self):
        document = copy.deepcopy(ZAR)
        document["terms"]["price_breaks"][0]["currency"] = "EUR"
        report = self.validate(document)
        self.assertFalse(report["semantic_valid"])
        self.assertTrue(any("must match the base unit-price currency" in error for error in report["errors"]))

    def test_price_unit_must_match_order_unit(self):
        document = copy.deepcopy(ZAR)
        document["terms"]["unit_price"]["per_unit_code"] = "item"
        report = self.validate(document)
        self.assertFalse(report["semantic_valid"])
        self.assertTrue(any("must match product.pack.order_unit_code" in error for error in report["errors"]))

    def test_price_break_thresholds_must_increase_and_respect_minimum_order(self):
        document = copy.deepcopy(ZAR)
        document["terms"]["price_breaks"] = [
            {"minimum_quantity": "10", "unit_price_amount": "235.00", "currency": "ZAR"},
            {"minimum_quantity": "9", "unit_price_amount": "240.00", "currency": "ZAR"},
        ]
        report = self.validate(document)
        self.assertFalse(report["semantic_valid"])
        self.assertTrue(any("strictly increasing" in error for error in report["errors"]))

    def test_stock_quantity_must_be_known_positive_and_match_order_unit_for_publish(self):
        document = self.make_publishable_candidate()
        document["availability"]["quantity"] = "1"
        document["availability"]["quantity_unit_code"] = "item"
        document["terms"]["minimum_order_quantity"] = "2"
        report = self.validate(document)
        self.assertFalse(report["semantic_valid"])
        self.assertTrue(any("must match product.pack.order_unit_code" in error for error in report["errors"]))

    def test_lead_and_delivery_ranges_must_not_be_reversed(self):
        document = copy.deepcopy(ZAR)
        document["availability"]["lead_time_min"] = 8
        document["availability"]["lead_time_max"] = 2
        document["fulfillment"]["estimated_delivery_days_min"] = 12
        document["fulfillment"]["estimated_delivery_days_max"] = 3
        report = self.validate(document)
        self.assertFalse(report["semantic_valid"])
        self.assertTrue(any("lead_time_min" in error for error in report["errors"]))
        self.assertTrue(any("estimated_delivery_days_min" in error for error in report["errors"]))

    def test_cross_border_offer_requires_cross_border_fulfillment(self):
        document = copy.deepcopy(EUR)
        document["fulfillment"]["cross_border_supported"] = False
        report = self.validate(document)
        self.assertFalse(report["semantic_valid"])
        self.assertTrue(any("must be true for cross-border" in error for error in report["errors"]))

    def test_duplicate_json_keys_are_rejected(self):
        with TemporaryDirectory() as temporary_directory:
            source = Path(temporary_directory) / "duplicate-offer-key-fixture.json"
            source.write_text('{"schema":"first","schema":"second"}', encoding="utf-8")
            with self.assertRaises(DuplicateJsonKey):
                _load_json(source)

    def test_unknown_or_unverified_third_party_code_lists_are_not_claimed_as_validated(self):
        report = self.validate(ZAR)
        self.assertIn("official code-list membership", report["notice"])
        self.assertFalse(report["transaction_authorized"])


if __name__ == "__main__":
    unittest.main()
