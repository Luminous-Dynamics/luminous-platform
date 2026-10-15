#!/usr/bin/env python3
"""Enforce append-only review history for every jurisdiction pack changed in a Git range."""
from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path, PurePosixPath
from typing import Any

from check_review_history import check_review_history


def _git(repo: Path, *args: str, check: bool = True) -> bytes:
    result = subprocess.run(
        ["git", "-C", str(repo), *args],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if check and result.returncode != 0:
        message = result.stderr.decode("utf-8", errors="replace").strip()
        raise RuntimeError(f"git {' '.join(args)} failed: {message}")
    return result.stdout


def _exists(repo: Path, object_spec: str) -> bool:
    result = subprocess.run(
        ["git", "-C", str(repo), "cat-file", "-e", object_spec],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    return result.returncode == 0


def _load_pack(repo: Path, object_spec: str) -> dict[str, Any]:
    raw = _git(repo, "show", object_spec)
    value = json.loads(raw)
    if not isinstance(value, dict):
        raise ValueError(f"{object_spec} must contain a JSON object")
    return value


def _bootstrap_errors(pack: dict[str, Any]) -> list[str]:
    """A pack first introduced in this history starts as an unreviewed draft."""
    errors: list[str] = []
    if pack.get("publication_state") != "draft":
        errors.append("new pack must start with publication_state=draft")

    for ledger_name in ("source_review_receipts", "requirement_review_receipts"):
        ledger = pack.get(ledger_name)
        if not isinstance(ledger, list) or ledger:
            errors.append(f"new pack must start with an empty {ledger_name} ledger")

    sources = pack.get("sources")
    if not isinstance(sources, list) or not sources:
        errors.append("new pack must declare at least one source")
    else:
        for index, source in enumerate(sources):
            if not isinstance(source, dict):
                errors.append(f"new pack source at index {index} must be an object")
                continue
            if source.get("review_state") != "unreviewed":
                errors.append(
                    f"new pack source {source.get('source_id', index)} must start unreviewed"
                )
            if source.get("review_receipt_id") is not None:
                errors.append(
                    f"new pack source {source.get('source_id', index)} must start without a review receipt pointer"
                )

    requirements = pack.get("requirements")
    if not isinstance(requirements, list) or not requirements:
        errors.append("new pack must declare at least one requirement")
    else:
        for index, requirement in enumerate(requirements):
            if not isinstance(requirement, dict):
                errors.append(f"new pack requirement at index {index} must be an object")
                continue
            if requirement.get("review_state") != "draft":
                errors.append(
                    f"new pack requirement {requirement.get('requirement_id', index)} must start in draft state"
                )
            if requirement.get("review_receipt_id") is not None:
                errors.append(
                    f"new pack requirement {requirement.get('requirement_id', index)} must start without a review receipt pointer"
                )
    return errors


def _is_pack_path(path: str, pack_dir: str) -> bool:
    candidate = PurePosixPath(path)
    parent = PurePosixPath(pack_dir)
    return candidate.parent == parent and candidate.suffix == ".json"


def check_git_review_history(
    repository: Path,
    base_sha: str,
    head_sha: str,
    pack_dir: str = "examples/jurisdiction-packs",
) -> dict[str, Any]:
    """Compare both review ledgers in every changed pack revision to its first parent."""
    repo = repository.resolve()
    _git(repo, "rev-parse", "--git-dir")
    _git(repo, "merge-base", "--is-ancestor", base_sha, head_sha)
    commits = _git(
        repo, "rev-list", "--reverse", "--first-parent", f"{base_sha}..{head_sha}"
    ).decode("ascii").splitlines()

    errors: list[str] = []
    revisions_checked = 0
    paths_checked: set[str] = set()

    for commit in commits:
        parents = _git(repo, "rev-list", "--parents", "-n", "1", commit).decode("ascii").split()
        if len(parents) < 2:
            errors.append(f"{commit}: expected a first parent for a commit inside the requested range")
            continue
        parent = parents[1]
        changed_output = _git(
            repo,
            "diff-tree",
            "--no-commit-id",
            "--name-only",
            "-r",
            "-z",
            parent,
            commit,
            "--",
            pack_dir,
        )
        changed_paths = sorted(
            {
                item.decode("utf-8", errors="strict")
                for item in changed_output.split(b"\0")
                if item and _is_pack_path(item.decode("utf-8", errors="strict"), pack_dir)
            }
        )

        for path in changed_paths:
            revisions_checked += 1
            paths_checked.add(path)
            current_spec = f"{commit}:{path}"
            parent_spec = f"{parent}:{path}"
            if not _exists(repo, current_spec):
                errors.append(
                    f"{commit} {path}: deleting a jurisdiction pack would erase its in-repository review history; retire it instead"
                )
                continue

            try:
                current = _load_pack(repo, current_spec)
            except (RuntimeError, ValueError, json.JSONDecodeError) as exc:
                errors.append(f"{commit} {path}: cannot load current pack: {exc}")
                continue

            if _exists(repo, parent_spec):
                try:
                    baseline = _load_pack(repo, parent_spec)
                except (RuntimeError, ValueError, json.JSONDecodeError) as exc:
                    errors.append(f"{commit} {path}: cannot load parent pack: {exc}")
                    continue
                for violation in check_review_history(baseline, current):
                    errors.append(f"{commit} {path}: {violation}")
            else:
                for violation in _bootstrap_errors(current):
                    errors.append(f"{commit} {path}: {violation}")

    return {
        "base_sha": base_sha,
        "head_sha": head_sha,
        "commits_checked": len(commits),
        "pack_revisions_checked": revisions_checked,
        "pack_paths_checked": sorted(paths_checked),
        "passed": not errors,
        "errors": errors,
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository", required=True, type=Path)
    parser.add_argument("--base-sha", required=True)
    parser.add_argument("--head-sha", required=True)
    parser.add_argument("--pack-dir", default="examples/jurisdiction-packs")
    args = parser.parse_args(argv)
    try:
        report = check_git_review_history(
            args.repository, args.base_sha, args.head_sha, args.pack_dir
        )
    except (RuntimeError, ValueError, OSError) as exc:
        report = {
            "base_sha": args.base_sha,
            "head_sha": args.head_sha,
            "passed": False,
            "errors": [str(exc)],
        }
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
