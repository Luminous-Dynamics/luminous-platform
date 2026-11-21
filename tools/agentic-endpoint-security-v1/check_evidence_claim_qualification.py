#!/usr/bin/env python3
"""Validate claim-scoped obligation/evidence closure without a composite score."""
from __future__ import annotations
import json
import re
from pathlib import Path
from typing import Any

ROOT=Path(__file__).resolve().parents[2]
BASE=ROOT/"benchmarks"/"agentic-endpoint-security-v1"
SHA40=re.compile(r"^[a-f0-9]{40}$")
DIGEST=re.compile(r"^sha256:[a-f0-9]{64}$")
PROFILES={
 "synthetic_contract":["checker","mutations"],
 "exact_head_hosted":["checker","mutations","workflow"],
 "isolated_runtime":["checker","mutations","workflow","attestation","runtime","currentness"],
 "independent_effect_observation":["checker","mutations","workflow","attestation","runtime","currentness","observer_fault","target_effect","timebase"],
 "bounded_edit_contract":["checker","mutations","continuation"],
}
CATEGORIES=("checker","mutations","workflow","attestation","runtime","currentness","observer_fault","target_effect","timebase","continuation")


def load(path:Path)->Any:
    try:return json.loads(path.read_text(encoding="utf-8"))
    except (OSError,json.JSONDecodeError) as exc:raise SystemExit(f"cannot load {path}: {exc}") from exc


def assess_evidence(e:dict[str,Any],claim:dict[str,Any])->str:
    if e["status"]=="failed":return "failed"
    if e["status"]!="pass":return "unknown"
    if e["subject_commit"]!=claim["subject_commit"] or e["subject_digest"]!=claim["subject_digest"]:
        return "failed"
    kind=e["evidence_type"]
    if kind=="checker":return "valid" if e["source_kind"]=="repository_checker" else "failed"
    if kind=="mutations":return "valid" if e["source_kind"]=="repository_mutation_suite" else "failed"
    if kind=="workflow":
        if e["run_status"] in {"queued","in_progress"}:return "unknown"
        good=(e["source_kind"]=="github_actions_api" and bool(e["run_id"])
              and e["run_status"]=="completed" and e["run_conclusion"]=="success"
              and e["run_head_sha"]==claim["subject_commit"]
              and set(claim["required_checks"]).issubset(e["passed_checks"]))
        return "valid" if good else "failed"
    if kind=="attestation":
        good=(e["source_kind"]=="in_toto_attestation_verifier" and e["signature_verified"] is True
              and e["attester_id"] in claim["trusted_attesters"]
              and e["attestation_subject_digest"]==claim["subject_digest"])
        return "valid" if good else "failed"
    if kind=="runtime":
        if e["disposable"] is False:return "failed"
        if not all((e["runtime_id"],e["model_id"],e["environment_id"],e["tool_descriptor_digest"])):
            return "unknown"
        good=(e["source_kind"]=="isolated_runtime_adapter" and e["disposable"] is True
              and e["tool_descriptor_digest"]==claim["current_descriptor_digest"])
        return "valid" if good else "failed"
    if kind=="currentness":
        if e["currentness_state"] in {"stale","superseded"} or e["superseded_by"] is not None:return "failed"
        good=(e["currentness_state"]=="current" and e["policy_revision"]==claim["current_policy_revision"]
              and e["descriptor_digest"]==claim["current_descriptor_digest"])
        return "valid" if good else "failed"
    if kind=="observer_fault":
        good=(e["source_kind"]=="observer_fault_campaign" and e["fault_campaign_executed"] is True
              and e["observer_fault_qualified"] is True and e["observer_independent"] is True
              and e["observer_fault_domain_id"]==e["observer_domain_id"]
              and e["observer_fault_domain_id"]!=claim["subject_domain_id"])
        return "valid" if good else "failed"
    if kind=="target_effect":
        if e["source_kind"]=="agent_self_report" or e["observer_independent"] is not True or e["observer_domain_id"]==claim["subject_domain_id"]:
            return "failed"
        if e["target_observation_complete"] is not True or not e["observer_identity"]:return "unknown"
        if e["target_observation_state"]!=claim["effect_expectation"] or e["target_observation_state"] not in {"observed","no_effect"}:
            return "unknown"
        if claim["effect_expectation"]=="observed" and (not e["effect_id"] or e["effect_id"]!=claim["expected_effect_id"]):return "failed"
        if claim["effect_expectation"]=="no_effect" and e["effect_id"] is not None:return "failed"
        covered=(set(claim["scope"]["resource_ids"]).issubset(e["resource_ids"])
                 and set(claim["scope"]["operations"]).issubset(e["operations"]))
        return "valid" if covered and e["source_kind"]=="resource_owner_observer" else "failed"
    if kind=="timebase":
        return "valid" if e["source_kind"]=="timebase_verifier" and e["timebase_state"]=="qualified" and bool(e["timebase_id"]) else "unknown"
    if kind=="continuation":
        if e["proof_complete"] is not True or e["replay_free"] is not True:return "failed"
        return "valid" if e["witness_complete"] is True else "unknown"
    return "failed"


def evaluate(case:dict[str,Any])->dict[str,Any]:
    claim=case["claim"];evidence={item["evidence_id"]:item for item in case["evidence"]}
    by_category={item["category"]:item for item in case["obligations"]}
    statuses={category:"missing" for category in CATEGORIES}
    invalid=[]
    for category,obligation in by_category.items():
        results=[
            assess_evidence(evidence[eid],claim)
            if eid in evidence and evidence[eid]["evidence_type"]==category
            else "failed" if eid in evidence
            else "missing"
            for eid in obligation["evidence_ids"]
        ]
        if "valid" in results:statuses[category]="discharged"
        elif "failed" in results:statuses[category]="failed"
        else:statuses[category]="missing_or_unknown"
        for eid in obligation["evidence_ids"]:
            if eid in evidence and (evidence[eid]["evidence_type"]!=category or assess_evidence(evidence[eid],claim)=="failed"):invalid.append(eid)
    required=PROFILES[claim["requested_profile"]]
    residual=[]
    for category in required:
        if statuses[category]!="discharged":
            obligation=by_category.get(category)
            residual.append(obligation["obligation_id"] if obligation else f"MISSING-{claim['claim_id']}-{category.upper()}")
    if not claim["bundle_sealed"] or not claim["bundle_digest"]:
        residual.append(f"CLAIM-BUNDLE-{claim['claim_id']}")
    supported=[] if not claim["bundle_sealed"] or not claim["bundle_digest"] else sorted(
        name for name,categories in PROFILES.items() if all(statuses[c]=="discharged" for c in categories)
    )
    if not residual:state="qualified"
    elif invalid or not claim["bundle_sealed"] or not claim["bundle_digest"]:state="failed"
    else:state="indeterminate"
    return {
      "obligation_status":statuses,
      "discharged_obligation_ids":sorted(o["obligation_id"] for o in case["obligations"] if statuses[o["category"]]=="discharged"),
      "residual_obligation_ids":sorted(residual),
      "invalid_evidence_ids":sorted(set(invalid)),
      "supported_profiles":supported,
      "claim_state":state,
    }


def validate_case(case:dict[str,Any])->None:
    sid=case.get("scenario_id","<unknown>")
    if set(case)!={"scenario_id","control_class","claim","obligations","evidence","expected"}:
        raise ValueError(f"{sid}: case fields differ from ECQ contract")
    claim=case["claim"]
    claim_fields={"claim_id","requested_profile","assertion","subject_repository","subject_commit","subject_digest","subject_domain_id","current_policy_revision","current_descriptor_digest","trusted_attesters","required_checks","scope","effect_expectation","expected_effect_id","bundle_sealed","bundle_digest"}
    if set(claim)!=claim_fields:
        raise ValueError(f"{sid}: claim fields differ from ECQ contract")
    if claim["effect_expectation"] not in {"observed","no_effect",None}:
        raise ValueError(f"{sid}: invalid effect expectation")
    if claim["requested_profile"] not in PROFILES:raise ValueError(f"{sid}: unknown claim profile")
    if not SHA40.fullmatch(claim["subject_commit"]) or not DIGEST.fullmatch(claim["subject_digest"]):
        raise ValueError(f"{sid}: malformed immutable subject identity")
    if not DIGEST.fullmatch(claim["current_descriptor_digest"]):raise ValueError(f"{sid}: malformed current descriptor digest")
    if claim["bundle_digest"] is not None and not DIGEST.fullmatch(claim["bundle_digest"]):
        raise ValueError(f"{sid}: malformed claim bundle digest")
    if not claim["trusted_attesters"] or len(set(claim["trusted_attesters"]))!=len(claim["trusted_attesters"]):
        raise ValueError(f"{sid}: trusted attester policy must be explicit and unique")
    if not claim["required_checks"] or len(set(claim["required_checks"]))!=len(claim["required_checks"]):
        raise ValueError(f"{sid}: required check set must be explicit and unique")
    if claim["effect_expectation"]=="observed" and not claim["expected_effect_id"]:
        raise ValueError(f"{sid}: observed-effect claim requires a canonical effect id")
    if claim["effect_expectation"]=="no_effect" and claim["expected_effect_id"] is not None:
        raise ValueError(f"{sid}: no-effect claim cannot name a committed effect id")
    if claim["effect_expectation"] is None and claim["expected_effect_id"] is not None:
        raise ValueError(f"{sid}: unspecified effect expectation cannot bind an effect id")
    if len(set(claim["scope"]["resource_ids"]))!=len(claim["scope"]["resource_ids"]) or len(set(claim["scope"]["operations"]))!=len(claim["scope"]["operations"]):
        raise ValueError(f"{sid}: duplicate scope item")
    categories=[o["category"] for o in case["obligations"]]
    if len(categories)!=len(set(categories)):raise ValueError(f"{sid}: duplicate obligation category")
    ids=set()
    for obligation in case["obligations"]:
        if set(obligation)!={"obligation_id","category","mandatory","evidence_ids"}:
            raise ValueError(f"{sid}: obligation fields differ from ECQ contract")
        if obligation["category"] not in CATEGORIES or obligation["mandatory"] is not True:raise ValueError(f"{sid}: invalid/optional obligation")
        if not obligation["evidence_ids"] or len(obligation["evidence_ids"])!=len(set(obligation["evidence_ids"])):
            raise ValueError(f"{sid}: obligation evidence mapping must be nonempty and unique")
    evidence_fields={"evidence_id","evidence_type","status","source_kind","subject_commit","subject_digest","run_id","run_status","run_conclusion","run_head_sha","passed_checks","attester_id","signature_verified","attestation_subject_digest","runtime_id","model_id","environment_id","tool_descriptor_digest","disposable","currentness_state","policy_revision","descriptor_digest","superseded_by","observer_identity","observer_domain_id","observer_independent","observer_fault_qualified","fault_campaign_executed","observer_fault_domain_id","target_observation_state","target_observation_complete","effect_id","resource_ids","operations","timebase_state","timebase_id","proof_complete","witness_complete","replay_free","continuation_outcome"}
    for item in case["evidence"]:
        if set(item)!=evidence_fields:raise ValueError(f"{sid}: evidence fields differ from ECQ contract")
        if item["evidence_id"] in ids:raise ValueError(f"{sid}: duplicate evidence id")
        ids.add(item["evidence_id"])
        if item["evidence_type"] not in CATEGORIES or item["status"] not in {"pass","failed","unknown"}:
            raise ValueError(f"{sid}: invalid evidence type/status")
        if item["subject_commit"] is not None and not SHA40.fullmatch(item["subject_commit"]):raise ValueError(f"{sid}: malformed evidence subject commit")
        if item["subject_digest"] is not None and not DIGEST.fullmatch(item["subject_digest"]):raise ValueError(f"{sid}: malformed evidence subject digest")
    actual=evaluate(case)
    if case["expected"]!=actual:raise ValueError(f"{sid}: expected {case['expected']!r}, computed {actual!r}")


def validate_suite(suite:dict[str,Any],schema:dict[str,Any])->dict[str,int]:
    if suite.get("profile")!="luminous-evidence-claim-qualification" or suite.get("version")!="0.1":
        raise ValueError("ECQ profile/version changed")
    if suite.get("status")!="synthetic_claim_interpretation_contract_not_cryptographically_verified" or suite.get("no_aggregate_score") is not True:
        raise ValueError("ECQ claim ceiling/no-composite invariant changed")
    constraints={"synthetic_fixtures_only":True,"no_signature_crypto_verification":True,"no_live_api_or_effect_lookup":True,
      "no_production_assurance_claims":True,"exact_subject_binding_required":True,"claim_gap_is_set_valued":True}
    if suite.get("constraints")!=constraints or suite.get("profile_requirements")!=PROFILES:
        raise ValueError("ECQ safety constraints/profile requirements changed")
    if schema.get("$id")!="urn:luminous:evidence-claim-qualification:0.1":raise ValueError("ECQ schema identity changed")
    cases=suite.get("cases",[])
    if len(cases)!=20:raise ValueError("ECQ v0.1 requires 20 controls")
    counts={"positive_control":0,"negative_control":0,"indeterminate_control":0}
    seen=set()
    for case in cases:
        if case["scenario_id"] in seen:raise ValueError("duplicate ECQ scenario")
        seen.add(case["scenario_id"])
        if case["control_class"] not in counts:raise ValueError("invalid ECQ control class")
        counts[case["control_class"]]+=1
        validate_case(case)
    expected={"positive_control":4,"negative_control":12,"indeterminate_control":4}
    if counts!=expected:raise ValueError(f"ECQ control distribution changed: {counts}")
    return {"cases":len(cases),**counts}


def main()->int:
    suite=load(BASE/"evidence-claim-qualification.fixtures.json")
    schema=load(BASE/"evidence-claim-qualification.schema.json")
    print(f"ECQ=STRUCTURE_CHECKED SUMMARY={json.dumps(validate_suite(suite,schema),sort_keys=True)} NO_AGGREGATE_SCORE=true CLAIM=SYNTHETIC_ONLY")
    return 0


if __name__=="__main__":raise SystemExit(main())
