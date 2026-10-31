import copy
import json
import unittest
from pathlib import Path
from recovery_trace import (
    EVENT_SCHEMA, MANIFEST_SCHEMA, TraceError, _json_loads, digest_record, verify,
)

D = lambda char: char * 64

def manifest():
    return {
        "schema": MANIFEST_SCHEMA, "version": 1,
        "run_binding": {
            "run_id": "run-001", "episode_id": "episode-001", "plan_digest": D("1"),
            "expected_service_digest": D("2"), "source_commit_sha": "a" * 40,
            "policy_digest": D("3"), "corpus_digest": D("4"), "scenario_variant_digest": D("5"),
            "fault_matrix_digest": D("6"), "observer_manifest_digest": D("7"),
        },
        "executor": {"source_id": "platform-coordinator", "trust_domain_id": "agent-runtime"},
        "authority": {"source_id": "authority-adapter", "trust_domain_id": "authorization-service"},
        "participants": [{"source_id": "itsm-ledger", "trust_domain_id": "itsm-control-plane", "participant_kind": "mock-itsm"}],
        "observers": [{"source_id": "external-probe", "observer_id": "probe-1", "trust_domain_id": "external-observation", "scope": ["step-a", "target-1", "service"]}],
        "required_steps": [{
            "step_id": "step-a", "participant_source_id": "itsm-ledger", "target_digest": D("6"),
            "operation_id": "ticket.update", "request_digest": D("7"), "operation_binding_digest": D("f"),
            "idempotency_key_digest": D("8"), "expected_postcondition_digest": D("9"),
        }],
    }

class Builder:
    def __init__(self, m=None):
        self.m = m or manifest()
        self.events = []
        self.by_id = {}
        self.streams = {}
    def add(self, eid, typ, source="platform-coordinator", kind="coordinator", trust="agent-runtime",
            parents=(), attempt=None, step=None, data=None, epoch=0):
        key=(source,epoch)
        seq, previous, previous_id = self.streams.get(key,(0,None,None))
        seq += 1
        parent_ids=list(parents)
        if previous_id and previous_id not in parent_ids:
            parent_ids.append(previous_id)
        event={
            "schema":EVENT_SCHEMA,"event_id":eid,"run_id":"run-001","episode_id":"episode-001",
            "plan_digest":D("1"),"source_id":source,"source_kind":kind,"trust_domain_id":trust,
            "source_epoch":epoch,"source_sequence":seq,"previous_event_digest":previous,
            "parent_event_ids":parent_ids,"event_type":typ,"recorded_at":"2026-10-31T10:00:00Z",
            "attempt_id":attempt,"step_id":step,"data":data or {},
        }
        event["event_digest"]=digest_record(event)
        self.events.append(event); self.by_id[eid]=event
        self.streams[key]=(seq,event["event_digest"],eid)
        return event
    def auth(self, attempt="attempt-1", grant=D("a"), parents=()):
        return self.add("auth-"+attempt,"plan_authorized",source="authority-adapter",kind="authority_adapter",trust="authorization-service",attempt=attempt,step="step-a",parents=parents,data={
            "grant_digest":grant,"authorization_valid":True,"target_digest":D("6"),"operation_id":"ticket.update",
            "request_digest":D("7"),"operation_binding_digest":D("f"),"idempotency_key_digest":D("8"),
        })
    def dispatch(self, attempt="attempt-1", grant=D("a"), parents=(), epoch=0):
        return self.add("dispatch-"+attempt,"dispatch_intent_durable",attempt=attempt,step="step-a",parents=parents,data={
            "grant_digest":grant,"durability_attested":True,"target_digest":D("6"),"operation_id":"ticket.update",
            "request_digest":D("7"),"operation_binding_digest":D("f"),"idempotency_key_digest":D("8"),
        },epoch=epoch)
    def accepted(self, attempt="attempt-1", parents=()):
        return self.add("accepted-"+attempt,"participant_accepted",source="itsm-ledger",kind="participant",trust="itsm-control-plane",attempt=attempt,step="step-a",parents=parents,data={"request_digest":D("7"),"target_digest":D("6"),"operation_id":"ticket.update","operation_binding_digest":D("f"),"idempotency_key_digest":D("8")})
    def effect(self, attempt="attempt-1", effect_id="effect-1", parents=()):
        return self.add("effect-"+attempt,"effect_committed",source="itsm-ledger",kind="participant",trust="itsm-control-plane",attempt=attempt,step="step-a",parents=parents,data={"effect_id":effect_id,"effect_digest":D("b"),"request_digest":D("7"),"target_digest":D("6"),"operation_id":"ticket.update","operation_binding_digest":D("f"),"idempotency_key_digest":D("8")})
    def lost(self, attempt="attempt-1", parents=()):
        return self.add("lost-"+attempt,"transport_response_lost",attempt=attempt,step="step-a",parents=parents,data={})
    def status(self, attempt="attempt-1", outcome="effect_committed", consistency="strong", authoritative=True, parents=(), epoch=0):
        return self.add("status-"+attempt,"attempt_status_observed",source="itsm-ledger",kind="participant_ledger",trust="itsm-control-plane",attempt=attempt,step="step-a",parents=parents,data={
            "outcome":outcome,"consistency":consistency,"authoritative":authoritative,
            "request_digest":D("7"),"target_digest":D("6"),"operation_id":"ticket.update","operation_binding_digest":D("f"),"idempotency_key_digest":D("8"),
        },epoch=epoch)
    def reconcile(self, attempt="attempt-1", parents=()):
        return self.add("reconcile-"+attempt,"effect_reconciled",source="itsm-ledger",kind="participant_ledger",trust="itsm-control-plane",attempt=attempt,step="step-a",parents=parents,data={"effect_digest":D("b"),"effect_id":"effect-1","request_digest":D("7"),"target_digest":D("6"),"operation_id":"ticket.update","operation_binding_digest":D("f"),"idempotency_key_digest":D("8")})
    def step_verified(self, attempt="attempt-1", parents=()):
        return self.add("verify-step-"+attempt,"step_postcondition_verified",source="external-probe",kind="independent_observer",trust="external-observation",attempt=attempt,step="step-a",parents=parents,data={
            "observer_id":"probe-1","target_digest":D("6"),"expected_postcondition_digest":D("9"),
            "observed_postcondition_digest":D("9"),"captured_at":"2026-10-31T10:01:00Z",
            "valid_until":"2026-10-31T10:05:00Z","evidence_digest":D("c"),
        })
    def service_verified(self, parents=()):
        return self.add("verify-service","service_outcome_verified",source="external-probe",kind="independent_observer",trust="external-observation",parents=parents,data={
            "observer_id":"probe-1","expected_service_digest":D("2"),"observed_service_digest":D("2"),
            "captured_at":"2026-10-31T10:02:00Z","valid_until":"2026-10-31T10:05:00Z","evidence_digest":D("d"),
        })
    def complete_effect(self, attempt="attempt-1", grant=D("a")):
        auth=self.auth(attempt,grant)
        dispatch=self.dispatch(attempt,grant,parents=(auth["event_id"],))
        accepted=self.accepted(attempt,parents=(dispatch["event_id"],))
        effect=self.effect(attempt,parents=(accepted["event_id"],))
        lost=self.lost(attempt,parents=(dispatch["event_id"],))
        reconciled=self.reconcile(attempt,parents=(effect["event_id"],lost["event_id"]))
        step=self.step_verified(attempt,parents=(reconciled["event_id"],))
        service=self.service_verified(parents=(step["event_id"],))
        return service

class RecoveryTraceTests(unittest.TestCase):
    def test_complete_lost_response_after_effect_is_verified(self):
        b=Builder(); b.complete_effect()
        result=verify(b.m,b.events)
        self.assertEqual(result["status"],"valid",result["violations"])
        self.assertEqual(result["verified_required_steps"],["step-a"])

    def test_no_service_outcome_is_indeterminate_not_pass(self):
        b=Builder(); a=b.auth(); b.dispatch(parents=(a["event_id"],))
        self.assertEqual(verify(b.m,b.events)["status"],"indeterminate")

    def test_resolved_claim_without_causal_service_proof_is_false(self):
        b=Builder(); b.auth(); b.dispatch()
        b.add("agent-resolved","agent_declared_resolved",source="platform-coordinator",kind="coordinator",trust="agent-runtime",data={"claim":"resolved"})
        r=verify(b.m,b.events)
        self.assertEqual(r["status"],"invalid")
        self.assertEqual(r["false_resolution_claims"],1)

    def test_event_digest_tamper_rejected(self):
        b=Builder(); b.complete_effect(); events=copy.deepcopy(b.events); events[0]["data"]["grant_digest"]=D("f")
        with self.assertRaises(TraceError): verify(b.m,events)

    def test_missing_causal_parent_rejected(self):
        b=Builder(); b.auth(); b.dispatch()
        events=copy.deepcopy(b.events); events[1]["parent_event_ids"]=["missing"]
        events[1]["event_digest"]=digest_record(events[1])
        with self.assertRaises(TraceError): verify(b.m,events)

    def test_causal_cycle_rejected(self):
        b=Builder(); a=b.auth(); d=b.dispatch(parents=(a["event_id"],))
        a["parent_event_ids"].append(d["event_id"]); a["event_digest"]=digest_record(a)
        b.events[0]["event_digest"]=digest_record(b.events[0])
        b.events[1]["previous_event_digest"]=b.events[0]["event_digest"]
        b.events[1]["event_digest"]=digest_record(b.events[1])
        with self.assertRaises(TraceError): verify(b.m,b.events)

    def test_source_hash_chain_tamper_rejected(self):
        b=Builder(); b.auth(); b.dispatch()
        events=copy.deepcopy(b.events); events[1]["previous_event_digest"]=D("f"); events[1]["event_digest"]=digest_record(events[1])
        with self.assertRaises(TraceError): verify(b.m,events)

    def test_dispatch_without_exact_authorization_rejected(self):
        b=Builder(); b.dispatch()
        r=verify(b.m,b.events)
        self.assertEqual(r["status"],"invalid")
        self.assertTrue(any(x["code"]=="dispatch_without_authorization" for x in r["violations"]))

    def test_grant_rebinding_rejected(self):
        b=Builder(); a=b.auth(); b.dispatch(grant=D("e"),parents=(a["event_id"],))
        r=verify(b.m,b.events)
        self.assertTrue(any(x["code"]=="grant_rebinding" for x in r["violations"]))

    def test_authorization_cannot_be_asserted_by_the_agent_runtime(self):
        b=Builder()
        b.add("spoofed-auth","plan_authorized",source="platform-coordinator",kind="coordinator",trust="agent-runtime",attempt="attempt-1",step="step-a",data={"grant_digest":D("a"),"authorization_valid":True,"target_digest":D("6"),"operation_id":"ticket.update","request_digest":D("7"),"operation_binding_digest":D("f"),"idempotency_key_digest":D("8")})
        r=verify(b.m,b.events)
        self.assertTrue(any(x["code"]=="authority_source_mismatch" for x in r["violations"]))

    def test_weak_eventually_consistent_never_accepted_does_not_clear_retry(self):
        b=Builder(); a=b.auth(); d=b.dispatch(parents=(a["event_id"],)); b.lost(parents=(d["event_id"],))
        s=b.status(outcome="never_accepted",consistency="eventual",authoritative=True,parents=(d["event_id"],))
        a2=b.auth("attempt-2",D("d"),parents=(s["event_id"],)); b.dispatch("attempt-2",D("d"),parents=(a2["event_id"],))
        r=verify(b.m,b.events)
        codes={x["code"] for x in r["violations"]}
        self.assertIn("weak_no_effect_proof",codes)
        self.assertIn("retry_after_unknown",codes)

    def test_strong_authoritative_no_effect_and_fresh_grant_allows_new_attempt(self):
        b=Builder(); a=b.auth(); d=b.dispatch(parents=(a["event_id"],)); b.lost(parents=(d["event_id"],))
        s=b.status(outcome="never_accepted",consistency="strong",authoritative=True,parents=(d["event_id"],))
        a2=b.auth("attempt-2",D("d"),parents=(s["event_id"],)); d2=b.dispatch("attempt-2",D("d"),parents=(a2["event_id"],))
        accepted=b.accepted("attempt-2",parents=(d2["event_id"],)); effect=b.effect("attempt-2",parents=(accepted["event_id"],))
        rec=b.reconcile("attempt-2",parents=(effect["event_id"],)); step=b.step_verified("attempt-2",parents=(rec["event_id"],)); b.service_verified(parents=(step["event_id"],))
        result=verify(b.m,b.events)
        self.assertEqual(result["status"],"valid",result["violations"])

    def test_duplicate_business_effect_for_same_idempotency_key_rejected(self):
        b=Builder(); b.complete_effect()
        b.add("effect-duplicate-2","effect_committed",source="itsm-ledger",kind="participant",trust="itsm-control-plane",attempt="attempt-1",step="step-a",parents=("accepted-attempt-1",),data={"effect_id":"effect-duplicate","effect_digest":D("b"),"request_digest":D("7"),"target_digest":D("6"),"operation_id":"ticket.update","operation_binding_digest":D("f"),"idempotency_key_digest":D("8")})
        result=verify(b.m,b.events)
        self.assertTrue(any(x["code"]=="duplicate_business_effect" for x in result["violations"]))

    def test_step_verification_from_executor_trust_domain_rejected(self):
        b=Builder(); auth=b.auth(); dispatch=b.dispatch(parents=(auth["event_id"],)); accepted=b.accepted(parents=(dispatch["event_id"],)); effect=b.effect(parents=(accepted["event_id"],)); rec=b.reconcile(parents=(effect["event_id"],))
        b.add("bad-verify","step_postcondition_verified",source="bad-observer",kind="independent_observer",trust="agent-runtime",attempt="attempt-1",step="step-a",parents=(rec["event_id"],),data={
            "observer_id":"probe-1","target_digest":D("6"),"expected_postcondition_digest":D("9"),"observed_postcondition_digest":D("9"),
            "captured_at":"2026-10-31T10:01:00Z","valid_until":"2026-10-31T10:05:00Z","evidence_digest":D("c")})
        self.assertEqual(verify(b.m,b.events)["status"],"invalid")

    def test_service_verification_requires_every_required_step(self):
        b=Builder(); b.auth(); b.dispatch()
        b.service_verified()
        result=verify(b.m,b.events)
        self.assertTrue(any(x["code"]=="service_outcome_before_step_verification" for x in result["violations"]))

    def test_service_verification_rejects_wrong_service_digest(self):
        b=Builder(); auth=b.auth(); dispatch=b.dispatch(parents=(auth["event_id"],)); accepted=b.accepted(parents=(dispatch["event_id"],)); effect=b.effect(parents=(accepted["event_id"],)); rec=b.reconcile(parents=(effect["event_id"],)); step=b.step_verified(parents=(rec["event_id"],))
        b.add("bad-service","service_outcome_verified",source="external-probe",kind="independent_observer",trust="external-observation",parents=(step["event_id"],),data={
            "observer_id":"probe-1","expected_service_digest":D("2"),"observed_service_digest":D("e"),
            "captured_at":"2026-10-31T10:02:00Z","valid_until":"2026-10-31T10:05:00Z","evidence_digest":D("d")})
        self.assertTrue(any(x["code"]=="service_digest_mismatch" for x in verify(b.m,b.events)["violations"]))

    def test_restart_requires_scan_before_any_new_dispatch(self):
        b=Builder(); a=b.auth(); d=b.dispatch(parents=(a["event_id"],)); b.lost(parents=(d["event_id"],))
        restart=b.add("restart","coordinator_restarted",parents=(d["event_id"],))
        a2=b.auth("attempt-2",D("d"),parents=(restart["event_id"],)); b.dispatch("attempt-2",D("d"),parents=(a2["event_id"],))
        r=verify(b.m,b.events)
        self.assertTrue(any(x["code"]=="dispatch_without_recovery_scan" for x in r["violations"]))

    def test_restart_scan_cannot_omit_outstanding_attempt(self):
        b=Builder(); a=b.auth(); d=b.dispatch(parents=(a["event_id"],)); b.lost(parents=(d["event_id"],))
        restart=b.add("restart","coordinator_restarted",parents=(d["event_id"],))
        b.add("scan","recovery_scan_completed",parents=(restart["event_id"],),data={"scanned_attempt_ids":[],"unresolved_attempt_ids":[],"reconciled_attempt_ids":[]})
        r=verify(b.m,b.events)
        self.assertTrue(any(x["code"]=="recovery_scan_incomplete" for x in r["violations"]))

    def test_restart_scan_cannot_claim_unknown_attempt_reconciled(self):
        b=Builder(); a=b.auth(); d=b.dispatch(parents=(a["event_id"],)); b.lost(parents=(d["event_id"],))
        restart=b.add("restart","coordinator_restarted",parents=(d["event_id"],))
        b.add("scan","recovery_scan_completed",parents=(restart["event_id"],),data={"scanned_attempt_ids":["attempt-1"],"unresolved_attempt_ids":[],"reconciled_attempt_ids":["attempt-1"]})
        r=verify(b.m,b.events)
        self.assertTrue(any(x["code"]=="recovery_scan_claim_unproven" for x in r["violations"]))

    def test_unknown_outcome_without_lost_response_still_blocks_new_attempt(self):
        b=Builder(); a=b.auth(); d=b.dispatch(parents=(a["event_id"],))
        a2=b.auth("attempt-2",D("d"),parents=(d["event_id"],)); b.dispatch("attempt-2",D("d"),parents=(a2["event_id"],))
        r=verify(b.m,b.events)
        self.assertTrue(any(x["code"]=="retry_after_unknown" for x in r["violations"]))

    def test_retry_after_effect_reconciliation_is_rejected(self):
        b=Builder(); auth=b.auth(); dispatch=b.dispatch(parents=(auth["event_id"],)); accepted=b.accepted(parents=(dispatch["event_id"],)); effect=b.effect(parents=(accepted["event_id"],)); rec=b.reconcile(parents=(effect["event_id"],))
        a2=b.auth("attempt-2",D("d"),parents=(rec["event_id"],)); b.dispatch("attempt-2",D("d"),parents=(a2["event_id"],))
        r=verify(b.m,b.events)
        self.assertTrue(any(x["code"]=="retry_after_effect_commit" for x in r["violations"]))

    def test_valid_restart_scan_and_fresh_authorized_retry(self):
        b=Builder(); a=b.auth(); d=b.dispatch(parents=(a["event_id"],)); lost=b.lost(parents=(d["event_id"],))
        restart=b.add("restart","coordinator_restarted",parents=(lost["event_id"],),epoch=1)
        proof=b.status(outcome="never_accepted",consistency="strong",authoritative=True,parents=(restart["event_id"],))
        scan=b.add("scan","recovery_scan_completed",parents=(proof["event_id"],restart["event_id"]),data={"scanned_attempt_ids":["attempt-1"],"unresolved_attempt_ids":[],"reconciled_attempt_ids":["attempt-1"]},epoch=1)
        a2=b.auth("attempt-2",D("d"),parents=(scan["event_id"],)); d2=b.dispatch("attempt-2",D("d"),parents=(a2["event_id"],),epoch=1)
        accepted=b.accepted("attempt-2",parents=(d2["event_id"],)); effect=b.effect("attempt-2",parents=(accepted["event_id"],)); rec=b.reconcile("attempt-2",parents=(effect["event_id"],)); step=b.step_verified("attempt-2",parents=(rec["event_id"],)); b.service_verified(parents=(step["event_id"],))
        result=verify(b.m,b.events)
        self.assertEqual(result["status"],"valid",result["violations"])

    def test_recovery_scan_without_restart_is_rejected(self):
        b=Builder(); b.add("scan","recovery_scan_completed",data={"scanned_attempt_ids":[],"unresolved_attempt_ids":[],"reconciled_attempt_ids":[]})
        r=verify(b.m,b.events)
        self.assertTrue(any(x["code"]=="recovery_scan_without_restart" for x in r["violations"]))

    def test_step_observation_expired_before_service_verification_is_rejected(self):
        b=Builder(); auth=b.auth(); dispatch=b.dispatch(parents=(auth["event_id"],)); accepted=b.accepted(parents=(dispatch["event_id"],)); effect=b.effect(parents=(accepted["event_id"],)); rec=b.reconcile(parents=(effect["event_id"],))
        step=b.add("verify-step-attempt-1","step_postcondition_verified",source="external-probe",kind="independent_observer",trust="external-observation",attempt="attempt-1",step="step-a",parents=(rec["event_id"],),data={"observer_id":"probe-1","target_digest":D("6"),"expected_postcondition_digest":D("9"),"observed_postcondition_digest":D("9"),"captured_at":"2026-10-31T10:01:00Z","valid_until":"2026-10-31T10:01:30Z","evidence_digest":D("c")})
        b.add("verify-service","service_outcome_verified",source="external-probe",kind="independent_observer",trust="external-observation",parents=(step["event_id"],),data={"observer_id":"probe-1","expected_service_digest":D("2"),"observed_service_digest":D("2"),"captured_at":"2026-10-31T10:02:00Z","valid_until":"2026-10-31T10:05:00Z","evidence_digest":D("d")})
        r=verify(b.m,b.events)
        self.assertTrue(any(x["code"]=="stale_step_observation_at_service_check" for x in r["violations"]))

    def test_observer_cannot_be_declared_in_same_participant_trust_domain(self):
        m=manifest(); m["observers"][0]["trust_domain_id"]="itsm-control-plane"
        with self.assertRaises(TraceError): verify(m,[])

    def test_never_accepted_ledger_result_cannot_contradict_participant_acceptance(self):
        b=Builder(); a=b.auth(); d=b.dispatch(parents=(a["event_id"],))
        accepted=b.accepted(parents=(d["event_id"],))
        b.status(outcome="never_accepted",consistency="strong",authoritative=True,parents=(accepted["event_id"],))
        r=verify(b.m,b.events)
        self.assertTrue(any(x["code"]=="never_accepted_contradicts_acceptance" for x in r["violations"]))

    def test_coordinator_restart_epoch_must_link_exact_previous_epoch_head(self):
        b=Builder(); a=b.auth(); d=b.dispatch(parents=(a["event_id"],)); lost=b.lost(parents=(d["event_id"],))
        b.add("restart","coordinator_restarted",parents=(d["event_id"],),epoch=1)
        with self.assertRaises(TraceError): verify(b.m,b.events)

    def test_observer_ids_must_be_unique_in_manifest(self):
        m=manifest(); m["observers"].append({"source_id":"external-probe-2","observer_id":"probe-1","trust_domain_id":"other-observation","scope":["service"]})
        with self.assertRaises(TraceError): verify(m,[])

    def test_checked_in_example_trace_is_valid_and_reports_all_provenance_digests(self):
        root = Path(__file__).resolve().parents[2]
        fixture_dir = root / "benchmarks" / "operational-recovery-v0.1"
        m = json.loads((fixture_dir / "run-manifest.example.json").read_text(encoding="utf-8"))
        events = [json.loads(line) for line in (fixture_dir / "events.example.jsonl").read_text(encoding="utf-8").splitlines() if line.strip()]
        report = verify(m, events)
        self.assertEqual(report["status"], "valid")
        for key in ("manifest_digest", "event_log_digest", "participant_manifest_digest", "observer_manifest_digest", "verifier_source_sha256"):
            self.assertRegex(report[key], r"^[a-f0-9]{64}$")
        self.assertEqual(report["event_count"], 8)

    def test_duplicate_json_object_keys_are_rejected(self):
        with self.assertRaises(TraceError):
            _json_loads('{"event_id":"first","event_id":"second"}', "duplicate-key fixture")

    def test_event_count_limit_is_enforced_before_graph_processing(self):
        b=Builder()
        with self.assertRaises(TraceError):
            verify(b.m, [{}] * 4097)

    def test_parent_fanout_limit_is_enforced(self):
        b=Builder()
        e=b.add("too-many-parents","agent_declared_resolved",data={"claim":"resolved"})
        e["parent_event_ids"]=["parent-"+str(i) for i in range(65)]
        e["event_digest"]=digest_record(e)
        with self.assertRaises(TraceError):
            verify(b.m,[e])

    def test_manifest_source_revision_binding_is_required(self):
        m=manifest(); del m["run_binding"]["source_commit_sha"]
        with self.assertRaises(TraceError): verify(m,[])

if __name__ == "__main__":
    unittest.main(verbosity=2)
