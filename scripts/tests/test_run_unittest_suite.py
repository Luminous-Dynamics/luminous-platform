"""Regression tests for the fail-closed unittest runner itself.

These tests create temporary toy suites and invoke the real runner as a subprocess.
They prove that empty discovery and deliberate failures return non-zero before the
runner is relied upon to qualify the real contract suites.
"""
from __future__ import annotations

import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


RUNNER = Path(__file__).resolve().parents[1] / "run_unittest_suite.py"


class FailClosedRunnerTests(unittest.TestCase):
    def run_runner(self, files: dict[str, str], minimum: int = 1, *, missing_dir: bool = False):
        with tempfile.TemporaryDirectory(prefix="luminous-runner-test-") as temporary:
            root = Path(temporary)
            test_dir = root / "tests"
            if not missing_dir:
                test_dir.mkdir()
                for name, source in files.items():
                    (test_dir / name).write_text(source, encoding="utf-8")
            env = os.environ.copy()
            env["PYTHONDONTWRITEBYTECODE"] = "1"
            return subprocess.run(
                [
                    sys.executable,
                    str(RUNNER),
                    str(test_dir),
                    "--minimum-tests",
                    str(minimum),
                ],
                capture_output=True,
                text=True,
                env=env,
                timeout=15,
                check=False,
            )

    def test_empty_discovery_fails_instead_of_reporting_pass(self):
        result = self.run_runner({})
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("DISCOVERED_TEST_CASES=0", result.stdout)
        self.assertNotIn("TEST_SUITE_RESULT=PASS", result.stdout)

    def test_minimum_discovery_floor_is_enforced(self):
        result = self.run_runner(
            {
                "test_sample.py": (
                    "import unittest\n"
                    "class Sample(unittest.TestCase):\n"
                    " def test_one(self): self.assertTrue(True)\n"
                )
            },
            minimum=2,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("minimum required is 2", result.stderr)
        self.assertNotIn("TEST_SUITE_RESULT=PASS", result.stdout)

    def test_a_passing_suite_reports_exact_executed_count(self):
        result = self.run_runner(
            {
                "test_sample.py": (
                    "import unittest\n"
                    "class Sample(unittest.TestCase):\n"
                    " def test_one(self): self.assertTrue(True)\n"
                )
            }
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("DISCOVERED_TEST_CASES=1", result.stdout)
        self.assertIn("EXECUTED_TEST_CASES=1", result.stdout)
        self.assertIn("TEST_SUITE_RESULT=PASS tests=1", result.stdout)

    def test_deliberately_failing_sentinel_fails_the_runner(self):
        result = self.run_runner(
            {
                "test_sentinel.py": (
                    "import unittest\n"
                    "class Sentinel(unittest.TestCase):\n"
                    " def test_deliberate_failure(self): self.fail('sentinel detected')\n"
                )
            }
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("sentinel detected", result.stderr)
        self.assertNotIn("TEST_SUITE_RESULT=PASS", result.stdout)

    def test_broken_test_import_fails_closed(self):
        result = self.run_runner(
            {
                "test_broken.py": (
                    "import module_that_must_not_exist_for_runner_self_test\n"
                )
            }
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("discovery reported import or loading errors", result.stderr)
        self.assertNotIn("TEST_SUITE_RESULT=PASS", result.stdout)

    def test_missing_test_directory_fails_closed(self):
        result = self.run_runner({}, missing_dir=True)
        self.assertEqual(result.returncode, 2)
        self.assertIn("test directory does not exist", result.stderr)


if __name__ == "__main__":
    unittest.main(verbosity=2)
