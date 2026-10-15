"""Tests for offline source snapshot capture manifests."""
from __future__ import annotations

import hashlib
import json
import os
import sys
import tempfile
import unittest
from pathlib import Path

from jsonschema import Draft202012Validator, FormatChecker, ValidationError

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "tools" / "civic-pack-validator"))
from capture_source_snapshot import CaptureManifestError, build_capture_manifest, main  # noqa: E402

SCHEMA = json.loads((ROOT / "schemas" / "source-capture-manifest-v0.1.0.schema.json").read_text(encoding="utf-8"))
CANONICAL = "https://example.gov/source"
BASE_TIME = "2026-10-14T18:00:00Z"


def build(data=b"captured bytes", **overrides):
    kwargs = {
        "source_id": "example.official-source",
        "canonical_url": CANONICAL,
        "captured_at": BASE_TIME,
    }
    kwargs.update(overrides)
    return build_capture_manifest(data, **kwargs)


class CaptureManifestTests(unittest.TestCase):
    def test_schema_rejects_unchanged_state_without_a_baseline(self):
        manifest = build()
        manifest["comparison"]["state"] = "unchanged"
        with self.assertRaises(ValidationError):
            Draft202012Validator(SCHEMA, format_checker=FormatChecker()).validate(manifest)

    def test_supplied_empty_final_url_is_not_silently_defaulted(self):
        with self.assertRaises(CaptureManifestError):
            build(final_url="")

    def test_malformed_previous_capture_metadata_is_rejected(self):
        first = build()
        malformed = {**first, "content_type": 17}
        with self.assertRaises(CaptureManifestError):
            build(previous_manifest=malformed, captured_at="2026-10-15T18:00:00Z")

    def test_manifest_hashes_exact_bytes_and_validates_schema(self):
        data = b"raw\x00bytes\n"
        manifest = build(data)
        self.assertEqual(manifest["snapshot"]["sha256"], hashlib.sha256(data).hexdigest())
        self.assertEqual(manifest["snapshot"]["size_bytes"], len(data))
        Draft202012Validator(SCHEMA, format_checker=FormatChecker()).validate(manifest)

    def test_capture_time_is_normalized_to_utc(self):
        manifest = build(captured_at="2026-10-14T20:00:00+02:00")
        self.assertEqual(manifest["captured_at"], "2026-10-14T18:00:00Z")

    def test_first_capture_is_a_baseline_not_a_review(self):
        manifest = build()
        self.assertEqual(manifest["comparison"]["state"], "baseline")
        self.assertTrue(manifest["human_review_required"])
        self.assertFalse(manifest["review_state_changed"])
        self.assertIsNone(manifest["comparison"]["previous_snapshot_sha256"])

    def test_identical_bytes_and_metadata_are_unchanged(self):
        first = build(b"same bytes")
        second = build(
            b"same bytes",
            captured_at="2026-10-15T18:00:00Z",
            previous_manifest=first,
        )
        self.assertEqual(second["comparison"]["state"], "unchanged")
        self.assertEqual(second["comparison"]["change_signals"], [])

    def test_changed_bytes_are_reported_without_changing_review_state(self):
        first = build(b"old")
        second = build(b"new", captured_at="2026-10-15T18:00:00Z", previous_manifest=first)
        self.assertEqual(second["comparison"]["state"], "changed")
        self.assertEqual(second["comparison"]["change_signals"], ["snapshot_bytes_changed"])
        self.assertFalse(second["review_state_changed"])

    def test_redirect_target_change_is_detected_even_with_same_bytes(self):
        first = build(b"same", final_url=CANONICAL)
        second = build(
            b"same",
            final_url="https://redirected.example.gov/source",
            captured_at="2026-10-15T18:00:00Z",
            previous_manifest=first,
        )
        self.assertEqual(second["comparison"]["state"], "changed")
        self.assertEqual(second["comparison"]["change_signals"], ["final_url_changed"])

    def test_content_type_change_is_detected(self):
        first = build(b"same", content_type="text/html")
        second = build(
            b"same",
            content_type="application/pdf",
            captured_at="2026-10-15T18:00:00Z",
            previous_manifest=first,
        )
        self.assertEqual(second["comparison"]["change_signals"], ["content_type_changed"])

    def test_non_https_canonical_url_is_rejected(self):
        with self.assertRaises(CaptureManifestError):
            build(canonical_url="http://example.gov/source")

    def test_credentialed_url_is_rejected(self):
        with self.assertRaises(CaptureManifestError):
            build(canonical_url="https://user:secret@example.gov/source")

    def test_empty_capture_is_rejected(self):
        with self.assertRaises(CaptureManifestError):
            build(b"")

    def test_timezone_naive_capture_time_is_rejected(self):
        with self.assertRaises(CaptureManifestError):
            build(captured_at="2026-10-14T18:00:00")

    def test_backdated_capture_is_rejected(self):
        first = build()
        with self.assertRaises(CaptureManifestError):
            build(b"next", captured_at="2026-10-14T17:59:59Z", previous_manifest=first)

    def test_malformed_https_port_is_rejected(self):
        with self.assertRaises(CaptureManifestError):
            build(canonical_url="https://example.gov:bad/source")

    def test_whitespace_in_host_is_rejected(self):
        with self.assertRaises(CaptureManifestError):
            build(canonical_url="https://example .gov/source")

    def test_output_cannot_overwrite_snapshot_bytes(self):
        with tempfile.TemporaryDirectory() as directory:
            snapshot = Path(directory) / "capture.bin"
            original = b"preserve captured bytes"
            snapshot.write_bytes(original)
            result = main([
                "--source-id", "example.official-source",
                "--canonical-url", CANONICAL,
                "--snapshot", str(snapshot),
                "--captured-at", BASE_TIME,
                "--output", str(snapshot),
            ])
            self.assertEqual(result, 1)
            self.assertEqual(snapshot.read_bytes(), original)

    def test_output_cannot_overwrite_snapshot_through_hard_link(self):
        with tempfile.TemporaryDirectory() as directory:
            snapshot = Path(directory) / "capture.bin"
            output_alias = Path(directory) / "output.json"
            original = b"preserve captured bytes"
            snapshot.write_bytes(original)
            os.link(snapshot, output_alias)
            result = main([
                "--source-id", "example.official-source",
                "--canonical-url", CANONICAL,
                "--snapshot", str(snapshot),
                "--captured-at", BASE_TIME,
                "--output", str(output_alias),
            ])
            self.assertEqual(result, 1)
            self.assertEqual(snapshot.read_bytes(), original)

    def test_prior_manifest_identity_mismatch_is_rejected(self):
        first = build()
        with self.assertRaises(CaptureManifestError):
            build(previous_manifest={**first, "source_id": "other.source"}, captured_at="2026-10-15T18:00:00Z")

    def test_inconsistent_previous_comparison_is_rejected(self):
        first = build(b"original")
        tampered = json.loads(json.dumps(first))
        tampered["comparison"]["state"] = "unchanged"
        tampered["comparison"]["change_signals"] = []
        with self.assertRaises(CaptureManifestError):
            build(
                b"different",
                captured_at="2026-10-15T18:00:00Z",
                previous_manifest=tampered,
            )

    def test_different_canonical_url_is_rejected(self):
        first = build()
        with self.assertRaises(CaptureManifestError):
            build(previous_manifest={**first, "canonical_url": "https://other.gov/"}, captured_at="2026-10-15T18:00:00Z")

    def test_result_is_reproducible_for_same_inputs(self):
        self.assertEqual(build(b"stable"), build(b"stable"))


if __name__ == "__main__":
    unittest.main()
