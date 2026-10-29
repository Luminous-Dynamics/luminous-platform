import unittest

from differential_qualification import compare_history, history_from_shapes
from linearizability_oracle import check_history


class DifferentialGateMutationTests(unittest.TestCase):
    def test_false_negative_mutation_is_detected(self):
        history = history_from_shapes((0, 0))
        baseline = compare_history(history)
        self.assertEqual(baseline["reason"], None)
        self.assertEqual(baseline["optimized_status"], "LINEARIZABLE")
        self.assertTrue(baseline["reference_has_linearization"])

        def false_negative(candidate, *, max_states):
            result = dict(check_history(candidate, max_states=max_states))
            result["status"] = "NON_LINEARIZABLE"
            result["linearization"] = None
            return result

        mutant = compare_history(history, optimized_checker=false_negative)
        self.assertEqual(mutant["reason"], "linearizability_decision_disagreement")

    def test_search_exhaustion_is_not_treated_as_negative(self):
        history = history_from_shapes((0, 0))

        def budget_mutant(candidate, *, max_states):
            result = dict(check_history(candidate, max_states=max_states))
            result["status"] = "UNKNOWN_LIMIT"
            result["linearization"] = None
            return result

        mutant = compare_history(history, optimized_checker=budget_mutant)
        self.assertEqual(mutant["reason"], "optimized_checker_unknown_limit")

    def test_false_positive_witness_mutation_is_detected(self):
        # The first operation strictly precedes the second; reverse order is illegal.
        history = history_from_shapes((0, 1))
        baseline = compare_history(history)
        self.assertEqual(baseline["reason"], None)
        self.assertEqual(baseline["optimized_status"], "LINEARIZABLE")

        def invalid_witness(candidate, *, max_states):
            result = dict(check_history(candidate, max_states=max_states))
            result["linearization"] = list(reversed(result["linearization"]))
            return result

        mutant = compare_history(history, optimized_checker=invalid_witness)
        self.assertEqual(mutant["reason"], "optimized_witness_rejected_by_reference")

    def test_unrecognized_checker_status_fails_closed(self):
        history = history_from_shapes((0, 0))

        def status_mutant(candidate, *, max_states):
            result = dict(check_history(candidate, max_states=max_states))
            result["status"] = "MAYBE"
            return result

        mutant = compare_history(history, optimized_checker=status_mutant)
        self.assertEqual(mutant["reason"], "optimized_checker_unrecognized_status")


if __name__ == "__main__":
    unittest.main()
