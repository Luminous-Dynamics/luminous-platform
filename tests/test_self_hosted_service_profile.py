# Copyright (C) 2026 Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Regression tests for self-hosted service profile declaration validation."""
from __future__ import annotations

import copy
import json
import subprocess
import sys
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

    def test_private_endpoint_requires_restricted_private_source_cidrs(self):
        profile = copy.deepcopy(BASE)
        profile["network"]["endpoints"][0]["allowed_source_cidrs"] = ["0.0.0.0/0"]
        report = self.validate(profile)
        self.assertEqual(report["result"], "INVALID")
        self.assertTrue(any("only explicit private source CIDRs" in error["message"] for error in report["errors"]))

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
