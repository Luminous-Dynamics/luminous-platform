# Evidence Independence Profile auditor

This crate audits the declared control-domain graph in an EIP v0.1 profile. It does not authenticate the declarations or perform Draft 2020-12 JSON Schema validation.

## Run

```sh
cargo test
cargo run --release -- ../fixtures/itops-process-smoke.profile.json ../profile.schema.json
```

The command emits deterministic JSON with the exact profile SHA-256, optional schema SHA-256, structural issues, dependency cycles, per-claim/per-dimension outcomes, and the resulting claim ceiling. Passing the schema records its digest and checks that it declares Draft 2020-12; it does not execute JSON Schema validation.

## Outcome meanings

- `fail`: the source and comparison dependency closures share at least one declared domain ID in a required dimension.
- `indeterminate`: a required domain is unknown/not-applicable, a dependency inventory is not asserted complete, or the profile is structurally invalid.
- `declared_separate_unverified`: the declared domain IDs are disjoint and the involved dependency inventories are asserted complete. This is not an authenticated independence result.

There is deliberately no `pass` or `verified_independent` outcome. The auditor cannot authenticate the evidence references or prove that dependency inventories are complete.

## Algorithm

1. Validate core component and claim references, identity uniqueness, required policy semantics and dimensions.
2. Build the component dependency graph and reject dangling references or cycles.
3. Compute the transitive dependency closure for the evidence source and every comparison component.
4. For each required dimension, union declared domain IDs across each closure and compute their intersection.
5. Shared IDs fail that dimension; incomplete/unknown data produces indeterminate; only fully declared disjoint data produces declared-separate-unverified.
6. Hash the exact input bytes and serialize a stable JSON report.

The schema is the canonical structural contract. Run it through a Draft 2020-12 validator independently before treating the semantic audit report as a well-formed profile report. The semantic auditor intentionally says `schema_validation: not_performed_by_semantic_auditor`.

## Limits

The algorithm can detect only dependencies represented in the profile. A malicious or mistaken author can omit an edge or lie about a domain ID. Authenticated runtime monitors, independent deployment evidence and source-verifiable receipts are needed to establish the truth of those declarations. The tool does not measure real-world independence or physical truth.
