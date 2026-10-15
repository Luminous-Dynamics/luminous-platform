"""Tests for exact-scope workflow evaluation."""
from __future__ import annotations

import copy
import json
import unittest
from datetime import datetime, timezone
from pathlib import Path

from evaluate_workflow import evaluate_workflow
from validate_pack import requirement_claim_sha256

ROOT = Path(__file__).resolve().parents[3]
SCHEMA_PATH = ROOT / "schemas" / "jurisdiction-pack-v0.1.0.schema.json"
PACK_PATH = ROOT / "examples" / "jurisdiction-packs" / "za-city-of-johannesburg-food-service-0.1.0.json"
SCHEMA_BYTES = SCHEMA_PATH.read_bytes()
PACK_BYTES = PACK_PATH.read_bytes()
SCHEMA = json.loads(SCHEMA_BYTES)
PACK = json.loads(PACK_BYTES)
AS_OF = datetime(2026, 10, 13, 15, 0, tzinfo=timezone.utc)
JURISDICTION = "za/gauteng/city-of-johannesburg"
WORKFLOW = "business.restaurant-launch"
BUSINESS_TYPE = "sit_down_restaurant"


def evaluate(pack, *, jurisdiction=JURISDICTION, workflow=WORKFLOW, business_type=BUSINESS_TYPE, facts=None):
    raw = json.dumps(pack, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")
    if facts is None:
        facts = {"handles_food": True, "business_entity_registration_intended": True}
    return evaluate_workflow(
        SCHEMA,
        pack,
        schema_bytes=SCHEMA_BYTES,
        pack_bytes=raw,
        as_of=AS_OF,
        jurisdiction_id=jurisdiction,
        workflow_id=workflow,
        business_type=business_type,
        facts=facts,
    )


def _attach_source_receipt(pack, source):
    """Create a synthetic, internally consistent receipt for evaluator tests only."""
    checked_at = "2026-10-01T12:00:00Z"
    reviewer_id = "synthetic-source-reviewer"
    reviewer_role = "qualified-source-reviewer"
    receipt_id = f"review.{source['source_id']}.2026-10-01"
    digest = __import__("hashlib").sha256(
        f"synthetic-test-bytes:{source['source_id']}".encode("utf-8")
    ).hexdigest()
    receipt = {
        "receipt_id": receipt_id,
        "source_id": source["source_id"],
        "canonical_url": source["canonical_url"],
        "decision": "current",
        "published_at": source["published_at"],
        "effective_from": source["effective_from"],
        "captured_at": checked_at,
        "reviewed_at": checked_at,
        "next_review_at": "2026-10-20T12:00:00Z",
        "reviewer_id": reviewer_id,
        "reviewer_role": reviewer_role,
        "snapshot_sha256": digest,
        "reuse": source["reuse"],
        "source_license_url": source.get("source_license_url"),
        "attribution": source.get("attribution"),
        "rights_notes": source["rights_notes"],
        "review_scope": {
            "authority_confirmed": True,
            "effective_status_checked": True,
            "content_reviewed": True,
            "applicability_reviewed": True,
            "reuse_rights_reviewed": True,
        },
    }
    source.update({
        "review_state": "current",
        "checked_at": checked_at,
        "reviewed_at": checked_at,
        "next_review_at": receipt["next_review_at"],
        "reviewer_id": reviewer_id,
        "reviewer_role": reviewer_role,
        "snapshot_sha256": digest,
        "review_receipt_id": receipt_id,
    })
    pack["source_review_receipts"].append(receipt)


def _attach_requirement_receipt(pack, requirement):
    """Bind test claim state to the exact claim fields and source receipts."""
    reviewer_id = "synthetic-claim-reviewer"
    reviewer_role = "qualified-claim-reviewer"
    receipt_id = f"review.{requirement['requirement_id']}.2026-10-01"
    source_receipt_ids = [
        next(s["review_receipt_id"] for s in pack["sources"] if s["source_id"] == source_ref)
        for source_ref in requirement["source_refs"]
    ]
    receipt = {
        "receipt_id": receipt_id,
        "requirement_id": requirement["requirement_id"],
        "decision": "verified",
        "reviewed_at": "2026-10-01T13:00:00Z",
        "next_review_at": "2026-10-20T13:00:00Z",
        "reviewer_id": reviewer_id,
        "reviewer_role": reviewer_role,
        "claim_sha256": requirement_claim_sha256(requirement),
        "source_receipt_ids": source_receipt_ids,
        "review_scope": {
            "source_support_checked": True,
            "jurisdiction_checked": True,
            "applicability_checked": True,
            "user_language_checked": True,
            "completion_effect_checked": True,
        },
    }
    requirement.update({
        "review_state": "verified",
        "reviewer_id": reviewer_id,
        "reviewer_role": reviewer_role,
        "review_receipt_id": receipt_id,
    })
    pack["requirement_review_receipts"].append(receipt)


def _refresh_claim_receipt(pack, requirement):
    """Append a new synthetic claim receipt after a test changes reviewed content."""
    previous = next(
        item for item in pack["requirement_review_receipts"]
        if item["receipt_id"] == requirement["review_receipt_id"]
    )
    reviewed_at = "2026-10-02T13:00:00Z"
    receipt_id = f"review.{requirement['requirement_id']}.2026-10-02"
    receipt = copy.deepcopy(previous)
    receipt.update({
        "receipt_id": receipt_id,
        "reviewed_at": reviewed_at,
        "next_review_at": "2026-10-21T13:00:00Z",
        "claim_sha256": requirement_claim_sha256(requirement),
        "source_receipt_ids": [
            next(s["review_receipt_id"] for s in pack["sources"] if s["source_id"] == source_ref)
            for source_ref in requirement["source_refs"]
        ],
    })
    pack["requirement_review_receipts"].append(receipt)
    requirement["review_receipt_id"] = receipt_id


def reviewed_pack():
    pack = copy.deepcopy(PACK)
    pack["publication_state"] = "published"
    pack.setdefault("source_review_receipts", [])
    pack.setdefault("requirement_review_receipts", [])
    for source in pack["sources"]:
        _attach_source_receipt(pack, source)
    for requirement in pack["requirements"]:
        requirement["applicability"]["conditions"] = []
        requirement["applicability"]["unknowns"] = []
    pack["requirements"][0]["completion_effect"] = "informational_only"
    for requirement in pack["requirements"]:
        _attach_requirement_receipt(pack, requirement)
    return pack


class WorkflowEvaluatorTests(unittest.TestCase):
    def test_draft_pack_cannot_emit_current_guidance(self):
        result = evaluate(copy.deepcopy(PACK))
        self.assertEqual(result["execution_state"], "pack_not_published")
        self.assertEqual(result["requirements"], [])

    def test_wrong_jurisdiction_stops_evaluation(self):
        result = evaluate(reviewed_pack(), jurisdiction="za/gauteng/other-municipality")
        self.assertEqual(result["execution_state"], "scope_confirmation_required")
        self.assertEqual(result["scope_status"], "jurisdiction_mismatch")
        self.assertEqual(result["requirements"], [])

    def test_wrong_workflow_stops_evaluation(self):
        result = evaluate(reviewed_pack(), workflow="business.other")
        self.assertEqual(result["scope_status"], "workflow_mismatch")
        self.assertEqual(result["requirements"], [])

    def test_unknown_business_type_stops_evaluation(self):
        result = evaluate(reviewed_pack(), business_type="food-thing-unknown")
        self.assertEqual(result["scope_status"], "business_type_out_of_scope")
        self.assertEqual(result["requirements"], [])

    def test_current_matching_pack_returns_only_guidance_candidates(self):
        result = evaluate(reviewed_pack())
        self.assertEqual(result["execution_state"], "guidance_ready")
        self.assertGreater(len(result["requirements"]), 0)
        self.assertTrue(all(r["authority_outcome"] == "unknown" for r in result["requirements"]))
        self.assertTrue(all(r["user_checkbox_effect"] == "records_user_progress_only" for r in result["requirements"]))
        self.assertFalse(any(r["display_state"] in {"approved", "refused", "satisfied", "legally_compliant"} for r in result["requirements"]))

    def test_conditions_or_unknowns_require_scope_confirmation(self):
        pack = reviewed_pack()
        pack["requirements"][0]["applicability"]["unknowns"] = ["Premises classification is not known"]
        _refresh_claim_receipt(pack, pack["requirements"][0])
        result = evaluate(pack)
        target = next(r for r in result["requirements"] if r["requirement_id"] == "joburg.confirm-licence-scope")
        self.assertEqual(target["display_state"], "needs_scope_confirmation")

    def test_authority_confirmation_requirement_is_not_approval(self):
        pack = reviewed_pack()
        pack["requirements"][1]["completion_effect"] = "must_confirm_with_authority"
        _refresh_claim_receipt(pack, pack["requirements"][1])
        result = evaluate(pack)
        target = next(r for r in result["requirements"] if r["requirement_id"] == "za.confirm-food-premises-process")
        self.assertEqual(target["display_state"], "authority_confirmation_required")
        self.assertEqual(target["authority_outcome"], "unknown")

    def test_missing_fact_evaluates_unknown_and_requests_confirmation(self):
        result = evaluate(reviewed_pack(), facts={})
        self.assertEqual(result["execution_state"], "guidance_ready")
        self.assertTrue(result["requirements"])
        self.assertTrue(all(r["display_state"] == "needs_scope_confirmation" for r in result["requirements"]))
        self.assertTrue(all(r["predicate_evaluation"] == "needs_scope_confirmation" for r in result["requirements"]))

    def test_false_predicate_removes_requirement_from_guidance(self):
        result = evaluate(reviewed_pack(), facts={
            "handles_food": False,
            "business_entity_registration_intended": True,
        })
        ids = {r["requirement_id"] for r in result["requirements"]}
        self.assertNotIn("joburg.confirm-licence-scope", ids)
        self.assertNotIn("za.confirm-food-premises-process", ids)
        self.assertIn("za.consider-cipc-registration", ids)
        self.assertIn("joburg.confirm-licence-scope", result["not_applicable_requirement_ids"])

    def test_invalid_fact_type_fails_closed(self):
        result = evaluate(reviewed_pack(), facts={"handles_food": 1})
        self.assertEqual(result["execution_state"], "invalid_context")
        self.assertIn("handles_food", result["invalid_fact_values"])
        self.assertEqual(result["requirements"], [])

    def test_unknown_fact_key_fails_closed(self):
        result = evaluate(reviewed_pack(), facts={"government_approved": True})
        self.assertEqual(result["execution_state"], "invalid_context")
        self.assertIn("government_approved", result["invalid_fact_keys"])
        self.assertEqual(result["requirements"], [])

    def test_non_object_fact_context_fails_closed(self):
        result = evaluate(reviewed_pack(), facts=["handles_food"])
        self.assertEqual(result["execution_state"], "invalid_context")
        self.assertEqual(result["requirements"], [])

    def test_tampered_source_review_state_fails_closed(self):
        pack = reviewed_pack()
        pack["sources"][0]["review_state"] = "unreviewed"
        result = evaluate(pack)
        self.assertEqual(result["execution_state"], "invalid_pack")
        self.assertEqual(result["requirements"], [])


if __name__ == "__main__":
    unittest.main()
