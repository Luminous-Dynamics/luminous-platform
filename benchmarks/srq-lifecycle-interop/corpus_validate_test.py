#!/usr/bin/env python3
"""Negative controls for the frozen SRQ corpus validator."""
from __future__ import annotations

import copy
import unittest
from pathlib import Path

from schema_validate import load_strict_json
from corpus_validate import semantic_errors

ROOT = Path(__file__).resolve().parent
MANIFEST, _ = load_strict_json(ROOT / "corpus-v1.json", 2 * 1024 * 1024)
LOCK, _ = load_strict_json(ROOT / "upstream-lock-v1.json", 2 * 1024 * 1024)


class CorpusSemanticTests(unittest.TestCase):
    def test_frozen_manifest_is_semantically_valid(self) -> None:
        self.assertEqual(semantic_errors(copy.deepcopy(MANIFEST), copy.deepcopy(LOCK)), [])

    def test_duplicate_case_id_is_rejected(self) -> None:
        candidate = copy.deepcopy(MANIFEST)
        candidate["cases"][1]["id"] = candidate["cases"][0]["id"]
        codes = {error["code"] for error in semantic_errors(candidate, copy.deepcopy(LOCK))}
        self.assertIn("case-inventory", codes)

    def test_missing_case_is_rejected(self) -> None:
        candidate = copy.deepcopy(MANIFEST)
        candidate["cases"].pop()
        codes = {error["code"] for error in semantic_errors(candidate, copy.deepcopy(LOCK))}
        self.assertIn("case-inventory", codes)

    def test_unknown_replacement_case_is_rejected_even_when_count_matches(self) -> None:
        candidate = copy.deepcopy(MANIFEST)
        candidate["cases"][0]["id"] = "unregistered-new-case"
        codes = {error["code"] for error in semantic_errors(candidate, copy.deepcopy(LOCK))}
        self.assertIn("case-inventory", codes)

    def test_release_pass_mutation_is_rejected(self) -> None:
        candidate = copy.deepcopy(MANIFEST)
        candidate["release_invariant"]["release_pass_allowed"] = True
        codes = {error["code"] for error in semantic_errors(candidate, copy.deepcopy(LOCK))}
        self.assertIn("release-invariant", codes)

    def test_claim_ceiling_invariant_mutation_is_rejected(self) -> None:
        candidate = copy.deepcopy(MANIFEST)
        candidate["release_invariant"]["accepted_structural_control_must_leave_claim_ceiling"] = "domain_validated"
        codes = {error["code"] for error in semantic_errors(candidate, copy.deepcopy(LOCK))}
        self.assertIn("release-invariant", codes)

    def test_upstream_blob_mutation_is_rejected(self) -> None:
        candidate = copy.deepcopy(MANIFEST)
        candidate["upstream_inputs"][0]["git_blob_sha1"] = "0" * 40
        codes = {error["code"] for error in semantic_errors(candidate, copy.deepcopy(LOCK))}
        self.assertIn("upstream-pin-mismatch", codes)

    def test_target_metric_mutation_is_rejected(self) -> None:
        candidate = copy.deepcopy(MANIFEST)
        candidate["target_metrics"]["negative_false_accept_count"] = 1
        codes = {error["code"] for error in semantic_errors(candidate, copy.deepcopy(LOCK))}
        self.assertIn("target-metric", codes)

    def test_schema_disposition_mutation_is_rejected(self) -> None:
        candidate = copy.deepcopy(MANIFEST)
        candidate["cases"][0]["expected_schema_result"] = "VALID"
        codes = {error["code"] for error in semantic_errors(candidate, copy.deepcopy(LOCK))}
        self.assertIn("case-schema-disposition", codes)

    def test_positive_control_tier_and_claim_ceiling_are_bound(self) -> None:
        candidate = copy.deepcopy(MANIFEST)
        candidate["positive_controls"][0]["expected_evidence_claim_ceiling"] = "domain_validated"
        codes = {error["code"] for error in semantic_errors(candidate, copy.deepcopy(LOCK))}
        self.assertIn("positive-control-evidence-ceiling", codes)

    def test_duplicate_upstream_pin_is_rejected(self) -> None:
        candidate = copy.deepcopy(MANIFEST)
        candidate["upstream_inputs"][1] = copy.deepcopy(candidate["upstream_inputs"][0])
        codes = {error["code"] for error in semantic_errors(candidate, copy.deepcopy(LOCK))}
        self.assertTrue({"upstream-pin-mismatch", "duplicate-upstream-pin"} & codes)

    def test_wrong_protocol_identity_is_rejected(self) -> None:
        candidate = copy.deepcopy(MANIFEST)
        candidate["protocol_id"] = "srq-lifecycle-adversarial-v2"
        codes = {error["code"] for error in semantic_errors(candidate, copy.deepcopy(LOCK))}
        self.assertIn("protocol-identity", codes)


if __name__ == "__main__":
    unittest.main(verbosity=2)
