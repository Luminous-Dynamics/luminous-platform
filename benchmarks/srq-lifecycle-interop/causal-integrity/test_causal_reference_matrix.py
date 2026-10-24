import copy
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import causal_reference_matrix as matrix


class CausalReferenceMatrixTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.protocol = matrix.load_protocol()

    def test_frozen_edge_inventory_has_fourteen_relations(self):
        self.assertEqual(len(self.protocol["edges"]), 14)
        self.assertEqual(self.protocol["target_metrics"]["expected_state_count"], 143)

    def test_matrix_is_exhaustive_over_every_target_and_invalid_sentinel(self):
        report = matrix.run_matrix(self.protocol)
        self.assertEqual(report["status"], "PASS")
        self.assertEqual(report["observed_state_count"], 143)
        self.assertEqual(report["observed_edge_count"], 14)
        self.assertEqual(report["accepted_count"], 14)
        self.assertEqual(report["rejected_count"], 129)
        for key in ("false_accept_count", "false_reject_count", "unexpected_error_count", "false_release_pass_count"):
            self.assertEqual(report[key], 0, key)

    def test_each_edge_accepts_only_the_declared_predecessor(self):
        report = matrix.run_matrix(self.protocol)
        for case in report["cases"]:
            with self.subTest(case=case["case_id"]):
                if case["candidate_reference"] is not None and case["candidate_reference"] == "":
                    self.assertEqual(case["expected_outcome"], "REJECT")
                self.assertEqual(case["observed_outcome"], case["expected_outcome"], case["errors"])

    def test_wrong_reference_oracle_target_fails_closed(self):
        changed = copy.deepcopy(self.protocol)
        changed["edges"][0]["target_kind"] = "planned"
        with self.assertRaises(ValueError):
            matrix.validate_protocol(changed)

    def test_metrics_drift_fails_closed(self):
        changed = copy.deepcopy(self.protocol)
        changed["target_metrics"]["expected_state_count"] += 1
        with self.assertRaises(ValueError):
            matrix.validate_protocol(changed)


if __name__ == "__main__":
    unittest.main(verbosity=2)
