# ITOPS Autonomy Boundary Catalog Linter

This small Rust CLI checks cross-field invariants in the candidate scenario catalog and prints the exact catalog's SHA-256 digest.

It is **not** an agent runner, policy enforcement runtime, JSON Schema validator, or benchmark result generator. The JSON Schema remains the structural contract; the linter adds domain invariants that a schema cannot easily express.

## Run

From the repository root:

```bash
cargo run --manifest-path benchmarks/itops-autonomy-boundary-v0.1/catalog-lint/Cargo.toml --   benchmarks/itops-autonomy-boundary-v0.1/case-catalog.v0.1.json   benchmarks/itops-autonomy-boundary-v0.1/case-catalog.schema.json
```

Expected output includes:

- `ITOPS_CATALOG_LINT=PASS`
- `CASE_COUNT=20`
- `CASE_CATALOG_SHA256=<digest>`
- `STATUS=candidate_not_frozen`

The digest is computed from the exact catalog file bytes. Bind it into a run manifest only after the scenario set has been reviewed and frozen.

## Cross-field invariants checked

- Catalog case IDs exactly match the result-schema case-ID enum and are unique.
- All fields required by the catalog schema are present.
- Execution is constrained to isolated synthetic environments with no production network routes.
- A case with an unresolved prior effect must choose `RECONCILE`, set zero dispatch bounds, and keep participant effect status as `unknown_until_reconcile`.
- New attempts while a prior outcome is unknown always have a zero safety bound.
- Non-valid authority states cannot have positive dispatch bounds.
- In v0.1, only the exact-scope `CHG-002` case may have a positive dispatch bound, capped at one.
- Required handoffs identify at least one packet field; no-handoff cases do not require packet fields.

The linter intentionally does not decide whether task text is clear, whether an oracle is truly independent, whether a model behaved safely, or whether a service recovered. Those require the future runner and environment. It also does not perform a full JSON Schema implementation; run the catalog through a Draft 2020-12 validator as a separate check.
