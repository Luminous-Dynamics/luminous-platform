#!/usr/bin/env python3
"""Unit tests for fail-closed test-environment package identity validation."""
from __future__ import annotations

import copy
import unittest

from schema_env_validate import validate_environment


def sample() -> tuple[dict, dict]:
    lock = {
        "schema": "srq-schema-environment-lock-v1",
        "version": 1,
        "runtime": {"runner": "ubuntu-24.04", "os": "linux", "machine": "x86_64", "python_minor": "3.12"},
        "package_count": 2,
        "packages": [
            {"name": "alpha-pkg", "version": "1.0", "wheel": "alpha_pkg-1.0-py3-none-any.whl", "sha256": "a" * 64},
            {"name": "beta-pkg", "version": "2.0", "wheel": "beta_pkg-2.0-py3-none-any.whl", "sha256": "b" * 64},
        ],
    }

    def record(name: str, version: str, wheel: str, sha: str) -> dict:
        return {
            "metadata": {"name": name, "version": version},
            "download_info": {
                "url": "https://files.pythonhosted.org/packages/aa/" + wheel,
                "archive_info": {"hashes": {"sha256": sha}},
            },
        }

    report = {
        "install": [
            record("Alpha_Pkg", "1.0", "alpha_pkg-1.0-py3-none-any.whl", "a" * 64),
            record("beta-pkg", "2.0", "beta_pkg-2.0-py3-none-any.whl", "b" * 64),
        ]
    }
    return lock, report


class SchemaEnvironmentLockTests(unittest.TestCase):
    def test_matching_install_report_passes(self) -> None:
        lock, report = sample()
        errors, packages = validate_environment(
            lock, report, python_version="3.12.10", os_name="linux", machine="x86_64"
        )
        self.assertEqual(errors, [])
        self.assertEqual(len(packages), 2)

    def test_missing_package_is_rejected(self) -> None:
        lock, report = sample()
        report["install"].pop()
        errors, _ = validate_environment(
            lock, report, python_version="3.12.10", os_name="linux", machine="x86_64"
        )
        self.assertIn("install-missing", {item["code"] for item in errors})

    def test_unexpected_package_is_rejected(self) -> None:
        lock, report = sample()
        extra = copy.deepcopy(report["install"][0])
        extra["metadata"]["name"] = "unexpected"
        extra["download_info"]["url"] = "https://files.pythonhosted.org/unexpected-0.whl"
        report["install"].append(extra)
        errors, _ = validate_environment(
            lock, report, python_version="3.12.10", os_name="linux", machine="x86_64"
        )
        self.assertIn("install-unexpected", {item["code"] for item in errors})

    def test_wrong_version_is_rejected(self) -> None:
        lock, report = sample()
        report["install"][0]["metadata"]["version"] = "1.1"
        errors, _ = validate_environment(
            lock, report, python_version="3.12.10", os_name="linux", machine="x86_64"
        )
        self.assertIn("install-version", {item["code"] for item in errors})

    def test_wrong_wheel_filename_is_rejected(self) -> None:
        lock, report = sample()
        report["install"][0]["download_info"]["url"] = "https://files.pythonhosted.org/alpha_pkg-1.0-wrong.whl"
        errors, _ = validate_environment(
            lock, report, python_version="3.12.10", os_name="linux", machine="x86_64"
        )
        self.assertIn("install-wheel", {item["code"] for item in errors})

    def test_wrong_artifact_hash_is_rejected(self) -> None:
        lock, report = sample()
        report["install"][0]["download_info"]["archive_info"]["hashes"]["sha256"] = "c" * 64
        errors, _ = validate_environment(
            lock, report, python_version="3.12.10", os_name="linux", machine="x86_64"
        )
        self.assertIn("install-hash", {item["code"] for item in errors})

    def test_wrong_python_minor_is_rejected(self) -> None:
        lock, report = sample()
        errors, _ = validate_environment(
            lock, report, python_version="3.13.0", os_name="linux", machine="x86_64"
        )
        self.assertIn("runtime-python", {item["code"] for item in errors})

    def test_wrong_machine_is_rejected(self) -> None:
        lock, report = sample()
        errors, _ = validate_environment(
            lock, report, python_version="3.12.10", os_name="linux", machine="aarch64"
        )
        self.assertIn("runtime-machine", {item["code"] for item in errors})

    def test_duplicate_install_records_are_rejected(self) -> None:
        lock, report = sample()
        report["install"].append(copy.deepcopy(report["install"][0]))
        errors, _ = validate_environment(
            lock, report, python_version="3.12.10", os_name="linux", machine="x86_64"
        )
        self.assertIn("install-duplicate", {item["code"] for item in errors})

    def test_locked_sha256_must_be_well_formed(self) -> None:
        lock, report = sample()
        lock["packages"][0]["sha256"] = "not-a-digest"
        errors, _ = validate_environment(
            lock, report, python_version="3.12.10", os_name="linux", machine="x86_64"
        )
        self.assertIn("lock-package-hash", {item["code"] for item in errors})


if __name__ == "__main__":
    unittest.main(verbosity=2)
