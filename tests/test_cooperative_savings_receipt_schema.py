# Copyright (C) 2026 Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Structural and arithmetic consistency checks for portable savings receipts."""

from __future__ import annotations

import copy
import json
import unittest
from decimal import Decimal
from pathlib import Path

from jsonschema import Draft202012Validator, FormatChecker

ROOT = Path(__file__).resolve().parents[1]
SCHEMA = json.loads(
    (ROOT / "schemas" / "cooperative-savings-receipt-v1.schema.json").read_text(encoding="utf-8")
)
FIXTURE = json.loads(
    (ROOT / "business-packs" / "cooperative-buying" / "fixtures" / "example-savings-receipt.json").read_text(encoding="utf-8")
)
VALIDATOR = Draft202012Validator(SCHEMA, format_checker=FormatChecker())


class CooperativeSavingsReceiptTests(unittest.TestCase):
    def errors(self, document: dict) -> list:
        return sorted(
            VALIDATOR.iter_errors(document),
            key=lambda error: (list(map(str, error.absolute_path)), error.message),
        )

    def test_schema_is_valid_draft_2020_12(self):
        Draft202012Validator.check_schema(SCHEMA)
        self.assertEqual(SCHEMA["$id"], "urn:luminous:cooperative-savings-receipt:v1")

    def test_illustrative_receipt_is_structurally_valid(self):
        errors = self.errors(FIXTURE)
        self.assertEqual(errors, [], [error.message for error in errors])
        self.assertTrue(FIXTURE["metadata"]["fixture_only"])
        self.assertEqual(FIXTURE["metadata"]["status"], "illustrative")
        self.assertFalse(FIXTURE["claim"]["evidence_authenticated_by_calculator"])

    def test_mixed_baseline_claim_class_is_supported_explicitly(self):
        self.assertIn("MixedBaselineEvidence", SCHEMA["properties"]["claim"]["properties"]["class"]["enum"])

    def test_receipt_arithmetic_is_exact_and_self_consistent(self):
        calculation = FIXTURE["calculation"]
        baseline = Decimal(calculation["baseline_merchandise"]) + Decimal(calculation["baseline_other_costs"])
        actual = (
            Decimal(calculation["actual_merchandise"])
            + Decimal(calculation["actual_other_costs"])
            + Decimal(calculation["participation_costs"])
        )
        self.assertEqual(baseline, Decimal(calculation["baseline_total"]))
        self.assertEqual(actual, Decimal(calculation["actual_total"]))
        self.assertEqual(baseline - actual, Decimal(calculation["net_difference"]))
        self.assertEqual(calculation["net_difference"], "120")

    def test_fixture_cannot_be_promoted_to_computed_status(self):
        document = copy.deepcopy(FIXTURE)
        document["metadata"]["status"] = "computed"
        errors = self.errors(document)
        self.assertTrue(errors)
        self.assertTrue(any("illustrative" in error.message for error in errors))

    def test_computed_receipt_requires_source_and_implementation_revision(self):
        document = copy.deepcopy(FIXTURE)
        document["metadata"]["fixture_only"] = False
        document["metadata"]["status"] = "computed"
        errors = self.errors(document)
        self.assertTrue(errors)
        self.assertTrue(any("source_revision" in error.message for error in errors))
        self.assertTrue(any("implementation_digest_sha256" in error.message for error in errors))

    def test_binary_float_is_not_accepted_for_money(self):
        document = copy.deepcopy(FIXTURE)
        document["calculation"]["net_difference"] = 120.0
        errors = self.errors(document)
        self.assertTrue(errors)
        self.assertTrue(
            any(list(error.absolute_path) == ["calculation", "net_difference"] for error in errors),
            [error.message for error in errors],
        )

    def test_evidence_authenticated_flag_cannot_be_claimed_by_this_calculator(self):
        document = copy.deepcopy(FIXTURE)
        document["claim"]["evidence_authenticated_by_calculator"] = True
        errors = self.errors(document)
        self.assertTrue(errors)

    def test_unknown_fields_fail_closed(self):
        document = copy.deepcopy(FIXTURE)
        document["claim"]["hidden_commission"] = "0"
        errors = self.errors(document)
        self.assertTrue(errors)
        self.assertTrue(any("Additional properties" in error.message for error in errors))


if __name__ == "__main__":
    unittest.main()
