#!/usr/bin/env bash
set -euo pipefail

: "\${PGHOST:=127.0.0.1}"
: "\${PGPORT:=5432}"
: "\${PGUSER:=postgres}"
: "\${PGPASSWORD:?PGPASSWORD is required}"
: "\${PGDATABASE:=opsbench_test}"

psqlq() {
  psql -X -v ON_ERROR_STOP=1 -q -h "$PGHOST" -p "$PGPORT" -U "$PGUSER" -d "$PGDATABASE" "$@"
}

until pg_isready -h "$PGHOST" -p "$PGPORT" -U "$PGUSER" -d "$PGDATABASE" >/dev/null 2>&1; do
  sleep 1
done

ROOT="$(cd "$(dirname "\${BASH_SOURCE[0]}")/../../.." && pwd)"
MIGRATION="$ROOT/research/continuous-opsbench-v0.1/postgres/001_participant_topology_effect_ledger.sql"
psqlq -f "$MIGRATION"

psqlq <<'SQL'
CREATE TABLE public.opsbench_business_state (
    business_key TEXT PRIMARY KEY,
    state_value TEXT NOT NULL
);
INSERT INTO public.opsbench_business_state (business_key, state_value)
VALUES ('row-1', 'before');
INSERT INTO opsbench_participant.participant_scope_head
    (participant_id, scope_id, topology_epoch, topology_digest)
VALUES ('participant-1', 'scope-1', 7, decode(repeat('01', 32), 'hex'));
SQL

seed_auth() {
  local receipt_id="$1"
  local attempt_id="$2"
  local logical_key="$3"
  local operation="$4"
  local target="$5"
  local request_hex="$6"
  local topology_epoch="$7"
  local topology_hex="$8"

  psqlq -c "
    INSERT INTO opsbench_participant.verified_authorization_receipt (
      receipt_id, receipt_sha256, decision, participant_id, scope_id,
      tenant_id, attempt_id, logical_action_key, operation, target,
      request_sha256, expected_topology_epoch, expected_topology_sha256,
      policy_epoch, policy_sha256, not_before, expires_at,
      authenticated_publisher_id, authenticated_at
    ) VALUES (
      '$receipt_id', decode(repeat('a1', 32), 'hex'), 'allow',
      'participant-1', 'scope-1', 'tenant-1', '$attempt_id',
      '$logical_key', '$operation', '$target', decode('$request_hex', 'hex'),
      $topology_epoch, decode('$topology_hex', 'hex'), 1,
      decode(repeat('b2', 32), 'hex'),
      clock_timestamp() - interval '1 minute',
      clock_timestamp() + interval '1 hour',
      'integration-test-auth-adapter', clock_timestamp()
    );
  "
}

begin_call() {
  local receipt_id="$1"
  local effect_id="$2"
  local attempt_id="$3"
  local logical_key="$4"
  local operation="$5"
  local target="$6"
  local request_hex="$7"
  local topology_epoch="$8"
  local topology_hex="$9"
  printf "SELECT disposition, effect_id, attempt_id, committed_topology_epoch, participant_sequence FROM opsbench_participant.begin_local_effect('%s', '%s', 'participant-1', 'scope-1', 'tenant-1', '%s', '%s', '%s', '%s', decode('%s', 'hex'), %s, decode('%s', 'hex'));" \
    "$receipt_id" "$effect_id" "$attempt_id" "$logical_key" "$operation" "$target" "$request_hex" "$topology_epoch" "$topology_hex"
}

finish_call() {
  local attempt_id="$1"
  local logical_key="$2"
  printf "SELECT effect_id, committed_topology_epoch, participant_sequence FROM opsbench_participant.finish_local_effect('participant-1', 'scope-1', 'tenant-1', '%s', '%s', decode(repeat('c3', 32), 'hex'));" \
    "$logical_key" "$attempt_id"
}

# 1. Local effect, participant receipt and business-row update commit atomically.
seed_auth auth-initial attempt-initial logical-initial update row-1 \
  "$(printf 'aa%.0s' {1..32})" 7 "$(printf '01%.0s' {1..32})"
{
  echo "BEGIN ISOLATION LEVEL READ COMMITTED;"
  begin_call auth-initial effect-initial attempt-initial logical-initial update row-1 \
    "$(printf 'aa%.0s' {1..32})" 7 "$(printf '01%.0s' {1..32})"
  echo "UPDATE public.opsbench_business_state SET state_value = 'committed-initial' WHERE business_key = 'row-1';"
  finish_call attempt-initial logical-initial
  echo "COMMIT;"
} | psqlq

initial_state="$(psqlq -At -c "SELECT state::text || ':' || committed_topology_epoch || ':' || participant_sequence FROM opsbench_participant.logical_effect WHERE logical_action_key = 'logical-initial';")"
[[ "$initial_state" == "committed:7:1" ]] || { echo "unexpected initial effect state: $initial_state" >&2; exit 1; }
business_state="$(psqlq -At -c "SELECT state_value FROM public.opsbench_business_state WHERE business_key = 'row-1';")"
[[ "$business_state" == "committed-initial" ]] || { echo "business update not committed atomically" >&2; exit 1; }

# 2. A rollback removes both participant intent and local business state.
seed_auth auth-rollback attempt-rollback logical-rollback update row-1 \
  "$(printf 'dd%.0s' {1..32})" 7 "$(printf '01%.0s' {1..32})"
{
  echo "BEGIN ISOLATION LEVEL READ COMMITTED;"
  begin_call auth-rollback effect-rollback attempt-rollback logical-rollback update row-1 \
    "$(printf 'dd%.0s' {1..32})" 7 "$(printf '01%.0s' {1..32})"
  echo "UPDATE public.opsbench_business_state SET state_value = 'must-rollback' WHERE business_key = 'row-1';"
  echo "ROLLBACK;"
} | psqlq
rollback_count="$(psqlq -At -c "SELECT count(*) FROM opsbench_participant.logical_effect WHERE logical_action_key = 'logical-rollback';")"
rollback_state="$(psqlq -At -c "SELECT state_value FROM public.opsbench_business_state WHERE business_key = 'row-1';")"
[[ "$rollback_count" == "0" && "$rollback_state" == "committed-initial" ]] || {
  echo "transaction rollback did not revert business and participant rows" >&2; exit 1;
}

# 3. A local attempt cannot commit if finish_local_effect was omitted.
seed_auth auth-unfinished attempt-unfinished logical-unfinished update row-1 \
  "$(printf 'ee%.0s' {1..32})" 7 "$(printf '01%.0s' {1..32})"
if {
  echo "BEGIN ISOLATION LEVEL READ COMMITTED;"
  begin_call auth-unfinished effect-unfinished attempt-unfinished logical-unfinished update row-1 \
    "$(printf 'ee%.0s' {1..32})" 7 "$(printf '01%.0s' {1..32})"
  echo "COMMIT;"
} | psqlq >/tmp/opsbench-unfinished.out 2>&1; then
  echo "deferred trigger accepted an unfinished local effect" >&2
  exit 1
fi
grep -q "was not finalized" /tmp/opsbench-unfinished.out

# 4. Exact retry after topology advances returns the original receipt and sequence.
psqlq -c "SELECT opsbench_participant.publish_topology('participant-1', 'scope-1', 8, decode(repeat('02', 32), 'hex'));"
seed_auth auth-replay attempt-replay logical-initial update row-1 \
  "$(printf 'aa%.0s' {1..32})" 8 "$(printf '02%.0s' {1..32})"
replay="$(
  begin_call auth-replay effect-initial attempt-replay logical-initial update row-1 \
    "$(printf 'aa%.0s' {1..32})" 8 "$(printf '02%.0s' {1..32})" | psqlq -At
)"
[[ "$replay" == "idempotent_replay|effect-initial|attempt-initial|7|1" ]] || {
  echo "idempotent replay did not return the original receipt: $replay" >&2; exit 1;
}

# 5. Reusing the idempotency key for a different request is a hard conflict.
seed_auth auth-conflict attempt-conflict logical-initial update row-1 \
  "$(printf 'ff%.0s' {1..32})" 8 "$(printf '02%.0s' {1..32})"
if begin_call auth-conflict effect-initial attempt-conflict logical-initial update row-1 \
  "$(printf 'ff%.0s' {1..32})" 8 "$(printf '02%.0s' {1..32})" | psqlq >/tmp/opsbench-conflict.out 2>&1; then
  echo "idempotency key accepted a conflicting request digest" >&2
  exit 1
fi
grep -q "idempotency key reused" /tmp/opsbench-conflict.out

# 6. A new action cannot commit using an obsolete topology epoch/digest.
seed_auth auth-stale attempt-stale logical-stale update row-1 \
  "$(printf 'ab%.0s' {1..32})" 7 "$(printf '01%.0s' {1..32})"
if begin_call auth-stale effect-stale attempt-stale logical-stale update row-1 \
  "$(printf 'ab%.0s' {1..32})" 7 "$(printf '01%.0s' {1..32})" | psqlq >/tmp/opsbench-stale.out 2>&1; then
  echo "stale topology precondition was accepted" >&2
  exit 1
fi
grep -q "stale participant topology precondition" /tmp/opsbench-stale.out

# 7. Independent sessions race a local effect against topology publication.
seed_auth auth-race attempt-race logical-race update row-1 \
  "$(printf 'ac%.0s' {1..32})" 8 "$(printf '02%.0s' {1..32})"
race_sql="$(
  printf "BEGIN ISOLATION LEVEL READ COMMITTED; "
  begin_call auth-race effect-race attempt-race logical-race update row-1 \
    "$(printf 'ac%.0s' {1..32})" 8 "$(printf '02%.0s' {1..32})"
  printf " SELECT pg_sleep(2); UPDATE public.opsbench_business_state SET state_value = 'race-committed' WHERE business_key = 'row-1'; "
  finish_call attempt-race logical-race
  printf " COMMIT;"
)"
psqlq -c "$race_sql" >/tmp/opsbench-race-effect.out 2>&1 &
effect_pid=$!
sleep 0.3
psqlq -c "SELECT opsbench_participant.publish_topology('participant-1', 'scope-1', 9, decode(repeat('03', 32), 'hex'));" >/tmp/opsbench-race-topology.out 2>&1 &
topology_pid=$!
wait "$effect_pid"
wait "$topology_pid"

race_result="$(psqlq -At -c "SELECT committed_topology_epoch || ':' || participant_sequence FROM opsbench_participant.logical_effect WHERE logical_action_key = 'logical-race';")"
head_result="$(psqlq -At -c "SELECT topology_epoch FROM opsbench_participant.participant_scope_head WHERE participant_id = 'participant-1' AND scope_id = 'scope-1';")"
[[ "$race_result" == "8:2" && "$head_result" == "9" ]] || {
  echo "topology/effect race violated a valid serialization order: effect=$race_result head=$head_result" >&2; exit 1;
}

# 8. Two independent sessions race the same logical key; only one effect commits.
seed_auth auth-same-a attempt-same-a logical-same update row-1 \
  "$(printf 'ad%.0s' {1..32})" 9 "$(printf '03%.0s' {1..32})"
seed_auth auth-same-b attempt-same-b logical-same update row-1 \
  "$(printf 'ad%.0s' {1..32})" 9 "$(printf '03%.0s' {1..32})"
same_a_sql="$(
  printf "BEGIN ISOLATION LEVEL READ COMMITTED; "
  begin_call auth-same-a effect-same attempt-same-a logical-same update row-1 \
    "$(printf 'ad%.0s' {1..32})" 9 "$(printf '03%.0s' {1..32})"
  printf " SELECT pg_sleep(2); UPDATE public.opsbench_business_state SET state_value = 'same-key-committed' WHERE business_key = 'row-1'; "
  finish_call attempt-same-a logical-same
  printf " COMMIT;"
)"
psqlq -c "$same_a_sql" >/tmp/opsbench-same-a.out 2>&1 &
same_a_pid=$!
sleep 0.3
(
  begin_call auth-same-b effect-same attempt-same-b logical-same update row-1 \
    "$(printf 'ad%.0s' {1..32})" 9 "$(printf '03%.0s' {1..32})" | psqlq -At
) >/tmp/opsbench-same-b.out 2>&1 &
same_b_pid=$!
wait "$same_a_pid"
wait "$same_b_pid"

same_disposition="$(cat /tmp/opsbench-same-b.out)"
[[ "$same_disposition" == idempotent_replay* ]] || {
  echo "second concurrent attempt was not reconciled as idempotent replay: $same_disposition" >&2; exit 1;
}
same_count="$(psqlq -At -c "SELECT count(*) FROM opsbench_participant.logical_effect WHERE logical_action_key = 'logical-same';")"
same_attempt_count="$(psqlq -At -c "SELECT count(*) FROM opsbench_participant.effect_attempt WHERE logical_action_key = 'logical-same';")"
same_sequence="$(psqlq -At -c "SELECT participant_sequence FROM opsbench_participant.logical_effect WHERE logical_action_key = 'logical-same';")"
[[ "$same_count" == "1" && "$same_attempt_count" == "1" && "$same_sequence" == "3" ]] || {
  echo "concurrent idempotency race created duplicate state: logical=$same_count attempts=$same_attempt_count sequence=$same_sequence" >&2; exit 1;
}

# 9. Equal topology epoch with a different digest is rejected.
psqlq -c "SELECT opsbench_participant.publish_topology('participant-1', 'scope-1', 10, decode(repeat('04', 32), 'hex'));"
seed_auth auth-same-epoch-wrong-digest attempt-same-epoch-wrong-digest logical-same-epoch-wrong-digest update row-1 \
  "$(printf 'ae%.0s' {1..32})" 10 "$(printf '03%.0s' {1..32})"
if begin_call auth-same-epoch-wrong-digest effect-same-epoch-wrong-digest attempt-same-epoch-wrong-digest logical-same-epoch-wrong-digest update row-1 \
  "$(printf 'ae%.0s' {1..32})" 10 "$(printf '03%.0s' {1..32})" | psqlq >/tmp/opsbench-digest-mismatch.out 2>&1; then
  echo "topology digest mismatch was accepted" >&2
  exit 1
fi
grep -q "stale participant topology precondition" /tmp/opsbench-digest-mismatch.out

echo "PostgreSQL participant CAS integration passed:"
echo "  local transaction + participant receipt atomicity"
echo "  rollback, unfinished-commit rejection and idempotent replay"
echo "  conflicting key, stale epoch and topology digest rejection"
echo "  independent-session topology/effect serialization"
echo "  independent-session concurrent duplicate suppression"
