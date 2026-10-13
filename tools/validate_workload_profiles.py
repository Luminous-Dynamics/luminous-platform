#!/usr/bin/env python3
"""Validate workload-profile examples and security-negative test cases.

This checks the profile schema contract only. It does not provision containers,
create VMs, fetch guest images, or qualify a runtime on physical hardware.
"""

from __future__ import annotations

import copy
import json
import sys
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator


ROOT = Path(__file__).resolve().parents[1]
SCHEMA_PATH = ROOT / "schemas" / "workload-profile.schema.json"
PROFILE_DIR = ROOT / "profiles" / "workloads"


def load_json(path: Path) -> Any:
    try:
        with path.open("r", encoding="utf-8") as source:
            return json.load(source)
    except (OSError, json.JSONDecodeError) as exc:
        raise ValueError(f"{path.relative_to(ROOT)}: unable to read valid JSON: {exc}") from exc


def describe_errors(errors: list[Any]) -> str:
    lines: list[str] = []
    for error in sorted(errors, key=lambda item: (list(map(str, item.absolute_path)), item.message)):
        location = "$"
        for part in error.absolute_path:
            location += f"[{part}]" if isinstance(part, int) else f".{part}"
        lines.append(f"  {location}: {error.message}")
    return "\n".join(lines)


def expect_invalid(
    validator: Draft202012Validator,
    source_profile: dict[str, Any],
    mutate: Any,
    name: str,
) -> None:
    candidate = copy.deepcopy(source_profile)
    mutate(candidate)
    errors = list(validator.iter_errors(candidate))
    if not errors:
        raise AssertionError(f"negative test unexpectedly accepted: {name}")
    print(f"PASS negative: {name}")


def main() -> int:
    schema = load_json(SCHEMA_PATH)
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema)

    profiles = sorted(PROFILE_DIR.glob("*.json"))
    if not profiles:
        raise RuntimeError(f"no workload profile fixtures found in {PROFILE_DIR}")

    loaded: dict[str, dict[str, Any]] = {}
    for path in profiles:
        profile = load_json(path)
        errors = list(validator.iter_errors(profile))
        if errors:
            print(f"FAIL profile: {path.relative_to(ROOT)}")
            print(describe_errors(errors))
            return 1

        if profile.get("metadata", {}).get("qualification") != "example_only":
            raise AssertionError(
                f"{path.relative_to(ROOT)} is an example fixture and must not claim qualification"
            )
        if profile.get("evidence", {}).get("status") != "design_only":
            raise AssertionError(
                f"{path.relative_to(ROOT)} is an example fixture and must stay design_only"
            )

        loaded[path.name] = profile
        print(f"PASS profile: {path.relative_to(ROOT)}")

    oci = loaded.get("oci-example.json")
    microvm = loaded.get("nixos-microvm-example.json")
    full_vm = loaded.get("ubuntu-full-vm-example.json")
    if oci is None or microvm is None or full_vm is None:
        raise RuntimeError("required OCI, NixOS microVM, and Ubuntu full-VM fixtures must all exist")

    negative_cases = [
        (
            oci,
            lambda candidate: candidate["deployment"].update({"privileged": True}),
            "reject privileged OCI container",
        ),
        (
            oci,
            lambda candidate: candidate["deployment"]["hostNamespaces"].update({"pid": True}),
            "reject host PID namespace",
        ),
        (
            oci,
            lambda candidate: candidate["deployment"]["image"].update(
                {"ref": "registry.example.invalid/luminous/example:latest"}
            ),
            "reject mutable image tag without digest",
        ),
        (
            oci,
            lambda candidate: candidate["security"].update({"hostControl": "root"}),
            "reject workload host-control authority",
        ),
        (
            oci,
            lambda candidate: candidate["security"]["secrets"].update({"referencesOnly": False}),
            "reject inline or embedded secret material",
        ),
        (
            oci,
            lambda candidate: candidate["deployment"].update({"hostPath": "/"}),
            "reject undeclared broad host path",
        ),
        (
            oci,
            lambda candidate: candidate["deployment"]["ports"].append(
                {"containerPort": 8080, "protocol": "tcp", "exposure": "public"}
            ),
            "reject public port exposure",
        ),
        (
            microvm,
            lambda candidate: candidate["deployment"].update({"guestStoreWritable": True}),
            "reject unknown microVM host-store field",
        ),
        (
            full_vm,
            lambda candidate: candidate["deployment"].update({"isolatedNetwork": False}),
            "reject non-isolated compatibility VM",
        ),
        (
            full_vm,
            lambda candidate: candidate.update({"hostBootMutation": True}),
            "reject unknown top-level authority field",
        ),
    ]

    for profile, mutate, label in negative_cases:
        expect_invalid(validator, profile, mutate, label)

    print(
        f"PASS: schema valid; {len(profiles)} profiles accepted; "
        f"{len(negative_cases)} security-negative cases rejected."
    )
    print("NOTE: schema validation is not a runtime, security, or hardware qualification.")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (AssertionError, OSError, ValueError, RuntimeError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        raise SystemExit(1)
