#!/usr/bin/env bash
set -euo pipefail

schema="benchmarks/srq-v1/sqlite-d1-challenge-consumption-reference.sql"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

command -v sqlite3 >/dev/null 2>&1 || {
  echo "BLOCKED: sqlite3 CLI is required for the D1 challenge-consumption reference." >&2
  exit 2
}
[[ -f "$schema" ]] || { echo "FAIL: D1 reference schema is missing." >&2; exit 1; }

readonly challenge_id="d1-observation-0001"
readonly target_token="target-token-A"
readonly audience="srq-verifier-A"
readonly decision_id="decision-42"
readonly policy_generation="42"
readonly digest_a="$(printf 'a%.0s' {1..64})"
readonly digest_b="$(printf 'b%.0s' {1..64})"
readonly digest_c="$(printf 'c%.0s' {1..64})"
readonly nonce_plain="one-use-d1-observation-nonce-0001"
readonly nonce_sha="$(printf '%s' "$nonce_plain" | sha256sum | awk '{print $1}')"
readonly issued_at="1000"
readonly expires_at="2000"
readonly deployment="/nix/store/0123456789abcdef0123456789abcdef-srq-system"
readonly signer_key_id="01020304"
readonly boot_instance="boot-instance-0001"
readonly verifier_now="1500"

fail() { echo "FAIL: $*" >&2; exit 1; }
digest_text() { printf '%s' "$1" | sha256sum | awk '{print $1}'; }

init_db() {
  local db="$1"
  sqlite3 -batch -bail "$db" < "$schema" >/dev/null
  sqlite3 -batch -bail "$db" <<SQL
PRAGMA foreign_keys=ON;
PRAGMA synchronous=FULL;
INSERT INTO d1_observation_challenges (
  challenge_id,target_token,audience,decision_id,policy_generation,
  approved_artifact_sha256,effective_policy_sha256,corpus_sha256,
  expected_deployment_id,expected_signer_key_id_hex,nonce_sha256,
  issued_at_unix_seconds,expires_at_unix_seconds,required_assurance_tier,
  consumed_receipt_sha256
) VALUES (
  '$challenge_id','$target_token','$audience','$decision_id',$policy_generation,
  '$digest_a','$digest_b','$digest_c',
  '$deployment','$signer_key_id','$nonce_sha',
  $issued_at,$expires_at,'D1',NULL
);
SQL
}

# Interface shape matches the fields a durable ChallengeConsumer must bind.
# Values in this test script are fixed fixtures; a production Rust adapter must
# use prepared statements and must independently verify the COSE envelope first.
attempt_consume() {
  local db="$1" now="$2" target="$3" audience_arg="$4" decision="$5"
  local generation="$6" approved="$7" effective="$8" corpus="$9" deployment_arg="${10}"
  local signer_key="${11}" nonce_hash="${12}" issued="${13}" expiry="${14}"
  local envelope="${15}" boot_id="${16}" challenge="${17}"
  local receipt_material receipt_sha

  receipt_material="$(printf '%s\n' \
    "profile=srq-eat-boot-observation-v1" \
    "challenge=$challenge" "target=$target" "audience=$audience_arg" \
    "decision=$decision" "generation=$generation" "approved=$approved" \
    "effective=$effective" "corpus=$corpus" "deployment=$deployment_arg" \
    "signer_key_id=$signer_key" "nonce_sha256=$nonce_hash" \
    "issued=$issued" "expires=$expiry" "verifier_now=$now" \
    "envelope_sha256=$envelope" "boot_instance_id=$boot_id")"
  receipt_sha="$(digest_text "$receipt_material")"

  sqlite3 -batch -bail "$db" <<SQL
.output /dev/null
PRAGMA foreign_keys=ON;
PRAGMA busy_timeout=10000;
PRAGMA synchronous=FULL;
BEGIN IMMEDIATE;
CREATE TEMP TABLE assert_one_change(value INTEGER NOT NULL CHECK (value=1));
INSERT INTO d1_verified_observation_receipts (
  receipt_sha256,challenge_id,profile_id,signer_key_id_hex,target_token,
  audience,decision_id,policy_generation,approved_artifact_sha256,
  effective_policy_sha256,corpus_sha256,deployment_id,boot_instance_id,
  nonce_sha256,challenge_issued_at_unix_seconds,
  challenge_expires_at_unix_seconds,verifier_now_unix_seconds,
  envelope_sha256,committed_at_unix_seconds
) VALUES (
  '$receipt_sha','$challenge','srq-eat-boot-observation-v1','$signer_key','$target',
  '$audience_arg','$decision',$generation,'$approved',
  '$effective','$corpus','$deployment_arg','$boot_id',
  '$nonce_hash',$issued,$expiry,$now,
  '$envelope',$now
);
UPDATE d1_observation_challenges
   SET consumed_receipt_sha256='$receipt_sha'
 WHERE challenge_id='$challenge'
   AND target_token='$target'
   AND audience='$audience_arg'
   AND decision_id='$decision'
   AND policy_generation=$generation
   AND approved_artifact_sha256='$approved'
   AND effective_policy_sha256='$effective'
   AND corpus_sha256='$corpus'
   AND expected_deployment_id='$deployment_arg'
   AND expected_signer_key_id_hex='$signer_key'
   AND nonce_sha256='$nonce_hash'
   AND issued_at_unix_seconds=$issued
   AND expires_at_unix_seconds=$expiry
   AND required_assurance_tier='D1'
   AND consumed_receipt_sha256 IS NULL
   AND $now >= issued_at_unix_seconds
   AND $now < expires_at_unix_seconds;
INSERT INTO assert_one_change SELECT changes();
COMMIT;
.output stdout
SELECT '$receipt_sha';
SQL
}

assert_clean_unconsumed() {
  local db="$1"
  [[ "$(sqlite3 -batch "$db" 'SELECT count(*) FROM d1_verified_observation_receipts;')" == "0" ]] \
    || fail "rejected D1 request left a receipt"
  [[ -z "$(sqlite3 -batch "$db" 'SELECT coalesce(consumed_receipt_sha256, "") FROM d1_observation_challenges;')" ]] \
    || fail "rejected D1 request consumed its challenge"
  [[ "$(sqlite3 -batch "$db" 'PRAGMA integrity_check;')" == "ok" ]] \
    || fail "rejected D1 request left the database inconsistent"
}

expect_rejected() {
  local label="$1" now="$2" target="$3" audience_arg="$4" decision="$5"
  local generation="$6" approved="$7" effective="$8" corpus="$9"
  local deployment_arg="${10}" signer_key="${11}" nonce_hash="${12}" issued="${13}" expiry="${14}"
  local db="$tmp/reject-$label.sqlite" envelope
  init_db "$db"
  envelope="$(digest_text "envelope-$label")"
  if attempt_consume "$db" "$now" "$target" "$audience_arg" "$decision" \
      "$generation" "$approved" "$effective" "$corpus" "$deployment_arg" \
      "$signer_key" "$nonce_hash" "$issued" "$expiry" "$envelope" \
      "$boot_instance" "$challenge_id" >"$tmp/reject-$label.out" 2>&1; then
    fail "invalid D1 request was accepted: $label"
  fi
  assert_clean_unconsumed "$db"
  echo "PASS: D1 challenge rejects $label without mutation"
}

# Case 1: the correct challenge-bound envelope is consumed exactly once and the
# committed receipt and challenge pointer agree after opening a new connection.
success_db="$tmp/success.sqlite"
init_db "$success_db"
success_envelope="$(digest_text 'valid-signed-envelope-fixture')"
receipt_sha="$(attempt_consume "$success_db" "$verifier_now" "$target_token" "$audience" \
  "$decision_id" "$policy_generation" "$digest_a" "$digest_b" "$digest_c" \
  "$deployment" "$signer_key_id" "$nonce_sha" "$issued_at" "$expires_at" \
  "$success_envelope" "$boot_instance" "$challenge_id")"
[[ "$receipt_sha" =~ ^[0-9a-f]{64}$ ]] || fail "successful transaction did not return a receipt digest (captured output: $(printf %q "$receipt_sha"))"
[[ "$(sqlite3 -batch "$success_db" 'SELECT count(*) FROM d1_verified_observation_receipts;')" == "1" ]] || fail "success did not persist one receipt"
[[ "$(sqlite3 -batch "$success_db" 'SELECT consumed_receipt_sha256 FROM d1_observation_challenges;')" == "$receipt_sha" ]] || fail "challenge did not point to the committed receipt"
[[ "$(sqlite3 -batch "$success_db" 'SELECT count(*) FROM d1_verified_observation_receipts r JOIN d1_observation_challenges c ON c.challenge_id=r.challenge_id WHERE r.receipt_sha256=c.consumed_receipt_sha256;')" == "1" ]] || fail "receipt and challenge consumption are inconsistent after reopen"
[[ "$(sqlite3 -batch "$success_db" 'PRAGMA integrity_check;')" == "ok" ]] || fail "successful D1 database failed integrity_check"
echo "PASS: D1 receipt and one-use challenge pointer commit atomically and reconcile after reopen"

# Case 2: exact replay of the same challenge cannot create a second receipt.
if attempt_consume "$success_db" "$verifier_now" "$target_token" "$audience" \
    "$decision_id" "$policy_generation" "$digest_a" "$digest_b" "$digest_c" \
    "$deployment" "$signer_key_id" "$nonce_sha" "$issued_at" "$expires_at" \
    "$success_envelope" "$boot_instance" "$challenge_id" >"$tmp/replay.out" 2>&1; then
  fail "D1 challenge replay was accepted"
fi
[[ "$(sqlite3 -batch "$success_db" 'SELECT count(*) FROM d1_verified_observation_receipts;')" == "1" ]] || fail "replay created a second receipt"
[[ "$(sqlite3 -batch "$success_db" 'SELECT consumed_receipt_sha256 FROM d1_observation_challenges;')" == "$receipt_sha" ]] || fail "replay replaced the original receipt"
echo "PASS: D1 challenge replay after reopen is rejected without replacing the receipt"

# Cases 3-14: independently mutate each binding or freshness attribute.
expect_rejected "wrong-target" "$verifier_now" "target-token-B" "$audience" "$decision_id" "$policy_generation" "$digest_a" "$digest_b" "$digest_c" "$deployment" "$signer_key_id" "$nonce_sha" "$issued_at" "$expires_at"
expect_rejected "wrong-audience" "$verifier_now" "$target_token" "srq-verifier-B" "$decision_id" "$policy_generation" "$digest_a" "$digest_b" "$digest_c" "$deployment" "$signer_key_id" "$nonce_sha" "$issued_at" "$expires_at"
expect_rejected "wrong-decision" "$verifier_now" "$target_token" "$audience" "decision-43" "$policy_generation" "$digest_a" "$digest_b" "$digest_c" "$deployment" "$signer_key_id" "$nonce_sha" "$issued_at" "$expires_at"
expect_rejected "wrong-generation" "$verifier_now" "$target_token" "$audience" "$decision_id" "43" "$digest_a" "$digest_b" "$digest_c" "$deployment" "$signer_key_id" "$nonce_sha" "$issued_at" "$expires_at"
expect_rejected "wrong-approved-artifact" "$verifier_now" "$target_token" "$audience" "$decision_id" "$policy_generation" "$digest_c" "$digest_b" "$digest_c" "$deployment" "$signer_key_id" "$nonce_sha" "$issued_at" "$expires_at"
expect_rejected "wrong-effective-policy" "$verifier_now" "$target_token" "$audience" "$decision_id" "$policy_generation" "$digest_a" "$digest_c" "$digest_c" "$deployment" "$signer_key_id" "$nonce_sha" "$issued_at" "$expires_at"
expect_rejected "wrong-corpus" "$verifier_now" "$target_token" "$audience" "$decision_id" "$policy_generation" "$digest_a" "$digest_b" "$digest_a" "$deployment" "$signer_key_id" "$nonce_sha" "$issued_at" "$expires_at"
expect_rejected "wrong-deployment" "$verifier_now" "$target_token" "$audience" "$decision_id" "$policy_generation" "$digest_a" "$digest_b" "$digest_c" "/nix/store/ffffffffffffffffffffffffffffffff-wrong" "$signer_key_id" "$nonce_sha" "$issued_at" "$expires_at"
expect_rejected "wrong-signer-key" "$verifier_now" "$target_token" "$audience" "$decision_id" "$policy_generation" "$digest_a" "$digest_b" "$digest_c" "$deployment" "deadbeef" "$nonce_sha" "$issued_at" "$expires_at"
expect_rejected "wrong-nonce" "$verifier_now" "$target_token" "$audience" "$decision_id" "$policy_generation" "$digest_a" "$digest_b" "$digest_c" "$deployment" "$signer_key_id" "$(digest_text 'different-nonce')" "$issued_at" "$expires_at"
expect_rejected "expired" "$expires_at" "$target_token" "$audience" "$decision_id" "$policy_generation" "$digest_a" "$digest_b" "$digest_c" "$deployment" "$signer_key_id" "$nonce_sha" "$issued_at" "$expires_at"
expect_rejected "challenge-issued-time-mismatch" "$verifier_now" "$target_token" "$audience" "$decision_id" "$policy_generation" "$digest_a" "$digest_b" "$digest_c" "$deployment" "$signer_key_id" "$nonce_sha" "999" "$expires_at"

# Immutable authorized challenge data and append-only receipts cannot be edited
# after a decision has been recorded.
if sqlite3 -batch -bail "$success_db" "UPDATE d1_observation_challenges SET audience='attacker' WHERE challenge_id='$challenge_id';" >/dev/null 2>&1; then
  fail "challenge binding mutation succeeded"
fi
if sqlite3 -batch -bail "$success_db" "UPDATE d1_verified_observation_receipts SET audience='attacker' WHERE receipt_sha256='$receipt_sha';" >/dev/null 2>&1; then
  fail "receipt mutation succeeded"
fi
if sqlite3 -batch -bail "$success_db" "DELETE FROM d1_verified_observation_receipts WHERE receipt_sha256='$receipt_sha';" >/dev/null 2>&1; then
  fail "receipt deletion succeeded"
fi
echo "PASS: consumed challenge bindings and committed D1 receipts are immutable"

# Case 15: 64 independent writers compete to consume the same fresh challenge.
# One transaction may commit; all others must see the consumed challenge and fail.
race_db="$tmp/race.sqlite"
init_db "$race_db"
race_worker() {
  local attempt="$1" envelope result
  envelope="$(digest_text "race-envelope-$attempt")"
  if result="$(attempt_consume "$race_db" "$verifier_now" "$target_token" "$audience" \
      "$decision_id" "$policy_generation" "$digest_a" "$digest_b" "$digest_c" \
      "$deployment" "$signer_key_id" "$nonce_sha" "$issued_at" "$expires_at" \
      "$envelope" "boot-race" "$challenge_id" 2>"$tmp/d1-race-$attempt.err")"; then
    printf 'd1_race_trial=%02d outcome=consumed receipt_sha256=%s\n' "$attempt" "$result"
  else
    printf 'd1_race_trial=%02d outcome=rejected\n' "$attempt"
  fi
}
pids=()
for attempt in $(seq 1 64); do
  race_worker "$attempt" > "$tmp/d1-race-$attempt.out" &
  pids+=("$!")
done
for pid in "${pids[@]}"; do
  wait "$pid" || fail "D1 concurrent worker crashed unexpectedly"
done
cat "$tmp"/d1-race-*.out | LC_ALL=C sort
successes="$(grep -h -c 'outcome=consumed' "$tmp"/d1-race-*.out | awk '{sum += $1} END {print sum + 0}')"
rejections="$(grep -h -c 'outcome=rejected' "$tmp"/d1-race-*.out | awk '{sum += $1} END {print sum + 0}')"
[[ "$successes" == "1" && "$rejections" == "63" ]] || fail "64 D1 consumers produced success=$successes rejected=$rejections"
[[ "$(sqlite3 -batch "$race_db" 'SELECT count(*) FROM d1_verified_observation_receipts;')" == "1" ]] || fail "D1 race committed other than one receipt"
[[ "$(sqlite3 -batch "$race_db" 'SELECT count(*) FROM d1_observation_challenges WHERE consumed_receipt_sha256 IS NOT NULL;')" == "1" ]] || fail "D1 race consumed other than one challenge"
[[ "$(sqlite3 -batch "$race_db" 'SELECT count(*) FROM d1_verified_observation_receipts r JOIN d1_observation_challenges c ON c.challenge_id=r.challenge_id WHERE r.receipt_sha256=c.consumed_receipt_sha256;')" == "1" ]] || fail "D1 race receipt and challenge pointer diverged"
[[ "$(sqlite3 -batch "$race_db" 'PRAGMA integrity_check;')" == "ok" ]] || fail "D1 race failed integrity_check"
echo "PASS: exactly one durable D1 receipt across 64 independent challenge consumers"


# Cases 17-20: terminate an independent SQLite CLI process at four commit
# boundaries, 25 fresh-database repetitions each. SIGKILL is real userspace
# process death on a running kernel; it is not a kernel crash or power-loss test.
command -v setsid >/dev/null 2>&1 || {
  echo "BLOCKED: setsid is required for D1 transaction-boundary tests." >&2
  exit 2
}

run_d1_sigkill_case() {
  local label="$1" phase="$2" db="$3"
  local marker="$tmp/$label.marker" script="$tmp/$label.sql" pid deadline
  local envelope trial_boot_instance receipt_material receipt_sha

  envelope="$(digest_text "d1-sigkill-envelope-$label")"
  trial_boot_instance="boot-sigkill-$label"
  receipt_material="$(printf '%s\n' \
    "profile=srq-eat-boot-observation-v1" \
    "challenge=$challenge_id" "target=$target_token" "audience=$audience" \
    "decision=$decision_id" "generation=$policy_generation" "approved=$digest_a" \
    "effective=$digest_b" "corpus=$digest_c" "deployment=$deployment" \
    "signer_key_id=$signer_key_id" "nonce_sha256=$nonce_sha" \
    "issued=$issued_at" "expires=$expires_at" "verifier_now=$verifier_now" \
    "envelope_sha256=$envelope" "boot_instance_id=$trial_boot_instance")"
  receipt_sha="$(digest_text "$receipt_material")"
  rm -f "$marker"

  {
    cat <<SQL
PRAGMA foreign_keys=ON;
PRAGMA busy_timeout=10000;
PRAGMA synchronous=FULL;
BEGIN IMMEDIATE;
SQL
    if [[ "$phase" == "after_begin" ]]; then
      printf ".shell sh -c 'touch %s; exec sleep 60'\n" "$marker"
    fi

    cat <<SQL
INSERT INTO d1_verified_observation_receipts (
  receipt_sha256,challenge_id,profile_id,signer_key_id_hex,target_token,
  audience,decision_id,policy_generation,approved_artifact_sha256,
  effective_policy_sha256,corpus_sha256,deployment_id,boot_instance_id,
  nonce_sha256,challenge_issued_at_unix_seconds,
  challenge_expires_at_unix_seconds,verifier_now_unix_seconds,
  envelope_sha256,committed_at_unix_seconds
) VALUES (
  '$receipt_sha','$challenge_id','srq-eat-boot-observation-v1','$signer_key_id','$target_token',
  '$audience','$decision_id',$policy_generation,'$digest_a',
  '$digest_b','$digest_c','$deployment','$trial_boot_instance',
  '$nonce_sha',$issued_at,$expires_at,$verifier_now,
  '$envelope',$verifier_now
);
SQL
    if [[ "$phase" == "after_receipt_insert" ]]; then
      printf ".shell sh -c 'touch %s; exec sleep 60'\n" "$marker"
    fi

    cat <<SQL
CREATE TEMP TABLE assert_one_change(value INTEGER NOT NULL CHECK(value=1));
UPDATE d1_observation_challenges
   SET consumed_receipt_sha256='$receipt_sha'
 WHERE challenge_id='$challenge_id'
   AND target_token='$target_token'
   AND audience='$audience'
   AND decision_id='$decision_id'
   AND policy_generation=$policy_generation
   AND approved_artifact_sha256='$digest_a'
   AND effective_policy_sha256='$digest_b'
   AND corpus_sha256='$digest_c'
   AND expected_deployment_id='$deployment'
   AND expected_signer_key_id_hex='$signer_key_id'
   AND nonce_sha256='$nonce_sha'
   AND issued_at_unix_seconds=$issued_at
   AND expires_at_unix_seconds=$expires_at
   AND required_assurance_tier='D1'
   AND consumed_receipt_sha256 IS NULL
   AND $verifier_now >= issued_at_unix_seconds
   AND $verifier_now < expires_at_unix_seconds;
INSERT INTO assert_one_change SELECT changes();
SQL
    if [[ "$phase" == "before_commit" ]]; then
      printf ".shell sh -c 'touch %s; exec sleep 60'\n" "$marker"
    fi
    printf 'COMMIT;\n'
    if [[ "$phase" == "after_commit" ]]; then
      printf ".shell sh -c 'touch %s; exec sleep 60'\n" "$marker"
    fi
  } > "$script"

  setsid sqlite3 -batch -bail "$db" < "$script" > "$tmp/$label.sqlite.log" 2>&1 &
  pid="$!"
  deadline=$((SECONDS + 10))
  while [[ ! -f "$marker" ]]; do
    if ! kill -0 "$pid" 2>/dev/null; then
      cat "$tmp/$label.sqlite.log" >&2
      fail "D1 SQLite process exited before kill checkpoint ($label)"
    fi
    if ((SECONDS >= deadline)); then
      kill -KILL -- "-$pid" 2>/dev/null || true
      fail "D1 SQLite process did not reach kill checkpoint ($label)"
    fi
    sleep 0.01
  done

  kill -KILL -- "-$pid" || fail "failed to SIGKILL D1 SQLite process group ($label)"
  if wait "$pid"; then
    fail "D1 SQLite worker exited successfully rather than being killed ($label)"
  fi

  if [[ "$phase" == "after_commit" ]]; then
    [[ "$(sqlite3 -batch "$db" 'SELECT count(*) FROM d1_verified_observation_receipts;')" == "1" ]] \
      || fail "post-commit kill did not preserve one D1 receipt ($label)"
    [[ "$(sqlite3 -batch "$db" "SELECT consumed_receipt_sha256 FROM d1_observation_challenges WHERE challenge_id='$challenge_id';")" == "$receipt_sha" ]] \
      || fail "post-commit kill lost the consumed-receipt pointer ($label)"
    [[ "$(sqlite3 -batch "$db" "SELECT count(*) FROM d1_verified_observation_receipts r JOIN d1_observation_challenges c ON c.challenge_id=r.challenge_id WHERE r.receipt_sha256=c.consumed_receipt_sha256 AND r.receipt_sha256='$receipt_sha' AND r.boot_instance_id='$trial_boot_instance';")" == "1" ]] \
      || fail "post-commit receipt/challenge records diverged ($label)"
    [[ "$(sqlite3 -batch "$db" 'PRAGMA integrity_check;')" == "ok" ]] \
      || fail "post-commit database integrity check failed ($label)"
    echo "PASS: D1 SIGKILL after COMMIT preserves the one receipt and consumed challenge"
  else
    [[ "$(sqlite3 -batch "$db" 'SELECT count(*) FROM d1_verified_observation_receipts;')" == "0" ]] \
      || fail "pre-commit kill persisted a D1 receipt ($label)"
    [[ -z "$(sqlite3 -batch "$db" "SELECT coalesce(consumed_receipt_sha256,'') FROM d1_observation_challenges WHERE challenge_id='$challenge_id';")" ]] \
      || fail "pre-commit kill left the challenge consumed ($label)"
    [[ "$(sqlite3 -batch "$db" 'PRAGMA integrity_check;')" == "ok" ]] \
      || fail "pre-commit database integrity check failed ($label)"
    echo "PASS: D1 SIGKILL before COMMIT rolls back receipt and challenge consumption"
  fi
}

for phase in after_begin after_receipt_insert before_commit after_commit; do
  for trial in $(seq 1 25); do
    label="d1-$phase-$trial"
    case_db="$tmp/$label.sqlite"
    init_db "$case_db"
    run_d1_sigkill_case "$label" "$phase" "$case_db"
  done
done
echo "PASS: 100 D1 SQLite SIGKILL trials across four transaction boundaries"

sqlite_version="$(sqlite3 --version | awk '{print $1}')"
printf 'sqlite_version=%s journal_mode=%s synchronous=FULL d1_cases=20 contenders=64 process_kill_trials=100 commit_boundaries=4\n' \
  "$sqlite_version" "$(sqlite3 -batch "$success_db" 'PRAGMA journal_mode;')"
