#!/usr/bin/env python3
"""Read-only qualification check for security-audit merge protection.

This tool does not change repository rules. It exits successfully only when
GitHub's effective ruleset or classic branch-protection API confirms strict
up-to-date checking and both required contexts are bound to the GitHub Actions
integration ID (15368). Name-only legacy contexts do not qualify.
"""
from __future__ import annotations

import hashlib
import json
import os
import platform
import sys
import urllib.error
import urllib.parse
import urllib.request
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

API = "https://api.github.com"
API_VERSION = "2022-11-28"
GITHUB_ACTIONS_INTEGRATION_ID = 15368
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


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z")


def policy_digest(policy: dict[str, dict[str, Any]] = POLICY) -> str:
    canonical = {
        "schema": SCHEMA,
        "required_verifier_context": VERIFIER_CONTEXT,
        "required_integration_id": GITHUB_ACTIONS_INTEGRATION_ID,
        "repositories": policy,
    }
    encoded = json.dumps(canonical, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def receipt_provenance() -> dict[str, str]:
    """Bind receipt identity to exact running source, policy and interpreter."""
    source_bytes = Path(__file__).read_bytes()
    return {
        "auditor_sha256": hashlib.sha256(source_bytes).hexdigest(),
        "policy_sha256": policy_digest(),
        "python_implementation": platform.python_implementation(),
        "python_version": platform.python_version(),
        "api_version": API_VERSION,
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


def ruleset_identity(rule: Any) -> dict[str, Any] | None:
    """Validate the provenance fields present on an effective branch-rule entry."""
    if not isinstance(rule, dict):
        return None
    ruleset_id = rule.get("ruleset_id")
    source_type = rule.get("ruleset_source_type")
    source = rule.get("ruleset_source")
    if (not isinstance(ruleset_id, int) or isinstance(ruleset_id, bool) or ruleset_id <= 0
            or not isinstance(source_type, str) or source_type not in {"Repository", "Organization", "Enterprise"}
            or not isinstance(source, str) or not source.strip()):
        return None
    return {"ruleset_id": ruleset_id, "source_type": source_type, "source": source}


def ruleset_contexts(rule: Any, required_contexts: set[str]) -> set[str]:
    """Return required contexts bound to Actions in one strict required-check rule."""
    if not isinstance(rule, dict) or rule.get("type") != "required_status_checks":
        return set()
    parameters = rule.get("parameters")
    if not isinstance(parameters, dict) or parameters.get("strict_required_status_checks_policy") is not True:
        return set()
    entries = parameters.get("required_status_checks")
    if not isinstance(entries, list):
        return set()
    by_context: dict[str, list[Any]] = {}
    for item in entries:
        if isinstance(item, dict) and isinstance(item.get("context"), str) and item["context"]:
            by_context.setdefault(item["context"], []).append(item.get("integration_id"))
    return {
        context for context in required_contexts
        if context in by_context
        and all(app_id == GITHUB_ACTIONS_INTEGRATION_ID for app_id in by_context[context])
    }


def classic_bound_contexts(payload: Any, required_contexts: set[str]) -> set[str]:
    """Require modern branch-protection checks[] entries bound to GitHub Actions.

    Legacy contexts[] identifies names only and cannot establish which app supplied
    a status. It is intentionally insufficient for an authorization-grade PASS.
    """
    if not isinstance(payload, dict) or not isinstance(payload.get("checks"), list):
        return set()
    by_context: dict[str, list[Any]] = {}
    for item in payload["checks"]:
        if isinstance(item, dict) and isinstance(item.get("context"), str) and item["context"]:
            by_context.setdefault(item["context"], []).append(item.get("app_id"))
    return {
        context for context in required_contexts
        if context in by_context
        and all(app_id == GITHUB_ACTIONS_INTEGRATION_ID for app_id in by_context[context])
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
    if len(active_rules) > 100:
        return {
            "status": "INCOMPLETE",
            "source": None,
            "observed_contexts": [],
            "missing_contexts": sorted(required_contexts),
            "reason": "effective branch-rules response exceeds the documented 100-entry page limit",
        }

    # Effective branch rules are additive. A strict app-bound required-check rule
    # may come from a repository, organization, or enterprise ruleset; the two
    # contexts need not be stored in the same ruleset.
    qualified_contexts: set[str] = set()
    qualifying_sources: list[dict[str, Any]] = []
    for rule in active_rules:
        identity = ruleset_identity(rule)
        if identity is None:
            return {
                "status": "INCOMPLETE",
                "source": None,
                "observed_contexts": [],
                "missing_contexts": sorted(required_contexts),
                "reason": "effective branch-rules response contains an entry with malformed ruleset provenance",
            }
        contexts = ruleset_contexts(rule, required_contexts)
        if contexts:
            qualified_contexts.update(contexts)
            qualifying_sources.append(identity)
    if required_contexts <= qualified_contexts:
        source_labels = sorted({
            f"{item['source_type']}:{item['source']}#{item['ruleset_id']}"
            for item in qualifying_sources
        })
        return {
            "status": "PASS",
            "source": source_labels,
            "observed_contexts": sorted(qualified_contexts),
            "missing_contexts": [],
            "reason": (
                "active strict required-status-check rules collectively require all contexts, "
                f"each pinned to integration_id {GITHUB_ACTIONS_INTEGRATION_ID}"
            ),
        }

    if len(active_rules) >= 100:
        # Even a well-formed first page may hide additional active rules. If
        # neither the rules page nor a fully qualifying classic policy proved
        # both checks above, absence is not established.
        if (isinstance(classic_protection, dict)
                and classic_protection.get("strict") is True
                and not (required_contexts - classic_bound_contexts(classic_protection, required_contexts))):
            contexts = classic_bound_contexts(classic_protection, required_contexts)
            return {
                "status": "PASS",
                "source": "classic-branch-protection",
                "observed_contexts": sorted(contexts),
                "missing_contexts": [],
                "reason": (
                    "classic required-status-check protection is strict and pins all required contexts "
                    f"to integration_id {GITHUB_ACTIONS_INTEGRATION_ID}"
                ),
            }
        return {
            "status": "INCOMPLETE",
            "source": None,
            "observed_contexts": [],
            "missing_contexts": sorted(required_contexts),
            "reason": "effective branch-rules page may be truncated; unable to prove complete required-check configuration",
        }

    if isinstance(classic_protection, dict):
        observed = classic_contexts(classic_protection)
        contexts = classic_bound_contexts(classic_protection, required_contexts)
        missing = required_contexts - contexts
        if classic_protection.get("strict") is True and not missing:
            return {
                "status": "PASS",
                "source": "classic-branch-protection",
                "observed_contexts": sorted(contexts),
                "missing_contexts": [],
                "reason": (
                    "classic required-status-check protection is strict and pins all required contexts "
                    f"to integration_id {GITHUB_ACTIONS_INTEGRATION_ID}"
                ),
            }
        reasons = []
        if classic_protection.get("strict") is not True:
            reasons.append("classic protection strict is not true")
        if missing:
            reasons.append("required contexts missing, malformed, or not pinned to GitHub Actions")
        return {
            "status": "FAIL",
            "source": "classic-branch-protection",
            "observed_contexts": sorted(observed),
            "bound_contexts": sorted(contexts),
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
        full_name = metadata.get("full_name") if isinstance(metadata, dict) else None
        if not isinstance(full_name, str) or full_name.lower() != repo.lower():
            raise AuditError("repository metadata is missing or names a different repository")
        if metadata.get("default_branch") != branch:
            result.update(
                status="FAIL",
                reason=f"configured branch {branch!r} differs from API default branch {metadata.get('default_branch')!r}",
            )
            return result

        active_rules = get_json(
            f"/repos/{owner}/{name}/rules/branches/{encoded_branch}?per_page=100&page=1", token
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
    selected = list(sys.argv[1:])
    if selected == ["--help"] or selected == ["-h"]:
        print("Usage: audit_security_required_checks.py [Luminous-Dynamics/mycelix] [Luminous-Dynamics/symthaea] [Luminous-Dynamics/luminous-platform]")
        return 0

    token = os.environ.get("GITHUB_TOKEN", "").strip()
    if not token:
        print(json.dumps({
            "schema": SCHEMA,
            **receipt_provenance(),
            "generated_at_utc": utc_now(),
            "status": "INCOMPLETE",
            "reason": "GITHUB_TOKEN is required; use a read-only token with repository metadata and administration-read access",
            "required_verifier_context": VERIFIER_CONTEXT,
            "required_integration_id": GITHUB_ACTIONS_INTEGRATION_ID,
            "repositories": [],
        }, sort_keys=True))
        return 2
    unknown = sorted(set(selected) - set(POLICY))
    if unknown or len(selected) != len(set(selected)):
        print(json.dumps({
            "schema": SCHEMA,
            **receipt_provenance(),
            "generated_at_utc": utc_now(),
            "status": "FAIL",
            "reason": "arguments must be unique repository names from the audited policy; options are not accepted",
            "required_verifier_context": VERIFIER_CONTEXT,
            "required_integration_id": GITHUB_ACTIONS_INTEGRATION_ID,
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
        **receipt_provenance(),
        "generated_at_utc": utc_now(),
        "status": overall,
        "required_verifier_context": VERIFIER_CONTEXT,
        "required_integration_id": GITHUB_ACTIONS_INTEGRATION_ID,
        "repositories": rows,
    }
    print(json.dumps(receipt, sort_keys=True, separators=(",", ":")))
    return 0 if overall == "PASS" else (2 if overall == "INCOMPLETE" else 1)


if __name__ == "__main__":
    raise SystemExit(main())
