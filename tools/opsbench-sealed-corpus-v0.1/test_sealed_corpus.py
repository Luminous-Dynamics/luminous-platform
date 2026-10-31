import copy
import hashlib
import json
import unittest
from pathlib import Path

from check_sealed_corpus import ContractError, check_access_audit, check_corpus_commitment, load_json

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "benchmarks" / "opsbench-sealed-corpus-v0.1"
PREREG_PATH = ROOT / "benchmarks" / "opsbench-common-core-v0.1" / "preregistration.example.json"
DOMAIN = b"luminous-opsbench-sealed-corpus-v0.1\x00"


def fixtures():
    manifest_bytes = (BASE / "manifest.example.json").read_bytes()
    registration_bytes = (BASE / "registration.example.json").read_bytes()
    salt = (BASE / "salt.example.txt").read_bytes()
    preregistration_bytes = PREREG_PATH.read_bytes()
    audit = json.loads((BASE / "access-audit.example.json").read_text())
    policy = json.loads((BASE / "access-policy.example.json").read_text())
    return json.loads(manifest_bytes), manifest_bytes, json.loads(registration_bytes), salt, preregistration_bytes, audit, policy


def rebind_commitment(manifest, manifest_bytes, registration, salt, preregistration_bytes):
    registration["manifest_sha256"] = hashlib.sha256(manifest_bytes).hexdigest()
    registration["salt_sha256"] = hashlib.sha256(salt).hexdigest()
    registration["commitment_sha256"] = hashlib.sha256(DOMAIN + salt + b"\x00" + manifest_bytes).hexdigest()
    registration["study_id"] = manifest["study_id"]
    preregistration_digest = hashlib.sha256(preregistration_bytes).hexdigest()
    manifest["preregistration_sha256"] = preregistration_digest
    registration["preregistration_sha256"] = preregistration_digest


def manifest_bytes(manifest):
    return (json.dumps(manifest, indent=2, ensure_ascii=False) + "\n").encode()


class SealedCorpusTests(unittest.TestCase):
    def test_public_example_commitment_matches_exact_manifest_and_salt(self):
        manifest, raw, registration, salt, prereg_bytes, _audit, _policy = fixtures()
        report = check_corpus_commitment(manifest, raw, registration, salt, prereg_bytes)
        self.assertEqual(report["status"], "commitment_match")
        self.assertEqual(report["item_count"], 3)
        self.assertTrue(report["example_only"])
        self.assertFalse(report["commitment_publisher_authenticity_verified"])
        self.assertFalse(report["manifest_privacy_verified"])
        self.assertEqual(report["split_scenario_counts"], {"development": 1, "screening": 1, "confirmatory": 1})

    def test_raw_manifest_byte_change_breaks_commitment(self):
        manifest, raw, registration, salt, prereg_bytes, _audit, _policy = fixtures()
        with self.assertRaisesRegex(ContractError, "raw manifest digest mismatch"):
            check_corpus_commitment(manifest, raw + b" ", registration, salt, prereg_bytes)

    def test_salt_change_breaks_commitment(self):
        manifest, raw, registration, salt, prereg_bytes, _audit, _policy = fixtures()
        with self.assertRaisesRegex(ContractError, "salt digest mismatch"):
            check_corpus_commitment(manifest, raw, registration, salt + b"x", prereg_bytes)

    def test_commitment_digest_tamper_rejected(self):
        manifest, raw, registration, salt, prereg_bytes, _audit, _policy = fixtures()
        registration["commitment_sha256"] = "0" * 64
        with self.assertRaisesRegex(ContractError, "salted corpus commitment mismatch"):
            check_corpus_commitment(manifest, raw, registration, salt, prereg_bytes)

    def test_registration_must_precede_first_trial(self):
        manifest, raw, registration, salt, prereg_bytes, _audit, _policy = fixtures()
        registration["registered_at"] = "2026-10-31T15:00:00Z"
        with self.assertRaisesRegex(ContractError, "registration must precede first scored trial"):
            check_corpus_commitment(manifest, raw, registration, salt, prereg_bytes)

    def test_salt_must_have_at_least_256_bits(self):
        manifest, raw, registration, _salt, _audit, _policy = fixtures()
        with self.assertRaisesRegex(ContractError, "minimum 32 bytes"):
            check_corpus_commitment(manifest, raw, registration, b"tiny")

    def test_duplicate_scenario_variant_rejected(self):
        manifest, _raw, registration, salt, prereg_bytes, _audit, _policy = fixtures()
        manifest["items"].append(copy.deepcopy(manifest["items"][0]))
        changed = manifest_bytes(manifest)
        rebind_commitment(manifest, changed, registration, salt, prereg_bytes)
        with self.assertRaisesRegex(ContractError, "duplicate scenario/variant key"):
            check_corpus_commitment(manifest, changed, registration, salt, prereg_bytes)

    def test_fault_family_group_overlap_across_splits_rejected(self):
        manifest, _raw, registration, salt, prereg_bytes, _audit, _policy = fixtures()
        manifest["items"][1]["fault_family_group_id"] = manifest["items"][0]["fault_family_group_id"]
        changed = manifest_bytes(manifest)
        rebind_commitment(manifest, changed, registration, salt, prereg_bytes)
        with self.assertRaisesRegex(ContractError, "fault-family group leakage"):
            check_corpus_commitment(manifest, changed, registration, salt, prereg_bytes)

    def test_topology_group_overlap_across_splits_rejected(self):
        manifest, _raw, registration, salt, prereg_bytes, _audit, _policy = fixtures()
        manifest["items"][2]["topology_group_id"] = manifest["items"][1]["topology_group_id"]
        changed = manifest_bytes(manifest)
        rebind_commitment(manifest, changed, registration, salt, prereg_bytes)
        with self.assertRaisesRegex(ContractError, "topology group leakage"):
            check_corpus_commitment(manifest, changed, registration, salt, prereg_bytes)

    def test_scenario_id_cannot_cross_splits(self):
        manifest, _raw, registration, salt, prereg_bytes, _audit, _policy = fixtures()
        manifest["items"][1]["scenario_id"] = manifest["items"][0]["scenario_id"]
        manifest["items"][1]["variant_id"] = "v2"
        changed = manifest_bytes(manifest)
        rebind_commitment(manifest, changed, registration, salt, prereg_bytes)
        with self.assertRaisesRegex(ContractError, "one scenario ID appears in multiple splits"):
            check_corpus_commitment(manifest, changed, registration, salt, prereg_bytes)

    def test_sealed_confirmatory_manifest_requires_500_scenarios(self):
        manifest, _raw, registration, salt, prereg_bytes, _audit, _policy = fixtures()
        manifest["status"] = "sealed_before_trials"
        registration["status"] = "sealed_before_trials"
        prereg = json.loads(prereg_bytes)
        prereg["status"] = "registered_before_trials"
        prereg_bytes = (json.dumps(prereg, indent=2) + "\n").encode()
        changed = manifest_bytes(manifest)
        rebind_commitment(manifest, changed, registration, salt, prereg_bytes)
        with self.assertRaisesRegex(ContractError, "confirmatory scenario count below"):
            check_corpus_commitment(manifest, changed, registration, salt, prereg_bytes)

    def test_revealed_manifest_requires_post_trial_reveal_time(self):
        manifest, _raw, registration, salt, prereg_bytes, _audit, _policy = fixtures()
        manifest["status"] = "revealed_after_study"
        registration["status"] = "revealed_after_study"
        registration["revealed_at"] = "2026-10-31T13:30:00Z"
        prereg = json.loads(prereg_bytes)
        prereg["status"] = "completed"
        prereg_bytes = (json.dumps(prereg, indent=2) + "\n").encode()
        changed = manifest_bytes(manifest)
        rebind_commitment(manifest, changed, registration, salt, prereg_bytes)
        with self.assertRaisesRegex(ContractError, "post-study reveal timestamp precedes"):
            check_corpus_commitment(manifest, changed, registration, salt, prereg_bytes)

    def test_example_access_audit_is_a_structural_pass(self):
        _manifest, _raw, _registration, _salt, _prereg_bytes, audit, policy = fixtures()
        report = check_access_audit(audit, policy)
        self.assertEqual(report["status"], "pass")
        self.assertTrue(report["coverage_complete"])
        self.assertEqual(report["blocked_forbidden_access_attempts"], 0)
        self.assertFalse(report["audit_event_provenance_verified"])
        self.assertFalse(report["iam_network_enforcement_verified"])

    def test_successful_sut_answer_key_read_fails_access_audit(self):
        _manifest, _raw, _registration, _salt, _prereg_bytes, audit, policy = fixtures()
        changed = copy.deepcopy(audit)
        changed["events"].append({
            "event_id": "evt-secret-read",
            "timestamp": "2026-10-31T14:00:05Z",
            "principal_role": "sut",
            "action": "read",
            "resource_class": "answer_key",
            "disposition": "allowed",
            "endpoint": None,
            "evidence_sha256": "5" * 64,
        })
        report = check_access_audit(changed, policy)
        self.assertEqual(report["status"], "fail")
        self.assertEqual(len(report["violations"]), 1)

    def test_denied_forbidden_read_is_recorded_not_misreported_as_leak(self):
        _manifest, _raw, _registration, _salt, _prereg_bytes, audit, policy = fixtures()
        changed = copy.deepcopy(audit)
        changed["events"].append({
            "event_id": "evt-blocked-secret-read",
            "timestamp": "2026-10-31T14:00:05Z",
            "principal_role": "sut",
            "action": "read",
            "resource_class": "answer_key",
            "disposition": "denied",
            "endpoint": None,
            "evidence_sha256": "6" * 64,
        })
        report = check_access_audit(changed, policy)
        self.assertEqual(report["status"], "pass")
        self.assertEqual(report["blocked_forbidden_access_attempts"], 1)

    def test_missing_access_channel_is_indeterminate(self):
        _manifest, _raw, _registration, _salt, _prereg_bytes, audit, policy = fixtures()
        changed = copy.deepcopy(audit)
        changed["coverage"]["network"] = False
        report = check_access_audit(changed, policy)
        self.assertEqual(report["status"], "indeterminate")
        self.assertFalse(report["coverage_complete"])

    def test_unapproved_network_egress_fails(self):
        _manifest, _raw, _registration, _salt, _prereg_bytes, audit, policy = fixtures()
        changed = copy.deepcopy(audit)
        changed["events"][2]["endpoint"] = "public-search.example"
        report = check_access_audit(changed, policy)
        self.assertEqual(report["status"], "fail")

    def test_canary_hit_fails_even_if_other_access_is_allowed(self):
        _manifest, _raw, _registration, _salt, _prereg_bytes, audit, policy = fixtures()
        changed = copy.deepcopy(audit)
        changed["canary_hits"] = 1
        report = check_access_audit(changed, policy)
        self.assertEqual(report["status"], "fail")
        self.assertIn("benchmark contamination canary observed", report["violations"][0]["reason"])

    def test_unknown_access_event_makes_audit_indeterminate(self):
        _manifest, _raw, _registration, _salt, _prereg_bytes, audit, policy = fixtures()
        changed = copy.deepcopy(audit)
        changed["events"].append({
            "event_id": "evt-unknown",
            "timestamp": "2026-10-31T14:00:06Z",
            "principal_role": "sut",
            "action": "read",
            "resource_class": "answer_key",
            "disposition": "unknown",
            "endpoint": None,
            "evidence_sha256": "7" * 64,
        })
        report = check_access_audit(changed, policy)
        self.assertEqual(report["status"], "indeterminate")
        self.assertEqual(report["unknown_access_event_count"], 1)

    def test_duplicate_json_keys_rejected(self):
        with self.assertRaisesRegex(ContractError, "duplicate JSON key"):
            load_json(b'{"a":1,"a":2}', "test")


if __name__ == "__main__":
    unittest.main()
