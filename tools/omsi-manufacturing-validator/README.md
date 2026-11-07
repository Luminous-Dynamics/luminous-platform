# OMSI Manufacturing Semantic Validator

A standalone Rust crate for the synthetic OMSI Manufacturing Asset Operations 0.1 incubation profile.

## Scope

The validator performs bounded semantic checks for a supplied profile input and runs the machine-readable expectations in the committed conformance-vector document. Its versioned JSON output includes a rule-bound decision trace: check ID, result state, input field references, explanation, and any resulting disposition. The trace makes the interpretation reviewable; it is not a signature or a proof that the upstream facts are true. The output contract is described in [semantic-report.schema.json](schema/semantic-report.schema.json). It deliberately does not:

- authenticate the issuer or origin of the trusted tenant binding;
- verify source-system identity, digital signatures, calibration certificates, or measured physical truth;
- infer an atomic snapshot, source freshness, event history, or a complete provenance chain;
- convert units or apply uncertainty to the nominal comparison;
- authorize quality release, machine control, or any other consequential effect.

The comparison rule is exactly `nominal-point-estimate-range/0.1`: use checked decimal strings and inclusive source-provided bounds only when the measurement and specification unit system/code match exactly. The value `not_evaluable` is returned for missing or incompatible units. This is a diagnostic comparison, not a quality disposition.

The validator assumes that any tenant binding supplied to it has already been selected by a trusted caller. It does enforce that the asset's declared tenant matches that binding and that linked source references remain inside the declared source-instance namespace. Passing this validator does not authenticate those declarations.

## Run from the repository root

```sh
cargo fmt --manifest-path tools/omsi-manufacturing-validator/Cargo.toml --all -- --check
cargo test --manifest-path tools/omsi-manufacturing-validator/Cargo.toml
cargo run --manifest-path tools/omsi-manufacturing-validator/Cargo.toml -- validate-vectors docs/standards/omsi/profiles/manufacturing-asset-operations/0.1/CONFORMANCE-VECTORS.json
```

The vector command exits 0 only when all machine-readable expected outcomes match the computed semantic results. It checks expected disposition types/reasons, the comparison rule ID, and optional measurement/scope/revision fields. The human-readable `assertions` strings are documentation and are not interpreted as executable tests.

Evaluate one input object (the content of a vector's `input`) by writing it to a JSON file or piping it through stdin:

```sh
cargo run --manifest-path tools/omsi-manufacturing-validator/Cargo.toml -- evaluate-input input.json
cat input.json | cargo run --manifest-path tools/omsi-manufacturing-validator/Cargo.toml -- evaluate-input -
```

A rejected or conflicted input is emitted as JSON and results in exit code 1 for `evaluate-input`. Clean projections and projections with explicitly disclosed loss return exit code 0. Invalid JSON, filesystem errors, or CLI usage errors return exit code 2.

## Raw-byte replay envelope

Create an unsigned envelope that binds the exact bytes of the input artifact to the validator's report:

```sh
cargo run --manifest-path tools/omsi-manufacturing-validator/Cargo.toml -- create-envelope input.json > envelope.json
cargo run --manifest-path tools/omsi-manufacturing-validator/Cargo.toml -- verify-envelope envelope.json input.json
```

The envelope schema is [replay-envelope.schema.json](schema/replay-envelope.schema.json). It records the SHA-256 digest, byte length, and explicit digest scope `exact_input_bytes`, with `canonicalization=none`. Therefore even whitespace changes create a different input digest. This is intentional: the bound identity is the exact supplied artifact, not an implied canonical JSON object.

`verify-envelope` recomputes the digest from the supplied bytes, parses that input, re-runs the semantic evaluator, and compares its JSON result with the embedded report. It returns `status=replay_consistent` only when those checks and the envelope contract match; a mismatch exits 1. Invalid envelope JSON or file/CLI errors exit 2.

**Security boundary:** the envelope is unsigned. It can detect a changed input or altered report when compared with a preserved envelope, but an attacker able to replace both the artifact and envelope can recompute the digest. The digest does not authenticate a source, signer, calibration certificate, or physical measurement. Replay consistency shows agreement with the validator implementation that ran; it is not proof of source truth, cross-implementation equivalence, quality conformity, product release, or authorization. No RFC 8785 JSON canonicalization or cryptographic signature is claimed.

## Decimal and timestamp profile

- Decimal values are strings, not JSON floating-point numbers. The parser rejects exponent notation, leading zeroes, leading plus signs, and precision outside its bounded profile (up to 18 integer digits and 18 fractional digits).
- Comparisons rescale checked decimal coefficients and do not use binary floating point.
- Timestamps require a full date and seconds, uppercase `T`, and an explicit `Z` or numeric `±HH:MM` offset. Fractional seconds are limited to nine digits. Invalid calendar dates, naive timestamps, leap seconds, and out-of-profile forms are rejected.
- Timestamp comparisons normalize offsets to UTC without consulting the process timezone. This establishes only the order implied by supplied timestamp values—not source-clock trust or causality.
- Calibration validity is represented as the half-open interval `[valid_from, valid_until)`. The source status claim and interval coverage are checked separately from certificate authenticity.

## Test design

Integration tests load the exact committed vector file via `include_str!` and assert that the evaluator reproduces its machine-readable expectations. Additional adversarial and metamorphic tests cover high-precision decimal comparisons, unit non-equivalence, equivalent decimal spellings, multiple timestamp offsets for the same instant, a 601-case generated boundary sweep, invalid dates, inverted ranges/intervals, missing snapshot tokens, unknown fields, attempt to promote a source claim, malformed decimal values, negative uncertainty, duplicate/mutated conformance vectors, report-contract invariants, and replay-envelope tampering.

This profile validator does not replace an independently reviewed JSON Schema engine. The CI workflow runs formatting, Rust tests, the committed vector command, a replay-envelope round-trip, and Clippy against the exact PR head.


## Local microbenchmarks

Run the committed conformance-vector workload in release mode and save the JSON report:

```sh
RUSTC_VERSION="$(rustc --version)" \
OMSI_BENCHMARK_HOST_LABEL="reference-host-01" \
OMSI_BENCHMARK_CPU_LABEL="record the CPU model here" \
cargo run --release --manifest-path tools/omsi-manufacturing-validator/Cargo.toml -- \
  benchmark-vectors docs/standards/omsi/profiles/manufacturing-asset-operations/0.1/CONFORMANCE-VECTORS.json 1000 \
  > omsi-benchmark-report.json
```

The optional final argument is iterations per vector (default 1000, allowed range 1–100,000). The harness warms each case and reports separate measurements for JSON parsing, semantic evaluation, replay-envelope creation and replay-envelope verification. The output contains p50/p95/p99 and other latency statistics, estimated observation/input throughput, platform architecture, case IDs and SHA-256 of the exact source vector-document bytes.

The report schema is [benchmark-report.schema.json](schema/benchmark-report.schema.json). Its status is deliberately `measured_no_acceptance_claim`: the harness measures work but does not decide whether a performance target has passed. Shared hosted CI should smoke-test report shape without making noisy wall-clock timings a merge gate.

The eight current vectors are a microbenchmark smoke workload, **not** enough to establish a production service-level objective. Compare numbers only with a matching corpus digest and named hardware/toolchain. Before claiming a production SLO, expand to at least 1,000 behaviorally distinct cases, pin a reference host, capture multiple release-mode runs, and measure peak memory separately.

See [the OMSI benchmark scorecard](../../docs/standards/omsi/benchmark-scorecard-0.1.md) for proposed correctness, tamper-resistance, latency, throughput, memory, interoperability, privacy and multi-agent acceptance targets. These are targets, not measured results.
