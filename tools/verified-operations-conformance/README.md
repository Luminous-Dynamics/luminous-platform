# Verified Operations conformance runner

This crate validates the synthetic-only pilot corpus in `profiles/verified-operations/`.

## Run

From the repository root with Rust 1.99.0 installed:

```sh
cargo test --manifest-path tools/verified-operations-conformance/Cargo.toml
cargo run --manifest-path tools/verified-operations-conformance/Cargo.toml -- \
  profiles/verified-operations/pilot-v1.schema.json \
  profiles/verified-operations/synthetic-pilot-v1.json
```

The runner checks the Draft 2020-12 schema, validates the 28-case corpus, enforces unique scenario IDs and fail-closed metadata invariants, and independently recomputes expected classifications and reason codes. The test file currently contains 22 distinct Rust test functions. Outcome coverage is computed from the declared UTC epoch window, sampling interval and valid sample offsets—not a caller-supplied ratio. The runner checks boundary completeness, cadence alignment, duplicate/out-of-range offsets, calculated sample denominator, minimum derived coverage, maximum-gap enforcement, compatible units, independent-measurement presence and numeric target comparison. It enables format assertions and refuses external schema reference retrieval.

## Claim limits

- The facts and scenarios are synthetic. No customer systems or provider API is called.
- A classifier/test passing proves only that the authored contract and implementation agree. It does not prove a source observation true, validate a real adapter, or qualify an actuator or privileged operation.
- The corpus prohibits automatic mutation in all scenarios. It is not an authorization API and must never be imported as a runtime command policy.
- The direct Rust dependencies are exact-version pinned. The synthetic sample offsets establish only schedule coverage in this contract; production evidence must bind actual values, source identities, timestamps, unit semantics and verifier results. A committed `Cargo.lock` is still required before treating transitive dependency resolution as reproducible or promoting this tool beyond contract-test use.
- CI must pass against the exact source head. Queued, skipped, absent, or older-head runs are not passes.
