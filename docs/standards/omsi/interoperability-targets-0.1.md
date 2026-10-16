# OMSI Manufacturing Interoperability Targets 0.1

**Status:** research and qualification backlog; no standards or vendor conformance is claimed.  
**Machine-readable contract:** [interoperability-targets-0.1.json](./interoperability-targets-0.1.json)

## Why this is a separate qualification layer

OMSI's current examples are synthetic JSON fixtures. Schema-derived coverage is a restricted closed-object JSON Schema field-inventory profile. It guards against some silent field loss, but it does not establish ISO 8000, QIF, MTConnect, OPC UA, XML/XSD or vendor conformance.

The next paradigm shift is to treat each target as a **version-pinned executable conformance bundle**: named normative version, authorized machine-readable schema/model assets, positive and negative fixtures, independent validator, approved mapping decisions, expected semantic outcome, artifact digests and provenance linking each step. “Schema parses” must not be confused with “source implementation conforms”; path accounting must not be confused with semantic equivalence.

## Target order and qualification goals

### 1. ISO 8000-110:2021 — characteristic-data exchange

ISO 8000-110 specifies computer-checkable requirements for characteristic-data message syntax, semantic encoding, and conformance to a data specification. Importantly, ISO's published scope excludes provenance, accuracy and completeness information, referring to ISO 8000-120, ISO 8000-130 and ISO 8000-140 for those distinct concerns. Do not implement all of these meanings as one ambiguous “valid data” flag.

**Acceptance targets:** 100% agreement with an independent validator over the pinned corpus; 100% inventory field accounting; zero unsupported schema constructs accepted as conformant.

### 2. ISO 8000-120:2016 — provenance of characteristic data

Use the separately defined provenance information model and capture/exchange requirements as a standards research input for property-value pedigree. Do not assume the standard supplies a wire format, authenticates the producer, or substitutes for cryptographic signature verification.

**Acceptance targets:** 100% coverage of required provenance assertions for the chosen use case; 100% reconstruction of source-to-derived-value lineage across the fixture corpus; zero tampered or conflicting provenance links left unflagged.

### 3. ISO 8000-130:2016 — accuracy assertions

This part concerns the representation and exchange of statements/assertions about accuracy; it does not prescribe a universal accuracy requirement or a measurement method. Keep a reported accuracy assertion distinct from measurement uncertainty, calibration validity, instrument state and the truth of a physical reading.

**Acceptance targets:** 100% of accuracy assertions carry reviewed scope and source/evidence; zero cases where a schema-valid or signed assertion alone upgrades the OMSI decision to “measurement proven”; 100% independent recalculation where OMSI itself computes a measure.

### 4. ISO 8000-140:2016 — completeness assertions

Use its conceptual model for statements/assertions about data completeness. Completeness expectations depend on use case and population; this part does not establish one universal completeness threshold or supply a complete exchange format. Bind each completeness claim to a versioned inventory, population and revision.

**Acceptance targets:** 100% completeness assertions bind to an inventory and population; 100% detection of missing required characteristics; zero expected thresholds misreported as observed completeness results.

### 5. ISO 8000-66:2021 — manufacturing data-quality indicators

Use its assessment-indicator model to measure the data-quality process, not as a manufacturing source-record parser. Every indicator should bind definition/version, population, time window, input evidence and calculation activity. An objective is never a measured result; missing measurements cannot silently become a pass.

**Acceptance targets:** 100% traceability from indicator to evidence/calculation; zero target-to-measurement mislabeling; 100% independent recalculation agreement on committed indicator fixtures before release.

### 6. ISO 8000-210:2024 — sensor-data quality characteristics

Use its discrete sensor-data characteristics and anomaly taxonomy to label quality concerns in sampled observations. It does not define the methods to measure or improve data quality, and it excludes analogue, image, video and audio sensors from this scope.

**Acceptance targets:** 100% agreement with a pinned anomaly/characteristic fixture oracle; 100% preservation of original sample bytes when annotations are added; zero unsupported characteristics silently accepted as conformant.

### 7. ISO 8000-220:2025 — sensor-data quality measurement

Use its quality measures for the characteristics specified by ISO 8000-210. Each measure needs its named characteristic, calculation method, sample/window population and assumptions. Separate actual result from target, and keep invalid or insufficient samples from receiving optimistic scores.

**Acceptance targets:** 100% independent calculation agreement on pinned numeric fixtures; 100% repeatability for identical bytes/profile/window; zero invalid or insufficient populations accepted as high quality.

### 8. ISO/TS 8000-230:2026 — sensor-data cleansing guidelines

The newly published 2026 technical specification provides guidelines for cleansing sensor anomalies, including principles/process/implementation guidance and examples. It does not specify detailed repair algorithms or a real-time cleansing process. OMSI should record cleansing as an explicit, reversible derivation activity rather than silently overwrite the observation.

**Acceptance targets:** 100% of cleaned values retain a link to immutable raw samples and method/version; 100% reconstructability of cleansing activities; zero unreviewed repairs overwrite a physical observation; false-positive and missed-anomaly rates are measured on a labeled corpus before release.

### 9. ISO 23952:2020 QIF — quality-information interoperability

Build a separate XML/XSD adapter for a pinned QIF schema release once the authoritative schema bundle and representative documents are available. Map identifiers, measurement/unit semantics, specification/revision and uncertainty where represented. Do not reduce QIF to the current JSON field-inventory mechanism. ISO confirmed the 2020 edition remains current following its 2026 review.

**Acceptance targets:** 100% agreement with the pinned independent XSD validator; zero silent field loss; 100% round-trip semantic agreement for explicitly declared lossless fields. Lossy mappings require explicit declarations and reviewed vectors.

### 10. MTConnect + OPC UA Companion Specifications — device interoperability

Pin specific MTConnect and OPC UA Companion Specification versions, then test the data path—not just isolated parsers. NIST's published companion-specification test passed varied structures through a server/client path and checked accurate receipt by the client. Reuse that methodology with explicit identity, asset scope, timestamp and unit assertions.

**Acceptance targets:** 100% expected observation receipt on positive fixtures; zero unexpected acceptance of malformed, unauthorized or version-mismatched nodes; 100% semantic retention on declared lossless fields; independent agent agreement for artifact/graph checks.

## Cross-cutting release gates

| Gate | Target |
|---|---:|
| Exact schema/spec, source, adapter-map and report digests recorded | 100% |
| Independent authoritative schema-validator agreement | 100% |
| Mapping inventory accounted exactly once | 100% |
| Unexpected negative-control acceptance | 0 |
| Silent loss of source fields | 0 |
| Unsupported schema constructs accepted as conformant | 0 |
| Provenance for every derived/cleansed value is reconstructable | 100% |
| Semantic result reproduced by an independent evaluator | 100% |
| Sensitive or unauthorized source data leaked into public receipts | 0 |

These are proposed engineering acceptance criteria—not normative requirements from the named standards, measured outcomes or industrial performance benchmarks.

## Evidence boundary

The machine-readable matrix intentionally records all ten targets as `not_implemented` and `not_assessed`. Real qualification requires authoritative normative assets, their version/access provenance, representative records, independent reference validators and negative controls. A passing schema, mapping, quality metric or replay check does not authenticate a producer, prove a physical measurement true, or authorize product release.

## References

- [ISO 8000-110:2021 — characteristic-data exchange](https://www.iso.org/standard/78501.html)
- [ISO 8000-120:2016 — provenance](https://www.iso.org/standard/62393.html)
- [ISO 8000-130:2016 — accuracy information](https://www.iso.org/standard/62394.html)
- [ISO 8000-140:2016 — completeness information](https://www.iso.org/standard/62395.html)
- [ISO 8000-66:2021 — manufacturing data-quality indicators](https://www.iso.org/standard/76390.html)
- [ISO 8000-210:2024 — sensor-data quality characteristics](https://www.iso.org/standard/84274.html)
- [ISO 8000-220:2025 — sensor-data quality measurement](https://www.iso.org/standard/86062.html)
- [ISO/TS 8000-230:2026 — sensor-data cleansing guidelines](https://www.iso.org/standard/89681.html)
- [ISO 23952:2020 QIF](https://www.iso.org/standard/77461.html)
- [NIST: Testing of the MTConnect–OPC UA Companion Specification](https://www.nist.gov/publications/testing-mtconnect-opc-ua-companion-specification)
- [NIST: Digital Thread for Manufacturing](https://www.nist.gov/programs-projects/digital-thread-manufacturing)

This document is a roadmap, not a conformance declaration.
