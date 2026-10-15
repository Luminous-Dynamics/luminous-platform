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

The runner checks the Draft 2020-12 scenario schema and all 28 synthetic decision cases. The same Rust crate also validates a portable evidence bundle: SHA-256 over exact UTF-8 source payloads; a versioned digest over normalized measurement fields; unique evidence IDs and source-event identities; a versioned, allow-listed Home Assistant energy normalizer that binds payload `entity_id` to the declared `resource_id`, and raw `state`/`unit_of_measurement` to normalized fields; power units such as W are rejected for the energy-counter metric; only telemetry-sample and derived-measurement record kinds are supported; resolved forward-only lineage; exact-decimal subtraction for the supported derived-measurement fixture; resolved decision references; matching measurement units; and decision/classification consistency. Outcome coverage still derives from the declared UTC window and sample offsets rather than caller assertions. The test file now contains 31 distinct Rust test functions. Format assertions are enabled and external schema retrieval is disabled.

## Portable evidence receipt

Run this command to validate the evidence-bundle fixture as well as the scenario corpus:

    cargo run --manifest-path tools/verified-operations-conformance/Cargo.toml -- profiles/verified-operations/evidence-bundle-v1.schema.json profiles/verified-operations/synthetic-evidence-bundle-v1.json

The bundle links an objective decision to source records and a derived measurement. The normalized SHA-256 digest uses the exact UTF-8 framing `luminous-operations-measurement-v1\nmetric_name\nvalue_decimal\nunit\nvalidity`; decimal values are strings so JSON-number reformatting cannot change this digest. Raw payload and normalized-field digests detect content mismatch. The conformance slice accepts `subtract` as the only derived-measurement operation and checks arithmetic with exact decimal scaling; it is deliberately narrow rather than a general expression engine.

The v1 profile deliberately rejects a production mode and only accepts the reviewed normalization profile for each record kind; only synthetic and redacted-pilot bundles are accepted until authenticated source and signer contracts are implemented. Digest checks do **not** authenticate the source, adapter, collector, time source or author. An attacker able to replace both content and its unsigned digest can forge a self-consistent bundle. Production qualification still needs authenticated source identities, signature or challenge-verification policy, key revocation, clock-quality bounds, adapter-specific raw-to-normalized conformance, secure retention/redaction and a reproducible lockfile. The fixture carries no secrets and executes no action.

## Claim limits

- The facts and scenarios are synthetic. No customer systems or provider API is called.
- A classifier/test passing proves only that the authored contract and implementation agree. It does not prove a source observation true, validate a real adapter, or qualify an actuator or privileged operation.
- The corpus prohibits automatic mutation in all scenarios. It is not an authorization API and must never be imported as a runtime command policy.
- The direct Rust dependencies are exact-version pinned. The synthetic sample offsets establish only schedule coverage in this contract; production evidence must bind actual values, source identities, timestamps, unit semantics and verifier results. A committed `Cargo.lock` is still required before treating transitive dependency resolution as reproducible or promoting this tool beyond contract-test use.
- CI must pass against the exact source head. Queued, skipped, absent, or older-head runs are not passes.
