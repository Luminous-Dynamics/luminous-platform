# Native Work Case Contract V1

**Status:** versioned snapshot schema, in-memory state-machine model, and SQLite persistence reference. Not a production PSA service or qualified ConnectWise integration.

## Artifacts

- \`schema.json\`: JSON Schema Draft 2020-12 for the portable snapshot.
- \`examples/incident-work-case.json\`: deterministic synthetic fixture.
- \`work_case_model.py\`: domain/state-machine model.
- \`sqlite_work_case_store.py\`: local persistence conformance reference.
- \`tests/test_work_case_model.py\`: lifecycle and domain boundary tests.
- \`tests/test_sqlite_work_case_store.py\`: restart/replay, transaction, mapping, concurrency, and tenant-negative tests.

## Reproduce locally

~~~sh
python3 -m venv /tmp/work-case-v1-venv
/tmp/work-case-v1-venv/bin/python -m pip install --no-cache-dir -r contracts/operations-event-v1/requirements.txt
PYTHONDONTWRITEBYTECODE=1 /tmp/work-case-v1-venv/bin/python -m unittest discover -s contracts/work-case-v1/tests -p 'test_*.py' -v
~~~

See [Work-Case Storage and Tenant-Isolation Decision](../../docs/WORK_CASE_STORAGE_AND_TENANCY_DECISION.md) for the proposed SQLite-versus-PostgreSQL deployment boundary and production RLS requirements.

## Persistence and event semantics

The SQLite reference commits case snapshot, revision, append-only activity row, idempotency response, external mapping (when present), and a minimal outbox event in one local transaction. Fault-injection tests target selected write boundaries. Each mutation opens a fresh connection; tests reconstruct the store against the same database file and run duplicate calls through separate store objects. Initialization is fail-closed: only an empty unversioned database is initialized, future or unsupported versions are rejected, and a versioned database with missing required tables, columns, or indexes is rejected rather than silently repaired. WAL mode is confirmed after schema preflight and required on subsequent connections; a backend that cannot provide WAL (including SQLite's per-connection `:memory:` database) is rejected instead of silently using a different journal mode.

The outbox represents pending local delivery, not exactly-once messaging. A future dispatcher needs authenticated destination configuration, lease/claim/ack behavior, retry bounds, a visible dead-letter state, per-case ordering, reconciliation, and idempotent consumers. The event envelope omits summaries, evidence payloads, and credentials by design.

## Lifecycle

| Current state | Allowed next states |
|---|---|
| open | in_progress, waiting, cancelled |
| in_progress | waiting, resolved, cancelled |
| waiting | in_progress, resolved, cancelled |
| resolved | open (reopen with reason), closed |
| closed | none |
| cancelled | none |

Every mutation is tenant-scoped and revision-checked. Reuse of an idempotency key with different semantics conflicts. External mapping uniqueness is scoped by tenant, configured connection, provider, and external ID. A real API must resolve connection identity through trusted connector configuration, not trust a request-body string as authority.

## Qualification boundary

These tests cover this model and the selected local SQLite runtime only. They do not establish power-loss guarantees on every filesystem/hardware combination, distributed consensus, production database semantics, multi-region failover, API authentication, complete tenant isolation, backup/restore, data retention, SLA/billing correctness, live ConnectWise behavior, cryptographic evidence verification, or remote actions. This is not a production migration or service deployment.

Required next gates: visible exact-head CI job logs; process-kill/restore/disk-full tests; schema migration tests from every released version; tenant-negative API tests across list/search/attachments/evidence/export/subscriptions; current-authorization/revocation checks; provider-specific API conformance; and an operator-visible outbox dispatcher with retries/dead letters.
