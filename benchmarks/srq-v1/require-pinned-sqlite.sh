#!/usr/bin/env bash
set -euo pipefail

readonly expected_version="3.54.1"
readonly expected_source_id="2026-10-16 14:45:20 555e07ef33f69a0957d24d8e555b702464a131d6f1607f4c3c1e4551e8e6ca76"

command -v sqlite3 >/dev/null 2>&1 || {
  echo "BLOCKED: sqlite3 CLI is required." >&2
  exit 2
}
version_line="$(sqlite3 --version)"
[[ "$version_line" == "${expected_version} "* ]] || {
  echo "BLOCKED: reference tests require pinned SQLite ${expected_version}; got: $version_line" >&2
  exit 2
}
source_id="$(sqlite3 :memory: 'SELECT sqlite_source_id();')"
[[ "$source_id" == "$expected_source_id" ]] || {
  echo "BLOCKED: reference test SQLite source ID is not the pinned upstream build." >&2
  exit 2
}
printf 'PASS: reference SQLite is pinned: %s\n' "$version_line"
