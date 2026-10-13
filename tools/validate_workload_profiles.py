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

from jsonschema import Draft202012Validator, FormatChecker


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


EXPECTED_SCHEMA_ERROR_FRAGMENTS: dict[str, tuple[str, ...]] = {
    "reject privileged OCI container": ("deployment.privileged",),
    "reject host PID namespace": ("deployment.hostNamespaces.pid",),
    "reject mutable image tag without digest": ("deployment.image.ref",),
    "reject custom seccomp profile without an identified policy artifact": ("deployment.seccompProfile",),
    "reject wildcard hostname peer": ("peer",),
    "reject malformed IPv4 network peer": ("peer",),
    "reject workload host-control authority": ("security.hostControl",),
    "reject inline or embedded secret material": ("security.secrets.referencesOnly",),
    "reject undeclared broad host path": ("hostPath",),
    "reject public port exposure": ("deployment.ports", "exposure"),
    "reject unknown microVM host-store field": ("guestStoreWritable",),
    "reject non-isolated compatibility VM": ("deployment.isolatedNetwork",),
    "reject qualification/evidence status mismatch": ("evidence.status",),
    "reject hardware qualification without evidence receipt": ("receiptReferences",),
    "reject CI qualification without passed tests and receipt": ("testsPassed",),
    "reject allowlisted egress without explicit rules": ("security.network.rules",),
    "reject egress rule while egress policy is deny": ("security.network.rules",),
    "reject writable read-only configuration mount": ("deployment", "access"),
    "reject unknown top-level authority field": ("hostBootMutation",),
}


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

    expected_fragments = EXPECTED_SCHEMA_ERROR_FRAGMENTS.get(name)
    if expected_fragments is None:
        raise AssertionError(f"negative test has no expected diagnostic target: {name}")

    relevant_errors = []
    for error in errors:
        error_path = ".".join(str(part) for part in error.absolute_path)
        if (
            all(fragment in error_path for fragment in expected_fragments)
            or all(fragment in error.message for fragment in expected_fragments)
        ):
            relevant_errors.append(error)
    if not relevant_errors:
        details = describe_errors(errors)
        raise AssertionError(
            f"negative test rejected for an unrelated reason: {name}; "
            f"expected one of {expected_fragments!r}; actual errors:\n{details}"
        )

    print(f"PASS negative: {name}")


def semantic_errors(profile: dict[str, Any]) -> list[str]:
    """Check cross-field invariants JSON Schema cannot express portably."""
    errors: list[str] = []
    deployment = profile["deployment"]
    kind = deployment["kind"]
    security = profile["security"]
    resources = profile["resources"]

    if kind == "nixos_microvm":
        if set(profile["placement"]["allowedHosts"]) != {"nixos"}:
            errors.append("NixOS microVM profile must be limited to the NixOS reference host")
        if resources["cpuMilli"] < deployment["vcpus"] * 1000:
            errors.append("resource CPU budget must cover configured microVM vCPUs")
        if resources["memoryMiB"] < deployment["memoryMiB"]:
            errors.append("resource memory budget must cover configured microVM memory")
        if security["hostDataAccess"] == "none" and deployment["shares"]:
            errors.append("hostDataAccess=none forbids microVM host shares")
        if security["hostDataAccess"] == "declared_mounts_only" and not deployment["shares"]:
            errors.append("declared_mounts_only requires at least one declared microVM share")

    elif kind == "full_vm":
        if resources["cpuMilli"] < deployment["vcpus"] * 1000:
            errors.append("resource CPU budget must cover configured full-VM vCPUs")
        if resources["memoryMiB"] < deployment["memoryMiB"]:
            errors.append("resource memory budget must cover configured full-VM memory")
        if resources.get("diskMiB", 0) < deployment["diskGiB"] * 1024:
            errors.append("resource disk budget must cover configured full-VM disk capacity")

    elif kind == "oci_container":
        volumes = deployment["volumes"]
        if security["hostDataAccess"] == "none" and volumes:
            errors.append("hostDataAccess=none forbids OCI host/data volumes")
        if security["hostDataAccess"] == "declared_mounts_only" and not volumes:
            errors.append("declared_mounts_only requires at least one declared OCI volume")

    return errors


def main() -> int:
    schema = load_json(SCHEMA_PATH)
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema, format_checker=FormatChecker())

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

        semantic = semantic_errors(profile)
        if semantic:
            print(f"FAIL semantic constraints: {path.relative_to(ROOT)}")
            for error in semantic:
                print(f"  {error}")
            return 1

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
            lambda candidate: candidate["deployment"].update({"seccompProfile": "custom_pinned"}),
            "reject custom seccomp profile without an identified policy artifact",
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
            lambda candidate: candidate["metadata"].update({"qualification": "unverified"}),
            "reject qualification/evidence status mismatch",
        ),
        (
            full_vm,
            lambda candidate: (
                candidate["metadata"].update({"qualification": "hardware_qualified"}),
                candidate["evidence"].update({"status": "hardware_qualified", "testsPassed": ["guest_boot_shutdown"]}),
            ),
            "reject hardware qualification without evidence receipt",
        ),
        (
            full_vm,
            lambda candidate: (
                candidate["metadata"].update({"qualification": "ci_verified"}),
                candidate["evidence"].update({"status": "ci_verified"}),
            ),
            "reject CI qualification without passed tests and receipt",
        ),
        (
            oci,
            lambda candidate: candidate["security"]["network"].update({"egress": "allowlisted"}),
            "reject allowlisted egress without explicit rules",
        ),
        (
            oci,
            lambda candidate: candidate["security"]["network"]["rules"].append(
                {
                    "direction": "egress",
                    "protocol": "tcp",
                    "peerType": "hostname",
                    "peer": "example.org",
                    "port": 443,
                    "justification": "deliberately inconsistent fixture",
                }
            ),
            "reject egress rule while egress policy is deny",
        ),
        (
            oci,
            lambda candidate: candidate["security"]["network"].update(
                {
                    "egress": "allowlisted",
                    "rules": [
                        {
                            "direction": "egress",
                            "protocol": "tcp",
                            "peerType": "hostname",
                            "peer": "*",
                            "port": 443,
                            "justification": "wildcards must not be accepted as hostnames",
                        }
                    ],
                }
            ),
            "reject wildcard hostname peer",
        ),
        (
            oci,
            lambda candidate: candidate["security"]["network"].update(
                {
                    "egress": "allowlisted",
                    "rules": [
                        {
                            "direction": "egress",
                            "protocol": "tcp",
                            "peerType": "ipv4",
                            "peer": "999.1.1.1",
                            "port": 443,
                            "justification": "invalid address must be rejected",
                        }
                    ],
                }
            ),
            "reject malformed IPv4 network peer",
        ),
        (
            oci,
            lambda candidate: candidate["deployment"]["volumes"].append(
                {"source": "readonly_config", "target": "/etc/example", "access": "read_write"}
            ),
            "reject writable read-only configuration mount",
        ),
        (
            full_vm,
            lambda candidate: candidate.update({"hostBootMutation": True}),
            "reject unknown top-level authority field",
        ),
    ]

    for profile, mutate, label in negative_cases:
        expect_invalid(validator, profile, mutate, label)

    semantic_negative_cases = [
        (
            full_vm,
            lambda candidate: candidate["resources"].update({"memoryMiB": 1024}),
            "reject VM memory above its resource budget",
            "resource memory budget",
        ),
        (
            full_vm,
            lambda candidate: candidate["resources"].update({"cpuMilli": 1000}),
            "reject VM vCPUs above its resource CPU budget",
            "resource CPU budget",
        ),
        (
            full_vm,
            lambda candidate: candidate["resources"].update({"diskMiB": 1024}),
            "reject VM disk capacity above its resource disk budget",
            "resource disk budget",
        ),
        (
            microvm,
            lambda candidate: candidate["placement"].update({"allowedHosts": ["nixos", "ubuntu"]}),
            "reject microVM placement on an unqualified host",
            "NixOS microVM profile must be limited",
        ),
        (
            microvm,
            lambda candidate: candidate["deployment"]["shares"].append(
                {
                    "sourceId": "shared-data",
                    "target": "/data",
                    "access": "read_write",
                    "protocol": "virtiofs",
                }
            ),
            "reject microVM share when hostDataAccess is none",
            "hostDataAccess=none forbids microVM host shares",
        ),
        (
            microvm,
            lambda candidate: candidate["security"].update({"hostDataAccess": "declared_mounts_only"}),
            "reject microVM declared-mounts policy without a share",
            "declared_mounts_only requires at least one declared microVM share",
        ),
        (
            oci,
            lambda candidate: candidate["deployment"]["volumes"].append(
                {
                    "source": "dedicated_state",
                    "target": "/var/lib/example",
                    "access": "read_write",
                }
            ),
            "reject OCI volume when hostDataAccess is none",
            "hostDataAccess=none forbids OCI host/data volumes",
        ),
        (
            oci,
            lambda candidate: candidate["security"].update({"hostDataAccess": "declared_mounts_only"}),
            "reject OCI declared-mounts policy without a volume",
            "declared_mounts_only requires at least one declared OCI volume",
        ),
    ]
    for profile, mutate, label, expected_fragment in semantic_negative_cases:
        candidate = copy.deepcopy(profile)
        mutate(candidate)
        errors = semantic_errors(candidate)
        if not any(expected_fragment in error for error in errors):
            raise AssertionError(
                f"semantic negative failed its intended invariant: {label}; "
                f"expected {expected_fragment!r}; actual errors: {errors!r}"
            )
        print(f"PASS semantic negative: {label}")

    print(
        f"PASS: schema valid; {len(profiles)} profiles accepted; "
        f"{len(negative_cases)} schema-negative and {len(semantic_negative_cases)} semantic-negative cases rejected."
    )
    print("NOTE: schema validation is not a runtime, security, or hardware qualification.")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (AssertionError, OSError, ValueError, RuntimeError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        raise SystemExit(1)
