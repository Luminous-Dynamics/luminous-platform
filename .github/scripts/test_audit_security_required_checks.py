import importlib.util
from pathlib import Path
import unittest

SCRIPT = Path(__file__).with_name("audit_security_required_checks.py")
spec = importlib.util.spec_from_file_location("required_check_auditor", SCRIPT)
module = importlib.util.module_from_spec(spec)
assert spec.loader is not None
spec.loader.exec_module(module)


class RequiredCheckPolicyTests(unittest.TestCase):
    def setUp(self):
        self.required = {
            "Security Audit / Independent Verifier",
            "Shared security audit / Deterministic audit verdict and evidence completeness",
        }

    def strict_rule(self, contexts=None, strict=True):
        return {
            "type": "required_status_checks",
            "ruleset_id": 42,
            "parameters": {
                "strict_required_status_checks_policy": strict,
                "required_status_checks": [
                    {"context": context} for context in (
                        sorted(self.required) if contexts is None else contexts
                    )
                ],
            },
        }

    def test_strict_active_ruleset_requires_both_exact_contexts(self):
        decision = module.assess_policy([self.strict_rule()], None, 404, self.required)
        self.assertEqual(decision["status"], "PASS")
        self.assertEqual(decision["source"], "ruleset:42")
        self.assertEqual(decision["missing_contexts"], [])

    def test_ruleset_with_loose_policy_does_not_qualify(self):
        decision = module.assess_policy([self.strict_rule(strict=False)], None, 404, self.required)
        self.assertEqual(decision["status"], "FAIL")
        self.assertEqual(set(decision["missing_contexts"]), self.required)

    def test_ruleset_missing_producer_check_does_not_qualify(self):
        decision = module.assess_policy([
            self.strict_rule(contexts=["Security Audit / Independent Verifier"])
        ], None, 404, self.required)
        self.assertEqual(decision["status"], "FAIL")
        self.assertIn(next(x for x in self.required if x.startswith("Shared security audit")), decision["missing_contexts"])

    def test_ruleset_missing_verifier_status_does_not_qualify(self):
        decision = module.assess_policy([
            self.strict_rule(contexts=["Shared security audit / Deterministic audit verdict and evidence completeness"])
        ], None, 404, self.required)
        self.assertEqual(decision["status"], "FAIL")
        self.assertIn("Security Audit / Independent Verifier", decision["missing_contexts"])

    def test_classic_protection_strict_checks_array_qualifies(self):
        classic = {
            "strict": True,
            "checks": [{"context": context, "app_id": 15368} for context in self.required],
            "contexts": [],
        }
        decision = module.assess_policy([], classic, None, self.required)
        self.assertEqual(decision["status"], "PASS")
        self.assertEqual(decision["source"], "classic-branch-protection")

    def test_classic_protection_legacy_contexts_qualifies(self):
        classic = {"strict": True, "contexts": sorted(self.required)}
        decision = module.assess_policy([], classic, None, self.required)
        self.assertEqual(decision["status"], "PASS")

    def test_classic_loose_policy_fails_even_when_contexts_exist(self):
        classic = {"strict": False, "contexts": sorted(self.required)}
        decision = module.assess_policy([], classic, None, self.required)
        self.assertEqual(decision["status"], "FAIL")
        self.assertEqual(decision["missing_contexts"], [])

    def test_missing_classic_protection_fails_when_no_strict_ruleset_exists(self):
        decision = module.assess_policy([], None, 404, self.required)
        self.assertEqual(decision["status"], "FAIL")

    def test_forbidden_classic_policy_is_incomplete_not_pass(self):
        decision = module.assess_policy([], None, 403, self.required)
        self.assertEqual(decision["status"], "INCOMPLETE")

    def test_non_list_effective_rules_are_incomplete(self):
        decision = module.assess_policy({"rules": []}, None, 404, self.required)
        self.assertEqual(decision["status"], "INCOMPLETE")

    def test_unknown_and_malformed_rules_never_qualify(self):
        decision = module.assess_policy([
            {"type": "required_status_checks", "parameters": {
                "strict_required_status_checks_policy": "true",
                "required_status_checks": [{"context": context} for context in self.required],
            }}
        ], None, 404, self.required)
        self.assertEqual(decision["status"], "FAIL")

    def test_classic_contexts_merge_checks_and_legacy_fields(self):
        contexts = module.classic_contexts({
            "contexts": ["Security Audit / Independent Verifier"],
            "checks": [{"context": "Shared security audit / Deterministic audit verdict and evidence completeness"}],
        })
        self.assertEqual(contexts, self.required)

    def test_policy_requires_default_branch_match(self):
        # The repository-default branch consistency is enforced in audit_repository;
        # this asserts that policy always names the branch that must be checked.
        for policy in module.POLICY.values():
            self.assertEqual(policy["default_branch"], "main")

    def test_receipt_policy_names_all_three_repositories(self):
        self.assertEqual(set(module.POLICY), {
            "Luminous-Dynamics/mycelix",
            "Luminous-Dynamics/symthaea",
            "Luminous-Dynamics/luminous-platform",
        })
        for repo, policy in module.POLICY.items():
            with self.subTest(repo=repo):
                self.assertIn("Deterministic audit verdict and evidence completeness", policy["producer_context"])
                self.assertEqual(policy["default_branch"], "main")


if __name__ == "__main__":
    unittest.main()
