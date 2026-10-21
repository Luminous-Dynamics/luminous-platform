#!/usr/bin/env bash
set -euo pipefail

manifest="tools/srq-eat-cose-verifier/Cargo.toml"

help_output="$(cargo run --manifest-path "$manifest" --locked --quiet -- --help 2>&1)"
printf '%s\n' "$help_output"
grep -Fq 'Usage: srq-eat-cose-verify --token FILE --trust-policy JSON --challenge JSON' <<< "$help_output"
if grep -Fq -- '--now-epoch' <<< "$help_output"; then
  echo "FAIL: public CLI advertises caller-controlled verifier time." >&2
  exit 1
fi
echo "PASS: CLI help does not expose a caller-supplied verification clock"

set +e
output="$(cargo run --manifest-path "$manifest" --locked --quiet -- --now-epoch 4102444800 2>&1)"
status=$?
set -e
if [[ "$status" -eq 0 ]]; then
  echo "FAIL: caller-supplied future time was accepted." >&2
  exit 1
fi
grep -Fq 'unknown argument: --now-epoch' <<< "$output"
echo "PASS: CLI rejects caller-supplied future time before reading inputs"

# Deterministic expiry/freshness tests remain in the Rust library tests, where
# time is an explicit test parameter. Production CLI gets time from SystemTime.
echo "LIMIT: SystemTime still depends on the verifier host clock; this change does not establish a hardware-backed trusted clock."
