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

The vector command exits 0 only when all machine-readable expected outcomes match the computed semantic results. It checks expected disposition types/reasons, the comparison rule ID, and optional measurement/scope/freshness fields. The human-readable `assertions` strings are documentation and are not interpreted as executable tests.

Evaluate one input object (the content of a vector's `input`) by writing it to a JSON file or piping it through stdin:

```sh
cargo run --manifest-path tools/omsi-manufacturing-validator/Cargo.toml -- evaluate-input input.json
cat input.json | cargo run --manifest-path tools/omsi-manufacturing-validator/Cargo.toml -- evaluate-input -
```

A rejected or conflicted input is emitted as JSON and results in exit code 1 for `evaluate-input`. Clean projections and projections with explicitly disclosed loss return exit code 0. Invalid JSON, filesystem errors, or CLI usage errors return exit code 2.

## Decimal and timestamp profile

- Decimal values are strings, not JSON floating-point numbers. The parser rejects exponent notation, leading zeroes, leading plus signs, and precision outside its bounded profile (up to 18 integer digits and 18 fractional digits).
- Comparisons rescale checked decimal coefficients and do not use binary floating point.
- Timestamps require a full date and seconds, uppercase `T`, and an explicit `Z` or numeric `±HH:MM` offset. Fractional seconds are limited to nine digits. Invalid calendar dates, naive timestamps, leap seconds, and out-of-profile forms are rejected.
- Timestamp comparisons normalize offsets to UTC without consulting the process timezone. This establishes only the order implied by supplied timestamp values—not source-clock trust or causality.
- Calibration validity is represented as the half-open interval `[valid_from, valid_until)`. The source status claim and interval coverage are checked separately from certificate authenticity.

## Test design

Integration tests load the exact committed vector file via `include_str!` and assert that the evaluator reproduces its machine-readable expectations. Additional adversarial and metamorphic tests cover high-precision decimal comparisons, unit non-equivalence, equivalent decimal spellings, multiple timestamp offsets for the same instant, a 601-case generated boundary sweep, invalid dates, inverted ranges/intervals, missing snapshot tokens, unknown fields, attempt to promote a source claim, malformed decimal values, negative uncertainty, duplicate/mutated conformance vectors, and report-contract invariants.

This profile validator does not replace an independently reviewed JSON Schema engine. The CI workflow runs formatting, Rust tests, the committed vector command, and Clippy against the exact PR head.
