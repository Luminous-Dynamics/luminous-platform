# Copyright (C) 2026 Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Semantic checks for code-list activation, freshness, and review metadata."""

from __future__ import annotations

import copy
import json
import sys
import unittest
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))

from validate_code_list_manifest import validate_manifest  # noqa: E402

SCHEMA = json.loads(
    (ROOT / "schemas" / "code-list-snapshot-manifest-v1.schema.json").read_text(encoding="utf-8")
)
FIXTURE = json.loads(
    (ROOT / "business-packs" / "cooperative-buying" / "fixtures" / "example-code-list-manifest.json").read_text(encoding="utf-8")
)
NOW = datetime(2026, 10, 9, 0, 30, tzinfo=timezone.utc)


def active_candidate() -> dict:
    document = copy.deepcopy(FIXTURE)
    document["metadata"]["status"] = "active"
    document["metadata"]["fixture_only"] = False
    document["source"].update({
        "retrieval_status": "verified_against_authority",
        "retrieved_at": "2026-10-09T00:00:00Z",
        "source_payload_sha256": "a" * 64,
        "retained_source_reference": "fixture:source-payload"
    })
    document["snapshot"].update({
        "record_count": 10,
        "duplicate_code_count": 0,
        "normalized_payload_sha256": "b" * 64,
        "retained_payload_reference": "fixture:normalized-payload",
        "build_or_source_digest_sha256": "c" * 64
    })
    document["validation"].update({
        "status": "passed",
        "checks": [
            "source_digest_checked",
            "schema_validated",
            "duplicate_codes_checked",
            "effective_dates_checked"
        ],
        "reviewed_at": "2026-10-09T00:10:00Z",
        "reviewer_id": "fixture:reviewer",
        "review_report_sha256": "d" * 64,
        "review_report_reference": "fixture:review-report"
    })
    return document


class CodeListManifestSemanticTests(unittest.TestCase):
    def validate(self, document=None, *, now=NOW):
        return validate_manifest(
            copy.deepcopy(FIXTURE if document is None else document),
            schema=SCHEMA,
            now=now,
        )

    def test_unretrieved_fixture_is_not_eligible_for_activation(self):
        report = self.validate()
        self.assertTrue(report["structural_valid"])
        self.assertTrue(report["semantic_valid"], report["errors"])
        self.assertFalse(report["activation_eligible"])
        self.assertEqual(report["result"], "DECLARATION_VALID_NOT_ACTIVE")

    def test_reviewed_current_manifest_declaration_meets_metadata_gates(self):
        report = self.validate(active_candidate())
        self.assertTrue(report["structural_valid"])
        self.assertTrue(report["semantic_valid"], report["errors"])
        self.assertTrue(report["activation_eligible"])
        self.assertEqual(report["result"], "ACTIVE_DECLARATION_ELIGIBLE_FOR_RUNTIME_VERIFICATION")

    def test_active_manifest_past_review_deadline_is_rejected(self):
        document = active_candidate()
        report = self.validate(document, now=datetime(2026, 10, 10, 0, 0, tzinfo=timezone.utc))
        self.assertFalse(report["semantic_valid"])
        self.assertFalse(report["activation_eligible"])
        self.assertTrue(any("review deadline has passed" in error for error in report["errors"]))

    def test_active_manifest_without_authority_checked_source_is_rejected(self):
        document = active_candidate()
        document["source"]["retrieval_status"] = "retrieved"
        report = self.validate(document)
        self.assertFalse(report["semantic_valid"])
        self.assertFalse(report["activation_eligible"])
        self.assertTrue(any("source-authority verification" in error for error in report["errors"]))

    def test_active_manifest_with_future_review_timestamp_is_rejected(self):
        document = active_candidate()
        document["validation"]["reviewed_at"] = "2026-10-10T00:00:00Z"
        report = self.validate(document)
        self.assertFalse(report["semantic_valid"])
        self.assertTrue(any("review time must not be in the future" in error for error in report["errors"]))

    def test_active_manifest_outside_effectivity_window_is_rejected(self):
        document = active_candidate()
        document["snapshot"]["effective_until"] = "2026-10-09T00:20:00Z"
        report = self.validate(document)
        self.assertFalse(report["semantic_valid"])
        self.assertTrue(any("registry is no longer effective" in error for error in report["errors"]))

    def test_nonactive_manifest_never_claims_activation_eligibility(self):
        document = active_candidate()
        document["metadata"]["status"] = "stale"
        report = self.validate(document)
        self.assertTrue(report["semantic_valid"], report["errors"])
        self.assertFalse(report["activation_eligible"])


if __name__ == "__main__":
    unittest.main()
