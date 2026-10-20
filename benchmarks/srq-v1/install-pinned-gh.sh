#!/usr/bin/env bash
set -euo pipefail

readonly version="2.102.0"
readonly expected_sha256="bb766f710eef8ede859c18578c72c327597cd4c8a85b06001b1f3843c6019386"
readonly archive_name="gh_${version}_linux_amd64.tar.gz"
readonly archive_url="https://github.com/cli/cli/releases/download/v${version}/${archive_name}"

if [[ "$(uname -s)" != "Linux" || "$(uname -m)" != "x86_64" ]]; then
  echo "BLOCKED: this pinned installer supports only Linux x86_64 runners." >&2
  exit 2
fi

: "${RUNNER_TEMP:=${TMPDIR:-/tmp}}"
install_root="${RUNNER_TEMP}/srq-gh-${version}"
mkdir -p "$install_root"
archive_path="$install_root/$archive_name"

curl --fail --location --retry 3 --retry-all-errors --silent --show-error \
  "$archive_url" --output "$archive_path"
printf '%s  %s\n' "$expected_sha256" "$archive_path" | sha256sum --check --status || {
  echo "BLOCKED: downloaded GitHub CLI archive failed pinned SHA-256 verification." >&2
  exit 2
}

tar -xzf "$archive_path" -C "$install_root"
gh_bin="$install_root/gh_${version}_linux_amd64/bin"
version_line="$("$gh_bin/gh" --version | sed -n '1p')"
[[ "$version_line" == "gh version ${version} "* ]] || {
  echo "BLOCKED: installed GitHub CLI version is not ${version}: $version_line" >&2
  exit 2
}

if [[ -n "${GITHUB_PATH:-}" ]]; then
  printf '%s\n' "$gh_bin" >> "$GITHUB_PATH"
else
  echo "Installed pinned GitHub CLI at $gh_bin; add this directory to PATH before verification." >&2
fi

printf 'Pinned verifier installed: %s\n' "$version_line"
