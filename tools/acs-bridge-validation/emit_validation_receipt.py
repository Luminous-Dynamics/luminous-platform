#!/usr/bin/env python3
"""Emit a machine-readable receipt for ACS schema/corpus CI.

The receipt records execution and byte-identity evidence only. It never claims
ACS-Core conformance, host-hook enforcement, Nixward runtime qualification, or
disposable-VM qualification.
"""
from __future__ import annotations

import ast
import hashlib
import json
import os
import platform
import re
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
SOURCE_PATHS = (
    ".github/workflows/acs-nixward-bridge-corpus.yml",
    "docs/fixtures/acs-v0.1.0-schema-lock.json",
    "docs/fixtures/acs-nixward-bridge-v1-cases.json",
    "docs/schemas/acs-nixward-bridge-test-corpus-v1.schema.json",
    "tools/acs-bridge-validation/emit_validation_receipt.py",
    "tools/acs-bridge-validation/validate_corpus.py",
    "tools/acs-bridge-validation/verify_schema_lock.py",
    "tests/acs-bridge-validation/test_emit_validation_receipt.py",
    "tests/acs-bridge-validation/test_validate_corpus.py",
    "tests/acs-bridge-validation/test_verify_schema_lock.py",
)
TEST_PATHS = (
    "tests/acs-bridge-validation/test_emit_validation_receipt.py",
    "tests/acs-bridge-validation/test_validate_corpus.py",
    "tests/acs-bridge-validation/test_verify_schema_lock.py",
)
LOCK_PATH = "docs/fixtures/acs-v0.1.0-schema-lock.json"
CORPUS_PATH = "docs/fixtures/acs-nixward-bridge-v1-cases.json"
CORPUS_SCHEMA_PATH = "docs/schemas/acs-nixward-bridge-test-corpus-v1.schema.json"
VENDOR_PREFIX = "vendor/agent-control-standard/"
EXPECTED_VENDOR_FILES = 45


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON object key: {key!r}")
        result[key] = value
    return result


def _strict_json_bytes(raw: bytes, path: str) -> Any:
    return json.loads(
        raw.decode("utf-8", errors="strict"),
        object_pairs_hook=_unique_object,
        parse_constant=lambda value: (_ for _ in ()).throw(
            ValueError(f"non-standard JSON numeric constant: {value}")
        ),
    )


def _sha256(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def _git_blob_sha(raw: bytes) -> str:
    return hashlib.sha1(f"blob {len(raw)}\0".encode("ascii") + raw).hexdigest()


def _test_inventory(root: Path) -> dict[str, Any]:
    files: list[dict[str, Any]] = []
    total = 0
    errors: list[str] = []
    for relative in TEST_PATHS:
        path = root / relative
        try:
            tree = ast.parse(path.read_text(encoding="utf-8"), filename=relative)
        except (OSError, UnicodeError, SyntaxError) as exc:
            errors.append(f"{relative}: cannot inventory test functions: {exc}")
            files.append({"path": relative, "test_names": [], "count": 0})
            continue
        names = sorted(
            node.name
            for node in ast.walk(tree)
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef))
            and node.name.startswith("test_")
        )
        files.append({"path": relative, "test_names": names, "count": len(names)})
        total += len(names)
    return {"files": files, "count": total, "errors": errors}


def _safe_vendor_path(root: Path, value: Any) -> tuple[Path | None, str | None]:
    if not isinstance(value, str) or not value.startswith(VENDOR_PREFIX):
        return None, "vendored path is outside the expected ACS package"
    parts = Path(value).parts
    if ".." in parts or Path(value).is_absolute():
        return None, "vendored path contains an unsafe component"
    vendor_root = (root / "vendor/agent-control-standard").resolve()
    candidate = root / value
    try:
        resolved = candidate.resolve(strict=True)
        resolved.relative_to(vendor_root)
    except (OSError, ValueError):
        return None, "vendored file is missing or resolves outside the ACS package"
    if candidate.is_symlink() or not resolved.is_file():
        return None, "vendored path is not a regular, non-symlink file"
    return resolved, None


def _bundle_evidence(root: Path, lock: Any) -> dict[str, Any]:
    entries: list[dict[str, Any]] = []
    errors: list[str] = []
    if not isinstance(lock, dict) or not isinstance(lock.get("files"), list):
        return {"manifest_entry_count": 0, "expected_file_count": EXPECTED_VENDOR_FILES,
                "matched_file_count": 0, "all_files_match": False, "files": [],
                "errors": ["schema lock is not an object with a files array"]}
    for item in lock["files"]:
        if not isinstance(item, dict):
            errors.append("schema lock contains a non-object entry")
            continue
        relative = item.get("vendored_path")
        safe_path, path_error = _safe_vendor_path(root, relative)
        if path_error:
            errors.append(f"{relative!r}: {path_error}")
            entries.append({"path": relative, "matched": False, "error": path_error})
            continue
        try:
            raw = safe_path.read_bytes()  # type: ignore[union-attr]
        except OSError as exc:
            errors.append(f"{relative!r}: unreadable: {exc}")
            entries.append({"path": relative, "matched": False, "error": "unreadable"})
            continue
        actual = {
            "git_blob_sha": _git_blob_sha(raw),
            "sha256": _sha256(raw),
            "size_bytes": len(raw),
        }
        expected = {
            "git_blob_sha": item.get("git_blob_sha"),
            "sha256": item.get("sha256"),
            "size_bytes": item.get("upstream_size_bytes"),
        }
        matched = actual == expected
        if not matched:
            errors.append(f"{relative}: actual byte identity differs from the lock")
        entries.append({"path": relative, "expected": expected, "actual": actual, "matched": matched})
    matched_count = sum(1 for item in entries if item.get("matched") is True)
    return {
        "manifest_entry_count": len(lock["files"]),
        "expected_file_count": EXPECTED_VENDOR_FILES,
        "matched_file_count": matched_count,
        "all_files_match": len(lock["files"]) == EXPECTED_VENDOR_FILES
            and len(entries) == EXPECTED_VENDOR_FILES
            and matched_count == EXPECTED_VENDOR_FILES,
        "upstream_repository": lock.get("upstream_repository"),
        "upstream_release_tag": lock.get("upstream_release_tag"),
        "upstream_release_commit": lock.get("upstream_release_commit"),
        "upstream_tree_object": lock.get("upstream_tree_object"),
        "upstream_commit_signature": lock.get("upstream_commit_signature"),
        "files": entries,
        "errors": errors,
    }


def _outcome_status(
    outcomes: dict[str, str],
    *,
    test_summary_valid: bool,
    bundle_valid: bool,
    corpus_valid: bool,
    sources_complete: bool,
) -> str:
    """Return PASS only on complete positive evidence; otherwise fail closed."""
    if any(value in {"failure", "cancelled"} for value in outcomes.values()):
        return "FAIL"
    if not all(value == "success" for value in outcomes.values()):
        return "BLOCKED"
    if not test_summary_valid or not bundle_valid or not corpus_valid or not sources_complete:
        return "FAIL"
    return "PASS"


def build_receipt(root: Path, env: dict[str, str], test_log: str) -> dict[str, Any]:
    outcomes = {
        "unit_tests": env.get("ACS_UNIT_TESTS_OUTCOME", "unknown"),
        "schema_lock_verifier": env.get("ACS_SCHEMA_LOCK_OUTCOME", "unknown"),
        "corpus_validator": env.get("ACS_CORPUS_OUTCOME", "unknown"),
    }
    inventory = _test_inventory(root)
    match = re.search(r"^Ran\s+(\d+)\s+tests?\b", test_log, re.MULTILINE)
    reported_count = int(match.group(1)) if match else None
    test_ok = bool(re.search(r"^OK\s*$", test_log, re.MULTILINE))
    test_failed = bool(re.search(r"^FAILED(?:\s|$)", test_log, re.MULTILINE))
    test_summary_valid = (
        reported_count is not None
        and reported_count == inventory["count"]
        and not inventory["errors"]
        and test_ok
        and not test_failed
    )

    source_hashes: list[dict[str, Any]] = []
    source_errors: list[str] = []
    for relative in SOURCE_PATHS:
        try:
            raw = (root / relative).read_bytes()
            source_hashes.append({"path": relative, "sha256": _sha256(raw), "size_bytes": len(raw)})
        except OSError as exc:
            source_errors.append(f"{relative}: unreadable: {exc}")

    lock: Any = None
    corpus: Any = None
    json_errors: list[str] = []
    for relative, target_name in ((LOCK_PATH, "lock"), (CORPUS_PATH, "corpus")):
        try:
            value = _strict_json_bytes((root / relative).read_bytes(), relative)
            if target_name == "lock":
                lock = value
            else:
                corpus = value
        except (OSError, UnicodeError, json.JSONDecodeError, ValueError) as exc:
            json_errors.append(f"{relative}: strict JSON read failed: {exc}")

    bundle = _bundle_evidence(root, lock)
    case_ids: list[str] = []
    corpus_count: int | None = None
    if isinstance(corpus, dict) and isinstance(corpus.get("cases"), list):
        corpus_count = len(corpus["cases"])
        case_ids = [
            str(case.get("id")) for case in corpus["cases"]
            if isinstance(case, dict) and isinstance(case.get("id"), str)
        ]
    corpus_valid = (
        corpus_count == 32
        and case_ids == [f"BR-{number:03d}" for number in range(1, 33)]
        and not json_errors
    )
    sources_complete = not source_errors and not inventory["errors"]
    status = _outcome_status(
        outcomes,
        test_summary_valid=test_summary_valid,
        bundle_valid=bundle.get("all_files_match") is True,
        corpus_valid=corpus_valid,
        sources_complete=sources_complete,
    )
    claims = {
        "schema_and_corpus_ci": status,
        "acs_core_conformance": "NOT_EVALUATED",
        "acs_wire_authentication_and_replay_protection": "NOT_EVALUATED",
        "host_hook_failure_enforcement": "NOT_EVALUATED",
        "nixward_runtime_integration": "NOT_EVALUATED",
        "indeterminate_effect_reconciliation": "NOT_EVALUATED",
        "disposable_vm_qualification": "NOT_EVALUATED",
        "upstream_publisher_authenticity": "UNVERIFIED_UNSIGNED_SOURCE",
    }
    run_id = env.get("GITHUB_RUN_ID")
    repository = env.get("GITHUB_REPOSITORY")
    server_url = env.get("GITHUB_SERVER_URL", "https://github.com").rstrip("/")
    return {
        "receipt_format": "luminous-acs-nixward-validation-receipt-v1",
        "status": status,
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "run": {
            "repository": repository,
            "workflow": env.get("GITHUB_WORKFLOW"),
            "run_id": run_id,
            "run_attempt": env.get("GITHUB_RUN_ATTEMPT"),
            "event_name": env.get("GITHUB_EVENT_NAME"),
            "ref": env.get("GITHUB_REF"),
            "head_sha": env.get("GITHUB_SHA"),
            "url": f"{server_url}/{repository}/actions/runs/{run_id}" if repository and run_id else None,
        },
        "runtime": {"python": platform.python_version(), "platform": platform.platform()},
        "checks": {
            "step_outcomes": outcomes,
            "tests_expected_from_source": inventory["count"],
            "tests_reported_by_unittest": reported_count,
            "test_summary_valid": test_summary_valid,
            "test_log_sha256": _sha256(test_log.encode("utf-8")) if test_log else None,
            "source_test_inventory": inventory["files"],
        },
        "inputs": {
            "source_files": source_hashes,
            "source_file_errors": source_errors,
            "corpus": {
                "path": CORPUS_PATH,
                "format": corpus.get("corpus_format") if isinstance(corpus, dict) else None,
                "sha256": _sha256((root / CORPUS_PATH).read_bytes()) if (root / CORPUS_PATH).is_file() else None,
                "case_count": corpus_count,
                "case_ids": case_ids,
            },
            "corpus_schema": {
                "path": CORPUS_SCHEMA_PATH,
                "sha256": _sha256((root / CORPUS_SCHEMA_PATH).read_bytes()) if (root / CORPUS_SCHEMA_PATH).is_file() else None,
            },
            "schema_lock": {
                "path": LOCK_PATH,
                "sha256": _sha256((root / LOCK_PATH).read_bytes()) if (root / LOCK_PATH).is_file() else None,
                "bundle_integrity": bundle,
            },
            "strict_json_errors": json_errors,
        },
        "claims": claims,
        "limitations": [
            "This receipt records CI inputs and structural validation only.",
            "It does not establish ACS-Core conformance or authenticate the upstream publisher.",
            "It does not evaluate ACS wire signatures, replay protection, host hooks, Nixward runtime effects, recovery, or a disposable VM.",
            "A file digest establishes byte identity, not truth, trustworthiness, or authorization.",
        ],
    }


def main() -> int:
    env = dict(os.environ)
    log_path = env.get("ACS_TEST_LOG")
    test_log = ""
    if log_path:
        try:
            test_log = Path(log_path).read_text(encoding="utf-8", errors="replace")
        except OSError:
            pass
    output = env.get("ACS_RECEIPT_OUTPUT")
    if not output:
        print("FAIL: ACS_RECEIPT_OUTPUT is required", file=sys.stderr)
        return 2
    try:
        receipt = build_receipt(ROOT, env, test_log)
        target = Path(output)
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    except Exception as exc:  # receipt generation failure must fail the workflow
        print(f"FAIL: unable to emit validation receipt: {exc}", file=sys.stderr)
        return 2
    print(f"{receipt['status']}: emitted {receipt['receipt_format']} to {output}")
    if receipt["status"] != "PASS":
        print("LIMIT: receipt is not a passing schema/corpus CI record; inspect checks and source errors.", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
