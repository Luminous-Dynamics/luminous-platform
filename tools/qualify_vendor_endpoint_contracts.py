#!/usr/bin/env python3
"""Run exact-head, redacted qualification checks for synthetic vendor endpoints.

This runner validates the checked-in synthetic endpoint profile, approval records,
capability registry, mutation receipt, and their local contract tests. It does not contact an endpoint, verify
a cryptographic approval, authorize a target, or qualify an adapter/device.
Raw validator/test output and traceback text are intentionally discarded.
"""
from __future__ import annotations

import argparse
import hashlib
import importlib.metadata
import json
import os
import platform
import subprocess
import sys
import tempfile
import unittest
from datetime import datetime, timezone
from pathlib import Path
from typing import Mapping, TextIO

ROOT = Path(__file__).resolve().parents[1]
REPORT_SCHEMA_VERSION = "luminous.vendor-endpoint-qualification-report.v1"
EXPECTED_PYTHON_VERSION = "3.13.7"
EXPECTED_JSONSCHEMA_VERSION = "4.26.0"
EXPECTED_CRYPTOGRAPHY_VERSION = "50.0.2"
VALIDATORS = {
    "endpoint_profile": "tools/validate_vendor_management_endpoint_profile.py",
    "endpoint_approval_binding": "tools/validate_vendor_management_endpoint_approval.py",
    "capability_registry": "tools/validate_vendor_adapter_registry.py",
    "mutation_receipt": "tools/validate_vendor_operation_receipt.py",
}
TEST_SUITE_PATTERNS = (
    "test_vendor_management_endpoint_profile.py",
    "test_vendor_management_endpoint_approval.py",
    "test_vendor_management_endpoint_approval_signature.py",
    "test_vendor_adapter_registry.py",
    "test_vendor_operation_receipt.py",
    "test_vendor_endpoint_qualification_runner.py",
)
INPUT_PATHS = (
    ".github/workflows/hardware-portfolio.yml",
    "requirements-hardware-portfolio.txt",
    "schemas/vendor-management-endpoint-profile-v1.schema.json",
    "schemas/vendor-management-endpoint-approval-v1.schema.json",
    "schemas/vendor-management-approval-trust-store-v1.schema.json",
    "schemas/vendor-adapter-capability-registry-v1.schema.json",
    "schemas/vendor-operation-effect-receipt-v1.schema.json",
    "security/vendor-adapter-capability-registry-v1.json",
    "tests/fixtures/vendor-operation-effect-receipt-prepared-v1.json",
    "tests/fixtures/vendor-management-endpoint-profile-v1.json",
    "tests/fixtures/vendor-management-endpoint-approval-v1.json",
    "tools/validate_vendor_management_endpoint_profile.py",
    "tools/validate_vendor_management_endpoint_approval.py",
    "tools/verify_vendor_management_endpoint_approval_signature.py",
    "tools/validate_vendor_adapter_registry.py",
    "tools/validate_vendor_operation_receipt.py",
    "tests/test_vendor_management_endpoint_profile.py",
    "tests/test_vendor_management_endpoint_approval.py",
    "tests/test_vendor_management_endpoint_approval_signature.py",
    "tests/test_vendor_adapter_registry.py",
    "tests/test_vendor_operation_receipt.py",
    "tests/test_vendor_endpoint_qualification_runner.py",
    "docs/GLOBAL_VENDOR_INTEROPERABILITY_AND_FIRMWARE_SERVICES_V1.md",
    "docs/VENDOR_ADAPTER_EXECUTION_CONTRACT_V1.md",
    "docs/VENDOR_ADAPTER_PILOT_EXECUTION_PLAYBOOK_V1.md",
    "tools/qualify_vendor_endpoint_contracts.py",
)
FAILURE_CODES = {
    "INPUT_MISSING",
    "INPUT_DIGEST_CHANGED",
    "REVISION_MISMATCH",
    "PYTHON_VERSION_MISMATCH",
    "JSONSCHEMA_VERSION_MISMATCH",
    "CRYPTOGRAPHY_VERSION_MISMATCH",
    "PROFILE_VALIDATOR_FAILED",
    "APPROVAL_VALIDATOR_FAILED",
    "TEST_SUITE_FAILED",
    "TEST_WORKER_FAILED",
    "CAPABILITY_REGISTRY_FAILED",
    "RECEIPT_VALIDATOR_FAILED",
}
INCOMPLETE_CODES = {
    "CHECKED_OUT_HEAD_UNAVAILABLE",
    "COMMIT_SHA_UNAVAILABLE",
    "JSONSCHEMA_VERSION_UNAVAILABLE",
    "CRYPTOGRAPHY_VERSION_UNAVAILABLE",
}


class _DiscardingStream:
    """Accept unittest output without retaining diagnostics or tracebacks."""

    def write(self, value: str) -> int:
        return len(value)

    def flush(self) -> None:
        return None


def canonical_json_bytes(value: object) -> bytes:
    return json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=False, allow_nan=False
    ).encode("utf-8")


def input_hashes(root: Path, paths: tuple[str, ...] = INPUT_PATHS) -> dict[str, str | None]:
    result: dict[str, str | None] = {}
    for relative in sorted(paths):
        try:
            result[relative] = hashlib.sha256((root / relative).read_bytes()).hexdigest()
        except OSError:
            result[relative] = None
    return result


def manifest_digest(hashes: Mapping[str, str | None]) -> str:
    return hashlib.sha256(canonical_json_bytes(dict(hashes))).hexdigest()


def child_environment(source: Mapping[str, str] | None = None) -> dict[str, str]:
    """Create a minimal child environment; do not pass tokens or ambient config."""
    source = os.environ if source is None else source
    env = {
        "LANG": "C.UTF-8",
        "LC_ALL": "C.UTF-8",
        "TZ": "UTC",
        "PYTHONNOUSERSITE": "1",
        "PYTHONDONTWRITEBYTECODE": "1",
    }
    if source.get("PATH"):
        env["PATH"] = source["PATH"]
    # Windows compatibility is harmless; the qualification workflow is Ubuntu-only.
    for name in ("SYSTEMROOT", "WINDIR"):
        if source.get(name):
            env[name] = source[name]
    return env


def successful_suite(summary: Mapping[str, object]) -> bool:
    integer_fields = (
        "tests_run", "failures", "errors", "skipped",
        "expected_failures", "unexpected_successes", "discovery_errors",
    )
    if summary.get("state") != "passed":
        return False
    if not isinstance(summary.get("tests_run"), int) or summary["tests_run"] < 1:
        return False
    for field in integer_fields[1:]:
        value = summary.get(field)
        if not isinstance(value, int) or value != 0:
            return False
    return True


def qualification_status(blocker_codes: list[str] | tuple[str, ...]) -> str:
    codes = set(blocker_codes)
    if codes.intersection(FAILURE_CODES):
        return "failed"
    if codes:
        return "incomplete"
    return "passed"


def _write_worker_summary(path: Path, summary: dict[str, object]) -> None:
    path.write_text(
        json.dumps(summary, sort_keys=True, separators=(",", ":"), allow_nan=False) + "\n",
        encoding="utf-8",
    )


def _run_test_worker(pattern: str, summary_path: Path) -> int:
    """Child-only test entry point. It emits aggregate counts, never details."""
    try:
        os.chdir(ROOT)
        loader = unittest.TestLoader()
        suite = loader.discover(start_dir="tests", pattern=pattern, top_level_dir=".")
        result = unittest.TextTestRunner(stream=_DiscardingStream(), verbosity=0).run(suite)
        summary: dict[str, object] = {
            "pattern": pattern,
            "tests_run": result.testsRun,
            "failures": len(result.failures),
            "errors": len(result.errors),
            "skipped": len(result.skipped),
            "expected_failures": len(result.expectedFailures),
            "unexpected_successes": len(result.unexpectedSuccesses),
            "discovery_errors": len(loader.errors),
        }
        summary["state"] = (
            "passed" if not loader.errors and successful_suite(
                {**summary, "state": "passed" if result.wasSuccessful() else "failed"}
            ) else "failed"
        )
        _write_worker_summary(summary_path, summary)
        return 0 if summary["state"] == "passed" else 1
    except BaseException:
        # Do not serialize exception messages: test failures can contain input data.
        try:
            _write_worker_summary(summary_path, {
                "pattern": pattern,
                "tests_run": 0,
                "failures": 0,
                "errors": 1,
                "skipped": 0,
                "expected_failures": 0,
                "unexpected_successes": 0,
                "discovery_errors": 1,
                "state": "runner_error",
            })
        except BaseException:
            pass
        return 2


def run_test_suite(root: Path, pattern: str) -> tuple[dict[str, object], bool]:
    with tempfile.TemporaryDirectory(prefix="vendor-endpoint-qualification-") as temp_dir:
        summary_path = Path(temp_dir) / "summary.json"
        try:
            process = subprocess.run(
                [
                    sys.executable, str(Path(__file__).resolve()),
                    "--test-suite", pattern, "--summary-file", str(summary_path),
                ],
                cwd=root,
                env=child_environment(),
                stdin=subprocess.DEVNULL,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                timeout=240,
                check=False,
            )
        except subprocess.TimeoutExpired:
            return ({"pattern": pattern, "state": "worker_timeout"}, True)
        except OSError:
            return ({"pattern": pattern, "state": "worker_unavailable"}, True)
        try:
            summary = json.loads(summary_path.read_text(encoding="utf-8"))
        except (OSError, ValueError):
            return ({"pattern": pattern, "state": "worker_summary_unavailable"}, True)
        if not isinstance(summary, dict) or summary.get("pattern") != pattern:
            return ({"pattern": pattern, "state": "worker_summary_invalid"}, True)
        summary["worker_exit_code"] = process.returncode
        failed = process.returncode != 0 or not successful_suite(summary)
        return summary, failed


def run_validator(root: Path, relative_script: str) -> dict[str, object]:
    try:
        process = subprocess.run(
            [sys.executable, str(root / relative_script)],
            cwd=root,
            env=child_environment(),
            stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            timeout=60,
            check=False,
        )
        return {
            "script": relative_script,
            "return_code": process.returncode,
            "timed_out": False,
            "state": "passed" if process.returncode == 0 else "failed",
        }
    except subprocess.TimeoutExpired:
        return {"script": relative_script, "return_code": None, "timed_out": True, "state": "timeout"}
    except OSError:
        return {"script": relative_script, "return_code": None, "timed_out": False, "state": "unavailable"}


def checked_out_sha(root: Path) -> str | None:
    try:
        process = subprocess.run(
            ["git", "rev-parse", "HEAD"],
            cwd=root,
            env=child_environment(),
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            timeout=5,
            check=False,
            text=True,
        )
    except (OSError, subprocess.TimeoutExpired):
        return None
    value = process.stdout.strip()
    if process.returncode != 0 or len(value) not in (40, 64) or any(c not in "0123456789abcdef" for c in value):
        return None
    return value


def dependency_version(distribution: str) -> str | None:
    try:
        return importlib.metadata.version(distribution)
    except importlib.metadata.PackageNotFoundError:
        return None


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--report",
        type=Path,
        default=Path("artifacts/vendor-endpoint-qualification.json"),
        help="Report destination (only aggregate outcomes and input digests are written).",
    )
    parser.add_argument("--test-suite", choices=TEST_SUITE_PATTERNS, help=argparse.SUPPRESS)
    parser.add_argument("--summary-file", type=Path, help=argparse.SUPPRESS)
    args = parser.parse_args(argv)

    if args.test_suite:
        if not args.summary_file:
            return 2
        return _run_test_worker(args.test_suite, args.summary_file)
    if args.summary_file:
        return 2

    root = ROOT
    os.chdir(root)
    started_at = datetime.now(timezone.utc).isoformat(timespec="seconds")
    expected_sha = os.environ.get("GITHUB_SHA")
    observed_sha = checked_out_sha(root)
    commit_sha = expected_sha or observed_sha
    blockers: list[str] = []
    if observed_sha is None:
        blockers.append("CHECKED_OUT_HEAD_UNAVAILABLE")
    if commit_sha is None:
        blockers.append("COMMIT_SHA_UNAVAILABLE")
    if expected_sha and observed_sha and expected_sha != observed_sha:
        blockers.append("REVISION_MISMATCH")

    python_version = platform.python_version()
    jsonschema_version = dependency_version("jsonschema")
    cryptography_version = dependency_version("cryptography")
    if python_version != EXPECTED_PYTHON_VERSION:
        blockers.append("PYTHON_VERSION_MISMATCH")
    if jsonschema_version is None:
        blockers.append("JSONSCHEMA_VERSION_UNAVAILABLE")
    elif jsonschema_version != EXPECTED_JSONSCHEMA_VERSION:
        blockers.append("JSONSCHEMA_VERSION_MISMATCH")
    if cryptography_version is None:
        blockers.append("CRYPTOGRAPHY_VERSION_UNAVAILABLE")
    elif cryptography_version != EXPECTED_CRYPTOGRAPHY_VERSION:
        blockers.append("CRYPTOGRAPHY_VERSION_MISMATCH")

    before_hashes = input_hashes(root)
    missing_inputs = [path for path, digest in before_hashes.items() if digest is None]
    if missing_inputs:
        blockers.append("INPUT_MISSING")
    before_manifest_digest = manifest_digest(before_hashes)

    validators = {
        name: run_validator(root, script) for name, script in VALIDATORS.items()
    }
    if validators["endpoint_profile"]["state"] != "passed":
        blockers.append("PROFILE_VALIDATOR_FAILED")
    if validators["endpoint_approval_binding"]["state"] != "passed":
        blockers.append("APPROVAL_VALIDATOR_FAILED")
    if validators["capability_registry"]["state"] != "passed":
        blockers.append("CAPABILITY_REGISTRY_FAILED")
    if validators["mutation_receipt"]["state"] != "passed":
        blockers.append("RECEIPT_VALIDATOR_FAILED")

    test_suites: list[dict[str, object]] = []
    for pattern in TEST_SUITE_PATTERNS:
        summary, failed = run_test_suite(root, pattern)
        test_suites.append(summary)
        if failed:
            if str(summary.get("state", "")).startswith("worker_") or summary.get("state") == "runner_error":
                blockers.append("TEST_WORKER_FAILED")
            else:
                blockers.append("TEST_SUITE_FAILED")

    after_hashes = input_hashes(root)
    if before_hashes != after_hashes:
        blockers.append("INPUT_DIGEST_CHANGED")

    blocker_codes = sorted(set(blockers))
    report = {
        "report_schema_version": REPORT_SCHEMA_VERSION,
        "scope": "synthetic_endpoint_profile_approval_signature_capability_registry_and_mutation_receipt_contract_tests_only",
        "started_at_utc": started_at,
        "finished_at_utc": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "revision": {
            "github_sha": expected_sha,
            "checked_out_head_sha": observed_sha,
            "bound_commit_sha": commit_sha,
            "github_sha_matches_checked_out_head": (
                expected_sha == observed_sha if expected_sha and observed_sha else None
            ),
        },
        "runtime": {
            "python_version": python_version,
            "expected_python_version": EXPECTED_PYTHON_VERSION,
            "implementation": platform.python_implementation(),
            "platform": platform.system() + "-" + platform.machine(),
            "jsonschema_version": jsonschema_version,
            "expected_jsonschema_version": EXPECTED_JSONSCHEMA_VERSION,
            "cryptography_version": cryptography_version,
            "expected_cryptography_version": EXPECTED_CRYPTOGRAPHY_VERSION,
        },
        "input_sha256": before_hashes,
        "input_manifest_sha256": before_manifest_digest,
        "inputs_unchanged_after_execution": before_hashes == after_hashes,
        "missing_inputs": missing_inputs,
        "validators": validators,
        "test_suites": test_suites,
        "blocker_codes": blocker_codes,
        "status": qualification_status(blocker_codes),
        "limitations": [
            "synthetic fixture validation only",
            "no endpoint or device contacted by the runner",
            "no production signature or signer trust material evaluated (only ephemeral synthetic verifier tests)",
            "no complete decision-revocation/supersession ledger evaluated",
            "no revocation or authorization-at-use verification",
            "no fresh target inventory binding, tested-capability scope, or adapter qualification",
        ],
        "diagnostics_policy": "raw stdout, stderr, test identifiers and traceback text are not persisted",
    }

    report_path = args.report if args.report.is_absolute() else root / args.report
    try:
        report_path.parent.mkdir(parents=True, exist_ok=True)
        with tempfile.NamedTemporaryFile(
            mode="w", encoding="utf-8", newline="\n", dir=report_path.parent,
            prefix=".vendor-endpoint-qualification-", suffix=".tmp", delete=False,
        ) as temporary:
            json.dump(report, temporary, sort_keys=True, indent=2, ensure_ascii=False, allow_nan=False)
            temporary.write("\n")
            temporary_path = Path(temporary.name)
        os.replace(temporary_path, report_path)
    except OSError:
        print("ERROR: unable to write qualification report")
        return 2

    test_count = sum(int(item.get("tests_run", 0)) for item in test_suites)
    failure_count = sum(int(item.get("failures", 0)) + int(item.get("errors", 0)) for item in test_suites)
    print(
        "vendor endpoint qualification: "
        + report["status"].upper()
        + f"; validators={sum(item['state'] == 'passed' for item in validators.values())}/{len(validators)}"
        + f"; tests={test_count}; test_failures_or_errors={failure_count}"
        + f"; blockers={','.join(blocker_codes) if blocker_codes else 'none'}"
        + "; report_written=yes"
    )
    return 0 if report["status"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
