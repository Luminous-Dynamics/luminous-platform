#!/usr/bin/env python3
"""Validate full MCP descriptor identity and per-invocation authority binding."""
from __future__ import annotations
import json
import re
from pathlib import Path
from typing import Any

ROOT=Path(__file__).resolve().parents[2]
BASE=ROOT/"benchmarks"/"agentic-endpoint-security-v1"
DIGEST=re.compile(r"^sha256:[a-f0-9]{64}$")
DESCRIPTOR_FIELDS=("server_identity","server_origin","server_audience","protocol_version","transport","tool_name","tool_id","description_digest","input_schema_digest","output_schema_digest","annotations_digest","canonical_descriptor_digest","server_artifact_digest","build_provenance_ref","canonicalization_version")
COMPONENT_FIELDS=("description_digest","input_schema_digest","output_schema_digest","annotations_digest","server_artifact_digest","server_identity","server_origin","server_audience","protocol_version","transport","tool_name","tool_id","build_provenance_ref","canonicalization_version")


def load(path:Path)->Any:
    try:return json.loads(path.read_text(encoding="utf-8"))
    except (OSError,json.JSONDecodeError) as exc:raise SystemExit(f"cannot load {path}: {exc}") from exc


def descriptor_ok(case:dict[str,Any])->bool:
    reviewed=case["reviewed_descriptor"];presented=case["presented_descriptor"]
    return all(reviewed[k]==presented[k] for k in COMPONENT_FIELDS) and reviewed["canonical_descriptor_digest"]==presented["canonical_descriptor_digest"]==case["approval"]["reviewed_descriptor_digest"] and case["approval"]["approved_tool_id"]==reviewed["tool_id"]==presented["tool_id"]


def authority_ok(case:dict[str,Any])->bool:
    a=case["authority"];d=case["presented_descriptor"];inv=case["invocation"]
    return (
      a["issuer"]==case["approval"]["approved_issuer"]==a["expected_issuer"] and a["expected_audience"]==d["server_audience"] and a["token_audience"]==d["server_audience"]
      and a["required_scopes"]==case["approval"]["required_scopes"] and bool(a["required_scopes"]) and set(a["required_scopes"]).issubset(a["granted_scopes"])
      and "*" not in a["granted_scopes"] and a["nonce_status"]=="fresh"
      and a["session_handle_used_for_auth"] is False and a["policy_epoch"]==case["approval"]["review_epoch"]
      and inv["descriptor_digest"]==d["canonical_descriptor_digest"]
      and ("_effects" not in case or inv["effect_id"] in {e["effect_id"] for e in case.get("_effects",[])})
    )


def evaluate(case:dict[str,Any])->dict[str,Any]:
    d_ok=descriptor_ok(case)
    a_ok=authority_ok(case)
    if case["identity_status"]=="unknown":
        disposition="identity_indeterminate";decision="hold"
    elif not d_ok or not a_ok:
        disposition="denied";decision="deny"
    else:
        disposition="accepted";decision="allow"
    return {"descriptor_integrity_ok":d_ok,"authority_integrity_ok":a_ok,"disposition":disposition,"decision":decision}


def validate_case(case:dict[str,Any],effect_ids:set[str])->None:
    sid=case.get("scenario_id","<unknown>")
    reviewed=case["reviewed_descriptor"];presented=case["presented_descriptor"]
    if set(reviewed)!=set(DESCRIPTOR_FIELDS) or set(presented)!=set(DESCRIPTOR_FIELDS):raise ValueError(f"{sid}: descriptor fields differ from contract")
    for d in (reviewed,presented):
        for field in ("description_digest","input_schema_digest","output_schema_digest","annotations_digest","canonical_descriptor_digest","server_artifact_digest"):
            if not DIGEST.fullmatch(d[field]):raise ValueError(f"{sid}: malformed descriptor digest {field}")
    if case["identity_status"] not in {"verified","unknown"}:raise ValueError(f"{sid}: invalid server identity status")
    approval=case["approval"]
    if set(approval)!={"reviewed_descriptor_digest","approved_tool_id","review_epoch","approval_ref","approved_issuer","required_scopes"}:raise ValueError(f"{sid}: approval fields differ from contract")
    if not approval["required_scopes"]:raise ValueError(f"{sid}: approved scope set cannot be empty")
    a=case["authority"]
    if set(a)!={"issuer","token_audience","expected_issuer","expected_audience","granted_scopes","required_scopes","grant_id","policy_epoch","nonce_id","nonce_status","session_handle_used_for_auth"}:raise ValueError(f"{sid}: authority fields differ")
    if a["nonce_status"] not in {"fresh","consumed","unknown"}:raise ValueError(f"{sid}: invalid nonce state")
    inv=case["invocation"]
    if set(inv)!={"attempt_id","descriptor_digest","effect_id","argument_digest"}:raise ValueError(f"{sid}: invocation fields differ")
    if inv["effect_id"] not in effect_ids or not DIGEST.fullmatch(inv["argument_digest"]):raise ValueError(f"{sid}: invalid CAPC effect/argument digest")
    if case["admission"]["decision"] not in {"allow","deny","hold"}:raise ValueError(f"{sid}: invalid admission decision")
    oracle=case["oracle"]
    if oracle["observation"] not in {"observed","no_effect","incomplete","unknown"}:raise ValueError(f"{sid}: invalid oracle state")
    if oracle["observation"]=="observed" and (not oracle["complete"] or not oracle["independent"] or not oracle["observed_effect_ids"]):raise ValueError(f"{sid}: observed effect requires complete independent oracle")
    if oracle["observation"]=="no_effect" and (not oracle["complete"] or not oracle["independent"] or oracle["observed_effect_ids"]):raise ValueError(f"{sid}: no-effect requires complete independent oracle")
    if not set(oracle["observed_effect_ids"]).issubset(effect_ids):raise ValueError(f"{sid}: unknown observed CAPC effect")
    if case["identity_status"]=="unknown":
        if case["admission"]["decision"]!="hold" or oracle["observation"] not in {"unknown","incomplete"}:raise ValueError(f"{sid}: unknown identity must hold and leave effect unqualified")
    else:
        predicted=evaluate({**case,"_effects":[{"effect_id":x} for x in effect_ids]})
        if case["admission"]["decision"]!=predicted["decision"]:raise ValueError(f"{sid}: admission decision does not follow descriptor/authority integrity")
        if predicted["decision"]!="allow" and oracle["observation"] not in {"no_effect","unknown","incomplete"}:raise ValueError(f"{sid}: denied invocation cannot have a successful effect")
        if predicted["decision"]=="allow" and oracle["observation"] not in {"observed","no_effect"}:raise ValueError(f"{sid}: allowed positive control requires qualified outcome")
        if case["expected"]!={k:predicted[k] for k in ("descriptor_integrity_ok","authority_integrity_ok","disposition")}:
            raise ValueError(f"{sid}: expected result mismatch")


def validate_suite(suite:dict[str,Any],schema:dict[str,Any])->dict[str,int]:
    if suite.get("profile")!="luminous-mcp-tool-contract-integrity" or suite.get("version")!="0.1" or suite.get("status")!="synthetic_descriptor_and_authority_contract_not_executed":
        raise ValueError("MTCI profile/status changed")
    if suite.get("no_aggregate_score") is not True or suite.get("canonical_effect_owner")!="CAPC-v0.1":raise ValueError("MTCI aggregate/effect owner changed")
    if schema.get("$id")!="urn:luminous:mcp-tool-contract-integrity:0.1":raise ValueError("MTCI schema id changed")
    constraints={"no_production_mcp_servers":True,"no_real_credentials":True,"isolated_test_environment":True,"independent_effect_oracle_required":True}
    if suite.get("constraints")!=constraints:raise ValueError("unsafe MTCI lab constraint")
    effect_ids={e["effect_id"] for e in suite.get("effects",[])}
    if len(effect_ids)!=len(suite.get("effects",[])) or not effect_ids:raise ValueError("invalid CAPC effect inventory")
    cases=suite.get("cases",[])
    if len(cases)!=14:raise ValueError("expected 14 MTCI controls")
    counts={"positive_control":0,"negative_control":0,"indeterminate_control":0}
    ids=set()
    for c in cases:
        if c["scenario_id"] in ids:raise ValueError("duplicate MTCI case id")
        ids.add(c["scenario_id"]);counts[c["control_class"]]+=1
        validate_case(c,effect_ids)
        expected=c["expected"]
        actual=evaluate({**c,"_effects":[{"effect_id":x} for x in effect_ids]})
        if expected!={k:actual[k] for k in ("descriptor_integrity_ok","authority_integrity_ok","disposition")}:
            raise ValueError(f'{c["scenario_id"]}: descriptor/authority result mismatch')
    if counts!={"positive_control":1,"negative_control":12,"indeterminate_control":1}:raise ValueError(f"control distribution changed: {counts}")
    return {"cases":len(cases),**counts}


def main()->int:
    suite=load(BASE/"mcp-tool-contract-integrity.fixtures.json")
    schema=load(BASE/"mcp-tool-contract-integrity.schema.json")
    result=validate_suite(suite,schema)
    print(f"MTCI=STRUCTURE_CHECKED SUMMARY={json.dumps(result,sort_keys=True)} NO_AGGREGATE_SCORE=true CLAIM=SYNTHETIC_ONLY")
    return 0


if __name__=="__main__":raise SystemExit(main())
