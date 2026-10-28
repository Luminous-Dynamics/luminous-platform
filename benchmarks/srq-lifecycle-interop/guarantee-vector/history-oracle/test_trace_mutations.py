import hashlib
import json
import unittest
from pathlib import Path

from trace_mutation_qualification import MUTATIONS, run_corpus


class TraceMutationAdequacyTests(unittest.TestCase):
    def test_positive_control_is_valid(self):
        report = run_corpus(Path(__file__).resolve().parent)
        self.assertEqual(report["baseline_positive_control"]["status"], "PASS")
        self.assertEqual(report["baseline_positive_control"]["violation_count"], 0)

    def test_every_declared_trace_mutation_is_detected_by_its_expected_code(self):
        report = run_corpus(Path(__file__).resolve().parent)
        self.assertEqual(report["status"], "PASS")
        self.assertEqual(report["mutation_count"], len(MUTATIONS))
        self.assertEqual(report["detected_expected_mutations"], len(MUTATIONS))
        self.assertEqual(report["survived_or_wrong_violation_count"], 0)
        self.assertEqual(report["invalid_mutation_count"], 0)
        history_hashes = []
        for result in report["mutations"]:
            with self.subTest(mutation=result["mutation_id"]):
                self.assertEqual(result["status"], "DETECTED")
                self.assertIn(result["expected_code"], result["observed_codes"])
                self.assertIn(result["expected_invariant"], result["observed_invariants"])
                self.assertIsInstance(result["mutated_history"], dict)
                canonical = json.dumps(
                    result["mutated_history"], sort_keys=True, separators=(",", ":"), ensure_ascii=False
                ).encode("utf-8")
                digest = hashlib.sha256(canonical).hexdigest()
                self.assertEqual(result["mutated_history_sha256"], digest)
                history_hashes.append(digest)
        self.assertEqual(len(set(history_hashes)), len(MUTATIONS))


if __name__ == "__main__":
    unittest.main()
