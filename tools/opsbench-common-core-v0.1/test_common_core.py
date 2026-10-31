import copy
import json
import unittest
from pathlib import Path
from check_common_core import (
    ContractError, load, validate_crosswalk, validate_prereg, strict_object,
)

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "benchmarks" / "opsbench-common-core-v0.1"

class CommonCoreContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.crosswalk = load(BASE / "external-benchmark-crosswalk.json")
        cls.prereg = load(BASE / "preregistration.example.json")

    def test_checked_in_crosswalk_and_preregistration_pass(self):
        validate_crosswalk(self.crosswalk)
        validate_prereg(self.prereg)

    def test_duplicate_json_keys_are_rejected(self):
        with self.assertRaises(ContractError):
            strict_object([("status","draft"),("status","registered")])

    def test_crosswalk_direct_score_translation_cannot_be_enabled(self):
        changed=copy.deepcopy(self.crosswalk)
        changed["benchmarks"][0]["direct_score_comparison_allowed"]=True
        with self.assertRaises(ContractError): validate_crosswalk(changed)

    def test_crosswalk_requires_benchmark_limitations(self):
        changed=copy.deepcopy(self.crosswalk)
        changed["benchmarks"][0]["limitations"]=[]
        with self.assertRaises(ContractError): validate_crosswalk(changed)

    def test_itbench_metric_must_remain_diagnostic_not_task_success(self):
        changed=copy.deepcopy(self.crosswalk)
        item=next(x for x in changed["benchmarks"] if x["id"]=="itbench")
        item["primary_metric"]["name"]="strict whole-task success"
        with self.assertRaises(ContractError): validate_crosswalk(changed)

    def test_enterpriseops_metric_must_remain_strict_whole_task_success(self):
        changed=copy.deepcopy(self.crosswalk)
        item=next(x for x in changed["benchmarks"] if x["id"]=="servicenow_enterpriseops_gym")
        item["primary_metric"]["name"]="action count"
        with self.assertRaises(ContractError): validate_crosswalk(changed)

    def test_public_opsbench_corpus_cannot_be_marked_blind(self):
        changed=copy.deepcopy(self.crosswalk)
        item=next(x for x in changed["benchmarks"] if x["id"]=="luminous_opsbench_v0_1")
        item["reported_scope"]["blind_holdout"]=True
        with self.assertRaises(ContractError): validate_crosswalk(changed)

    def test_crosswalk_requires_non_equivalence_rules(self):
        changed=copy.deepcopy(self.crosswalk)
        changed["comparison_contract"]["never_translate"]=[]
        with self.assertRaises(ContractError): validate_crosswalk(changed)

    def test_development_and_screening_splits_must_be_disjoint(self):
        changed=copy.deepcopy(self.prereg)
        changed["scenario_split"]["screening_scenario_ids"].append(changed["scenario_split"]["development_scenario_ids"][0])
        with self.assertRaises(ContractError): validate_prereg(changed)

    def test_all_started_and_missing_evidence_denominators_are_mandatory(self):
        changed=copy.deepcopy(self.prereg)
        changed["denominators"]["crash_included"]=False
        with self.assertRaises(ContractError): validate_prereg(changed)
        changed=copy.deepcopy(self.prereg)
        changed["denominators"]["missing_safety_assessment_is_not_pass"]=False
        with self.assertRaises(ContractError): validate_prereg(changed)

    def test_same_tools_permissions_and_budgets_are_required(self):
        changed=copy.deepcopy(self.prereg)
        changed["arms"][0]["tool_surface_digest"]="0"*64
        with self.assertRaises(ContractError): validate_prereg(changed)
        changed=copy.deepcopy(self.prereg)
        changed["fairness_protocol"]["same_permission_profile"]=False
        with self.assertRaises(ContractError): validate_prereg(changed)

    def test_comparison_arms_cannot_be_removed(self):
        changed=copy.deepcopy(self.prereg)
        changed["arms"]=[a for a in changed["arms"] if a["system_type"]!="deterministic_automation"]
        with self.assertRaises(ContractError): validate_prereg(changed)

    def test_repeated_seeds_must_be_at_least_five_and_clustered_by_scenario(self):
        changed=copy.deepcopy(self.prereg)
        changed["repetition_policy"]["screening_seeds_per_scenario"]=1
        with self.assertRaises(ContractError): validate_prereg(changed)
        changed=copy.deepcopy(self.prereg)
        changed["repetition_policy"]["analysis_cluster"]="trial"
        with self.assertRaises(ContractError): validate_prereg(changed)

    def test_safety_gates_must_remain_zero(self):
        changed=copy.deepcopy(self.prereg)
        changed["safety_gates"]["unauthorized_effects"]=1
        with self.assertRaises(ContractError): validate_prereg(changed)

    def test_external_score_translation_and_unsupported_sota_claims_rejected(self):
        changed=copy.deepcopy(self.prereg)
        changed["claim_policy"]["direct_external_score_translation_allowed"]=True
        with self.assertRaises(ContractError): validate_prereg(changed)
        changed=copy.deepcopy(self.prereg)
        changed["claim_policy"]["sota_claim_requires_same_harness_confirmatory_trials"]=False
        with self.assertRaises(ContractError): validate_prereg(changed)

    def test_minimum_confirmatory_size_and_clustered_uncertainty_required(self):
        changed=copy.deepcopy(self.prereg)
        changed["scenario_split"]["confirmatory_scenario_count_target"]=20
        with self.assertRaises(ContractError): validate_prereg(changed)
        changed=copy.deepcopy(self.prereg)
        changed["analysis"]["cluster_bootstrap_samples"]=100
        with self.assertRaises(ContractError): validate_prereg(changed)

    def test_unique_arm_ids_and_metrics_required(self):
        changed=copy.deepcopy(self.prereg)
        changed["arms"][1]["arm_id"]=changed["arms"][0]["arm_id"]
        with self.assertRaises(ContractError): validate_prereg(changed)
        changed=copy.deepcopy(self.prereg)
        changed["metrics"]=["strict_service_task_success"]
        with self.assertRaises(ContractError): validate_prereg(changed)

    def test_registration_time_must_be_timezone_aware(self):
        changed=copy.deepcopy(self.prereg)
        changed["registered_at"]="2026-10-31T12:00:00"
        with self.assertRaises(ContractError): validate_prereg(changed)

if __name__=="__main__":
    unittest.main(verbosity=2)
