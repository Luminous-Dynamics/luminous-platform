# Agentic Endpoint Assurance Model

Dependency-free Rust reference model for [Agentic Endpoint Security Benchmark v1](../../docs/AGENTIC_ENDPOINT_SECURITY_BENCHMARK_V1.md).

## Run locally

```sh
python3 tools/agentic-endpoint-security-v1/check_corpus.py
cargo fmt --manifest-path tools/agentic-endpoint-security-v1/Cargo.toml -- --check
cargo test --locked --manifest-path tools/agentic-endpoint-security-v1/Cargo.toml
cargo run --locked --manifest-path tools/agentic-endpoint-security-v1/Cargo.toml
```

This crate intentionally has no third-party dependencies. The evaluator accepts normalized model facts and is **not** a cryptographic verifier, operating-system observer, policy enforcement component, production authorization service, or security proof. Synthetic example output must not be counted as benchmark execution evidence.

The standard-library Python checker is test tooling only: it freezes the 20 corpus IDs/outcomes and rejects weakened lab-safety/scoring constraints. The PR workflow uses pinned action commits, a pinned Rust toolchain, offline Cargo test/build steps, and read-only repository permissions. It is not part of an endpoint runtime. The Rust unit suite targets contract-level negative controls: denied actions with effects, forbidden effects, missing telemetry, same-trust-domain observations, unknown fields, stale state and missing required rollback witnesses. Runtime prevention still requires the benchmark's T1+ lab adapter and an independent effect observer.
