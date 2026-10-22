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
