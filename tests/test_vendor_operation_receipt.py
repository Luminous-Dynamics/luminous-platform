"""Adversarial tests for mutation authorization and durable effect receipts."""

from __future__ import annotations

import copy
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))

from validate_hardware_portfolio import load_json  # noqa: E402
from validate_vendor_operation_receipt import receipt_errors, registry_binding_errors  # noqa: E402


class VendorOperationReceiptTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.fixture = load_json(ROOT / "tests/fixtures/vendor-operation-effect-receipt-prepared-v1.json")
        cls.schema = load_json(ROOT / "schemas/vendor-operation-effect-receipt-v1.schema.json")
        cls.registry = load_json(ROOT / "security/vendor-adapter-capability-registry-v1.json")

    def errors(self, receipt):
        return receipt_errors(receipt, self.schema)

    def test_prepared_fixture_is_valid_but_not_authorized(self) -> None:
        self.assertEqual(self.errors(copy.deepcopy(self.fixture)), [])
        self.assertEqual(self.fixture["state"], "PREPARED")
        self.assertEqual(self.fixture["authorization"]["state"], "pending")
        self.assertEqual(self.fixture["effect_state"], "NO_EFFECT")

    def test_prepared_receipt_may_reference_planned_operation_without_authorizing_it(self) -> None:
        self.assertEqual(registry_binding_errors(copy.deepcopy(self.fixture), copy.deepcopy(self.registry)), [])

    def test_authorization_is_blocked_until_registry_operation_is_tested(self) -> None:
        receipt = self._authorization_verified_receipt()
        errors = registry_binding_errors(receipt, copy.deepcopy(self.registry))
        self.assertTrue(any("requires registry state=tested" in error for error in errors))

    def test_authorized_receipt_must_match_exact_adapter_model_revision_and_region(self) -> None:
        receipt = self._authorized_applied_receipt()
        registry = self._synthetic_test_registry()
        receipt["adapter_revision"] = "a" * 40
        self.assertEqual(registry_binding_errors(receipt, registry), [])

        drifted = copy.deepcopy(receipt)
        drifted["target"]["hardware_revision"] = "different-revision"
        errors = registry_binding_errors(drifted, registry)
        self.assertTrue(any("hardware_revision must exactly match" in error for error in errors))

    def test_authorized_receipt_must_bind_adapter_source_revision(self) -> None:
        receipt = self._authorized_applied_receipt()
        registry = self._synthetic_test_registry()
        receipt["adapter_revision"] = "f" * 40
        errors = registry_binding_errors(receipt, registry)
        self.assertTrue(any("adapter_revision must match" in error for error in errors))

    def test_configuration_mutation_requires_a3_or_higher_maturity(self) -> None:
        receipt = self._authorized_applied_receipt()
        registry = self._synthetic_test_registry()
        adapter = next(item for item in registry["adapters"] if item["adapter_id"] == receipt["adapter_id"])
        adapter["maturity"] = "A2_STATE_CONTRACT_TESTED"
        errors = registry_binding_errors(receipt, registry)
        self.assertTrue(any("requires at least A3_CHANGE_QUALIFIED" in error for error in errors))

    def _synthetic_test_registry(self):
        """Return an explicitly synthetic exact-scope registry for contract tests only."""
        registry = copy.deepcopy(self.registry)
        adapter = next(item for item in registry["adapters"] if item["adapter_id"] == self.fixture["adapter_id"])
        adapter.update({
            "maturity": "A3_CHANGE_QUALIFIED",
            "implementation_state": "LAB_TESTED",
            "model_scope_state": "exact_models_tested",
            "region_scope_state": "region_scoped",
            "maturity_evidence_refs": ["evidence:synthetic-adapter-contract-tests"],
        })
        operation = next(item for item in adapter["operations"] if item["operation"] == "configuration_apply")
        operation.update({
            "state": "tested",
            "evidence_refs": ["evidence:synthetic-physical-fixture-test"],
            "tested_revision": "a" * 40,
            "tested_scope": {
                "model": self.fixture["target"]["model"],
                "product_family": self.fixture["target"]["product_family"],
                "hardware_revision": self.fixture["target"]["hardware_revision"],
                "software_version": self.fixture["target"]["software_version"],
                "region": self.fixture["target"]["region"],
                "test_environment": "synthetic fixture only; not a physical device claim",
            },
            "security_controls": {
                "action_digest_bound_authorization": True,
                "fresh_prestate_required": True,
                "uncertain_outcome_policy": "NEVER_AUTOMATIC_REPLAY",
                "recovery_evidence_refs": ["evidence:synthetic-recovery-contract"],
            },
        })
        return registry

    def test_target_vendor_and_family_must_match_adapter_declaration(self) -> None:
        receipt = copy.deepcopy(self.fixture)
        receipt["target"]["vendor"] = "Palo Alto Networks"
        errors = registry_binding_errors(receipt, copy.deepcopy(self.registry))
        self.assertTrue(any("target.vendor must match" in error for error in errors))

        receipt = copy.deepcopy(self.fixture)
        receipt["target"]["product_family"] = "Meraki MX"
        errors = registry_binding_errors(receipt, copy.deepcopy(self.registry))
        self.assertTrue(any("product_family must be explicitly declared" in error for error in errors))

    def test_authorized_receipt_must_match_product_family(self) -> None:
        receipt = self._authorized_applied_receipt()
        registry = self._synthetic_test_registry()
        receipt["adapter_revision"] = "a" * 40
        receipt["target"]["product_family"] = "Meraki MX"
        errors = registry_binding_errors(receipt, registry)
        self.assertTrue(any("product_family must be explicitly declared" in error for error in errors))
        self.assertTrue(any("must exactly match registry tested_scope.product_family" in error for error in errors))

    def test_target_digest_must_bind_to_target_identity(self) -> None:
        receipt = copy.deepcopy(self.fixture)
        receipt["intent"]["target_digest"] = "f" * 64
        self.assertTrue(any("target_digest must match" in error for error in self.errors(receipt)))

    def test_verified_authorization_must_bind_action_target_and_prestate(self) -> None:
        receipt = self._authorization_verified_receipt()
        receipt["authorization"]["authorized_action_digest"] = "f" * 64
        errors = self.errors(receipt)
        self.assertTrue(any("exact intent.action_digest" in error for error in errors))

    def test_authorization_must_bind_policy_and_profile_revision(self) -> None:
        receipt = self._authorization_verified_receipt()
        receipt["authorization"]["authorized_policy_revision"] = "f" * 40
        receipt["authorization"]["authorized_profile_digest"] = "8" * 64
        errors = self.errors(receipt)
        self.assertTrue(any("exact intent.policy_revision" in error for error in errors))
        self.assertTrue(any("exact intent.profile_digest" in error for error in errors))

    def test_verified_authorization_must_have_a_journal_event(self) -> None:
        receipt = copy.deepcopy(self.fixture)
        receipt["authorization"].update({
            "state": "verified",
            "verified_at": "2026-10-10T19:21:00+02:00",
            "authorized_action_digest": receipt["intent"]["action_digest"],
            "authorized_target_digest": receipt["intent"]["target_digest"],
            "authorized_prestate_digest": receipt["intent"]["prestate_digest"],
            "authorized_policy_revision": receipt["intent"]["policy_revision"],
            "authorized_profile_digest": receipt["intent"]["profile_digest"],
            "signature_evidence_refs": ["evidence:signed-approval"],
        })
        errors = self.errors(receipt)
        self.assertTrue(any("must have an AUTHORIZATION_VERIFIED journal event" in error for error in errors))

    def test_apply_started_requires_verified_approval_and_prestate_refresh(self) -> None:
        receipt = copy.deepcopy(self.fixture)
        receipt["state"] = "APPLY_STARTED"
        receipt["effect_state"] = "EFFECT_POSSIBLE"
        receipt["journal"].append({
            "state": "APPLY_STARTED",
            "occurred_at": "2026-10-10T19:22:00+02:00",
            "actor_ref": "principal:fixture-executor",
            "evidence_refs": ["evidence:fixture-before-apply"],
            "prestate_digest": receipt["intent"]["prestate_digest"],
        })
        errors = self.errors(receipt)
        self.assertTrue(any("verified before APPLY_STARTED" in error for error in errors))
        self.assertTrue(any("authorization must be verified" in error for error in errors))
        self.assertTrue(any("recovery_evidence_refs are required before any mutation starts" in error for error in errors))

    def test_expired_authorization_cannot_start_mutation(self) -> None:
        receipt = self._authorized_applied_receipt()
        receipt["authorization"]["expires_at"] = "2026-10-10T19:21:30+02:00"
        errors = self.errors(receipt)
        self.assertTrue(any("approval was expired at APPLY_STARTED" in error for error in errors))

    def test_prestate_drift_at_apply_start_is_rejected(self) -> None:
        receipt = self._authorized_applied_receipt()
        apply_event = next(item for item in receipt["journal"] if item["state"] == "APPLY_STARTED")
        apply_event["prestate_digest"] = "f" * 64
        errors = self.errors(receipt)
        self.assertTrue(any("must revalidate the authorized pre-state" in error for error in errors))

    def test_verified_result_must_match_desired_state_digest(self) -> None:
        receipt = self._authorized_applied_receipt()
        receipt["state"] = "VERIFIED"
        receipt["effect_state"] = "EFFECT_VERIFIED"
        receipt["journal"].append({
            "state": "VERIFIED",
            "occurred_at": "2026-10-10T19:24:00+02:00",
            "actor_ref": "principal:fixture-verifier",
            "evidence_refs": ["evidence:verified-observation"],
            "prestate_digest": None,
        })
        receipt["evidence"] = {
            "raw_artifact_refs": ["evidence:device-state"],
            "independent_verifier_ref": "principal:independent-reviewer",
            "verified_state_digest": "f" * 64,
        }
        errors = self.errors(receipt)
        self.assertTrue(any("must match intent.desired_state_digest" in error for error in errors))

    def test_indeterminate_effect_cannot_be_replayed_in_same_receipt(self) -> None:
        receipt = self._authorized_applied_receipt()
        receipt["journal"][3]["state"] = "INDETERMINATE"
        receipt["journal"][3]["occurred_at"] = "2026-10-10T19:23:00+02:00"
        receipt["journal"][3]["prestate_digest"] = None
        receipt["journal"].append({
            "state": "APPLY_STARTED",
            "occurred_at": "2026-10-10T19:24:00+02:00",
            "actor_ref": "principal:fixture-executor",
            "evidence_refs": ["evidence:replay-attempt"],
            "prestate_digest": receipt["intent"]["prestate_digest"],
        })
        receipt["state"] = "APPLY_STARTED"
        receipt["effect_state"] = "EFFECT_POSSIBLE"
        errors = self.errors(receipt)
        self.assertTrue(any("invalid transition INDETERMINATE -> APPLY_STARTED" in error for error in errors))
        self.assertTrue(any("cannot repeat APPLY_STARTED" in error for error in errors))

    def test_complete_independently_verified_receipt_passes(self) -> None:
        receipt = self._authorized_applied_receipt()
        receipt["state"] = "VERIFIED"
        receipt["effect_state"] = "EFFECT_VERIFIED"
        receipt["journal"].append({
            "state": "VERIFIED",
            "occurred_at": "2026-10-10T19:24:00+02:00",
            "actor_ref": "principal:fixture-verifier",
            "evidence_refs": ["evidence:verified-observation"],
            "prestate_digest": None,
        })
        receipt["evidence"] = {
            "raw_artifact_refs": ["evidence:packet-capture", "evidence:device-state"],
            "independent_verifier_ref": "principal:independent-reviewer",
            "verified_state_digest": receipt["intent"]["desired_state_digest"],
        }
        self.assertEqual(self.errors(receipt), [])

    def test_verified_receipt_requires_independent_evidence(self) -> None:
        receipt = self._authorized_applied_receipt()
        receipt["state"] = "VERIFIED"
        receipt["effect_state"] = "EFFECT_VERIFIED"
        receipt["journal"].append({
            "state": "VERIFIED",
            "occurred_at": "2026-10-10T19:24:00+02:00",
            "actor_ref": "principal:fixture-verifier",
            "evidence_refs": ["evidence:verified-observation"],
            "prestate_digest": None,
        })
        receipt["evidence"] = {
            "raw_artifact_refs": ["evidence:verified-observation"],
            "independent_verifier_ref": "principal:fixture-verifier",
            "verified_state_digest": receipt["intent"]["desired_state_digest"],
        }
        errors = self.errors(receipt)
        self.assertTrue(any("must be independent" in error for error in errors))

    def test_privacy_contract_rejects_secret_or_raw_config_publication(self) -> None:
        receipt = copy.deepcopy(self.fixture)
        receipt["privacy"]["raw_configuration_in_shared_dht"] = True
        self.assertTrue(self.errors(receipt))

    def test_alternative_os_flash_requires_known_good_image_before_effect(self) -> None:
        receipt = self._authorized_applied_receipt()
        receipt["operation"] = "alternative_os_flash"
        receipt["recovery"]["known_good_image_ref"] = None
        self.assertTrue(any("known_good_image_ref is required" in error for error in self.errors(receipt)))

    def _authorization_verified_receipt(self):
        receipt = copy.deepcopy(self.fixture)
        receipt["authorization"].update({
            "state": "verified",
            "verified_at": "2026-10-10T19:21:00+02:00",
            "authorized_action_digest": receipt["intent"]["action_digest"],
            "authorized_target_digest": receipt["intent"]["target_digest"],
            "authorized_prestate_digest": receipt["intent"]["prestate_digest"],
            "authorized_policy_revision": receipt["intent"]["policy_revision"],
            "authorized_profile_digest": receipt["intent"]["profile_digest"],
            "signature_evidence_refs": ["evidence:signed-approval"],
        })
        receipt["state"] = "AUTHORIZATION_VERIFIED"
        receipt["effect_state"] = "NO_EFFECT"
        receipt["journal"].append({
            "state": "AUTHORIZATION_VERIFIED",
            "occurred_at": "2026-10-10T19:21:00+02:00",
            "actor_ref": "principal:fixture-authorizer",
            "evidence_refs": ["evidence:signed-approval"],
            "prestate_digest": None,
        })
        return receipt

    def _authorized_applied_receipt(self):
        receipt = self._authorization_verified_receipt()
        receipt["recovery"]["recovery_evidence_refs"] = ["evidence:physical-or-config-recovery-drill"]
        receipt["state"] = "APPLIED_UNVERIFIED"
        receipt["effect_state"] = "EFFECT_APPLIED"
        receipt["journal"].extend([
            {
                "state": "APPLY_STARTED",
                "occurred_at": "2026-10-10T19:22:00+02:00",
                "actor_ref": "principal:fixture-executor",
                "evidence_refs": ["evidence:prestate-recheck"],
                "prestate_digest": receipt["intent"]["prestate_digest"],
            },
            {
                "state": "APPLIED_UNVERIFIED",
                "occurred_at": "2026-10-10T19:23:00+02:00",
                "actor_ref": "principal:fixture-executor",
                "evidence_refs": ["evidence:apply-result"],
                "prestate_digest": None,
            },
        ])
        return receipt


if __name__ == "__main__":
    unittest.main()
