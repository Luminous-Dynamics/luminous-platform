#!/usr/bin/env python3
"""Mutation-qualify the SRQ history oracle using independent adversarial traces.

This does not mutate the checker implementation and does not qualify a sink.
It changes one history at a time and requires the checker to emit the named
security violation. Malformed traces, generic nonzero exits, or unrelated
violations do not count as detection.
"""
from __future__ import annotations

import argparse
from copy import deepcopy
from dataclasses import dataclass
import hashlib
import json
import os
from pathlib import Path
from typing import Any, Callable

from history_oracle import InvalidHistory, VIOLATION_INVARIANTS, check_history

SCHEMA = "srq-trace-mutation-report-v1"
DIGEST_E = "e" * 64


@dataclass(frozen=True)
class MutationCase:
    mutation_id: str
    expected_code: str
    description: str
    mutate: Callable[[dict[str, Any]], None]

    @property
    def expected_invariant(self) -> str:
        return VIOLATION_INVARIANTS[self.expected_code]


def renumber(history: dict[str, Any]) -> None:
    for index, event in enumerate(history["events"]):
        event["seq"] = index


def changed_intent(history):
    history["events"][4]["effect_digest"] = DIGEST_E


def duplicate_logical_effect(history):
    duplicate = deepcopy(history["events"][1])
    duplicate.update({"attempt_id": "a2", "effect_id": "e2"})
    history["events"].insert(5, duplicate)
    renumber(history)


def stale_worker(history):
    history["events"].insert(1, {"seq": 1, "kind": "fence_advanced", "fencing_token": 2})
    renumber(history)


def committed_missing_receipt(history):
    history["events"][3]["receipt_id"] = "missing-receipt"


def delete_receipt(history):
    del history["events"][2]
    renumber(history)


def receipt_digest_mismatch(history):
    history["events"][2]["effect_digest"] = DIGEST_E


def receipt_wrong_sink(history):
    history["events"][2]["sink_identity_digest"] = DIGEST_E


def effect_wrong_sink(history):
    history["events"][1]["sink_identity_digest"] = DIGEST_E


def unknown_claims_receipt(history):
    history["events"][5]["outcome"] = "unknown"


def rejected_after_effect(history):
    history["events"][3]["outcome"] = "rejected"
    history["events"][3]["receipt_id"] = None


def duplicate_receipt_id(history):
    history["events"].append(deepcopy(history["events"][2]))
    renumber(history)


def nonmonotonic_fence(history):
    history["events"].append({
        "seq": len(history["events"]), "kind": "fence_advanced", "fencing_token": 1
    })


def sequence_gap(history):
    history["events"][2]["seq"] = 9


def duplicate_attempt_id(history):
    event = deepcopy(history["events"][4])
    event["attempt_id"] = "a1"
    history["events"].append(event)
    renumber(history)


def orphan_effect(history):
    event = deepcopy(history["events"][1])
    event.update({"attempt_id": "ghost", "effect_id": "orphan"})
    history["events"].insert(0, event)
    renumber(history)


def duplicate_effect_id(history):
    event = deepcopy(history["events"][1])
    event["attempt_id"] = "a2"
    history["events"].insert(5, event)
    renumber(history)


def receipt_without_effect(history):
    event = deepcopy(history["events"][2])
    event.update({"attempt_id": "a2", "receipt_id": "r-ghost", "effect_id": "ghost"})
    history["events"].append(event)
    renumber(history)


def receipt_wrong_attempt(history):
    # Move the second invocation before the first receipt so that the wrong
    # attribution is evaluated against an already-known, distinct attempt.
    second_invocation = history["events"].pop(4)
    history["events"].insert(2, second_invocation)
    history["events"][3]["attempt_id"] = "a2"
    renumber(history)


def stale_invocation_fence(history):
    history["events"][4]["fencing_token"] = 0


def effect_fence_mismatch(history):
    history["events"][1]["fencing_token"] = 2


def effect_intent_mismatch(history):
    history["events"][1]["effect_digest"] = DIGEST_E


def multiple_terminal_responses(history):
    history["events"].append({
        "seq": len(history["events"]), "kind": "response", "attempt_id": "a1",
        "outcome": "committed", "receipt_id": "r1"
    })


def rejected_with_receipt(history):
    history["events"][5]["outcome"] = "rejected"


MUTATIONS = (
    MutationCase("changed_intent_same_key", "KEY_REUSED_FOR_DIFFERENT_INTENT", "Reuse an idempotency key with a changed effect digest.", changed_intent),
    MutationCase("duplicate_logical_effect", "DUPLICATE_LOGICAL_EFFECT", "Commit a second effect under one key.", duplicate_logical_effect),
    MutationCase("stale_worker_after_fence", "STALE_WORKER_EFFECT", "Advance fencing before an old worker commits.", stale_worker),
    MutationCase("committed_without_receipt", "COMMITTED_RESPONSE_WITHOUT_PRIOR_RECEIPT", "Claim commitment without a known receipt.", committed_missing_receipt),
    MutationCase("effect_receipt_gap", "EFFECT_WITHOUT_RECEIPT", "Remove the receipt for an observed effect.", delete_receipt),
    MutationCase("receipt_wrong_effect_digest", "RECEIPT_DOES_NOT_BIND_EFFECT", "Change a receipt's effect digest.", receipt_digest_mismatch),
    MutationCase("receipt_wrong_sink", "RECEIPT_WRONG_SINK", "Bind a receipt to a different sink.", receipt_wrong_sink),
    MutationCase("effect_wrong_sink", "EFFECT_WRONG_SINK", "Record an effect against a different sink identity.", effect_wrong_sink),
    MutationCase("unknown_response_with_receipt", "UNKNOWN_RESPONSE_CLAIMS_RECEIPT", "Claim a receipt while reporting an unknown outcome.", unknown_claims_receipt),
    MutationCase("rejected_after_effect", "REJECTED_ATTEMPT_HAS_EFFECT", "Reject an attempt that already committed an effect.", rejected_after_effect),
    MutationCase("duplicate_receipt_identifier", "DUPLICATE_RECEIPT_ID", "Reuse a receipt identifier for another event.", duplicate_receipt_id),
    MutationCase("nonmonotonic_fence", "FENCE_NOT_MONOTONIC", "Replay a fencing-token advance at the current value.", nonmonotonic_fence),
    MutationCase("sequence_gap", "NON_CONTIGUOUS_SEQUENCE", "Create a gap in the recorded event sequence.", sequence_gap),
    MutationCase("duplicate_attempt_identifier", "DUPLICATE_INVOCATION_ID", "Reuse a previously observed attempt identifier.", duplicate_attempt_id),
    MutationCase("orphan_effect", "EVENT_WITHOUT_PRIOR_INVOCATION", "Record an effect without a prior invocation.", orphan_effect),
    MutationCase("duplicate_effect_identifier", "DUPLICATE_EFFECT_ID", "Reuse an effect identifier.", duplicate_effect_id),
    MutationCase("receipt_without_effect", "RECEIPT_WITHOUT_PRIOR_EFFECT", "Record a receipt for an effect that was never observed.", receipt_without_effect),
    MutationCase("receipt_attributed_to_wrong_attempt", "RECEIPT_COMMIT_ATTRIBUTED_TO_WRONG_ATTEMPT", "Attribute an effect receipt to a different attempt.", receipt_wrong_attempt),
    MutationCase("stale_invocation_fence", "INVOCATION_USED_STALE_OR_UNISSUED_FENCE", "Invoke with a fencing token older than the active token.", stale_invocation_fence),
    MutationCase("effect_fence_mismatch", "EFFECT_FENCE_DIFFERS_FROM_INVOCATION", "Commit using a token different from the invocation token.", effect_fence_mismatch),
    MutationCase("effect_intent_mismatch", "EFFECT_INTENT_MISMATCH", "Record an effect that does not match the invoked intent.", effect_intent_mismatch),
    MutationCase("multiple_terminal_responses", "MULTIPLE_TERMINAL_RESPONSES", "Emit two terminal responses for one attempt.", multiple_terminal_responses),
    MutationCase("rejected_response_with_receipt", "REJECTED_RESPONSE_HAS_RECEIPT", "Return a receipt while claiming the operation was rejected.", rejected_with_receipt),
)


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load_and_validate_manifest(root: Path) -> dict[str, Any]:
    manifest_path = root / "trace-mutation-protocol-v1.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    expected = [
        {
            "mutation_id": item.mutation_id,
            "expected_code": item.expected_code,
            "expected_invariant": item.expected_invariant,
            "description": item.description,
        }
        for item in MUTATIONS
    ]
    if manifest.get("schema") != "srq-trace-mutation-protocol-v1":
        raise ValueError("unsupported trace mutation protocol schema")
    declared = [
        {
            "mutation_id": item.get("mutation_id"),
            "expected_code": item.get("expected_code"),
            "expected_invariant": item.get("expected_invariant"),
            "description": item.get("description"),
        }
        for item in manifest.get("mutations", [])
    ]
    if declared != expected:
        raise ValueError("manifest mutation list/order/description/expected invariant differs from executable corpus")
    acceptance = manifest.get("acceptance", {})
    expected_acceptance = {
        "mutation_count": len(MUTATIONS),
        "detected_expected_mutations": len(MUTATIONS),
        "survived_or_wrong_violation_count": 0,
        "invalid_mutation_count": 0,
        "positive_control_status": "PASS",
    }
    if acceptance != expected_acceptance:
        raise ValueError("manifest acceptance criteria differ from the executable corpus gate")
    guardrails = manifest.get("guardrails", {})
    if (
        guardrails.get("checker_source_is_not_mutated") is not True
        or guardrails.get("malformed_histories_do_not_count_as_detected") is not True
        or guardrails.get("qualification_status") != "BLOCKED"
        or guardrails.get("claim_ceiling") != "none"
    ):
        raise ValueError("manifest guardrails or claim ceiling are missing or inconsistent")
    return manifest


def run_corpus(root: Path) -> dict[str, Any]:
    load_and_validate_manifest(root)
    fixture_path = root / "fixtures" / "valid-retry.json"
    baseline = json.loads(fixture_path.read_text(encoding="utf-8"))
    baseline_report = check_history(deepcopy(baseline))
    if baseline_report["safety_status"] != "PASS" or baseline_report["violation_count"] != 0:
        raise ValueError("positive-control trace does not satisfy the baseline oracle")

    results = []
    for mutation in MUTATIONS:
        history = deepcopy(baseline)
        try:
            mutation.mutate(history)
            observed = check_history(history)
            codes = sorted({item["code"] for item in observed["violations"]})
            invariant_ids = observed["invariants_violated"]
            if (
                observed["safety_status"] == "FAIL"
                and mutation.expected_code in codes
                and mutation.expected_invariant in invariant_ids
            ):
                status = "DETECTED"
            elif observed["safety_status"] == "INVALID_HISTORY":
                status = "INVALID_MUTATION"
            else:
                status = "SURVIVED_OR_WRONG_VIOLATION"
            results.append({
                "mutation_id": mutation.mutation_id,
                "expected_code": mutation.expected_code,
                "expected_invariant": mutation.expected_invariant,
                "status": status,
                "observed_codes": codes,
                "observed_invariants": invariant_ids,
            })
        except (InvalidHistory, KeyError, IndexError, TypeError, ValueError) as exc:
            results.append({
                "mutation_id": mutation.mutation_id,
                "expected_code": mutation.expected_code,
                "expected_invariant": mutation.expected_invariant,
                "status": "INVALID_MUTATION",
                "error": str(exc),
            })

    detected = sum(item["status"] == "DETECTED" for item in results)
    survived = sum(item["status"] == "SURVIVED_OR_WRONG_VIOLATION" for item in results)
    invalid = sum(item["status"] == "INVALID_MUTATION" for item in results)
    return {
        "schema": SCHEMA,
        "status": "PASS" if detected == len(MUTATIONS) and survived == 0 and invalid == 0 else "FAIL",
        "qualification_status": "BLOCKED",
        "claim_ceiling": "none",
        "scope": "history_oracle_trace_mutation_adequacy_only",
        "baseline_positive_control": {
            "status": baseline_report["safety_status"],
            "violation_count": baseline_report["violation_count"],
            "fixture_sha256": sha256_file(fixture_path),
        },
        "mutation_count": len(MUTATIONS),
        "detected_expected_mutations": detected,
        "survived_or_wrong_violation_count": survived,
        "invalid_mutation_count": invalid,
        "mutations": results,
        "harness_commit_sha": os.environ.get("SRQ_TRACE_MUTATION_CHECKOUT_SHA") or os.environ.get("GITHUB_SHA"),
        "workflow_event_sha": os.environ.get("GITHUB_SHA"),
        "source_head_sha": os.environ.get("SRQ_TRACE_MUTATION_PR_HEAD_SHA") or os.environ.get("GITHUB_SHA"),
        "source_sha256": {
            name: sha256_file(root / name)
            for name in (
                "history_oracle.py",
                "history-protocol-v1.json",
                "trace_mutation_qualification.py",
                "test_trace_mutations.py",
                "trace-mutation-protocol-v1.json",
                "fixtures/valid-retry.json",
            )
        },
        "note": "A detected fixture mutation shows sensitivity to that selected trace fault; it does not prove checker completeness, recorder honesty, sink durability, or production correctness.",
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, help="optional JSON report path")
    args = parser.parse_args(argv)
    root = Path(__file__).resolve().parent
    try:
        report = run_corpus(root)
    except (OSError, json.JSONDecodeError, ValueError) as exc:
        report = {
            "schema": SCHEMA,
            "status": "INVALID_PROTOCOL_OR_FIXTURE",
            "qualification_status": "BLOCKED",
            "claim_ceiling": "none",
            "error": str(exc),
            "workflow_event_sha": os.environ.get("GITHUB_SHA"),
        }
    rendered = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered, encoding="utf-8")
    print(rendered, end="")
    return 0 if report.get("status") == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
