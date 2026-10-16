# OMSI Manufacturing Interoperability Targets 0.1

**Status:** research and qualification backlog; no standards or vendor conformance is claimed.
**Machine-readable contract:** [interoperability-targets-0.1.json](./interoperability-targets-0.1.json)

## Why this is a separate qualification layer

OMSI's current examples are synthetic JSON fixtures. Schema-derived coverage is a restricted closed-object JSON Schema field-inventory profile. That guards against one kind of silent field loss, but it does not establish ISO 8000, QIF, MTConnect, OPC UA, XML/XSD or vendor conformance.

The next paradigm shift is to treat each real target as a **version-pinned executable conformance bundle**: named normative version, authorized machine-readable schema/model assets, positive and negative fixtures, independent validator, approved mapping decisions, expected semantic outcome, artifact digests and provenance linking those steps. “Schema parses” must not be confused with “source implementation conforms”; path accounting must not be confused with semantic equivalence.

## Target order

### 1. ISO 8000-110:2021 — characteristic-data exchange

Use as a reference for computer-checkable message syntax, semantic encoding and conformance to a characteristic-data specification. Before adding an adapter, pin the normative version and data specification, obtain authorized machine-readable assets, and create representative positive/negative messages.

**Acceptance targets:** 100% agreement with an independent validator over the pinned corpus; 100% inventory field accounting; zero unsupported schema constructs accepted as conformant.

### 2. ISO 8000-66:2021 — manufacturing data-quality indicators

Use as a framework for measuring the data-quality process, not as a manufacturing source-record parser. Every indicator should bind its definition/version, population, time window, input evidence and calculation activity. An objective is never a measured result; missing measurements cannot silently become a pass.

**Acceptance targets:** 100% traceability from indicator to evidence/calculation; zero target-to-measurement mislabeling; 100% independent recalculation agreement on committed indicator fixtures before release.

### 3. ISO 23952:2020 QIF — quality-information interoperability

Build a separate XML/XSD adapter for a pinned QIF schema release once the authoritative schema bundle and sample documents are available. Map identifiers, measurement/unit semantics, specification/revision and uncertainty when represented. Do not reduce QIF to the current JSON field-inventory mechanism.

**Acceptance targets:** 100% agreement with the pinned independent XSD validator; zero silent field loss; 100% round-trip semantic agreement for explicitly declared lossless fields. Lossy mappings require explicit declarations and reviewed vectors.

### 4. MTConnect + OPC UA Companion Specifications — device interoperability

Pin specific MTConnect and OPC UA Companion Specification versions, then test the whole data path—not just isolated parsers. NIST’s published companion-specification test used varying structures through a server/client path and checked accurate receipt by the client. Reuse that methodology with explicit identity, scope, timestamp and unit assertions.

**Acceptance targets:** 100% expected observation receipt on positive fixtures; zero unexpected acceptance of malformed, unauthorized or version-mismatched nodes; 100% semantic retention on declared lossless fields; independent agent agreement for the artifact/graph checks.

## Cross-cutting release gates

| Gate | Target |
|---|---:|
| Exact schema/spec, source, adapter-map and report digests recorded | 100% |
| Independent authoritative schema-validator agreement | 100% |
| Mapping inventory accounted exactly once | 100% |
| Unexpected negative-control acceptance | 0 |
| Silent loss of source fields | 0 |
| Unsupported schema constructs accepted as conformant | 0 |
| Semantic result reproduced by an independent evaluator | 100% |
| Sensitive or unauthorized source data leaked into public receipts | 0 |

These are proposed engineering acceptance criteria—not normative requirements from the named standards, measured outcomes or industrial performance benchmarks.

## Evidence boundary

The machine-readable matrix intentionally records each standard target as `not_implemented` and `not_assessed`. Real qualification requires authoritative normative assets, their version/access provenance, representative records, independent reference validators and negative controls. A passing schema, mapping or replay check does not authenticate a producer, prove a physical measurement true, or authorize product release.

## References

- [ISO 8000-110:2021 — characteristic-data exchange](https://www.iso.org/standard/78501.html)
- [ISO 8000-66:2021 — manufacturing data-quality indicators](https://www.iso.org/standard/76390.html)
- [ISO 23952:2020 QIF](https://www.iso.org/standard/77461.html)
- [NIST: Testing of the MTConnect–OPC UA Companion Specification](https://www.nist.gov/publications/testing-mtconnect-opc-ua-companion-specification)
- [NIST: Digital Thread for Manufacturing](https://www.nist.gov/programs-projects/digital-thread-manufacturing)

This document is a roadmap, not a conformance declaration.