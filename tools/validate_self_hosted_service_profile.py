#!/usr/bin/env python3
# Copyright (C) 2026 Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Strict declaration validator for self-hosted service profiles.

This validates declared structure and safety invariants. It does not deploy,
build, observe, or qualify an application runtime.
"""
from __future__ import annotations

import argparse
import ipaddress
import json
import re
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
_DNS_LABEL = r"(?:[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?)"
_DNS_HOSTNAME_RE = re.compile(rf"^(?=.{{1,253}}$){_DNS_LABEL}(?:\.{_DNS_LABEL})+$")

_PRIVATE_NETWORKS = (
    ipaddress.ip_network("10.0.0.0/8"),
    ipaddress.ip_network("172.16.0.0/12"),
    ipaddress.ip_network("192.168.0.0/16"),
    ipaddress.ip_network("fc00::/7"),
)

# Special-purpose source space that must not be smuggled into a broader CIDR
# merely because the first/last address of that broader network is_global.
# Snapshot derived from IANA's IPv4/IPv6 Special-Purpose Address Registries;
# keep the source policy conservative and review updates to those registries.
_NON_GENERAL_PUBLIC_SOURCE_NETWORKS = tuple(
    ipaddress.ip_network(cidr)
    for cidr in (
        # IPv4 special-purpose ranges that are not general public source space.
        "0.0.0.0/8",
        "100.64.0.0/10",
        "127.0.0.0/8",
        "169.254.0.0/16",
        "192.0.0.0/24",
        "192.0.2.0/24",
        "192.88.99.0/24",  # Deprecated 6to4 relay anycast.
        "198.18.0.0/15",
        "198.51.100.0/24",
        "203.0.113.0/24",
        "240.0.0.0/4",
        "255.255.255.255/32",
        # IPv6 special-purpose ranges not intended as general public sources.
        "::/128",
        "::1/128",
        "::ffff:0:0/96",
        "64:ff9b:1::/48",
        "100::/64",
        "100:0:0:1::/64",
        "2001::/23",
        "2001:2::/48",
        "2001:db8::/32",
        "2002::/16",
        "3fff::/20",
        "5f00::/16",
        "fe80::/10",
    )
)


def _is_private_address(address: ipaddress.IPv4Address | ipaddress.IPv6Address) -> bool:
    """Recognize RFC1918 IPv4 and IPv6 ULA only—not every special-purpose range."""
    return any(
        address.version == network.version and address in network
        for network in _PRIVATE_NETWORKS
    )


def _is_private_network(network: ipaddress.IPv4Network | ipaddress.IPv6Network) -> bool:
    """Require a CIDR to fit wholly inside one RFC1918 or IPv6 ULA range."""
    return any(
        network.version == private.version and network.subnet_of(private)
        for private in _PRIVATE_NETWORKS
    )


def _is_acceptable_public_source_network(
    network: ipaddress.IPv4Network | ipaddress.IPv6Network,
) -> bool:
    """Accept private source ranges, globally routed unicast, or explicit world-wide /0.

    Python's is_global follows IANA special-purpose registry semantics, but broad
    CIDR aggregates can contain special-use space even when their endpoints look
    global. Keep non-default aggregates bounded and ensure both endpoints are
    global. Exact /0 is an intentional, explicit public-wide allow-list.
    """
    default_route = ipaddress.ip_network("0.0.0.0/0" if network.version == 4 else "::/0")
    if network == default_route or _is_private_network(network):
        return True

    minimum_prefix = 8 if network.version == 4 else 16
    if network.prefixlen < minimum_prefix:
        return False

    # A broad aggregate that crosses into RFC1918/ULA is not a private CIDR;
    # don't let it pass as a public range merely because its endpoints are global.
    if any(
        network.version == private.version
        and network.overlaps(private)
        and not network.subnet_of(private)
        for private in _PRIVATE_NETWORKS
    ):
        return False

    if any(
        network.version == special.version and network.overlaps(special)
        for special in _NON_GENERAL_PUBLIC_SOURCE_NETWORKS
    ):
        return False

    first, last = network.network_address, network[-1]
    return (
        first.is_global
        and last.is_global
        and not network.is_multicast
        and not network.is_loopback
        and not network.is_link_local
        and not network.is_unspecified
        and not network.is_reserved
    )



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


def _safe_profile_digest(profile: Any) -> str | None:
    """Return a canonical digest when input is JSON-compatible; invalid input must not crash reporting."""
    try:
        return profile_digest(profile)
    except (TypeError, ValueError, OverflowError):
        return None


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
        dependencies = profile["dependencies"]["items"]
        data = profile["data"]
        qualification = profile["qualification"]
        metadata = profile["metadata"]

        endpoint_ids = [endpoint["id"] for endpoint in endpoints]
        if len(endpoint_ids) != len(set(endpoint_ids)):
            errors.append({"path": "/network/endpoints", "message": "endpoint IDs must be unique"})

        health_ids = [check["id"] for check in profile["health"]["checks"]]
        if len(health_ids) != len(set(health_ids)):
            errors.append({"path": "/health/checks", "message": "health-check IDs must be unique"})

        dependency_ids = [dependency["id"] for dependency in dependencies]
        if len(dependency_ids) != len(set(dependency_ids)):
            errors.append({"path": "/dependencies/items", "message": "dependency IDs must be unique"})
        dependencies_by_id = {dependency["id"]: dependency for dependency in dependencies}
        for index, dependency in enumerate(dependencies):
            if dependency["role"] in {"platform-control-plane", "coordination-service", "telemetry-export"} and dependency["required_for_start"]:
                errors.append({
                    "path": f"/dependencies/items/{index}/required_for_start",
                    "message": "control-plane, coordination, and telemetry dependencies cannot be required for service startup",
                })

        active_socket_claims: list[tuple[str, str, str, int, int]] = []
        active_ingress_claims: list[tuple[str, str, str, int, int, set[str]]] = []
        for index, endpoint in enumerate(endpoints):
            path = f"/network/endpoints/{index}"
            try:
                bind_address = ipaddress.ip_address(endpoint["bind_address"])
                bind_key = str(bind_address)
            except ValueError:
                errors.append({"path": path + "/bind_address", "message": "bind_address must be a literal IPv4 or IPv6 address"})
                bind_address = None
                bind_key = None

            if "port_range" in endpoint and endpoint["port_range"][0] > endpoint["port_range"][1]:
                errors.append({"path": path + "/port_range", "message": "port range start must not exceed end"})
            if endpoint["enabled"] and endpoint["exposure"] == "disabled":
                errors.append({"path": path, "message": "an enabled endpoint cannot have disabled exposure"})
            if not endpoint["enabled"] and endpoint["exposure"] != "disabled":
                errors.append({"path": path, "message": "a disabled endpoint must declare exposure=disabled"})

            if endpoint["kind"] == "ingress-route":
                ingress_id = endpoint["ingress_dependency_id"]
                ingress_dependency = dependencies_by_id.get(ingress_id)
                if ingress_dependency is None:
                    errors.append({
                        "path": path + "/ingress_dependency_id",
                        "message": "ingress route must reference a declared dependency",
                    })
                elif ingress_dependency["role"] != "reverse-proxy":
                    errors.append({
                        "path": path + "/ingress_dependency_id",
                        "message": "ingress route dependency must have role='reverse-proxy'",
                    })
                normalized_hosts = {
                    hostname.rstrip(".").lower()
                    for hostname in endpoint["ingress_hostnames"]
                }
                if "" in normalized_hosts:
                    errors.append({
                        "path": path + "/ingress_hostnames",
                        "message": "ingress hostnames must not be blank",
                    })
                for host_index, raw_hostname in enumerate(endpoint["ingress_hostnames"]):
                    hostname = raw_hostname.rstrip(".").lower()
                    try:
                        ipaddress.ip_address(hostname)
                    except ValueError:
                        hostname_is_ip_literal = False
                    else:
                        hostname_is_ip_literal = True
                    hostname_is_ambiguous_numeric_name = bool(hostname) and all(
                        label.isdigit() for label in hostname.split(".")
                    )
                    if (
                        not _DNS_HOSTNAME_RE.fullmatch(hostname)
                        or hostname_is_ip_literal
                        or hostname_is_ambiguous_numeric_name
                    ):
                        errors.append({
                            "path": f"{path}/ingress_hostnames/{host_index}",
                            "message": "ingress hostnames must be valid ASCII DNS names using IDNA A-labels, not IP literals or all-numeric dotted names, and without spaces, underscores, or wildcards",
                        })
                if len(normalized_hosts) != len(endpoint["ingress_hostnames"]):
                    errors.append({
                        "path": path + "/ingress_hostnames",
                        "message": "ingress hostnames must be unique after case and trailing-dot normalization",
                    })
            else:
                normalized_hosts = set()

            if endpoint["enabled"] and bind_key is not None and ("port" in endpoint or "port_range" in endpoint):
                start_port, end_port = (
                    (endpoint["port_range"][0], endpoint["port_range"][1])
                    if "port_range" in endpoint
                    else (endpoint["port"], endpoint["port"])
                )
                if endpoint["kind"] == "listener":
                    for prior_id, prior_bind, prior_version, prior_start, prior_end in active_socket_claims:
                        if (
                            prior_bind == bind_key
                            and prior_version == endpoint["protocol"]
                            and start_port <= prior_end
                            and prior_start <= end_port
                        ):
                            errors.append({
                                "path": path,
                                "message": (
                                    f"enabled listener overlaps ports/protocol with endpoint {prior_id!r} "
                                    "on the same bind address"
                                ),
                            })
                    active_socket_claims.append((
                        endpoint["id"], bind_key, endpoint["protocol"], start_port, end_port
                    ))
                else:
                    ingress_id = endpoint["ingress_dependency_id"]
                    for prior_id, prior_ingress, prior_protocol, prior_start, prior_end, prior_hosts in active_ingress_claims:
                        if (
                            prior_ingress == ingress_id
                            and prior_protocol == endpoint["protocol"]
                            and start_port <= prior_end
                            and prior_start <= end_port
                            and normalized_hosts.intersection(prior_hosts)
                        ):
                            errors.append({
                                "path": path,
                                "message": (
                                    f"ingress hostname/port overlaps endpoint {prior_id!r} "
                                    "on the same declared reverse proxy"
                                ),
                            })
                    # This contract identifies the proxy by dependency ID. Backend
                    # addresses are not listener identities: identical host/port claims
                    # on the same proxy conflict even if their backend bind addresses differ.
                    active_ingress_claims.append((
                        endpoint["id"], ingress_id, endpoint["protocol"],
                        start_port, end_port, normalized_hosts
                    ))

            source_networks = []
            for source_index, source_cidr in enumerate(endpoint["allowed_source_cidrs"]):
                try:
                    source_networks.append(ipaddress.ip_network(source_cidr, strict=True))
                except ValueError:
                    errors.append({
                        "path": f"{path}/allowed_source_cidrs/{source_index}",
                        "message": "allowed source must be a valid IPv4 or IPv6 CIDR",
                    })

            if endpoint["enabled"]:
                if bind_address is None:
                    pass  # The malformed address is already reported above.
                elif bind_address.is_unspecified or bind_address.is_multicast:
                    errors.append({
                        "path": path + "/bind_address",
                        "message": "enabled endpoints cannot bind to wildcard or multicast addresses",
                    })

                if endpoint["exposure"] == "loopback":
                    if bind_address is not None and not bind_address.is_loopback:
                        errors.append({
                            "path": path + "/bind_address",
                            "message": "loopback exposure requires a loopback bind address",
                        })
                    if endpoint["allowed_source_cidrs"]:
                        errors.append({
                            "path": path + "/allowed_source_cidrs",
                            "message": "loopback endpoints do not use remote source CIDRs",
                        })
                elif endpoint["exposure"] == "private-network":
                    if bind_address is not None and not _is_private_address(bind_address):
                        errors.append({
                            "path": path + "/bind_address",
                            "message": "private-network exposure requires an explicitly private bind address",
                        })
                    if not source_networks:
                        errors.append({
                            "path": path + "/allowed_source_cidrs",
                            "message": "private-network exposure requires explicit allowed source CIDRs",
                        })
                    for source_index, source_network in enumerate(source_networks):
                        if bind_address is not None and source_network.version != bind_address.version:
                            errors.append({
                                "path": f"{path}/allowed_source_cidrs/{source_index}",
                                "message": "private endpoint source CIDR family must match the declared bind-address family",
                            })
                        if (
                            not _is_private_network(source_network)
                            or source_network.is_unspecified
                            or source_network.prefixlen == 0
                        ):
                            errors.append({
                                "path": f"{path}/allowed_source_cidrs/{source_index}",
                                "message": "private-network exposure accepts only explicit private source CIDRs, not wildcard ranges",
                            })
                elif endpoint["exposure"] == "public-network":
                    if bind_address is not None and (bind_address.is_loopback or bind_address.is_link_local):
                        errors.append({
                            "path": path + "/bind_address",
                            "message": "public-network endpoints cannot bind to loopback or link-local addresses",
                        })
                    if not source_networks:
                        errors.append({
                            "path": path + "/allowed_source_cidrs",
                            "message": "public-network exposure requires an explicit source policy, including an explicit public-wide CIDR if intended",
                        })
                    else:
                        for source_index, source_network in enumerate(source_networks):
                            if not _is_acceptable_public_source_network(source_network):
                                errors.append({
                                    "path": f"{path}/allowed_source_cidrs/{source_index}",
                                    "message": (
                                        "public-network source CIDR must be a private RFC1918/IPv6 ULA range, "
                                        "a bounded globally routable unicast range, or an explicit 0.0.0.0/0 "
                                        "or ::/0; special-purpose and over-broad non-default ranges are rejected"
                                    ),
                                })
                        if all(_is_private_network(source_network) for source_network in source_networks):
                            errors.append({
                                "path": path + "/allowed_source_cidrs",
                                "message": "public-network exposure must declare at least one explicitly routed/public source range; private CIDRs alone contradict public exposure",
                            })

        public_endpoints = [
            endpoint["id"] for endpoint in endpoints
            if endpoint["exposure"] == "public-network"
        ]
        approved_public_endpoint_ids = set(network["public_exposure_approved_endpoint_ids"])
        if approved_public_endpoint_ids != set(public_endpoints):
            errors.append({
                "path": "/network/public_exposure_approved_endpoint_ids",
                "message": "public exposure approval scope must exactly match the currently declared public endpoint IDs",
            })
        if public_endpoints and not network["public_exposure_approved"]:
            errors.append({
                "path": "/network/public_exposure_approved",
                "message": "public-network endpoints require explicit exposure approval",
            })
        if network["public_exposure_approved"] and not public_endpoints:
            errors.append({
                "path": "/network/public_exposure_approved",
                "message": "public exposure approval must be scoped to at least one currently declared public endpoint",
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

        capture_policy = data.get("sensitive_capture")
        if (
            capture_policy
            and capture_policy["default_state"] != "disabled-by-default"
            and not capture_policy.get("policy_reference")
        ):
            errors.append({
                "path": "/data/sensitive_capture/policy_reference",
                "message": "enabled/opt-in sensitive capture requires a reviewed policy reference",
            })

        mode = source_pin["mode"]
        expected_pin_mode = {
            "nixos-native": "host-flake-lock",
            "oci-container": "oci-digest",
            "virtual-machine": "guest-image-digest",
        }[runtime["kind"]]

        if mode not in {expected_pin_mode, "unresolved"}:
            errors.append({
                "path": "/runtime/source_pin/mode",
                "message": (
                    f"runtime kind {runtime['kind']!r} requires source-pin mode "
                    f"{expected_pin_mode!r} or 'unresolved'"
                ),
            })

        if mode == "unresolved":
            if source_pin.get("digest") is not None:
                errors.append({
                    "path": "/runtime/source_pin/digest",
                    "message": "an unresolved source pin must not claim a resolved digest",
                })
            if qualification["maturity"] not in {"M0_PROPOSED", "M1_DECLARATION_VALID"}:
                blockers.append("runtime source pin is unresolved for the claimed maturity")
        elif mode == "oci-digest":
            if not source_pin.get("digest"):
                errors.append({
                    "path": "/runtime/source_pin/digest",
                    "message": "OCI workloads require an immutable SHA-256 digest; mutable tags are insufficient",
                })
            elif source_pin["reference"].rsplit("@", 1)[-1] != source_pin["digest"]:
                errors.append({
                    "path": "/runtime/source_pin/reference",
                    "message": "OCI reference must embed the same @sha256 digest declared in source_pin.digest",
                })
        elif mode == "guest-image-digest" and not source_pin.get("digest"):
            errors.append({
                "path": "/runtime/source_pin/digest",
                "message": "VM appliance profiles require a concrete guest-image SHA-256 digest",
            })

        if (
            mode == "host-flake-lock"
            and not source_pin.get("digest")
            and qualification["maturity"] in {
                "M2_SANDBOX_TESTED", "M3_CONTROLLED_DEPLOYMENT", "M4_INDEPENDENT_ASSESSMENT"
            }
        ):
            blockers.append(
                "NixOS runtime maturity above declaration-only requires the built derivation/closure digest"
            )

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
        "profile_digest_sha256": _safe_profile_digest(profile) if isinstance(profile, (dict, list)) else None,
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

    schema: dict[str, Any] | None = None
    profile: dict[str, Any] | None = None
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
            "profile_digest_sha256": _safe_profile_digest(profile) if profile is not None else None,
            "schema_digest_sha256": _safe_profile_digest(schema) if schema is not None else None,
            "errors": [{"path": "/", "message": str(exc)}],
            "blockers": [],
            "notice": "Invalid input is rejected before runtime qualification.",
        }

    print(json.dumps(report, ensure_ascii=False, sort_keys=True, indent=2))
    return {"VALID_DECLARATION": 0, "INVALID": 2, "QUALIFICATION_BLOCKED": 4}.get(report["result"], 2)


if __name__ == "__main__":
    raise SystemExit(main())
