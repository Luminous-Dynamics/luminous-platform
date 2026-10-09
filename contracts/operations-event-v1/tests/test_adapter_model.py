"""Adversarial tests for the synthetic PSA normalization and inbox model."""

from __future__ import annotations

import copy
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from adapter_model import (
    IdempotencyConflict,
    IngestionBoundary,
    MemoryInbox,
    NormalizationError,
    PsaIncidentSnapshot,
    TenantBindingError,
    TrustedPsaConnection,
    normalize_incident_snapshot,
)


class PsaAdapterModelTests(unittest.TestCase):
    def setUp(self) -> None:
        self.connection = TrustedPsaConnection(
            connection_id="connectwise-connection-demo",
            source_uri="urn:luminousdynamics:integration:connectwise-demo",
            company_tenant_map={"company-42": "tenant-demo-001"},
            incident_id_map={
                ("company-42", "7301"): "incident-local-7301",
                ("company-42", "7302"): "incident-local-7302",
            },
        )
        self.boundary = IngestionBoundary(
            connection_id=self.connection.connection_id,
            source_uri=self.connection.source_uri,
            company_tenant_map=self.connection.company_tenant_map,
            incident_id_map=self.connection.incident_id_map,
        )
        self.snapshot = PsaIncidentSnapshot(
            company_id="company-42",
            ticket_id="7301",
            revision="revision-1",
            status="new",
            redacted_summary="Synthetic test incident; no customer data",
            occurred_at="2026-10-09T08:00:00Z",
            priority=2,
        )
        self.event = normalize_incident_snapshot(
            self.snapshot, self.connection, observed_at="2026-10-09T08:00:02Z"
        )

    def test_normalization_is_deterministic(self) -> None:
        again = normalize_incident_snapshot(
            self.snapshot, self.connection, observed_at="2026-10-09T08:00:02Z"
        )
        self.assertEqual(self.event, again)

    def test_tenant_actor_source_come_from_trusted_connection(self) -> None:
        self.assertEqual(self.event["tenantid"], "tenant-demo-001")
        self.assertEqual(self.event["source"], self.connection.source_uri)
        self.assertEqual(self.event["actorid"], self.connection.connection_id)

    def test_external_company_and_revision_are_preserved(self) -> None:
        source_ref = self.event["data"]["source_reference"]
        self.assertEqual(source_ref["company_id"], "company-42")
        self.assertEqual(source_ref["revision"], "revision-1")

    def test_revision_change_creates_distinct_event_identity(self) -> None:
        newer = PsaIncidentSnapshot(
            company_id=self.snapshot.company_id,
            ticket_id=self.snapshot.ticket_id,
            revision="revision-2",
            status="open",
            redacted_summary=self.snapshot.redacted_summary,
            occurred_at="2026-10-09T08:01:00Z",
            priority=2,
        )
        updated = normalize_incident_snapshot(
            newer, self.connection, observed_at="2026-10-09T08:01:02Z"
        )
        self.assertNotEqual(self.event["id"], updated["id"])
        self.assertNotEqual(
            self.event["idempotencykey"], updated["idempotencykey"]
        )

    def test_unmapped_external_company_fails_closed(self) -> None
        snapshot = PsaIncidentSnapshot(
            company_id="company-attacker",
            ticket_id="7301",
            revision="revision-1",
            status="new",
            redacted_summary="synthetic",
        )
        with self.assertRaises(NormalizationError):
            normalize_incident_snapshot(
                snapshot, self.connection, observed_at="2026-10-09T08:00:02Z"
            )

    def test_unmapped_ticket_fails_closed(self) -> None:
        snapshot = PsaIncidentSnapshot(
            company_id="company-42",
            ticket_id="9999",
            revision="revision-1",
            status="new",
            redacted_summary="synthetic",
        )
        with self.assertRaises(NormalizationError):
            normalize_incident_snapshot(
                snapshot, self.connection, observed_at="2026-10-09T08:00:02Z"
            )

    def test_invalid_normalized_status_fails_closed(self) -> None:
        snapshot = copy.copy(self.snapshot)
        snapshot = PsaIncidentSnapshot(
            company_id=snapshot.company_id,
            ticket_id=snapshot.ticket_id,
            revision=snapshot.revision,
            status="execute",
            redacted_summary=snapshot.redacted_summary,
        )
        with self.assertRaises(NormalizationError):
            normalize_incident_snapshot(
                snapshot, self.connection, observed_at="2026-10-09T08:00:02Z"
            )

    def test_invalid_timestamp_fails_closed(self) -> None:
        with self.assertRaises(NormalizationError):
            normalize_incident_snapshot(
                self.snapshot, self.connection, observed_at="yesterday"
            )

    def test_missing_revision_fails_closed(self) -> None:
        snapshot = PsaIncidentSnapshot(
            company_id="company-42",
            ticket_id="7301",
            revision="",
            status="new",
            redacted_summary="synthetic",
        )
        with self.assertRaises(NormalizationError):
            normalize_incident_snapshot(
                snapshot, self.connection, observed_at="2026-10-09T08:00:02Z"
            )

    def test_idempotency_scope_includes_tenant(self) -> None:
        inbox = MemoryInbox()
        self.assertEqual(inbox.accept(self.event, self.boundary), "ACCEPTED")
        separate_tenant = copy.deepcopy(self.event)
        separate_tenant["id"] = "event-for-another-tenant"
        separate_tenant["source"] = "urn:luminousdynamics:integration:connectwise-tenant2"
        separate_tenant["tenantid"] = "tenant-demo-002"
        separate_tenant["actorid"] = "connectwise-connection-tenant2"
        separate_tenant["subject"] = "incident/incident-local-tenant2"
        separate_boundary = IngestionBoundary(
            connection_id="connectwise-connection-tenant2",
            source_uri="urn:luminousdynamics:integration:connectwise-tenant2",
            company_tenant_map={"company-42": "tenant-demo-002"},
            incident_id_map={
                ("company-42", "7301"): "incident-local-tenant2",
            },
        )
        self.assertEqual(inbox.accept(separate_tenant, separate_boundary), "ACCEPTED")

    def test_first_event_is_accepted(self) -> None:
        inbox = MemoryInbox()
        self.assertEqual(inbox.accept(self.event, self.boundary), "ACCEPTED")

    def test_exact_replay_is_duplicate(self) -> None:
        inbox = MemoryInbox()
        self.assertEqual(inbox.accept(self.event, self.boundary), "ACCEPTED")
        self.assertEqual(inbox.accept(self.event, self.boundary), "DUPLICATE")

    def test_same_event_redelivered_with_later_observation_time_is_duplicate(self) -> None:
        inbox = MemoryInbox()
        self.assertEqual(inbox.accept(self.event, self.boundary), "ACCEPTED")
        redelivery = copy.deepcopy(self.event)
        redelivery["observedat"] = "2026-10-09T08:05:00Z"
        self.assertEqual(inbox.accept(redelivery, self.boundary), "DUPLICATE")

    def test_same_source_and_id_with_changed_content_conflicts(self) -> None:
        inbox = MemoryInbox()
        self.assertEqual(inbox.accept(self.event, self.boundary), "ACCEPTED")
        changed = copy.deepcopy(self.event)
        changed["data"]["summary"] = "different summary"
        with self.assertRaises(IdempotencyConflict):
            inbox.accept(changed, self.boundary)

    def test_reused_business_key_with_changed_effect_conflicts(self) -> None:
        inbox = MemoryInbox()
        self.assertEqual(inbox.accept(self.event, self.boundary), "ACCEPTED")
        changed = copy.deepcopy(self.event)
        changed["id"] = "different-delivery-id"
        changed["data"]["summary"] = "different summary"
        with self.assertRaises(IdempotencyConflict):
            inbox.accept(changed, self.boundary)

    def test_same_business_effect_with_new_delivery_id_is_duplicate(self) -> None:
        inbox = MemoryInbox()
        self.assertEqual(inbox.accept(self.event, self.boundary), "ACCEPTED")
        redelivery = copy.deepcopy(self.event)
        redelivery["id"] = "new-delivery-id"
        redelivery["observedat"] = "2026-10-09T08:01:00Z"
        self.assertEqual(inbox.accept(redelivery, self.boundary), "DUPLICATE")
        self.assertEqual(inbox.accept(redelivery, self.boundary), "DUPLICATE")

    def test_wrong_tenant_binding_rejected(self) -> None:
        inbox = MemoryInbox()
        wrong_tenant = copy.deepcopy(self.event)
        wrong_tenant["tenantid"] = "tenant-victim-002"
        with self.assertRaises(TenantBindingError):
            inbox.accept(wrong_tenant, self.boundary)

    def test_wrong_local_resource_mapping_rejected(self) -> None:
        inbox = MemoryInbox()
        wrong_resource = copy.deepcopy(self.event)
        wrong_resource["subject"] = "incident/another-local-incident"
        with self.assertRaises(TenantBindingError):
            inbox.accept(wrong_resource, self.boundary)

    def test_unmapped_company_in_event_rejected_at_receiver(self) -> None:
        inbox = MemoryInbox()
        wrong_company = copy.deepcopy(self.event)
        wrong_company["data"]["source_reference"]["company_id"] = "company-unmapped"
        with self.assertRaises(TenantBindingError):
            inbox.accept(wrong_company, self.boundary)

    def test_source_reference_system_must_match_authenticated_adapter(self) -> None:
        inbox = MemoryInbox()
        wrong_system = copy.deepcopy(self.event)
        wrong_system["data"]["source_reference"]["system"] = "other-psa"
        with self.assertRaises(TenantBindingError):
            inbox.accept(wrong_system, self.boundary)

    def test_source_reference_resource_type_must_be_service_ticket(self) -> None:
        inbox = MemoryInbox()
        wrong_resource_type = copy.deepcopy(self.event)
        wrong_resource_type["data"]["source_reference"]["resource_type"] = "user"
        with self.assertRaises(TenantBindingError):
            inbox.accept(wrong_resource_type, self.boundary)

    def test_wrong_source_binding_rejected(self) -> None:
        inbox = MemoryInbox()
        wrong_source = copy.deepcopy(self.event)
        wrong_source["source"] = "urn:untrusted:other-connector"
        with self.assertRaises(TenantBindingError):
            inbox.accept(wrong_source, self.boundary)

    def test_wrong_actor_binding_rejected(self) -> None:
        inbox = MemoryInbox()
        wrong_actor = copy.deepcopy(self.event)
        wrong_actor["actorid"] = "different-connection"
        with self.assertRaises(TenantBindingError):
            inbox.accept(wrong_actor, self.boundary)

    def test_unknown_execution_field_in_data_rejected(self) -> None:
        inbox = MemoryInbox()
        bad_event = copy.deepcopy(self.event)
        bad_event["data"]["execute_command"] = "reboot"
        with self.assertRaises(NormalizationError):
            inbox.accept(bad_event, self.boundary)


if __name__ == "__main__":
    unittest.main(verbosity=2)
