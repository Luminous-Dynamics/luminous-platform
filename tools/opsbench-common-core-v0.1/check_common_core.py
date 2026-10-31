#!/usr/bin/env python3
"""Structural contract checker for the OpsBench common-core crosswalk and preregistration."""
from __future__ import annotations
import json
import sys
from pathlib import Path
from datetime import datetime

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "benchmarks" / "opsbench-common-core-v0.1"
CORE_METRICS = {
    "strict_service_task_success", "root_cause_localization", "decision_quality",
    "false_resolution_rate", "critical_effect_incidence", "safety_assessment_coverage",
    "recovery_completion_rate", "unknown_attempt_duration",
}
FAIRNESS_KEYS = {
    "same_initial_snapshot", "same_task_request", "same_tenant_scope",
    "same_permission_profile", "same_time_budget", "same_tool_call_budget",
    "same_independent_oracle", "same_fault_schedule", "mismatches_disclosed",
}
DENOMINATOR_KEYS = {
    "started_episode_included", "timeout_included", "crash_included", "blocked_included",
    "indeterminate_included", "evaluator_errors_separately_reported",
    "failed_attempt_cost_included", "missing_safety_assessment_is_not_pass", "unknown_cost_is_null",
}
SAFETY_KEYS = {
    "unauthorized_effects", "duplicate_business_effects", "false_service_resolution_claims",
    "cross_tenant_effects", "resolved_without_independent_verification",
}

class ContractError(ValueError):
    pass

def strict_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ContractError(f"duplicate JSON key: {key}")
        result[key] = value
    return result

def load(path):
    try:
        return json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=strict_object)
    except (OSError, json.JSONDecodeError) as exc:
        raise ContractError(f"{path}: {exc}") from exc

def require(obj, keys, where):
    if not isinstance(obj, dict):
        raise ContractError(f"{where}: expected object")
    missing = sorted(set(keys) - set(obj))
    if missing:
        raise ContractError(f"{where}: missing {missing}")

def validate_crosswalk(c):
    require(c, ("schema", "version", "status", "as_of", "rule", "benchmarks", "comparison_contract"), "crosswalk")
    if c["schema"] != "luminous-external-operations-benchmark-crosswalk-v0.1" or type(c["version"]) is not int or c["version"] != 1:
        raise ContractError("crosswalk: unsupported schema/version")
    if c["status"] != "research_reference_only":
        raise ContractError("crosswalk: must remain reference-only")
    if not isinstance(c["benchmarks"], list) or len(c["benchmarks"]) < 5:
        raise ContractError("crosswalk: at least five benchmark entries required")
    seen = set()
    for i, b in enumerate(c["benchmarks"]):
        where=f"crosswalk.benchmarks[{i}]"
        require(b, ("id", "name", "evaluation_surface", "reported_scope", "capabilities", "limitations", "direct_score_comparison_allowed", "sources"), where)
        if not isinstance(b["id"], str) or not b["id"] or b["id"] in seen:
            raise ContractError(f"{where}: duplicate/empty ID")
        seen.add(b["id"])
        if b["direct_score_comparison_allowed"] is not False:
            raise ContractError(f"{where}: direct score translation must remain prohibited")
        if not isinstance(b["limitations"], list) or not b["limitations"]:
            raise ContractError(f"{where}: limitations must be explicit")
        if not isinstance(b["sources"], list) or not b["sources"] or any(not isinstance(s,str) or not s.startswith("https://") for s in b["sources"]):
            raise ContractError(f"{where}: source links required")
        if not isinstance(b["capabilities"], list) or not b["capabilities"]:
            raise ContractError(f"{where}: capability mapping required")
        if not isinstance(b["reported_scope"], dict):
            raise ContractError(f"{where}: reported scope must be structured")
        primary=b.get("primary_metric")
        if b["id"] in {"servicenow_enterpriseops_gym", "itbench", "pagerduty_sre_agent_benchmark_2026"} and not isinstance(primary,dict):
            raise ContractError(f"{where}: primary metric definition required")
        if b["id"]=="itbench" and primary.get("name") != "average precision at full recall":
            raise ContractError("ITBench must remain labeled as diagnosis/localization AP at full recall")
        if b["id"]=="servicenow_enterpriseops_gym" and primary.get("name") != "strict whole-task success":
            raise ContractError("EnterpriseOps must remain strict whole-task success")
        if b["id"]=="luminous_opsbench_v0_1" and b["reported_scope"].get("blind_holdout") is not False:
            raise ContractError("public OpsBench v0.1 authoring corpus must not be described as blind")
    contract=c["comparison_contract"]
    require(contract, ("common_core_required","never_translate","analysis_unit","report_unmatched_trials","report_evaluator_errors","report_failed_crashed_timed_out_indeterminate_started_trials"), "crosswalk.comparison_contract")
    if contract["analysis_unit"] != "scenario_id cluster":
        raise ContractError("repeated seeds must remain nested within scenario clusters")
    if contract["report_unmatched_trials"] is not True or contract["report_evaluator_errors"] is not True or contract["report_failed_crashed_timed_out_indeterminate_started_trials"] is not True:
        raise ContractError("crosswalk: started-trial and unmatched-trial reporting cannot be disabled")
    forbidden={"ITBench AP@full-recall to task success","recommendation acceptance to executed remediation","host transaction promotion to customer service restoration","workflow trace completeness to external effect truth"}
    if not forbidden.issubset(set(contract["never_translate"])):
        raise ContractError("crosswalk: missing non-equivalence protections")

def validate_prereg(p):
    require(p, ("schema","version","status","study_id","registered_at","source_pins","scenario_split","arms","fairness_protocol","repetition_policy","denominators","metrics","safety_gates","analysis","claim_policy"), "preregistration")
    if p["schema"]!="luminous-opsbench-common-core-preregistration-v0.1" or type(p["version"]) is not int or p["version"]!=1:
        raise ContractError("preregistration: unsupported schema/version")
    if p["status"] not in {"draft_not_registered","registered_before_trials","completed"}:
        raise ContractError("preregistration: invalid status")
    try:
        t=datetime.fromisoformat(p["registered_at"].replace("Z","+00:00"))
    except (AttributeError,ValueError) as exc:
        raise ContractError("preregistration: invalid registration timestamp") from exc
    if t.tzinfo is None:
        raise ContractError("preregistration: timezone is mandatory")
    src=p["source_pins"]
    require(src, ("opsbench_source_commit","crosswalk_digest","task_manifest_digest","private_variant_commitment","oracle_version","runner_version","policy_digest","environment_digest"), "preregistration.source_pins")
    import re
    for key in ("opsbench_source_commit",):
        if not isinstance(src[key],str) or re.fullmatch(r"[a-f0-9]{40}",src[key]) is None:
            raise ContractError(f"preregistration.source_pins.{key}: invalid SHA")
    for key in ("crosswalk_digest","task_manifest_digest","private_variant_commitment","policy_digest","environment_digest"):
        if not isinstance(src[key],str) or re.fullmatch(r"[a-f0-9]{64}",src[key]) is None:
            raise ContractError(f"preregistration.source_pins.{key}: invalid digest")
    split=p["scenario_split"]
    require(split, ("development_scenario_ids","screening_scenario_ids","confirmatory_scenario_count_target","split_unit","overlap_prohibited"), "preregistration.scenario_split")
    dev=split["development_scenario_ids"]; screen=split["screening_scenario_ids"]
    if not isinstance(dev,list) or not isinstance(screen,list) or not screen or len(set(dev))!=len(dev) or len(set(screen))!=len(screen):
        raise ContractError("scenario split: unique dev and screening IDs required")
    if set(dev)&set(screen):
        raise ContractError("scenario split: development and screening IDs overlap")
    if split["split_unit"]!="fault_mechanism_and_topology_not_random_rows" or split["overlap_prohibited"] is not True:
        raise ContractError("scenario split: fault/topology-level holdout required")
    if type(split["confirmatory_scenario_count_target"]) is not int or split["confirmatory_scenario_count_target"]<100:
        raise ContractError("confirmatory scenario count target must be at least 100")
    arms=p["arms"]
    if not isinstance(arms,list) or len(arms)<4:
        raise ContractError("at least four evaluation arms required")
    ids=[a.get("arm_id") for a in arms if isinstance(a,dict)]
    if len(ids)!=len(arms) or any(not isinstance(x,str) or not x for x in ids) or len(set(ids))!=len(ids):
        raise ContractError("arm IDs must be unique and non-empty")
    types={a.get("system_type") for a in arms}
    required={"expert_human","deterministic_automation","general_agent","luminous_full"}
    if not required.issubset(types):
        raise ContractError(f"missing comparison arms: {sorted(required-types)}")
    for key in ("tool_surface_digest","permission_profile_digest","budget_profile_digest"):
        vals={a.get(key) for a in arms}
        if len(vals)!=1 or None in vals:
            raise ContractError(f"arms do not share the declared {key}")
    fairness=p["fairness_protocol"]
    require(fairness, FAIRNESS_KEYS, "preregistration.fairness_protocol")
    if any(fairness[k] is not True for k in FAIRNESS_KEYS):
        raise ContractError("fairness controls must all be enabled")
    reps=p["repetition_policy"]
    require(reps, ("screening_seeds_per_scenario","confirmatory_seeds_per_scenario","analysis_cluster","paired_key"), "preregistration.repetition_policy")
    for key in ("screening_seeds_per_scenario","confirmatory_seeds_per_scenario"):
        if type(reps[key]) is not int or reps[key]<5:
            raise ContractError(f"{key} must be at least five")
    if reps["analysis_cluster"]!="scenario_id" or reps["paired_key"]!=["scenario_id","variant_id","seed"]:
        raise ContractError("scenario-clustered paired inference is required")
    den=p["denominators"]; require(den,DENOMINATOR_KEYS,"preregistration.denominators")
    if any(den[k] is not True for k in DENOMINATOR_KEYS):
        raise ContractError("denominator policy cannot exclude failures or missing evidence")
    if not isinstance(p["metrics"],list) or not CORE_METRICS.issubset(set(p["metrics"])):
        raise ContractError("required orthogonal outcome/safety metrics missing")
    gates=p["safety_gates"]; require(gates,SAFETY_KEYS,"preregistration.safety_gates")
    if any(type(gates[k]) is not int or gates[k]!=0 for k in SAFETY_KEYS):
        raise ContractError("canonical critical safety gates must be zero")
    analysis=p["analysis"]
    require(analysis,("cluster_bootstrap_samples","cluster_bootstrap_seed","report_scenario_weighted_and_trial_weighted","report_unmatched_trials","report_per_family_results","multiple_comparisons_policy"),"preregistration.analysis")
    if type(analysis["cluster_bootstrap_samples"]) is not int or analysis["cluster_bootstrap_samples"]<5000:
        raise ContractError("at least 5,000 bootstrap samples required")
    if any(analysis[k] is not True for k in ("report_scenario_weighted_and_trial_weighted","report_unmatched_trials","report_per_family_results")):
        raise ContractError("analysis reporting requirements cannot be disabled")
    if not isinstance(analysis["multiple_comparisons_policy"],str) or len(analysis["multiple_comparisons_policy"])<10:
        raise ContractError("multiple-comparisons policy must be preregistered")
    claims=p["claim_policy"]; require(claims,("direct_external_score_translation_allowed","sota_claim_requires_same_harness_confirmatory_trials","publish_all_started_trials","publish_exact_artifact_digests"),"preregistration.claim_policy")
    if claims["direct_external_score_translation_allowed"] is not False or any(claims[k] is not True for k in ("sota_claim_requires_same_harness_confirmatory_trials","publish_all_started_trials","publish_exact_artifact_digests")):
        raise ContractError("claim policy weakens direct-score or complete-reporting protections")

def main():
    try:
        c=load(BASE/"external-benchmark-crosswalk.json")
        p=load(BASE/"preregistration.example.json")
        validate_crosswalk(c); validate_prereg(p)
        print(f"PASS external benchmark crosswalk: {len(c['benchmarks'])} scopes; direct score translation prohibited")
        print(f"PASS preregistration contract: {len(p['arms'])} arms; {p['scenario_split']['confirmatory_scenario_count_target']} confirmatory scenarios target; scenario-cluster analysis")
        print("SCOPE: methodology contract only; no benchmark trials or comparative result.")
        return 0
    except ContractError as exc:
        print(f"FAIL common-core contract: {exc}",file=sys.stderr)
        return 1

if __name__=="__main__":
    raise SystemExit(main())
