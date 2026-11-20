#!/usr/bin/env python3
"""Mutation tests for provenance-preserving source-to-sink agent flows."""
from __future__ import annotations
import copy
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import check_information_flow_lifecycle as checker  # noqa: E402


def fixture(case_id: str) -> dict:
    suite = checker.load(checker.BASE / "information-flow-lifecycle.fixtures.json")
    return copy.deepcopy(next(item for item in suite["cases"] if item["scenario_id"] == case_id))


def test_frozen_corpus_matches_independent_reference_outcomes() -> None:
    suite = checker.load(checker.BASE / "information-flow-lifecycle.fixtures.json")
    schema = checker.load(checker.BASE / "information-flow-lifecycle.schema.json")
    counts = checker.validate_suite(suite, schema)
    assert counts == {"positive_control": 5, "negative_control": 12, "indeterminate_control": 2}


def test_fully_specified_user_parameters_are_preserved() -> None:
    result = checker.evaluate_case(fixture("IFL-001"))
    assert result["admission_decision"] == "allow"
    assert result["system_admission_mismatches"] == 0
    assert result["unauthorized_disclosures"] == 0


def test_parameter_open_named_source_may_fill_only_delegated_amount() -> None:
    result = checker.evaluate_case(fixture("IFL-002"))
    assert result["admission_decision"] == "allow"
    assert result["flow_decision"] == "eligible"


def test_attacker_origin_cannot_replace_fixed_recipient() -> None:
    result = checker.evaluate_case(fixture("IFL-003"))
    assert result["flow_decision"] == "denied"
    assert result["untrusted_parameter_count"] == 1
    assert result["admission_decision"] == "deny"


def test_named_source_cannot_change_action_selection() -> None:
    result = checker.evaluate_case(fixture("IFL-004"))
    assert result["action_decision"] == "denied"
    assert result["action_origin_violations"] > 0


def test_flow_cannot_substitute_action_origin_node() -> None:
    result = checker.evaluate_case(fixture("IFL-020"))
    assert result["action_decision"] == "denied"
    assert result["action_origin_violations"] == 2
    assert result["unsafe_admissions"] == 1
    assert result["unauthorized_disclosures"] == 1


def test_action_open_is_bounded_to_finite_authorized_set() -> None:
    allowed = checker.evaluate_case(fixture("IFL-005"))
    blocked = checker.evaluate_case(fixture("IFL-006"))
    assert allowed["admission_decision"] == "allow"
    assert blocked["action_decision"] == "denied"
    assert blocked["admission_decision"] == "deny"


def test_summary_cannot_launder_untrusted_source_lineage() -> None:
    result = checker.evaluate_case(fixture("IFL-007"))
    assert result["lineage_erasure_count"] > 0
    assert result["flow_decision"] == "denied"


def test_embedding_and_memory_derivative_preserve_source_set() -> None:
    result = checker.evaluate_case(fixture("IFL-008"))
    assert result["lineage_erasure_count"] > 0
    assert result["flow_decision"] == "denied"


def test_child_agent_handoff_cannot_reset_origin() -> None:
    result = checker.evaluate_case(fixture("IFL-009"))
    assert result["lineage_erasure_count"] > 0
    assert result["flow_decision"] == "denied"


def test_join_uses_union_of_sources_and_worst_integrity() -> None:
    result = checker.evaluate_case(fixture("IFL-010"))
    assert result["flow_decision"] == "denied"
    assert result["untrusted_parameter_count"] == 1
    assert result["unsafe_admissions"] == 1
    assert result["unauthorized_disclosures"] == 1


def test_secret_to_public_sink_requires_exact_declassification_receipt() -> None:
    blocked = checker.evaluate_case(fixture("IFL-011"))
    allowed = checker.evaluate_case(fixture("IFL-012"))
    assert blocked["admission_decision"] == "deny"
    assert blocked["declassification_receipt_failures"] > 0
    assert allowed["admission_decision"] == "allow"
    assert allowed["declassification_receipt_failures"] == 0


def test_expired_or_misbound_declassification_receipt_is_rejected() -> None:
    result = checker.evaluate_case(fixture("IFL-013"))
    assert result["admission_decision"] == "deny"
    assert result["declassification_receipt_failures"] > 0


def test_cross_tenant_memory_reuse_is_rejected() -> None:
    result = checker.evaluate_case(fixture("IFL-014"))
    assert result["cross_tenant_flow_count"] == 1
    assert result["cross_purpose_flow_count"] == 1
    assert result["admission_decision"] == "deny"


def test_unknown_source_receipt_holds_instead_of_becoming_trusted() -> None:
    result = checker.evaluate_case(fixture("IFL-015"))
    assert result["unknown_lineage_count"] > 0
    assert result["admission_decision"] == "hold"
    assert result["disposition"] == "indeterminate"


def test_cyclic_lineage_remains_unknown() -> None:
    result = checker.evaluate_case(fixture("IFL-016"))
    assert result["cyclic_lineage_count"] > 0
    assert result["admission_decision"] == "hold"


def test_benign_named_source_workflow_remains_usable() -> None:
    result = checker.evaluate_case(fixture("IFL-017"))
    assert result["admission_decision"] == "allow"
    assert result["false_refusals"] == 0


def test_blocked_exfiltration_without_effect_is_not_a_breach() -> None:
    result = checker.evaluate_case(fixture("IFL-018"))
    assert result["admission_decision"] == "deny"
    assert result["unauthorized_disclosures"] == 0
    assert result["disposition"] == "qualified"


def test_sink_observation_must_match_exact_bound_parameter_digests() -> None:
    result = checker.evaluate_case(fixture("IFL-019"))
    assert result["observed_payload_mismatches"] == 2
    assert result["unauthorized_disclosures"] == 1
    assert result["disposition"] == "violation"


def test_same_payload_without_declassification_cannot_use_an_authorized_sink() -> None:
    case = fixture("IFL-012")
    case["declassification_receipts"][0]["audience"] = "different-audience"
    result = checker.evaluate_case(case)
    assert result["admission_decision"] == "deny"
    assert result["declassification_receipt_failures"] > 0


def test_observer_must_be_both_complete_and_independent() -> None:
    case = fixture("IFL-010")
    case["observer"] = {"complete": True, "independent": False}
    result = checker.evaluate_case(case)
    assert result["independently_observed_effects"] == 0
    assert result["unauthorized_disclosures"] == 0
    assert result["disposition"] == "violation"  # unsafe admission is independently visible


def test_positive_control_false_refusal_is_reported_separately() -> None:
    case = fixture("IFL-002")
    case["flow"]["system_admission"] = "deny"
    case["flow"]["effect_observation"] = "none"
    case["flow"]["observed_payload_digest"] = None
    case["flow"]["observed_bindings"] = []
    result = checker.evaluate_case(case)
    assert result["admission_decision"] == "allow"
    assert result["false_refusals"] == 1
    assert result["unauthorized_disclosures"] == 0


def test_duplicate_lineage_node_identity_fails_closed() -> None:
    case = fixture("IFL-001")
    case["lineage_nodes"].append(copy.deepcopy(case["lineage_nodes"][0]))
    try:
        checker.LineageEvaluator(case)
    except ValueError:
        return
    raise AssertionError("duplicate lineage-node identity was accepted")


def _assert_structure_rejects(mutate) -> None:
    suite = checker.load(checker.BASE / "information-flow-lifecycle.fixtures.json")
    schema = checker.load(checker.BASE / "information-flow-lifecycle.schema.json")
    mutate(suite["cases"][0])
    try:
        checker.validate_structure(suite, schema)
    except ValueError:
        return
    raise AssertionError("malformed flow binding was accepted")


def test_duplicate_parameter_binding_fields_are_rejected() -> None:
    _assert_structure_rejects(
        lambda case: case["flow"]["parameter_bindings"].append(
            copy.deepcopy(case["flow"]["parameter_bindings"][0])
        )
    )


def test_duplicate_observed_binding_fields_are_rejected() -> None:
    _assert_structure_rejects(
        lambda case: case["flow"]["observed_bindings"].append(
            copy.deepcopy(case["flow"]["observed_bindings"][0])
        )
    )


def test_observed_field_set_must_match_bound_parameter_set() -> None:
    _assert_structure_rejects(
        lambda case: case["flow"]["observed_bindings"].pop()
    )


if __name__ == "__main__":
    functions = [
        value for name, value in globals().items()
        if name.startswith("test_") and callable(value)
    ]
    suite = unittest.TestSuite(
        unittest.FunctionTestCase(fn, description=fn.__name__)
        for fn in functions
    )
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    raise SystemExit(0 if result.wasSuccessful() else 1)
