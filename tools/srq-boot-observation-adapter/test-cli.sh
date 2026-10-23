#!/usr/bin/env bash
set -euo pipefail

manifest_dir="$(cd -- "$(dirname -- "$0")" && pwd)"
manifest="$manifest_dir/Cargo.toml"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

fixture="$tmp/root"
mkdir -p "$fixture/proc/sys/kernel/random" "$fixture/run"
printf '%s\n' '01234567-89ab-cdef-0123-456789abcdef' > "$fixture/proc/sys/kernel/random/boot_id"

store_a="/nix/store/0123456789abcdfghijklmnpqrsvwxyz-nixos-system-srq-cli-a"
store_b="/nix/store/1123456789abcdfghijklmnpqrsvwxyz-nixos-system-srq-cli-b"
ln -s "$store_a" "$fixture/run/booted-system"
ln -s "$store_a" "$fixture/run/current-system"

cargo run --quiet --manifest-path "$manifest" --locked -- --root "$fixture" > "$tmp/consistent.json"
jq -e '
  .schema == "srq-boot-observation-diagnostic-v1" and
  .evidence_tier == "D0-local-diagnostic" and
  .authenticity_status == "NOT_AUTHENTICATED" and
  .status == "OBSERVED_CONSISTENT_DIAGNOSTIC" and
  .promotion_binding_status == "NOT_PROVIDED" and
  .measurement_status == "NOT_MEASURED" and
  .signature_status == "NOT_AUTHENTICATED" and
  .booted_deployment_id == .current_userspace_deployment_id
' "$tmp/consistent.json" >/dev/null
echo "PASS: CLI emits valid D0 JSON and never claims authenticated evidence"

rm "$fixture/run/current-system"
ln -s "$store_b" "$fixture/run/current-system"
set +e
cargo run --quiet --manifest-path "$manifest" --locked -- --root "$fixture" > "$tmp/diverged.json"
exit_code=$?
set -e
[[ "$exit_code" -eq 3 ]] || {
  echo "FAIL: divergent deployment should return exit code 3, received $exit_code" >&2
  exit 1
}
jq -e '
  .status == "BLOCKED" and
  (.reason_codes | index("BOOTED_CURRENT_DIVERGENCE") != null) and
  .authenticity_status == "NOT_AUTHENTICATED" and
  .signature_status == "NOT_AUTHENTICATED"
' "$tmp/diverged.json" >/dev/null
echo "PASS: CLI blocks divergent booted/current deployments with a machine-readable reason"
