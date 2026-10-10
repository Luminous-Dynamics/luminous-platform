#!/usr/bin/env python3
"""Validate a vendor management endpoint profile without making a network request."""

from __future__ import annotations

import argparse
import ipaddress
import re
import sys
from datetime import date
from pathlib import Path
from typing import Any
from urllib.parse import unquote, urlsplit

from jsonschema import Draft202012Validator, FormatChecker

from validate_hardware_portfolio import load_json

_KIND_TRANSPORT = {
    "vendor_cloud_api": "https",
    "local_device_api": "https",
    "netconf_ssh": "ssh",
    "netconf_tls": "tls",
    "gnmi_tls": "grpc_tls",
    "snmpv3": "snmpv3_usm",
}
_AUTH_MODES = {
    "https": {
        "api_token_reference",
        "oauth_profile_reference",
        "mtls_client_certificate_reference",
    },
    "ssh": {"ssh_key_reference"},
    "tls": {"mtls_client_certificate_reference"},
    "grpc_tls": {"mtls_client_certificate_reference"},
    "snmpv3_usm": {"snmpv3_authpriv_reference"},
}
_TLS_TRANSPORTS = {"https", "tls", "grpc_tls"}
_HOST_LABEL = re.compile(r"^[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?$")
_NUMERIC_DOTTED_HOST = re.compile(r"^[0-9.]+$")


def _host_errors(host: str, field: str = "$.endpoint.host") -> list[str]:
    errors: list[str] = []
    if not host or host != host.lower() or host.endswith("."):
        errors.append(f"{field} must be lowercase and have no trailing dot")
        return errors
    if any(char in host for char in ("*", "/", "\\", "@", "%", "?", "#", " ")):
        errors.append(f"{field} must be a literal host or IP address, not a URI or wildcard")
        return errors
    try:
        address = ipaddress.ip_address(host)
        if str(address) != host:
            errors.append(f"{field} IP address must use its canonical textual representation")
        return errors
    except ValueError:
        pass
    if ":" in host:
        errors.append(f"{field} contains an invalid IPv6 address")
        return errors
    if _NUMERIC_DOTTED_HOST.fullmatch(host):
        errors.append(f"{field} resembles an invalid dotted-decimal IP address")
        return errors
    labels = host.split(".")
    if any(not label or len(label) > 63 or not _HOST_LABEL.fullmatch(label) for label in labels):
        errors.append(f"{field} must use valid DNS labels or a valid literal IP address")
    return errors


def _origin_errors(value: str, field: str) -> list[str]:
    errors: list[str] = []
    try:
        parsed = urlsplit(value)
        port = parsed.port
    except ValueError:
        return [f"{field} is not a valid origin"]
    if parsed.scheme != "https":
        errors.append(f"{field} must use https")
    if not parsed.hostname or parsed.username is not None or parsed.password is not None:
        errors.append(f"{field} must contain a host and no user information")
    else:
        if parsed.hostname != parsed.hostname.lower() or parsed.hostname.endswith("."):
            errors.append(f"{field} host must be lowercase and have no trailing dot")
        errors.extend(_host_errors(parsed.hostname, f"{field}.host"))
    if parsed.path not in ("", "/") or parsed.query or parsed.fragment:
        errors.append(f"{field} must be an origin only (no non-root path, query, or fragment)")
    if "*" in parsed.netloc:
        errors.append(f"{field} cannot use wildcard hosts")
    if port is not None and not (1 <= port <= 65535):
        errors.append(f"{field} port is outside the valid range")
    if parsed.hostname and not errors:
        host_wire = f"[{parsed.hostname}]" if ":" in parsed.hostname else parsed.hostname
        expected = f"https://{host_wire}" + (f":{port}" if port is not None and port != 443 else "")
        if value != expected:
            errors.append(f"{field} must use canonical origin syntax (lowercase host, no path or default port)")
    return errors


def _path_errors(path: str, field: str = "$.endpoint.api_base_path") -> list[str]:
    errors: list[str] = []
    decoded = unquote(path)
    if not path.startswith("/") or path.startswith("//"):
        errors.append(f"{field} must be an absolute path beginning with one slash")
    if path != "/" and path.endswith("/"):
        errors.append(f"{field} must not have a trailing slash unless it is the root path")
    if "?" in path or "#" in path or "\\" in path or "%" in path:
        errors.append(f"{field} cannot contain query, fragment, backslash, or percent-encoding characters")
    if any(segment in (".", "..") for segment in decoded.split("/")):
        errors.append(f"{field} cannot contain dot-segments, including percent-encoded dot-segments")
    return errors


def profile_errors(profile: dict[str, Any], schema: dict[str, Any]) -> list[str]:
    """Return JSON Schema and semantic violations for the frozen profile."""
    errors: list[str] = []
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema, format_checker=FormatChecker())
    for error in sorted(validator.iter_errors(profile), key=lambda item: list(map(str, item.path))):
        where = "$" + "".join(
            f"[{part}]" if isinstance(part, int) else f".{part}"
            for part in error.absolute_path
        )
        errors.append(f"{where}: {error.message}")
    if errors:
        return errors

    try:
        date.fromisoformat(profile["as_of"])
    except (TypeError, ValueError):
        errors.append("$.as_of must be an ISO calendar date")
        return errors

    endpoint = profile["endpoint"]
    transport = endpoint["transport"]
    if _KIND_TRANSPORT[endpoint["kind"]] != transport:
        errors.append("$.endpoint.transport must match the protocol implied by endpoint.kind")

    errors.extend(_host_errors(endpoint["host"]))
    if transport == "https":
        if endpoint["api_base_path"] is None:
            errors.append("$.endpoint.api_base_path is required for HTTPS API transports")
        else:
            errors.extend(_path_errors(endpoint["api_base_path"]))
    elif endpoint["api_base_path"] is not None:
        errors.append("$.endpoint.api_base_path must be null for non-HTTP API transports")

    peer = profile["peer_verification"]
    if transport in _TLS_TRANSPORTS:
        if not peer["hostname_validation"]:
            errors.append("$.peer_verification.hostname_validation must be true for TLS transports")
        if peer["minimum_tls_version"] not in {"TLSv1.2", "TLSv1.3"}:
            errors.append("$.peer_verification.minimum_tls_version must be TLSv1.2 or TLSv1.3")
        if peer["method"] not in {"platform_trust_store", "pinned_certificate_sha256", "protocol_specific_trust"}:
            errors.append("$.peer_verification.method is incompatible with TLS transport")
    elif transport == "ssh":
        if peer["method"] != "ssh_known_hosts":
            errors.append("$.peer_verification.method must be ssh_known_hosts for SSH")
        if peer["hostname_validation"]:
            errors.append("$.peer_verification.hostname_validation must be false for non-TLS SSH transport")
        if peer["minimum_tls_version"] is not None:
            errors.append("$.peer_verification.minimum_tls_version must be null for SSH")
    else:
        if peer["method"] != "protocol_specific_trust":
            errors.append("$.peer_verification.method must be protocol_specific_trust for SNMPv3")
        if peer["hostname_validation"]:
            errors.append("$.peer_verification.hostname_validation must be false for SNMPv3")
        if peer["minimum_tls_version"] is not None:
            errors.append("$.peer_verification.minimum_tls_version must be null for SNMPv3")

    if profile["authentication"]["mode"] not in _AUTH_MODES[transport]:
        errors.append("$.authentication.mode is incompatible with endpoint.transport")
    if profile["authentication"]["expected_principal_ref"] != profile["scope"]["expected_principal_ref"]:
        errors.append("$.authentication.expected_principal_ref must match scope.expected_principal_ref")
    if profile["authentication"]["expected_tenant_or_org_ref"] != profile["scope"]["expected_tenant_or_org_ref"]:
        errors.append("$.authentication.expected_tenant_or_org_ref must match scope.expected_tenant_or_org_ref")

    credential_ref = profile["authentication"]["credential_reference"]
    if not credential_ref.startswith(("secretref:", "credential-profile:", "vaultref:")):
        errors.append("$.authentication.credential_reference must be an opaque secret-manager reference, not secret material")
    if any(marker in credential_ref.lower() for marker in ("password=", "token=", "secret=")):
        errors.append("$.authentication.credential_reference must not embed credential values")

    redirect = profile["redirect_policy"]
    limits = profile["limits"]
    if redirect["mode"] == "deny":
        if redirect["allowed_origins"]:
            errors.append("$.redirect_policy.allowed_origins must be empty when mode=deny")
        if limits["max_redirects"] != 0:
            errors.append("$.limits.max_redirects must be zero when redirects are denied")
    else:
        if transport != "https":
            errors.append("$.redirect_policy.allowlisted_origins is supported only for HTTPS API transports")
        if not redirect["allowed_origins"]:
            errors.append("$.redirect_policy.allowed_origins must be non-empty when mode=allowlisted_origins")
        if limits["max_redirects"] < 1:
            errors.append("$.limits.max_redirects must be positive when redirects are allowlisted")
        for index, origin in enumerate(redirect["allowed_origins"]):
            errors.extend(_origin_errors(origin, f"$.redirect_policy.allowed_origins[{index}]"))

    if limits["request_timeout_ms"] < limits["connect_timeout_ms"]:
        errors.append("$.limits.request_timeout_ms must be at least connect_timeout_ms")
    if limits["max_retries"] > 3:
        errors.append("$.limits.max_retries must remain bounded to at most 3 for the read-only profile")

    for dimension_name, dimension in profile["scope"].items():
        if not isinstance(dimension, dict) or "state" not in dimension:
            continue
        prefix = f"$.scope.{dimension_name}"
        if dimension["state"] == "assessed":
            if not dimension["value"]:
                errors.append(f"{prefix}.value is required when state=assessed")
            if not dimension["evidence_refs"]:
                errors.append(f"{prefix}.evidence_refs is required when state=assessed")
        elif dimension["value"] is not None:
            errors.append(f"{prefix}.value must be null unless state=assessed")
        if dimension["state"] in {"unresolved", "not_applicable"} and not dimension["rationale"]:
            errors.append(f"{prefix}.rationale is required when state is unresolved or not_applicable")

    return errors


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--profile",
        type=Path,
        default=Path("tests/fixtures/vendor-management-endpoint-profile-v1.json"),
    )
    parser.add_argument(
        "--schema",
        type=Path,
        default=Path("schemas/vendor-management-endpoint-profile-v1.schema.json"),
    )
    args = parser.parse_args(argv)
    try:
        profile = load_json(args.profile)
        schema = load_json(args.schema)
        errors = profile_errors(profile, schema)
    except (OSError, ValueError) as exc:
        print(f"FAIL: unable to load profile or schema: {exc}", file=sys.stderr)
        return 2
    if errors:
        print("FAIL: endpoint profile violates the contract")
        for error in errors:
            print(f"- {error}")
        return 1
    print("VALID: endpoint profile contract only; no endpoint was contacted")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
