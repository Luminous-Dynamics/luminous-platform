#!/usr/bin/env python3
"""Independent safety oracle for observed SRQ sink histories.

The input is an adapter-produced, totally ordered event history. This tool does not
invoke a sink, establish durability, authenticate evidence, or prove that the event
recorder is honest. It checks whether the recorded behavior violates the declared
single-sink idempotency, receipt-binding, response, and fencing contract.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
from collections import defaultdict
from pathlib import Path
from typing import Any

SCHEMA = "srq-sink-history-v1"
REPORT_SCHEMA = "srq-history-oracle-report-v1"
HEX_DIGEST = re.compile(r"^[0-9a-f]{64}$")
INTENT_FIELDS = ("request_digest", "effect_digest", "policy_digest")


class InvalidHistory(ValueError):
    """The trace cannot be interpreted under the frozen history schema."""


def _nonempty_string(value: Any, field: str) -> str:
    if not isinstance(value, str) or not value:
        raise InvalidHistory(f"{field} must be a non-empty string")
    return value


def _digest(value: Any, field: str) -> str:
    if not isinstance(value, str) or not HEX_DIGEST.fullmatch(value):
        raise InvalidHistory(f"{field} must be a lowercase SHA-256 hex digest")
    return value


def _uint(value: Any, field: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value < 0:
        raise InvalidHistory(f"{field} must be a non-negative integer")
    return value


def _intent(event: dict[str, Any], prefix: str) -> tuple[str, str, str]:
    return tuple(_digest(event.get(k), f"{prefix}.{k}") for k in INTENT_FIELDS)  # type: ignore[return-value]


def check_history(history: dict[str, Any]) -> dict[str, Any]:
    """Check one normalized history and return safety plus separate progress metrics.

    Event sequence numbers define the recorder's observed total order; they are not
    wall-clock timestamps. All event claims are only as trustworthy as the adapter
    that records them.
    """
    if not isinstance(history, dict) or history.get("schema") != SCHEMA:
        raise InvalidHistory(f"top-level schema must be {SCHEMA!r}")
    sink_digest = _digest(history.get("sink_identity_digest"), "sink_identity_digest")
    initial_fence = _uint(history.get("initial_fencing_token", 0), "initial_fencing_token")
    events = history.get("events")
    if not isinstance(events, list):
        raise InvalidHistory("events must be an array")

    violations: list[dict[str, Any]] = []
    attempts: dict[str, dict[str, Any]] = {}
    intents_by_key: dict[str, tuple[str, str, str]] = {}
    effects_by_id: dict[str, dict[str, Any]] = {}
    effects_by_key: dict[str, list[dict[str, Any]]] = defaultdict(list)
    receipts_by_id: dict[str, dict[str, Any]] = {}
    responses: dict[str, list[dict[str, Any]]] = defaultdict(list)
    active_fence = initial_fence
    seen_sequences: set[int] = set()

    def violation(code: str, seq: int, **context: Any) -> None:
        violations.append({"code": code, "seq": seq, "context": context})

    for index, event in enumerate(events):
        if not isinstance(event, dict):
            raise InvalidHistory(f"events[{index}] must be an object")
        seq = _uint(event.get("seq"), f"events[{index}].seq")
        if seq in seen_sequences:
            violation("DUPLICATE_SEQUENCE", seq)
        seen_sequences.add(seq)
        if seq != index:
            violation("NON_CONTIGUOUS_SEQUENCE", seq, expected=index)
        kind = _nonempty_string(event.get("kind"), f"events[{index}].kind")
        if kind not in {"invoke", "fence_advanced", "effect_committed", "receipt_committed", "response"}:
            raise InvalidHistory(f"events[{index}].kind is unsupported: {kind!r}")

        if kind == "fence_advanced":
            new_fence = _uint(event.get("fencing_token"), f"events[{index}].fencing_token")
            if new_fence <= active_fence:
                violation("FENCE_NOT_MONOTONIC", seq, previous=active_fence, observed=new_fence)
            else:
                active_fence = new_fence
            continue

        attempt_id = _nonempty_string(event.get("attempt_id"), f"events[{index}].attempt_id")
        if kind == "invoke":
            if attempt_id in attempts:
                violation("DUPLICATE_INVOCATION_ID", seq, attempt_id=attempt_id)
                continue
            key = _nonempty_string(event.get("idempotency_key"), f"events[{index}].idempotency_key")
            intent = _intent(event, f"events[{index}]")
            fence = _uint(event.get("fencing_token"), f"events[{index}].fencing_token")
            previous_intent = intents_by_key.get(key)
            if previous_intent is not None and previous_intent != intent:
                violation("KEY_REUSED_FOR_DIFFERENT_INTENT", seq, idempotency_key=key)
            else:
                intents_by_key[key] = intent
            if fence != active_fence:
                violation("INVOCATION_USED_STALE_OR_UNISSUED_FENCE", seq, observed=fence, active=active_fence)
            attempts[attempt_id] = {
                "attempt_id": attempt_id,
                "idempotency_key": key,
                "intent": intent,
                "fencing_token": fence,
                "invoke_seq": seq,
            }
            continue

        if attempt_id not in attempts:
            violation("EVENT_WITHOUT_PRIOR_INVOCATION", seq, attempt_id=attempt_id, kind=kind)
            continue
        attempt = attempts[attempt_id]

        if kind == "effect_committed":
            effect_id = _nonempty_string(event.get("effect_id"), f"events[{index}].effect_id")
            observed_intent = _intent(event, f"events[{index}]")
            observed_sink = _digest(event.get("sink_identity_digest"), f"events[{index}].sink_identity_digest")
            fence = _uint(event.get("fencing_token"), f"events[{index}].fencing_token")
            if effect_id in effects_by_id:
                violation("DUPLICATE_EFFECT_ID", seq, effect_id=effect_id)
                continue
            if observed_intent != attempt["intent"]:
                violation("EFFECT_INTENT_MISMATCH", seq, attempt_id=attempt_id, effect_id=effect_id)
            if observed_sink != sink_digest:
                violation("EFFECT_WRONG_SINK", seq, effect_id=effect_id)
            if fence != attempt["fencing_token"]:
                violation("EFFECT_FENCE_DIFFERS_FROM_INVOCATION", seq, effect_id=effect_id)
            if fence != active_fence:
                violation("STALE_WORKER_EFFECT", seq, effect_id=effect_id, observed=fence, active=active_fence)
            effect = {
                "effect_id": effect_id,
                "attempt_id": attempt_id,
                "idempotency_key": attempt["idempotency_key"],
                "intent": observed_intent,
                "sink_identity_digest": observed_sink,
                "fencing_token": fence,
                "seq": seq,
            }
            effects_by_id[effect_id] = effect
            effects_by_key[attempt["idempotency_key"]].append(effect)
            continue

        if kind == "receipt_committed":
            receipt_id = _nonempty_string(event.get("receipt_id"), f"events[{index}].receipt_id")
            effect_id = _nonempty_string(event.get("effect_id"), f"events[{index}].effect_id")
            observed_intent = _intent(event, f"events[{index}]")
            observed_sink = _digest(event.get("sink_identity_digest"), f"events[{index}].sink_identity_digest")
            if receipt_id in receipts_by_id:
                violation("DUPLICATE_RECEIPT_ID", seq, receipt_id=receipt_id)
                continue
            effect = effects_by_id.get(effect_id)
            if effect is None:
                violation("RECEIPT_WITHOUT_PRIOR_EFFECT", seq, receipt_id=receipt_id, effect_id=effect_id)
            elif effect["attempt_id"] != attempt_id:
                violation("RECEIPT_COMMIT_ATTRIBUTED_TO_WRONG_ATTEMPT", seq, receipt_id=receipt_id)
            if observed_intent != attempt["intent"]:
                violation("RECEIPT_INTENT_MISMATCH", seq, receipt_id=receipt_id)
            if observed_sink != sink_digest:
                violation("RECEIPT_WRONG_SINK", seq, receipt_id=receipt_id)
            if effect is not None and observed_intent != effect["intent"]:
                violation("RECEIPT_DOES_NOT_BIND_EFFECT", seq, receipt_id=receipt_id, effect_id=effect_id)
            receipts_by_id[receipt_id] = {
                "receipt_id": receipt_id,
                "effect_id": effect_id,
                "attempt_id": attempt_id,
                "idempotency_key": attempt["idempotency_key"],
                "intent": observed_intent,
                "sink_identity_digest": observed_sink,
                "seq": seq,
            }
            continue

        if kind == "response":
            outcome = _nonempty_string(event.get("outcome"), f"events[{index}].outcome")
            if outcome not in {"committed", "unknown", "rejected"}:
                raise InvalidHistory(f"events[{index}].outcome must be committed, unknown, or rejected")
            receipt_ref = event.get("receipt_id")
            if receipt_ref is not None and (not isinstance(receipt_ref, str) or not receipt_ref):
                raise InvalidHistory(f"events[{index}].receipt_id must be a non-empty string or null")
            response = {"outcome": outcome, "receipt_id": receipt_ref, "seq": seq}
            responses[attempt_id].append(response)
            if len(responses[attempt_id]) > 1:
                violation("MULTIPLE_TERMINAL_RESPONSES", seq, attempt_id=attempt_id)
            if outcome == "committed":
                if receipt_ref is None or receipt_ref not in receipts_by_id:
                    violation("COMMITTED_RESPONSE_WITHOUT_PRIOR_RECEIPT", seq, attempt_id=attempt_id)
                else:
                    receipt = receipts_by_id[receipt_ref]
                    if receipt["idempotency_key"] != attempt["idempotency_key"] or receipt["intent"] != attempt["intent"]:
                        violation("RESPONSE_RECEIPT_INTENT_MISMATCH", seq, attempt_id=attempt_id, receipt_id=receipt_ref)
            elif outcome == "rejected":
                if any(e["attempt_id"] == attempt_id for e in effects_by_id.values()):
                    violation("REJECTED_ATTEMPT_HAS_EFFECT", seq, attempt_id=attempt_id)
                if receipt_ref is not None:
                    violation("REJECTED_RESPONSE_HAS_RECEIPT", seq, attempt_id=attempt_id)
            elif outcome == "unknown" and receipt_ref is not None:
                violation("UNKNOWN_RESPONSE_CLAIMS_RECEIPT", seq, attempt_id=attempt_id)
            continue

    for key, effects in effects_by_key.items():
        distinct = {effect["effect_id"] for effect in effects}
        if len(distinct) > 1:
            violation("DUPLICATE_LOGICAL_EFFECT", max(e["seq"] for e in effects), idempotency_key=key, effect_ids=sorted(distinct))

    receipt_effects = {receipt["effect_id"] for receipt in receipts_by_id.values()}
    for effect_id, effect in effects_by_id.items():
        if effect_id not in receipt_effects:
            violation("EFFECT_WITHOUT_RECEIPT", effect["seq"], effect_id=effect_id)
    for receipt_id, receipt in receipts_by_id.items():
        effect = effects_by_id.get(receipt["effect_id"])
        if effect is None:
            continue
        if receipt["intent"] != effect["intent"] or receipt["idempotency_key"] != effect["idempotency_key"]:
            violation("RECEIPT_EFFECT_BINDING_MISMATCH", receipt["seq"], receipt_id=receipt_id, effect_id=effect["effect_id"])

    attempt_ids = sorted(attempts)
    unresolved = [aid for aid in attempt_ids if not responses.get(aid) or responses[aid][-1]["outcome"] == "unknown"]
    terminal = sum(bool(responses.get(aid)) and responses[aid][-1]["outcome"] in {"committed", "rejected"} for aid in attempt_ids)
    effect_keys = {effect["idempotency_key"] for effect in effects_by_id.values()}
    blocked = sum(bool(responses.get(aid)) and responses[aid][-1]["outcome"] == "rejected" for aid in attempt_ids)
    unknown = sum(bool(responses.get(aid)) and responses[aid][-1]["outcome"] == "unknown" for aid in attempt_ids)
    return {
        "schema": REPORT_SCHEMA,
        "oracle_version": 1,
        "safety_status": "PASS" if not violations else "FAIL",
        "violation_count": len(violations),
        "violations": violations,
        "liveness_observations": {
            "claim": "NOT_EVALUATED",
            "attempt_count": len(attempt_ids),
            "terminal_response_count": terminal,
            "rejected_response_count": blocked,
            "unknown_response_count": unknown,
            "unresolved_or_unknown_attempt_ids": unresolved,
            "logical_intent_count": len(intents_by_key),
            "effect_count": len(effects_by_id),
            "effectful_intent_count": len(effect_keys),
            "receipt_count": len(receipts_by_id),
            "note": "Safety PASS does not establish progress. Supply workload-specific completion bounds to assess liveness; universal rejection is not success.",
        },
    }


def _sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _add_provenance(report: dict[str, Any]) -> dict[str, Any]:
    root = Path(__file__).resolve().parent
    report["harness_commit_sha"] = os.environ.get("GITHUB_SHA")
    report["source_head_sha"] = os.environ.get("SRQ_HISTORY_ORACLE_PR_HEAD_SHA") or os.environ.get("GITHUB_SHA")
    report["source_sha256"] = {
        name: _sha256_file(root / name)
        for name in ("history_oracle.py", "test_history_oracle.py", "history-protocol-v1.json")
    }
    return report


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("history", type=Path, help="JSON history document")
    parser.add_argument("--output", type=Path, help="optional JSON report path")
    args = parser.parse_args(argv)
    try:
        history = json.loads(args.history.read_text(encoding="utf-8"))
        report = check_history(history)
    except (OSError, json.JSONDecodeError, InvalidHistory) as exc:
        report = {"schema": REPORT_SCHEMA, "safety_status": "INVALID_HISTORY", "error": str(exc)}
        report = _add_provenance(report)
        rendered = json.dumps(report, indent=2, sort_keys=True) + "\n"
        if args.output:
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(rendered, encoding="utf-8")
        print(rendered, end="")
        return 2
    report = _add_provenance(report)
    rendered = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered, encoding="utf-8")
    print(rendered, end="")
    return 0 if report["safety_status"] == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
