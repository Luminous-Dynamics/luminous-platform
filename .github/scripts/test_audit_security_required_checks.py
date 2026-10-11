import hashlib
import importlib.util
import io
import json
from contextlib import redirect_stdout
from pathlib import Path
import unittest
from unittest.mock import patch

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
            "ruleset_source_type": "Repository",
            "ruleset_source": "owner/repo",
            "parameters": {
                "strict_required_status_checks_policy": strict,
                "required_status_checks": [
                    {"context": context, "integration_id": module.GITHUB_ACTIONS_INTEGRATION_ID}
                    for context in (sorted(self.required) if contexts is None else contexts)
                ],
            },
        }

    def test_strict_active_ruleset_requires_both_exact_contexts(self):
        decision = module.assess_policy([self.strict_rule()], None, 404, self.required)
        self.assertEqual(decision["status"], "PASS")
        self.assertEqual(decision["source"], ["Repository:owner/repo#42"])
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
            "checks": [{"context": context, "app_id": module.GITHUB_ACTIONS_INTEGRATION_ID} for context in self.required],
            "contexts": [],
        }
        decision = module.assess_policy([], classic, None, self.required)
        self.assertEqual(decision["status"], "PASS")
        self.assertEqual(decision["source"], ["classic-branch-protection"])

    def test_classic_protection_name_only_contexts_do_not_qualify(self):
        classic = {"strict": True, "contexts": sorted(self.required)}
        decision = module.assess_policy([], classic, None, self.required)
        self.assertEqual(decision["status"], "FAIL")
        self.assertEqual(set(decision["missing_contexts"]), self.required)

    def test_classic_protection_wrong_app_id_does_not_qualify(self):
        classic = {
            "strict": True,
            "checks": [{"context": context, "app_id": 99999} for context in self.required],
            "contexts": [],
        }
        decision = module.assess_policy([], classic, None, self.required)
        self.assertEqual(decision["status"], "FAIL")
        self.assertEqual(set(decision["missing_contexts"]), self.required)

    def test_classic_protection_missing_app_id_does_not_qualify(self):
        classic = {
            "strict": True,
            "checks": [{"context": context} for context in self.required],
            "contexts": sorted(self.required),
        }
        decision = module.assess_policy([], classic, None, self.required)
        self.assertEqual(decision["status"], "FAIL")
        self.assertEqual(set(decision["missing_contexts"]), self.required)

    def test_classic_loose_policy_fails_even_when_contexts_exist(self):
        classic = {"strict": False, "contexts": sorted(self.required)}
        decision = module.assess_policy([], classic, None, self.required)
        self.assertEqual(decision["status"], "FAIL")
        self.assertEqual(set(decision["missing_contexts"]), self.required)

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
            {"type": "required_status_checks", "ruleset_id": 50,
             "ruleset_source_type": "Repository", "ruleset_source": "owner/repo",
             "parameters": {
                "strict_required_status_checks_policy": "true",
                "required_status_checks": [{"context": context} for context in self.required],
             }}
        ], None, 404, self.required)
        self.assertEqual(decision["status"], "INCOMPLETE")

    def test_classic_contexts_merge_checks_and_legacy_fields(self):
        contexts = module.classic_contexts({
            "contexts": ["Security Audit / Independent Verifier"],
            "checks": [{"context": "Shared security audit / Deterministic audit verdict and evidence completeness"}],
        })
        self.assertEqual(contexts, self.required)

    def test_required_contexts_can_be_split_between_active_ruleset_and_classic_protection(self):
        producer = self.strict_rule(contexts=[
            "Shared security audit / Deterministic audit verdict and evidence completeness"
        ])
        classic = {
            "strict": True,
            "contexts": [],
            "checks": [{"context": "Security Audit / Independent Verifier",
                        "app_id": module.GITHUB_ACTIONS_INTEGRATION_ID}],
        }
        result = module.assess_policy([producer], classic, None, self.required)
        self.assertEqual(result["status"], "PASS")
        self.assertEqual(set(result["observed_contexts"]), self.required)
        self.assertEqual(set(result["source"]), {"Repository:owner/repo#42", "classic-branch-protection"})

    def test_ruleset_duplicate_context_with_conflicting_app_binding_fails(self):
        correct = self.strict_rule()
        conflicting = self.strict_rule(contexts=["Security Audit / Independent Verifier"])
        conflicting["ruleset_id"] = 43
        conflicting["ruleset_source_type"] = "Organization"
        conflicting["ruleset_source"] = "Luminous-Dynamics"
        conflicting["parameters"]["required_status_checks"][0]["integration_id"] = 99999
        result = module.assess_policy([correct, conflicting], None, 404, self.required)
        self.assertEqual(result["status"], "FAIL")
        self.assertIn("Security Audit / Independent Verifier", result["conflicting_contexts"])

    def test_classic_missing_app_binding_cannot_be_masked_by_ruleset(self):
        correct = self.strict_rule()
        classic = {
            "strict": True,
            "contexts": [],
            "checks": [{"context": context} for context in self.required],
        }
        result = module.assess_policy([correct], classic, None, self.required)
        self.assertEqual(result["status"], "FAIL")
        self.assertEqual(set(result["conflicting_contexts"]), self.required)

    def test_classic_wrong_app_binding_cannot_be_masked_by_ruleset(self):
        correct = self.strict_rule()
        classic = {
            "strict": True,
            "contexts": [],
            "checks": [
                {"context": context, "app_id": 777}
                for context in self.required
            ],
        }
        result = module.assess_policy([correct], classic, None, self.required)
        self.assertEqual(result["status"], "FAIL")
        self.assertEqual(set(result["conflicting_contexts"]), self.required)

    def test_malformed_classic_protection_is_incomplete(self):
        result = module.assess_policy([], {"contexts": sorted(self.required)}, None, self.required)
        self.assertEqual(result["status"], "INCOMPLETE")

    def test_required_contexts_can_be_split_across_strict_active_rulesets(self):
        producer = self.strict_rule(contexts=[
            "Shared security audit / Deterministic audit verdict and evidence completeness"
        ])
        verifier = self.strict_rule(contexts=["Security Audit / Independent Verifier"])
        verifier["ruleset_id"] = 43
        verifier["ruleset_source_type"] = "Organization"
        verifier["ruleset_source"] = "Luminous-Dynamics"
        result = module.assess_policy([producer, verifier], None, 404, self.required)
        self.assertEqual(result["status"], "PASS")
        self.assertEqual(result["source"], [
            "Organization:Luminous-Dynamics#43",
            "Repository:owner/repo#42",
        ])

    def test_malformed_ruleset_provenance_is_incomplete_not_pass(self):
        producer = self.strict_rule()
        producer.pop("ruleset_source_type")
        result = module.assess_policy([producer], None, 404, self.required)
        self.assertEqual(result["status"], "INCOMPLETE")

    def test_oversized_rules_response_is_incomplete(self):
        rules = [{"type": "deletion", "ruleset_id": i + 1,
                  "ruleset_source_type": "Repository", "ruleset_source": "owner/repo",
                  "parameters": {}} for i in range(101)]
        result = module.assess_policy(rules, None, None, self.required)
        self.assertEqual(result["status"], "INCOMPLETE")
        self.assertIn("exceeds the documented", result["reason"])

    def test_page_limit_with_qualified_contexts_is_still_incomplete(self):
        rules = [self.strict_rule()]
        rules.extend({"type": "deletion", "ruleset_id": i + 100,
                      "ruleset_source_type": "Repository", "ruleset_source": "owner/repo",
                      "parameters": {}} for i in range(99))
        result = module.assess_policy(rules, None, 404, self.required)
        self.assertEqual(result["status"], "INCOMPLETE")
        self.assertIn("pagination completeness is unverified", result["reason"])

    def test_conflict_is_not_masked_by_malformed_classic_response(self):
        correct = self.strict_rule()
        conflict = self.strict_rule(contexts=["Security Audit / Independent Verifier"])
        conflict["ruleset_id"] = 51
        conflict["parameters"]["required_status_checks"][0]["integration_id"] = 777
        result = module.assess_policy([correct, conflict], {"contexts": sorted(self.required)}, None, self.required)
        self.assertEqual(result["status"], "FAIL")
        self.assertIn("Security Audit / Independent Verifier", result["conflicting_contexts"])

    def test_page_limit_with_unresolved_contexts_is_incomplete(self):
        rules = [{"type": "deletion", "ruleset_id": i + 1,
                  "ruleset_source_type": "Repository", "ruleset_source": "owner/repo",
                  "parameters": {}} for i in range(100)]
        result = module.assess_policy(rules, None, 404, self.required)
        self.assertEqual(result["status"], "INCOMPLETE")

    def test_unhashable_ruleset_source_type_is_incomplete_not_crash(self):
        malformed = self.strict_rule()
        malformed["ruleset_source_type"] = ["Repository"]
        result = module.assess_policy([malformed], None, 404, self.required)
        self.assertEqual(result["status"], "INCOMPLETE")

    def test_ruleset_wrong_integration_id_does_not_qualify(self):
        rule = self.strict_rule()
        for item in rule["parameters"]["required_status_checks"]:
            item["integration_id"] = 99999
        decision = module.assess_policy([rule], None, 404, self.required)
        self.assertEqual(decision["status"], "FAIL")
        self.assertEqual(set(decision["missing_contexts"]), self.required)

    def test_ruleset_missing_integration_id_does_not_qualify(self):
        rule = self.strict_rule()
        for item in rule["parameters"]["required_status_checks"]:
            item.pop("integration_id")
        decision = module.assess_policy([rule], None, 404, self.required)
        self.assertEqual(decision["status"], "FAIL")
        self.assertEqual(set(decision["missing_contexts"]), self.required)

    def test_ruleset_conflicting_duplicate_binding_does_not_qualify(self):
        rule = self.strict_rule()
        rule["parameters"]["required_status_checks"].append({
            "context": "Security Audit / Independent Verifier",
            "integration_id": None,
        })
        decision = module.assess_policy([rule], None, 404, self.required)
        self.assertEqual(decision["status"], "FAIL")
        self.assertIn("Security Audit / Independent Verifier", decision["missing_contexts"])

    def test_audit_repository_qualifies_bound_active_rules_even_if_classic_api_forbidden(self):
        repo = "Luminous-Dynamics/mycelix"
        metadata = {"full_name": repo, "default_branch": "main"}
        rule = self.strict_rule()
        with patch.object(module, "get_json", side_effect=[
            metadata, [rule], module.ApiError(403, "not authorized to inspect legacy protection")
        ]) as mocked:
            result = module.audit_repository(repo, module.POLICY[repo], "token")
        self.assertEqual(result["status"], "PASS")
        self.assertEqual(result["source"], ["Repository:owner/repo#42"])
        self.assertEqual(mocked.call_args_list[0].args[0], f"/repos/{repo}")
        self.assertEqual(mocked.call_args_list[1].args[0], f"/repos/{repo}/rules/branches/main?per_page=100&page=1")
        self.assertEqual(mocked.call_args_list[2].args[0], f"/repos/{repo}/branches/main/protection/required_status_checks")

    def test_audit_repository_fails_if_default_branch_changed(self):
        repo = "Luminous-Dynamics/mycelix"
        with patch.object(module, "get_json", return_value={
            "full_name": repo, "default_branch": "trunk"
        }) as mocked:
            result = module.audit_repository(repo, module.POLICY[repo], "token")
        self.assertEqual(result["status"], "FAIL")
        self.assertIn("differs from API default branch", result["reason"])
        mocked.assert_called_once()

    def test_malformed_repository_metadata_is_incomplete(self):
        repo = "Luminous-Dynamics/mycelix"
        with patch.object(module, "get_json", return_value={"full_name": None, "default_branch": "main"}):
            result = module.audit_repository(repo, module.POLICY[repo], "token")
        self.assertEqual(result["status"], "INCOMPLETE")

    def test_effective_rules_api_failure_is_incomplete(self):
        repo = "Luminous-Dynamics/mycelix"
        with patch.object(module, "get_json", side_effect=[
            {"full_name": repo, "default_branch": "main"},
            module.ApiError(403, "forbidden"),
        ]):
            result = module.audit_repository(repo, module.POLICY[repo], "token")
        self.assertEqual(result["status"], "INCOMPLETE")

    def test_unknown_cli_options_are_not_silently_ignored(self):
        output = io.StringIO()
        with patch.object(module.sys, "argv", ["audit_security_required_checks.py", "--audit-all"]), \
             patch.dict(module.os.environ, {"GITHUB_TOKEN": "test-token"}), \
             redirect_stdout(output):
            exit_code = module.main()
        receipt = json.loads(output.getvalue())
        self.assertEqual(exit_code, 1)
        self.assertEqual(receipt["status"], "FAIL")
        self.assertEqual(receipt["unknown_repositories"], ["--audit-all"])

    def test_policy_digest_is_deterministic_and_policy_sensitive(self):
        original = module.policy_digest()
        self.assertRegex(original, r"^[0-9a-f]{64}$")
        self.assertEqual(original, module.policy_digest())
        changed = dict(module.POLICY)
        changed["Luminous-Dynamics/mycelix"] = {
            **module.POLICY["Luminous-Dynamics/mycelix"],
            "default_branch": "trunk",
        }
        self.assertNotEqual(original, module.policy_digest(changed))

    def test_missing_token_receipt_binds_exact_tool_and_runtime(self):
        output = io.StringIO()
        with patch.object(module.sys, "argv", ["audit_security_required_checks.py"]), \
             patch.dict(module.os.environ, {}, clear=True), redirect_stdout(output):
            exit_code = module.main()
        receipt = json.loads(output.getvalue())
        self.assertEqual(exit_code, 2)
        self.assertEqual(receipt["status"], "INCOMPLETE")
        self.assertEqual(receipt["required_verifier_context"], module.VERIFIER_CONTEXT)
        self.assertEqual(receipt["required_integration_id"], module.GITHUB_ACTIONS_INTEGRATION_ID)
        self.assertEqual(receipt["auditor_sha256"], hashlib.sha256(SCRIPT.read_bytes()).hexdigest())
        self.assertEqual(receipt["policy_sha256"], module.policy_digest())
        self.assertTrue(receipt["python_version"])
        self.assertTrue(receipt["python_implementation"])
        self.assertEqual(receipt["api_version"], module.API_VERSION)

    def test_help_does_not_require_a_token(self):
        output = io.StringIO()
        with patch.object(module.sys, "argv", ["audit_security_required_checks.py", "--help"]), \
             patch.dict(module.os.environ, {}, clear=True), redirect_stdout(output):
            exit_code = module.main()
        self.assertEqual(exit_code, 0)
        self.assertIn("Usage:", output.getvalue())

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
