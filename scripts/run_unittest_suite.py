#!/usr/bin/env python3
"""Fail-closed unittest discovery runner for repository CI.

Python's unittest discovery can exit successfully when it discovers zero tests.
This wrapper makes the minimum test count explicit and verifies the executed
count matches discovery before returning success.
"""
from __future__ import annotations

import argparse
import sys
import unittest
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("tests_dir", help="Directory containing test_*.py files")
    parser.add_argument(
        "--minimum-tests",
        type=int,
        default=1,
        help="Minimum discovered test cases required to proceed (default: 1)",
    )
    parser.add_argument(
        "--pattern",
        default="test_*.py",
        help="unittest discovery filename pattern (default: test_*.py)",
    )
    args = parser.parse_args()

    if args.minimum_tests < 1:
        parser.error("--minimum-tests must be at least 1")

    tests_dir = Path(args.tests_dir).resolve()
    if not tests_dir.is_dir():
        print(f"ERROR: test directory does not exist: {tests_dir}", file=sys.stderr)
        return 2

    # Test modules in these suites import their sibling domain/model modules.
    sys.path.insert(0, str(tests_dir.parent))

    loader = unittest.TestLoader()
    suite = loader.discover(start_dir=str(tests_dir), pattern=args.pattern)
    discovered = suite.countTestCases()
    print(f"DISCOVERED_TEST_CASES={discovered}")

    if loader.errors:
        print("ERROR: unittest discovery reported import or loading errors:", file=sys.stderr)
        for error in loader.errors:
            print(error, file=sys.stderr)
        return 2

    if discovered < args.minimum_tests:
        print(
            f"ERROR: discovered {discovered} tests; minimum required is "
            f"{args.minimum_tests}. Refusing to report success.",
            file=sys.stderr,
        )
        return 2

    result = unittest.TextTestRunner(verbosity=2).run(suite)
    print(f"EXECUTED_TEST_CASES={result.testsRun}")
    if not result.wasSuccessful():
        return 1
    if result.testsRun != discovered:
        print(
            f"ERROR: executed {result.testsRun} tests but discovered {discovered}; "
            "refusing to report success.",
            file=sys.stderr,
        )
        return 2

    print(f"TEST_SUITE_RESULT=PASS tests={result.testsRun}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
