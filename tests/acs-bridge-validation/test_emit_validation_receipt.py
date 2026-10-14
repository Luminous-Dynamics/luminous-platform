from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "tools/acs-bridge-validation/emit_validation_receipt.py"
SPEC = importlib.util.spec_from_file_location("acs_validation_receipt", SCRIPT)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"unable to import validation receipt emitter from {SCRIPT}")
receipt = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = receipt
SPEC.loader.exec_module(receipt)


class ValidationReceiptTests(unittest.TestCase):
    def test_git_blob_digest_matches_git_object_format(self) -> None:
        self.assertEqual(
            "3b18e512dba79e4c8300dd08aeb37f8e728b8dad",
            receipt._git_blob_sha(b"hello world\n"),
        )

    def test_every_current_test_function_is_inventoried(self) -> None:
        inventory = receipt._test_inventory(ROOT)
        self.assertEqual([], inventory["errors"])
        self.assertGreater(inventory["count"], 0)
        self.assertEqual(
            inventory["count"],
            sum(len(item["test_names"]) for item in inventory["files"]),
        )
        self.assertEqual(
            [item["path"] for item in inventory["files"]],
            list(receipt.TEST_PATHS),
        )

    def test_only_all_successful_checks_with_complete_evidence_can_pass(self) -> None:
        outcomes = {
            "unit_tests": "success",
            "schema_lock_verifier": "success",
            "corpus_validator": "success",
        }
        self.assertEqual(
            "PASS",
            receipt._outcome_status(
                outcomes,
                test_summary_valid=True,
                bundle_valid=True,
                corpus_valid=True,
                sources_complete=True,
            ),
        )

    def test_failed_step_fails_receipt_even_if_other_evidence_looks_good(self) -> None:
        outcomes = {
            "unit_tests": "success",
            "schema_lock_verifier": "failure",
            "corpus_validator": "success",
        }
        self.assertEqual(
            "FAIL",
            receipt._outcome_status(
                outcomes,
                test_summary_valid=True,
                bundle_valid=True,
                corpus_valid=True,
                sources_complete=True,
            ),
        )

    def test_skipped_check_blocks_receipt(self) -> None:
        outcomes = {
            "unit_tests": "success",
            "schema_lock_verifier": "skipped",
            "corpus_validator": "skipped",
        }
        self.assertEqual(
            "BLOCKED",
            receipt._outcome_status(
                outcomes,
                test_summary_valid=False,
                bundle_valid=False,
                corpus_valid=False,
                sources_complete=True,
            ),
        )

    def test_missing_or_inconsistent_test_summary_cannot_pass(self) -> None:
        outcomes = {
            "unit_tests": "success",
            "schema_lock_verifier": "success",
            "corpus_validator": "success",
        }
        for valid in (False,):
            with self.subTest(valid=valid):
                self.assertEqual(
                    "FAIL",
                    receipt._outcome_status(
                        outcomes,
                        test_summary_valid=valid,
                        bundle_valid=True,
                        corpus_valid=True,
                        sources_complete=True,
                    ),
                )

    def test_receipt_binds_current_sources_and_keeps_runtime_claims_unqualified(self) -> None:
        inventory = receipt._test_inventory(ROOT)
        log = f"Ran {inventory['count']} tests in 0.001s\n\nOK\n"
        env = {
            "ACS_UNIT_TESTS_OUTCOME": "success",
            "ACS_SCHEMA_LOCK_OUTCOME": "success",
            "ACS_CORPUS_OUTCOME": "success",
            "GITHUB_REPOSITORY": "Luminous-Dynamics/luminous-platform",
            "GITHUB_WORKFLOW": "ACS-Nixward Bridge Corpus",
            "GITHUB_RUN_ID": "12345",
            "GITHUB_RUN_ATTEMPT": "1",
            "GITHUB_EVENT_NAME": "pull_request",
            "GITHUB_REF": "refs/pull/40/merge",
            "GITHUB_SHA": "abc123",
        }
        result = receipt.build_receipt(ROOT, env, log)
        self.assertEqual("PASS", result["status"])
        self.assertEqual("luminous-acs-nixward-validation-receipt-v1", result["receipt_format"])
        self.assertEqual(inventory["count"], result["checks"]["tests_reported_by_unittest"])
        self.assertTrue(result["inputs"]["schema_lock"]["bundle_integrity"]["all_files_match"])
        self.assertEqual(45, result["inputs"]["schema_lock"]["bundle_integrity"]["matched_file_count"])
        self.assertEqual(32, result["inputs"]["corpus"]["case_count"])
        self.assertEqual("NOT_EVALUATED", result["claims"]["acs_core_conformance"])
        self.assertEqual("NOT_EVALUATED", result["claims"]["nixward_runtime_integration"])
        self.assertEqual("UNVERIFIED_UNSIGNED_SOURCE", result["claims"]["upstream_publisher_authenticity"])

    def test_path_outside_acs_vendor_tree_is_rejected(self) -> None:
        path, error = receipt._safe_vendor_path(ROOT, "../../etc/passwd")
        self.assertIsNone(path)
        self.assertIsNotNone(error)


if __name__ == "__main__":
    unittest.main()
