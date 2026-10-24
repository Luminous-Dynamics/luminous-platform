#!/usr/bin/env python3
"""Validate the exact installed wheel set against the schema test environment lock."""
from __future__ import annotations

import argparse
import hashlib
import json
import platform
import re
import sys
from pathlib import Path
from typing import Any
from urllib.parse import urlparse

from schema_validate import load_strict_json

REPORT_SCHEMA = "srq-schema-environment-validation-v1"
LOCK_SCHEMA = "srq-schema-environment-lock-v1"


def sha256(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def canonical_name(name: str) -> str:
    return re.sub(r"[-_.]+", "-", name).lower()


def install_hash(item: dict[str, Any]) -> str | None:
    info = item.get("download_info")
    if not isinstance(info, dict):
        return None
    archive = info.get("archive_info")
    if not isinstance(archive, dict):
        return None
    hashes = archive.get("hashes")
    if isinstance(hashes, dict) and isinstance(hashes.get("sha256"), str):
        return hashes["sha256"].lower()
    legacy = archive.get("hash")
    if isinstance(legacy, str) and legacy.startswith("sha256="):
        return legacy.partition("=")[2].lower()
    return None


def validate_environment(
    lock: Any,
    install_report: Any,
    *,
    python_version: str,
    os_name: str,
    machine: str,
) -> tuple[list[dict[str, str]], list[dict[str, str]]]:
    errors: list[dict[str, str]] = []
    installed_summary: list[dict[str, str]] = []

    def error(code: str, message: str) -> None:
        errors.append({"code": code, "message": message})

    if not isinstance(lock, dict) or lock.get("schema") != LOCK_SCHEMA or lock.get("version") != 1:
        return ([{"code": "lock-identity", "message": "environment lock schema/version mismatch"}], [])
    runtime = lock.get("runtime")
    if not isinstance(runtime, dict):
        return ([{"code": "lock-runtime", "message": "lock runtime must be an object"}], [])
    if runtime.get("os") != os_name:
        error("runtime-os", f"expected OS {runtime.get('os')!r}; observed {os_name!r}")
    if runtime.get("machine") != machine:
        error("runtime-machine", f"expected machine {runtime.get('machine')!r}; observed {machine!r}")
    expected_minor = runtime.get("python_minor")
    observed_minor = ".".join(python_version.split(".")[:2])
    if observed_minor != expected_minor:
        error("runtime-python", f"expected Python {expected_minor!r}; observed {observed_minor!r}")

    packages = lock.get("packages")
    if not isinstance(packages, list) or lock.get("package_count") != len(packages):
        return (errors + [{"code": "lock-package-count", "message": "package_count must equal packages length"}], [])
    expected: dict[str, dict[str, Any]] = {}
    for package in packages:
        if not isinstance(package, dict):
            error("lock-package-shape", "each locked package must be an object")
            continue
        name = package.get("name")
        if not isinstance(name, str) or not name:
            error("lock-package-name", "locked package name must be a non-empty string")
            continue
        key = canonical_name(name)
        if key in expected:
            error("lock-package-duplicate", f"duplicate locked package {name!r}")
        expected[key] = package
        if not isinstance(package.get("version"), str) or not isinstance(package.get("wheel"), str):
            error("lock-package-fields", f"locked package {name!r} requires version and wheel")
        if not isinstance(package.get("sha256"), str) or not re.fullmatch(r"[0-9a-f]{64}", package["sha256"]):
            error("lock-package-hash", f"locked package {name!r} has invalid SHA-256")

    records = install_report.get("install") if isinstance(install_report, dict) else None
    if not isinstance(records, list):
        return (errors + [{"code": "install-report-shape", "message": "pip install report must contain an install array"}], installed_summary)

    observed: dict[str, dict[str, Any]] = {}
    for record in records:
        if not isinstance(record, dict):
            error("install-record-shape", "each pip install record must be an object")
            continue
        metadata = record.get("metadata")
        if not isinstance(metadata, dict) or not isinstance(metadata.get("name"), str):
            error("install-record-metadata", "pip record requires metadata.name")
            continue
        name = metadata["name"]
        key = canonical_name(name)
        if key in observed:
            error("install-duplicate", f"pip report contains duplicate package {name!r}")
            continue
        observed[key] = record
        expected_record = expected.get(key)
        if expected_record is None:
            error("install-unexpected", f"unexpected package installed: {name!r}")
            continue

        version = metadata.get("version")
        if version != expected_record.get("version"):
            error("install-version", f"{name}: expected {expected_record.get('version')!r}; observed {version!r}")
        info = record.get("download_info")
        if not isinstance(info, dict):
            error("install-download-info", f"{name}: missing download_info")
            continue
        url = info.get("url")
        filename = Path(urlparse(url).path).name if isinstance(url, str) else ""
        if filename != expected_record.get("wheel"):
            error("install-wheel", f"{name}: expected wheel {expected_record.get('wheel')!r}; observed {filename!r}")
        observed_hash = install_hash(record)
        if observed_hash != expected_record.get("sha256"):
            error("install-hash", f"{name}: SHA-256 mismatch (expected {expected_record.get('sha256')!r}, observed {observed_hash!r})")
        if isinstance(version, str) and isinstance(filename, str) and observed_hash is not None:
            installed_summary.append({
                "name": key,
                "version": version,
                "wheel": filename,
                "sha256": observed_hash,
            })

    for key, package in expected.items():
        if key not in observed:
            error("install-missing", f"required locked package not installed: {package.get('name', key)!r}")

    return sorted(errors, key=lambda item: (item["code"], item["message"])), sorted(installed_summary, key=lambda item: item["name"])


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--lock", required=True, type=Path)
    parser.add_argument("--install-report", required=True, type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()

    try:
        lock, lock_raw = load_strict_json(args.lock, maximum=4 * 1024 * 1024)
        report, report_raw = load_strict_json(args.install_report, maximum=16 * 1024 * 1024)
        errors, installed = validate_environment(
            lock,
            report,
            python_version=platform.python_version(),
            os_name=sys.platform,
            machine=platform.machine(),
        )
        result = "INVALID" if errors else "VALID"
        output: dict[str, Any] = {
            "schema": REPORT_SCHEMA,
            "result": result,
            "environment_lock": {
                "path_label": args.lock.name,
                "sha256": sha256(lock_raw),
                "bytes": len(lock_raw),
            },
            "pip_install_report": {
                "path_label": args.install_report.name,
                "sha256": sha256(report_raw),
                "bytes": len(report_raw),
            },
            "runtime": {
                "python_version": platform.python_version(),
                "os": sys.platform,
                "machine": platform.machine(),
                "expected_package_count": lock.get("package_count") if isinstance(lock, dict) else None,
                "observed_package_count": len(report.get("install", [])) if isinstance(report, dict) and isinstance(report.get("install"), list) else None,
            },
            "installed_packages": installed,
            "errors": errors,
            "limitations": [
                "The report proves conformance to the repository's package wheel hashes, not that packages are benign.",
                "Runtime is deliberately scoped to ubuntu-24.04 / CPython 3.12 / Linux x86_64.",
                "This test-only environment has no production runtime authority."
            ],
        }
    except Exception as exc:
        result = "ERROR"
        output = {
            "schema": REPORT_SCHEMA,
            "result": "ERROR",
            "error": f"{type(exc).__name__}: {exc}",
            "errors": [],
            "limitations": ["No package environment conformance result was produced."],
        }

    encoded = json.dumps(output, sort_keys=True, separators=(",", ":"), ensure_ascii=False) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(encoded, encoding="utf-8")
    print(encoded, end="")
    return 0 if result == "VALID" else (1 if result == "INVALID" else 2)


if __name__ == "__main__":
    raise SystemExit(main())
