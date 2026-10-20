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

# Publish enough execution identity to aid later signed-bundle correlation.
sqlite_version="$(sqlite3 --version | awk '{print $1}')"
printf 'sqlite_version=%s journal_mode=%s synchronous=FULL cases=3\n' \
  "$sqlite_version" "$(sqlite3 -batch "$success_db" "PRAGMA journal_mode;")"
