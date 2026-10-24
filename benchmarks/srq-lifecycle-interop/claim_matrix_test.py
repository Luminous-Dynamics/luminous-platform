#!/usr/bin/env python3
"""Unit tests for the claim-ceiling cross-product and its canonical profiles."""
from __future__ import annotations

import copy
import unittest
from pathlib import Path

from jsonschema import Draft202012Validator, FormatChecker

from schema_validate import load_strict_json
from claim_matrix import (
    CLAIM_CEILINGS,
    EXPECTED_ALLOWED,
    canonical_receipt,
    make_cases,
    validate_protocol,
)

ROOT = Path(__file__).resolve().parent
REPO_ROOT = ROOT.parents[1]
MYCELIX_ROOT = REPO_ROOT / "integration-upstream" / "mycelix"
PROTOCOL, _ = load_strict_json(ROOT / "claim-ceiling-matrix-v1.json", 2 * 1024 * 1024)
LOCK, _ = load_strict_json(ROOT / "upstream-lock-v1.json", 2 * 1024 * 1024)
SCHEMA, _ = load_strict_json(MYCELIX_ROOT / "research/evidence-plane/receipt-v0.schema.json", 2 * 1024 * 1024)
BASE_RECEIPT, _ = load_strict_json(MYCELIX_ROOT / "research/evidence-plane/examples/synthetic-planned-receipt-v0.json", 2 * 1024 * 1024)


class ClaimCeilingMatrixTests(unittest.TestCase):
    def test_protocol_mapping_and_upstream_pin_are_valid(self) -> None:
        self.assertEqual(validate_protocol(copy.deepcopy(PROTOCOL), copy.deepcopy(LOCK)), [])

    def test_cross_product_has_exactly_25_unique_cases(self) -> None:
        base = {
            "schema_version": "0.1.0",
            "receipt_id": "synthetic:matrix-test",
            "created_at": "2026-10-24T00:00:00Z",
            "run_status": "planned",
            "evidence_tier": "S0",
            "claim_ceiling": "synthetic_only",
            "subject": {
                "repository": "synthetic:repo",
                "commit": None,
                "artifact_path": "synthetic/subject.json",
                "artifact_sha256": None,
            },
            "sources": [],
            "transforms": [],
            "benchmark": {
                "protocol_id": "synthetic",
                "protocol_sha256": None,
                "pre_registered_at": None,
                "split_strategy": "not_applicable",
                "split_lock_sha256": None,
                "budget": {"wall_clock_seconds": 0, "cpu_core_seconds": 0, "gpu_seconds": 0, "peak_memory_bytes": 0, "network_policy": "none"},
                "baseline_arms": [],
                "primary_metrics": ["synthetic structural check"],
                "results_summary": {"state": "not_run", "observed_result_count": 0, "note": "synthetic"},
            },
            "gates": {
                "source_integrity": "unknown",
                "rights_stewardship": "unknown",
                "transform_reproducibility": "unknown",
                "split_lock": "unknown",
                "budget_parity": "unknown",
                "negative_controls": "unknown",
                "exact_head": "unknown",
                "safety_critical_regression": "unknown",
                "independent_verification": "unknown",
                "domain_validation": "unknown",
            },
            "independent_verification": {
                "status": "none", "verifier_kind": "none", "verifier_ref": None,
                "replayed_commit": None, "note": "synthetic",
            },
            "limitations": ["Synthetic fixture only."],
        }
        cases = list(make_cases(PROTOCOL, base))
        self.assertEqual(len(cases), 25)
        self.assertEqual(len({case["id"] for case in cases}), 25)
        self.assertEqual(sum(case["expected_accept"] for case in cases), 8)
        self.assertEqual(sum(not case["expected_accept"] for case in cases), 17)

    def test_exact_allowed_claims_by_tier(self) -> None:
        for tier, allowed in EXPECTED_ALLOWED.items():
            actual = [profile["allowed_claim_ceilings"] for profile in PROTOCOL["canonical_profiles"] if profile["evidence_tier"] == tier]
            self.assertEqual(actual, [allowed])

    def test_all_five_canonical_receipts_pass_the_pinned_schema(self) -> None:
        validator = Draft202012Validator(SCHEMA, format_checker=FormatChecker())
        for tier in ("S0", "S1", "S2", "S3", "S4"):
            with self.subTest(tier=tier):
                receipt = canonical_receipt(BASE_RECEIPT, tier)
                errors = list(validator.iter_errors(receipt))
                self.assertEqual(errors, [], "; ".join(error.message for error in errors))

    def test_canonical_profile_builder_fills_status_and_tier(self) -> None:
        base = {
            "run_status": "planned",
            "evidence_tier": "S0",
            "claim_ceiling": "synthetic_only",
            "subject": {"commit": None, "artifact_sha256": None},
            "sources": [],
            "transforms": [],
            "benchmark": {"results_summary": {"state": "not_run", "observed_result_count": 0}},
            "gates": {
                "source_integrity": "unknown", "rights_stewardship": "unknown",
                "independent_verification": "unknown", "negative_controls": "unknown",
                "exact_head": "unknown", "domain_validation": "unknown",
            },
            "independent_verification": {
                "status": "none", "verifier_kind": "none", "verifier_ref": None,
                "replayed_commit": None, "note": "synthetic",
            },
        }
        for tier in ("S0", "S1", "S2", "S3", "S4"):
            receipt = canonical_receipt(base, tier)
            profile = next(x for x in PROTOCOL["canonical_profiles"] if x["evidence_tier"] == tier)
            self.assertEqual(receipt["run_status"], profile["run_status"])
            self.assertEqual(receipt["claim_ceiling"], profile["canonical_claim_ceiling"])

    def test_unknown_tier_is_not_added_to_protocol(self) -> None:
        candidate = copy.deepcopy(PROTOCOL)
        candidate["evidence_tiers"].append("S5")
        errors = validate_protocol(candidate, copy.deepcopy(LOCK))
        self.assertTrue(any("evidence-tier inventory changed" in error for error in errors))

    def test_claim_ceiling_allowlist_mutation_is_rejected(self) -> None:
        candidate = copy.deepcopy(PROTOCOL)
        candidate["canonical_profiles"][0]["allowed_claim_ceilings"].append("domain_validated")
        errors = validate_protocol(candidate, copy.deepcopy(LOCK))
        self.assertTrue(any("canonical profiles" in error for error in errors))

    def test_claim_matrix_cannot_enable_release_pass(self) -> None:
        candidate = copy.deepcopy(PROTOCOL)
        candidate["release_invariant"]["release_pass_allowed"] = True
        errors = validate_protocol(candidate, copy.deepcopy(LOCK))
        self.assertTrue(any("release-blocked invariant" in error for error in errors))

    def test_nonzero_false_accept_target_is_rejected(self) -> None:
        candidate = copy.deepcopy(PROTOCOL)
        candidate["target_metrics"]["false_accept_count"] = 1
        errors = validate_protocol(candidate, copy.deepcopy(LOCK))
        self.assertTrue(any("zero-error target" in error for error in errors))

    def test_source_schema_pin_must_match_immutable_lock(self) -> None:
        candidate = copy.deepcopy(PROTOCOL)
        candidate["source_contract"]["git_blob_sha1"] = "0" * 40
        errors = validate_protocol(candidate, copy.deepcopy(LOCK))
        self.assertTrue(any("immutable lock entry" in error for error in errors))


if __name__ == "__main__":
    unittest.main(verbosity=2)
