# Holochain Attestation Share Package V1

**Status:** synthetic contract for design/qualification; not a production hApp or adapter.

This package is the deliberately narrow public payload for the first proposed Mycelix/Holochain integration. It supports cross-organization incident acknowledgement, evidence-manifest reference, dispute, and supersession records.

## Files

- `share-package.schema.json`: JSON Schema Draft 2020-12 for the share package.
- `rust/`: Rust-native schema and semantic validator, compiled with JSON Schema format assertions enabled and network reference retrieval disabled.
- `examples/evidence-attestation.synthetic.json`: deterministic synthetic fixture with opaque references.
- `rust/tests/contract.rs`: Rust positive and adversarial tests for schema and semantic invariants.

## Privacy and trust boundary

The payload intentionally excludes free-form text, ticket titles, customer names, hostnames, external provider IDs, credentials, raw diagnostics, screen/audio, and raw evidence. Use the Rust crate under `rust/` for application-side schema and cross-field checks; bare JSON Schema validation alone cannot enforce uniqueness of `manifestRef` across objects that differ in other fields. The semantic layer normalizes UUIDs before rejecting self-referential disputes/supersessions, and diagnostics expose only a stable rule code and JSON Pointer—not rejected values. The local PostgreSQL service must resolve opaque references only under authenticated tenant/connection context.

The `issuerRoleClaim`, `issuedAt`, `assessmentClaim`, `approvedForDht`, and `policyRef` fields are claims or references—not proof that a participant is entitled to publish a record. The publishing adapter must enforce actual authenticated identity, tenant/customer sharing policy, and the approval requirement before invoking Holochain. The integrity zome should validate structural/domain invariants and any deterministic trust dependencies available within the DNA.

The `authorityEffect` field is fixed to `none`. Nothing in this record authorizes remote access, host changes, provider write-back, billing, or settlement. Xenia, Nixward, and the service-management backend must apply their own current authorization rules at the action boundary.

A digest is an integrity commitment, not proof that the underlying evidence is authentic or true, and not a confidentiality mechanism. Public or partner-visible DHT metadata may reveal relationships, timing, stable references, and record size. Use random opaque identifiers, minimize the record, and review metadata leakage before publishing any real customer data.

## Intended publication lifecycle

1. Commit the canonical work case and publication intent to the PostgreSQL state/outbox transaction.
2. Construct and validate a share package only from explicitly approved fields.
3. Authenticate and authorize the publisher; check the actual privacy/share policy independently of payload claims.
4. Publish asynchronously to the selected, version-pinned Mycelix/Holochain hApp using a stable publication identity.
5. Reconcile the result as pending, observed, rejected, conflicted, or retry-required. A successful zome call or a queued outbox item must not be shown as independently verified.
6. Store the Holochain action/entry reference and the local reconciliation result as separate provenance from the underlying technical outcome.

Do not attempt a distributed transaction between PostgreSQL and Holochain. Retried publications must be idempotent at the adapter/domain layer; DHT visibility and validation may be delayed or unresolved.

## Test

From the repository root:

```sh
cargo +1.96.0 test --manifest-path contracts/holochain-attestation-v1/rust/Cargo.toml --all-targets
```

The Rust contract suite defines 20 positive/adversarial test cases covering schema shape, conditional record semantics, case-insensitive UUID self-reference, duplicate manifest IDs, malformed input, and error-value redaction. CI fails closed if fewer than 20 Rust tests execute. It does **not** test Holochain zome validation, multi-agent DHT propagation, identity/membership binding, privacy of network metadata, PostgreSQL/outbox recovery, or production readiness. Those require a selected hApp and a compatible pinned conductor/toolchain.

Python remains in the separate Operations Event synthetic/reference-model tests only; the Holochain share-package runtime validator and its contract tests are Rust. No Python interpreter or Python package is required to consume this Rust crate.
