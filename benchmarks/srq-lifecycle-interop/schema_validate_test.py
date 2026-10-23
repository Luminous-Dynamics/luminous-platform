#!/usr/bin/env python3
"""Unit tests for strict JSON parsing and schema-policy helpers."""
from __future__ import annotations

import hashlib
import os
import tempfile
import unittest
from pathlib import Path

import schema_validate as validator


class StrictJsonInputTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory(prefix="srq-schema-test-")
        self.root = Path(self.temp.name)

    def tearDown(self) -> None:
        self.temp.cleanup()

    def test_duplicate_top_level_keys_rejected(self) -> None:
        path = self.root / "duplicate.json"
        path.write_text('{"tier":"S0","tier":"S4"}', encoding="utf-8")
        with self.assertRaisesRegex(validator.DuplicateJsonKey, "duplicate object key"):
            validator.load_strict_json(path, 1024)

    def test_duplicate_nested_keys_rejected(self) -> None:
        path = self.root / "nested-duplicate.json"
        path.write_text('{"subject":{"sha":"a","sha":"b"}}', encoding="utf-8")
        with self.assertRaisesRegex(validator.DuplicateJsonKey, "duplicate object key"):
            validator.load_strict_json(path, 1024)

    def test_nonstandard_nan_rejected(self) -> None:
        path = self.root / "nan.json"
        path.write_text('{"result":NaN}', encoding="utf-8")
        with self.assertRaisesRegex(validator.NonStandardJsonConstant, "non-standard JSON"):
            validator.load_strict_json(path, 1024)

    def test_input_byte_limit_is_checked_before_unbounded_read(self) -> None:
        path = self.root / "oversized.json"
        path.write_bytes(b'{"result":"' + b"x" * 64 + b'"}')
        with self.assertRaisesRegex(ValueError, "exceeds byte limit"):
            validator.load_strict_json(path, 16)

    @unittest.skipUnless(hasattr(os, "O_NOFOLLOW"), "requires O_NOFOLLOW")
    def test_symlink_input_rejected(self) -> None:
        source = self.root / "source.json"
        source.write_text('{"ok":true}', encoding="utf-8")
        link = self.root / "linked.json"
        link.symlink_to(source)
        with self.assertRaisesRegex(ValueError, "securely open input"):
            validator.load_strict_json(link, 1024)


class SchemaPolicyHelperTests(unittest.TestCase):
    def test_remote_references_are_detected_but_local_refs_allowed(self) -> None:
        schema = {
            "$defs": {"local": {"type": "string"}},
            "properties": {
                "internal": {"$ref": "#/$defs/local"},
                "external": {"$ref": "https://example.invalid/schema.json"},
            },
        }
        refs = validator.find_remote_refs(schema)
        self.assertEqual(len(refs), 1)
        self.assertIn("https://example.invalid/schema.json", refs[0])

    def test_formats_are_found_recursively(self) -> None:
        schema = {
            "properties": {
                "observed_at": {"format": "date-time"},
                "source": {"items": {"properties": {"url": {"format": "uri"}}}},
            }
        }
        self.assertEqual(validator.find_formats(schema), {"date-time", "uri"})

    def test_instance_pointer_uses_rfc6901_escaping(self) -> None:
        self.assertEqual(
            validator.pointer(["sources", 0, "path/name~revision"]),
            "/sources/0/path~1name~0revision",
        )

    def test_digest_is_sha256_hex(self) -> None:
        value = validator.digest(b"srq-test")
        self.assertEqual(len(value), 64)
        self.assertEqual(value, hashlib.sha256(b"srq-test").hexdigest())


if __name__ == "__main__":
    unittest.main(verbosity=2)
