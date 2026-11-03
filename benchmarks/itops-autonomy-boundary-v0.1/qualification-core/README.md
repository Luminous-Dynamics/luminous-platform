# ITOPS qualification core (prototype)

This crate is the first executable building block for the runner requested in
[Luminous Platform issue #213](https://github.com/Luminous-Dynamics/luminous-platform/issues/213).

## Current scope

- Loads the v0.1 candidate corpus and binds it to the exact input-byte SHA-256.
- Validates the candidate catalog and synthetic result example against their
  full Draft 2020-12 JSON Schemas, with format assertions enabled and no
  implicit remote/file schema retrieval.
- Validates the 23-case v0.1 count, unique case IDs, unresolved-outcome safety
  bounds, and all three cross-domain workflow DAGs.
- Computes a deterministic SHA-256 digest for each typed workflow plan.
- Implements an exact-scope authorization predicate binding tenant, target,
  operation, expiry/revocation, and workflow-plan digest.
- Implements a deterministic participant-effect ledger model with idempotency
  keys and an unknown-outcome lock.
- Treats stale, future-dated, missing-source, or digest-less service
  observations as unknown.

## Run

From this directory:

```sh
cargo test
cargo run --release
```

The executable validates both schemas and prints the exact corpus digest and
each workflow digest. It accepts optional paths in this order:
`CATALOG_JSON CATALOG_SCHEMA_JSON RESULT_EXAMPLE_JSON RESULT_SCHEMA_JSON`.

## Explicit non-claims

This is not yet an agent runner, production runtime gate, independent process
boundary, independent service observer, or measured benchmark. The complete JSON Schema validator is present, but has not yet been compiled or run. The effect ledger and observer are in-process
deterministic models for unit testing. The next issue #213 stages are process
separation, an isolated synthetic episode environment, result generation and
validation, and repeated experiment arms. The corpus remains
`candidate_not_frozen`.
