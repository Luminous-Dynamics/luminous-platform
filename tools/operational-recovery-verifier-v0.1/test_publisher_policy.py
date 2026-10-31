import copy
import json
import unittest
from pathlib import Path

from check_publisher_policy import PolicyError, check_attestation

ROOT = Path(__file__).resolve().parents[2]
BENCH = ROOT / "benchmarks" / "operational-recovery-v0.1"


def fixtures():
    statement_bytes = (BENCH / "attestation-statement.example.json").read_bytes()
    policy_bytes = (BENCH / "publisher-policy.example.json").read_bytes()
    manifest_bytes = (BENCH / "run-manifest.example.json").read_bytes()
    segment_bytes = (BENCH / "events-platform-coordinator-epoch-0.example.jsonl").read_bytes()
    return (
        json.loads(statement_bytes),
        json.loads(policy_bytes),
        manifest_bytes,
        segment_bytes,
        policy_bytes,
    )


class PublisherPolicyTests(unittest.TestCase):
    def test_exact_unsigned_statement_fixture_matches_pinned_policy_and_bytes(self):
        statement, policy, manifest, segment, policy_bytes = fixtures()
        result = check_attestation(statement, policy, manifest, segment, policy_bytes)
        self.assertEqual(result["status"], "policy_match")
        self.assertEqual(result["event_count"], 2)
        self.assertFalse(result["cryptographic_signature_verification_performed"])
        self.assertFalse(result["transparency_inclusion_verification_performed"])
        self.assertFalse(result["causal_episode_semantics_verification_performed"])

    def test_manifest_subject_byte_mismatch_rejected(self):
        statement, policy, manifest, segment, policy_bytes = fixtures()
        with self.assertRaisesRegex(PolicyError, "manifest subject digest mismatch"):
            check_attestation(statement, policy, manifest + b" ", segment, policy_bytes)

    def test_event_segment_subject_byte_mismatch_rejected(self):
        statement, policy, manifest, segment, policy_bytes = fixtures()
        with self.assertRaisesRegex(PolicyError, "event segment subject digest mismatch"):
            check_attestation(statement, policy, manifest, segment + b"\n", policy_bytes)

    def test_publisher_policy_must_match_signed_policy_digest(self):
        statement, policy, manifest, segment, policy_bytes = fixtures()
        changed = copy.deepcopy(policy)
        changed["publishers"][0]["allowed_event_types"].append("service_outcome_verified")
        changed_bytes = json.dumps(changed, indent=2).encode() + b"\n"
        with self.assertRaisesRegex(PolicyError, "publisher policy digest mismatch"):
            check_attestation(statement, changed, manifest, segment, changed_bytes)

    def test_producer_role_substitution_rejected(self):
        statement, policy, manifest, segment, policy_bytes = fixtures()
        statement["predicate"]["producer"]["source_kind"] = "participant"
        with self.assertRaisesRegex(PolicyError, "no exact identity is authorized"):
            check_attestation(statement, policy, manifest, segment, policy_bytes)

    def test_event_record_from_another_source_cannot_be_signed_as_coordinator(self):
        statement, policy, manifest, segment, policy_bytes = fixtures()
        rows = [json.loads(line) for line in segment.splitlines()]
        rows[0]["source_id"] = "itsm-ledger"
        altered = b"\n".join(json.dumps(row, separators=(",", ":")).encode() for row in rows) + b"\n"
        # The statement digest also binds the original bytes; either binding or producer check must reject it.
        with self.assertRaises(PolicyError):
            check_attestation(statement, policy, manifest, altered, policy_bytes)

    def test_wildcard_spiffe_identity_rejected(self):
        statement, policy, manifest, segment, policy_bytes = fixtures()
        changed = copy.deepcopy(policy)
        changed["publishers"][0]["identity"]["spiffe_id"] = "spiffe://luminous.example/ns/*/sa/*"
        with self.assertRaisesRegex(PolicyError, "wildcard identity"):
            check_attestation(statement, changed, manifest, segment, json.dumps(changed, indent=2).encode() + b"\n")

    def test_untrusted_oidc_issuer_rejected(self):
        statement, policy, manifest, segment, policy_bytes = fixtures()
        changed = copy.deepcopy(policy)
        changed["publishers"][1]["identity"] = {
            "profile": "sigstore_oidc",
            "oidc_issuer": "https://attacker.invalid",
            "oidc_subject": "repo:Luminous-Dynamics/luminous-platform:pull_request",
        }
        changed["trusted_oidc_issuers"] = ["https://token.actions.githubusercontent.com"]
        changed_bytes = json.dumps(changed, indent=2).encode() + b"\n"
        with self.assertRaisesRegex(PolicyError, "not in the pinned trust allowlist"):
            check_attestation(statement, changed, manifest, segment, changed_bytes)

    def test_missing_identity_pin_rejected(self):
        statement, policy, manifest, segment, policy_bytes = fixtures()
        changed = copy.deepcopy(policy)
        del changed["publishers"][0]["identity"]["spiffe_id"]
        changed_bytes = json.dumps(changed, indent=2).encode() + b"\n"
        with self.assertRaises(PolicyError):
            check_attestation(statement, changed, manifest, segment, changed_bytes)

    def test_transparency_requirement_cannot_be_disabled(self):
        statement, policy, manifest, segment, policy_bytes = fixtures()
        changed = copy.deepcopy(policy)
        changed["verification_requirements"]["require_transparency_inclusion"] = False
        changed_bytes = json.dumps(changed, indent=2).encode() + b"\n"
        with self.assertRaisesRegex(PolicyError, "verification requirements must remain fail-closed"):
            check_attestation(statement, changed, manifest, segment, changed_bytes)

    def test_revocation_unavailable_must_remain_indeterminate(self):
        statement, policy, manifest, segment, policy_bytes = fixtures()
        changed = copy.deepcopy(policy)
        changed["verification_requirements"]["revocation_unavailable_result"] = "accept"
        changed_bytes = json.dumps(changed, indent=2).encode() + b"\n"
        with self.assertRaisesRegex(PolicyError, "verification requirements must remain fail-closed"):
            check_attestation(statement, changed, manifest, segment, changed_bytes)

    def test_event_type_outside_role_policy_rejected(self):
        statement, policy, manifest, segment, policy_bytes = fixtures()
        changed = copy.deepcopy(policy)
        changed["publishers"][0]["allowed_event_types"] = ["service_outcome_verified"]
        changed_bytes = json.dumps(changed, indent=2).encode() + b"\n"
        with self.assertRaisesRegex(PolicyError, "not authorized for publisher role"):
            check_attestation(statement, changed, manifest, segment, changed_bytes)

    def test_event_digest_tamper_rejected(self):
        statement, policy, manifest, segment, policy_bytes = fixtures()
        rows = [json.loads(line) for line in segment.splitlines()]
        rows[0]["data"]["durability_attested"] = False
        altered = b"\n".join(json.dumps(row, separators=(",", ":")).encode() for row in rows) + b"\n"
        statement = copy.deepcopy(statement)
        # Rebind artifact digest to isolate the source event digest check.
        import hashlib
        statement["subject"][1]["digest"]["sha256"] = hashlib.sha256(altered).hexdigest()
        with self.assertRaisesRegex(PolicyError, "event digest mismatch"):
            check_attestation(statement, policy, manifest, altered, policy_bytes)

    def test_timezone_is_required_for_attestation_produced_at(self):
        statement, policy, manifest, segment, policy_bytes = fixtures()
        statement["predicate"]["produced_at"] = "2026-10-31T10:00:00"
        with self.assertRaisesRegex(PolicyError, "timezone required"):
            check_attestation(statement, policy, manifest, segment, policy_bytes)

    def test_event_run_binding_mismatch_rejected_even_when_segment_digest_is_rebound(self):
        statement, policy, manifest, segment, policy_bytes = fixtures()
        import hashlib
        rows = [json.loads(line) for line in segment.splitlines()]
        rows[0]["run_id"] = "different-run"
        altered = b"\n".join(json.dumps(row, separators=(",", ":")).encode() for row in rows) + b"\n"
        statement = copy.deepcopy(statement)
        statement["subject"][1]["digest"]["sha256"] = hashlib.sha256(altered).hexdigest()
        with self.assertRaisesRegex(PolicyError, "frozen run binding mismatch"):
            check_attestation(statement, policy, manifest, altered, policy_bytes)

    def test_source_sequence_gap_rejected_even_when_subject_digest_is_rebound(self):
        statement, policy, manifest, segment, policy_bytes = fixtures()
        import hashlib
        rows = [json.loads(line) for line in segment.splitlines()]
        rows[1]["source_sequence"] = 3
        altered = b"\n".join(json.dumps(row, separators=(",", ":")).encode() for row in rows) + b"\n"
        statement = copy.deepcopy(statement)
        statement["subject"][1]["digest"]["sha256"] = hashlib.sha256(altered).hexdigest()
        with self.assertRaisesRegex(PolicyError, "source epoch/sequence gap"):
            check_attestation(statement, policy, manifest, altered, policy_bytes)


if __name__ == "__main__":
    unittest.main()
