#!/usr/bin/env python3
"""Mutation tests for subject-bound assurance claim/evidence qualification."""
from __future__ import annotations
import copy
import sys
import unittest
from pathlib import Path
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE))
import check_evidence_claim_qualification as checker  # noqa: E402


def inputs():
    suite=checker.load(checker.BASE/"evidence-claim-qualification.fixtures.json")
    schema=checker.load(checker.BASE/"evidence-claim-qualification.schema.json")
    return suite,schema


def case(suite,scenario):
    return next(item for item in suite["cases"] if item["scenario_id"]==scenario)


def reject(suite,schema):
    try:checker.validate_suite(suite,schema)
    except ValueError:return
    raise AssertionError("mutated ECQ suite was accepted")


class EvidenceClaimTests(unittest.TestCase):
    def test_frozen_twenty_case_claim_qualification_panel(self):
        suite,schema=inputs()
        self.assertEqual(checker.validate_suite(suite,schema),{"cases":20,"positive_control":4,"negative_control":12,"indeterminate_control":4})

    def test_synthetic_checker_and_mutations_support_only_synthetic_profile(self):
        suite,_=inputs()
        result=checker.evaluate(case(suite,"ECQ-001"))
        self.assertEqual(result["claim_state"],"qualified")
        self.assertEqual(result["supported_profiles"],["synthetic_contract"])

    def test_exact_head_workflow_must_match_subject_commit(self):
        suite,_=inputs()
        result=checker.evaluate(case(suite,"ECQ-003"))
        self.assertEqual(result["claim_state"],"failed")
        self.assertIn("workflow",result["obligation_status"] if "workflow" in result["obligation_status"] else result["residual_obligation_ids"][0])

    def test_in_progress_workflow_is_indeterminate_not_pass(self):
        suite,_=inputs()
        self.assertEqual(checker.evaluate(case(suite,"ECQ-004"))["claim_state"],"indeterminate")

    def test_failed_required_check_fails_hosted_claim(self):
        suite,_=inputs()
        self.assertEqual(checker.evaluate(case(suite,"ECQ-005"))["claim_state"],"failed")

    def test_attestation_subject_digest_must_match_claim_subject(self):
        suite,_=inputs()
        self.assertEqual(checker.evaluate(case(suite,"ECQ-006"))["claim_state"],"failed")

    def test_signature_without_trusted_attester_is_not_enough(self):
        suite,_=inputs()
        self.assertEqual(checker.evaluate(case(suite,"ECQ-008"))["claim_state"],"failed")

    def test_missing_runtime_identity_is_indeterminate(self):
        suite,_=inputs()
        self.assertEqual(checker.evaluate(case(suite,"ECQ-009"))["claim_state"],"indeterminate")

    def test_non_disposable_runtime_cannot_qualify(self):
        suite,_=inputs()
        self.assertEqual(checker.evaluate(case(suite,"ECQ-010"))["claim_state"],"failed")

    def test_agent_self_report_is_not_target_effect_evidence(self):
        suite,_=inputs()
        self.assertEqual(checker.evaluate(case(suite,"ECQ-011"))["claim_state"],"failed")

    def test_synthetic_observer_suite_does_not_qualify_deployed_independence(self):
        suite,_=inputs()
        self.assertEqual(checker.evaluate(case(suite,"ECQ-012"))["claim_state"],"failed")

    def test_stale_currentness_does_not_support_runtime_claim(self):
        suite,_=inputs()
        self.assertEqual(checker.evaluate(case(suite,"ECQ-013"))["claim_state"],"failed")

    def test_unqualified_timebase_is_indeterminate(self):
        suite,_=inputs()
        self.assertEqual(checker.evaluate(case(suite,"ECQ-014"))["claim_state"],"indeterminate")

    def test_unsealed_claim_bundle_fails(self):
        suite,_=inputs()
        self.assertEqual(checker.evaluate(case(suite,"ECQ-015"))["claim_state"],"failed")

    def test_superseded_evidence_is_not_current(self):
        suite,_=inputs()
        self.assertEqual(checker.evaluate(case(suite,"ECQ-016"))["claim_state"],"failed")

    def test_runtime_descriptor_must_match_current_claim_descriptor(self):
        suite,_=inputs()
        self.assertEqual(checker.evaluate(case(suite,"ECQ-017"))["claim_state"],"failed")

    def test_bounded_edit_proof_supports_only_its_own_profile(self):
        suite,_=inputs()
        result=checker.evaluate(case(suite,"ECQ-018"))
        self.assertEqual(result["claim_state"],"qualified")
        self.assertEqual(result["supported_profiles"],["bounded_edit_contract"])

    def test_complete_independent_no_effect_observation_is_a_scoped_negative_claim(self):
        suite,_=inputs()
        result=checker.evaluate(case(suite,"ECQ-019"))
        self.assertEqual(result["claim_state"],"qualified")
        self.assertIn("independent_effect_observation",result["supported_profiles"])

    def test_missing_critical_observer_obligation_remains_residual(self):
        suite,_=inputs()
        result=checker.evaluate(case(suite,"ECQ-020"))
        self.assertEqual(result["claim_state"],"indeterminate")
        self.assertTrue(any("OBSERVER_FAULT" in item for item in result["residual_obligation_ids"]))

    def test_required_checks_cannot_be_silently_expanded(self):
        suite,schema=inputs()
        case(suite,"ECQ-002")["evidence"][2]["passed_checks"].remove("frozen-mutation-suite")
        reject(suite,schema)

    def test_wrong_subject_commit_is_not_repaired_by_valid_signature(self):
        suite,schema=inputs()
        evidence=case(suite,"ECQ-006")["evidence"]
        att=next(x for x in evidence if x["evidence_type"]=="attestation")
        att["subject_commit"]="f"*40
        reject(suite,schema)

    def test_duplicate_obligation_categories_are_rejected(self):
        suite,schema=inputs()
        case(suite,"ECQ-001")["obligations"].append(copy.deepcopy(case(suite,"ECQ-001")["obligations"][0]))
        reject(suite,schema)

    def test_duplicate_evidence_ids_are_rejected(self):
        suite,schema=inputs()
        c=case(suite,"ECQ-001")
        c["evidence"].append(copy.deepcopy(c["evidence"][0]))
        reject(suite,schema)

    def test_claim_support_is_not_collapsed_to_scalar_score(self):
        suite,schema=inputs()
        suite["no_aggregate_score"]=False
        reject(suite,schema)

    def test_signature_verification_flag_is_not_omitted(self):
        suite,schema=inputs()
        evidence=next(x for x in case(suite,"ECQ-007")["evidence"] if x["evidence_type"]=="attestation")
        del evidence["signature_verified"]
        reject(suite,schema)

    def test_target_observation_must_cover_claimed_resource_and_operation(self):
        suite,_=inputs()
        result=checker.evaluate(case(suite,"ECQ-020"))
        self.assertIn("observer_fault",result["residual_obligation_ids"][0])

if __name__=="__main__":unittest.main()
