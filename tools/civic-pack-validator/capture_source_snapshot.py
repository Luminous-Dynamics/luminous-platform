#!/usr/bin/env python3
"""Create a deterministic manifest for an already-captured source snapshot.

This utility performs no network requests and never marks a source as reviewed.
It hashes the exact bytes supplied by the operator and compares them with an
optional previous manifest to flag byte, redirect-target, or content-type changes.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any
from urllib.parse import urlsplit

MANIFEST_VERSION = "0.1.0"
SOURCE_ID_PATTERN = re.compile(r"^[a-z0-9][a-z0-9._-]{2,127}$")
SHA256_PATTERN = re.compile(r"^[a-f0-9]{64}$")


class CaptureManifestError(ValueError):
    """Raised when capture inputs are inconsistent or unsafe."""


def parse_instant(value: str) -> datetime:
    if not isinstance(value, str):
        raise CaptureManifestError("captured_at must be a timezone-aware ISO 8601 string")
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError as exc:
        raise CaptureManifestError("captured_at must be a timezone-aware ISO 8601 string") from exc
    if parsed.tzinfo is None or parsed.utcoffset() is None:
        raise CaptureManifestError("captured_at must include a timezone")
    return parsed.astimezone(timezone.utc)


def format_instant(value: datetime) -> str:
    return value.astimezone(timezone.utc).isoformat().replace("+00:00", "Z")


def validate_https_url(value: str, label: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise CaptureManifestError(f"{label} must be an absolute HTTPS URL")
    try:
        parsed = urlsplit(value)
        hostname = parsed.hostname
        # Accessing .port forces urllib to reject non-numeric/out-of-range ports.
        _port = parsed.port
    except ValueError as exc:
        raise CaptureManifestError(f"{label} is not a valid URL") from exc
    if (
        parsed.scheme.lower() != "https"
        or not hostname
        or parsed.username is not None
        or parsed.password is not None
        or any(char.isspace() or ord(char) < 33 for char in (hostname or ""))
    ):
        raise CaptureManifestError(
            f"{label} must be an absolute HTTPS URL without embedded credentials"
        )
    return value


def load_previous_manifest(value: dict[str, Any], *, source_id: str, canonical_url: str) -> dict[str, Any]:
    if not isinstance(value, dict) or value.get("manifest_version") != MANIFEST_VERSION:
        raise CaptureManifestError("previous manifest has an unsupported or missing manifest_version")
    if value.get("source_id") != source_id:
        raise CaptureManifestError("previous manifest source_id does not match")
    if value.get("canonical_url") != canonical_url:
        raise CaptureManifestError("previous manifest canonical_url does not match")
    snapshot = value.get("snapshot")
    if value.get("capture_method") != "offline_sha256_of_provided_bytes":
        raise CaptureManifestError("previous manifest has an unsupported capture_method")
    if value.get("network_access_performed") is not False:
        raise CaptureManifestError("previous manifest must record network_access_performed=false")
    if value.get("human_review_required") is not True or value.get("review_state_changed") is not False:
        raise CaptureManifestError("previous manifest must not imply review or alter review state")
    if not isinstance(snapshot, dict) or not isinstance(snapshot.get("sha256"), str):
        raise CaptureManifestError("previous manifest has no snapshot SHA-256")
    if not SHA256_PATTERN.fullmatch(snapshot["sha256"]):
        raise CaptureManifestError("previous manifest snapshot SHA-256 is malformed")
    if type(snapshot.get("size_bytes")) is not int or snapshot["size_bytes"] < 1:
        raise CaptureManifestError("previous manifest snapshot size is missing or invalid")
    previous_content_type = value.get("content_type")
    if previous_content_type is not None and not isinstance(previous_content_type, str):
        raise CaptureManifestError("previous manifest content_type must be a string or null")
    previous_captured_at = parse_instant(value.get("captured_at"))
    final_url = validate_https_url(value.get("final_url"), "previous final_url")
    comparison = value.get("comparison")
    if not isinstance(comparison, dict):
        raise CaptureManifestError("previous manifest comparison metadata is missing")
    comparison_state = comparison.get("state")
    if comparison_state not in {"baseline", "unchanged", "changed"}:
        raise CaptureManifestError("previous manifest comparison state is missing or invalid")
    comparison_signals = comparison.get("change_signals")
    allowed_signals = {
        "snapshot_bytes_changed",
        "final_url_changed",
        "content_type_changed",
    }
    if (
        not isinstance(comparison_signals, list)
        or any(signal not in allowed_signals for signal in comparison_signals)
        or len(comparison_signals) != len(set(comparison_signals))
    ):
        raise CaptureManifestError("previous manifest change_signals are malformed")
    prior_sha = comparison.get("previous_snapshot_sha256")
    prior_url = comparison.get("previous_final_url")
    prior_content_type = comparison.get("previous_content_type")
    if prior_content_type is not None and not isinstance(prior_content_type, str):
        raise CaptureManifestError("previous manifest comparison content type is malformed")
    expected_signals: list[str] = []
    if prior_sha is not None and snapshot["sha256"] != prior_sha:
        expected_signals.append("snapshot_bytes_changed")
    if prior_url is not None and final_url != prior_url:
        expected_signals.append("final_url_changed")
    if value.get("content_type") != prior_content_type and comparison_state != "baseline":
        expected_signals.append("content_type_changed")
    if comparison_state == "baseline":
        if prior_sha is not None or prior_url is not None or prior_content_type is not None or comparison_signals:
            raise CaptureManifestError("baseline manifest must not claim prior comparison data")
    else:
        if not isinstance(prior_sha, str) or not SHA256_PATTERN.fullmatch(prior_sha):
            raise CaptureManifestError("compared manifest is missing the prior snapshot digest")
        validate_https_url(prior_url, "previous comparison final_url")
        if comparison_signals != expected_signals:
            raise CaptureManifestError("previous manifest comparison signals do not match its recorded values")
        if comparison_state == "unchanged" and expected_signals:
            raise CaptureManifestError("previous manifest claims unchanged despite differing capture metadata")
        if comparison_state == "changed" and not expected_signals:
            raise CaptureManifestError("previous manifest claims changed without a recorded change")
    return {
        "snapshot_sha256": snapshot["sha256"],
        "final_url": final_url,
        "content_type": previous_content_type,
        "captured_at": previous_captured_at,
    }


def build_capture_manifest(
    snapshot_bytes: bytes,
    *,
    source_id: str,
    canonical_url: str,
    captured_at: str,
    final_url: str | None = None,
    content_type: str | None = None,
    previous_manifest: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Hash already-captured bytes and describe only observable capture metadata."""
    if not isinstance(source_id, str) or not SOURCE_ID_PATTERN.fullmatch(source_id):
        raise CaptureManifestError("source_id does not match the pack identifier convention")
    canonical_url = validate_https_url(canonical_url, "canonical_url")
    final_url = validate_https_url(canonical_url if final_url is None else final_url, "final_url")
    captured = parse_instant(captured_at)
    if not isinstance(snapshot_bytes, bytes):
        raise CaptureManifestError("snapshot input must be raw bytes")
    if not snapshot_bytes:
        raise CaptureManifestError("snapshot is empty; refuse to create evidence metadata")
    if content_type is not None:
        if not isinstance(content_type, str) or not content_type.strip():
            raise CaptureManifestError("content_type must be null or a non-empty string")
        content_type = content_type.strip()

    digest = hashlib.sha256(snapshot_bytes).hexdigest()
    comparison_state = "baseline"
    previous_sha: str | None = None
    previous_final_url: str | None = None
    previous_content_type: str | None = None
    change_signals: list[str] = []
    if previous_manifest is not None:
        previous = load_previous_manifest(
            previous_manifest, source_id=source_id, canonical_url=canonical_url
        )
        if captured <= previous["captured_at"]:
            raise CaptureManifestError("captured_at must be strictly later than the previous capture")
        previous_sha = previous["snapshot_sha256"]
        previous_final_url = previous["final_url"]
        previous_content_type = previous["content_type"]
        if digest != previous_sha:
            change_signals.append("snapshot_bytes_changed")
        if final_url != previous_final_url:
            change_signals.append("final_url_changed")
        if content_type != previous_content_type:
            change_signals.append("content_type_changed")
        comparison_state = "changed" if change_signals else "unchanged"

    return {
        "manifest_version": MANIFEST_VERSION,
        "source_id": source_id,
        "canonical_url": canonical_url,
        "final_url": final_url,
        "captured_at": format_instant(captured),
        "capture_method": "offline_sha256_of_provided_bytes",
        "network_access_performed": False,
        "content_type": content_type,
        "snapshot": {
            "sha256": digest,
            "size_bytes": len(snapshot_bytes),
        },
        "comparison": {
            "state": comparison_state,
            "previous_snapshot_sha256": previous_sha,
            "previous_final_url": previous_final_url,
            "previous_content_type": previous_content_type,
            "change_signals": change_signals,
        },
        "human_review_required": True,
        "review_state_changed": False,
    }


def load_json_object(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise CaptureManifestError(f"cannot read JSON object from {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise CaptureManifestError(f"{path} must contain a JSON object")
    return value


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-id", required=True)
    parser.add_argument("--canonical-url", required=True)
    parser.add_argument("--snapshot", required=True, type=Path, help="Already-captured bytes; this tool does not fetch URLs")
    parser.add_argument("--captured-at", required=True, help="Timezone-aware ISO 8601 capture time")
    parser.add_argument("--final-url", help="Observed final URL after redirects")
    parser.add_argument("--content-type", help="Observed Content-Type header, if available")
    parser.add_argument("--previous-manifest", type=Path)
    parser.add_argument("--output", type=Path, help="Write the manifest here; stdout by default")
    args = parser.parse_args(argv)

    try:
        if args.output is not None:
            output_path = args.output.resolve()
            protected_inputs = {args.snapshot.resolve()}
            if args.previous_manifest is not None:
                protected_inputs.add(args.previous_manifest.resolve())
            aliases_input = output_path in protected_inputs
            if not aliases_input and output_path.exists():
                for protected_input in protected_inputs:
                    if protected_input.exists() and os.path.samefile(output_path, protected_input):
                        aliases_input = True
                        break
            if aliases_input:
                raise CaptureManifestError(
                    "--output must not overwrite or alias the snapshot or previous manifest"
                )
        snapshot_bytes = args.snapshot.read_bytes()
        previous = load_json_object(args.previous_manifest) if args.previous_manifest else None
        manifest = build_capture_manifest(
            snapshot_bytes,
            source_id=args.source_id,
            canonical_url=args.canonical_url,
            captured_at=args.captured_at,
            final_url=args.final_url,
            content_type=args.content_type,
            previous_manifest=previous,
        )
        encoded = json.dumps(manifest, ensure_ascii=False, indent=2, sort_keys=True) + "\n"
        if args.output:
            args.output.write_text(encoded, encoding="utf-8")
        else:
            sys.stdout.write(encoded)
        return 0
    except (OSError, CaptureManifestError) as exc:
        sys.stderr.write(f"capture-manifest error: {exc}\n")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
