"""Semantic validation for Holochain share packages.

Schema validation is necessary but not sufficient: this module also rejects
reused manifest references even when the surrounding objects differ, and
self-referential disputes/supersessions.
"""
from __future__ import annotations

import json
from pathlib import Path
from typing import Any
from uuid import UUID

from jsonschema import Draft202012Validator, FormatChecker

SCHEMA_PATH = Path(__file__).resolve().parent / "share-package.schema.json"


def load_schema() -> dict[str, Any]:
    with SCHEMA_PATH.open("r", encoding="utf-8") as handle:
        schema = json.load(handle)
    Draft202012Validator.check_schema(schema)
    return schema


def _instance_path(path: Any) -> str:
    parts = [str(part).replace("~", "~0").replace("/", "~1") for part in path]
    return "/" + "/".join(parts) if parts else "/"


def _same_uuid(left: object, right: object) -> bool:
    if not isinstance(left, str) or not isinstance(right, str):
        return False
    try:
        return UUID(left) == UUID(right)
    except ValueError:
        return False


def validate_share_package(instance: object) -> list[str]:
    """Return deterministic validation errors; an empty list means shape and local semantics pass.

    This validates only the share-package contract. It does not authenticate
    issuers, authorize publication, verify evidence signatures, validate
    Holochain DHT state, or prove the represented real-world claim.
    """
    validator = Draft202012Validator(load_schema(), format_checker=FormatChecker())
    schema_errors = [
        f"schema {_instance_path(error.absolute_path)}: failed {error.validator}"
        for error in validator.iter_errors(instance)
    ]
    semantic_errors: list[str] = []

    if isinstance(instance, dict):
        share_id = instance.get("shareId")
        target_id = instance.get("targetShareId")
        record_type = instance.get("recordType")
        if isinstance(record_type, str) and record_type in {"dispute", "supersession"} and _same_uuid(target_id, share_id):
            semantic_errors.append(
                "semantic /targetShareId: dispute/supersession cannot target its own shareId"
            )

        manifests = instance.get("evidenceManifest", [])
        if isinstance(manifests, list):
            seen: set[str] = set()
            for index, item in enumerate(manifests):
                if not isinstance(item, dict):
                    continue
                ref = item.get("manifestRef")
                if not isinstance(ref, str):
                    continue
                if ref in seen:
                    semantic_errors.append(
                        f"semantic /evidenceManifest/{index}/manifestRef: duplicate manifestRef"
                    )
                else:
                    seen.add(ref)

    return sorted(schema_errors + semantic_errors)
