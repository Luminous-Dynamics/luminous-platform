# SQLite durable-seam reference model

**Status:** local conformance harness; not production implementation  
**Scope:** Operations Event Contract V1, synthetic data only

## What this adds

`durable_sqlite_model.py` complements, rather than replaces, `adapter_model.py`'s `MemoryInbox`. It uses SQLite constraints and transactions to exercise a durable local integration seam:

- Event identity is unique on configured source + CloudEvents `id`; identical replay is a duplicate, while changed content for a reused identity conflicts.
- Business-effect idempotency is unique on configured connection + canonical tenant + `idempotencykey`; a repeated key with changed semantic effect conflicts.
- Explicit source/company/tenant/ticket/local-incident bindings are checked before mutation. A binding conflict is quarantined using only an event digest, a reason code, and a timestamp; raw payload content is not copied into the quarantine table.
- Local inbox, idempotency record, incident snapshot state, and outgoing outbox row commit in the same SQLite transaction. Test-only injected exceptions before commit must roll all those writes back.
- Revision handling is conservative. Older revisions do not replace newer state; conflicting state for the same revision fails. When revisions cannot be compared under an explicitly configured adapter policy, the observation is quarantined rather than sorted lexically or accepted by arrival time.
- Outbox dispatch uses retryable leases and preserves per-incident ordering: a later unacknowledged message for the same incident cannot be claimed while an earlier one remains pending or leased. Lease expiry can cause the same message to be delivered again; this is **at-least-once**, not exactly-once network delivery. A late or wrong owner cannot acknowledge an expired/reassigned lease.

The test suite includes restarts against the same SQLite file, duplicate concurrent submissions, transaction fault injection, tenant/resource mapping negatives, opaque revision ordering, equal-timestamp outbox ordering, and outbox lease expiry/acknowledgement cases. Because the model has no dead-letter transition yet, a permanently failing earlier message intentionally blocks later messages for that incident rather than letting the timeline silently reorder.

## Reproduce

From the repository root, run the complete contract test suite:

```sh
python3 -m venv /tmp/operations-event-contract-venv
/tmp/operations-event-contract-venv/bin/python -m pip install --no-cache-dir -r contracts/operations-event-v1/requirements.txt
PYTHONDONTWRITEBYTECODE=1 /tmp/operations-event-contract-venv/bin/python -m unittest discover -s contracts/operations-event-v1/tests -p 'test_*.py' -v
```

Only the existing pinned JSON Schema validator is an external dependency; SQLite and concurrency utilities come from Python's standard library.

## Claim boundaries

This is a model for tests, not an operational service. It does not implement ConnectWise HTTP/webhook intake, provider-specific authentication, token refresh, provider rate limits, production migrations, retries against a real downstream destination, independent signatures/receipts, or privileged-action authorization. An exception-based fault-injection test is not a process-kill or power-loss test. Local SQLite semantics are not evidence of a different production database's isolation guarantees, filesystem durability configuration, tenant isolation, distributed multi-writer behavior, or production scale.

The test revision comparator only recognizes numeric tokens such as `revision-2`. Do not use it for a real provider unless that provider's contract documents a stable ordered revision scheme. The default is fail-closed when the model cannot establish an ordering rule.

## Production continuation gates

1. Translate this state machine into the selected production database and publish versioned migrations with uniqueness constraints at the database boundary.
2. Execute subprocess-kill/restart recovery tests around transaction commit, acknowledgement, and outbox send/ack gaps; test backup/restore and disk-full behavior.
3. Implement authenticated provider parsing and enforce tenant/resource authorization independently of adapter-supplied labels.
4. Add stale-revision reconciliation against the provider's documented revision/conditional-update mechanism, without guessing from timestamps.
5. Test tenant isolation across reads, search, attachments, exports, event subscriptions, and writes using independently created tenant identities.
6. Add an explicit dead-letter/quarantine transition, bounded retry/backoff, operator alerting, and reviewed replay semantics; while an earlier incident event is unresolved, do not silently release later events out of order.
7. Keep outbox consumers idempotent and document that external side effects can repeat when a send succeeds but its acknowledgement is lost.

Passing this harness can qualify only these test cases on the exact revision. It cannot be summarized as production-ready, exactly-once, or database isolation proven.
