#!/usr/bin/env python3
"""Run the network-free vendor endpoint contract gate and emit a redacted evidence manifest.

The manifest records command outcomes and input digests, never stdout/stderr. It is a
local qualification-process receipt, not a signature, CI attestation, or device approval.
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
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
INPUTS = (
    "schemas/vendor-management-endpoint-profile-v1.schema.json",
    "schemas/vendor-management-endpoint-approval-v1.schema.json",
    "tests/fixtures/vendor-management-endpoint-profile-v1.json",
    "tests/fixtures/vendor-management-endpoint-approval-v1.json",
    "tools/validate_vendor_management_endpoint_profile.py",
    "tools/validate_vendor_management_endpoint_approval.py",
    "tests/test_vendor_management_endpoint_profile.py",
    "tests/test_vendor_management_endpoint_approval.py",
)
COMMANDS = (
    ("profile_validator", (sys.executable, "tools/validate_vendor_management_endpoint_profile.py")),
    ("profile_tests", (sys.executable, "-m", "unittest", "discover", "-s", "tests", "-p", "test_vendor_management_endpoint_profile.py")),
    ("approval_validator", (sys.executable, "tools/validate_vendor_management_endpoint_approval.py")),
    ("approval_tests", (sys.executable, "-m", "unittest", "discover", "-s", "tests", "-p", "test_vendor_management_endpoint_approval.py")),
)


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def source_revision() -> str | None:
    supplied = os.environ.get("GITHUB_SHA")
    if supplied and len(supplied) == 40 and all(ch in "0123456789abcdefABCDEF" for ch in supplied):
        return supplied.lower()
    try:
        result = subprocess.run(
            ["git", "rev-parse", "--verify", "HEAD"],
            cwd=ROOT, check=True, text=True, stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL, timeout=5,
        )
        value = result.stdout.strip()
        return value if len(value) == 40 and all(ch in "0123456789abcdefABCDEF" for ch in value) else None
    except (OSError, subprocess.SubprocessError):
        return None


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=Path("artifacts/endpoint-contract-qualification.json"))
    args = parser.parse_args()
    output = args.output if args.output.is_absolute() else ROOT / args.output

    inputs: dict[str, Any] = {}
    missing: list[str] = []
    for relative in INPUTS:
        path = ROOT / relative
        if not path.is_file():
            missing.append(relative)
        else:
            inputs[relative] = {"sha256": sha256_file(path), "bytes": path.stat().st_size}

    results = []
    for name, command in COMMANDS:
        if missing:
            results.append({"name": name, "status": "not_run_missing_input", "exit_code": None})
            continue
        try:
            # Deliberately discard process output: exception text, fixture values, and secrets
            # must never be copied into the durable qualification manifest.
            proc = subprocess.run(
                command, cwd=ROOT, stdin=subprocess.DEVNULL,
                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                timeout=180, check=False,
            )
            results.append({"name": name, "status": "pass" if proc.returncode == 0 else "fail", "exit_code": proc.returncode})
        except subprocess.TimeoutExpired:
            results.append({"name": name, "status": "timeout", "exit_code": None})
        except OSError:
            results.append({"name": name, "status": "could_not_start", "exit_code": None})

    try:
        dependency_versions = {"jsonschema": importlib.metadata.version("jsonschema")}
    except importlib.metadata.PackageNotFoundError:
        dependency_versions = {"jsonschema": None}

    manifest = {
        "schema": "luminous.vendor-endpoint-contract-qualification.v1",
        "created_at": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "source_revision": source_revision(),
        "execution": "network-free subprocess gate; stdout and stderr discarded",
        "runtime": {"python": platform.python_version(), "implementation": platform.python_implementation(), "dependencies": dependency_versions},
        "inputs": inputs,
        "missing_inputs": missing,
        "checks": results,
        "result": "pass" if not missing and all(item["status"] == "pass" for item in results) else "fail",
        "limitations": [
            "does not verify signatures, signer delegation, trust-store freshness, or revocation",
            "does not bind a target digest to fresh independent inventory",
            "does not test hardware, a vendor endpoint, or a real credential",
            "does not establish CI provenance or independently protected artifact integrity",
        ],
    }
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(manifest, sort_keys=True, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"{manifest['result'].upper()}: wrote redacted endpoint contract manifest ({len(results)} checks; source revision {'known' if manifest['source_revision'] else 'unknown'})")
    return 0 if manifest["result"] == "pass" else 1


if __name__ == "__main__":
    raise SystemExit(main())
