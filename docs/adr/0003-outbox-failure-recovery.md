# ADR 0003: Outbox Failure and Recovery Contract

**Status:** Implemented in prototype; hosted qualification pending; runtime replay is intentionally not exposed  
**Date:** 2026-10-10  
**Scope:** PostgreSQL Operations Event V1 transactional outbox  
**Related:** [ADR 0001](./0001-production-operational-state-store.md), [issue #5](https://github.com/Luminous-Dynamics/luminous-platform/issues/5), [issue #18](https://github.com/Luminous-Dynamics/luminous-platform/issues/18)

## Context

A retry delay alone does not bound the number of deliveries. A permanently failing predecessor can block an incident indefinitely, and directly writable lifecycle columns make the transition protocol dependent on application discipline. Terminalizing a row must not silently authorize later events to leapfrog it.

## Decision

### Lifecycle authority

- PostgreSQL owns lease claim, acknowledgement, retry scheduling, and dead-letter transitions through narrowly granted `SECURITY DEFINER` routines.
- The routines use `search_path = pg_catalog, ops, pg_temp`, bind the explicit tenant parameter to transaction-local tenant context, and require the current worker's unexpired lease for acknowledgement/failure transitions.
- The runtime application role has no direct `UPDATE` permission on outbox status, attempts, lease ownership/expiry, acknowledgement, or scheduling metadata.
- The definer owner is `luminous_ops_retry_owner`, a dedicated `NOLOGIN`, non-superuser, `NOBYPASSRLS` role. It gets only the outbox read and column update permissions required by these routines; runtime identities must not be members of it.

### Stable failure taxonomy

Persist only stable codes, never raw provider exception strings or remote response bodies.

Retryable codes: `TRANSIENT_NETWORK`, `REMOTE_RATE_LIMITED`, `REMOTE_SERVER_ERROR`, `REMOTE_ACK_UNKNOWN`, and `UNCLASSIFIED`.

Immediate dead-letter codes: `AUTHENTICATION_REJECTED`, `AUTHORIZATION_REJECTED`, `DESTINATION_MISMATCH`, `TLS_IDENTITY_REJECTED`, `PAYLOAD_REJECTED`, and `REMOTE_CONTRACT_MISMATCH`.

### Finite delivery attempts

- Maximum: 12 lease/dispatch attempts per outbox row. Claims increment the counter; attempts include expired worker leases because a worker can crash after an external effect but before acknowledgement.
- Retry delay: deterministic exponential backoff beginning at 5 seconds, doubling to a 1-hour ceiling.
- A retryable failure recorded on attempt 12 dead-letters the row.
- If a worker disappears on attempt 12 without reporting a failure, the next claim operation terminalizes the expired lease with `ATTEMPT_LIMIT_EXHAUSTED` instead of re-leasing it.
- A dead-lettered row has no active lease and carries a stable last failure code/time plus a dead-letter timestamp. The tenant-scoped query returns metadata only, not payloads.

### Ordering and delivery semantics

A predecessor is considered incomplete unless its status is `DELIVERED`. Therefore, `DEAD_LETTERED` does not satisfy or skip an incident sequence; later rows for that incident remain blocked. This is intentional fail-closed behavior. Delivery remains at-least-once, not exactly-once.

A crash after a remote send but before its acknowledgement remains ambiguous. The attempt ceiling is not proof the provider did not apply the operation; external consumers must use the stable outbox identity for idempotency where supported.

## Explicit non-goals

This migration does not add a “skip predecessor” action, mark a dead-lettered row as delivered, mutate its payload, or expose replay to the ordinary delivery worker. An authenticated, tenant-authorized operator recovery path must provide immutable recovery audit and handle ambiguous remote outcomes before replay can be offered. These prerequisites are tracked in issue #18.

## Required exact-head evidence

- PostgreSQL integration tests demonstrate direct runtime UPDATE denial for all delivery-state columns.
- Wrong-tenant, wrong-owner, expired-lease, transient retry, immediate permanent-failure dead-letter, 12-attempt dead-letter, and expired final-lease cases are tested.
- Dead-letter inspection is tenant-scoped, bounded, redacted, and verified not to return payload bytes.
- A dead-lettered predecessor prevents claim of its successor.
- Rust contract/PostgreSQL jobs complete successfully on the exact source SHA using the pinned PostgreSQL image. Authored tests and queued jobs are not qualification evidence.
