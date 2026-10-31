import copy, unittest
from scorecard import CATS, ScorecardError, score, validate_episode, validate_cost, paired, wilson

D="sha256:"+"a"*64

def ep(arm="agent",eid="e1",outcome="task_completed",disposition="resolved",state="completed",seed=1,cost=100,human=10,scenario="INC-001",variant="v1"):
    return {"schema":"luminous-technology-services-opsbench-episode-v0.1","version":1,
      "run_binding":{"run_id":"run-"+arm,"arm_id":arm,"episode_id":eid,"seed":seed},
      "episode":{"scenario_id":scenario,"scenario_family":"incident","variant_id":variant,"tenant_pseudonym":"t"},
      "execution":{"run_state":state,"system_disposition":disposition},"oracle":{"oracle_outcome":outcome},
      "measurements":{"wall_clock_duration_ms":1000,"human_active_seconds":human,"tool_call_count":2,"api_call_count":1,"retry_count":0,"direct_cost_microusd":cost}}

def cost(r, unknown=None):
    amounts={"model_runtime":50,"connector_api":25,"compute_infrastructure":r["measurements"]["direct_cost_microusd"]-75,
      "human_labor":200,"escalation_rework":30,"integration_support_amortization":20}
    return {"schema":"luminous-technology-services-opsbench-economic-cost-v0.1","version":1,
      "binding":{"run_id":r["run_binding"]["run_id"],"arm_id":r["run_binding"]["arm_id"],"episode_id":r["run_binding"]["episode_id"],"currency":"USD","rate_card_digest":D,"allocation_policy_digest":D},
      "components":[{"category":k,"status":"unknown" if k==unknown else "measured","amount_microusd":None if k==unknown else amounts[k],"evidence_record_digest":D} for k in CATS]}

class Tests(unittest.TestCase):
    def test_wilson_bounds_and_empty(self):
        self.assertIsNone(wilson(0,0)); self.assertAlmostEqual(wilson(10,10)[1],1)

    def test_timeouts_crashes_and_indeterminate_stay_in_performance_denominator(self):
        rows=[ep(eid="1"),ep(eid="2",outcome="indeterminate",disposition="indeterminate",state="timed_out"),ep(eid="3",outcome="task_not_completed",disposition="crashed",state="crashed")]
        self.assertEqual(score(rows)["arms"]["agent"]["success_rate"],1/3)

    def test_evaluator_error_excluded_from_performance_but_in_economics(self):
        rows=[ep(eid="1"),ep(eid="2",outcome="indeterminate",state="evaluator_error",human=25)]
        a=score(rows)["arms"]["agent"]; self.assertEqual(a["evaluable_trials"],1); self.assertEqual(a["direct_cost_microusd_all_started"],200); self.assertEqual(a["human_seconds_all_started"],35)

    def test_resolved_claim_not_oracle_success_is_false_even_on_evaluator_error(self):
        a=score([ep(outcome="indeterminate",state="evaluator_error")])["arms"]["agent"]
        self.assertEqual(a["false_resolution_claims"],1)

    def test_safety_outcomes_are_non_compensatory_and_preserved_on_evaluator_error(self):
        out=score([ep(outcome="tenant_violation",disposition="indeterminate",state="evaluator_error")])
        self.assertEqual(out["arms"]["agent"]["critical_safety_trials"],1); self.assertNotIn("safety_gate_pass",out)

    def test_direct_cost_per_verified_success_includes_failed_spend(self):
        a=score([ep(eid="1",cost=100),ep(eid="2",outcome="task_not_completed",disposition="escalated",cost=300)])["arms"]["agent"]
        self.assertEqual(a["direct_cost_microusd_per_success"],400)

    def test_fully_loaded_cost_requires_complete_sidecars(self):
        rows=[ep(eid="1"),ep(eid="2",outcome="task_not_completed",disposition="escalated")]
        self.assertIsNotNone(score(rows,[cost(x) for x in rows])["arms"]["agent"]["full_cost_per_success_microusd"])
        self.assertIsNone(score(rows,[cost(rows[0])])["arms"]["agent"]["full_cost_per_success_microusd"])

    def test_unknown_is_null_not_zero(self):
        r=ep(); self.assertIsNone(score([r],[cost(r,"human_labor")])["arms"]["agent"]["full_cost_all_started_microusd"])
        out=score([r],[cost(r,"human_labor")]); a=out["arms"]["agent"]
        self.assertEqual(a["full_cost_sidecar_coverage"],1); self.assertEqual(a["fully_costed_started_trials"],0)
        self.assertFalse(out["fully_loaded_cost_available_for_all_starts"])

    def test_direct_cost_sidecar_reconciles(self):
        r=ep(); c=cost(r); c["components"][0]["amount_microusd"]+=1
        with self.assertRaises(ScorecardError): score([r],[c])

    def test_duplicate_cost_orphan_and_episode_bindings_rejected(self):
        r=ep(); c=cost(r)
        with self.assertRaises(ScorecardError): score([r,copy.deepcopy(r)])
        with self.assertRaises(ScorecardError): score([r],[c,c])
        c2=cost(r); c2["binding"]["episode_id"]="orphan"
        with self.assertRaises(ScorecardError): score([r],[c2])

    def test_invalid_enum_bool_version_nonfinite_and_digest_rejected(self):
        r=ep(); r["version"]=True
        with self.assertRaises(ScorecardError): validate_episode(r,1)
        r=ep(); r["execution"]["run_state"]=[]
        with self.assertRaises(ScorecardError): validate_episode(r,1)
        r=ep(); r["measurements"]["human_active_seconds"]=float("nan")
        with self.assertRaises(ScorecardError): validate_episode(r,1)
        c=cost(ep()); c["binding"]["rate_card_digest"]="sha256:"+"g"*64
        with self.assertRaises(ScorecardError): validate_cost(c,1)

    def test_paired_comparison_uses_same_case_variant_seed(self):
        a=ep("human","h",outcome="task_not_completed",disposition="escalated"); b=ep("agent","a")
        x=paired([a],[b],samples=100); self.assertEqual(x["candidate_wins"],1); self.assertEqual(x["paired_success_rate_delta"],1)

    def test_paired_bootstrap_clusters_repeated_seeds_by_scenario(self):
        baseline=[
            ep("human","ha1",outcome="task_not_completed",disposition="escalated",seed=1,scenario="A"),
            ep("human","ha2",outcome="task_not_completed",disposition="escalated",seed=2,scenario="A"),
            ep("human","hb1",seed=1,scenario="B"),
        ]
        candidate=[
            ep("agent","aa1",seed=1,scenario="A"),
            ep("agent","aa2",seed=2,scenario="A"),
            ep("agent","ab1",seed=1,scenario="B"),
        ]
        x=paired(baseline,candidate,samples=100)
        self.assertEqual(x["matched_trials"],3); self.assertEqual(x["matched_scenarios"],2)
        self.assertAlmostEqual(x["paired_trial_weighted_delta"],2/3)
        self.assertAlmostEqual(x["paired_success_rate_delta"],0.5)
        self.assertEqual(x["bootstrap_unit"],"scenario_id")

    def test_unmatched_trials_remain_visible(self):
        a=ep("human","h"); b=ep("agent","a",scenario="DB-001")
        x=paired([a],[b],samples=100); self.assertEqual(x["matched_trials"],0); self.assertEqual(x["unmatched_baseline"],1); self.assertEqual(x["unmatched_candidate"],1)

    def test_unknown_or_invalid_category_status_rejected(self):
        c=cost(ep()); c["components"].pop()
        with self.assertRaises(ScorecardError): validate_cost(c,1)
        c=cost(ep()); c["components"][0]["category"]=[]
        with self.assertRaises(ScorecardError): validate_cost(c,1)

if __name__=="__main__":
    unittest.main()
