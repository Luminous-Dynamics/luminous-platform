"""Adversarial tests for the native work-case contract."""
from __future__ import annotations

from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
import json
import sys
import unittest

from jsonschema import Draft202012Validator, FormatChecker

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
from work_case_model import (  # noqa: E402
    ALLOWED_TRANSITIONS, CaseState, Forbidden, IdempotencyConflict,
    InvalidTransition, MappingConflict, NotFound, Principal, RevisionConflict,
    ValidationError, WorkCaseStore,
)

T0 = "2026-10-09T08:00:00Z"
T1 = "2026-10-09T08:01:00Z"

# Independent expected-state oracle; do not derive this from the implementation.
ORACLE_TRANSITIONS = {
    CaseState.OPEN: frozenset({CaseState.IN_PROGRESS, CaseState.WAITING, CaseState.CANCELLED}),
    CaseState.IN_PROGRESS: frozenset({CaseState.WAITING, CaseState.RESOLVED, CaseState.CANCELLED}),
    CaseState.WAITING: frozenset({CaseState.IN_PROGRESS, CaseState.RESOLVED, CaseState.CANCELLED}),
    CaseState.RESOLVED: frozenset({CaseState.OPEN, CaseState.CLOSED}),
    CaseState.CLOSED: frozenset(),
    CaseState.CANCELLED: frozenset(),
}


class WorkCaseCoreTests(unittest.TestCase):
    def setUp(self):
        self.store = WorkCaseStore()
        self.tech = Principal("tenant-a", "tech-1", "technician")
        self.other = Principal("tenant-b", "tech-2", "technician")
        self.requester = Principal("tenant-a", "requester-1", "requester")
        self.case = self.create_case()

    def create_case(self, case_id="case-1", principal=None, key="create-1"):
        return self.store.create_case(
            principal or self.tech, idempotency_key=key, command_id=f"command-{key}",
            occurred_at=T0, case_id=case_id, kind="incident", title="Disk pressure",
            summary="Redacted synthetic summary", customer_id="customer-1",
            site_id="site-1", asset_ids=("asset-1",), priority=2,
        )

    def transition(self, target, *, expected=1, key=None, reason="synthetic operator reason",
                   actor=None, case_id="case-1", timestamp=T1):
        command_key = key or f"transition-{target}-{expected}"
        return self.store.transition(
            actor or self.tech, case_id, target_state=target, expected_revision=expected,
            reason=reason, idempotency_key=command_key, command_id=f"command-{command_key}",
            occurred_at=timestamp,
        )

    def test_snapshot_and_synthetic_example_match_schema(self):
        schema = json.loads((ROOT / "schema.json").read_text())
        Draft202012Validator.check_schema(schema)
        validator = Draft202012Validator(schema, format_checker=FormatChecker())
        example = json.loads((ROOT / "examples" / "incident-work-case.json").read_text())
        self.assertEqual(list(validator.iter_errors(example)), [])
        self.assertEqual(list(validator.iter_errors(self.case.to_dict())), [])

    def test_schema_rejects_unknown_fields_and_bad_digest(self):
        schema = json.loads((ROOT / "schema.json").read_text())
        validator = Draft202012Validator(schema, format_checker=FormatChecker())
        unknown = dict(self.case.to_dict(), command="execute")
        self.assertTrue(list(validator.iter_errors(unknown)))
        bad = dict(self.case.to_dict())
        bad["evidence_refs"] = [{
            "evidence_id": "e-1", "digest_sha256": "not-a-digest",
            "classification": "internal", "evidence_kind": "simulation",
            "issuer": "test", "created_at": T0,
        }]
        self.assertTrue(list(validator.iter_errors(bad)))

    def test_full_lifecycle_has_a_contiguous_append_only_timeline(self):
        assigned = self.store.assign(
            self.tech, "case-1", assignee_id="tech-3", expected_revision=1,
            reason="route to endpoint specialist", idempotency_key="assign-1",
            command_id="command-assign-1", occurred_at=T1,
        )
        self.assertEqual((assigned.revision, assigned.assignee_id), (2, "tech-3"))
        in_progress = self.transition("in_progress", expected=2, key="start")
        resolved = self.transition("resolved", expected=3, key="resolve")
        closed = self.transition("closed", expected=4, key="close")
        self.assertEqual((in_progress.revision, resolved.revision, closed.revision), (3, 4, 5))
        timeline = self.store.history(self.tech, "case-1")
        self.assertEqual([item.sequence for item in timeline], [1, 2, 3, 4, 5])
        self.assertEqual([item.new_revision for item in timeline], [1, 2, 3, 4, 5])
        self.assertEqual([item.prior_revision for item in timeline], [0, 1, 2, 3, 4])
        self.assertEqual(dict(timeline[-1].details)["to"], "closed")

    def test_published_transition_matrix_is_enforced(self):
        self.assertEqual(ALLOWED_TRANSITIONS, ORACLE_TRANSITIONS)
        paths = {
            CaseState.OPEN: [],
            CaseState.IN_PROGRESS: [CaseState.IN_PROGRESS],
            CaseState.WAITING: [CaseState.IN_PROGRESS, CaseState.WAITING],
            CaseState.RESOLVED: [CaseState.IN_PROGRESS, CaseState.RESOLVED],
            CaseState.CLOSED: [CaseState.IN_PROGRESS, CaseState.RESOLVED, CaseState.CLOSED],
            CaseState.CANCELLED: [CaseState.CANCELLED],
        }
        # The source-state setup is built only by public legal transitions.
        for source, allowed_targets in ORACLE_TRANSITIONS.items():
            for target in CaseState:
                with self.subTest(source=source.value, target=target.value):
                    store = WorkCaseStore()
                    principal = self.tech
                    case_id = "matrix-case"
                    record = store.create_case(
                        principal, idempotency_key="create", command_id="create",
                        occurred_at=T0, case_id=case_id, kind="incident", title="Matrix",
                        summary="Synthetic matrix fixture", customer_id="customer",
                    )
                    for index, step in enumerate(paths[source]):
                        record = store.transition(
                            principal, case_id, target_state=step.value,
                            expected_revision=record.revision, reason="construct test source state",
                            idempotency_key=f"path-{index}", command_id=f"path-{index}",
                            occurred_at=T0,
                        )
                    args = dict(
                        principal=principal, case_id=case_id, target_state=target.value,
                        expected_revision=record.revision, reason="matrix assertion",
                        idempotency_key="target", command_id="target", occurred_at=T1,
                    )
                    if target in allowed_targets:
                        self.assertEqual(store.transition(**args).state, target)
                    else:
                        with self.assertRaises(InvalidTransition):
                            store.transition(**args)

    def test_resolved_can_reopen_but_closed_and_cancelled_are_terminal(self):
        self.transition("in_progress")
        self.transition("resolved", expected=2, key="resolve")
        reopened = self.transition("open", expected=3, key="reopen", reason="issue recurred")
        self.assertEqual((reopened.state, reopened.revision), (CaseState.OPEN, 4))
        self.transition("in_progress", expected=4, key="start-again")
        self.transition("resolved", expected=5, key="resolve-again")
        closed = self.transition("closed", expected=6, key="close-again")
        with self.assertRaises(InvalidTransition):
            self.transition("open", expected=closed.revision, key="reopen-closed")
        cancelled = self.create_case("case-cancelled", key="create-cancelled")
        result = self.store.transition(
            self.tech, cancelled.case_id, target_state="cancelled", expected_revision=1,
            reason="request withdrawn", idempotency_key="cancel", command_id="cancel",
            occurred_at=T1,
        )
        self.assertEqual(result.state, CaseState.CANCELLED)
        with self.assertRaises(InvalidTransition):
            self.store.transition(
                self.tech, cancelled.case_id, target_state="open", expected_revision=2,
                reason="cannot reopen terminal cancellation", idempotency_key="cancel-reopen",
                command_id="cancel-reopen", occurred_at=T1,
            )

    def test_stale_revision_fails_without_state_or_history_changes(self):
        before = self.store.history(self.tech, "case-1")
        with self.assertRaises(RevisionConflict):
            self.transition("in_progress", expected=9, key="stale")
        self.assertEqual(self.store.get_case(self.tech, "case-1"), self.case)
        self.assertEqual(self.store.history(self.tech, "case-1"), before)

    def test_exact_replay_returns_original_result_without_duplicate_history(self):
        first = self.transition("in_progress", key="stable-key")
        replay = self.store.transition(
            self.tech, "case-1", target_state="in_progress", expected_revision=1,
            reason="synthetic operator reason", idempotency_key="stable-key",
            command_id="different-transport-command-id", occurred_at="2026-10-09T08:10:00Z",
        )
        self.assertEqual(replay, first)
        self.assertEqual(len(self.store.history(self.tech, "case-1")), 2)

    def test_same_idempotency_key_with_different_semantics_conflicts(self):
        self.transition("in_progress", key="reused-key")
        with self.assertRaises(IdempotencyConflict):
            self.store.transition(
                self.tech, "case-1", target_state="waiting", expected_revision=1,
                reason="different effect", idempotency_key="reused-key",
                command_id="second-command", occurred_at=T1,
            )

    def test_concurrent_duplicate_commands_collapse_within_one_process(self):
        def submit(_):
            return self.store.transition(
                self.tech, "case-1", target_state="in_progress", expected_revision=1,
                reason="one intended effect", idempotency_key="concurrent-key",
                command_id="same-command", occurred_at=T1,
            )
        with ThreadPoolExecutor(max_workers=8) as executor:
            results = list(executor.map(submit, range(24)))
        self.assertTrue(all(item == results[0] for item in results))
        self.assertEqual(self.store.get_case(self.tech, "case-1").revision, 2)
        self.assertEqual(len(self.store.history(self.tech, "case-1")), 2)

    def test_cross_tenant_reads_and_writes_are_indistinguishable_from_missing(self):
        with self.assertRaises(NotFound):
            self.store.get_case(self.other, "case-1")
        with self.assertRaises(NotFound):
            self.store.transition(
                self.other, "case-1", target_state="in_progress", expected_revision=1,
                reason="forged tenant context", idempotency_key="cross-tenant",
                command_id="forged", occurred_at=T1,
            )
        self.assertEqual(self.store.get_case(self.tech, "case-1").revision, 1)
        self.assertEqual(len(self.store.history(self.tech, "case-1")), 1)

    def test_requester_can_open_case_but_cannot_change_lifecycle(self):
        created = self.create_case("case-request", principal=self.requester, key="requester-create")
        with self.assertRaises(Forbidden):
            self.transition("in_progress", expected=1, key="requester-transition", actor=self.requester,
                            case_id=created.case_id)
        self.assertEqual(len(self.store.history(self.requester, created.case_id)), 1)

    def test_external_ids_are_explicit_unique_mappings_not_local_primary_keys(self):
        linked = self.store.link_external_reference(
            self.tech, "case-1", provider="ConnectWise-PSA", connection_id="cw-instance-a",
            external_id="ticket-7301", expected_revision=1,
            reason="import from source system", idempotency_key="map-1",
            command_id="map-command-1", occurred_at=T1,
        )
        self.assertEqual((linked.case_id, linked.revision), ("case-1", 2))
        self.assertEqual(
            self.store.find_by_external_reference(self.tech, "connectwise-psa", "cw-instance-a", "ticket-7301"),
            linked,
        )
        second = self.create_case("case-2", key="create-2")
        # The same external ID can legitimately exist in another configured instance.
        second_link = self.store.link_external_reference(
            self.tech, second.case_id, provider="connectwise-psa", connection_id="cw-instance-b",
            external_id="ticket-7301", expected_revision=1, reason="different source instance",
            idempotency_key="map-2", command_id="map-command-2", occurred_at=T1,
        )
        self.assertEqual(
            self.store.find_by_external_reference(self.tech, "connectwise-psa", "cw-instance-b", "ticket-7301"),
            second_link,
        )
        third = self.create_case("case-3", key="create-3")
        with self.assertRaises(MappingConflict):
            self.store.link_external_reference(
                self.tech, third.case_id, provider="connectwise-psa", connection_id="cw-instance-a",
                external_id="ticket-7301", expected_revision=1,
                reason="must not hijack mapping", idempotency_key="map-3",
                command_id="map-command-3", occurred_at=T1,
            )

    def test_evidence_kind_is_not_automatically_verification(self):
        linked = self.store.add_evidence_reference(
            self.tech, "case-1", evidence_id="receipt-1", digest_sha256="a" * 64,
            classification="internal", evidence_kind="simulation", issuer="synthetic-test",
            created_at=T1, expected_revision=1, reason="attach simulation reference",
            idempotency_key="evidence-1", command_id="evidence-command-1", occurred_at=T1,
        )
        item = linked.to_dict()["evidence_refs"][0]
        self.assertEqual(item["evidence_kind"], "simulation")
        self.assertNotIn("verified", item)
        self.assertEqual(linked.revision, 2)

    def test_bad_digest_priority_and_timezone_are_rejected(self):
        with self.assertRaises(ValidationError):
            self.store.add_evidence_reference(
                self.tech, "case-1", evidence_id="bad", digest_sha256="xyz",
                classification="internal", evidence_kind="observation", issuer="test",
                created_at=T1, expected_revision=1, reason="bad digest", idempotency_key="bad-evidence",
                command_id="bad-evidence", occurred_at=T1,
            )
        with self.assertRaises(ValidationError):
            self.store.create_case(
                self.tech, idempotency_key="bad-priority", command_id="bad-priority",
                occurred_at=T0, case_id="bad-priority", kind="incident", title="Bad",
                summary="Synthetic", customer_id="customer", priority=True,
            )
        with self.assertRaises(ValidationError):
            self.store.create_case(
                self.tech, idempotency_key="bad-time", command_id="bad-time",
                occurred_at="2026-10-09T08:00:00", case_id="bad-time", kind="incident",
                title="Bad", summary="Synthetic", customer_id="customer",
            )
        self.assertEqual(self.store.get_case(self.tech, "case-1").revision, 1)


if __name__ == "__main__":
    unittest.main()
