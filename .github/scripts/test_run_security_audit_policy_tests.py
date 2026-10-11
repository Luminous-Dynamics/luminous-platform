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
        self.assertEqual(sum(runner.EXPECTED_TEST_METHODS.values()), 71)

    def test_exact_inventory_is_accepted(self):
        self.assertEqual(
            runner.validate_test_inventory(dict(runner.EXPECTED_TEST_METHODS)),
            [],
        )

    def test_missing_suite_is_rejected(self):
        actual = dict(runner.EXPECTED_TEST_METHODS)
        actual.pop("test_audit_security_required_checks.py")
        errors = runner.validate_test_inventory(actual)
        self.assertTrue(any("required test suite missing" in error for error in errors))

    def test_wrong_test_method_count_is_rejected(self):
        actual = dict(runner.EXPECTED_TEST_METHODS)
        actual["test_verify_security_audit_trust_anchor.py"] = 0
        errors = runner.validate_test_inventory(actual)
        self.assertTrue(any("expected 34 test methods, observed 0" in error for error in errors))

    def test_duplicate_or_additional_suite_is_rejected(self):
        actual = dict(runner.EXPECTED_TEST_METHODS)
        actual["test_unreviewed.py"] = 1
        errors = runner.validate_test_inventory(actual)
        self.assertTrue(any("unexpected test suite present" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
