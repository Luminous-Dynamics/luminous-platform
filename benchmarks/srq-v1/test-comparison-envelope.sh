#!/usr/bin/env bash
set -euo pipefail
script_dir="$(cd -- "$(dirname -- "$0")" && pwd)"
builder="$script_dir/build-comparison-statement.sh"
signer="$script_dir/create-comparison-envelope.sh"
verifier="$script_dir/verify-comparison-envelope.sh"
tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT
command -v jq >/dev/null 2>&1 || { echo "BLOCKED: jq required." >&2; exit 2; }
command -v openssl >/dev/null 2>&1 || { echo "BLOCKED: openssl required." >&2; exit 2; }
root="$tmpdir/evidence"
mkdir -p "$root"
printf '{"trial":"one","outcome":"old_complete"}\n' > "$root/trial-data.json"
trial_sha="$(sha256sum "$root/trial-data.json" | cut -d ' ' -f 1)"
challenge="$(openssl rand -hex 32)"
jq -n '{fixture:true,baseline_id:"fixture-baseline-v1",srq_source_sha:"1111111111111111111111111111111111111111"}' > "$root/comparison-lock.json"
lock_sha="$(sha256sum "$root/comparison-lock.json" | cut -d ' ' -f 1)"
jq -n --arg lock_sha "$lock_sha" --arg challenge "$challenge" --arg trial_sha "$trial_sha" '
  {schema_version:1,profile_id:"SRQ-MATCHED-COMPARISON-RUN-V1",fixture:true,run_challenge:$challenge,
   comparison_lock:{baseline_id:"fixture-baseline-v1",lock_sha256:$lock_sha},
   source:{expected_srq_source_sha:"1111111111111111111111111111111111111111",actual_srq_checkout_sha:"1111111111111111111111111111111111111111",baseline_source:{}},
   environment:{os_image:"fixture",architecture:"x86_64",kernel:"fixture",filesystem:"fixture",storage_class:"fixture",runner_class:"fixture"},
   pairing:{protocol:"paired_fault_trials_v1",minimum_paired_trials:5,order_policy:"randomized_per_pair",order_seed:42},
   artifacts:{trial_data:{path:"trial-data.json",sha256:$trial_sha}},
   non_claims:["Synthetic contract test only.","No runtime safety claim."]}
' > "$root/comparison-run.json"

statement="$tmpdir/statement.json"
envelope="$tmpdir/envelope.json"
private="$tmpdir/attestor-private.pem"
public="$tmpdir/attestor-public.pem"
wrong_public="$tmpdir/wrong-public.pem"
key_id="fixture-attestor-v1"
bash "$builder" --evidence-root "$root" --comparison-lock-path comparison-lock.json --comparison-run-path comparison-run.json --output "$statement"
jq -e '
  ._type == "https://in-toto.io/Statement/v1" and
  .predicateType == "https://luminousdynamics.io/attestations/srq-matched-comparison/v1" and
  .predicate.evidence_status == "fixture_only" and
  (.subject | length == 3)
' "$statement" >/dev/null
echo "PASS: portable in-toto statement binds run, lock and trial artifact"
openssl genpkey -algorithm Ed25519 -out "$private"
openssl pkey -in "$private" -pubout -out "$public"
openssl genpkey -algorithm Ed25519 -out "$tmpdir/wrong-private.pem"
openssl pkey -in "$tmpdir/wrong-private.pem" -pubout -out "$wrong_public"
bash "$signer" --statement "$statement" --private-key "$private" --key-id "$key_id" --output "$envelope"
bash "$verifier" --public-key "$public" --expected-key-id "$key_id" --evidence-root "$root" "$envelope"
echo "PASS: DSSE envelope verified by externally supplied public key"

# Negative control: the run receipt declares trial-data.json, so a newly signed
# statement that omits that subject must fail even though its remaining hashes verify.
jq '.subject |= map(select(.name != "trial-data.json"))' "$statement" > "$tmpdir/missing-subject.json"
bash "$signer" --statement "$tmpdir/missing-subject.json" --private-key "$private" --key-id "$key_id" --output "$tmpdir/missing-subject-envelope.json"
if bash "$verifier" --public-key "$public" --expected-key-id "$key_id" --evidence-root "$root" "$tmpdir/missing-subject-envelope.json" >"$tmpdir/missing-subject.out" 2>&1; then
  echo "FAIL: signed statement with an omitted declared artifact was accepted" >&2; exit 1
fi
echo "PASS: signed statement with missing artifact subject rejected"

# Negative control: predicate metadata may not substitute a different artifact digest
# than the immutable run receipt carries.
jq '.predicate.artifacts.trial_data.sha256 = ("0" * 64)' "$statement" > "$tmpdir/altered-manifest.json"
bash "$signer" --statement "$tmpdir/altered-manifest.json" --private-key "$private" --key-id "$key_id" --output "$tmpdir/altered-manifest-envelope.json"
if bash "$verifier" --public-key "$public" --expected-key-id "$key_id" --evidence-root "$root" "$tmpdir/altered-manifest-envelope.json" >"$tmpdir/altered-manifest.out" 2>&1; then
  echo "FAIL: signed statement with an altered artifact manifest was accepted" >&2; exit 1
fi
echo "PASS: signed statement with altered artifact manifest rejected"

# Non-fixture appraisal must match a challenge supplied from outside the evidence bundle.
# This prevents the bundle itself from deciding what freshness nonce the verifier expected.
nonfixture_root="$tmpdir/nonfixture-evidence"
mkdir -p "$nonfixture_root"
cp "$root/trial-data.json" "$nonfixture_root/trial-data.json"
jq '.fixture = false' "$root/comparison-lock.json" > "$nonfixture_root/comparison-lock.json"
nonfixture_lock_sha="$(sha256sum "$nonfixture_root/comparison-lock.json" | cut -d ' ' -f 1)"
jq --arg lock_sha "$nonfixture_lock_sha" '.fixture = false | .comparison_lock.lock_sha256 = $lock_sha' "$root/comparison-run.json" > "$nonfixture_root/comparison-run.json"
nonfixture_statement="$tmpdir/nonfixture-statement.json"
nonfixture_envelope="$tmpdir/nonfixture-envelope.json"
bash "$builder" --evidence-root "$nonfixture_root" --comparison-lock-path comparison-lock.json --comparison-run-path comparison-run.json --output "$nonfixture_statement"
bash "$signer" --statement "$nonfixture_statement" --private-key "$private" --key-id "$key_id" --output "$nonfixture_envelope"
if bash "$verifier" --public-key "$public" --expected-key-id "$key_id" --evidence-root "$nonfixture_root" "$nonfixture_envelope" >"$tmpdir/missing-challenge.out" 2>&1; then
  echo "FAIL: non-fixture envelope accepted without verifier-issued challenge" >&2; exit 1
fi
echo "PASS: non-fixture appraisal requires verifier-issued challenge"
expected_challenge="$(jq -r '.run_challenge' "$nonfixture_root/comparison-run.json")"
if [[ "${expected_challenge:0:1}" == "0" ]]; then
  wrong_challenge="1${expected_challenge:1}"
else
  wrong_challenge="0${expected_challenge:1}"
fi
if bash "$verifier" --public-key "$public" --expected-key-id "$key_id" --expected-run-challenge "$wrong_challenge" --evidence-root "$nonfixture_root" "$nonfixture_envelope" >"$tmpdir/wrong-challenge.out" 2>&1; then
  echo "FAIL: non-fixture envelope accepted with wrong verifier-issued challenge" >&2; exit 1
fi
echo "PASS: wrong verifier-issued challenge rejected"
bash "$verifier" --public-key "$public" --expected-key-id "$key_id" --expected-run-challenge "$expected_challenge" --evidence-root "$nonfixture_root" "$nonfixture_envelope"
echo "PASS: exact externally supplied challenge accepted"

# A non-Ed25519 private key must be rejected by the signer rather than producing an envelope
# that downstream Ed25519-only verification will inevitably reject.
openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 -out "$tmpdir/rsa-private.pem" 2>/dev/null
if bash "$signer" --statement "$statement" --private-key "$tmpdir/rsa-private.pem" --key-id "$key_id" --output "$tmpdir/rsa-envelope.json" >"$tmpdir/rsa-signer.out" 2>&1; then
  echo "FAIL: signer accepted an RSA private key" >&2; exit 1
fi
echo "PASS: non-Ed25519 signing key rejected"

if bash "$verifier" --public-key "$wrong_public" --expected-key-id "$key_id" --evidence-root "$root" "$envelope" >"$tmpdir/wrong-key.out" 2>&1; then
  echo "FAIL: envelope accepted under the wrong key" >&2; exit 1
fi
echo "PASS: wrong attestor key rejected"

jq '.signatures[0].sig = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=="' "$envelope" > "$tmpdir/tampered-envelope.json"
if bash "$verifier" --public-key "$public" --expected-key-id "$key_id" --evidence-root "$root" "$tmpdir/tampered-envelope.json" >"$tmpdir/tampered-signature.out" 2>&1; then
  echo "FAIL: tampered DSSE signature was accepted" >&2; exit 1
fi
echo "PASS: tampered DSSE signature rejected"

printf '{"trial":"one","outcome":"new_complete"}\n' > "$root/trial-data.json"
if bash "$verifier" --public-key "$public" --expected-key-id "$key_id" --evidence-root "$root" "$envelope" >"$tmpdir/tampered-subject.out" 2>&1; then
  echo "FAIL: changed evidence subject bytes were accepted" >&2; exit 1
fi
echo "PASS: changed evidence subject bytes rejected"
echo "NON-CLAIM: this test proves the envelope/subject integrity checks for synthetic evidence; it does not prove runtime measurement truth."
