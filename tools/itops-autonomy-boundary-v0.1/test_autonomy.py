from pathlib import Path
import copy
import json
import unittest

from check_autonomy import validate

ROOT = Path(__file__).resolve().parents[2]
TASKS = json.loads((ROOT / "benchmarks/itops-autonomy-boundary-v0.1/task-manifest.json").read_text())
TASK_SCHEMA = json.loads((ROOT / "benchmarks/itops-autonomy-boundary-v0.1/task-manifest.schema.json").read_text())
ORACLE = json.loads((ROOT / "benchmarks/itops-autonomy-boundary-v0.1/evaluator-corpus.json").read_text())
ORACLE_SCHEMA = json.loads((ROOT / "benchmarks/itops-autonomy-boundary-v0.1/evaluator-corpus.schema.json").read_text())


class AutonomyBoundaryTests(unittest.TestCase):
    def validate_all(self, tasks=TASKS, task_schema=TASK_SCHEMA, oracle=ORACLE, oracle_schema=ORACLE_SCHEMA):
        return validate(tasks, task_schema, oracle, oracle_schema)

    def test_frozen_64_pairs_and_128_tasks_pass(self):
        result = self.validate_all()
        self.assertEqual(result["pairs"], 64)
        self.assertEqual(result["task_variants"], 128)

    def test_duplicate_task_id_rejected(self):
        changed = copy.deepcopy(TASKS)
        changed["cases"][1]["id"] = changed["cases"][0]["id"]
        with self.assertRaises(ValueError):
            self.validate_all(tasks=changed)

    def test_expected_decision_cannot_leak_into_sut_manifest(self):
        changed = copy.deepcopy(TASKS)
        changed["cases"][0]["expected_decision"] = "ACT"
        with self.assertRaises(ValueError):
            self.validate_all(tasks=changed)

    def test_pair_id_cannot_leak_into_sut_manifest(self):
        changed = copy.deepcopy(TASKS)
        changed["cases"][0]["pair_id"] = "PAIR-001"
        with self.assertRaises(ValueError):
            self.validate_all(tasks=changed)

    def test_oracle_cannot_be_relabelled_as_sut_input(self):
        changed = copy.deepcopy(ORACLE)
        changed["visibility"] = "system_under_test_input_only"
        with self.assertRaises(ValueError):
            self.validate_all(oracle=changed)

    def test_missing_counterfactual_pair_rejected(self):
        changed = copy.deepcopy(ORACLE)
        changed["pairs"].pop()
        changed["pair_count"] = 63
        with self.assertRaises(ValueError):
            self.validate_all(oracle=changed)

    def test_duplicate_pair_id_rejected(self):
        changed = copy.deepcopy(ORACLE)
        changed["pairs"][1]["pair_id"] = changed["pairs"][0]["pair_id"]
        with self.assertRaises(ValueError):
            self.validate_all(oracle=changed)

    def test_pair_cannot_reference_unknown_task(self):
        changed = copy.deepcopy(ORACLE)
        changed["pairs"][0]["act_variant"]["task_id"] = "ATB-9999"
        with self.assertRaises(ValueError):
            self.validate_all(oracle=changed)

    def test_pair_must_change_one_field(self):
        changed = copy.deepcopy(ORACLE)
        changed["pairs"][0]["controlled_perturbation"]["only_changed_field"] = False
        with self.assertRaises(ValueError):
            self.validate_all(oracle=changed)

    def test_read_only_act_variant_can_have_zero_effects(self):
        changed = copy.deepcopy(ORACLE)
        case = next(x for x in changed["pairs"] if x["act_variant"]["expected_business_effect_count"] == 0)
        self.assertEqual(case["act_variant"]["expected_decision"], "ACT")
        self.assertEqual(self.validate_all(oracle=changed)["pairs"], 64)

    def test_act_effect_count_cannot_be_negative(self):
        changed = copy.deepcopy(ORACLE)
        changed["pairs"][0]["act_variant"]["expected_business_effect_count"] = -1
        with self.assertRaises(ValueError):
            self.validate_all(oracle=changed)

    def test_counterfactual_effect_budget_is_zero(self):
        changed = copy.deepcopy(ORACLE)
        changed["pairs"][0]["counterfactual_variant"]["expected_business_effect_count"] = 1
        with self.assertRaises(ValueError):
            self.validate_all(oracle=changed)

    def test_counterfactual_mutation_is_forbidden(self):
        changed = copy.deepcopy(ORACLE)
        changed["pairs"][0]["counterfactual_variant"]["must_not_mutate"] = False
        with self.assertRaises(ValueError):
            self.validate_all(oracle=changed)

    def test_all_eight_families_have_eight_pairs(self):
        changed = copy.deepcopy(ORACLE)
        changed["pairs"][0]["family"] = "unknown-family"
        with self.assertRaises(ValueError):
            self.validate_all(oracle=changed)

    def test_hard_safety_gate_cannot_be_weakened(self):
        changed = copy.deepcopy(ORACLE)
        changed["hard_gates"]["unauthorized_mutations"] = 1
        with self.assertRaises(ValueError):
            self.validate_all(oracle=changed)

    def test_targets_are_hypotheses_not_results(self):
        changed = copy.deepcopy(ORACLE)
        changed["initial_targets_not_results"]["held_out_pairwise_decision_accuracy_percent_min"] = 50
        with self.assertRaises(ValueError):
            self.validate_all(oracle=changed)

    def test_non_escalation_case_has_no_handoff_rubric(self):
        changed = copy.deepcopy(ORACLE)
        case = next(x for x in changed["pairs"] if x["counterfactual_variant"]["expected_decision"] == "HALT")
        case["required_handoff_fields"] = ["episode_ref"]
        with self.assertRaises(ValueError):
            self.validate_all(oracle=changed)

    def test_escalation_requires_complete_handoff(self):
        changed = copy.deepcopy(ORACLE)
        case = next(x for x in changed["pairs"] if x["counterfactual_variant"]["expected_decision"] == "ESCALATE")
        case["required_handoff_fields"] = []
        with self.assertRaises(ValueError):
            self.validate_all(oracle=changed)

    def test_task_schema_cannot_allow_hidden_fields(self):
        changed = copy.deepcopy(TASK_SCHEMA)
        changed["properties"]["cases"]["items"]["additionalProperties"] = True
        with self.assertRaises(ValueError):
            self.validate_all(task_schema=changed)

    def test_oracle_schema_cannot_allow_new_fields(self):
        changed = copy.deepcopy(ORACLE_SCHEMA)
        changed["properties"]["pairs"]["items"]["additionalProperties"] = True
        with self.assertRaises(ValueError):
            self.validate_all(oracle_schema=changed)

    def test_oracle_commit_pin_must_be_filled(self):
        changed = copy.deepcopy(ORACLE)
        changed["task_manifest_commit"] = "pending_exact_head_pin"
        with self.assertRaises(ValueError):
            self.validate_all(oracle=changed)


if __name__ == "__main__":
    unittest.main()
