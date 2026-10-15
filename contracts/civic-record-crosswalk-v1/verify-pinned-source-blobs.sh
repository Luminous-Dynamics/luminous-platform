#!/usr/bin/env bash
# Verify that each public artifact named by the civic crosswalk exists at the
# pinned full commit and hashes to the recorded Git blob SHA-1. This is a
# read-only audit step; it never executes code from a source repository.
set -euo pipefail

manifest="${1:-contracts/civic-record-crosswalk-v1/crosswalk.json}"
if [[ "${2:-}" != "" || ! -f "$manifest" ]]; then
  echo "usage: $0 [path/to/crosswalk.json]" >&2
  exit 2
fi

for command_name in jq curl git; do
  command -v "$command_name" >/dev/null 2>&1 || {
    printf 'required command not found: %s\n' "$command_name" >&2
    exit 2
  }
done

tmpdir="$(mktemp -d)"
trap 'rm -rf -- "$tmpdir"' EXIT
pins_file="$tmpdir/pins.tsv"
download_file="$tmpdir/source"
jq -er '
  [.contracts[] | select(.availability == "present_at_snapshot")
   | [.repository, .snapshot_commit_sha, .path, .git_blob_sha1] | @tsv]
  | .[]
' "$manifest" > "$pins_file"

pin_count="$(wc -l < "$pins_file" | tr -d '[:space:]')"
if [[ ! "$pin_count" =~ ^[1-9][0-9]*$ ]]; then
  echo "crosswalk contains no present-at-snapshot artifacts to verify" >&2
  exit 1
fi

verified=0
while IFS=$'\t' read -r repository revision path expected_blob; do
  if [[ ! "$repository" =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ ]]; then
    printf 'invalid repository locator: %s\n' "$repository" >&2
    exit 1
  fi
  if [[ ! "$revision" =~ ^[a-f0-9]{40}$ ]]; then
    printf 'invalid full commit SHA for %s: %s\n' "$repository" "$revision" >&2
    exit 1
  fi
  if [[ ! "$expected_blob" =~ ^[a-f0-9]{40}$ ]]; then
    printf 'invalid Git blob SHA-1 for %s:%s\n' "$repository" "$path" >&2
    exit 1
  fi
  if [[ ! "$path" =~ ^[A-Za-z0-9._/-]+$ || "$path" == /* ]]; then
    printf 'invalid repository-relative path: %s\n' "$path" >&2
    exit 1
  fi
  case "/$path/" in
    *"/../"*|*"/./"*)
      printf 'unsafe path component in source pin: %s\n' "$path" >&2
      exit 1
      ;;
  esac

  url="https://raw.githubusercontent.com/$repository/$revision/$path"
  if ! curl --fail --location --silent --show-error --retry 2 --retry-delay 1 \
      --max-time 30 --output "$download_file" "$url"; then
    printf 'failed to retrieve pinned public artifact: %s@%s:%s\n' \
      "$repository" "$revision" "$path" >&2
    exit 1
  fi

  actual_blob="$(git hash-object "$download_file")"
  if [[ "$actual_blob" != "$expected_blob" ]]; then
    printf 'blob mismatch for %s@%s:%s (expected %s, got %s)\n' \
      "$repository" "$revision" "$path" "$expected_blob" "$actual_blob" >&2
    exit 1
  fi
  printf 'verified %s@%s:%s -> %s\n' "$repository" "$revision" "$path" "$actual_blob"
  verified=$((verified + 1))
done < "$pins_file"

printf 'verified %s pinned public artifacts\n' "$verified"
