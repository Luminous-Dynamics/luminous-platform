"""Tests for exact-scope workflow evaluation."""
from __future__ import annotations

import copy
import json
import unittest
from datetime import datetime, timezone
from pathlib import Path

from evaluate_workflow import evaluate_workflow

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


def reviewed_pack():
    pack = copy.deepcopy(PACK)
    pack["publication_state"] = "published"
    for source in pack["sources"]:
        source.update({
            "review_state": "current",
            "checked_at": "2026-10-01T12:00:00Z",
            "reviewed_at": "2026-10-01T12:00:00Z",
            "reviewer_role": "qualified-source-reviewer",
            "next_review_at": "2026-10-20T12:00:00Z",
        })
    for requirement in pack["requirements"]:
        requirement["review_state"] = "verified"
        requirement["applicability"]["conditions"] = []
        requirement["applicability"]["unknowns"] = []
    pack["requirements"][0]["completion_effect"] = "informational_only"
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
        result = evaluate(pack)
        target = next(r for r in result["requirements"] if r["requirement_id"] == "joburg.confirm-licence-scope")
        self.assertEqual(target["display_state"], "needs_scope_confirmation")

    def test_authority_confirmation_requirement_is_not_approval(self):
        pack = reviewed_pack()
        pack["requirements"][1]["completion_effect"] = "must_confirm_with_authority"
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

    def test_unreviewed_source_makes_pack_unpublishable(self):
        pack = reviewed_pack()
        pack["sources"][0]["review_state"] = "unreviewed"
        result = evaluate(pack)
        self.assertEqual(result["execution_state"], "publication_blocked")
        self.assertEqual(result["requirements"], [])


if __name__ == "__main__":
    unittest.main()
