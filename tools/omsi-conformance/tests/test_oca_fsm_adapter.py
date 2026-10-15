import copy
import sys
import unittest
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

import omsi_validate as core
import oca_fsm_adapter as adapter

FIXTURES = ROOT / "fixtures"
NOW = datetime(2026, 10, 15, 12, 0, 0, tzinfo=timezone.utc)


def fixture(name):
    return core.load(FIXTURES / name)


def inputs():
    return (
        fixture("oca-fsm-order-snapshot.json"),
        fixture("oca-tenant-binding.json"),
        fixture("oca-stage-map.json"),
    )


class TestOcaFsmReadOnlyAdapter(unittest.TestCase):
    def test_pinned_completed_order_maps_to_closed_not_verified(self):
        source, binding, stage_map = inputs()
        result = adapter.map_order(source, binding, stage_map, NOW)
        self.assertEqual(result["service_case"]["normalized_status"], "closed")
        self.assertTrue(result["source_claims"]["stage_says_closed"])
        self.assertTrue(result["source_claims"]["customer_signature_present"])
        self.assertEqual(result["assurance"]["independent_verification"], "not_provided_by_source")
        self.assertEqual(result["assurance"]["workmanship_quality"], "not_verified")
        self.assertEqual(result["assurance"]["customer_scope_approval"], "not_provided_by_source")

    def test_mapping_preserves_separate_time_windows(self):
        source, binding, stage_map = inputs()
        result = adapter.map_order(source, binding, stage_map, NOW)
        self.assertNotEqual(result["work_order"]["request_window"], result["work_order"]["scheduled_window"])
        self.assertNotEqual(result["work_order"]["scheduled_window"], result["work_order"]["actual_interval"])
        self.assertEqual(result["work_order"]["actual_interval"]["start"], "2026-10-14T10:15:00Z")

    def test_source_ids_are_namespaced_not_display_name_only(self):
        source, binding, stage_map = inputs()
        result = adapter.map_order(source, binding, stage_map, NOW)
        self.assertTrue(result["id"].startswith("odoo-instance://fixture.invalid/plumbing-db|fsm.order|101"))
        self.assertEqual(result["source"]["display_name"], "FSM/2026/0101")
        self.assertEqual(result["source"]["record_id"], "101")

    def test_raw_signature_html_and_contact_data_are_not_exported(self):
        source, binding, stage_map = inputs()
        result = adapter.map_order(source, binding, stage_map, NOW)
        self.assertEqual(result["privacy"], {
            "raw_signature_exported": False,
            "raw_html_exported": False,
            "precise_contact_exported": False,
        })
        self.assertNotIn("signed_by", result)
        self.assertNotIn("signature", result)
        self.assertIn("raw_signature_payload_omitted_by_privacy_policy", result["service_case"]["mapping_losses"])

    def test_unknown_stage_is_unmapped_not_promoted(self):
        source, binding, stage_map = inputs()
        source["order"]["stage"]["id"] = "custom-stage-77"
        stage_map["entries"] = []
        result = adapter.map_order(source, binding, stage_map, NOW)
        self.assertEqual(result["service_case"]["normalized_status"], "unmapped")
        self.assertIn("source_stage_unmapped", result["service_case"]["mapping_losses"])
        self.assertEqual(result["assurance"]["workmanship_quality"], "not_verified")

    def test_stage_mapping_fingerprint_mismatch_fails_closed(self):
        source, binding, stage_map = inputs()
        stage_map["entries"][0]["source_name"] = "Different label"
        with self.assertRaisesRegex(core.ConformanceError, "stage mapping fingerprint conflicts"):
            adapter.map_order(source, binding, stage_map, NOW)

    def test_closed_source_stage_cannot_map_to_active_status(self):
        source, binding, stage_map = inputs()
        stage_map["entries"][0]["normalized_status"] = "in_progress"
        with self.assertRaisesRegex(core.ConformanceError, "closed source stage cannot map to an active"):
            adapter.map_order(source, binding, stage_map, NOW)

    def test_company_id_is_not_inferred_as_tenant(self):
        source, binding, stage_map = inputs()
        binding["source_company_id"] = "8"
        with self.assertRaisesRegex(core.ConformanceError, "OCA company does not match explicit tenant binding"):
            adapter.map_order(source, binding, stage_map, NOW)

    def test_wrong_source_instance_binding_is_rejected(self):
        source, binding, stage_map = inputs()
        binding["source_instance_namespace"] = "odoo-instance://other.invalid/other-db"
        with self.assertRaisesRegex(core.ConformanceError, "source instance mismatch"):
            adapter.map_order(source, binding, stage_map, NOW)

    def test_revoked_or_unavailable_tenant_binding_is_rejected(self):
        for status in ("revoked", "unavailable"):
            with self.subTest(status=status):
                source, binding, stage_map = inputs()
                binding["status"] = status
                with self.assertRaisesRegex(core.ConformanceError, "binding is not active"):
                    adapter.map_order(source, binding, stage_map, NOW)

    def test_closed_order_with_required_todo_activity_emits_conflict(self):
        source, binding, stage_map = inputs()
        source["activities"][0]["state"] = "todo"
        source["activities"][0]["completed_on"] = None
        source["activities"][0]["completed_by"] = None
        result = adapter.map_order(source, binding, stage_map, NOW)
        self.assertEqual(result["source_claims"]["required_activities_state"], "required_activity_incomplete")
        self.assertIn("source_stage_closed_with_required_activity_incomplete", result["service_case"]["mapping_conflicts"])
        self.assertEqual(result["service_case"]["normalized_status"], "closed")
        self.assertEqual(result["assurance"]["workmanship_quality"], "not_verified")

    def test_required_signature_missing_is_reported_as_conflict(self):
        source, binding, stage_map = inputs()
        source["order"]["signature_present"] = False
        source["order"]["signed_by_present"] = False
        source["order"]["signed_on"] = None
        result = adapter.map_order(source, binding, stage_map, NOW)
        self.assertIn("source_stage_requires_signature_but_signature_absent", result["service_case"]["mapping_conflicts"])
        self.assertEqual(result["assurance"]["customer_scope_approval"], "not_provided_by_source")

    def test_missing_activity_addon_is_explicit_not_silently_empty(self):
        source, binding, stage_map = inputs()
        source["source"]["optional_addons"] = []
        source["activity_addon_state"] = "not_installed"
        source["activities"] = []
        result = adapter.map_order(source, binding, stage_map, NOW)
        self.assertEqual(result["source_claims"]["required_activities_state"], "addon_not_installed")
        self.assertIn("activity_addon_not_installed", result["service_case"]["mapping_losses"])

    def test_incomplete_actual_interval_is_disclosed_not_guessed(self):
        source, binding, stage_map = inputs()
        source["order"]["date_end"] = None
        result = adapter.map_order(source, binding, stage_map, NOW)
        self.assertIn("partial_actual_work_interval", result["service_case"]["mapping_conflicts"])
        self.assertIsNone(result["work_order"]["actual_interval"]["end"])

    def test_inverted_time_window_is_rejected(self):
        source, binding, stage_map = inputs()
        source["order"]["scheduled_date_start"] = "2026-10-14T11:00:00Z"
        source["order"]["scheduled_date_end"] = "2026-10-14T10:00:00Z"
        with self.assertRaisesRegex(core.ConformanceError, "scheduled window ends before it starts"):
            adapter.map_order(source, binding, stage_map, NOW)

    def test_future_source_revision_is_rejected(self):
        source, binding, stage_map = inputs()
        source["order"]["write_date"] = "2026-10-16T00:00:00Z"
        with self.assertRaisesRegex(core.ConformanceError, "source revision is dated in the future"):
            adapter.map_order(source, binding, stage_map, NOW)

    def test_unsupported_source_commit_is_rejected(self):
        source, binding, stage_map = inputs()
        source["source"]["repository_commit"] = "0" * 40
        with self.assertRaisesRegex(core.ConformanceError, "unsupported OCA repository commit"):
            adapter.map_order(source, binding, stage_map, NOW)

    def test_optional_invoice_link_is_not_payment_or_quality_evidence(self):
        source, binding, stage_map = inputs()
        source["source"]["optional_addons"].append({
            "name": "fieldservice_account", "version": "19.0.1.0.0", "license": "AGPL-3", "maturity": "Beta"
        })
        source["invoice_addon_state"] = "exported"
        source["invoice_refs"] = ["INV-900"]
        result = adapter.map_order(source, binding, stage_map, NOW)
        self.assertEqual(result["source_claims"]["invoice_link_state"], "linked_references_present")
        self.assertEqual(result["source_claims"]["invoice_refs"], [
            "odoo-instance://fixture.invalid/plumbing-db|account.move|INV-900"
        ])
        self.assertEqual(result["assurance"]["invoice_payment"], "not_established")

    def test_unknown_source_fields_are_rejected(self):
        source, binding, stage_map = inputs()
        source["order"]["authority_granted"] = True
        with self.assertRaisesRegex(core.ConformanceError, "source schema error"):
            adapter.map_order(source, binding, stage_map, NOW)

    def test_evaluation_clock_requires_timezone(self):
        source, binding, stage_map = inputs()
        with self.assertRaisesRegex(core.ConformanceError, "evaluation time must include a timezone"):
            adapter.map_order(source, binding, stage_map, datetime(2026, 10, 15, 12, 0, 0))


if __name__ == "__main__":
    unittest.main()
