"""Adversarial tests for detached Ed25519 approval authentication."""
from __future__ import annotations

import base64
import copy
import hashlib
import json
import tempfile
import unittest
from datetime import datetime
from pathlib import Path

from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

from tools import verify_vendor_management_endpoint_approval_signature as verifier

ROOT = Path(__file__).resolve().parents[1]
KEY_REF = "key:synthetic-approval-test"
PRINCIPAL_REF = "principal:synthetic-approver"
VALID_FROM = "2026-01-01T00:00:00Z"
VALID_UNTIL = "2027-01-01T00:00:00Z"
AS_OF = datetime.fromisoformat("2026-10-11T10:05:00+02:00")


def _canonical_store(store: dict) -> bytes:
    return json.dumps(
        store, sort_keys=True, separators=(",", ":"), ensure_ascii=False, allow_nan=False
    ).encode("utf-8")


class DetachedApprovalSignatureTests(unittest.TestCase):
    def setUp(self) -> None:
        self.private_key = Ed25519PrivateKey.generate()
        public_key_bytes = self.private_key.public_key().public_bytes(
            encoding=serialization.Encoding.Raw,
            format=serialization.PublicFormat.Raw,
        )
        profile = verifier.strict_json_loads(
            (ROOT / "tests/fixtures/vendor-management-endpoint-profile-v1.json").read_bytes()
        )
        profile["scope"]["deployment_jurisdiction"]["state"] = "not_applicable"
        profile["scope"]["deployment_jurisdiction"]["value"] = None
        profile["scope"]["deployment_jurisdiction"]["rationale"] = (
            "Synthetic test scope only; no live authorization claim."
        )
        profile["scope"]["hardware_or_commercial_region"] = {
            "state": "assessed",
            "value": "lab-za",
            "rationale": "Synthetic test scope only; no live regional qualification claim.",
            "evidence_refs": ["synthetic-evidence:region-review"],
        }
        self.profile = profile

        approval = verifier.strict_json_loads(
            (ROOT / "tests/fixtures/vendor-management-endpoint-approval-v1.json").read_bytes()
        )
        approval["state"] = "approved"
        approval["profile_digest"] = verifier.canonical_profile_digest(self.profile)
        approval["approver_ref"] = PRINCIPAL_REF
        approval["allowed_operations"] = ["inventory_read"]
        approval["signature_scheme_ref"] = verifier.SIGNATURE_SCHEME
        approval["signing_key_ref"] = KEY_REF
        approval["signed_payload_digest"] = "0" * 64
        approval["signature_evidence_ref"] = "sha256:" + "0" * 64
        self.approval = approval

        self.trust_store = {
            "schema_version": "luminous.vendor-management-approval-trust-store.v1",
            "trust_store_id": "synthetic-test-trust",
            "revision": 1,
            "purpose": "vendor_management_endpoint_approval",
            "principals": [{
                "principal_ref": PRINCIPAL_REF,
                "key_refs": [KEY_REF],
                "roles": ["endpoint_approval_signer"],
                "state": "active",
                "valid_from": VALID_FROM,
                "valid_until": VALID_UNTIL,
                "revoked_at": None,
            }],
            "keys": [{
                "key_ref": KEY_REF,
                "algorithm": "ed25519",
                "public_key_base64": base64.b64encode(public_key_bytes).decode("ascii"),
                "state": "active",
                "valid_from": VALID_FROM,
                "valid_until": VALID_UNTIL,
                "revoked_at": None,
            }],
        }
        self.registry = verifier.strict_json_loads(
            (ROOT / "security/vendor-adapter-capability-registry-v1.json").read_bytes()
        )
        self.registry_schema = verifier.strict_json_loads(
            (ROOT / "schemas/vendor-adapter-capability-registry-v1.schema.json").read_bytes()
        )
        adapter_record = next(
            item for item in self.registry["adapters"]
            if item["adapter_id"] == self.profile["adapter"]["adapter_id"]
        )
        adapter_record["maturity"] = "A1_READ_ONLY_TESTED"
        adapter_record["implementation_state"] = "LAB_TESTED"
        adapter_record["model_scope_state"] = "exact_models_tested"
        adapter_record["region_scope_state"] = "region_scoped"
        adapter_record["maturity_evidence_refs"] = ["synthetic-evidence:adapter-a1"]
        inventory = next(item for item in adapter_record["operations"] if item["operation"] == "inventory_read")
        inventory["state"] = "tested"
        inventory["evidence_refs"] = ["synthetic-evidence:inventory-read"]
        inventory["tested_revision"] = self.profile["adapter"]["adapter_revision"]
        inventory["tested_scope"] = {
            "model": self.profile["scope"]["model"],
            "product_family": self.profile["adapter"]["product_family"],
            "hardware_revision": self.profile["scope"]["hardware_revision"],
            "software_version": self.profile["scope"]["software_version"],
            "region": "lab-za",
            "test_environment": "synthetic-test-only",
        }
        self.signature = self._resign()

    def _resign(self) -> bytes:
        message = verifier.signature_message_bytes(self.approval)
        self.approval["signed_payload_digest"] = hashlib.sha256(message).hexdigest()
        signature = self.private_key.sign(message)
        self.approval["signature_evidence_ref"] = "sha256:" + hashlib.sha256(signature).hexdigest()
        return signature

    def _verify(
        self,
        approval: dict | None = None,
        signature: bytes | None = None,
        store: dict | None = None,
        expected_pin: str | None = None,
        as_of: datetime = AS_OF,
    ) -> str | None:
        actual_store = self.trust_store if store is None else store
        raw = _canonical_store(actual_store)
        pin = hashlib.sha256(raw).hexdigest() if expected_pin is None else expected_pin
        return verifier.verify_approval_signature(
            approval=copy.deepcopy(self.approval if approval is None else approval),
            signature=self.signature if signature is None else signature,
            trust_store_raw=raw,
            expected_trust_store_sha256=pin,
            expected_trust_store_revision=actual_store["revision"],
            as_of=as_of,
        )

    def test_synthetic_signed_approval_verifies(self) -> None:
        self.assertIsNone(self._verify())

    def test_synthetic_registry_scope_matches_only_exact_tested_target(self) -> None:
        approval_schema = verifier.strict_json_loads(
            (ROOT / "schemas/vendor-management-endpoint-approval-v1.schema.json").read_bytes()
        )
        profile_schema = verifier.strict_json_loads(
            (ROOT / "schemas/vendor-management-endpoint-profile-v1.schema.json").read_bytes()
        )
        self.assertEqual(verifier.registry_errors(self.registry, self.registry_schema), [])
        self.assertEqual(
            verifier.approval_errors(self.approval, approval_schema, self.profile, profile_schema),
            [],
        )
        self.assertEqual(
            verifier.capability_scope_error_codes(self.approval, self.profile, self.registry),
            [],
        )

    def test_planned_operation_is_not_authorizable(self) -> None:
        registry = copy.deepcopy(self.registry)
        adapter = next(item for item in registry["adapters"]
                       if item["adapter_id"] == self.profile["adapter"]["adapter_id"])
        next(item for item in adapter["operations"]
             if item["operation"] == "inventory_read")["state"] = "planned"
        self.assertIn(
            "CAPABILITY_OPERATION_NOT_TESTED",
            verifier.capability_scope_error_codes(self.approval, self.profile, registry),
        )

    def test_tested_revision_must_match_profile_adapter_revision(self) -> None:
        registry = copy.deepcopy(self.registry)
        adapter = next(item for item in registry["adapters"]
                       if item["adapter_id"] == self.profile["adapter"]["adapter_id"])
        next(item for item in adapter["operations"]
             if item["operation"] == "inventory_read")["tested_revision"] = "b" * 40
        self.assertIn(
            "CAPABILITY_TESTED_REVISION_MISMATCH",
            verifier.capability_scope_error_codes(self.approval, self.profile, registry),
        )

    def test_tested_model_drift_blocks_operation(self) -> None:
        registry = copy.deepcopy(self.registry)
        adapter = next(item for item in registry["adapters"]
                       if item["adapter_id"] == self.profile["adapter"]["adapter_id"])
        next(item for item in adapter["operations"]
             if item["operation"] == "inventory_read")["tested_scope"]["model"] = "different-model"
        self.assertIn(
            "CAPABILITY_TESTED_SCOPE_MISMATCH",
            verifier.capability_scope_error_codes(self.approval, self.profile, registry),
        )

    def test_hardware_commercial_region_must_be_assessed(self) -> None:
        profile = copy.deepcopy(self.profile)
        profile["scope"]["hardware_or_commercial_region"]["state"] = "unresolved"
        profile["scope"]["hardware_or_commercial_region"]["value"] = None
        self.assertIn(
            "CAPABILITY_HARDWARE_REGION_ASSESSMENT_REQUIRED",
            verifier.capability_scope_error_codes(self.approval, profile, self.registry),
        )

    def test_tested_region_must_match_assessed_hardware_commercial_region(self) -> None:
        registry = copy.deepcopy(self.registry)
        adapter = next(item for item in registry["adapters"]
                       if item["adapter_id"] == self.profile["adapter"]["adapter_id"])
        next(item for item in adapter["operations"]
             if item["operation"] == "inventory_read")["tested_scope"]["region"] = "lab-eu"
        self.assertIn(
            "CAPABILITY_TESTED_SCOPE_MISMATCH",
            verifier.capability_scope_error_codes(self.approval, self.profile, registry),
        )

    def test_capability_implementation_must_be_lab_tested_or_qualified(self) -> None:
        registry = copy.deepcopy(self.registry)
        adapter = next(item for item in registry["adapters"]
                       if item["adapter_id"] == self.profile["adapter"]["adapter_id"])
        adapter["implementation_state"] = "NOT_IMPLEMENTED"
        self.assertIn(
            "CAPABILITY_IMPLEMENTATION_NOT_TESTED",
            verifier.capability_scope_error_codes(self.approval, self.profile, registry),
        )

    def test_approval_profile_contract_binds_the_signed_profile(self) -> None:
        approval_schema = verifier.strict_json_loads(
            (ROOT / "schemas/vendor-management-endpoint-approval-v1.schema.json").read_bytes()
        )
        profile_schema = verifier.strict_json_loads(
            (ROOT / "schemas/vendor-management-endpoint-profile-v1.schema.json").read_bytes()
        )
        self.assertEqual(
            verifier.approval_errors(
                self.approval, approval_schema, self.profile, profile_schema
            ),
            [],
        )

    def test_payload_edit_breaks_the_signed_payload_digest(self) -> None:
        changed = copy.deepcopy(self.approval)
        changed["allowed_operations"].append("health_read")
        self.assertEqual(self._verify(approval=changed), "SIGNED_PAYLOAD_DIGEST_MISMATCH")

    def test_tampered_signature_is_rejected_even_when_sidecar_digest_is_updated(self) -> None:
        changed_signature = bytearray(self.signature)
        changed_signature[0] ^= 1
        changed_signature = bytes(changed_signature)
        changed_approval = copy.deepcopy(self.approval)
        changed_approval["signature_evidence_ref"] = (
            "sha256:" + hashlib.sha256(changed_signature).hexdigest()
        )
        self.assertEqual(
            self._verify(approval=changed_approval, signature=changed_signature),
            "SIGNATURE_INVALID",
        )

    def test_trust_store_pin_mismatch_fails_closed(self) -> None:
        self.assertEqual(self._verify(expected_pin="0" * 64), "TRUST_STORE_DIGEST_MISMATCH")

    def test_trust_store_revision_rollback_fails_closed_even_with_matching_old_digest(self) -> None:
        older = copy.deepcopy(self.trust_store)
        older["revision"] = 1
        raw = _canonical_store(older)
        pin = hashlib.sha256(raw).hexdigest()
        self.assertEqual(
            verifier.verify_approval_signature(
                approval=copy.deepcopy(self.approval),
                signature=self.signature,
                trust_store_raw=raw,
                expected_trust_store_sha256=pin,
                expected_trust_store_revision=2,
                as_of=AS_OF,
            ),
            "TRUST_STORE_REVISION_PIN_MISMATCH",
        )

    def test_trust_store_revision_pin_rejects_boolean(self) -> None:
        raw = _canonical_store(self.trust_store)
        self.assertEqual(
            verifier.verify_approval_signature(
                approval=copy.deepcopy(self.approval),
                signature=self.signature,
                trust_store_raw=raw,
                expected_trust_store_sha256=hashlib.sha256(raw).hexdigest(),
                expected_trust_store_revision=True,
                as_of=AS_OF,
            ),
            "TRUST_STORE_REVISION_PIN_INVALID",
        )

    def test_signature_sidecar_digest_must_match(self) -> None:
        changed = copy.deepcopy(self.approval)
        changed["signature_evidence_ref"] = "sha256:" + "1" * 64
        self.assertEqual(self._verify(approval=changed), "SIGNATURE_EVIDENCE_DIGEST_MISMATCH")

    def test_signing_key_must_be_delegated_to_the_approver(self) -> None:
        changed = copy.deepcopy(self.approval)
        changed["signing_key_ref"] = "key:unknown-signing-key"
        changed["signed_payload_digest"] = hashlib.sha256(
            verifier.signature_message_bytes(changed)
        ).hexdigest()
        self.assertEqual(self._verify(approval=changed), "KEY_NOT_DELEGATED_TO_APPROVER")

    def test_trust_store_schema_rejects_empty_key_delegation_list(self) -> None:
        store = copy.deepcopy(self.trust_store)
        store["principals"][0]["key_refs"] = []
        self.assertEqual(self._verify(store=store), "TRUST_STORE_SCHEMA_INVALID")

    def test_key_revoked_at_use_is_rejected(self) -> None:
        store = copy.deepcopy(self.trust_store)
        store["keys"][0]["state"] = "revoked"
        store["keys"][0]["revoked_at"] = "2026-10-11T08:00:00Z"
        self.assertEqual(self._verify(store=store), "SIGNING_KEY_REVOKED_AT_AS_OF")

    def test_future_key_revocation_does_not_rewrite_past_time_assessment(self) -> None:
        store = copy.deepcopy(self.trust_store)
        store["keys"][0]["state"] = "revoked"
        store["keys"][0]["revoked_at"] = "2026-10-11T08:06:00Z"
        self.assertIsNone(self._verify(store=store))

    def test_approver_revoked_at_use_is_rejected(self) -> None:
        store = copy.deepcopy(self.trust_store)
        store["principals"][0]["state"] = "revoked"
        store["principals"][0]["revoked_at"] = "2026-10-11T08:00:00Z"
        self.assertEqual(self._verify(store=store), "APPROVER_REVOKED_AT_AS_OF")

    def test_approval_outside_its_time_window_is_not_current(self) -> None:
        later = datetime.fromisoformat("2026-10-11T10:11:00+02:00")
        self.assertEqual(self._verify(as_of=later), "APPROVAL_NOT_CURRENT")

    def test_pending_record_cannot_be_used_as_an_approval(self) -> None:
        changed = copy.deepcopy(self.approval)
        changed["state"] = "pending"
        self.assertEqual(self._verify(approval=changed), "DECISION_NOT_APPROVED")

    def test_naive_as_of_time_is_rejected(self) -> None:
        self.assertEqual(
            self._verify(as_of=datetime(2026, 10, 11, 10, 5)),
            "AS_OF_TIME_INVALID",
        )

    def test_duplicate_json_members_are_rejected(self) -> None:
        with self.assertRaises(ValueError):
            verifier.strict_json_loads(b'{"state":"pending","state":"approved"}')

    def test_excessive_json_nesting_is_rejected_before_standard_parser_recursion(self) -> None:
        depth = verifier.MAX_JSON_NESTING + 1
        raw = ("[" * depth + "0" + "]" * depth).encode("utf-8")
        with self.assertRaisesRegex(ValueError, "nesting"):
            verifier.strict_json_loads(raw)

    def test_signature_canonicalizer_rejects_excessive_programmatic_nesting(self) -> None:
        value: object = 0
        for _ in range(verifier.MAX_JSON_NESTING + 1):
            value = [value]
        with self.assertRaisesRegex(ValueError, "nesting"):
            verifier.canonical_json_bytes(value)

    def test_bounded_reader_accepts_exact_limit_and_rejects_oversize(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "input.json"
            path.write_bytes(b"1234")
            self.assertEqual(verifier.read_bounded_file(path, 4), b"1234")
            path.write_bytes(b"12345")
            with self.assertRaisesRegex(ValueError, "byte limit"):
                verifier.read_bounded_file(path, 4)

    def test_bounded_reader_rejects_symlinks_and_non_regular_files(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            target = root / "target.json"
            target.write_bytes(b"{}")
            link = root / "link.json"
            link.symlink_to(target)
            with self.assertRaises(OSError):
                verifier.read_bounded_file(link)

            with self.assertRaises(ValueError):
                verifier.read_bounded_file(root)

    def test_negative_zero_json_integer_is_rejected_before_canonicalization(self) -> None:
        with self.assertRaisesRegex(ValueError, "negative zero"):
            verifier.strict_json_loads(b'{"profile_revision":-0}')

    def test_floating_point_json_numbers_are_rejected_before_canonicalization(self) -> None:
        for raw in (b'{"value":1.0}', b'{"value":1e-7}', b'{"value":-0.0}'):
            with self.subTest(raw=raw):
                with self.assertRaisesRegex(ValueError, "floating-point"):
                    verifier.strict_json_loads(raw)

    def test_non_finite_json_numbers_are_rejected_before_canonicalization(self) -> None:
        for raw in (b'{"value":NaN}', b'{"value":Infinity}', b'{"value":-Infinity}'):
            with self.subTest(raw=raw):
                with self.assertRaisesRegex(ValueError, "non-finite"):
                    verifier.strict_json_loads(raw)

    def test_safe_json_integer_boundaries_are_accepted(self) -> None:
        for raw, expected in (
            (b'{"value":9007199254740991}', 9007199254740991),
            (b'{"value":-9007199254740991}', -9007199254740991),
        ):
            with self.subTest(raw=raw):
                self.assertEqual(verifier.strict_json_loads(raw), {"value": expected})

    def test_integers_outside_safe_interoperable_range_are_rejected(self) -> None:
        for raw in (
            b'{"value":9007199254740992}',
            b'{"value":-9007199254740992}',
            b'{"value":999999999999999999999999999999999}',
        ):
            with self.subTest(raw=raw):
                with self.assertRaisesRegex(ValueError, "safe interoperable JSON integer range"):
                    verifier.strict_json_loads(raw)

    def test_signature_canonicalizer_rejects_programmatic_unsafe_integer(self) -> None:
        with self.assertRaisesRegex(ValueError, "safe interoperable JSON integer range"):
            verifier.canonical_json_bytes({"profile_revision": 9007199254740992})

    def test_signature_payload_excludes_only_circular_fields(self) -> None:
        original = verifier.signature_message_bytes(self.approval)
        changed_references = copy.deepcopy(self.approval)
        changed_references["signed_payload_digest"] = "f" * 64
        changed_references["signature_evidence_ref"] = "sha256:" + "e" * 64
        self.assertEqual(original, verifier.signature_message_bytes(changed_references))
        changed_payload = copy.deepcopy(self.approval)
        changed_payload["policy_revision"] = "c" * 40
        self.assertNotEqual(original, verifier.signature_message_bytes(changed_payload))

    def test_trust_store_schema_rejects_malformed_key_encoding(self) -> None:
        store = copy.deepcopy(self.trust_store)
        store["keys"][0]["public_key_base64"] = "not-base64"
        self.assertEqual(self._verify(store=store), "TRUST_STORE_SCHEMA_INVALID")


    def test_key_revocation_at_exact_use_time_fails_closed(self) -> None:
        store = copy.deepcopy(self.trust_store)
        store["keys"][0]["state"] = "revoked"
        store["keys"][0]["revoked_at"] = "2026-10-11T08:05:00Z"
        self.assertEqual(self._verify(store=store), "SIGNING_KEY_REVOKED_AT_AS_OF")

    def test_key_validity_end_is_exclusive(self) -> None:
        store = copy.deepcopy(self.trust_store)
        store["keys"][0]["valid_until"] = AS_OF.astimezone().isoformat().replace("+02:00", "Z")
        self.assertEqual(self._verify(store=store), "SIGNING_KEY_NOT_CURRENT")

    def test_principal_validity_start_is_inclusive(self) -> None:
        store = copy.deepcopy(self.trust_store)
        store["principals"][0]["valid_from"] = "2026-10-11T08:05:00Z"
        self.assertIsNone(self._verify(store=store))


if __name__ == "__main__":
    unittest.main()
