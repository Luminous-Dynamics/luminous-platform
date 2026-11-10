# DWF adversarial schedule harness v0.1

A standalone deterministic model-level schedule generator for Platform issue #363 and PR #353. This package does not enter the evaluator, scheduler, dispatch, transport, or production runtime.

## Function

- Generates bounded event schedules from fixed seeds with a deterministic SplitMix64 generator.
- Emits a canonical JSON event list and per-schedule SHA-256.
- Computes an aggregate SHA-256 over the ordered schedule digests.
- Runs a test-only lifecycle model, then checks accepted transitions using a separate audit-ledger implementation that does not inspect model state or call the transition function.
- Reports per-event accepted/rejected counts.
- Deterministically minimizes any discovered invariant violation by attempting single-event deletion.
- Exercises authority epochs, revocation, partitions, reconnect, lost acknowledgements, completion, durable receipts, restart, reconciliation, independent oracle outcomes, acceptance and duplicate-safe retries.

## Run

With a Rust 2024 toolchain and Cargo dependencies available:

    cargo test
    cargo run --release -- --seed 350 --schedules 1000 --max-events 19 --out report.json

Export one exact event array and replay it byte-for-byte:

    cargo run --release -- --seed 350 --max-events 19 --emit-schedule --out events.json
    cargo run --release -- --replay events.json

Default run: 1,000 schedules, seed 350, maximum 19 events per schedule. Exit status is 0 with no detected invariant violation, 2 if a violation is found, and 1 for invocation or input errors.

## Evidence boundary

The report is always marked modelled. This is not a network, storage, transport, cryptographic, attestation, independent-execution, or physical-device test. A modelled result is not physical qualification.

The model and checker are separate implementations but authored together; this is not an independently certified formal specification. The checker validates safety of accepted transitions. It does not prove that every rejected transition ought to be rejected; positive-control tests cover selected intended paths. The bounded schedule generator is not exhaustive over arbitrary histories. Preserve the seed, harness version, event JSON and digest with any reported outcome.

No test or compile result is claimed until an exact-head Rust toolchain run has been executed.
