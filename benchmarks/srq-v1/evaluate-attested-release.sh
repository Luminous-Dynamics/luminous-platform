#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "$0")" && pwd)"
archive=""
expected_source_sha=""
expected_source_ref=""
expected_workflow_ref=""
expected_run_id=""
expected_run_attempt=""
output_envelope=""
output_report=""
output_verification=""
require_qualified=false

usage() {
  echo "Usage: $0 --archive PATH --expected-source-sha SHA40 --expected-source-ref refs/heads/main --expected-workflow-ref REF --expected-run-id ID --expected-run-attempt N --output-envelope PATH --output-report PATH --output-verification PATH [--require-qualified]" >&2
}

while (($#)); do
  case "$1" in
    --archive) [[ $# -ge 2 ]] || { usage; exit 2; }; archive="$2"; shift 2 ;;
    --expected-source-sha) [[ $# -ge 2 ]] || { usage; exit 2; }; expected_source_sha="$2"; shift 2 ;;
    --expected-source-ref) [[ $# -ge 2 ]] || { usage; exit 2; }; expected_source_ref="$2"; shift 2 ;;
    --expected-workflow-ref) [[ $# -ge 2 ]] || { usage; exit 2; }; expected_workflow_ref="$2"; shift 2 ;;
    --expected-run-id) [[ $# -ge 2 ]] || { usage; exit 2; }; expected_run_id="$2"; shift 2 ;;
    --expected-run-attempt) [[ $# -ge 2 ]] || { usage; exit 2; }; expected_run_attempt="$2"; shift 2 ;;
    --output-envelope) [[ $# -ge 2 ]] || { usage; exit 2; }; output_envelope="$2"; shift 2 ;;
    --output-report) [[ $# -ge 2 ]] || { usage; exit 2; }; output_report="$2"; shift 2 ;;
    --output-verification) [[ $# -ge 2 ]] || { usage; exit 2; }; output_verification="$2"; shift 2 ;;
    --require-qualified) require_qualified=true; shift ;;
    *) usage; exit 2 ;;
  esac
done

[[ -n "$archive" && -n "$output_envelope" && -n "$output_report" && -n "$output_verification" ]] || {
  usage; exit 2;
}
[[ "$output_envelope" != "$archive" && "$output_report" != "$archive" && "$output_verification" != "$archive" ]] || {
  echo "BLOCKED: output paths must not overwrite the attested input archive." >&2; exit 2;
}

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
bundle_root="$tmp/extracted"
manifest_path="$(bash "$script_dir/verify-evidence-bundle.sh" \
  --archive "$archive" \
  --expected-source-sha "$expected_source_sha" \
  --expected-source-ref "$expected_source_ref" \
  --expected-workflow-ref "$expected_workflow_ref" \
  --expected-run-id "$expected_run_id" \
  --expected-run-attempt "$expected_run_attempt" \
  --extract-to "$bundle_root")"
bundle_dir="$(dirname -- "$manifest_path")"

bash "$script_dir/verify-attestation.sh" \
  --artifact "$archive" \
  --expected-source-sha "$expected_source_sha" \
  --expected-source-ref "$expected_source_ref" \
  --output "$tmp/attestation-verification.json"

eval_args=(
  --expected-source-sha "$expected_source_sha"
  --evidence-root "$bundle_dir"
  --output "$tmp/release-evaluation.json"
  "$bundle_dir/srq-update-004-model-result.json"
  "$bundle_dir/srq-evidence-003-model-result.json"
)
if [[ "$require_qualified" == "true" ]]; then
  eval_args+=(--require-qualified)
fi
bash "$script_dir/evaluate-release.sh" "${eval_args[@]}"

mkdir -p "$(dirname -- "$output_report")" "$(dirname -- "$output_envelope")" "$(dirname -- "$output_verification")"
cp "$tmp/release-evaluation.json" "$output_report"
cp "$tmp/attestation-verification.json" "$output_verification"

archive_sha="$(sha256sum "$archive" | cut -d ' ' -f 1)"
manifest_sha="$(sha256sum "$manifest_path" | cut -d ' ' -f 1)"
verification_sha="$(sha256sum "$output_verification" | cut -d ' ' -f 1)"
report_sha="$(sha256sum "$output_report" | cut -d ' ' -f 1)"
jq -n \
  --slurpfile report "$output_report" \
  --arg source_sha "$expected_source_sha" \
  --arg source_ref "$expected_source_ref" \
  --arg workflow_ref "$expected_workflow_ref" \
  --arg run_id "$expected_run_id" \
  --arg run_attempt "$expected_run_attempt" \
  --arg archive_sha "$archive_sha" \
  --arg manifest_sha "$manifest_sha" \
  --arg verification_sha "$verification_sha" \
  --arg report_sha "$report_sha" \
  '{
    schema_version: 1,
    profile_id: "SRQ-1",
    integrity_status: "ATTESTED_EVIDENCE_VERIFIED",
    attestation_policy: {
      repository: "Luminous-Dynamics/luminous-platform",
      signer_workflow: "Luminous-Dynamics/luminous-platform/.github/workflows/srq-v1.yml",
      source_ref: $source_ref,
      source_sha: $source_sha,
      run_id: $run_id,
      run_attempt: $run_attempt,
      predicate_type: "https://slsa.dev/provenance/v1",
      oidc_issuer: "https://token.actions.githubusercontent.com",
      verifier: "GitHub CLI 2.102.0"
    },
    digests: {
      attested_bundle_sha256: $archive_sha,
      validated_manifest_sha256: $manifest_sha,
      verifier_output_sha256: $verification_sha,
      evaluation_report_sha256: $report_sha
    },
    evaluation: {
      overall_status: $report[0].overall_status,
      total_release_gates: $report[0].summary.total_release_gates,
      passed_at_qualifying_tier: $report[0].summary.passed_at_qualifying_tier
    },
    non_claims: [
      "Attestation authenticates workflow provenance and the archived bytes, not the truth of every test assertion.",
      "The source is trusted only under the configured main-branch repository rules and expected source SHA/ref policy.",
      "This report does not establish QEMU, physical-device, firmware-root-of-trust, TPM, or real power-loss qualification.",
      "An ATTESTED_EVIDENCE_VERIFIED integrity status is separate from SRQ release qualification; NOT_QUALIFIED remains NOT_QUALIFIED."
    ]
  }' > "$output_envelope"

printf 'Authenticated evidence integrity: ATTESTED_EVIDENCE_VERIFIED\n'
jq -r '"SRQ release status: \(.evaluation.overall_status) (\(.evaluation.passed_at_qualifying_tier)/\(.evaluation.total_release_gates) qualifying gates)." ' "$output_envelope"
