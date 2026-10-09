"""Adversarial tests for the minimized Holochain share-package contract."""
from __future__ import annotations

import copy
import json
import unittest
from pathlib import Path

from jsonschema import Draft202012Validator, FormatChecker

ROOT = Path(__file__).resolve().parents[1]
SCHEMA_PATH = ROOT / "share-package.schema.json"
FIXTURE_PATH = ROOT / "examples" / "evidence-attestation.synthetic.json"


def load_json(path: Path) -> object:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


class HolochainSharePackageContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.schema = load_json(SCHEMA_PATH)
        cls.fixture = load_json(FIXTURE_PATH)
        Draft202012Validator.check_schema(cls.schema)
        cls.validator = Draft202012Validator(
            cls.schema, format_checker=FormatChecker()
        )

    def errors(self, instance: object) -> list:
        return list(self.validator.iter_errors(instance))

    def test_schema_is_valid_draft_2020_12(self) -> None:
        Draft202012Validator.check_schema(self.schema)

    def test_synthetic_fixture_is_valid(self) -> None:
        self.assertEqual([], self.errors(self.fixture))

    def test_rejects_unrecognized_free_text_or_ticket_fields(self) -> None:
        event = copy.deepcopy(self.fixture)
        event["ticketDescription"] = "synthetic customer details"
        self.assertTrue(self.errors(event))

    def test_rejects_raw_sensitive_data_declaration(self) -> None:
        event = copy.deepcopy(self.fixture)
        event["privacyReview"]["rawSensitiveDataIncluded"] = True
        self.assertTrue(self.errors(event))

    def test_rejects_unapproved_share_classification(self) -> None:
        event = copy.deepcopy(self.fixture)
        event["privacyReview"]["dataClass"] = "tenant-private"
        self.assertTrue(self.errors(event))

    def test_rejects_any_execution_authority(self) -> None:
        event = copy.deepcopy(self.fixture)
        event["authorityEffect"] = "remote-execution"
        self.assertTrue(self.errors(event))

    def test_dispute_requires_target_record_and_matching_statement(self) -> None:
        event = copy.deepcopy(self.fixture)
        event["recordType"] = "dispute"
        event["statementCode"] = "disputed"
        self.assertTrue(self.errors(event), "dispute without target must fail")
        event["targetShareId"] = self.fixture["shareId"]
        self.assertEqual([], self.errors(event))

    def test_supersession_requires_target_record_and_matching_statement(self) -> None:
        event = copy.deepcopy(self.fixture)
        event["recordType"] = "supersession"
        event["statementCode"] = "supersedes"
        self.assertTrue(self.errors(event), "supersession without target must fail")
        event["targetShareId"] = self.fixture["shareId"]
        self.assertEqual([], self.errors(event))

    def test_evidence_attestation_requires_at_least_one_manifest(self) -> None:
        event = copy.deepcopy(self.fixture)
        del event["evidenceManifest"]
        self.assertTrue(self.errors(event), "evidence attestation without a manifest must fail")
        event["evidenceManifest"] = []
        self.assertTrue(self.errors(event), "empty evidence manifest must fail")

    def test_incident_acknowledgement_may_omit_evidence_manifest(self) -> None:
        event = copy.deepcopy(self.fixture)
        event["recordType"] = "incident-acknowledgement"
        event["statementCode"] = "received"
        del event["evidenceManifest"]
        self.assertEqual([], self.errors(event))

    def test_record_type_and_statement_code_cannot_disagree(self) -> None:
        event = copy.deepcopy(self.fixture)
        event["recordType"] = "incident-acknowledgement"
        event["statementCode"] = "reviewed"
        self.assertTrue(self.errors(event))

    def test_rejects_malformed_evidence_digest(self) -> None:
        event = copy.deepcopy(self.fixture)
        event["evidenceManifest"][0]["digest"] = "not-a-sha256-digest"
        self.assertTrue(self.errors(event))

    def test_rejects_unknown_issuer_role_claim(self) -> None:
        event = copy.deepcopy(self.fixture)
        event["issuerRoleClaim"] = "platform-root-authority"
        self.assertTrue(self.errors(event))

    def test_rejects_invalid_timestamp(self) -> None:
        event = copy.deepcopy(self.fixture)
        event["issuedAt"] = "yesterday"
        self.assertTrue(self.errors(event))

    def test_limits_evidence_manifest_references(self) -> None:
        event = copy.deepcopy(self.fixture)
        event["evidenceManifest"] = event["evidenceManifest"] * 17
        self.assertTrue(self.errors(event))

    def test_schema_never_promotes_issuer_role_claim_to_authority(self) -> None:
        self.assertEqual("none", self.fixture["authorityEffect"])
        self.assertIn(
            "claim",
            self.schema["properties"]["issuerRoleClaim"]["description"].lower(),
        )
        self.assertIn(
            "authorization",
            self.schema["description"].lower(),
        )


if __name__ == "__main__":
    unittest.main()
