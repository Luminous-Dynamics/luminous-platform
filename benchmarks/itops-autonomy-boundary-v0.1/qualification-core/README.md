# ITOPS qualification core (prototype)

This crate is the first executable building block for the runner requested in
[Luminous Platform issue #213](https://github.com/Luminous-Dynamics/luminous-platform/issues/213).

## Current scope

- Loads the v0.1 candidate corpus and binds it to the exact input-byte SHA-256.
- Validates both schema documents against the Draft 2020-12 meta-schema, then
  validates the candidate catalog and synthetic result example against those
  schemas with format assertions enabled and no implicit remote/file retrieval.
- Validates the 23-case v0.1 count, unique case IDs, unresolved-outcome safety
  bounds, and all three cross-domain workflow DAGs.
- Computes a deterministic SHA-256 digest for each typed workflow plan.
- Implements an exact-scope authorization predicate binding tenant, target,
  operation, expiry/revocation, dispatch budget, and workflow-plan digest;
  dispatch rechecks the latest grant to prevent stale permits being used after
  revocation, expiry, or target/operation/plan rebinding.
- Implements a deterministic participant-effect ledger model with idempotency
  keys and an unknown-outcome lock.
- Treats stale, future-dated, missing-source, or malformed-digest service
  observations as unknown.
- Includes 15 unit-test sources for catalog loading, schema/meta-schema
  validity and rejection, result/case semantic alignment, exact-scope
  authorization, dispatch-time grant revalidation, grant budgets, idempotency,
  unresolved-attempt locking, and observation freshness.

## Run

From this directory:

```sh
cargo test
cargo run --release
```

The executable first validates both schema documents against the Draft
2020-12 meta-schema, then validates the catalogue and example result with
format assertions enabled. It prints the exact corpus digest and each
workflow digest. It accepts optional paths in this order:
`CATALOG_JSON CATALOG_SCHEMA_JSON RESULT_EXAMPLE_JSON RESULT_SCHEMA_JSON`.

## Explicit non-claims

This is not yet an agent runner, production runtime gate, independent process
boundary, independent service observer, or measured benchmark. The complete JSON Schema validator is present, but has not yet been compiled or run. The effect ledger and observer are in-process
deterministic models for unit testing. The next issue #213 stages are process
separation, an isolated synthetic episode environment, result generation and
validation, and repeated experiment arms. The corpus remains
`candidate_not_frozen`.
