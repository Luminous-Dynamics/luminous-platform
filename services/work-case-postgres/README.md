# Rust Work Case PostgreSQL Repository

**Status:** initial PostgreSQL adapter slice; not yet production-qualified.

This crate uses exact-pinned SQLx 0.9.0 and explicit SQL with compile-time-checked query macros. Its current scope is case creation, same-semantics idempotent replay, tenant-scoped reads, append-only creation activity, and one minimal outbox event. Lifecycle transitions, API/authentication, provider intake, and outbox dispatch remain unimplemented.

## Trust boundary

The auth/session layer must construct VerifiedPrincipal only after authenticating the actor and resolving tenant membership from trusted server-side state. Never populate it directly from caller-controlled tenant_id, actor_id, role, or connection fields. This type documents a service-layer contract; it is not a cryptographic capability.

Each repository operation sets app.tenant_id transaction-locally, uses explicit tenant predicates, and also relies on PostgreSQL RLS as a second barrier. A test-only query deliberately omits the tenant predicate to test RLS itself. The runtime role is distinct from the migration/table owner and has no UPDATE/DELETE permission on case_activity or command_idempotency.

## Local qualification prerequisites

Use Rust 1.96 and a disposable PostgreSQL 18 database. From the repository root:

~~~sh
export DATABASE_URL='postgres://postgres:postgres@localhost:5432/work_case_test'
export APP_DATABASE_URL='postgres://work_case_app:work_case_test_password@localhost:5432/work_case_test'
cargo +1.96.0 install sqlx-cli --version 0.9.0 --no-default-features --features rustls,postgres
cargo +1.96.0 sqlx migrate run --source services/work-case-postgres/migrations
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f services/work-case-postgres/tests/bootstrap_app_role.sql
cargo +1.96.0 test --manifest-path services/work-case-postgres/Cargo.toml --all-targets
~~~

The bootstrap SQL and its password are for disposable test databases only. Production migrations must run under a separately managed privileged identity; the service must use a least-privilege app role.

## Test scope

The PostgreSQL suite covers same-semantics replay, conflicting idempotency reuse, one activity and outbox event per creation, duplicate concurrent commands over pooled connections, RLS isolation, and transaction-local context when reusing one connection. Missing DATABASE_URL or APP_DATABASE_URL is a hard failure, not a skipped test.

These tests do not qualify API authentication, lifecycle transitions, external provider interoperability, dispatch/retry/dead-letter recovery, backup/restore, upgrade from prior released schemas, or production operations. SQLx compile-time checking also does not prove authorization correctness.

A Cargo.lock has not been generated in this environment because package resolution is unavailable here. Direct dependencies are exact-pinned, but reproducible transitive dependency locking is not claimed until a reviewed lockfile is committed.
