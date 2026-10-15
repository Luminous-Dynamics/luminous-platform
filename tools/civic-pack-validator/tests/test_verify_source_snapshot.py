"""Tests for retained snapshot verification against capture manifests."""
from __future__ import annotations

import hashlib
import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "tools" / "civic-pack-validator"))
from capture_source_snapshot import build_capture_manifest  # noqa: E402
from verify_source_snapshot import verify_source_snapshot  # noqa: E402

SCHEMA_BYTES = (ROOT / "schemas" / "source-capture-manifest-v0.1.0.schema.json").read_bytes()
BASE_TIME = "2026-10-14T18:00:00Z"


def manifest_for(snapshot: bytes, **overrides):
    args = {
        "source_id": "example.official-source",
        "canonical_url": "https://example.gov/source",
        "captured_at": BASE_TIME,
    }
    args.update(overrides)
    return build_capture_manifest(snapshot, **args)


def report_for(manifest, snapshot: bytes):
    encoded = json.dumps(
        manifest, ensure_ascii=False, sort_keys=True, separators=(",", ":")
    ).encode("utf-8")
    return verify_source_snapshot(encoded, snapshot, SCHEMA_BYTES)


class VerifySourceSnapshotTests(unittest.TestCase):
    def test_exact_retained_bytes_match_manifest(self):
        snapshot = b"exact\x00captured bytes\n"
        report = report_for(manifest_for(snapshot), snapshot)
        self.assertTrue(report["integrity_passed"], report["checks"])
        self.assertEqual(report["snapshot_sha256"], hashlib.sha256(snapshot).hexdigest())
        self.assertEqual(report["snapshot_size_bytes"], len(snapshot))
        self.assertFalse(report["network_access_performed"])
        self.assertFalse(report["provenance_authenticated"])
        self.assertFalse(report["legal_accuracy_verified"])
        self.assertTrue(report["human_review_required"])

    def test_same_length_byte_tampering_fails_digest_check(self):
        report = report_for(manifest_for(b"before"), b"bafter")
        self.assertFalse(report["integrity_passed"])
        self.assertEqual(
            next(c["status"] for c in report["checks"] if c["check_id"] == "snapshot.sha256"),
            "FAIL",
        )

    def test_size_mismatch_is_reported_independently(self):
        report = report_for(manifest_for(b"short"), b"longer")
        statuses = {c["check_id"]: c["status"] for c in report["checks"]}
        self.assertEqual(statuses["snapshot.sha256"], "FAIL")
        self.assertEqual(statuses["snapshot.size_bytes"], "FAIL")

    def test_wrong_declared_digest_fails_schema_or_digest_check(self):
        manifest = manifest_for(b"payload")
        manifest["snapshot"]["sha256"] = "0" * 64
        report = report_for(manifest, b"payload")
        self.assertFalse(report["integrity_passed"])
        self.assertEqual(
            next(c["status"] for c in report["checks"] if c["check_id"] == "snapshot.sha256"),
            "FAIL",
        )

    def test_schema_violation_fails_even_if_bytes_match(self):
        snapshot = b"payload"
        manifest = manifest_for(snapshot)
        manifest["network_access_performed"] = True
        report = report_for(manifest, snapshot)
        self.assertFalse(report["integrity_passed"])
        self.assertEqual(
            next(c["status"] for c in report["checks"] if c["check_id"] == "manifest.schema_validation"),
            "FAIL",
        )

    def test_malformed_manifest_json_fails_without_exception(self):
        report = verify_source_snapshot(b"{not json", b"payload", SCHEMA_BYTES)
        self.assertFalse(report["integrity_passed"])
        self.assertEqual(
            next(c["status"] for c in report["checks"] if c["check_id"] == "manifest.json"),
            "FAIL",
        )

    def test_non_object_manifest_fails_without_exception(self):
        report = verify_source_snapshot(b"[]", b"payload", SCHEMA_BYTES)
        self.assertFalse(report["integrity_passed"])
        self.assertEqual(
            next(c["status"] for c in report["checks"] if c["check_id"] == "manifest.root_type"),
            "FAIL",
        )

    def test_malformed_schema_fails_closed(self):
        report = verify_source_snapshot(b"{}", b"payload", b"{")
        self.assertFalse(report["integrity_passed"])
        self.assertEqual(
            next(c["status"] for c in report["checks"] if c["check_id"] == "schema.meta_schema"),
            "FAIL",
        )

    def test_report_never_contains_source_bytes(self):
        secret = b"private-source-material-do-not-emit"
        report = report_for(manifest_for(secret), secret)
        self.assertNotIn(secret.decode("ascii"), json.dumps(report))


if __name__ == "__main__":
    unittest.main()
