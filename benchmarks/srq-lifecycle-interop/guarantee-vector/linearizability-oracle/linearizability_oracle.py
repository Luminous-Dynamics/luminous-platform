#!/usr/bin/env python3
"""Bounded linearizability checker for SRQ idempotent-sink histories."""
from __future__ import annotations
import argparse
from dataclasses import dataclass
import hashlib, json, os, re
from pathlib import Path
from typing import Any

HISTORY_SCHEMA = "srq-linearizability-history-v1"
REPORT_SCHEMA = "srq-linearizability-report-v1"
DIGEST = re.compile(r"^[0-9a-f]{64}$")
INTENT_FIELDS = ("request_digest", "effect_digest", "policy_digest")
RESULTS = {"committed", "unknown", "key_conflict"}
MAX_OPERATIONS = 64
State = tuple[tuple[str, str, str, str, str, str, str], ...]

class InvalidHistory(ValueError):
    pass

@dataclass(frozen=True)
class Operation:
    op_id: str
    start_ns: int
    end_ns: int
    key: str
    intent: tuple[str, str, str]
    result: str
    effect_id: str | None
    receipt_id: str | None

def _string(value: Any, field: str) -> str:
    if not isinstance(value, str) or not value:
        raise InvalidHistory(f"{field} must be a non-empty string")
    return value

def _uint(value: Any, field: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value < 0:
        raise InvalidHistory(f"{field} must be a non-negative integer")
    return value

def _digest(value: Any, field: str) -> str:
    if not isinstance(value, str) or not DIGEST.fullmatch(value):
        raise InvalidHistory(f"{field} must be a lowercase 64-hex SHA-256 digest")
    return value

def parse_history(history: Any) -> tuple[str, list[Operation]]:
    if not isinstance(history, dict) or history.get("schema") != HISTORY_SCHEMA:
        raise InvalidHistory(f"top-level schema must be {HISTORY_SCHEMA!r}")
    if history.get("clock_domain") != "single_observer_monotonic":
        raise InvalidHistory("clock_domain must be single_observer_monotonic")
    sink = _digest(history.get("sink_identity_digest"), "sink_identity_digest")
    raw_ops = history.get("operations")
    if not isinstance(raw_ops, list) or not raw_ops:
        raise InvalidHistory("operations must be a non-empty array")
    if len(raw_ops) > MAX_OPERATIONS:
        raise InvalidHistory(f"operation count exceeds bounded checker limit {MAX_OPERATIONS}")
    operations: list[Operation] = []
    seen_ids: set[str] = set()
    for index, raw in enumerate(raw_ops):
        prefix = f"operations[{index}]"
        if not isinstance(raw, dict):
            raise InvalidHistory(f"{prefix} must be an object")
        op_id = _string(raw.get("op_id"), f"{prefix}.op_id")
        if op_id in seen_ids:
            raise InvalidHistory(f"duplicate op_id: {op_id}")
        seen_ids.add(op_id)
        start = _uint(raw.get("start_ns"), f"{prefix}.start_ns")
        end = _uint(raw.get("end_ns"), f"{prefix}.end_ns")
        if end < start:
            raise InvalidHistory(f"{prefix}.end_ns precedes start_ns")
        key = _string(raw.get("idempotency_key"), f"{prefix}.idempotency_key")
        intent = tuple(_digest(raw.get(name), f"{prefix}.{name}") for name in INTENT_FIELDS)
        result = _string(raw.get("result"), f"{prefix}.result")
        if result not in RESULTS:
            raise InvalidHistory(f"{prefix}.result must be one of {sorted(RESULTS)}")
        effect_id, receipt_id = raw.get("effect_id"), raw.get("receipt_id")
        if result == "committed":
            effect_id = _string(effect_id, f"{prefix}.effect_id")
            receipt_id = _string(receipt_id, f"{prefix}.receipt_id")
        elif effect_id is not None or receipt_id is not None:
            raise InvalidHistory(f"{prefix}: non-committed results must not claim effect_id or receipt_id")
        operations.append(Operation(op_id, start, end, key, intent, result, effect_id, receipt_id))
    return sink, operations

def _freeze(mapping: dict[str, tuple[tuple[str, str, str], str, str, str]]) -> State:
    return tuple((key, intent[0], intent[1], intent[2], mode, effect_id, receipt_id)
                 for key, (intent, mode, effect_id, receipt_id) in sorted(mapping.items()))

def _state_map(state: State) -> dict[str, tuple[tuple[str, str, str], str, str, str]]:
    return {item[0]: ((item[1], item[2], item[3]), item[4], item[5], item[6]) for item in state}

def _ids_available(state: State, effect_id: str, receipt_id: str, key: str) -> bool:
    if not effect_id or not receipt_id:
        return False
    for item in state:
        if item[0] != key and item[4] == "applied":
            if item[5] == effect_id or item[6] == receipt_id:
                return False
    return True

def _transitions(state: State, op: Operation) -> list[State]:
    mapping = _state_map(state)
    old = mapping.get(op.key)
    if old is None:
        if op.result == "committed":
            assert op.effect_id is not None and op.receipt_id is not None
            if not _ids_available(state, op.effect_id, op.receipt_id, op.key):
                return []
            updated = dict(mapping)
            updated[op.key] = (op.intent, "applied", op.effect_id, op.receipt_id)
            return [_freeze(updated)]
        if op.result == "key_conflict":
            return []
        reserved = dict(mapping)
        reserved[op.key] = (op.intent, "none", "", "")
        uncertain = dict(mapping)
        uncertain[op.key] = (op.intent, "unknown", "", "")
        return list(dict.fromkeys([state, _freeze(reserved), _freeze(uncertain)]))

    prior_intent, mode, prior_effect, prior_receipt = old
    if prior_intent != op.intent:
        return [state] if op.result in {"key_conflict", "unknown"} else []
    if op.result == "key_conflict":
        return []
    if op.result == "unknown":
        transitions = [state]
        if mode == "none":
            updated = dict(mapping)
            updated[op.key] = (prior_intent, "unknown", "", "")
            transitions.append(_freeze(updated))
        return list(dict.fromkeys(transitions))
    assert op.effect_id is not None and op.receipt_id is not None
    if mode == "applied":
        return [state] if (op.effect_id == prior_effect and op.receipt_id == prior_receipt) else []
    if not _ids_available(state, op.effect_id, op.receipt_id, op.key):
        return []
    updated = dict(mapping)
    updated[op.key] = (prior_intent, "applied", op.effect_id, op.receipt_id)
    return [_freeze(updated)]

def check_history(history: Any, *, max_states: int = 100_000) -> dict[str, Any]:
    if isinstance(max_states, bool) or not isinstance(max_states, int) or max_states < 1:
        raise ValueError("max_states must be a positive integer")
    sink_digest, operations = parse_history(history)
    count = len(operations)
    # Only strict separation implies real-time precedence; equal endpoints are ambiguous.
    precedes = [[i != j and operations[i].end_ns < operations[j].start_ns
                 for j in range(count)] for i in range(count)]
    visited: set[tuple[int, State]] = set()
    explored_states = 0
    limit_hit = False

    def search(remaining: int, state: State) -> list[str] | None:
        nonlocal explored_states, limit_hit
        if remaining == 0:
            return []
        cache_key = (remaining, state)
        if cache_key in visited:
            return None
        if explored_states >= max_states:
            limit_hit = True
            return None
        explored_states += 1
        visited.add(cache_key)
        indices = [i for i in range(count) if remaining & (1 << i)]
        eligible = [i for i in indices if not any(
            remaining & (1 << j) and precedes[j][i] for j in indices if j != i
        )]
        eligible.sort(key=lambda i: (operations[i].start_ns, operations[i].op_id))
        for i in eligible:
            next_remaining = remaining & ~(1 << i)
            for next_state in _transitions(state, operations[i]):
                suffix = search(next_remaining, next_state)
                if suffix is not None:
                    return [operations[i].op_id, *suffix]
        return None

    witness = search((1 << count) - 1, ())
    status = ("LINEARIZABLE" if witness is not None else
              "UNKNOWN_LIMIT" if limit_hit else "NON_LINEARIZABLE")
    return {
        "schema": REPORT_SCHEMA,
        "status": status,
        "qualification_status": "BLOCKED",
        "claim_ceiling": "none",
        "scope": "bounded_linearizability_of_simplified_idempotent_sink_contract",
        "clock_domain": "single_observer_monotonic",
        "sink_identity_digest": sink_digest,
        "operation_count": count,
        "explored_states": explored_states,
        "max_states": max_states,
        "search_limit_hit": limit_hit,
        "linearization": witness,
        "liveness_claim": "NOT_EVALUATED",
        "note": "A witness applies only to the supplied finite history and simplified model. UNKNOWN_LIMIT is inconclusive; traces and recorder are not authenticated, and no real sink is qualified."
    }

def _sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()

def add_provenance(report: dict[str, Any]) -> dict[str, Any]:
    root = Path(__file__).resolve().parent
    names = ["linearizability_oracle.py", "test_linearizability_oracle.py",
             "linearizability-protocol-v1.json", "fixtures/valid-concurrent-same-key.json",
             "fixtures/invalid-duplicate-effect.json", "fixtures/invalid-realtime-conflict.json"]
    report["workflow_event_sha"] = os.environ.get("GITHUB_SHA")
    report["harness_commit_sha"] = os.environ.get("SRQ_LIN_CHECKOUT_SHA") or os.environ.get("GITHUB_SHA")
    report["source_head_sha"] = os.environ.get("SRQ_LIN_PR_HEAD_SHA") or os.environ.get("GITHUB_SHA")
    report["source_sha256"] = {name: _sha256_file(root / name) for name in names}
    return report

def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("history", type=Path, help="JSON operation history")
    parser.add_argument("--max-states", type=int, default=100_000)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args(argv)
    try:
        history = json.loads(args.history.read_text(encoding="utf-8"))
        report = check_history(history, max_states=args.max_states)
    except (OSError, json.JSONDecodeError, InvalidHistory, ValueError) as exc:
        report = {"schema": REPORT_SCHEMA, "status": "INVALID_HISTORY",
                  "qualification_status": "BLOCKED", "claim_ceiling": "none", "error": str(exc)}
        report = add_provenance(report)
        rendered = json.dumps(report, indent=2, sort_keys=True) + "\n"
        if args.output:
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(rendered, encoding="utf-8")
        print(rendered, end="")
        return 2
    report = add_provenance(report)
    rendered = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered, encoding="utf-8")
    print(rendered, end="")
    return {"LINEARIZABLE": 0, "NON_LINEARIZABLE": 1, "UNKNOWN_LIMIT": 3}.get(report["status"], 2)

if __name__ == "__main__":
    raise SystemExit(main())
