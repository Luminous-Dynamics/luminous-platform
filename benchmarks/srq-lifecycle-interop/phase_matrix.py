#!/usr/bin/env python3
"""Exhaustive black-box conformance test for Nixward phase/record presence.

Runs the actual lifecycle adapter over all 64 evidence-record masks for each
of the nine declared phases. This tests field-presence semantics, not nested
evidence validity, chronology, authorization authenticity or target recovery.
"""
from __future__ import annotations

import argparse
import copy
import hashlib
import json
import math
import statistics
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from typing import Any

from schema_validate import load_strict_json

REPORT_SCHEMA = "srq-transaction-phase-presence-matrix-assessment-v1"
PROTOCOL_SCHEMA = "srq-transaction-phase-presence-matrix-v1"
PROTOCOL_ID = "srq-transaction-phase-presence-matrix-v1"
LOCK_NAME = "upstream-lock-v1.json"
MAX_INPUT = 2 * 1024 * 1024


def sha256(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def expected_accept(required: list[str], present: set[str]) -> bool:
    return set(required).issubset(present)


def build_cases(protocol: dict[str, Any]):
    fields = protocol["record_fields"]
    for phase_spec in protocol["phase_requirements"]:
        phase = phase_spec["phase"]
        required = phase_spec["required_records"]
        for mask in range(1 << len(fields)):
            present = {field for bit, field in enumerate(fields) if mask & (1 << bit)}
            yield {
                "id": f"{phase}:{mask:02x}",
                "phase": phase,
                "presence_mask": f"{mask:02x}",
                "present_records": [field for field in fields if field in present],
                "required_records": list(required),
                "expected_accept": expected_accept(required, present),
                "present": present,
            }


def validate_protocol(protocol: Any, lock: Any) -> list[str]:
    errors: list[str] = []
    if not isinstance(protocol, dict):
        return ["protocol root must be an object"]
    if protocol.get("schema") != PROTOCOL_SCHEMA or protocol.get("protocol_id") != PROTOCOL_ID or protocol.get("version") != 1:
        errors.append("protocol identity/version mismatch")
    fields = protocol.get("record_fields")
    phases = protocol.get("phase_requirements")
    if fields != ["validation", "authorization", "snapshot", "application", "verification", "outcome"]:
        errors.append("record field order/inventory changed")
        fields = []
    if not isinstance(phases, list) or len(phases) != 9:
        errors.append("exactly nine phase definitions are required")
        phases = []
    expected_requirements = {
        "planned": [],
        "validated": ["validation"],
        "authorized": ["validation", "authorization"],
        "snapshotted": ["validation", "authorization", "snapshot"],
        "applied": ["validation", "authorization", "snapshot", "application"],
        "verified": ["validation", "authorization", "snapshot", "application", "verification"],
        "promoted": ["validation", "authorization", "snapshot", "application", "verification", "outcome"],
        "failed": ["outcome"],
        "recovered": ["snapshot", "outcome"],
    }
    observed: dict[str, list[str]] = {}
    for entry in phases:
        if not isinstance(entry, dict):
            errors.append("phase definition must be an object")
            continue
        phase = entry.get("phase")
        required = entry.get("required_records")
        if not isinstance(phase, str) or not isinstance(required, list):
            errors.append("phase name and required_records are malformed")
            continue
        if phase in observed:
            errors.append(f"duplicate phase definition: {phase}")
        observed[phase] = required
        if len(required) != len(set(required)) or any(item not in fields for item in required):
            errors.append(f"phase {phase!r} contains duplicate or unknown record requirements")
    if observed != expected_requirements:
        errors.append("phase requirement mapping differs from the frozen v1 contract")
    enumeration = protocol.get("enumeration", {})
    if not isinstance(enumeration, dict) or (
        enumeration.get("presence_masks_per_phase") != 64
        or enumeration.get("phase_count") != 9
        or enumeration.get("expected_case_count") != 576
        or enumeration.get("expected_accepted_count") != 175
        or enumeration.get("expected_rejected_count") != 401
    ):
        errors.append("enumeration counts changed")
    target = protocol.get("target_metrics", {})
    if not isinstance(target, dict) or any(target.get(key) != 0 for key in ("false_accept_count", "false_reject_count", "release_false_pass_count")):
        errors.append("zero-error target metrics changed")
    invariant = protocol.get("release_invariant", {})
    if not isinstance(invariant, dict) or (
        invariant.get("every_accepted_candidate_must_leave_release_status") != "BLOCKED"
        or invariant.get("every_accepted_candidate_must_leave_claim_ceiling") != "none"
        or invariant.get("release_pass_allowed") is not False
    ):
        errors.append("release-blocked invariant changed")
    source = protocol.get("source_contract", {})
    if not isinstance(source, dict) or not isinstance(lock, dict):
        errors.append("source contract or lock is malformed")
    else:
        matches = [
            pin for pin in lock.get("pins", [])
            if isinstance(pin, dict)
            and pin.get("repository") == source.get("repository")
            and pin.get("commit_sha") == source.get("commit_sha")
            and pin.get("path") == source.get("path")
            and pin.get("git_blob_sha1") == source.get("git_blob_sha1")
        ]
        if len(matches) != 1:
            errors.append("source contract does not match exactly one immutable lock entry")
    cases = list(build_cases(protocol)) if not errors else []
    if cases:
        accepted = sum(1 for item in cases if item["expected_accept"])
        if (len(cases), accepted, len(cases) - accepted) != (576, 175, 401):
            errors.append("generated acceptance matrix differs from frozen enumeration counts")
    return errors


def percentile(sorted_values: list[float], pct: float) -> float:
    if not sorted_values:
        return 0.0
    index = max(0, min(len(sorted_values) - 1, math.ceil(pct * len(sorted_values)) - 1))
    return sorted_values[index]


def run_matrix(
    protocol_path: Path,
    lock_path: Path,
    transaction_fixture_path: Path,
    validator_path: Path,
    nixward_root: Path,
    mycelix_root: Path,
    sovereign_boot_root: Path,
    boot_observation_path: Path,
    output_path: Path,
) -> dict[str, Any]:
    protocol, protocol_raw = load_strict_json(protocol_path, MAX_INPUT)
    lock, lock_raw = load_strict_json(lock_path, MAX_INPUT)
    base_transaction, base_raw = load_strict_json(transaction_fixture_path, MAX_INPUT)
    errors = validate_protocol(protocol, lock)
    if errors:
        raise ValueError("protocol preflight failed: " + "; ".join(errors))

    output_path.parent.mkdir(parents=True, exist_ok=True)
    report: dict[str, Any] = {
        "schema": REPORT_SCHEMA,
        "protocol": {
            "protocol_id": protocol["protocol_id"],
            "version": protocol["version"],
            "git_blob_sha1": None,
            "raw_sha256": sha256(protocol_raw),
        },
        "inputs": {
            "transaction_fixture_sha256": sha256(base_raw),
            "upstream_lock_sha256": sha256(lock_raw),
            "phase_matrix_protocol_sha256": sha256(protocol_raw),
        },
        "result": "RUNNING",
        "scope": protocol["scope"],
        "expected_case_count": 576,
        "completed_case_count": 0,
        "expected_accept_count": 175,
        "observed_accept_count": 0,
        "false_accept_count": 0,
        "false_reject_count": 0,
        "release_false_pass_count": 0,
        "cases": [],
        "phase_summary": {},
        "performance_ms": {},
        "limitations": [
            "Only the phase label versus presence/absence of six record fields is tested.",
            "Record contents, signature validity, authorization, event chronology, command/result consistency, boot health semantics and physical recovery are not qualified.",
            "The underlying lifecycle adapter deliberately treats later-stage records as permitted to be pre-populated for an earlier phase.",
            "Passing means the observed record-presence acceptance matrix matched the frozen protocol; it does not qualify a release.",
        ],
    }
    protocol_blob = subprocess.run(
        ["git", "-C", str(protocol_path.parents[2]), "hash-object", "--", str(protocol_path.relative_to(protocol_path.parents[2]))],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    report["protocol"]["git_blob_sha1"] = protocol_blob

    fields = protocol["record_fields"]
    phase_requirements = {entry["phase"]: entry["required_records"] for entry in protocol["phase_requirements"]}
    false_accepts = false_rejects = release_false_passes = observed_accepts = 0
    durations_ms: list[float] = []
    phase_summary: dict[str, Any] = {}
    validator_errors: list[dict[str, Any]] = []
    with tempfile.TemporaryDirectory(prefix="srq-phase-matrix-") as scratch:
        scratch_path = Path(scratch)
        candidate_path = scratch_path / "candidate-transaction.json"
        assessment_path = scratch_path / "candidate-assessment.json"
        for case in build_cases(protocol):
            candidate = copy.deepcopy(base_transaction)
            candidate["phase"] = case["phase"]
            for field in fields:
                candidate[field] = (
                    {"synthetic_phase_matrix_sentinel": f"{case['phase']}/{case['presence_mask']}/{field}"}
                    if field in case["present"]
                    else None
                )
            raw = (json.dumps(candidate, sort_keys=True, separators=(",", ":"), ensure_ascii=False) + "\n").encode("utf-8")
            candidate_path.write_bytes(raw)
            assessment_path.unlink(missing_ok=True)
            command = [
                "bash", str(validator_path),
                "--nixward-root", str(nixward_root),
                "--mycelix-root", str(mycelix_root),
                "--sovereign-boot-root", str(sovereign_boot_root),
                "--boot-observation", str(boot_observation_path),
                "--transaction-fixture", str(candidate_path),
                "--output", str(assessment_path),
            ]
            start = time.perf_counter()
            completed = subprocess.run(command, capture_output=True, text=True, check=False)
            elapsed = (time.perf_counter() - start) * 1000.0
            durations_ms.append(elapsed)
            actual_accept = completed.returncode == 0
            rejection_reason = ""
            if not actual_accept:
                for line in completed.stderr.splitlines():
                    if "CONTRACT VIOLATION:" in line:
                        rejection_reason = line.strip()
                if not rejection_reason:
                    validator_errors.append({
                        "id": case["id"],
                        "return_code": completed.returncode,
                        "stderr_tail": completed.stderr[-1200:],
                    })
            assessment_release_status = None
            assessment_claim_ceiling = None
            if actual_accept:
                observed_accepts += 1
                try:
                    assessment = json.loads(assessment_path.read_text(encoding="utf-8"))
                    assessment_release_status = assessment.get("release_status")
                    assessment_claim_ceiling = assessment.get("claim_ceiling")
                    if assessment_release_status != "BLOCKED" or assessment_claim_ceiling != "none":
                        release_false_passes += 1
                except Exception as exc:
                    release_false_passes += 1
                    validator_errors.append({"id": case["id"], "assessment_error": f"{type(exc).__name__}: {exc}"})
            matched = actual_accept == case["expected_accept"] and (actual_accept or bool(rejection_reason))
            if case["expected_accept"] and not actual_accept:
                false_rejects += 1
            if not case["expected_accept"] and actual_accept:
                false_accepts += 1
            result = "MATCH" if matched else "MISMATCH"
            if result == "MISMATCH" and not case["expected_accept"] and actual_accept:
                result = "FALSE_ACCEPT"
            elif result == "MISMATCH" and case["expected_accept"] and not actual_accept:
                result = "FALSE_REJECT"
            record = {
                "id": case["id"],
                "phase": case["phase"],
                "presence_mask": case["presence_mask"],
                "present_records": case["present_records"],
                "required_records": case["required_records"],
                "expected_accept": case["expected_accept"],
                "actual_accept": actual_accept,
                "result": result,
                "validator_exit_code": completed.returncode,
                "rejection_reason": rejection_reason or None,
                "candidate_raw_sha256": sha256(raw),
                "elapsed_ms": round(elapsed, 3),
                "assessment_release_status": assessment_release_status,
                "assessment_claim_ceiling": assessment_claim_ceiling,
            }
            report["cases"].append(record)
            phase_row = phase_summary.setdefault(case["phase"], {
                "case_count": 0,
                "expected_accept_count": 0,
                "observed_accept_count": 0,
                "false_accept_count": 0,
                "false_reject_count": 0,
            })
            phase_row["case_count"] += 1
            phase_row["expected_accept_count"] += int(case["expected_accept"])
            phase_row["observed_accept_count"] += int(actual_accept)
            phase_row["false_accept_count"] += int(not case["expected_accept"] and actual_accept)
            phase_row["false_reject_count"] += int(case["expected_accept"] and not actual_accept)

    sorted_durations = sorted(durations_ms)
    report["phase_summary"] = phase_summary
    report["completed_case_count"] = len(report["cases"])
    report["observed_accept_count"] = observed_accepts
    report["false_accept_count"] = false_accepts
    report["false_reject_count"] = false_rejects
    report["release_false_pass_count"] = release_false_passes
    report["performance_ms"] = {
        "total": round(sum(durations_ms), 3),
        "p50": round(percentile(sorted_durations, 0.50), 3),
        "p95": round(percentile(sorted_durations, 0.95), 3),
        "p99": round(percentile(sorted_durations, 0.99), 3),
        "max": round(max(sorted_durations, default=0.0), 3),
    }
    report["validator_errors"] = validator_errors
    passed = (
        report["completed_case_count"] == report["expected_case_count"]
        and false_accepts == 0
        and false_rejects == 0
        and release_false_passes == 0
        and observed_accepts == report["expected_accept_count"]
        and not validator_errors
    )
    report["result"] = "PASS" if passed else "FAIL"
    output_path.write_text(json.dumps(report, sort_keys=True, separators=(",", ":"), ensure_ascii=False) + "\n", encoding="utf-8")
    return report


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--protocol", type=Path, default=Path(__file__).with_name("transaction-phase-matrix-v1.json"))
    parser.add_argument("--lock", type=Path, default=Path(__file__).with_name("upstream-lock-v1.json"))
    parser.add_argument("--transaction-fixture", required=True, type=Path)
    parser.add_argument("--validator", type=Path, default=Path(__file__).with_name("validate.sh"))
    parser.add_argument("--nixward-root", required=True, type=Path)
    parser.add_argument("--mycelix-root", required=True, type=Path)
    parser.add_argument("--sovereign-boot-root", required=True, type=Path)
    parser.add_argument("--boot-observation", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    try:
        report = run_matrix(
            args.protocol, args.lock, args.transaction_fixture, args.validator,
            args.nixward_root, args.mycelix_root, args.sovereign_boot_root,
            args.boot_observation, args.output,
        )
    except Exception as exc:
        print(f"BLOCKED: phase matrix did not produce a conformance result: {type(exc).__name__}: {exc}", file=sys.stderr)
        return 2
    print(
        f"PHASE_MATRIX_{report['result']}: {report['completed_case_count']} cases; "
        f"{report['observed_accept_count']} accepted; "
        f"{report['false_accept_count']} false accepts; "
        f"{report['false_reject_count']} false rejects; "
        f"{report['release_false_pass_count']} false release passes."
    )
    return 0 if report["result"] == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
