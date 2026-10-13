import copy
import sys
import unittest
from datetime import datetime
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

import omsi_validate as v
from nixward_adapter import build_adapter_binding, validate_adapter_binding

NOW = datetime.fromisoformat("2026-10-12T12:05:00+00:00")


def operation():
    return v.load(ROOT / "fixtures" / "operation.json")


def nixward_transaction(**overrides):
    record = {
        "schema": "luminous-nixward-system-transaction-v1",
        "version": 1,
        "transaction_id": "3" * 64,
        "target_machine_digest": "2" * 64,
        "plan_digest": "1" * 64,
        "phase": "planned",
        "authorization": None,
        "validation": None,
        "snapshot": None,
        "application": None,
        "verification": None,
        "outcome": None,
    }
    record.update(overrides)
    return record


def binding(transaction=None):
    return build_adapter_binding(
        operation(),
        nixward_transaction=transaction if transaction is not None else nixward_transaction(),
        now=NOW,
    )


def lifecycle_transaction(phase, recovered_from="failed"):
    tx = nixward_transaction(phase=phase)
    if phase == "planned":
        return tx

    tx["validation"] = {"evidence_digest": "a" * 64, "summary": "fixture validation"}
    if phase == "validated":
        return tx

    tx["authorization"] = {
        "issuer": "test-owner",
        "evidence_digest": "b" * 64,
        "evidence_kind": "GeneralChange",
        "expires_at_ms": 1791807300000,
        "signer_key_id": None,
        "challenge_blake3": None,
        "replay_key": None,
    }
    if phase == "authorized":
        return tx

    tx["snapshot"] = {
        "pre_state_digest": "c" * 64,
        "recovery_binding_digest": "d" * 64,
    }
    if phase == "snapshotted":
        return tx

    exit_status = 1 if phase == "failed" or (phase == "recovered" and recovered_from == "failed") else 0
    tx["application"] = {
        "command_digest": "e" * 64,
        "started_at_ms": 1000,
        "finished_at_ms": 1100,
        "exit_status": exit_status,
    }
    if phase == "applied":
        return tx
    if phase == "failed":
        tx["outcome"] = {"phase": "failed", "reason": "fixture command failure"}
        return tx

    needs_verification = phase in {"verified", "promoted"} or (
        phase == "recovered" and recovered_from == "verified"
    )
    if needs_verification:
        tx["verification"] = {
            "post_state_digest": "f" * 64,
            "evidence_digest": "7" * 64,
            "boot_healthy": True,
        }
    if phase == "verified":
        return tx
    if phase == "promoted":
        tx["outcome"] = {"phase": "promoted", "reason": None}
        return tx
    if phase == "recovered":
        tx["outcome"] = {"phase": "recovered", "reason": "fixture recovery"}
        return tx
    raise ValueError(f"unsupported fixture phase: {phase}")


class TestNixwardAdapterBinding(unittest.TestCase):
    def test_binding_schema_and_digest_namespaces_are_explicit(self):
        record = binding()
        v.validate_schema("nixward_adapter_binding", record)
        self.assertTrue(record["omsi_manifest_digest"].startswith("sha256:"))
        self.assertTrue(record["nixward_plan_digest"].startswith("blake3:"))
        self.assertTrue(record["nixward_target_machine_digest"].startswith("blake3:"))
        self.assertTrue(record["nixward_transaction_id"].startswith("blake3:"))

    def test_exact_operation_and_transaction_envelope_validate(self):
        validate_adapter_binding(
            binding(), operation(),
            nixward_transaction=nixward_transaction(), now=NOW,
        )

    def test_manifest_mutation_invalidates_binding(self):
        record = binding()
        changed = operation()
        changed["action"]["arguments"]["mode"] = "different-mode"
        changed["action"]["arguments_digest"] = v.digest(
            changed["action"]["arguments"], v.ACTION_ARGUMENTS_DOMAIN
        )
        with self.assertRaisesRegex(v.ConformanceError, "omsi_manifest_digest"):
            validate_adapter_binding(
                record, changed, nixward_transaction=nixward_transaction(), now=NOW,
            )

    def test_wrong_nixward_plan_rejected(self):
        changed = nixward_transaction(plan_digest="4" * 64)
        with self.assertRaisesRegex(v.ConformanceError, "nixward_plan_digest"):
            validate_adapter_binding(
                binding(), operation(), nixward_transaction=changed, now=NOW,
            )

    def test_wrong_machine_binding_rejected(self):
        changed = nixward_transaction(target_machine_digest="4" * 64)
        with self.assertRaisesRegex(v.ConformanceError, "nixward_target_machine_digest"):
            validate_adapter_binding(
                binding(), operation(), nixward_transaction=changed, now=NOW,
            )

    def test_wrong_transaction_identity_rejected(self):
        changed = nixward_transaction(transaction_id="4" * 64)
        with self.assertRaisesRegex(v.ConformanceError, "nixward_transaction_id"):
            validate_adapter_binding(
                binding(), operation(), nixward_transaction=changed, now=NOW,
            )

    def test_binding_tenant_substitution_rejected(self):
        record = binding()
        record["tenant_id"] = "tenant-b"
        with self.assertRaisesRegex(v.ConformanceError, "tenant_id"):
            validate_adapter_binding(
                record, operation(), nixward_transaction=nixward_transaction(), now=NOW,
            )

    def test_binding_correlation_substitution_rejected(self):
        record = binding()
        record["correlation_id"] = "other-flow"
        with self.assertRaisesRegex(v.ConformanceError, "correlation_id"):
            validate_adapter_binding(
                record, operation(), nixward_transaction=nixward_transaction(), now=NOW,
            )

    def test_malformed_nixward_digest_rejected(self):
        for bad in ("A" * 64, "g" * 64, "1" * 63, "blake3:" + "1" * 64):
            with self.subTest(value=bad), self.assertRaises(v.ConformanceError):
                binding(nixward_transaction(plan_digest=bad))

    def test_unsupported_nixward_transaction_schema_rejected(self):
        changed = nixward_transaction(schema="other-transaction-schema")
        with self.assertRaisesRegex(v.ConformanceError, "unsupported Nixward transaction schema"):
            binding(changed)

    def test_unsupported_nixward_transaction_version_rejected(self):
        changed = nixward_transaction(version=2)
        with self.assertRaisesRegex(v.ConformanceError, "unsupported Nixward transaction version"):
            binding(changed)
    def test_missing_nixward_transaction_phase_rejected(self):
        changed = nixward_transaction()
        del changed["phase"]
        with self.assertRaisesRegex(v.ConformanceError, "unsupported or missing Nixward transaction phase"):
            binding(changed)

    def test_unknown_nixward_transaction_phase_rejected(self):
        changed = nixward_transaction(phase="indeterminate")
        with self.assertRaisesRegex(v.ConformanceError, "unsupported or missing Nixward transaction phase"):
            binding(changed)

    def test_digest_algorithm_namespace_substitution_rejected(self):
        record = binding()
        record["omsi_manifest_digest"] = "blake3:" + "1" * 64
        with self.assertRaisesRegex(v.ConformanceError, "schema error"):
            validate_adapter_binding(
                record, operation(), nixward_transaction=nixward_transaction(), now=NOW,
            )

    def test_additional_unbound_fields_rejected(self):
        record = binding()
        record["authorized"] = True
        with self.assertRaisesRegex(v.ConformanceError, "schema error"):
            validate_adapter_binding(
                record, operation(), nixward_transaction=nixward_transaction(), now=NOW,
            )

    def test_operation_argument_tampering_rejected_before_binding(self):
        changed = copy.deepcopy(operation())
        changed["action"]["arguments"]["mode"] = "different-mode"
        with self.assertRaisesRegex(v.ConformanceError, "argument digest mismatch"):
            build_adapter_binding(
                changed, nixward_transaction=nixward_transaction(), now=NOW,
            )


    def test_all_coherent_nixward_lifecycle_phases_bind(self):
        cases = [
            lifecycle_transaction("planned"),
            lifecycle_transaction("validated"),
            lifecycle_transaction("authorized"),
            lifecycle_transaction("snapshotted"),
            lifecycle_transaction("applied"),
            lifecycle_transaction("verified"),
            lifecycle_transaction("promoted"),
            lifecycle_transaction("failed"),
            lifecycle_transaction("recovered", recovered_from="failed"),
            lifecycle_transaction("recovered", recovered_from="verified"),
        ]
        for tx in cases:
            with self.subTest(phase=tx["phase"], application=tx["application"]):
                v.validate_schema("nixward_transaction", tx)
                record = binding(tx)
                validate_adapter_binding(
                    record, operation(), nixward_transaction=tx, now=NOW,
                )

    def test_planned_phase_rejects_future_validation_receipt(self):
        tx = lifecycle_transaction("planned")
        tx["validation"] = {"evidence_digest": "a" * 64, "summary": "unexpected"}
        with self.assertRaisesRegex(v.ConformanceError, "must not contain validation evidence"):
            binding(tx)

    def test_validated_phase_requires_validation_receipt(self):
        tx = lifecycle_transaction("validated")
        tx["validation"] = None
        with self.assertRaisesRegex(v.ConformanceError, "requires validation evidence"):
            binding(tx)

    def test_authorized_phase_requires_authorization_receipt(self):
        tx = lifecycle_transaction("authorized")
        tx["authorization"] = None
        with self.assertRaisesRegex(v.ConformanceError, "requires authorization evidence"):
            binding(tx)

    def test_snapshotted_phase_requires_snapshot_receipt(self):
        tx = lifecycle_transaction("snapshotted")
        tx["snapshot"] = None
        with self.assertRaisesRegex(v.ConformanceError, "requires snapshot evidence"):
            binding(tx)

    def test_applied_phase_requires_application_receipt(self):
        tx = lifecycle_transaction("applied")
        tx["application"] = None
        with self.assertRaisesRegex(v.ConformanceError, "requires application evidence"):
            binding(tx)

    def test_applied_phase_rejects_nonzero_exit_status(self):
        tx = lifecycle_transaction("applied")
        tx["application"]["exit_status"] = 1
        with self.assertRaisesRegex(v.ConformanceError, "applied phase requires exit status zero"):
            binding(tx)

    def test_verified_phase_requires_verification_receipt(self):
        tx = lifecycle_transaction("verified")
        tx["verification"] = None
        with self.assertRaisesRegex(v.ConformanceError, "requires verification evidence"):
            binding(tx)

    def test_verified_phase_rejects_nonzero_application(self):
        tx = lifecycle_transaction("verified")
        tx["application"]["exit_status"] = 1
        with self.assertRaisesRegex(v.ConformanceError, "verified phase requires exit status zero"):
            binding(tx)

    def test_promoted_phase_requires_promoted_outcome(self):
        tx = lifecycle_transaction("promoted")
        tx["outcome"]["phase"] = "recovered"
        with self.assertRaisesRegex(v.ConformanceError, "promoted outcome is inconsistent"):
            binding(tx)

    def test_promoted_phase_rejects_unhealthy_boot(self):
        tx = lifecycle_transaction("promoted")
        tx["verification"]["boot_healthy"] = False
        with self.assertRaisesRegex(v.ConformanceError, "unhealthy boot cannot be promoted"):
            binding(tx)

    def test_failed_phase_requires_nonzero_exit_status(self):
        tx = lifecycle_transaction("failed")
        tx["application"]["exit_status"] = 0
        with self.assertRaisesRegex(v.ConformanceError, "failed phase requires nonzero exit status"):
            binding(tx)

    def test_failed_phase_requires_failed_outcome(self):
        tx = lifecycle_transaction("failed")
        tx["outcome"] = None
        with self.assertRaisesRegex(v.ConformanceError, "requires outcome evidence"):
            binding(tx)

    def test_failed_phase_rejects_mismatched_outcome_phase(self):
        tx = lifecycle_transaction("failed")
        tx["outcome"]["phase"] = "promoted"
        with self.assertRaisesRegex(v.ConformanceError, "failed outcome is inconsistent"):
            binding(tx)

    def test_recovered_successful_apply_requires_verification(self):
        tx = lifecycle_transaction("recovered", recovered_from="verified")
        tx["verification"] = None
        with self.assertRaisesRegex(v.ConformanceError, "requires verification evidence"):
            binding(tx)

    def test_recovered_failed_apply_rejects_success_verification(self):
        tx = lifecycle_transaction("recovered", recovered_from="failed")
        tx["verification"] = {
            "post_state_digest": "f" * 64,
            "evidence_digest": "7" * 64,
            "boot_healthy": True,
        }
        with self.assertRaisesRegex(v.ConformanceError, "must not contain verification evidence"):
            binding(tx)

    def test_application_timestamps_must_be_ordered(self):
        tx = lifecycle_transaction("applied")
        tx["application"]["started_at_ms"] = 1200
        tx["application"]["finished_at_ms"] = 1100
        with self.assertRaisesRegex(v.ConformanceError, "timestamp interval is invalid"):
            binding(tx)

    def test_transaction_envelope_rejects_unexpected_fields(self):
        tx = lifecycle_transaction("planned")
        tx["administrator_approved"] = True
        with self.assertRaisesRegex(v.ConformanceError, "schema error"):
            binding(tx)


    def test_zero_validation_evidence_digest_rejected(self):
        tx = lifecycle_transaction("validated")
        tx["validation"]["evidence_digest"] = "0" * 64
        with self.assertRaisesRegex(v.ConformanceError, "validation evidence digest must be non-zero"):
            binding(tx)

    def test_zero_verification_digest_rejected(self):
        tx = lifecycle_transaction("verified")
        tx["verification"]["post_state_digest"] = "0" * 64
        with self.assertRaisesRegex(v.ConformanceError, "verification post-state digest must be non-zero"):
            binding(tx)

    def test_zero_nixward_identity_digests_rejected(self):
        for field in ("transaction_id", "target_machine_digest", "plan_digest"):
            with self.subTest(field=field):
                tx = nixward_transaction(**{field: "0" * 64})
                with self.assertRaisesRegex(
                    v.ConformanceError, f"Nixward transaction {field} digest must be non-zero"
                ):
                    binding(tx)

    def test_zero_authority_challenge_and_replay_digests_rejected(self):
        for field, label in (("challenge_blake3", "challenge"), ("replay_key", "replay key")):
            with self.subTest(field=field):
                tx = lifecycle_transaction("authorized")
                tx["authorization"][field] = "0" * 64
                with self.assertRaisesRegex(
                    v.ConformanceError, f"Nixward authorization {label} digest must be non-zero"
                ):
                    binding(tx)


    def test_unknown_nixward_approval_evidence_kind_rejected(self):
        tx = lifecycle_transaction("authorized")
        tx["authorization"]["evidence_kind"] = "explicit_approval"
        with self.assertRaisesRegex(v.ConformanceError, "schema error"):
            binding(tx)


    def test_authorization_issuer_uses_utf8_byte_limit(self):
        tx = lifecycle_transaction("authorized")
        tx["authorization"]["issuer"] = "é" * 129
        with self.assertRaisesRegex(v.ConformanceError, "issuer has invalid byte length"):
            binding(tx)

    def test_empty_signer_key_id_rejected(self):
        tx = lifecycle_transaction("authorized")
        tx["authorization"]["signer_key_id"] = ""
        with self.assertRaisesRegex(v.ConformanceError, "signer key ID has invalid byte length"):
            binding(tx)

    def test_validation_summary_uses_utf8_byte_limit(self):
        tx = lifecycle_transaction("validated")
        tx["validation"]["summary"] = "é" * 2049
        with self.assertRaisesRegex(v.ConformanceError, "validation summary exceeds byte limit"):
            binding(tx)

    def test_outcome_reason_uses_utf8_byte_limit(self):
        tx = lifecycle_transaction("recovered")
        tx["outcome"]["reason"] = "é" * 2049
        with self.assertRaisesRegex(v.ConformanceError, "outcome reason exceeds byte limit"):
            binding(tx)


if __name__ == "__main__":
    unittest.main()
