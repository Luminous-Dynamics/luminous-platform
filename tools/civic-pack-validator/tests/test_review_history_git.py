"""Integration tests for all-pack append-only review-history enforcement."""
from __future__ import annotations

import json
import subprocess
import tempfile
import unittest
from pathlib import Path

from check_review_history_git import check_git_review_history


PACK_DIR = "examples/jurisdiction-packs"


def draft_pack(pack_id: str) -> dict:
    return {
        "pack_id": pack_id,
        "publication_state": "draft",
        "sources": [{"source_id": f"{pack_id}.source", "review_state": "unreviewed", "review_receipt_id": None}],
        "source_review_receipts": [],
        "requirements": [{"requirement_id": f"{pack_id}.claim", "review_state": "draft", "review_receipt_id": None}],
        "requirement_review_receipts": [],
    }


class GitReviewHistoryTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.repo = Path(self.temp.name)
        self.git("init", "-b", "main")
        self.git("config", "user.name", "Civic history test")
        self.git("config", "user.email", "civic-history-test@example.invalid")
        (self.repo / "README.md").write_text("fixture\n", encoding="utf-8")
        self.git("add", "README.md")
        self.git("commit", "-m", "base fixture")
        self.base = self.git("rev-parse", "HEAD").strip()

    def tearDown(self):
        self.temp.cleanup()

    def git(self, *args):
        return subprocess.check_output(["git", "-C", str(self.repo), *args], text=True).strip()

    def write_pack(self, filename: str, pack: dict):
        directory = self.repo / PACK_DIR
        directory.mkdir(parents=True, exist_ok=True)
        (directory / filename).write_text(json.dumps(pack, sort_keys=True) + "\n", encoding="utf-8")

    def commit_all(self, message: str):
        self.git("add", "-A")
        self.git("commit", "-m", message)
        return self.git("rev-parse", "HEAD")

    def check(self, head: str):
        return check_git_review_history(self.repo, self.base, head, PACK_DIR)

    def test_every_new_top_level_pack_is_checked(self):
        self.write_pack("za-city-a.json", draft_pack("za.city-a"))
        self.write_pack("za-city-b.json", draft_pack("za.city-b"))
        head = self.commit_all("add two draft packs")
        report = self.check(head)
        self.assertTrue(report["passed"], report["errors"])
        self.assertEqual(report["pack_revisions_checked"], 2)
        self.assertEqual(report["pack_paths_checked"], [
            f"{PACK_DIR}/za-city-a.json", f"{PACK_DIR}/za-city-b.json"
        ])

    def test_new_pack_with_prepopulated_review_history_fails(self):
        pack = draft_pack("za.city-a")
        pack["source_review_receipts"] = [{"receipt_id": "review.seeded"}]
        self.write_pack("za-city-a.json", pack)
        head = self.commit_all("add pack with seeded history")
        report = self.check(head)
        self.assertFalse(report["passed"])
        self.assertIn("empty source_review_receipts", " ".join(report["errors"]))

    def test_new_pack_with_verified_claim_fails_bootstrap(self):
        pack = draft_pack("za.city-a")
        pack["requirements"][0]["review_state"] = "verified"
        self.write_pack("za-city-a.json", pack)
        head = self.commit_all("add reviewed claim without prior history")
        report = self.check(head)
        self.assertFalse(report["passed"])
        self.assertIn("must start in draft state", " ".join(report["errors"]))

    def test_pack_deletion_fails_closed(self):
        self.write_pack("za-city-a.json", draft_pack("za.city-a"))
        self.commit_all("add pack")
        (self.repo / PACK_DIR / "za-city-a.json").unlink()
        head = self.commit_all("delete pack")
        report = self.check(head)
        self.assertFalse(report["passed"])
        self.assertIn("deleting a jurisdiction pack", " ".join(report["errors"]))

    def test_receipt_rewrite_on_non_johannesburg_pack_is_caught(self):
        pack = draft_pack("za.city-a")
        pack["source_review_receipts"] = [
            {"receipt_id": "source.review.v1", "source_id": "za.city-a.source", "decision": "stale"}
        ]
        self.write_pack("za-city-a.json", pack)
        self.commit_all("add base pack with existing history")
        pack["source_review_receipts"][0]["decision"] = "current"
        self.write_pack("za-city-a.json", pack)
        head = self.commit_all("rewrite earlier source receipt")
        report = self.check(head)
        self.assertFalse(report["passed"])
        self.assertIn("source_review_receipts", " ".join(report["errors"]))
        self.assertIn("was changed", " ".join(report["errors"]))

    def test_appending_new_receipt_is_allowed(self):
        pack = draft_pack("za.city-a")
        self.write_pack("za-city-a.json", pack)
        self.commit_all("add draft pack")
        pack["source_review_receipts"].append(
            {"receipt_id": "source.review.v1", "source_id": "za.city-a.source", "decision": "stale"}
        )
        self.write_pack("za-city-a.json", pack)
        head = self.commit_all("append source review receipt")
        report = self.check(head)
        self.assertTrue(report["passed"], report["errors"])
        self.assertEqual(report["pack_revisions_checked"], 2)

    def test_nested_json_is_not_mistaken_for_a_pack(self):
        nested = self.repo / PACK_DIR / "archive" / "old.json"
        nested.parent.mkdir(parents=True)
        nested.write_text(json.dumps(draft_pack("za.archive")), encoding="utf-8")
        head = self.commit_all("add nested archive record")
        report = self.check(head)
        self.assertTrue(report["passed"], report["errors"])
        self.assertEqual(report["pack_revisions_checked"], 0)


if __name__ == "__main__":
    unittest.main()
