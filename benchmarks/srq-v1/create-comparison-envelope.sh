#!/usr/bin/env bash
set -euo pipefail
statement=""
private_key=""
key_id=""
output=""
usage() { echo "Usage: $0 --statement FILE --private-key KEY.pem --key-id ID --output ENVELOPE.json" >&2; }
fail() { echo "FAIL: $*" >&2; exit 1; }
while (($#)); do
  case "$1" in
    --statement) [[ $# -ge 2 ]] || { usage; exit 2; }; statement="$2"; shift 2 ;;
    --private-key) [[ $# -ge 2 ]] || { usage; exit 2; }; private_key="$2"; shift 2 ;;
    --key-id) [[ $# -ge 2 ]] || { usage; exit 2; }; key_id="$2"; shift 2 ;;
    --output) [[ $# -ge 2 ]] || { usage; exit 2; }; output="$2"; shift 2 ;;
    *) usage; exit 2 ;;
  esac
done
[[ -f "$statement" && -f "$private_key" && -n "$key_id" && -n "$output" ]] || { usage; exit 2; }
[[ "$key_id" =~ ^[A-Za-z0-9][A-Za-z0-9._:-]{0,120}$ ]] || fail "invalid key ID"
command -v jq >/dev/null 2>&1 || { echo "BLOCKED: jq is required." >&2; exit 2; }
command -v openssl >/dev/null 2>&1 || { echo "BLOCKED: openssl is required." >&2; exit 2; }
command -v base64 >/dev/null 2>&1 || { echo "BLOCKED: base64 is required." >&2; exit 2; }
jq -e '
  ._type == "https://in-toto.io/Statement/v1" and
  .predicateType == "https://luminousdynamics.io/attestations/srq-matched-comparison/v1" and
  .predicate.profile_id == "SRQ-MATCHED-COMPARISON-ATTESTATION-V1"
' "$statement" >/dev/null || fail "statement type/profile is invalid"

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT
payload_file="$tmpdir/payload.json"
pae_file="$tmpdir/pae.bin"
sig_file="$tmpdir/signature.bin"
jq -cS '.' "$statement" > "$payload_file"
payload_type="application/vnd.in-toto+json"
type_len="$(printf '%s' "$payload_type" | wc -c | tr -d ' ')"
payload_len="$(wc -c < "$payload_file" | tr -d ' ')"
{ printf 'DSSEv1 %s %s %s ' "$type_len" "$payload_type" "$payload_len"; cat "$payload_file"; } > "$pae_file"
openssl pkeyutl -sign -inkey "$private_key" -rawin -in "$pae_file" -out "$sig_file" || fail "could not sign DSSE PAE"
sig_b64="$(base64 -w0 < "$sig_file")"
payload_b64="$(base64 -w0 < "$payload_file")"
jq -cnS --arg type "$payload_type" --arg payload "$payload_b64" --arg kid "$key_id" --arg sig "$sig_b64" \
  '{payloadType:$type,payload:$payload,signatures:[{keyid:$kid,sig:$sig}]}' > "$output"
printf 'DSSE_ENVELOPE_SIGNED: key_id=%s payload_type=%s\n' "$key_id" "$payload_type"
echo "NON-CLAIM: signature authenticates the statement bytes under this key; key trust and truth of runtime claims require separate appraisal."
