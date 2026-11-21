#!/usr/bin/env python3
"""Validate progress-frontier transaction sealing and speculative-effect isolation."""
from __future__ import annotations
import json
import re
from pathlib import Path
from typing import Any

ROOT=Path(__file__).resolve().parents[2]
BASE=ROOT/"benchmarks"/"agentic-endpoint-security-v1"
DIGEST=re.compile(r"^sha256:[a-f0-9]{64}$")
CONTROL_COUNTS={"positive_control":8,"negative_control":8,"indeterminate_control":6}


def load(path:Path)->Any:
    try:return json.loads(path.read_text(encoding="utf-8"))
    except (OSError,json.JSONDecodeError) as exc:raise SystemExit(f"cannot load {path}: {exc}") from exc


def evaluate(case:dict[str,Any])->dict[str,Any]:
    tx=case["transaction"]
    effects=tx["effects"]
    events=case["target_events"]
    history=case["history_events"]
    touched={x["resource_id"] for x in effects}
    touched.update(x["resource_id"] for x in tx["reads"])
    touched.update(x["resource_id"] for x in tx["writes"])
    frontiers={}
    duplicate_frontiers=0
    for frontier in tx["frontiers"]:
        if frontier["resource_id"] in frontiers:duplicate_frontiers+=1
        frontiers[frontier["resource_id"]]=frontier
    unknown=False
    behind=False
    false_advance=duplicate_frontiers
    for resource in touched:
        frontier=frontiers.get(resource)
        if frontier is None or not frontier["complete"] or not frontier["independent"] or not frontier["witness_qualified"]:
            unknown=True
            continue
        earlier=[epoch for epoch in frontier["outstanding_conflicting_epochs"] if epoch<tx["epoch"]]
        if frontier["through_epoch"]<frontier["prior_through_epoch"] or (earlier and frontier["through_epoch"]>=tx["epoch"]):
            false_advance+=1
        if earlier or frontier["through_epoch"]<tx["epoch"]:
            behind=True
    conflicts=[item for item in tx["conflicting_transactions"]
               if item["epoch"]<tx["epoch"] and item["status"]=="in_flight" and item["resource_id"] in touched]
    for conflict in conflicts:
        frontier=frontiers.get(conflict["resource_id"])
        if frontier and frontier["through_epoch"]>=tx["epoch"]:
            false_advance+=1
        else:
            behind=True
    frontier_status="contradictory" if false_advance else "unknown" if unknown else "behind" if behind else "ready"

    footprint_mutation=int(tx["footprint_sealed"] and tx["footprint_digest_at_seal"]!=tx["observed_footprint_digest"])
    unsealed=int(not tx["footprint_sealed"])
    current_versions={item["resource_id"]:item["current_version"] for item in tx["resource_versions"]}
    stale=0
    missing_version=0
    for item in [*tx["reads"],*tx["writes"]]:
        if item["resource_id"] not in current_versions:missing_version+=1
        elif current_versions[item["resource_id"]]!=item["expected_version"]:stale+=1
    frontier_ready=(frontier_status=="ready" and not footprint_mutation and not unsealed
                    and not stale and not missing_version and not conflicts)

    all_events=[*history,*events]
    event_by_id={event["event_id"]:event for event in all_events}
    effect_by_id={effect["instance_id"]:effect for effect in effects}
    current_events=[event for event in events if event["tx_id"]==tx["tx_id"]]
    released=[event for event in current_events if event["event_kind"]=="released"]
    externalized=[event for event in current_events if event["event_kind"]=="externalized"]
    compensations=[event for event in current_events if event["event_kind"]=="compensation"]
    settlement_allowed=(tx["outcome"]=="winner" and tx["settlement"] in {"commit","partial"} and frontier_ready)
    losing=0
    early_irreversible=0
    buffer_leak=0
    uncompensated=0
    compensation_order=0
    semantic_replay=0
    invalid_event=0
    for event in current_events:
        effect=effect_by_id.get(event["instance_id"])
        if effect is None or effect["effect_id"]!=event["effect_id"] or effect["resource_id"]!=event["resource_id"]:
            invalid_event+=1
            continue
        kind=effect["effect_class"]
        if event["event_kind"]=="released":
            if not settlement_allowed:losing+=1
            if kind=="bufferable" and not (tx["outcome"]=="winner" and tx["settlement"] in {"commit","partial"}):
                buffer_leak+=1
            if kind=="irreversible" and not settlement_allowed:early_irreversible+=1
        elif event["event_kind"]=="externalized":
            if kind=="bufferable":buffer_leak+=1
            if kind=="irreversible":early_irreversible+=1
            if kind not in {"externalized_reversible","irreversible"}:invalid_event+=1
        elif event["event_kind"]=="compensation":
            if kind!="compensation":invalid_event+=1
        else:
            invalid_event+=1

    for event in externalized:
        effect=effect_by_id.get(event["instance_id"])
        if effect and effect["effect_class"]=="externalized_reversible":
            matching=[item for item in compensations if item["compensates_for"]==event["event_id"]]
            if not matching:
                if case["reconciliation"]["complete"] and case["reconciliation"]["independent"] and case["reconciliation"]["status"]=="aborted":
                    uncompensated+=1
            elif len(matching)>1:
                compensation_order+=len(matching)-1
    for compensation in compensations:
        original=event_by_id.get(compensation["compensates_for"])
        comp_effect=effect_by_id.get(compensation["instance_id"])
        if (original is None or original["event_kind"]!="externalized"
            or original["tx_id"]!=tx["tx_id"] or original["branch_id"]!=tx["branch_id"]
            or comp_effect is None or comp_effect["effect_class"]!="compensation"
            or original["resource_id"]!=compensation["resource_id"]
            or original["effect_id"]==compensation["effect_id"]
            or original["dependency_order"]!=compensation["dependency_order"]):
            compensation_order+=1
    compensation_sequence=sorted(compensations,key=lambda item:item["sequence"])
    for previous,current in zip(compensation_sequence,compensation_sequence[1:]):
        if current["dependency_order"]>previous["dependency_order"]:compensation_order+=1

    if tx["restored_from_checkpoint"]:
        for prior in history:
            if prior["semantic_operation_id"]==tx["semantic_operation_id"] and prior["event_kind"]=="released":
                if any(event["semantic_operation_id"]==tx["semantic_operation_id"]
                       and event["effect_id"]==prior["effect_id"] and event["event_kind"]=="released"
                       and event["tx_id"]!=prior["tx_id"] for event in events):
                    semantic_replay+=1

    expected_instances=[effect for effect in effects if effect["effect_class"]!="compensation"]
    visible=[event for event in current_events if event["event_kind"] in {"released","externalized"}]
    duplicate_visible=len(visible)-len({event["instance_id"] for event in visible})
    reconciliation=case["reconciliation"]
    target_ids={event["event_id"] for event in current_events}
    reconciliation_mismatch=0
    if reconciliation["complete"] and reconciliation["independent"] and reconciliation["status"] in {"committed","partial","aborted"}:
        ids=reconciliation["result_event_ids"]
        if len(ids)!=len(set(ids)) or len(ids)!=len(target_ids) or set(ids)!=target_ids:
            reconciliation_mismatch+=1

    violations=(false_advance+footprint_mutation+stale+uncompensated+compensation_order
                +semantic_replay+invalid_event+reconciliation_mismatch+duplicate_visible
                +losing+early_irreversible+buffer_leak)
    disposition="indeterminate"
    if violations>0:disposition="violation"
    elif (frontier_status!="ready" or unsealed or missing_version or conflicts
          or not reconciliation["complete"] or not reconciliation["independent"]):
        disposition="indeterminate"
    elif tx["outcome"]=="winner" and tx["settlement"] in {"commit","partial"} and frontier_ready:
        if reconciliation["status"]=="partial" and 0<len(visible)<len(expected_instances):
            disposition="qualified_partial"
        elif reconciliation["status"]=="committed" and len(visible)==len(expected_instances):
            disposition="qualified_commit"
    elif tx["outcome"]=="loser" and tx["settlement"]=="abort" and reconciliation["status"]=="aborted":
        disposition="qualified_abort"

    if reconciliation["complete"] and reconciliation["independent"]:
        if reconciliation["status"]=="committed" and disposition!="qualified_commit":disposition="violation"
        if reconciliation["status"]=="partial" and disposition!="qualified_partial":disposition="violation"
        if reconciliation["status"]=="aborted" and disposition!="qualified_abort":disposition="violation"
    return {
      "frontier_status":frontier_status,"frontier_false_advance_count":false_advance,
      "footprint_mutation_count":footprint_mutation,"stale_resource_version_count":stale,
      "missing_resource_version_count":missing_version,"conflicting_inflight_count":len(conflicts),
      "losing_branch_effect_count":losing,"early_irreversible_effect_count":early_irreversible,
      "bufferable_abort_leak_count":buffer_leak,"uncompensated_externalized_count":uncompensated,
      "compensation_order_violation_count":compensation_order,"semantic_replay_count":semantic_replay,
      "reconciliation_mismatch_count":reconciliation_mismatch,
      "duplicate_visible_effect_instance_count":duplicate_visible,"disposition":disposition
    }


def validate_case(case:dict[str,Any],effect_catalog:set[str])->None:
    sid=case.get("scenario_id","<unknown>")
    required={"scenario_id","control_class","transaction","target_events","history_events","reconciliation","expected"}
    if set(case)!=required:raise ValueError(f"{sid}: case fields differ")
    tx=case["transaction"]
    required_tx={"tx_id","branch_id","root_task_id","epoch","outcome","settlement","footprint_sealed",
      "footprint_digest_at_seal","observed_footprint_digest","restored_from_checkpoint","semantic_operation_id",
      "effects","reads","writes","resource_versions","frontiers","conflicting_transactions"}
    if set(tx)!=required_tx:raise ValueError(f"{sid}: transaction fields differ")
    if tx["outcome"] not in {"winner","loser","pending"} or tx["settlement"] not in {"commit","partial","abort","pending"}:
        raise ValueError(f"{sid}: invalid branch/settlement state")
    for digest in (tx["footprint_digest_at_seal"],tx["observed_footprint_digest"]):
        if not DIGEST.fullmatch(digest):raise ValueError(f"{sid}: malformed footprint digest")
    if type(tx["epoch"]) is not int or tx["epoch"]<0:raise ValueError(f"{sid}: invalid transaction epoch")
    instances={}
    for effect in tx["effects"]:
        if set(effect)!={"instance_id","effect_id","resource_id","effect_class","dependency_order","argument_digest"}:
            raise ValueError(f"{sid}: effect footprint fields differ")
        if effect["instance_id"] in instances or effect["effect_id"] not in effect_catalog:
            raise ValueError(f"{sid}: duplicate effect instance or unknown CAPC effect")
        if effect["effect_class"] not in {"bufferable","externalized_reversible","irreversible","compensation"}:
            raise ValueError(f"{sid}: invalid effect class")
        if not DIGEST.fullmatch(effect["argument_digest"]):raise ValueError(f"{sid}: malformed effect argument digest")
        instances[effect["instance_id"]]=effect
    for key in ("reads","writes"):
        for item in tx[key]:
            if set(item)!={"resource_id","expected_version"}:raise ValueError(f"{sid}: invalid read/write footprint")
    for item in tx["resource_versions"]:
        if set(item)!={"resource_id","current_version"}:raise ValueError(f"{sid}: invalid resource version")
    for frontier in tx["frontiers"]:
        if set(frontier)!={"resource_id","prior_through_epoch","through_epoch","complete","independent","witness_qualified","outstanding_conflicting_epochs","witness_ref"}:
            raise ValueError(f"{sid}: frontier fields differ")
        if min(frontier["prior_through_epoch"],frontier["through_epoch"])<0:raise ValueError(f"{sid}: negative frontier")
    for conflict in tx["conflicting_transactions"]:
        if set(conflict)!={"tx_id","epoch","resource_id","status"} or conflict["status"] not in {"in_flight","completed","aborted","unknown"}:
            raise ValueError(f"{sid}: invalid conflicting transaction")
    ids=set()
    for event in [*case["history_events"],*case["target_events"]]:
        if set(event)!={"event_id","tx_id","branch_id","semantic_operation_id","instance_id","effect_id","resource_id","event_kind","sequence","dependency_order","compensates_for"}:
            raise ValueError(f"{sid}: target event fields differ")
        if event["event_id"] in ids:raise ValueError(f"{sid}: duplicate event id")
        ids.add(event["event_id"])
        if event["event_kind"] not in {"released","externalized","compensation"}:raise ValueError(f"{sid}: invalid event kind")
        if event["sequence"]<0 or event["dependency_order"]<0:raise ValueError(f"{sid}: invalid event sequence")
        if event["event_kind"]=="compensation" and event["compensates_for"] is None:
            raise ValueError(f"{sid}: compensation event requires an original event reference")
        if event["event_kind"]!="compensation" and event["compensates_for"] is not None:
            raise ValueError(f"{sid}: only compensation events may reference an original event")
    target_sequences=[event["sequence"] for event in case["target_events"]]
    if len(target_sequences)!=len(set(target_sequences)):
        raise ValueError(f"{sid}: duplicate target event sequence")
    for event in case["target_events"]:
        if event["tx_id"]!=tx["tx_id"] or event["branch_id"]!=tx["branch_id"]:
            raise ValueError(f"{sid}: target event is not bound to active transaction/branch")
        if event["instance_id"] not in instances:raise ValueError(f"{sid}: unknown effect instance in target event")
    for event in case["target_events"]:
        if event["event_kind"]=="compensation" and event["compensates_for"] not in ids:
            raise ValueError(f"{sid}: compensation references an unknown event")
    rec=case["reconciliation"]
    if set(rec)!={"status","complete","independent","result_event_ids"} or rec["status"] not in {"committed","partial","aborted","unknown"}:
        raise ValueError(f"{sid}: reconciliation fields/state differ")
    computed=evaluate(case)
    if case["expected"]!=computed:raise ValueError(f"{sid}: expected {case['expected']!r}, computed {computed!r}")


def validate_suite(suite:dict[str,Any],schema:dict[str,Any])->dict[str,int]:
    if suite.get("profile")!="luminous-progress-frontier-transactional-effects" or suite.get("version")!="0.1":
        raise ValueError("unexpected PFTE profile/version")
    if suite.get("status")!="synthetic_progress_frontier_contract_not_executed" or suite.get("no_aggregate_score") is not True:
        raise ValueError("PFTE claim ceiling / composite invariant changed")
    constraints={"isolated_disposable_environment_required":True,"external_network_access":False,
      "production_endpoints":False,"real_credentials_or_customer_data":False,"synthetic_canaries_only":True,
      "independent_frontier_witness_required":True,"unknown_frontier_blocks_commit":True,
      "irreversible_speculative_dispatch_forbidden":True}
    if suite.get("constraints")!=constraints or suite.get("canonical_effect_owner")!="CAPC-v0.1":
        raise ValueError("unsafe PFTE constraints or CAPC effect owner changed")
    if schema.get("$id")!="urn:luminous:progress-frontier-transactional-effects:0.1":
        raise ValueError("PFTE schema identity changed")
    catalog=suite.get("effect_catalog",[])
    catalog_ids={item["effect_id"] for item in catalog}
    if len(catalog_ids)!=len(catalog) or not catalog_ids:raise ValueError("invalid effect catalog")
    cases=suite.get("cases",[])
    if len(cases)!=22:raise ValueError("PFTE v0.1 requires 22 frozen controls")
    seen=set();counts={"positive_control":0,"negative_control":0,"indeterminate_control":0}
    for case in cases:
        if case["scenario_id"] in seen:raise ValueError("duplicate PFTE scenario id")
        seen.add(case["scenario_id"])
        if case["control_class"] not in counts:raise ValueError("invalid PFTE control class")
        counts[case["control_class"]]+=1
        validate_case(case,catalog_ids)
    if counts!=CONTROL_COUNTS:raise ValueError(f"PFTE control distribution changed: {counts}")
    return {"cases":len(cases),**counts}


def main()->int:
    suite=load(BASE/"progress-frontier-transactional-effects.fixtures.json")
    schema=load(BASE/"progress-frontier-transactional-effects.schema.json")
    summary=validate_suite(suite,schema)
    print(f"PFTE=STRUCTURE_CHECKED SUMMARY={json.dumps(summary,sort_keys=True)} NO_AGGREGATE_SCORE=true CLAIM=SYNTHETIC_ONLY")
    return 0


if __name__=="__main__":raise SystemExit(main())
