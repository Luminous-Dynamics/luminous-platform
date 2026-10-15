"""Tests for append-only source and claim review history enforcement."""
from __future__ import annotations

import copy
import unittest

from check_review_history import check_review_history


BASE = {
    "pack_id": "example-pack",
    "source_review_receipts": [
        {"receipt_id": "review.source-a.v1", "source_id": "source-a", "decision": "current"},
        {"receipt_id": "review.source-b.v1", "source_id": "source-b", "decision": "changed"},
    ],
    "requirement_review_receipts": [
        {"receipt_id": "review.requirement-a.v1", "requirement_id": "requirement-a", "decision": "verified"},
    ],
}


class ReviewHistoryTests(unittest.TestCase):
    def test_identical_history_passes(self):
        self.assertEqual(check_review_history(copy.deepcopy(BASE), copy.deepcopy(BASE)), [])

    def test_new_receipts_can_only_be_appended(self):
        current = copy.deepcopy(BASE)
        current["source_review_receipts"].append(
            {"receipt_id": "review.source-a.v2", "source_id": "source-a", "decision": "current"}
        )
        current["requirement_review_receipts"].append(
            {"receipt_id": "review.requirement-a.v2", "requirement_id": "requirement-a", "decision": "verified"}
        )
        self.assertEqual(check_review_history(BASE, current), [])

    def test_removed_receipt_fails(self):
        current = copy.deepcopy(BASE)
        current["source_review_receipts"].pop()
        self.assertIn("source_review_receipts history was shortened", " ".join(
            check_review_history(BASE, current)
        ))

    def test_edited_source_receipt_fails(self):
        current = copy.deepcopy(BASE)
        current["source_review_receipts"][0]["decision"] = "withdrawn"
        self.assertIn("source_review_receipts", " ".join(check_review_history(BASE, current)))
        self.assertIn("was changed", " ".join(check_review_history(BASE, current)))

    def test_edited_requirement_receipt_fails(self):
        current = copy.deepcopy(BASE)
        current["requirement_review_receipts"][0]["decision"] = "stale"
        self.assertIn("requirement_review_receipts", " ".join(check_review_history(BASE, current)))
        self.assertIn("was changed", " ".join(check_review_history(BASE, current)))

    def test_reordered_receipts_fail(self):
        current = copy.deepcopy(BASE)
        current["source_review_receipts"].reverse()
        self.assertIn("was changed", " ".join(check_review_history(BASE, current)))

    def test_old_pack_can_introduce_empty_ledgers(self):
        current = copy.deepcopy(BASE)
        current["source_review_receipts"] = []
        current["requirement_review_receipts"] = []
        self.assertEqual(check_review_history({"pack_id": "old-pack"}, current), [])

    def test_old_pack_cannot_introduce_prepopulated_ledgers(self):
        current = copy.deepcopy(BASE)
        errors = check_review_history({"pack_id": "old-pack"}, current)
        self.assertIn("source_review_receipts was introduced with pre-populated history", " ".join(errors))
        self.assertIn("requirement_review_receipts was introduced with pre-populated history", " ".join(errors))

    def test_non_array_ledger_fails_closed(self):
        current = copy.deepcopy(BASE)
        current["requirement_review_receipts"] = None
        self.assertIn("requirement_review_receipts must be arrays", " ".join(
            check_review_history(BASE, current)
        ))


if __name__ == "__main__":
    unittest.main()
