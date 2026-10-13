"""Adversarial tests for the deterministic jurisdiction-pack validator."""
from __future__ import annotations

import copy
import json
import unittest
from datetime import datetime, timezone
from pathlib import Path

from validate_pack import validate_pack

ROOT = Path(__file__).resolve().parents[3]
SCHEMA_PATH = ROOT / "schemas" / "jurisdiction-pack-v0.1.0.schema.json"
PACK_PATH = ROOT / "examples" / "jurisdiction-packs" / "za-city-of-johannesburg-food-service-0.1.0.json"
SCHEMA_BYTES = SCHEMA_PATH.read_bytes()
PACK_BYTES = PACK_PATH.read_bytes()
SCHEMA = json.loads(SCHEMA_BYTES)
PACK = json.loads(PACK_BYTES)
AS_OF = datetime(2026, 10, 13, 15, 0, tzinfo=timezone.utc)


def report_for(pack):
    raw = json.dumps(pack, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")
    return validate_pack(SCHEMA, pack, schema_bytes=SCHEMA_BYTES, pack_bytes=raw, as_of=AS_OF)


def check_status(report, check_id):
    return next(check["status"] for check in report["checks"] if check["check_id"] == check_id)


class JurisdictionPackValidatorTests(unittest.TestCase):
    def test_reference_pack_is_valid_but_not_publishable(self):
        report = report_for(copy.deepcopy(PACK))
        self.assertTrue(report["validation_passed"])
        self.assertFalse(report["publication_eligible"])
        self.assertEqual(check_status(report, "publication.gate"), "BLOCKED")
        self.assertTrue(report["publication_blockers"])
        self.assertEqual(report["pack_id"], PACK["pack_id"])

    def test_missing_source_reference_fails(self):
        pack = copy.deepcopy(PACK)
        pack["requirements"][0]["source_refs"] = ["missing.source"]
        report = report_for(pack)
        self.assertFalse(report["validation_passed"])
        self.assertEqual(check_status(report, "pack.source_references"), "FAIL")

    def test_duplicate_source_ids_fail(self):
        pack = copy.deepcopy(PACK)
        pack["sources"][1]["source_id"] = pack["sources"][0]["source_id"]
        report = report_for(pack)
        self.assertFalse(report["validation_passed"])
        self.assertEqual(check_status(report, "pack.unique_identifiers"), "FAIL")

    def test_out_of_scope_business_type_fails(self):
        pack = copy.deepcopy(PACK)
        pack["requirements"][0]["applicability"]["business_types"].append("unknown_business")
        report = report_for(pack)
        self.assertFalse(report["validation_passed"])
        self.assertEqual(check_status(report, "pack.applicability_scope"), "FAIL")

    def test_ai_cannot_be_granted_requirement_authority(self):
        pack = copy.deepcopy(PACK)
        pack["decision_contract"]["ai_may_create_requirements"] = True
        report = report_for(pack)
        self.assertFalse(report["validation_passed"])
        self.assertEqual(check_status(report, "pack.schema_validation"), "FAIL")

    def test_published_pack_with_draft_evidence_fails_closed(self):
        pack = copy.deepcopy(PACK)
        pack["publication_state"] = "published"
        report = report_for(pack)
        self.assertFalse(report["validation_passed"])
        self.assertFalse(report["publication_eligible"])
        self.assertEqual(check_status(report, "publication.gate"), "FAIL")

    def test_expired_review_deadline_blocks_publication(self):
        pack = copy.deepcopy(PACK)
        source = pack["sources"][0]
        source.update({
            "review_state": "current",
            "checked_at": "2026-10-01T12:00:00Z",
            "reviewed_at": "2026-10-01T12:00:00Z",
            "reviewer_role": "qualified-source-reviewer",
            "next_review_at": "2026-10-12T12:00:00Z",
        })
        pack["publication_state"] = "candidate"
        pack["requirements"][0]["review_state"] = "verified"
        report = report_for(pack)
        self.assertTrue(report["validation_passed"])
        self.assertFalse(report["publication_eligible"])
        self.assertIn("review deadline reached", " ".join(report["publication_blockers"]))

    def test_pack_and_schema_digests_are_reported(self):
        report = report_for(copy.deepcopy(PACK))
        self.assertRegex(report["input_sha256"], r"^[a-f0-9]{64}$")
        self.assertRegex(report["schema_sha256"], r"^[a-f0-9]{64}$")
        self.assertEqual(report["evaluated_at"], "2026-10-13T15:00:00Z")

    def test_report_is_reproducible_for_same_inputs_and_timestamp(self):
        first = report_for(copy.deepcopy(PACK))
        second = report_for(copy.deepcopy(PACK))
        self.assertEqual(first, second)


if __name__ == "__main__":
    unittest.main()
