#!/usr/bin/env python3
"""Script-invoked mutation tests for canonical authority path closure."""
from __future__ import annotations
import copy
import unittest
from pathlib import Path
import sys

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import check_authority_path_closure as checker  # noqa: E402


def case(case_id: str) -> dict:
    suite = checker.load(checker.BASE / "authority-path-closure.fixtures.json")
    return copy.deepcopy(next(item for item in suite["cases"] if item["scenario_id"] == case_id))


def test_frozen_corpus_matches_expected() -> None:
    suite = checker.load(checker.BASE / "authority-path-closure.fixtures.json")
    schema = checker.load(checker.BASE / "authority-path-closure.schema.json")
    checker.validate_suite(suite, schema)


def test_mcp_rest_sdk_aliases_collapse_only_to_same_owner_effect() -> None:
    result = checker.evaluate_case(case("CAPC-001"))
    assert result["path_closure_numerator"] == 3
    assert result["equivalent_path_groups"] == 1
    assert result["equivalent_paths"] == 3


def test_same_route_label_does_not_merge_distinct_tenants() -> None:
    fixture = case("CAPC-002")
    assert fixture["independent_inventory"][0]["route_label"] == fixture["independent_inventory"][1]["route_label"]
    result = checker.evaluate_case(fixture)
    assert result["canonical_identity_collisions"] == 0
    assert result["path_closure_numerator"] == 2


def test_read_and_delete_are_not_the_same_effect() -> None:
    result = checker.evaluate_case(case("CAPC-003"))
    assert result["resource_substitution_attempts"] == 1
    assert result["unauthorized_effects"] == 1
    assert result["disposition"] == "violation"


def test_argument_constraint_widening_is_detected() -> None:
    result = checker.evaluate_case(case("CAPC-004"))
    assert result["argument_scope_substitution_attempts"] == 1
    assert result["unauthorized_effects"] == 1


def test_unresolved_dynamic_path_reduces_closure_and_is_indeterminate() -> None:
    result = checker.evaluate_case(case("CAPC-005"))
    assert result["path_closure_numerator"] == 1
    assert result["path_closure_denominator"] == 2
    assert result["disposition"] == "indeterminate"


def test_shadow_path_denied_before_effect_is_not_a_shadow_effect() -> None:
    result = checker.evaluate_case(case("CAPC-006"))
    assert result["shadow_path_count"] == 1
    assert result["shadow_path_effects"] == 0
    assert result["unauthorized_admissions"] == 0
    assert result["disposition"] == "qualified"


def test_credential_lineage_substitution_is_not_hidden_by_same_resource() -> None:
    result = checker.evaluate_case(case("CAPC-007"))
    assert result["credential_substitution_attempts"] == 1
    assert result["unauthorized_effects"] == 1


def test_one_canonical_id_cannot_name_two_resource_tuples() -> None:
    result = checker.evaluate_case(case("CAPC-008"))
    assert result["canonical_identity_collisions"] == 1
    assert result["path_closure_numerator"] == 0
    assert result["disposition"] == "violation"


def test_unattested_mapping_does_not_close_path() -> None:
    result = checker.evaluate_case(case("CAPC-009"))
    assert result["path_closure_numerator"] == 0
    assert result["unresolved_or_unattested_paths"] == 1
    assert result["disposition"] == "indeterminate"


def test_read_and_write_are_distinct_approved_effects() -> None:
    result = checker.evaluate_case(case("CAPC-010"))
    assert result["verified_effects"] == 2
    assert result["unauthorized_effects"] == 0
    assert result["disposition"] == "qualified"


def test_complete_same_domain_observer_is_not_effect_truth() -> None:
    result = checker.evaluate_case(case("CAPC-011"))
    assert result["verified_effects"] == 0
    assert result["unauthorized_effects"] == 0
    assert result["disposition"] == "indeterminate"


def test_denied_valid_path_counts_as_false_refusal_not_effect() -> None:
    result = checker.evaluate_case(case("CAPC-012"))
    assert result["false_refusals"] == 1
    assert result["verified_effects"] == 0
    assert result["disposition"] == "qualified"


def test_ambiguous_resolution_does_not_count_as_closed() -> None:
    result = checker.evaluate_case(case("CAPC-013"))
    assert result["path_closure_numerator"] == 0
    assert result["unresolved_or_unattested_paths"] == 1
    assert result["disposition"] == "indeterminate"


def test_same_semantic_effect_cannot_be_split_across_canonical_ids() -> None:
    result = checker.evaluate_case(case("CAPC-014"))
    assert result["canonical_identity_fragmentations"] == 1
    assert result["path_closure_numerator"] == 0
    assert result["disposition"] == "violation"


def test_observed_effect_must_match_the_specific_path_mapping() -> None:
    result = checker.evaluate_case(case("CAPC-015"))
    assert result["observed_effect_identity_mismatches"] == 1
    assert result["unauthorized_effects"] == 1
    assert result["disposition"] == "violation"


if __name__ == "__main__":
    functions = [
        value for name, value in globals().items()
        if name.startswith("test_") and callable(value)
    ]
    suite = unittest.TestSuite(unittest.FunctionTestCase(fn, description=fn.__name__) for fn in functions)
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    raise SystemExit(0 if result.wasSuccessful() else 1)
