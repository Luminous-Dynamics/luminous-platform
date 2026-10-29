# Agentic Endpoint Assurance Model

Dependency-free Rust reference model for [Agentic Endpoint Security Benchmark v1](../../docs/AGENTIC_ENDPOINT_SECURITY_BENCHMARK_V1.md).

## Run locally

```sh
python3 tools/agentic-endpoint-security-v1/check_corpus.py
python3 tools/agentic-endpoint-security-v1/check_adaptive_protocol.py
python3 tools/agentic-endpoint-security-v1/test_adaptive_protocol.py
cargo fmt --manifest-path tools/agentic-endpoint-security-v1/Cargo.toml -- --check
cargo test --locked --manifest-path tools/agentic-endpoint-security-v1/Cargo.toml
cargo run --locked --manifest-path tools/agentic-endpoint-security-v1/Cargo.toml
```

This crate intentionally has no third-party dependencies. The evaluator accepts normalized model facts and is **not** a cryptographic verifier, operating-system observer, policy enforcement component, production authorization service, or security proof. Synthetic example output must not be counted as benchmark execution evidence.

The standard-library Python checkers are test tooling only: one freezes the 20 fixed-corpus IDs/outcomes and lab constraints; the other freezes adaptive attack families, query budgets, comparison arms, sample-size rules, and anti-gaming constraints. Mutation tests ensure key adaptive protocol requirements cannot silently be weakened. The PR workflow uses pinned action commits, Rust 1.96.0, offline Cargo test/build steps, and read-only repository permissions. It is not part of an endpoint runtime. The Rust suite contains 14 tests, including binding-field mutations and an exhaustive 29,160-state contract matrix. The model compares exact tuples but does not authenticate their source, consume nonces durably, observe OS effects, or prevent actions. Runtime prevention still requires the benchmark's T1+ lab adapter and an independent effect observer.
