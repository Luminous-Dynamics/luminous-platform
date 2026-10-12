"""Adversarial tests for hardware portfolio validation."""

from __future__ import annotations

import copy
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))

from validate_hardware_portfolio import catalog_errors, load_json  # noqa: E402


class HardwarePortfolioTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.catalog = load_json(ROOT / "hardware/portfolio-v1.json")
        cls.schema = load_json(ROOT / "schemas/hardware-portfolio-v1.schema.json")

    def errors(self, payload):
        return catalog_errors(payload, self.schema)

    def test_committed_catalog_is_valid_without_qualification_claims(self) -> None:
        self.assertEqual(self.errors(copy.deepcopy(self.catalog)), [])
        self.assertEqual(self.catalog["register_state"], "research_baseline_not_qualified")
        self.assertTrue(all(
            item["maturity"] in {"H0_CANDIDATE", "GAP_UNSELECTED", "REFERENCE_ONLY"}
            for item in self.catalog["items"]
        ))

    def test_duplicate_ids_fail(self) -> None:
        payload = copy.deepcopy(self.catalog)
        payload["items"].append(copy.deepcopy(payload["items"][0]))
        self.assertTrue(any("duplicates" in error for error in self.errors(payload)))

    def test_non_https_source_fails(self) -> None:
        payload = copy.deepcopy(self.catalog)
        payload["items"][0]["source_urls"][0] = "http://example.org/hardware"
        self.assertTrue(any("HTTPS URLs" in error for error in self.errors(payload)))

    def test_h1_commercial_sku_can_be_reviewed_without_a_source_repo_revision(self) -> None:
        payload = copy.deepcopy(self.catalog)
        item = next(entry for entry in payload["items"] if entry["id"] == "dell-poweredge-t160")
        item.update({
            "maturity": "H1_SOURCE_REVIEWED",
            "source_revision": None,
            "named_model_or_revision": "PowerEdge T160; exact Dell order code, BOM and board revision",
            "source_completeness": "product_documentation_reviewed",
            "license_status": "vendor_terms_reviewed",
            "bill_of_materials_status": "complete_and_reviewed",
        })
        self.assertEqual(self.errors(payload), [])

    def test_h1_open_design_requires_a_pinned_source_and_model_revision(self) -> None:
        payload = copy.deepcopy(self.catalog)
        item = next(entry for entry in payload["items"] if entry["id"] == "ocp-minipack3")
        item.update({
            "maturity": "H1_SOURCE_REVIEWED",
            "source_revision": None,
            "named_model_or_revision": "Minipack3 design package v1.0 exact immutable package revision",
            "source_completeness": "native_design_and_bom_reviewed",
            "license_status": "compatible_open_license_reviewed",
            "bill_of_materials_status": "complete_and_reviewed",
        })
        self.assertTrue(any("immutable design-source revision" in error for error in self.errors(payload)))

    def test_h1_without_exact_model_or_source_revision_fails_closed(self) -> None:
        payload = copy.deepcopy(self.catalog)
        item = next(entry for entry in payload["items"] if entry["id"] == "dell-poweredge-t160")
        item.update({
            "maturity": "H1_SOURCE_REVIEWED",
            "source_revision": None,
            "named_model_or_revision": None,
            "source_completeness": "product_documentation_reviewed",
            "license_status": "vendor_terms_reviewed",
        })
        self.assertTrue(any("exact product SKU/model" in error for error in self.errors(payload)))

    def test_h2_without_verified_supply_or_bom_fails(self) -> None:
        payload = copy.deepcopy(self.catalog)
        item = payload["items"][0]
        item.update({
            "maturity": "H2_BUILDABLE",
            "license_status": "compatible_open_license_reviewed",
            "source_completeness": "native_design_and_bom_reviewed",
            "source_revision": "abcdef1234567890",
            "availability_status": "not_verified",
            "bill_of_materials_status": "unknown",
        })
        self.assertTrue(any("verified procurement/manufacturing path" in error for error in self.errors(payload)))
        self.assertTrue(any("reviewed complete BOM" in error for error in self.errors(payload)))

    def test_h5_without_authority_evidence_and_empty_blockers_fails(self) -> None:
        payload = copy.deepcopy(self.catalog)
        item = payload["items"][0]
        item.update({
            "maturity": "H5_DEPLOYMENT_AUTHORIZED",
            "license_status": "compatible_open_license_reviewed",
            "source_completeness": "native_design_and_bom_reviewed",
            "source_revision": "abcdef1234567890",
            "availability_status": "verified_stock",
            "bill_of_materials_status": "complete_and_reviewed",
            "deployment_authorization": "not_authorized",
            "deployment_profile": None,
            "qualification_evidence_refs": [],
            "open_blockers": ["authorization still missing"],
        })
        errors = self.errors(payload)
        self.assertTrue(any("requires qualification evidence references" in error for error in errors))
        self.assertTrue(any("requires named-profile deployment authorization" in error for error in errors))
        self.assertTrue(any("requires deployment_profile" in error for error in errors))
        self.assertTrue(any("H5 cannot have open_blockers" in error for error in errors))

    def test_gap_cannot_be_promoted_to_product_maturity(self) -> None:
        payload = copy.deepcopy(self.catalog)
        gap = next(item for item in payload["items"] if item["maturity"] == "GAP_UNSELECTED")
        gap["maturity"] = "H1_SOURCE_REVIEWED"
        self.assertTrue(any("selection_gap items cannot be promoted" in error for error in self.errors(payload)))

    def test_unknown_top_level_fields_fail_closed(self) -> None:
        payload = copy.deepcopy(self.catalog)
        payload["unexpected_authority_override"] = True
        self.assertTrue(any("Additional properties" in error for error in self.errors(payload)))

    def test_non_finite_json_constants_are_rejected(self) -> None:
        temporary = ROOT / "hardware/.nonfinite-json-test.tmp.json"
        try:
            temporary.write_text('{"value": NaN}', encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "non-standard JSON constant"):
                load_json(temporary)
        finally:
            temporary.unlink(missing_ok=True)

    def test_duplicate_json_keys_are_rejected(self) -> None:
        temporary = ROOT / "hardware/.duplicate-key-test.tmp.json"
        try:
            temporary.write_text('{"a": 1, "a": 2}', encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "duplicate JSON key"):
                load_json(temporary)
        finally:
            temporary.unlink(missing_ok=True)


if __name__ == "__main__":
    unittest.main()
