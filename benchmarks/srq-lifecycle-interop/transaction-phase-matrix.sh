#!/usr/bin/env bash
set -euo pipefail
readonly script_dir="$(cd -- "$(dirname -- "$0")" && pwd)"
readonly validator="$script_dir/validate.sh"
readonly fixture_relative="docs/fixtures/system-transaction-v1.json"
if (($# != 5)); then
  echo "Usage: $0 NIXWARD_ROOT MYCELIX_ROOT SOVEREIGN_BOOT_ROOT BOOT_OBSERVATION_JSON OUTPUT_DIR" >&2
  exit 2
fi
nixward_root="$1"; mycelix_root="$2"; sovereign_boot_root="$3"; boot_observation="$4"; output_dir="$5"
readonly repository_root="$(git -C "$script_dir/../.." rev-parse --show-toplevel)"
readonly fixture="$nixward_root/$fixture_relative"
[[ -f "$fixture" ]] || { echo "BLOCKED: missing pinned transaction fixture: $fixture" >&2; exit 2; }
readonly lock_file="$script_dir/upstream-lock-v1.json"
readonly phase_protocol="$script_dir/transaction-phase-state-protocol-v1.json"
jq -e '
  .schema == "srq-transaction-phase-state-protocol-v1"
  and .protocol_id == "nixward-transaction-phase-presence-v1"
  and .version == 1
  and (.record_bits | map(.name) == ["validation","authorization","snapshot","application","verification","outcome"])
  and (.record_bits | map(.bit) == [0,1,2,3,4,5])
  and (.phases | map(.id) == ["planned","validated","authorized","snapshotted","applied","verified","promoted","failed","recovered"])
  and (.phases | length == 9)
  and ([.phases[].id] | length == (unique | length))
  and all(.phases[]; (.required_records | type == "array") and ((.required_records | length) == (.required_records | unique | length)) and (.required_records | all(.[]; . as $record | ["validation","authorization","snapshot","application","verification","outcome"] | index($record) != null)))
  and .oracle.kind == "minimum_record_presence"
  and .oracle.expected_state_count == 576
  and .oracle.expected_accept_count == 175
  and .oracle.expected_reject_count == 401
  and .target_metrics.false_accept_count == 0
  and .target_metrics.false_reject_count == 0
  and .target_metrics.unexpected_error_count == 0
  and .target_metrics.false_release_pass_count == 0
  and .release_invariant.every_accepted_state_release_status == "BLOCKED"
  and .release_invariant.every_accepted_state_claim_ceiling == "none"
  and .release_invariant.release_pass_allowed == false
' "$phase_protocol" >/dev/null || { echo "BLOCKED: transaction phase oracle protocol is invalid or differs from v1." >&2; exit 2; }
phase_protocol_id="$(jq -er '.protocol_id' "$phase_protocol")"
phase_protocol_blob="$(git -C "$repository_root" hash-object -- benchmarks/srq-lifecycle-interop/transaction-phase-state-protocol-v1.json)"
phase_protocol_sha256="$(sha256sum "$phase_protocol" | cut -d ' ' -f 1)"
readonly expected_fixture_commit="$(jq -er '.pins[] | select(.key == "nixward_transaction_fixture") | .commit_sha' "$lock_file")"
readonly expected_fixture_blob="$(jq -er '.pins[] | select(.key == "nixward_transaction_fixture") | .git_blob_sha1' "$lock_file")"
actual_fixture_commit="$(git -C "$nixward_root" rev-parse HEAD)"
actual_fixture_blob="$(git -C "$nixward_root" hash-object -- "$fixture_relative")"
[[ "$actual_fixture_commit" == "$expected_fixture_commit" && "$actual_fixture_blob" == "$expected_fixture_blob" ]] || {
  echo "BLOCKED: transaction phase matrix source fixture does not match the immutable upstream lock." >&2
  printf 'expected commit/blob: %s / %s\nobserved commit/blob: %s / %s\n' "$expected_fixture_commit" "$expected_fixture_blob" "$actual_fixture_commit" "$actual_fixture_blob" >&2
  exit 2
}
git -C "$nixward_root" diff --quiet -- "$fixture_relative" || { echo "BLOCKED: pinned Nixward transaction fixture has local edits." >&2; exit 2; }

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
records="$tmp/phase-matrix-records.jsonl"
: > "$records"
mkdir -p "$output_dir"
report="$output_dir/transaction-phase-matrix-assessment.json"
states=0; expected_accept_count=0; expected_reject_count=0

has_bit() {
  local mask="$1" bit="$2"
  (( (mask >> bit) & 1 ))
}

for phase in $(jq -er '.phases[].id' "$phase_protocol"); do
  for mask in $(seq 0 63); do
    v=0; a=0; s=0; ap=0; ver=0; o=0
    has_bit "$mask" 0 && v=1
    has_bit "$mask" 1 && a=1
    has_bit "$mask" 2 && s=1
    has_bit "$mask" 3 && ap=1
    has_bit "$mask" 4 && ver=1
    has_bit "$mask" 5 && o=1
    candidate="$tmp/candidate-$phase-$mask.json"
    assessment="$tmp/assessment-$phase-$mask.json"
    log="$tmp/validator-$phase-$mask.log"
    jq --arg phase "$phase" --argjson v "$v" --argjson a "$a" --argjson s "$s" --argjson ap "$ap" --argjson ver "$ver" --argjson o "$o" '
      .phase=$phase
      | .validation=(if $v==1 then {"status":"pass","validator_ref":"synthetic-phase-matrix"} else null end)
      | .authorization=(if $a==1 then {"status":"authorized","authorization_ref":"synthetic-phase-matrix"} else null end)
      | .snapshot=(if $s==1 then {"snapshot_digest":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"} else null end)
      | .application=(if $ap==1 then {"status":"applied","application_ref":"synthetic-phase-matrix"} else null end)
      | .verification=(if $ver==1 then {"status":"healthy","verification_ref":"synthetic-phase-matrix"} else null end)
      | .outcome=(if $o==1 then {"status":"terminal","outcome_ref":"synthetic-phase-matrix"} else null end)
    ' "$fixture" > "$candidate"

    required_mask=0
    while IFS= read -r required_record; do
      case "$required_record" in
        validation) bit=0 ;;
        authorization) bit=1 ;;
        snapshot) bit=2 ;;
        application) bit=3 ;;
        verification) bit=4 ;;
        outcome) bit=5 ;;
        *) echo "BLOCKED: unknown required record in protocol: $required_record" >&2; exit 2 ;;
      esac
      required_mask=$((required_mask | (1 << bit)))
    done < <(jq -r --arg phase "$phase" '.phases[] | select(.id == $phase) | .required_records[]?' "$phase_protocol")

    if (( (mask & required_mask) == required_mask )); then
      expected="ACCEPT"
    else
      expected="REJECT"
    fi

    observed="ERROR"; reason=""; release_status="UNKNOWN"; claim_ceiling="UNKNOWN"; status=0
    if bash "$validator" --nixward-root "$nixward_root" --mycelix-root "$mycelix_root" --sovereign-boot-root "$sovereign_boot_root" --boot-observation "$boot_observation" --transaction-fixture "$candidate" --output "$assessment" >"$log" 2>&1; then
      if jq -e --arg phase "$phase" '
        .schema=="srq-lifecycle-interop-assessment-v1"
        and .contract_status=="PASS"
        and .release_status=="BLOCKED"
        and .claim_ceiling=="none"
        and .transaction.phase==$phase
        and (.release_blockers|length>0)
      ' "$assessment" >/dev/null; then
        observed="ACCEPT"
        release_status="$(jq -er '.release_status' "$assessment")"
        claim_ceiling="$(jq -er '.claim_ceiling' "$assessment")"
      else
        observed="ERROR"; reason="accepted process did not emit required blocked assessment"
      fi
    else
      status=$?
      reason="$(grep -F "CONTRACT VIOLATION:" "$log" | tail -n 1 || true)"
      if [[ -n "$reason" ]]; then
        observed="REJECT"
      else
        observed="ERROR"
        if [[ -z "$reason" ]]; then reason="validator failed without expected contract-violation diagnostic"; fi
      fi
    fi

    candidate_sha256="$(sha256sum "$candidate" | cut -d ' ' -f 1)"
    presence="$(jq -cn --argjson v "$v" --argjson a "$a" --argjson s "$s" --argjson ap "$ap" --argjson ver "$ver" --argjson o "$o" '{validation:($v==1),authorization:($a==1),snapshot:($s==1),application:($ap==1),verification:($ver==1),outcome:($o==1)}')"
    jq -cn --arg id "$phase-mask-$mask" --arg phase "$phase" --arg expected "$expected" --arg observed "$observed" --argjson mask "$mask" --argjson presence "$presence" --arg candidate_sha256 "$candidate_sha256" --arg reason "$reason" --arg release_status "$release_status" --arg claim_ceiling "$claim_ceiling" --argjson validator_exit_status "$status" '{
      id:$id,phase:$phase,record_mask:$mask,record_presence:$presence,expected_outcome:$expected,observed_outcome:$observed,
      validator_exit_status:$validator_exit_status,rejection_reason:(if $reason=="" then null else $reason end),
      release_status:$release_status,claim_ceiling:$claim_ceiling,candidate_raw_sha256:$candidate_sha256
    }' >> "$records"
    states=$((states+1))
    if [[ "$expected" == "ACCEPT" ]]; then expected_accept_count=$((expected_accept_count+1)); else expected_reject_count=$((expected_reject_count+1)); fi
    if [[ "$expected" != "$observed" || "$observed" == "ERROR" ]]; then
      printf 'FAIL: phase=%s mask=%s expected=%s observed=%s\n' "$phase" "$mask" "$expected" "$observed" >&2
    fi
  done
done

fixture_commit="$actual_fixture_commit"
fixture_blob="$actual_fixture_blob"
fixture_sha256="$(sha256sum "$fixture" | cut -d ' ' -f 1)"
script_blob="$(git -C "$repository_root" hash-object -- benchmarks/srq-lifecycle-interop/transaction-phase-matrix.sh)"
script_sha256="$(sha256sum "$script_dir/transaction-phase-matrix.sh" | cut -d ' ' -f 1)"
harness_commit="$(git -C "$repository_root" rev-parse HEAD)"

jq -s --slurpfile protocol "$phase_protocol" --arg protocol_id "$phase_protocol_id" --arg protocol_blob "$phase_protocol_blob" --arg protocol_sha256 "$phase_protocol_sha256" --arg harness_commit "$harness_commit" --arg script_blob "$script_blob" --arg script_sha256 "$script_sha256" --arg fixture_commit "$fixture_commit" --arg fixture_blob "$fixture_blob" --arg fixture_sha256 "$fixture_sha256" --argjson accept_count "$expected_accept_count" --argjson reject_count "$expected_reject_count" --argjson state_count "$states" '
  . as $cases
  | {
      schema:"srq-transaction-phase-state-space-assessment-v1",
      release_status:"BLOCKED",
      claim_ceiling:"none",
      harness:{commit_sha:$harness_commit,script_git_blob_sha1:$script_blob,script_raw_sha256:$script_sha256},
      source_fixture:{repository:"Luminous-Dynamics/nixward",commit_sha:$fixture_commit,git_blob_sha1:$fixture_blob,raw_sha256:$fixture_sha256},
      protocol:{protocol_id:$protocol_id,version:$protocol[0].version,git_blob_sha1:$protocol_blob,raw_sha256:$protocol_sha256},
      target_metrics:$protocol[0].target_metrics,
      expected_state_count:$state_count,observed_state_count:($cases|length),
      expected_accept_count:$accept_count,expected_reject_count:$reject_count,
      false_accept_count:([$cases[]|select(.expected_outcome=="REJECT" and .observed_outcome=="ACCEPT")]|length),
      false_reject_count:([$cases[]|select(.expected_outcome=="ACCEPT" and .observed_outcome=="REJECT")]|length),
      unexpected_error_count:([$cases[]|select(.observed_outcome=="ERROR")]|length),
      false_release_pass_count:([$cases[]|select(.observed_outcome=="ACCEPT" and (.release_status!="BLOCKED" or .claim_ceiling!="none"))]|length),
      accepted_with_blocked_release_count:([$cases[]|select(.observed_outcome=="ACCEPT" and .release_status=="BLOCKED" and .claim_ceiling=="none")]|length),
      by_phase:([$cases[]|.phase]|unique|map(. as $p|{phase:$p,observed:([$cases[]|select(.phase==$p)]|length),accepted:([$cases[]|select(.phase==$p and .observed_outcome=="ACCEPT")]|length),rejected:([$cases[]|select(.phase==$p and .observed_outcome=="REJECT")]|length)})),
      cases:$cases,
      limitations:[
        "Exhaustive only over the six declared companion-record presence bits and nine known Nixward v1 phases.",
        "Companion records are synthetic sentinels; this matrix does not validate nested record semantics, signatures, authorization cryptography, recovery bindings or actual host mutation.",
        "The oracle encodes minimum record-presence prerequisites and deliberately does not reject extra records on earlier phases.",
        "This is a structural state-space conformance test, not release qualification."
      ]
    }
  | .status = (if .expected_state_count==$protocol[0].oracle.expected_state_count and .expected_accept_count==$protocol[0].oracle.expected_accept_count and .expected_reject_count==$protocol[0].oracle.expected_reject_count and .false_accept_count==$protocol[0].target_metrics.false_accept_count and .false_reject_count==$protocol[0].target_metrics.false_reject_count and .unexpected_error_count==$protocol[0].target_metrics.unexpected_error_count and .false_release_pass_count==$protocol[0].target_metrics.false_release_pass_count then "PASS" else "FAIL" end)
' "$records" > "$report"

jq -e --slurpfile protocol "$phase_protocol" --arg protocol_id "$phase_protocol_id" --arg protocol_blob "$phase_protocol_blob" --arg protocol_sha256 "$phase_protocol_sha256" --argjson expected "$states" --argjson accepted "$expected_accept_count" --argjson rejected "$expected_reject_count" '
  .schema=="srq-transaction-phase-state-space-assessment-v1"
  and .status=="PASS" and .release_status=="BLOCKED" and .claim_ceiling=="none"
  and .protocol.protocol_id==$protocol_id and .protocol.version==$protocol[0].version
  and .protocol.git_blob_sha1==$protocol_blob and .protocol.raw_sha256==$protocol_sha256
  and .expected_state_count==$expected and .observed_state_count==$expected
  and .expected_accept_count==$accepted and .expected_reject_count==$rejected
  and $protocol[0].oracle.expected_state_count==$expected
  and $protocol[0].oracle.expected_accept_count==$accepted
  and $protocol[0].oracle.expected_reject_count==$rejected
  and .false_accept_count==$protocol[0].target_metrics.false_accept_count
  and .false_reject_count==$protocol[0].target_metrics.false_reject_count
  and .unexpected_error_count==$protocol[0].target_metrics.unexpected_error_count
  and .false_release_pass_count==$protocol[0].target_metrics.false_release_pass_count
  and .accepted_with_blocked_release_count==$accepted
  and ([.cases[].id]|length==(unique|length))
' "$report" >/dev/null || { echo "FAIL: transaction phase state-space conformance failed; report: $report" >&2; exit 1; }

printf 'PASS: exhaustive transaction phase matrix states=%s accepted=%s rejected=%s; false-accept=0 false-reject=0 false-release-pass=0\n' "$states" "$expected_accept_count" "$expected_reject_count"
