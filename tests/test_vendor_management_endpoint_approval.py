"""Adversarial tests for external endpoint approval/profile binding.

These tests do not verify real signatures or authorize connections.
"""

from __future__ import annotations

import copy
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))

from validate_hardware_portfolio import load_json  # noqa: E402
from validate_vendor_management_endpoint_approval import (  # noqa: E402
    approval_errors,
    canonical_profile_digest,
)


class VendorManagementEndpointApprovalTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.approval = load_json(ROOT / "tests/fixtures/vendor-management-endpoint-approval-v1.json")
        cls.approval_schema = load_json(ROOT / "schemas/vendor-management-endpoint-approval-v1.schema.json")
        cls.profile = load_json(ROOT / "tests/fixtures/vendor-management-endpoint-profile-v1.json")
        cls.profile_schema = load_json(ROOT / "schemas/vendor-management-endpoint-profile-v1.schema.json")

    def errors(self, approval=None, profile=None):
        return approval_errors(
            copy.deepcopy(approval if approval is not None else self.approval),
            self.approval_schema,
            copy.deepcopy(profile if profile is not None else self.profile),
            self.profile_schema,
        )

    def approved_record_and_profile(self):
        approval = copy.deepcopy(self.approval)
        profile = copy.deepcopy(self.profile)
        for name, value, evidence in (
            ("deployment_jurisdiction", "test-lab", "evidence:jurisdiction-assessment"),
            ("hardware_or_commercial_region", "test-lab", "evidence:hardware-region-assessment"),
        ):
            profile["scope"][name]["state"] = "assessed"
            profile["scope"][name]["value"] = value
            profile["scope"][name]["evidence_refs"] = [evidence]
        approval["state"] = "approved"
        approval["profile_digest"] = canonical_profile_digest(profile)
        approval["approver_ref"] = "principal:independent-approver"
        approval["allowed_operations"] = ["inventory_read"]
        approval["signature_scheme_ref"] = "signature-scheme:fixture-v1"
        approval["signing_key_ref"] = "keyref:fixture-approver-key"
        approval["signed_payload_digest"] = "c" * 64
        approval["signature_evidence_ref"] = "evidence:synthetic-signature"
        return approval, profile

    def test_pending_fixture_is_bound_to_the_frozen_profile(self) -> None:
        self.assertEqual(self.errors(), [])

    def test_profile_digest_is_deterministic_across_object_key_order(self) -> None:
        profile = copy.deepcopy(self.profile)
        reverse_top = dict(reversed(list(profile.items())))
        self.assertEqual(canonical_profile_digest(profile), canonical_profile_digest(reverse_top))

    def test_profile_digest_accepts_safe_integer_boundaries(self) -> None:
        for value in (9007199254740991, -9007199254740991):
            with self.subTest(value=value):
                profile = copy.deepcopy(self.profile)
                profile["profile_revision"] = value
                self.assertEqual(len(canonical_profile_digest(profile)), 64)

    def test_profile_digest_rejects_integer_outside_safe_interoperable_range(self) -> None:
        for value in (9007199254740992, -9007199254740992):
            with self.subTest(value=value):
                profile = copy.deepcopy(self.profile)
                profile["profile_revision"] = value
                with self.assertRaisesRegex(ValueError, "safe interoperable JSON integer range"):
                    canonical_profile_digest(profile)

    def test_unsupported_digest_scheme_is_rejected(self) -> None:
        approval = copy.deepcopy(self.approval)
        approval["profile_digest_scheme"] = "unspecified"
        self.assertTrue(any("profile_digest_scheme" in error for error in self.errors(approval=approval)))

    def test_endpoint_change_invalidates_prior_digest(self) -> None:
        profile = copy.deepcopy(self.profile)
        profile["endpoint"]["host"] = "other.example.invalid"
        errors = self.errors(profile=profile)
        self.assertTrue(any("profile_digest must match" in error for error in errors))

    def test_profile_revision_mismatch_is_rejected(self) -> None:
        approval = copy.deepcopy(self.approval)
        approval["profile_revision"] = 2
        self.assertTrue(any("profile_revision must match" in error for error in self.errors(approval=approval)))

    def test_adapter_revision_mismatch_is_rejected(self) -> None:
        approval = copy.deepcopy(self.approval)
        approval["adapter"]["adapter_revision"] = "d" * 40
        self.assertTrue(any("adapter.adapter_revision must exactly match" in error for error in self.errors(approval=approval)))

    def test_target_model_mismatch_is_rejected(self) -> None:
        approval = copy.deepcopy(self.approval)
        approval["target"]["model"] = "different-model"
        self.assertTrue(any("target.model must exactly match" in error for error in self.errors(approval=approval)))

    def test_target_hardware_revision_mismatch_is_rejected(self) -> None:
        approval = copy.deepcopy(self.approval)
        approval["target"]["hardware_revision"] = "rev-999"
        self.assertTrue(any("target.hardware_revision must exactly match" in error for error in self.errors(approval=approval)))

    def test_target_software_version_mismatch_is_rejected(self) -> None:
        approval = copy.deepcopy(self.approval)
        approval["target"]["software_version"] = "fw-999"
        self.assertTrue(any("target.software_version must exactly match" in error for error in self.errors(approval=approval)))

    def test_principal_scope_mismatch_is_rejected(self) -> None:
        approval = copy.deepcopy(self.approval)
        approval["expected_principal_ref"] = "principal:different"
        self.assertTrue(any("expected_principal_ref must match" in error for error in self.errors(approval=approval)))

    def test_tenant_or_org_scope_mismatch_is_rejected(self) -> None:
        approval = copy.deepcopy(self.approval)
        approval["expected_tenant_or_org_ref"] = "scope:different-tenant"
        self.assertTrue(any("expected_tenant_or_org_ref must match" in error for error in self.errors(approval=approval)))

    def test_mutating_operation_cannot_be_approved(self) -> None:
        approval = copy.deepcopy(self.approval)
        approval["allowed_operations"] = ["configuration_apply"]
        self.assertTrue(any("allowed_operations" in error for error in self.errors(approval=approval)))

    def test_approved_record_requires_signature_metadata(self) -> None:
        approval = copy.deepcopy(self.approval)
        approval["state"] = "approved"
        self.assertTrue(self.errors(approval=approval))

    def test_approved_record_requires_distinct_requester_and_approver(self) -> None:
        approval, profile = self.approved_record_and_profile()
        approval["approver_ref"] = approval["requested_by_ref"]
        self.assertTrue(any("distinct principals" in error for error in self.errors(approval, profile)))

    def test_approved_record_rejects_unresolved_jurisdiction(self) -> None:
        approval = copy.deepcopy(self.approval)
        approval["state"] = "approved"
        approval["approver_ref"] = "principal:independent-approver"
        approval["allowed_operations"] = ["inventory_read"]
        approval["signature_scheme_ref"] = "signature-scheme:fixture-v1"
        approval["signing_key_ref"] = "keyref:fixture-approver-key"
        approval["signed_payload_digest"] = "c" * 64
        approval["signature_evidence_ref"] = "evidence:synthetic-signature"
        self.assertTrue(any("deployment_jurisdiction remains unresolved" in error for error in self.errors(approval)))

    def test_approved_record_accepts_fully_scoped_synthetic_record_shape(self) -> None:
        approval, profile = self.approved_record_and_profile()
        self.assertEqual(self.errors(approval, profile), [])

    def test_expiry_must_be_later_than_not_before(self) -> None:
        approval = copy.deepcopy(self.approval)
        approval["expires_at"] = "2026-10-11T09:00:00+02:00"
        approval["not_before"] = "2026-10-11T10:00:00+02:00"
        self.assertTrue(any("expires_at must be later than $.not_before" in error for error in self.errors(approval=approval)))

    def test_timestamp_requires_explicit_timezone(self) -> None:
        approval = copy.deepcopy(self.approval)
        approval["created_at"] = "2026-10-11T10:00:00"
        self.assertTrue(self.errors(approval=approval))

    def test_rejected_decision_requires_reason_code(self) -> None:
        approval = copy.deepcopy(self.approval)
        approval["state"] = "rejected"
        self.assertTrue(self.errors(approval=approval))

    def test_rejected_decision_requires_reason_code(self) -> None:
        approval = copy.deepcopy(self.approval)
        approval.update({
            "state": "rejected",
            "approver_ref": "principal:independent-approver",
            "signature_scheme_ref": "signature-scheme:fixture-v1",
            "signing_key_ref": "keyref:fixture-approver-key",
            "signed_payload_digest": "c" * 64,
            "signature_evidence_ref": "evidence:synthetic-signature",
        })
        self.assertTrue(any("reason_code" in error for error in self.errors(approval=approval)))

    def test_signed_rejection_also_requires_distinct_requester_and_approver(self) -> None:
        approval = copy.deepcopy(self.approval)
        approval.update({
            "state": "rejected",
            "approver_ref": approval["requested_by_ref"],
            "signature_scheme_ref": "signature-scheme:fixture-v1",
            "signing_key_ref": "keyref:fixture-approver-key",
            "signed_payload_digest": "c" * 64,
            "signature_evidence_ref": "evidence:synthetic-signature",
            "reason_code": "policy_denied",
        })
        self.assertTrue(any("distinct principals" in error for error in self.errors(approval=approval)))

    def test_schema_diagnostics_never_echo_invalid_sensitive_values(self) -> None:
        approval = copy.deepcopy(self.approval)
        marker = "SYNTHETIC-SECRET-MUST-NOT-APPEAR"
        approval["profile_digest"] = marker
        errors = self.errors(approval=approval)
        self.assertTrue(errors)
        self.assertNotIn(marker, "\n".join(errors))

    def test_signature_presence_is_not_claimed_to_be_signature_verification(self) -> None:
        source = (ROOT / "tools/validate_vendor_management_endpoint_approval.py").read_text()
        self.assertIn("does not authenticate a signer", source)
        self.assertIn("signature authentication, signer trust", source)


if __name__ == "__main__":
    unittest.main()
