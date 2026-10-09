# Work-Case Storage and Tenant-Isolation Decision

**Status:** proposed architecture decision; approval and implementation qualification remain open.  
**Scope:** native work-case service, local/self-hosted and hosted Luminous deployments.  
**Decision date:** 2026-10-09.

## Decision

Use one versioned domain and repository contract, with persistence choices constrained by deployment profile:

- **SQLite:** local, single-host, single-organization deployments and deterministic reference/conformance tests. The database stays on a local filesystem alongside its WAL/SHM files. Do not place a WAL database on a network filesystem or let separate hosts concurrently access the same file.
- **PostgreSQL:** default candidate for managed/multi-tenant, multi-worker, hybrid, and other deployments requiring concurrent independent application processes, centralized backups, and an explicit server-side tenant boundary. The production adapter must be qualified before this becomes a supported deployment claim.
- **Do not make the product depend on the database vendor.** The service contract, lifecycle state machine, idempotency semantics, mapping scope, and evidence model stay vendor-neutral. SQL migrations and concurrency tests are backend-specific.

This is an architecture decision, not a claim that either backend has passed production qualification. The current SQLite implementation is a local reference model only.

## Why

SQLite's official WAL documentation says readers can proceed alongside a writer, but there is still only one writer at a time; the WAL index relies on shared memory and does not work over a network filesystem. SQLite's own deployment guidance recommends a client/server database such as PostgreSQL when many different machines need simultaneous reads and writes.

Sources:
- [SQLite Write-Ahead Logging](https://www.sqlite.org/wal.html)
- [SQLite Over a Network: Caveats and Considerations](https://www.sqlite.org/useovernet.html)

PostgreSQL provides row-level security policies that can constrain which rows a database role may read or mutate. When RLS is enabled with no applicable policy, access defaults to deny; table owners normally bypass RLS unless FORCE ROW LEVEL SECURITY is used, and superuser or BYPASSRLS roles always bypass it. Therefore RLS is useful defense in depth but is not a magic tenant boundary if the application connects as an over-privileged role.

Source:
- [PostgreSQL 18 — Row Security Policies](https://www.postgresql.org/docs/current/ddl-rowsecurity.html)

## Required hosted PostgreSQL design

1. **Trusted tenant context.** The API authenticates a principal and resolves the tenant on the server. A request-body tenant_id, case ID, provider label, or integration ID is not authority.
2. **RLS as a second barrier.** Enable and force RLS on tenant-owned tables where appropriate. Use a least-privilege application role that is not table owner, superuser, or BYPASSRLS; ensure schema migrations run under a distinct privileged role.
3. **Transaction-local context.** Bind verified tenant context to the database transaction, not to a pooled session that may later serve a different tenant. Clear/reset any session state before releasing connections. Test missing tenant context and connection-pool reuse.
4. **Composite tenant keys.** Include tenant scope in primary/unique keys and foreign-key relationships for cases, activity, mapping, idempotency, evidence links, and outbox records. Verify every join preserves tenant scope.
5. **Mutation boundary.** Use typed commands, expected revisions, database-enforced uniqueness, durable idempotency, and a transaction covering case state + activity + idempotency outcome + outbox row. A transaction does not provide exactly-once network effects; consumers must be idempotent.
6. **External mappings.** Enforce uniqueness on (tenant_id, connection_id, provider, external_id). A connection ID must be resolved from the authenticated connector registry; callers cannot choose another connection simply by submitting its identifier.
7. **Outbox.** A production dispatcher needs a documented lease/claim/ack protocol, bounded retry, operator-visible dead-letter state, per-case ordering where required, reconciliation, and replay-safe consumers. Dispatch only redacted, allowlisted event payloads.
8. **Evidence and retention.** Keep evidence payloads in their appropriate access-controlled store; work-case records carry classified references and integrity digests, not session keys or raw screen/audio. Define retention, export, legal hold, backup and verified deletion semantics before production.
9. **Migration and recovery.** Publish versioned migrations, backup/restore tests, process-kill/crash-window tests, disk-full behavior, recovery drills, and an explicit rollback plan. Never treat an in-memory test or schema-valid fixture as database qualification.

## Qualification matrix

| Claim | Current SQLite reference | Required before production claim |
|---|---|---|
| Domain transitions and expected revision | Model-level tests are authored | Exact-head visible CI and an independent state-machine oracle |
| Atomic local state/history/idempotency/outbox | Transaction reference plus fault-injection tests are authored | Run tests; process-kill, restore, disk-full and migration tests |
| Concurrent duplicate submission | Same-host multi-store test is authored | Exact tests pass; production database contention and retry behavior qualified |
| Tenant isolation | The API is not implemented | API negatives across list/search/read/write/attachments/evidence/export/subscriptions plus RLS/database-role tests |
| Cross-machine concurrency | Not provided by this model | PostgreSQL-backed integration and load/failure tests |
| Live ConnectWise interoperability | Not implemented | Product-specific authentication, least-privilege sandbox account, rate-limit/pagination/revision tests |
| Operational recovery | Not qualified | Documented backup, restore, upgrade, rollback and data export exercises |

## Avoid premature abstraction

Do not implement a generic database framework or multiple full storage backends before a consumer requires them. First define the repository/service interface and qualify one production backend. Keep the SQLite reference small and deterministic. Add SQLite as a supported customer deployment profile only after backup, upgrade, restore, and concurrency requirements are explicitly bounded; otherwise label it as a local development/conformance mode.
