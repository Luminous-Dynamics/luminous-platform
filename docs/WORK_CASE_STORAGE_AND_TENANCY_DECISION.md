# Work-Case Storage and Tenant-Isolation Decision

**Status:** proposed architecture decision; driver choice recorded, implementation and production qualification remain open.  
**Scope:** native work-case service, local/reference conformance and hosted Luminous deployments.  
**Decision date:** 2026-10-09.

## Decision

Use one versioned domain and repository contract, with a deliberately narrow production persistence choice:

- **PostgreSQL 18.x is the authoritative database for the first shared, multi-user production service.** Pin the exact server image/package to a reviewed immutable release artifact in deployment and CI. This is the target architecture, not a statement that an adapter or deployment has passed qualification.
- **Use Rust + SQLx 0.9.x as the initial PostgreSQL client/toolkit baseline.** Keep SQL explicit rather than introducing an ORM or a generic multi-database layer. Use SQLx's PostgreSQL driver, bounded async pool, migrations, and compile-time checked `query!` / `query_as!` macros for static application queries where practical. Review and pin the exact crate version and feature set in the Rust crate manifest; check in the application's lockfile where appropriate. SQLx 0.9 documents Rust 1.94 as its minimum supported version, compatible with this repository's Rust 1.96 contract toolchain.
- **SQLite remains a Python reference/conformance harness in this work.** It is not the production service or the production repository. A Rust SQLite profile may be considered separately later, but must have its own bounded deployment definition and backup, upgrade, restore, concurrency, and recovery qualification. Do not keep two production databases in parity just because SQLx supports both.
- **Do not launch with multiple production database backends.** Define a narrow repository/service boundary around the domain, but qualify PostgreSQL first. Domain lifecycle, expected-revision, idempotency, mapping scope, evidence classification, and outbox semantics remain independent of SQLx and PostgreSQL.
- **Production domain logic, repository implementations, API handlers, tenant authorization, and outbox delivery must be Rust.** The checked-in Python model and SQLite store remain non-production reference/conformance harnesses; they must not become production services or privileged execution components.
- **Treat the outbox as at-least-once.** A downstream send may succeed while its acknowledgement is lost. Retry with a stable outbox identity and require idempotent consumers; do not claim exactly-once external side effects.

This distinction matters: PostgreSQL is the database; SQL is the query language; SQLx is the Rust client/toolkit. There is no separate "Rust SQL" database to switch to.

## Why PostgreSQL + SQLx

PostgreSQL supplies server-side transactional integrity, concurrent multi-worker access, centralized recovery/backup operations, and row-level security (RLS). SQLx fits this Rust-first service because it is asynchronous, provides a PostgreSQL driver and connection pool, supports versioned migrations, and can check static SQL against database metadata. We prefer ordinary, reviewed SQL over an opaque ORM layer for tenant-critical joins, idempotency claims, revision checks, and outbox ordering.

Compile-time query checking is a useful defect barrier, **not** an authorization proof: it cannot establish tenant isolation, correct transaction boundaries, the absence of confused-deputy behavior, or safe recovery. CI must check query metadata against the migration schema (with a PostgreSQL test database or correctly refreshed SQLx offline metadata) and run behavior tests against a real PostgreSQL server. Do not select SQLx's runtime-generic `Any` driver or abstract away PostgreSQL-specific semantics for a second backend that is not being shipped.

Sources:
- [SQLx project and feature documentation](https://github.com/transact-rs/sqlx)
- [SQLx 0.9.0 release notes and MSRV policy](https://github.com/transact-rs/sqlx/blob/main/CHANGELOG.md)
- [SQLx checked query macro requirements](https://docs.rs/sqlx/latest/sqlx/macro.query.html)
- [PostgreSQL 18 — Row Security Policies](https://www.postgresql.org/docs/18/ddl-rowsecurity.html)

## Required hosted PostgreSQL design

1. **Trusted tenant context.** The API authenticates a principal and resolves the tenant on the server. A request-body `tenant_id`, case ID, provider label, or integration ID is not authority.
2. **RLS as a second barrier.** Enable and force RLS on tenant-owned tables where appropriate. Use a least-privilege application role that is not table owner, superuser, or `BYPASSRLS`; schema migrations run under a distinct privileged role.
3. **Transaction-local context.** Start a transaction, bind the verified tenant to that transaction (for example with parameterized `set_config('app.tenant_id', $1, true)`), then perform all reads/writes through the same transaction handle. Do not use pooled session-global `SET` state. Test missing context, rollback, connection-pool reuse, and concurrent tenants.
4. **Explicit query scoping.** Every repository query still binds the trusted tenant ID, and every join preserves tenant scope. RLS is defense in depth, not a replacement for explicit authorization or tenant predicates. Policies must default-deny when context is absent.
5. **Composite tenant keys.** Include tenant scope in primary/unique keys and foreign-key relationships for cases, activity, mappings, idempotency, evidence links, and outbox records.
6. **Mutation boundary.** Use typed Rust commands, expected revisions, database-enforced uniqueness, durable idempotency, and one transaction spanning case state + activity + idempotency outcome + outbox row. Handle unique-constraint races and serialization/deadlock errors explicitly; retry only transactions whose effects are known to be safe to retry.
7. **External mappings.** Enforce uniqueness on `(tenant_id, connection_id, provider, external_id)`. Resolve `connection_id` from authenticated connector configuration; a caller cannot choose another connection by submitting its identifier.
8. **Outbox.** Implement a production dispatcher with a documented lease/claim/ack protocol, bounded retry, operator-visible dead-letter state, per-case ordering where required, reconciliation, and replay-safe consumers. Dispatch only redacted, allowlisted event payloads. A local database transaction cannot make a remote network effect exactly-once.
9. **Evidence and retention.** Keep evidence payloads in their appropriate access-controlled store; work-case records carry classified references and integrity digests, not session keys or raw screen/audio. Define retention, export, legal hold, backup and verified deletion semantics before production.
10. **Migrations and recovery.** Publish versioned SQL migrations with forward-upgrade expectations. Test fresh install, every supported upgrade path, interrupted migration recovery, backup/restore, process kill, disk full, and an explicit rollback/export plan. Never treat in-memory tests or schema-valid fixtures as database qualification.

## Current implementation status — 2026-10-09

The first Rust/PostgreSQL adapter slice now exists at [services/work-case-postgres](../services/work-case-postgres/README.md). It implements case creation, idempotent replay/conflict detection, tenant-scoped reads, one append-only creation activity, and one minimal outbox event inside a transaction. Its migration declares tenant-scoped composite keys, foreign keys, default-deny RLS policies on all tenant-owned tables, and transaction-local tenant context. PostgreSQL tests are wired to use a separate non-owner runtime role.

**Qualification is pending.** The authored repository tests have not yet been shown to pass in CI. The workflow must execute the tests against the actual disposable PostgreSQL image and exact source SHA. This slice does not yet implement lifecycle transitions, an authenticated API, provider intake, or a dispatcher. The direct dependencies are exact-pinned, but no reviewed Cargo.lock has been committed yet, so transitive dependency resolution is not yet reproducibly locked.

## Implementation sequence

1. Preserve the Work Case V1 contract and its Python reference tests as the domain/conformance oracle. Their test counts are not PostgreSQL evidence.
2. Add a focused Rust crate for the production service/repository with SQLx 0.9.x, pinned feature selection, checked static queries, and versioned PostgreSQL migrations. Avoid creating a generic database framework.
3. Provision a pinned PostgreSQL 18.x service in CI and run integration tests using separate migration and least-privilege application roles. Check the SQLx query metadata and execute all migration and repository tests against that server.
4. Exercise actual database behavior: competing revision updates, concurrent duplicate idempotency claims from separate connections, transaction rollback at write boundaries, cross-tenant negative tests, missing RLS context, pooled-connection reuse, and concurrent outbox leases/ordering.
5. Keep the PR draft and all production-readiness claims blocked until exact-head CI executes every required step and publishes the tested SHA, PostgreSQL version, migration version, test count and result. A queued or skipped job is not a pass.

## Current qualification boundary

The current SQLite implementation is a local Python reference model only. It atomically commits the case snapshot/history/idempotency/external mapping/outbox in its own local transaction and has fail-closed schema-shape checks. Those properties describe that reference implementation, not the future Rust/PostgreSQL service.

The reference schema initializer only initializes an empty schema-version-0 database; it rejects pre-existing unversioned tables, unsupported/future versions and schema-version-1 stores missing required tables, columns, indexes, primary keys, unique constraints or tenant-scoped foreign keys. WAL mode is checked rather than assumed. These regression tests are authored but must not be considered passed until visible exact-head CI executes them.

| Claim | Current Python/SQLite reference | Required before production claim |
|---|---|---|
| Domain transitions and expected revision | Reference tests authored | Exact-head CI and independent state-machine oracle |
| Atomic state/history/idempotency/outbox | Local transaction and fault-injection tests authored | Run reference tests; PostgreSQL rollback and process-kill qualification |
| Concurrent duplicate submission | Same-host multi-store test authored | Concurrent PostgreSQL contention/race tests over independent connections |
| Tenant isolation | Production API is not implemented | Rust API negatives plus application-role RLS and pool-reuse tests |
| Cross-machine concurrency | Not provided by reference | PostgreSQL-backed integration and load/failure tests |
| Live provider interoperability | Not implemented | Product-specific auth, rate-limit, pagination and revision tests |
| Operational recovery | Not qualified | Versioned migrations, backup/restore, disk-full and recovery exercises |

## Avoid premature breadth

Start with one PostgreSQL production backend. Do not add a Rust SQLite adapter, ORM, generic database switch, multi-region writes, sharding or distributed transactions until a demonstrated deployment requirement justifies it. Keep the service and database operations understandable for sovereign deployments: installation, TLS trust, secret rotation, health checks, migrations, backups, restore rehearsal, export and recovery documentation are part of the product.
