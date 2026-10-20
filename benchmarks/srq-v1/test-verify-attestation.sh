#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "$0")" && pwd)"
verifier="$script_dir/verify-attestation.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$tmp/bin"
artifact="$tmp/evidence.tar.gz"
printf 'synthetic subject; no cryptographic claim\n' > "$artifact"
source_sha="0123456789abcdef0123456789abcdef01234567"
args_file="$tmp/gh-args"

cat > "$tmp/bin/gh" <<'FAKE_GH'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "--version" ]]; then
  printf 'gh version %s (test double)\n' "${FAKE_GH_VERSION:-2.102.0}"
  exit 0
fi
printf '%s\n' "$@" > "$GH_ARGS_FILE"
[[ "${1:-}" == "attestation" && "${2:-}" == "verify" ]] || exit 91
case "${FAKE_GH_MODE:-pass}" in
  pass) printf '[{"verificationResult":{"statement":{"subject":[{"digest":{"sha256":"${FAKE_SUBJECT_SHA:-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa}"}}]}}}]\n' ;;
  reject) echo "synthetic signature failure" >&2; exit 1 ;;
  empty) printf '[]\n' ;;
  invalid-json) printf 'not-json\n' ;;
  *) exit 92 ;;
esac
FAKE_GH
chmod +x "$tmp/bin/gh"

fail() { echo "FAIL: $*" >&2; exit 1; }
expect_reject() {
  local expected="$1"
  shift
  if PATH="$tmp/bin:$PATH" GH_ARGS_FILE="$args_file" "$@" >/dev/null 2>&1; then
    fail "$expected was unexpectedly accepted"
  fi
}

PATH="$tmp/bin:$PATH" GH_ARGS_FILE="$args_file" \
  bash "$verifier" --artifact "$artifact" --expected-source-sha "$source_sha" \
  --expected-source-ref refs/heads/main > "$tmp/verification.json"
jq -e 'type == "array" and length == 1' "$tmp/verification.json" >/dev/null
grep -Fxq -- "Luminous-Dynamics/luminous-platform" "$args_file" || fail "trusted repository was not enforced"
grep -Fxq -- "Luminous-Dynamics/luminous-platform/.github/workflows/srq-v1.yml" "$args_file" || fail "trusted signer workflow was not enforced"
grep -Fxq -- "https://token.actions.githubusercontent.com" "$args_file" || fail "OIDC issuer was not enforced"
grep -Fxq -- "https://slsa.dev/provenance/v1" "$args_file" || fail "SLSA predicate type was not enforced"
grep -Fxq -- "$source_sha" "$args_file" || fail "exact source SHA was not passed to verifier"
grep -Fxq -- "refs/heads/main" "$args_file" || fail "trusted source ref was not passed to verifier"
echo "PASS: attestation verifier invokes the pinned CLI under the fixed SRQ trust policy"

if PATH="$tmp/bin:$PATH" GH_ARGS_FILE="$args_file" FAKE_GH_VERSION=2.101.0 \
  bash "$verifier" --artifact "$artifact" --expected-source-sha "$source_sha" \
  --expected-source-ref refs/heads/main >/dev/null 2>&1; then
  fail "wrong GitHub CLI version was accepted"
fi
echo "PASS: wrong verifier version fails closed"

if PATH="$tmp/bin:$PATH" GH_ARGS_FILE="$args_file" FAKE_GH_MODE=reject \
  bash "$verifier" --artifact "$artifact" --expected-source-sha "$source_sha" \
  --expected-source-ref refs/heads/main >/dev/null 2>&1; then
  fail "failed cryptographic-verifier result was accepted"
fi
echo "PASS: CLI signature/identity rejection fails closed"

for mode in empty invalid-json; do
  if PATH="$tmp/bin:$PATH" GH_ARGS_FILE="$args_file" FAKE_GH_MODE="$mode" \
    bash "$verifier" --artifact "$artifact" --expected-source-sha "$source_sha" \
    --expected-source-ref refs/heads/main >/dev/null 2>&1; then
    fail "empty or malformed verifier output was accepted ($mode)"
  fi
done
echo "PASS: empty or malformed verifier output fails closed"

if PATH="$tmp/bin:$PATH" GH_ARGS_FILE="$args_file" \
  bash "$verifier" --artifact "$artifact" --expected-source-sha "bad-sha" \
  --expected-source-ref refs/heads/main >/dev/null 2>&1; then
  fail "malformed source SHA was accepted"
fi
if PATH="$tmp/bin:$PATH" GH_ARGS_FILE="$args_file" \
  bash "$verifier" --artifact "$artifact" --expected-source-sha "$source_sha" \
  --expected-source-ref refs/heads/research >/dev/null 2>&1; then
  fail "non-main source ref was accepted"
fi
echo "PASS: malformed SHA and non-policy refs fail closed"

if PATH="$tmp/bin:$PATH" GH_ARGS_FILE="$args_file" \
  bash "$verifier" --artifact "$tmp/missing.tar.gz" --expected-source-sha "$source_sha" \
  --expected-source-ref refs/heads/main >/dev/null 2>&1; then
  fail "missing subject was accepted"
fi
echo "PASS: missing evidence subject fails closed"
