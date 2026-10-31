import unittest

from fault_campaign import BENCH, run_campaign


class RecoveryFaultCampaignTests(unittest.TestCase):
    def test_all_declared_scenarios_match_expected_verifier_outcome(self):
        report = run_campaign(BENCH / "fault-matrix.json")
        self.assertEqual(report["status"], "pass", report["cases"])
        self.assertEqual(report["case_count"], 15)
        self.assertEqual(report["failed_case_count"], 0)

    def test_campaign_is_deterministic_for_pinned_inputs(self):
        first = run_campaign(BENCH / "fault-matrix.json")
        second = run_campaign(BENCH / "fault-matrix.json")
        self.assertEqual(first["report_sha256"], second["report_sha256"])
        self.assertEqual(first["matrix_sha256"], second["matrix_sha256"])
        self.assertEqual(first["verifier_source_sha256"], second["verifier_source_sha256"])

    def test_missing_service_evidence_is_counted_as_indeterminate(self):
        report = run_campaign(BENCH / "fault-matrix.json")
        indeterminate = [r for r in report["cases"] if r["expected_status"] == "indeterminate"]
        self.assertGreaterEqual(len(indeterminate), 2)
        self.assertTrue(all(r["actual_status"] == "indeterminate" for r in indeterminate))

    def test_critical_faults_remain_invalid_not_averaged_away(self):
        report = run_campaign(BENCH / "fault-matrix.json")
        invalid = [r for r in report["cases"] if r["expected_status"] == "invalid"]
        self.assertGreaterEqual(len(invalid), 8)
        self.assertTrue(all(r["actual_status"] == "invalid" for r in invalid))


if __name__ == "__main__":
    unittest.main()
