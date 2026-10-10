"""Adversarial tests for mutation authorization and durable effect receipts."""

from __future__ import annotations

import copy
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))

from validate_hardware_portfolio import load_json  # noqa: E402
from validate_vendor_operation_receipt import receipt_errors  # noqa: E402


class VendorOperationReceiptTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.fixture = load_json(ROOT / "tests/fixtures/vendor-operation-effect-receipt-prepared-v1.json")
        cls.schema = load_json(ROOT / "schemas/vendor-operation-effect-receipt-v1.schema.json")

    def errors(self, receipt):
        return receipt_errors(receipt, self.schema)

    def test_prepared_fixture_is_valid_but_not_authorized(self) -> None:
        self.assertEqual(self.errors(copy.deepcopy(self.fixture)), [])
        self.assertEqual(self.fixture["state"], "PREPARED")
        self.assertEqual(self.fixture["authorization"]["state"], "pending")
        self.assertEqual(self.fixture["effect_state"], "NO_EFFECT")

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
