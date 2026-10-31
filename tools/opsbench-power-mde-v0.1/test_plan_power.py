import copy
import hashlib
import json
import unittest
from pathlib import Path

from plan_power import PlanningError, load_json, plan, required_n, sample_sd

ROOT = Path(__file__).resolve().parents[2]
PILOT_PATH = ROOT / "benchmarks" / "opsbench-power-mde-v0.1" / "pilot.example.json"
PREREG_PATH = ROOT / "benchmarks" / "opsbench-common-core-v0.1" / "preregistration.example.json"


def fixture():
    pilot = json.loads(PILOT_PATH.read_text())
    prereg_bytes = PREREG_PATH.read_bytes()
    prereg = json.loads(prereg_bytes)
    return pilot, prereg, prereg_bytes


class PowerPlannerTests(unittest.TestCase):
    def test_example_is_deterministic_and_not_confirmatory(self):
        pilot, prereg, prereg_bytes = fixture()
        one = plan(pilot, prereg, prereg_bytes)
        two = plan(pilot, prereg, prereg_bytes)
        self.assertEqual(one, two)
        self.assertEqual(one["status"], "example_only")
        self.assertEqual(one["recommended_confirmatory_scenario_count"], 500)

    def test_unmatched_clusters_are_explicit_not_zero_filled(self):
        pilot, prereg, prereg_bytes = fixture()
        report = plan(pilot, prereg, prereg_bytes)
        self.assertEqual(report["unmatched_candidate_scenario_ids"], ["SCR-005"])
        self.assertEqual(report["pilot_scenario_cluster_count"], 4)
        self.assertEqual(report["unmatched_candidate_scenario_count"], 1)

    def test_confirmatory_pilot_is_rejected(self):
        pilot, prereg, prereg_bytes = fixture()
        pilot["status"] = "confirmatory"
        with self.assertRaisesRegex(PlanningError, "screening/example-only"):
            plan(pilot, prereg, prereg_bytes)

    def test_preregistration_digest_mismatch_is_rejected(self):
        pilot, prereg, prereg_bytes = fixture()
        pilot["preregistration_sha256"] = "0" * 64
        with self.assertRaisesRegex(PlanningError, "preregistration digest mismatch"):
            plan(pilot, prereg, prereg_bytes)

    def test_development_or_unregistered_scenario_is_rejected(self):
        pilot, prereg, prereg_bytes = fixture()
        pilot["screening_scenario_ids"][0] = "DEV-001"
        with self.assertRaisesRegex(PlanningError, "exactly match the preregistered screening split"):
            plan(pilot, prereg, prereg_bytes)

    def test_duplicate_pilot_cluster_is_rejected(self):
        pilot, prereg, prereg_bytes = fixture()
        pilot["scenario_cluster_effects"].append(copy.deepcopy(pilot["scenario_cluster_effects"][0]))
        with self.assertRaisesRegex(PlanningError, "duplicate scenario-cluster effects"):
            plan(pilot, prereg, prereg_bytes)

    def test_zero_variance_uses_finite_conservative_floor(self):
        pilot, prereg, prereg_bytes = fixture()
        for row in pilot["scenario_cluster_effects"]:
            row["paired_difference"] = 0.10
        report = plan(pilot, prereg, prereg_bytes)
        self.assertTrue(report["variance_degenerate_floor_applied"])
        self.assertEqual(report["planning_standard_deviation"], 0.05)
        self.assertGreaterEqual(report["recommended_confirmatory_scenario_count"], 500)

    def test_more_comparisons_require_more_scenarios(self):
        pilot, prereg, prereg_bytes = fixture()
        sd = sample_sd([r["paired_difference"] for r in pilot["scenario_cluster_effects"]])
        one = required_n(sd, 0.10, 0.80, 0.05, 1)
        four = required_n(sd, 0.10, 0.80, 0.05, 4)
        self.assertGreater(four, one)

    def test_smaller_mde_requires_more_scenarios(self):
        pilot, prereg, prereg_bytes = fixture()
        sd = sample_sd([r["paired_difference"] for r in pilot["scenario_cluster_effects"]])
        five = required_n(sd, 0.05, 0.80, 0.05, 2)
        fifteen = required_n(sd, 0.15, 0.80, 0.05, 2)
        self.assertGreater(five, fifteen)

    def test_seed_repeats_are_not_an_input_sample_size(self):
        pilot, prereg, prereg_bytes = fixture()
        report = plan(pilot, prereg, prereg_bytes)
        self.assertEqual(report["pilot_scenario_cluster_count"], len(pilot["scenario_cluster_effects"]))
        self.assertIn("Scenario IDs, not repeated seeds", " ".join(report["limits"]))

    def test_invalid_effect_outside_probability_difference_range_rejected(self):
        pilot, prereg, prereg_bytes = fixture()
        pilot["scenario_cluster_effects"][0]["paired_difference"] = 1.2
        with self.assertRaisesRegex(PlanningError, "must be finite and in"):
            plan(pilot, prereg, prereg_bytes)

    def test_simulation_is_seed_deterministic(self):
        pilot, prereg, prereg_bytes = fixture()
        a = plan(pilot, prereg, prereg_bytes)
        b = plan(pilot, prereg, prereg_bytes)
        self.assertEqual(a["simulated_power_at_recommended_count"], b["simulated_power_at_recommended_count"])

    def test_duplicate_and_nonfinite_json_rejected(self):
        for payload in (b'{"x":1,"x":2}', b'{"x":NaN}'):
            path = Path("/tmp/not-used")
            with self.assertRaises(PlanningError):
                from plan_power import strict_object, reject_constant
                json.loads(payload, object_pairs_hook=strict_object, parse_constant=reject_constant)


    def test_real_screening_requires_prior_immutable_registration(self):
        pilot, prereg, _ = fixture()
        pilot["status"] = "screening"
        prereg["status"] = "registered_before_trials"
        prereg["registered_at"] = "2026-10-31T12:00:00Z"
        prereg_bytes = json.dumps(prereg, sort_keys=True).encode()
        pilot["preregistration_sha256"] = hashlib.sha256(prereg_bytes).hexdigest()
        pilot["pilot_started_at"] = "2026-10-31T13:00:00Z"
        self.assertGreaterEqual(plan(pilot, prereg, prereg_bytes)["recommended_confirmatory_scenario_count"], 500)

    def test_screening_started_before_registration_is_rejected(self):
        pilot, prereg, _ = fixture()
        pilot["status"] = "screening"
        prereg["status"] = "registered_before_trials"
        prereg["registered_at"] = "2026-10-31T14:00:00Z"
        prereg_bytes = json.dumps(prereg, sort_keys=True).encode()
        pilot["preregistration_sha256"] = hashlib.sha256(prereg_bytes).hexdigest()
        pilot["pilot_started_at"] = "2026-10-31T13:00:00Z"
        with self.assertRaisesRegex(PlanningError, "preregistration must predate screening pilot start"):
            plan(pilot, prereg, prereg_bytes)


if __name__ == "__main__":
    unittest.main()
