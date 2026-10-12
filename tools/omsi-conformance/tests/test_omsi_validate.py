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
    def test_receipt_binds_exact_operation(self): v.validate_receipt(receipt(), op())
    def test_receipt_tenant_spoofing_rejected(self):
        x=receipt(); x["target"]["tenant_id"]="tenant-b"
        with self.assertRaisesRegex(v.ConformanceError, "another tenant"): v.validate_receipt(x, op())
    def test_verified_receipt_requires_post_state(self):
        x=receipt(); del x["verification"]["post_state_digest"]
        with self.assertRaisesRegex(v.ConformanceError, "post-state digest"): v.validate_receipt(x, op())
    def test_same_authority_domain_is_not_independent(self):
        x=receipt(); x["verification"]["verifier_authority_domain"]="control-a"
        with self.assertRaisesRegex(v.ConformanceError, "distinct declared authority domains"): v.validate_receipt(x, op())
    def test_event_sequence_fixture_passes(self): v.validate_events(load_fixture("events.json"))
    def test_invalid_transition_rejected(self):
        x=load_fixture("events.json"); x[1]["from_state"]="running"
        with self.assertRaises(v.ConformanceError): v.validate_events(x)
    def test_event_replay_rejected(self):
        x=load_fixture("events.json"); x[1]["event_id"]=x[0]["event_id"]
        with self.assertRaisesRegex(v.ConformanceError, "duplicate event id"): v.validate_events(x)
    def test_duplicate_json_key_rejected(self):
        with self.assertRaisesRegex(v.ConformanceError, "duplicate JSON key"): v.strict_loads('{"a":1,"a":2}')
    def test_negative_zero_rejected(self):
        with self.assertRaisesRegex(v.ConformanceError, "negative zero"): v.strict_loads('{"a":-0}')
    def test_float_in_canonical_json_rejected(self):
        with self.assertRaisesRegex(v.ConformanceError, "floating-point"): v.canonical_bytes({"n": 1.25})
    def test_digest_independent_of_key_order(self):
        self.assertEqual(v.digest({"a":1,"b":2}), v.digest({"b":2,"a":1}))
    def test_signature_shape_is_not_signature_verification(self):
        x=grant(); x["proof"]["signature_base64"]="AA=="
        v.validate_grant(op(), x, NOW)

if __name__ == "__main__": unittest.main()
