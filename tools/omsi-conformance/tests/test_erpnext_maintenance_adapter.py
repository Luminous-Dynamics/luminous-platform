import sys
import unittest
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
import omsi_validate as core
import erpnext_maintenance_adapter as adapter

FIXTURES = ROOT / "fixtures"
NOW = datetime(2026, 10, 15, 12, 0, 0, tzinfo=timezone.utc)

def fixture(name):
    return core.load(FIXTURES / name)

def inputs():
    return fixture("erpnext-maintenance-log-snapshot.json"), fixture("erpnext-tenant-binding.json")

class TestErpnextMaintenanceLogAdapter(unittest.TestCase):
    def test_completed_log_maps_closed_but_not_verified(self):
        s, b = inputs(); out = adapter.map_maintenance_log(s, b, NOW)
        self.assertEqual(out["maintenance"]["normalized_status"], "closed")
        self.assertEqual(out["assurance"]["workmanship_quality"], "not_verified")

    def test_date_only_completion_is_preserved(self):
        s, b = inputs(); out = adapter.map_maintenance_log(s, b, NOW)
        self.assertEqual(out["maintenance"]["completion_date"], "2026-10-14")
        self.assertNotIn("completion_timestamp", out["maintenance"])

    def test_document_status_and_maintenance_status_remain_distinct(self):
        s, b = inputs(); out = adapter.map_maintenance_log(s, b, NOW)
        self.assertEqual(out["maintenance"]["source_docstatus"], 1)
        self.assertEqual(out["maintenance"]["source_status"], "Completed")

    def test_future_due_date_is_allowed(self):
        s, b = inputs(); s["log"]["due_date"] = "2026-12-01"
        self.assertEqual(adapter.map_maintenance_log(s, b, NOW)["maintenance"]["due_date"], "2026-12-01")

    def test_planned_future_maintenance_is_not_completed(self):
        s, b = inputs()
        s["log"].update(maintenance_status="Planned", docstatus=0, completion_date=None, due_date="2026-10-20")
        out = adapter.map_maintenance_log(s, b, NOW)
        self.assertEqual(out["maintenance"]["normalized_status"], "scheduled")
        self.assertEqual(out["maintenance"]["due_date"], "2026-10-20")
        self.assertIsNone(out["maintenance"]["completion_date"])

    def test_stale_planned_status_with_past_due_date_is_disclosed_as_conflict(self):
        s, b = inputs()
        s["log"].update(maintenance_status="Planned", docstatus=0, completion_date=None, due_date="2026-10-14")
        out = adapter.map_maintenance_log(s, b, NOW)
        self.assertEqual(out["maintenance"]["normalized_status"], "scheduled")
        self.assertIn("planned_status_with_past_due_date", out["maintenance"]["mapping_conflicts"])

    def test_source_ids_are_instance_and_doctype_namespaced(self):
        s, b = inputs(); out = adapter.map_maintenance_log(s, b, NOW)
        self.assertEqual(out["id"], "erpnext-instance://fixture.invalid/maintenance-db|Asset Maintenance Log|AML-SYN-001")

    def test_external_id_escapes_delimiters_in_source_controlled_components(self):
        def mapped_id(instance, record_name):
            s, b = inputs()
            s["source"]["instance_namespace"] = instance
            b["source_instance_namespace"] = instance
            s["log"]["name"] = record_name
            return adapter.map_maintenance_log(s, b, NOW)["id"]

        # These two tuples collided under raw pipe-joining. Keep each component unambiguous.
        first = mapped_id(
            "erpnext-instance://fixture.invalid/maintenance-db",
            "AML-SYN-001|Asset Maintenance Log|CHILD",
        )
        second = mapped_id(
            "erpnext-instance://fixture.invalid/maintenance-db|Asset Maintenance Log|AML-SYN-001",
            "CHILD",
        )
        self.assertNotEqual(first, second)
        self.assertIn("%7C", first)
        self.assertIn("%7C", second)

    def test_company_mismatch_rejected(self):
        s, b = inputs(); b["source_company_id"] = "OTHER-COMPANY"
        with self.assertRaisesRegex(core.ConformanceError, "company does not match explicit tenant binding"):
            adapter.map_maintenance_log(s, b, NOW)

    def test_instance_mismatch_rejected(self):
        s, b = inputs(); b["source_instance_namespace"] = "erpnext-instance://other.invalid/db"
        with self.assertRaisesRegex(core.ConformanceError, "source instance mismatch"):
            adapter.map_maintenance_log(s, b, NOW)

    def test_inactive_binding_rejected(self):
        s, b = inputs(); b["status"] = "revoked"
        with self.assertRaisesRegex(core.ConformanceError, "binding is not active"):
            adapter.map_maintenance_log(s, b, NOW)

    def test_unsupported_repository_commit_rejected(self):
        s, b = inputs(); s["source"]["repository_commit"] = "0" * 40
        with self.assertRaisesRegex(core.ConformanceError, "unsupported ERPNext source revision"):
            adapter.map_maintenance_log(s, b, NOW)

    def test_changed_license_indicator_requires_review(self):
        s, b = inputs(); s["source"]["license"] = "AGPL-3"
        with self.assertRaisesRegex(core.ConformanceError, "source license indicator changed"):
            adapter.map_maintenance_log(s, b, NOW)

    def test_future_export_timestamp_rejected(self):
        s, b = inputs(); s["exported_at"] = "2026-10-16T00:00:00Z"
        with self.assertRaisesRegex(core.ConformanceError, "export is dated in the future"):
            adapter.map_maintenance_log(s, b, NOW)

    def test_site_timezone_controls_date_only_future_check(self):
        s, b = inputs()
        b["site_timezone"] = "Pacific/Kiritimati"
        # 2026-10-16 is not future in the configured UTC+14 site at this instant.
        s["log"]["completion_date"] = "2026-10-16"
        out = adapter.map_maintenance_log(s, b, NOW)
        self.assertEqual(out["maintenance"]["completion_date"], "2026-10-16")

    def test_invalid_site_timezone_rejected(self):
        s, b = inputs(); b["site_timezone"] = "Mars/Olympus_Mons"
        with self.assertRaisesRegex(core.ConformanceError, "not a valid IANA timezone"):
            adapter.map_maintenance_log(s, b, NOW)

    def test_mapping_context_records_binding_timezone_and_utc_evaluation_instant(self):
        s, b = inputs()
        b["site_timezone"] = "Pacific/Kiritimati"
        out = adapter.map_maintenance_log(s, b, NOW)
        self.assertEqual(out["mapping_context"], {
            "adapter_id": "org.omsi.erpnext-maintenance-adapter/0.1",
            "binding_id": "erpnext-binding-syn-01",
            "site_timezone": "Pacific/Kiritimati",
            "evaluated_at": "2026-10-15T12:00:00Z",
        })

    def test_same_inputs_and_evaluation_clock_produce_identical_projection(self):
        s1, b1 = inputs()
        s2, b2 = inputs()
        out1 = adapter.map_maintenance_log(s1, b1, NOW)
        out2 = adapter.map_maintenance_log(s2, b2, NOW)
        self.assertEqual(core.canonical_bytes(out1), core.canonical_bytes(out2))

    def test_overdue_status_remains_overdue_not_blocked(self):
        s, b = inputs()
        s["log"].update(maintenance_status="Overdue", docstatus=0, completion_date=None)
        out = adapter.map_maintenance_log(s, b, NOW)
        self.assertEqual(out["maintenance"]["normalized_status"], "overdue")

    def test_task_next_due_date_is_preserved_separately_from_log_due_date(self):
        s, b = inputs(); out = adapter.map_maintenance_log(s, b, NOW)
        self.assertEqual(out["maintenance"]["due_date"], "2026-10-14")
        self.assertEqual(out["maintenance"]["next_task_due_date"], "2026-11-14")
        self.assertNotEqual(out["maintenance"]["due_date"], out["maintenance"]["next_task_due_date"])

    def test_task_last_completion_date_is_not_reconstructed_as_log_history(self):
        s, b = inputs()
        # The pinned controller overwrites the task date when a different log
        # is submitted; it does not compare calendar ordering. A later-submitted
        # historical log can therefore move this source field backward.
        s["task"]["last_completion_date"] = "2026-10-13"
        out = adapter.map_maintenance_log(s, b, NOW)
        self.assertEqual(out["maintenance"]["completion_date"], "2026-10-14")
        self.assertEqual(out["maintenance"]["last_task_completion_date"], "2026-10-13")
        self.assertIn(
            "task_last_completion_date_is_not_reconstructed_from_event_history",
            out["maintenance"]["mapping_losses"],
        )

    def test_future_completion_date_rejected(self):
        s, b = inputs(); s["log"]["completion_date"] = "2026-10-16"
        with self.assertRaisesRegex(core.ConformanceError, "completion date is in the future"):
            adapter.map_maintenance_log(s, b, NOW)

    def test_future_task_last_completion_date_rejected(self):
        s, b = inputs(); s["task"]["last_completion_date"] = "2026-10-17"
        with self.assertRaisesRegex(core.ConformanceError, "last-completion date is in the future"):
            adapter.map_maintenance_log(s, b, NOW)

    def test_completed_requires_completion_date(self):
        s, b = inputs(); s["log"]["completion_date"] = None
        with self.assertRaisesRegex(core.ConformanceError, "Completed log requires a completion date"):
            adapter.map_maintenance_log(s, b, NOW)

    def test_non_completed_cannot_have_completion_date(self):
        s, b = inputs(); s["log"].update(maintenance_status="Planned", docstatus=0)
        with self.assertRaisesRegex(core.ConformanceError, "non-completed log cannot have a completion date"):
            adapter.map_maintenance_log(s, b, NOW)

    def test_submitted_log_must_be_completed_or_cancelled(self):
        s, b = inputs(); s["log"].update(maintenance_status="Planned", docstatus=1, completion_date=None)
        with self.assertRaisesRegex(core.ConformanceError, "submitted log must be Completed or Cancelled"):
            adapter.map_maintenance_log(s, b, NOW)

    def test_document_cancellation_with_completed_source_status_is_disclosed(self):
        s, b = inputs(); s["log"]["docstatus"] = 2
        out = adapter.map_maintenance_log(s, b, NOW)
        self.assertEqual(out["maintenance"]["normalized_status"], "cancelled")
        self.assertIn("document_cancelled_but_maintenance_status_not_cancelled", out["maintenance"]["mapping_conflicts"])

    def test_cancelled_source_status_without_cancelled_docstatus_is_disclosed(self):
        s, b = inputs(); s["log"].update(maintenance_status="Cancelled", docstatus=0, completion_date=None)
        out = adapter.map_maintenance_log(s, b, NOW)
        self.assertEqual(out["maintenance"]["normalized_status"], "cancelled")
        self.assertIn("maintenance_status_cancelled_but_document_not_submitted", out["maintenance"]["mapping_conflicts"])

    def test_submitted_cancelled_maintenance_status_is_valid_without_doc_cancellation(self):
        s, b = inputs()
        s["log"].update(maintenance_status="Cancelled", docstatus=1, completion_date=None)
        out = adapter.map_maintenance_log(s, b, NOW)
        self.assertEqual(out["maintenance"]["normalized_status"], "cancelled")
        self.assertEqual(out["maintenance"]["source_docstatus"], 1)
        self.assertEqual(out["maintenance"]["source_status"], "Cancelled")
        self.assertNotIn(
            "maintenance_status_cancelled_but_document_not_submitted",
            out["maintenance"]["mapping_conflicts"],
        )
        self.assertNotIn(
            "document_cancelled_but_maintenance_status_not_cancelled",
            out["maintenance"]["mapping_conflicts"],
        )

    def test_plan_task_relationship_mismatch_rejected(self):
        s, b = inputs(); s["plan"]["task_id"] = "another-task"
        with self.assertRaisesRegex(core.ConformanceError, "plan/task reference mismatch"):
            adapter.map_maintenance_log(s, b, NOW)

    def test_log_plan_relationship_mismatch_rejected(self):
        s, b = inputs(); s["log"]["asset_maintenance_id"] = "another-plan"
        with self.assertRaisesRegex(core.ConformanceError, "log/plan reference mismatch"):
            adapter.map_maintenance_log(s, b, NOW)

    def test_log_task_relationship_mismatch_rejected(self):
        s, b = inputs(); s["log"]["task_id"] = "another-task"
        with self.assertRaisesRegex(core.ConformanceError, "log/task reference mismatch"):
            adapter.map_maintenance_log(s, b, NOW)

    def test_historical_log_certificate_flag_may_differ_from_current_task_requirement(self):
        s, b = inputs(); s["log"]["has_certificate"] = False
        out = adapter.map_maintenance_log(s, b, NOW)
        self.assertIn(
            "current_task_certificate_requirement_may_postdate_historical_log",
            out["maintenance"]["mapping_losses"],
        )
        self.assertNotIn(
            "active_log_certificate_flag_disagrees_with_task_requirement",
            out["maintenance"]["mapping_conflicts"],
        )

    def test_active_log_certificate_flag_mismatch_is_conflict(self):
        s, b = inputs()
        s["log"].update(
            maintenance_status="Planned", docstatus=0, completion_date=None,
            due_date="2026-10-20", has_certificate=False,
        )
        out = adapter.map_maintenance_log(s, b, NOW)
        self.assertIn(
            "active_log_certificate_flag_disagrees_with_task_requirement",
            out["maintenance"]["mapping_conflicts"],
        )

    def test_certificate_flag_does_not_prove_attachment_exists(self):
        s, b = inputs(); s["log"]["certificate_attachment_present"] = False
        out = adapter.map_maintenance_log(s, b, NOW)
        self.assertIn("certificate_flag_without_attachment_reference", out["maintenance"]["mapping_conflicts"])
        self.assertEqual(out["assurance"]["certificate_authenticity"], "not_established")

    def test_required_missing_certificate_is_a_conflict(self):
        s, b = inputs(); s["log"]["certificate_attachment_present"] = False
        out = adapter.map_maintenance_log(s, b, NOW)
        self.assertIn("required_certificate_attachment_missing", out["maintenance"]["mapping_conflicts"])

    def test_raw_action_text_and_attachment_bytes_are_not_exported(self):
        s, b = inputs(); out = adapter.map_maintenance_log(s, b, NOW)
        self.assertFalse(out["privacy"]["raw_actions_performed_exported"])
        self.assertFalse(out["privacy"]["certificate_bytes_exported"])
        self.assertNotIn("actions_performed", out)
        self.assertNotIn("certificate_attachment_ref", out)

    def test_assignment_is_not_a_credential_claim(self):
        s, b = inputs(); out = adapter.map_maintenance_log(s, b, NOW)
        self.assertTrue(out["source_claims"]["assigned_user_present"])
        self.assertEqual(out["assurance"]["practitioner_credential_status"], "not_provided_by_source")

    def test_unknown_source_fields_rejected(self):
        s, b = inputs(); s["log"]["authority_granted"] = True
        with self.assertRaisesRegex(core.ConformanceError, "snapshot schema error"):
            adapter.map_maintenance_log(s, b, NOW)

    def test_evaluation_clock_requires_timezone(self):
        s, b = inputs()
        with self.assertRaisesRegex(core.ConformanceError, "evaluation clock must include a timezone"):
            adapter.map_maintenance_log(s, b, datetime(2026, 10, 15, 12, 0, 0))

    def test_source_license_and_revision_are_preserved(self):
        s, b = inputs(); out = adapter.map_maintenance_log(s, b, NOW)
        self.assertEqual(out["source"]["repository_commit"], adapter.PINNED_COMMIT)
        self.assertEqual(out["source"]["license"], "GPL-3.0")

    def test_future_tenant_binding_configuration_rejected(self):
        s, b = inputs(); b["configured_at"] = "2026-10-15T12:00:01Z"
        with self.assertRaisesRegex(core.ConformanceError, "configuration is dated in the future"):
            adapter.map_maintenance_log(s, b, NOW)


if __name__ == "__main__":
    unittest.main()
