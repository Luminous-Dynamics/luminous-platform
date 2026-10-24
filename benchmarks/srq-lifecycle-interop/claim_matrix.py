#!/usr/bin/env python3
"""Test every Mycelix evidence-tier/claim-ceiling pair against schema and adapter."""
from __future__ import annotations

import argparse
import copy
import hashlib
import json
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from typing import Any

from schema_validate import load_strict_json

REPORT_SCHEMA = "srq-evidence-claim-ceiling-matrix-assessment-v1"
PROTOCOL_SCHEMA = "srq-evidence-claim-ceiling-matrix-v1"
PROTOCOL_ID = "srq-evidence-claim-ceiling-matrix-v1"
DRAFT_URI = "https://json-schema.org/draft/2020-12/schema"
MAX_INPUT = 2 * 1024 * 1024
EXPECTED_SCHEMA_SOURCE = {
    "repository": "Luminous-Dynamics/mycelix",
    "commit_sha": "b02cefab8474262aef23b61a4ac3811fe92e29bf",
    "path": "research/evidence-plane/receipt-v0.schema.json",
    "git_blob_sha1": "e2b90a995f70d0a771b04e7f5fc077e1c0394f86",
}

EXPECTED_ALLOWED = {
    "S0": ["synthetic_only"],
    "S1": ["synthetic_only", "transcription_only"],
    "S2": ["synthetic_only", "transcription_only", "source_reproducible"],
    "S3": ["independently_reproduced"],
    "S4": ["domain_validated"],
}
EXPECTED_STATUS = {
    "S0": "planned",
    "S1": "executed",
    "S2": "executed",
    "S3": "independently_checked",
    "S4": "domain_validated",
}
EXPECTED_CANONICAL_CLAIM = {
    "S0": "synthetic_only",
    "S1": "transcription_only",
    "S2": "source_reproducible",
    "S3": "independently_reproduced",
    "S4": "domain_validated",
}
CLAIM_CEILINGS = [
    "synthetic_only",
    "transcription_only",
    "source_reproducible",
    "independently_reproduced",
    "domain_validated",
]


def sha256(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def validate_protocol(protocol: Any, lock: Any) -> list[str]:
    errors: list[str] = []
    if not isinstance(protocol, dict):
        return ["protocol root must be an object"]
    if protocol.get("schema") != PROTOCOL_SCHEMA or protocol.get("protocol_id") != PROTOCOL_ID or protocol.get("version") != 1:
        errors.append("protocol identity/version mismatch")
    if protocol.get("evidence_tiers") != ["S0", "S1", "S2", "S3", "S4"]:
        errors.append("evidence-tier inventory changed")
    if protocol.get("claim_ceilings") != CLAIM_CEILINGS:
        errors.append("claim-ceiling inventory changed")
    profiles = protocol.get("canonical_profiles")
    expected_profiles = [
        {
            "evidence_tier": tier,
            "run_status": EXPECTED_STATUS[tier],
            "canonical_claim_ceiling": EXPECTED_CANONICAL_CLAIM[tier],
            "allowed_claim_ceilings": EXPECTED_ALLOWED[tier],
        }
        for tier in ("S0", "S1", "S2", "S3", "S4")
    ]
    if profiles != expected_profiles:
        errors.append("canonical profiles or allowed claim-ceiling map changed")
    enumeration = protocol.get("enumeration", {})
    if not isinstance(enumeration, dict) or (
        enumeration.get("tier_count") != 5
        or enumeration.get("claim_ceiling_count") != 5
        or enumeration.get("expected_case_count") != 25
        or enumeration.get("expected_accept_count") != 8
        or enumeration.get("expected_reject_count") != 17
    ):
        errors.append("enumeration cardinalities changed")
    metrics = protocol.get("target_metrics", {})
    if not isinstance(metrics, dict) or any(
        metrics.get(key) != 0
        for key in (
            "false_accept_count",
            "false_reject_count",
            "schema_adapter_disagreement_count",
            "release_false_pass_count",
        )
    ):
        errors.append("zero-error target metrics changed")
    invariant = protocol.get("release_invariant", {})
    if not isinstance(invariant, dict) or (
        invariant.get("every_accepted_candidate_must_leave_release_status") != "BLOCKED"
        or invariant.get("every_accepted_candidate_must_leave_claim_ceiling") != "none"
        or invariant.get("release_pass_allowed") is not False
    ):
        errors.append("release-blocked invariant changed")
    source = protocol.get("source_contract", {})
    if source != EXPECTED_SCHEMA_SOURCE:
        errors.append("Mycelix schema source contract identity changed")
    pins = lock.get("pins", []) if isinstance(lock, dict) else []
    matching = [
        item for item in pins
        if isinstance(item, dict)
        and item.get("repository") == source.get("repository")
        and item.get("commit_sha") == source.get("commit_sha")
        and item.get("path") == source.get("path")
        and item.get("git_blob_sha1") == source.get("git_blob_sha1")
    ]
    if len(matching) != 1:
        errors.append("Mycelix schema source pin does not match exactly one immutable lock entry")
    return errors


def canonical_receipt(base: dict[str, Any], tier: str) -> dict[str, Any]:
    receipt = copy.deepcopy(base)
    receipt["run_status"] = EXPECTED_STATUS[tier]
    receipt["evidence_tier"] = tier
    receipt["claim_ceiling"] = EXPECTED_CANONICAL_CLAIM[tier]
    summary = receipt["benchmark"]["results_summary"]
    if tier == "S0":
        summary["state"] = "not_run"
        summary["observed_result_count"] = 0
    else:
        summary["state"] = "available"
        summary["observed_result_count"] = 1

    if tier in {"S2", "S3", "S4"}:
        receipt["sources"] = [
            {
                "source_id": "synthetic-matrix-source-001",
                "url": "https://example.invalid/srq-matrix/source.json",
                "observed_at": "2026-10-24T00:00:00Z",
                "media_type": "application/json",
                "raw_bytes_ingested": True,
                "byte_count": 1,
                "sha256": "a" * 64,
                "rights_status": "public_domain",
                "contact_status": "not_required",
                "temporal_coverage": None,
                "spatial_coverage": None,
                "metadata_conflicts": [],
                "artifact_path": "synthetic/source.json",
            }
        ]
        receipt["subject"]["artifact_sha256"] = "b" * 64
        receipt["gates"]["source_integrity"] = "pass"
        receipt["gates"]["rights_stewardship"] = "pass"

    if tier in {"S3", "S4"}:
        commit = "c" * 40
        receipt["subject"]["commit"] = commit
        receipt["independent_verification"] = {
            "status": "passed",
            "verifier_kind": "independent_implementation",
            "verifier_ref": "synthetic:claim-ceiling-matrix-verifier",
            "replayed_commit": commit,
            "note": "Synthetic structural control only; no verifier ran.",
        }
        receipt["gates"]["independent_verification"] = "pass"
        receipt["gates"]["negative_controls"] = "pass"
        receipt["gates"]["exact_head"] = "pass"
    if tier == "S4":
        receipt["gates"]["domain_validation"] = "pass"
    return receipt


def make_cases(protocol: dict[str, Any], base: dict[str, Any]):
    profiles = {item["evidence_tier"]: item for item in protocol["canonical_profiles"]}
    for tier in protocol["evidence_tiers"]:
        profile = profiles[tier]
        canonical = canonical_receipt(base, tier)
        for claim in protocol["claim_ceilings"]:
            candidate = copy.deepcopy(canonical)
            candidate["claim_ceiling"] = claim
            yield {
                "id": f"{tier}:{claim}",
                "evidence_tier": tier,
                "run_status": profile["run_status"],
                "claim_ceiling": claim,
                "expected_accept": claim in profile["allowed_claim_ceilings"],
                "candidate": candidate,
            }


def run_matrix(
    protocol_path: Path,
    lock_path: Path,
    schema_path: Path,
    base_receipt_path: Path,
    validator_path: Path,
    nixward_root: Path,
    mycelix_root: Path,
    sovereign_boot_root: Path,
    boot_observation_path: Path,
    schema_validator_path: Path,
    output_path: Path,
) -> dict[str, Any]:
    protocol, protocol_raw = load_strict_json(protocol_path, MAX_INPUT)
    lock, lock_raw = load_strict_json(lock_path, MAX_INPUT)
    schema, schema_raw = load_strict_json(schema_path, MAX_INPUT)
    base_receipt, base_raw = load_strict_json(base_receipt_path, MAX_INPUT)
    errors = validate_protocol(protocol, lock)
    if errors:
        raise ValueError("protocol preflight failed: " + "; ".join(errors))
    if not isinstance(schema, dict) or schema.get("$schema") != DRAFT_URI:
        raise ValueError("pinned Mycelix schema must declare Draft 2020-12")

    schema_repo_root = schema_path.parents[2]
    schema_relative_path = schema_path.relative_to(schema_repo_root)
    actual_schema_commit = subprocess.run(
        ["git", "-C", str(schema_repo_root), "rev-parse", "HEAD"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    actual_schema_blob = subprocess.run(
        ["git", "-C", str(schema_repo_root), "hash-object", "--", str(schema_relative_path)],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    if actual_schema_commit != protocol["source_contract"]["commit_sha"]:
        raise ValueError(f"Mycelix schema commit mismatch: expected {protocol['source_contract']['commit_sha']}, found {actual_schema_commit}")
    if actual_schema_blob != protocol["source_contract"]["git_blob_sha1"]:
        raise ValueError(f"Mycelix schema blob mismatch: expected {protocol['source_contract']['git_blob_sha1']}, found {actual_schema_blob}")

    protocol_blob = subprocess.run(
        ["git", "-C", str(protocol_path.parents[2]), "hash-object", "--", str(protocol_path.relative_to(protocol_path.parents[2]))],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    schema_blob = subprocess.run(
        ["git", "-C", str(schema_path.parents[2]), "hash-object", "--", str(schema_path.relative_to(schema_path.parents[2]))],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()

    output_path.parent.mkdir(parents=True, exist_ok=True)
    report: dict[str, Any] = {
        "schema": REPORT_SCHEMA,
        "protocol": {
            "protocol_id": protocol["protocol_id"],
            "version": protocol["version"],
            "git_blob_sha1": protocol_blob,
            "raw_sha256": sha256(protocol_raw),
        },
        "schema_input": {
            "repository": "Luminous-Dynamics/mycelix",
            "commit_sha": actual_schema_commit,
            "git_blob_sha1": actual_schema_blob,
            "raw_sha256": sha256(schema_raw),
        },
        "base_receipt_sha256": sha256(base_raw),
        "result": "RUNNING",
        "expected_case_count": 25,
        "completed_case_count": 0,
        "expected_accept_count": 8,
        "observed_accept_count": 0,
        "false_accept_count": 0,
        "false_reject_count": 0,
        "schema_adapter_disagreement_count": 0,
        "release_false_pass_count": 0,
        "cases": [],
        "tier_summary": {},
        "limitations": [
            "All non-S0 receipts use synthetic sentinel source, artifact, commit and verifier fields; these are not real evidence.",
            "The matrix tests only tier/claim-ceiling eligibility around canonical status-specific receipt shapes.",
            "It does not establish truth, rights beyond the synthetic schema fields, actual execution, independent reproduction, domain truth or release qualification.",
        ],
    }
    false_accepts = false_rejects = disagreements = release_false_passes = observed_accepts = 0
    schema_accepted_count = 0
    durations_ms: list[float] = []
    tier_summary: dict[str, Any] = {}
    validator_errors: list[dict[str, Any]] = []

    with tempfile.TemporaryDirectory(prefix="srq-claim-ceiling-matrix-") as scratch:
        root = Path(scratch)
        candidate_path = root / "candidate.json"
        schema_report_path = root / "schema-report.json"
        assessment_path = root / "adapter-assessment.json"
        for case in make_cases(protocol, base_receipt):
            raw = (json.dumps(case["candidate"], sort_keys=True, separators=(",", ":"), ensure_ascii=False) + "\n").encode("utf-8")
            candidate_path.write_bytes(raw)
            schema_report_path.unlink(missing_ok=True)
            assessment_path.unlink(missing_ok=True)
            started = time.perf_counter()
            schema_process = subprocess.run(
                [
                    sys.executable, str(schema_validator_path),
                    "--schema", str(schema_path),
                    "--instance", str(candidate_path),
                    "--expect", "valid",
                    "--output", str(schema_report_path),
                ],
                capture_output=True,
                text=True,
                check=False,
            )
            schema_result = "ERROR"
            schema_errors: list[Any] = []
            if schema_report_path.exists():
                try:
                    schema_report = json.loads(schema_report_path.read_text(encoding="utf-8"))
                    schema_result = schema_report.get("result", "ERROR")
                    schema_errors = schema_report.get("errors", [])
                except Exception as exc:
                    schema_result = "ERROR"
                    validator_errors.append({"id": case["id"], "schema_report_error": f"{type(exc).__name__}: {exc}"})
            if schema_process.returncode == 2 or schema_result == "ERROR":
                validator_errors.append({"id": case["id"], "schema_return_code": schema_process.returncode, "stderr": schema_process.stderr[-1000:]})

            adapter_process = subprocess.run(
                [
                    "bash", str(validator_path),
                    "--nixward-root", str(nixward_root),
                    "--mycelix-root", str(mycelix_root),
                    "--sovereign-boot-root", str(sovereign_boot_root),
                    "--boot-observation", str(boot_observation_path),
                    "--evidence-plane-receipt", str(candidate_path),
                    "--output", str(assessment_path),
                ],
                capture_output=True,
                text=True,
                check=False,
            )
            elapsed_ms = (time.perf_counter() - started) * 1000.0
            durations_ms.append(elapsed_ms)
            schema_accept = schema_result == "VALID"
            adapter_accept = adapter_process.returncode == 0
            if schema_accept:
                schema_accepted_count += 1
            rejection_reason = ""
            if not adapter_accept:
                rejection_reason = next((line.strip() for line in adapter_process.stderr.splitlines() if "CONTRACT VIOLATION:" in line), "")
                if not rejection_reason:
                    validator_errors.append({"id": case["id"], "adapter_return_code": adapter_process.returncode, "stderr": adapter_process.stderr[-1000:]})
            assessment_release_status = None
            assessment_claim_ceiling = None
            if adapter_accept:
                observed_accepts += 1
                try:
                    assessment = json.loads(assessment_path.read_text(encoding="utf-8"))
                    assessment_release_status = assessment.get("release_status")
                    assessment_claim_ceiling = assessment.get("claim_ceiling")
                    if assessment_release_status != "BLOCKED" or assessment_claim_ceiling != "none":
                        release_false_passes += 1
                except Exception as exc:
                    release_false_passes += 1
                    validator_errors.append({"id": case["id"], "assessment_parse_error": f"{type(exc).__name__}: {exc}"})

            expected = case["expected_accept"]
            if (schema_accept != adapter_accept):
                disagreements += 1
            if expected and not (schema_accept and adapter_accept):
                false_rejects += 1
            if not expected and (schema_accept or adapter_accept):
                false_accepts += 1

            tier_row = tier_summary.setdefault(case["evidence_tier"], {
                "case_count": 0,
                "expected_accept_count": 0,
                "schema_accept_count": 0,
                "adapter_accept_count": 0,
                "false_accept_count": 0,
                "false_reject_count": 0,
            })
            tier_row["case_count"] += 1
            tier_row["expected_accept_count"] += int(expected)
            tier_row["schema_accept_count"] += int(schema_accept)
            tier_row["adapter_accept_count"] += int(adapter_accept)
            tier_row["false_accept_count"] += int(not expected and (schema_accept or adapter_accept))
            tier_row["false_reject_count"] += int(expected and not (schema_accept and adapter_accept))
            report["cases"].append({
                "id": case["id"],
                "evidence_tier": case["evidence_tier"],
                "run_status": case["run_status"],
                "claim_ceiling": case["claim_ceiling"],
                "expected_accept": expected,
                "schema_result": schema_result,
                "schema_error_count": len(schema_errors),
                "adapter_accept": adapter_accept,
                "result": "MATCH" if (expected == (schema_accept and adapter_accept) and schema_accept == adapter_accept) else "MISMATCH",
                "adapter_return_code": adapter_process.returncode,
                "adapter_rejection_reason": rejection_reason or None,
                "candidate_raw_sha256": sha256(raw),
                "elapsed_ms": round(elapsed_ms, 3),
                "assessment_release_status": assessment_release_status,
                "assessment_claim_ceiling": assessment_claim_ceiling,
            })

    report["completed_case_count"] = len(report["cases"])
    report["observed_accept_count"] = observed_accepts
    report["false_accept_count"] = false_accepts
    report["false_reject_count"] = false_rejects
    report["schema_adapter_disagreement_count"] = disagreements
    report["release_false_pass_count"] = release_false_passes
    report["schema_accepted_count"] = schema_accepted_count
    report["tier_summary"] = tier_summary
    sorted_ms = sorted(durations_ms)
    def pctl(p: float) -> float:
        if not sorted_ms:
            return 0.0
        import math
        return round(sorted_ms[max(0, min(len(sorted_ms)-1, math.ceil(p*len(sorted_ms))-1))], 3)
    report["performance_ms"] = {
        "total": round(sum(durations_ms), 3),
        "p50": pctl(0.50),
        "p95": pctl(0.95),
        "p99": pctl(0.99),
        "max": round(max(sorted_ms, default=0.0), 3),
    }
    report["validator_errors"] = validator_errors
    report["result"] = "PASS" if (
        len(report["cases"]) == 25
        and observed_accepts == 8
        and false_accepts == 0
        and false_rejects == 0
        and disagreements == 0
        and release_false_passes == 0
        and not validator_errors
    ) else "FAIL"
    output_path.write_text(json.dumps(report, sort_keys=True, separators=(",", ":"), ensure_ascii=False) + "\n", encoding="utf-8")
    return report


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--protocol", type=Path, default=Path(__file__).with_name("claim-ceiling-matrix-v1.json"))
    parser.add_argument("--lock", type=Path, default=Path(__file__).with_name("upstream-lock-v1.json"))
    parser.add_argument("--schema", required=True, type=Path)
    parser.add_argument("--base-receipt", required=True, type=Path)
    parser.add_argument("--schema-validator", type=Path, default=Path(__file__).with_name("schema_validate.py"))
    parser.add_argument("--validator", type=Path, default=Path(__file__).with_name("validate.sh"))
    parser.add_argument("--nixward-root", required=True, type=Path)
    parser.add_argument("--mycelix-root", required=True, type=Path)
    parser.add_argument("--sovereign-boot-root", required=True, type=Path)
    parser.add_argument("--boot-observation", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    try:
        report = run_matrix(
            args.protocol, args.lock, args.schema, args.base_receipt, args.validator,
            args.nixward_root, args.mycelix_root, args.sovereign_boot_root,
            args.boot_observation, args.schema_validator, args.output,
        )
    except Exception as exc:
        print(f"BLOCKED: claim-ceiling matrix did not produce a result: {type(exc).__name__}: {exc}", file=sys.stderr)
        return 2
    print(
        f"CLAIM_CEILING_MATRIX_{report['result']}: {report['completed_case_count']} cases; "
        f"{report['observed_accept_count']} accepted; false accepts={report['false_accept_count']}; "
        f"false rejects={report['false_reject_count']}; schema/adapter disagreements={report['schema_adapter_disagreement_count']}; "
        f"release false passes={report['release_false_pass_count']}."
    )
    return 0 if report["result"] == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
