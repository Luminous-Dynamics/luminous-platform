# Local-first outage resilience reference model

This crate is a bounded, in-memory metadata reference model for queue admission, expiry, quotas, route eligibility, deterministic batch selection and acknowledgement state. It is deliberately not a production transport or durable queue.

## Scope and safety boundaries

- It stores message metadata only, never payload bytes.
- It does not implement BPv7, BPSec, end-to-end cryptography, signatures, sockets, persistence, a mesh protocol or a live router adapter.
- Fields describing payload protection and adapter-verified acknowledgements are trusted caller assertions. They are not cryptographic proof. A production adapter must verify actual ciphertext/integrity and authenticate acknowledgements before passing those assertions.
- Default delivery semantics are at-least-once. Stable message identifiers and receiver-side durable idempotency are required to prevent duplicate application effects. This crate does not claim exactly-once transport or business effects.
- Expired messages are never newly selected for forwarding. Terminal dedupe history is retained for a declared interval and pruned to a bounded history.
- Route selection is metadata-only and requires the caller to pass authorized, available, security-profile-compatible route eligibility. No automatic relay discovery or provider permissions are inferred.
- An in-memory passing test is not evidence of durable writes, crash consistency, field delivery, power safety, security audit or deployment authorization.

## State model

Admission validates identifier/digest shape, payload size, destination allowlist, creation-time skew, expiry/TTL, sensitive-message protection declaration, total and sender quotas, and bounded dedupe history. Identical duplicate IDs are idempotent while the original record remains in retention; same ID with changed metadata is rejected. Batch selection sorts essential before ordinary before bulk, then earliest expiry, creation time and stable ID. It respects per-batch byte and item budgets but does not claim a starvation-free scheduling policy.

A verified acknowledgement is supplied by a trusted adapter and is checked against the message ID, destination, payload digest and accepted timestamp window. The verifier here checks matching metadata only, not the acknowledgement signature itself.

## Run

From the repository root with Rust installed:

    cargo test --manifest-path tools/offline-delivery-contract-core/Cargo.toml --all-targets
    cargo clippy --manifest-path tools/offline-delivery-contract-core/Cargo.toml --all-targets -- -D warnings

The associated profile validator is a separate crate. Its target contracts are documented in the parent RFC and profile JSON.
