"""Regression suite for the deployment profile declaration validator."""
from __future__ import annotations

import copy
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))

from validate_deployment_profile import (  # noqa: E402
    DEFAULT_SCHEMA_PATH,
    _load_schema,
    parse_profile_yaml,
    profile_digest,
    validate_profile_data,
    validate_profile_text,
)

SCHEMA = _load_schema(DEFAULT_SCHEMA_PATH)
FIXTURE = ROOT / "profiles" / "examples" / "local-nixos-pilot.yaml"
BASE_PROFILE = parse_profile_yaml(FIXTURE.read_text(encoding="utf-8"))


class DeploymentProfileValidatorTests(unittest.TestCase):
    def validate(self, profile=None):
        return validate_profile_data(
            copy.deepcopy(BASE_PROFILE if profile is None else profile),
            SCHEMA,
            source="unit-test",
        )

    def test_example_is_valid_declaration_not_qualified(self):
        report = self.validate()
        self.assertEqual(report["result"], "VALID_DECLARATION")
        self.assertFalse(report["qualification_evaluated"])
        self.assertTrue(report["profile_digest_sha256"].startswith("sha256:"))
        self.assertIn("does not run an adapter", report["notice"])

    def test_unknown_schema_version_is_invalid(self):
        profile = copy.deepcopy(BASE_PROFILE)
        profile["schema"] = "luminous.deployment-profile/v99"
        report = self.validate(profile)
        self.assertEqual(report["result"], "INVALID")
        self.assertTrue(any("const" in err["message"] or "was expected" in err["message"] for err in report["errors"]))

    def test_unknown_field_is_invalid(self):
        profile = copy.deepcopy(BASE_PROFILE)
        profile["undocumented_authority_override"] = True
        report = self.validate(profile)
        self.assertEqual(report["result"], "INVALID")
        self.assertTrue(any("Additional properties" in err["message"] for err in report["errors"]))

    def test_duplicate_yaml_key_is_rejected(self):
        text = "schema: luminous.deployment-profile/v1\nschema: luminous.deployment-profile/v1\n"
        report = validate_profile_text(text, SCHEMA)
        self.assertEqual(report["result"], "INVALID")
        self.assertIn("duplicate key", report["errors"][0]["message"])

    def test_yaml_anchors_and_aliases_are_rejected(self):
        text = "schema: luminous.deployment-profile/v1\ndefaults: &defaults {}\ncopy: *defaults\n"
        report = validate_profile_text(text, SCHEMA)
        self.assertEqual(report["result"], "INVALID")
        self.assertIn("anchors and aliases", report["errors"][0]["message"])

    def test_explicit_yaml_tags_are_rejected(self):
        text = "schema: !custom luminous.deployment-profile/v1\n"
        report = validate_profile_text(text, SCHEMA)
        self.assertEqual(report["result"], "INVALID")
        self.assertIn("explicit YAML tags", report["errors"][0]["message"])

    def test_remote_control_requires_explicit_network_exposure(self):
        profile = copy.deepcopy(BASE_PROFILE)
        profile["services"]["remote_control"]["enabled"] = True
        profile["services"]["remote_control"]["authentication_authority"] = "customer-local-auth"
        # No network_exposure is supplied: remote control must not inherit an implicit scope.
        report = self.validate(profile)
        self.assertEqual(report["result"], "INVALID")
        self.assertTrue(any("explicit active network_exposure" in err["message"] for err in report["errors"]))

    def test_yaml_11_yes_is_not_coerced_to_boolean(self):
        text = FIXTURE.read_text(encoding="utf-8").replace(
            "privileged_execution: false", "privileged_execution: yes"
        )
        report = validate_profile_text(text, SCHEMA)
        self.assertEqual(report["result"], "INVALID")
        self.assertTrue(any("boolean" in err["message"] for err in report["errors"]))

    def test_privileged_execution_without_authority_is_invalid(self):
        profile = copy.deepcopy(BASE_PROFILE)
        profile["authority"]["privileged_execution"] = True
        report = self.validate(profile)
        self.assertEqual(report["result"], "INVALID")
        self.assertTrue(any("authorization_authority" in err["message"] for err in report["errors"]))

    def test_remote_control_without_authentication_authority_is_invalid(self):
        profile = copy.deepcopy(BASE_PROFILE)
        profile["services"]["remote_control"]["enabled"] = True
        profile["services"]["remote_control"]["network_exposure"] = "private_network"
        report = self.validate(profile)
        self.assertEqual(report["result"], "INVALID")
        self.assertTrue(any("authentication_authority" in err["message"] for err in report["errors"]))

    def test_qualified_status_without_runtime_evidence_is_blocked(self):
        profile = copy.deepcopy(BASE_PROFILE)
        profile["metadata"]["status"] = "qualified"
        report = self.validate(profile)
        self.assertEqual(report["result"], "QUALIFICATION_BLOCKED")
        self.assertFalse(report["qualification_evaluated"])
        self.assertTrue(any("evidence_refs" in item for item in report["blockers"]))

    def test_m1_claim_cannot_pass_by_adding_reference_strings(self):
        profile = copy.deepcopy(BASE_PROFILE)
        profile["qualification"]["claimed_level"] = "M1"
        profile["qualification"]["evidence_refs"] = ["evidence:example"]
        profile["qualification"]["limitations"] = ["Example reference is not verified."]
        profile["target"]["tested_platform_revisions"] = ["deadbeef"]
        report = self.validate(profile)
        self.assertEqual(report["result"], "QUALIFICATION_BLOCKED")
        self.assertFalse(report["qualification_evaluated"])
        self.assertTrue(any("not evaluated" in item for item in report["blockers"]))

    def test_profile_digest_is_independent_of_mapping_key_order(self):
        profile = copy.deepcopy(BASE_PROFILE)
        reversed_top_level = dict(reversed(list(profile.items())))
        self.assertEqual(profile_digest(profile), profile_digest(reversed_top_level))

    def test_profile_digest_changes_when_semantics_change(self):
        profile = copy.deepcopy(BASE_PROFILE)
        original = profile_digest(profile)
        profile["metadata"]["description"] += " Updated."
        self.assertNotEqual(original, profile_digest(profile))


if __name__ == "__main__":
    unittest.main()
