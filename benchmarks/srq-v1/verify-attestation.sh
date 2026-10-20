#!/usr/bin/env bash
set -euo pipefail

readonly trusted_repo="Luminous-Dynamics/luminous-platform"
readonly trusted_workflow="${trusted_repo}/.github/workflows/srq-v1.yml"
readonly trusted_source_ref="refs/heads/main"
readonly trusted_oidc_issuer="https://token.actions.githubusercontent.com"
readonly trusted_predicate_type="https://slsa.dev/provenance/v1"
readonly pinned_gh_version="2.102.0"

artifact=""
expected_source_sha=""
expected_source_ref=""
output="-"

usage() {
  echo "Usage: $0 --artifact PATH --expected-source-sha SHA40 --expected-source-ref refs/heads/main [--output JSON]" >&2
}

while (($#)); do
  case "$1" in
    --artifact) [[ $# -ge 2 ]] || { usage; exit 2; }; artifact="$2"; shift 2 ;;
    --expected-source-sha) [[ $# -ge 2 ]] || { usage; exit 2; }; expected_source_sha="$2"; shift 2 ;;
    --expected-source-ref) [[ $# -ge 2 ]] || { usage; exit 2; }; expected_source_ref="$2"; shift 2 ;;
    --output) [[ $# -ge 2 ]] || { usage; exit 2; }; output="$2"; shift 2 ;;
    *) usage; exit 2 ;;
  esac
done

[[ -n "$artifact" && -f "$artifact" && ! -L "$artifact" ]] || {
  echo "BLOCKED: attestation subject must be an existing regular file, not a symlink." >&2
  exit 2
}
[[ "$expected_source_sha" =~ ^[0-9a-f]{40}$ ]] || {
  echo "BLOCKED: expected source SHA must be 40 lowercase hexadecimal characters." >&2
  exit 2
}
[[ "$expected_source_ref" == "$trusted_source_ref" ]] || {
  echo "BLOCKED: SRQ-1 provenance policy accepts only refs/heads/main." >&2
  exit 2
}
command -v gh >/dev/null 2>&1 || {
  echo "BLOCKED: pinned GitHub CLI verifier is not installed." >&2
  exit 2
}
command -v jq >/dev/null 2>&1 || {
  echo "BLOCKED: jq is required to validate verifier output." >&2
  exit 2
}

version_line="$(gh --version | sed -n '1p')"
[[ "$version_line" == "gh version ${pinned_gh_version} "* ]] || {
  echo "BLOCKED: expected GitHub CLI ${pinned_gh_version}; found: $version_line" >&2
  exit 2
}

# The CLI verifies the signature, subject digest, OIDC identity, predicate type,
# source repository, exact workflow path, source commit and source ref.
if ! verification_json="$(gh attestation verify "$artifact" \
  --repo "$trusted_repo" \
  --signer-workflow "$trusted_workflow" \
  --cert-oidc-issuer "$trusted_oidc_issuer" \
  --predicate-type "$trusted_predicate_type" \
  --source-digest "$expected_source_sha" \
  --source-ref "$expected_source_ref" \
  --format json)"; then
  echo "BLOCKED: GitHub attestation verification rejected or could not verify the evidence bundle." >&2
  exit 1
fi

if ! printf '%s\n' "$verification_json" | jq -e 'type == "array" and length > 0' >/dev/null; then
  echo "BLOCKED: verifier returned no machine-readable verified attestation." >&2
  exit 1
fi

if [[ "$output" == "-" ]]; then
  printf '%s\n' "$verification_json"
else
  printf '%s\n' "$verification_json" > "$output"
fi
echo "ATTESTED_EVIDENCE_VERIFIED: repo, signer workflow, OIDC issuer, predicate, source ref, source SHA and artifact digest were checked." >&2
