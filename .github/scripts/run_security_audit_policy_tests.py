#!/usr/bin/env python3
"""Fail-closed runner for the independent security-audit policy test inventory."""
from __future__ import annotations

import importlib.util
from pathlib import Path
import sys
import unittest
from typing import Any

SCRIPT_DIR = Path(__file__).resolve().parent
EXPECTED_TEST_METHODS: dict[str, int] = {
    "test_verify_security_audit_trust_anchor.py": 35,
    "test_audit_security_required_checks.py": 35,
    "test_run_security_audit_policy_tests.py": 7,
}


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
    try:
        for filename in EXPECTED_TEST_METHODS:
            suites[filename] = load_test_suite(filename)
    except Exception as exc:
        print(f"FAIL: unable to load required policy test inventory: {type(exc).__name__}: {exc}", file=sys.stderr)
        return 1

    counts = {name: suite.countTestCases() for name, suite in suites.items()}
    errors = validate_test_inventory(counts)
    if errors:
        for error in errors:
            print(f"FAIL: {error}", file=sys.stderr)
        return 1

    combined = unittest.TestSuite()
    for filename in EXPECTED_TEST_METHODS:
        combined.addTests(suites[filename])
    expected_total = sum(EXPECTED_TEST_METHODS.values())
    result = unittest.TextTestRunner(verbosity=2).run(combined)
    if not result_is_qualified(result, expected_total):
        print(
            "FAIL: required policy test run did not execute the exact inventory "
            f"(expected={expected_total}, executed={result.testsRun}, "
            f"skipped={len(result.skipped)}, expected_failures={len(result.expectedFailures)}, "
            f"unexpected_successes={len(result.unexpectedSuccesses)}, "
            f"failures={len(result.failures)}, errors={len(result.errors)})",
            file=sys.stderr,
        )
        return 1
    print(
        "PASS: exact policy test inventory executed "
        f"({', '.join(f'{name}={counts[name]}' for name in EXPECTED_TEST_METHODS)}; "
        f"total={result.testsRun})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
