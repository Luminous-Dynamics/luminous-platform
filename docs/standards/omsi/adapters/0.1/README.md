# OMSI Manufacturing Adapter Map 0.1

**Status:** synthetic adapter-accounting contract; not a source-system certification.

An adapter can produce valid JSON while silently dropping fields that matter to scope, provenance, measurement, or quality interpretation. This profile makes that loss visible before an adapter output is considered for OMSI evaluation.

## CLI

```sh
cargo run --release --manifest-path tools/omsi-manufacturing-validator/Cargo.toml -- \
  audit-adapter-map \
  docs/standards/omsi/adapters/0.1/examples/mes-flat-map.json \
  docs/standards/omsi/adapters/0.1/examples/mes-flat-record.json
```

The command inventories the leaf paths in the exact JSON source-record bytes. Nested objects recurse into JSON Pointer paths; arrays are treated as atomic fields so an adapter must explicitly classify or transform a collection rather than having the audit silently choose an item shape.

Every observed source leaf must be covered exactly once:

| Disposition | Required contract fields | Meaning |
|---|---|---|
| `preserve` | `target_path` | Source field is mapped unchanged to a declared target path |
| `transform` | `target_path`, `transform_id` | A named transformation is intended; this audit does not execute it |
| `drop` | `reason` | Field is intentionally omitted with a reason |
| `unmapped` | `reason` | Field is known but no target mapping is agreed yet |

A missing or duplicate mapping fails accounting. Explicit `drop` and `unmapped` dispositions remain visible in separate counters; they are never counted as preserved. The report binds exact source-record and contract bytes with SHA-256 and explicitly declares no canonicalization.

## Synthetic examples

The `examples/` directory has two differently shaped examples: flat MES-like records and nested QMS-like records. The CI smoke test asserts complete field accounting for both. These are synthetic fixtures, not evidence of conformance with any actual vendor or manufacturing standard.

## Limitations

- The audit covers fields observed in the supplied record. Repeat it across representative records and add coverage from the source schema to discover optional fields that do not appear in one sample.
- A transform ID names the intended transformation but does not run or prove it. A next step is a separate transformation-vector suite that checks concrete input/output pairs and semantic equivalence.
- Complete accounting does not prove that the target mapping is correct, that the source is authentic, that a measurement is true, or that a quality-release decision is authorized.
- Unknown or secret fields are not safe merely because they are classified. Data owners must review whether each field may enter an OMSI report or a public Mycelix receipt.
