# Rust public-source contract validator

This crate validates source catalogue documents and observations offline. It performs no network requests, has no service listener, and does not enable ingestion. It uses strict Serde structures and domain-specific checks; it is not a full JSON Schema Draft 2020-12 engine.

## Run

From the repository root:

    cargo test --manifest-path contracts/public-source-catalogue-v1/rust/Cargo.toml --all-targets
    cargo run --manifest-path contracts/public-source-catalogue-v1/rust/Cargo.toml --bin validate-public-source -- catalogue contracts/public-source-catalogue-v1/examples/catalogue.json
    cargo run --manifest-path contracts/public-source-catalogue-v1/rust/Cargo.toml --bin validate-public-source -- observation contracts/public-source-catalogue-v1/examples/observation-v1.json

## Enforced invariants
- Unknown object fields are rejected.
- Catalogue schema version, ISO review date, source ID syntax and uniqueness, absolute HTTPS URLs, non-empty metadata, distinct access methods and disabled ingestion are checked.
- Observations require a timestamp with explicit timezone, HTTPS source URL, explicit raw-value state and coherent normalization metadata.
- Unnormalised observations preserve raw values exactly.
- Applied normalization must identify a non-empty rule ID and version; it is forbidden for omitted or not-retrieved rows.
- Non-fixture observations require a SHA-256 snapshot digest.
- The crate does not authenticate publishers, establish endpoint permission, validate all JSON Schema keywords, verify digests against downloaded bytes, or prove source data correctness.

## Language boundary
All implementation logic here is Rust. Do not add handwritten JavaScript/TypeScript or Node.js to this contract. See docs/LANGUAGE_AND_RUNTIME_POLICY.md.

## Qualification status
Tests are authored but no passing exact-head CI result is claimed. The GitHub Actions runner assignment issue must be resolved or an approved controlled local qualification method must provide reproducible logs bound to the exact commit.
