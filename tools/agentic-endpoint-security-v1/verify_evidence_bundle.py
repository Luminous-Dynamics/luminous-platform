#!/usr/bin/env python3
"""Verify a downloaded Luminous CI evidence bundle against externally supplied run identity.

Checks consistency and claim ceilings only. SHA-256 files do not authenticate the
workflow, artifact publisher, host kernel, or truthfulness of an observation.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Any

MANIFEST_SCHEMA = "luminous.ci-evidence-bundle-manifest.v1"
HEX40 = re.compile(r"^[0-9a-f]{40}$")
HEX64 = re.compile(r"^[0-9a-f]{64}$")
CHECKSUM_LINE = re.compile(r"^([0-9a-f]{64})  ([A-Za-z0-9._-]+)$")
STEP_OUTCOMES = ("format", "rust_tests", "runtime_test", "clippy")
ALLOWED_STEP_RESULTS = {"success", "failure", "cancelled", "skipped", "not_run"}

RECEIPTS: dict[str, dict[str, Any]] = {
    "luminous.landlock-preopened-fd-result.v1": {
        "scope": "preopened_writable_fd_ordering_microtest",
        "required_true": (
            "preopened_write_succeeded_after_landlock",
            "parent_oracle_observed_preopened_write",
            "cloexec_descriptor_absent_after_exec",
            "new_path_write_open_denied_after_landlock",
            "parent_oracle_safe_control_unchanged",
            "parent_original_fd_cloexec",
        ),
        "false_fields": {"full_descriptor_hygiene_coverage": False},
        "minimum_landlock_abi": 3,
    },
    "luminous.fd-inheritance-boundary-result.v1": {
        "scope": "exec_boundary_descriptor_hygiene_probe_only",
        "required_true": (
            "negative_control_inherited_fd_readable",
            "cloexec_control_descriptor_absent_after_exec",
            "parent_descriptor_survived_marking",
            "parent_descriptor_cloexec_flag_set",
            "close_range_unshare_enabled",
            "close_range_cloexec_enabled",
        ),
        "false_fields": {},
        "minimum_landlock_abi": None,
    },
}

BUNDLE_FILES = {
    "landlock-preopened-fd-result.jsonl",
    "fd-hygiene-result.jsonl",
    "run-context.txt",
    "evidence-manifest.json",
}
RESULT_FILES = {
    "landlock-preopened-fd-result.jsonl",
    "fd-hygiene-result.jsonl",
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def load_json_object(raw: bytes, label: str) -> dict[str, Any]:
    try:
        value = json.loads(raw.decode("utf-8"))
    except (UnicodeError, json.JSONDecodeError) as exc:
        raise ValueError(f"{label} is not valid UTF-8 JSON: {exc}") from exc
    require(isinstance(value, dict), f"{label} must contain a JSON object")
    return value


def parse_context(raw: bytes) -> dict[str, str]:
    try:
        lines = raw.decode("utf-8").splitlines()
    except UnicodeError as exc:
        raise ValueError("run-context.txt is not valid UTF-8") from exc
    context: dict[str, str] = {}
    for line_number, line in enumerate(lines, 1):
        require("=" in line, f"run-context.txt line {line_number} is malformed")
        key, value = line.split("=", 1)
        require(bool(key) and key not in context, f"duplicate or empty context key: {key!r}")
        context[key] = value
    required = {
        "workflow_run_id",
        "workflow_run_attempt",
        "event_name",
        "workflow",
        "expected_checkout_sha",
        "format_outcome",
        "rust_test_outcome",
        "runtime_test_outcome",
        "clippy_outcome",
    }
    require(required == set(context), "run-context.txt fields are missing or unexpected")
    return context


def verify_checksums(root: Path) -> None:
    checksum_path = root / "SHA256SUMS"
    require(checksum_path.is_file() and not checksum_path.is_symlink(),
            "SHA256SUMS is missing or is a symbolic link")
    try:
        lines = checksum_path.read_text(encoding="utf-8").splitlines()
    except (OSError, UnicodeError) as exc:
        raise ValueError(f"cannot read SHA256SUMS: {exc}") from exc
    expected: dict[str, str] = {}
    for index, line in enumerate(lines, 1):
        match = CHECKSUM_LINE.fullmatch(line)
        require(match is not None, f"SHA256SUMS line {index} is malformed")
        digest, filename = match.groups()
        require(filename in BUNDLE_FILES, f"unexpected checksum path: {filename}")
        require(filename not in expected, f"duplicate checksum entry: {filename}")
        expected[filename] = digest
    checksum_names = set(expected)
    result_names = checksum_names & RESULT_FILES
    required_names = {"run-context.txt", "evidence-manifest.json"}
    require(checksum_names == required_names | result_names and len(result_names) == 1,
            "SHA256SUMS must cover run-context, evidence-manifest, and exactly one runtime receipt")

    for filename, expected_digest in expected.items():
        path = root / filename
        require(path.is_file() and not path.is_symlink(),
                f"bundle file is missing or is a symbolic link: {filename}")
        actual_digest = sha256_bytes(path.read_bytes())
        require(actual_digest == expected_digest, f"checksum mismatch: {filename}")


def runtime_receipt_qualifies(
    receipt: dict[str, Any],
    expected_checkout_sha: str,
) -> bool:
    schema = receipt.get("schema")
    contract = RECEIPTS.get(schema)
    require(contract is not None, f"unsupported runtime receipt schema: {schema!r}")
    require(receipt.get("status") in {"pass", "not_emitted"},
            "runtime receipt status must be pass or not_emitted")
    require(receipt.get("scope") == contract["scope"],
            "runtime receipt scope is missing or broader than its contract")
    for key, expected in contract["false_fields"].items():
        require(receipt.get(key) is expected,
                f"runtime receipt claim ceiling is missing or escalated: {key}")

    if receipt["status"] == "not_emitted":
        return False

    checkout_sha = receipt.get("checkout_sha")
    require(isinstance(checkout_sha, str) and HEX40.fullmatch(checkout_sha) is not None,
            "passing receipt must bind a 40-character lowercase checkout SHA")
    require(checkout_sha == expected_checkout_sha,
            "runtime receipt checkout SHA does not match the externally expected SHA")

    kernel_release = receipt.get("kernel_release")
    require(
        isinstance(kernel_release, str)
        and kernel_release not in {"", "unknown"}
        and re.fullmatch(r"[A-Za-z0-9._+-]+", kernel_release) is not None,
        "passing receipt must bind a sanitized kernel release",
    )
    if contract["minimum_landlock_abi"] is not None:
        abi = receipt.get("landlock_abi")
        require(type(abi) is int and abi >= contract["minimum_landlock_abi"],
                "passing Landlock receipt must record ABI >= 3")

    controls_pass = all(receipt.get(key) is True for key in contract["required_true"])
    return controls_pass


def verify_bundle(
    bundle_dir: Path,
    expected_checkout_sha: str,
    expected_run_id: str,
) -> dict[str, Any]:
    require(HEX40.fullmatch(expected_checkout_sha) is not None,
            "expected checkout SHA must be 40 lowercase hexadecimal characters")
    require(expected_run_id.isdigit() and int(expected_run_id) > 0,
            "expected workflow run ID must be a positive integer")
    root = bundle_dir.resolve()
    require(root.is_dir(), "bundle directory does not exist")

    verify_checksums(root)
    manifests = load_json_object((root / "evidence-manifest.json").read_bytes(),
                                 "evidence-manifest.json")
    require(manifests.get("schema") == MANIFEST_SCHEMA, "unsupported evidence manifest schema")
    require(manifests.get("manifest_authenticity") ==
            "not_signed; checksum verifies bundle consistency only",
            "manifest must disclose that checksums do not authenticate the producer")

    require(manifests.get("expected_checkout_sha") == expected_checkout_sha,
            "manifest checkout SHA does not match the externally expected SHA")
    require(str(manifests.get("workflow_run_id")) == expected_run_id,
            "manifest run ID does not match the externally expected run ID")
    attempt = manifests.get("workflow_run_attempt")
    require(isinstance(attempt, str) and attempt.isdigit() and int(attempt) > 0,
            "manifest run attempt must be a positive numeric string")
    require(isinstance(manifests.get("workflow"), str) and manifests["workflow"],
            "manifest workflow name is missing")
    require(isinstance(manifests.get("workflow_sha"), str)
            and HEX40.fullmatch(manifests["workflow_sha"]) is not None,
            "manifest must bind a valid workflow SHA")
    require(manifests.get("event_name") in {"pull_request", "push"},
            "unexpected workflow event")
    require(manifests.get("runner_os") == "Linux", "runtime bundle must identify Linux runner OS")
    require(manifests.get("runner_arch") in {"X64", "ARM64"},
            "runtime bundle must identify a supported runner architecture")

    context = parse_context((root / "run-context.txt").read_bytes())
    require(context["expected_checkout_sha"] == expected_checkout_sha,
            "run-context checkout SHA mismatch")
    require(context["workflow_run_id"] == expected_run_id,
            "run-context run ID mismatch")
    require(context["workflow_run_attempt"] == attempt, "run-context attempt mismatch")
    require(context["event_name"] == manifests["event_name"], "run-context event mismatch")
    require(context["workflow"] == manifests["workflow"], "run-context workflow mismatch")

    outcomes = manifests.get("step_outcomes")
    require(isinstance(outcomes, dict) and set(outcomes) == set(STEP_OUTCOMES),
            "manifest step-outcome inventory is incomplete or unexpected")
    context_outcomes = {
        "format": context["format_outcome"],
        "rust_tests": context["rust_test_outcome"],
        "runtime_test": context["runtime_test_outcome"],
        "clippy": context["clippy_outcome"],
    }
    require(outcomes == context_outcomes, "manifest and run-context outcomes disagree")
    for key, value in outcomes.items():
        require(value in ALLOWED_STEP_RESULTS, f"unsupported step outcome: {key}={value!r}")

    receipt_name = manifests.get("receipt_path")
    require(receipt_name in RESULT_FILES, "manifest receipt path is not one of the known result files")
    require(receipt_name in BUNDLE_FILES, "manifest receipt path is not part of this bundle")
    raw_receipt = (root / receipt_name).read_bytes()
    require(manifests.get("receipt_sha256") == sha256_bytes(raw_receipt),
            "receipt digest does not match evidence-manifest.json")
    receipt_lines = [line for line in raw_receipt.decode("utf-8", errors="replace").splitlines()
                     if line.strip()]
    require(len(receipt_lines) == 1, "runtime result file must contain exactly one nonempty JSON line")
    receipt = load_json_object(receipt_lines[0].encode("utf-8"), receipt_name)
    require(manifests.get("receipt_status") == receipt.get("status"),
            "manifest and runtime receipt status disagree")
    require(manifests.get("receipt_schema") == receipt.get("schema"),
            "manifest and runtime receipt schema disagree")

    receipt_pass = runtime_receipt_qualifies(receipt, expected_checkout_sha)
    steps_pass = all(value == "success" for value in outcomes.values())
    derived_status = (
        "candidate_pass_for_declared_microtest"
        if receipt_pass and steps_pass
        else "not_qualified"
    )
    require(manifests.get("status") == derived_status,
            "manifest qualification status does not follow receipt and step evidence")

    if receipt.get("status") == "pass":
        require(manifests.get("kernel_release") == receipt.get("kernel_release"),
                "manifest kernel release does not match receipt")
    else:
        require(manifests.get("kernel_release") is None,
                "non-emitted receipt cannot claim a kernel release")

    return {
        "schema": MANIFEST_SCHEMA,
        "status": derived_status,
        "expected_checkout_sha": expected_checkout_sha,
        "workflow_run_id": expected_run_id,
        "receipt_schema": receipt.get("schema"),
        "receipt_status": receipt.get("status"),
        "step_outcomes": outcomes,
        "receipt_sha256": manifests["receipt_sha256"],
        "manifest_authenticity": "not_signed; checksum verifies bundle consistency only",
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bundle-dir", type=Path, required=True)
    parser.add_argument("--expected-checkout-sha", required=True)
    parser.add_argument("--expected-run-id", required=True)
    args = parser.parse_args()
    try:
        report = verify_bundle(
            args.bundle_dir,
            args.expected_checkout_sha,
            args.expected_run_id,
        )
    except (OSError, ValueError, KeyError, TypeError) as exc:
        print(f"FAIL evidence bundle: {exc}", file=sys.stderr)
        return 2
    print(json.dumps(report, sort_keys=True))
    if report["status"] != "candidate_pass_for_declared_microtest":
        print("NOT QUALIFIED: bundle is internally consistent but does not meet the micro-test gate.",
              file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
