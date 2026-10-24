#!/usr/bin/env python3
"""Exhaustive event-reference target matrix for the synthetic SRQ causal contract."""
from __future__ import annotations

import argparse
import copy
import hashlib
import json
import os
import sys
import time
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
PARENT = HERE.parent
sys.path.insert(0, str(HERE))
import causal_integrity as ci

PROTOCOL_PATH = HERE / "reference-matrix-protocol-v1.json"
PROTOCOL_RAW_SHA256 = "65e92596be506b0681fd74bca62522a2f406e174296452f6087c167aa84a87f7"
EXPECTED_EDGES = [
    {"mode": "success", "source_kind": "applied", "field": "authorization_event_id", "target_kind": "authorized"},
    {"mode": "success", "source_kind": "applied", "field": "snapshot_event_id", "target_kind": "snapshotted"},
    {"mode": "success", "source_kind": "verified", "field": "application_event_id", "target_kind": "applied"},
    {"mode": "success", "source_kind": "outcome", "field": "verification_event_id", "target_kind": "verified"},
    {"mode": "success", "source_kind": "promoted", "field": "outcome_event_id", "target_kind": "outcome"},
    {"mode": "success", "source_kind": "promoted", "field": "verification_event_id", "target_kind": "verified"},
    {"mode": "failed", "source_kind": "applied", "field": "authorization_event_id", "target_kind": "authorized"},
    {"mode": "failed", "source_kind": "applied", "field": "snapshot_event_id", "target_kind": "snapshotted"},
    {"mode": "failed", "source_kind": "failed", "field": "application_event_id", "target_kind": "applied"},
    {"mode": "recovered", "source_kind": "applied", "field": "authorization_event_id", "target_kind": "authorized"},
    {"mode": "recovered", "source_kind": "applied", "field": "snapshot_event_id", "target_kind": "snapshotted"},
    {"mode": "recovered", "source_kind": "failed", "field": "application_event_id", "target_kind": "applied"},
    {"mode": "recovered", "source_kind": "recovered", "field": "failed_event_id", "target_kind": "failed"},
    {"mode": "recovered", "source_kind": "recovered", "field": "snapshot_event_id", "target_kind": "snapshotted"},
]
EXPECTED_METRICS = {
    "expected_state_count": 143,
    "expected_edge_count": 14,
    "expected_accept_count": 14,
    "expected_reject_count": 129,
    "false_accept_count": 0,
    "false_reject_count": 0,
    "unexpected_error_count": 0,
    "false_release_pass_count": 0,
}


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_protocol() -> dict[str, Any]:
    raw = PROTOCOL_PATH.read_bytes()
    digest = hashlib.sha256(raw).hexdigest()
    if digest != PROTOCOL_RAW_SHA256:
        raise ValueError(f"reference matrix protocol digest mismatch: expected {PROTOCOL_RAW_SHA256}, got {digest}")
    protocol = json.loads(raw.decode("utf-8"), object_pairs_hook=_unique_object)
    validate_protocol(protocol)
    return protocol


def validate_protocol(protocol: Any) -> None:
    if not isinstance(protocol, dict):
        raise ValueError("protocol must be an object")
    if protocol.get("schema") != "srq-causal-reference-matrix-protocol-v1":
        raise ValueError("protocol schema mismatch")
    if protocol.get("protocol_id") != "srq-causal-reference-matrix-v1" or protocol.get("version") != 1:
        raise ValueError("protocol identity/version mismatch")
    if protocol.get("edges") != EXPECTED_EDGES:
        raise ValueError("reference-edge oracle differs from the frozen v1 inventory")
    if protocol.get("target_metrics") != EXPECTED_METRICS:
        raise ValueError("reference-matrix target metrics drift")
    if protocol.get("release_invariant") != {
        "qualification_status": "BLOCKED", "claim_ceiling": "none", "release_pass_allowed": False
    }:
        raise ValueError("release-blocked invariant drift")
    if protocol.get("candidate_domain", {}).get("expected_rule") != (
        "Accept exactly when the reference equals the unique event ID of the declared target_kind and that target precedes the source event."
    ):
        raise ValueError("candidate-domain oracle rule drift")


def _set_reference(candidate: dict[str, Any], source_kind: str, field: str, value: Any) -> None:
    matches = [event for event in candidate["events"] if event.get("kind") == source_kind]
    if len(matches) != 1:
        raise ValueError(f"expected exactly one source event of kind {source_kind}")
    matches[0]["payload"][field] = value


def run_matrix(protocol: dict[str, Any] | None = None, commit_sha: str | None = None) -> dict[str, Any]:
    protocol = protocol or load_protocol()
    validate_protocol(protocol)
    causal_protocol = ci.load_protocol()
    results = []
    for edge in protocol["edges"]:
        baseline = ci.make_candidate(edge["mode"])
        event_ids = [event["event_id"] for event in baseline["events"]]
        # Keep order deterministic and include same-chain, wrong-chain, dangling, empty and null values.
        candidates = event_ids + ["missing-event", "", None]
        target_events = [event for event in baseline["events"] if event.get("kind") == edge["target_kind"]]
        source_events = [event for event in baseline["events"] if event.get("kind") == edge["source_kind"]]
        if len(target_events) != 1 or len(source_events) != 1:
            raise ValueError(f"edge target/source kind is not unique: {edge}")
        target_event = target_events[0]
        source_event = source_events[0]
        if target_event.get("sequence", -1) >= source_event.get("sequence", -1):
            raise ValueError(f"reference target is not a predecessor: {edge}")
        target_id = target_event["event_id"]
        for candidate_index, value in enumerate(candidates):
            candidate = copy.deepcopy(baseline)
            _set_reference(candidate, edge["source_kind"], edge["field"], value)
            ci.seal(candidate)  # Recompute the unkeyed chain so wrong refs test semantics, not stale hashes.
            expected = "ACCEPT" if value == target_id else "REJECT"
            started = time.perf_counter_ns()
            try:
                errors = ci.validate_candidate(candidate, causal_protocol)
                observed = "ACCEPT" if not errors else "REJECT"
                reason = errors
            except Exception as exc:
                observed = "ERROR"
                reason = [f"{type(exc).__name__}: {exc}"]
            elapsed = time.perf_counter_ns() - started
            results.append({
                "case_id": f'{edge["mode"]}:{edge["source_kind"]}.{edge["field"]}:candidate-{candidate_index}',
                "mode": edge["mode"],
                "source_kind": edge["source_kind"],
                "reference_field": edge["field"],
                "expected_target_kind": edge["target_kind"],
                "candidate_reference": value,
                "expected_outcome": expected,
                "observed_outcome": observed,
                "candidate_sha256": ci.sha256(candidate),
                "errors": reason,
                "elapsed_ns": elapsed,
                "qualification_status": "BLOCKED",
                "claim_ceiling": "none",
            })
    false_accept = sum(r["expected_outcome"] == "REJECT" and r["observed_outcome"] == "ACCEPT" for r in results)
    false_reject = sum(r["expected_outcome"] == "ACCEPT" and r["observed_outcome"] == "REJECT" for r in results)
    unexpected_error = sum(r["observed_outcome"] == "ERROR" for r in results)
    false_release_pass = sum(
        r["observed_outcome"] == "ACCEPT" and
        (r["qualification_status"] != "BLOCKED" or r["claim_ceiling"] != "none")
        for r in results
    )
    accepted = sum(r["observed_outcome"] == "ACCEPT" for r in results)
    rejected = sum(r["observed_outcome"] == "REJECT" for r in results)
    targets = protocol["target_metrics"]
    observed_state_count = len(results)
    status = "PASS" if (
        observed_state_count == targets["expected_state_count"]
        and accepted == targets["expected_accept_count"]
        and rejected == targets["expected_reject_count"]
        and false_accept == targets["false_accept_count"]
        and false_reject == targets["false_reject_count"]
        and unexpected_error == targets["unexpected_error_count"]
        and false_release_pass == targets["false_release_pass_count"]
    ) else "FAIL"
    return {
        "schema": "srq-causal-reference-matrix-report-v1",
        "protocol_id": protocol["protocol_id"],
        "protocol_raw_sha256": hashlib.sha256(PROTOCOL_PATH.read_bytes()).hexdigest(),
        "oracle_script_raw_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        "parent_checker_raw_sha256": hashlib.sha256((HERE / "causal_integrity.py").read_bytes()).hexdigest(),
        "harness_commit_sha": commit_sha or os.environ.get("GITHUB_SHA", "UNPINNED_LOCAL"),
        "status": status,
        "qualification_status": "BLOCKED",
        "claim_ceiling": "none",
        "expected_state_count": targets["expected_state_count"],
        "observed_state_count": observed_state_count,
        "expected_edge_count": targets["expected_edge_count"],
        "observed_edge_count": len(protocol["edges"]),
        "accepted_count": accepted,
        "rejected_count": rejected,
        "false_accept_count": false_accept,
        "false_reject_count": false_reject,
        "unexpected_error_count": unexpected_error,
        "false_release_pass_count": false_release_pass,
        "cases": results,
        "limitations": protocol["limitations"],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, help="write the machine-readable report to this path")
    parser.add_argument("--commit-sha", help="exact checkout commit to bind into the report")
    args = parser.parse_args()
    try:
        report = run_matrix(commit_sha=args.commit_sha)
    except Exception as exc:
        print(f"BLOCKED: reference matrix protocol/runtime error: {type(exc).__name__}: {exc}", file=sys.stderr)
        return 2
    encoded = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(encoded, encoding="utf-8")
    else:
        print(encoded, end="")
    print(
        f'{report["status"]}: states={report["observed_state_count"]} accepted={report["accepted_count"]} '
        f'rejected={report["rejected_count"]} false-accept={report["false_accept_count"]} '
        f'false-reject={report["false_reject_count"]} errors={report["unexpected_error_count"]}',
        file=sys.stderr,
    )
    return 0 if report["status"] == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
