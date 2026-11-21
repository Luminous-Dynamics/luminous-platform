#!/usr/bin/env python3
"""Script-invoked mutation tests for observer independence and common-cause failure."""
from __future__ import annotations
import copy
import unittest
from pathlib import Path
import sys

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import check_observer_independence_closure as checker  # noqa: E402


def fixture(case_id: str) -> dict:
    suite = checker.load(checker.BASE / "observer-independence-closure.fixtures.json")
    return copy.deepcopy(next(item for item in suite["cases"] if item["scenario_id"] == case_id))


def profile(suite: dict) -> dict:
    return suite["claim_profiles"]["critical_effect_truth_v0"]


def test_frozen_corpus_matches_expected() -> None:
    suite = checker.load(checker.BASE / "observer-independence-closure.fixtures.json")
    schema = checker.load(checker.BASE / "observer-independence-closure.schema.json")
    assert checker.validate_suite(suite, schema) == {
        "positive_control": 5,
        "negative_control": 8,
        "indeterminate_control": 3,
    }


def test_different_processes_sharing_event_stream_and_signing_key_are_not_independent() -> None:
    result = checker.evaluate_case(fixture("OIC-002"), {
        "required_disjoint_domains": checker.DOMAIN_FIELDS[:7],
        "minimum_independent_witnesses": 2,
    })
    assert result["maximum_independent_witnesses"] == 1
    assert result["final_disposition"] == "unqualified"


def test_shared_signing_root_collapses_two_nominal_witnesses() -> None:
    result = checker.evaluate_case(fixture("OIC-003"), {
        "required_disjoint_domains": checker.DOMAIN_FIELDS[:7],
        "minimum_independent_witnesses": 2,
    })
    assert result["correlated_pair_count"] == 1
    assert result["final_disposition"] == "unqualified"


def test_shared_credential_root_is_a_common_cause() -> None:
    result = checker.evaluate_case(fixture("OIC-004"), {
        "required_disjoint_domains": checker.DOMAIN_FIELDS[:7],
        "minimum_independent_witnesses": 2,
    })
    assert result["maximum_independent_witnesses"] == 1


def test_shared_clock_storage_and_control_plane_are_detected() -> None:
    for case_id in ("OIC-005", "OIC-006", "OIC-007"):
        result = checker.evaluate_case(fixture(case_id), {
            "required_disjoint_domains": checker.DOMAIN_FIELDS[:7],
            "minimum_independent_witnesses": 2,
        })
        assert result["maximum_independent_witnesses"] == 1
        assert result["final_disposition"] == "unqualified"


def test_missing_critical_domain_is_unknown_not_unique() -> None:
    result = checker.evaluate_case(fixture("OIC-008"), {
        "required_disjoint_domains": checker.DOMAIN_FIELDS[:7],
        "minimum_independent_witnesses": 2,
    })
    assert result["unknown_domain_witnesses"] == 1
    assert result["maximum_independent_witnesses"] == 1
    assert result["final_disposition"] == "indeterminate"


def test_independent_disagreement_is_not_majority_voted_away() -> None:
    result = checker.evaluate_case(fixture("OIC-009"), {
        "required_disjoint_domains": checker.DOMAIN_FIELDS[:7],
        "minimum_independent_witnesses": 2,
    })
    assert result["maximum_independent_witnesses"] == 2
    assert result["observation_agreement"] == "conflict"
    assert result["final_disposition"] == "indeterminate"


def test_independent_no_effect_quorum_is_a_valid_positive_control() -> None:
    result = checker.evaluate_case(fixture("OIC-010"), {
        "required_disjoint_domains": checker.DOMAIN_FIELDS[:7],
        "minimum_independent_witnesses": 2,
    })
    assert result["observation_agreement"] == "no_effect"
    assert result["final_disposition"] == "qualified_no_effect"


def test_observers_sharing_subject_event_stream_do_not_qualify() -> None:
    result = checker.evaluate_case(fixture("OIC-011"), {
        "required_disjoint_domains": checker.DOMAIN_FIELDS[:7],
        "minimum_independent_witnesses": 2,
    })
    assert result["subject_correlated_witnesses"] == ["a", "b"]
    assert result["maximum_independent_witnesses"] == 0


def test_pairwise_maximum_independent_set_can_exclude_correlated_witnesses() -> None:
    result = checker.evaluate_case(fixture("OIC-012"), {
        "required_disjoint_domains": checker.DOMAIN_FIELDS[:7],
        "minimum_independent_witnesses": 2,
    })
    assert result["maximum_independent_witnesses"] == 2
    assert result["final_disposition"] == "qualified_effect"


def test_profile_scope_is_explicit_and_non_required_domains_are_not_silently_imposed() -> None:
    result = checker.evaluate_case(fixture("OIC-013"), {
        "required_disjoint_domains": checker.DOMAIN_FIELDS[:7],
        "minimum_independent_witnesses": 2,
    })
    assert result["maximum_independent_witnesses"] == 2
    assert result["final_disposition"] == "qualified_effect"


def test_unknown_witness_does_not_poison_a_separate_independent_quorum() -> None:
    result = checker.evaluate_case(fixture("OIC-014"), {
        "required_disjoint_domains": checker.DOMAIN_FIELDS[:7],
        "minimum_independent_witnesses": 2,
    })
    assert result["unknown_domain_witnesses"] == 1
    assert result["maximum_independent_witnesses"] == 2
    assert result["final_disposition"] == "qualified_no_effect"


def test_common_credential_and_control_plane_still_count_as_single_failure_domain() -> None:
    result = checker.evaluate_case(fixture("OIC-015"), {
        "required_disjoint_domains": checker.DOMAIN_FIELDS[:7],
        "minimum_independent_witnesses": 2,
    })
    assert result["maximum_independent_witnesses"] == 1
    assert result["final_disposition"] == "unqualified"


def test_missing_all_required_domains_remains_indeterminate() -> None:
    result = checker.evaluate_case(fixture("OIC-016"), {
        "required_disjoint_domains": checker.DOMAIN_FIELDS[:7],
        "minimum_independent_witnesses": 2,
    })
    assert result["unknown_domain_witnesses"] == 2
    assert result["final_disposition"] == "indeterminate"


def test_claim_profile_cannot_drop_required_failure_domains() -> None:
    suite = checker.load(checker.BASE / "observer-independence-closure.fixtures.json")
    schema = checker.load(checker.BASE / "observer-independence-closure.schema.json")
    suite["claim_profiles"]["critical_effect_truth_v0"]["required_disjoint_domains"] = ["process_runtime"]
    try:
        checker.validate_suite(suite, schema)
    except ValueError:
        return
    raise AssertionError("weakened independence profile was accepted")


def test_duplicate_witness_id_fails_closed() -> None:
    case = fixture("OIC-001")
    case["witnesses"].append(copy.deepcopy(case["witnesses"][0]))
    try:
        checker.evaluate_case(case, {
            "required_disjoint_domains": checker.DOMAIN_FIELDS[:7],
            "minimum_independent_witnesses": 2,
        })
    except Exception:
        return
    # Duplicate identities are rejected by suite structure validation; keep the
    # case-level evaluator free of any first/last-wins interpretation.
    raise AssertionError("duplicate witness identity was accepted")


if __name__ == "__main__":
    functions = [
        value for name, value in globals().items()
        if name.startswith("test_") and callable(value)
    ]
    suite = unittest.TestSuite(unittest.FunctionTestCase(fn, description=fn.__name__) for fn in functions)
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    raise SystemExit(0 if result.wasSuccessful() else 1)
