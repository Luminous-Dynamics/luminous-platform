"""Adversarial tests for the government-network control evidence matrix."""

from __future__ import annotations

import copy
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))

from validate_hardware_control_evidence_matrix import matrix_errors  # noqa: E402
from validate_hardware_portfolio import load_json  # noqa: E402


class HardwareControlEvidenceMatrixTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.matrix = load_json(ROOT / "security/control-evidence-matrix-v1.json")
        cls.schema = load_json(ROOT / "schemas/hardware-control-evidence-matrix-v1.schema.json")

    def errors(self, matrix):
        return matrix_errors(matrix, self.schema)

    def test_baseline_is_valid_but_all_controls_are_unassessed(self) -> None:
        self.assertEqual(self.errors(copy.deepcopy(self.matrix)), [])
        self.assertEqual(self.matrix["register_status"], "baseline_not_evaluated")
        self.assertTrue(all(control["status"] == "NOT_EVALUATED" for control in self.matrix["controls"]))

    def test_duplicate_control_id_fails(self) -> None:
        matrix = copy.deepcopy(self.matrix)
        matrix["controls"].append(copy.deepcopy(matrix["controls"][0]))
        self.assertTrue(any("duplicates" in error for error in self.errors(matrix)))

    def test_unknown_source_reference_fails(self) -> None:
        matrix = copy.deepcopy(self.matrix)
        matrix["controls"][0]["mapped_controls"].append("made-up-authority:AC-2")
        self.assertTrue(any("unknown source" in error for error in self.errors(matrix)))

    def test_http_authority_link_fails_closed(self) -> None:
        matrix = copy.deepcopy(self.matrix)
        matrix["references"][0]["url"] = "http://example.org/control"
        self.assertTrue(any("HTTPS" in error for error in self.errors(matrix)))

    def test_pass_without_assessment_record_fails(self) -> None:
        matrix = copy.deepcopy(self.matrix)
        matrix["controls"][0]["status"] = "PASS"
        self.assertTrue(any("requires an assessment record" in error for error in self.errors(matrix)))

    def test_pass_with_open_gap_fails(self) -> None:
        matrix = copy.deepcopy(self.matrix)
        control = matrix["controls"][0]
        control["status"] = "PASS"
        control["assessment_record"] = {
            "assessor": "reviewer-1",
            "assessed_on": "2026-10-10",
            "tested_revision": "a" * 40,
            "scope": "Exact lab revision; explicitly bounded test scope.",
            "result": "PASS",
            "evidence_refs": ["https://example.org/evidence/receipt.json"],
            "limitations": []
        }
        self.assertTrue(any("PASS cannot have open gaps" in error for error in self.errors(matrix)))

    def test_not_evaluated_cannot_carry_a_pass_record(self) -> None:
        matrix = copy.deepcopy(self.matrix)
        matrix["controls"][0]["assessment_record"] = {
            "assessor": "reviewer-1",
            "assessed_on": "2026-10-10",
            "tested_revision": "a" * 40,
            "scope": "Exact lab revision; explicitly bounded test scope.",
            "result": "PASS",
            "evidence_refs": ["https://example.org/evidence/receipt.json"],
            "limitations": []
        }
        self.assertTrue(any("must not carry an assessment record" in error for error in self.errors(matrix)))


if __name__ == "__main__":
    unittest.main()
