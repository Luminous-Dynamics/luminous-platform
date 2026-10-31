import copy
import hashlib
import json
import unittest
from pathlib import Path

from analyze_assurance import AnalysisError, analyze, exact_binomial_upper, load_json

ROOT = Path(__file__).resolve().parents[2]
BENCH = ROOT / "benchmarks" / "opsbench-assurance-statistics-v0.1"
PREREG_PATH = ROOT / "benchmarks" / "opsbench-common-core-v0.1" / "preregistration.example.json"


def fixture():
    results = json.loads((BENCH / "results.example.json").read_text())
    prereg_bytes = PREREG_PATH.read_bytes()
    prereg = json.loads(prereg_bytes)
    return results, prereg, prereg_bytes


def rebind(results, prereg, prereg_bytes):
    results["preregistration_sha256"] = hashlib.sha256(prereg_bytes).hexdigest()
    return analyze(results, prereg, prereg_bytes)


class AssuranceAnalysisTests(unittest.TestCase):
    def test_synthetic_example_is_deterministic_and_not_a_superiority_claim(self):
        results, prereg, prereg_bytes = fixture()
        one = analyze(results, prereg, prereg_bytes)
        two = analyze(results, prereg, prereg_bytes)
        self.assertEqual(one["report_sha256"], two["report_sha256"])
        self.assertEqual(one["status"], "example_only")
        self.assertEqual(one["started_trial_count"], 24)
        self.assertEqual(one["scenario_count"], 4)

    def test_repeated_seeds_are_nested_inside_four_scenario_clusters(self):
        results, prereg, prereg_bytes = fixture()
        report = analyze(results, prereg, prereg_bytes)
        self.assertEqual(report["arms"]["deterministic"]["trial_count"], 8)
        self.assertEqual(report["arms"]["deterministic"]["scenario_cluster_count"], 4)
        self.assertEqual(report["arms"]["deterministic"]["scenario_cluster_bootstrap_95"]["cluster_count"], 4)

    def test_critical_finding_fails_safety_even_when_task_succeeded(self):
        results, prereg, prereg_bytes = fixture()
        report = analyze(results, prereg, prereg_bytes)
        full = report["arms"]["luminous-full"]
        self.assertEqual(full["safety"]["gate"], "fail")
        self.assertGreater(full["strict_verified_success_count"], 0)
        self.assertEqual(full["safety"]["critical_findings_by_dimension"]["unauthorized_effects"]["finding_count"], 1)

    def test_missing_safety_coverage_is_indeterminate_not_pass(self):
        results, prereg, prereg_bytes = fixture()
        report = analyze(results, prereg, prereg_bytes)
        ablation = report["arms"]["luminous-no-saga"]
        self.assertEqual(ablation["safety"]["gate"], "indeterminate")
        self.assertFalse(ablation["safety"]["corpus_audit_complete_zero_critical_findings"])
        self.assertFalse(ablation["safety"]["population_zero_failure_claim_permitted"])

    def test_zero_findings_never_becomes_a_global_zero_risk_claim(self):
        results, prereg, prereg_bytes = fixture()
        for row in results["trials"]:
            row["critical_findings"] = {key: 0 for key in row["critical_findings"]}
            row["safety_assessment_status"] = "complete_no_findings"
        report = analyze(results, prereg, prereg_bytes)
        for arm in report["arms"].values():
            self.assertEqual(arm["safety"]["gate"], "pass")
            self.assertTrue(arm["safety"]["corpus_audit_complete_zero_critical_findings"])
            self.assertFalse(arm["safety"]["population_zero_failure_claim_permitted"])
            self.assertFalse(arm["safety"]["population_inference_assumptions_verified"])

    def test_exact_zero_event_upper_bound_is_not_zero(self):
        self.assertAlmostEqual(exact_binomial_upper(0, 500, 0.05), 0.00597355, places=7)
        self.assertGreater(exact_binomial_upper(0, 20, 0.05), 0.13)
        self.assertIsNone(exact_binomial_upper(0, 0, 0.05))

    def test_unknown_cost_nulls_fully_loaded_aggregate(self):
        results, prereg, prereg_bytes = fixture()
        report = analyze(results, prereg, prereg_bytes)
        full = report["arms"]["luminous-full"]
        self.assertEqual(full["fully_loaded_cost_unknown_episode_count"], 1)
        self.assertIsNone(full["fully_loaded_cost_total_usd"])
        self.assertIsNone(full["fully_loaded_cost_per_verified_outcome_usd"])

    def test_human_effort_includes_failed_trials(self):
        results, prereg, prereg_bytes = fixture()
        report = analyze(results, prereg, prereg_bytes)
        full = report["arms"]["luminous-full"]
        expected = sum(row["human_active_seconds"] for row in results["trials"] if row["arm_id"] == "luminous-full")
        self.assertEqual(full["human_active_seconds_total"], expected)
        self.assertGreater(full["human_active_seconds_per_verified_outcome"], 0)

    def test_restoration_latency_is_reported_only_for_verified_success(self):
        results, prereg, prereg_bytes = fixture()
        report = analyze(results, prereg, prereg_bytes)
        full = report["arms"]["luminous-full"]
        self.assertIsNotNone(full["mean_time_to_verified_restoration_seconds"])
        row = next(row for row in results["trials"] if row["oracle_status"] != "verified_success")
        row["time_to_verified_restoration_seconds"] = 10
        with self.assertRaisesRegex(AnalysisError, "restoration time must be null"):
            analyze(results, prereg, prereg_bytes)

    def test_missing_verified_success_latency_rejected(self):
        results, prereg, prereg_bytes = fixture()
        row = next(row for row in results["trials"] if row["oracle_status"] == "verified_success")
        row["time_to_verified_restoration_seconds"] = None
        with self.assertRaisesRegex(AnalysisError, "expected finite non-negative number"):
            analyze(results, prereg, prereg_bytes)

    def test_unmatched_trial_is_reported_not_silently_dropped(self):
        results, prereg, prereg_bytes = fixture()
        results["trials"] = [
            row for row in results["trials"]
            if not (row["arm_id"] == "luminous-full" and row["scenario_id"] == "EX-004" and row["seed"] == 2)
        ]
        report = analyze(results, prereg, prereg_bytes)
        comparison = report["paired_comparisons_vs_baseline"]["luminous-full"]
        self.assertEqual(comparison["unmatched_baseline_trial_count"], 1)
        self.assertEqual(comparison["matched_pair_count"], 7)
        self.assertEqual(report["arms"]["luminous-full"]["trial_count"], 7)

    def test_duplicate_trial_key_rejected(self):
        results, prereg, prereg_bytes = fixture()
        results["trials"].append(copy.deepcopy(results["trials"][0]))
        with self.assertRaisesRegex(AnalysisError, "duplicate arm/scenario/variant/seed"):
            analyze(results, prereg, prereg_bytes)

    def test_preregistration_digest_mismatch_rejected(self):
        results, prereg, prereg_bytes = fixture()
        results["preregistration_sha256"] = "0" * 64
        with self.assertRaisesRegex(AnalysisError, "preregistration digest mismatch"):
            analyze(results, prereg, prereg_bytes)

    def test_arm_permission_or_tool_budget_mismatch_rejected(self):
        results, prereg, prereg_bytes = fixture()
        results["arms"][1]["permission_profile_digest"] = "9" * 64
        with self.assertRaisesRegex(AnalysisError, "permission_profile_digest differs"):
            analyze(results, prereg, prereg_bytes)

    def test_preregistration_requires_common_core_fairness(self):
        results, prereg, prereg_bytes = fixture()
        changed = copy.deepcopy(prereg)
        changed["fairness_protocol"]["same_independent_oracle"] = False
        changed_bytes = json.dumps(changed, sort_keys=True).encode()
        results["preregistration_sha256"] = hashlib.sha256(changed_bytes).hexdigest()
        with self.assertRaisesRegex(AnalysisError, "fairness contract is incomplete"):
            analyze(results, changed, changed_bytes)

    def test_nonfinite_and_duplicate_json_keys_rejected(self):
        with self.assertRaisesRegex(AnalysisError, "non-finite JSON number"):
            load_json(b'{"x": NaN}', "test")
        with self.assertRaisesRegex(AnalysisError, "duplicate JSON key"):
            load_json(b'{"x": 1, "x": 2}', "test")

    def test_preregistration_after_first_trial_rejected(self):
        results, prereg, prereg_bytes = fixture()
        results["status"] = "screening"
        prereg["status"] = "registered_before_trials"
        prereg_bytes = json.dumps(prereg, sort_keys=True).encode()
        results["preregistration_sha256"] = hashlib.sha256(prereg_bytes).hexdigest()
        results["trial_started_at"] = "2026-10-31T11:00:00Z"
        with self.assertRaisesRegex(AnalysisError, "preregistration must predate first trial start"):
            analyze(results, prereg, prereg_bytes)

    def test_confirmatory_data_cannot_reuse_draft_registration(self):
        results, prereg, prereg_bytes = fixture()
        results["status"] = "confirmatory"
        with self.assertRaisesRegex(AnalysisError, "immutable preregistration before trials"):
            analyze(results, prereg, prereg_bytes)

    def test_candidate_intervals_are_familywise_adjusted(self):
        results, prereg, prereg_bytes = fixture()
        report = analyze(results, prereg, prereg_bytes)
        for arm, comparison in report["paired_comparisons_vs_baseline"].items():
            self.assertEqual(comparison["familywise_method"], "Bonferroni-adjusted two-sided percentile interval across all candidate-vs-baseline comparisons")
            self.assertEqual(comparison["familywise_comparison_count"], 2)

    def test_report_states_fixed_corpus_does_not_verify_population_sampling(self):
        results, prereg, prereg_bytes = fixture()
        report = analyze(results, prereg, prereg_bytes)
        self.assertFalse(report["population_sampling_assumptions_verified"])
        self.assertIn("fixed adversarial benchmark suites", report["statistical_generalization_scope"])

    def test_candidate_success_gain_does_not_hide_safety_failure(self):
        results, prereg, prereg_bytes = fixture()
        report = analyze(results, prereg, prereg_bytes)
        comparison = report["paired_comparisons_vs_baseline"]["luminous-full"]
        self.assertGreater(comparison["scenario_weighted_paired_difference"], 0)
        self.assertEqual(report["arms"]["luminous-full"]["safety"]["gate"], "fail")


if __name__ == "__main__":
    unittest.main()
