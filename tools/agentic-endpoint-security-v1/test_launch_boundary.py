import copy
import json
import unittest
from pathlib import Path

from check_launch_boundary import validate


ROOT = Path(__file__).resolve().parents[2]
MANIFEST = ROOT / "benchmarks" / "agentic-endpoint-security-v1" / "launch-boundary-contract.json"


class LaunchBoundaryContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.document = json.loads(MANIFEST.read_text(encoding="utf-8"))

    def test_checked_in_contract_satisfies_schema(self):
        counts = validate(self.document)
        self.assertEqual(counts["scenarios"], 5)
        self.assertEqual(counts["mutation_controls"], 5)
        self.assertEqual(counts["source_semantics"], 3)

    def test_external_network_cannot_be_enabled(self):
        mutated = copy.deepcopy(self.document)
        mutated["execution_constraints"]["external_network_access"] = True
        with self.assertRaises(ValueError):
            validate(mutated)

    def test_expected_exposure_is_not_misreported_as_containment(self):
        mutated = copy.deepcopy(self.document)
        mutated["scenarios"][0]["expected_outcome"] = "BLOCK_BEFORE_EFFECT"
        with self.assertRaises(ValueError):
            validate(mutated)

    def test_parent_oracle_assertion_is_mandatory(self):
        mutated = copy.deepcopy(self.document)
        mutated["scenarios"][0]["required_assertions"].remove(
            "parent_oracle_observes_exact_mutation"
        )
        with self.assertRaises(ValueError):
            validate(mutated)

    def test_close_on_exec_mitigation_cannot_be_removed(self):
        mutated = copy.deepcopy(self.document)
        mutated["scenarios"].pop(1)
        with self.assertRaises(ValueError):
            validate(mutated)

    def test_claim_ceiling_cannot_be_escalated(self):
        mutated = copy.deepcopy(self.document)
        mutated["claim_scope"]["full_descriptor_hygiene_coverage"] = True
        with self.assertRaises(ValueError):
            validate(mutated)

    def test_parent_original_descriptor_must_be_cloexec(self):
        mutated = copy.deepcopy(self.document)
        mutated["fixtures"]["original_parent_descriptor_must_be_cloexec"] = False
        with self.assertRaises(ValueError):
            validate(mutated)

    def test_inconclusive_infrastructure_is_not_containment(self):
        mutated = copy.deepcopy(self.document)
        mutated["scenarios"][4]["expected_outcome"] = "BLOCK_BEFORE_EFFECT"
        with self.assertRaises(ValueError):
            validate(mutated)

    def test_mutation_oracle_cannot_be_weakened(self):
        mutated = copy.deepcopy(self.document)
        mutated["mutation_controls"][0]["expected_control"] = "alert_only"
        with self.assertRaises(ValueError):
            validate(mutated)

    def test_marker_lengths_must_remain_equal(self):
        mutated = copy.deepcopy(self.document)
        mutated["fixtures"]["baseline_and_mutation_same_length"] = False
        with self.assertRaises(ValueError):
            validate(mutated)

    def test_receipt_must_bind_exact_checkout_sha(self):
        mutated = copy.deepcopy(self.document)
        mutated["receipt_contract"]["required_fields"].remove("checkout_sha")
        with self.assertRaises(ValueError):
            validate(mutated)

    def test_receipt_must_identify_kernel_and_landlock_abi(self):
        mutated = copy.deepcopy(self.document)
        mutated["receipt_contract"]["required_fields"].remove("kernel_release")
        with self.assertRaises(ValueError):
            validate(mutated)

    def test_receipt_cannot_claim_complete_descriptor_hygiene(self):
        mutated = copy.deepcopy(self.document)
        mutated["receipt_contract"]["expected_values"]["full_descriptor_hygiene_coverage"] = True
        with self.assertRaises(ValueError):
            validate(mutated)


if __name__ == "__main__":
    unittest.main()
