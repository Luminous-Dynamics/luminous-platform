#!/usr/bin/env python3
"""Validation-only guard for the external benchmark evidence registry.

The benchmark runtime remains Rust. This script checks the public JSON Schema
and cross-field constraints that a JSON Schema cannot conveniently express.
"""
from __future__ import annotations

import copy
import csv
import hashlib
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
SREGYM_0508_COHORT_CSV_PATH = ROOT / "sregym-0508-cohort.csv"
SREGYM_0508_REGISTRY_IDS_PATH = ROOT / "sregym-0508-registry-ids.txt"
SREGYM_0508_RECONCILIATION_PATH = ROOT / "sregym-0508-source-reconciliation.json"


def native_sregym_source_errors(manifest: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    cohort = manifest["cohort"]
    source = cohort["cohort_manifest_source"]
    reconciliation = cohort["source_reconciliation"]

    try:
        csv_bytes = SREGYM_0508_COHORT_CSV_PATH.read_bytes()
        csv_text = csv_bytes.decode("utf-8")
        rows = list(csv.DictReader(csv_text.splitlines()))
        registry_ids = [
            line.strip()
            for line in SREGYM_0508_REGISTRY_IDS_PATH.read_text(encoding="utf-8").splitlines()
            if line.strip()
        ]
        source_record = read_json(SREGYM_0508_RECONCILIATION_PATH)
    except (OSError, UnicodeDecodeError, csv.Error) as exc:
        return [f"source snapshots: cannot load exact SREGym-0508 cohort/registry inputs: {exc}"]

    git_blob = hashlib.sha1(
        b"blob " + str(len(csv_bytes)).encode("ascii") + b"\0" + csv_bytes
    ).hexdigest()
    if git_blob != source["git_blob_sha"]:
        errors.append("cohort_manifest_source: archived CSV bytes do not match the upstream Git blob SHA")
    if source["repository"] != source_record["cohort"]["repository"] \
        or source["commit_sha"] != source_record["cohort"]["commit_sha"] \
        or source["path"] != source_record["cohort"]["manifest_path"] \
        or source["git_blob_sha"] != source_record["cohort"]["manifest_git_blob_sha"]:
        errors.append("cohort_manifest_source: manifest source differs from the captured reconciliation record")

    ids = [row.get("problem_id", "") for row in rows]
    categories = {row.get("category", "") for row in rows}
    if len(rows) != 90 or len(set(ids)) != 90 or any(not item for item in ids):
        errors.append("cohort manifest: expected exactly 90 unique non-empty problem IDs")
    if not rows or "category" not in rows[0] or "problem_id" not in rows[0]:
        errors.append("cohort manifest: expected problem_id,category CSV columns")
    if not categories.issubset({"Ported", "Similar Failures", "New Failures"}):
        errors.append("cohort manifest: unknown cohort category")
    if len(registry_ids) != 90 or len(set(registry_ids)) != 90:
        errors.append("registry ID snapshot: expected 90 unique registered problem IDs")
    if set(ids) != set(registry_ids):
        errors.append("source reconciliation: candidate registry IDs do not exactly match the official cohort CSV")

    candidate = source_record["candidate_source"]
    expected_reconciliation = {
        "candidate_repository": candidate["repository"],
        "candidate_commit_sha": candidate["commit_sha"],
        "candidate_commit_date_utc": candidate["commit_date_utc"],
        "registry_path": candidate["registry_path"],
        "registry_blob_sha": candidate["registry_git_blob_sha"],
        "registry_id_count": candidate["registered_problem_count"],
        "registry_ids_snapshot_path": candidate["derived_registry_ids_snapshot"],
        "tasklist_yml_example_path": candidate["tasklist_yml_example_path"],
        "tasklist_yml_example_blob_sha": candidate["tasklist_yml_example_blob_sha"],
        "tasklist_yml_example_id_count": candidate["tasklist_yml_example_id_count"],
        "tasklist_yml_runtime_policy": candidate["runtime_selection"],
        "cohort_registry_match": "exact_set_match",
        "execution_revision_attested": False,
    }
    for key, expected in expected_reconciliation.items():
        if reconciliation.get(key) != expected:
            errors.append(f"source reconciliation: {key} does not match the archived source record")

    pins = manifest["source_pins"]
    if pins["sregym_commit_sha"] != candidate["commit_sha"]:
        errors.append("source_pins: selected SREGym commit differs from the exact-90 registry candidate")
    if pins["registry_path"] != candidate["registry_path"] or pins["registry_blob_sha"] != candidate["registry_git_blob_sha"]:
        errors.append("source_pins: selected registry file does not match the candidate source")
    if pins["scenario_manifest_repository"] != source["repository"] \
        or pins["scenario_manifest_commit_sha"] != source["commit_sha"] \
        or pins["scenario_manifest_path"] != source["path"] \
        or pins["scenario_manifest_blob_sha"] != source["git_blob_sha"]:
        errors.append("source_pins: scenario manifest does not match the exact archived upstream cohort CSV")
    if pins["tasklist_yml_must_be_absent"] is not True:
        errors.append("source_pins: tasklist.yml must be absent to select all 90 registered IDs on this revision")
    if manifest["execution_environment"]["tasklist_override_state"] != "absent" \
        or manifest["execution_environment"]["tasklist_override_digest"] is not None:
        errors.append("execution_environment: the ignored tasklist.yml override must be absent for the full 90-ID default selection")
    if reconciliation["execution_revision_attested"]:
        errors.append("source reconciliation: candidate revision is a proven cohort match, not an attested historical leaderboard execution revision")

    return errors


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

    inventory = cohort["cohort_observation_source"]
    capture_status = inventory["capture_status"]
    if capture_status == "metadata_only" and (
        inventory["git_blob_sha"] is not None or inventory["content_sha256"] is not None
    ):
        errors.append("cohort.cohort_observation_source: metadata_only cannot claim captured bytes")
    if capture_status == "git_blob_only" and (
        inventory["git_blob_sha"] is None or inventory["content_sha256"] is not None
    ):
        errors.append("cohort.cohort_observation_source: git_blob_only requires a Git blob SHA and no content SHA-256")
    if capture_status == "content_sha256_verified" and inventory["content_sha256"] is None:
        errors.append("cohort.cohort_observation_source: content_sha256_verified requires the raw content SHA-256")

    accounting = manifest["accounting"]
    if accounting["expected_attempt_rows"] != cohort["expected_attempt_rows"]:
        errors.append("accounting: expected_attempt_rows must match the cohort denominator")
    if accounting["completed_attempt_rows"] + accounting["missing_attempt_rows"] != accounting["expected_attempt_rows"]:
        errors.append("accounting: completed + missing attempt rows must equal the expected denominator")
    if accounting["started_attempt_rows"] > accounting["expected_attempt_rows"] + accounting["retried_attempt_rows"]:
        errors.append("accounting: started attempts cannot exceed expected attempts plus declared retries")
    if accounting["completed_attempt_rows"] > accounting["started_attempt_rows"]:
        errors.append("accounting: completed attempt rows cannot exceed attempts actually started")
    if manifest["profile_status"] == "executed" and accounting["started_attempt_rows"] < accounting["expected_attempt_rows"]:
        errors.append("executed: every expected attempt must have been started")
    if accounting["dropped_attempt_rows"] > accounting["missing_attempt_rows"]:
        errors.append("accounting: dropped rows must be included in the missing-row count")
    if accounting["dropped_attempt_rows"] != 0:
        errors.append("accounting: dropped attempt rows are prohibited; retain every started attempt's terminal record")
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
    if not protocol["native_benchmark_harness_unchanged"]:
        errors.append("native_protocol: the official native benchmark harness/task/scorer must remain unchanged")

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
        if design["common_context_budget_digest"] is None or design["common_tool_budget_digest"] is None:
            errors.append("ready/executed: context and tool budgets must be digest-pinned")
        if design["comparison_mode"] == "matched_tool_surface_ablation" and design["capability_equivalence_artifact_digest"] is None:
            errors.append("ready/executed matched ablation: capability-equivalence evidence is mandatory")

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
    return native_sregym_semantic_errors(manifest) + native_sregym_source_errors(manifest)


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

    wrong_registry_candidate = copy.deepcopy(manifest)
    wrong_registry_candidate["source_pins"]["sregym_commit_sha"] = "2f5fd1c73f5f1f9b2702d15cdc79bb25dfa774a9"
    expect_native_rejected("later 91-ID registry substituted for exact 90-ID candidate", wrong_registry_candidate, schema)

    tampered_cohort_blob = copy.deepcopy(manifest)
    tampered_cohort_blob["cohort"]["cohort_manifest_source"]["git_blob_sha"] = "f" * 40
    expect_native_rejected("cohort CSV blob substitution", tampered_cohort_blob, schema)


    tasklist_override = copy.deepcopy(manifest)
    tasklist_override["execution_environment"]["tasklist_override_state"] = "present_pinned"
    tasklist_override["execution_environment"]["tasklist_override_digest"] = "c" * 64
    expect_native_rejected("86-ID example task list used as the full 90-ID cohort", tasklist_override, schema)


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

    matched = copy.deepcopy(manifest)
    matched["study_design"]["comparison_mode"] = "matched_tool_surface_ablation"
    matched["study_design"]["common_model_snapshot_digest"] = "1" * 64
    matched["study_design"]["common_agent_scaffold_digest"] = "2" * 64
    matched["study_design"]["common_prompt_digest"] = "3" * 64
    matched["study_design"]["common_permission_profile_digest"] = "4" * 64
    matched["study_design"]["common_reasoning_effort"] = "medium"
    matched["study_design"]["common_sampling_policy"] = "temperature=0;seeded"
    matched["study_design"]["common_context_budget_digest"] = "5" * 64
    matched["study_design"]["common_tool_budget_digest"] = "6" * 64
    matched["study_design"]["capability_equivalence_artifact_digest"] = "7" * 64
    matched["arms"][0].update({
        "arm_kind": "native_reference",
        "model_provider": "provider-x",
        "model_snapshot_digest": "1" * 64,
        "agent_scaffold_digest": "2" * 64,
        "prompt_digest": "3" * 64,
        "reasoning_effort": "medium",
        "sampling_policy": "temperature=0;seeded",
        "permission_profile_digest": "4" * 64,
        "tool_surface_digest": "8" * 64,
        "tool_surface_description": "Pinned native reference surface.",
    })
    candidate_arm = copy.deepcopy(matched["arms"][0])
    candidate_arm.update({
        "arm_id": "structured-gateway",
        "arm_kind": "structured_gateway",
        "tool_surface_digest": "9" * 64,
        "tool_surface_description": "Semantically capability-matched structured gateway.",
    })
    matched["arms"].append(candidate_arm)
    matched_errors = validate_native_sregym(matched, schema)
    if matched_errors:
        raise SystemExit(f"matched-tool-surface positive control unexpectedly failed: {matched_errors}")
    print("accepted as expected: matched model/tool-surface positive control")

    mismatched_model = copy.deepcopy(matched)
    mismatched_model["arms"][1]["model_snapshot_digest"] = "a" * 64
    expect_native_rejected("tool-surface arms with mismatched models", mismatched_model, schema)

    matched_contamination = copy.deepcopy(matched)
    matched_contamination["arms"][0]["luminous_supplemental_gates_enabled"] = True
    expect_native_rejected("native reference contaminated by supplemental gates", matched_contamination, schema)


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
