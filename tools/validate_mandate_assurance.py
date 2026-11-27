#!/usr/bin/env python3
"""Synthetic mandate-assurance corpus checker; not a cryptographic verifier."""
from __future__ import annotations

import copy
import json
from pathlib import Path
from jsonschema import Draft202012Validator

ROOT = Path("research/agentic-threat-resilience-v0.1")
SCHEMA_PATH = ROOT / "schemas/mandate-assurance-corpus-v0.1.schema.json"
CORPUS_PATH = ROOT / "scenarios/mandate-assurance-corpus-v0.1.json"


def party_equal(left: dict, right: dict) -> bool:
    return left.get("id") == right.get("id") and left.get("scheme") == right.get("scheme")


def status_result(status: str, revoked_code: str, unresolved_code: str, invalid: set[str], indeterminate: set[str]) -> None:
    if status == "revoked":
        invalid.add(revoked_code)
    elif status in ("stale", "unknown"):
        indeterminate.add(unresolved_code)


def validation_result(case: dict) -> tuple[str, list[str]]:
    invalid: set[str] = set()
    indeterminate: set[str] = set()
    chain = case["chain"]
    request = case["request"]
    authorities = {item["id"]: item for item in case["authority_assertions"]}

    ids = [item["id"] for item in chain]
    if len(ids) != len(set(ids)):
        invalid.add("duplicate_mandate_id")
    root = chain[0]
    if root["parent_id"] is not None:
        invalid.add("root_parent_mismatch")
    if root["issuer_role"] != "mandate_issuer":
        invalid.add("mandate_issuer_role_mismatch")

    # Cryptographic result fields are verifier outputs in a synthetic fixture.
    for index, mandate in enumerate(chain):
        if mandate["signature_state"] == "invalid":
            invalid.add("mandate_signature_invalid")
        elif mandate["signature_state"] == "unknown":
            indeterminate.add("mandate_signature_unresolved")
        status_result(mandate["status"], "mandate_revoked", "mandate_status_unresolved", invalid, indeterminate)
        if request["current_time"] < mandate["not_before"]:
            invalid.add("mandate_not_yet_valid")
        if request["current_time"] >= mandate["expires_at"]:
            invalid.add("mandate_expired")
        if index == 0:
            continue
        parent = chain[index - 1]
        if mandate["parent_id"] != parent["id"]:
            invalid.add("parent_link_mismatch")
        if mandate["issuer"] != parent["subject"] or mandate["signing_key_jkt"] != parent["cnf_jkt"]:
            invalid.add("subdelegation_presenter_mismatch")
        if mandate["issuer_role"] != "holder_key":
            invalid.add("subdelegation_issuer_role_mismatch")
        if not party_equal(mandate["principal"], parent["principal"]):
            invalid.add("principal_chain_mismatch")
        if mandate["delegator"] != {"id": parent["subject"], "scheme": "urn:luminous:agent"}:
            invalid.add("subdelegator_mismatch")
        if not set(mandate["permissions"]).issubset(set(parent["permissions"])):
            invalid.add("delegation_not_attenuated")
        if not set(mandate["audiences"]).issubset(set(parent["audiences"])):
            invalid.add("audience_not_attenuated")
        if mandate["expires_at"] > parent["expires_at"]:
            invalid.add("expiry_not_attenuated")
        if mandate["max_depth"] > max(parent["max_depth"] - 1, 0):
            invalid.add("depth_not_attenuated")

    # Authority evidence is a separate, root-only proof. One supporting assertion
    # is sufficient; absent/invalid alternatives do not widen the accepted scope.
    supporting = False
    unresolved = False
    referenced = root["authority_refs"]
    matching_records = [authorities[ref] for ref in referenced if ref in authorities]
    if not matching_records:
        invalid.add("authority_missing")
    else:
        identity_records = [
            item for item in matching_records
            if party_equal(item["principal"], root["principal"])
            and party_equal(item["delegator"], root["delegator"])
        ]
        if not identity_records:
            invalid.add("authority_party_mismatch")
        for authority in identity_records:
            if authority["issuer_role"] != "authority_source":
                invalid.add("authority_role_mismatch")
                continue
            if authority["signature_state"] == "invalid":
                invalid.add("authority_signature_invalid")
                continue
            if authority["signature_state"] == "unknown":
                unresolved = True
            if authority["status"] == "revoked":
                invalid.add("authority_revoked")
                continue
            if authority["status"] in ("stale", "unknown"):
                unresolved = True
            if request["current_time"] < authority["not_before"] or request["current_time"] >= authority["expires_at"]:
                invalid.add("authority_outside_validity")
                continue
            if not set(root["permissions"]).issubset(set(authority["powers"])):
                continue
            if root["max_depth"] > authority["max_depth"]:
                continue
            if authority["signature_state"] == "valid" and authority["status"] == "active":
                supporting = True
        if not supporting:
            if unresolved and not invalid.intersection({
                "authority_role_mismatch", "authority_signature_invalid", "authority_revoked",
                "authority_outside_validity", "authority_party_mismatch"
            }):
                indeterminate.add("authority_status_unresolved")
            elif "authority_role_mismatch" not in invalid and "authority_signature_invalid" not in invalid:
                invalid.add("authority_insufficient")

    if request["pop_state"] != "valid":
        invalid.add("pop_invalid")
    leaf = chain[-1]
    if request["presenter"] != leaf["subject"] or request["presenter_jkt"] != leaf["cnf_jkt"]:
        invalid.add("pop_binding_mismatch")
    if request["audience"] != request["relying_party"] or request["relying_party"] not in leaf["audiences"]:
        invalid.add("audience_mismatch")
    if request["requested_scope"] not in leaf["permissions"]:
        invalid.add("requested_scope_not_granted")

    codes = sorted(invalid | indeterminate)
    if invalid:
        return "invalid", codes
    if indeterminate:
        return "indeterminate", codes
    return "valid", codes


def authorization_decision(validation: str, case: dict) -> str:
    request = case["request"]
    if validation != "valid":
        return "deny"
    if not request["principal_entitled"]:
        return "deny"
    if request["rp_policy"] != "allow":
        return "deny"
    if not request["obligations_discharged"]:
        return "deny"
    return "allow"


def run_mutation_controls(cases: list[dict]) -> int:
    """Exercise single-field adversarial mutations without trusting fixture labels."""
    by_id = {case["case_id"]: case for case in cases}
    total = 0

    def cloned(case_id: str) -> dict:
        return copy.deepcopy(by_id[case_id])

    def expect(case: dict, label: str, validation: str, authorization: str) -> None:
        nonlocal total
        actual_validation, _ = validation_result(case)
        actual_authorization = authorization_decision(actual_validation, case)
        if (actual_validation, actual_authorization) != (validation, authorization):
            raise SystemExit(
                f"mutation {label}: expected {(validation, authorization)} "
                f"got {(actual_validation, actual_authorization)}"
            )
        total += 1

    mutations = []
    case = cloned("MND-001")
    case["authority_assertions"][0]["powers"] = ["payments:read@other"]
    mutations.append(("authority-scope-shrink", case, "invalid", "deny"))

    case = cloned("MND-001")
    case["authority_assertions"][0]["issuer_role"] = "mandate_issuer"
    mutations.append(("authority-role-confusion", case, "invalid", "deny"))

    case = cloned("MND-001")
    case["authority_assertions"][0]["signature_state"] = "invalid"
    mutations.append(("authority-signature-invalid", case, "invalid", "deny"))

    case = cloned("MND-001")
    case["authority_assertions"][0]["status"] = "revoked"
    mutations.append(("authority-revoked", case, "invalid", "deny"))

    case = cloned("MND-001")
    case["authority_assertions"][0]["status"] = "stale"
    mutations.append(("authority-status-stale", case, "indeterminate", "deny"))

    case = cloned("MND-001")
    case["chain"][0]["authority_refs"] = []
    mutations.append(("authority-reference-removed", case, "invalid", "deny"))

    case = cloned("MND-001")
    case["chain"][0]["issuer_role"] = "authority_source"
    mutations.append(("mandate-issuer-role-confusion", case, "invalid", "deny"))

    case = cloned("MND-001")
    case["chain"][0]["signature_state"] = "invalid"
    mutations.append(("mandate-signature-invalid", case, "invalid", "deny"))

    case = cloned("MND-001")
    case["chain"][0]["status"] = "revoked"
    mutations.append(("mandate-revoked", case, "invalid", "deny"))

    case = cloned("MND-001")
    case["chain"][0]["expires_at"] = case["request"]["current_time"]
    mutations.append(("mandate-expired-at-boundary", case, "invalid", "deny"))

    case = cloned("MND-001")
    case["request"]["pop_state"] = "replayed"
    mutations.append(("proof-replay", case, "invalid", "deny"))

    case = cloned("MND-001")
    case["request"]["presenter_jkt"] = "attacker-key"
    mutations.append(("presenter-key-substitution", case, "invalid", "deny"))

    case = cloned("MND-001")
    case["request"]["presenter"] = "different-agent"
    mutations.append(("presenter-subject-substitution", case, "invalid", "deny"))

    case = cloned("MND-001")
    case["request"]["audience"] = "rp-payroll"
    mutations.append(("audience-substitution", case, "invalid", "deny"))

    case = cloned("MND-001")
    case["request"]["requested_scope"] = "payments:write@invoice-42"
    mutations.append(("operation-scope-expansion", case, "invalid", "deny"))

    case = cloned("MND-001")
    case["request"]["principal_entitled"] = False
    mutations.append(("rp-principal-entitlement-denied", case, "valid", "deny"))

    case = cloned("MND-001")
    case["request"]["rp_policy"] = "deny"
    mutations.append(("rp-policy-denied", case, "valid", "deny"))

    case = cloned("MND-001")
    case["request"]["obligations_discharged"] = False
    mutations.append(("rp-obligation-pending", case, "valid", "deny"))

    case = cloned("MND-011")
    case["chain"][1]["signing_key_jkt"] = "substituted-parent-key"
    mutations.append(("child-issuer-key-substitution", case, "invalid", "deny"))

    case = cloned("MND-011")
    case["chain"][1]["permissions"].append("payments:write@invoice-42")
    mutations.append(("child-permission-widening", case, "invalid", "deny"))

    case = cloned("MND-011")
    case["chain"][1]["expires_at"] = case["chain"][0]["expires_at"] + 1
    mutations.append(("child-expiry-widening", case, "invalid", "deny"))

    case = cloned("MND-011")
    case["chain"][2]["max_depth"] = 1
    mutations.append(("child-depth-widening", case, "invalid", "deny"))

    case = cloned("MND-011")
    case["chain"][2]["principal"]["id"] = "different-principal"
    mutations.append(("principal-lineage-substitution", case, "invalid", "deny"))

    case = cloned("MND-011")
    case["chain"][2]["parent_id"] = "unrelated-parent"
    mutations.append(("parent-mandate-substitution", case, "invalid", "deny"))

    for label, case, validation, authorization in mutations:
        expect(case, label, validation, authorization)

    return total


def main() -> None:
    schema = json.loads(SCHEMA_PATH.read_text())
    corpus = json.loads(CORPUS_PATH.read_text())
    Draft202012Validator.check_schema(schema)
    errors = sorted(Draft202012Validator(schema).iter_errors(corpus), key=lambda error: list(error.absolute_path))
    if errors:
        for error in errors:
            location = "/".join(str(part) for part in error.absolute_path)
            print(f"schema {location}: {error.message}")
        raise SystemExit(f"{len(errors)} schema error(s)")
    cases = corpus["cases"]
    ids = [case["case_id"] for case in cases]
    if len(ids) != len(set(ids)):
        raise SystemExit("duplicate case IDs")
    if len(cases) != 12:
        raise SystemExit(f"expected 12 cases, got {len(cases)}")

    for case in cases:
        actual_validation, actual_codes = validation_result(case)
        actual_authorization = authorization_decision(actual_validation, case)
        expected = case["expected"]
        if actual_validation != expected["validation"]:
            raise SystemExit(f"{case['case_id']}: validation expected {expected['validation']} got {actual_validation}")
        if actual_authorization != expected["authorization"]:
            raise SystemExit(f"{case['case_id']}: authorization expected {expected['authorization']} got {actual_authorization}")
        if actual_codes != sorted(expected["codes"]):
            raise SystemExit(f"{case['case_id']}: codes expected {sorted(expected['codes'])} got {actual_codes}")

    # Ensure the corpus contains both directions of the core non-substitution
    # property: valid mandate but RP denial, and invalid mandate with denial.
    by_id = {case["case_id"]: case for case in cases}
    if by_id["MND-009"]["expected"]["validation"] != "valid" or by_id["MND-009"]["expected"]["authorization"] != "deny":
        raise SystemExit("principal entitlement must remain an RP-only predicate")
    if by_id["MND-010"]["expected"]["validation"] != "valid" or by_id["MND-010"]["expected"]["authorization"] != "deny":
        raise SystemExit("execution obligations must remain an RP-only predicate")
    if not any(case["expected"]["validation"] == "indeterminate" for case in cases):
        raise SystemExit("corpus must include a fail-closed indeterminate control")

    mutation_count = run_mutation_controls(cases)
    print(
        f"mandate assurance corpus valid: {len(cases)}/{len(cases)} expected "
        f"validation/RP outcomes; {mutation_count} adversarial mutations rejected "
        f"or correctly separated; synthetic inputs only"
    )


if __name__ == "__main__":
    main()
