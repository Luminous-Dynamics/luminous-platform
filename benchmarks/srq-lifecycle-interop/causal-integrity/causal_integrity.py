#!/usr/bin/env python3
"""Synthetic structural causal-binding benchmark; not a production verifier."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import sys
import time
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
PROTOCOL_PATH = HERE / "causal-integrity-protocol-v1.json"
MUTATIONS_PATH = HERE / "causal-integrity-mutations-v1.json"
HEX64 = re.compile(r"^[0-9a-f]{64}$")
EVENT_KEYS = {"event_id", "sequence", "kind", "transaction_id", "target_machine_digest", "plan_digest", "artifact_digest", "previous_event_sha256", "payload", "payload_sha256", "event_sha256"}
ROOT_KEYS = {"schema", "transaction", "events"}
TRANSACTION_KEYS = {"transaction_id", "target_machine_digest", "plan_digest", "artifact_digest", "policy_digest"}


def canonical_bytes(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False, allow_nan=False).encode("utf-8")


def sha256(value: Any) -> str:
    return hashlib.sha256(canonical_bytes(value)).hexdigest()


def event_digest(event: dict[str, Any]) -> str:
    return sha256({k: v for k, v in event.items() if k != "event_sha256"})


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_protocol() -> dict[str, Any]:
    with PROTOCOL_PATH.open("r", encoding="utf-8") as stream:
        protocol = json.load(stream, object_pairs_hook=_unique_object)
    validate_protocol(protocol)
    return protocol


def validate_protocol(protocol: Any) -> None:
    if not isinstance(protocol, dict):
        raise ValueError("protocol must be an object")
    if protocol.get("schema") != "srq-transaction-causal-binding-protocol-v1":
        raise ValueError("unexpected protocol schema")
    if protocol.get("protocol_id") != "srq-transaction-causal-binding-v1" or protocol.get("version") != 1:
        raise ValueError("unexpected protocol identity/version")
    if protocol.get("identity_fields") != ["transaction_id", "target_machine_digest", "plan_digest", "artifact_digest", "policy_digest"]:
        raise ValueError("identity field inventory drift")
    expected_sequences = [
        ["planned", "validated", "authorized", "snapshotted", "applied", "verified", "outcome", "promoted"],
        ["planned", "validated", "authorized", "snapshotted", "applied", "failed"],
        ["planned", "validated", "authorized", "snapshotted", "applied", "failed", "recovered"],
    ]
    if protocol.get("allowed_event_sequences") != expected_sequences:
        raise ValueError("allowed event sequence protocol drift")
    if len(protocol.get("attack_cases", [])) != protocol.get("target_metrics", {}).get("expected_attack_case_count"):
        raise ValueError("attack inventory/count mismatch")
    if len(protocol.get("positive_controls", [])) != protocol.get("target_metrics", {}).get("expected_positive_control_count"):
        raise ValueError("positive-control inventory/count mismatch")
    generated = build_corpus(protocol)
    generated_attacks = [case["case_id"] for case in generated if case["expected"] == "REJECT"]
    generated_controls = [case["case_id"] for case in generated if case["expected"] == "ACCEPT"]
    if protocol.get("attack_cases") != generated_attacks:
        raise ValueError("frozen attack IDs/order differ from executable corpus")
    if protocol.get("positive_controls") != generated_controls:
        raise ValueError("frozen positive-control IDs/order differ from executable corpus")
    targets = protocol.get("target_metrics", {})
    if any(targets.get(name) != 0 for name in ("false_accept_count", "false_reject_count", "unexpected_error_count", "false_release_pass_count")):
        raise ValueError("zero-error target metrics drift")
    if protocol.get("release_invariant") != {"release_status": "BLOCKED", "claim_ceiling": "none", "release_pass_allowed": False}:
        raise ValueError("release invariant drift")


def _err(errors: list[str], message: str) -> None:
    if message not in errors:
        errors.append(message)


def _references(reference: Any, expected_kind: str, event_by_kind: dict[str, dict[str, Any]], by_id: dict[str, dict[str, Any]], current: dict[str, Any]) -> bool:
    target = by_id.get(reference) if isinstance(reference, str) else None
    return bool(target and target.get("kind") == expected_kind and target.get("sequence", -1) < current.get("sequence", -1) and event_by_kind.get(expected_kind) is target)


def validate_candidate(candidate: Any, protocol: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    if not isinstance(candidate, dict) or set(candidate) != ROOT_KEYS:
        return ["candidate root must contain exactly schema, transaction, events"]
    if candidate.get("schema") != "srq-causal-transaction-candidate-v1":
        _err(errors, "candidate schema mismatch")
    tx = candidate.get("transaction")
    if not isinstance(tx, dict) or set(tx) != TRANSACTION_KEYS:
        return ["transaction must contain exactly the four identity fields"]
    for field in protocol["identity_fields"]:
        value = tx.get(field)
        if not isinstance(value, str) or not HEX64.fullmatch(value):
            _err(errors, f"invalid root identity digest: {field}")
    events = candidate.get("events")
    if not isinstance(events, list) or not events:
        return errors + ["events must be a non-empty array"]
    kinds = [e.get("kind") if isinstance(e, dict) else None for e in events]
    if kinds not in protocol["allowed_event_sequences"]:
        _err(errors, "event kinds do not match any frozen v1 sequence")
    event_ids = [e.get("event_id") for e in events if isinstance(e, dict)]
    if any(not isinstance(event_id, str) for event_id in event_ids):
        _err(errors, "every event ID must be a string")
    elif len(set(event_ids)) != len(events):
        _err(errors, "event IDs must be unique")

    by_id: dict[str, dict[str, Any]] = {}
    previous_digest = None
    for index, event in enumerate(events):
        if not isinstance(event, dict) or set(event) != EVENT_KEYS:
            _err(errors, f"event {index} does not match the closed event envelope")
            continue
        event_id = event.get("event_id")
        if isinstance(event_id, str):
            by_id.setdefault(event_id, event)
        sequence = event.get("sequence")
        if not isinstance(sequence, int) or isinstance(sequence, bool) or sequence != index:
            _err(errors, f"event {index} sequence must equal its zero-based position")
        if not isinstance(event_id, str) or not event_id:
            _err(errors, f"event {index} has an empty event_id")
        if event.get("transaction_id") != tx.get("transaction_id"):
            _err(errors, f"event {index} transaction identity mismatch")
        if event.get("target_machine_digest") != tx.get("target_machine_digest"):
            _err(errors, f"event {index} target identity mismatch")
        if event.get("plan_digest") != tx.get("plan_digest"):
            _err(errors, f"event {index} plan identity mismatch")
        if event.get("artifact_digest") != tx.get("artifact_digest"):
            _err(errors, f"event {index} artifact identity mismatch")
        if event.get("previous_event_sha256") != previous_digest:
            _err(errors, f"event {index} previous-event chain mismatch")
        payload = event.get("payload")
        if not isinstance(payload, dict):
            _err(errors, f"event {index} payload must be an object")
            payload = {}
        if event.get("payload_sha256") != sha256(payload):
            _err(errors, f"event {index} payload digest mismatch")
        if event.get("event_sha256") != event_digest(event):
            _err(errors, f"event {index} envelope digest mismatch")
        previous_digest = event.get("event_sha256") if isinstance(event.get("event_sha256"), str) else None

    event_by_kind = {e.get("kind"): e for e in events if isinstance(e, dict) and isinstance(e.get("kind"), str)}
    for index, event in enumerate(events):
        if not isinstance(event, dict) or not isinstance(event.get("kind"), str):
            continue
        kind = event["kind"]
        payload = event.get("payload") if isinstance(event.get("payload"), dict) else {}
        required_fields = protocol["required_payload_fields"].get(kind)
        if required_fields is None:
            _err(errors, f"event {index} has unknown kind")
            continue
        if set(payload) != set(required_fields):
            _err(errors, f"{kind} payload fields differ from frozen protocol")
        if kind == "planned" and payload.get("intent") != "apply":
            _err(errors, "planned intent must be apply")
        elif kind == "validated":
            if payload.get("status") != "pass" or payload.get("validated_plan_digest") != tx.get("plan_digest") or payload.get("validated_target_machine_digest") != tx.get("target_machine_digest") or payload.get("validated_artifact_digest") != tx.get("artifact_digest"):
                _err(errors, "validation does not bind the root plan and target")
        elif kind == "authorized":
            if payload.get("authorized_plan_digest") != tx.get("plan_digest") or payload.get("authorized_target_machine_digest") != tx.get("target_machine_digest") or payload.get("authorized_artifact_digest") != tx.get("artifact_digest") or payload.get("policy_digest") != tx.get("policy_digest") or payload.get("decision") != "allow":
                _err(errors, "authorization does not bind the root plan, target, and policy")
            for field in ("nonce", "signer_key_id"):
                if not isinstance(payload.get(field), str) or not payload[field]:
                    _err(errors, f"authorization {field} must be non-empty")
            if not isinstance(payload.get("signature_sha256"), str) or not HEX64.fullmatch(payload["signature_sha256"]):
                _err(errors, "authorization signature digest is malformed")
        elif kind == "snapshotted":
            if payload.get("target_machine_digest") != tx.get("target_machine_digest"):
                _err(errors, "snapshot target does not match root target")
            if not isinstance(payload.get("pre_state_digest"), str) or not HEX64.fullmatch(payload["pre_state_digest"]):
                _err(errors, "snapshot pre-state digest is malformed")
            if not isinstance(payload.get("snapshot_id"), str) or not payload["snapshot_id"]:
                _err(errors, "snapshot ID must be non-empty")
        elif kind == "applied":
            if not _references(payload.get("authorization_event_id"), "authorized", event_by_kind, by_id, event):
                _err(errors, "application authorization reference is invalid")
            if not _references(payload.get("snapshot_event_id"), "snapshotted", event_by_kind, by_id, event):
                _err(errors, "application snapshot reference is invalid")
            for field in ("applied_artifact_digest", "expected_post_state_digest"):
                if not isinstance(payload.get(field), str) or not HEX64.fullmatch(payload[field]):
                    _err(errors, f"application {field} is malformed")
            if payload.get("applied_artifact_digest") != tx.get("artifact_digest"):
                _err(errors, "applied artifact differs from the transaction authorized artifact")
            generation = payload.get("generation_after")
            if not isinstance(generation, int) or isinstance(generation, bool) or generation < 1:
                _err(errors, "application generation_after must be a positive integer")
        elif kind == "verified":
            if not _references(payload.get("application_event_id"), "applied", event_by_kind, by_id, event):
                _err(errors, "verification application reference is invalid")
            applied = event_by_kind.get("applied", {}).get("payload", {})
            if payload.get("observed_state_digest") != applied.get("expected_post_state_digest"):
                _err(errors, "verification observed state differs from expected post-state")
            if payload.get("observed_generation") != applied.get("generation_after"):
                _err(errors, "verification observed generation differs from applied generation")
            if payload.get("result") != "pass":
                _err(errors, "successful lifecycle path requires passing verification")
            if not isinstance(payload.get("verifier_id"), str) or not payload["verifier_id"]:
                _err(errors, "verification verifier_id must be non-empty")
        elif kind == "outcome":
            if not _references(payload.get("verification_event_id"), "verified", event_by_kind, by_id, event):
                _err(errors, "outcome verification reference is invalid")
            if payload.get("status") != "succeeded":
                _err(errors, "success-path outcome status must be succeeded")
        elif kind == "promoted":
            if not _references(payload.get("outcome_event_id"), "outcome", event_by_kind, by_id, event):
                _err(errors, "promotion outcome reference is invalid")
            if not _references(payload.get("verification_event_id"), "verified", event_by_kind, by_id, event):
                _err(errors, "promotion verification reference is invalid")
            if payload.get("target_state_digest") != event_by_kind.get("verified", {}).get("payload", {}).get("observed_state_digest"):
                _err(errors, "promotion target-state digest differs from verified state")
        elif kind == "failed":
            if not isinstance(payload.get("reason_code"), str) or not payload["reason_code"]:
                _err(errors, "failure reason_code must be non-empty")
            if not _references(payload.get("application_event_id"), "applied", event_by_kind, by_id, event):
                _err(errors, "failure application reference is invalid")
        elif kind == "recovered":
            if not _references(payload.get("failed_event_id"), "failed", event_by_kind, by_id, event):
                _err(errors, "recovery failed-event reference is invalid")
            if not _references(payload.get("snapshot_event_id"), "snapshotted", event_by_kind, by_id, event):
                _err(errors, "recovery snapshot reference is invalid")
            snapshot_digest = event_by_kind.get("snapshotted", {}).get("payload", {}).get("pre_state_digest")
            if payload.get("restored_state_digest") != snapshot_digest:
                _err(errors, "recovered state digest differs from snapshot pre-state")
            if not isinstance(payload.get("recovery_observation_sha256"), str) or not HEX64.fullmatch(payload["recovery_observation_sha256"]):
                _err(errors, "recovery observation digest is malformed")
    return errors


def _event(event_id: str, sequence: int, kind: str, tx: dict[str, str], payload: dict[str, Any]) -> dict[str, Any]:
    return {
        "event_id": event_id, "sequence": sequence, "kind": kind,
        "transaction_id": tx["transaction_id"], "target_machine_digest": tx["target_machine_digest"],
        "plan_digest": tx["plan_digest"], "artifact_digest": tx["artifact_digest"], "previous_event_sha256": None,
        "payload": payload, "payload_sha256": "", "event_sha256": ""
    }


def seal(candidate: dict[str, Any]) -> dict[str, Any]:
    """Recompute unkeyed fixture hashes. This does not authenticate a candidate."""
    previous = None
    for event in candidate["events"]:
        event["previous_event_sha256"] = previous
        event["payload_sha256"] = sha256(event["payload"])
        event["event_sha256"] = event_digest(event)
        previous = event["event_sha256"]
    return candidate


def make_candidate(mode: str = "success") -> dict[str, Any]:
    tx = {"transaction_id": "a" * 64, "target_machine_digest": "b" * 64, "plan_digest": "c" * 64, "artifact_digest": "8" * 64, "policy_digest": "d" * 64}
    ids = {"planned": "evt-plan", "validated": "evt-validated", "authorized": "evt-authorized", "snapshotted": "evt-snapshot", "applied": "evt-applied", "verified": "evt-verified", "outcome": "evt-outcome", "promoted": "evt-promoted", "failed": "evt-failed", "recovered": "evt-recovered"}
    payloads: list[tuple[str, dict[str, Any]]] = [
        ("planned", {"intent": "apply"}),
        ("validated", {"status": "pass", "validated_plan_digest": tx["plan_digest"], "validated_target_machine_digest": tx["target_machine_digest"], "validated_artifact_digest": tx["artifact_digest"]}),
        ("authorized", {"authorized_plan_digest": tx["plan_digest"], "authorized_target_machine_digest": tx["target_machine_digest"], "authorized_artifact_digest": tx["artifact_digest"], "policy_digest": tx["policy_digest"], "decision": "allow", "nonce": "synthetic-nonce-01", "signer_key_id": "synthetic-signer", "signature_sha256": "e" * 64}),
        ("snapshotted", {"target_machine_digest": tx["target_machine_digest"], "pre_state_digest": "f" * 64, "snapshot_id": "snapshot-001"}),
        ("applied", {"authorization_event_id": ids["authorized"], "snapshot_event_id": ids["snapshotted"], "applied_artifact_digest": tx["artifact_digest"], "expected_post_state_digest": "2" * 64, "generation_after": 7})
    ]
    if mode == "success":
        payloads.extend([
            ("verified", {"application_event_id": ids["applied"], "observed_state_digest": "2" * 64, "observed_generation": 7, "result": "pass", "verifier_id": "synthetic-independent-verifier"}),
            ("outcome", {"verification_event_id": ids["verified"], "status": "succeeded"}),
            ("promoted", {"outcome_event_id": ids["outcome"], "verification_event_id": ids["verified"], "target_state_digest": "2" * 64})
        ])
    elif mode in ("failed", "recovered"):
        payloads.append(("failed", {"reason_code": "synthetic-fault", "application_event_id": ids["applied"]}))
        if mode == "recovered":
            payloads.append(("recovered", {"failed_event_id": ids["failed"], "snapshot_event_id": ids["snapshotted"], "restored_state_digest": "f" * 64, "recovery_observation_sha256": "3" * 64}))
    else:
        raise ValueError(f"unknown fixture mode: {mode}")
    events = [_event(ids[kind], i, kind, tx, payload) for i, (kind, payload) in enumerate(payloads)]
    return seal({"schema": "srq-causal-transaction-candidate-v1", "transaction": tx, "events": events})


def load_mutation_manifest(protocol: dict[str, Any]) -> dict[str, Any]:
    raw = MUTATIONS_PATH.read_bytes()
    expected_digest = protocol.get("mutation_manifest_sha256")
    observed_digest = hashlib.sha256(raw).hexdigest()
    if not isinstance(expected_digest, str) or expected_digest != observed_digest:
        raise ValueError("mutation manifest digest differs from the pinned protocol digest")
    manifest = json.loads(raw.decode("utf-8"), object_pairs_hook=_unique_object)
    if not isinstance(manifest, dict) or set(manifest) != {"schema", "protocol_id", "version", "positive_controls", "attacks"}:
        raise ValueError("mutation manifest root differs from the closed schema")
    if manifest.get("schema") != "srq-transaction-causal-binding-mutations-v1" or manifest.get("protocol_id") != protocol.get("protocol_id") or manifest.get("version") != 1:
        raise ValueError("mutation manifest identity/version mismatch")
    controls = manifest.get("positive_controls")
    attacks = manifest.get("attacks")
    if not isinstance(controls, list) or not isinstance(attacks, list):
        raise ValueError("mutation manifest controls and attacks must be arrays")
    if [case.get("case_id") for case in controls if isinstance(case, dict)] != protocol.get("positive_controls"):
        raise ValueError("positive-control IDs/order differ from the pinned protocol")
    if [case.get("case_id") for case in attacks if isinstance(case, dict)] != protocol.get("attack_cases"):
        raise ValueError("attack IDs/order differ from the pinned protocol")
    for case in controls:
        if not isinstance(case, dict) or set(case) != {"case_id", "mode", "expected"} or case.get("expected") != "ACCEPT" or case.get("mode") not in ("success", "failed", "recovered"):
            raise ValueError("invalid positive-control declaration")
    for case in attacks:
        if not isinstance(case, dict) or set(case) != {"case_id", "mode", "expected", "reseal", "mutations"}:
            raise ValueError("invalid attack declaration shape")
        if case.get("expected") != "REJECT" or case.get("mode") not in ("success", "failed", "recovered") or not isinstance(case.get("reseal"), bool):
            raise ValueError("invalid attack disposition, mode, or reseal flag")
        mutations = case.get("mutations")
        if not isinstance(mutations, list) or not mutations:
            raise ValueError("every attack must contain at least one mutation")
        for mutation in mutations:
            if not isinstance(mutation, dict) or mutation.get("op") not in ("set", "swap_events"):
                raise ValueError("unknown mutation operation")
            if mutation["op"] == "set":
                if set(mutation) != {"op", "path", "value"} or not isinstance(mutation.get("path"), list) or not mutation["path"]:
                    raise ValueError("set mutation requires a non-empty path and value")
                if any(not isinstance(token, (str, int)) or isinstance(token, bool) for token in mutation["path"]):
                    raise ValueError("mutation path tokens must be strings or integer indexes")
            elif set(mutation) != {"op", "indices"} or not isinstance(mutation.get("indices"), list) or len(mutation["indices"]) != 2 or any(not isinstance(n, int) or isinstance(n, bool) or n < 0 for n in mutation["indices"]):
                raise ValueError("swap_events requires two non-negative integer indexes")
    ids = [case["case_id"] for case in controls + attacks]
    if len(ids) != len(set(ids)):
        raise ValueError("duplicate case ID in mutation manifest")
    return manifest


def apply_mutation(candidate: dict[str, Any], mutation: dict[str, Any]) -> None:
    if mutation["op"] == "swap_events":
        events = candidate.get("events")
        left, right = mutation["indices"]
        if not isinstance(events, list) or left >= len(events) or right >= len(events):
            raise ValueError("swap_events index is out of range")
        events[left], events[right] = events[right], events[left]
        return
    path = mutation["path"]
    node: Any = candidate
    for token in path[:-1]:
        if isinstance(node, dict) and isinstance(token, str) and token in node:
            node = node[token]
        elif isinstance(node, list) and isinstance(token, int) and not isinstance(token, bool) and 0 <= token < len(node):
            node = node[token]
        else:
            raise ValueError("mutation path does not resolve")
    final = path[-1]
    if isinstance(node, dict) and isinstance(final, str):
        node[final] = mutation["value"]
    elif isinstance(node, list) and isinstance(final, int) and not isinstance(final, bool) and 0 <= final < len(node):
        node[final] = mutation["value"]
    else:
        raise ValueError("mutation destination is invalid")


def build_corpus(protocol: dict[str, Any]) -> list[dict[str, Any]]:
    manifest = load_mutation_manifest(protocol)
    cases: list[dict[str, Any]] = []
    for control in manifest["positive_controls"]:
        cases.append({"case_id": control["case_id"], "expected": control["expected"], "candidate": make_candidate(control["mode"])})
    for attack in manifest["attacks"]:
        candidate = make_candidate(attack["mode"])
        for mutation in attack["mutations"]:
            apply_mutation(candidate, mutation)
        if attack["reseal"]:
            seal(candidate)
        cases.append({"case_id": attack["case_id"], "expected": attack["expected"], "candidate": candidate})
    return cases


def run_corpus(protocol: dict[str, Any], commit_sha: str | None = None) -> dict[str, Any]:
    results = []
    for case in build_corpus(protocol):
        started = time.perf_counter_ns()
        try:
            errors = validate_candidate(case["candidate"], protocol)
            observed = "REJECT" if errors else "ACCEPT"
            failure_reason = errors
        except Exception as exc:
            observed = "ERROR"
            failure_reason = [f"{type(exc).__name__}: {exc}"]
        results.append({
            "case_id": case["case_id"], "expected_outcome": case["expected"], "observed_outcome": observed,
            "candidate_sha256": sha256(case["candidate"]), "error_count": len(failure_reason),
            "errors": failure_reason, "elapsed_ns": time.perf_counter_ns() - started,
            "release_status": case["candidate"].get("release_status", "BLOCKED"),
            "claim_ceiling": case["candidate"].get("claim_ceiling", "none")
        })
    false_accept = sum(x["expected_outcome"] == "REJECT" and x["observed_outcome"] == "ACCEPT" for x in results)
    false_reject = sum(x["expected_outcome"] == "ACCEPT" and x["observed_outcome"] == "REJECT" for x in results)
    unexpected_error = sum(x["observed_outcome"] == "ERROR" for x in results)
    false_release_pass = sum(x["observed_outcome"] == "ACCEPT" and (x["release_status"] != "BLOCKED" or x["claim_ceiling"] != "none") for x in results)
    attack_count = sum(x["expected_outcome"] == "REJECT" for x in results)
    positive_count = sum(x["expected_outcome"] == "ACCEPT" for x in results)
    targets = protocol["target_metrics"]
    status = "PASS" if (
        attack_count == targets["expected_attack_case_count"]
        and positive_count == targets["expected_positive_control_count"]
        and false_accept == targets["false_accept_count"]
        and false_reject == targets["false_reject_count"]
        and unexpected_error == targets["unexpected_error_count"]
        and false_release_pass == targets["false_release_pass_count"]
    ) else "FAIL"
    return {
        "schema": "srq-causal-integrity-benchmark-report-v1",
        "protocol_id": protocol["protocol_id"], "protocol_raw_sha256": hashlib.sha256(PROTOCOL_PATH.read_bytes()).hexdigest(),
        "mutation_manifest_raw_sha256": hashlib.sha256(MUTATIONS_PATH.read_bytes()).hexdigest(),
        "harness_commit_sha": commit_sha or os.environ.get("GITHUB_SHA", "UNPINNED_LOCAL"),
        "harness_script_raw_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        "status": status, "qualification_status": "BLOCKED", "claim_ceiling": "none",
        "observed_case_count": len(results), "positive_control_count": positive_count, "attack_case_count": attack_count,
        "false_accept_count": false_accept, "false_reject_count": false_reject,
        "unexpected_error_count": unexpected_error, "false_release_pass_count": false_release_pass,
        "cases": results, "limitations": protocol["limitations"]
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true", help="run the frozen synthetic positive/negative corpus")
    parser.add_argument("--output", type=Path, help="write JSON report to this path (stdout if omitted)")
    parser.add_argument("--commit-sha", help="exact checkout commit to bind into the report")
    args = parser.parse_args()
    if not args.self_test:
        parser.error("only --self-test is currently supported; no production transaction is accepted")
    try:
        protocol = load_protocol()
        report = run_corpus(protocol, args.commit_sha)
    except Exception as exc:
        print(f"BLOCKED: benchmark protocol/runtime error: {type(exc).__name__}: {exc}", file=sys.stderr)
        return 2
    encoded = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(encoded, encoding="utf-8")
    else:
        print(encoded, end="")
    print(f"{report['status']}: cases={report['observed_case_count']} positives={report['positive_control_count']} attacks={report['attack_case_count']} false-accept={report['false_accept_count']} false-reject={report['false_reject_count']} errors={report['unexpected_error_count']} release={report['qualification_status']}", file=sys.stderr)
    return 0 if report["status"] == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
