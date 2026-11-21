#!/usr/bin/env python3
"""Mutation tests for resource-enforced frontier fencing and late-writer resistance."""
from __future__ import annotations
import copy
import sys
import unittest
from pathlib import Path
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import check_commit_frontier_fencing as checker  # noqa: E402


def inputs():
    suite = checker.load(checker.BASE / "commit-frontier-fencing.fixtures.json")
    schema = checker.load(checker.BASE / "commit-frontier-fencing.schema.json")
    return suite, schema


def case(suite, case_id):
    return next(item for item in suite["cases"] if item["case_id"] == case_id)


def reject(suite, schema):
    try:
        checker.validate_suite(suite, schema)
    except ValueError:
        return
    raise AssertionError("mutated CFF suite was accepted")


class CommitFrontierFencingTests(unittest.TestCase):
    def test_frozen_fence_panel(self):
        suite, schema = inputs()
        self.assertEqual(checker.validate_suite(suite, schema), {
            "cases": 26, "positive_control": 7, "negative_control": 16, "indeterminate_control": 3
        })

    def test_current_resource_generation_allows_write(self):
        suite, _ = inputs()
        self.assertEqual(checker.evaluate(case(suite, "CFF-001"))["disposition"], "qualified_allow")

    def test_stale_generation_is_denied(self):
        suite, _ = inputs()
        self.assertEqual(checker.evaluate(case(suite, "CFF-002"))["disposition"], "qualified_deny")

    def test_acceptance_of_stale_generation_is_violation(self):
        suite, _ = inputs()
        self.assertEqual(checker.evaluate(case(suite, "CFF-003"))["stale_token_acceptance_count"], 1)

    def test_late_epoch_operation_is_denied(self):
        suite, _ = inputs()
        self.assertEqual(checker.evaluate(case(suite, "CFF-004"))["disposition"], "qualified_deny")

    def test_acceptance_of_late_epoch_write_is_violation(self):
        suite, _ = inputs()
        self.assertEqual(checker.evaluate(case(suite, "CFF-005"))["late_epoch_acceptance_count"], 1)

    def test_takeover_between_check_and_write_is_rejected(self):
        suite, _ = inputs()
        outcome = checker.evaluate(case(suite, "CFF-007"))
        self.assertGreater(outcome["toctou_race_count"], 0)
        self.assertEqual(outcome["disposition"], "violation")

    def test_same_active_grant_can_perform_multiple_atomic_writes(self):
        suite, _ = inputs()
        self.assertEqual(checker.evaluate(case(suite, "CFF-008"))["disposition"], "qualified_allow")

    def test_token_cannot_cross_resource(self):
        suite, _ = inputs()
        self.assertEqual(checker.evaluate(case(suite, "CFF-010"))["disposition"], "qualified_deny")

    def test_token_is_bound_to_transaction_and_branch(self):
        suite, _ = inputs()
        self.assertEqual(checker.evaluate(case(suite, "CFF-011"))["disposition"], "violation")

    def test_old_policy_epoch_cannot_mutate(self):
        suite, _ = inputs()
        self.assertGreater(checker.evaluate(case(suite, "CFF-012"))["policy_epoch_acceptance_count"], 0)

    def test_highwater_rollback_is_detected(self):
        suite, _ = inputs()
        self.assertGreater(checker.evaluate(case(suite, "CFF-014"))["high_water_regression_count"], 0)

    def test_stale_replica_certificate_is_not_accepted(self):
        suite, _ = inputs()
        self.assertEqual(checker.evaluate(case(suite, "CFF-015"))["disposition"], "qualified_deny")

    def test_stale_resource_version_cannot_be_overwritten(self):
        suite, _ = inputs()
        self.assertGreater(checker.evaluate(case(suite, "CFF-017"))["stale_resource_version_acceptance_count"], 0)

    def test_missing_owner_or_highwater_evidence_is_indeterminate(self):
        suite, _ = inputs()
        self.assertEqual(checker.evaluate(case(suite, "CFF-018"))["disposition"], "indeterminate")

    def test_independent_resource_owners_have_independent_fences(self):
        suite, _ = inputs()
        self.assertEqual(checker.evaluate(case(suite, "CFF-019"))["disposition"], "qualified_allow")

    def test_successor_generation_can_commit_after_new_frontier(self):
        suite, _ = inputs()
        self.assertEqual(checker.evaluate(case(suite, "CFF-025"))["disposition"], "qualified_allow")

    def test_unexpected_rejection_is_a_false_reject(self):
        suite, _ = inputs()
        self.assertEqual(checker.evaluate(case(suite, "CFF-024"))["disposition"], "false_reject")

    def test_wrong_frontier_certificate_binding_is_rejected(self):
        suite, schema = inputs()
        case(suite, "CFF-001")["fence"]["frontier_certificate_id"] = "frontier:other"
        reject(suite, schema)

    def test_non_atomic_compare_and_write_cannot_qualify(self):
        suite, _ = inputs()
        self.assertGreater(checker.evaluate(case(suite, "CFF-007"))["atomic_fence_failure_count"], 0)

    def test_reconciliation_cannot_hide_an_observed_effect(self):
        suite, schema = inputs()
        case(suite, "CFF-001")["reconciliation"]["observed_event_ids"] = []
        reject(suite, schema)

    def test_duplicate_event_id_is_rejected(self):
        suite, schema = inputs()
        current = case(suite, "CFF-001")
        current["target_events"].append(copy.deepcopy(current["target_events"][0]))
        reject(suite, schema)

    def test_incomplete_frontier_witness_stays_indeterminate(self):
        suite, _ = inputs()
        self.assertEqual(checker.evaluate(case(suite, "CFF-023"))["disposition"], "indeterminate")

    def test_allowed_late_data_still_needs_target_fence(self):
        suite, _ = inputs()
        self.assertEqual(checker.evaluate(case(suite, "CFF-026"))["disposition"], "qualified_allow")

    def test_owner_high_water_regression_cannot_be_hidden_by_old_token(self):
        suite, _ = inputs()
        self.assertEqual(checker.evaluate(case(suite, "CFF-014"))["disposition"], "violation")

    def test_version_chain_must_match_each_atomic_write(self):
        suite, schema = inputs()
        current = case(suite, "CFF-008")
        current["target_events"][1]["resource_version_before"] = "v1"
        current["expected"] = checker.evaluate(current)
        self.assertGreater(current["expected"]["stale_resource_version_acceptance_count"], 0)
        reject(suite, schema)


if __name__ == "__main__":
    unittest.main()
