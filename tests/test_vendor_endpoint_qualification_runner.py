"""Contract tests for the exact-head vendor endpoint qualification runner."""
from __future__ import annotations

import hashlib
import tempfile
import unittest
from pathlib import Path

from tools import qualify_vendor_endpoint_contracts as qualification


class QualificationRunnerContractTests(unittest.TestCase):
    @staticmethod
    def _worker_summary(**overrides: object) -> dict[str, object]:
        summary: dict[str, object] = {
            "pattern": "test_vendor_endpoint_qualification_runner.py",
            "state": "passed",
            "tests_run": 1,
            "failures": 0,
            "errors": 0,
            "skipped": 0,
            "expected_failures": 0,
            "unexpected_successes": 0,
            "discovery_errors": 0,
        }
        summary.update(overrides)
        return summary

    def test_manifest_digest_is_stable_across_mapping_order(self) -> None:
        left = {"b": "22", "a": "11"}
        right = {"a": "11", "b": "22"}
        self.assertEqual(qualification.manifest_digest(left), qualification.manifest_digest(right))

    def test_receipt_validator_dependencies_are_bound_into_manifest(self) -> None:
        required = {
            "tools/validate_hardware_portfolio.py",
            "tools/validate_vendor_adapter_registry.py",
            "tools/validate_vendor_operation_receipt.py",
            "schemas/vendor-operation-effect-receipt-v1.schema.json",
            "tests/fixtures/vendor-operation-effect-receipt-prepared-v1.json",
            "tests/test_vendor_operation_receipt.py",
        }
        self.assertTrue(required.issubset(set(qualification.INPUT_PATHS)))

    def test_all_invoked_code_is_bound_into_input_manifest(self) -> None:
        self.assertEqual(qualification.manifest_coverage_gaps(), [])

    def test_unmanifested_validator_or_test_suite_blocks_qualification(self) -> None:
        gaps = qualification.manifest_coverage_gaps(
            validators={"new_validator": "tools/validate_new_contract.py"},
            test_suite_patterns=("test_new_contract.py",),
            input_paths=("tools/qualify_vendor_endpoint_contracts.py",),
        )
        self.assertEqual(
            gaps,
            ["tests/test_new_contract.py", "tools/validate_new_contract.py"],
        )

    def test_input_hashes_match_sha256_of_file_bytes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "fixture.json").write_bytes(b'{"synthetic":true}\n')
            actual = qualification.input_hashes(root, ("fixture.json",))
            expected = hashlib.sha256(b'{"synthetic":true}\n').hexdigest()
            self.assertEqual(actual, {"fixture.json": expected})

    def test_missing_input_is_explicit_not_silently_omitted(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            actual = qualification.input_hashes(Path(directory), ("missing.json",))
            self.assertEqual(actual, {"missing.json": None})

    def test_child_environment_does_not_forward_token_or_home_values(self) -> None:
        child = qualification.child_environment({
            "PATH": "/usr/bin",
            "HOME": "/private/home",
            "GITHUB_TOKEN": "do-not-forward",
            "VENDOR_API_TOKEN": "do-not-forward",
            "AWS_SECRET_ACCESS_KEY": "do-not-forward",
        })
        self.assertEqual(child.get("PATH"), "/usr/bin")
        self.assertNotIn("HOME", child)
        self.assertNotIn("GITHUB_TOKEN", child)
        self.assertNotIn("VENDOR_API_TOKEN", child)
        self.assertNotIn("AWS_SECRET_ACCESS_KEY", child)
        self.assertEqual(child.get("PYTHONNOUSERSITE"), "1")

    def test_suite_success_requires_one_or_more_executed_tests(self) -> None:
        self.assertFalse(qualification.successful_suite({
            "state": "passed", "tests_run": 0, "failures": 0, "errors": 0,
            "skipped": 0, "expected_failures": 0, "unexpected_successes": 0,
            "discovery_errors": 0,
        }))

    def test_suite_success_rejects_boolean_test_counts(self) -> None:
        # bool is a subclass of int in Python; JSON true must not count as 1 test.
        self.assertFalse(qualification.successful_suite({
            "state": "passed", "tests_run": True, "failures": 0, "errors": 0,
            "skipped": 0, "expected_failures": 0, "unexpected_successes": 0,
            "discovery_errors": 0,
        }))
        self.assertFalse(qualification.successful_suite({
            "state": "passed", "tests_run": 1, "failures": False, "errors": 0,
            "skipped": 0, "expected_failures": 0, "unexpected_successes": 0,
            "discovery_errors": 0,
        }))

    def test_suite_success_rejects_skips_even_when_test_runner_is_successful(self) -> None:
        self.assertFalse(qualification.successful_suite({
            "state": "passed", "tests_run": 1, "failures": 0, "errors": 0,
            "skipped": 1, "expected_failures": 0, "unexpected_successes": 0,
            "discovery_errors": 0,
        }))

    def test_worker_summary_contract_accepts_only_clean_pass_evidence(self) -> None:
        summary = self._worker_summary()
        self.assertEqual(
            qualification.worker_summary_errors(
                summary, "test_vendor_endpoint_qualification_runner.py", 0
            ),
            [],
        )

    def test_worker_summary_contract_rejects_open_or_incomplete_schemas(self) -> None:
        extra = self._worker_summary(debug="must not enter evidence")
        missing = self._worker_summary()
        del missing["errors"]
        for summary in (extra, missing):
            with self.subTest(summary=summary):
                self.assertEqual(
                    qualification.worker_summary_errors(
                        summary, "test_vendor_endpoint_qualification_runner.py", 0
                    ),
                    ["SUMMARY_FIELDS_MISMATCH"],
                )

    def test_worker_summary_contract_rejects_boolean_and_negative_counters(self) -> None:
        for value in (True, -1):
            with self.subTest(value=value):
                summary = self._worker_summary(tests_run=value)
                self.assertEqual(
                    qualification.worker_summary_errors(
                        summary, "test_vendor_endpoint_qualification_runner.py", 0
                    ),
                    ["SUMMARY_COUNTER_INVALID"],
                )

    def test_worker_summary_contract_rejects_state_exit_mismatch(self) -> None:
        self.assertEqual(
            qualification.worker_summary_errors(
                self._worker_summary(), "test_vendor_endpoint_qualification_runner.py", 1
            ),
            ["PASSED_SUMMARY_EXIT_MISMATCH"],
        )
        self.assertEqual(
            qualification.worker_summary_errors(
                self._worker_summary(state="failed"), "test_vendor_endpoint_qualification_runner.py", 0
            ),
            ["FAILED_SUMMARY_EXIT_MISMATCH"],
        )

    def test_worker_summary_contract_requires_failure_evidence_for_failed_state(self) -> None:
        self.assertEqual(
            qualification.worker_summary_errors(
                self._worker_summary(state="failed"),
                "test_vendor_endpoint_qualification_runner.py",
                1,
            ),
            ["FAILED_SUMMARY_HAS_NO_FAILURE_EVIDENCE"],
        )
        self.assertEqual(
            qualification.worker_summary_errors(
                self._worker_summary(state="failed", errors=1),
                "test_vendor_endpoint_qualification_runner.py",
                1,
            ),
            [],
        )

    def test_worker_summary_contract_fixes_runner_error_shape(self) -> None:
        runner_error = self._worker_summary(
            state="runner_error",
            tests_run=0,
            errors=1,
            discovery_errors=1,
        )
        self.assertEqual(
            qualification.worker_summary_errors(
                runner_error, "test_vendor_endpoint_qualification_runner.py", 2
            ),
            [],
        )
        runner_error["discovery_errors"] = 0
        self.assertEqual(
            qualification.worker_summary_errors(
                runner_error, "test_vendor_endpoint_qualification_runner.py", 2
            ),
            ["RUNNER_ERROR_SUMMARY_INCONSISTENT"],
        )

    def test_worker_summary_contract_binds_the_requested_suite_identity(self) -> None:
        self.assertEqual(
            qualification.worker_summary_errors(
                self._worker_summary(),
                "test_vendor_operation_receipt.py",
                0,
            ),
            ["SUMMARY_PATTERN_MISMATCH"],
        )

    def test_clean_evidence_is_the_only_pass_state(self) -> None:
        self.assertEqual(qualification.qualification_status([]), "passed")

    def test_unavailable_evidence_is_incomplete_not_pass(self) -> None:
        self.assertEqual(
            qualification.qualification_status(["COMMIT_SHA_UNAVAILABLE"]),
            "incomplete",
        )

    def test_failed_validator_is_never_downgraded_to_incomplete(self) -> None:
        self.assertEqual(
            qualification.qualification_status(["PROFILE_VALIDATOR_FAILED"]),
            "failed",
        )

    def test_failed_receipt_validator_is_never_downgraded_to_incomplete(self) -> None:
        self.assertEqual(
            qualification.qualification_status(["RECEIPT_VALIDATOR_FAILED"]),
            "failed",
        )


if __name__ == "__main__":
    unittest.main()
