# OMSI Manufacturing Asset Operations Profile 0.1 — incubation

**Status:** synthetic pilot proposal; not adopted or certified  
**Date:** 2026-10-15  
**Implementation status:** mapping and conformance-vector design only; no vendor adapter, live connector, safety qualification or production execution is implemented here.

## 1. Purpose

Test whether the proposed OMSI interchange/evidence envelope generalizes from field-service maintenance records to manufacturing maintenance and quality observations without erasing manufacturing-specific semantics.

This profile is intentionally read-only. It covers synthetic asset references, maintenance work references, quality observations, measurement context, calibration references, source lifecycle claims, and provenance. It does not define production dispatch, machine actuation, batch recipes, product release, or an industrial safety function.

## 2. Standards relationship

Reuse domain standards rather than duplicate them:

- [ISA-95 / IEC 62264](https://www.isa.org/standards-and-publications/isa-standards/isa-95-standard) for the enterprise/manufacturing-operations boundary and relevant manufacturing object concepts. ISA lists ANSI/ISA-95.00.01-2025 as the current Part 1 edition; any implementation must pin the exact purchased/available parts and applicable profile.
- [ISO 55000:2024](https://www.iso.org/standard/83053.html) for asset-management terminology and lifecycle principles; this profile does not claim ISO 55001 conformity.
- [OPC UA, OPC 10000-1](https://reference.opcfoundation.org/specs/OPC-10000-1/4.1) where an implementation uses OPC UA information models or industrial data access. OPC UA supports information exchange and command/control; this profile does not expose its control capabilities.
- [UCUM](https://ucum.org/ucum) for unit codes where appropriate. A UCUM code identifies a unit expression; it does not by itself define calibration validity, measurement uncertainty, permissible operating bounds or quality acceptance.

These references are anchors for later mapping research, not a statement that this draft is compliant with any of them. Exact revisions, companion specifications, licensing/applicability and supported subsets must be pinned before a vendor adapter is accepted.

## 3. In-scope record concepts

| Concept | Meaning in this profile | Not equivalent to |
|---|---|---|
| Asset reference | Namespaced source identity for a machine, component, line or instrument | Physical identity proof, tenant authorization or configuration attestation |
| Maintenance work reference | The source work-order/maintenance record associated with an asset or observation | Permission to operate, repair or change equipment |
| Quality observation | One typed assertion about a measured or inspected property | A complete inspection, batch genealogy, process capability or product release |
| Measurement | Decimal value, quantity kind, unit, observation time, method, instrument, calibration reference and uncertainty where available | A verified measurement solely because it is numeric |
| Specification reference | Versioned expected bounds/criteria and units | Legal standard, safety limit or release authority unless separately established by a domain profile |
| Calibration reference | Link to the calibration record and its stated validity/status | Proof that calibration was correctly performed or that the instrument was accurate in use |
| Source lifecycle claim | Original source state with source object/revision attribution | Independent quality disposition or safe-to-operate authorization |
| Provenance edge | Typed relationship to a source record/revision | Complete lineage unless the bounded lineage reader verifies the declared scope |

The source adapter must keep source-native identifiers and status values. Every normalized value must name the profile/mapping version that produced it.

## 4. Minimum observation fields

A candidate observation fixture includes:

- Source pin: source system, source instance/site namespace, object type, external ID and record revision where supplied.
- Trusted scope binding: independently configured binding ID and resolved tenant; source site/company IDs remain source references, never authorization evidence by themselves.
- Typed references: asset, maintenance/work order, specification, instrument and calibration record, all namespaced.
- Times: observed/event time, source-record modified/revision metadata (if exposed), export time and ingestion time are separate fields. Preserve source precision and timezone interpretation. Naive timestamps must not be silently treated as UTC.
- Measurement: decimal lexical representation, quantity kind, unit system/code, method and instrument reference; include uncertainty and sample context if available. Do not use binary floating-point equality for threshold assertions. Missing units make a comparable measurement unavailable.
- Specification: source ID/revision, declared range/limit, units and any tolerances. Never manufacture limits if the source doesn't provide them.
- Calibration: reference, state, valid interval, issuer/method metadata when available; absence is not equivalent to valid.
- Claim/evidence: source-reported assessment, evidence references, assurance level and mapping loss/conflict information.
- Snapshot semantics: export identity, extraction time/interval, source cursor/watermark or revision token if supported, and the declared consistency mode.

A measurement with a valid calibration reference can still be wrong. A measurement falling within the declared numeric range can still be insufficient for product release. This profile reports what source data and bounded rules support; it does not authorize quality release.

### Explicit comparison rule for conformance fixtures

The fixture rule ID is `nominal-point-estimate-range/0.1`. It compares the exact decimal nominal value with the inclusive source-provided lower and upper bounds only when the measurement and specification unit codes match exactly. It does not convert units, apply measurement uncertainty, establish calibration authenticity, or decide product acceptance. The fixture value may be reported as inside/outside the declared numeric bounds, but that is a diagnostic comparison—not a quality decision, process capability calculation, or release authorization. Any richer decision rule must be separately versioned with validated unit conversion and uncertainty semantics.

## 5. Mapping and disposition rules

1. Preserve source assertions beside any derived comparison. Never overwrite a source status with a locally computed condition.
2. A valid, explicit unit is required for unit-aware comparison. Missing or incompatible units make the comparison not_evaluable and emit a conflict or unmapped disposition as defined by the source mapping.
3. A missing or expired calibration status prevents a calibration_valid_at_observation claim. Preserve the observation and report the calibration gap; do not silently discard an otherwise attributable reading.
4. A measurement outside source-provided bounds conflicts with an accepted source claim. Preserve both. No local comparison may create a release or rejection authority the adapter does not possess.
5. A cross-tenant / cross-source-instance relation is a scope rejection; do not fetch opportunistically across the boundary.
6. Unknown critical source revision/schema or ambiguous identity is rejected. Missing optional revision metadata is a loss and prohibits freshness claims; export time is not a substitute.
7. A source status such as Accepted, Released, Complete or Good is preserved as a source claim and never promoted to independent verification.
8. All reads are read-only. The profile must not write to a PLC, MES, SCADA, ERP, quality system or machine interface. No actuation path is specified.
9. Use stable result categories—reject, conflict, loss, unmapped, not_provided—and include the affected field and mapping rule. Do not coerce absence into false or success.
10. Preserve enough context to reproduce a comparison: exact decimal operands, unit codes, specification revision, algorithm/rule-set version, and any unit conversion policy.

## 6. Synthetic conformance vectors

[Conformance vectors](CONFORMANCE-VECTORS.json) are intentionally JSON-parseable test data, not a normative wire schema. Each vector names its fixture input, expected disposition and assertions. They cover one positive case plus missing and dimensionally incompatible units, expired calibration, tenant mismatch, naive time, status-versus-measurement conflict and missing record-revision metadata.

The accompanying [JSON Schema](CONFORMANCE-VECTORS.schema.json) validates the vector-file structure only; it does not execute semantic assertions or validate the OMSI interchange envelope. A structural schema pass is not a conformance pass.

A future executable validator must fail closed when the supplied input cannot be interpreted under the declared profile version. The validator must check the assertions, not merely load the JSON.

## 7. Required negative controls before a real adapter

In addition to the linked synthetic vectors, future tests must cover duplicate/colliding source IDs, changed spec revision between observation and review, incompatible dimensional units, precision truncation, calibration revoked after export, asset moved between tenant scopes, record changing during snapshot traversal, repeated/missing pages, a partial lineage marked complete, stale telemetry, timestamp clock-quality limits, attached certificate present but invalid, raw sensitive data leakage, and any write attempt through a purported read-only connector.

## 8. Conformance claims

This is proposal-level evidence only. Passing these vectors would not demonstrate live MES/ERP/OPC UA interoperability, valid calibration, safe machine operation, good manufacturing practice, regulatory compliance, product release authority or production readiness. C2 or higher source-snapshot/live/cross-implementation claims require their separate evidence gates; queued CI and authored vectors are not passes.

## 9. Acceptance gates for this pilot

1. A JSON Schema for the fixture format is independently reviewed before the envelope is treated as stable.
2. At least two separate source projections map the same semantic concepts while preserving vendor-specific fields and loss declarations.
3. Decimal/unit comparisons use a pinned and tested unit policy with explicit rejection of incompatible quantities.
4. Every positive case has at least one meaningful negative control.
5. A second implementation consumes the output and preserves source claims, conflicts, calibration limits and source revision gaps.
6. Live source access, if ever proposed, is a separate read-only qualification with tenant isolation, stale/partial snapshot, pagination, revocation and zero-write tests.
7. Any future quality release or equipment-control workflow has a separate, reviewed domain authority and safety case; it does not inherit that power from this interchange profile.


## Executable semantic validator (incubation)

A separate Rust implementation is proposed at [`tools/omsi-manufacturing-validator`](../../../../../../tools/omsi-manufacturing-validator/README.md). It checks the committed vector file against computed machine-readable outcomes and includes additional adversarial tests for decimal precision, timestamp normalization, tenant/reference scope, malformed fields, and assurance upgrades.

From the repository root, run:

```sh
cargo fmt --manifest-path tools/omsi-manufacturing-validator/Cargo.toml --all -- --check
cargo test --manifest-path tools/omsi-manufacturing-validator/Cargo.toml
cargo run --manifest-path tools/omsi-manufacturing-validator/Cargo.toml -- validate-vectors docs/standards/omsi/profiles/manufacturing-asset-operations/0.1/CONFORMANCE-VECTORS.json
```

The validator does not authenticate the binding, source system, calibration issuer, or physical measurement. Its comparison is nominal-value-only and does not authorize product release or machine control. Its CI status must be read from the exact code head; authored tests are not a passing result.


### Versioned semantic report

The validator emits `omsi-manufacturing-semantic-report/0.1`, including a rule-bound `decision_trace`. Every entry names the rule, outcome state, input field references, and explanation. The report also declares `decision_authority=diagnostic_only`, `quality_release_authority=none`, `equipment_control_authority=none`, and `origin_authentication=not_performed`. These are operational boundaries, not decorative metadata.

The [report schema](../../../../../../tools/omsi-manufacturing-validator/schema/semantic-report.schema.json) describes this output contract. Neither the report nor its trace is cryptographically signed or independently authenticated in this pilot. An integrator must treat the trace as the validator's reproducible explanation, not a proof of source truth or authorization.
