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
PLAN = "1" * 64
TARGET = "2" * 64
TRANSACTION = "3" * 64


def operation():
    return v.load(ROOT / "fixtures" / "operation.json")


def binding(**overrides):
    ids = {
        "nixward_plan_digest": PLAN,
        "nixward_target_machine_digest": TARGET,
        "nixward_transaction_id": TRANSACTION,
    }
    ids.update(overrides)
    return build_adapter_binding(operation(), now=NOW, **ids)


class TestNixwardAdapterBinding(unittest.TestCase):
    def test_binding_schema_and_digest_namespaces_are_explicit(self):
        record = binding()
        v.validate_schema("nixward_adapter_binding", record)
        self.assertTrue(record["omsi_manifest_digest"].startswith("sha256:"))
        self.assertTrue(record["nixward_plan_digest"].startswith("blake3:"))
        self.assertTrue(record["nixward_target_machine_digest"].startswith("blake3:"))
        self.assertTrue(record["nixward_transaction_id"].startswith("blake3:"))

    def test_exact_operation_and_nixward_identities_validate(self):
        validate_adapter_binding(
            binding(), operation(), nixward_plan_digest=PLAN,
            nixward_target_machine_digest=TARGET,
            nixward_transaction_id=TRANSACTION, now=NOW,
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
                record, changed, nixward_plan_digest=PLAN,
                nixward_target_machine_digest=TARGET,
                nixward_transaction_id=TRANSACTION, now=NOW,
            )

    def test_wrong_nixward_plan_rejected(self):
        with self.assertRaisesRegex(v.ConformanceError, "nixward_plan_digest"):
            validate_adapter_binding(
                binding(), operation(), nixward_plan_digest="4" * 64,
                nixward_target_machine_digest=TARGET,
                nixward_transaction_id=TRANSACTION, now=NOW,
            )

    def test_wrong_machine_binding_rejected(self):
        with self.assertRaisesRegex(v.ConformanceError, "nixward_target_machine_digest"):
            validate_adapter_binding(
                binding(), operation(), nixward_plan_digest=PLAN,
                nixward_target_machine_digest="4" * 64,
                nixward_transaction_id=TRANSACTION, now=NOW,
            )

    def test_wrong_transaction_identity_rejected(self):
        with self.assertRaisesRegex(v.ConformanceError, "nixward_transaction_id"):
            validate_adapter_binding(
                binding(), operation(), nixward_plan_digest=PLAN,
                nixward_target_machine_digest=TARGET,
                nixward_transaction_id="4" * 64, now=NOW,
            )

    def test_binding_tenant_substitution_rejected(self):
        record = binding()
        record["tenant_id"] = "tenant-b"
        with self.assertRaisesRegex(v.ConformanceError, "tenant_id"):
            validate_adapter_binding(
                record, operation(), nixward_plan_digest=PLAN,
                nixward_target_machine_digest=TARGET,
                nixward_transaction_id=TRANSACTION, now=NOW,
            )

    def test_binding_correlation_substitution_rejected(self):
        record = binding()
        record["correlation_id"] = "other-flow"
        with self.assertRaisesRegex(v.ConformanceError, "correlation_id"):
            validate_adapter_binding(
                record, operation(), nixward_plan_digest=PLAN,
                nixward_target_machine_digest=TARGET,
                nixward_transaction_id=TRANSACTION, now=NOW,
            )

    def test_malformed_nixward_digest_rejected(self):
        for bad in ("A" * 64, "g" * 64, "1" * 63, "blake3:" + "1" * 64):
            with self.subTest(value=bad), self.assertRaises(v.ConformanceError):
                binding(nixward_plan_digest=bad)

    def test_digest_algorithm_namespace_substitution_rejected(self):
        record = binding()
        record["omsi_manifest_digest"] = "blake3:" + "1" * 64
        with self.assertRaisesRegex(v.ConformanceError, "schema error"):
            validate_adapter_binding(
                record, operation(), nixward_plan_digest=PLAN,
                nixward_target_machine_digest=TARGET,
                nixward_transaction_id=TRANSACTION, now=NOW,
            )

    def test_additional_unbound_fields_rejected(self):
        record = binding()
        record["authorized"] = True
        with self.assertRaisesRegex(v.ConformanceError, "schema error"):
            validate_adapter_binding(
                record, operation(), nixward_plan_digest=PLAN,
                nixward_target_machine_digest=TARGET,
                nixward_transaction_id=TRANSACTION, now=NOW,
            )

    def test_operation_argument_tampering_rejected_before_binding(self):
        changed = copy.deepcopy(operation())
        changed["action"]["arguments"]["mode"] = "different-mode"
        with self.assertRaisesRegex(v.ConformanceError, "argument digest mismatch"):
            build_adapter_binding(
                changed, nixward_plan_digest=PLAN,
                nixward_target_machine_digest=TARGET,
                nixward_transaction_id=TRANSACTION, now=NOW,
            )


if __name__ == "__main__":
    unittest.main()
