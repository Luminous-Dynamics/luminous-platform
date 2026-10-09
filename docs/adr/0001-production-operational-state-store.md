# ADR 0001: Production operational state store

**Status:** Proposed for review  
**Date:** 2026-10-09  
**Scope:** Shared Luminous Platform service-management and integration state  
**Decision owner:** Platform maintainers

## Context

The Operations Event Contract currently has a SQLite-backed reference model. It is valuable because it exercises local uniqueness constraints, transactional inbox/state/outbox writes, replay handling, revision conflicts, and retryable outbox leases without requiring a live provider.

That model is a conformance harness, not the production persistence layer. It does not establish production tenant isolation, distributed-writer behavior, process-kill/power-loss recovery, backup/restore correctness, production migrations, or behavior under the selected deployment's filesystem and durability settings.

The first shared service needs one transaction to atomically record a normalized domain mutation, inbox/idempotency state, append-only activity, and any resulting outbox event. Duplicate delivery and concurrent command races must be settled by database constraints—not by an in-process mutex. Tenant-scoped identifiers and access checks must survive joins, exports, search, attachments, and retries.

## Decision proposed

1. **Use PostgreSQL 18.x as the authoritative database for the first shared, multi-user production service.** Pin the deployment/test image or package to an immutable, reviewed release artifact. Reassess the baseline when the deployment matrix and dependency lockfile are qualified; do not silently float production to a new major version.
2. **Keep the SQLite implementation as a deterministic reference model and local conformance harness.** Its passing tests must never be presented as proof that the PostgreSQL implementation or any production deployment is qualified.
3. **Do not launch with multiple production database backends.** Define a narrow repository/service boundary around the domain, but qualify one production database first. A SQLite-only offline/single-node profile can be considered later with its own backup, concurrent access, upgrade, and recovery qualification.
4. **Keep the first release single-region and transactionally simple.** PostgreSQL is the system of record for case state, idempotency, activity, integration inbox, and outbox. Avoid distributed transactions across the provider and local database; use durable local commits plus at-least-once outbox delivery and idempotent consumers.
5. **Use database-enforced integrity.** Enforce event-identity uniqueness, business-idempotency uniqueness, required foreign keys, and tenant-scoped relationship constraints in PostgreSQL. A Rust lock, cache, adapter, or UI check is not a substitute for a database constraint.
6. **Use explicit tenant authorization plus defense-in-depth row-level security (RLS).** Resolve tenant context from an authenticated principal or configured provider connection, never from an untrusted request field alone. Every service query and mutation must bind that trusted tenant explicitly. Where RLS is enabled, use default-deny policies, transaction-local tenant context, an application role that is neither table owner nor BYPASSRLS, and FORCE ROW LEVEL SECURITY where appropriate. RLS is an additional barrier, not the only authorization mechanism.
7. **Treat the outbox as at-least-once.** A downstream send can succeed while its acknowledgement is lost. Retry with a stable outbox identity; require idempotent consumers and preserve required per-resource ordering. Do not claim exactly-once external side effects.

## Minimum relational boundaries

The first production schema should represent at least:

- **Tenant and principal bindings:** canonical tenant identity, authenticated connection identity, provider identity, and the configuration that binds external company/site identifiers to local resources.
- **Operational cases and activity:** tenant-scoped case identity, case kind and lifecycle, expected revision, typed state transitions, actor/reason/causality metadata, and append-only activity records.
- **External mappings:** connection + provider + external resource type + external identifier → an explicitly mapped local resource. Unknown or ambiguous mappings remain quarantined; never pick a target by heuristic.
- **Integration inbox:** authenticated source binding, CloudEvents source/id, content digest, normalized outcome, processing time, and enough linkage to the resulting business effect to support replay diagnostics.
- **Business idempotency:** unique key scoped by authenticated connection and canonical tenant, plus a digest of semantic effect. Reuse with the same effect is a no-op; reuse with different semantics is a conflict.
- **Integration outbox:** stable message ID, aggregate/resource identity, monotonically assigned local sequence, delivery state, attempt/lease metadata, redacted failure classification, and acknowledgement time.
- **Evidence references:** opaque identifier, digest, classification, provenance and verification status kept separate from the case's workflow state. A digest alone does not establish truth or authorization.

Use typed relational columns and constraints for identities, lifecycle, authority relationships, ordering and idempotency. Use JSONB only where variable provider payload or versioned extension data justifies it; do not make the normalized domain model an unvalidated JSON document.

## Transaction and concurrency rules

A receiver's accept operation must follow one database transaction:

1. Establish and validate trusted connection and tenant context.
2. Validate the event profile and normalized payload; resolve an explicit external-to-local mapping.
3. Claim (authenticated source binding, CloudEvents source, event id) under a unique constraint. Equal replay is a duplicate; changed content is a conflict.
4. Claim (connection id, canonical tenant id, idempotency key) under a unique constraint. Equal semantic effect is a duplicate; a different effect is a conflict.
5. Check the provider revision using a provider-specific documented comparator. Do not sort opaque revisions lexically or use arrival time as a substitute.
6. A stale revision or an equal revision with identical state still consumes its idempotency key, but creates no state mutation or outbox event. A contradictory equal revision or an unorderable revision is quarantined without claiming the key.
7. Apply any accepted case/resource change and append activity.
8. Insert resulting outbox rows.
9. Commit, then acknowledge receipt to the source.

All local writes in that sequence commit or roll back together. Unknown mappings, invalid bindings, conflicting identities, and unorderable/stale revisions must have explicit outcomes and must not mutate business state or create an outgoing business event.

Outbox dispatch should lease rows transactionally and be recoverable after worker termination. SKIP LOCKED can help multiple workers claim independent rows, but it must not bypass a predecessor when the product requires per-case ordering. A permanently failing predecessor requires visible retry/dead-letter policy and a reviewed replay path; later events must not silently leapfrog it.

## Required qualification before production use

The PostgreSQL implementation is **not qualified** until the following evidence is available for the exact source revision and migration set:

- [ ] PostgreSQL integration tests run against the pinned major/minor image using the same database role and connection-pool behavior as the service.
- [ ] Concurrent duplicate event and business-idempotency races are exercised from independent connections and, where feasible, independent processes. Exactly one business effect and one corresponding outbox effect survive.
- [ ] Cross-tenant negative tests cover reads, list/search, writes, transition commands, attachments, evidence references, exports, event subscriptions, and mapping resolution.
- [ ] RLS tests run as the actual application role, verify missing tenant context fails closed, verify connection-pool reuse cannot leak tenant context, and verify the role cannot bypass RLS or own protected tables.
- [ ] Process-kill tests cover before commit, after commit/before source acknowledgement, and external send/before acknowledgement. Recovery has no lost committed event and no duplicated local business effect.
- [ ] Outbox concurrency tests cover lease expiry, stale acknowledgements, retries, two workers, and same-case ordering.
- [ ] Migration tests cover fresh install, upgrade from the previous schema, interrupted upgrade recovery, and reconciliation/count checks.
- [ ] Backup/restore and disk-full/failure-path tests are documented for the supported deployment profile.
- [ ] CI reports the exact tested commit, applied migration version, database version, discovered test count, and test result. Empty job discovery, missing checks, skipped jobs, or queued runs are not a pass.
- [ ] The qualification report explicitly excludes live provider authentication, provider-specific rate limits/revisions, independently verified evidence signatures, and remote execution unless those are separately tested.

## Deployment and operational consequences

PostgreSQL adds an operational service that customers or a managed deployment must monitor, patch, back up, and restore. A sovereign deployment is incomplete without an installation path, secret rotation, health checks, schema-upgrade procedure, backup verification, restore rehearsal, export, and migration documentation. The platform must make those operations understandable rather than describing self-hosting as a checkbox.

Initially avoid multi-region writes, database sharding, and a generic multi-database abstraction. First measure case/inbox/outbox volume, index behavior, lock contention, retention needs, and restore time on a representative synthetic corpus. Scale the design in response to measurements, not speculative breadth.

## References

- [PostgreSQL 18 — Row Security Policies](https://www.postgresql.org/docs/18/ddl-rowsecurity.html): row policies restrict row visibility and mutation, default-deny applies when RLS is enabled without an applicable policy, and table owners/superusers/BYPASSRLS roles require special treatment.
- [PostgreSQL 18 — Transaction isolation](https://www.postgresql.org/docs/18/transaction-iso.html): define and test the actual transaction-isolation assumptions made by the receiver and dispatcher.
- [CloudEvents 1.0 JSON event format](https://github.com/cloudevents/spec/blob/main/cloudevents/formats/json-format.md): the event identity envelope is interoperable metadata, not a trust or authorization proof.
- [Luminous Platform issue #5](https://github.com/Luminous-Dynamics/luminous-platform/issues/5): tenant-scoped durable inbox/outbox qualification.
- [Luminous Platform issue #7](https://github.com/Luminous-Dynamics/luminous-platform/issues/7): native work-case contract and lifecycle acceptance criteria.

## Rust/PostgreSQL prototype increment (2026-10-09)

The Operations Event V1 Rust crate now has an initial PostgreSQL receiver prototype and a versioned migration under `contracts/operations-event-v1/rust/`. Its intended transaction applies a source event only after schema validation, an authenticated-connector binding supplied by the service boundary, an explicit external-resource mapping, and a configured revision comparator are resolved. It uses database unique keys for event identity and business-effect idempotency; a locked per-incident head serializes revision decisions and local outbox sequence allocation. Inbox identity, effect key, incident state, append-only activity, and outbox row are intended to commit or roll back together.

A separate cluster-administrator provisioning script creates the non-login `luminous_ops_app` role with no superuser or RLS-bypass attribute. The transactional schema migration assumes that role already exists, then grants explicit table privileges and installs forced tenant RLS policies. Runtime login roles must be provisioned separately and granted membership in the app role; schema migrations run under a separate owner/migration identity and do not need CREATEROLE. The tenant context is transaction-local. RLS is a defense-in-depth barrier against scope mistakes, not protection against compromise of an application role that can set arbitrary tenant context.

The prototype deliberately quarantines revisions it cannot order and supports a numeric-suffix comparator only when explicitly configured for a provider whose contract guarantees that ordering. It stores digests and stable reason codes for quarantine rather than raw rejected events. Outbox claims use row locks and block later events for an incident while an earlier event remains undelivered; sends are at-least-once, and lease acknowledgements require the current owner and an unexpired lease.

**Qualification boundary:** this prototype is not production-qualified. The authored PostgreSQL integration suite covers replay/idempotency, concurrent same-key conflicting effects under different event IDs, revision ordering, database-trigger fault rollback, RLS default deny, lease recovery, and a child-process exit immediately after commit/before source acknowledgement followed by redelivery. The column-level grants limit insert and update privileges to fields required by the Rust receiver/dispatcher. These are authored checks only: the suite must compile and pass on the exact PR head against the pinned PostgreSQL CI image before any of those checks count as observed evidence. The one subprocess case does not cover termination before commit or an outbox send succeeding before acknowledgement; no live ConnectWise authentication, production principal resolver, migrations-from-prior-version test, power-loss test, backup/restore rehearsal, or independent-process multi-instance deployment qualification is implied.

## Related limitations

This ADR chooses a target for the first shared production service. It does not itself implement migrations, the PostgreSQL repository, RLS policies, authenticated provider intake, a hosted ConnectWise adapter, or a qualified service. The existing SQLite model remains useful evidence only for the behavior that its own tests execute.
