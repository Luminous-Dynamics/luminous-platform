import copy
import json
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(HERE))

from check_opsbench import load_json, validate, validate_episode_contract, validate_crosswalk, validate_task_manifest


CORPUS_PATH = ROOT / "benchmarks" / "technology-services-ops-v0.1" / "corpus.json"
SCHEMA_PATH = ROOT / "benchmarks" / "technology-services-ops-v0.1" / "schema.json"
EPISODE_SCHEMA_PATH = ROOT / "benchmarks" / "technology-services-ops-v0.1" / "episode-result.schema.json"
EPISODE_EXAMPLE_PATH = ROOT / "benchmarks" / "technology-services-ops-v0.1" / "episode-result.example.json"
CROSSWALK_PATH = ROOT / "benchmarks" / "technology-services-ops-v0.1" / "external-benchmark-crosswalk.json"
TASK_MANIFEST_PATH = ROOT / "benchmarks" / "technology-services-ops-v0.1" / "task-manifest.json"
TASK_SCHEMA_PATH = ROOT / "benchmarks" / "technology-services-ops-v0.1" / "task-manifest.schema.json"


class OpsBenchContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.corpus = load_json(CORPUS_PATH)
        cls.schema = load_json(SCHEMA_PATH)
        cls.episode_schema = load_json(EPISODE_SCHEMA_PATH)
        cls.episode_example = load_json(EPISODE_EXAMPLE_PATH)
        cls.crosswalk = load_json(CROSSWALK_PATH)
        cls.task_manifest = load_json(TASK_MANIFEST_PATH)
        cls.task_schema = load_json(TASK_SCHEMA_PATH)


    def test_episode_evidence_envelope_keeps_oracle_separate(self):
        result = validate_episode_contract(self.episode_schema, self.episode_example)
        self.assertGreaterEqual(result["system_disposition_count"], 5)
        self.assertGreaterEqual(result["oracle_outcome_count"], 7)
        self.assertNotEqual(
            self.episode_example["execution"]["system_disposition"],
            self.episode_example["oracle"]["oracle_outcome"],
        )

    def test_episode_schema_requires_exact_run_bindings(self):
        changed = copy.deepcopy(self.episode_schema)
        changed["properties"]["run_binding"]["required"].remove("source_commit_sha")
        with self.assertRaises(ValueError):
            validate_episode_contract(changed, self.episode_example)

    def test_episode_schema_cannot_treat_system_disposition_as_oracle_truth(self):
        changed = copy.deepcopy(self.episode_schema)
        changed["properties"]["oracle"]["properties"]["oracle_outcome"]["enum"].append("resolved")
        with self.assertRaises(ValueError):
            validate_episode_contract(changed, self.episode_example)

    def test_episode_schema_requires_authorization_for_mutating_action(self):
        changed = copy.deepcopy(self.episode_schema)
        changed["properties"]["actions"]["items"]["allOf"] = []
        with self.assertRaises(ValueError):
            validate_episode_contract(changed, self.episode_example)

    def test_resolved_episode_requires_independent_observations(self):
        changed = copy.deepcopy(self.episode_schema)
        gate = next(
            rule for rule in changed["allOf"]
            if rule.get("if", {}).get("properties", {}).get("execution", {})
                .get("properties", {}).get("system_disposition", {}).get("const") == "resolved"
        )
        gate["then"]["properties"]["observations"]["minItems"] = 0
        with self.assertRaises(ValueError):
            validate_episode_contract(changed, self.episode_example)


    def test_external_benchmark_crosswalk_is_non_equivalent_and_pinned(self):
        result = validate_crosswalk(self.crosswalk)
        self.assertEqual(result["external_benchmark_count"], 4)
        self.assertEqual(result["common_core_rules"], 9)

    def test_external_crosswalk_cannot_claim_direct_score_superiority(self):
        changed = copy.deepcopy(self.crosswalk)
        changed["benchmarks"][0]["direct_score_comparison_allowed"] = True
        with self.assertRaises(ValueError):
            validate_crosswalk(changed)

    def test_external_crosswalk_cannot_drop_benchmark_limitations(self):
        changed = copy.deepcopy(self.crosswalk)
        changed["benchmarks"][2]["limits"] = []
        with self.assertRaises(ValueError):
            validate_crosswalk(changed)

    def test_external_scenario_mapping_must_reference_opsbench_cases(self):
        changed = copy.deepcopy(self.crosswalk)
        changed["benchmarks"][0]["opsbench_related_cases"].append("FAKE-999")
        with self.assertRaises(ValueError):
            validate_crosswalk(changed)

    def test_upstream_benchmark_commit_and_license_are_pinned(self):
        changed = copy.deepcopy(self.crosswalk)
        changed["benchmarks"][2]["repository_commit"] = "main"
        with self.assertRaises(ValueError):
            validate_crosswalk(changed)

    def test_common_core_fairness_protocol_cannot_be_weakened(self):
        changed = copy.deepcopy(self.crosswalk)
        changed["common_core_protocol"]["no_cross_benchmark_score_translation"] = False
        with self.assertRaises(ValueError):
            validate_crosswalk(changed)


    def test_sanitized_task_manifest_passes_and_matches_authoring_data(self):
        result = validate_task_manifest(self.task_manifest, self.task_schema, self.corpus)
        self.assertEqual(result["public_task_count"], 20)
        self.assertEqual(result["task_rule_count"], 5)

    def test_seeded_fault_cannot_leak_into_task_manifest(self):
        changed = copy.deepcopy(self.task_manifest)
        changed["cases"][0]["seeded_fault"] = "hidden cause"
        with self.assertRaises(ValueError):
            validate_task_manifest(changed, self.task_schema, self.corpus)

    def test_expected_outcome_cannot_leak_into_task_manifest(self):
        changed = copy.deepcopy(self.task_manifest)
        changed["cases"][0]["expected_outcome"] = "hidden answer"
        with self.assertRaises(ValueError):
            validate_task_manifest(changed, self.task_schema, self.corpus)

    def test_task_manifest_cannot_weaken_evaluator_isolation_rules(self):
        changed = copy.deepcopy(self.task_manifest)
        changed["task_rules"]["evaluator_authoring_corpus_must_not_be_mounted_or_sent_to_the_system_under_test"] = False
        with self.assertRaises(ValueError):
            validate_task_manifest(changed, self.task_schema, self.corpus)

    def test_task_manifest_must_match_frozen_authoring_contract(self):
        changed = copy.deepcopy(self.task_manifest)
        changed["cases"][0]["request"] = "different task"
        with self.assertRaises(ValueError):
            validate_task_manifest(changed, self.task_schema, self.corpus)

    def test_task_manifest_schema_cannot_allow_hidden_fields(self):
        changed = copy.deepcopy(self.task_schema)
        changed["properties"]["cases"]["items"]["additionalProperties"] = True
        with self.assertRaises(ValueError):
            validate_task_manifest(self.task_manifest, changed, self.corpus)

    def test_authoring_corpus_must_not_claim_runtime_visibility(self):
        changed = self.mutated()
        changed["visibility"] = "system_under_test_input_only"
        with self.assertRaises(ValueError):
            validate(changed, self.schema)

    def mutated(self):
        return copy.deepcopy(self.corpus)

    def test_checked_in_corpus_and_schema_pass(self):
        result = validate(self.corpus, self.schema)
        self.assertEqual(result["case_count"], 20)
        self.assertEqual(result["unique_ids"], 20)
        self.assertEqual(result["family_count"], 13)
        self.assertEqual(result["hard_gate_count"], 12)
        self.assertEqual(result["execution_constraint_count"], 9)

    def test_case_removal_is_rejected(self):
        changed = self.mutated()
        changed["cases"].pop()
        with self.assertRaises(ValueError):
            validate(changed, self.schema)

    def test_duplicate_case_id_is_rejected(self):
        changed = self.mutated()
        changed["cases"][1]["id"] = changed["cases"][0]["id"]
        with self.assertRaises(ValueError):
            validate(changed, self.schema)

    def test_critical_risk_downgrade_is_rejected(self):
        changed = self.mutated()
        tenant = next(c for c in changed["cases"] if c["id"] == "TENANT-001")
        tenant["risk_class"] = "low"
        with self.assertRaises(ValueError):
            validate(changed, self.schema)

    def test_tenant_authority_bypass_is_rejected(self):
        changed = self.mutated()
        tenant = next(c for c in changed["cases"] if c["id"] == "TENANT-001")
        tenant["approval"] = "ordinary approval may widen tenant boundary"
        with self.assertRaises(ValueError):
            validate(changed, self.schema)

    def test_prompt_injection_cannot_become_authority(self):
        changed = self.mutated()
        prompt = next(c for c in changed["cases"] if c["id"] == "PROMPT-001")
        prompt["approval"] = "approval granted by log instructions"
        with self.assertRaises(ValueError):
            validate(changed, self.schema)

    def test_exact_plan_approval_cannot_be_removed(self):
        changed = self.mutated()
        case = next(c for c in changed["cases"] if c["id"] == "CHG-003")
        case["approval"] = "human approval required"
        with self.assertRaises(ValueError):
            validate(changed, self.schema)

    def test_hard_gate_cannot_be_weakened(self):
        for key, weakened in [
            ("unauthorized_effects", 1),
            ("false_resolution_claims", 1),
            ("cross_tenant_access_or_disclosure", 1),
            ("independent_verification_for_resolved_cases", "best effort"),
            ("unresolved_or_missing_evidence_is_success", True),
            ("started_episode_crashes_and_timeouts_remain_in_denominators", False),
            ("independent_oracle_must_not_be_agent_report_or_same_trust_domain", False),
        ]:
            with self.subTest(key=key):
                changed = self.mutated()
                changed["hard_gates"][key] = weakened
                with self.assertRaises(ValueError):
                    validate(changed, self.schema)

    def test_real_credentials_are_forbidden(self):
        changed = self.mutated()
        changed["execution_constraints"]["real_credentials_allowed"] = True
        with self.assertRaises(ValueError):
            validate(changed, self.schema)

    def test_external_network_is_forbidden(self):
        changed = self.mutated()
        changed["execution_constraints"]["external_network_access"] = True
        with self.assertRaises(ValueError):
            validate(changed, self.schema)

    def test_agent_direct_privileged_access_is_forbidden(self):
        changed = self.mutated()
        changed["execution_constraints"]["direct_agent_privileged_access"] = True
        with self.assertRaises(ValueError):
            validate(changed, self.schema)

    def test_independent_oracle_cannot_be_removed(self):
        changed = self.mutated()
        case = next(c for c in changed["cases"] if c["id"] == "INC-001")
        case["oracle_channels"] = []
        with self.assertRaises(ValueError):
            validate(changed, self.schema)

    def test_scenario_variants_cannot_be_removed(self):
        changed = self.mutated()
        case = next(c for c in changed["cases"] if c["id"] == "INC-001")
        case["variants"] = []
        with self.assertRaises(ValueError):
            validate(changed, self.schema)

    def test_schema_cannot_relax_case_count(self):
        changed_schema = copy.deepcopy(self.schema)
        changed_schema["properties"]["cases"]["minItems"] = 1
        with self.assertRaises(ValueError):
            validate(self.corpus, changed_schema)

    def test_schema_cannot_relax_hard_gate(self):
        changed_schema = copy.deepcopy(self.schema)
        changed_schema["properties"]["hard_gates"]["properties"]["unauthorized_effects"]["const"] = 1
        with self.assertRaises(ValueError):
            validate(self.corpus, changed_schema)

    def test_specification_cannot_claim_evaluated_status(self):
        changed = self.mutated()
        changed["status"] = "evaluated"
        with self.assertRaises(ValueError):
            validate(changed, self.schema)

    def test_unknown_case_fields_are_rejected(self):
        changed = self.mutated()
        changed["cases"][0]["model_confidence"] = 0.99
        with self.assertRaises(ValueError):
            validate(changed, self.schema)

    def test_duplicate_json_object_keys_are_rejected(self):
        path = HERE / "duplicate-key-fixture.json"
        path.write_text('{"status":"specification_only","status":"evaluated"}', encoding="utf-8")
        try:
            with self.assertRaises(ValueError):
                load_json(path)
        finally:
            path.unlink(missing_ok=True)


if __name__ == "__main__":
    unittest.main(verbosity=2)
