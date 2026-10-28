#!/usr/bin/env python3
"""Deterministic generated-history and single-fault mutation smoke benchmark."""
from __future__ import annotations
import copy
import random
from history_checker import check_history

SEED = 20261028
CASES = 10_000
RNG = random.Random(SEED)


def valid_trace(index: int) -> dict:
    key_count = RNG.randint(1, 8)
    trace = {
        "schema": "srq-side-effect-history-v1",
        "trace_id": f"generated-{index}",
        "sink_id": "sink-generated",
        "fencing_required": True,
        "operations": [],
        "transactions": [],
        "fence_events": [],
    }
    sequence = 1
    for key_index in range(key_count):
        key = f"key-{index}-{key_index}"
        digest = f"sha256:{index:08x}-{key_index:02x}"
        transaction_id = f"tx-{index}-{key_index}"
        effect_id = f"effect-{index}-{key_index}"
        receipt_id = f"receipt-{index}-{key_index}"
        token = key_index + 1
        trace["fence_events"].append({"event_seq": sequence, "token": token})
        sequence += 1
        trace["transactions"].append({
            "transaction_id": transaction_id,
            "key": key,
            "request_digest": digest,
            "effect_id": effect_id,
            "receipt_id": receipt_id,
            "fencing_token": token,
            "commit_seq": sequence,
        })
        sequence += 1
        for retry in range(RNG.randint(1, 4)):
            trace["operations"].append({
                "op_id": f"op-{index}-{key_index}-{retry}",
                "key": key,
                "request_digest": digest,
                "outcome": "success",
                "transaction_id": transaction_id,
                "effect_id": effect_id,
                "receipt_id": receipt_id,
                "invoke_seq": RNG.randint(0, 1000),
                "return_seq": RNG.randint(1001, 2000),
            })
    return trace


def inject_one_fault(trace: dict, fault_index: int) -> None:
    if fault_index == 0:
        trace["operations"][0]["effect_id"] = "forged-effect"
    elif fault_index == 1:
        trace["operations"][0]["request_digest"] = "sha256:different-intent"
    elif fault_index == 2:
        original = trace["transactions"][0]
        trace["transactions"].append({
            **original,
            "transaction_id": "duplicate-tx",
            "effect_id": "new-effect",
            "receipt_id": "new-receipt",
            "fencing_token": max(item["token"] for item in trace["fence_events"]),
            "commit_seq": max(item["commit_seq"] for item in trace["transactions"]) + 1,
        })
    elif fault_index == 3:
        trace["transactions"][0]["receipt_id"] = ""
    else:
        trace["fence_events"].append({
            "event_seq": max(item["event_seq"] for item in trace["fence_events"]) + 1,
            "token": 1,
        })


def main() -> int:
    valid_false_rejects = 0
    mutant_survivors = 0
    failure_codes: dict[str, int] = {}
    for index in range(CASES):
        trace = valid_trace(index)
        report = check_history(trace)
        if report["status"] != "PASS":
            valid_false_rejects += 1
            for violation in report["violations"]:
                code = violation["code"]
                failure_codes[code] = failure_codes.get(code, 0) + 1
        mutant = copy.deepcopy(trace)
        inject_one_fault(mutant, index % 5)
        if check_history(mutant)["status"] == "PASS":
            mutant_survivors += 1
    result = {
        "schema": "srq-generated-history-smoke-report-v1",
        "seed": SEED,
        "generated_valid_histories": CASES,
        "valid_false_rejects": valid_false_rejects,
        "generated_single_fault_mutants": CASES,
        "mutant_survivors": mutant_survivors,
        "valid_failure_codes": failure_codes,
        "status": "PASS" if not valid_false_rejects and not mutant_survivors else "FAIL",
        "qualification_status": "BLOCKED",
        "claim_ceiling": "none",
        "scope": "deterministic-generated-checker-smoke-not-adapter-or-linearizability-evidence",
    }
    import json
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0 if result["status"] == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
