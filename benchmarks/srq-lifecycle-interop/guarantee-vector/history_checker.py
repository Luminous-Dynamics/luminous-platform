#!/usr/bin/env python3
"""Independent checker for SRQ real-sink delivery histories.

Research/qualification harness only. It validates the supplied journal and
observed request outcomes; it does not authenticate the journal or prove that
an adapter emitted complete, truthful records. It is deliberately not a full
linearizability checker.
"""
from __future__ import annotations
import argparse
import json
from pathlib import Path
from typing import Any

SCHEMA = "srq-side-effect-history-v1"
REPORT_SCHEMA = "srq-side-effect-history-report-v1"


def _nonempty(value: Any) -> bool:
    return isinstance(value, str) and bool(value.strip())


def _integer(value: Any) -> bool:
    return isinstance(value, int) and not isinstance(value, bool) and value >= 0


def check_history(trace: dict[str, Any]) -> dict[str, Any]:
    violations: list[dict[str, str]] = []

    def add(code: str, detail: str) -> None:
        violations.append({"code": code, "detail": detail})

    if not isinstance(trace, dict):
        trace = {}
        add("INVALID_ROOT", "trace must be a JSON object")
    if trace.get("schema") != SCHEMA:
        add("SCHEMA_MISMATCH", f"expected schema {SCHEMA!r}")
    if not _nonempty(trace.get("trace_id")):
        add("INVALID_TRACE_ID", "trace_id must be a non-empty string")
    if not _nonempty(trace.get("sink_id")):
        add("INVALID_SINK_ID", "sink_id must be a non-empty string")
    if not isinstance(trace.get("fencing_required"), bool):
        add("INVALID_FENCING_POLICY", "fencing_required must be a boolean")

    operations = trace.get("operations")
    transactions = trace.get("transactions")
    fence_events = trace.get("fence_events")
    if not isinstance(operations, list):
        operations = []
        add("INVALID_OPERATIONS", "operations must be an array")
    if not isinstance(transactions, list):
        transactions = []
        add("INVALID_TRANSACTIONS", "transactions must be an array")
    if not isinstance(fence_events, list):
        fence_events = []
        add("INVALID_FENCE_EVENTS", "fence_events must be an array")

    valid_ops: list[dict[str, Any]] = []
    op_ids: set[str] = set()
    allowed_outcomes = {"success", "rejected", "unknown"}
    for index, op in enumerate(operations):
        prefix = f"operations[{index}]"
        if not isinstance(op, dict):
            add("INVALID_OPERATION", f"{prefix} must be an object")
            continue
        required_strings = ("op_id", "key", "request_digest", "outcome")
        if not all(_nonempty(op.get(key)) for key in required_strings):
            add("INVALID_OPERATION_FIELDS", f"{prefix} requires non-empty op_id/key/request_digest/outcome")
            continue
        if op["op_id"] in op_ids:
            add("DUPLICATE_OPERATION_ID", f"operation id {op['op_id']!r} is repeated")
        op_ids.add(op["op_id"])
        if op["outcome"] not in allowed_outcomes:
            add("INVALID_OUTCOME", f"{prefix} outcome must be one of {sorted(allowed_outcomes)}")
            continue
        if "invoke_seq" in op and not _integer(op["invoke_seq"]):
            add("INVALID_INVOKE_SEQ", f"{prefix}.invoke_seq must be a non-negative integer")
        if "return_seq" in op and op["return_seq"] is not None and not _integer(op["return_seq"]):
            add("INVALID_RETURN_SEQ", f"{prefix}.return_seq must be null or a non-negative integer")
        if op["outcome"] == "success":
            for key in ("transaction_id", "effect_id", "receipt_id"):
                if not _nonempty(op.get(key)):
                    add("SUCCESS_WITHOUT_BINDING", f"{prefix} success requires {key}")
        valid_ops.append(op)

    valid_txs: list[dict[str, Any]] = []
    tx_ids: set[str] = set()
    effect_ids: set[str] = set()
    receipt_ids: set[str] = set()
    tx_by_id: dict[str, dict[str, Any]] = {}
    tx_by_key: dict[str, dict[str, Any]] = {}
    ledger_seqs: set[int] = set()
    for index, tx in enumerate(transactions):
        prefix = f"transactions[{index}]"
        if not isinstance(tx, dict):
            add("INVALID_TRANSACTION", f"{prefix} must be an object")
            continue
        required = ("transaction_id", "key", "request_digest", "effect_id", "receipt_id")
        if not all(_nonempty(tx.get(key)) for key in required):
            add("PARTIAL_ATOMIC_TRANSACTION", f"{prefix} must bind transaction, key, intent, effect, and receipt IDs")
            continue
        if tx["transaction_id"] in tx_ids:
            add("DUPLICATE_TRANSACTION_ID", f"transaction id {tx['transaction_id']!r} is repeated")
        if tx["effect_id"] in effect_ids:
            add("DUPLICATE_EFFECT_ID", f"effect id {tx['effect_id']!r} is reused")
        if tx["receipt_id"] in receipt_ids:
            add("DUPLICATE_RECEIPT_ID", f"receipt id {tx['receipt_id']!r} is reused")
        if tx["key"] in tx_by_key:
            add("MULTIPLE_EFFECTS_FOR_KEY", f"idempotency key {tx['key']!r} has more than one committed transaction")
        if not _integer(tx.get("commit_seq")):
            add("INVALID_COMMIT_SEQ", f"{prefix}.commit_seq must be a non-negative integer")
        elif tx["commit_seq"] in ledger_seqs:
            add("DUPLICATE_LEDGER_SEQUENCE", f"durable ledger sequence {tx['commit_seq']} is reused")
        else:
            ledger_seqs.add(tx["commit_seq"])
        if tx.get("fencing_token") is not None and not _integer(tx.get("fencing_token")):
            add("INVALID_FENCING_TOKEN", f"{prefix}.fencing_token must be null or a non-negative integer")
        tx_ids.add(tx["transaction_id"])
        effect_ids.add(tx["effect_id"])
        receipt_ids.add(tx["receipt_id"])
        tx_by_id.setdefault(tx["transaction_id"], tx)
        tx_by_key.setdefault(tx["key"], tx)
        valid_txs.append(tx)

    valid_fences: list[dict[str, Any]] = []
    fence_seqs: set[int] = set()
    for index, event in enumerate(fence_events):
        prefix = f"fence_events[{index}]"
        if not isinstance(event, dict):
            add("INVALID_FENCE_EVENT", f"{prefix} must be an object")
            continue
        if not _integer(event.get("event_seq")) or not _integer(event.get("token")):
            add("INVALID_FENCE_FIELDS", f"{prefix} requires non-negative integer event_seq and token")
            continue
        if event["event_seq"] in ledger_seqs or event["event_seq"] in fence_seqs:
            add("DUPLICATE_LEDGER_SEQUENCE", f"durable ledger sequence {event['event_seq']} is reused")
        fence_seqs.add(event["event_seq"])
        valid_fences.append(event)

    tx_digest_by_key = {tx["key"]: tx["request_digest"] for tx in valid_txs}
    for op in valid_ops:
        tx = tx_by_id.get(op.get("transaction_id")) if _nonempty(op.get("transaction_id")) else None
        if op["outcome"] == "success":
            if tx is None:
                add("SUCCESS_WITHOUT_COMMITTED_TRANSACTION", f"operation {op['op_id']!r} references no committed transaction")
                continue
            if tx.get("key") != op.get("key") or tx.get("request_digest") != op.get("request_digest"):
                add("SUCCESS_INTENT_BINDING_MISMATCH", f"operation {op['op_id']!r} does not match its transaction's key and request digest")
            if tx.get("effect_id") != op.get("effect_id") or tx.get("receipt_id") != op.get("receipt_id"):
                add("SUCCESS_RECEIPT_EFFECT_MISMATCH", f"operation {op['op_id']!r} returned IDs that differ from the committed transaction")
        elif op["outcome"] == "rejected" and op.get("reject_reason") == "idempotency_key_payload_mismatch":
            known_digest = tx_digest_by_key.get(op.get("key"))
            if known_digest is None or known_digest == op.get("request_digest"):
                add("UNJUSTIFIED_KEY_MISMATCH_REJECTION", f"operation {op['op_id']!r} claims key-payload mismatch without a different committed intent")
            if any(tx.get("key") == op.get("key") and tx.get("request_digest") == op.get("request_digest") for tx in valid_txs):
                add("REJECTED_REQUEST_HAD_EFFECT", f"rejected mismatch operation {op['op_id']!r} also has a committed effect")
        elif op["outcome"] == "unknown" and tx is not None:
            if tx.get("key") != op.get("key") or tx.get("request_digest") != op.get("request_digest"):
                add("UNKNOWN_OUTCOME_BINDING_MISMATCH", f"unknown operation {op['op_id']!r} references a transaction for another intent")

    by_seq = sorted(
        [(e["event_seq"], "fence", e) for e in valid_fences]
        + [(t["commit_seq"], "transaction", t) for t in valid_txs if _integer(t.get("commit_seq"))],
        key=lambda item: item[0],
    )
    current_fence: int | None = None
    previous_fence: int | None = None
    seen_fence_tokens: set[int] = set()
    for seq, kind, entry in by_seq:
        if kind == "fence":
            token = entry["token"]
            if previous_fence is not None and token <= previous_fence:
                add("FENCE_NOT_MONOTONIC", f"fencing token {token} at sequence {seq} did not exceed prior token {previous_fence}")
            if token in seen_fence_tokens:
                add("FENCE_TOKEN_REUSED", f"fencing token {token} appears in more than one advancement event")
            seen_fence_tokens.add(token)
            previous_fence = token
            current_fence = token
        else:
            token = entry.get("fencing_token")
            fencing_required = trace.get("fencing_required") is True
            if fencing_required and current_fence is None:
                add("EFFECT_WITHOUT_FENCE", f"transaction {entry['transaction_id']!r} committed before any fencing token was established")
            elif fencing_required and token != current_fence:
                add("STALE_OR_WRONG_FENCE", f"transaction {entry['transaction_id']!r} used token {token!r}; current sink token is {current_fence!r}")
            elif token is not None and current_fence is not None and token != current_fence:
                add("STALE_OR_WRONG_FENCE", f"transaction {entry['transaction_id']!r} used token {token!r}; current sink token is {current_fence!r}")

    successes: dict[tuple[str, str], dict[str, str]] = {}
    for op in valid_ops:
        if op.get("outcome") != "success":
            continue
        identity = (op.get("key", ""), op.get("request_digest", ""))
        value = {name: str(op.get(name, "")) for name in ("transaction_id", "effect_id", "receipt_id")}
        prior = successes.setdefault(identity, value)
        if prior != value:
            add("RETRY_RESULT_DIVERGENCE", f"successful retries for key {identity[0]!r} and intent {identity[1]!r} returned different effect/receipt identities")

    for op in valid_ops:
        committed_digest = tx_digest_by_key.get(op.get("key"))
        if committed_digest is None or committed_digest == op.get("request_digest"):
            continue
        if op.get("outcome") == "success":
            add("KEY_REUSED_FOR_DIFFERENT_INTENT", f"operation {op['op_id']!r} succeeded with a different intent under an already committed key")
        if op.get("outcome") == "rejected" and op.get("reject_reason") not in ("idempotency_key_payload_mismatch", "validation_error"):
            add("WRONG_KEY_REUSE_REJECTION_REASON", f"operation {op['op_id']!r} has a different intent but an unexpected rejection reason")

    return {
        "schema": REPORT_SCHEMA,
        "status": "PASS" if not violations else "FAIL",
        "qualification_status": "BLOCKED",
        "claim_ceiling": "none",
        "scope": "observed-idempotency-atomic-receipt-and-fencing-consistency-not-full-linearizability",
        "trace_id": trace.get("trace_id"),
        "sink_id": trace.get("sink_id"),
        "operation_count": len(operations),
        "transaction_count": len(transactions),
        "fence_event_count": len(fence_events),
        "violation_count": len(violations),
        "violations": violations,
        "note": "Checker output is only as complete and trustworthy as the supplied operation history and durable sink journal. This is not cryptographic attestation, a completeness proof, or a full linearizability proof.",
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("trace", type=Path, help="JSON trace adhering to srq-side-effect-history-v1")
    parser.add_argument("--output", type=Path, help="optional path for JSON report")
    args = parser.parse_args()
    try:
        trace = json.loads(args.trace.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        report = {"schema": REPORT_SCHEMA, "status": "INVALID_INPUT", "qualification_status": "BLOCKED", "claim_ceiling": "none", "violations": [{"code": "TRACE_READ_FAILED", "detail": str(exc)}]}
    else:
        report = check_history(trace)
    rendered = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered, encoding="utf-8")
    print(rendered, end="")
    return 0 if report.get("status") == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
