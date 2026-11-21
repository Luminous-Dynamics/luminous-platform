#!/usr/bin/env python3
"""Mutation tests for OIC common-cause fault-plan and recovery semantics."""
from __future__ import annotations

import copy
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import check_observer_common_cause_fault_injection as checker  # noqa: E402


def fixture(case_id: str) -> dict:
    suite = checker.load(checker.BASE / "observer-common-cause-fault-injection.fixtures.json")
    return copy.deepcopy(next(item for item in suite["cases"] if item["scenario_id"] == case_id))


def inputs():
    suite = checker.load(checker.BASE / "observer-common-cause-fault-injection.fixtures.json")
    schema = checker.load(checker.BASE / "observer-common-cause-fault-injection.schema.json")
    oic_suite = checker.oic.load(checker.oic.BASE / "observer-independence-closure.fixtures.json")
    oic_schema = checker.oic.load(checker.oic.BASE / "observer-independence-closure.schema.json")
    checker.oic.validate_suite(oic_suite, oic_schema)
    return suite, schema, oic_suite


def reject(case: dict, oic_suite: dict) -> None:
    try:
        checker.validate_case(case, oic_suite)
    except ValueError:
        return
    raise AssertionError("mutated CFI case was accepted")


def test_frozen_cfi_corpus_matches_oic_reference() -> None:
    suite, schema, oic_suite = inputs()
    assert checker.validate_suite(suite, schema, oic_suite) == {
        "positive_control": 5, "negative_control": 8, "indeterminate_control": 3
    }


def test_all_seven_required_domains_are_covered() -> None:
    suite = checker.load(checker.BASE / "observer-common-cause-fault-injection.fixtures.json")
    seen = {f["domain"] for c in suite["cases"] for f in c["posthoc_ground_truth"]["fault_plan"]}
    assert seen == set(checker.REQUIRED_DOMAINS)


def test_single_process_failure_preserves_unaffected_quorum() -> None:
    _, _, oic_suite = inputs()
    out = checker.evaluate_episode(fixture("CFI-002"), oic_suite)
    assert out["surviving_witness_ids"] == ["b", "c"]
    assert out["maximum_independent_witnesses"] == 2
    assert out["final_disposition"] == "qualified_effect"


def test_shared_event_stream_failure_removes_both_correlated_observers() -> None:
    _, _, oic_suite = inputs()
    case = fixture("CFI-003")
    base = next(c for c in oic_suite["cases"] if c["scenario_id"] == case["oic_scenario_id"])
    assert checker.impacted_witnesses(base, case["posthoc_ground_truth"]["fault_plan"]) == ["a", "b"]
    out = checker.evaluate_episode(case, oic_suite)
    assert out["maximum_independent_witnesses"] == 0
    assert out["final_disposition"] == "unqualified"


def test_compound_failures_reduce_three_observers_below_quorum() -> None:
    _, _, oic_suite = inputs()
    out = checker.evaluate_episode(fixture("CFI-014"), oic_suite)
    assert out["surviving_witness_ids"] == ["c"]
    assert out["maximum_independent_witnesses"] == 1
    assert out["final_disposition"] == "unqualified"


def test_recovery_requires_fresh_epoch_evidence_and_attestation() -> None:
    _, _, oic_suite = inputs()
    case = fixture("CFI-015")
    out = checker.evaluate_episode(case, oic_suite)
    assert out["maximum_independent_witnesses"] == 3
    assert out["final_disposition"] == "qualified_effect"
    checker.validate_case(case, oic_suite)


def test_recovery_cannot_reuse_pre_fault_attestation() -> None:
    _, _, oic_suite = inputs()
    case = fixture("CFI-015")
    case["evaluation_input"]["recovered_witnesses"][0]["replacement_witness"]["attestation_ref"] = "attestation:a"
    reject(case, oic_suite)


def test_recovery_epoch_must_strictly_advance() -> None:
    _, _, oic_suite = inputs()
    case = fixture("CFI-015")
    rec = case["evaluation_input"]["recovered_witnesses"][0]
    rec["to_evidence_epoch"] = rec["from_evidence_epoch"]
    reject(case, oic_suite)


def test_posthoc_truth_does_not_change_evaluator_disposition() -> None:
    _, _, oic_suite = inputs()
    case = fixture("CFI-002")
    before = checker.evaluate_episode(case, oic_suite)
    case["posthoc_ground_truth"]["fault_actually_injected"] = "unknown"
    case["posthoc_ground_truth"]["resource_effect_truth"] = "no_effect"
    after = checker.evaluate_episode(case, oic_suite)
    assert before == after


def test_fault_plan_must_be_hidden_until_disposition_commit() -> None:
    _, _, oic_suite = inputs()
    case = fixture("CFI-002")
    case["fault_plan_visibility"] = "visible_to_evaluator"
    reject(case, oic_suite)


def test_fault_target_must_match_exact_oic_dependency_identity() -> None:
    _, _, oic_suite = inputs()
    case = fixture("CFI-003")
    case["posthoc_ground_truth"]["fault_plan"][0]["dependency_ref"] = "friendly-name-only"
    reject(case, oic_suite)


def test_unqualified_observer_does_not_become_no_effect() -> None:
    _, _, oic_suite = inputs()
    out = checker.evaluate_episode(fixture("CFI-016"), oic_suite)
    assert out["maximum_independent_witnesses"] == 1
    assert out["final_disposition"] == "indeterminate"


def test_disagreement_is_not_majority_or_truth_overridden() -> None:
    _, _, oic_suite = inputs()
    out = checker.evaluate_episode(fixture("CFI-009"), oic_suite)
    assert out["observation_agreement"] == "conflict"
    assert out["final_disposition"] == "indeterminate"


def test_qualified_no_effect_must_match_posthoc_truth() -> None:
    _, _, oic_suite = inputs()
    case = fixture("CFI-010")
    case["posthoc_ground_truth"]["resource_effect_truth"] = "effect"
    reject(case)


def test_faulted_witness_cannot_be_reintroduced_without_recovery_record() -> None:
    _, _, oic_suite = inputs()
    case = fixture("CFI-002")
    case["evaluation_input"]["available_witness_ids"].append("a")
    reject(case)


def test_recovered_witness_must_no_longer_depend_on_faulted_root() -> None:
    _, _, oic_suite = inputs()
    case = fixture("CFI-015")
    case["evaluation_input"]["recovered_witnesses"][0]["replacement_witness"]["domains"]["process_runtime"] = "a:process_runtime"
    reject(case)


if __name__ == "__main__":
    functions = [fn for name, fn in globals().items() if name.startswith("test_") and callable(fn)]
    suite = unittest.TestSuite(unittest.FunctionTestCase(fn, description=fn.__name__) for fn in functions)
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    raise SystemExit(0 if result.wasSuccessful() else 1)
