#!/usr/bin/env python3
"""Mutation tests for resource-scoped progress-frontier settlement."""
from __future__ import annotations
import copy
import sys
import unittest
from pathlib import Path
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE))
import check_progress_frontier_transactions as checker  # noqa: E402


def inputs():
    suite=checker.load(checker.BASE/"progress-frontier-transactional-effects.fixtures.json")
    schema=checker.load(checker.BASE/"progress-frontier-transactional-effects.schema.json")
    return suite,schema


def case(suite,sid):
    return next(item for item in suite["cases"] if item["scenario_id"]==sid)


def reject(suite,schema):
    try:checker.validate_suite(suite,schema)
    except ValueError:return
    raise AssertionError("mutated PFTE suite was accepted")


class ProgressFrontierTests(unittest.TestCase):
    def test_frozen_frontier_panel(self):
        suite,schema=inputs()
        self.assertEqual(checker.validate_suite(suite,schema),{"cases":22,"positive_control":8,"negative_control":8,"indeterminate_control":6})

    def test_sealed_transaction_commits_after_frontier(self):
        suite,_=inputs()
        self.assertEqual(checker.evaluate(case(suite,"PFTE-001"))["disposition"],"qualified_commit")

    def test_unsealed_footprint_cannot_settle(self):
        suite,_=inputs()
        self.assertEqual(checker.evaluate(case(suite,"PFTE-002"))["disposition"],"indeterminate")

    def test_unqualified_frontier_is_unknown_not_ready(self):
        suite,_=inputs()
        self.assertEqual(checker.evaluate(case(suite,"PFTE-003"))["frontier_status"],"unknown")

    def test_frontier_behind_transaction_epoch_holds(self):
        suite,_=inputs()
        self.assertEqual(checker.evaluate(case(suite,"PFTE-004"))["frontier_status"],"behind")

    def test_frontier_cannot_advance_past_inflight_earlier_conflict(self):
        suite,_=inputs()
        outcome=checker.evaluate(case(suite,"PFTE-005"))
        self.assertEqual(outcome["frontier_false_advance_count"],1)
        self.assertEqual(outcome["disposition"],"violation")

    def test_unrelated_lagging_resource_does_not_globally_block(self):
        suite,_=inputs()
        self.assertEqual(checker.evaluate(case(suite,"PFTE-006"))["disposition"],"qualified_commit")

    def test_earlier_conflicting_write_blocks_later_epoch(self):
        suite,_=inputs()
        self.assertEqual(checker.evaluate(case(suite,"PFTE-007"))["disposition"],"violation")

    def test_disjoint_resource_transactions_can_settle(self):
        suite,_=inputs()
        self.assertEqual(checker.evaluate(case(suite,"PFTE-008"))["disposition"],"qualified_commit")

    def test_losing_branch_suppresses_bufferable_effect(self):
        suite,_=inputs()
        self.assertEqual(checker.evaluate(case(suite,"PFTE-009"))["disposition"],"qualified_abort")

    def test_losing_branch_released_effect_is_violation(self):
        suite,_=inputs()
        self.assertEqual(checker.evaluate(case(suite,"PFTE-010"))["bufferable_abort_leak_count"],1)

    def test_externalized_reversible_effects_compensate_in_reverse_order(self):
        suite,_=inputs()
        outcome=checker.evaluate(case(suite,"PFTE-011"))
        self.assertEqual(outcome["uncompensated_externalized_count"],0)
        self.assertEqual(outcome["compensation_order_violation_count"],0)
        self.assertEqual(outcome["disposition"],"qualified_abort")

    def test_missing_compensation_is_violation_with_complete_target_ledger(self):
        suite,_=inputs()
        self.assertEqual(checker.evaluate(case(suite,"PFTE-012"))["uncompensated_externalized_count"],1)

    def test_irreversible_effect_cannot_escape_pending_branch(self):
        suite,_=inputs()
        self.assertEqual(checker.evaluate(case(suite,"PFTE-013"))["early_irreversible_effect_count"],1)

    def test_winning_branch_can_release_buffered_effect_after_frontier(self):
        suite,_=inputs()
        self.assertEqual(checker.evaluate(case(suite,"PFTE-014"))["disposition"],"qualified_commit")

    def test_checkpoint_restore_cannot_replay_committed_semantic_intent(self):
        suite,_=inputs()
        self.assertEqual(checker.evaluate(case(suite,"PFTE-015"))["semantic_replay_count"],1)

    def test_sealed_footprint_cannot_be_widened(self):
        suite,_=inputs()
        self.assertEqual(checker.evaluate(case(suite,"PFTE-016"))["footprint_mutation_count"],1)

    def test_stale_resource_version_cannot_be_overwritten(self):
        suite,_=inputs()
        self.assertEqual(checker.evaluate(case(suite,"PFTE-017"))["stale_resource_version_count"],2)

    def test_partial_release_is_not_reported_as_full_commit(self):
        suite,_=inputs()
        self.assertEqual(checker.evaluate(case(suite,"PFTE-018"))["disposition"],"qualified_partial")

    def test_incomplete_compensation_evidence_remains_indeterminate(self):
        suite,_=inputs()
        outcome=checker.evaluate(case(suite,"PFTE-019"))
        self.assertEqual(outcome["disposition"],"indeterminate")

    def test_frontier_regression_is_detected(self):
        suite,_=inputs()
        self.assertEqual(checker.evaluate(case(suite,"PFTE-020"))["frontier_false_advance_count"],1)

    def test_multi_resource_transaction_requires_each_frontier(self):
        suite,_=inputs()
        self.assertEqual(checker.evaluate(case(suite,"PFTE-022"))["frontier_status"],"behind")

    def test_compensation_must_reference_same_transaction_externalized_effect(self):
        suite, schema = inputs()
        current = case(suite, "PFTE-011")
        compensation = next(item for item in current["target_events"] if item["event_kind"] == "compensation")
        compensation["compensates_for"] = "ev-006"
        reject(suite, schema)

    def test_one_externalized_effect_cannot_receive_duplicate_compensations(self):
        suite, _ = inputs()
        current = copy.deepcopy(case(suite, "PFTE-011"))
        compensation = next(item for item in current["target_events"] if item["event_id"] == "ev-11d")
        compensation["compensates_for"] = "ev-11b"
        outcome = checker.evaluate(current)
        self.assertGreater(outcome["compensation_order_violation_count"], 0)
        self.assertEqual(outcome["disposition"], "violation")

    def test_target_event_sequences_are_unique(self):
        suite, schema = inputs()
        current = case(suite, "PFTE-021")
        current["target_events"][1]["sequence"] = current["target_events"][0]["sequence"]
        reject(suite, schema)

    def test_duplicate_target_event_id_is_rejected(self):
        suite,schema=inputs()
        current=case(suite,"PFTE-001")
        current["target_events"].append(copy.deepcopy(current["target_events"][0]))
        reject(suite,schema)

    def test_missing_frontier_for_touched_resource_is_not_ready(self):
        suite,_=inputs()
        current=copy.deepcopy(case(suite,"PFTE-001"))
        current["transaction"]["frontiers"]=[]
        self.assertEqual(checker.evaluate(current)["frontier_status"],"unknown")

    def test_false_commit_claim_with_unqualified_frontier_is_violation(self):
        suite,_=inputs()
        current=copy.deepcopy(case(suite,"PFTE-001"))
        current["transaction"]["frontiers"][0]["witness_qualified"]=False
        current["expected"]=checker.evaluate(current)
        self.assertEqual(current["expected"]["disposition"],"violation")

if __name__=="__main__":unittest.main()
