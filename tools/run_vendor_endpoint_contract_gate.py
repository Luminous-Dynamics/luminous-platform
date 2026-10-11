#!/usr/bin/env python3
"""Run the vendor endpoint contract gate and emit a secret-safe JSON receipt.

The receipt records outcomes and input digests, never captured command output.
It is evidence of this run only; it is not a signature or authorization decision.
"""

from __future__ import annotations

import hashlib
import json
import os
import platform
import re
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "artifacts" / "vendor-endpoint-contract-gate.json"
INPUTS = (
    "schemas/vendor-management-endpoint-profile-v1.schema.json",
    "schemas/vendor-management-endpoint-approval-v1.schema.json",
    "tests/fixtures/vendor-management-endpoint-profile-v1.json",
    "tests/fixtures/vendor-management-endpoint-approval-v1.json",
    "tools/validate_vendor_management_endpoint_profile.py",
    "tools/validate_vendor_management_endpoint_approval.py",
    "tests/test_vendor_management_endpoint_profile.py",
    "tests/test_vendor_management_endpoint_approval.py",
    ".github/workflows/hardware-portfolio.yml",
)
COMMANDS = (
    ("profile_validator", [sys.executable, "tools/validate_vendor_management_endpoint_profile.py"]),
    ("profile_tests", [sys.executable, "-m", "unittest", "discover", "-s", "tests", "-p", "test_vendor_management_endpoint_profile.py", "-v"]),
    ("approval_validator", [sys.executable, "tools/validate_vendor_management_endpoint_approval.py"]),
    ("approval_tests", [sys.executable, "-m", "unittest", "discover", "-s", "tests", "-p", "test_vendor_management_endpoint_approval.py", "-v"]),
)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def current_commit() -> str | None:
    supplied = os.environ.get("GITHUB_SHA")
    if supplied and re.fullmatch(r"[0-9a-fA-F]{40,64}", supplied):
        return supplied.lower()
    try:
        result = subprocess.run(
            ["git", "rev-parse", "HEAD"], cwd=ROOT, check=True,
            stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True, timeout=5,
        )
        value = result.stdout.strip()
        return value if re.fullmatch(r"[0-9a-fA-F]{40,64}", value) else None
    except (OSError, subprocess.SubprocessError):
        return None


def test_counts(output: str) -> dict[str, int | None]:
    # unittest writes its summary to stderr; count is parsed only for numeric
    # summary fields. Individual test names and captured output are never stored.
    match = re.search(r"Ran (\d+) tests? in [0-9.]+s", output)
    failures = re.search(r"failures=(\d+)", output)
    errors = re.search(r"errors=(\d+)", output)
    skipped = re.search(r"skipped=(\d+)", output)
    return {
        "tests_run": int(match.group(1)) if match else None,
        "failures": int(failures.group(1)) if failures else 0,
        "errors": int(errors.group(1)) if errors else 0,
        "skipped": int(skipped.group(1)) if skipped else 0,
        "summary_parsed": match is not None,
    }


def run_one(name: str, command: list[str]) -> dict[str, Any]:
    try:
        result = subprocess.run(
            command, cwd=ROOT, stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
            text=True, errors="replace", timeout=180, check=False,
            env={**os.environ, "PYTHONDONTWRITEBYTECODE": "1"},
        )
        record: dict[str, Any] = {"name": name, "return_code": result.returncode}
        if name.endswith("_tests"):
            record.update(test_counts(result.stdout))
        # Deliberately omit stdout/stderr, exception text, and user-controlled
        # input values. A failed command is represented by a code, not its log.
        return record
    except subprocess.TimeoutExpired:
        return {"name": name, "return_code": None, "timed_out": True}
    except OSError:
        return {"name": name, "return_code": None, "launch_error": True}


def main() -> int:
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    missing = [path for path in INPUTS if not (ROOT / path).is_file()]
    results = [run_one(name, command) for name, command in COMMANDS] if not missing else []
    files = {
        path: {"sha256": sha256(ROOT / path), "bytes": (ROOT / path).stat().st_size}
        for path in INPUTS if (ROOT / path).is_file()
    }
    failed = bool(missing) or len(results) != len(COMMANDS) or any(
        item.get("return_code") != 0 for item in results
    )
    report = {
        "contract": "luminous-vendor-endpoint-contract-gate-v1",
        "status": "fail" if failed else "pass",
        "evidence_class": "network_free_local_contract_checks",
        "authorization_effect": "none",
        "commit": current_commit(),
        "started_at_utc": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "runtime": {
            "python": platform.python_version(),
            "implementation": platform.python_implementation(),
            "platform": sys.platform,
        },
        "missing_inputs": missing,
        "inputs": files,
        "commands": results,
        "limitations": [
            "No vendor endpoint or physical device was contacted.",
            "No cryptographic signature, signer delegation, revocation, live inventory identity, or capability qualification was verified.",
            "A passing receipt is not an approval to connect or mutate a device.",
        ],
    }
    OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps({
        "status": report["status"],
        "receipt": str(OUTPUT.relative_to(ROOT)),
        "commands": len(results),
        "failed_commands": sum(item.get("return_code") != 0 for item in results),
        "missing_inputs": len(missing),
        "commit_available": report["commit"] is not None,
    }, sort_keys=True))
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
