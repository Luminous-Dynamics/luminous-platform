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
        s, b = inputs(); s["log"].update(maintenance_status="Planned", docstatus=0, completion_date=None)
        out = adapter.map_maintenance_log(s, b, NOW)
        self.assertEqual(out["maintenance"]["normalized_status"], "scheduled")
        self.assertIsNone(out["maintenance"]["completion_date"])

    def test_source_ids_are_instance_and_doctype_namespaced(self):
        s, b = inputs(); out = adapter.map_maintenance_log(s, b, NOW)
        self.assertEqual(out["id"], "erpnext-instance://fixture.invalid/maintenance-db|Asset Maintenance Log|AML-SYN-001")

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
        self.assertIn("maintenance_status_cancelled_but_document_not_cancelled", out["maintenance"]["mapping_conflicts"])

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

    def test_certificate_requirement_vs_log_flag_is_reported(self):
        s, b = inputs(); s["log"]["has_certificate"] = False
        out = adapter.map_maintenance_log(s, b, NOW)
        self.assertIn("task_certificate_requirement_disagrees_with_log_flag", out["maintenance"]["mapping_conflicts"])

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
