# Rust Work Case PostgreSQL Repository

**Status:** initial PostgreSQL adapter slice; not yet production-qualified.

This Rust crate implements a contract-shaped Work Case V1 create/read path using exact-pinned SQLx 0.9.0, explicit SQL and compile-time-checked query macros. It uses opaque string identifiers and idempotency keys compatible with the reference model, typed case fields, RFC 3339 timestamps, append-only creation activity, a minimal outbox event, and normalized external/evidence reference tables. State transitions, provider intake, an authenticated API, and an outbox dispatcher remain unimplemented.

## Trust boundary

The auth/session layer must construct VerifiedPrincipal only after authenticating the actor and resolving tenant membership from trusted server-side state. Do not populate it from request-body tenant_id, actor_id, or role fields. The type documents a service-layer contract; it is not a cryptographic capability.

Every repository operation sets app.tenant_id transaction-locally and uses explicit tenant predicates; PostgreSQL RLS is a second barrier. A test-only query deliberately omits the tenant predicate to test RLS itself. The runtime role is not the migration/table owner and has no UPDATE/DELETE permission on case_activity or command_idempotency.

## Local qualification prerequisites

Use Rust 1.96 and a disposable PostgreSQL 18 database. From the repository root:

~~~sh
export DATABASE_URL='postgres://postgres:postgres@localhost:5432/work_case_test?sslmode=disable'
export APP_DATABASE_URL='postgres://work_case_app:work_case_test_password@localhost:5432/work_case_test?sslmode=disable'
cargo +1.96.0 install sqlx-cli --version 0.9.0 --no-default-features --features rustls,postgres
cargo +1.96.0 sqlx migrate run --source services/work-case-postgres/migrations
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f services/work-case-postgres/tests/bootstrap_app_role.sql
cargo +1.96.0 test --manifest-path services/work-case-postgres/Cargo.toml --all-targets
~~~

The bootstrap SQL and its password are for disposable test databases only. Production migrations must run under a separately managed privileged identity; the service must use a least-privilege app role.

## Test scope

The PostgreSQL suite covers contract-compatible opaque keys/IDs, same-semantics replay, conflicting idempotency reuse, one activity and outbox event per creation, concurrent duplicate claims, tenant RLS isolation, and transaction-local context on a reused connection. Missing DATABASE_URL or APP_DATABASE_URL is a hard failure, not a skipped test.

These tests do not qualify authentication, lifecycle transitions, provider interoperability, dispatch/retry/dead-letter recovery, backup/restore, upgrades from historical schemas, or production operations. SQLx compile-time checking does not prove authorization correctness.

A Cargo.lock has not been generated in this environment because package resolution is unavailable here. Direct dependencies are exact-pinned, but reproducible transitive dependency locking is not claimed until a reviewed lockfile is committed.
