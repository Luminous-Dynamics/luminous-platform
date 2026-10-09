"""Replay, transaction fault-injection, tenant binding and outbox tests."""
from __future__ import annotations

from concurrent.futures import ThreadPoolExecutor
from dataclasses import replace
from contextlib import closing
import json
from pathlib import Path
import sqlite3
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
from adapter_model import (IdempotencyConflict, IngestionBoundary, PsaIncidentSnapshot,
                           TrustedPsaConnection, normalize_incident_snapshot)
from durable_sqlite_model import SQLiteInbox, numeric_revision_compare


class DurableSQLiteInboxTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.db = Path(self.tmp.name) / "inbox.sqlite3"
        self.connection = TrustedPsaConnection(
            connection_id="connectwise-demo", source_uri="urn:luminous:connectwise-demo",
            company_tenant_map={"company-42":"tenant-demo"},
            incident_id_map={("company-42","7301"):"incident-7301"})
        self.boundary = IngestionBoundary(
            connection_id=self.connection.connection_id, source_uri=self.connection.source_uri,
            company_tenant_map=self.connection.company_tenant_map,
            incident_id_map=self.connection.incident_id_map)
        self.store = SQLiteInbox(self.db,self.boundary,revision_compare=numeric_revision_compare)
        self.snapshot = PsaIncidentSnapshot(
            company_id="company-42",ticket_id="7301",revision="revision-1",status="new",
            redacted_summary="Synthetic incident; no customer data",
            occurred_at="2026-10-09T08:00:00Z",priority=2)
        self.event = self.make_event()

    def tearDown(self):
        self.tmp.cleanup()

    def make_event(self, *, revision="revision-1",status="new",summary=None,observed_at="2026-10-09T08:00:02Z"):
        snapshot = replace(self.snapshot,revision=revision,status=status,
            redacted_summary=summary if summary is not None else self.snapshot.redacted_summary)
        return normalize_incident_snapshot(snapshot,self.connection,observed_at=observed_at)

    def test_atomic_first_accept_persists_state_and_outbox(self):
        self.assertEqual(self.store.accept(self.event,now=100),"ACCEPTED")
        self.assertEqual(self.store.counts(),{"inbox_events":1,"business_effects":1,"incident_state":1,
            "outbox_events":1,"quarantined_events":0})
        self.assertEqual(self.store.get_incident("tenant-demo","incident-7301")["payload"]["status"],"new")

    def test_replay_is_duplicate_across_restarts_and_observedat_changes(self):
        self.store.accept(self.event,now=100)
        self.store = SQLiteInbox(self.db,self.boundary,revision_compare=numeric_revision_compare)
        replay = self.make_event(observed_at="2026-10-09T08:05:00Z")
        self.assertEqual(self.store.accept(replay,now=200),"DUPLICATE")
        self.assertEqual(self.store.counts()["outbox_events"],1)

    def test_reused_event_identity_with_changed_payload_conflicts(self):
        self.store.accept(self.event,now=100)
        changed = dict(self.event)
        changed["data"] = dict(self.event["data"],summary="changed")
        with self.assertRaises(IdempotencyConflict):
            self.store.accept(changed,now=101)

    def test_same_business_effect_with_alternate_delivery_id_is_duplicate(self):
        self.store.accept(self.event,now=100)
        alternate = dict(self.event,id="alternate-delivery-id")
        self.assertEqual(self.store.accept(alternate,now=101),"DUPLICATE")
        self.assertEqual(self.store.counts()["outbox_events"],1)

    def test_changed_business_effect_under_same_key_conflicts(self):
        self.store.accept(self.event,now=100)
        changed = dict(self.make_event(summary="different"),id="alternate-delivery-id")
        with self.assertRaises(IdempotencyConflict):
            self.store.accept(changed,now=101)

    def test_faults_rollback_inbox_effect_state_and_outbox(self):
        empty = {"inbox_events":0,"business_effects":0,"incident_state":0,"outbox_events":0,"quarantined_events":0}
        for point in ("after_inbox","after_state","after_outbox","before_commit"):
            with self.subTest(point=point), self.assertRaises(RuntimeError):
                self.store.accept(self.event,now=100,fail_at=point)
            self.assertEqual(self.store.counts(),empty)
        self.assertEqual(self.store.accept(self.event,now=101),"ACCEPTED")

    def test_stale_revision_does_not_roll_state_back(self):
        newer = self.make_event(revision="revision-2",status="in_progress",summary="new state")
        older = self.make_event(revision="revision-1",status="new",summary="old state")
        self.assertEqual(self.store.accept(newer,now=200),"ACCEPTED")
        self.assertEqual(self.store.accept(older,now=201),"STALE")
        state = self.store.get_incident("tenant-demo","incident-7301")
        self.assertEqual(state["revision"],"revision-2")
        self.assertEqual(state["payload"]["summary"],"new state")
        self.assertEqual(self.store.counts()["outbox_events"],1)

    def test_unknown_revision_order_is_quarantined(self):
        fail_closed = SQLiteInbox(self.db,self.boundary,revision_compare=None)
        self.assertEqual(fail_closed.accept(self.event,now=100),"ACCEPTED")
        opaque = self.make_event(revision="opaque-new-token",status="open")
        self.assertEqual(fail_closed.accept(opaque,now=101),"QUARANTINED")
        self.assertEqual(fail_closed.get_incident("tenant-demo","incident-7301")["revision"],"revision-1")
        self.assertEqual(fail_closed.counts()["outbox_events"],1)
        self.assertEqual(fail_closed.counts()["quarantined_events"],1)

    def test_cross_tenant_binding_quarantines_without_raw_payload(self):
        bad = dict(self.event,tenantid="tenant-victim")
        self.assertEqual(self.store.accept(bad,now=100),"QUARANTINED")
        self.assertEqual(self.store.counts()["business_effects"],0)
        self.assertEqual(self.store.counts()["outbox_events"],0)
        with closing(sqlite3.connect(self.db)) as db:
            columns={row[1] for row in db.execute("PRAGMA table_info(quarantined_events)")}
        self.assertEqual(columns,{"quarantine_id","event_digest","reason_code","quarantined_at"})

    def test_unmapped_resource_is_quarantined(self):
        bad = dict(self.event,subject="incident/not-mapped")
        self.assertEqual(self.store.accept(bad,now=100),"QUARANTINED")
        self.assertEqual(self.store.counts()["incident_state"],0)

    def test_parallel_duplicate_submissions_make_one_effect(self):
        def submit(_):
            store = SQLiteInbox(self.db,self.boundary,revision_compare=numeric_revision_compare)
            return store.accept(self.event,now=100)
        with ThreadPoolExecutor(max_workers=8) as pool:
            results=list(pool.map(submit,range(8)))
        self.assertEqual(results.count("ACCEPTED"),1)
        self.assertEqual(results.count("DUPLICATE"),7)
        self.assertEqual(self.store.counts()["business_effects"],1)
        self.assertEqual(self.store.counts()["outbox_events"],1)

    def test_outbox_lease_expiry_retries_at_least_once(self):
        self.store.accept(self.event,now=100)
        first=self.store.claim_outbox("worker-a",now=100,lease_seconds=10)
        self.assertEqual(first["attempts"],1)
        self.assertIsNone(self.store.claim_outbox("worker-b",now=109,lease_seconds=10))
        second=self.store.claim_outbox("worker-b",now=110,lease_seconds=10)
        self.assertEqual(second["outbox_id"],first["outbox_id"])
        self.assertEqual(second["attempts"],2)
        self.assertFalse(self.store.acknowledge_outbox(first["outbox_id"],"worker-a",now=111))
        self.assertTrue(self.store.acknowledge_outbox(second["outbox_id"],"worker-b",now=112))

    def test_outbox_preserves_order_for_each_incident(self):
        self.store.accept(self.make_event(revision="revision-1",status="new"),now=100)
        self.store.accept(self.make_event(revision="revision-2",status="open",summary="second update"),now=100)
        first=self.store.claim_outbox("worker-a",now=102,lease_seconds=20)
        self.assertEqual(json.loads(first["payload_json"])["revision"],"revision-1")
        # The second update is blocked until the first lease is acknowledged.
        self.assertIsNone(self.store.claim_outbox("worker-b",now=103,lease_seconds=20))
        self.assertTrue(self.store.acknowledge_outbox(first["outbox_id"],"worker-a",now=104))
        second=self.store.claim_outbox("worker-b",now=105,lease_seconds=20)
        self.assertEqual(json.loads(second["payload_json"])["revision"],"revision-2")

    def test_outbox_ack_requires_current_owner_and_live_lease(self):
        self.store.accept(self.event,now=100)
        claim=self.store.claim_outbox("worker",now=101,lease_seconds=5)
        self.assertFalse(self.store.acknowledge_outbox(claim["outbox_id"],"intruder",now=102))
        self.assertFalse(self.store.acknowledge_outbox(claim["outbox_id"],"worker",now=106))
        self.assertEqual(self.store.list_outbox()[0]["status"],"LEASED")


if __name__ == "__main__":
    unittest.main(verbosity=2)
