# Copyright (C) 2026 Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Structural regression tests for the proposed global cooperative-offer contract."""

from __future__ import annotations

import copy
import json
import unittest
from pathlib import Path

from jsonschema import Draft202012Validator, FormatChecker

ROOT = Path(__file__).resolve().parents[1]
SCHEMA_PATH = ROOT / "schemas" / "cooperative-offer-v1.schema.json"
FIXTURE_DIR = ROOT / "business-packs" / "cooperative-buying" / "fixtures"
SCHEMA = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
VALIDATOR = Draft202012Validator(SCHEMA, format_checker=FormatChecker())
FIXTURE_PATHS = sorted(FIXTURE_DIR.glob("example-offer-*.json"))


class CooperativeOfferSchemaTests(unittest.TestCase):
    def load_fixture(self, path: Path) -> dict:
        return json.loads(path.read_text(encoding="utf-8"))

    def errors(self, document: dict) -> list:
        return sorted(
            VALIDATOR.iter_errors(document),
            key=lambda err: (list(map(str, err.absolute_path)), err.message),
        )

    def test_schema_is_valid_draft_2020_12(self):
        Draft202012Validator.check_schema(SCHEMA)
        self.assertEqual(SCHEMA["$id"], "urn:luminous:cooperative-offer:v1")

    def test_synthetic_fixtures_are_structurally_valid_drafts(self):
        self.assertGreaterEqual(len(FIXTURE_PATHS), 2)
        for path in FIXTURE_PATHS:
            with self.subTest(fixture=path.name):
                document = self.load_fixture(path)
                self.assertEqual(self.errors(document), [])
                self.assertTrue(document["metadata"]["fixture_only"])
                self.assertEqual(document["metadata"]["status"], "draft")

    def test_global_shape_is_not_hardcoded_to_one_currency_or_country(self):
        documents = [self.load_fixture(path) for path in FIXTURE_PATHS]
        currencies = {item["terms"]["unit_price"]["currency"] for item in documents}
        countries = {item["supplier"]["seller_country_code"] for item in documents}
        self.assertTrue({"ZAR", "EUR"}.issubset(currencies))
        self.assertTrue({"ZA", "DE"}.issubset(countries))
        # Codes are syntactically checked here; this test does not query live ISO code lists.
        for document in documents:
            self.assertEqual(self.errors(document), [])

    def test_unit_code_systems_are_required_for_global_comparability(self):
        document = self.load_fixture(FIXTURE_PATHS[0])
        del document["product"]["pack"]["order_unit_code_system"]
        errors = self.errors(document)
        self.assertTrue(errors)
        self.assertTrue(any("order_unit_code_system" in error.message for error in errors))

    def test_binary_floating_point_money_amount_is_rejected(self):
        document = self.load_fixture(FIXTURE_PATHS[0])
        document["terms"]["unit_price"]["amount"] = 250.0
        errors = self.errors(document)
        self.assertTrue(errors)
        self.assertTrue(
            any(list(error.absolute_path) == ["terms", "unit_price", "amount"] for error in errors),
            [error.message for error in errors],
        )

    def test_currency_code_must_have_uppercase_three_letter_shape(self):
        document = self.load_fixture(FIXTURE_PATHS[0])
        document["terms"]["unit_price"]["currency"] = "Zar"
        errors = self.errors(document)
        self.assertTrue(errors)
        self.assertTrue(any("currency" in error.message.lower() or "does not match" in error.message for error in errors))

    def test_unknown_cost_is_not_encoded_as_zero(self):
        document = self.load_fixture(FIXTURE_PATHS[0])
        freight = next(charge for charge in document["terms"]["charges"] if charge["kind"] == "freight")
        self.assertEqual(freight["value"], "unknown")
        self.assertNotEqual(freight["value"], "0")

    def test_synthetic_fixture_cannot_be_marked_published(self):
        document = self.load_fixture(FIXTURE_PATHS[0])
        document["metadata"]["status"] = "published"
        errors = self.errors(document)
        self.assertTrue(errors)
        self.assertTrue(
            any(list(error.absolute_path) == ["metadata", "status"] for error in errors),
            [error.message for error in errors],
        )

    def test_unknown_fields_fail_closed(self):
        document = self.load_fixture(FIXTURE_PATHS[0])
        document["hidden_platform_fee"] = "0"
        errors = self.errors(document)
        self.assertTrue(errors)
        self.assertTrue(any("Additional properties" in error.message for error in errors))

    def test_tax_treatment_cannot_be_omitted(self):
        document = self.load_fixture(FIXTURE_PATHS[0])
        del document["terms"]["tax_treatment"]
        errors = self.errors(document)
        self.assertTrue(errors)
        self.assertTrue(any("tax_treatment" in error.message for error in errors))

    def test_cross_border_fixture_references_more_than_one_local_profile(self):
        document = next(
            self.load_fixture(path)
            for path in FIXTURE_PATHS
            if self.load_fixture(path)["jurisdiction"]["transaction_mode"] == "cross_border"
        )
        self.assertTrue(document["fulfillment"]["cross_border_supported"])
        self.assertGreaterEqual(len(document["jurisdiction"]["required_profile_refs"]), 2)
        # A profile reference is not proof that the profile is reviewed or active.
        self.assertTrue(all("proposed" in ref for ref in document["jurisdiction"]["required_profile_refs"]))


if __name__ == "__main__":
    unittest.main()
