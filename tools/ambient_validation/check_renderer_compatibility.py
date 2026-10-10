#!/usr/bin/env python3
"""Audit Scene Pack v1 against a versioned renderer capability profile.

Compatibility is a source-contract check, not proof that a renderer builds or runs.
Exit codes: 0 compatible, 1 incompatible, 2 invalid input/profile/manifest.
"""
from __future__ import annotations

import argparse
import json
import sys
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker
from validate_scene_pack import DEFAULT_SCHEMA, Issue, check_schema, load_json_file, validate_manifest

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_PROFILE_SCHEMA = ROOT / "contracts/renderer-capability-profile-v1.schema.json"
DEFAULT_PROFILE = ROOT / "contracts/examples/sovereign-visual-core.source-inspection.profile.json"
PRESENTATION_FIELDS = ("motion", "maxFps", "brightness", "composition", "safeRegions")
PALETTE_FIELDS = ("canvas", "substrate", "filament", "node", "lichen", "glow")
SIMULATION_SUPPORT = {
    "branchLimit": ("branchLimitConfigurable", "simulation.branch_limit.not_configurable"),
    "maxDepth": ("maxDepthConfigurable", "simulation.max_depth.not_configurable"),
    "growthRate": ("growthRateConfigurable", "simulation.growth_rate.not_configurable"),
    "fixedStepHz": ("fixedStepHzConfigurable", "simulation.fixed_step.not_configurable"),
    "pulsePeriodSeconds": ("pulsePeriodConfigurable", "simulation.pulse_period.not_configurable"),
    "driftAmplitude": ("driftAmplitudeConfigurable", "simulation.drift_amplitude.not_configurable"),
}


@dataclass(frozen=True)
class CompatibilityIssue:
    code: str
    path: str
    message: str


@dataclass(frozen=True)
class CompatibilityReport:
    manifest_scene_id: str
    profile_id: str
    renderer_commit: str
    qualification_state: str
    compatible: bool
    issues: tuple[CompatibilityIssue, ...]

    def to_dict(self) -> dict[str, Any]:
        return {
            "manifestSceneId": self.manifest_scene_id,
            "profileId": self.profile_id,
            "rendererCommit": self.renderer_commit,
            "qualificationState": self.qualification_state,
            "compatible": self.compatible,
            "issueCount": len(self.issues),
            "issues": [asdict(issue) for issue in self.issues],
            "nonClaims": [
                "A compatible report is not a build, runtime, visual, performance, lifecycle, or security qualification.",
                "The profile is only as current as its pinned source commit and supplied evidence."
            ]
        }


def validate_profile(profile_schema: Any, profile: Any) -> list[Issue]:
    schema_issues = check_schema(profile_schema)
    if schema_issues:
        return schema_issues
    validator = Draft202012Validator(profile_schema, format_checker=FormatChecker())
    errors = sorted(validator.iter_errors(profile), key=lambda e: (tuple(map(str, e.absolute_path)), e.message))
    return [Issue("profile.invalid", f"at /{'/'.join(map(str, e.absolute_path))}: {e.message}") for e in errors]


def audit_compatibility(manifest: dict[str, Any], profile: dict[str, Any]) -> CompatibilityReport:
    """Return deterministic blockers; does not assume absent profile features exist."""
    issues: list[CompatibilityIssue] = []

    def add(code: str, path: str, message: str) -> None:
        issue = CompatibilityIssue(code, path, message)
        if issue not in issues:
            issues.append(issue)

    simulation = manifest.get("simulation", {})
    if simulation.get("engine") != "mycelial-network-v1":
        add("engine.unsupported", "/simulation/engine", "profile does not declare support for this engine")
    if profile["seedEncoding"] != "uint32-domain-separated-blake3-v1":
        add("seed.encoding.unsupported", "/simulation/seed", "Scene Pack uint32 seed encoding is not implemented by this renderer profile")

    palette = profile["palette"]
    for field in PALETTE_FIELDS:
        if not palette["configurable"] or field not in palette["supportedFields"]:
            add("palette.field_unmapped", f"/palette/{field}", f"renderer profile does not consume palette.{field}")

    simulation_cfg = profile["simulation"]
    parameters = simulation.get("parameters", {})
    for field, (capability, code) in SIMULATION_SUPPORT.items():
        if not simulation_cfg[capability]:
            path = f"/simulation/fixedStepHz" if field == "fixedStepHz" else f"/simulation/parameters/{field}"
            add(code, path, f"renderer profile does not implement Scene Pack setting {field}")
    requested_branches = parameters.get("branchLimit")
    aggregate_branches = manifest.get("resourceBudget", {}).get("maxBranches")
    if isinstance(requested_branches, int) and requested_branches > simulation_cfg["maxBranchLimit"]:
        add("simulation.branch_limit.exceeds_core_ceiling", "/simulation/parameters/branchLimit",
            f"requested {requested_branches} exceeds profile ceiling {simulation_cfg['maxBranchLimit']}")
    if not simulation_cfg["branchLimitEnforcedBeforeSpawn"]:
        add("simulation.branch_limit.not_enforced", "/simulation/parameters/branchLimit",
            "renderer must enforce the effective limit before branch creation")
    if isinstance(requested_branches, int) and isinstance(aggregate_branches, int) and requested_branches > aggregate_branches:
        add("budget.branch_limit.exceeds_resource_budget", "/simulation/parameters/branchLimit",
            "simulation branchLimit exceeds resourceBudget.maxBranches")
    requested_depth = parameters.get("maxDepth")
    if isinstance(requested_depth, int) and requested_depth > simulation_cfg["fixedMaxDepth"]:
        add("simulation.max_depth.exceeds_core_ceiling", "/simulation/parameters/maxDepth",
            f"requested depth {requested_depth} exceeds profile ceiling {simulation_cfg['fixedMaxDepth']}")

    resources = profile["resources"]
    budget = manifest.get("resourceBudget", {})
    requested_memory = budget.get("maxMemoryMiB")
    if not resources["manifestMemoryBudgetEnforced"] or resources["maxMemoryMiB"] is None:
        add("budget.memory.not_enforced", "/resourceBudget/maxMemoryMiB",
            "renderer profile does not enforce the manifest's aggregate memory ceiling")
    elif isinstance(requested_memory, int) and requested_memory > resources["maxMemoryMiB"]:
        add("budget.memory.exceeds_core_ceiling", "/resourceBudget/maxMemoryMiB",
            f"requested {requested_memory} MiB exceeds profile ceiling {resources['maxMemoryMiB']} MiB")

    presentations = manifest.get("presentations", {})
    presentation_profile = profile["presentation"]
    for variant, config in presentations.items():
        if variant not in presentation_profile["supportedVariants"]:
            add("presentation.variant.unsupported", f"/presentations/{variant}",
                f"renderer profile does not declare support for presentation variant {variant}")
        if not isinstance(config, dict):
            continue
        for field in PRESENTATION_FIELDS:
            if field not in presentation_profile["configurableFields"]:
                add("presentation.field_unmapped", f"/presentations/{variant}/{field}",
                    f"renderer profile does not map presentation field {field}")
    if budget.get("maxFps") and "maxFps" not in presentation_profile["configurableFields"]:
        add("presentation.fps_budget.unenforced", "/resourceBudget/maxFps",
            "no declared presentation adapter applies the FPS ceiling")
    if presentations.get("staticFallback", {}).get("composition") == "gradient-only" and not presentation_profile["staticGradientFallback"]:
        add("presentation.static_gradient.unsupported", "/presentations/staticFallback",
            "profile cannot produce the required gradient-only static fallback")

    lifecycle = manifest.get("lifecycle", {})
    for field in ("onLock", "onSuspend", "onWake"):
        policy = lifecycle.get(field)
        if policy is not None and f"{field}:{policy}" not in profile["lifecycle"]["supportedPolicies"]:
            add("lifecycle.policy.unsupported", f"/lifecycle/{field}",
                f"renderer/host profile does not enforce lifecycle policy {field}={policy}")

    renderer = profile["renderer"]
    return CompatibilityReport(
        manifest_scene_id=str(manifest.get("sceneId", "")),
        profile_id=str(profile.get("profileId", "")),
        renderer_commit=str(renderer.get("commit", "")),
        qualification_state=str(renderer.get("qualificationState", "unknown")),
        compatible=(len(issues) == 0),
        issues=tuple(issues),
    )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("manifest", type=Path)
    parser.add_argument("--schema", type=Path, default=DEFAULT_SCHEMA)
    parser.add_argument("--profile", type=Path, default=DEFAULT_PROFILE)
    parser.add_argument("--profile-schema", type=Path, default=DEFAULT_PROFILE_SCHEMA)
    parser.add_argument("--json", action="store_true", help="emit a machine-readable report")
    args = parser.parse_args(argv)
    try:
        schema = load_json_file(args.schema)
        manifest = load_json_file(args.manifest)
        profile_schema = load_json_file(args.profile_schema)
        profile = load_json_file(args.profile)
    except (OSError, ValueError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 2
    manifest_issues = validate_manifest(schema, manifest, package_root=args.manifest.parent)
    if manifest_issues:
        for issue in manifest_issues:
            print(f"ERROR: manifest {issue}", file=sys.stderr)
        return 2
    profile_issues = validate_profile(profile_schema, profile)
    if profile_issues:
        for issue in profile_issues:
            print(f"ERROR: capability profile {issue}", file=sys.stderr)
        return 2

    report = audit_compatibility(manifest, profile)
    if args.json:
        print(json.dumps(report.to_dict(), indent=2, sort_keys=True))
    else:
        state = "COMPATIBLE" if report.compatible else "INCOMPATIBLE"
        print(f"{state}: scene={report.manifest_scene_id} profile={report.profile_id}")
        print(f"renderer_commit={report.renderer_commit} qualification={report.qualification_state}")
        for issue in report.issues:
            print(f"BLOCKER [{issue.code}] {issue.path}: {issue.message}")
    return 0 if report.compatible else 1


if __name__ == "__main__":
    raise SystemExit(main())
