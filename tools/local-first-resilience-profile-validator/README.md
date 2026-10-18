# Local-first resilience profile validator

Offline, fail-closed validation for the local-first outage resilience benchmark profile. It runs JSON Schema Draft 2020-12 validation and locks the profile identity, all 14 benchmark operator/target/status tuples, deployment gates, comparison guards, mandatory non-claims and required follow-up artifacts.

## Run

    cargo test --manifest-path tools/local-first-resilience-profile-validator/Cargo.toml --all-targets
    cargo clippy --manifest-path tools/local-first-resilience-profile-validator/Cargo.toml --all-targets -- -D warnings
    cargo run --manifest-path tools/local-first-resilience-profile-validator/Cargo.toml -- schemas/local-first-outage-resilience-benchmark-profile-v1.schema.json profiles/local-first-outage-resilience-benchmark-profile-v1.json

## Evidence and non-claims

The validator has no network retrieval: external schema references are rejected before constructing a validator. It emits digests of the exact schema/profile bytes. Passing means only that draft structure and policy invariants are satisfied. It does not prove message delivery, cryptography, persistence, hardware operation, field performance, relay permission, spectrum rights or regulatory authorization.

All target values remain proposed hypotheses or baseline-first. The separate offline-delivery-contract-core crate is a metadata-only reference model, not a durable queue, cryptographic verifier, BPv7 implementation or production transport.
