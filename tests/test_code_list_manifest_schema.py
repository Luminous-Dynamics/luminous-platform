# Copyright (C) 2026 Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Schema tests for global code-list provenance and stale-registry boundaries."""

from __future__ import annotations

import copy
import json
import unittest
from pathlib import Path

from jsonschema import Draft202012Validator, FormatChecker

ROOT = Path(__file__).resolve().parents[1]
SCHEMA = json.loads(
    (ROOT / "schemas" / "code-list-snapshot-manifest-v1.schema.json").read_text(encoding="utf-8")
)
FIXTURE = json.loads(
    (ROOT / "business-packs" / "cooperative-buying" / "fixtures" / "example-code-list-manifest.json").read_text(encoding="utf-8")
)
VALIDATOR = Draft202012Validator(SCHEMA, format_checker=FormatChecker())


class CodeListManifestTests(unittest.TestCase):
    def errors(self, document: dict) -> list:
        return sorted(
            VALIDATOR.iter_errors(document),
            key=lambda error: (list(map(str, error.absolute_path)), error.message),
        )

    def test_manifest_schema_is_valid_draft_2020_12(self):
        Draft202012Validator.check_schema(SCHEMA)
        self.assertEqual(SCHEMA["$id"], "urn:luminous:code-list-snapshot-manifest:v1")

    def test_illustrative_manifest_is_valid_but_not_retrieved(self):
        errors = self.errors(FIXTURE)
        self.assertEqual(errors, [], [error.message for error in errors])
        self.assertTrue(FIXTURE["metadata"]["fixture_only"])
        self.assertEqual(FIXTURE["metadata"]["status"], "illustrative")
        self.assertEqual(FIXTURE["source"]["retrieval_status"], "not_retrieved")
        self.assertEqual(FIXTURE["validation"]["status"], "not_run")
        self.assertNotIn("source_payload_sha256", FIXTURE["source"])
        self.assertNotIn("normalized_payload_sha256", FIXTURE["snapshot"])

    def test_fixture_only_manifest_cannot_be_activated(self):
        document = copy.deepcopy(FIXTURE)
        document["metadata"]["status"] = "active"
        errors = self.errors(document)
        self.assertTrue(errors)
        self.assertTrue(any("illustrative" in error.message for error in errors))

    def test_active_manifest_requires_retained_source_and_normalized_payload_digests(self):
        document = copy.deepcopy(FIXTURE)
        document["metadata"]["fixture_only"] = False
        document["metadata"]["status"] = "active"
        errors = self.errors(document)
        self.assertTrue(errors)
        self.assertTrue(any("source_payload_sha256" in error.message for error in errors))
        self.assertTrue(any("normalized_payload_sha256" in error.message for error in errors))
        self.assertTrue(any("review_report_sha256" in error.message for error in errors))

    def test_active_manifest_rejects_duplicate_codes(self):
        document = copy.deepcopy(FIXTURE)
        document["metadata"]["fixture_only"] = False
        document["metadata"]["status"] = "active"
        document["source"].update({
            "retrieved_at": "2026-10-09T00:00:00Z",
            "retrieval_status": "verified_against_authority",
            "source_payload_sha256": "a" * 64,
            "retained_source_reference": "fixture:source-payload"
        })
        document["snapshot"].update({
            "record_count": 10,
            "duplicate_code_count": 1,
            "normalized_payload_sha256": "b" * 64,
            "retained_payload_reference": "fixture:normalized-payload",
            "build_or_source_digest_sha256": "c" * 64
        })
        document["validation"].update({
            "status": "passed",
            "reviewer_id": "fixture:reviewer",
            "review_report_sha256": "d" * 64,
            "review_report_reference": "fixture:review-report",
            "checks": ["source_digest_checked", "source_authority_checked", "schema_validated", "duplicate_codes_checked", "effective_dates_checked"]
        })
        errors = self.errors(document)
        self.assertTrue(errors)
        self.assertTrue(any(list(error.absolute_path) == ["snapshot", "duplicate_code_count"] for error in errors))

    def test_code_list_domain_is_not_hardcoded_to_currency(self):
        document = copy.deepcopy(FIXTURE)
        document["code_list"]["domain"] = "unit_of_measure"
        errors = self.errors(document)
        self.assertEqual(errors, [], [error.message for error in errors])
        document["code_list"]["domain"] = "customer_price"
        errors = self.errors(document)
        self.assertTrue(errors)


if __name__ == "__main__":
    unittest.main()
