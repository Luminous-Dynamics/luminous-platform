import copy
import unittest
from pathlib import Path

from evaluate_voi import StudyError, evaluate, load_json, probe_information

ROOT = Path(__file__).resolve().parents[2]
STUDY_PATH = ROOT / "benchmarks" / "opsbench-active-evidence-voi-v0.1" / "study.example.json"


def fixture():
    return load_json(STUDY_PATH.read_bytes(), "study")


class ActiveEvidenceVOITests(unittest.TestCase):
    def test_example_report_is_explicitly_non_confirmatory(self):
        report = evaluate(fixture())
        self.assertEqual(report["status"], "example_only")
        self.assertEqual(report["scenario_count"], 4)
        self.assertIn("oracle_perfect_information", report["strategies"])
        self.assertFalse(report["automatic_execution_authorized"])
        self.assertIn("does not execute probes", report["claim_ceiling"])

    def test_all_strategies_obey_the_same_hard_budgets(self):
        study = fixture()
        report = evaluate(study)
        for result in report["scenario_results"].values():
            for strategy in (
                "random", "cheapest_first", "entropy_per_cost",
                "fixed_runbook", "risk_adjusted_voi",
            ):
                row = result[strategy]
                policy = study["policy"]
                self.assertLessEqual(row["probe_count"], policy["max_probes"])
                self.assertLessEqual(row["cost_units"], policy["max_cost_units"] + 1e-12)
                self.assertLessEqual(row["latency_ms"], policy["max_latency_ms"])
                self.assertLessEqual(row["privacy_penalty_units"], policy["max_privacy_penalty_units"] + 1e-12)
                self.assertLessEqual(row["human_seconds"], policy["max_human_seconds"] + 1e-12)

    def test_oracle_is_explicitly_evaluator_only_perfect_information_upper_bound(self):
        report = evaluate(fixture())
        oracle = report["strategies"]["oracle_perfect_information"]
        self.assertEqual(oracle["unsafe_recommendation_count"], 0)
        self.assertEqual(oracle["mean_realized_decision_loss"], 0.0)
        self.assertEqual(oracle["mean_probes"], 0.0)
        for result in report["scenario_results"].values():
            self.assertEqual(result["oracle_perfect_information"]["stop_reason"],
                             "evaluator_only_perfect_information_upper_bound")

    def test_write_capable_probe_is_rejected(self):
        study = fixture()
        study["probes"][0]["read_only"] = False
        with self.assertRaisesRegex(StudyError, "write-capable probes are forbidden"):
            evaluate(study)

    def test_probe_source_must_match_authorized_source_and_trust_domain(self):
        study = fixture()
        study["probes"][0]["trust_domain_id"] = "untrusted-domain"
        with self.assertRaisesRegex(StudyError, "source/trust domain/scope not authorized"):
            evaluate(study)

    def test_probe_scope_escape_is_rejected(self):
        study = fixture()
        study["probes"][0]["scope_id"] = "different-tenant"
        with self.assertRaisesRegex(StudyError, "scope differs from study scope"):
            evaluate(study)

    def test_likelihoods_must_sum_to_one(self):
        study = fixture()
        study["probes"][0]["likelihoods"]["dependency_degradation"]["signal"] = 0.7
        with self.assertRaisesRegex(StudyError, "likelihoods must sum to one"):
            evaluate(study)

    def test_prior_probabilities_must_sum_to_one(self):
        study = fixture()
        study["scenarios"][0]["prior"]["dependency_degradation"] = 0.5
        with self.assertRaisesRegex(StudyError, "prior probabilities must sum to one"):
            evaluate(study)

    def test_every_hypothesis_requires_a_mutating_response_option(self):
        study = fixture()
        study["actions"] = [a for a in study["actions"] if a["target_hypothesis"] != "network_partition"]
        with self.assertRaisesRegex(StudyError, "mutating response for every hypothesis"):
            evaluate(study)

    def test_every_source_hypothesis_prior_and_outcome_deck_is_pinned(self):
        study = fixture()
        study["scenarios"][0]["outcome_deck"][0]["source_id"] = "forged-source"
        with self.assertRaisesRegex(StudyError, "oracle outcome publisher identity mismatch"):
            evaluate(study)

    def test_stale_observation_does_not_update_the_posterior(self):
        study = fixture()
        study["fixed_runbook_order"] = [
            "network-path", "dependency-latency", "dependency-trace", "deploy-diff",
            "deploy-runtime", "credential-audit", "issuer-health", "network-route",
        ]
        row = next(s for s in study["scenarios"] if s["scenario_id"] == "VOI-004")
        result = evaluate(study)["scenario_results"]["VOI-004"]["fixed_runbook"]
        self.assertTrue(result["observations"][0]["stale"])
        self.assertFalse(result["observations"][0]["posterior_updated"])
        self.assertEqual(result["observations"][0]["posterior_after"], row["prior"])
        self.assertGreater(result["stale_evidence_count"], 0)

    def test_duplicate_evidence_id_cannot_be_counted_twice(self):
        study = fixture()
        study["fixed_runbook_order"] = [
            "dependency-latency", "dependency-trace", "deploy-diff", "deploy-runtime",
            "credential-audit", "issuer-health", "network-route", "network-path",
        ]
        row = next(s for s in study["scenarios"] if s["scenario_id"] == "VOI-001")
        row["outcome_deck"][1]["evidence_id"] = row["outcome_deck"][0]["evidence_id"]
        result = evaluate(study)["scenario_results"]["VOI-001"]["fixed_runbook"]
        self.assertGreaterEqual(len(result["observations"]), 2)
        first, second = result["observations"][:2]
        self.assertFalse(first["duplicate"])
        self.assertTrue(second["duplicate"])
        self.assertFalse(second["posterior_updated"])
        self.assertEqual(first["posterior_after"], second["posterior_after"])
        self.assertGreater(result["duplicate_evidence_count"], 0)

    def test_correlated_probes_in_one_group_are_not_double_counted(self):
        study = fixture()
        study["probes"][1]["correlation_group"] = study["probes"][0]["correlation_group"]
        study["fixed_runbook_order"] = [
            "dependency-latency", "dependency-trace", "deploy-diff", "deploy-runtime",
            "credential-audit", "issuer-health", "network-route", "network-path",
        ]
        result = evaluate(study)["scenario_results"]["VOI-001"]["fixed_runbook"]
        selected = [x["probe_id"] for x in result["observations"]]
        self.assertIn("dependency-latency", selected)
        self.assertNotIn("dependency-trace", selected)

    def test_impossible_observation_is_preserved_as_anomaly_not_forced_into_posterior(self):
        study = fixture()
        probe = study["probes"][0]
        for hid in probe["likelihoods"]:
            probe["likelihoods"][hid] = {"signal": 0.0, "clear": 1.0}
        row = study["scenarios"][0]
        row["outcome_deck"][0]["outcome"] = "signal"
        result = evaluate(study)["scenario_results"]["VOI-001"]["fixed_runbook"]
        first = result["observations"][0]
        self.assertTrue(first["impossible_under_model"])
        self.assertFalse(first["posterior_updated"])
        self.assertEqual(first["posterior_after"], row["prior"])
        self.assertGreater(result["impossible_observation_count"], 0)

    def test_risk_adjusted_voi_stops_when_no_permitted_action_can_improve(self):
        study = fixture()
        study["policy"]["max_action_risk"] = 0.0
        result = evaluate(study)["scenario_results"]["VOI-001"]["risk_adjusted_voi"]
        self.assertEqual(result["probe_count"], 0)
        self.assertEqual(result["stop_reason"], "non_positive_net_voi")
        self.assertTrue(result["escalated"])

    def test_entropy_reduction_is_not_treated_as_decision_value(self):
        study = fixture()
        study["policy"]["max_action_risk"] = 0.0
        distribution = study["scenarios"][0]["prior"]
        info = probe_information(distribution, study["probes"][1], {
            a["action_id"]: a for a in study["actions"]
        }, study["policy"]["max_action_risk"])
        self.assertGreater(info["entropy_reduction"], 0.0)
        self.assertEqual(info["evsi"], 0.0)
        self.assertLessEqual(info["net_value"], 0.0)

    def test_mutating_recommendations_must_satisfy_declared_risk_threshold(self):
        study = fixture()
        study["policy"]["max_cost_units"] = 0.0
        report = evaluate(study)
        for scenario in report["scenario_results"].values():
            for strategy in ("random", "cheapest_first", "entropy_per_cost", "fixed_runbook", "risk_adjusted_voi"):
                self.assertFalse(scenario[strategy]["direct_recommendation"])
                self.assertTrue(scenario[strategy]["escalated"])

    def test_resource_cost_conversion_is_preregistered_and_used_in_net_voi(self):
        study = fixture()
        distribution = study["scenarios"][0]["prior"]
        actions = {action["action_id"]: action for action in study["actions"]}
        probe = study["probes"][1]
        ordinary = probe_information(distribution, probe, actions, study["policy"])
        changed_policy = dict(study["policy"])
        changed_policy["cost_loss_per_unit"] = 10.0
        expensive = probe_information(distribution, probe, actions, changed_policy)
        self.assertAlmostEqual(
            ordinary["net_value"] - expensive["net_value"],
            probe["cost_units"] * 9.0,
            places=12,
        )

    def test_privacy_penalty_conversion_is_preregistered_and_used_in_net_voi(self):
        study = fixture()
        distribution = study["scenarios"][0]["prior"]
        actions = {action["action_id"]: action for action in study["actions"]}
        probe = study["probes"][1]
        ordinary = probe_information(distribution, probe, actions, study["policy"])
        changed_policy = dict(study["policy"])
        changed_policy["privacy_loss_per_unit"] = 8.0
        private = probe_information(distribution, probe, actions, changed_policy)
        self.assertAlmostEqual(
            ordinary["net_value"] - private["net_value"],
            probe["privacy_penalty_units"] * 7.0,
            places=12,
        )

    def test_probe_correlation_group_is_not_reused_within_one_strategy(self):
        study = fixture()
        report = evaluate(study)
        probes = {p["probe_id"]: p for p in study["probes"]}
        for scenario in report["scenario_results"].values():
            row = scenario["risk_adjusted_voi"]
            groups = [probes[item["probe_id"]]["correlation_group"] for item in row["observations"]]
            self.assertEqual(len(groups), len(set(groups)))

    def test_seeded_strategy_outputs_are_deterministic(self):
        study = fixture()
        first = evaluate(copy.deepcopy(study))
        second = evaluate(copy.deepcopy(study))
        self.assertEqual(first["report_sha256"], second["report_sha256"])
        self.assertEqual(first["strategies"], second["strategies"])

    def test_oracle_hidden_truth_is_not_read_by_non_oracle_strategy_selection(self):
        study = fixture()
        baseline = evaluate(study)
        changed = copy.deepcopy(study)
        for scenario in changed["scenarios"]:
            scenario["truth_hypothesis"] = (
                "bad_deploy" if scenario["truth_hypothesis"] != "bad_deploy" else "dependency_degradation"
            )
        counterfactual = evaluate(changed)
        for strategy in ("random", "cheapest_first", "entropy_per_cost", "fixed_runbook", "risk_adjusted_voi"):
            for sid in baseline["scenario_results"]:
                left = baseline["scenario_results"][sid][strategy]
                right = counterfactual["scenario_results"][sid][strategy]
                self.assertEqual([o["probe_id"] for o in left["observations"]],
                                 [o["probe_id"] for o in right["observations"]])


    def test_decision_voi_prefers_probe_that_can_cross_safety_threshold(self):
        study = fixture()
        report = evaluate(study)
        risk_first = report["scenario_results"]["VOI-001"]["risk_adjusted_voi"]["observations"][0]["probe_id"]
        cheap_first = report["scenario_results"]["VOI-001"]["cheapest_first"]["observations"][0]["probe_id"]
        high_quality = {"dependency-trace", "deploy-runtime", "issuer-health", "network-path"}
        low_cost = {"dependency-latency", "deploy-diff", "credential-audit", "network-route"}
        self.assertIn(risk_first, high_quality)
        self.assertIn(cheap_first, low_cost)

    def test_voi_prediction_and_realized_decision_risk_are_reported_separately(self):
        report = evaluate(fixture())
        row = report["scenario_results"]["VOI-001"]["risk_adjusted_voi"]
        self.assertIn("mean_predicted_evsi_per_probe", row)
        self.assertIn("mean_realized_risk_reduction_per_probe", row)
        strategy = report["strategies"]["risk_adjusted_voi"]
        self.assertIn("mean_predicted_evsi_per_probe", strategy)
        self.assertIn("mean_realized_risk_reduction_per_probe", strategy)

    def test_report_never_authorizes_probe_or_remediation_execution(self):
        report = evaluate(fixture())
        self.assertIs(report["automatic_execution_authorized"], False)
        self.assertEqual(report["status"], "example_only")



if __name__ == "__main__":
    unittest.main()
