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

Every observed source leaf must have exactly one classification. A missing or duplicate mapping for a field present in the record fails accounting. Explicit `drop` and `unmapped` dispositions remain visible in separate counters; they are never counted as preserved. Each mapping may set `presence` to `required` or `optional`; omitted `presence` defaults to `required` for backward compatibility. A declared required path missing from a record blocks projection. A declared optional path missing from that record is skipped and counted, but remains visible in `mapping_paths_not_observed_in_record`. The report binds exact source-record and contract bytes with SHA-256 and explicitly declares no canonicalization. CI uses a separately implemented, pinned Rust JSON Schema Draft 2020-12 validator as a dev-only dependency to validate the schema documents and real audit/projection/vector/replay/benchmark/provenance-manifest reports; this is stronger than JSON parsing alone. Those Rust tests still require an exact-head CI pass. The adapter-map, source-record, transform-vector and benchmark-vector CLI inputs are capped at 16 MiB per artifact; source-field collection is capped at 100,000 observed leaf paths.

## Source-schema coverage beyond one sample

`audit-adapter-map` can only account for paths observed in a supplied record. A field may therefore appear fully accounted-for when an optional source property simply did not occur in that record. `audit-adapter-schema-coverage` adds a second, schema-derived inventory:

```sh
cargo run --release --manifest-path tools/omsi-manufacturing-validator/Cargo.toml -- \
  audit-adapter-schema-coverage \
  docs/standards/omsi/adapters/0.1/examples/mes-optional-map.json \
  docs/standards/omsi/adapters/0.1/examples/mes-optional-record.schema.json \
  docs/standards/omsi/adapters/0.1/examples/mes-optional-record.json
```

The command compares every schema-declared scalar or array leaf path against the adapter map, detects mappings outside the declared field inventory, and checks that each mapping's `presence` agrees with the source schema. In the committed example, four schema leaf paths are mapped although only `/temperature` appears in the sample record; `/unit`, `/operator_note`, and `/metadata/note` remain covered and explicitly listed as absent from that record. The schema declares `metadata` optional but `note` required when `metadata` is present, and the command rejects a sample containing `metadata: {}`. Removing the mapping for an absent optional field, misdeclaring its presence, or supplying an open-object schema must fail the gate.

**Fail-closed scope:** this is a field-path inventory profile, not a full JSON Schema implementation. It accepts an object root with explicit `properties`, `required`, and `additionalProperties: false`; nested closed objects are traversed; arrays are atomic paths. It checks object/array/scalar types, required properties (including required children when an optional parent object is present), and undeclared keys. At traversed object/leaf schema nodes it rejects `$ref`, conditional/composition keywords, pattern-based fields, open objects and unsupported type shapes rather than guessing. Array items are deliberately treated as opaque values, so this inventory does not inspect constructs inside `items` or enforce item-level constraints. It also does not apply every value-level constraint (for example, enum/pattern/range), validate XML/XSD, or establish that a vendor schema is authentic. The Draft 2020-12 test dependency separately meta-validates the synthetic schema fixture and checks its sample instance; the coverage command itself does not claim runtime source-schema validation.

The versioned report [schema](../../../../../tools/omsi-manufacturing-validator/schema/adapter-schema-coverage-report.schema.json) binds the exact adapter-map, source-schema and sample-record bytes by SHA-256. The coverage rate is a hard gate only for the field-inventory subset that the command explicitly supports, not proof of semantic equivalence. Before qualifying a real vendor adapter, derive the inventory from the full source schema and add representative records that exercise optional, repeated, null, nested and vendor-extension cases.

## Real-standards interoperability backlog

The [OMSI interoperability target matrix 0.1](../../interoperability-targets-0.1.md) is the next qualification layer after this synthetic JSON path. Its [machine-readable target contract](../../interoperability-targets-0.1.json) and [Draft 2020-12 contract schema](../../../../../tools/omsi-manufacturing-validator/schema/interoperability-target-matrix.schema.json) define ten deliberately unimplemented targets across ISO 8000-110/120/130/140 characteristic-data exchange, provenance, accuracy and completeness; ISO 8000-66 data-quality indicators; ISO 8000-210/220 sensor-data quality; ISO/TS 8000-230 sensor cleansing; ISO 23952 QIF; and MTConnect/OPC UA device interoperability.

The matrix separates source-standard semantics from OMSI's current limited field-inventory engine. QIF requires a pinned XML/XSD path and authoritative schema assets; MTConnect/OPC UA requires a version-pinned device/server/client test, not only fixture parsing. Every target stays labeled `not_implemented` and `not_assessed` until its normative assets, positive/negative fixtures, independent validator and evidence chain are pinned and checked. The contract's independent JSON Schema test also rejects a false conformant status when a target is still marked unimplemented; this is a status-integrity check, not a standards-conformance test. Synthetic maps and a successful JSON schema audit are not a standards-conformance claim.

## Executing projections

`apply-adapter-map` uses the same field-accounting contract and then applies a small, explicit transform registry:

- `qms-decimal-normalization/0.1`: exact string-decimal normalization using OMSI's decimal parser; no binary floating point.
- `qms-unit-code-crosswalk/0.1`: a small allowlisted unit-name-to-code crosswalk. It maps codes, not physical quantities; it does not convert units.

Unknown transform IDs, unsupported unit codes, unmapped fields, missing declared source paths, or colliding/overlapping output pointers block the entire projection. No partial projection is emitted. Explicitly dropped fields remain visible in the audit report. Adding a transformation requires named vectors and a version change; do not silently broaden an existing crosswalk.

The projection CLI emits `omsi-manufacturing-adapter-projection/0.1`, validated against [the projection report schema](../../../../../tools/omsi-manufacturing-validator/schema/adapter-projection-report.schema.json). CI asserts the QMS example normalizes `0.007200` to `0.0072` and `millimeter` to `mm`, while preserving the raw `REVIEW` status. It also checks that the MES projection preserves the raw `Accepted` status and explicitly counts the intentionally dropped note.

The versioned [transform-vector corpus](./TRANSFORM-VECTORS.json) declares exact accepted outputs and rejection reasons for the registered transforms. `verify-transform-vectors` executes that corpus and fails on any mismatch. It currently covers exact decimal normalization (including maximum supported precision, negative zero and scientific-notation rejection) and the small unit-code crosswalk. These are implementation contract tests, not validation of a standards-body unit ontology; add new transform IDs only with explicit vectors and review.

## Rechecking the entire evidence chain

The v0.2 `create-provenance-manifest` command requires schema coverage before it emits a manifest. It re-runs `audit-adapter-schema-coverage` against the exact source-record, source-schema, and adapter-map bytes and requires the submitted coverage report to match with `schema_coverage_complete`. It then re-runs `apply-adapter-map`, requires the supplied projection report to match, requires semantic input to equal the projection payload, recomputes the semantic report, and verifies the replay envelope against the **exact semantic-input bytes**. Any missing or inconsistent schema/coverage/derivation link blocks manifest creation.

The eight-artifact v0.2 manifest binds the source record, source schema, adapter map, schema-coverage report, projection report, semantic input, semantic report, and replay envelope by SHA-256 and byte length. Five explicit activities record schema-coverage audit, adapter projection, semantic-input extraction, semantic evaluation, and replay-envelope creation, connected by 14 typed edges. The `schema_coverage` section repeats the schema/coverage digests and field counts for convenient downstream policy checks; the builder regenerates that report instead of trusting its claims. The graph declares `w3c-prov-dm-aligned-subset/0.2`, using `used` and `wasGeneratedBy` without claiming complete W3C PROV serialization conformance. Declared transform IDs do not imply that every optional field's transformation executed. All digests cover exact bytes with no JSON canonicalization; source values and local filesystem paths are not embedded.

Example using the committed synthetic QMS-like fixture:

```sh
set -euo pipefail
base=docs/standards/omsi/adapters/0.1/examples
tmp=${TMPDIR:-/tmp}/omsi-provenance
mkdir -p "$tmp"

cargo run --release --manifest-path tools/omsi-manufacturing-validator/Cargo.toml -- \
  audit-adapter-schema-coverage "$base/qms-nested-map.json" \
  "$base/qms-nested-record.schema.json" "$base/qms-nested-record.json" \
  > "$tmp/schema-coverage.json"
cargo run --release --manifest-path tools/omsi-manufacturing-validator/Cargo.toml -- \
  apply-adapter-map "$base/qms-nested-map.json" "$base/qms-nested-record.json" > "$tmp/projection.json"
jq '.projection' "$tmp/projection.json" > "$tmp/semantic-input.json"
cargo run --release --manifest-path tools/omsi-manufacturing-validator/Cargo.toml -- \
  create-envelope "$tmp/semantic-input.json" > "$tmp/replay-envelope.json"
jq '.report' "$tmp/replay-envelope.json" > "$tmp/semantic-report.json"
cargo run --release --manifest-path tools/omsi-manufacturing-validator/Cargo.toml -- \
  create-provenance-manifest "$base/qms-nested-record.json" "$base/qms-nested-record.schema.json" \
  "$base/qms-nested-map.json" "$tmp/schema-coverage.json" "$tmp/projection.json" \
  "$tmp/semantic-input.json" "$tmp/semantic-report.json" "$tmp/replay-envelope.json" \
  > "$tmp/provenance-manifest.json"
cargo run --release --manifest-path tools/omsi-manufacturing-validator/Cargo.toml -- \
  verify-provenance-manifest "$tmp/provenance-manifest.json" "$base/qms-nested-record.json" \
  "$base/qms-nested-record.schema.json" "$base/qms-nested-map.json" \
  "$tmp/schema-coverage.json" "$tmp/projection.json" "$tmp/semantic-input.json" \
  "$tmp/semantic-report.json" "$tmp/replay-envelope.json"
```

The manifest contract is defined in [provenance-manifest.schema.json](../../../../../tools/omsi-manufacturing-validator/schema/provenance-manifest.schema.json); the producer-side verifier report uses [provenance-verification-report.schema.json](../../../../../tools/omsi-manufacturing-validator/schema/provenance-verification-report.schema.json). `verify-provenance-manifest` requires the manifest plus all eight original artifact inputs and reconstructs the full expected manifest.

A separate Rust crate, `tools/omsi-provenance-verifier`, provides an independent `verify-bundle` implementation for the **artifact-binding and graph layer**. It independently checks exact-byte artifact digests/lengths and roles/order, the fixed activity/edge graph, and the restricted schema-coverage inventory, including required/optional mapping presence and absent optional field lists. Its adversarial tests recompute the artifact/report hashes after forging the reported schema-field count or erasing an absent optional field; the independent checker must still reject the bundle. This blocks a producer from relying on a self-consistent hash around a false coverage summary.

The independent crate deliberately does **not** execute adapter transforms or recompute the manufacturing semantic decision; both crates currently use `serde_json` and `sha2`. This is independent source implementation for byte binding, graph, and the restricted field inventory—not yet an independent semantic evaluator, different-language implementation, source authentication, or measurement-truth proof. The workflow smoke test runs both verifiers, and exact-head CI is still the evidence gate.

**Trust boundary:** this is a local consistency and mutation-detection aid, not an authenticated attestation. Because the manifest is unsigned, an actor able to replace the inputs and recompute all outputs can produce another consistent bundle. A production-grade next stage should package the manifest as a standard attestation with authenticated signer identity and explicit predicate type, then verify it independently; never confuse signing with measurement truth.

## Synthetic examples

The `examples/` directory contains flat MES-like records, nested QMS-like records, and a record where two explicitly optional fields are absent. CI checks field accounting and executable projection for both shapes and verifies that absent optional fields are skipped/countable without being confused with dropped data. These are synthetic fixtures, not evidence of conformance with any actual vendor or manufacturing standard.

## Limitations

- The record audit covers fields observed in the supplied record. `audit-adapter-schema-coverage` adds a restricted closed-object schema-path inventory so absent optional properties are still required to have mappings; unsupported schema constructs fail closed. This inventory is not full JSON Schema conformance and does not establish semantic correctness.
- Transformation vectors exercise only the enumerated cases; they do not prove vendor-wide semantic equivalence, standard-body ontology alignment, or dimensional unit conversion. Expand vectors from reviewed source schemas and independent reference implementations.
- Complete accounting does not prove that the target mapping is correct, that the source is authentic, that a measurement is true, or that a quality-release decision is authorized.
- Unknown or secret fields are not safe merely because they are classified. Data owners must review whether each field may enter an OMSI report or a public Mycelix receipt.
