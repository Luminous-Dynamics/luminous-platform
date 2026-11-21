#!/usr/bin/env python3
"""Dependency-free checker for declared/code/observed tool capability surfaces."""
from __future__ import annotations
import json
import re
import sys
from collections import Counter
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "benchmarks" / "agentic-endpoint-security-v1"
DIGEST_RE = re.compile(r"^sha256:[a-f0-9]{64}$")
COMMIT_RE = re.compile(r"^[a-f0-9]{40}$")
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


def canonical_tuple(effect: dict[str, Any]) -> tuple[str, ...]:
    return tuple(effect[field] for field in EFFECT_FIELDS[1:])


def artifact_binding_complete(binding: dict[str, Any]) -> bool:
    return (
        isinstance(binding.get("source_commit"), str)
        and COMMIT_RE.fullmatch(binding["source_commit"]) is not None
        and all(DIGEST_RE.fullmatch(binding.get(field) or "") for field in (
            "source_tree_digest", "dependency_lock_digest", "binary_digest",
            "runtime_config_digest", "tool_manifest_digest",
        ))
        and isinstance(binding.get("build_provenance_ref"), str)
        and bool(binding["build_provenance_ref"].strip())
    )


def evaluate_case(case: dict[str, Any]) -> dict[str, Any]:
    declared = {effect["effect_id"]: effect for effect in case["declared_effects"]}
    implementation = {
        item["effect"]["effect_id"]: item["effect"]
        for item in case["implementation_effects"]
        if item["effect"] is not None and item["reachability"] in {"reachable", "conditional"}
    }
    observed = {item["effect"]["effect_id"]: item["effect"] for item in case["observed_effects"]}
    d_ids, p_ids, o_ids = set(declared), set(implementation), set(observed)
    potential_undeclared = p_ids - d_ids
    observed_undeclared = o_ids - d_ids
    observed_unmodeled = o_ids - p_ids
    overdeclared = d_ids - p_ids
    binding_complete = artifact_binding_complete(case["artifact_binding"])
    observer_qualified = case["observer"]["complete"] is True and case["observer"]["independent"] is True
    analysis_complete = case["static_analysis"]["complete"] is True
    unresolved_count = sum(item["reachability"] == "unresolved" for item in case["implementation_effects"])
    if observer_qualified and observed_unmodeled:
        disposition = "observed_unmodeled_effect"
    elif observer_qualified and observed_undeclared:
        disposition = "observed_undeclared_effect"
    elif not binding_complete or not analysis_complete or not observer_qualified:
        disposition = "indeterminate"
    elif potential_undeclared:
        disposition = "implementation_expands_declaration"
    elif overdeclared:
        disposition = "declaration_overstates_implementation"
    else:
        disposition = "matched"
    return {
        "declared_effect_count": len(d_ids),
        "implementation_effect_count": len(p_ids),
        "observed_effect_count": len(o_ids),
        "potential_undeclared_effect_count": len(potential_undeclared),
        "observed_undeclared_effect_count": len(observed_undeclared),
        "observed_unmodeled_effect_count": len(observed_unmodeled),
        "declaration_overstatement_count": len(overdeclared),
        "unresolved_implementation_path_count": unresolved_count,
        "artifact_binding_complete": binding_complete,
        "observer_qualified": observer_qualified,
        "disposition": disposition,
    }


def validate_case(case: dict[str, Any]) -> dict[str, Any]:
    sid = case.get("scenario_id", "<unknown>")
    if set(case) != {
        "scenario_id", "control_class", "artifact_binding", "declared_effects",
        "implementation_effects", "static_analysis", "observed_effects", "observer", "expected",
    }:
        raise ValueError(f"{sid}: unexpected case fields")
    binding = case["artifact_binding"]
    if set(binding) != {
        "tool_name", "human_version_label", "comparison_group_id", "variant_id",
        "source_commit", "source_tree_digest", "dependency_lock_digest", "build_provenance_ref",
        "binary_digest", "runtime_config_digest", "tool_manifest_digest",
    }:
        raise ValueError(f"{sid}: artifact-binding fields differ from contract")
    analysis = case["static_analysis"]
    if set(analysis) != {"analyser_id", "analyser_version", "analysis_manifest_digest", "complete", "unresolved_constructs"}:
        raise ValueError(f"{sid}: analysis fields differ from contract")
    if not DIGEST_RE.fullmatch(analysis["analysis_manifest_digest"]):
        raise ValueError(f"{sid}: malformed analysis manifest digest")
    has_unresolved = bool(analysis["unresolved_constructs"]) or any(
        item["reachability"] == "unresolved" for item in case["implementation_effects"]
    )
    if analysis["complete"] and has_unresolved:
        raise ValueError(f"{sid}: complete analysis cannot contain unresolved paths")
    all_effects = list(case["declared_effects"])
    all_effects.extend(item["effect"] for item in case["implementation_effects"] if item["effect"] is not None)
    all_effects.extend(item["effect"] for item in case["observed_effects"])
    id_to_tuple: dict[str, tuple[str, ...]] = {}
    tuple_to_id: dict[tuple[str, ...], str] = {}
    for effect in all_effects:
        if set(effect) != set(EFFECT_FIELDS) or not DIGEST_RE.fullmatch(effect["argument_constraint_digest"]):
            raise ValueError(f"{sid}: malformed CAPC canonical effect tuple")
        semantic = canonical_tuple(effect)
        if effect["effect_id"] in id_to_tuple and id_to_tuple[effect["effect_id"]] != semantic:
            raise ValueError(f"{sid}: canonical effect ID collision")
        if semantic in tuple_to_id and tuple_to_id[semantic] != effect["effect_id"]:
            raise ValueError(f"{sid}: canonical effect identity fragmentation")
        id_to_tuple[effect["effect_id"]] = semantic
        tuple_to_id[semantic] = effect["effect_id"]
    seen_paths: set[str] = set()
    for item in case["implementation_effects"]:
        if set(item) != {"effect", "reachability", "path_ref", "evidence_ref"}:
            raise ValueError(f"{sid}: malformed implementation path")
        if item["reachability"] not in {"reachable", "conditional", "unresolved"}:
            raise ValueError(f"{sid}: invalid reachability class")
        if not item["path_ref"] or not item["evidence_ref"] or item["path_ref"] in seen_paths:
            raise ValueError(f"{sid}: duplicate or unsupported implementation path identity")
        seen_paths.add(item["path_ref"])
        if item["reachability"] == "unresolved" and item["effect"] is not None:
            raise ValueError(f"{sid}: unresolved path cannot assert an exact canonical effect")
        if item["reachability"] != "unresolved" and item["effect"] is None:
            raise ValueError(f"{sid}: resolved path must bind a canonical effect")
    for item in case["observed_effects"]:
        if set(item) != {"effect", "evidence_ref"} or not item["evidence_ref"]:
            raise ValueError(f"{sid}: observed effect needs evidence reference")
    if set(case["observer"]) != {"complete", "independent"}:
        raise ValueError(f"{sid}: observer completeness and independence must remain distinct")
    expected = evaluate_case(case)
    if case["expected"] != expected:
        raise ValueError(f"{sid}: expected {case['expected']!r}, computed {expected!r}")
    return expected


def validate_suite(suite: dict[str, Any], schema: dict[str, Any]) -> dict[str, int]:
    if suite.get("profile") != "luminous-tool-capability-realization" or suite.get("version") != "0.1":
        raise ValueError("unexpected CARI profile or version")
    if suite.get("status") != "synthetic_fixture_suite_not_executed" or suite.get("no_aggregate_score") is not True:
        raise ValueError("CARI claim ceiling or non-aggregate invariant changed")
    if suite.get("canonical_effect_owner") != "CAPC-v0.1":
        raise ValueError("CARI must consume CAPC canonical effect identities")
    if schema.get("$schema") != "https://json-schema.org/draft/2020-12/schema" or schema.get("$id") != "urn:luminous:tool-capability-realization:0.1":
        raise ValueError("unexpected CARI schema identity")
    cases = suite.get("cases")
    if not isinstance(cases, list) or len(cases) != 12:
        raise ValueError(f"expected 12 CARI scenarios, found {len(cases) if isinstance(cases, list) else 'non-array'}")
    seen: set[str] = set()
    counts: Counter[str] = Counter()
    labels_by_group: dict[str, set[str]] = {}
    variants: dict[str, tuple[str | None, str | None]] = {}
    for case in cases:
        sid = case.get("scenario_id")
        if sid in seen:
            raise ValueError(f"duplicate CARI scenario id: {sid}")
        seen.add(sid)
        counts[case["control_class"]] += 1
        validate_case(case)
        binding = case["artifact_binding"]
        labels_by_group.setdefault(binding["comparison_group_id"], set()).add(binding["human_version_label"])
        variants[binding["variant_id"]] = (binding["source_commit"], binding["binary_digest"])
    if labels_by_group.get("capability-study") != {"1.0.0"}:
        raise ValueError("the same-friendly-label comparison is missing")
    if variants.get("build-a") is None or variants.get("build-b") is None or variants["build-a"] == variants["build-b"]:
        raise ValueError("same friendly version label with distinct source/build identities must remain distinct")
    expected_counts = {"positive_control": 3, "negative_control": 5, "indeterminate_control": 4}
    if dict(counts) != expected_counts:
        raise ValueError(f"unexpected CARI control distribution: {dict(counts)!r}")
    return dict(counts)


def main() -> int:
    suite = load(BASE / "tool-capability-realization.fixtures.json")
    schema = load(BASE / "tool-capability-realization.schema.json")
    counts = validate_suite(suite, schema)
    print(f"CARI_SCHEMA=STRUCTURE_CHECKED CASES=12 CONTROLS={json.dumps(counts, sort_keys=True)} NO_AGGREGATE_SCORE=true CLAIM=SYNTHETIC_ONLY")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
