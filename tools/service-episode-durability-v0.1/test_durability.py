import copy
import unittest

from check_durability import validate
from pathlib import Path
import json

ROOT = Path(__file__).resolve().parents[2]
MATRIX = json.loads((ROOT / "benchmarks" / "service-episode-durability-v0.1" / "crash-matrix.json").read_text())
SCHEMA = json.loads((ROOT / "benchmarks" / "service-episode-durability-v0.1" / "crash-matrix.schema.json").read_text())


class DurabilityContractTests(unittest.TestCase):
    def test_frozen_contract_passes(self):
        result = validate(MATRIX, SCHEMA)
        self.assertEqual(result["case_count"], 24)
        self.assertEqual(result["family_count"], 8)

    def test_duplicate_case_id_rejected(self):
        changed = copy.deepcopy(MATRIX)
        changed["cases"][1]["id"] = changed["cases"][0]["id"]
        with self.assertRaises(ValueError):
            validate(changed, SCHEMA)

    def test_case_removal_rejected(self):
        changed = copy.deepcopy(MATRIX)
        changed["cases"].pop()
        with self.assertRaises(ValueError):
            validate(changed, SCHEMA)

    def test_case_count_schema_cannot_shrink(self):
        changed = copy.deepcopy(SCHEMA)
        changed["properties"]["cases"]["minItems"] = 1
        with self.assertRaises(ValueError):
            validate(MATRIX, changed)

    def test_unknown_case_fields_rejected(self):
        changed = copy.deepcopy(MATRIX)
        changed["cases"][0]["hidden_expected_answer"] = "safe"
        with self.assertRaises(ValueError):
            validate(changed, SCHEMA)

    def test_automatic_external_replay_rejected(self):
        changed = copy.deepcopy(MATRIX)
        changed["cases"][0]["automatic_external_replay"] = True
        with self.assertRaises(ValueError):
            validate(changed, SCHEMA)

    def test_schema_cannot_allow_automatic_external_replay(self):
        changed = copy.deepcopy(SCHEMA)
        changed["properties"]["cases"]["items"]["properties"]["automatic_external_replay"]["const"] = True
        with self.assertRaises(ValueError):
            validate(MATRIX, changed)

    def test_lost_acknowledged_event_gate_is_hard_zero(self):
        changed = copy.deepcopy(MATRIX)
        changed["hard_gates"]["max_lost_acknowledged_events"] = 1
        with self.assertRaises(ValueError):
            validate(changed, SCHEMA)

    def test_duplicate_effect_gate_is_hard_zero(self):
        changed = copy.deepcopy(MATRIX)
        changed["hard_gates"]["max_duplicate_business_effects"] = 1
        with self.assertRaises(ValueError):
            validate(changed, SCHEMA)

    def test_unknown_effect_cannot_be_marked_safe(self):
        changed = copy.deepcopy(MATRIX)
        case = next(x for x in changed["cases"] if x["id"] == "DISPATCH-002")
        case["expected_recovery"] = "safe_to_resume"
        with self.assertRaises(ValueError):
            validate(changed, SCHEMA)

    def test_corrupt_journal_cannot_be_marked_safe(self):
        changed = copy.deepcopy(MATRIX)
        case = next(x for x in changed["cases"] if x["id"] == "STORE-003")
        case["expected_recovery"] = "safe_to_resume"
        with self.assertRaises(ValueError):
            validate(changed, SCHEMA)

    def test_stale_owner_must_be_fenced(self):
        changed = copy.deepcopy(MATRIX)
        case = next(x for x in changed["cases"] if x["id"] == "FENCE-001")
        case["expected_recovery"] = "safe_to_resume"
        with self.assertRaises(ValueError):
            validate(changed, SCHEMA)

    def test_participant_oracle_must_be_independent(self):
        changed = copy.deepcopy(MATRIX)
        changed["external_effect_oracle"]["independent_of"] = "coordinator_event_log"
        with self.assertRaises(ValueError):
            validate(changed, SCHEMA)

    def test_in_memory_reference_is_not_durable_comparator(self):
        changed = copy.deepcopy(MATRIX)
        arm = next(x for x in changed["baseline_arms"] if x["id"] == "in_memory_reference")
        arm["role"] = "durable_candidate"
        with self.assertRaises(ValueError):
            validate(changed, SCHEMA)

    def test_target_values_are_hypotheses_not_results(self):
        changed = copy.deepcopy(MATRIX)
        changed["initial_targets_not_results"]["same_node_restart_rto_p95_ms"] = 1
        with self.assertRaises(ValueError):
            validate(changed, SCHEMA)

    def test_all_eight_failure_families_are_required(self):
        changed = copy.deepcopy(MATRIX)
        changed["cases"] = [x for x in changed["cases"] if x["family"] != "snapshot-replay"]
        with self.assertRaises(ValueError):
            validate(changed, SCHEMA)

    def test_nixward_owns_host_journal(self):
        changed = copy.deepcopy(MATRIX)
        changed["invariants"].remove("platform_coordinator_journal_does_not_duplicate_nixward_host_journal")
        with self.assertRaises(ValueError):
            validate(changed, SCHEMA)

    def test_comparison_must_not_equate_workflow_history_with_external_truth(self):
        changed = copy.deepcopy(MATRIX)
        changed["comparison_rules"][2] = "workflow history is sufficient proof of external effect"
        with self.assertRaises(ValueError):
            validate(changed, SCHEMA)


    def test_restate_rust_sdk_is_a_required_comparator(self):
        changed = copy.deepcopy(MATRIX)
        changed["baseline_arms"] = [x for x in changed["baseline_arms"] if x["id"] != "restate_rust_sdk_0_13"]
        with self.assertRaises(ValueError):
            validate(changed, SCHEMA)


    def test_schema_cannot_weaken_duplicate_effect_gate(self):
        changed = copy.deepcopy(SCHEMA)
        changed["properties"]["hard_gates"]["properties"]["max_duplicate_business_effects"]["const"] = 1
        with self.assertRaises(ValueError):
            validate(MATRIX, changed)

    def test_schema_must_require_complete_case_identity(self):
        changed = copy.deepcopy(SCHEMA)
        changed["properties"]["cases"]["items"]["required"].remove("participant_state")
        with self.assertRaises(ValueError):
            validate(MATRIX, changed)

    def test_schema_must_preserve_all_eight_fault_families(self):
        changed = copy.deepcopy(SCHEMA)
        changed["properties"]["cases"]["items"]["properties"]["family"]["enum"].remove("snapshot-replay")
        with self.assertRaises(ValueError):
            validate(MATRIX, changed)

    def test_schema_must_include_all_five_comparison_arms(self):
        changed = copy.deepcopy(SCHEMA)
        changed["properties"]["baseline_arms"]["maxItems"] = 4
        with self.assertRaises(ValueError):
            validate(MATRIX, changed)

if __name__ == "__main__":
    unittest.main()