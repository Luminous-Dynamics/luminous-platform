#!/usr/bin/env python3
"""Read-only qualification check for security-audit merge protection.

This tool does not change repository rules. It exits successfully only when
GitHub's effective branch rules OR classic branch-protection API confirm that
both the producer's terminal audit check and the independent verifier status
are required with strict (up-to-date) checking enabled.
"""
from __future__ import annotations

import json
import os
import sys
import urllib.error
import urllib.parse
import urllib.request
from datetime import datetime, timezone
from typing import Any

API = os.environ.get("GITHUB_API_URL", "https://api.github.com").rstrip("/")
API_VERSION = "2022-11-28"
SCHEMA = "luminous.security-audit.required-check-policy.v1"
VERIFIER_CONTEXT = "Security Audit / Independent Verifier"
POLICY: dict[str, dict[str, Any]] = {
    "Luminous-Dynamics/mycelix": {
        "default_branch": "main",
        "producer_context": "Shared security audit / Deterministic audit verdict and evidence completeness",
    },
    "Luminous-Dynamics/symthaea": {
        "default_branch": "main",
        "producer_context": "Shared security audit / Deterministic audit verdict and evidence completeness",
    },
    "Luminous-Dynamics/luminous-platform": {
        "default_branch": "main",
        "producer_context": "Audit platform workflow definitions / Deterministic audit verdict and evidence completeness",
    },
}


class AuditError(RuntimeError):
    """An authoritative API response could not be qualified."""


class ApiError(AuditError):
    def __init__(self, status: int, message: str):
        super().__init__(message)
        self.status = status


def get_json(path: str, token: str) -> Any:
    request = urllib.request.Request(
        API + path,
        method="GET",
        headers={
            "Accept": "application/vnd.github+json",
            "Authorization": f"Bearer {token}",
            "X-GitHub-Api-Version": API_VERSION,
            "User-Agent": "luminous-required-check-policy-auditor/1",
        },
    )
    try:
        with urllib.request.urlopen(request, timeout=20) as response:
            payload = response.read()
    except urllib.error.HTTPError as exc:
        detail = exc.read(1000).decode("utf-8", errors="replace")
        raise ApiError(exc.code, f"GitHub API GET {path}: HTTP {exc.code}: {detail}") from exc
    except (urllib.error.URLError, TimeoutError) as exc:
        raise AuditError(f"GitHub API unavailable for {path}: {type(exc).__name__}") from exc
    try:
        return json.loads(payload)
    except (json.JSONDecodeError, UnicodeDecodeError) as exc:
        raise AuditError(f"GitHub API returned invalid JSON for {path}") from exc


def classic_contexts(payload: Any) -> set[str]:
    """Read modern checks[] and legacy contexts[] from branch protection."""
    if not isinstance(payload, dict):
        return set()
    result: set[str] = set()
    contexts = payload.get("contexts")
    checks = payload.get("checks")
    if isinstance(contexts, list):
        result.update(x for x in contexts if isinstance(x, str) and x)
    if isinstance(checks, list):
        result.update(
            item["context"] for item in checks
            if isinstance(item, dict) and isinstance(item.get("context"), str) and item["context"]
        )
    return result


def ruleset_contexts(rule: Any) -> set[str]:
    if not isinstance(rule, dict) or rule.get("type") != "required_status_checks":
        return set()
    parameters = rule.get("parameters")
    if not isinstance(parameters, dict) or parameters.get("strict_required_status_checks_policy") is not True:
        return set()
    entries = parameters.get("required_status_checks")
    if not isinstance(entries, list):
        return set()
    return {
        item["context"] for item in entries
        if isinstance(item, dict) and isinstance(item.get("context"), str) and item["context"]
    }


def assess_policy(
    active_rules: Any,
    classic_protection: Any | None,
    classic_error_status: int | None,
    required_contexts: set[str],
) -> dict[str, Any]:
    """Return a bounded policy decision without assuming missing fields are safe."""
    if not isinstance(active_rules, list):
        return {
            "status": "INCOMPLETE",
            "source": None,
            "observed_contexts": [],
            "missing_contexts": sorted(required_contexts),
            "reason": "effective branch-rules response is not a list",
        }

    for rule in active_rules:
        contexts = ruleset_contexts(rule)
        if required_contexts <= contexts:
            label = f"ruleset:{rule.get('ruleset_id', 'unknown')}"
            return {
                "status": "PASS",
                "source": label,
                "observed_contexts": sorted(contexts),
                "missing_contexts": [],
                "reason": "active required-status-check rule requires all contexts with strict up-to-date policy",
            }

    if isinstance(classic_protection, dict):
        contexts = classic_contexts(classic_protection)
        missing = required_contexts - contexts
        if classic_protection.get("strict") is True and not missing:
            return {
                "status": "PASS",
                "source": "classic-branch-protection",
                "observed_contexts": sorted(contexts),
                "missing_contexts": [],
                "reason": "classic required-status-check protection is strict and contains all required contexts",
            }
        reasons = []
        if classic_protection.get("strict") is not True:
            reasons.append("classic protection strict is not true")
        if missing:
            reasons.append("required contexts missing from classic protection")
        return {
            "status": "FAIL",
            "source": "classic-branch-protection",
            "observed_contexts": sorted(contexts),
            "missing_contexts": sorted(missing),
            "reason": "; ".join(reasons),
        }

    if classic_error_status == 404:
        return {
            "status": "FAIL",
            "source": None,
            "observed_contexts": [],
            "missing_contexts": sorted(required_contexts),
            "reason": "no strict active ruleset containing all required contexts and no classic required-status-check protection",
        }
    return {
        "status": "INCOMPLETE",
        "source": None,
        "observed_contexts": [],
        "missing_contexts": sorted(required_contexts),
        "reason": f"classic required-status-check policy could not be inspected (HTTP {classic_error_status or 'unknown'})",
    }


def audit_repository(repo: str, policy: dict[str, Any], token: str) -> dict[str, Any]:
    owner, name = repo.split("/", 1)
    branch = policy["default_branch"]
    required = {VERIFIER_CONTEXT, policy["producer_context"]}
    result: dict[str, Any] = {
        "repository": repo,
        "default_branch": branch,
        "required_contexts": sorted(required),
        "status": "INCOMPLETE",
    }
    encoded_branch = urllib.parse.quote(branch, safe="")
    try:
        metadata = get_json(f"/repos/{owner}/{name}", token)
        if not isinstance(metadata, dict) or metadata.get("full_name", "").lower() != repo.lower():
            raise AuditError("repository metadata is missing or names a different repository")
        if metadata.get("default_branch") != branch:
            result.update(
                status="FAIL",
                reason=f"configured branch {branch!r} differs from API default branch {metadata.get('default_branch')!r}",
            )
            return result

        active_rules = get_json(
            f"/repos/{owner}/{name}/rules/branches/{encoded_branch}", token
        )
        classic: Any | None = None
        classic_error_status: int | None = None
        try:
            classic = get_json(
                f"/repos/{owner}/{name}/branches/{encoded_branch}/protection/required_status_checks",
                token,
            )
        except ApiError as exc:
            classic_error_status = exc.status
        decision = assess_policy(active_rules, classic, classic_error_status, required)
        result.update(decision)
        return result
    except ApiError as exc:
        result.update(status="INCOMPLETE", reason=str(exc))
        return result
    except AuditError as exc:
        result.update(status="INCOMPLETE", reason=str(exc))
        return result


def main() -> int:
    token = os.environ.get("GITHUB_TOKEN", "").strip()
    if not token:
        print(json.dumps({
            "schema": SCHEMA,
            "generated_at_utc": datetime.now(timezone.utc).isoformat(timespec="seconds"),
            "status": "INCOMPLETE",
            "reason": "GITHUB_TOKEN is required; use a read-only token with repository metadata and administration-read access",
            "repositories": [],
        }, sort_keys=True))
        return 2

    selected = [arg for arg in sys.argv[1:] if not arg.startswith("-")]
    unknown = sorted(set(selected) - set(POLICY))
    if unknown or len(selected) != len(set(selected)):
        print(json.dumps({
            "schema": SCHEMA,
            "generated_at_utc": datetime.now(timezone.utc).isoformat(timespec="seconds"),
            "status": "FAIL",
            "reason": "repository arguments must be unique members of the audited repository policy",
            "unknown_repositories": unknown,
            "repositories": [],
        }, sort_keys=True))
        return 1

    targets = selected or sorted(POLICY)
    rows = [audit_repository(repo, POLICY[repo], token) for repo in targets]
    statuses = {row["status"] for row in rows}
    overall = "PASS" if statuses == {"PASS"} else ("INCOMPLETE" if "INCOMPLETE" in statuses else "FAIL")
    receipt = {
        "schema": SCHEMA,
        "generated_at_utc": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "status": overall,
        "required_verifier_context": VERIFIER_CONTEXT,
        "repositories": rows,
    }
    print(json.dumps(receipt, sort_keys=True, separators=(",", ":")))
    return 0 if overall == "PASS" else (2 if overall == "INCOMPLETE" else 1)


if __name__ == "__main__":
    raise SystemExit(main())
