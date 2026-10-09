#!/usr/bin/env bash
set -euo pipefail

# Fetch, normalize, and retain the publisher bytes as a new bundle.
# This hashes the files but cannot cryptographically authenticate SIX.

readonly SOURCE_URL="https://www.six-group.com/dam/download/financial-information/data-center/iso-currrency/lists/list-one.xml"
readonly IMPORTER_ID="luminous-six-iso4217-list-one-xml/v1"
readonly SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
readonly MANIFEST_PATH="${SCRIPT_DIR}/../crates/cooperative-commerce-core/Cargo.toml"

if [[ "$#" -ne 1 ]]; then
  printf 'Usage: %s <snapshot-parent-directory>\n' "$0" >&2
  exit 64
fi

command -v curl >/dev/null 2>&1 || { echo "required command not found: curl" >&2; exit 69; }
command -v sha256sum >/dev/null 2>&1 || { echo "required command not found: sha256sum" >&2; exit 69; }
command -v cargo >/dev/null 2>&1 || { echo "required command not found: cargo" >&2; exit 69; }
command -v date >/dev/null 2>&1 || { echo "required command not found: date" >&2; exit 69; }

output_parent="$1"
mkdir -p -- "$output_parent"
output_parent="$(cd -- "$output_parent" && pwd -P)"
tmp_dir="$(mktemp -d "${output_parent}/.six-list-one.XXXXXX")"
cleanup() {
  if [[ -n "${tmp_dir:-}" && -d "$tmp_dir" ]]; then
    rm -rf -- "$tmp_dir"
  fi
}
trap cleanup EXIT

curl \
  --fail \
  --silent \
  --show-error \
  --proto '=https' \
  --tlsv1.2 \
  --connect-timeout 15 \
  --max-time 60 \
  --max-filesize 10485760 \
  --output "${tmp_dir}/list-one.xml" \
  "$SOURCE_URL"

retrieved_at="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
snapshot_stamp="$(date -u '+%Y%m%dT%H%M%SZ')"

cargo run --locked --quiet \
  --manifest-path "$MANIFEST_PATH" \
  --bin six-iso4217-import -- "${tmp_dir}/list-one.xml" \
  > "${tmp_dir}/list-one.normalized.json"

source_sha256="$(sha256sum "${tmp_dir}/list-one.xml" | cut -d ' ' -f 1)"
normalized_sha256="$(sha256sum "${tmp_dir}/list-one.normalized.json" | cut -d ' ' -f 1)"
[[ "$source_sha256" =~ ^[a-f0-9]{64}$ ]] || { echo "invalid source SHA-256 output" >&2; exit 70; }
[[ "$normalized_sha256" =~ ^[a-f0-9]{64}$ ]] || { echo "invalid normalized SHA-256 output" >&2; exit 70; }

cat > "${tmp_dir}/provenance.json" <<EOF
{
  "schema": "luminous.code-list-acquisition-record/v1",
  "publisher": "SIX",
  "source_url": "${SOURCE_URL}",
  "retrieved_at": "${retrieved_at}",
  "source_payload_sha256": "${source_sha256}",
  "normalized_payload_sha256": "${normalized_sha256}",
  "importer_id": "${IMPORTER_ID}",
  "authentication_status": "not_independently_authenticated",
  "review_status": "not_reviewed",
  "registry_activation": "disabled"
}
EOF

bundle_name="six-list-one-${snapshot_stamp}-${source_sha256:0:12}"
bundle_path="${output_parent}/${bundle_name}"
if [[ -e "$bundle_path" || -L "$bundle_path" ]]; then
  printf 'Refusing to overwrite an existing snapshot: %s\n' "$bundle_path" >&2
  exit 73
fi

# Same-filesystem rename makes the retained three-file bundle visible together.
mv -T -- "$tmp_dir" "$bundle_path"
tmp_dir=""
printf 'Snapshot retained at: %s\n' "$bundle_path"
printf 'Source SHA-256:       %s\n' "$source_sha256"
printf 'Normalized SHA-256:   %s\n' "$normalized_sha256"
printf 'Authentication:       NOT VERIFIED\n'
printf 'Activation:           DISABLED\n'
