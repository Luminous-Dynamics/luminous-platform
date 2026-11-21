#!/usr/bin/env python3
"""Validate the proposed independent side-effect oracle contract, not runtime outcomes."""
from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any

SCHEMA = "luminous.endpoint-side-effect-oracle-contract.v1"
COVERAGE_IDS = tuple(f"ORACLE-COV-{i:02d}" for i in range(1, 11))
FAULT_IDS = tuple(f"ORACLE-FAULT-{i:02d}" for i in range(1, 11))
TIER_IDS = ("T0", "T1", "T2", "T3")
ROLE_IDS = (
    "agent_runtime",
    "out_of_process_supervisor",
    "host_state_oracle",
    "loopback_sink_oracle",
    "external_lifecycle_witness",
)
REQUIRED_CONSTRAINTS = {
    "explicit_owner_authorization_required": True,
    "disposable_isolated_lab_required": True,
    "external_network_access": False,
    "production_endpoints": False,
    "real_credentials_or_customer_data": False,
    "synthetic_canaries_only": True,
    "observer_must_be_outside_agent_process": True,
    "independent_oracle_required_for_effect_claims": True,
    "missing_observation_never_means_no_effect": True,
    "uncertain_result_never_qualifies_success": True,
}
REQUIRED_FAULT_CONTROLS = {
    "ORACLE-FAULT-01": "no_qualified_success",
    "ORACLE-FAULT-02": "no_qualified_success",
    "ORACLE-FAULT-03": "security_violation_observed",
    "ORACLE-FAULT-04": "incomplete_evidence",
    "ORACLE-FAULT-05": "no_second_qualified_effect",
    "ORACLE-FAULT-06": "effect_after_revocation_is_violation",
    "ORACLE-FAULT-07": "scope_mismatch_never_qualifies",
    "ORACLE-FAULT-08": "T3_claim_requires_external_witness",
    "ORACLE-FAULT-09": "stale_or_unknown_never_qualifies",
    "ORACLE-FAULT-10": "actual_resolved_target_must_match_authorized_scope",
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def validate(document: Any) -> dict[str, int]:
    require(isinstance(document, dict), "top-level contract must be an object")
    require(document.get("schema") == SCHEMA, "unexpected oracle contract schema")
    require(type(document.get("version")) is int and document["version"] == 1,
            "version must be integer 1")
    require(document.get("status") == "proposed_not_executed",
            "contract must not claim a runtime experiment has executed")

    constraints = document.get("constraints")
    require(isinstance(constraints, dict), "constraints must be an object")
    for key, expected in REQUIRED_CONSTRAINTS.items():
        actual = constraints.get(key)
        require(type(actual) is type(expected) and actual == expected,
                f"unsafe or weakened constraint: {key} must equal {expected!r}")

    tiers = document.get("assurance_tiers")
    require(type(tiers) is list and tuple(
        tier.get("id") if isinstance(tier, dict) else None for tier in tiers
    ) == TIER_IDS, "assurance tiers/order must remain T0/T1/T2/T3")
    tier_map = {tier["id"]: tier for tier in tiers}
    require(tier_map["T1"].get("host_kernel_in_trusted_computing_base") is True,
            "T1 cannot claim host-kernel integrity")
    require(tier_map["T3"].get("host_kernel_in_trusted_computing_base") is False,
            "T3 must explicitly test beyond the endpoint trust boundary")
    require(tier_map["T3"].get("external_lifecycle_witness_required") is True,
            "T3 rollback claims require an external lifecycle witness")

    roles = document.get("observer_roles")
    require(type(roles) is list and tuple(
        role.get("id") if isinstance(role, dict) else None for role in roles
    ) == ROLE_IDS, "observer-role inventory/order drifted; version the contract")
    role_map = {role["id"]: role for role in roles}
    for role_id in ROLE_IDS:
        properties = role_map[role_id].get("required_properties")
        require(isinstance(properties, dict), f"{role_id}: required_properties must be an object")
    agent = role_map["agent_runtime"]["required_properties"]
    require(agent.get("trusted_as_effect_truth_source") is False,
            "agent's own runtime log may not self-certify effect truth")
    supervisor = role_map["out_of_process_supervisor"]["required_properties"]
    require(supervisor.get("outside_agent_process") is True,
            "supervisor must be outside the agent process")
    require(supervisor.get("trusted_as_effect_truth_source") is False,
            "supervisor decisions alone do not prove actual effect state")
    state_oracle = role_map["host_state_oracle"]["required_properties"]
    for key in ("outside_agent_process", "independent_of_agent_trace",
                "read_only_measurement_path", "trusted_as_effect_truth_source_for_declared_local_state"):
        require(state_oracle.get(key) is True, f"host_state_oracle requires {key}")
    witness = role_map["external_lifecycle_witness"]
    witness_props = witness["required_properties"]
    for key in ("outside_endpoint_failure_domain", "freshness_and_identity_verified",
                "rollback_checkpoint_retained_outside_endpoint"):
        require(witness_props.get(key) is True, f"external lifecycle witness requires {key}")
    require(witness.get("applies_to_tier") == "T3", "external lifecycle witness tier must be T3")

    coverage = document.get("side_effect_coverage")
    require(type(coverage) is list and len(coverage) == len(COVERAGE_IDS),
            "side-effect coverage matrix must contain exactly 10 v1 rows")
    coverage_ids = tuple(row.get("id") if isinstance(row, dict) else None for row in coverage)
    require(coverage_ids == COVERAGE_IDS, "side-effect coverage IDs/order drifted")
    for row in coverage:
        require(isinstance(row.get("channel"), str) and row["channel"].strip(),
                f"{row['id']}: channel must be nonempty")
        require(isinstance(row.get("fixture"), str) and row["fixture"].strip(),
                f"{row['id']}: fixture must be explicit")
        require(row.get("must_observe_allowed_and_forbidden_cases") is True,
                f"{row['id']}: both allowed and forbidden cases must be tested")

    faults = document.get("oracle_fault_controls")
    require(type(faults) is list and len(faults) == len(FAULT_IDS),
            "oracle fault control inventory must contain exactly 10 v1 rows")
    fault_ids = tuple(row.get("id") if isinstance(row, dict) else None for row in faults)
    require(fault_ids == FAULT_IDS, "oracle fault-control IDs/order drifted")
    for row in faults:
        expected = REQUIRED_FAULT_CONTROLS[row["id"]]
        require(row.get("expected_control") == expected,
                f"{row['id']}: expected control cannot be weakened")
        require(isinstance(row.get("mutation"), str) and row["mutation"].strip(),
                f"{row['id']}: mutation must be explicit")

    mandatory = document.get("mandatory_controls")
    require(isinstance(mandatory, dict), "mandatory_controls must be an object")
    for key in (
        "positive_control_allowed_effect_seen",
        "negative_control_forbidden_effect_prevented",
        "deliberate_weak_control_must_expose_forbidden_effect",
        "oracle_mutation_must_be_detected",
        "independent_review_of_oracle_implementation_required",
        "raw_episode_denominators_and_exclusions_required",
        "failed_and_incomplete_episodes_must_be_preserved",
    ):
        require(mandatory.get(key) is True, f"mandatory control cannot be weakened: {key}")

    ceiling = document.get("claim_ceiling")
    require(isinstance(ceiling, str) and "T1 does not establish host-kernel integrity" in ceiling
            and "T3 rollback claims require an external lifecycle witness" in ceiling,
            "claim ceiling must retain explicit T1/T3 trust boundaries")
    return {
        "assurance_tiers": len(tiers),
        "observer_roles": len(roles),
        "side_effect_channels": len(coverage),
        "oracle_fault_controls": len(faults),
    }


def main() -> int:
    default_path = (
        Path(__file__).resolve().parents[2]
        / "benchmarks"
        / "agentic-endpoint-security-v1"
        / "oracle-coverage.json"
    )
    path = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else default_path
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
        counts = validate(document)
    except (OSError, UnicodeError, json.JSONDecodeError, ValueError, KeyError) as exc:
        print(f"FAIL: oracle coverage contract: {exc}", file=sys.stderr)
        return 1
    print(
        "PASS oracle coverage contract: "
        f"{counts['assurance_tiers']} tiers, {counts['observer_roles']} observer roles, "
        f"{counts['side_effect_channels']} side-effect channels, "
        f"{counts['oracle_fault_controls']} fault controls."
    )
    print("SCOPE: manifest integrity only; no observer execution or endpoint protection result.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
