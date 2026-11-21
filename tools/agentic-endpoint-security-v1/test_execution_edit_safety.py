#!/usr/bin/env python3
"""Mutation tests for checkpoint/fork/restore/merge safety."""
from __future__ import annotations
import copy
import sys
import unittest
from pathlib import Path
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE))
import check_execution_edit_safety as checker

def inputs():
 s=checker.load(checker.BASE/"execution-edit-safety.fixtures.json")
 schema=checker.load(checker.BASE/"execution-edit-safety.schema.json")
 return s,schema

def case(suite,sid):
 return next(c for c in suite["cases"] if c["scenario_id"]==sid)

def reject(suite,schema):
 try:checker.validate_suite(suite,schema)
 except ValueError:return
 raise AssertionError("mutated EESPE suite was accepted")

class ExecutionEditTests(unittest.TestCase):
 def test_frozen_checkpoint_fork_restore_merge_panel(self):
  s,schema=inputs()
  self.assertEqual(checker.validate_suite(s,schema),{"cases":14,"positive_control":4,"negative_control":8,"indeterminate_control":2})
 def test_restore_cannot_replay_committed_semantic_operation(self):
  s,_=inputs();self.assertIn("external_operation_replay_or_unresolved_retry",checker.evaluate(case(s,"EESPE-003"))["exclusion_reasons"]["replay-new-uuid"])
 def test_consumed_authority_cannot_resurrect(self):
  s,_=inputs();self.assertIn("consumed_authority_resurrection",checker.evaluate(case(s,"EESPE-004"))["exclusion_reasons"]["reuse-consumed-authority"])
 def test_fork_cannot_repeat_prior_external_effect(self):
  s,_=inputs();self.assertEqual(checker.evaluate(case(s,"EESPE-005"))["disposition"],"no_safe_continuation_proved")
 def test_merge_cannot_commit_same_semantic_operation_twice(self):
  s,_=inputs();self.assertIn("external_operation_replay_or_unresolved_retry",checker.evaluate(case(s,"EESPE-006"))["exclusion_reasons"]["merge-two-branches-repeat"])
 def test_merge_must_preserve_every_required_result(self):
  s,_=inputs();self.assertIn("required_result_obligation_dropped",checker.evaluate(case(s,"EESPE-007"))["exclusion_reasons"]["drop-required-secondary"])
 def test_conflicting_branch_writes_are_excluded(self):
  s,_=inputs();self.assertIn("merge_write_conflict",checker.evaluate(case(s,"EESPE-008"))["exclusion_reasons"]["conflicting-branch-writes"])
 def test_stale_policy_and_evidence_cannot_be_carried_forward(self):
  s,_=inputs();self.assertEqual(checker.evaluate(case(s,"EESPE-009"))["disposition"],"no_safe_continuation_proved")
 def test_incomplete_authoritative_history_fails_closed(self):
  s,_=inputs();self.assertIn("authoritative_history_or_target_ledger_incomplete",checker.evaluate(case(s,"EESPE-010"))["exclusion_reasons"]["continue-from-untrusted-summary"])
 def test_independent_nonconflicting_merge_is_allowed(self):
  s,_=inputs();self.assertEqual(checker.evaluate(case(s,"EESPE-011"))["disposition"],"safe_edit_accepted")
 def test_unresolved_call_must_be_preserved_and_not_reissued(self):
  s,_=inputs()
  result=checker.evaluate(case(s,"EESPE-012"))
  self.assertEqual(result["safe_continuation_ids"],["preserve-pending-and-wait"])
 def test_no_safe_continuation_returns_complete_witness(self):
  s,_=inputs();self.assertEqual(checker.evaluate(case(s,"EESPE-013"))["disposition"],"no_safe_continuation_proved")
 def test_stale_evidence_has_no_safe_merge(self):
  s,_=inputs();self.assertEqual(checker.evaluate(case(s,"EESPE-014"))["expected_edit_decision"],"reject_no_safe_continuation")
 def test_edited_candidate_cannot_omit_unresolved_call(self):
  s,schema=inputs()
  c=case(s,"EESPE-002")["candidates"][0]
  c["results_pending"]=[]
  reject(s,schema)
 def test_finite_candidate_set_cannot_be_declared_non_exhaustive(self):
  s,schema=inputs();case(s,"EESPE-001")["candidate_set_exhaustive"]=False
  reject(s,schema)
 def test_edit_must_be_atomic_while_external_call_is_outstanding(self):
  s,schema=inputs();case(s,"EESPE-012")["candidates"][1]["atomic_edit_enforcement"]=False
  reject(s,schema)
 def test_checker_rejects_stale_resource_base_version(self):
  s,schema=inputs();case(s,"EESPE-011")["candidates"][0]["resource_writes"][0]["base_version"]="old-version"
  reject(s,schema)

if __name__=="__main__":unittest.main()
