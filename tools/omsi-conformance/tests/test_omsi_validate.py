import copy
import sys
import unittest
from datetime import datetime
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
import omsi_validate as v

NOW = datetime.fromisoformat("2026-10-12T12:05:00+00:00")
def load_fixture(name): return v.load(ROOT / "fixtures" / name)
def op(): return load_fixture("operation.json")
def grant(): return load_fixture("grant.json")
def receipt(): return load_fixture("receipt.json")

class TestOMSI(unittest.TestCase):
    def test_schema_files_are_valid(self):
        for path in v.SCHEMAS.values():
            v.Draft202012Validator.check_schema(v.load(path))
    def test_operation_fixture_passes(self): v.validate_operation(op(), NOW)
    def test_argument_digest_tampering_rejected(self):
        x=op(); x["action"]["arguments"]["mode"]="switch"
        with self.assertRaisesRegex(v.ConformanceError, "digest mismatch"): v.validate_operation(x, NOW)
    def test_cross_tenant_operation_rejected(self):
        x=op(); x["target"]["tenant_id"]="tenant-b"
        with self.assertRaisesRegex(v.ConformanceError, "tenant differ"): v.validate_operation(x, NOW)
    def test_stale_precondition_rejected(self):
        x=op(); x["preconditions"]["observed_at"]="2026-10-12T11:57:00Z"
        with self.assertRaisesRegex(v.ConformanceError, "stale"): v.validate_operation(x, NOW)
    def test_expired_operation_rejected(self):
        with self.assertRaisesRegex(v.ConformanceError, "expired"): v.validate_operation(op(), datetime.fromisoformat("2026-10-12T12:16:00+00:00"))
    def test_grant_binds_exact_operation(self): v.validate_grant(op(), grant(), NOW)
    def test_grant_executor_substitution_rejected(self):
        x=grant(); x["authorized_principal"]="attacker"
        with self.assertRaisesRegex(v.ConformanceError, "authorized_principal"): v.validate_grant(op(), x, NOW)
    def test_grant_cannot_outlive_manifest(self):
        x=grant(); x["expires_at"]="2026-10-12T12:20:00Z"
        with self.assertRaisesRegex(v.ConformanceError, "exceeds manifest expiry"): v.validate_grant(op(), x, NOW)
    def test_grant_cannot_predate_manifest(self):
        x=grant(); x["issued_at"]="2026-10-12T11:59:00Z"; x["expires_at"]="2026-10-12T12:14:00Z"
        with self.assertRaisesRegex(v.ConformanceError, "cannot predate"): v.validate_grant(op(), x, NOW)
    def test_float_in_grant_extension_rejected(self):
        x=grant(); x["extensions"]={"approximation":1.5}
        with self.assertRaisesRegex(v.ConformanceError, "floating-point"): v.validate_grant(op(), x, NOW)
    def test_empty_grant_issuer_rejected(self):
        x=grant(); x["issued_by"]=""
        with self.assertRaisesRegex(v.ConformanceError, "minLength"): v.validate_grant(op(), x, NOW)
    def test_receipt_binds_exact_operation(self): v.validate_receipt(receipt(), op())
    def test_receipt_tenant_spoofing_rejected(self):
        x=receipt(); x["target"]["tenant_id"]="tenant-b"
        with self.assertRaisesRegex(v.ConformanceError, "another tenant"): v.validate_receipt(x, op())
    def test_verified_receipt_requires_post_state(self):
        x=receipt(); del x["verification"]["post_state_digest"]
        with self.assertRaisesRegex(v.ConformanceError, "post-state digest"): v.validate_receipt(x, op())
    def test_nonverified_outcome_cannot_claim_verified_result(self):
        x=receipt(); x["outcome"]="indeterminate"
        with self.assertRaisesRegex(v.ConformanceError, "non-verified outcome"): v.validate_receipt(x, op())
    def test_receipt_operation_substitution_rejected(self):
        x=receipt(); x["operation_id"]="op-other"
        with self.assertRaisesRegex(v.ConformanceError, "operation ID mismatch"): v.validate_receipt(x, op())
    def test_receipt_cannot_predate_manifest(self):
        x=receipt(); x["issued_at"]="2026-10-12T12:00:00Z"
        with self.assertRaisesRegex(v.ConformanceError, "cannot predate"): v.validate_receipt(x, op())
    def test_same_authority_domain_is_not_independent(self):
        x=receipt(); x["verification"]["verifier_authority_domain"]="control-a"
        with self.assertRaisesRegex(v.ConformanceError, "distinct declared authority domains"): v.validate_receipt(x, op())
    def test_event_sequence_fixture_passes(self): v.validate_events(load_fixture("events.json"), op())
    def test_event_sequence_cannot_predate_manifest(self):
        x=load_fixture("events.json"); x[0]["occurred_at"]="2026-10-12T12:00:00Z"
        with self.assertRaisesRegex(v.ConformanceError, "cannot predate"): v.validate_events(x, op())
    def test_event_sequence_must_bind_operation(self):
        x=load_fixture("events.json"); x[0]["operation_id"]="op-other"
        with self.assertRaisesRegex(v.ConformanceError, "operation_id does not match"): v.validate_events(x, op())
    def test_malformed_event_shape_fails_as_conformance_error(self):
        with self.assertRaisesRegex(v.ConformanceError, "schema error"): v.validate_events([{"schema":"org.omsi.operation-event/0.1"}])
    def test_invalid_transition_rejected(self):
        x=load_fixture("events.json"); x[1]["from_state"]="running"
        with self.assertRaises(v.ConformanceError): v.validate_events(x)
    def test_event_replay_rejected(self):
        x=load_fixture("events.json"); x[1]["event_id"]=x[0]["event_id"]
        with self.assertRaisesRegex(v.ConformanceError, "duplicate event id"): v.validate_events(x)
    def test_event_sequence_cannot_mix_tenants(self):
        x=load_fixture("events.json"); x[1]["tenant_id"]="tenant-b"
        with self.assertRaisesRegex(v.ConformanceError, "mixes tenant_id"): v.validate_events(x)
    def test_duplicate_json_key_rejected(self):
        with self.assertRaisesRegex(v.ConformanceError, "duplicate JSON key"): v.strict_loads('{"a":1,"a":2}')
    def test_negative_zero_rejected(self):
        with self.assertRaisesRegex(v.ConformanceError, "negative zero"): v.strict_loads('{"a":-0}')
    def test_float_in_canonical_json_rejected(self):
        with self.assertRaisesRegex(v.ConformanceError, "floating-point"): v.canonical_bytes({"n": 1.25})
    def test_digest_independent_of_key_order(self):
        domain="org.omsi.test/0.1"
        self.assertEqual(v.digest({"a":1,"b":2}, domain), v.digest({"b":2,"a":1}, domain))
    def test_digest_separates_semantic_domains(self):
        obj={"a":1}
        self.assertNotEqual(v.digest(obj, "org.omsi.action-arguments/0.1"), v.digest(obj, "org.omsi.operation-manifest/0.1"))
    def test_digest_rejects_nul_in_domain(self):
        with self.assertRaisesRegex(v.ConformanceError, "cannot contain NUL"): v.digest({"a":1}, "org.omsi.bad/0.1\\x00x")
    def test_signature_shape_is_not_signature_verification(self):
        x=grant(); x["proof"]["signature_base64"]="AA=="
        v.validate_grant(op(), x, NOW)

if __name__ == "__main__": unittest.main()
