#!/usr/bin/env python3
"""Fail-closed runner for the independent security-audit policy test inventory."""
from __future__ import annotations

import hashlib
import importlib.util
import json
import os
from pathlib import Path
import platform
import re
import subprocess
import sys
import unittest
from datetime import datetime, timezone
from typing import Any

SCRIPT_DIR = Path(__file__).resolve().parent
REPOSITORY_ROOT = SCRIPT_DIR.parent.parent
RECEIPT_PATH = REPOSITORY_ROOT / "security-audit-policy-test-receipt.json"
RECEIPT_SCHEMA = "luminous.security-audit.policy-test-receipt.v1"
EXPECTED_TEST_METHODS: dict[str, int] = {
    "test_verify_security_audit_trust_anchor.py": 35,
    "test_audit_security_required_checks.py": 39,
    "test_run_security_audit_policy_tests.py": 7,
}

EXPECTED_TEST_NAMES: dict[str, tuple[str, ...]] = {
    "test_verify_security_audit_trust_anchor.py": (
        "test_api_rejects_bad_json",
        "test_artifact_digest_and_run_metadata_are_checked",
        "test_artifact_evidence_file_digest_mismatch_is_rejected",
        "test_artifact_missing_manifest_file_is_rejected",
        "test_artifact_without_digest_or_with_expiry_is_rejected",
        "test_artifact_zip_path_traversal_is_rejected",
        "test_authoritative_pr_merge_sha_rejects_mismatched_pr_state",
        "test_best_effort_failure_status_clears_same_head_success",
        "test_best_effort_failure_status_rejects_untrusted_or_invalid_identity",
        "test_best_effort_failure_status_survives_unexpected_status_api_failure",
        "test_binding_rejects_wrong_path_fork_sha_and_event",
        "test_blob_mismatch_fails_closed",
        "test_blob_type_missing_and_mismatch_fail_closed",
        "test_duplicate_json_keys_are_rejected_even_when_artifact_digest_matches",
        "test_nested_verdict_path_is_rejected",
        "test_noncanonical_sha_is_rejected",
        "test_only_latest_exact_head_run_attempt_is_eligible",
        "test_pass_with_findings_is_distinct_and_consistent",
        "test_pending_status_only_for_uncompleted_runs",
        "test_policy_pins_are_canonical",
        "test_pr_merge_state_must_be_known_and_not_behind",
        "test_pr_test_merge_sha_recheck_accepts_unchanged_commit",
        "test_pr_test_merge_sha_recheck_rejects_changed_commit",
        "test_run_pr_association_must_match_exact_open_pr",
        "test_valid_run_binding_and_expected_attempt",
        "test_verdict_accepts_only_exact_subject_and_required_lanes",
        "test_verdict_engine_commit_is_policy_bound",
        "test_verdict_freshness_rejects_stale_and_future_timestamps",
        "test_verdict_ref_must_match_exact_pull_request_number",
        "test_verdict_rejects_wrong_workflow_ref_url_and_unknown_fields",
        "test_verdict_workflow_path_must_match_authoritative_run_path",
        "test_verdict_workflow_sha_must_match_authoritative_merge_sha",
        "test_verifier_self_tests_require_explicit_success",
        "test_workflow_status_write_is_scoped_to_verifier_job",
        "test_wrong_head_or_workflow_is_never_latest",
    ),
    "test_audit_security_required_checks.py": (
        "test_audit_repository_fails_if_default_branch_changed",
        "test_audit_repository_qualifies_bound_active_rules_even_if_classic_api_forbidden",
        "test_classic_contexts_merge_checks_and_legacy_fields",
        "test_classic_loose_policy_fails_even_when_contexts_exist",
        "test_classic_missing_app_binding_cannot_be_masked_by_ruleset",
        "test_classic_protection_missing_app_id_does_not_qualify",
        "test_classic_protection_name_only_contexts_do_not_qualify",
        "test_classic_protection_strict_checks_array_qualifies",
        "test_classic_protection_wrong_app_id_does_not_qualify",
        "test_classic_wrong_app_binding_cannot_be_masked_by_ruleset",
        "test_conflict_is_not_masked_by_malformed_classic_response",
        "test_effective_rules_api_failure_is_incomplete",
        "test_forbidden_classic_policy_is_incomplete_not_pass",
        "test_help_does_not_require_a_token",
        "test_malformed_classic_protection_is_incomplete",
        "test_malformed_repository_metadata_is_incomplete",
        "test_malformed_ruleset_provenance_is_incomplete_not_pass",
        "test_missing_classic_protection_fails_when_no_strict_ruleset_exists",
        "test_missing_token_receipt_binds_exact_tool_and_runtime",
        "test_non_list_effective_rules_are_incomplete",
        "test_oversized_rules_response_is_incomplete",
        "test_page_limit_with_qualified_contexts_is_still_incomplete",
        "test_page_limit_with_unresolved_contexts_is_incomplete",
        "test_policy_digest_is_deterministic_and_policy_sensitive",
        "test_policy_requires_default_branch_match",
        "test_receipt_policy_names_all_three_repositories",
        "test_required_contexts_can_be_split_across_strict_active_rulesets",
        "test_required_contexts_can_be_split_between_active_ruleset_and_classic_protection",
        "test_ruleset_conflicting_duplicate_binding_does_not_qualify",
        "test_ruleset_duplicate_context_with_conflicting_app_binding_fails",
        "test_ruleset_missing_integration_id_does_not_qualify",
        "test_ruleset_missing_producer_check_does_not_qualify",
        "test_ruleset_missing_verifier_status_does_not_qualify",
        "test_ruleset_with_loose_policy_does_not_qualify",
        "test_ruleset_wrong_integration_id_does_not_qualify",
        "test_strict_active_ruleset_requires_both_exact_contexts",
        "test_unhashable_ruleset_source_type_is_incomplete_not_crash",
        "test_unknown_and_malformed_rules_never_qualify",
        "test_unknown_cli_options_are_not_silently_ignored",
    ),
    "test_run_security_audit_policy_tests.py": (
        "test_duplicate_or_additional_suite_is_rejected",
        "test_exact_inventory_is_accepted",
        "test_expected_inventory_has_exact_required_suites_and_counts",
        "test_missing_suite_is_rejected",
        "test_result_count_must_match_exact_inventory",
        "test_runner_rejects_green_results_with_skips_or_expected_failures",
        "test_wrong_test_method_count_is_rejected",
    ),
}



def file_sha256(path: Path) -> str | None:
    try:
        return hashlib.sha256(path.read_bytes()).hexdigest()
    except OSError:
        return None


def current_git_sha() -> str | None:
    try:
        result = subprocess.run(
            ["git", "rev-parse", "HEAD"],
            cwd=REPOSITORY_ROOT,
            check=True,
            capture_output=True,
            text=True,
            timeout=10,
        )
    except (OSError, subprocess.SubprocessError):
        return None
    value = result.stdout.strip()
    return value if re.fullmatch(r"[0-9a-f]{40}", value) else None


def build_receipt(
    status: str,
    counts: dict[str, int],
    method_names: dict[str, tuple[str, ...]],
    result: Any | None,
    inventory_errors: list[str],
) -> dict[str, Any]:
    """Bind the test outcome to the actual checkout, source bytes and test inventory."""
    checkout_sha = current_git_sha()
    expected_head = os.environ.get("EXPECTED_TEST_HEAD_SHA", "").strip()
    exact_head_match = (
        bool(re.fullmatch(r"[0-9a-f]{40}", expected_head))
        and checkout_sha == expected_head
    )
    evidence_paths = [
        ".github/scripts/verify_security_audit_trust_anchor.py",
        ".github/scripts/test_verify_security_audit_trust_anchor.py",
        ".github/scripts/audit_security_required_checks.py",
        ".github/scripts/test_audit_security_required_checks.py",
        ".github/scripts/run_security_audit_policy_tests.py",
        ".github/scripts/test_run_security_audit_policy_tests.py",
        ".github/workflows/security-audit-verifier-tests.yml",
    ]
    source_hashes = {
        relative: file_sha256(REPOSITORY_ROOT / relative)
        for relative in evidence_paths
    }
    source_hashes_complete = all(
        isinstance(value, str) and bool(re.fullmatch(r"[0-9a-f]{64}", value))
        for value in source_hashes.values()
    )
    normalized_inventory = {
        "expected_method_counts": EXPECTED_TEST_METHODS,
        "expected_method_names": EXPECTED_TEST_NAMES,
        "observed_method_counts": counts,
        "observed_method_names": method_names,
    }
    inventory_bytes = json.dumps(
        normalized_inventory, sort_keys=True, separators=(",", ":"), ensure_ascii=True
    ).encode("utf-8")
    metrics = {
        "tests_run": getattr(result, "testsRun", 0) if result is not None else 0,
        "was_successful": result.wasSuccessful() if result is not None else False,
        "skipped": len(result.skipped) if result is not None else 0,
        "expected_failures": len(result.expectedFailures) if result is not None else 0,
        "unexpected_successes": len(result.unexpectedSuccesses) if result is not None else 0,
        "failures": len(result.failures) if result is not None else 0,
        "errors": len(result.errors) if result is not None else 0,
    }
    qualified = (
        status == "PASS"
        and exact_head_match
        and source_hashes_complete
        and not inventory_errors
        and result is not None
        and result_is_qualified(result, sum(EXPECTED_TEST_METHODS.values()))
    )
    server = os.environ.get("GITHUB_SERVER_URL", "").rstrip("/")
    repository = os.environ.get("GITHUB_REPOSITORY", "")
    run_id = os.environ.get("GITHUB_RUN_ID", "")
    run_url = f"{server}/{repository}/actions/runs/{run_id}" if server and repository and run_id else None
    return {
        "schema": RECEIPT_SCHEMA,
        "status": "PASS" if qualified else "FAIL",
        "generated_at_utc": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "repository": repository or None,
        "event_name": os.environ.get("GITHUB_EVENT_NAME") or None,
        "workflow_run_url": run_url,
        "run_attempt": os.environ.get("GITHUB_RUN_ATTEMPT") or None,
        "expected_head_sha": expected_head or None,
        "checked_out_head_sha": checkout_sha,
        "exact_head_match": exact_head_match,
        "python_implementation": platform.python_implementation(),
        "python_version": platform.python_version(),
        "source_sha256": source_hashes,
        "inventory_sha256": hashlib.sha256(inventory_bytes).hexdigest(),
        "expected_method_counts": EXPECTED_TEST_METHODS,
        "observed_method_counts": counts,
        "expected_method_names": EXPECTED_TEST_NAMES,
        "observed_method_names": method_names,
        "inventory_errors": inventory_errors,
        "test_result": metrics,
    }


def write_receipt(receipt: dict[str, Any]) -> bool:
    try:
        RECEIPT_PATH.write_text(
            json.dumps(receipt, sort_keys=True, indent=2, ensure_ascii=True) + "\n",
            encoding="utf-8",
        )
    except OSError as exc:
        print(
            f"FAIL: unable to write exact-head test receipt: {type(exc).__name__}: {exc}",
            file=sys.stderr,
        )
        return False
    return True


def validate_test_inventory(actual: dict[str, int],
                            expected: dict[str, int] = EXPECTED_TEST_METHODS) -> list[str]:
    """Return errors unless the exact expected files and method counts are present."""
    errors: list[str] = []
    expected_names = set(expected)
    actual_names = set(actual)
    for name in sorted(expected_names - actual_names):
        errors.append(f"required test suite missing: {name}")
    for name in sorted(actual_names - expected_names):
        errors.append(f"unexpected test suite present: {name}")
    for name in sorted(expected_names & actual_names):
        if type(actual[name]) is not int or actual[name] != expected[name]:
            errors.append(f"{name}: expected {expected[name]} test methods, observed {actual[name]!r}")
    return errors


def validate_test_names(
    actual: dict[str, tuple[str, ...]],
    expected: dict[str, tuple[str, ...]] = EXPECTED_TEST_NAMES,
) -> list[str]:
    """Require the exact reviewed method-name set, not merely a matching count."""
    errors: list[str] = []
    expected_files = set(expected)
    actual_files = set(actual)
    for filename in sorted(expected_files - actual_files):
        errors.append(f"required test-name inventory missing: {filename}")
    for filename in sorted(actual_files - expected_files):
        errors.append(f"unexpected test-name inventory: {filename}")
    for filename in sorted(expected_files & actual_files):
        methods = actual[filename]
        if len(methods) != len(set(methods)):
            errors.append(f"{filename}: duplicate test method names detected")
        missing = sorted(set(expected[filename]) - set(methods))
        extra = sorted(set(methods) - set(expected[filename]))
        if missing or extra:
            errors.append(f"{filename}: test-name inventory differs (missing={missing}, extra={extra})")
    return errors


def suite_test_method_names(suite: unittest.TestSuite) -> tuple[str, ...]:
    """Flatten a unittest suite into a stable sorted tuple of test method names."""
    names: list[str] = []
    for item in suite:
        if isinstance(item, unittest.TestSuite):
            names.extend(suite_test_method_names(item))
        else:
            names.append(item.id().rsplit(".", 1)[-1])
    return tuple(sorted(names))


def result_is_qualified(result: Any, expected_total: int) -> bool:
    """Reject green unittest results that skipped, expected-failed or miscounted tests."""
    return (
        result.wasSuccessful()
        and result.testsRun == expected_total
        and not result.skipped
        and not result.expectedFailures
        and not result.unexpectedSuccesses
    )


def load_test_suite(filename: str) -> unittest.TestSuite:
    path = SCRIPT_DIR / filename
    if not path.is_file():
        raise FileNotFoundError(f"required test module is missing: {filename}")
    module_name = f"security_audit_tests_{path.stem}"
    spec = importlib.util.spec_from_file_location(module_name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load required test module: {filename}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    suite = unittest.defaultTestLoader.loadTestsFromModule(module)
    return suite


def main() -> int:
    suites: dict[str, unittest.TestSuite] = {}
    counts: dict[str, int] = {}
    method_names: dict[str, tuple[str, ...]] = {}
    try:
        for filename in EXPECTED_TEST_METHODS:
            suites[filename] = load_test_suite(filename)
    except Exception as exc:
        errors = [f"unable to load required policy test inventory: {type(exc).__name__}: {exc}"]
        write_receipt(build_receipt("FAIL", counts, method_names, None, errors))
        for error in errors:
            print(f"FAIL: {error}", file=sys.stderr)
        return 1

    counts = {name: suite.countTestCases() for name, suite in suites.items()}
    method_names = {name: suite_test_method_names(suite) for name, suite in suites.items()}
    errors = validate_test_inventory(counts) + validate_test_names(method_names)
    if errors:
        receipt_written = write_receipt(build_receipt("FAIL", counts, method_names, None, errors))
        for error in errors:
            print(f"FAIL: {error}", file=sys.stderr)
        return 1

    combined = unittest.TestSuite()
    for filename in EXPECTED_TEST_METHODS:
        combined.addTests(suites[filename])
    expected_total = sum(EXPECTED_TEST_METHODS.values())
    result = unittest.TextTestRunner(verbosity=2).run(combined)
    tests_passed = result_is_qualified(result, expected_total)
    receipt = build_receipt(
        "PASS" if tests_passed else "FAIL", counts, method_names, result, []
    )
    receipt_written = write_receipt(receipt)
    if receipt["status"] != "PASS" or not receipt_written:
        print(
            "FAIL: exact-head policy test qualification failed "
            f"(status={receipt['status']}, head_match={receipt['exact_head_match']}, "
            f"expected={expected_total}, executed={result.testsRun}, "
            f"skipped={len(result.skipped)}, expected_failures={len(result.expectedFailures)}, "
            f"unexpected_successes={len(result.unexpectedSuccesses)}, "
            f"failures={len(result.failures)}, errors={len(result.errors)}, "
            f"receipt_written={receipt_written})",
            file=sys.stderr,
        )
        return 1
    print(
        "PASS: exact policy test inventory executed "
        f"({', '.join(f'{name}={counts[name]}' for name in EXPECTED_TEST_METHODS)}; "
        f"total={result.testsRun}); exact-head receipt written to {RECEIPT_PATH.name}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
