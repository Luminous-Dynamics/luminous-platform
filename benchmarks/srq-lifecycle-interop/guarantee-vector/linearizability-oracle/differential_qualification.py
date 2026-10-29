#!/usr/bin/env python3
"""Differentially check the SRQ linearizability oracle against brute-force semantics."""
from __future__ import annotations
import argparse, hashlib, itertools, json, os
from pathlib import Path
from typing import Any, Iterable
from differential_reference import reference_accepts_witness, reference_has_linearization
from linearizability_oracle import InvalidHistory, check_history

SCHEMA = "srq-linearizability-differential-report-v1"
DIGESTS = {"request_0":"b"*64,"effect_0":"c"*64,"policy_0":"d"*64,
           "request_1":"e"*64,"effect_1":"f"*64,"policy_1":"a"*64}
INTERVALS = ((0,1),(2,3),(0,3))
RESULT_SHAPES = (("committed","effect-0","receipt-0"),("committed","effect-0","receipt-1"),
                 ("committed","effect-1","receipt-0"),("committed","effect-1","receipt-1"),
                 ("unknown",None,None),("key_conflict",None,None))


def operation_shape(index: int, shape_index: int) -> dict[str, Any]:
    shape_index %= 72
    interval_index = shape_index % 3
    result_index = (shape_index // 3) % 6
    key_index = (shape_index // 18) % 2
    intent_index = (shape_index // 36) % 2
    start_ns, end_ns = INTERVALS[interval_index]
    result, effect_id, receipt_id = RESULT_SHAPES[result_index]
    operation = {
        "op_id":f"op-{index}","start_ns":start_ns,"end_ns":end_ns,
        "idempotency_key":f"key-{key_index}",
        "request_digest":DIGESTS[f"request_{intent_index}"],
        "effect_digest":DIGESTS[f"effect_{intent_index}"],
        "policy_digest":DIGESTS[f"policy_{intent_index}"],"result":result,
    }
    if result == "committed":
        operation["effect_id"],operation["receipt_id"] = effect_id,receipt_id
    return operation


def history_from_shapes(shape_indices: tuple[int,...]) -> dict[str,Any]:
    return {"schema":"srq-linearizability-history-v1","clock_domain":"single_observer_monotonic",
            "sink_identity_digest":"9"*64,
            "operations":[operation_shape(index,shape) for index,shape in enumerate(shape_indices)]}


def deterministic_corpus() -> Iterable[tuple[str,dict[str,Any]]]:
    # Exhaustive ordered pairs over the 72-shape domain: 72 squared = 5,184.
    for left,right in itertools.product(range(72),repeat=2):
        yield "exhaustive_2op",history_from_shapes((left,right))
    # Deterministic, spread-out triples/quads. These tiers are sampled, not exhaustive.
    seen: set[tuple[int,...]] = set()
    for sample in range(512):
        indices=(sample%72,((sample//72)+sample*29+11)%72,(sample*43+7)%72)
        if indices not in seen:
            seen.add(indices)
            yield "sampled_3op",history_from_shapes(indices)
    seen.clear()
    for sample in range(128):
        indices=(sample%72,((sample//72)+sample*23+5)%72,(sample*31+9)%72,(sample*47+15)%72)
        if indices not in seen:
            seen.add(indices)
            yield "sampled_4op",history_from_shapes(indices)


def canonical_hash(value: Any) -> str:
    return hashlib.sha256(json.dumps(value,sort_keys=True,separators=(",",":"),ensure_ascii=False).encode("utf-8")).hexdigest()


def _file_hash(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def compare_history(history: dict[str, Any], *, max_states: int=100_000, optimized_checker=None) -> dict[str, Any]:
    """Compare one history; optimized_checker is injectable only to test gate sensitivity."""
    checker = optimized_checker or check_history
    try:
        optimized = checker(history, max_states=max_states)
    except InvalidHistory as exc:
        return {
            "optimized": None,
            "optimized_status": "INVALID_HISTORY",
            "reference_has_linearization": None,
            "reference_witness": None,
            "reason": "generated_history_invalid",
            "error": str(exc),
        }

    reference, reference_witness = reference_has_linearization(history)
    status = optimized.get("status")
    positive = status == "LINEARIZABLE"
    reason = None
    if status == "UNKNOWN_LIMIT":
        reason = "optimized_checker_unknown_limit"
    elif status == "INVALID_HISTORY":
        reason = "optimized_checker_invalid_history"
    elif status not in {"LINEARIZABLE", "NON_LINEARIZABLE"}:
        reason = "optimized_checker_unrecognized_status"
    elif positive != reference:
        reason = "linearizability_decision_disagreement"
    elif positive and not reference_accepts_witness(history, optimized.get("linearization", [])):
        reason = "optimized_witness_rejected_by_reference"

    return {
        "optimized": optimized,
        "optimized_status": status,
        "reference_has_linearization": reference,
        "reference_witness": reference_witness,
        "reason": reason,
    }


def run_differential(max_states: int=100_000, mismatch_limit: int=10) -> dict[str,Any]:
    root=Path(__file__).resolve().parent
    protocol=json.loads((root/"differential-protocol-v1.json").read_text(encoding="utf-8"))
    expected_corpus_sha256=protocol["corpus_sha256"]
    comparisons=0
    by_tier={"exhaustive_2op":0,"sampled_3op":0,"sampled_4op":0}
    status_counts={"optimized_linearizable":0,"optimized_non_linearizable":0,"optimized_unknown_limit":0,
                   "optimized_invalid_history":0,"reference_linearizable":0,"reference_non_linearizable":0}
    disagreements=[]
    disagreement_count=0
    corpus_hash=hashlib.sha256()
    for tier,history in deterministic_corpus():
        comparisons+=1
        by_tier[tier]+=1
        canonical=json.dumps(history,sort_keys=True,separators=(",",":"),ensure_ascii=False)
        corpus_hash.update(canonical.encode("utf-8")+b"\n")
        comparison=compare_history(history,max_states=max_states)
        optimized=comparison["optimized"]
        reason=comparison["reason"]
        if optimized is None:
            status_counts["optimized_invalid_history"]+=1
        elif optimized["status"]=="LINEARIZABLE":
            status_counts["optimized_linearizable"]+=1
        elif optimized["status"]=="NON_LINEARIZABLE":
            status_counts["optimized_non_linearizable"]+=1
        elif optimized["status"]=="UNKNOWN_LIMIT":
            status_counts["optimized_unknown_limit"]+=1
        else:
            status_counts["optimized_invalid_history"]+=1
        reference=comparison["reference_has_linearization"]
        reference_witness=comparison["reference_witness"]
        if reference is not None:
            status_counts["reference_linearizable" if reference else "reference_non_linearizable"]+=1
        if reason:
            disagreement_count+=1
        if reason and len(disagreements)<mismatch_limit:
            entry={"reason":reason,"tier":tier,"history_sha256":canonical_hash(history),"history":history,
                   "optimized_status":comparison["optimized_status"]}
            if optimized is not None:
                entry["optimized_witness"]=optimized.get("linearization")
            else:
                entry["error"]=comparison.get("error")
            if reference is not None:
                entry["reference_has_linearization"]=reference
                entry["reference_witness"]=reference_witness
            disagreements.append(entry)
    expected_total=5824
    expected_tiers={"exhaustive_2op":5184,"sampled_3op":512,"sampled_4op":128}
    return {
        "schema":SCHEMA,"status":"PASS" if comparisons==expected_total and by_tier==expected_tiers and disagreement_count==0 and corpus_hash.hexdigest()==expected_corpus_sha256 else "FAIL",
        "qualification_status":"BLOCKED","claim_ceiling":"none","scope":"differential_check_of_bounded_linearizability_model_only",
        "comparisons":comparisons,"expected_comparisons":expected_total,"history_counts_by_tier":by_tier,
        "operation_shape_count":72,"exhaustive_two_operation_pair_count":5184,
        "sampled_three_operation_histories":by_tier["sampled_3op"],"sampled_four_operation_histories":by_tier["sampled_4op"],
        "comparison_status_counts":status_counts,"disagreement_count_observed":disagreement_count,
        "disagreement_examples_retained":len(disagreements),"disagreements":disagreements,
        "history_corpus_sha256":corpus_hash.hexdigest(),"expected_corpus_sha256":expected_corpus_sha256,
        "corpus_hash_matches_protocol":corpus_hash.hexdigest()==expected_corpus_sha256,
        "max_states_per_history":max_states,
        "workflow_event_sha":os.environ.get("GITHUB_SHA"),
        "harness_commit_sha":os.environ.get("SRQ_DIFF_CHECKOUT_SHA") or os.environ.get("GITHUB_SHA"),
        "source_head_sha":os.environ.get("SRQ_DIFF_PR_HEAD_SHA") or os.environ.get("GITHUB_SHA"),
        "source_sha256":{name:_file_hash(root/name) for name in (
            "linearizability_oracle.py","differential_reference.py","differential_qualification.py",
            "test_differential_qualification.py","differential-protocol-v1.json")},
        "note":"Finite-corpus agreement is evidence, not proof. Reference semantics and state encoding are independent; only input decoding is shared."
    }


def main(argv: list[str]|None=None) -> int:
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--max-states",type=int,default=100_000)
    parser.add_argument("--output",type=Path)
    args=parser.parse_args(argv)
    report=run_differential(max_states=args.max_states)
    rendered=json.dumps(report,indent=2,sort_keys=True)+"\n"
    if args.output:
        args.output.parent.mkdir(parents=True,exist_ok=True)
        args.output.write_text(rendered,encoding="utf-8")
    print(rendered,end="")
    return 0 if report["status"]=="PASS" else 1


if __name__=="__main__":
    raise SystemExit(main())
