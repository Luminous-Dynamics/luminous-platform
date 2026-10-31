import copy
import hashlib
import json
import unittest
from pathlib import Path

from plan_power import PlanningError, load_json, plan_power

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "benchmarks" / "opsbench-assurance-statistics-v0.1"
PREREG_PATH = ROOT / "benchmarks" / "opsbench-common-core-v0.1" / "preregistration.example.json"


def fixtures():
    plan_bytes = (BASE / "power-plan.example.json").read_bytes()
    prereg_bytes = PREREG_PATH.read_bytes()
    return json.loads(plan_bytes), plan_bytes, json.loads(prereg_bytes), prereg_bytes


def serialize(value):
    return (json.dumps(value, indent=2) + "\n").encode()


def rebind(plan, prereg_bytes):
    plan["preregistration_sha256"] = hashlib.sha256(prereg_bytes).hexdigest()


class PowerPlanTests(unittest.TestCase):
    def test_example_report_is_deterministic_and_only_planning(self):
        plan, raw, prereg, prereg_bytes = fixtures()
        one = plan_power(plan, raw, prereg, prereg_bytes)
        two = plan_power(plan, raw, prereg, prereg_bytes)
        self.assertEqual(one["report_sha256"], two["report_sha256"])
        self.assertEqual(one["status"], "example_only")
        self.assertEqual(one["pilot_split"], "screening")
        self.assertIn("planning diagnostic", one["claim_ceiling"])

    def test_repeat_seeds_do_not_inflate_cluster_count(self):
        plan, raw, prereg, prereg_bytes = fixtures()
        report = plan_power(plan, raw, prereg, prereg_bytes)
        self.assertEqual(report["pilot_scenario_cluster_count"], 5)
        self.assertEqual(report["pilot_nested_seed_total_not_inferential_n"], 25)
        for candidate in report["candidate_plans"].values():
            self.assertEqual(candidate["pilot_paired_cluster_count"], 5)

    def test_recommended_n_respects_minimum_floor_and_safety_margin(self):
        plan, raw, prereg, prereg_bytes = fixtures()
        report = plan_power(plan, raw, prereg, prereg_bytes)
        self.assertGreaterEqual(report["recommendation"]["required_scenario_count"], 500)
        for result in report["candidate_plans"].values():
            self.assertGreaterEqual(result["recommended_confirmatory_scenario_clusters"], 500)
            self.assertGreaterEqual(result["after_safety_margin_scenario_clusters"], result["normal_approximation_scenario_clusters"])

    def test_candidate_comparison_family_must_match_candidate_arms(self):
        plan, _raw, prereg, prereg_bytes = fixtures()
        plan["planning_parameters"]["candidate_comparison_count"] = 1
        with self.assertRaisesRegex(PlanningError, "must equal the candidate arm family size"):
            plan_power(plan, serialize(plan), prereg, prereg_bytes)

    def test_preregistration_digest_mismatch_rejected(self):
        plan, raw, prereg, prereg_bytes = fixtures()
        plan["preregistration_sha256"] = "0" * 64
        with self.assertRaisesRegex(PlanningError, "exact preregistration digest mismatch"):
            plan_power(plan, serialize(plan), prereg, prereg_bytes)

    def test_confirmatory_scenario_cannot_enter_screening_pilot(self):
        plan, _raw, prereg, prereg_bytes = fixtures()
        plan["pilot_scenario_ids"][0] = "CONF-001"
        plan["pilot_clusters"][0]["scenario_id"] = "CONF-001"
        with self.assertRaisesRegex(PlanningError, "outside the preregistered screening split"):
            plan_power(plan, serialize(plan), prereg, prereg_bytes)

    def test_development_scenario_cannot_enter_screening_pilot(self):
        plan, _raw, prereg, prereg_bytes = fixtures()
        plan["pilot_scenario_ids"][0] = "DEV-001"
        plan["pilot_clusters"][0]["scenario_id"] = "DEV-001"
        with self.assertRaisesRegex(PlanningError, "outside the preregistered screening split"):
            plan_power(plan, serialize(plan), prereg, prereg_bytes)

    def test_duplicate_scenario_cluster_rejected(self):
        plan, _raw, prereg, prereg_bytes = fixtures()
        plan["pilot_clusters"][1]["scenario_id"] = plan["pilot_clusters"][0]["scenario_id"]
        with self.assertRaisesRegex(PlanningError, "duplicate scenario cluster"):
            plan_power(plan, serialize(plan), prereg, prereg_bytes)

    def test_seed_count_is_metadata_not_a_weight(self):
        plan, raw, prereg, prereg_bytes = fixtures()
        baseline = plan_power(plan, raw, prereg, prereg_bytes)
        plan["pilot_clusters"][0]["seed_count"] = 500
        changed = plan_power(plan, serialize(plan), prereg, prereg_bytes)
        self.assertEqual(
            baseline["candidate_plans"]["luminous-full"]["pilot_paired_mean_difference"],
            changed["candidate_plans"]["luminous-full"]["pilot_paired_mean_difference"],
        )
        self.assertEqual(changed["pilot_scenario_cluster_count"], 5)
        self.assertEqual(changed["pilot_nested_seed_total_not_inferential_n"], 520)

    def test_unknown_pair_is_not_converted_to_zero_and_is_reported(self):
        plan, _raw, prereg, prereg_bytes = fixtures()
        prereg = copy.deepcopy(prereg)
        prereg["scenario_split"]["screening_scenario_ids"].extend(["SCR-006", "SCR-007"])
        prereg_bytes = serialize(prereg)
        plan["pilot_scenario_ids"].extend(["SCR-006", "SCR-007"])
        plan["pilot_clusters"].extend([
            {"scenario_id":"SCR-006","seed_count":5,"arm_success_rates":{"deterministic":0.5,"luminous-full":None,"luminous-no-saga":0.6}},
            {"scenario_id":"SCR-007","seed_count":5,"arm_success_rates":{"deterministic":0.5,"luminous-full":0.6,"luminous-no-saga":0.7}}
        ])
        rebind(plan, prereg_bytes)
        report = plan_power(plan, serialize(plan), prereg, prereg_bytes)
        self.assertEqual(report["candidate_plans"]["luminous-full"]["pilot_unmatched_cluster_count"], 1)
        self.assertEqual(report["candidate_plans"]["luminous-full"]["pilot_paired_cluster_count"], 6)

    def test_missing_pairs_below_minimum_rejected(self):
        plan, _raw, prereg, prereg_bytes = fixtures()
        for row in plan["pilot_clusters"]:
            row["arm_success_rates"]["luminous-full"] = None
        with self.assertRaisesRegex(PlanningError, "at least five exact paired screening clusters"):
            plan_power(plan, serialize(plan), prereg, prereg_bytes)

    def test_zero_variance_uses_explicit_floor_not_zero_variance_shortcut(self):
        plan, _raw, prereg, prereg_bytes = fixtures()
        for row in plan["pilot_clusters"]:
            row["arm_success_rates"]["luminous-full"] = row["arm_success_rates"]["deterministic"]
        report = plan_power(plan, serialize(plan), prereg, prereg_bytes)
        full = report["candidate_plans"]["luminous-full"]
        self.assertTrue(full["variance_degenerate"])
        self.assertEqual(full["planning_sd_after_variance_floor"], plan["planning_parameters"]["variance_floor"])
        self.assertGreater(full["normal_approximation_scenario_clusters"], 2)

    def test_comparison_arms_must_have_equal_tool_permission_and_budget_profiles(self):
        plan, _raw, prereg, _prereg_bytes = fixtures()
        prereg = copy.deepcopy(prereg)
        candidate = next(x for x in prereg["arms"] if x["arm_id"] == "luminous-full")
        candidate["permission_profile_digest"] = "f" * 64
        prereg_bytes = serialize(prereg)
        rebind(plan, prereg_bytes)
        with self.assertRaisesRegex(PlanningError, "not common-core fair"):
            plan_power(plan, serialize(plan), prereg, prereg_bytes)

    def test_registered_screening_pilot_requires_registration_timestamp_before_pilot(self):
        plan, _raw, prereg, _ = fixtures()
        plan["status"] = "screening_pilot"
        prereg = copy.deepcopy(prereg)
        prereg["status"] = "registered_before_trials"
        prereg["registered_at"] = "2026-10-31T14:00:00Z"
        prereg_bytes = serialize(prereg)
        rebind(plan, prereg_bytes)
        with self.assertRaisesRegex(PlanningError, "must predate screening pilot"):
            plan_power(plan, serialize(plan), prereg, prereg_bytes)

    def test_duplicate_json_keys_and_nonfinite_values_rejected(self):
        with self.assertRaisesRegex(PlanningError, "duplicate JSON key"):
            load_json(b'{"x":1,"x":2}', "test")
        with self.assertRaisesRegex(PlanningError, "non-finite JSON number"):
            load_json(b'{"x":NaN}', "test")


if __name__ == "__main__":
    unittest.main()
