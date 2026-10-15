#!/usr/bin/env python3
"""Deterministic tests for the Luminous CloudEvents profile and sample payload."""

from __future__ import annotations

import copy
import json
import sys
import unittest
from pathlib import Path

from jsonschema import Draft202012Validator, FormatChecker

ROOT = Path(__file__).resolve().parents[1]
SCHEMA_PATH = ROOT / "schema.json"
FIXTURE_PATH = ROOT / "examples" / "incident.snapshot.json"
INCIDENT_PAYLOAD_SCHEMA_PATH = ROOT / "payloads" / "incident-snapshot-v1.schema.json"


def load_json(path: Path) -> object:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


class OperationsEventContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.schema = load_json(SCHEMA_PATH)
        cls.valid_event = load_json(FIXTURE_PATH)
        cls.incident_payload_schema = load_json(INCIDENT_PAYLOAD_SCHEMA_PATH)
        Draft202012Validator.check_schema(cls.schema)
        Draft202012Validator.check_schema(cls.incident_payload_schema)
        cls.validator = Draft202012Validator(
            cls.schema, format_checker=FormatChecker()
        )
        cls.incident_payload_validator = Draft202012Validator(
            cls.incident_payload_schema, format_checker=FormatChecker()
        )

    def assert_invalid(self, event: object, reason: str) -> None:
        with self.subTest(reason=reason):
            self.assertTrue(
                list(self.validator.iter_errors(event)),
                msg=f"invalid event accepted: {reason}",
            )

    def test_schemas_are_valid_draft_2020_12(self) -> None:
        Draft202012Validator.check_schema(self.schema)
        Draft202012Validator.check_schema(self.incident_payload_schema)

    def test_incident_snapshot_fixture_is_valid_cloudevent(self) -> None:
        self.assertEqual([], list(self.validator.iter_errors(self.valid_event)))

    def test_payload_schema_matches_dataschema(self) -> None:
        self.assertEqual(
            self.incident_payload_schema["$id"],
            self.valid_event["dataschema"],
        )
        self.assertEqual(
            [],
            list(self.incident_payload_validator.iter_errors(self.valid_event["data"])),
        )

    def test_reject_missing_tenant(self) -> None:
        event = copy.deepcopy(self.valid_event)
        del event["tenantid"]
        self.assert_invalid(event, "canonical tenant identity is required")

    def test_reject_malformed_correlation_id(self) -> None:
        event = copy.deepcopy(self.valid_event)
        event["correlationid"] = "bad id"
        self.assert_invalid(event, "correlation ID must be an opaque single-line ID")

    def test_reject_unknown_execution_like_field(self) -> None:
        event = copy.deepcopy(self.valid_event)
        event["execute_command"] = "reboot"
        self.assert_invalid(event, "unknown extension is rejected by this local profile")

    def test_reject_invalid_evidence_digest(self) -> None:
        event = copy.deepcopy(self.valid_event)
        event["data"]["evidence_refs"] = [{
            "artifact_id": "evidence-1",
            "sha256": "not-a-digest",
            "classification": "internal",
        }]
        self.assertTrue(
            list(self.incident_payload_validator.iter_errors(event["data"])),
            msg="invalid evidence digest was accepted",
        )

    def test_reject_unsupported_producer(self) -> None:
        event = copy.deepcopy(self.valid_event)
        event["producersystem"] = "unknown-trust-root"
        self.assert_invalid(event, "producer labels are not authentication credentials")

    def test_reject_invalid_dataschema(self) -> None:
        event = copy.deepcopy(self.valid_event)
        event["dataschema"] = "../../command"
        self.assert_invalid(event, "dataschema must resolve to the local versioned registry")

    def test_reject_unknown_actor_kind(self) -> None:
        event = copy.deepcopy(self.valid_event)
        event["actorkind"] = "ticket"
        self.assert_invalid(event, "resource type cannot impersonate an actor")

    def test_reject_unknown_incident_payload_field(self) -> None:
        payload = copy.deepcopy(self.valid_event["data"])
        payload["execute_command"] = "reboot"
        self.assertTrue(
            list(self.incident_payload_validator.iter_errors(payload)),
            msg="unknown incident payload property was accepted",
        )

    def test_reject_missing_incident_summary(self) -> None:
        payload = copy.deepcopy(self.valid_event["data"])
        del payload["summary"]
        self.assertTrue(
            list(self.incident_payload_validator.iter_errors(payload)),
            msg="summary is a required incident payload field",
        )

    def test_reject_invalid_observed_timestamp(self) -> None:
        event = copy.deepcopy(self.valid_event)
        event["observedat"] = "yesterday"
        self.assert_invalid(event, "observedat must be an RFC 3339 timestamp")

    def test_allowed_extension_attribute_names_follow_cloudevents_convention(self) -> None:
        core = {
            "specversion", "id", "source", "type", "subject", "time",
            "datacontenttype", "dataschema", "data"
        }
        extension_names = set(self.valid_event) - core
        self.assertTrue(extension_names)
        for name in extension_names:
            with self.subTest(name=name):
                self.assertRegex(name, r"^[a-z][a-z0-9]{0,19}$")
                self.assertNotEqual(name, "data")

    def test_reject_wrong_cloudevents_version(self) -> None:
        event = copy.deepcopy(self.valid_event)
        event["specversion"] = "2.0"
        self.assert_invalid(event, "this profile is CloudEvents 1.0")

    def test_reject_unknown_extension_wrapped_into_data(self) -> None:
        payload = copy.deepcopy(self.valid_event["data"])
        payload["trusted"] = True
        self.assertTrue(
            list(self.incident_payload_validator.iter_errors(payload)),
            msg="unknown payload property was accepted",
        )


if __name__ == "__main__":
    suite = unittest.defaultTestLoader.loadTestsFromTestCase(
        OperationsEventContractTests
    )
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    sys.exit(0 if result.wasSuccessful() else 1)
