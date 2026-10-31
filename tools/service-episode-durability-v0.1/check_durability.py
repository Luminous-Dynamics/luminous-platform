#!/usr/bin/env python3
"""Dependency-free contract guard for Service-Episode Durability v0.1."""
from __future__ import annotations
import json
import sys
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
MATRIX = ROOT / "benchmarks" / "service-episode-durability-v0.1" / "crash-matrix.json"
SCHEMA = ROOT / "benchmarks" / "service-episode-durability-v0.1" / "crash-matrix.schema.json"

FAMILIES = {
    "intent-boundary", "pre-dispatch", "participant-admission", "receipt-commit",
    "coordinator-fencing", "snapshot-replay", "storage-integrity", "compensation-cancellation",
}
EXPECTED_IDS = {
    "INTENT-001","INTENT-002","INTENT-003","DISPATCH-001","DISPATCH-002",
    "ADMISSION-001","ADMISSION-002","ADMISSION-003","RECEIPT-001","RECEIPT-002",
    "FENCE-001","FENCE-002","FENCE-003","SNAPSHOT-001","SNAPSHOT-002",
    "SNAPSHOT-003","SNAPSHOT-004","STORE-001","STORE-002","STORE-003",
    "STORE-004","CANCEL-001","COMP-001","COMP-002",
}
EXPECTED_GATES = {
    "max_lost_acknowledged_events": 0,
    "max_duplicate_business_effects": 0,
    "max_unauthorized_dispatches": 0,
    "max_automatic_replays_with_unknown_effect": 0,
    "max_stale_owner_commits_after_fence": 0,
    "valid_history_replay_rate_percent": 100,
    "malformed_history_fail_closed_rate_percent": 100,
    "run_binding_correlation_rate_percent": 100,
}
EXPECTED_TARGETS = {
    "acknowledged_commit_rpo_events": 0,
    "same_node_restart_rto_p95_ms": 5000,
    "p95_episode_latency_overhead_percent": 10,
    "human_adjudications_per_1000_episodes_max": 1,
}
TOP_KEYS = {
    "schema","version","benchmark","benchmark_version","status","scope","claim_ceiling",
    "recovery_unit","external_effect_oracle","invariants","hard_gates",
    "initial_targets_not_results","metrics","baseline_arms","comparison_rules","cases",
}
CASE_KEYS = {
    "id","family","crash_point","intent_durability","participant_state","journal_state",
    "expected_recovery","required_gate","automatic_external_replay","expected_effect_count","rationale",
}

def fail(message: str) -> None:
    raise ValueError(message)

def require(condition: bool, message: str) -> None:
    if not condition:
        fail(message)

def unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            fail(f"duplicate JSON key: {key}")
        result[key] = value
    return result

def load(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=unique_object)
    except (OSError, UnicodeError, json.JSONDecodeError) as exc:
        fail(f"cannot load JSON {path}: {exc}")

def validate(matrix: Any, schema: Any) -> dict[str, int]:
    require(isinstance(matrix, dict) and set(matrix) == TOP_KEYS, "matrix top-level contract drifted")
    require(matrix["schema"] == "luminous-service-episode-durability-matrix", "wrong schema ID")
    require(type(matrix["version"]) is int and matrix["version"] == 1, "wrong version")
    require(matrix["benchmark"] == "service-episode-durability" and matrix["benchmark_version"] == "0.1",
            "benchmark identity drifted")
    require(matrix["status"] == "synthetic_evaluator_only_not_runtime_input", "matrix must remain evaluator-only")
    require(matrix["claim_ceiling"] == "E0_E1_contract_and_synthetic_fault_matrix_only", "claim ceiling drifted")
    require(matrix["recovery_unit"] == "episode", "recovery unit must remain the service episode")

    oracle = matrix["external_effect_oracle"]
    require(set(oracle) == {"required","owner","independent_of","purpose"} and oracle["required"] is True,
            "independent external effect oracle is mandatory")
    require("participant" in oracle["owner"] and "coordinator_event_log" in oracle["independent_of"],
            "effect oracle must be participant-owned and independent of coordinator history")

    invariants = matrix["invariants"]
    require(isinstance(invariants, list) and len(invariants) == 10 and len(set(invariants)) == 10,
            "exactly ten unique safety invariants are required")
    require("platform_coordinator_journal_does_not_duplicate_nixward_host_journal" in invariants,
            "Platform/Nixward journal ownership boundary is missing")

    require(matrix["hard_gates"] == EXPECTED_GATES, "zero-tolerance safety gates may not be weakened")
    require(matrix["initial_targets_not_results"] == EXPECTED_TARGETS, "initial target hypotheses drifted")
    metrics = matrix["metrics"]
    require(isinstance(metrics, list) and len(metrics) == 12 and len(set(metrics)) == 12,
            "exactly twelve separate performance/recovery metrics are required")
    arms = matrix["baseline_arms"]
    require(isinstance(arms, list) and {x.get("id") for x in arms} ==
            {"platform_minimal_coordinator_journal","temporal_event_history_replay",
             "dbos_postgres_checkpoint_recovery","in_memory_reference"},
            "fair comparison arms are incomplete")
    require(next(x for x in arms if x["id"] == "in_memory_reference")["role"] == "non_durable_control_only",
            "in-memory arm must remain a non-durable control")
    rules = matrix["comparison_rules"]
    require(isinstance(rules, list) and len(rules) == 5 and len(set(rules)) == 5,
            "comparison fairness protocol drifted")
    require(any("does not prove arbitrary external connector effect outcome" in x for x in rules),
            "durable workflow history must not be equated with external-effect truth")

    cases = matrix["cases"]
    require(isinstance(cases, list) and len(cases) == 24, "exactly 24 crash cases are required")
    ids: set[str] = set()
    families: set[str] = set()
    for case in cases:
        require(isinstance(case, dict) and set(case) == CASE_KEYS, "crash case fields drifted")
        cid = case["id"]
        require(cid in EXPECTED_IDS and cid not in ids, f"unknown or duplicate case ID: {cid}")
        ids.add(cid)
        require(case["family"] in FAMILIES, f"{cid}: unknown family")
        families.add(case["family"])
        require(case["automatic_external_replay"] is False, f"{cid}: external replay must never be automatic")
        require(case["expected_recovery"] in {
            "safe_to_resume","outcome_unknown","quarantined","fenced","reconcile_required"
        }, f"{cid}: invalid recovery disposition")
        require(isinstance(case["rationale"], str) and len(case["rationale"]) >= 30,
                f"{cid}: rationale too short")
        if case["participant_state"] in {"may_be_sent","accepted_pending","effect_committed","receipt_received"}:
            require(case["expected_recovery"] in {"outcome_unknown","reconcile_required","fenced"},
                    f"{cid}: ambiguous participant activity cannot be marked safe")
        if case["journal_state"] in {"torn_tail","corrupt","event_gap"}:
            require(case["expected_recovery"] in {"quarantined","reconcile_required"},
                    f"{cid}: invalid journal cannot be replayed as valid")
        if case["journal_state"] == "ownership_changed":
            require(case["expected_recovery"] == "fenced", f"{cid}: changed owner must fence stale process")
    require(ids == EXPECTED_IDS, "crash case inventory changed")
    require(families == FAMILIES, "all eight crash fault families must be represented")

    require(isinstance(schema, dict) and schema.get("$schema") ==
            "https://json-schema.org/draft/2020-12/schema", "schema must be Draft 2020-12")
    require(schema.get("properties", {}).get("cases", {}).get("minItems") == 24 and
            schema.get("properties", {}).get("cases", {}).get("maxItems") == 24,
            "JSON Schema must freeze exactly 24 crash cases")
    require(schema.get("additionalProperties") is False, "schema must reject unknown top-level fields")
    item_schema = schema.get("properties", {}).get("cases", {}).get("items", {})
    require(set(item_schema.get("properties", {})) == CASE_KEYS and
            item_schema.get("additionalProperties") is False,
            "JSON Schema case field inventory drifted")
    require(item_schema.get("properties", {}).get("automatic_external_replay", {}).get("const") is False,
            "JSON Schema must forbid automatic external replay")
    return {"case_count":len(ids), "family_count":len(families), "metric_count":len(metrics),
            "hard_gate_count":len(matrix["hard_gates"]), "comparison_arm_count":len(arms)}

def main() -> int:
    try:
        result = validate(load(MATRIX), load(SCHEMA))
    except (ValueError, KeyError, TypeError, StopIteration) as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        return 1
    print("PASS: Service-Episode Durability v0.1 structural contract")
    for key, value in result.items():
        print(f"{key}={value}")
    print("SCOPE: synthetic crash-matrix integrity only; no runtime durability qualification.")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
