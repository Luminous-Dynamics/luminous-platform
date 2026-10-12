# Copyright (C) 2026 Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Regression tests for self-hosted service profile declaration validation."""
from __future__ import annotations

import copy
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))

from validate_deployment_profile import ProfileInputError, parse_profile_yaml, profile_digest  # noqa: E402
from validate_self_hosted_service_profile import (  # noqa: E402
    load_schema,
    validate_profile_data,
)

SCHEMA_PATH = ROOT / "schemas" / "self-hosted-service-profile-v1.schema.json"
FIXTURE = ROOT / "profiles" / "services" / "asterisk-pbx-nixos-v1.yaml"
NEXTCLOUD_FIXTURE = ROOT / "profiles" / "services" / "nextcloud-nixos-v1.yaml"
SCHEMA = load_schema(SCHEMA_PATH)
BASE = parse_profile_yaml(FIXTURE.read_text(encoding="utf-8"))
NEXTCLOUD = parse_profile_yaml(NEXTCLOUD_FIXTURE.read_text(encoding="utf-8"))


class SelfHostedServiceProfileTests(unittest.TestCase):
    def validate(self, profile=None):
        return validate_profile_data(copy.deepcopy(BASE if profile is None else profile), SCHEMA, "unit-test")

    def test_schema_pin_modes_match_runtime_backend_contract(self):
        expected = {
            "nixos-native": "host-flake-lock",
            "oci-container": "oci-digest",
            "virtual-machine": "guest-image-digest",
        }
        declared_modes = set(SCHEMA["properties"]["runtime"]["properties"]["source_pin"]["properties"]["mode"]["enum"])
        self.assertEqual(declared_modes, set(expected.values()) | {"unresolved"})
        self.assertNotIn("source-revision", declared_modes)

    def test_schema_is_valid_and_fixture_is_valid_declaration_only(self):
        Draft202012Validator.check_schema(SCHEMA)
        report = self.validate()
        self.assertEqual(report["result"], "VALID_DECLARATION")
        self.assertFalse(report["qualification_evaluated"])
        self.assertEqual(report["errors"], [])
        self.assertEqual(BASE["qualification"]["maturity"], "M0_PROPOSED")
        self.assertEqual(BASE["qualification"]["qualified_capabilities"], [])
        self.assertTrue(report["profile_digest_sha256"].startswith("sha256:"))

    def test_ingress_route_requires_declared_reverse_proxy_dependency(self):
        profile = copy.deepcopy(NEXTCLOUD)
        profile["network"]["endpoints"][0]["ingress_dependency_id"] = "missing-proxy"
        report = validate_profile_data(profile, SCHEMA, "unit-test")
        self.assertEqual(report["result"], "INVALID")
        self.assertTrue(any("must reference a declared dependency" in error["message"] for error in report["errors"]))

    def test_ingress_route_dependency_must_have_reverse_proxy_role(self):
        profile = copy.deepcopy(NEXTCLOUD)
        for dependency in profile["dependencies"]["items"]:
            if dependency["id"] == "tls-reverse-proxy":
                dependency["role"] = "external-provider"
        report = validate_profile_data(profile, SCHEMA, "unit-test")
        self.assertEqual(report["result"], "INVALID")
        self.assertTrue(any("role='reverse-proxy'" in error["message"] for error in report["errors"]))

    def test_ingress_routes_with_different_hostnames_can_share_proxy_listener(self):
        profile = copy.deepcopy(NEXTCLOUD)
        endpoint = copy.deepcopy(profile["network"]["endpoints"][0])
        endpoint["id"] = "private-https-other-host"
        endpoint["ingress_hostnames"] = ["files.example.invalid"]
        profile["network"]["endpoints"].append(endpoint)
        report = validate_profile_data(profile, SCHEMA, "unit-test")
        self.assertEqual(report["result"], "VALID_DECLARATION")

    def test_ingress_collision_canonicalizes_equivalent_ipv6_bind_addresses(self):
        profile = copy.deepcopy(NEXTCLOUD)
        endpoint = profile["network"]["endpoints"][0]
        endpoint["bind_address"] = "fd00::1"
        endpoint["allowed_source_cidrs"] = ["fd00::/64"]
        duplicate = copy.deepcopy(endpoint)
        duplicate["id"] = "private-https-ipv6-alias"
        duplicate["bind_address"] = "fd00:0:0:0:0:0:0:1"
        profile["network"]["endpoints"].append(duplicate)

        report = validate_profile_data(profile, SCHEMA, "unit-test")
        self.assertEqual(report["result"], "INVALID")
        self.assertTrue(any(
            "ingress hostname/port overlaps" in error["message"]
            for error in report["errors"]
        ))

    def test_ingress_collision_cannot_hide_behind_different_backend_address(self):
        profile = copy.deepcopy(NEXTCLOUD)
        first = profile["network"]["endpoints"][0]
        duplicate = copy.deepcopy(first)
        duplicate["id"] = "private-https-same-host-different-backend"
        duplicate["bind_address"] = "192.168.50.21"
        profile["network"]["endpoints"].append(duplicate)

        report = validate_profile_data(profile, SCHEMA, "unit-test")
        self.assertEqual(report["result"], "INVALID")
        self.assertTrue(any(
            "ingress hostname/port overlaps" in error["message"]
            for error in report["errors"]
        ))

    def test_duplicate_ingress_hostname_on_same_proxy_is_rejected(self):
        profile = copy.deepcopy(NEXTCLOUD)
        endpoint = copy.deepcopy(profile["network"]["endpoints"][0])
        endpoint["id"] = "private-https-duplicate"
        profile["network"]["endpoints"].append(endpoint)
        report = validate_profile_data(profile, SCHEMA, "unit-test")
        self.assertEqual(report["result"], "INVALID")
        self.assertTrue(any("ingress hostname/port overlaps" in error["message"] for error in report["errors"]))

    def test_ingress_route_rejects_ipv4_literal_and_ambiguous_numeric_names(self):
        for hostname in ("192.0.2.10", "127.1"):
            with self.subTest(hostname=hostname):
                profile = copy.deepcopy(NEXTCLOUD)
                profile["network"]["endpoints"][0]["ingress_hostnames"] = [hostname]
                report = validate_profile_data(profile, SCHEMA, "unit-test")
                self.assertEqual(report["result"], "INVALID")
                self.assertTrue(any(
                    "valid ASCII DNS names" in error["message"]
                    for error in report["errors"]
                ))

    def test_ingress_route_rejects_surrounding_hostname_whitespace(self):
        profile = copy.deepcopy(NEXTCLOUD)
        profile["network"]["endpoints"][0]["ingress_hostnames"] = [" cloud.example.invalid "]
        report = validate_profile_data(profile, SCHEMA, "unit-test")
        self.assertEqual(report["result"], "INVALID")
        self.assertTrue(any(
            "valid ASCII DNS names" in error["message"]
            for error in report["errors"]
        ))

    def test_ingress_hostname_case_and_trailing_dot_duplicates_are_rejected(self):
        profile = copy.deepcopy(NEXTCLOUD)
        profile["network"]["endpoints"][0]["ingress_hostnames"] = [
            "cloud.example.invalid", "CLOUD.EXAMPLE.INVALID."
        ]
        report = validate_profile_data(profile, SCHEMA, "unit-test")
        self.assertEqual(report["result"], "INVALID")
        self.assertTrue(any("unique after case and trailing-dot normalization" in error["message"] for error in report["errors"]))

    def test_ingress_route_rejects_invalid_dns_hostname(self):
        profile = copy.deepcopy(NEXTCLOUD)
        profile["network"]["endpoints"][0]["ingress_hostnames"] = ["cloud server.example.invalid"]
        report = validate_profile_data(profile, SCHEMA, "unit-test")
        self.assertEqual(report["result"], "INVALID")
        self.assertTrue(any("valid ASCII DNS names" in error["message"] for error in report["errors"]))

    def test_ingress_route_accepts_valid_dns_hostname_with_normalized_trailing_dot(self):
        profile = copy.deepcopy(NEXTCLOUD)
        profile["network"]["endpoints"][0]["ingress_hostnames"] = ["CLOUD.EXAMPLE.INVALID."]
        report = validate_profile_data(profile, SCHEMA, "unit-test")
        self.assertEqual(report["result"], "VALID_DECLARATION")

    def test_listener_cannot_declare_ingress_route_metadata(self):
        profile = copy.deepcopy(BASE)
        profile["network"]["endpoints"][0]["ingress_dependency_id"] = "tls-reverse-proxy"
        profile["network"]["endpoints"][0]["ingress_hostnames"] = ["pbx.example.invalid"]
        report = self.validate(profile)
        self.assertEqual(report["result"], "INVALID")

    def test_nextcloud_profile_is_valid_and_declares_stateful_dependencies(self):
        report = validate_profile_data(copy.deepcopy(NEXTCLOUD), SCHEMA, "nextcloud-fixture")
        self.assertEqual(report["result"], "VALID_DECLARATION")
        self.assertFalse(report["qualification_evaluated"])
        self.assertTrue(NEXTCLOUD["data"]["stateful"])
        self.assertNotIn("sensitive_capture", NEXTCLOUD["data"])
        self.assertEqual(
            {item["role"] for item in NEXTCLOUD["dependencies"]["items"]},
            {"data-store", "reverse-proxy", "backup-service"},
        )
        self.assertEqual(NEXTCLOUD["qualification"]["maturity"], "M0_PROPOSED")
        self.assertEqual(NEXTCLOUD["qualification"]["qualified_capabilities"], [])

    def test_unknown_top_level_field_is_rejected(self):
        profile = copy.deepcopy(BASE)
        profile["autonomous_fix"] = True
        self.assertEqual(self.validate(profile)["result"], "INVALID")

    def test_public_exposure_requires_explicit_approval(self):
        profile = copy.deepcopy(BASE)
        profile["network"]["endpoints"][0]["exposure"] = "public-network"
        report = self.validate(profile)
        self.assertEqual(report["result"], "INVALID")
        self.assertTrue(any("explicit exposure approval" in error["message"] for error in report["errors"]))

    def test_public_exposure_rejects_special_use_and_overbroad_source_ranges(self):
        invalid_ranges = (
            "127.0.0.0/8",       # loopback
            "169.254.0.0/16",    # link-local
            "224.0.0.0/4",       # multicast
            "192.0.2.0/24",      # documentation-only
            "100.64.0.0/10",     # shared carrier-grade NAT space
            "240.0.0.0/4",       # reserved
            "192.88.99.0/24",    # deprecated 6to4 relay anycast; endpoints look global
            "192.88.98.0/23",    # endpoints look global but range overlaps deprecated special-use space
            "198.0.0.0/8",       # endpoints look global but range includes benchmarking space
            "128.0.0.0/1",       # over-broad aggregate
            "8000::/1",           # over-broad IPv6 aggregate
            "2001:db8::/32",     # IPv6 documentation-only
        )
        for source_cidr in invalid_ranges:
            with self.subTest(source_cidr=source_cidr):
                profile = copy.deepcopy(BASE)
                network = profile["network"]
                network["public_exposure_approved"] = True
                network["public_exposure_assessment_ref"] = "review:synthetic-public-edge"
                network["public_exposure_approved_endpoint_ids"] = ["sip-udp"]
                endpoint = network["endpoints"][0]
                endpoint["exposure"] = "public-network"
                endpoint["allowed_source_cidrs"] = [source_cidr]
                report = self.validate(profile)
                self.assertEqual(report["result"], "INVALID")
                self.assertTrue(any(
                    "special-purpose and over-broad non-default ranges are rejected" in error["message"]
                    for error in report["errors"]
                ))

    def test_public_exposure_accepts_explicit_public_wide_default_route(self):
        profile = copy.deepcopy(BASE)
        network = profile["network"]
        network["public_exposure_approved"] = True
        network["public_exposure_assessment_ref"] = "review:synthetic-public-edge"
        network["public_exposure_approved_endpoint_ids"] = ["sip-udp"]
        endpoint = network["endpoints"][0]
        endpoint["exposure"] = "public-network"
        endpoint["allowed_source_cidrs"] = ["0.0.0.0/0"]
        report = self.validate(profile)
        self.assertEqual(report["result"], "VALID_DECLARATION")

    def test_public_exposure_rejects_private_only_source_ranges_even_with_approval(self):
        profile = copy.deepcopy(BASE)
        profile["network"]["public_exposure_approved"] = True
        profile["network"]["public_exposure_assessment_ref"] = "review:synthetic-public-edge"
        profile["network"]["endpoints"][0]["exposure"] = "public-network"
        report = self.validate(profile)
        self.assertEqual(report["result"], "INVALID")
        self.assertTrue(any("private CIDRs alone contradict public exposure" in error["message"] for error in report["errors"]))

    def test_loopback_exposure_requires_loopback_bind(self):
        profile = copy.deepcopy(BASE)
        profile["network"]["endpoints"][0]["exposure"] = "loopback"
        report = self.validate(profile)
        self.assertEqual(report["result"], "INVALID")
        self.assertTrue(any("requires a loopback bind address" in error["message"] for error in report["errors"]))

    def test_public_endpoint_rejects_loopback_and_link_local_binds(self):
        for bind_address in ("127.0.0.1", "fe80::1"):
            with self.subTest(bind_address=bind_address):
                profile = copy.deepcopy(BASE)
                profile["network"]["public_exposure_approved"] = True
                profile["network"]["public_exposure_assessment_ref"] = "review:synthetic-public-edge"
                endpoint = profile["network"]["endpoints"][0]
                endpoint["exposure"] = "public-network"
                endpoint["bind_address"] = bind_address
                endpoint["allowed_source_cidrs"] = ["0.0.0.0/0"]
                report = self.validate(profile)
                self.assertEqual(report["result"], "INVALID")
                self.assertTrue(any("cannot bind to loopback or link-local" in error["message"] for error in report["errors"]))

    def test_public_approval_cannot_silently_expand_to_new_endpoint(self):
        profile = copy.deepcopy(BASE)
        profile["network"]["public_exposure_approved"] = True
        profile["network"]["public_exposure_assessment_ref"] = "review:synthetic-public-edge"
        profile["network"]["public_exposure_approved_endpoint_ids"] = ["sip-udp"]
        profile["network"]["endpoints"][0]["exposure"] = "public-network"
        profile["network"]["endpoints"][0]["allowed_source_cidrs"] = ["0.0.0.0/0"]
        profile["network"]["endpoints"][2]["exposure"] = "public-network"
        profile["network"]["endpoints"][2]["allowed_source_cidrs"] = ["0.0.0.0/0"]
        report = self.validate(profile)
        self.assertEqual(report["result"], "INVALID")
        self.assertTrue(any("scope must exactly match" in error["message"] for error in report["errors"]))

    def test_public_exposure_can_use_private_bind_behind_an_explicit_edge(self):
        profile = copy.deepcopy(BASE)
        profile["network"]["public_exposure_approved"] = True
        profile["network"]["public_exposure_assessment_ref"] = "review:synthetic-public-edge"
        profile["network"]["public_exposure_approved_endpoint_ids"] = ["sip-udp"]
        endpoint = profile["network"]["endpoints"][0]
        endpoint["exposure"] = "public-network"
        endpoint["bind_address"] = "192.168.50.10"
        endpoint["public_edge_mapping_ref"] = "edge-review:pbx-public-sip"
        endpoint["allowed_source_cidrs"] = ["0.0.0.0/0"]
        report = self.validate(profile)
        self.assertEqual(report["result"], "VALID_DECLARATION")

    def test_private_bind_for_public_endpoint_requires_edge_mapping_reference(self):
        profile = copy.deepcopy(BASE)
        network = profile["network"]
        network["public_exposure_approved"] = True
        network["public_exposure_assessment_ref"] = "review:synthetic-public-edge"
        network["public_exposure_approved_endpoint_ids"] = ["sip-udp"]
        endpoint = network["endpoints"][0]
        endpoint["exposure"] = "public-network"
        endpoint["bind_address"] = "192.168.50.10"
        endpoint["allowed_source_cidrs"] = ["0.0.0.0/0"]

        report = self.validate(profile)
        self.assertEqual(report["result"], "INVALID")
        self.assertTrue(any(
            "explicit edge/NAT mapping reference" in error["message"]
            for error in report["errors"]
        ))

    def test_public_bind_rejects_documentation_reserved_and_mapped_addresses(self):
        for bind_address in ("192.0.2.10", "240.0.0.1", "255.255.255.255", "::ffff:8.8.8.8"):
            with self.subTest(bind_address=bind_address):
                profile = copy.deepcopy(BASE)
                network = profile["network"]
                network["public_exposure_approved"] = True
                network["public_exposure_assessment_ref"] = "review:synthetic-public-edge"
                network["public_exposure_approved_endpoint_ids"] = ["sip-udp"]
                endpoint = network["endpoints"][0]
                endpoint["exposure"] = "public-network"
                endpoint["bind_address"] = bind_address
                endpoint["allowed_source_cidrs"] = ["0.0.0.0/0"]
                report = self.validate(profile)
                self.assertEqual(report["result"], "INVALID")
                self.assertTrue(any(
                    "globally routable unicast address" in error["message"]
                    for error in report["errors"]
                ))

    def test_public_approval_must_target_a_declared_public_endpoint(self):
        profile = copy.deepcopy(BASE)
        profile["network"]["public_exposure_approved"] = True
        profile["network"]["public_exposure_assessment_ref"] = "review:synthetic-public-edge"
        report = self.validate(profile)
        self.assertEqual(report["result"], "INVALID")
        self.assertTrue(any("at least one currently declared public endpoint" in error["message"] for error in report["errors"]))

    def test_endpoint_must_declare_exactly_one_port_or_port_range(self):
        profile = copy.deepcopy(BASE)
        profile["network"]["endpoints"][0].pop("port", None)
        report = self.validate(profile)
        self.assertEqual(report["result"], "INVALID")

    def test_unsupported_icmp_endpoint_protocol_is_rejected(self):
        profile = copy.deepcopy(BASE)
        profile["network"]["endpoints"][0]["protocol"] = "icmp"
        report = self.validate(profile)
        self.assertEqual(report["result"], "INVALID")

    def test_public_approval_requires_review_reference(self):
        profile = copy.deepcopy(BASE)
        profile["network"]["public_exposure_approved"] = True
        profile["network"]["endpoints"][0]["exposure"] = "public-network"
        report = self.validate(profile)
        self.assertEqual(report["result"], "INVALID")
        self.assertTrue(any("assessment reference" in error["message"] for error in report["errors"]))

    def test_overlapping_enabled_listener_claims_are_rejected(self):
        profile = copy.deepcopy(BASE)
        duplicate = copy.deepcopy(profile["network"]["endpoints"][0])
        duplicate["id"] = "sip-udp-duplicate"
        profile["network"]["endpoints"].append(duplicate)
        report = self.validate(profile)
        self.assertEqual(report["result"], "INVALID")
        self.assertTrue(any("overlaps ports/protocol" in error["message"] for error in report["errors"]))

    def test_equivalent_ipv6_spellings_do_not_bypass_listener_collision_check(self):
        profile = copy.deepcopy(BASE)
        first = profile["network"]["endpoints"][0]
        first["bind_address"] = "fd12:3456:789a::10"
        first["allowed_source_cidrs"] = ["fd12:3456:789a::/64"]
        duplicate = copy.deepcopy(first)
        duplicate["id"] = "same-listener-expanded-ipv6"
        duplicate["bind_address"] = "fd12:3456:789a:0:0:0:0:10"
        profile["network"]["endpoints"].append(duplicate)
        report = self.validate(profile)
        self.assertEqual(report["result"], "INVALID")
        self.assertTrue(any("overlaps ports/protocol" in error["message"] for error in report["errors"]))

    def test_same_port_on_distinct_protocols_is_allowed(self):
        profile = copy.deepcopy(BASE)
        tcp_endpoint = copy.deepcopy(profile["network"]["endpoints"][0])
        tcp_endpoint["id"] = "sip-tcp-lab"
        tcp_endpoint["protocol"] = "tcp"
        profile["network"]["endpoints"].append(tcp_endpoint)
        report = self.validate(profile)
        self.assertEqual(report["result"], "VALID_DECLARATION")

    def test_inverted_port_range_is_rejected(self):
        profile = copy.deepcopy(BASE)
        profile["network"]["endpoints"][2]["port_range"] = [20000, 10000]
        report = self.validate(profile)
        self.assertEqual(report["result"], "INVALID")
        self.assertTrue(any("must not exceed" in error["message"] for error in report["errors"]))

    def test_private_endpoint_cannot_bind_to_wildcard_address(self):
        profile = copy.deepcopy(BASE)
        profile["network"]["endpoints"][0]["bind_address"] = "0.0.0.0"
        report = self.validate(profile)
        self.assertEqual(report["result"], "INVALID")
        self.assertTrue(any("wildcard or multicast" in error["message"] for error in report["errors"]))

    def test_private_endpoint_rejects_unspecified_source_cidr(self):
        profile = copy.deepcopy(BASE)
        profile["network"]["endpoints"][0]["allowed_source_cidrs"] = ["0.0.0.0/0"]
        report = self.validate(profile)
        self.assertEqual(report["result"], "INVALID")
        self.assertTrue(any("only explicit private source CIDRs" in error["message"] for error in report["errors"]))

    def test_documentation_network_is_not_accepted_as_private_source(self):
        profile = copy.deepcopy(BASE)
        profile["network"]["endpoints"][0]["allowed_source_cidrs"] = ["192.0.2.0/24"]
        report = self.validate(profile)
        self.assertEqual(report["result"], "INVALID")
        self.assertTrue(any("only explicit private source CIDRs" in error["message"] for error in report["errors"]))

    def test_noncanonical_cidr_with_host_bits_is_rejected(self):
        profile = copy.deepcopy(BASE)
        profile["network"]["endpoints"][0]["allowed_source_cidrs"] = ["192.168.50.1/24"]
        report = self.validate(profile)
        self.assertEqual(report["result"], "INVALID")
        self.assertTrue(any("valid IPv4 or IPv6 CIDR" in error["message"] for error in report["errors"]))

    def test_private_ipv6_ula_bind_and_source_are_supported(self):
        profile = copy.deepcopy(BASE)
        endpoint = profile["network"]["endpoints"][0]
        endpoint["bind_address"] = "fd12:3456:789a::10"
        endpoint["allowed_source_cidrs"] = ["fd12:3456:789a::/64"]
        report = self.validate(profile)
        self.assertEqual(report["result"], "VALID_DECLARATION")

    def test_private_source_cidr_family_must_match_bind_address(self):
        profile = copy.deepcopy(BASE)
        profile["network"]["endpoints"][0]["allowed_source_cidrs"] = ["fd12:3456:789a::/64"]
        report = self.validate(profile)
        self.assertEqual(report["result"], "INVALID")
        self.assertTrue(any("family must match" in error["message"] for error in report["errors"]))

    def test_invalid_source_cidr_is_rejected(self):
        profile = copy.deepcopy(BASE)
        profile["network"]["endpoints"][0]["allowed_source_cidrs"] = ["not-a-cidr"]
        report = self.validate(profile)
        self.assertEqual(report["result"], "INVALID")
        self.assertTrue(any("valid IPv4 or IPv6 CIDR" in error["message"] for error in report["errors"]))

    def test_duplicate_health_check_ids_are_rejected(self):
        profile = copy.deepcopy(BASE)
        profile["health"]["checks"][1]["id"] = profile["health"]["checks"][0]["id"]
        report = self.validate(profile)
        self.assertEqual(report["result"], "INVALID")
        self.assertTrue(any("health-check IDs must be unique" in error["message"] for error in report["errors"]))

    def test_enabled_endpoint_cannot_be_marked_disabled(self):
        profile = copy.deepcopy(BASE)
        profile["network"]["endpoints"][0]["exposure"] = "disabled"
        report = self.validate(profile)
        self.assertEqual(report["result"], "INVALID")

    def test_stateful_service_requires_periodic_and_pre_upgrade_backups(self):
        profile = copy.deepcopy(BASE)
        profile["data"]["backup_policy"] = "none"
        report = self.validate(profile)
        self.assertEqual(report["result"], "INVALID")
        self.assertTrue(any("periodic backups" in error["message"] for error in report["errors"]))

    def test_sensitive_capture_requires_policy_reference_when_enabled(self):
        profile = copy.deepcopy(BASE)
        profile["data"]["sensitive_capture"]["default_state"] = "explicit-opt-in-with-notice"
        report = self.validate(profile)
        self.assertEqual(report["result"], "INVALID")
        self.assertTrue(any("sensitive capture requires" in error["message"] for error in report["errors"]))

    def test_coordination_service_cannot_be_required_for_runtime_start(self):
        profile = copy.deepcopy(BASE)
        profile["dependencies"]["items"] = [{
            "id": "mycelix-coordinator",
            "role": "coordination-service",
            "required_for_start": True,
            "failure_policy": "stop-service-safely",
            "data_shared": "metadata-only",
        }]
        report = self.validate(profile)
        self.assertEqual(report["result"], "INVALID")
        self.assertTrue(any("cannot be required for service startup" in error["message"] for error in report["errors"]))

    def test_duplicate_dependency_ids_are_rejected(self):
        profile = copy.deepcopy(BASE)
        dependency = {
            "id": "local-db",
            "role": "data-store",
            "required_for_start": True,
            "failure_policy": "stop-service-safely",
            "data_shared": "application-state",
        }
        profile["dependencies"]["items"] = [copy.deepcopy(dependency), copy.deepcopy(dependency)]
        report = self.validate(profile)
        self.assertEqual(report["result"], "INVALID")
        self.assertTrue(any("dependency IDs must be unique" in error["message"] for error in report["errors"]))

    def test_oci_container_rejects_non_oci_pin_mode(self):
        profile = copy.deepcopy(BASE)
        profile["runtime"]["kind"] = "oci-container"
        profile["runtime"]["source_pin"]["mode"] = "host-flake-lock"
        report = self.validate(profile)
        self.assertEqual(report["result"], "INVALID")
        self.assertTrue(any("requires source-pin mode 'oci-digest'" in error["message"] for error in report["errors"]))

    def test_oci_container_requires_digest_even_with_oci_pin_mode(self):
        profile = copy.deepcopy(BASE)
        profile["runtime"]["kind"] = "oci-container"
        profile["runtime"]["source_pin"] = {
            "mode": "oci-digest",
            "reference": "registry.example.invalid/example/service:latest",
            "digest": None,
        }
        profile["runtime"]["update_strategy"] = "digest-refresh-reviewed"
        report = self.validate(profile)
        self.assertEqual(report["result"], "INVALID")
        self.assertTrue(any("immutable SHA-256 digest" in error["message"] for error in report["errors"]))

    def test_digest_pinned_oci_reference_is_valid_declaration(self):
        profile = copy.deepcopy(BASE)
        digest = "sha256:" + "a" * 64
        profile["metadata"]["id"] = "org.example.synthetic-oci-fixture"
        profile["application"]["name"] = "Synthetic OCI conformance fixture"
        profile["application"]["upstream_homepage"] = "https://example.invalid/"
        profile["application"]["source_repository"] = "https://example.invalid/source"
        profile["runtime"]["kind"] = "oci-container"
        profile["runtime"]["operating_system"] = "Linux OCI image"
        profile["runtime"]["source_pin"] = {
            "mode": "oci-digest",
            "reference": "registry.example.invalid/example/service@" + digest,
            "digest": digest,
        }
        profile["runtime"]["update_strategy"] = "digest-refresh-reviewed"
        report = self.validate(profile)
        self.assertEqual(report["result"], "VALID_DECLARATION")
        self.assertFalse(report["qualification_evaluated"])

    def test_oci_digest_pin_requires_explicit_registry_and_repository(self):
        digest = "sha256:" + "e" * 64
        invalid_references = (
            digest,
            "@" + digest,
            "service@" + digest,
            "registry.example.invalid/@" + digest,
            "registry.example.invalid/example/service @" + digest,
            "registry.example.invalid/example/service@sha256:" + "f" * 64,
        )
        for reference in invalid_references:
            with self.subTest(reference=reference):
                profile = copy.deepcopy(BASE)
                profile["runtime"]["kind"] = "oci-container"
                profile["runtime"]["source_pin"] = {
                    "mode": "oci-digest",
                    "reference": reference,
                    "digest": digest,
                }
                profile["runtime"]["update_strategy"] = "digest-refresh-reviewed"
                report = self.validate(profile)
                self.assertEqual(report["result"], "INVALID")
                self.assertTrue(any(
                    "explicit registry/repository locator" in error["message"]
                    for error in report["errors"]
                ))

    def test_oci_reference_digest_must_match_declared_digest(self):
        profile = copy.deepcopy(BASE)
        profile["runtime"]["kind"] = "oci-container"
        profile["runtime"]["source_pin"] = {
            "mode": "oci-digest",
            "reference": "registry.example.invalid/example/service:latest",
            "digest": "sha256:" + "b" * 64,
        }
        profile["runtime"]["update_strategy"] = "digest-refresh-reviewed"
        report = self.validate(profile)
        self.assertEqual(report["result"], "INVALID")
        self.assertTrue(any("must embed the same @sha256 digest" in error["message"] for error in report["errors"]))

    def test_vm_appliance_profile_can_be_declared_without_qualification(self):
        profile = copy.deepcopy(BASE)
        digest = "sha256:" + "c" * 64
        profile["metadata"]["id"] = "org.example.synthetic-vm-fixture"
        profile["application"]["name"] = "Synthetic VM appliance fixture"
        profile["application"]["upstream_homepage"] = "https://example.invalid/"
        profile["application"]["source_repository"] = "https://example.invalid/source"
        profile["runtime"]["kind"] = "virtual-machine"
        profile["runtime"]["operating_system"] = "Debian guest"
        profile["runtime"]["source_pin"] = {
            "mode": "guest-image-digest",
            "reference": "debian-guest-image",
            "digest": digest,
        }
        profile["runtime"]["update_strategy"] = "guest-os-managed-reviewed"
        profile["data"].pop("sensitive_capture", None)
        profile["dependencies"]["items"] = [{
            "id": "guest-virtualization-host",
            "role": "application",
            "required_for_start": True,
            "failure_policy": "stop-service-safely",
            "data_shared": "none",
        }]
        profile["lifecycle"]["installation"] = "appliance-provisioning"
        profile["lifecycle"]["upgrade"] = "guest-os-managed"
        profile["lifecycle"]["rollback"] = "snapshot-restore"
        report = self.validate(profile)
        self.assertEqual(report["result"], "VALID_DECLARATION")
        self.assertFalse(report["qualification_evaluated"])

    def test_native_nixos_runtime_rejects_oci_pin_mode(self):
        profile = copy.deepcopy(BASE)
        digest = "sha256:" + "d" * 64
        profile["runtime"]["source_pin"] = {
            "mode": "oci-digest",
            "reference": "registry.example.invalid/example/service@" + digest,
            "digest": digest,
        }
        report = self.validate(profile)
        self.assertEqual(report["result"], "INVALID")
        self.assertTrue(any("runtime kind 'nixos-native' requires source-pin mode 'host-flake-lock'" in error["message"] for error in report["errors"]))

    def test_vm_runtime_rejects_host_flake_pin_mode(self):
        profile = copy.deepcopy(BASE)
        profile["runtime"]["kind"] = "virtual-machine"
        profile["runtime"]["source_pin"]["mode"] = "host-flake-lock"
        profile["runtime"]["update_strategy"] = "guest-os-managed-reviewed"
        report = self.validate(profile)
        self.assertEqual(report["result"], "INVALID")
        self.assertTrue(any("runtime kind 'virtual-machine' requires source-pin mode 'guest-image-digest'" in error["message"] for error in report["errors"]))

    def test_vm_guest_image_pin_requires_digest(self):
        profile = copy.deepcopy(BASE)
        profile["runtime"]["kind"] = "virtual-machine"
        profile["runtime"]["source_pin"] = {
            "mode": "guest-image-digest",
            "reference": "debian-guest-image",
            "digest": None,
        }
        profile["runtime"]["update_strategy"] = "guest-os-managed-reviewed"
        report = self.validate(profile)
        self.assertEqual(report["result"], "INVALID")
        self.assertTrue(any("concrete guest-image SHA-256 digest" in error["message"] for error in report["errors"]))

    def test_nixos_advanced_maturity_requires_derivation_digest(self):
        profile = copy.deepcopy(BASE)
        profile["qualification"]["maturity"] = "M2_SANDBOX_TESTED"
        report = self.validate(profile)
        self.assertEqual(report["result"], "QUALIFICATION_BLOCKED")
        self.assertTrue(any("built derivation/closure digest" in blocker for blocker in report["blockers"]))

    def test_non_finite_invalid_value_does_not_crash_digest_reporting(self):
        profile = copy.deepcopy(BASE)
        profile["metadata"]["description"] = float("nan")
        report = self.validate(profile)
        self.assertEqual(report["result"], "INVALID")
        self.assertIsNone(report["profile_digest_sha256"])

    def test_validator_never_qualifies_runtime_even_with_references(self):
        profile = copy.deepcopy(BASE)
        profile["metadata"]["status"] = "qualified"
        profile["qualification"]["maturity"] = "M3_CONTROLLED_DEPLOYMENT"
        profile["qualification"]["qualified_capabilities"] = ["private-lab-sip-call-flow"]
        profile["qualification"]["evidence_refs"] = ["https://example.invalid/evidence/receipt.json"]
        report = self.validate(profile)
        self.assertEqual(report["result"], "QUALIFICATION_BLOCKED")
        self.assertFalse(report["qualification_evaluated"])
        self.assertTrue(any("declarations only" in blocker for blocker in report["blockers"]))

    def test_unresolved_source_cannot_claim_tested_maturity(self):
        profile = copy.deepcopy(BASE)
        profile["runtime"]["source_pin"]["mode"] = "unresolved"
        profile["qualification"]["maturity"] = "M2_SANDBOX_TESTED"
        report = self.validate(profile)
        self.assertEqual(report["result"], "QUALIFICATION_BLOCKED")
        self.assertTrue(any("source pin is unresolved" in blocker for blocker in report["blockers"]))

    def test_strict_yaml_parser_rejects_duplicate_keys(self):
        with self.assertRaises(ProfileInputError):
            parse_profile_yaml("schema: luminous.self-hosted-service-profile/v1\nschema: duplicate\n")

    def test_profile_digest_is_independent_of_mapping_key_order(self):
        reversed_profile = dict(reversed(list(BASE.items())))
        self.assertEqual(profile_digest(BASE), profile_digest(reversed_profile))

    def test_cli_reports_malformed_yaml_in_stable_json_shape(self):
        with tempfile.TemporaryDirectory() as temp_dir:
            bad_profile = Path(temp_dir) / "duplicate-key.yaml"
            bad_profile.write_text(
                "schema: luminous.self-hosted-service-profile/v1\\nschema: duplicate\\n",
                encoding="utf-8",
            )
            proc = subprocess.run(
                [sys.executable, str(ROOT / "tools" / "validate_self_hosted_service_profile.py"), str(bad_profile)],
                cwd=ROOT,
                text=True,
                capture_output=True,
                check=False,
            )
        self.assertEqual(proc.returncode, 2, msg=proc.stderr + proc.stdout)
        report = json.loads(proc.stdout)
        self.assertEqual(report["result"], "INVALID")
        self.assertFalse(report["qualification_evaluated"])
        self.assertIsNone(report["profile_digest_sha256"])
        self.assertTrue(report["schema_digest_sha256"].startswith("sha256:"))
        self.assertTrue(report["errors"])

    def test_cli_emits_valid_declaration_without_qualifying_runtime(self):
        proc = subprocess.run(
            [sys.executable, str(ROOT / "tools" / "validate_self_hosted_service_profile.py"), str(FIXTURE)],
            cwd=ROOT,
            text=True,
            capture_output=True,
            check=False,
        )
        self.assertEqual(proc.returncode, 0, msg=proc.stderr + proc.stdout)
        report = json.loads(proc.stdout)
        self.assertEqual(report["result"], "VALID_DECLARATION")
        self.assertFalse(report["qualification_evaluated"])


if __name__ == "__main__":
    unittest.main()
