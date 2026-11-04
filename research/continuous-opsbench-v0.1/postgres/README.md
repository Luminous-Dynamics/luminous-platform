# PostgreSQL participant CAS v0.1 — transaction and recovery contract

Status: reference migration and design only. It has not yet been executed against PostgreSQL in this environment. No durable-adapter or crash-recovery qualification is claimed.

Migration: postgres/001_participant_topology_effect_ledger.sql

## 1. Scope and architecture

This migration turns the in-memory semantics in PR #242 into a database-owned reference boundary for **local PostgreSQL effects**.

The participant owns:
- the authoritative topology head for each participant/scope;
- the stable tenant-scoped logical-action identity;
- per-attempt history;
- the committed participant sequence and receipt digest;
- exact authorization-receipt binding references.

Platform Saga remains the coordinator. The existing effect broker remains the authorization boundary. Nixward remains the host transaction/recovery authority.

The trusted source-authentication adapter must verify authorization signatures/trust roots and insert normalized receipts into verified_authorization_receipt. The database schema verifies that the exact normalized fields match the mutation; it does not perform cryptographic verification.

## 2. Linearization and isolation

The reference protocol uses PostgreSQL READ COMMITTED with a participant-scope row lock.

- begin_local_effect locks participant_scope_head FOR UPDATE.
- publish_topology locks the same participant_scope_head FOR UPDATE.
- begin_local_effect then locks logical_effect and inserts the attempt row.
- The row lock remains held until the enclosing transaction commits or aborts.
- finish_local_effect increments participant_sequence and marks the attempt/logical effect committed.
- The actual local business-row update must occur in that same transaction between begin and finish.

The **linearization point** for a local effect is the transaction commit containing the business-row change, participant sequence increment and committed receipt state. A topology publication and local effect are totally ordered by their acquisition of the shared scope-head row lock.

The protocol intentionally uses a coarse per-participant/scope lock for v0.1. It is easier to reason about and test than a read-then-write application mutex. A future optimization may shard locks only after preserving the same serialization and idempotency properties under concurrency tests.

Use one consistent lock order:

1. participant_scope_head;
2. logical_effect;
3. effect_attempt.

The migration does not set a database-wide isolation level. The adapter must explicitly use READ COMMITTED for this reference protocol and retry serialization/deadlock failures only after authoritative reconciliation. Never translate an uncertain transaction result into NeverAccepted.

## 3. Local-effect transaction usage

The intended local caller sequence is:

    BEGIN ISOLATION LEVEL READ COMMITTED;

    SELECT * FROM opsbench_participant.begin_local_effect(
        :verified_authorization_receipt_id,
        :stable_effect_id,
        :participant_id,
        :scope_id,
        :tenant_id,
        :attempt_id,
        :logical_action_key,
        :operation,
        :target,
        :request_sha256,
        :expected_topology_epoch,
        :expected_topology_sha256
    );

    -- Apply the actual participant-owned business-row mutation here.
    -- It must be in the SAME PostgreSQL transaction.

    SELECT * FROM opsbench_participant.finish_local_effect(
        :participant_id,
        :scope_id,
        :tenant_id,
        :logical_action_key,
        :attempt_id,
        :participant_receipt_sha256
    );

    COMMIT;

A deferred constraint trigger rejects commit if a local attempt inserted by begin_local_effect remains accepted_pending. If the transaction aborts, both business-row changes and the effect ledger/sequence update roll back.

This reference is not a generic stored procedure for arbitrary business operations. A production adapter must bind the specific business mutation to the same transaction and verify that it changes only the intended target.

## 4. Idempotency and topology replay

The stable key is:

    (participant_id, scope_id, tenant_id, logical_action_key)

The stable effect identity is participant/scope/tenant/logical key plus operation, target and exact request digest. Attempt ID and current topology epoch are not part of that stable fingerprint.

A retry after a successful commit must carry a fresh exact authorization receipt. The adapter validates that receipt and the exact request binding, then returns the original participant receipt before rejecting on the now-advanced topology head. It must not increment the sequence or create another effect.

A conflicting operation, target, request digest or effect ID under the same logical key is rejected.

An accepted_pending or outcome_unknown logical action blocks a new attempt. A new attempt can be started only after a separate trusted reconciler has established a strong NeverAccepted result for the exact prior attempt. The reconciliation publisher and evidence schema are deliberately not implemented in this migration.

## 5. Remote-provider boundary

PostgreSQL cannot atomically commit a mutation in an unrelated cloud API, Kubernetes control plane or payment provider.

For remote participants:
- persist a durable intent before dispatch;
- use provider-side idempotency/conditional operations where available;
- preserve outcome_unknown after response loss or process crash;
- reconcile through authoritative provider evidence;
- never use a missing response or database connectivity failure as NeverAccepted.

The local functions in this migration must not be used to claim exactly-once remote execution. The remote adapter is a separate conformance target.

## 6. Required independent-session and crash-cut tests

The next implementation slice must execute the migration against a pinned disposable PostgreSQL version and use independent sessions/processes:

1. commit vs topology-publication race in both lock acquisition orders;
2. two processes begin the same idempotency key concurrently;
3. same key/same operation/target/request returns one committed receipt;
4. same key/different request or target conflicts;
5. permit bound to old topology cannot be retargeted;
6. fresh current-epoch authorization replays the old receipt after topology advances;
7. process termination before begin, after row lock, after pending insert, after local business update, after finish and before commit;
8. lost response after commit, then restart and return the original receipt;
9. commit attempted without finish is rejected by the deferred constraint trigger;
10. database disconnect, deadlock, lock timeout and ambiguous commit remain unknown and block retries;
11. participant_sequence remains unique and monotonic across restarts.

Every race test must use independent database connections. At least the process-restart cases must use independent OS processes or containers; Rust threads sharing one connection pool are not sufficient evidence.

## 7. Remaining work and claim ceiling

Not yet implemented/qualified:
- migration execution and rollback test;
- Postgres-backed Rust adapter;
- authenticated authorization receipt ingestion and database role grants;
- trusted NeverAccepted reconciliation receipt path;
- remote provider idempotency adapters;
- process-level crash-cut harness;
- exact-head CI against a pinned PostgreSQL image digest.

A passing future campaign will qualify only the pinned PostgreSQL build, migration digest, adapter code and tested fault model. It will not prove generic atomicity with arbitrary external providers or every production storage configuration.


## Disposable PostgreSQL integration harness

The accompanying workflow uses a disposable PostgreSQL 16.4 service and executes the migration rather than merely validating the SQL file as text.

The shell harness exercises:
- migration application;
- local business-row mutation and participant receipt in one transaction;
- transaction rollback removing both business and participant changes;
- deferred rejection when a local attempt is not finalized;
- exact idempotent replay after topology advances, preserving original commit epoch and sequence;
- conflicting request under an existing logical idempotency key;
- stale topology epoch and topology-digest rejection;
- independent-session race between local effect commit and topology publication;
- independent-session race on the same logical idempotency key, expecting one attempt and one committed sequence;
- same-epoch/different-digest rejection.

The race tests use separate psql processes and independent database connections. They are not a full crash-cut/restart campaign: process termination at each WAL/commit boundary, response-loss recovery, deadlock/lock-timeout classification, and authentication-adapter integration remain follow-up work.

The CI service image is the explicit PostgreSQL 16.4 Alpine tag and the job verifies the server reports version 16.4. The run establishes behavior only for that disposable configuration and test cases; it does not qualify arbitrary production database settings or remote provider effects.
