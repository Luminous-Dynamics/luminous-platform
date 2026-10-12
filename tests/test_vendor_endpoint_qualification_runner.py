"""Contract tests for the exact-head vendor endpoint qualification runner."""
from __future__ import annotations

import hashlib
import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

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

    def test_child_execution_budget_leaves_workflow_headroom(self) -> None:
        worst_case = (
            len(qualification.VALIDATORS) * qualification.VALIDATOR_TIMEOUT_SECONDS
            + len(qualification.TEST_SUITE_PATTERNS) * qualification.TEST_WORKER_TIMEOUT_SECONDS
        )
        self.assertLessEqual(worst_case, qualification.MAX_CHILD_EXECUTION_BUDGET_SECONDS)
        self.assertLess(qualification.MAX_CHILD_EXECUTION_BUDGET_SECONDS, 600)

    def test_report_destination_only_allows_safe_artifact_json(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            expected = root / "artifacts" / "report.json"
            self.assertEqual(
                qualification.safe_report_destination(root, Path("artifacts/report.json")),
                (expected, "artifacts/report.json"),
            )
            self.assertIsNone(
                qualification.safe_report_destination(
                    root, Path("tools/qualify_vendor_endpoint_contracts.py")
                )
            )
            self.assertIsNone(
                qualification.safe_report_destination(root, Path("artifacts/report.txt"))
            )

            expected.parent.mkdir()
            expected.write_text('{"not_a_report":true}\n', encoding="utf-8")
            self.assertIsNone(
                qualification.safe_report_destination(root, Path("artifacts/report.json"))
            )
            expected.write_text(
                json.dumps({"report_schema_version": qualification.REPORT_SCHEMA_VERSION}),
                encoding="utf-8",
            )
            self.assertEqual(
                qualification.safe_report_destination(root, Path("artifacts/report.json")),
                (expected, "artifacts/report.json"),
            )

    def test_report_destination_rejects_external_symlink_into_checkout(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            parent = Path(directory)
            root = parent / "checkout"
            (root / "artifacts").mkdir(parents=True)
            alias = parent / "external-alias"
            alias.symlink_to(root / "artifacts", target_is_directory=True)
            self.assertIsNone(
                qualification.safe_report_destination(root, alias / "report.json")
            )

    def test_report_destination_rejects_tracked_artifact_overwrite(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            tracked = root / "artifacts" / "report.json"
            tracked.parent.mkdir()
            tracked.write_text(
                json.dumps({"report_schema_version": qualification.REPORT_SCHEMA_VERSION}),
                encoding="utf-8",
            )
            commands = (
                ["git", "init", "-q"],
                ["git", "config", "user.email", "qualification-test@example.invalid"],
                ["git", "config", "user.name", "Qualification Test"],
                ["git", "add", "artifacts/report.json"],
                ["git", "commit", "-q", "-m", "tracked report fixture"],
            )
            for command in commands:
                subprocess.run(
                    command, cwd=root, check=True,
                    stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                )
            self.assertIsNone(
                qualification.safe_report_destination(root, Path("artifacts/report.json"))
            )

    def test_report_destination_rejects_symlinked_artifact_directory(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            outside = root / "outside"
            outside.mkdir()
            (root / "artifacts").symlink_to(outside, target_is_directory=True)
            self.assertIsNone(
                qualification.safe_report_destination(root, Path("artifacts/report.json"))
            )

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

    def test_manifest_hashing_rejects_file_changed_during_read(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            fixture = root / "input.json"
            fixture.write_bytes(b'{"synthetic":true}\\n')
            real_read = os.read
            mutated = False

            def read_then_mutate(descriptor: int, length: int) -> bytes:
                nonlocal mutated
                chunk = real_read(descriptor, length)
                if not mutated:
                    mutated = True
                    fixture.write_bytes(b'{"synthetic":false}\\n')
                return chunk

            with patch.object(qualification.os, "read", side_effect=read_then_mutate):
                self.assertEqual(
                    qualification.input_hashes(root, ("input.json",)),
                    {"input.json": None},
                )

    def test_manifest_hashing_rejects_non_regular_fifo_without_blocking(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            fifo = root / "input.fifo"
            os.mkfifo(fifo)
            self.assertEqual(
                qualification.input_hashes(root, ("input.fifo",)),
                {"input.fifo": None},
            )

    def test_input_tree_freeze_removes_write_bits_and_restores_modes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            item = root / "nested" / "input.json"
            item.parent.mkdir()
            item.write_text("{}\\n", encoding="utf-8")
            original_file_mode = item.stat().st_mode & 0o777
            original_directory_mode = item.parent.stat().st_mode & 0o777
            modes = qualification.make_input_tree_read_only(root, ("nested/input.json",))
            self.assertIsNotNone(modes)
            self.assertEqual(item.stat().st_mode & 0o222, 0)
            self.assertEqual(item.parent.stat().st_mode & 0o222, 0)
            self.assertTrue(qualification.restore_input_tree_permissions(root, modes or {}))
            self.assertEqual(item.stat().st_mode & 0o777, original_file_mode)
            self.assertEqual(item.parent.stat().st_mode & 0o777, original_directory_mode)

    def test_input_tree_freeze_rejects_symlinked_manifest_input(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            target = root / "actual.json"
            target.write_text("{}\\n", encoding="utf-8")
            link = root / "input.json"
            link.symlink_to(target)
            self.assertIsNone(
                qualification.make_input_tree_read_only(root, ("input.json",))
            )

    def test_input_tree_freeze_rolls_back_partial_permission_changes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            item = root / "nested" / "input.json"
            item.parent.mkdir()
            item.write_text("{}\\n", encoding="utf-8")
            original_file_mode = item.stat().st_mode & 0o777
            original_directory_mode = item.parent.stat().st_mode & 0o777
            real_chmod = os.chmod
            calls = 0

            def fail_once(path: str | os.PathLike[str], mode: int) -> None:
                nonlocal calls
                calls += 1
                if calls == 2:
                    raise OSError("synthetic permission failure")
                real_chmod(path, mode)

            with patch.object(qualification.os, "chmod", side_effect=fail_once):
                result = qualification.make_input_tree_read_only(root, ("nested/input.json",))
            self.assertIsNone(result)
            self.assertEqual(item.stat().st_mode & 0o777, original_file_mode)
            self.assertEqual(item.parent.stat().st_mode & 0o777, original_directory_mode)

    def test_input_freeze_raises_when_partial_rollback_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            item = root / "nested" / "input.json"
            item.parent.mkdir()
            item.write_text("{}\\n", encoding="utf-8")
            real_chmod = os.chmod
            calls = 0

            def fail_freeze_and_first_restore(path: str | os.PathLike[str], mode: int) -> None:
                nonlocal calls
                calls += 1
                if calls in (2, 3):
                    raise OSError("synthetic permission failure")
                real_chmod(path, mode)

            with patch.object(qualification.os, "chmod", side_effect=fail_freeze_and_first_restore):
                with self.assertRaises(qualification.InputFreezeError):
                    qualification.make_input_tree_read_only(root, ("nested/input.json",))

    def test_source_tree_clean_detects_tracked_and_untracked_source_changes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            tracked = root / "tools" / "tracked.py"
            tracked.parent.mkdir()
            tracked.write_text("VALUE = 1\\n", encoding="utf-8")
            gitignore = root / ".gitignore"
            gitignore.write_text("ignored-source.py\n", encoding="utf-8")
            commands = (
                ["git", "init", "-q"],
                ["git", "config", "user.email", "qualification-test@example.invalid"],
                ["git", "config", "user.name", "Qualification Test"],
                ["git", "add", ".gitignore", "tools/tracked.py"],
                ["git", "commit", "-q", "-m", "synthetic fixture"],
            )
            for command in commands:
                subprocess.run(
                    command, cwd=root, check=True,
                    stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                )
            self.assertTrue(qualification.source_tree_clean(root))
            tracked.write_text("VALUE = 2\\n", encoding="utf-8")
            self.assertFalse(qualification.source_tree_clean(root))
            tracked.write_text("VALUE = 1\\n", encoding="utf-8")
            self.assertTrue(qualification.source_tree_clean(root))
            untracked = root / "tests" / "injected.py"
            untracked.parent.mkdir()
            untracked.write_text("VALUE = 3\\n", encoding="utf-8")
            self.assertFalse(qualification.source_tree_clean(root))
            untracked.unlink()
            root_untracked = root / "unexpected.py"
            root_untracked.write_text("VALUE = 4\\n", encoding="utf-8")
            self.assertFalse(qualification.source_tree_clean(root))
            root_untracked.unlink()
            ignored = root / "ignored-source.py"
            ignored.write_text("VALUE = 5\\n", encoding="utf-8")
            subprocess.run(
                ["git", "check-ignore", "-q", "ignored-source.py"],
                cwd=root, check=True,
                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
            )
            self.assertFalse(qualification.source_tree_clean(root))
            ignored.unlink()
            report = root / "artifacts" / "qualification-report.json"
            report.parent.mkdir()
            report.write_text("{}\\n", encoding="utf-8")
            self.assertTrue(
                qualification.source_tree_clean(root, ("artifacts/qualification-report.json",))
            )
            report.write_text('{"extra":true}\\n', encoding="utf-8")
            self.assertFalse(qualification.source_tree_clean(root, ()))

    def test_child_integrity_fence_rejects_head_change(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            fixture = root / "fixture.json"
            fixture.write_text("{}\\n", encoding="utf-8")
            hashes = qualification.input_hashes(root, ("fixture.json",))
            with (
                patch.object(qualification, "checked_out_sha", return_value="b" * 40),
                patch.object(qualification, "source_tree_clean", return_value=True),
            ):
                self.assertEqual(
                    qualification.child_execution_integrity_errors(
                        root, "a" * 40, hashes
                    ),
                    ["REVISION_MISMATCH"],
                )

    def test_input_manifest_fence_detects_changed_bytes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            fixture = root / "fixture.json"
            fixture.write_bytes(b'{"frozen":true}\\n')
            baseline = qualification.input_hashes(root, ("fixture.json",))
            self.assertTrue(qualification.input_manifest_unchanged(root, baseline))
            fixture.write_bytes(b'{"frozen":false}\\n')
            self.assertFalse(qualification.input_manifest_unchanged(root, baseline))

    def test_dirty_worktree_blocks_all_child_checks(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for relative in qualification.INPUT_PATHS:
                path = root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b"baseline input\\n")
            report_path = root / "artifacts" / "qualification-report.json"
            old_cwd = Path.cwd()
            try:
                with (
                    patch.object(qualification, "ROOT", root),
                    patch.object(qualification, "checked_out_sha", return_value="a" * 40),
                    patch.object(qualification, "source_tree_clean", return_value=False),
                    patch.object(qualification.platform, "python_version", return_value="3.13.7"),
                    patch.object(
                        qualification,
                        "dependency_version",
                        side_effect={"jsonschema": "4.26.0", "cryptography": "50.0.2"}.get,
                    ),
                    patch.dict(os.environ, {"GITHUB_SHA": "a" * 40}),
                    patch.object(qualification, "run_validator") as run_validator,
                    patch.object(qualification, "run_test_suite") as run_test_suite,
                ):
                    return_code = qualification.main(["--report", str(report_path)])
                    report = json.loads(report_path.read_text(encoding="utf-8"))
                    self.assertEqual(return_code, 1)
                    self.assertEqual(report["blocker_codes"], ["SOURCE_TREE_DIRTY"])
                    run_validator.assert_not_called()
                    run_test_suite.assert_not_called()
                    self.assertTrue(all(
                        validator["state"] == "blocked_source_integrity"
                        for validator in report["validators"].values()
                    ))
            finally:
                os.chdir(old_cwd)

    def test_unmanifested_source_drift_invalidates_child_result(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for relative in qualification.INPUT_PATHS:
                path = root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b"baseline input\\n")
            unrelated_source = root / "README.md"
            unrelated_source.write_text("baseline documentation\\n", encoding="utf-8")
            report_path = root / "artifacts" / "qualification-report.json"
            old_cwd = Path.cwd()

            def successful_validator(_root: Path, relative_script: str) -> dict[str, object]:
                unrelated_source.write_text("changed documentation\\n", encoding="utf-8")
                return {
                    "script": relative_script,
                    "return_code": 0,
                    "timed_out": False,
                    "state": "passed",
                }

            try:
                with (
                    patch.object(qualification, "ROOT", root),
                    patch.object(qualification, "checked_out_sha", return_value="a" * 40),
                    patch.object(qualification, "source_tree_clean", side_effect=[True, False, False]),
                    patch.object(qualification.platform, "python_version", return_value="3.13.7"),
                    patch.object(
                        qualification,
                        "dependency_version",
                        side_effect={"jsonschema": "4.26.0", "cryptography": "50.0.2"}.get,
                    ),
                    patch.dict(os.environ, {"GITHUB_SHA": "a" * 40}),
                    patch.object(qualification, "run_validator", side_effect=successful_validator) as run_validator,
                    patch.object(qualification, "run_test_suite") as run_test_suite,
                ):
                    return_code = qualification.main(["--report", str(report_path)])
                    report = json.loads(report_path.read_text(encoding="utf-8"))
                    self.assertEqual(return_code, 1)
                    self.assertEqual(report["blocker_codes"], ["SOURCE_TREE_DIRTY"])
                    self.assertEqual(run_validator.call_count, 1)
                    run_test_suite.assert_not_called()
                    self.assertEqual(
                        report["validators"]["endpoint_profile"]["state"],
                        "execution_integrity_lost",
                    )
                    self.assertFalse(
                        report["source_integrity"]["execution_integrity_preserved"]
                    )
                    self.assertTrue(all(
                        validator["state"] == "blocked_execution_integrity_lost"
                        for name, validator in report["validators"].items()
                        if name != "endpoint_profile"
                    ))
            finally:
                os.chdir(old_cwd)

    def test_manifest_drift_stops_later_checks_and_discards_current_result(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for relative in qualification.INPUT_PATHS:
                path = root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b"baseline input\\n")
            report_path = root / "artifacts" / "qualification-report.json"
            changed_path = root / qualification.INPUT_PATHS[0]
            old_cwd = Path.cwd()

            def mutating_validator(_root: Path, relative_script: str) -> dict[str, object]:
                # Simulate a child bypassing ordinary read-only mode bits.
                os.chmod(changed_path, 0o644)
                changed_path.write_bytes(b"mutated input\\n")
                return {
                    "script": relative_script,
                    "return_code": 0,
                    "timed_out": False,
                    "state": "passed",
                }

            try:
                with (
                    patch.object(qualification, "ROOT", root),
                    patch.object(qualification, "checked_out_sha", return_value="a" * 40),
                    patch.object(qualification, "source_tree_clean", return_value=True),
                    patch.object(qualification.platform, "python_version", return_value="3.13.7"),
                    patch.object(
                        qualification,
                        "dependency_version",
                        side_effect={"jsonschema": "4.26.0", "cryptography": "50.0.2"}.get,
                    ),
                    patch.dict(os.environ, {"GITHUB_SHA": "a" * 40}),
                    patch.object(qualification, "run_validator", side_effect=mutating_validator) as run_validator,
                    patch.object(qualification, "run_test_suite") as run_test_suite,
                ):
                    return_code = qualification.main(["--report", str(report_path)])
                    report = json.loads(report_path.read_text(encoding="utf-8"))
                    self.assertEqual(return_code, 1)
                    self.assertEqual(run_validator.call_count, 1)
                    run_test_suite.assert_not_called()
                    self.assertEqual(report["status"], "failed")
                    self.assertEqual(report["blocker_codes"], ["INPUT_DIGEST_CHANGED"])
                    self.assertEqual(
                        report["validators"]["endpoint_profile"]["state"],
                        "input_drift_detected",
                    )
                    self.assertTrue(all(
                        report["validators"][name]["state"] == "blocked_execution_integrity_lost"
                        for name in ("endpoint_approval_binding", "capability_registry", "mutation_receipt")
                    ))
                    self.assertTrue(all(
                        suite["state"] == "blocked_execution_integrity_lost"
                        for suite in report["test_suites"]
                    ))
            finally:
                os.chdir(old_cwd)

    def test_manifest_hashing_rejects_symlinked_files_and_parents(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            actual_dir = root / "actual"
            actual_dir.mkdir()
            actual = actual_dir / "input.json"
            actual.write_text("{}\\n", encoding="utf-8")
            (root / "linked-file.json").symlink_to(actual)
            (root / "linked-dir").symlink_to(actual_dir, target_is_directory=True)

            self.assertEqual(
                qualification.input_hashes(root, ("linked-file.json",)),
                {"linked-file.json": None},
            )
            self.assertEqual(
                qualification.input_hashes(root, ("linked-dir/input.json",)),
                {"linked-dir/input.json": None},
            )

    def test_manifest_hashing_rejects_path_traversal(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            parent = Path(directory)
            root = parent / "root"
            root.mkdir()
            outside = parent / "outside-qualification-input.json"
            outside.write_text("not part of the manifest\\n", encoding="utf-8")
            self.assertEqual(
                qualification.input_hashes(root, ("../outside-qualification-input.json",)),
                {"../outside-qualification-input.json": None},
            )

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
            "pattern": "test_vendor_endpoint_qualification_runner.py", "state": "passed", "tests_run": 0, "failures": 0, "errors": 0,
            "skipped": 0, "expected_failures": 0, "unexpected_successes": 0,
            "discovery_errors": 0,
        }))

    def test_suite_success_rejects_boolean_test_counts(self) -> None:
        # bool is a subclass of int in Python; JSON true must not count as 1 test.
        self.assertFalse(qualification.successful_suite({
            "pattern": "test_vendor_endpoint_qualification_runner.py", "state": "passed", "tests_run": True, "failures": 0, "errors": 0,
            "skipped": 0, "expected_failures": 0, "unexpected_successes": 0,
            "discovery_errors": 0,
        }))
        self.assertFalse(qualification.successful_suite({
            "pattern": "test_vendor_endpoint_qualification_runner.py", "state": "passed", "tests_run": 1, "failures": False, "errors": 0,
            "skipped": 0, "expected_failures": 0, "unexpected_successes": 0,
            "discovery_errors": 0,
        }))

    def test_suite_success_rejects_unregistered_suite_identity(self) -> None:
        self.assertFalse(qualification.successful_suite({
            "pattern": "unregistered_test_suite.py",
            "state": "passed",
            "tests_run": 1,
            "failures": 0,
            "errors": 0,
            "skipped": 0,
            "expected_failures": 0,
            "unexpected_successes": 0,
            "discovery_errors": 0,
        }))

    def test_suite_success_rejects_skips_even_when_test_runner_is_successful(self) -> None:
        self.assertFalse(qualification.successful_suite({
            "pattern": "test_vendor_endpoint_qualification_runner.py", "state": "passed", "tests_run": 1, "failures": 0, "errors": 0,
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
