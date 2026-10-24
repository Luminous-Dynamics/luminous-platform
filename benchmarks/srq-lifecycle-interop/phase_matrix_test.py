#!/usr/bin/env python3
"""Unit tests for exhaustive transaction phase/presence enumeration."""
from __future__ import annotations

import copy
import unittest
from pathlib import Path

from schema_validate import load_strict_json
from phase_matrix import build_cases, validate_protocol

ROOT = Path(__file__).resolve().parent
PROTOCOL, _ = load_strict_json(ROOT / "transaction-phase-matrix-v1.json", 2 * 1024 * 1024)
LOCK, _ = load_strict_json(ROOT / "upstream-lock-v1.json", 2 * 1024 * 1024)


class PhaseMatrixModelTests(unittest.TestCase):
    def test_frozen_protocol_matches_lock_and_policy(self) -> None:
        self.assertEqual(validate_protocol(copy.deepcopy(PROTOCOL), copy.deepcopy(LOCK)), [])

    def test_enumerates_all_masks_for_all_nine_phases(self) -> None:
        cases = list(build_cases(PROTOCOL))
        self.assertEqual(len(cases), 576)
        counts: dict[str, int] = {}
        for case in cases:
            counts[case["phase"]] = counts.get(case["phase"], 0) + 1
        self.assertEqual(set(counts), {item["phase"] for item in PROTOCOL["phase_requirements"]})
        self.assertEqual(set(counts.values()), {64})

    def test_acceptance_and_rejection_cardinality_is_exact(self) -> None:
        cases = list(build_cases(PROTOCOL))
        accepted = sum(case["expected_accept"] for case in cases)
        self.assertEqual(accepted, 175)
        self.assertEqual(len(cases) - accepted, 401)

    def test_planned_phase_allows_all_record_presence_masks_by_contract(self) -> None:
        planned = [case for case in build_cases(PROTOCOL) if case["phase"] == "planned"]
        self.assertEqual(len(planned), 64)
        self.assertTrue(all(case["expected_accept"] for case in planned))

    def test_promoted_requires_every_record_in_frozen_prerequisite_set(self) -> None:
        promoted = [case for case in build_cases(PROTOCOL) if case["phase"] == "promoted"]
        self.assertEqual(sum(case["expected_accept"] for case in promoted), 1)
        self.assertEqual(
            set(promoted[-1]["present_records"]),
            {"validation", "authorization", "snapshot", "application", "verification", "outcome"},
        )

    def test_record_field_inventory_mutation_is_rejected(self) -> None:
        candidate = copy.deepcopy(PROTOCOL)
        candidate["record_fields"].append("unregistered_field")
        errors = validate_protocol(candidate, copy.deepcopy(LOCK))
        self.assertTrue(any("record field order/inventory" in error for error in errors))

    def test_phase_requirement_mutation_is_rejected(self) -> None:
        candidate = copy.deepcopy(PROTOCOL)
        candidate["phase_requirements"][6]["required_records"].remove("verification")
        errors = validate_protocol(candidate, copy.deepcopy(LOCK))
        self.assertTrue(any("phase requirement mapping" in error for error in errors))

    def test_release_pass_mutation_is_rejected(self) -> None:
        candidate = copy.deepcopy(PROTOCOL)
        candidate["release_invariant"]["release_pass_allowed"] = True
        errors = validate_protocol(candidate, copy.deepcopy(LOCK))
        self.assertTrue(any("release-blocked invariant" in error for error in errors))

    def test_zero_error_target_drift_is_rejected(self) -> None:
        candidate = copy.deepcopy(PROTOCOL)
        candidate["target_metrics"]["false_accept_count"] = 1
        errors = validate_protocol(candidate, copy.deepcopy(LOCK))
        self.assertTrue(any("zero-error target metrics" in error for error in errors))

    def test_upstream_blob_mutation_is_rejected(self) -> None:
        candidate = copy.deepcopy(PROTOCOL)
        candidate["source_contract"]["git_blob_sha1"] = "0" * 40
        errors = validate_protocol(candidate, copy.deepcopy(LOCK))
        self.assertTrue(any("immutable lock entry" in error for error in errors))

    def test_record_presence_is_independent_of_record_payload_content(self) -> None:
        case = next(case for case in build_cases(PROTOCOL) if case["phase"] == "failed" and case["presence_mask"] == "20")
        # Bit 5 is the outcome record: the test intentionally qualifies presence only.
        self.assertEqual(case["present_records"], ["outcome"])
        self.assertTrue(case["expected_accept"])


if __name__ == "__main__":
    unittest.main(verbosity=2)
