#!/usr/bin/env python3
"""Validate the frozen Landlock descriptor-ordering contract, not runtime results."""
from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any

SCHEMA = "luminous.endpoint-launch-boundary-contract.v1"
SCENARIO_IDS = ("LBC-001", "LBC-002", "LBC-003", "LBC-004", "LBC-005")
MUTATION_IDS = tuple(f"LBC-MUT-{index:02d}" for index in range(1, 6))
EXPECTED_OUTCOMES = {
    "LBC-001": "FORBIDDEN_EFFECT_OBSERVED",
    "LBC-002": "BLOCK_BEFORE_EFFECT",
    "LBC-003": "BLOCK_BEFORE_EFFECT",
    "LBC-004": "ALLOW_AND_OBSERVE",
    "LBC-005": "BLOCKED_INCOMPLETE_EVIDENCE",
}
REQUIRED_CONSTRAINTS = {
    "explicit_owner_authorization_required": True,
    "isolated_disposable_lab_required": True,
    "external_network_access": False,
    "real_credentials_or_customer_data": False,
    "host_mutation_outside_fixture": False,
    "synthetic_canaries_only": True,
    "unexpected_or_inconclusive_effect_state_fails": True,
    "parent_or_external_state_oracle_required": True,
}
EXPECTED_ASSERTIONS = {
    "LBC-001": {
        "descriptor_is_opened_before_restriction",
        "worker_confirms_landlock_abi_at_least_3",
        "write_through_preopened_descriptor_succeeds",
        "parent_oracle_observes_exact_mutation",
    },
    "LBC-002": {
        "parent_descriptor_has_cloexec_before_exec",
        "worker_observes_ebadf_for_delegated_descriptor",
        "parent_descriptor_remains_open",
        "parent_oracle_observes_baseline_unchanged",
    },
    "LBC-003": {
        "landlock_handles_write_file_and_truncate",
        "no_path_write_grant_is_added",
        "new_write_open_fails_with_permission_denied",
        "parent_oracle_observes_baseline_unchanged",
    },
    "LBC-004": {
        "original_parent_descriptor_has_cloexec",
        "only_explicit_duplicate_differs_in_cloexec_state",
        "worker_uses_explicit_duplicate_number",
        "claim_does_not_extend_to_other_fd_types",
    },
    "LBC-005": {
        "landlock_abi_unavailable_fails_probe",
        "missing_worker_marker_fails_probe",
        "parent_oracle_error_fails_probe",
        "missing_or_malformed_json_fails_contract",
    },
}
EXPECTED_MUTATIONS = {
    "LBC-MUT-01": "contract_checker_rejects_missing_parent_oracle_assertion",
    "LBC-MUT-02": "contract_checker_rejects_outcome_semantic_drift",
    "LBC-MUT-03": "contract_checker_rejects_unsafe_execution_constraint",
    "LBC-MUT-04": "contract_checker_rejects_claim_ceiling_escalation",
    "LBC-MUT-05": "contract_checker_rejects_missing_mitigation",
}
RECEIPT_FIELDS = (
    "schema",
    "status",
    "checkout_sha",
    "kernel_release",
    "landlock_abi",
    "preopened_write_succeeded_after_landlock",
    "parent_oracle_observed_preopened_write",
    "cloexec_descriptor_absent_after_exec",
    "new_path_write_open_denied_after_landlock",
    "parent_oracle_safe_control_unchanged",
    "parent_original_fd_cloexec",
    "full_descriptor_hygiene_coverage",
    "scope",
)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def validate(document: Any) -> dict[str, int]:
    require(isinstance(document, dict), "contract must be an object")
    require(document.get("schema") == SCHEMA, "unexpected launch-boundary schema")
    require(type(document.get("version")) is int and document["version"] == 1,
            "version must be integer 1")
    require(document.get("status") == "proposed_not_executed",
            "contract cannot claim an experiment has executed")

    constraints = document.get("execution_constraints")
    require(isinstance(constraints, dict), "execution_constraints must be an object")
    for key, expected in REQUIRED_CONSTRAINTS.items():
        actual = constraints.get(key)
        require(type(actual) is type(expected) and actual == expected,
                f"unsafe or weakened execution constraint: {key}")

    scope = document.get("claim_scope")
    require(isinstance(scope, dict), "claim_scope must be an object")
    require(scope.get("assurance_tier") == "T1", "this contract may only claim T1")
    require(scope.get("host_kernel_in_trusted_computing_base") is True,
            "T1 must keep host kernel in the trusted computing base")
    for key in ("full_descriptor_hygiene_coverage", "production_launcher_qualified",
                "general_endpoint_security_qualified"):
        require(scope.get(key) is False, f"claim escalation forbidden: {key}")
    require(scope.get("tested_platform") == "Linux runner with Landlock ABI >= 3",
            "tested platform/ABI floor drifted")

    sources = document.get("source_semantics")
    require(type(sources) is list and tuple(
        row.get("id") if isinstance(row, dict) else None for row in sources
    ) == ("DOC-01", "DOC-02", "DOC-03"), "source-semantic inventory/order drifted")
    for source in sources:
        require(isinstance(source.get("claim"), str) and source["claim"].strip(),
                f"{source['id']}: source claim is empty")
        require(isinstance(source.get("source"), str)
                and source["source"].startswith("https://"),
                f"{source['id']}: source URL must be HTTPS")

    fixtures = document.get("fixtures")
    require(isinstance(fixtures, dict), "fixtures must be an object")
    require(fixtures.get("directory_mode") == "0700", "fixture directory must stay private")
    require(fixtures.get("file_mode") == "0600", "fixture file mode drifted")
    require(fixtures.get("baseline_marker") == "SAFE-CANARY-0000000000000000\n",
            "baseline canary marker drifted")
    require(fixtures.get("mutation_marker") == "EVIL-CANARY-0000000000000000\n",
            "mutation canary marker drifted")
    require(fixtures.get("baseline_and_mutation_same_length") is True,
            "positive and negative controls require equal-length markers")
    require(fixtures.get("file_contains_only_synthetic_data") is True,
            "fixture must contain synthetic data only")
    require(fixtures.get("parent_oracle_read_only_measurement") is True,
            "parent oracle must be independent read-only measurement")
    require(fixtures.get("original_parent_descriptor_must_be_cloexec") is True,
            "original parent descriptor must not be a second inheritance channel")

    scenarios = document.get("scenarios")
    require(type(scenarios) is list and tuple(
        row.get("id") if isinstance(row, dict) else None for row in scenarios
    ) == SCENARIO_IDS, "scenario inventory/order drifted; version the contract")
    for row in scenarios:
        case_id = row["id"]
        require(row.get("expected_outcome") == EXPECTED_OUTCOMES[case_id],
                f"{case_id}: expected disposition cannot be weakened")
        required = row.get("required_assertions")
        require(type(required) is list and set(required) == EXPECTED_ASSERTIONS[case_id],
                f"{case_id}: required assertions missing or changed")
        require(isinstance(row.get("claim_ceiling"), str) and row["claim_ceiling"].strip(),
                f"{case_id}: claim ceiling is required")
    require(scenarios[0].get("arm") == "deliberately_weak_control",
            "LBC-001 must remain the deliberate exposure control")
    require(scenarios[0].get("interpretation") ==
            "This is an exposure-control pass, not a security-containment pass.",
            "weak-control exposure cannot be reported as containment")
    require(scenarios[1].get("arm") == "mitigation_control",
            "LBC-002 must remain the close-on-exec mitigation control")
    require(scenarios[2].get("arm") == "mitigation_control",
            "LBC-003 must remain the path denial mitigation control")
    require(scenarios[4].get("expected_outcome") == "BLOCKED_INCOMPLETE_EVIDENCE",
            "infrastructure loss must remain inconclusive, not a pass")

    mutations = document.get("mutation_controls")
    require(type(mutations) is list and tuple(
        row.get("id") if isinstance(row, dict) else None for row in mutations
    ) == MUTATION_IDS, "mutation-control inventory/order drifted")
    for row in mutations:
        case_id = row["id"]
        require(row.get("expected_control") == EXPECTED_MUTATIONS[case_id],
                f"{case_id}: mutation oracle cannot be weakened")
        require(isinstance(row.get("mutation"), str) and row["mutation"].strip(),
                f"{case_id}: mutation description is required")

    receipt = document.get("receipt_contract")
    require(isinstance(receipt, dict), "receipt_contract must be an object")
    require(receipt.get("schema") == "luminous.landlock-preopened-fd-result.v1",
            "runtime receipt schema drifted")
    receipt_fields = receipt.get("required_fields")
    require(type(receipt_fields) is list and tuple(receipt_fields) == RECEIPT_FIELDS,
            "runtime receipt field inventory/order drifted")
    receipt_values = receipt.get("expected_values")
    require(isinstance(receipt_values, dict), "receipt expected_values must be an object")
    require(receipt_values.get("status") == "pass",
            "qualified micro-test receipt status must remain pass")
    require(receipt_values.get("full_descriptor_hygiene_coverage") is False,
            "receipt cannot claim complete descriptor hygiene")
    require(receipt_values.get("scope") == "preopened_writable_fd_ordering_microtest",
            "receipt scope cannot be widened")
    identity = receipt.get("identity_requirements")
    require(isinstance(identity, dict), "receipt identity_requirements must be an object")
    require(identity.get("checkout_sha") == "40 lowercase hexadecimal characters",
            "receipt must bind an exact source checkout SHA")
    require(identity.get("kernel_release") == "nonempty sanitized release string",
            "receipt must identify the runtime kernel")
    require(type(identity.get("landlock_abi_minimum")) is int
            and identity["landlock_abi_minimum"] >= 3,
            "receipt must record the supported Landlock ABI floor")
    require(identity.get("both_arms_must_report_same_landlock_abi") is True,
            "both experiment arms must use the same Landlock ABI")

    reporting = document.get("reporting_requirements")
    require(isinstance(reporting, dict), "reporting_requirements must be an object")
    for key in (
        "preserve_expected_exposure_as_exposure_not_success",
        "parent_oracle_is_independent_of_child_stdout",
        "unknown_or_missing_result_is_incomplete",
        "do_not_aggregate_away_forbidden_effect",
        "bind_exact_checkout_sha_in_report",
        "record_kernel_and_landlock_abi",
        "record_all_trials_and_failures",
    ):
        require(reporting.get(key) is True, f"reporting requirement cannot be weakened: {key}")

    ceiling = document.get("claim_ceiling")
    require(isinstance(ceiling, str)
            and "does not establish full descriptor hygiene" in ceiling
            and "host-kernel integrity" in ceiling,
            "global claim ceiling must retain explicit limitations")
    return {"scenarios": len(scenarios), "mutation_controls": len(mutations),
            "source_semantics": len(sources), "receipt_fields": len(receipt_fields)}


def main() -> int:
    path = (Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else
            Path(__file__).resolve().parents[2] / "benchmarks" /
            "agentic-endpoint-security-v1" / "launch-boundary-contract.json")
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
        counts = validate(document)
    except (OSError, UnicodeError, json.JSONDecodeError, ValueError, KeyError) as exc:
        print(f"FAIL launch-boundary contract: {exc}", file=sys.stderr)
        return 1
    print("PASS launch-boundary contract: "
          f"{counts['scenarios']} scenarios, {counts['mutation_controls']} mutation controls, "
          f"{counts['source_semantics']} source-semantic records.")
    print("SCOPE: contract consistency only; no Landlock execution or endpoint protection result.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
