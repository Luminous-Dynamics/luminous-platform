#!/usr/bin/env bash
set -euo pipefail
public_key=""
expected_key_id=""
expected_run_challenge=""
evidence_root=""
envelope=""
usage() { echo "Usage: $0 --public-key KEY.pem --expected-key-id ID [--expected-run-challenge HEX64] --evidence-root DIR ENVELOPE.json" >&2; }
fail() { echo "NOT_VERIFIED: $*" >&2; exit 1; }
safe_relative() { local p="$1"; [[ "$p" =~ ^[A-Za-z0-9][A-Za-z0-9._/-]*$ && "$p" != *".."* && "$p" != *"//"* ]]; }
while (($#)); do
  case "$1" in
    --public-key) [[ $# -ge 2 ]] || { usage; exit 2; }; public_key="$2"; shift 2 ;;
    --expected-key-id) [[ $# -ge 2 ]] || { usage; exit 2; }; expected_key_id="$2"; shift 2 ;;
    --expected-run-challenge) [[ $# -ge 2 ]] || { usage; exit 2; }; expected_run_challenge="$2"; shift 2 ;;
    --evidence-root) [[ $# -ge 2 ]] || { usage; exit 2; }; evidence_root="$2"; shift 2 ;;
    -*) usage; exit 2 ;;
    *) [[ -z "$envelope" ]] || { usage; exit 2; }; envelope="$1"; shift ;;
  esac
done
[[ -f "$public_key" && -n "$expected_key_id" && -d "$evidence_root" && -f "$envelope" ]] || { usage; exit 2; }
command -v jq >/dev/null 2>&1 || { echo "BLOCKED: jq is required." >&2; exit 2; }
command -v openssl >/dev/null 2>&1 || { echo "BLOCKED: openssl is required." >&2; exit 2; }
command -v base64 >/dev/null 2>&1 || { echo "BLOCKED: base64 is required." >&2; exit 2; }
openssl pkey -pubin -in "$public_key" -text -noout 2>/dev/null | grep -q "ED25519 Public-Key" || fail "attestor trust key is not Ed25519"
jq -e --arg kid "$expected_key_id" '
  type == "object" and .payloadType == "application/vnd.in-toto+json" and
  (.payload | type == "string" and length > 0) and
  (.signatures | type == "array" and length == 1 and .[0].keyid == $kid and (.[0].sig | type == "string" and length > 0))
' "$envelope" >/dev/null || fail "DSSE envelope has invalid shape or unexpected key ID"

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT
payload_file="$tmpdir/payload.json"
canonical_file="$tmpdir/canonical.json"
pae_file="$tmpdir/pae.bin"
sig_file="$tmpdir/signature.bin"
jq -r '.payload' "$envelope" | base64 --decode > "$payload_file" 2>/dev/null || fail "payload is not valid base64"
jq -cS '.' "$payload_file" > "$canonical_file" || fail "payload is not valid JSON"
cmp -s "$payload_file" "$canonical_file" || fail "payload is not canonical compact sorted JSON"
payload_type="$(jq -r '.payloadType' "$envelope")"
type_len="$(printf '%s' "$payload_type" | wc -c | tr -d ' ')"
payload_len="$(wc -c < "$payload_file" | tr -d ' ')"
{ printf 'DSSEv1 %s %s %s ' "$type_len" "$payload_type" "$payload_len"; cat "$payload_file"; } > "$pae_file"
jq -r '.[0].sig' < <(jq '.signatures' "$envelope") | base64 --decode > "$sig_file" 2>/dev/null || fail "signature is not valid base64"
[[ "$(wc -c < "$sig_file" | tr -d ' ')" == "64" ]] || fail "signature is not Ed25519-sized"
openssl pkeyutl -verify -pubin -inkey "$public_key" -rawin -in "$pae_file" -sigfile "$sig_file" >/dev/null 2>&1 || fail "DSSE signature did not verify"

jq -e '
  ._type == "https://in-toto.io/Statement/v1" and
  .predicateType == "https://luminousdynamics.io/attestations/srq-matched-comparison/v1" and
  .predicate.schema_version == 1 and
  .predicate.profile_id == "SRQ-MATCHED-COMPARISON-ATTESTATION-V1" and
  (.predicate.evidence_status == "fixture_only" or .predicate.evidence_status == "submitted_for_appraisal") and
  (.predicate.run_challenge | type == "string" and test("^[0-9a-f]{64}$")) and
  .predicate.claim_ceiling == "artifact_integrity_and_structural_comparability_only" and
  (.subject | type == "array" and length >= 2) and
  (([.subject[].name] | length) == ([.subject[].name] | unique | length)) and
  all(.subject[];
    (.name | type == "string" and test("^[A-Za-z0-9][A-Za-z0-9._/-]*$") and (contains("..") | not) and (contains("//") | not)) and
    (.digest.sha256 | type == "string" and test("^[0-9a-f]{64}$")))
' "$payload_file" >/dev/null || fail "statement failed semantic profile checks"

evidence_status="$(jq -r '.predicate.evidence_status' "$payload_file")"
statement_challenge="$(jq -r '.predicate.run_challenge' "$payload_file")"
if [[ -n "$expected_run_challenge" && ! "$expected_run_challenge" =~ ^[0-9a-f]{64}$ ]]; then
  fail "expected run challenge must be 256-bit lowercase hexadecimal"
fi
if [[ "$evidence_status" == "submitted_for_appraisal" ]]; then
  [[ "$expected_run_challenge" =~ ^[0-9a-f]{64}$ ]] || fail "non-fixture appraisal requires an externally issued expected run challenge"
fi
if [[ -n "$expected_run_challenge" && "$statement_challenge" != "$expected_run_challenge" ]]; then
  fail "statement run challenge differs from verifier-issued expected challenge"
fi

root_real="$(realpath -e "$evidence_root")"
while IFS= read -r relative; do
  safe_relative "$relative" || fail "unsafe subject path: $relative"
  resolved="$(realpath -e "$root_real/$relative")" || fail "missing subject file: $relative"
  case "$resolved" in "$root_real"/*) ;; *) fail "subject path escapes evidence root: $relative" ;; esac
  [[ -f "$resolved" ]] || fail "subject is not a regular file: $relative"
  expected="$(jq -r --arg p "$relative" '.subject[] | select(.name == $p) | .digest.sha256' "$payload_file")"
  actual="$(sha256sum "$resolved" | cut -d ' ' -f 1)"
  [[ "$actual" == "$expected" ]] || fail "subject digest mismatch: $relative"
done < <(jq -r '.subject[].name' "$payload_file")

lock_path="$(jq -r '.predicate.comparison_lock_path' "$payload_file")"
run_path="$(jq -r '.predicate.comparison_run_path' "$payload_file")"
safe_relative "$lock_path" && safe_relative "$run_path" || fail "statement lock/run paths are unsafe"
lock_file="$(realpath -e "$root_real/$lock_path")"
run_file="$(realpath -e "$root_real/$run_path")"
lock_subject_sha="$(jq -r --arg p "$lock_path" '.subject[] | select(.name == $p) | .digest.sha256' "$payload_file")"
run_subject_sha="$(jq -r --arg p "$run_path" '.subject[] | select(.name == $p) | .digest.sha256' "$payload_file")"
[[ "$(sha256sum "$lock_file" | cut -d ' ' -f 1)" == "$lock_subject_sha" ]] || fail "comparison lock subject mismatch"
[[ "$(sha256sum "$run_file" | cut -d ' ' -f 1)" == "$run_subject_sha" ]] || fail "comparison run subject mismatch"
[[ "$lock_subject_sha" == "$(jq -r '.predicate.comparison_lock_sha256' "$payload_file")" ]] || fail "statement lock digest field mismatch"
[[ "$run_subject_sha" == "$(jq -r '.predicate.comparison_run_sha256' "$payload_file")" ]] || fail "statement run digest field mismatch"
jq -e --arg challenge "$(jq -r '.predicate.run_challenge' "$payload_file")" \
  --arg lock_sha "$lock_subject_sha" --arg baseline "$(jq -r '.predicate.baseline_id' "$payload_file")" '
  .run_challenge == $challenge and .comparison_lock.lock_sha256 == $lock_sha and .comparison_lock.baseline_id == $baseline
' "$run_file" >/dev/null || fail "run receipt does not match the signed statement predicate"
jq -e --arg baseline "$(jq -r '.predicate.baseline_id' "$payload_file")" '
  .baseline_id == $baseline
' "$lock_file" >/dev/null || fail "lock baseline identity does not match statement predicate"
jq -e --slurpfile run "$run_file" --slurpfile lock "$lock_file" '
  .predicate.artifacts == $run[0].artifacts and
  .predicate.source == $run[0].source and
  .predicate.environment == $run[0].environment and
  .predicate.pairing == $run[0].pairing and
  .predicate.non_claims == $run[0].non_claims and
  .predicate.evidence_status == (if $run[0].fixture then "fixture_only" else "submitted_for_appraisal" end) and
  $lock[0].fixture == $run[0].fixture and
  $lock[0].srq_source_sha == $run[0].source.actual_srq_checkout_sha
' "$payload_file" >/dev/null || fail "statement metadata or evidence status disagrees with bound receipt/lock"

# The subject inventory is a closed set: exactly the lock, run receipt and all artifacts
# declared by that receipt. A valid signature cannot make an omitted artifact disappear.
expected_subjects="$tmpdir/expected-subjects.txt"
actual_subjects="$tmpdir/actual-subjects.txt"
{
  printf '%s\n' "$lock_path" "$run_path"
  jq -r '.artifacts[] | .path' "$run_file"
} | LC_ALL=C sort -u > "$expected_subjects"
jq -r '.subject[].name' "$payload_file" | LC_ALL=C sort -u > "$actual_subjects"
cmp -s "$expected_subjects" "$actual_subjects" || fail "statement subject inventory does not exactly cover lock, run receipt and declared artifacts"

while IFS= "$expected_key_id" "$(jq -r '.subject | length' "$payload_file")" "$(jq -r '.predicate.evidence_status' "$payload_file")"
echo "NON-CLAIM: DSSE signature and subject-byte integrity do not appraise the truth of runtime observations or establish release qualification."
\t' read -r artifact_path expected_digest; do
  [[ -n "$artifact_path" && -n "$expected_digest" ]] || fail "malformed artifact manifest entry"
  subject_digest="$(jq -r --arg p "$artifact_path" '[.subject[] | select(.name == $p) | .digest.sha256] | if length == 1 then .[0] else empty end' "$payload_file")"
  [[ "$subject_digest" == "$expected_digest" ]] || fail "artifact manifest digest disagrees with in-toto subject: $artifact_path"
done < <(jq -r '.artifacts[] | [.path,.sha256] | @tsv' "$run_file")

printf 'DSSE_VERIFIED: key_id=%s subject_count=%s evidence_status=%s\n' "$expected_key_id" "$(jq -r '.subject | length' "$payload_file")" "$(jq -r '.predicate.evidence_status' "$payload_file")"
echo "NON-CLAIM: DSSE signature and subject-byte integrity do not appraise the truth of runtime observations or establish release qualification."
