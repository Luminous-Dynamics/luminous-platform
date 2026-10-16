# OMSI Manufacturing Adapter Map 0.1

**Status:** synthetic adapter-accounting contract; not a source-system certification.

An adapter can produce valid JSON while silently dropping fields that matter to scope, provenance, measurement, or quality interpretation. This profile makes that loss visible before an adapter output is considered for OMSI evaluation.

## CLI

```sh
cargo run --release --manifest-path tools/omsi-manufacturing-validator/Cargo.toml -- \
  audit-adapter-map \
  docs/standards/omsi/adapters/0.1/examples/mes-flat-map.json \
  docs/standards/omsi/adapters/0.1/examples/mes-flat-record.json

cargo run --release --manifest-path tools/omsi-manufacturing-validator/Cargo.toml -- \
  apply-adapter-map \
  docs/standards/omsi/adapters/0.1/examples/qms-nested-map.json \
  docs/standards/omsi/adapters/0.1/examples/qms-nested-record.json

cargo run --release --manifest-path tools/omsi-manufacturing-validator/Cargo.toml -- \
  verify-transform-vectors docs/standards/omsi/adapters/0.1/TRANSFORM-VECTORS.json
```

The command inventories the leaf paths in the exact JSON source-record bytes. Nested objects recurse into JSON Pointer paths; arrays are treated as atomic fields so an adapter must explicitly classify or transform a collection rather than having the audit silently choose an item shape.

Every observed source leaf must be covered exactly once:

| Disposition | Required contract fields | Meaning |
|---|---|---|
| `preserve` | `target_path` | Source field is mapped unchanged to a declared target path |
| `transform` | `target_path`, `transform_id` | A named, registered transformation is executed by `apply-adapter-map`; `audit-adapter-map` only accounts for the declaration |
| `drop` | `reason` | Field is intentionally omitted with a reason |
| `unmapped` | `reason` | Field is known but no target mapping is agreed yet |

Every observed source leaf must have exactly one classification. A missing or duplicate mapping for a field present in the record fails accounting. Explicit `drop` and `unmapped` dispositions remain visible in separate counters; they are never counted as preserved. Each mapping may set `presence` to `required` or `optional`; omitted `presence` defaults to `required` for backward compatibility. A declared required path missing from a record blocks projection. A declared optional path missing from that record is skipped and counted, but remains visible in `mapping_paths_not_observed_in_record`. The report binds exact source-record and contract bytes with SHA-256 and explicitly declares no canonicalization. The adapter-map, source-record, transform-vector and benchmark-vector CLI inputs are capped at 16 MiB per artifact; source-field collection is capped at 100,000 observed leaf paths.

## Executing projections

`apply-adapter-map` uses the same field-accounting contract and then applies a small, explicit transform registry:

- `qms-decimal-normalization/0.1`: exact string-decimal normalization using OMSI's decimal parser; no binary floating point.
- `qms-unit-code-crosswalk/0.1`: a small allowlisted unit-name-to-code crosswalk. It maps codes, not physical quantities; it does not convert units.

Unknown transform IDs, unsupported unit codes, unmapped fields, missing declared source paths, or colliding/overlapping output pointers block the entire projection. No partial projection is emitted. Explicitly dropped fields remain visible in the audit report. Adding a transformation requires named vectors and a version change; do not silently broaden an existing crosswalk.

The projection CLI emits `omsi-manufacturing-adapter-projection/0.1`, validated against [the projection report schema](../../../../../tools/omsi-manufacturing-validator/schema/adapter-projection-report.schema.json). CI asserts the QMS example normalizes `0.007200` to `0.0072` and `millimeter` to `mm`, while preserving the raw `REVIEW` status. It also checks that the MES projection preserves the raw `Accepted` status and explicitly counts the intentionally dropped note.

The versioned [transform-vector corpus](./TRANSFORM-VECTORS.json) declares exact accepted outputs and rejection reasons for the registered transforms. `verify-transform-vectors` executes that corpus and fails on any mismatch. It currently covers exact decimal normalization (including maximum supported precision, negative zero and scientific-notation rejection) and the small unit-code crosswalk. These are implementation contract tests, not validation of a standards-body unit ontology; add new transform IDs only with explicit vectors and review.

## Synthetic examples

The `examples/` directory contains flat MES-like records, nested QMS-like records, and a record where two explicitly optional fields are absent. CI checks field accounting and executable projection for both shapes and verifies that absent optional fields are skipped/countable without being confused with dropped data. These are synthetic fixtures, not evidence of conformance with any actual vendor or manufacturing standard.

## Limitations

- The audit covers fields observed in the supplied record. Repeat it across representative records and add coverage from the source schema to discover optional fields that do not appear in one sample.
- A transform ID names the intended transformation but does not run or prove it. A next step is a separate transformation-vector suite that checks concrete input/output pairs and semantic equivalence.
- Complete accounting does not prove that the target mapping is correct, that the source is authentic, that a measurement is true, or that a quality-release decision is authorized.
- Unknown or secret fields are not safe merely because they are classified. Data owners must review whether each field may enter an OMSI report or a public Mycelix receipt.
