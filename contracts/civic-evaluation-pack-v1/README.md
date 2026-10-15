# Civic Evaluation Pack V1

**Status:** Synthetic-only evaluation contract; research and conformance tooling, not a live service.
**Purpose:** Define a deterministic, replayable scenario pack that can be consumed by repository-specific evaluators without creating a fourth case model.

## Safety boundary

- This contract accepts synthetic scenario inputs only. Live ingestion and personal data are forbidden.
- Published statements, repair claims, and contextual observations do not establish an independently verified outcome.
- A cross-utility public statement is contextual evidence, not proof of incident-level causality.
- Stale, conflicting, or incomplete source coverage requires abstention and human review.
- A repair claim can become a verified outcome only with distinct-actor independent inspection evidence.
- Recurrence after a verified resolution must reopen the scenario outcome.
- `event_minute` and `recorded_minute` are synthetic relative clock offsets, not wall-clock timestamps. They model delayed delivery without implying a real incident time.

The Rust validator checks semantic invariants that a JSON Schema alone cannot enforce. It is deliberately an offline fixture validator, not a simulator, operational workflow engine, general JSON Schema validator, source authenticator, or action-authorisation mechanism.

## Files

- `schema.json`: structural schema for the versioned pack.
- `examples/joburg-water-synthetic-v1.json`: seven frozen synthetic scenarios, including positive and negative controls.
- `rust/`: strict Rust validator and regression tests.

## Validate offline

From the repository root:

    cargo +1.96.0 fmt --manifest-path contracts/civic-evaluation-pack-v1/rust/Cargo.toml --all -- --check
    cargo +1.96.0 test --manifest-path contracts/civic-evaluation-pack-v1/rust/Cargo.toml --all-targets
    cargo +1.96.0 run --manifest-path contracts/civic-evaluation-pack-v1/rust/Cargo.toml --bin validate-evaluation-pack -- validate contracts/civic-evaluation-pack-v1/examples/joburg-water-synthetic-v1.json

A successful result qualifies only this fixture validator and the checked-in synthetic invariants. It says nothing about real service outcomes, live data, model accuracy, municipal systems, or deployment readiness.
