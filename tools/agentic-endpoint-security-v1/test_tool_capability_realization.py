#!/usr/bin/env python3
"""Mutation tests for the declared/code/observed capability frontier."""
from __future__ import annotations
import copy
import sys
import unittest
from pathlib import Path
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import check_tool_capability_realization as checker  # noqa: E402


def case(case_id: str) -> dict:
    suite = checker.load(checker.BASE / "tool-capability-realization.fixtures.json")
    return copy.deepcopy(next(item for item in suite["cases"] if item["scenario_id"] == case_id))


def reject(mutated: dict) -> None:
    try:
        checker.validate_case(mutated)
    except ValueError:
        return
    raise AssertionError("malformed capability-realization case was accepted")


def test_frozen_corpus_matches_three_surface_outcomes() -> None:
    suite = checker.load(checker.BASE / "tool-capability-realization.fixtures.json")
    schema = checker.load(checker.BASE / "tool-capability-realization.schema.json")
    assert checker.validate_suite(suite, schema) == {"positive_control": 3, "negative_control": 5, "indeterminate_control": 4}


def test_dormant_undeclared_path_is_potential_not_observed_breach() -> None:
    out = checker.evaluate_case(case("CARI-002"))
    assert out["potential_undeclared_effect_count"] == 1
    assert out["observed_undeclared_effect_count"] == 0
    assert out["disposition"] == "implementation_expands_declaration"


def test_independent_observation_of_undeclared_effect_is_distinct_outcome() -> None:
    out = checker.evaluate_case(case("CARI-003"))
    assert out["observed_undeclared_effect_count"] == 1
    assert out["disposition"] == "observed_undeclared_effect"


def test_observed_effect_missing_from_static_surface_is_analysis_closure_failure() -> None:
    out = checker.evaluate_case(case("CARI-007"))
    assert out["observed_unmodeled_effect_count"] == 1
    assert out["disposition"] == "observed_unmodeled_effect"


def test_overdeclared_surface_is_not_misreported_as_implementation_expansion() -> None:
    out = checker.evaluate_case(case("CARI-004"))
    assert out["declaration_overstatement_count"] == 1
    assert out["potential_undeclared_effect_count"] == 0
    assert out["disposition"] == "declaration_overstates_implementation"


def test_unresolved_dynamic_module_remains_indeterminate() -> None:
    out = checker.evaluate_case(case("CARI-005"))
    assert out["unresolved_implementation_path_count"] == 1
    assert out["disposition"] == "indeterminate"


def test_untrusted_observer_claim_does_not_become_observed_effect_truth() -> None:
    out = checker.evaluate_case(case("CARI-008"))
    assert out["observed_undeclared_effect_count"] == 1
    assert out["observer_qualified"] is False
    assert out["disposition"] == "indeterminate"


def test_friendly_version_label_does_not_merge_distinct_builds() -> None:
    suite = checker.load(checker.BASE / "tool-capability-realization.fixtures.json")
    a = next(c for c in suite["cases"] if c["scenario_id"] == "CARI-009")
    b = next(c for c in suite["cases"] if c["scenario_id"] == "CARI-010")
    assert a["artifact_binding"]["human_version_label"] == b["artifact_binding"]["human_version_label"]
    assert a["artifact_binding"]["source_commit"] != b["artifact_binding"]["source_commit"]
    assert a["artifact_binding"]["binary_digest"] != b["artifact_binding"]["binary_digest"]


def test_missing_binary_build_binding_is_indeterminate() -> None:
    out = checker.evaluate_case(case("CARI-011"))
    assert out["artifact_binding_complete"] is False
    assert out["disposition"] == "indeterminate"


def test_same_canonical_effect_id_cannot_name_two_tuples() -> None:
    c = case("CARI-001")
    mutated = copy.deepcopy(c["declared_effects"][0])
    mutated["resource_id"] = "resource:other"
    c["observed_effects"].append({"effect": mutated, "evidence_ref": "oracle:mutated"})
    reject(c)


def test_same_semantic_effect_cannot_be_fragmented_across_ids() -> None:
    c = case("CARI-001")
    fragmented = copy.deepcopy(c["declared_effects"][0])
    fragmented["effect_id"] = "effect:alias-with-new-id"
    c["observed_effects"].append({"effect": fragmented, "evidence_ref": "oracle:fragmented"})
    reject(c)


def test_complete_static_analysis_cannot_contain_unresolved_path() -> None:
    c = case("CARI-005")
    c["static_analysis"]["complete"] = True
    reject(c)


def test_unresolved_path_cannot_assert_an_exact_effect() -> None:
    c = case("CARI-005")
    c["implementation_effects"][1]["effect"] = copy.deepcopy(c["declared_effects"][0])
    reject(c)


def test_bad_source_commit_does_not_qualify_artifact_binding() -> None:
    c = case("CARI-001")
    c["artifact_binding"]["source_commit"] = "main"
    out = checker.evaluate_case(c)
    assert out["artifact_binding_complete"] is False
    assert out["disposition"] == "indeterminate"


if __name__ == "__main__":
    functions = [fn for name, fn in globals().items() if name.startswith("test_") and callable(fn)]
    suite = unittest.TestSuite(unittest.FunctionTestCase(fn, description=fn.__name__) for fn in functions)
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    raise SystemExit(0 if result.wasSuccessful() else 1)
