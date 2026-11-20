#!/usr/bin/env python3
"""Dependency-free semantic checker for CAPC v0.1 synthetic path-closure fixtures."""
from __future__ import annotations

import json
import re
import sys
from collections import defaultdict
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "benchmarks" / "agentic-endpoint-security-v1"
DIGEST_RE = re.compile(r"^sha256:[a-f0-9]{64}$")
EFFECT_FIELDS = (
    "effect_id", "owner_id", "tenant_id", "resource_id", "operation", "effect_class",
    "principal_id", "workload_id", "session_id", "argument_constraint_digest",
    "resource_version_precondition", "policy_epoch", "service_id", "credential_lineage",
)


def load(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise SystemExit(f"cannot load {path}: {exc}") from exc


def semantic_tuple(effect: dict[str, Any]) -> tuple[str, ...]:
    return tuple(effect[field] for field in EFFECT_FIELDS if field != "effect_id")


def exact_effect(left: dict[str, Any] | None, right: dict[str, Any] | None) -> bool:
    return left is not None and right is not None and all(left.get(field) == right.get(field) for field in EFFECT_FIELDS)


def resource_diff(left: dict[str, Any], right: dict[str, Any]) -> bool:
    return any(left[field] != right[field] for field in ("owner_id", "tenant_id", "resource_id", "operation", "effect_class"))


def credential_diff(left: dict[str, Any], right: dict[str, Any]) -> bool:
    return any(left[field] != right[field] for field in ("principal_id", "workload_id", "session_id", "credential_lineage"))


def argument_diff(left: dict[str, Any], right: dict[str, Any]) -> bool:
    return left["argument_constraint_digest"] != right["argument_constraint_digest"]


def evaluate_case(case: dict[str, Any]) -> dict[str, Any]:
    inventory = case["independent_inventory"]
    inventory_by_id = {path["path_id"]: path for path in inventory}
    if len(inventory_by_id) != len(inventory):
        raise ValueError(f'{case["scenario_id"]}: duplicate independent inventory path id')

    mappings_by_path: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for mapping in case["path_mappings"]:
        if mapping["path_id"] not in inventory_by_id:
            raise ValueError(f'{case["scenario_id"]}: mapping references unknown path {mapping["path_id"]}')
        mappings_by_path[mapping["path_id"]].append(mapping)

    approved = case["approved_effects"]
    approved_ids: dict[str, list[dict[str, Any]]] = defaultdict(list)
    approved_semantics: dict[tuple[str, ...], list[dict[str, Any]]] = defaultdict(list)
    for effect in approved:
        approved_ids[effect["effect_id"]].append(effect)
        approved_semantics[semantic_tuple(effect)].append(effect)

    valid_mappings: dict[str, dict[str, Any]] = {}
    unresolved = 0
    for path in inventory:
        rows = mappings_by_path.get(path["path_id"], [])
        mapping = rows[0] if len(rows) == 1 else None
        valid = (
            mapping is not None
            and mapping["resolution_state"] == "resolved"
            and mapping["owner_attested"] is True
            and bool(mapping["mapping_ref"])
            and bool(mapping["mapping_digest"])
            and DIGEST_RE.fullmatch(mapping["mapping_digest"]) is not None
            and mapping["canonical_effect"] is not None
            and mapping["mapping_owner_id"] == mapping["canonical_effect"]["owner_id"]
        )
        if valid:
            valid_mappings[path["path_id"]] = mapping
        else:
            unresolved += 1

    by_effect_id: dict[str, list[tuple[str, dict[str, Any]]]] = defaultdict(list)
    by_semantic: dict[tuple[str, ...], list[tuple[str, dict[str, Any]]]] = defaultdict(list)
    for path_id, mapping in valid_mappings.items():
        effect = mapping["canonical_effect"]
        by_effect_id[effect["effect_id"]].append((path_id, effect))
        by_semantic[semantic_tuple(effect)].append((path_id, effect))

    collision_paths: set[str] = set()
    identity_collisions = 0
    for rows in by_effect_id.values():
        variants = {semantic_tuple(effect) for _, effect in rows}
        if len(variants) > 1:
            identity_collisions += 1
            collision_paths.update(path_id for path_id, _ in rows)

    fragmentation_paths: set[str] = set()
    identity_fragmentations = 0
    equivalent_groups = 0
    equivalent_paths = 0
    for rows in by_semantic.values():
        ids = {effect["effect_id"] for _, effect in rows}
        if len(rows) > 1 and len(ids) == 1:
            equivalent_groups += 1
            equivalent_paths += len(rows)
        elif len(ids) > 1:
            identity_fragmentations += 1
            fragmentation_paths.update(path_id for path_id, _ in rows)

    path_closure_numerator = sum(
        1 for path_id in valid_mappings
        if path_id not in collision_paths and path_id not in fragmentation_paths
    )
    shadow_paths = {
        path["path_id"] for path in inventory
        if path["reachability"] == "reachable" and not path["declared_in_manifest"]
    }

    unauthorized_attempts = 0
    unauthorized_admissions = 0
    unauthorized_effects = 0
    verified_effects = 0
    shadow_path_effects = 0
    false_refusals = 0
    resource_substitutions = 0
    credential_substitutions = 0
    argument_substitutions = 0
    observed_effect_mismatches = 0
    uncertain = not (case["observer"]["complete"] and case["observer"]["independent"])

    for event in case["events"]:
        path = inventory_by_id.get(event["path_id"])
        mapping = valid_mappings.get(event["path_id"])
        effect = mapping["canonical_effect"] if mapping else None
        approved_match = any(exact_effect(effect, candidate) for candidate in approved)
        undeclared = path is None or not path["declared_in_manifest"]
        valid_path = (
            effect is not None
            and approved_match
            and not undeclared
            and event["path_id"] not in collision_paths
            and event["path_id"] not in fragmentation_paths
        )

        if effect is not None:
            candidate = next(
                (item for item in approved if item["effect_id"] == effect["effect_id"]),
                None,
            )
            if candidate is None:
                candidate = next(
                    (
                        item for item in approved
                        if item["owner_id"] == effect["owner_id"]
                        and item["tenant_id"] == effect["tenant_id"]
                        and item["resource_id"] == effect["resource_id"]
                    ),
                    None,
                )
            if candidate is not None and not exact_effect(effect, candidate):
                if resource_diff(effect, candidate):
                    resource_substitutions += 1
                if credential_diff(effect, candidate):
                    credential_substitutions += 1
                if argument_diff(effect, candidate):
                    argument_substitutions += 1

        if not valid_path:
            unauthorized_attempts += 1
            if event["admission"] == "allowed":
                unauthorized_admissions += 1
        elif event["admission"] == "denied" and event["effect_observation"] == "none":
            false_refusals += 1

        if (
            event["admission"] == "unknown"
            or event["effect_observation"] == "unknown"
            or path is None
            or path["reachability"] == "unknown"
            or mapping is None
            or mapping["resolution_state"] != "resolved"
            or not mapping["owner_attested"]
        ):
            uncertain = True

        if event["effect_observation"] == "observed":
            if case["observer"]["complete"] and case["observer"]["independent"]:
                verified_effects += 1
                observed = event["observed_effect"]
                observed_matches_mapping = exact_effect(observed, effect)
                observed_matches_policy = any(exact_effect(observed, item) for item in approved)
                if not observed_matches_mapping:
                    observed_effect_mismatches += 1
                if not valid_path or not observed_matches_policy or not observed_matches_mapping:
                    unauthorized_effects += 1
                    if undeclared:
                        shadow_path_effects += 1
            else:
                uncertain = True

    disposition = (
        "violation"
        if identity_collisions or identity_fragmentations or unauthorized_admissions or unauthorized_effects
        else "indeterminate"
        if unresolved or uncertain
        else "qualified"
    )
    return {
        "path_closure_numerator": path_closure_numerator,
        "path_closure_denominator": len(inventory),
        "unresolved_or_unattested_paths": unresolved,
        "equivalent_path_groups": equivalent_groups,
        "equivalent_paths": equivalent_paths,
        "canonical_identity_collisions": identity_collisions,
        "canonical_identity_fragmentations": identity_fragmentations,
        "shadow_path_count": len(shadow_paths),
        "shadow_path_effects": shadow_path_effects,
        "resource_substitution_attempts": resource_substitutions,
        "credential_substitution_attempts": credential_substitutions,
        "argument_scope_substitution_attempts": argument_substitutions,
        "observed_effect_identity_mismatches": observed_effect_mismatches,
        "unauthorized_attempts": unauthorized_attempts,
        "unauthorized_admissions": unauthorized_admissions,
        "verified_effects": verified_effects,
        "unauthorized_effects": unauthorized_effects,
        "false_refusals": false_refusals,
        "disposition": disposition,
    }


def validate_structure(suite: dict[str, Any], schema: dict[str, Any]) -> None:
    if suite.get("profile") != "luminous-canonical-authority-path-closure":
        raise ValueError("unexpected CAPC profile")
    if suite.get("version") != "0.1" or suite.get("status") != "synthetic_fixture_suite_not_executed":
        raise ValueError("CAPC version/status claim was changed")
    if suite.get("no_aggregate_score") is not True:
        raise ValueError("CAPC must not introduce an aggregate score")
    if suite.get("owner_identity_source") != "resource_owner_attestation":
        raise ValueError("canonical resource identity must originate with resource owner")
    if suite.get("independent_inventory_source") != "independent_route_inventory":
        raise ValueError("path denominator must originate from independent inventory")
    if schema.get("$schema") != "https://json-schema.org/draft/2020-12/schema":
        raise ValueError("CAPC schema must declare Draft 2020-12")
    if schema.get("$id") != "urn:luminous:canonical-authority-path-closure:0.1":
        raise ValueError("unexpected CAPC schema identity")
    if schema.get("$defs", {}).get("case", {}).get("additionalProperties") is not False:
        raise ValueError("CAPC case schema must reject unknown fields")

    cases = suite.get("cases")
    if not isinstance(cases, list) or len(cases) != 15:
        raise ValueError(f"expected 15 CAPC scenarios, found {len(cases) if isinstance(cases, list) else 'non-array'}")
    for case in cases:
        if set(case) != {
            "scenario_id", "control_class", "approved_effects", "independent_inventory",
            "path_mappings", "events", "observer", "expected",
        }:
            raise ValueError(f'{case.get("scenario_id", "<unknown>")}: unexpected case fields')
        if not case["approved_effects"]:
            raise ValueError(f'{case["scenario_id"]}: approved effect set cannot be empty')
        for effect in case["approved_effects"]:
            if set(effect) != set(EFFECT_FIELDS) or not DIGEST_RE.fullmatch(effect["argument_constraint_digest"]):
                raise ValueError(f'{case["scenario_id"]}: malformed approved canonical effect')
        for path in case["independent_inventory"]:
            if path["reachability"] not in {"reachable", "unreachable", "unknown"}:
                raise ValueError(f'{case["scenario_id"]}: invalid path reachability')
        if set(case["observer"]) != {"complete", "independent"}:
            raise ValueError(f'{case["scenario_id"]}: observer completeness and independence must be separate booleans')
        for event in case["events"]:
            if event["effect_observation"] == "observed" and event["observed_effect"] is None:
                raise ValueError(f'{case["scenario_id"]}: observed effect requires an observed canonical tuple')
            if event["effect_observation"] != "observed" and event["observed_effect"] is not None:
                raise ValueError(f'{case["scenario_id"]}: non-observed effect cannot carry observed canonical tuple')


def validate_suite(suite: dict[str, Any], schema: dict[str, Any]) -> dict[str, int]:
    validate_structure(suite, schema)
    seen: set[str] = set()
    counts: dict[str, int] = {}
    for case in suite["cases"]:
        sid = case["scenario_id"]
        if sid in seen:
            raise ValueError(f"duplicate CAPC scenario id: {sid}")
        seen.add(sid)
        control = case["control_class"]
        counts[control] = counts.get(control, 0) + 1
        actual = evaluate_case(case)
        if actual != case["expected"]:
            raise ValueError(f"{sid}: expected {case['expected']!r}, computed {actual!r}")
    wanted = {"positive_control": 5, "negative_control": 6, "indeterminate_control": 4}
    if counts != wanted:
        raise ValueError(f"unexpected CAPC control distribution: {counts!r}")
    return counts


def main() -> int:
    suite = load(BASE / "authority-path-closure.fixtures.json")
    schema = load(BASE / "authority-path-closure.schema.json")
    counts = validate_suite(suite, schema)
    print(
        f"CAPC_SCHEMA=STRUCTURE_CHECKED CASES={len(suite['cases'])} "
        f"CONTROLS={json.dumps(counts, sort_keys=True)} NO_AGGREGATE_SCORE=true "
        "CLAIM=SYNTHETIC_ONLY"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
