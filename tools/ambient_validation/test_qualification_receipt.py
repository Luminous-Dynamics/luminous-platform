from __future__ import annotations

import copy
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from qualification_receipt import build_receipt, derive_evidence_level, outcome_status  # noqa: E402


def gates(**overrides):
    result = {
        name: {"status": "pass" if name == "exactHead" else "not-run"}
        for name in (
            "exactHead", "schemaValidation", "contractTests", "rendererBuild",
            "deterministicReplay", "resourceChecks", "lifecycleChecks", "visualCapture",
        )
    }
    result.update(overrides)
    return result


class QualificationReceiptTests(unittest.TestCase):
    def test_successful_contract_gates_are_not_renderer_qualification(self):
        candidate = gates(
            schemaValidation={"status": "pass"},
            contractTests={"status": "pass"},
        )
        self.assertEqual("contract-validated", derive_evidence_level(candidate))

    def test_compile_qualification_requires_renderer_build(self):
        candidate = gates(
            schemaValidation={"status": "pass"},
            contractTests={"status": "pass"},
            rendererBuild={"status": "pass"},
        )
        self.assertEqual("compile-qualified", derive_evidence_level(candidate))

    def test_runtime_qualification_requires_every_runtime_gate(self):
        candidate = gates(
            schemaValidation={"status": "pass"},
            contractTests={"status": "pass"},
            rendererBuild={"status": "pass"},
            deterministicReplay={"status": "pass"},
            resourceChecks={"status": "pass"},
            lifecycleChecks={"status": "pass"},
        )
        self.assertEqual("compile-qualified", derive_evidence_level(candidate))
        candidate["visualCapture"] = {"status": "pass"}
        self.assertEqual("runtime-qualified", derive_evidence_level(candidate))

    def test_failed_queued_cancelled_or_stale_head_gate_never_upgrades(self):
        for status in ("fail", "queued", "cancelled", "stale-head", "not-run"):
            with self.subTest(status=status):
                candidate = gates(
                    schemaValidation={"status": "pass"},
                    contractTests={"status": "pass"},
                    rendererBuild={"status": "pass"},
                    deterministicReplay={"status": status},
                    resourceChecks={"status": "pass"},
                    lifecycleChecks={"status": "pass"},
                    visualCapture={"status": "pass"},
                )
                self.assertEqual("compile-qualified", derive_evidence_level(candidate))
        self.assertEqual("insufficient", derive_evidence_level(gates(exactHead={"status": "stale-head"})))

    def test_workflow_outcomes_map_to_fail_closed_gate_states(self):
        self.assertEqual("pass", outcome_status("success"))
        self.assertEqual("fail", outcome_status("failure"))
        self.assertEqual("cancelled", outcome_status("cancelled"))
        self.assertEqual("not-run", outcome_status("skipped"))

    def test_receipt_refuses_stale_or_synthetic_head_identity(self):
        root = HERE.parents[1]
        with self.assertRaisesRegex(ValueError, "stale-head"):
            build_receipt(
                root,
                expected_commit="a" * 40,
                actual_commit="b" * 40,
                schema_outcome="success",
                tests_outcome="success",
                run_id="123",
                run_attempt="1",
                runner_os="Linux",
                workflow_url="https://github.com/Luminous-Dynamics/luminous-platform/actions/runs/123",
            )

    def test_receipt_binds_input_digests_and_does_not_upgrade_profile_state(self):
        root = HERE.parents[1]
        actual = "a" * 40
        receipt = build_receipt(
            root,
            expected_commit=actual,
            actual_commit=actual,
            schema_outcome="success",
            tests_outcome="success",
            run_id="123",
            run_attempt="1",
            runner_os="Linux",
            workflow_url="https://github.com/Luminous-Dynamics/luminous-platform/actions/runs/123",
        )
        self.assertEqual("contract-validated", receipt["derivedEvidenceLevel"])
        self.assertEqual("source-inspection-only", receipt["declaredRendererQualificationState"])
        self.assertRegex(receipt["inputs"]["scenePackSchema"]["sha256"], r"^[a-f0-9]{64}$")
        self.assertRegex(receipt["inputs"]["manifestFixture"]["sha256"], r"^[a-f0-9]{64}$")
        self.assertEqual("ab6eac9641e0673cf2de36018a35986c122b36a8", receipt["adapter"]["rendererCommit"])
        self.assertEqual("not-run", receipt["gates"]["rendererBuild"]["status"])
        self.assertEqual("not-run", receipt["gates"]["visualCapture"]["status"])


if __name__ == "__main__":
    unittest.main(verbosity=2)
