#!/usr/bin/env bash
set -euo pipefail

readonly version="3.54.1"
readonly archive_basename="sqlite-autoconf-3540100.tar.gz"
readonly archive_url="https://www.sqlite.org/2026/${archive_basename}"
readonly expected_sha3_256="b6ced585c750916b1052667239a12c25a7ff34e7f06e1e6e01ecb0427657d9c4"
readonly expected_source_id="2026-10-16 14:45:20 555e07ef33f69a0957d24d8e555b702464a131d6f1607f4c3c1e4551e8e6ca76"

[[ "$(uname -s)" == "Linux" && "$(uname -m)" == "x86_64" ]] || {
  echo "BLOCKED: pinned SQLite CLI installer supports Linux x86_64 only." >&2
  exit 2
}
for command_name in curl openssl tar make gcc; do
  command -v "$command_name" >/dev/null 2>&1 || {
    echo "BLOCKED: required build tool is missing: $command_name" >&2
    exit 2
  }
done

: "${RUNNER_TEMP:=${TMPDIR:-/tmp}}"
readonly root="${RUNNER_TEMP}/srq-sqlite-${version}"
readonly archive_path="${root}/${archive_basename}"
readonly prefix="${root}/install"
mkdir -p "$root"
curl --fail --location --retry 3 --retry-all-errors --silent --show-error \
  --output "$archive_path" "$archive_url"

actual_sha3_256="$(openssl dgst -sha3-256 "$archive_path" | awk '{print $NF}')"
[[ "$actual_sha3_256" == "$expected_sha3_256" ]] || {
  echo "BLOCKED: SQLite source archive SHA3-256 mismatch." >&2
  exit 2
}

mkdir -p "$root/source"
tar --extract --gzip --file "$archive_path" --directory "$root/source" --strip-components=1
cd "$root/source"
CFLAGS="-O2 -DSQLITE_THREADSAFE=1" ./configure --prefix="$prefix" --disable-readline
make -j2
make install

sqlite_bin="$prefix/bin/sqlite3"
version_line="$("$sqlite_bin" --version)"
[[ "$version_line" == "${version} "* ]] || {
  echo "BLOCKED: built SQLite CLI has unexpected version: $version_line" >&2
  exit 2
}
source_id="$("$sqlite_bin" :memory: 'SELECT sqlite_source_id();')"
[[ "$source_id" == "$expected_source_id" ]] || {
  echo "BLOCKED: built SQLite source ID does not match the pinned upstream release." >&2
  exit 2
}

if [[ -n "${GITHUB_PATH:-}" ]]; then
  printf '%s\n' "$prefix/bin" >> "$GITHUB_PATH"
fi
printf 'Pinned SQLite CLI: %s\n' "$version_line"
printf 'SQLite source ID: %s\n' "$source_id"
printf 'SQLite archive SHA3-256: %s\n' "$actual_sha3_256"
