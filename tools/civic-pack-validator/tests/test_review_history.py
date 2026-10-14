"""Tests for append-only source review history enforcement."""
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
}


class ReviewHistoryTests(unittest.TestCase):
    def test_identical_history_passes(self):
        self.assertEqual(check_review_history(copy.deepcopy(BASE), copy.deepcopy(BASE)), [])

    def test_new_receipts_can_only_be_appended(self):
        current = copy.deepcopy(BASE)
        current["source_review_receipts"].append(
            {"receipt_id": "review.source-a.v2", "source_id": "source-a", "decision": "current"}
        )
        self.assertEqual(check_review_history(BASE, current), [])

    def test_removed_receipt_fails(self):
        current = copy.deepcopy(BASE)
        current["source_review_receipts"].pop()
        self.assertIn("shortened", " ".join(check_review_history(BASE, current)))

    def test_edited_receipt_fails(self):
        current = copy.deepcopy(BASE)
        current["source_review_receipts"][0]["decision"] = "withdrawn"
        self.assertIn("was changed", " ".join(check_review_history(BASE, current)))

    def test_reordered_receipts_fail(self):
        current = copy.deepcopy(BASE)
        current["source_review_receipts"].reverse()
        self.assertIn("was changed", " ".join(check_review_history(BASE, current)))

    def test_old_pack_without_ledger_is_treated_as_empty_history(self):
        current = copy.deepcopy(BASE)
        self.assertEqual(check_review_history({"pack_id": "old-pack"}, current), [])


if __name__ == "__main__":
    unittest.main()
