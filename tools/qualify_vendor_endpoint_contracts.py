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
import stat
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
VALIDATOR_TIMEOUT_SECONDS = 20
TEST_WORKER_TIMEOUT_SECONDS = 30
MAX_CHILD_EXECUTION_BUDGET_SECONDS = 300
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
WORKER_COUNTER_FIELDS = (
    "tests_run",
    "failures",
    "errors",
    "skipped",
    "expected_failures",
    "unexpected_successes",
    "discovery_errors",
)
WORKER_SUMMARY_FIELDS = frozenset({"pattern", "state", *WORKER_COUNTER_FIELDS})
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
    "tools/validate_hardware_portfolio.py",
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
    "SOURCE_TREE_DIRTY",
    "INPUT_FREEZE_FAILED",
    "INPUT_FREEZE_RESTORE_FAILED",
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
    "INPUT_MANIFEST_INCOMPLETE",
}
INCOMPLETE_CODES = {
    "SOURCE_TREE_STATE_UNAVAILABLE",
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


def manifest_file_sha256(root: Path, relative: str) -> str | None:
    """Hash a regular manifest file through no-follow directory descriptors.

    The qualification workflow is Ubuntu-only. If the host lacks the descriptor
    flags required to prevent symlink traversal, fail closed rather than falling
    back to a check-then-open pathname race.
    """
    candidate = Path(relative)
    if candidate.is_absolute() or not candidate.parts or ".." in candidate.parts:
        return None
    if not hasattr(os, "O_NOFOLLOW") or not hasattr(os, "O_DIRECTORY"):
        return None

    directory_flags = os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW
    descriptors: list[int] = []
    try:
        current = os.open(root, directory_flags)
        descriptors.append(current)
        for part in candidate.parts[:-1]:
            current = os.open(part, directory_flags, dir_fd=current)
            descriptors.append(current)
        file_fd = os.open(
            candidate.parts[-1],
            os.O_RDONLY | os.O_NOFOLLOW | getattr(os, "O_NONBLOCK", 0),
            dir_fd=current,
        )
        descriptors.append(file_fd)
        before = os.fstat(file_fd)
        if not stat.S_ISREG(before.st_mode):
            return None

        digest = hashlib.sha256()
        bytes_read = 0
        while True:
            chunk = os.read(file_fd, 1024 * 1024)
            if not chunk:
                break
            digest.update(chunk)
            bytes_read += len(chunk)

        after = os.fstat(file_fd)
        stable_fields = ("st_dev", "st_ino", "st_size", "st_mtime_ns", "st_ctime_ns")
        if (
            bytes_read != before.st_size
            or any(getattr(before, field) != getattr(after, field) for field in stable_fields)
        ):
            return None
        return digest.hexdigest()
    except OSError:
        return None
    finally:
        for descriptor in reversed(descriptors):
            try:
                os.close(descriptor)
            except OSError:
                pass


def input_hashes(root: Path, paths: tuple[str, ...] = INPUT_PATHS) -> dict[str, str | None]:
    result: dict[str, str | None] = {}
    for relative in sorted(paths):
        result[relative] = manifest_file_sha256(root, relative)
    return result


def manifest_digest(hashes: Mapping[str, str | None]) -> str:
    return hashlib.sha256(canonical_json_bytes(dict(hashes))).hexdigest()


def input_manifest_unchanged(
    root: Path,
    expected_hashes: Mapping[str, str | None],
) -> bool:
    """Fence tracked inputs at subprocess boundaries, not only at finalization."""
    return input_hashes(root) == dict(expected_hashes)


def safe_report_destination(
    root: Path,
    requested: Path,
) -> tuple[Path, str | None] | None:
    """Prevent report writes from replacing source, Git metadata, or arbitrary files."""
    root_abs = Path(os.path.abspath(root))
    target = Path(os.path.abspath(requested if requested.is_absolute() else root_abs / requested))
    try:
        # Reject final or ancestor symlinks even when the lexical path is outside
        # the checkout but resolves back into it.
        if target.resolve(strict=False) != target:
            return None
        relative = target.relative_to(root_abs)
    except OSError:
        return None
    except ValueError:
        # A caller-selected destination outside the checkout is outside source state.
        return target, None

    if (
        not relative.parts
        or relative.parts[0] != "artifacts"
        or target.suffix.lower() != ".json"
        or relative.as_posix() in INPUT_PATHS
    ):
        return None

    parent = root_abs
    for part in relative.parts[:-1]:
        parent = parent / part
        if parent.is_symlink() or (parent.exists() and not parent.is_dir()):
            return None
    if target.is_symlink() or (target.exists() and not target.is_file()):
        return None

    if target.exists():
        try:
            existing = json.loads(target.read_text(encoding="utf-8"))
        except (OSError, ValueError):
            return None
        if (
            not isinstance(existing, dict)
            or existing.get("report_schema_version") != REPORT_SCHEMA_VERSION
        ):
            return None

    # Refuse overwriting tracked files even if they are not in INPUT_PATHS.
    if (root_abs / ".git").exists():
        try:
            tracked = subprocess.run(
                ["git", "ls-files", "--error-unmatch", "--", relative.as_posix()],
                cwd=root_abs,
                env=child_environment(),
                stdin=subprocess.DEVNULL,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                timeout=5,
                check=False,
            )
        except (OSError, subprocess.TimeoutExpired):
            return None
        if tracked.returncode != 1:
            return None
    return target, relative.as_posix()


def source_tree_clean(
    root: Path,
    allowed_untracked: tuple[str, ...] = (),
) -> bool | None:
    """Require a clean HEAD checkout except for explicitly permitted output paths."""
    try:
        tracked = subprocess.run(
            ["git", "diff", "--quiet", "HEAD", "--"],
            cwd=root,
            env=child_environment(),
            stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            timeout=5,
            check=False,
        )
        if tracked.returncode == 1:
            return False
        if tracked.returncode != 0:
            return None
        untracked = subprocess.run(
            ["git", "ls-files", "--others", "--"],
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
    if untracked.returncode != 0:
        return None
    observed = set(untracked.stdout.splitlines())
    return not bool(observed.difference(allowed_untracked))


def child_execution_integrity_errors(
    root: Path,
    expected_head: str,
    expected_hashes: Mapping[str, str | None],
    allowed_untracked: tuple[str, ...] = (),
) -> list[str]:
    """Re-bind each completed child to the original Git revision and source state."""
    errors: list[str] = []
    current_head = checked_out_sha(root)
    if current_head is None:
        errors.append("CHECKED_OUT_HEAD_UNAVAILABLE")
    elif current_head != expected_head:
        errors.append("REVISION_MISMATCH")
    tree_state = source_tree_clean(root, allowed_untracked=allowed_untracked)
    if tree_state is False:
        errors.append("SOURCE_TREE_DIRTY")
    elif tree_state is None:
        errors.append("SOURCE_TREE_STATE_UNAVAILABLE")
    if not input_manifest_unchanged(root, expected_hashes):
        errors.append("INPUT_DIGEST_CHANGED")
    return sorted(set(errors))


class InputFreezeError(RuntimeError):
    """Raised when a failed freeze cannot fully restore original permissions."""


def make_input_tree_read_only(
    root: Path,
    paths: tuple[str, ...] = INPUT_PATHS,
) -> dict[str, tuple[int, bool]] | None:
    """Remove write bits from manifest files and their parent directories."""
    modes: dict[str, tuple[int, bool]] = {}
    directories: set[str] = set()
    for relative in paths:
        path = root / relative
        if path.is_symlink() or not path.is_file():
            return None
        try:
            modes[relative] = (stat.S_IMODE(path.stat().st_mode), False)
        except OSError:
            return None
        parent = path.parent
        while parent != root:
            if parent.is_symlink() or not parent.is_dir():
                return None
            key = parent.relative_to(root).as_posix()
            directories.add(key)
            parent = parent.parent
    for relative in directories:
        try:
            modes[relative] = (stat.S_IMODE((root / relative).stat().st_mode), True)
        except OSError:
            return None

    try:
        for relative in sorted(paths):
            path = root / relative
            os.chmod(path, modes[relative][0] & ~0o222)
        for relative in sorted(directories, key=lambda item: item.count("/"), reverse=True):
            path = root / relative
            os.chmod(path, modes[relative][0] & ~0o222)
    except OSError as exc:
        if not restore_input_tree_permissions(root, modes):
            raise InputFreezeError("permission restoration failed after input-freeze error") from exc
        return None
    return modes


def restore_input_tree_permissions(
    root: Path,
    modes: Mapping[str, tuple[int, bool]],
) -> bool:
    """Restore original permission bits after all child processes have stopped."""
    ok = True
    entries = sorted(
        modes.items(),
        key=lambda item: (item[1][1], item[0].count("/")),
    )
    for relative, (mode, _is_directory) in entries:
        path = root / relative
        try:
            if path.is_symlink() or not path.exists():
                ok = False
                continue
            os.chmod(path, mode)
        except OSError:
            ok = False
    return ok


def manifest_coverage_gaps(
    validators: Mapping[str, str] | None = None,
    test_suite_patterns: tuple[str, ...] | None = None,
    input_paths: tuple[str, ...] | None = None,
) -> list[str]:
    """Return invoked qualification code not bound into the frozen input manifest."""
    validator_paths = VALIDATORS if validators is None else validators
    suite_patterns = TEST_SUITE_PATTERNS if test_suite_patterns is None else test_suite_patterns
    manifest_paths = INPUT_PATHS if input_paths is None else input_paths
    required = {
        "tools/qualify_vendor_endpoint_contracts.py",
        *validator_paths.values(),
        *(f"tests/{pattern}" for pattern in suite_patterns),
    }
    return sorted(required.difference(manifest_paths))


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
    if set(summary) != WORKER_SUMMARY_FIELDS:
        return False
    if summary.get("state") != "passed":
        return False
    pattern = summary.get("pattern")
    if not isinstance(pattern, str) or pattern not in TEST_SUITE_PATTERNS:
        return False
    for field in WORKER_COUNTER_FIELDS:
        value = summary.get(field)
        # bool is a subclass of int in Python; JSON booleans must never count.
        if type(value) is not int or value < 0:
            return False
        if field == "tests_run":
            if value < 1:
                return False
        elif value != 0:
            return False
    return True


def worker_summary_errors(
    summary: object,
    expected_pattern: str,
    process_exit_code: object,
) -> list[str]:
    """Validate child evidence as a closed, state/exit-code-consistent contract."""
    if not isinstance(summary, dict):
        return ["SUMMARY_NOT_OBJECT"]
    if set(summary) != WORKER_SUMMARY_FIELDS:
        return ["SUMMARY_FIELDS_MISMATCH"]
    if (
        expected_pattern not in TEST_SUITE_PATTERNS
        or type(summary.get("pattern")) is not str
        or summary.get("pattern") != expected_pattern
    ):
        return ["SUMMARY_PATTERN_MISMATCH"]
    state = summary.get("state")
    if state not in ("passed", "failed", "runner_error"):
        return ["SUMMARY_STATE_INVALID"]
    if type(process_exit_code) is not int:
        return ["WORKER_EXIT_CODE_INVALID"]

    counters: dict[str, int] = {}
    for field in WORKER_COUNTER_FIELDS:
        value = summary.get(field)
        if type(value) is not int or value < 0:
            return ["SUMMARY_COUNTER_INVALID"]
        counters[field] = value

    if state == "passed":
        if not successful_suite(summary):
            return ["PASSED_SUMMARY_NOT_CLEAN"]
        if process_exit_code != 0:
            return ["PASSED_SUMMARY_EXIT_MISMATCH"]
        return []

    if state == "failed":
        if process_exit_code != 1:
            return ["FAILED_SUMMARY_EXIT_MISMATCH"]
        failure_counts = (
            "failures", "errors", "skipped", "expected_failures",
            "unexpected_successes", "discovery_errors",
        )
        if counters["tests_run"] > 0 and not any(counters[name] for name in failure_counts):
            return ["FAILED_SUMMARY_HAS_NO_FAILURE_EVIDENCE"]
        return []

    # The only runner_error shape emitted by this worker is deliberately fixed;
    # it records a failed runner/discovery path without exposing exception text.
    expected_runner_error = {
        "tests_run": 0,
        "failures": 0,
        "errors": 1,
        "skipped": 0,
        "expected_failures": 0,
        "unexpected_successes": 0,
        "discovery_errors": 1,
    }
    if process_exit_code != 2 or counters != expected_runner_error:
        return ["RUNNER_ERROR_SUMMARY_INCONSISTENT"]
    return []


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
                timeout=TEST_WORKER_TIMEOUT_SECONDS,
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
        if worker_summary_errors(summary, pattern, process.returncode):
            # Never preserve malformed child counters/state in the qualification report.
            return ({"pattern": pattern, "state": "worker_summary_invalid"}, True)
        summary["worker_exit_code"] = process.returncode
        return summary, summary["state"] != "passed"


def run_validator(root: Path, relative_script: str) -> dict[str, object]:
    try:
        process = subprocess.run(
            [sys.executable, str(root / relative_script)],
            cwd=root,
            env=child_environment(),
            stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            timeout=VALIDATOR_TIMEOUT_SECONDS,
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
            timeout=3,
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
    resolved_report = safe_report_destination(root, args.report)
    if resolved_report is None:
        print("ERROR: unsafe report destination")
        return 2
    report_path, relative_report = resolved_report
    allowed_untracked = (relative_report,) if relative_report is not None else ()
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

    manifest_gaps = manifest_coverage_gaps()
    if manifest_gaps:
        blockers.append("INPUT_MANIFEST_INCOMPLETE")

    before_hashes = input_hashes(root)
    missing_inputs = [path for path, digest in before_hashes.items() if digest is None]
    if missing_inputs:
        blockers.append("INPUT_MISSING")
    before_manifest_digest = manifest_digest(before_hashes)

    source_tree_state = source_tree_clean(root, allowed_untracked=allowed_untracked)
    if source_tree_state is False:
        blockers.append("SOURCE_TREE_DIRTY")
    elif source_tree_state is None:
        blockers.append("SOURCE_TREE_STATE_UNAVAILABLE")

    revision_valid = observed_sha is not None and (not expected_sha or expected_sha == observed_sha)
    execution_ready = (
        source_tree_state is True
        and revision_valid
        and not manifest_gaps
        and not missing_inputs
    )
    input_modes: dict[str, tuple[int, bool]] | None = None
    input_modes_restored: bool | None = None
    if execution_ready:
        try:
            input_modes = make_input_tree_read_only(root)
        except InputFreezeError:
            blockers.extend(("INPUT_FREEZE_FAILED", "INPUT_FREEZE_RESTORE_FAILED"))
            input_modes_restored = False
            execution_ready = False
        else:
            if input_modes is None:
                blockers.append("INPUT_FREEZE_FAILED")
                # No freeze completed: the helper either made no changes or
                # rolled back its partial changes before returning None.
                input_modes_restored = True
                execution_ready = False

    validators: dict[str, dict[str, object]] = {}
    input_drift_detected = False
    for name, script in VALIDATORS.items():
        if not execution_ready:
            validators[name] = {
                "script": script,
                "return_code": None,
                "timed_out": False,
                "state": "blocked_source_integrity",
            }
            continue
        if input_drift_detected:
            validators[name] = {
                "script": script,
                "return_code": None,
                "timed_out": False,
                "state": "blocked_execution_integrity_lost",
            }
            continue
        result = run_validator(root, script)
        integrity_errors = child_execution_integrity_errors(
            root, observed_sha or "", before_hashes, allowed_untracked
        )
        if integrity_errors:
            blockers.extend(integrity_errors)
            input_drift_detected = True
            # Do not preserve the child's result when the revision or source fence moved.
            result_state = (
                "input_drift_detected"
                if integrity_errors == ["INPUT_DIGEST_CHANGED"]
                else "execution_integrity_lost"
            )
            validators[name] = {**result, "state": result_state}
        else:
            validators[name] = result

    stopped_validator_states = {
        "input_drift_detected", "execution_integrity_lost",
        "blocked_execution_integrity_lost", "blocked_source_integrity"
    }
    if validators["endpoint_profile"]["state"] not in ({"passed"} | stopped_validator_states):
        blockers.append("PROFILE_VALIDATOR_FAILED")
    if validators["endpoint_approval_binding"]["state"] not in ({"passed"} | stopped_validator_states):
        blockers.append("APPROVAL_VALIDATOR_FAILED")
    if validators["capability_registry"]["state"] not in ({"passed"} | stopped_validator_states):
        blockers.append("CAPABILITY_REGISTRY_FAILED")
    if validators["mutation_receipt"]["state"] not in ({"passed"} | stopped_validator_states):
        blockers.append("RECEIPT_VALIDATOR_FAILED")

    test_suites: list[dict[str, object]] = []
    for pattern in TEST_SUITE_PATTERNS:
        if not execution_ready:
            test_suites.append({"pattern": pattern, "state": "blocked_source_integrity"})
            continue
        if input_drift_detected:
            test_suites.append({"pattern": pattern, "state": "blocked_execution_integrity_lost"})
            continue
        summary, failed = run_test_suite(root, pattern)
        integrity_errors = child_execution_integrity_errors(
            root, observed_sha or "", before_hashes, allowed_untracked
        )
        if integrity_errors:
            blockers.extend(integrity_errors)
            input_drift_detected = True
            # Invalidate this suite's own result as well as all remaining suites.
            result_state = (
                "input_drift_detected"
                if integrity_errors == ["INPUT_DIGEST_CHANGED"]
                else "execution_integrity_lost"
            )
            test_suites.append({"pattern": pattern, "state": result_state})
            continue
        test_suites.append(summary)
        if failed:
            if str(summary.get("state", "")).startswith("worker_") or summary.get("state") == "runner_error":
                blockers.append("TEST_WORKER_FAILED")
            else:
                blockers.append("TEST_SUITE_FAILED")

    if input_modes is not None:
        input_modes_restored = restore_input_tree_permissions(root, input_modes)
        if not input_modes_restored:
            blockers.append("INPUT_FREEZE_RESTORE_FAILED")
    if execution_ready:
        # Final fence after permission restoration, before the report itself is written.
        # Catch source/head drift during cleanup as well as during child execution.
        blockers.extend(
            child_execution_integrity_errors(
                root, observed_sha or "", before_hashes, allowed_untracked
            )
        )
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
        "source_integrity": {
            "working_tree_clean": source_tree_state,
            "manifest_write_bits_removed_during_children": input_modes is not None,
            "original_permissions_restored": input_modes_restored,
            "preflight_execution_gate_passed": execution_ready,
            "execution_integrity_preserved": (
                execution_ready
                and input_modes is not None
                and input_modes_restored is True
                and not input_drift_detected
                and before_hashes == after_hashes
            ),
        },
        "missing_inputs": missing_inputs,
        "manifest_coverage_gaps": manifest_gaps,
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
            "manifest files and parent directories have write bits removed during child checks; this is not an OS-enforced sandbox, and a same-user child may defeat mode bits (persistent changes are caught by digest fences)",
        ],
        "diagnostics_policy": "raw stdout, stderr, test identifiers and traceback text are not persisted",
    }

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
