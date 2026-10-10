from __future__ import annotations

import copy
import unittest
from pathlib import Path

from check_renderer_compatibility import audit_compatibility, validate_profile
from validate_scene_pack import load_json_file, validate_manifest

ROOT = Path(__file__).resolve().parents[2]
MANIFEST_PATH = ROOT / "contracts/examples/first-germination.scene.json"
SCHEMA_PATH = ROOT / "contracts/ambient-scene-pack-v1.schema.json"
PROFILE_SCHEMA_PATH = ROOT / "contracts/renderer-capability-profile-v1.schema.json"
PROFILE_PATH = ROOT / "contracts/examples/sovereign-visual-core.source-inspection.profile.json"


class RendererCompatibilityTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.manifest = load_json_file(MANIFEST_PATH)
        cls.schema = load_json_file(SCHEMA_PATH)
        cls.profile = load_json_file(PROFILE_PATH)
        cls.profile_schema = load_json_file(PROFILE_SCHEMA_PATH)

    def test_first_germination_manifest_is_schema_valid_before_compatibility_audit(self):
        issues = validate_manifest(self.schema, self.manifest, package_root=MANIFEST_PATH.parent)
        self.assertEqual([], issues, "\n".join(map(str, issues)))

    def test_pinned_renderer_profile_matches_its_schema(self):
        issues = validate_profile(self.profile_schema, self.profile)
        self.assertEqual([], issues, "\n".join(map(str, issues)))
        self.assertEqual("13127ef41ea64312a8b206e6313bd9c48cc5c2a2", self.profile["renderer"]["commit"])
        self.assertEqual("source-inspection-only", self.profile["renderer"]["qualificationState"])

    def test_mismatched_engine_id_is_rejected(self):
        candidate = copy.deepcopy(self.profile)
        candidate["engineId"] = "different-engine-v1"
        candidate["engineVersions"] = ["1.0.0"]
        profile_issues = validate_profile(self.profile_schema, candidate)
        self.assertEqual([], profile_issues, "\n".join(map(str, profile_issues)))
        report = audit_compatibility(self.manifest, candidate)
        self.assertIn("engine.unsupported", {item.code for item in report.issues})

    def test_current_profile_fails_closed_for_known_scene_pack_gaps(self):
        report = audit_compatibility(self.manifest, self.profile)
        self.assertFalse(report.compatible)
        self.assertEqual("13127ef41ea64312a8b206e6313bd9c48cc5c2a2", report.renderer_commit)
        self.assertEqual("source-inspection-only", report.qualification_state)
        codes = {item.code for item in report.issues}
        self.assertTrue({
            "engine.version.unsupported",
            "manifest.adapter.not_implemented",
            "budget.whole_process.not_enforced",
            "presentation.variant.unsupported",
            "presentation.field_unmapped",
            "lifecycle.policy.unsupported",
        }.issubset(codes), f"missing expected blockers: {codes}")
        self.assertNotIn("seed.encoding.unsupported", codes)
        self.assertNotIn("palette.field_unmapped", codes)
        self.assertNotIn("simulation.branch_limit.not_configurable", codes)
        self.assertNotIn("simulation.max_depth.not_configurable", codes)
        self.assertNotIn("simulation.growth_rate.not_configurable", codes)
        self.assertNotIn("simulation.fixed_step.not_configurable", codes)
        self.assertNotIn("simulation.pulse_period.not_configurable", codes)
        self.assertNotIn("simulation.drift_amplitude.not_configurable", codes)
        self.assertNotIn("budget.renderer_estimate.not_enforced", codes)
        self.assertNotIn("presentation.static_gradient.unsupported", codes)
        self.assertNotIn("simulation.resource_branch_limit.not_configurable", codes)
        self.assertNotIn("budget.branch_ceiling.exceeds_core_ceiling", codes)

    def test_lower_profile_depth_ceiling_reports_typed_blocker_without_crashing(self):
        candidate = copy.deepcopy(self.profile)
        candidate["simulation"]["maxDepthCeiling"] = 1
        report = audit_compatibility(self.manifest, candidate)
        found = [item for item in report.issues if item.code == "simulation.max_depth.exceeds_core_ceiling"]
        self.assertEqual(1, len(found))
        self.assertIn("profile ceiling 1", found[0].message)

    def test_missing_independent_resource_branch_ceiling_is_reported(self):
        candidate = copy.deepcopy(self.profile)
        candidate["simulation"]["resourceBranchLimitConfigurable"] = False
        report = audit_compatibility(self.manifest, candidate)
        self.assertIn("simulation.resource_branch_limit.not_configurable", {item.code for item in report.issues})

    def test_a_fully_declared_profile_can_pass_compatibility_without_claiming_qualification(self):
        candidate = copy.deepcopy(self.profile)
        candidate["engineVersions"] = ["1.0.0"]
        candidate["seedEncodings"] = ["phrase-hash-blake3-utf8", "uint32-domain-separated-blake3-v1"]
        candidate["manifestAdapterImplemented"] = True
        candidate["palette"] = {
            "configurable": True,
            "supportedFields": ["canvas", "substrate", "filament", "node", "lichen", "glow"],
        }
        candidate["simulation"].update({
            "branchLimitConfigurable": True,
            "resourceBranchLimitConfigurable": True,
            "maxResourceBranchLimit": 8192,
            "maxDepthConfigurable": True,
            "growthRateConfigurable": True,
            "fixedStepHzConfigurable": True,
            "pulsePeriodConfigurable": True,
            "driftAmplitudeConfigurable": True,
            "branchLimitEnforcedBeforeSpawn": True,
            "maxBranchLimit": 8192,
            "maxDepthCeiling": 24,
        })
        candidate["resources"].update({
            "maxMemoryMiB": 2048,
            "rendererMemoryEstimateEnforced": True,
            "wholeProcessMemoryLimitEnforced": True,
        })
        candidate["presentation"] = {
            "supportedVariants": ["boot", "desktop", "idle", "lockedBackground", "staticFallback"],
            "configurableFields": ["motion", "maxFps", "brightness", "composition", "safeRegions"],
            "staticGradientFallback": True,
        }
        candidate["lifecycle"]["supportedPolicies"] = [
            "onLock:reduced-motion", "onSuspend:pause", "onWake:reinitialize-from-seed"
        ]
        issues = validate_profile(self.profile_schema, candidate)
        self.assertEqual([], issues, "\n".join(map(str, issues)))
        report = audit_compatibility(self.manifest, candidate)
        self.assertTrue(report.compatible, "\n".join(f"{i.code}: {i.message}" for i in report.issues))
        self.assertEqual("source-inspection-only", report.qualification_state)


if __name__ == "__main__":
    unittest.main(verbosity=2)
