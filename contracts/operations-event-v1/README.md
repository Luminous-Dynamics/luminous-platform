# Operations Event Contract V1

## Implementation-language boundary

**Production services and domain logic are Rust-first; production Python is not permitted.** The Rust crate under `rust/` is the canonical Operations Event V1 schema validator and trusted-context binding checker. Python files in this directory remain research-only reference models and test harnesses; they are not deployable adapters, daemons, API services, authorization boundaries, or durable repositories. They must not be imported or packaged by production binaries, and they do not qualify the Rust implementation or production deployment.

This folder defines the first portable cross-repository event contract for the proposed Xenia Operations Fabric. The envelope uses the **CloudEvents 1.0 structured JSON format** instead of introducing a proprietary envelope.

Automatic CI runs on relevant pull-request changes and relevant pushes to `main`; `workflow_dispatch` remains available for a standalone branch. This avoids executing identical Rust/PostgreSQL suites twice on every commit to a branch that already has a PR.

## Files

- `schema.json` — JSON Schema Draft 2020-12 for the strict Luminous CloudEvents profile.
- `payloads/incident-snapshot-v1.schema.json` — first vendor-neutral normalized incident payload schema.
- `examples/incident.snapshot.json` — synthetic, non-production CloudEvent example.
- `adapter_model.py` — deterministic normalization and in-memory inbox model for synthetic PSA records; not a live API client or durable storage.
- `durable_sqlite_model.py` — a separate SQLite-backed reference harness for transactional local state, deduplication, revision handling, per-incident ordered outbox leases, and fault injection; not production service code.
- `tests/test_contract.py` — envelope and incident-payload schema tests.
- `tests/test_adapter_model.py` — deterministic normalization, trusted tenant/source binding, duplicate-delivery and idempotency-conflict tests.
- `tests/test_durable_sqlite_model.py` — restart/replay, conflict, transaction fault-injection, concurrency, quarantine, stale-revision, and outbox lease tests.
- `DURABLE_SQLITE_REFERENCE_MODEL.md` — scope, semantics, limitations, and production continuation gates.
- `rust/` — canonical Rust-native envelope/payload validator, trusted connector-binding checks, PostgreSQL transactional receiver, versioned PostgreSQL migrations, and adversarial/integration tests.
- `requirements.txt` — Python validator pinned for research-only reference-model tests; not a production dependency.

Deployment order matters: a cluster administrator provisions the non-login RLS application role from `rust/deploy/postgres/00-operations-app-role.sql`; the separate schema-migration identity applies the SQLx migration; then a runtime login role is explicitly granted membership in `luminous_ops_app`. Do not run application traffic as a superuser or table owner.

Primary contract qualification (Rust + PostgreSQL 18.6): start an isolated PostgreSQL instance and set `DATABASE_URL` to an administrator URL and `APP_DATABASE_URL` to a login URL for the test-only runtime role (created by the integration test), then run:

```sh
rustup toolchain install 1.96.0 --profile minimal
cargo +1.96.0 test --manifest-path contracts/operations-event-v1/rust/Cargo.toml --all-targets
```

The PostgreSQL integration test exercises unique event identity, replay with a different receiver-local observation timestamp, digest-only quarantine of conflicting event identities and idempotency keys, quarantine outcome replay, explicit source-to-local mapping enforcement by database foreign keys, validation of the initial revision token before state writes, idempotency conflict precedence over malformed revision errors, stale-revision idempotency-key reservation and replay, numeric revision ordering, fail-closed opaque revisions under a connection explicitly configured for opaque ordering; a numeric-suffix policy rejects nonconforming initial revisions before mutation, concurrent duplicate submissions and concurrent different-semantics reuse of one business idempotency key under separate event IDs (exactly one effect, one conflict and one quarantine), all-or-nothing rollback on a database trigger failure, default-deny tenant RLS, transaction-local tenant context, outbox predecessor ordering, lease expiration, acknowledgement ownership, least-privilege column grants, and a subprocess-exit probe that terminates immediately after the receiver transaction commits but before the parent caller can acknowledge receipt. The parent then redelivers the same event and checks for a single durable business/outbox effect. This probe is authored but not yet observed passing. The CI service uses the PostgreSQL 18.6 multi-platform image manifest digest `sha256:3725f4e2499eef5134592b3b4ab79a543ed7f8e533b05b5b637af926630f6650`.

Optional reference-model research tests (Python; never a production runtime):

```sh
python3 -m venv /tmp/operations-reference-venv
/tmp/operations-reference-venv/bin/python -m pip install --no-cache-dir -r contracts/operations-event-v1/requirements.txt
PYTHONDONTWRITEBYTECODE=1 /tmp/operations-reference-venv/bin/python scripts/run_unittest_suite.py contracts/operations-event-v1/tests --minimum-tests 1
```

## Profile choices

The envelope uses CloudEvents core attributes `specversion`, `id`, `source`, `type`, `subject`, `time`, `datacontenttype`, and `dataschema`, plus a small allowlist of lowercase extension attributes: `tenantid`, `idempotencykey`, `observedat`, `correlationid`, `causationref`, `producersystem`, `producercomponent`, `producerversion`, `actorkind`, `actorid`, and `authorityref`.

The strict local profile rejects unknown extension attributes so the accepted metadata surface is explicit. This is stricter than general CloudEvents extensibility; external events must first be parsed by their adapter and normalized into this profile. Use the `application/cloudevents+json` media type for structured CloudEvents JSON.

## Rust validator boundary

The Rust crate embeds the checked-in Draft 2020-12 schemas, enables format assertions, disables network schema retrieval, and accepts only the exact registered `dataschema` URI. It emits stable rule codes and JSON Pointers without rejected values. `validate_event_for_connection` also compares source URI, producer, actor/connection identity, company-to-tenant mapping, and company/resource-to-local-incident mapping with a caller-supplied trusted context. That context must originate from authenticated connector configuration; this crate does not authenticate connections or itself provide durable idempotency.

The CI workflow qualifies the Rust contract separately from the Python reference-model research job. Passing the latter does not qualify the Rust crate or any production system.

## Conflict and quarantine replay behavior

Conflicting content under an already accepted event identity records a digest-only quarantine receipt without changing the original inbox outcome. Reusing a business idempotency key with different semantics keeps the new inbox event quarantined and returns an explicit conflict. Re-delivery of that quarantined event returns the stored quarantine reason keyed to the original event digest, even if later conflicting payloads reuse its event ID, instead of a generic duplicate result. Quarantine uniqueness includes the rejected-content digest, so identical repeated attempts collapse to one receipt while distinct conflicting contents remain distinguishable without storing raw payloads.

## Important limits

Schema-valid does **not** mean trusted, authorized, current, correctly sequenced, or safe to execute. The envelope schema validates metadata shape; `dataschema` identifies a specific domain payload schema. Neither schema is a policy engine, identity provider, proof verifier, or cross-tenant authorization mechanism.

Consumers MUST additionally:

1. Authenticate the source using the adapter's configured trust mechanism; never trust the `source` URI or `producersystem` label by itself.
2. Bind the canonical local tenant from authenticated connection context. Reject attempts to cross that connection's tenant scope.
3. Validate `data` using the exact `dataschema` before interpreting fields.
4. Enforce event de-duplication using CloudEvents' `source + id` identity and enforce durable business-effect idempotency using authenticated connection + tenant + `idempotencykey`. Reject conflicting content for a reused idempotency key.
5. Validate causal references and resource revisions where required; do not use `time`, `observedat`, or lexical ID order as authorization or globally trusted sequencing.
6. Persist business mutations and outgoing events through a transactional outbox or equivalent recovery-safe mechanism.
7. Re-check authorization at the privileged mutation boundary. An event, valid signature, ticket, AI recommendation, or `authorityref` string cannot authorize execution alone.
8. Keep credentials, access tokens, private keys, session keys, raw screen/audio payloads, and unredacted diagnostic outputs out of event payloads. Use classified, access-controlled evidence references.
9. Surface duplicate, delayed, invalid, conflicting, and dead-lettered messages to operators. Never turn sync failure into silent success.

## Synthetic adapter reference model

`adapter_model.py` accepts a vendor-neutral `PsaIncidentSnapshot` only after a real connector would have authenticated its source and parsed its product-specific payload. The caller supplies `TrustedPsaConnection` configuration with explicit external-company-to-tenant and external-ticket-to-local-incident mappings. The normalized event type is `io.luminousdynamics.ops.incident.snapshot.v1`, because this reference adapter emits source snapshots rather than asserting every revision is a creation event. The normalized source reference preserves both the external company ID and provider-adapter revision token so reconciliation does not have to infer company scope from a ticket ID. The receiver-side `IngestionBoundary` independently rechecks company→tenant and company+ticket→local-incident mappings before accepting the event; changing only `tenantid` or `subject` is rejected. Missing/ambiguous mappings fail closed; the source URI, actor identity and tenant are derived from trusted configuration rather than ticket fields. The revision token is opaque and must not be treated as lexically ordered unless the provider-specific contract guarantees ordering.

`MemoryInbox` is an executable in-memory state-machine model only. The receiver revalidates the adapter system, supported source resource type, company-to-tenant mapping, ticket-to-local-incident mapping, source URI, actor identity and tenant before mutating its in-memory state. It covers:
- exact `source + id` replay, even when the receiver-local `observedat` value differs on a later delivery;
- conflicting payload under an already-used event identity;
- same business idempotency key with changed semantics;
- same business effect received under a different delivery ID;
- tenant/source/actor binding failures;
- source-system and resource-type mismatch;
- malformed runtime source and summary values rejected before mapping/string operations;
- forged tenant/resource pairings and unmapped source-company rejection; and
- idempotency-key scope separation across tenants.

The `MemoryInbox` does not survive process restarts or implement transaction/outbox semantics. The separate `durable_sqlite_model.py` demonstrates these properties only for its tested local SQLite model; it is not a production database design and does not establish tenant isolation or crash guarantees for another database or deployment.

## ConnectWise adapter notes

ConnectWise's public developer documentation describes product-specific onboarding and gated API access. Its Cloud Services authentication guide documents OAuth 2.0 client credentials and warns to treat subscription keys as secrets and cache bearer tokens rather than requesting a fresh token before every call. Do not assume that flow applies to every ConnectWise product/API: confirm the selected PSA API's actual endpoint and authentication guide during adapter implementation.

Official references:

- [CloudEvents specification](https://cloudevents.io/)
- [CloudEvents JSON Event Format](https://github.com/cloudevents/spec/blob/main/cloudevents/formats/json-format.md)
- [ConnectWise Developer Network — Getting Started](https://developer.connectwise.com/Best_Practices/Getting_Started)
- [ConnectWise Cloud Services — Authentication](https://developers.cloudservices.connectwise.com/Guides/Authentication)
- [CISA — Guide to Securing Remote Access Software](https://www.cisa.gov/resources-tools/resources/guide-securing-remote-access-software)

This version intentionally defines no command-execution event. Start with incident lifecycle and evidence-reference events; privileged operation schemas require a separate authority and risk review.
