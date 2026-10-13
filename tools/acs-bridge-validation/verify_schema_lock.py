#!/usr/bin/env python3
"""Verify the locally vendored ACS schema bundle against its Git-blob lock.

This gives reproducible byte identity and offline $ref closure. It does not
authenticate the upstream publisher: the pinned ACS commit is currently
reported unsigned by GitHub, and updates require independent review.
"""
from __future__ import annotations

import hashlib
import json
import posixpath
import sys
from pathlib import Path
from typing import Any
from urllib.parse import urlsplit

from jsonschema import Draft202012Validator, SchemaError

ROOT = Path(__file__).resolve().parents[2]
LOCK_PATH = Path("docs/fixtures/acs-v0.1.0-schema-lock.json")
VENDOR_README = "vendor/agent-control-standard/README.md"
VENDOR_PREFIX = "vendor/agent-control-standard/"
SPEC_PREFIX = "specification/v0.1.0/"


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON object key: {key!r}")
        result[key] = value
    return result


def _load_json_bytes(raw: bytes, source: str) -> Any:
    try:
        text = raw.decode("utf-8", errors="strict")
        return json.loads(
            text,
            object_pairs_hook=_unique_object,
            parse_constant=lambda value: (_ for _ in ()).throw(
                ValueError(f"non-standard JSON numeric constant: {value}")
            ),
        )
    except (UnicodeError, json.JSONDecodeError, ValueError) as exc:
        raise ValueError(f"{source}: invalid strict UTF-8 JSON: {exc}") from exc


def _git_blob_sha(raw: bytes) -> str:
    header = f"blob {len(raw)}\0".encode("ascii")
    return hashlib.sha1(header + raw).hexdigest()


def _collect_refs(value: Any, source: str, refs: list[tuple[str, str]]) -> None:
    if isinstance(value, dict):
        for key, child in value.items():
            if key == "$ref" and isinstance(child, str):
                refs.append((source, child))
            _collect_refs(child, source, refs)
    elif isinstance(value, list):
        for child in value:
            _collect_refs(child, source, refs)


def verify_lock(root: Path | None = None) -> list[str]:
    """Return all reproducible bundle errors; an empty list means byte/closure checks pass."""
    root = (root or ROOT).resolve()
    errors: list[str] = []
    lock_path = root / LOCK_PATH
    try:
        lock = _load_json_bytes(lock_path.read_bytes(), str(LOCK_PATH))
    except (OSError, ValueError) as exc:
        return [str(exc)]

    if not isinstance(lock, dict):
        return ["schema lock must be a JSON object"]
    required = {
        "lock_format", "status", "upstream_repository", "upstream_release_tag",
        "upstream_release_commit", "upstream_commit_signature",
        "normative_specification_namespace", "expected_locked_file_count", "files",
    }
    missing = sorted(required - set(lock))
    if missing:
        return [f"schema lock is missing required fields: {', '.join(missing)}"]

    if lock["lock_format"] != "luminous-acs-schema-lock-v1":
        errors.append("unsupported schema lock format")
    if lock["status"] != "pinned_snapshot_not_publisher_authentication":
        errors.append("lock status must preserve the snapshot/authenticity distinction")
    if lock["upstream_repository"] != "GenAI-Security-Project/agent-control-standard":
        errors.append("unexpected ACS upstream repository")
    if lock["upstream_release_tag"] != "v0.1.1" or lock["normative_specification_namespace"] != "v0.1.0":
        errors.append("ACS release/specification namespace mismatch")
    if not isinstance(lock["upstream_release_commit"], str) or len(lock["upstream_release_commit"]) != 40:
        errors.append("upstream release commit must be a 40-character immutable commit SHA")
    signature = lock.get("upstream_commit_signature")
    if not isinstance(signature, dict) or "verified" not in signature or "reason" not in signature:
        errors.append("upstream signature-verification result must be explicit")

    files = lock.get("files")
    if not isinstance(files, list):
        return errors + ["schema lock files must be an array"]
    if lock.get("expected_locked_file_count") != len(files) or len(files) != 45:
        errors.append("expected exactly 45 locked upstream files (43 specification JSON files plus LICENSE and NOTICE)")

    source_paths: set[str] = set()
    present_source_paths: set[str] = set()
    vendor_paths: set[str] = set()
    parsed_documents: dict[str, Any] = {}
    for item in files:
        if not isinstance(item, dict):
            errors.append("each schema lock entry must be an object")
            continue
        source_path = item.get("upstream_path")
        vendor_path = item.get("vendored_path")
        expected_sha = item.get("git_blob_sha")
        expected_size = item.get("upstream_size_bytes")
        if not all(isinstance(value, str) and value for value in (source_path, vendor_path, expected_sha)):
            errors.append("schema lock entry is missing a path or blob SHA")
            continue
        if source_path in source_paths or vendor_path in vendor_paths:
            errors.append(f"duplicate schema lock path: {source_path}")
            continue
        source_paths.add(source_path)
        vendor_paths.add(vendor_path)
        if not vendor_path.startswith(VENDOR_PREFIX) or ".." in Path(vendor_path).parts:
            errors.append(f"unsafe vendor path in lock: {vendor_path}")
            continue
        candidate = root / vendor_path
        current = root
        symlink_component = False
        for component in Path(vendor_path).parts:
            current = current / component
            if current.is_symlink():
                symlink_component = True
                break
        if symlink_component:
            errors.append(f"symlink not permitted in vendored path: {vendor_path}")
            continue
        try:
            raw = candidate.read_bytes()
        except OSError:
            errors.append(f"vendored file is missing or unreadable: {vendor_path}")
            continue
        present_source_paths.add(source_path)
        if not isinstance(expected_size, int) or len(raw) != expected_size:
            errors.append(f"vendored byte size differs from upstream lock: {vendor_path}")
        actual_sha = _git_blob_sha(raw)
        if actual_sha != expected_sha:
            errors.append(f"Git blob SHA mismatch: {vendor_path}; expected {expected_sha}, got {actual_sha}")

        if source_path.startswith(SPEC_PREFIX) and source_path.endswith(".json"):
            try:
                document = _load_json_bytes(raw, vendor_path)
            except ValueError as exc:
                errors.append(str(exc))
                continue
            parsed_documents[source_path] = document
            if isinstance(document, dict) and "$schema" in document:
                if document["$schema"] != "https://json-schema.org/draft/2020-12/schema":
                    errors.append(f"unexpected JSON Schema dialect in {vendor_path}: {document['$schema']}")
                else:
                    try:
                        Draft202012Validator.check_schema(document)
                    except SchemaError as exc:
                        errors.append(f"invalid Draft 2020-12 schema {vendor_path}: {exc.message}")

    expected_count = lock.get("expected_locked_file_count")
    if len(source_paths) != expected_count:
        errors.append("schema lock path count does not match expected count")

    expected_bundle_files = vendor_paths | {VENDOR_README}
    vendor_root = root / "vendor/agent-control-standard"
    if vendor_root.exists():
        actual_bundle_files = {
            path.relative_to(root).as_posix()
            for path in vendor_root.rglob("*")
            if path.is_file() or path.is_symlink()
        }
        if actual_bundle_files != expected_bundle_files:
            extras = sorted(actual_bundle_files - expected_bundle_files)
            missing = sorted(expected_bundle_files - actual_bundle_files)
            if extras:
                errors.append("unlocked files exist in vendored ACS package: " + ", ".join(extras))
            if missing:
                errors.append("locked bundle files missing from vendored ACS package: " + ", ".join(missing))
    else:
        errors.append("vendored ACS root is missing")

    all_refs: list[tuple[str, str]] = []
    for source_path, document in parsed_documents.items():
        _collect_refs(document, source_path, all_refs)
    for source_path, ref in all_refs:
        if ref.startswith("#"):
            continue
        parts = urlsplit(ref)
        if parts.scheme or parts.netloc:
            errors.append(f"external $ref prohibited in offline schema bundle: {source_path} -> {ref}")
            continue
        ref_path = ref.split("#", 1)[0]
        if not ref_path:
            continue
        if ref_path.startswith("/") or "\\\\" in ref_path:
            errors.append(f"unsafe relative $ref: {source_path} -> {ref}")
            continue
        target = posixpath.normpath(posixpath.join(posixpath.dirname(source_path), ref_path))
        if not target.startswith(SPEC_PREFIX) or target not in present_source_paths:
            errors.append(f"unresolved/off-package $ref: {source_path} -> {ref} (resolved {target})")

    return errors


def main() -> int:
    errors = verify_lock()
    if errors:
        for error in errors:
            print(f"FAIL: {error}", file=sys.stderr)
        return 1

    lock = _load_json_bytes((ROOT / LOCK_PATH).read_bytes(), str(LOCK_PATH))
    signature = lock["upstream_commit_signature"]
    print(f"PASS: {len(lock['files'])} vendored upstream files match the locked Git blob IDs and byte sizes.")
    print(f"PASS: all vendored JSON parses strictly; local references and declared JSON Schemas validate offline.")
    if signature.get("verified") is False:
        print("WARNING: upstream commit is unsigned/unverified; byte identity does not authenticate publisher provenance.")
    print("LIMIT: this verifies the local snapshot, not ACS-Core conformance or runtime behavior.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
