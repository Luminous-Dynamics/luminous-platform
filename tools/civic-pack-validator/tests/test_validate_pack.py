"""Adversarial tests for the deterministic jurisdiction-pack validator."""
from __future__ import annotations

import copy
import hashlib
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


def attach_review_receipt(
    pack,
    source,
    *,
    decision="current",
    checked_at="2026-10-01T12:00:00Z",
    reviewed_at="2026-10-01T12:00:00Z",
    next_review_at="2026-10-20T12:00:00Z",
    review_scope=None,
):
    """Attach a synthetic receipt; test digests do not represent real source captures."""
    snapshot_sha256 = hashlib.sha256(
        f"synthetic-test-snapshot:{source['source_id']}:{checked_at}".encode("utf-8")
    ).hexdigest()
    receipt_id = f"review.{source['source_id']}.2026-10-01"
    reviewer_id = "test-reviewer"
    reviewer_role = "qualified-source-reviewer"
    if review_scope is None:
        review_scope = {
            "authority_confirmed": True,
            "effective_status_checked": True,
            "content_reviewed": True,
            "applicability_reviewed": True,
            "reuse_rights_reviewed": True,
        }
    receipt = {
        "receipt_id": receipt_id,
        "source_id": source["source_id"],
        "canonical_url": source["canonical_url"],
        "decision": decision,
        "published_at": source["published_at"],
        "effective_from": source["effective_from"],
        "captured_at": checked_at,
        "reviewed_at": reviewed_at,
        "next_review_at": next_review_at,
        "reviewer_id": reviewer_id,
        "reviewer_role": reviewer_role,
        "snapshot_sha256": snapshot_sha256,
        "reuse": source["reuse"],
        "source_license_url": source.get("source_license_url"),
        "attribution": source.get("attribution"),
        "rights_notes": source["rights_notes"],
        "review_scope": review_scope,
    }
    source.update({
        "review_state": decision,
        "checked_at": checked_at,
        "reviewed_at": reviewed_at,
        "next_review_at": next_review_at,
        "reviewer_id": reviewer_id,
        "reviewer_role": reviewer_role,
        "snapshot_sha256": snapshot_sha256,
        "review_receipt_id": receipt_id,
    })
    pack.setdefault("source_review_receipts", []).append(receipt)
    return receipt


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

    def test_duplicate_predicate_ids_fail(self):
        pack = copy.deepcopy(PACK)
        duplicate = copy.deepcopy(pack["requirements"][0]["applicability"]["predicates"][0])
        pack["requirements"][1]["applicability"]["predicates"].append(duplicate)
        report = report_for(pack)
        self.assertFalse(report["validation_passed"])
        self.assertEqual(check_status(report, "pack.unique_identifiers"), "FAIL")

    def test_unknown_predicate_field_fails_schema_validation(self):
        pack = copy.deepcopy(PACK)
        pack["requirements"][0]["applicability"]["predicates"][0]["field"] = "government_approved"
        report = report_for(pack)
        self.assertFalse(report["validation_passed"])
        self.assertEqual(check_status(report, "pack.schema_validation"), "FAIL")

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
        attach_review_receipt(pack, source, next_review_at="2026-10-12T12:00:00Z")
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


    def test_malformed_nested_shape_fails_without_crashing(self):
        pack = copy.deepcopy(PACK)
        pack["workflow"] = None
        report = report_for(pack)
        self.assertFalse(report["validation_passed"])
        self.assertEqual(check_status(report, "pack.schema_validation"), "FAIL")

    def test_future_review_timestamp_blocks_publication(self):
        pack = copy.deepcopy(PACK)
        source = pack["sources"][0]
        attach_review_receipt(
            pack, source,
            checked_at="2026-10-13T16:00:00Z",
            reviewed_at="2026-10-13T16:00:00Z",
            next_review_at="2026-11-13T16:00:00Z",
        )
        pack["publication_state"] = "candidate"
        pack["requirements"][0]["review_state"] = "verified"
        report = report_for(pack)
        self.assertTrue(report["validation_passed"])
        self.assertFalse(report["publication_eligible"])
        self.assertIn("in the future relative to --as-of", " ".join(report["publication_blockers"]))


    def test_non_https_action_url_fails(self):
        pack = copy.deepcopy(PACK)
        pack["requirements"][0]["user_actions"][0]["url"] = "javascript:alert(1)"
        report = report_for(pack)
        self.assertFalse(report["validation_passed"])
        self.assertEqual(check_status(report, "security.https_only"), "FAIL")

    def test_credentialed_https_url_fails(self):
        pack = copy.deepcopy(PACK)
        pack["requirements"][0]["user_actions"][0]["url"] = "https://trusted.example@attacker.example/path"
        report = report_for(pack)
        self.assertFalse(report["validation_passed"])
        self.assertEqual(check_status(report, "security.https_only"), "FAIL")

    def test_fully_reviewed_candidate_can_pass_publication_gate(self):
        pack = copy.deepcopy(PACK)
        pack["publication_state"] = "candidate"
        for source in pack["sources"]:
            attach_review_receipt(pack, source)
        for requirement in pack["requirements"]:
            requirement["review_state"] = "verified"
        report = report_for(pack)
        self.assertTrue(report["validation_passed"])
        self.assertTrue(report["publication_eligible"])
        self.assertEqual(check_status(report, "publication.evidence_gates"), "PASS")
        self.assertEqual(check_status(report, "publication.gate"), "PASS")

    def test_current_source_cannot_be_claimed_without_a_receipt(self):
        pack = copy.deepcopy(PACK)
        source = pack["sources"][0]
        source.update({
            "review_state": "current",
            "checked_at": "2026-10-01T12:00:00Z",
            "reviewed_at": "2026-10-01T12:00:00Z",
            "next_review_at": "2026-10-20T12:00:00Z",
            "reviewer_id": "unverifiable-reviewer",
            "reviewer_role": "qualified-source-reviewer",
            "snapshot_sha256": "a" * 64,
        })
        report = report_for(pack)
        self.assertFalse(report["validation_passed"])
        self.assertEqual(check_status(report, "source.review_receipts"), "FAIL")

    def test_current_receipt_requires_complete_review_scope(self):
        pack = copy.deepcopy(PACK)
        scope = {
            "authority_confirmed": True,
            "effective_status_checked": True,
            "content_reviewed": True,
            "applicability_reviewed": False,
            "reuse_rights_reviewed": True,
        }
        attach_review_receipt(pack, pack["sources"][0], review_scope=scope)
        report = report_for(pack)
        self.assertFalse(report["validation_passed"])
        self.assertIn("incomplete review scope", " ".join(
            check["details"] for check in report["checks"]
        ))

    def test_receipt_snapshot_digest_must_match_source_metadata(self):
        pack = copy.deepcopy(PACK)
        attach_review_receipt(pack, pack["sources"][0])
        pack["sources"][0]["snapshot_sha256"] = "a" * 64
        report = report_for(pack)
        self.assertFalse(report["validation_passed"])
        self.assertEqual(check_status(report, "source.review_receipts"), "FAIL")

    def test_review_interval_cannot_be_extended_by_metadata(self):
        pack = copy.deepcopy(PACK)
        source = pack["sources"][0]
        attach_review_receipt(pack, source, next_review_at="2026-12-01T12:00:00Z")
        pack["publication_state"] = "candidate"
        pack["requirements"][0]["review_state"] = "verified"
        report = report_for(pack)
        self.assertTrue(report["validation_passed"])
        self.assertFalse(report["publication_eligible"])
        self.assertIn("exceeds configured review interval", " ".join(report["publication_blockers"]))


if __name__ == "__main__":
    unittest.main()
