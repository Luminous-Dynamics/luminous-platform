#!/usr/bin/env python3
"""Create a source-bound Ambient qualification evidence receipt.

The receipt records observed gates and never upgrades a renderer's declared
qualification state merely because contract validation passed.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
from datetime import datetime, timezone
from importlib.metadata import PackageNotFoundError, version
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_RECEIPT_SCHEMA = ROOT / "contracts/ambient-qualification-receipt-v1.schema.json"
DEFAULT_PROFILE = ROOT / "contracts/examples/sovereign-visual-core.source-inspection.profile.json"
INPUTS = {
    "scenePackSchema": "contracts/ambient-scene-pack-v1.schema.json",
    "rendererProfileSchema": "contracts/renderer-capability-profile-v1.schema.json",
    "manifestFixture": "contracts/examples/first-germination.scene.json",
    "rendererProfile": "contracts/examples/sovereign-visual-core.source-inspection.profile.json",
}
GATE_NAMES = (
    "exactHead", "schemaValidation", "contractTests", "rendererBuild",
    "deterministicReplay", "resourceChecks", "lifecycleChecks", "visualCapture",
)


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def outcome_status(outcome: str) -> str:
    return {
        "success": "pass",
        "failure": "fail",
        "cancelled": "cancelled",
        "skipped": "not-run",
        "not-run": "not-run",
        "queued": "queued",
        "stale-head": "stale-head",
        "pass": "pass",
        "fail": "fail",
    }[outcome]


def derive_evidence_level(gates: dict[str, dict[str, Any]]) -> str:
    """Return the strongest evidence level supported by all required gates."""
    if gates["exactHead"]["status"] != "pass":
        return "insufficient"
    if not all(gates[name]["status"] == "pass" for name in ("schemaValidation", "contractTests")):
        return "insufficient"
    if gates["rendererBuild"]["status"] != "pass":
        return "contract-validated"
    runtime_gates = ("deterministicReplay", "resourceChecks", "lifecycleChecks", "visualCapture")
    if all(gates[name]["status"] == "pass" for name in runtime_gates):
        return "runtime-qualified"
    return "compile-qualified"


def read_actual_head(root: Path) -> str:
    result = subprocess.run(
        ["git", "-C", str(root), "rev-parse", "HEAD"],
        check=False, capture_output=True, text=True,
    )
    if result.returncode != 0:
        raise ValueError(f"cannot read git HEAD: {result.stderr.strip()}")
    actual = result.stdout.strip()
    if len(actual) != 40 or any(ch not in "0123456789abcdef" for ch in actual):
        raise ValueError(f"git returned an invalid HEAD SHA: {actual!r}")
    return actual


def build_receipt(
    root: Path,
    *,
    expected_commit: str,
    actual_commit: str,
    schema_outcome: str,
    tests_outcome: str,
    run_id: str,
    run_attempt: str,
    runner_os: str,
    workflow_url: str,
) -> dict[str, Any]:
    if expected_commit != actual_commit:
        raise ValueError(
            f"stale-head: expected {expected_commit}, checked out {actual_commit}; refusing to issue receipt"
        )
    if len(expected_commit) != 40 or any(ch not in "0123456789abcdef" for ch in expected_commit):
        raise ValueError("expected commit must be a 40-character lowercase Git SHA")
    if not run_id.isdigit() or not run_attempt.isdigit():
        raise ValueError("workflow run ID and attempt must be decimal strings")

    profile_path = root / INPUTS["rendererProfile"]
    profile = json.loads(profile_path.read_text(encoding="utf-8"))
    adapter = profile["adapter"]
    renderer = profile["renderer"]
    run_evidence = workflow_url or None

    gates: dict[str, dict[str, Any]] = {
        "exactHead": {
            "status": "pass",
            "detail": "checked-out Git HEAD matched the expected exact source SHA",
            "evidenceUrl": run_evidence,
        },
        "schemaValidation": {
            "status": outcome_status(schema_outcome),
            "detail": "Scene Pack v1 schema and example validation workflow step",
            "evidenceUrl": run_evidence,
        },
        "contractTests": {
            "status": outcome_status(tests_outcome),
            "detail": "ambient_validation unittest discovery workflow step",
            "evidenceUrl": run_evidence,
        },
    }
    for gate_name, detail in (
        ("rendererBuild", "renderer build is not part of this workflow"),
        ("deterministicReplay", "deterministic renderer replay is not part of this workflow"),
        ("resourceChecks", "runtime resource qualification is not part of this workflow"),
        ("lifecycleChecks", "host lifecycle qualification is not part of this workflow"),
        ("visualCapture", "rendered visual capture is not part of this workflow"),
    ):
        gates[gate_name] = {"status": "not-run", "detail": detail, "evidenceUrl": None}

    try:
        jsonschema_version = version("jsonschema")
    except PackageNotFoundError:
        jsonschema_version = "not-installed"

    workflow_repo = os.environ.get("GITHUB_REPOSITORY", "Luminous-Dynamics/luminous-platform")
    server_url = os.environ.get("GITHUB_SERVER_URL", "https://github.com").rstrip("/")
    resolved_workflow_url = workflow_url or (
        f"{server_url}/{workflow_repo}/actions/runs/{run_id}"
    )

    # Fill each gate with the resolved run URL; not-run gates intentionally have
    # no evidence link because no runtime artifact was produced for that gate.
    for gate_name in ("exactHead", "schemaValidation", "contractTests"):
        gates[gate_name]["evidenceUrl"] = resolved_workflow_url

    receipt = {
        "schemaVersion": 1,
        "receiptKind": "ambient-qualification-evidence",
        "createdAtUtc": datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z"),
        "subject": {
            "repository": workflow_repo,
            "commit": expected_commit,
        },
        "adapter": {
            "id": adapter["id"],
            "profileId": profile["profileId"],
            "rendererRepository": renderer["repository"],
            "rendererCommit": renderer["commit"],
            "sourceFiles": adapter["sourceFiles"],
        },
        "inputs": {
            name: {"path": relative_path, "sha256": sha256_file(root / relative_path)}
            for name, relative_path in INPUTS.items()
        },
        "workflow": {"runId": run_id, "runAttempt": run_attempt, "url": resolved_workflow_url},
        "toolchain": {
            "runnerOs": runner_os or os.environ.get("RUNNER_OS", "unknown"),
            "pythonVersion": sys.version.split()[0],
            "jsonschemaVersion": jsonschema_version,
        },
        "gates": gates,
        "derivedEvidenceLevel": derive_evidence_level(gates),
        "declaredRendererQualificationState": adapter["qualificationState"],
        "nonClaims": [
            "Contract validation does not prove the renderer builds or runs.",
            "This receipt does not qualify compositor, lockscreen, suspend/wake, or display-sleep behavior.",
            "A runtime-qualified evidence level requires all declared runtime gates to pass with exact-head identity.",
            "Git blob SHA-1 values identify source files; they are not signatures or attestations.",
        ],
    }

    schema_path = root / DEFAULT_RECEIPT_SCHEMA.relative_to(ROOT)
    schema = json.loads(schema_path.read_text(encoding="utf-8"))
    errors = sorted(
        Draft202012Validator(schema, format_checker=FormatChecker()).iter_errors(receipt),
        key=lambda error: (tuple(map(str, error.absolute_path)), error.message),
    )
    if errors:
        detail = "\n".join(
            f"/{'/'.join(map(str, error.absolute_path))}: {error.message}" for error in errors
        )
        raise ValueError(f"generated receipt did not validate against its schema:\n{detail}")
    return receipt


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    create = subparsers.add_parser("create", help="create a receipt for the exact checked-out revision")
    create.add_argument("--expected-commit", required=True)
    create.add_argument("--schema-outcome", choices=("success", "failure", "cancelled", "skipped"), required=True)
    create.add_argument("--tests-outcome", choices=("success", "failure", "cancelled", "skipped"), required=True)
    create.add_argument("--run-id", default=os.environ.get("GITHUB_RUN_ID", "0"))
    create.add_argument("--run-attempt", default=os.environ.get("GITHUB_RUN_ATTEMPT", "1"))
    create.add_argument("--runner-os", default=os.environ.get("RUNNER_OS", "unknown"))
    create.add_argument("--workflow-url", default=os.environ.get("AMBIENT_WORKFLOW_URL", ""))
    create.add_argument("--output", type=Path, required=True)
    args = parser.parse_args(argv)

    try:
        actual_commit = read_actual_head(ROOT)
        receipt = build_receipt(
            ROOT,
            expected_commit=args.expected_commit,
            actual_commit=actual_commit,
            schema_outcome=args.schema_outcome,
            tests_outcome=args.tests_outcome,
            run_id=args.run_id,
            run_attempt=args.run_attempt,
            runner_os=args.runner_os,
            workflow_url=args.workflow_url,
        )
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    except (OSError, ValueError, KeyError, json.JSONDecodeError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 2
    print(f"receipt={args.output}")
    print(f"subject_commit={receipt['subject']['commit']}")
    print(f"derived_evidence_level={receipt['derivedEvidenceLevel']}")
    print(f"renderer_qualification_state={receipt['declaredRendererQualificationState']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
