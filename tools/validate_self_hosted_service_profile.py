#!/usr/bin/env python3
# Copyright (C) 2026 Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Strict declaration validator for self-hosted service profiles.

This validates declared structure and safety invariants. It does not deploy,
build, observe, or qualify an application runtime.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

try:
    from jsonschema import Draft202012Validator, FormatChecker
except ImportError as exc:  # pragma: no cover - surfaced by CI smoke test
    print(f"Missing dependency from requirements-profile-validator.txt: {exc}", file=sys.stderr)
    raise SystemExit(3)

# Reuse the platform's strict YAML parser and canonical digest implementation.
from validate_deployment_profile import (  # noqa: E402
    ProfileInputError,
    parse_profile_yaml,
    profile_digest,
)

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_SCHEMA = ROOT / "schemas" / "self-hosted-service-profile-v1.schema.json"
VALIDATOR_VERSION = "0.1.0"


def load_schema(path: Path) -> dict[str, Any]:
    try:
        schema = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise ProfileInputError(f"cannot load service-profile JSON Schema: {exc}") from exc
    if not isinstance(schema, dict):
        raise ProfileInputError("service-profile schema root must be a JSON object")
    try:
        Draft202012Validator.check_schema(schema)
    except Exception as exc:
        raise ProfileInputError(f"invalid service-profile JSON Schema: {exc}") from exc
    return schema


def validate_profile_data(profile: Any, schema: dict[str, Any], source: str) -> dict[str, Any]:
    validator = Draft202012Validator(schema, format_checker=FormatChecker())
    schema_errors = sorted(
        validator.iter_errors(profile),
        key=lambda error: (list(map(str, error.absolute_path)), error.message),
    )
    errors: list[dict[str, Any]] = [
        {
            "path": "/" + "/".join(map(str, error.absolute_path)),
            "message": error.message,
        }
        for error in schema_errors
    ]
    blockers: list[str] = []

    if not schema_errors and isinstance(profile, dict):
        runtime = profile["runtime"]
        source_pin = runtime["source_pin"]
        network = profile["network"]
        endpoints = network["endpoints"]
        data = profile["data"]
        qualification = profile["qualification"]
        metadata = profile["metadata"]

        endpoint_ids = [endpoint["id"] for endpoint in endpoints]
        if len(endpoint_ids) != len(set(endpoint_ids)):
            errors.append({"path": "/network/endpoints", "message": "endpoint IDs must be unique"})

        for index, endpoint in enumerate(endpoints):
            path = f"/network/endpoints/{index}"
            if "port_range" in endpoint and endpoint["port_range"][0] > endpoint["port_range"][1]:
                errors.append({"path": path + "/port_range", "message": "port range start must not exceed end"})
            if endpoint["enabled"] and endpoint["exposure"] == "disabled":
                errors.append({"path": path, "message": "an enabled endpoint cannot have disabled exposure"})
            if not endpoint["enabled"] and endpoint["exposure"] != "disabled":
                errors.append({"path": path, "message": "a disabled endpoint must declare exposure=disabled"})

        public_endpoints = [
            endpoint["id"] for endpoint in endpoints
            if endpoint["exposure"] == "public-network"
        ]
        if public_endpoints and not network["public_exposure_approved"]:
            errors.append({
                "path": "/network/public_exposure_approved",
                "message": "public-network endpoints require explicit exposure approval",
            })
        if network["public_exposure_approved"] and not network.get("public_exposure_assessment_ref"):
            errors.append({
                "path": "/network/public_exposure_assessment_ref",
                "message": "approved public exposure requires a reviewable assessment reference",
            })

        if data["stateful"]:
            if data["backup_policy"] not in {
                "periodic-and-before-upgrade",
                "application-consistent-periodic-and-before-upgrade",
            }:
                errors.append({
                    "path": "/data/backup_policy",
                    "message": "stateful services require periodic backups and a pre-upgrade backup policy",
                })
            if data["restore_test"] == "not-required":
                errors.append({
                    "path": "/data/restore_test",
                    "message": "stateful services require a sandbox or pre-production restore test",
                })

        if (
            data["call_recording"] != "disabled-by-default"
            and not data.get("recording_policy_reference")
        ):
            errors.append({
                "path": "/data/recording_policy_reference",
                "message": "enabled/opt-in recording requires a reviewed policy reference",
            })

        mode = source_pin["mode"]
        if runtime["kind"] == "oci-container":
            if mode != "oci-digest" or not source_pin.get("digest"):
                errors.append({
                    "path": "/runtime/source_pin",
                    "message": "OCI workloads require an immutable OCI digest; mutable tags are insufficient",
                })
            elif source_pin["reference"].rsplit("@", 1)[-1] != source_pin["digest"]:
                errors.append({
                    "path": "/runtime/source_pin/reference",
                    "message": "OCI reference must embed the same @sha256 digest declared in source_pin.digest",
                })
        elif mode == "oci-digest" and not source_pin.get("digest"):
            errors.append({
                "path": "/runtime/source_pin/digest",
                "message": "oci-digest mode requires a concrete sha256 digest",
            })
        if mode == "unresolved" and qualification["maturity"] not in {
            "M0_PROPOSED", "M1_DECLARATION_VALID"
        }:
            blockers.append("runtime source pin is unresolved for the claimed maturity")

        advanced_claim = (
            qualification["maturity"] in {
                "M2_SANDBOX_TESTED", "M3_CONTROLLED_DEPLOYMENT", "M4_INDEPENDENT_ASSESSMENT"
            }
            or metadata["status"] == "qualified"
            or bool(qualification["qualified_capabilities"])
        )
        if advanced_claim:
            blockers.append(
                "this validator evaluates declarations only; runtime evidence must be independently evaluated"
            )
        if metadata["status"] == "qualified" and not qualification["evidence_refs"]:
            blockers.append("qualified status has no evidence references")
        if metadata["status"] == "qualified" and not qualification["qualified_capabilities"]:
            blockers.append("qualified status names no explicitly qualified capabilities")
        if metadata["status"] == "qualified" and qualification["maturity"] not in {
            "M3_CONTROLLED_DEPLOYMENT", "M4_INDEPENDENT_ASSESSMENT"
        }:
            blockers.append("qualified status requires at least M3_CONTROLLED_DEPLOYMENT maturity")

    if errors:
        result = "INVALID"
    elif blockers:
        result = "QUALIFICATION_BLOCKED"
    else:
        result = "VALID_DECLARATION"

    schema_digest = "sha256:" + __import__("hashlib").sha256(
        json.dumps(schema, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")
    ).hexdigest()
    return {
        "schema": "luminous.service-profile-validation-result/v1",
        "validator_version": VALIDATOR_VERSION,
        "source": source,
        "result": result,
        "qualification_evaluated": False,
        "profile_digest_sha256": profile_digest(profile) if isinstance(profile, (dict, list)) else None,
        "schema_digest_sha256": schema_digest,
        "errors": errors,
        "blockers": sorted(set(blockers)),
        "notice": (
            "Declaration validation is not a deployment, build, runtime observation, "
            "production-readiness decision, security audit, or legal-compliance determination."
        ),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("profile", type=Path, help="path to a strict YAML self-hosted service profile")
    parser.add_argument("--schema", type=Path, default=DEFAULT_SCHEMA, help="schema path")
    args = parser.parse_args()

    try:
        schema = load_schema(args.schema)
        profile = parse_profile_yaml(args.profile.read_text(encoding="utf-8"))
        report = validate_profile_data(profile, schema, source=str(args.profile))
    except (OSError, ProfileInputError) as exc:
        report = {
            "schema": "luminous.service-profile-validation-result/v1",
            "validator_version": VALIDATOR_VERSION,
            "source": str(args.profile),
            "result": "INVALID",
            "qualification_evaluated": False,
            "errors": [{"path": "/", "message": str(exc)}],
            "blockers": [],
            "notice": "Invalid input is rejected before runtime qualification.",
        }

    print(json.dumps(report, ensure_ascii=False, sort_keys=True, indent=2))
    return {"VALID_DECLARATION": 0, "INVALID": 2, "QUALIFICATION_BLOCKED": 4}.get(report["result"], 2)


if __name__ == "__main__":
    raise SystemExit(main())
