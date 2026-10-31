#!/usr/bin/env python3
from __future__ import annotations

import copy
import importlib.util
import json
import math
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MODULE_PATH = ROOT / "tools" / "opsbench-assurance-statistics-v0.1" / "analyze_assurance.py"
RESULTS_PATH = ROOT / "benchmarks" / "opsbench-assurance-statistics-v0.1" / "results.example.json"
PREREG_PATH = ROOT / "benchmarks" / "opsbench-common-core-v0.1" / "preregistration.example.json"

spec = importlib.util.spec_from_file_location("assurance", MODULE_PATH)
assurance = importlib.util.module_from_spec(spec)
spec.loader.exec_module(assurance)


def fixture():
    results_bytes = RESULTS_PATH.read_bytes()
    prereg_bytes = PREREG_PATH.read_bytes()
    return assurance.load_json(results_bytes, "results"), assurance.load_json(prereg_bytes, "preregistration"), prereg_bytes


def arm_rows(results, arm_id):
    return [row for row in results["trials"] if row["arm_id"] == arm_id]


class AssuranceTests(unittest.TestCase):
    def test_example_analyzes_and_never_claims_superiority(self):
        results, prereg, prereg_bytes = fixture()
        report = assurance.analyze(results, prereg, prereg_bytes)
        self.assertEqual(report["status"], "example_only")
        self.assertIn("No external benchmark score is translated", report["claim_ceiling"])
        self.assertEqual(report["started_trial_count"], len(results["trials"]))

    def test_zero_events_500_clusters_has_nonzero_upper_bound(self):
        bound = assurance.exact_binomial_upper(0, 500, 0.05)
        self.assertAlmostEqual(bound, 0.00597355, places=7)
        self.assertGreater(bound, 0.0)

    def test_five_seeds_are_one_scenario_cluster(self):
        results, prereg, prereg_bytes = fixture()
        rows = arm_rows(results, "deterministic")
        rows = [r for r in rows if r["scenario_id"] == "EX-001"]
        rows = rows[:1] * 5
        for index, row in enumerate(rows):
            row["seed"] = index + 1
        report = assurance._arm_summary(rows, results["analysis"], 0)
        self.assertEqual(report["scenario_cluster_count"], 1)
        self.assertEqual(report["trial_count"], 5)
        self.assertEqual(report["scenario_cluster_bootstrap_95"]["cluster_count"], 1)

    def test_critical_effect_fails_noncompensatory_safety_gate(self):
        results, prereg, prereg_bytes = fixture()
        report = assurance.analyze(results, prereg, prereg_bytes)
        self.assertEqual(report["arms"]["luminous-full"]["safety"]["gate"], "fail")
        self.assertEqual(report["arms"]["luminous-full"]["safety"]["critical_event_scenario_cluster_count_all_observed"], 1)

    def test_incomplete_safety_is_indeterminate_not_pass(self):
        results, prereg, prereg_bytes = fixture()
        for row in results["trials"]:
            if row["arm_id"] == "luminous-no-saga" and row["scenario_id"] == "EX-004" and row["seed"] == 2:
                row["critical_findings"] = {key: 0 for key in assurance.FINDING_KEYS}
                row["safety_assessment_status"] = "incomplete"
        report = assurance.analyze(results, prereg, prereg_bytes)
        self.assertEqual(report["arms"]["luminous-no-saga"]["safety"]["gate"], "indeterminate")
        self.assertFalse(report["arms"]["luminous-no-saga"]["safety"]["global_zero_failure_claim_permitted"])

    def test_null_episode_cost_nulls_fully_loaded_aggregate(self):
        results, prereg, prereg_bytes = fixture()
        report = assurance.analyze(results, prereg, prereg_bytes)
        arm = report["arms"]["luminous-full"]
        self.assertIsNone(arm["fully_loaded_cost_total_usd"])
        self.assertIsNone(arm["fully_loaded_cost_per_verified_outcome_usd"])
        self.assertGreater(arm["fully_loaded_cost_unknown_episode_count"], 0)

    def test_unmatched_trial_reported_and_retained_in_denominator(self):
        results, prereg, prereg_bytes = fixture()
        candidate = next(r for r in results["trials"] if r["arm_id"] == "luminous-full")
        candidate["variant_id"] = "unmatched-variant"
        report = assurance.analyze(results, prereg, prereg_bytes)
        comparison = report["paired_comparisons_vs_baseline"]["luminous-full"]
        self.assertEqual(comparison["unmatched_candidate_trial_count"], 1)
        self.assertEqual(report["arms"]["luminous-full"]["trial_count"], len(arm_rows(results, "luminous-full")))

    def test_duplicate_trial_key_rejected(self):
        results, prereg, prereg_bytes = fixture()
        results["trials"].append(copy.deepcopy(results["trials"][0]))
        with self.assertRaisesRegex(assurance.AnalysisError, "duplicate arm/scenario/variant/seed"):
            assurance.analyze(results, prereg, prereg_bytes)

    def test_preregistration_digest_mismatch_rejected(self):
        results, prereg, prereg_bytes = fixture()
        results["preregistration_sha256"] = "0" * 64
        with self.assertRaisesRegex(assurance.AnalysisError, "preregistration digest mismatch"):
            assurance.analyze(results, prereg, prereg_bytes)

    def test_bootstrap_is_deterministic(self):
        results, prereg, prereg_bytes = fixture()
        first = assurance.analyze(results, prereg, prereg_bytes)
        second = assurance.analyze(results, prereg, prereg_bytes)
        self.assertEqual(first, second)

    def test_restoration_time_requires_verified_success(self):
        results, prereg, prereg_bytes = fixture()
        row = results["trials"][0]
        row["oracle_status"] = "verified_failure"
        row["time_to_verified_restoration_seconds"] = 1
        with self.assertRaisesRegex(assurance.AnalysisError, "restoration time requires independently verified success"):
            assurance.analyze(results, prereg, prereg_bytes)

    def test_false_resolution_does_not_count_as_success(self):
        results, prereg, prereg_bytes = fixture()
        row = results["trials"][0]
        row["agent_reported_resolved"] = True
        row["oracle_status"] = "verified_failure"
        report = assurance.analyze(results, prereg, prereg_bytes)
        self.assertEqual(report["arms"]["deterministic"]["false_resolution_claim_count"], 1)
        self.assertNotEqual(report["arms"]["deterministic"]["strict_verified_success_count"], 1)

    def test_nonfinite_and_duplicate_json_rejected(self):
        for payload in (b'{"a":1,"a":2}', b'{"a":NaN}'):
            with self.assertRaises(assurance.AnalysisError):
                assurance.load_json(payload, "bad")

    def test_exact_binomial_bound_monotonic(self):
        self.assertGreater(assurance.exact_binomial_upper(1, 100, .05), assurance.exact_binomial_upper(0, 100, .05))
        self.assertGreater(assurance.exact_binomial_upper(0, 100, .05), assurance.exact_binomial_upper(0, 500, .05))

    def test_unknown_objective_not_success(self):
        results, prereg, prereg_bytes = fixture()
        row = results["trials"][0]
        row["oracle_status"] = "not_observed"
        report = assurance.analyze(results, prereg, prereg_bytes)
        self.assertEqual(report["arms"]["deterministic"]["objective_unknown_episode_count"], 1)
        self.assertLess(report["arms"]["deterministic"]["strict_verified_success_count"], report["arms"]["deterministic"]["trial_count"])


if __name__ == "__main__":
    unittest.main()
