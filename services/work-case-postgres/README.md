# Rust Work Case PostgreSQL Repository

**Status:** initial PostgreSQL adapter slice; not yet production-qualified.

This Rust crate implements a contract-shaped Work Case V1 create/read path plus revision-checked lifecycle transitions using exact-pinned SQLx 0.9.0, explicit SQL and compile-time-checked query macros. It uses opaque string identifiers and idempotency keys compatible with the reference model, typed case fields, RFC 3339 timestamps, append-only activity, a minimal outbox event, and normalized external/evidence reference tables. Provider intake, an authenticated API, assignment/evidence mutation endpoints, and an outbox dispatcher remain unimplemented.

## Trust boundary

The auth/session layer must construct VerifiedPrincipal only after authenticating the actor and resolving tenant membership from trusted server-side state. Do not populate it from request-body tenant_id, actor_id, or role fields. The repository constructor is crate-private so a caller outside this crate cannot fabricate a VerifiedPrincipal directly; an authenticated service boundary still has to be implemented before production use.

Every repository operation sets app.tenant_id transaction-locally and uses explicit tenant predicates; PostgreSQL RLS is a second barrier. The custom GUC is settable by the database role and is **not an identity proof**. These policies protect against omitted tenant predicates and accidental query widening; they do not protect against arbitrary SQL issued by a compromised application role. Server-side membership validation, parameterized SQL and prevention of arbitrary SQL remain mandatory. A test-only query deliberately omits the tenant predicate to test RLS itself. The runtime role is not the migration/table owner; it cannot update/delete activity, and its only idempotency update privilege is the one-time result-payload finalization guarded by database triggers.

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

The nine-test PostgreSQL suite covers contract-compatible opaque keys/IDs, same-semantics replay, conflicting idempotency reuse, one activity and outbox event per creation, concurrent duplicate claims, tenant RLS isolation, missing-context default denial, transaction-local context on a reused connection, valid lifecycle transitions, invalid transitions, stale-revision conflicts, original-response replay after the case has changed, the actual runtime role's schema/table privilege boundary (no object creation, activity mutation, mapping/evidence writes, or unqualified outbox delivery updates), and rollback of a failed create's idempotency claim with no partial activity/outbox side effects. Idempotency rows store the original response snapshot; a database trigger permits that snapshot to be finalized once and a deferred constraint trigger rejects committing an incomplete record. Missing DATABASE_URL or APP_DATABASE_URL is a hard failure, not a skipped test.

These tests do not qualify authentication, lifecycle transitions, provider interoperability, dispatch/retry/dead-letter recovery, backup/restore, upgrades from historical schemas, or production operations. SQLx compile-time checking does not prove authorization correctness.

A Cargo.lock has not been generated in this environment because package resolution is unavailable here. Direct dependencies are exact-pinned, but reproducible transitive dependency locking is not claimed until a reviewed lockfile is committed.
