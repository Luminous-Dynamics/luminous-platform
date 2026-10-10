#!/usr/bin/env python3
# Copyright (C) 2026 Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Strict, fail-closed validator for proposed Luminous deployment profiles.

Schema validity is not runtime qualification, production readiness, or legal compliance.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Any

try:
    import yaml
    from jsonschema import Draft202012Validator, FormatChecker
    from jsonschema.exceptions import SchemaError
    from yaml.constructor import ConstructorError
    from yaml.nodes import MappingNode, ScalarNode
    from yaml.tokens import AliasToken, AnchorToken, TagToken
except ImportError as exc:  # pragma: no cover - covered by CLI smoke test in CI
    print(
        "Missing validator dependency. Install requirements-profile-validator.txt. "
        f"Import detail: {exc}",
        file=sys.stderr,
    )
    raise SystemExit(3)

VALIDATOR_VERSION = "0.1.0"
RESULT_SCHEMA = "luminous.validation-result/v1"
REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_SCHEMA_PATH = REPO_ROOT / "schemas" / "deployment-profile-v1.schema.json"


class ProfileInputError(ValueError):
    """Raised when the input format is ambiguous or unsafe for this contract."""


class StrictSafeLoader(yaml.SafeLoader):
    """Safe YAML loader with duplicate-key and YAML 1.1 coercion protections."""

    # Avoid inheriting mutations back into yaml.SafeLoader.
    yaml_implicit_resolvers = {
        first: list(resolvers)
        for first, resolvers in yaml.SafeLoader.yaml_implicit_resolvers.items()
    }


# Use predictable YAML booleans only. In particular, "yes"/"no"/"on"/"off"
# remain strings and will fail schema validation when a boolean is required.
for first, resolvers in list(StrictSafeLoader.yaml_implicit_resolvers.items()):
    StrictSafeLoader.yaml_implicit_resolvers[first] = [
        (tag, pattern)
        for tag, pattern in resolvers
        if tag not in ("tag:yaml.org,2002:bool", "tag:yaml.org,2002:timestamp")
    ]
StrictSafeLoader.add_implicit_resolver(
    "tag:yaml.org,2002:bool",
    re.compile(r"^(?:true|false)$"),
    list("tf"),
)


def _construct_strict_mapping(
    loader: StrictSafeLoader, node: MappingNode, deep: bool = False
) -> dict[str, Any]:
    if not isinstance(node, MappingNode):
        raise ConstructorError(
            None, None, "expected a mapping node", getattr(node, "start_mark", None)
        )

    result: dict[str, Any] = {}
    for key_node, value_node in node.value:
        if not isinstance(key_node, ScalarNode):
            raise ConstructorError(
                "while constructing a deployment profile",
                node.start_mark,
                "mapping keys must be scalar strings",
                key_node.start_mark,
            )
        key = loader.construct_object(key_node, deep=deep)
        if not isinstance(key, str):
            raise ConstructorError(
                "while constructing a deployment profile",
                node.start_mark,
                "mapping keys must be strings",
                key_node.start_mark,
            )
        if key in result:
            raise ConstructorError(
                "while constructing a deployment profile",
                node.start_mark,
                f"duplicate key {key!r}",
                key_node.start_mark,
            )
        result[key] = loader.construct_object(value_node, deep=deep)
    return result


StrictSafeLoader.add_constructor(
    "tag:yaml.org,2002:map", _construct_strict_mapping
)


def parse_profile_yaml(text: str) -> dict[str, Any]:
    """Parse one strict YAML document, rejecting aliases, anchors and explicit tags."""
    try:
        for token in yaml.scan(text):
            if isinstance(token, (AnchorToken, AliasToken)):
                raise ProfileInputError(
                    "YAML anchors and aliases are not accepted in deployment profiles"
                )
            if isinstance(token, TagToken):
                raise ProfileInputError(
                    "explicit YAML tags are not accepted in deployment profiles"
                )
        parsed = yaml.load(text, Loader=StrictSafeLoader)
    except ProfileInputError:
        raise
    except yaml.YAMLError as exc:
        raise ProfileInputError(f"invalid YAML: {exc}") from exc

    if not isinstance(parsed, dict):
        raise ProfileInputError("profile root must be a YAML mapping")
    return parsed


def _canonical_json(data: Any) -> str:
    """Canonicalize JSON-compatible data for stable profile digests."""
    return json.dumps(
        data,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
        allow_nan=False,
    )


def profile_digest(data: Any) -> str:
    """Return the SHA-256 digest of the profile's canonical JSON form."""
    return "sha256:" + hashlib.sha256(_canonical_json(data).encode("utf-8")).hexdigest()


def _load_schema(schema_path: Path) -> dict[str, Any]:
    try:
        schema = json.loads(schema_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise ProfileInputError(f"cannot load JSON Schema: {exc}") from exc
    if not isinstance(schema, dict):
        raise ProfileInputError("profile schema root must be a JSON object")
    try:
        Draft202012Validator.check_schema(schema)
    except SchemaError as exc:
        raise ProfileInputError(f"cannot load a valid JSON Schema: {exc}") from exc
    return schema


def _path_text(path: Any) -> str:
    pieces = [str(part).replace("~", "~0").replace("/", "~1") for part in path]
    return "/" + "/".join(pieces) if pieces else "/"


def _error(
    message: str,
    *,
    errors: list[dict[str, str]],
    path: str = "/",
) -> None:
    errors.append({"path": path, "message": message})


def validate_profile_data(
    data: dict[str, Any],
    schema: dict[str, Any],
    *,
    source: str = "<memory>",
) -> dict[str, Any]:
    """Validate profile syntax/semantics without asserting runtime qualification."""
    result: dict[str, Any] = {
        "result_schema": RESULT_SCHEMA,
        "validator_version": VALIDATOR_VERSION,
        "source": source,
        "result": "INVALID",
        "qualification_evaluated": False,
        "profile_schema_id": schema.get("$id") if isinstance(schema, dict) else None,
        "profile_schema_version": data.get("schema") if isinstance(data, dict) else None,
        "schema_digest_sha256": profile_digest(schema) if isinstance(schema, dict) else None,
        "profile_digest_sha256": None,
        "errors": [],
        "blockers": [],
        "warnings": [],
        "unqualified_capabilities": [],
        "notice": (
            "This tool validates a declaration only. It does not run an adapter, "
            "verify runtime evidence, certify production readiness, or establish legal compliance."
        ),
    }

    try:
        result["profile_digest_sha256"] = profile_digest(data)
    except (TypeError, ValueError) as exc:
        _error(f"profile is not canonical JSON data: {exc}", errors=result["errors"])
        return result

    validator = Draft202012Validator(schema, format_checker=FormatChecker())
    schema_errors = sorted(
        validator.iter_errors(data),
        key=lambda err: (_path_text(err.absolute_path), err.message),
    )
    for error in schema_errors:
        _error(error.message, errors=result["errors"], path=_path_text(error.absolute_path))
    if result["errors"]:
        return result

    metadata = data["metadata"]
    target = data["target"]
    authority = data["authority"]
    services = data["services"]
    remote = services["remote_control"]
    export = services["telemetry_export"]
    data_policy = data["data"]
    lifecycle = data["lifecycle"]
    evidence = data["evidence"]
    qualification = data["qualification"]

    # Cross-field authority checks. The presence of a name is only a declaration;
    # it does not establish that the authority is real or correctly implemented.
    if authority["privileged_execution"]:
        if not authority.get("authorization_authority"):
            _error(
                "privileged execution requires a named authorization_authority",
                errors=result["errors"],
                path="/authority/authorization_authority",
            )
        if not target.get("adapter_ids"):
            _error(
                "privileged execution requires at least one explicit target adapter",
                errors=result["errors"],
                path="/target/adapter_ids",
            )
        if "authorization_reference" not in evidence["required_fields"]:
            _error(
                "privileged execution requires authorization_reference in required evidence fields",
                errors=result["errors"],
                path="/evidence/required_fields",
            )

    if remote["enabled"]:
        if not remote.get("authentication_authority"):
            _error(
                "remote control requires a named authentication_authority",
                errors=result["errors"],
                path="/services/remote_control/authentication_authority",
            )
        if remote.get("network_exposure") not in (
            "loopback_only",
            "private_network",
            "public_network",
        ):
            _error(
                "enabled remote control requires an explicit active network_exposure",
                errors=result["errors"],
                path="/services/remote_control/network_exposure",
            )
    elif remote.get("network_exposure") not in (None, "disabled"):
        _error(
            "disabled remote control must not declare an active network_exposure",
            errors=result["errors"],
            path="/services/remote_control/network_exposure",
        )

    if export["enabled"]:
        if not export.get("destination_classes"):
            _error(
                "telemetry export requires explicit destination_classes",
                errors=result["errors"],
                path="/services/telemetry_export/destination_classes",
            )
        if (
            data_policy["raw_telemetry"] == "local-only"
            and not export["approval_required"]
            and data_policy["evidence_export"] != "automatic-sanitized-only"
        ):
            _error(
                "local-only raw telemetry requires approved or explicitly sanitized export",
                errors=result["errors"],
                path="/services/telemetry_export/approval_required",
            )
        if data_policy["evidence_export"] == "disabled":
            _error(
                "telemetry_export cannot be enabled when data.evidence_export is disabled",
                errors=result["errors"],
                path="/data/evidence_export",
            )

    if lifecycle["export_exit_path"] == "absent":
        result["warnings"].append(
            "No customer export/exit path is declared; this is unsuitable for a sovereignty-focused deployment."
        )

    result["unqualified_capabilities"] = sorted(
        qualification["unqualified_capabilities"]
    )
    if result["unqualified_capabilities"]:
        result["warnings"].append(
            "Unqualified capabilities are explicitly listed and must not be marketed as available or verified."
        )

    claimed_level = qualification["claimed_level"]
    profile_status = metadata["status"]
    evidence_refs = qualification.get("evidence_refs", [])
    tested_revisions = target.get("tested_platform_revisions", [])
    limitations = qualification.get("limitations", [])
    qualified_capabilities = qualification["qualified_capabilities"]

    if profile_status == "qualified":
        if claimed_level == "M0":
            result["blockers"].append(
                "metadata.status is qualified but qualification.claimed_level remains M0"
            )
        if not qualified_capabilities:
            result["blockers"].append(
                "qualified status requires explicit qualified_capabilities scope"
            )
        if not evidence_refs:
            result["blockers"].append(
                "qualified status requires evidence_refs scoped to this profile and target"
            )
        if not tested_revisions:
            result["blockers"].append(
                "qualified status requires tested_platform_revisions"
            )
        if not limitations:
            result["blockers"].append(
                "qualified status requires explicit qualification limitations"
            )
        if not qualification.get("last_qualification_at"):
            result["blockers"].append(
                "qualified status requires last_qualification_at"
            )
        if not lifecycle.get("rollback") in ("required", "automated-tested"):
            result["blockers"].append(
                "qualified status requires a defined rollback path"
            )
        if lifecycle.get("export_exit_path") not in ("documented", "tested"):
            result["blockers"].append(
                "qualified status requires a documented or tested export/exit path"
            )

    if claimed_level != "M0":
        if not evidence_refs:
            result["blockers"].append(
                "qualification above M0 requires evidence_refs"
            )
        if not limitations:
            result["blockers"].append(
                "qualification above M0 requires explicit limitations"
            )
        if claimed_level == "independent-assessment":
            assessment = qualification.get("assessment")
            if not assessment:
                result["blockers"].append(
                    "independent-assessment requires assessor, scope, assessment_date, and exceptions"
                )
            if not evidence_refs:
                result["blockers"].append(
                    "independent-assessment requires references to assessment evidence"
                )
        result["blockers"].append(
            "runtime qualification is not evaluated by this declaration validator"
        )

    if profile_status == "qualified" or claimed_level != "M0":
        result["blockers"].append(
            "declared qualification must be verified by an adapter-specific qualification workflow"
        )

    result["errors"].sort(key=lambda item: (item["path"], item["message"]))
    result["blockers"] = sorted(set(result["blockers"]))
    result["warnings"] = sorted(set(result["warnings"]))

    if result["errors"]:
        result["result"] = "INVALID"
    elif result["blockers"]:
        result["result"] = "QUALIFICATION_BLOCKED"
    else:
        result["result"] = "VALID_DECLARATION"
    return result


def validate_profile_text(
    text: str, schema: dict[str, Any], *, source: str = "<memory>"
) -> dict[str, Any]:
    try:
        data = parse_profile_yaml(text)
    except ProfileInputError as exc:
        return {
            "result_schema": RESULT_SCHEMA,
            "validator_version": VALIDATOR_VERSION,
            "source": source,
            "result": "INVALID",
            "qualification_evaluated": False,
            "profile_schema_id": schema.get("$id") if isinstance(schema, dict) else None,
            "profile_schema_version": None,
            "schema_digest_sha256": profile_digest(schema) if isinstance(schema, dict) else None,
            "profile_digest_sha256": None,
            "errors": [{"path": "/", "message": str(exc)}],
            "blockers": [],
            "warnings": [],
            "unqualified_capabilities": [],
            "notice": (
                "This tool validates a declaration only. It does not run an adapter, "
                "verify runtime evidence, certify production readiness, or establish legal compliance."
            ),
        }
    return validate_profile_data(data, schema, source=source)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Validate a Luminous deployment profile declaration without claiming runtime qualification."
    )
    parser.add_argument("profile", type=Path, help="YAML deployment profile to validate")
    parser.add_argument(
        "--schema",
        type=Path,
        default=DEFAULT_SCHEMA_PATH,
        help=f"JSON Schema path (default: {DEFAULT_SCHEMA_PATH})",
    )
    parser.add_argument(
        "--output",
        choices=("json",),
        default="json",
        help="Report format (JSON is the stable machine-readable interface)",
    )
    args = parser.parse_args(argv)

    try:
        schema = _load_schema(args.schema)
        text = args.profile.read_text(encoding="utf-8")
    except (OSError, ProfileInputError) as exc:
        report = {
            "result_schema": RESULT_SCHEMA,
            "validator_version": VALIDATOR_VERSION,
            "source": str(args.profile),
            "result": "INVALID",
            "qualification_evaluated": False,
            "profile_schema_id": None,
            "profile_schema_version": None,
            "schema_digest_sha256": None,
            "profile_digest_sha256": None,
            "errors": [{"path": "/", "message": str(exc)}],
            "blockers": [],
            "warnings": [],
            "unqualified_capabilities": [],
            "notice": (
                "This tool validates a declaration only. It does not run an adapter, "
                "verify runtime evidence, certify production readiness, or establish legal compliance."
            ),
        }
    else:
        report = validate_profile_text(text, schema, source=str(args.profile))

    print(json.dumps(report, ensure_ascii=False, sort_keys=True, indent=2))
    if report["result"] == "VALID_DECLARATION":
        return 0
    if report["result"] == "QUALIFICATION_BLOCKED":
        return 2
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
