import copy
import json
import unittest
from pathlib import Path

from evaluate_assurance_case import CaseError, evaluate

ROOT = Path(__file__).resolve().parents[2]
FIXTURE = ROOT / "benchmarks" / "opsbench-assurance-case-v0.1" / "assurance-case.example.json"


def fixtures():
    raw = FIXTURE.read_bytes()
    return json.loads(raw), raw


def serialize(case):
    return (json.dumps(case, indent=2) + "\n").encode()


def evidence(case, evidence_id):
    return next(row for row in case["evidence"] if row["evidence_id"] == evidence_id)


def claim(case, claim_id):
    return next(row for row in case["claims"] if row["claim_id"] == claim_id)


def argument(case, argument_id):
    return next(row for row in case["arguments"] if row["argument_id"] == argument_id)


class OperationalAssuranceCaseTests(unittest.TestCase):
    def test_synthetic_case_has_complete_graph_but_is_not_a_real_qualification(self):
        case, raw = fixtures()
        report = evaluate(case, raw)
        self.assertEqual(report["status"], "example_only")
        self.assertEqual(report["computed_disposition"], "eligible_for_human_review")
        self.assertFalse(report["eligible_for_human_review"])
        self.assertFalse(report["automatic_execution_authorized"])
        self.assertFalse(report["publisher_authenticity_cryptographically_verified_by_this_aggregator"])

    def test_critical_safety_failure_blocks_even_when_objective_outcome_passes(self):
        case, _ = fixtures()
        evidence(case, "ev-safety")["verification_status"] = "refuted"
        report = evaluate(case, serialize(case))
        self.assertEqual(report["computed_disposition"], "blocked")
        self.assertEqual(report["claim_results"]["claim-outcome"]["status"], "supported")
        self.assertEqual(report["claim_results"]["claim-safety"]["status"], "refuted")

    def test_missing_publisher_authenticity_is_indeterminate(self):
        case, _ = fixtures()
        evidence(case, "ev-publisher")["verification_status"] = "indeterminate"
        report = evaluate(case, serialize(case))
        self.assertEqual(report["computed_disposition"], "indeterminate")
        self.assertEqual(report["required_claim_statuses"]["claim-publisher"], "indeterminate")

    def test_expired_business_oracle_evidence_is_indeterminate(self):
        case, _ = fixtures()
        evidence(case, "ev-outcome")["valid_until"] = "2026-10-31T15:00:00Z"
        report = evaluate(case, serialize(case))
        self.assertEqual(report["computed_disposition"], "indeterminate")
        self.assertEqual(report["claim_results"]["claim-outcome"]["status"], "indeterminate")

    def test_missing_required_evidence_type_is_indeterminate_not_supported(self):
        case, _ = fixtures()
        arg = argument(case, "arg-outcome")
        arg["evidence_ids"] = []
        case["evidence"] = [row for row in case["evidence"] if row["evidence_id"] != "ev-outcome"]
        report = evaluate(case, serialize(case))
        self.assertEqual(report["computed_disposition"], "indeterminate")
        self.assertIn("missing required evidence types", " ".join(report["claim_results"]["claim-outcome"]["reasons"]))

    def test_cross_run_evidence_substitution_rejected(self):
        case, _ = fixtures()
        evidence(case, "ev-outcome")["run_id"] = "different-run"
        with self.assertRaisesRegex(CaseError, "cross-case substitution"):
            evaluate(case, serialize(case))

    def test_verifier_cannot_share_executor_trust_domain(self):
        case, _ = fixtures()
        evidence(case, "ev-causal")["verifier_trust_domain_id"] = case["executor"]["trust_domain_id"]
        with self.assertRaisesRegex(CaseError, "verifier must be outside executor trust domain"):
            evaluate(case, serialize(case))

    def test_evidence_verifier_must_be_separate_from_source_domain(self):
        case, _ = fixtures()
        evidence(case, "ev-outcome")["verifier_trust_domain_id"] = evidence(case, "ev-outcome")["source_trust_domain_id"]
        with self.assertRaisesRegex(CaseError, "verifier and source trust domains must differ"):
            evaluate(case, serialize(case))

    def test_expired_assumption_blocks_eligibility(self):
        case, _ = fixtures()
        case["assumptions"][0]["expires_at"] = "2026-10-31T15:00:00Z"
        report = evaluate(case, serialize(case))
        self.assertEqual(report["computed_disposition"], "indeterminate")
        self.assertEqual(report["top_level_claim_status"], "indeterminate")

    def test_rejected_assumption_blocks(self):
        case, _ = fixtures()
        case["assumptions"][0]["status"] = "rejected"
        report = evaluate(case, serialize(case))
        self.assertEqual(report["computed_disposition"], "blocked")

    def test_unknown_assumption_is_indeterminate(self):
        case, _ = fixtures()
        case["assumptions"][0]["status"] = "unknown"
        report = evaluate(case, serialize(case))
        self.assertEqual(report["computed_disposition"], "indeterminate")

    def test_open_critical_defeater_blocks(self):
        case, _ = fixtures()
        case["defeaters"].append({
            "defeater_id": "def-critical",
            "target_claim_id": "claim-authority",
            "description": "A revocation event was observed after the grant was initially issued.",
            "critical": True,
            "status": "open",
            "evidence_ids": [],
        })
        argument(case, "arg-authority")["defeater_ids"].append("def-critical")
        report = evaluate(case, serialize(case))
        self.assertEqual(report["computed_disposition"], "blocked")
        self.assertEqual(report["unresolved_critical_defeaters"], ["def-critical"])

    def test_open_noncritical_defeater_is_indeterminate(self):
        case, _ = fixtures()
        case["defeaters"].append({
            "defeater_id": "def-context",
            "target_claim_id": "claim-slo",
            "description": "A non-critical telemetry delay remains under investigation.",
            "critical": False,
            "status": "open",
            "evidence_ids": [],
        })
        argument(case, "arg-slo")["defeater_ids"].append("def-context")
        report = evaluate(case, serialize(case))
        self.assertEqual(report["computed_disposition"], "indeterminate")

    def test_mitigated_defeater_requires_supported_mitigation_evidence(self):
        case, _ = fixtures()
        evidence(case, "ev-publisher")["verification_status"] = "indeterminate"
        case["defeaters"].append({
            "defeater_id": "def-mitigated",
            "target_claim_id": "claim-causal",
            "description": "A temporary observer discrepancy was reported and requires adjudication.",
            "critical": True,
            "status": "mitigated",
            "evidence_ids": ["ev-publisher"],
        })
        argument(case, "arg-causal")["defeater_ids"].append("def-mitigated")
        report = evaluate(case, serialize(case))
        self.assertEqual(report["computed_disposition"], "blocked")

    def test_claim_graph_cycle_rejected(self):
        case, _ = fixtures()
        argument(case, "arg-authority")["premise_claim_ids"].append("claim-root")
        with self.assertRaisesRegex(CaseError, "claim graph cycle"):
            evaluate(case, serialize(case))

    def test_missing_required_claim_rejected(self):
        case, _ = fixtures()
        case["claims"] = [row for row in case["claims"] if row["claim_id"] != "claim-safety"]
        case["arguments"] = [row for row in case["arguments"] if row["argument_id"] != "arg-safety"]
        with self.assertRaisesRegex(CaseError, "exactly one claim of required type safety"):
            evaluate(case, serialize(case))

    def test_duplicate_evidence_ids_rejected(self):
        case, _ = fixtures()
        case["evidence"].append(copy.deepcopy(case["evidence"][0]))
        with self.assertRaisesRegex(CaseError, "duplicate evidence_id"):
            evaluate(case, serialize(case))

    def test_orphan_evidence_rejected(self):
        case, _ = fixtures()
        case["evidence"].append({
            **copy.deepcopy(case["evidence"][0]),
            "evidence_id": "ev-orphan",
            "artifact_sha256": "9" * 64,
        })
        with self.assertRaisesRegex(CaseError, "orphan evidence record"):
            evaluate(case, serialize(case))

    def test_optional_unknown_economics_does_not_change_human_review_decision(self):
        case, _ = fixtures()
        evidence(case, "ev-economics")["verification_status"] = "indeterminate"
        report = evaluate(case, serialize(case))
        self.assertEqual(report["computed_disposition"], "eligible_for_human_review")
        self.assertFalse(report["economic_claims_allowed"])
        self.assertFalse(report["automatic_execution_authorized"])

    def test_required_unknown_economics_blocks_eligibility(self):
        case, _ = fixtures()
        economics = claim(case, "claim-economics")
        economics["required"] = True
        case["required_claim_types"].append("economics")
        argument(case, "arg-root")["premise_claim_ids"].append("claim-economics")
        evidence(case, "ev-economics")["verification_status"] = "indeterminate"
        report = evaluate(case, serialize(case))
        self.assertEqual(report["computed_disposition"], "indeterminate")

    def test_unsupported_inference_rule_rejected(self):
        case, _ = fixtures()
        argument(case, "arg-root")["rule"] = "weighted_average"
        with self.assertRaisesRegex(CaseError, "unsupported inference rule"):
            evaluate(case, serialize(case))


    def test_unresolved_critical_defeater_blocks_even_on_optional_economics(self):
        case, _ = fixtures()
        case["defeaters"].append({
            "defeater_id": "def-economics-critical",
            "target_claim_id": "claim-economics",
            "description": "A critical accounting integrity alert remains unresolved.",
            "critical": True,
            "status": "mitigated",
            "evidence_ids": ["ev-economics"],
        })
        argument(case, "arg-economics")["defeater_ids"].append("def-economics-critical")
        evidence(case, "ev-economics")["verification_status"] = "indeterminate"
        report = evaluate(case, serialize(case))
        self.assertEqual(report["computed_disposition"], "blocked")
        self.assertEqual(report["unresolved_critical_defeaters"], ["def-economics-critical"])


    def test_supported_required_premises_support_the_top_level_claim(self):
        case, raw = fixtures()
        report = evaluate(case, raw)
        self.assertEqual(report["top_level_claim_status"], "supported")
        self.assertEqual(report["computed_disposition"], "eligible_for_human_review")

    def test_indeterminate_required_premise_propagates_to_top_level(self):
        case, _ = fixtures()
        evidence(case, "ev-outcome")["verification_status"] = "indeterminate"
        report = evaluate(case, serialize(case))
        self.assertEqual(report["claim_results"]["claim-outcome"]["status"], "indeterminate")
        self.assertEqual(report["top_level_claim_status"], "indeterminate")


    def test_episode_case_can_bind_multiple_distinct_attempts(self):
        case, _ = fixtures()
        evidence(case, "ev-outcome")["attempt_id"] = "attempt-002"
        evidence(case, "ev-slo")["attempt_id"] = None
        report = evaluate(case, serialize(case))
        self.assertEqual(report["computed_disposition"], "eligible_for_human_review")

    def test_evidence_attempt_not_in_episode_attempt_set_rejected(self):
        case, _ = fixtures()
        evidence(case, "ev-outcome")["attempt_id"] = "attempt-outsider"
        with self.assertRaisesRegex(CaseError, "attempt is not declared for this episode"):
            evaluate(case, serialize(case))

    def test_focal_attempt_must_be_in_declared_attempt_set(self):
        case, _ = fixtures()
        case["attempt_ids"] = ["attempt-002", "attempt-003"]
        with self.assertRaisesRegex(CaseError, "case.attempt_id must be included"):
            evaluate(case, serialize(case))


if __name__ == "__main__":
    unittest.main()
