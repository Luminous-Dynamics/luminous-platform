#!/usr/bin/env python3
"""Validation-only guard for the external benchmark evidence registry.

The benchmark runtime remains Rust. This script checks the public JSON Schema
and cross-field constraints that a JSON Schema cannot conveniently express.
"""
from __future__ import annotations

import copy
import json
import sys
from datetime import date
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker

ROOT = Path(__file__).resolve().parent
SCHEMA_PATH = ROOT / "registry.schema.json"
SNAPSHOT_PATH = ROOT / "registry.snapshot-2026-11-08.json"
FORBIDDEN_METRICS = {
    "overall_sre_score",
    "overall_agent_score",
    "overall_governance_score",
    "composite_sre_score",
    "translated_sre_score",
}


def read_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise SystemExit(f"cannot load {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise SystemExit(f"{path} must contain a JSON object")
    return value


def semantic_errors(snapshot: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    snapshot_date = date.fromisoformat(snapshot["snapshot_date"])
    record_ids: set[str] = set()

    for record in snapshot["records"]:
        rid = record["id"]
        if rid in record_ids:
            errors.append(f"{rid}: duplicate record id")
        record_ids.add(rid)

        publication = record["publication"]
        observed = date.fromisoformat(publication["observed_at"])
        published_at = publication["published_at"]
        if observed > snapshot_date:
            errors.append(f"{rid}: observation date after snapshot date")
        if published_at is not None:
            published = date.fromisoformat(published_at)
            if published > observed or published > snapshot_date:
                errors.append(f"{rid}: publication date after observation/snapshot date")

        source = record["source"]
        capture = source["capture"]
        state = capture["status"]
        content_hash = capture["content_sha256"]
        blob_sha = source["raw_git_blob_sha"]
        if not source["reference_id"].strip():
            errors.append(f"{rid}: source reference identity is empty")
        if state == "metadata_only_no_content_hash" and content_hash is not None:
            errors.append(f"{rid}: metadata-only source claims a content hash")
        if state == "raw_git_blob_sha_only":
            if not blob_sha or content_hash is not None:
                errors.append(f"{rid}: Git-blob-only status needs blob SHA and no content SHA")
        if state == "content_sha256_verified" and content_hash is None:
            errors.append(f"{rid}: verified content capture has no SHA-256")

        metric_ids: set[str] = set()
        for metric in record["metrics"]:
            mid = metric["metric_id"]
            if mid in metric_ids:
                errors.append(f"{rid}: duplicate metric id {mid}")
            metric_ids.add(mid)
            if mid in FORBIDDEN_METRICS or "composite" in mid or mid.startswith("overall-"):
                errors.append(f"{rid}/{mid}: composite or translated aggregate is forbidden")

            if metric["value_semantics"] != "derived_from_reported_counts":
                if metric["value_semantics"] == "owner_claim" and metric["value"] is None:
                    errors.append(f"{rid}/{mid}: owner claim has no value")
                continue

            numerator = metric.get("numerator")
            denominator = metric.get("denominator")
            if numerator is None or denominator is None or denominator <= 0:
                errors.append(f"{rid}/{mid}: derived count metric requires a positive denominator")
                continue
            if numerator > denominator:
                errors.append(f"{rid}/{mid}: numerator exceeds denominator")
                continue

            unit = metric["unit"]
            if unit == "percent":
                expected = numerator / denominator * 100.0
                tolerance = 0.11
            elif unit == "fraction":
                expected = numerator / denominator
                tolerance = 0.0011
            else:
                continue
            value = metric["value"]
            if not isinstance(value, (int, float)) or abs(float(value) - expected) > tolerance:
                errors.append(
                    f"{rid}/{mid}: value {value!r} does not match "
                    f"{numerator}/{denominator} ({expected:.5f})"
                )

    return errors


def validate(snapshot: dict[str, Any], schema: dict[str, Any]) -> list[str]:
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema, format_checker=FormatChecker())
    errors = [
        f"{'/'.join(str(p) for p in err.absolute_path) or '$'}: {err.message}"
        for err in sorted(
            validator.iter_errors(snapshot),
            key=lambda item: list(map(str, item.absolute_path)),
        )
    ]
    if errors:
        return errors
    return semantic_errors(snapshot)


def expect_rejected(name: str, candidate: dict[str, Any], schema: dict[str, Any]) -> None:
    if not validate(candidate, schema):
        raise SystemExit(f"negative control unexpectedly passed: {name}")
    print(f"rejected as expected: {name}")



NATIVE_SREGYM_SCHEMA_PATH = ROOT / "native-sregym-replication.schema.json"
NATIVE_SREGYM_TEMPLATE_PATH = ROOT / "sregym-0508-luminous-replication.template.json"


def native_sregym_semantic_errors(manifest: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    cohort = manifest["cohort"]
    expected = cohort["scenario_count"] * cohort["attempts_per_scenario"]
    if cohort["expected_attempt_rows"] != expected:
        errors.append("cohort: expected_attempt_rows must equal scenario_count × attempts_per_scenario")
    if cohort["cohort_id"] == "SREGym-0508":
        if cohort["scenario_count"] != 90 or cohort["attempts_per_scenario"] != 3:
            errors.append("SREGym-0508: this replication profile is pinned to 90 scenarios × 3 attempts")
    elif cohort["cohort_id"] == "SREGym-Lite-1004":
        if cohort["scenario_count"] != 17:
            errors.append("SREGym-Lite-1004: the captured Lite cohort has 17 scenarios; do not substitute the full cohort")

    inventory = cohort["inventory_source"]
    capture_status = inventory["capture_status"]
    if capture_status == "metadata_only" and (
        inventory["git_blob_sha"] is not None or inventory["content_sha256"] is not None
    ):
        errors.append("cohort.inventory_source: metadata_only cannot claim captured bytes")
    if capture_status == "git_blob_only" and (
        inventory["git_blob_sha"] is None or inventory["content_sha256"] is not None
    ):
        errors.append("cohort.inventory_source: git_blob_only requires a Git blob SHA and no content SHA-256")
    if capture_status == "content_sha256_verified" and inventory["content_sha256"] is None:
        errors.append("cohort.inventory_source: content_sha256_verified requires the raw content SHA-256")

    accounting = manifest["accounting"]
    if accounting["expected_attempt_rows"] != cohort["expected_attempt_rows"]:
        errors.append("accounting: expected_attempt_rows must match the cohort denominator")
    if accounting["completed_attempt_rows"] + accounting["missing_attempt_rows"] != accounting["expected_attempt_rows"]:
        errors.append("accounting: completed + missing attempt rows must equal the expected denominator")
    if accounting["started_attempt_rows"] > accounting["expected_attempt_rows"] + accounting["retried_attempt_rows"]:
        errors.append("accounting: started attempts cannot exceed expected attempts plus declared retries")
    if accounting["dropped_attempt_rows"] > accounting["missing_attempt_rows"]:
        errors.append("accounting: dropped rows must be included in the missing-row count")
    if manifest["native_protocol"]["retry_policy"] == "no_retry" and accounting["retried_attempt_rows"] != 0:
        errors.append("accounting: no_retry protocol cannot contain retried attempts")
    if manifest["profile_status"] == "executed":
        if accounting["completed_attempt_rows"] != accounting["expected_attempt_rows"]:
            errors.append("executed: every expected attempt needs a retained terminal result row")
        if accounting["missing_attempt_rows"] != 0 or accounting["dropped_attempt_rows"] != 0:
            errors.append("executed: missing or dropped attempt rows prohibit a complete run claim")
        if accounting["attempt_receipt_bundle_digest"] is None or accounting["result_artifact_digest"] is None:
            errors.append("executed: exact attempt receipt and result artifact digests are required")

    protocol = manifest["native_protocol"]
    design = manifest["study_design"]
    arms = manifest["arms"]
    arm_ids = [arm["arm_id"] for arm in arms]
    if len(set(arm_ids)) != len(arm_ids):
        errors.append("arms: arm_id values must be unique")
    if not protocol["native_tool_surface_unchanged"]:
        errors.append("native_protocol: the official native reference arm/tool surface must remain unchanged")

    if design["comparison_mode"] == "native_replication":
        if len(arms) != 1:
            errors.append("native_replication: use exactly one candidate arm; multi-arm ablations belong in a separate manifest")
        if any(arm["luminous_supplemental_gates_enabled"] for arm in arms):
            errors.append("native_replication: Luminous supplemental gates cannot alter the native task/scorer arm")
    elif design["comparison_mode"] == "matched_tool_surface_ablation":
        if len(arms) < 2:
            errors.append("matched_tool_surface_ablation: at least two arms are required")
        if sum(arm["arm_kind"] == "native_reference" for arm in arms) != 1:
            errors.append("matched_tool_surface_ablation: exactly one native_reference arm is required")
        common_fields = {
            "model_snapshot_digest": design["common_model_snapshot_digest"],
            "agent_scaffold_digest": design["common_agent_scaffold_digest"],
            "prompt_digest": design["common_prompt_digest"],
            "permission_profile_digest": design["common_permission_profile_digest"],
            "reasoning_effort": design["common_reasoning_effort"],
            "sampling_policy": design["common_sampling_policy"],
        }
        if any(value is None for value in common_fields.values()):
            errors.append("matched_tool_surface_ablation: model/scaffold/prompt/permission/effort/sampling common conditions must be pinned")
        if design["capability_equivalence_artifact_digest"] is None:
            errors.append("matched_tool_surface_ablation: a capability-equivalence artifact is required")
        for arm in arms:
            for field, value in common_fields.items():
                if value is not None and arm[field] != value:
                    errors.append(f"arms/{arm['arm_id']}: {field} differs from the preregistered common condition")
            if arm["arm_kind"] == "native_reference" and arm["luminous_supplemental_gates_enabled"]:
                errors.append("native_reference arm cannot enable Luminous supplemental gates")
            if arm["luminous_supplemental_gates_enabled"] and arm["arm_kind"] != "luminous_evidence_gateway":
                errors.append(f"arms/{arm['arm_id']}: only the Luminous evidence-gateway arm may enable supplemental gates")

    ready_status = manifest["profile_status"] in {"ready_to_run", "executed"}
    if ready_status:
        pins = manifest["source_pins"]
        if any(pins[key] is None for key in [
            "sregym_commit_sha", "scenario_manifest_blob_sha", "scorer_path",
            "scorer_blob_sha", "submodules_digest", "container_lock_digest",
            "fault_injector_digest"
        ]):
            errors.append("ready/executed: exact SREGym commit, scenario blob, scorer, submodule, container and injector pins are mandatory")
        if protocol["native_scorer_revision"] is None or protocol["native_scorer_digest"] is None:
            errors.append("ready/executed: native scorer revision and digest are mandatory")
        environment = manifest["execution_environment"]
        if any(environment[key] is None for key in [
            "cluster_provider", "kubernetes_version", "node_image_digest",
            "helm_lock_digest", "runtime_lock_digest", "reset_protocol_digest",
            "network_profile_digest"
        ]) or not environment["application_images"]:
            errors.append("ready/executed: every execution-environment artifact must be pinned")
        if any(arm[key] is None for arm in arms for key in [
            "model_provider", "model_snapshot_digest", "agent_scaffold_digest",
            "prompt_digest", "reasoning_effort", "sampling_policy",
            "permission_profile_digest", "tool_surface_digest"
        ]):
            errors.append("ready/executed: each arm needs exact model, agent, prompt, permission and tool-surface identities")
        if design["preregistered_protocol_digest"] is None:
            errors.append("ready/executed: preregistered protocol digest is mandatory")

    supplemental = manifest["supplemental_metrics"]
    if supplemental["enabled"] and not supplemental["metrics"]:
        errors.append("supplemental_metrics: enabled requires at least one separately named metric")
    if not supplemental["enabled"] and supplemental["metrics"]:
        errors.append("supplemental_metrics: disabled cannot publish supplemental metric names")

    return errors


def validate_native_sregym(manifest: dict[str, Any], schema: dict[str, Any]) -> list[str]:
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema, format_checker=FormatChecker())
    errors = [
        f"{'/'.join(str(p) for p in err.absolute_path) or '$'}: {err.message}"
        for err in sorted(
            validator.iter_errors(manifest),
            key=lambda item: list(map(str, item.absolute_path)),
        )
    ]
    if errors:
        return errors
    return native_sregym_semantic_errors(manifest)


def expect_native_rejected(
    name: str, candidate: dict[str, Any], schema: dict[str, Any]
) -> None:
    if not validate_native_sregym(candidate, schema):
        raise SystemExit(f"native SREGym negative control unexpectedly passed: {name}")
    print(f"rejected as expected: native SREGym {name}")


def validate_native_sregym_profile() -> None:
    schema = read_json(NATIVE_SREGYM_SCHEMA_PATH)
    manifest = read_json(NATIVE_SREGYM_TEMPLATE_PATH)
    errors = validate_native_sregym(manifest, schema)
    if errors:
        for error in errors:
            print(f"ERROR: native SREGym manifest: {error}", file=sys.stderr)
        raise SystemExit(1)
    print(
        f"valid native SREGym planning manifest: {manifest['cohort']['cohort_id']} "
        f"{manifest['cohort']['scenario_count']} scenarios × "
        f"{manifest['cohort']['attempts_per_scenario']} attempts; "
        f"{manifest['accounting']['expected_attempt_rows']} expected attempt rows"
    )

    wrong_cohort = copy.deepcopy(manifest)
    wrong_cohort["cohort"]["cohort_id"] = "SREGym-Lite-1004"
    expect_native_rejected("Lite/full cohort substitution", wrong_cohort, schema)

    wrong_denominator = copy.deepcopy(manifest)
    wrong_denominator["cohort"]["expected_attempt_rows"] = 90
    wrong_denominator["accounting"]["expected_attempt_rows"] = 90
    expect_native_rejected("incorrect attempt denominator", wrong_denominator, schema)

    contaminated_native = copy.deepcopy(manifest)
    contaminated_native["arms"][0]["luminous_supplemental_gates_enabled"] = True
    expect_native_rejected("native arm contaminated by supplemental gate", contaminated_native, schema)

    dropped_rows = copy.deepcopy(manifest)
    dropped_rows["accounting"]["dropped_attempt_rows"] = 1
    expect_native_rejected("dropped attempts not represented as missing", dropped_rows, schema)

    falsely_ready = copy.deepcopy(manifest)
    falsely_ready["profile_status"] = "ready_to_run"
    expect_native_rejected("ready-to-run without pinned scorer/environment", falsely_ready, schema)

    executed_missing = copy.deepcopy(manifest)
    executed_missing["profile_status"] = "executed"
    executed_missing["accounting"]["completed_attempt_rows"] = 269
    executed_missing["accounting"]["missing_attempt_rows"] = 1
    executed_missing["accounting"]["attempt_receipt_bundle_digest"] = "a" * 64
    executed_missing["accounting"]["result_artifact_digest"] = "b" * 64
    expect_native_rejected("executed run with a missing attempt", executed_missing, schema)

def main() -> int:
    validate_native_sregym_profile()
    schema = read_json(SCHEMA_PATH)
    snapshot = read_json(SNAPSHOT_PATH)

    errors = validate(snapshot, schema)
    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 1

    print(
        f"valid registry: {len(snapshot['records'])} records; "
        f"snapshot {snapshot['snapshot_date']}"
    )

    missing_source = copy.deepcopy(snapshot)
    del missing_source["records"][0]["source"]["reference_id"]
    expect_rejected("missing source identity", missing_source, schema)

    missing_denominator = copy.deepcopy(snapshot)
    del missing_denominator["records"][0]["metrics"][0]["denominator"]
    expect_rejected("missing denominator", missing_denominator, schema)

    unknown_semantics = copy.deepcopy(snapshot)
    unknown_semantics["records"][0]["metrics"][0]["value_semantics"] = "unknown_metric"
    expect_rejected("unknown metric semantics", unknown_semantics, schema)

    bad_date_order = copy.deepcopy(snapshot)
    bad_date_order["records"][0]["publication"]["published_at"] = "2026-11-08"
    expect_rejected("publication date after observation/snapshot", bad_date_order, schema)

    composite = copy.deepcopy(snapshot)
    composite["records"][0]["metrics"][0]["metric_id"] = "overall_sre_score"
    expect_rejected("composite or translated score injection", composite, schema)

    print("all registry validation and negative controls passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
