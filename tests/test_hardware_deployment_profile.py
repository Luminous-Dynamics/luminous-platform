"""Adversarial tests for the draft hardware deployment profile."""

from __future__ import annotations

import copy
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))

from validate_hardware_deployment_profile import profile_errors  # noqa: E402
from validate_hardware_portfolio import load_json  # noqa: E402


class HardwareDeploymentProfileTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.profile = load_json(ROOT / "profiles/sa-single-site-office-lab-v1.json")
        cls.profile_schema = load_json(ROOT / "schemas/hardware-deployment-profile-v1.schema.json")
        cls.catalog = load_json(ROOT / "hardware/portfolio-v1.json")

    def errors(self, profile):
        return profile_errors(profile, self.profile_schema, self.catalog)

    def test_profile_is_structurally_valid_but_not_qualified(self) -> None:
        self.assertEqual(self.errors(copy.deepcopy(self.profile)), [])
        self.assertFalse(self.profile["deployment_authorization"])
        self.assertTrue(self.profile["not_for_production"])
        self.assertTrue(all(gate["status"] != "PASS" for gate in self.profile["acceptance_gates"]))

    def test_unknown_hardware_reference_fails(self) -> None:
        profile = copy.deepcopy(self.profile)
        profile["components"][0]["catalog_refs"] = ["invented-hardware-id"]
        self.assertTrue(any("catalog_refs unknown" in error for error in self.errors(profile)))

    def test_duplicate_vlan_id_fails(self) -> None:
        profile = copy.deepcopy(self.profile)
        profile["network"]["segments"][1]["vlan_id"] = profile["network"]["segments"][0]["vlan_id"]
        self.assertTrue(any("vlan_id duplicates" in error for error in self.errors(profile)))

    def test_permissive_inter_vlan_policy_fails_schema(self) -> None:
        profile = copy.deepcopy(self.profile)
        profile["network"]["inter_vlan_default_policy"] = "allow"
        self.assertTrue(any("inter_vlan_default_policy" in error for error in self.errors(profile)))

    def test_unapproved_cloud_management_fails_closed(self) -> None:
        profile = copy.deepcopy(self.profile)
        profile["network"]["management_plane"]["cloud_controller_allowed"] = True
        self.assertTrue(any("cloud control" in error for error in self.errors(profile)))

    def test_guest_without_internal_prefix_denial_fails(self) -> None:
        profile = copy.deepcopy(self.profile)
        guest_flow = next(flow for flow in profile["network"]["required_flow_contracts"] if flow["id"] == "guest-internet-only")
        guest_flow["limits"] = ["DHCP", "DNS", "established outbound traffic"]
        self.assertTrue(any("guest flow lacks explicit internal-prefix denial" in error for error in self.errors(profile)))

    def test_unreviewed_catalogue_gap_cannot_be_a_candidate(self) -> None:
        profile = copy.deepcopy(self.profile)
        component = next(item for item in profile["components"] if item["component_id"] == "identity-authenticators")
        component["selection_state"] = "candidate_not_approved"
        self.assertTrue(any("at least one H0_CANDIDATE" in error for error in self.errors(profile)))

    def test_acceptance_gate_cannot_claim_pass_without_evidence(self) -> None:
        profile = copy.deepcopy(self.profile)
        profile["acceptance_gates"][0]["status"] = "PASS"
        errors = self.errors(profile)
        self.assertTrue(errors)
        self.assertTrue(any("PASS" in error for error in errors))

    def test_price_subtotal_must_match_included_components(self) -> None:
        profile = copy.deepcopy(self.profile)
        profile["pricing_snapshot"]["partial_subtotal_min_zar"] += 1
        self.assertTrue(any("lower bounds" in error for error in self.errors(profile)))

    def test_profile_cannot_claim_deployment_authorization(self) -> None:
        profile = copy.deepcopy(self.profile)
        profile["deployment_authorization"] = True
        self.assertTrue(any("deployment_authorization" in error for error in self.errors(profile)))


if __name__ == "__main__":
    unittest.main()
