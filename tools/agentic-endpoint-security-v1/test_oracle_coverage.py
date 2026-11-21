import copy
import json
import unittest
from pathlib import Path

from check_oracle_coverage import validate


ROOT = Path(__file__).resolve().parents[2]
MANIFEST = ROOT / "benchmarks" / "agentic-endpoint-security-v1" / "oracle-coverage.json"


class OracleCoverageContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.document = json.loads(MANIFEST.read_text(encoding="utf-8"))

    def test_checked_in_contract_satisfies_schema(self):
        result = validate(self.document)
        self.assertEqual(result["assurance_tiers"], 4)
        self.assertEqual(result["observer_roles"], 5)
        self.assertEqual(result["side_effect_channels"], 10)
        self.assertEqual(result["oracle_fault_controls"], 10)

    def test_external_network_cannot_be_enabled(self):
        mutated = copy.deepcopy(self.document)
        mutated["constraints"]["external_network_access"] = True
        with self.assertRaises(ValueError):
            validate(mutated)

    def test_agent_log_cannot_be_trusted_as_effect_truth(self):
        mutated = copy.deepcopy(self.document)
        mutated["observer_roles"][0]["required_properties"]["trusted_as_effect_truth_source"] = True
        with self.assertRaises(ValueError):
            validate(mutated)

    def test_supervisor_cannot_self_certify_effect_truth(self):
        mutated = copy.deepcopy(self.document)
        mutated["observer_roles"][1]["required_properties"]["trusted_as_effect_truth_source"] = True
        with self.assertRaises(ValueError):
            validate(mutated)

    def test_missing_observer_channel_is_rejected(self):
        mutated = copy.deepcopy(self.document)
        mutated["side_effect_coverage"].pop()
        with self.assertRaises(ValueError):
            validate(mutated)

    def test_fault_control_cannot_be_weakened(self):
        mutated = copy.deepcopy(self.document)
        mutated["oracle_fault_controls"][2]["expected_control"] = "alert_only"
        with self.assertRaises(ValueError):
            validate(mutated)

    def test_oracle_mutation_control_is_mandatory(self):
        mutated = copy.deepcopy(self.document)
        mutated["mandatory_controls"]["oracle_mutation_must_be_detected"] = False
        with self.assertRaises(ValueError):
            validate(mutated)

    def test_t3_cannot_drop_external_lifecycle_witness(self):
        mutated = copy.deepcopy(self.document)
        mutated["assurance_tiers"][3]["external_lifecycle_witness_required"] = False
        with self.assertRaises(ValueError):
            validate(mutated)

    def test_t1_cannot_claim_host_kernel_integrity(self):
        mutated = copy.deepcopy(self.document)
        mutated["assurance_tiers"][1]["host_kernel_in_trusted_computing_base"] = False
        with self.assertRaises(ValueError):
            validate(mutated)

    def test_t3_claim_ceiling_cannot_be_weakened(self):
        mutated = copy.deepcopy(self.document)
        mutated["claim_ceiling"] = "all tiers prove rollback resistance"
        with self.assertRaises(ValueError):
            validate(mutated)


if __name__ == "__main__":
    unittest.main()
