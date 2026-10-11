import importlib.util
from pathlib import Path
import unittest

SCRIPT = Path(__file__).with_name("run_security_audit_policy_tests.py")
spec = importlib.util.spec_from_file_location("security_audit_test_runner", SCRIPT)
runner = importlib.util.module_from_spec(spec)
assert spec.loader is not None
spec.loader.exec_module(runner)


class SecurityPolicyTestRunnerTests(unittest.TestCase):
    def test_expected_inventory_has_exact_required_suites_and_counts(self):
        self.assertEqual(set(runner.EXPECTED_TEST_METHODS), {
            "test_verify_security_audit_trust_anchor.py",
            "test_audit_security_required_checks.py",
            "test_run_security_audit_policy_tests.py",
        })
        self.assertEqual(set(runner.EXPECTED_TEST_NAMES), set(runner.EXPECTED_TEST_METHODS))
        self.assertEqual(sum(runner.EXPECTED_TEST_METHODS.values()), 81)
        for filename, names in runner.EXPECTED_TEST_NAMES.items():
            self.assertEqual(len(names), runner.EXPECTED_TEST_METHODS[filename])
            self.assertEqual(names, tuple(sorted(names)))

    def test_exact_inventory_is_accepted(self):
        self.assertEqual(
            runner.validate_test_inventory(dict(runner.EXPECTED_TEST_METHODS)),
            [],
        )
        self.assertEqual(
            runner.validate_test_names(dict(runner.EXPECTED_TEST_NAMES)),
            [],
        )

    def test_missing_suite_is_rejected(self):
        actual = dict(runner.EXPECTED_TEST_METHODS)
        actual.pop("test_audit_security_required_checks.py")
        errors = runner.validate_test_inventory(actual)
        self.assertTrue(any("required test suite missing" in error for error in errors))
        name_errors = runner.validate_test_names({
            key: value for key, value in runner.EXPECTED_TEST_NAMES.items()
            if key != "test_audit_security_required_checks.py"
        })
        self.assertTrue(any("required test-name inventory missing" in error for error in name_errors))

    def test_wrong_test_method_count_is_rejected(self):
        actual = dict(runner.EXPECTED_TEST_METHODS)
        actual["test_verify_security_audit_trust_anchor.py"] = 0
        errors = runner.validate_test_inventory(actual)
        self.assertTrue(any("expected 35 test methods, observed 0" in error for error in errors))

        # Keep the suite count unchanged but replace a reviewed test name; the
        # exact method-name gate must catch the drift too.
        changed_names = dict(runner.EXPECTED_TEST_NAMES)
        filename = "test_audit_security_required_checks.py"
        names = list(changed_names[filename])
        names[0] = "test_unreviewed_replacement"
        changed_names[filename] = tuple(sorted(names))
        name_errors = runner.validate_test_names(changed_names)
        self.assertTrue(any("test-name inventory differs" in error for error in name_errors))

    def test_result_count_must_match_exact_inventory(self):
        result = unittest.TestResult()
        result.testsRun = 1
        self.assertTrue(runner.result_is_qualified(result, 1))
        self.assertFalse(runner.result_is_qualified(result, 2))

    def test_runner_rejects_green_results_with_skips_or_expected_failures(self):
        skipped = unittest.TestResult()
        skipped.testsRun = 1
        skipped.skipped.append((self.test_exact_inventory_is_accepted, "synthetic skip"))
        self.assertTrue(skipped.wasSuccessful())
        self.assertFalse(runner.result_is_qualified(skipped, 1))

        expected_failure = unittest.TestResult()
        expected_failure.testsRun = 1
        expected_failure.expectedFailures.append((self.test_exact_inventory_is_accepted, "synthetic expected failure"))
        self.assertTrue(expected_failure.wasSuccessful())
        self.assertFalse(runner.result_is_qualified(expected_failure, 1))

        unexpected_success = unittest.TestResult()
        unexpected_success.testsRun = 1
        unexpected_success.unexpectedSuccesses.append(self.test_exact_inventory_is_accepted)
        self.assertTrue(unexpected_success.wasSuccessful())
        self.assertFalse(runner.result_is_qualified(unexpected_success, 1))

    def test_duplicate_or_additional_suite_is_rejected(self):
        actual = dict(runner.EXPECTED_TEST_METHODS)
        actual["test_unreviewed.py"] = 1
        errors = runner.validate_test_inventory(actual)
        self.assertTrue(any("unexpected test suite present" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
