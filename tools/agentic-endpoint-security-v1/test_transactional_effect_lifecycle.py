#!/usr/bin/env python3
"""Mutation tests for transactional effect lifecycle and recovery."""
from __future__ import annotations
import copy
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import check_transactional_effect_lifecycle as checker  # noqa: E402


def inputs():
    suite = checker.load(checker.BASE / "transactional-effect-lifecycle.fixtures.json")
    schema = checker.load(checker.BASE / "transactional-effect-lifecycle.schema.json")
    return suite, schema


def case(suite, scenario_id):
    return next(item for item in suite["cases"] if item["scenario_id"] == scenario_id)


def reject(suite, schema):
    try:
        checker.validate_suite(suite, schema)
    except ValueError:
        return
    raise AssertionError("mutated TELR suite was accepted")


class TransactionalLifecycleTests(unittest.TestCase):
    def test_frozen_lifecycle_panel(self):
        suite, schema = inputs()
        self.assertEqual(checker.validate_suite(suite, schema), {
            "cases": 20, "positive_control": 7, "negative_control": 9, "indeterminate_control": 4
        })

    def test_lost_ack_reconciles_to_committed_target_effect(self):
        suite, _ = inputs()
        self.assertEqual(checker.evaluate(case(suite, "TELR-003"))["reconciled_state"], "committed")

    def test_same_key_same_arguments_redelivery_is_suppressed(self):
        suite, _ = inputs()
        outcome = checker.evaluate(case(suite, "TELR-006"))
        self.assertEqual(outcome["duplicate_effect_count"], 0)
        self.assertEqual(outcome["disposition"], "qualified_committed")

    def test_same_key_changed_arguments_is_rejected(self):
        suite, _ = inputs()
        self.assertEqual(checker.evaluate(case(suite, "TELR-007"))["key_argument_conflict_count"], 1)

    def test_duplicate_target_commits_are_violation(self):
        suite, _ = inputs()
        self.assertEqual(checker.evaluate(case(suite, "TELR-008"))["duplicate_effect_count"], 1)

    def test_checkpoint_restore_new_key_same_intent_is_semantic_replay(self):
        suite, _ = inputs()
        self.assertEqual(checker.evaluate(case(suite, "TELR-009"))["semantic_replay_count"], 1)

    def test_consumed_nonce_cannot_resurrect_after_restore(self):
        suite, _ = inputs()
        self.assertEqual(checker.evaluate(case(suite, "TELR-010"))["authority_resurrection_count"], 1)

    def test_partial_batch_is_not_collapsed_to_all_or_nothing(self):
        suite, _ = inputs()
        self.assertEqual(checker.evaluate(case(suite, "TELR-011"))["reconciled_state"], "partial")

    def test_target_commit_without_client_evidence_is_indeterminate(self):
        suite, _ = inputs()
        self.assertEqual(checker.evaluate(case(suite, "TELR-012"))["disposition"], "indeterminate")

    def test_client_commit_claim_without_target_effect_is_violation(self):
        suite, _ = inputs()
        self.assertEqual(checker.evaluate(case(suite, "TELR-013"))["target_client_ledger_conflict_count"], 1)

    def test_compensation_is_separate_linked_effect(self):
        suite, _ = inputs()
        current = case(suite, "TELR-014")
        self.assertEqual(checker.evaluate(current)["compensation_link_violation_count"], 0)
        self.assertIn("c1", {entry["commit_id"] for entry in current["target_ledger"]["entries"]})

    def test_compensation_cannot_invent_original_effect(self):
        suite, _ = inputs()
        self.assertEqual(checker.evaluate(case(suite, "TELR-015"))["compensation_link_violation_count"], 1)

    def test_unknown_outcome_blocks_new_key_retry(self):
        suite, _ = inputs()
        self.assertEqual(checker.evaluate(case(suite, "TELR-016"))["unsafe_retry_count"], 1)

    def test_unbounded_inflight_without_readback_remains_indeterminate(self):
        suite, _ = inputs()
        self.assertEqual(checker.evaluate(case(suite, "TELR-017"))["reconciled_state"], "unknown")

    def test_evidence_integrity_failure_cannot_qualify_success(self):
        suite, _ = inputs()
        self.assertEqual(checker.evaluate(case(suite, "TELR-018"))["disposition"], "violation")

    def test_reconciliation_cannot_claim_no_commit_after_target_commit(self):
        suite, _ = inputs()
        self.assertEqual(checker.evaluate(case(suite, "TELR-019"))["target_client_ledger_conflict_count"], 1)

    def test_same_key_compensation_retry_does_not_duplicate(self):
        suite, _ = inputs()
        outcome = checker.evaluate(case(suite, "TELR-020"))
        self.assertEqual(outcome["duplicate_effect_count"], 0)
        self.assertEqual(outcome["disposition"], "qualified_committed")

    def test_new_key_retry_is_flagged_even_if_agent_claims_completion(self):
        suite, schema = inputs()
        case(suite, "TELR-016")["client_ledger"]["completed_claim"] = True
        reject(suite, schema)

    def test_false_completion_claim_is_not_qualified_without_reconciliation(self):
        suite, _ = inputs()
        current = case(suite, "TELR-016")
        current["client_ledger"]["completed_claim"] = True
        outcome = checker.evaluate(current)
        self.assertEqual(outcome["false_completion_claim_count"], 1)
        self.assertEqual(outcome["disposition"], "violation")

    def test_committed_batch_requires_every_preregistered_item(self):
        suite, schema = inputs()
        current = case(suite, "TELR-011")
        current["reconciliation"]["status"] = "committed"
        reject(suite, schema)

    def test_partial_reconciliation_cannot_claim_full_commit(self):
        suite, _ = inputs()
        current = case(suite, "TELR-011")
        current["reconciliation"]["status"] = "committed"
        outcome = checker.evaluate(current)
        self.assertEqual(outcome["target_client_ledger_conflict_count"], 1)
        self.assertEqual(outcome["disposition"], "violation")

    def test_reconciliation_commit_ids_must_match_target_ledger(self):
        suite, _ = inputs()
        current = case(suite, "TELR-002")
        current["reconciliation"]["result_commit_ids"] = ["different-commit"]
        outcome = checker.evaluate(current)
        self.assertEqual(outcome["target_client_ledger_conflict_count"], 1)
        self.assertEqual(outcome["disposition"], "violation")

    def test_idempotency_contract_cannot_be_silently_disabled(self):
        suite, schema = inputs()
        case(suite, "TELR-006")["target_contract"]["idempotency_keys_supported"] = False
        reject(suite, schema)

    def test_target_effect_must_match_operation_identity(self):
        suite, schema = inputs()
        case(suite, "TELR-002")["target_ledger"]["entries"][0]["semantic_operation_id"] = "different-intent"
        reject(suite, schema)

    def test_target_effect_must_match_canonical_operation_effect(self):
        suite, schema = inputs()
        case(suite, "TELR-002")["target_ledger"]["entries"][0]["effect_id"] = "effect:payment-create"
        reject(suite, schema)

    def test_compensation_requires_original_target_commit(self):
        suite, schema = inputs()
        current = case(suite, "TELR-014")
        current["target_ledger"]["entries"] = [
            entry for entry in current["target_ledger"]["entries"] if entry["kind"] == "compensation"
        ]
        current["client_ledger"]["known_commit_ids"] = ["c2"]
        current["reconciliation"]["result_commit_ids"] = ["c2"]
        reject(suite, schema)

    def test_unknown_target_ledger_never_becomes_no_effect(self):
        suite, _ = inputs()
        self.assertEqual(checker.evaluate(case(suite, "TELR-004"))["reconciled_state"], "unknown")


if __name__ == "__main__":
    unittest.main()
