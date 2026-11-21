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
            "cases": 23, "positive_control": 9, "negative_control": 9, "indeterminate_control": 5
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

    def test_new_key_same_semantic_operation_is_still_a_duplicate_effect(self):
        suite, _ = inputs()
        current = case(suite, "TELR-002")
        current["attempts"].append({
            "attempt_id": "a2", "operation_id": "op-002-new", "idempotency_key": "new-key",
            "argument_digest": current["operation"]["argument_digest"], "dispatch_state": "sent",
            "ack_state": "received", "retry_parent_attempt_id": None,
            "reconciliation_before_attempt": "not_applicable", "restored_from_checkpoint": False,
            "authority_generation": 1, "grant_id": "grant:write", "nonce_id": "nonce-new"
        })
        current["target_ledger"]["entries"].append({
            "commit_id": "c2", "operation_id": "op-002-new",
            "semantic_operation_id": current["operation"]["semantic_operation_id"],
            "idempotency_key": "new-key", "argument_digest": current["operation"]["argument_digest"],
            "effect_id": current["operation"]["effect_id"], "kind": "effect",
            "batch_item_id": None, "compensates_for": None
        })
        current["client_ledger"]["known_commit_ids"].append("c2")
        current["reconciliation"]["result_commit_ids"].append("c2")
        outcome = checker.evaluate(current)
        self.assertEqual(outcome["duplicate_effect_count"], 1)
        self.assertEqual(outcome["disposition"], "violation")

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

    def test_revocation_proven_before_commit_blocks_effect(self):
        suite, _ = inputs()
        outcome = checker.evaluate(case(suite, "TELR-021"))
        self.assertEqual(outcome["authorization_linearization_state"], "revocation_precedes_commit")
        self.assertEqual(outcome["reconciled_state"], "not_committed")
        self.assertEqual(outcome["disposition"], "qualified_not_committed")

    def test_target_effect_after_proven_revocation_is_violation(self):
        suite, _ = inputs()
        current = case(suite, "TELR-021")
        current["target_ledger"]["entries"].append({
            "commit_id": "late-commit", "operation_id": "op-TELR-021",
            "semantic_operation_id": "intent-TELR-021", "idempotency_key": "key-TELR-021",
            "argument_digest": current["operation"]["argument_digest"],
            "effect_id": current["operation"]["effect_id"], "kind": "effect",
            "batch_item_id": None, "compensates_for": None,
            "grant_id": "grant:write", "nonce_id": "nonce-TELR-021"
        })
        current["client_ledger"]["known_commit_ids"] = ["late-commit"]
        current["reconciliation"]["status"] = "committed"
        current["reconciliation"]["result_commit_ids"] = ["late-commit"]
        outcome = checker.evaluate(current)
        self.assertEqual(outcome["authorization_linearization_violation_count"], 1)
        self.assertEqual(outcome["disposition"], "violation")

    def test_commit_proven_before_revocation_is_valid_at_linearization_point(self):
        suite, _ = inputs()
        outcome = checker.evaluate(case(suite, "TELR-023"))
        self.assertEqual(outcome["authorization_linearization_state"], "commit_precedes_revocation")
        self.assertEqual(outcome["disposition"], "qualified_committed")

    def test_overlapping_revocation_commit_intervals_remain_indeterminate(self):
        suite, _ = inputs()
        outcome = checker.evaluate(case(suite, "TELR-022"))
        self.assertEqual(outcome["authorization_linearization_state"], "revocation_commit_overlap")
        self.assertEqual(outcome["disposition"], "indeterminate")

    def test_unverified_timebase_does_not_prove_authorization_order(self):
        suite, _ = inputs()
        current = case(suite, "TELR-023")
        current["authorization_timing"]["commit_interval"]["timebase_verified"] = False
        self.assertEqual(checker.evaluate(current)["authorization_linearization_state"], "timebase_unqualified")

    def test_clock_domain_mismatch_keeps_authorization_order_unqualified(self):
        suite, _ = inputs()
        current = case(suite, "TELR-023")
        current["authorization_timing"]["revocation_interval"]["clock_domain"] = "independent-clock"
        outcome = checker.evaluate(current)
        self.assertEqual(outcome["authorization_linearization_state"], "timebase_unqualified")
        self.assertEqual(outcome["disposition"], "indeterminate")

    def test_reused_event_identity_is_rejected(self):
        suite, schema = inputs()
        current = case(suite, "TELR-023")
        current["authorization_timing"]["commit_interval"]["event_id"] = current["authorization_timing"]["grant_interval"]["event_id"]
        reject(suite, schema)

    def test_negative_uncertainty_interval_is_rejected(self):
        suite, schema = inputs()
        current = case(suite, "TELR-023")
        current["authorization_timing"]["commit_interval"]["lower_ns"] = -1
        reject(suite, schema)

    def test_touching_uncertainty_intervals_do_not_prove_strict_order(self):
        suite, _ = inputs()
        current = case(suite, "TELR-023")
        current["authorization_timing"]["grant_interval"]["upper_ns"] = current["authorization_timing"]["commit_interval"]["lower_ns"]
        self.assertEqual(checker.evaluate(current)["authorization_linearization_state"], "grant_order_unproven")

    def test_partial_batch_cannot_be_claimed_complete(self):
        suite, schema = inputs()
        case(suite, "TELR-011")["client_ledger"]["completed_claim"] = True
        reject(suite, schema)

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

    def test_compensation_requires_distinct_authority_and_operation(self):
        suite, schema = inputs()
        current = case(suite, "TELR-014")
        compensation = next(entry for entry in current["target_ledger"]["entries"] if entry["kind"] == "compensation")
        original = next(entry for entry in current["target_ledger"]["entries"] if entry["kind"] == "effect")
        compensation["grant_id"] = original["grant_id"]
        compensation_attempt = next(a for a in current["attempts"] if a["operation_id"] == compensation["operation_id"])
        compensation_attempt["grant_id"] = original["grant_id"]
        reject(suite, schema)

    def test_target_commit_must_bind_to_dispatch_authority_receipt(self):
        suite, schema = inputs()
        current = case(suite, "TELR-002")
        current["target_ledger"]["entries"][0]["grant_id"] = "grant:other"
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
