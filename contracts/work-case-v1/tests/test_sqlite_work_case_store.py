"""SQLite persistence, restart, crash-window, and tenant-boundary tests."""
from __future__ import annotations

from concurrent.futures import ThreadPoolExecutor
from contextlib import closing
from pathlib import Path
import json
import sqlite3
import tempfile
import unittest
import sys

from jsonschema import Draft202012Validator, FormatChecker

ROOT = Path(__file__).resolve().parents[1]
OPS_ROOT = ROOT.parent / "operations-event-v1"
sys.path.insert(0, str(ROOT))
sys.path.insert(0, str(OPS_ROOT))
from work_case_model import (  # noqa: E402
    CaseState, IdempotencyConflict, InvalidTransition, MappingConflict,
    NotFound, Principal, RevisionConflict, WorkCaseStore,
)
from sqlite_work_case_store import SQLiteWorkCaseStore  # noqa: E402

T0 = "2026-10-09T08:00:00Z"
T1 = "2026-10-09T08:01:00Z"
T2 = "2026-10-09T08:02:00Z"


class SQLiteWorkCaseStoreTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.db = Path(self.tmp.name) / "workcases.sqlite3"
        self.store = SQLiteWorkCaseStore(self.db)
        self.tech = Principal("tenant-a", "tech-1", "technician")
        self.other = Principal("tenant-b", "tech-2", "technician")
        self.case = self.create_case()

    def tearDown(self):
        self.tmp.cleanup()

    def create_case(self, case_id="case-1", key="create-1", fail_at=None):
        return self.store.create_case(
            self.tech, idempotency_key=key, command_id=f"command-{key}", occurred_at=T0,
            case_id=case_id, kind="incident", title="Disk pressure", summary="Synthetic redacted summary",
            customer_id="customer-1", site_id="site-1", asset_ids=("asset-1",), priority=2, fail_at=fail_at,
        )

    def transition(self, target, *, expected=1, key="transition-1", fail_at=None, case_id="case-1"):
        return self.store.transition(
            self.tech, case_id, target_state=target, expected_revision=expected,
            reason="synthetic operator reason", idempotency_key=key,
            command_id=f"command-{key}", occurred_at=T1, fail_at=fail_at,
        )

    def test_schema_fixture_and_store_snapshot_validate(self):
        schema = json.loads((ROOT / "schema.json").read_text())
        Draft202012Validator.check_schema(schema)
        validator = Draft202012Validator(schema, format_checker=FormatChecker())
        fixture = json.loads((ROOT / "examples" / "incident-work-case.json").read_text())
        self.assertEqual(list(validator.iter_errors(fixture)), [])
        self.assertEqual(list(validator.iter_errors(self.store.get_case(self.tech, "case-1").to_dict())), [])

    def test_schema_version_is_idempotent_and_future_versions_fail_closed(self):
        with closing(sqlite3.connect(self.db)) as db:
            self.assertEqual(db.execute("PRAGMA user_version").fetchone()[0], 1)
        SQLiteWorkCaseStore(self.db)
        self.assertEqual(self.store.get_case(self.tech, "case-1"), self.case)
        future_db = Path(self.tmp.name) / "future.sqlite3"
        with closing(sqlite3.connect(future_db)) as db:
            db.execute("PRAGMA user_version = 999")
            db.commit()
        with self.assertRaises(RuntimeError):
            SQLiteWorkCaseStore(future_db)

    def test_in_memory_database_fails_closed_when_wal_mode_is_unavailable(self):
        with self.assertRaisesRegex(RuntimeError, "WAL mode is required; observed journal mode 'memory'"):
            SQLiteWorkCaseStore(":memory:")

    def test_unversioned_database_with_existing_tables_fails_closed_before_wal_mutation(self):
        legacy_db = Path(self.tmp.name) / "legacy-unversioned.sqlite3"
        with closing(sqlite3.connect(legacy_db)) as db:
            db.execute("CREATE TABLE work_cases (legacy_payload TEXT)")
            db.commit()

        with self.assertRaisesRegex(RuntimeError, "unversioned database contains pre-existing tables"):
            SQLiteWorkCaseStore(legacy_db)

        with closing(sqlite3.connect(legacy_db)) as db:
            self.assertEqual(db.execute("PRAGMA user_version").fetchone()[0], 0)
            self.assertEqual(db.execute("PRAGMA journal_mode").fetchone()[0].lower(), "delete")
            tables = {
                row[0] for row in db.execute(
                    "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'"
                )
            }
        self.assertEqual(tables, {"work_cases"})

    def test_versioned_database_with_weakened_primary_key_fails_closed(self):
        damaged_db = Path(self.tmp.name) / "damaged-primary-key.sqlite3"
        SQLiteWorkCaseStore(damaged_db)
        with closing(sqlite3.connect(damaged_db)) as db:
            db.execute("PRAGMA foreign_keys=OFF")
            db.execute("DROP TABLE work_cases")
            db.execute(
                "CREATE TABLE work_cases ("
                "tenant_id TEXT NOT NULL, case_id TEXT NOT NULL, "
                "revision INTEGER NOT NULL CHECK(revision >= 1), payload_json TEXT NOT NULL)"
            )
            db.commit()

        with self.assertRaisesRegex(RuntimeError, "work_cases primary key mismatch"):
            SQLiteWorkCaseStore(damaged_db)

        with closing(sqlite3.connect(damaged_db)) as db:
            self.assertEqual(db.execute("PRAGMA user_version").fetchone()[0], 1)
            info = db.execute("PRAGMA table_info(work_cases)").fetchall()
            self.assertEqual([row[5] for row in info], [0, 0, 0, 0])

    def test_versioned_database_missing_history_table_fails_closed_without_recreating_it(self):
        damaged_db = Path(self.tmp.name) / "damaged-versioned.sqlite3"
        SQLiteWorkCaseStore(damaged_db)
        with closing(sqlite3.connect(damaged_db)) as db:
            db.execute("DROP TABLE case_activity")
            db.commit()

        with self.assertRaisesRegex(RuntimeError, "schema is incomplete: missing table case_activity"):
            SQLiteWorkCaseStore(damaged_db)

        with closing(sqlite3.connect(damaged_db)) as db:
            self.assertEqual(db.execute("PRAGMA user_version").fetchone()[0], 1)
            self.assertIsNone(
                db.execute(
                    "SELECT 1 FROM sqlite_master WHERE type='table' AND name='case_activity'"
                ).fetchone()
            )

    def test_same_state_assignment_is_a_noop_not_a_fake_revision(self):
        counts = self.store.counts()
        history = self.store.history(self.tech, "case-1")
        unchanged = self.store.assign(
            self.tech, "case-1", assignee_id=None, expected_revision=1,
            reason="already unassigned", idempotency_key="noop-assign",
            command_id="noop-assign", occurred_at=T1,
        )
        self.assertEqual(unchanged, self.case)
        after = self.store.counts()
        self.assertEqual(after["case_activity"], counts["case_activity"])
        self.assertEqual(after["case_outbox"], counts["case_outbox"])
        self.assertEqual(self.store.history(self.tech, "case-1"), history)

    def test_rolled_back_command_can_retry_with_the_same_idempotency_key(self):
        with self.assertRaises(RuntimeError):
            self.transition("in_progress", key="retry-after-rollback", fail_at="before_commit")
        recovered = self.transition("in_progress", key="retry-after-rollback")
        self.assertEqual(recovered.revision, 2)
        self.assertEqual(self.store.counts()["case_outbox"], 2)

    def test_restart_preserves_state_history_idempotency_and_outbox(self):
        changed = self.transition("in_progress")
        history = self.store.history(self.tech, "case-1")
        counts = self.store.counts()
        self.store = SQLiteWorkCaseStore(self.db)
        self.assertEqual(self.store.get_case(self.tech, "case-1"), changed)
        self.assertEqual(self.store.history(self.tech, "case-1"), history)
        replay = self.transition("in_progress", expected=1, key="transition-1")
        self.assertEqual(replay, changed)
        self.assertEqual(self.store.counts(), counts)
        self.assertEqual(self.store.integrity_check(), ("ok",))

    def test_conflicting_idempotency_key_is_rejected_without_change(self):
        self.transition("in_progress", key="stable")
        before = self.store.counts()
        with self.assertRaises(IdempotencyConflict):
            self.store.transition(
                self.tech, "case-1", target_state="waiting", expected_revision=1,
                reason="changed semantics", idempotency_key="stable", command_id="different", occurred_at=T2,
            )
        self.assertEqual(self.store.counts(), before)
        self.assertEqual(self.store.get_case(self.tech, "case-1").revision, 2)

    def test_fault_injection_rolls_back_all_state_history_idempotency_and_outbox(self):
        for point in ("after_case", "after_history", "after_outbox", "before_commit"):
            with self.subTest(point=point):
                case_before, history_before, counts_before = (
                    self.store.get_case(self.tech, "case-1"),
                    self.store.history(self.tech, "case-1"),
                    self.store.counts(),
                )
                with self.assertRaises(RuntimeError):
                    self.transition("in_progress", key=f"fault-{point}", fail_at=point)
                self.assertEqual(self.store.get_case(self.tech, "case-1"), case_before)
                self.assertEqual(self.store.history(self.tech, "case-1"), history_before)
                self.assertEqual(self.store.counts(), counts_before)
        self.assertEqual(self.transition("in_progress", key="after-faults").revision, 2)

    def test_failed_external_mapping_write_rolls_back_case_mapping_and_outbox(self):
        before = self.store.counts()
        with self.assertRaises(RuntimeError):
            self.store.link_external_reference(
                self.tech, "case-1", provider="connectwise-psa", connection_id="cw-a",
                external_id="ticket-1", expected_revision=1, reason="fault", idempotency_key="map-fault",
                command_id="map-fault", occurred_at=T1, fail_at="after_side_effect",
            )
        self.assertEqual(self.store.counts(), before)
        with self.assertRaises(NotFound):
            self.store.find_by_external_reference(self.tech, "connectwise-psa", "cw-a", "ticket-1")
        linked = self.store.link_external_reference(
            self.tech, "case-1", provider="connectwise-psa", connection_id="cw-a",
            external_id="ticket-1", expected_revision=1, reason="map", idempotency_key="map-valid",
            command_id="map-valid", occurred_at=T1,
        )
        self.assertEqual(linked.revision, 2)
        second = self.create_case("case-2", key="create-2")
        linked_b = self.store.link_external_reference(
            self.tech, second.case_id, provider="connectwise-psa", connection_id="cw-b",
            external_id="ticket-1", expected_revision=1, reason="other instance", idempotency_key="map-b",
            command_id="map-b", occurred_at=T1,
        )
        self.assertEqual(linked_b.case_id, "case-2")
        with self.assertRaises(MappingConflict):
            self.store.link_external_reference(
                self.tech, "case-2", provider="connectwise-psa", connection_id="cw-a", external_id="ticket-1",
                expected_revision=2, reason="hijack attempt", idempotency_key="map-conflict",
                command_id="map-conflict", occurred_at=T2,
            )

    def test_evidence_is_classified_not_automatically_verified(self):
        linked = self.store.add_evidence_reference(
            self.tech, "case-1", evidence_id="receipt-1", digest_sha256="a" * 64,
            classification="internal", evidence_kind="simulation", issuer="synthetic-test", created_at=T1,
            expected_revision=1, reason="attach simulation reference", idempotency_key="evidence",
            command_id="evidence", occurred_at=T1,
        )
        self.assertEqual(linked.revision, 2)
        self.assertEqual(linked.to_dict()["evidence_refs"][0]["evidence_kind"], "simulation")
        self.assertEqual(self.store.history(self.tech, "case-1")[-1].activity_type, "case.evidence_linked")

    def test_stale_revision_changes_nothing(self):
        before = self.store.counts()
        with self.assertRaises(RevisionConflict):
            self.transition("in_progress", expected=7, key="stale")
        self.assertEqual(self.store.counts(), before)
        self.assertEqual(self.store.get_case(self.tech, "case-1").revision, 1)

    def test_cross_tenant_reads_and_writes_do_not_disclose_or_mutate(self):
        with self.assertRaises(NotFound):
            self.store.get_case(self.other, "case-1")
        with self.assertRaises(NotFound):
            self.store.transition(
                self.other, "case-1", target_state="in_progress", expected_revision=1,
                reason="forged tenant", idempotency_key="cross-tenant", command_id="cross-tenant", occurred_at=T1,
            )
        self.assertEqual(self.store.get_case(self.tech, "case-1").revision, 1)
        self.assertEqual(len(self.store.history(self.tech, "case-1")), 1)

    def test_duplicate_command_is_atomic_across_multiple_store_instances(self):
        stores = [SQLiteWorkCaseStore(self.db) for _ in range(6)]
        def submit(index):
            return stores[index % len(stores)].transition(
                self.tech, "case-1", target_state="in_progress", expected_revision=1,
                reason="one intended effect", idempotency_key="concurrent", command_id="same-command", occurred_at=T1,
            )
        with ThreadPoolExecutor(max_workers=12) as pool:
            results = list(pool.map(submit, range(24)))
        self.assertTrue(all(item == results[0] for item in results))
        self.assertEqual(self.store.get_case(self.tech, "case-1").revision, 2)
        self.assertEqual(len(self.store.history(self.tech, "case-1")), 2)
        self.assertEqual(self.store.counts()["case_outbox"], 2)

    def test_lifecycle_and_terminal_states(self):
        self.transition("in_progress", key="start")
        self.transition("resolved", expected=2, key="resolve")
        self.transition("open", expected=3, key="reopen")
        self.transition("in_progress", expected=4, key="start-again")
        self.transition("resolved", expected=5, key="resolve-again")
        closed = self.transition("closed", expected=6, key="close")
        with self.assertRaises(InvalidTransition):
            self.transition("open", expected=closed.revision, key="reopen-closed")
        self.assertEqual(self.store.get_case(self.tech, "case-1").state, CaseState.CLOSED)

    def test_outbox_is_minimized_and_does_not_include_case_summary(self):
        with closing(sqlite3.connect(self.db)) as db:
            rows = db.execute("SELECT payload_json FROM case_outbox ORDER BY outbox_seq").fetchall()
        payloads = [json.loads(row[0]) for row in rows]
        self.assertTrue(payloads)
        for payload in payloads:
            self.assertNotIn("summary", payload)
            self.assertNotIn("evidence", payload)
            self.assertNotIn("secret", payload)


if __name__ == "__main__":
    unittest.main()
