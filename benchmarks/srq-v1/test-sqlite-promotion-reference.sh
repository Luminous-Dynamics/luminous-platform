#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "$0")" && pwd)"
schema="$script_dir/sqlite-promotion-reference.sql"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

command -v sqlite3 >/dev/null 2>&1 || {
  echo "BLOCKED: sqlite3 CLI is required for the SQLite reference experiment." >&2
  exit 2
}
[[ -f "$schema" ]] || { echo "FAIL: reference schema is missing." >&2; exit 1; }

readonly a="$(printf 'a%.0s' {1..64})"
readonly b="$(printf 'b%.0s' {1..64})"
readonly c="$(printf 'c%.0s' {1..64})"
readonly d="$(printf 'd%.0s' {1..64})"
readonly verifier="reference-verifier-v1"

fail() { echo "FAIL: $*" >&2; exit 1; }
init_db() {
  local db="$1"
  sqlite3 -batch "$db" < "$schema" > "$tmp/schema.log"
  sqlite3 -batch -bail "$db" <<SQL
PRAGMA foreign_keys=ON;
PRAGMA synchronous=FULL;
BEGIN IMMEDIATE;
INSERT INTO target_policy_state
  (target_id,generation,approved_policy_digest,effective_policy_digest,corpus_digest,last_decision_id)
VALUES ('target-1',0,'$a','$a','$d',NULL);
COMMIT;
SQL
}

# Case A: one successful transaction updates the policy, decision ledger, and
# challenge-consumption row atomically. A transaction-local assert enforces
# exactly one compare-and-swap update.
success_db="$tmp/success.sqlite"
init_db "$success_db"
sqlite3 -batch -bail "$success_db" <<SQL > "$tmp/success.log"
PRAGMA foreign_keys=ON;
PRAGMA synchronous=FULL;
BEGIN IMMEDIATE;
CREATE TEMP TABLE assert_one_change(value INTEGER NOT NULL CHECK (value=1));
INSERT INTO promotion_decisions
  (decision_id,target_id,challenge_id,previous_generation,next_generation,
   previous_effective_policy_digest,approved_policy_digest,effective_policy_digest,
   corpus_digest,verifier_id)
VALUES ('decision-1','target-1','challenge-1',0,1,'$a','$b','$c','$d','$verifier');
INSERT INTO challenge_consumptions
  (target_id,challenge_id,decision_id,consumed_generation)
VALUES ('target-1','challenge-1','decision-1',1);
UPDATE target_policy_state
   SET generation=1, approved_policy_digest='$b', effective_policy_digest='$c',
       corpus_digest='$d', last_decision_id='decision-1'
 WHERE target_id='target-1' AND generation=0 AND effective_policy_digest='$a';
INSERT INTO assert_one_change SELECT changes();
COMMIT;
SQL
[[ "$(sqlite3 -batch "$success_db" "SELECT generation || ':' || last_decision_id FROM target_policy_state WHERE target_id='target-1';")" == "1:decision-1" ]] || fail "promotion state was not committed"
[[ "$(sqlite3 -batch "$success_db" "SELECT count(*) FROM promotion_decisions WHERE decision_id='decision-1';")" == "1" ]] || fail "decision ledger row missing"
[[ "$(sqlite3 -batch "$success_db" "SELECT count(*) FROM challenge_consumptions WHERE target_id='target-1' AND challenge_id='challenge-1';")" == "1" ]] || fail "challenge consumption row missing"
[[ "$(sqlite3 -batch "$success_db" "PRAGMA synchronous;")" == "2" ]] || fail "FULL synchronous setting not active on verifying connection"
echo "PASS: successful SQLite reference transaction commits policy, decision and challenge together"

# Case B: force the compare-and-swap to match zero rows after inserting the
# ledger rows. A failed exact-one-row assertion must abort and roll back all
# prior writes in the same transaction.
stale_db="$tmp/stale.sqlite"
init_db "$stale_db"
if sqlite3 -batch -bail "$stale_db" <<SQL > "$tmp/stale.log" 2>&1
PRAGMA foreign_keys=ON;
PRAGMA synchronous=FULL;
BEGIN IMMEDIATE;
CREATE TEMP TABLE assert_one_change(value INTEGER NOT NULL CHECK (value=1));
INSERT INTO promotion_decisions
  (decision_id,target_id,challenge_id,previous_generation,next_generation,
   previous_effective_policy_digest,approved_policy_digest,effective_policy_digest,
   corpus_digest,verifier_id)
VALUES ('decision-stale','target-1','challenge-stale',7,8,'$a','$b','$c','$d','$verifier');
INSERT INTO challenge_consumptions
  (target_id,challenge_id,decision_id,consumed_generation)
VALUES ('target-1','challenge-stale','decision-stale',8);
UPDATE target_policy_state
   SET generation=8, approved_policy_digest='$b', effective_policy_digest='$c',
       corpus_digest='$d', last_decision_id='decision-stale'
 WHERE target_id='target-1' AND generation=7 AND effective_policy_digest='$a';
INSERT INTO assert_one_change SELECT changes();
COMMIT;
SQL
then
  fail "stale-generation transaction unexpectedly succeeded"
fi
[[ "$(sqlite3 -batch "$stale_db" "SELECT generation FROM target_policy_state WHERE target_id='target-1';")" == "0" ]] || fail "stale transaction mutated policy state"
[[ "$(sqlite3 -batch "$stale_db" "SELECT count(*) FROM promotion_decisions;")" == "0" ]] || fail "stale transaction left a decision row"
[[ "$(sqlite3 -batch "$stale_db" "SELECT count(*) FROM challenge_consumptions;")" == "0" ]] || fail "stale transaction consumed its challenge"
echo "PASS: stale-generation assertion rolls back policy, decision and challenge writes"

# Case C: replaying a committed challenge violates uniqueness and must not
# mutate either the state row or its decision/consumption records.
if sqlite3 -batch -bail "$success_db" <<SQL > "$tmp/replay.log" 2>&1
PRAGMA foreign_keys=ON;
PRAGMA synchronous=FULL;
BEGIN IMMEDIATE;
INSERT INTO promotion_decisions
  (decision_id,target_id,challenge_id,previous_generation,next_generation,
   previous_effective_policy_digest,approved_policy_digest,effective_policy_digest,
   corpus_digest,verifier_id)
VALUES ('decision-replay','target-1','challenge-1',1,2,'$c','$b','$c','$d','$verifier');
INSERT INTO challenge_consumptions
  (target_id,challenge_id,decision_id,consumed_generation)
VALUES ('target-1','challenge-1','decision-replay',2);
COMMIT;
SQL
then
  fail "replay of committed challenge unexpectedly succeeded"
fi
[[ "$(sqlite3 -batch "$success_db" "SELECT generation || ':' || last_decision_id FROM target_policy_state WHERE target_id='target-1';")" == "1:decision-1" ]] || fail "replay mutated state"
[[ "$(sqlite3 -batch "$success_db" "SELECT count(*) FROM promotion_decisions;")" == "1" ]] || fail "replay left a second decision"
[[ "$(sqlite3 -batch "$success_db" "SELECT count(*) FROM challenge_consumptions;")" == "1" ]] || fail "replay left a second consumption"
echo "PASS: duplicate challenge replay is rejected with no partial state"

# Case D: 64 independent writer processes compete for the same target/challenge.
# SQLite must serialize the write transaction; the unique challenge constraint and
# generation CAS together permit exactly one durable promotion.
race_db="$tmp/race.sqlite"
init_db "$race_db"
race_attempt() {
  local db="$1" attempt="$2" decision_id="decision-race-$2"
  local started_ns finished_ns elapsed_ns outcome
  started_ns="$(date +%s%N)"
  if sqlite3 -batch -bail "$db" <<SQL
PRAGMA foreign_keys=ON;
PRAGMA busy_timeout=10000;
PRAGMA synchronous=FULL;
BEGIN IMMEDIATE;
CREATE TEMP TABLE assert_one_change(value INTEGER NOT NULL CHECK (value=1));
INSERT INTO promotion_decisions
  (decision_id,target_id,challenge_id,previous_generation,next_generation,
   previous_effective_policy_digest,approved_policy_digest,effective_policy_digest,
   corpus_digest,verifier_id)
VALUES ('$decision_id','target-1','challenge-race',0,1,'$a','$b','$c','$d','$verifier');
INSERT INTO challenge_consumptions
  (target_id,challenge_id,decision_id,consumed_generation)
VALUES ('target-1','challenge-race','$decision_id',1);
UPDATE target_policy_state
   SET generation=1, approved_policy_digest='$b', effective_policy_digest='$c',
       corpus_digest='$d', last_decision_id='$decision_id'
 WHERE target_id='target-1' AND generation=0 AND effective_policy_digest='$a';
INSERT INTO assert_one_change SELECT changes();
COMMIT;
SQL
  then
    outcome="committed"
  else
    outcome="rejected"
  fi
  finished_ns="$(date +%s%N)"
  elapsed_ns=$((finished_ns - started_ns))
  printf 'race_result %s %s\n' "$outcome" "$elapsed_ns"
  [[ "$outcome" == "committed" ]]
}
pids=()
for attempt in $(seq 1 64); do
  race_attempt "$race_db" "$attempt" > "$tmp/race-$attempt.log" 2>&1 &
  pids+=("$!")
done
race_successes=0
race_rejections=0
for pid in "${pids[@]}"; do
  if wait "$pid"; then
    race_successes=$((race_successes + 1))
  else
    race_rejections=$((race_rejections + 1))
  fi
done
[[ "$race_successes" -eq 1 ]] || fail "64 independent processes committed $race_successes promotions; expected exactly one"
[[ "$race_rejections" -eq 63 ]] || fail "expected 63 rejected contenders, observed $race_rejections"
[[ "$(sqlite3 -batch "$race_db" "SELECT generation FROM target_policy_state WHERE target_id='target-1';")" == "1" ]] || fail "race left an unexpected target generation"
[[ "$(sqlite3 -batch "$race_db" "SELECT count(*) FROM promotion_decisions;")" == "1" ]] || fail "race committed more than one decision"
[[ "$(sqlite3 -batch "$race_db" "SELECT count(*) FROM challenge_consumptions WHERE target_id='target-1' AND challenge_id='challenge-race';")" == "1" ]] || fail "race consumed challenge more than once"
[[ "$(sqlite3 -batch "$race_db" "PRAGMA integrity_check;")" == "ok" ]] || fail "SQLite integrity_check failed after race"
echo "PASS: exactly one durable promotion across 64 independent writer processes"

# Preserve a simple latency distribution for this runner class. These numbers
# are exploratory, not a baseline comparison; use matched hardware and repeated
# runs before publishing performance claims.
all_latency="$tmp/race-all-latency-ns.txt"
rejected_latency="$tmp/race-rejected-latency-ns.txt"
committed_latency="$tmp/race-committed-latency-ns.txt"
: > "$all_latency"
: > "$rejected_latency"
: > "$committed_latency"
for attempt in $(seq 1 64); do
  log="$tmp/race-$attempt.log"
  awk '$1 == "race_result" { print $3 }' "$log" >> "$all_latency"
  awk '$1 == "race_result" && $2 == "rejected" { print $3 }' "$log" >> "$rejected_latency"
  awk '$1 == "race_result" && $2 == "committed" { print $3 }' "$log" >> "$committed_latency"
done
sort -n -o "$all_latency" "$all_latency"
sort -n -o "$rejected_latency" "$rejected_latency"
percentile_ns() {
  local file="$1" percentile="$2" count index
  count="$(wc -l < "$file")"
  ((count > 0)) || { echo "NA"; return 0; }
  index=$(((percentile * count + 99) / 100))
  ((index > 0)) || index=1
  sed -n "${index}p" "$file"
}
all_p50="$(percentile_ns "$all_latency" 50)"
all_p95="$(percentile_ns "$all_latency" 95)"
all_p99="$(percentile_ns "$all_latency" 99)"
reject_p50="$(percentile_ns "$rejected_latency" 50)"
reject_p95="$(percentile_ns "$rejected_latency" 95)"
reject_p99="$(percentile_ns "$rejected_latency" 99)"
winner_ns="$(awk 'NR == 1 { print $1 }' "$committed_latency")"
printf 'race_latency_ns all_p50=%s all_p95=%s all_p99=%s rejected_p50=%s rejected_p95=%s rejected_p99=%s winner=%s\n' \
  "$all_p50" "$all_p95" "$all_p99" "$reject_p50" "$reject_p95" "$reject_p99" "$winner_ns"

# Emit each raw per-contender outcome/elapsed time into the preserved log.
# The stderr/stdout SQL errors are also retained per attempt in the raw CI log.
for attempt in $(seq 1 64); do
  log="$tmp/race-$attempt.log"
  awk -v attempt="$attempt" '$1 == "race_result" { printf "race_trial=%02d outcome=%s latency_ns=%s\n", attempt, $2, $3 }' "$log"
done
printf 'host_os=%s architecture=%s filesystem=%s\n' "$(uname -sr)" "$(uname -m)" "$(stat -f -c %T "$tmp")"

# Case E/F: kill a real SQLite CLI process before COMMIT and after COMMIT
# but before the parent receives a result. The .shell command creates a marker
# only after SQLite has reached the requested point; setsid lets the parent kill
# the whole session/process group, including the marker's blocking shell.
command -v setsid >/dev/null 2>&1 || {
  echo "BLOCKED: setsid is required for the SQLite process-kill reference." >&2
  exit 2
}

run_sqlite_sigkill_case() {
  local label="$1" db="$2" phase="$3"
  local marker="$tmp/$label.marker" script="$tmp/$label.sql" pid deadline
  local decision_id="decision-$label" challenge_id="challenge-$label"
  rm -f "$marker"

  {
    printf 'PRAGMA foreign_keys=ON;\n'
    printf 'PRAGMA busy_timeout=10000;\n'
    printf 'PRAGMA synchronous=FULL;\n'
    printf 'BEGIN IMMEDIATE;\n'
    printf 'INSERT INTO promotion_decisions (decision_id,target_id,challenge_id,previous_generation,next_generation,previous_effective_policy_digest,approved_policy_digest,effective_policy_digest,corpus_digest,verifier_id) VALUES (''%s'',''target-1'',''%s'',0,1,''%s'',''%s'',''%s'',''%s'',''%s'');\n' "$decision_id" "$challenge_id" "$a" "$b" "$c" "$d" "$verifier"
    printf 'INSERT INTO challenge_consumptions (target_id,challenge_id,decision_id,consumed_generation) VALUES (''target-1'',''%s'',''%s'',1);\n' "$challenge_id" "$decision_id"
    printf 'UPDATE target_policy_state SET generation=1,approved_policy_digest=''%s'',effective_policy_digest=''%s'',corpus_digest=''%s'',last_decision_id=''%s'' WHERE target_id=''target-1'' AND generation=0 AND effective_policy_digest=''%s'';\n' "$b" "$c" "$d" "$decision_id" "$a"
    if [[ "$phase" == "before_commit" ]]; then
      printf ".shell sh -c 'touch %s; exec sleep 60'\n" "$marker"
      printf 'COMMIT;\n'
    else
      printf 'COMMIT;\n'
      printf ".shell sh -c 'touch %s; exec sleep 60'\n" "$marker"
    fi
  } > "$script"

  setsid sqlite3 -batch -bail "$db" < "$script" > "$tmp/$label.sqlite.log" 2>&1 &
  pid="$!"
  deadline=$((SECONDS + 10))
  while [[ ! -f "$marker" ]]; do
    if ! kill -0 "$pid" 2>/dev/null; then
      cat "$tmp/$label.sqlite.log" >&2
      fail "SQLite worker exited before kill checkpoint ($label)"
    fi
    if ((SECONDS >= deadline)); then
      kill -KILL -- "-$pid" 2>/dev/null || true
      fail "SQLite worker did not reach kill checkpoint ($label)"
    fi
    sleep 0.01
  done

  kill -KILL -- "-$pid" || fail "failed to SIGKILL SQLite process group ($label)"
  if wait "$pid"; then
    fail "SQLite worker unexpectedly exited successfully rather than being killed ($label)"
  fi

  if [[ "$phase" == "before_commit" ]]; then
    [[ "$(sqlite3 -batch "$db" "SELECT generation FROM target_policy_state WHERE target_id='target-1';")" == "0" ]] || fail "pre-commit kill changed generation"
    [[ "$(sqlite3 -batch "$db" "SELECT count(*) FROM promotion_decisions WHERE decision_id='$decision_id';")" == "0" ]] || fail "pre-commit kill persisted decision"
    [[ "$(sqlite3 -batch "$db" "SELECT count(*) FROM challenge_consumptions WHERE challenge_id='$challenge_id';")" == "0" ]] || fail "pre-commit kill consumed challenge"
    [[ "$(sqlite3 -batch "$db" "PRAGMA integrity_check;")" == "ok" ]] || fail "pre-commit kill left SQLite inconsistent"
    echo "PASS: SIGKILL before SQLite COMMIT rolls back decision, challenge and target state"
  else
    [[ "$(sqlite3 -batch "$db" "SELECT generation || ':' || last_decision_id FROM target_policy_state WHERE target_id='target-1';")" == "1:$decision_id" ]] || fail "post-commit kill lost committed state"
    [[ "$(sqlite3 -batch "$db" "SELECT count(*) FROM promotion_decisions WHERE decision_id='$decision_id';")" == "1" ]] || fail "post-commit kill did not preserve exactly one decision"
    [[ "$(sqlite3 -batch "$db" "SELECT count(*) FROM challenge_consumptions WHERE challenge_id='$challenge_id' AND decision_id='$decision_id';")" == "1" ]] || fail "post-commit kill did not preserve one challenge consumption"
    [[ "$(sqlite3 -batch "$db" "SELECT decision_id FROM promotion_decisions WHERE decision_id='$decision_id';")" == "$decision_id" ]] || fail "committed outcome cannot be reconciled by decision ID"
    [[ "$(sqlite3 -batch "$db" "PRAGMA integrity_check;")" == "ok" ]] || fail "post-commit kill left SQLite inconsistent"
    echo "PASS: SIGKILL after SQLite COMMIT preserves one discoverable outcome by decision ID"
  fi
}

precommit_db="$tmp/precommit.sqlite"
init_db "$precommit_db"
run_sqlite_sigkill_case "sqlite-precommit-kill" "$precommit_db" "before_commit"

postcommit_db="$tmp/postcommit.sqlite"
init_db "$postcommit_db"
run_sqlite_sigkill_case "sqlite-postcommit-kill" "$postcommit_db" "after_commit"

# Publish enough execution identity to aid later signed-bundle correlation.
sqlite_version="$(sqlite3 --version | awk '{print $1}')"
printf 'sqlite_version=%s journal_mode=%s synchronous=FULL cases=4 process_contenders=64\n' \
  "$sqlite_version" "$(sqlite3 -batch "$success_db" "PRAGMA journal_mode;")"
