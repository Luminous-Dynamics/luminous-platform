# OMSI Manufacturing Benchmark Scorecard 0.1

**Status:** targets and acceptance gates; most numeric performance values are provisional and not measured  
**Machine-readable scorecard:** [benchmark-scorecard-0.1.json](./benchmark-scorecard-0.1.json)  
**Current CLI:** `benchmark-vectors` in `tools/omsi-manufacturing-validator`

## Paradigm shift: benchmark trustworthiness before throughput

A manufacturing evidence system is only useful if it can avoid optimistic errors, explain why a conclusion was reached, preserve source meaning across adapters, and let a separate verifier reproduce the result. Therefore, performance is not a single score. The scorecard has hard correctness/security gates, reproducibility and cross-implementation gates, provisional software-performance targets, and later real-system interoperability gates.

A fast validator with a single false acceptance on a cross-tenant record is not “high performance.” No aggregate benchmark score may offset a safety, semantics or privacy failure.

## What has been implemented in this increment

The Rust CLI gains:

```sh
cargo run --release --manifest-path tools/omsi-manufacturing-validator/Cargo.toml -- \
  benchmark-vectors docs/standards/omsi/profiles/manufacturing-asset-operations/0.1/CONFORMANCE-VECTORS.json 1000
```

The command uses the exact vector-document bytes for a corpus SHA-256 and serialized input examples from each vector for workload cases. It measures four phases separately:

- JSON parsing;
- semantic evaluation;
- replay-envelope creation (including parsing, evaluation and digest);
- replay-envelope verification (including digest, parsing and fresh evaluation).

It emits a versioned JSON report with case IDs, corpus digest scope, OS/architecture, iteration and warm-up counts, sample count, min/p50/p95/p99/max/mean latency, latency-derived inverse-mean rates, separately measured phase wall-clock operations/sec and serialized-input MiB/sec, OS/architecture/build profile, optional host/CPU labels and a compiler version when `RUSTC_VERSION` is captured. The report explicitly states `performance_observation_only` and `measured_no_acceptance_claim`.

### New adapter field-accounting audit

The Rust CLI also has `audit-adapter-map <adapter-map.json> <source-record.json>`. It enumerates observed source-record leaf paths and requires each to have exactly one explicit `preserve`, `transform`, `drop`, or `unmapped` disposition. Duplicate declarations and uncovered observed paths fail accounting. Transformations require a named rule and target path; intentional drops and unresolved mappings carry a reason and remain separately counted. The report binds the exact map and source-record bytes by SHA-256 and grants no semantic or release authority.

Two differently shaped synthetic examples are checked by CI: [flat MES-like mapping](./adapters/0.1/examples/mes-flat-map.json) and [nested QMS-like mapping](./adapters/0.1/examples/qms-nested-map.json). The new `apply-adapter-map` command executes a small versioned transform registry: exact decimal normalization and an allowlisted unit-code crosswalk. A separate [10-case transform-vector corpus](./adapters/0.1/TRANSFORM-VECTORS.json) and `verify-transform-vectors` command enforce exact outputs/rejection reasons. CI asserts the transform corpus, concrete QMS output values, raw-status preservation, an explicitly counted MES drop, and a third record fixture where required/optional field presence is exercised end-to-end. Unknown transforms/units, unmapped fields, absent required source paths and overlapping targets block projection; absent optional source paths are skipped with an explicit counter and remain listed in the audit. No partial projection is emitted. This still does not prove vendor-schema conformance, physical measurement truth, or general transformation equivalence outside the tested vectors.

### Reproducibility contract

- The corpus digest is SHA-256 over the **exact bytes of the vector document**; it does not claim canonical JSON semantics.
- Benchmark inputs are the fixture input objects serialized by the current `serde_json` build. They are a stable per-build workload, not a claim that every JSON implementation produces the same bytes.
- The harness warms each case, uses `black_box`, records per-operation samples, and reports quantiles by nearest-rank selection.
- Resource bounds are explicit: at most 10,000 vector cases and 1,000,000 samples per phase; checked multiplication and rejection happen before per-case preparation or sample-buffer allocation. The iteration-per-case cap remains 100,000.
- Adapter/accounting/projection and transform-vector JSON files, plus benchmark corpora, are capped at 16 MiB per artifact. Adapter field-path traversal stops after 100,000 observed leaves instead of building an unbounded secondary path list.
- Each phase uses one pass with per-operation timers for latency quantiles and a separate batch pass for wall-clock throughput. The throughput pass omits per-operation timers and sample aggregation, while still including loop/dispatch and `black_box` overhead; it excludes process startup. The `*_from_sampled_time` rates are inverse-mean-latency equivalents, **not** throughput. The report names this separation with `throughput_measurement_mode`.
- Results vary with CPU, compiler version/profile, host scheduling and background load. This CLI is a reproducible workload/report format, not a laboratory-grade isolation system.
- Timing thresholds are intentionally not enforced by shared hosted CI. The benchmark corpus still contains eight committed vectors and is a microbenchmark smoke corpus, **not** large or representative enough to establish a production SLO.
- A separate deterministic assurance sweep has now been authored for **100,000 generated cases** in the test suite: 20,000 each for nominal numeric-range behavior (including boundary-adjacent values), equivalent decimal spellings, equivalent explicit timestamp offsets, missing-unit non-evaluability, and tenant-mismatch rejection. It is not counted as part of the eight-vector timing corpus, and it has not yet been compiled or executed in this environment.
- The benchmark command has been authored but not compiled or run locally in this environment. A current exact-head CI conclusion is not available. Do not quote performance numbers until an actual report is captured with the report and environment pinned.

## Next paradigm shift: evidence as a reproducible derivation graph

A report should not be treated as a free-standing JSON verdict. It should be the output node of a derivation graph whose source record, source schema, adapter contract, schema-coverage report, projection, semantic input, evaluator report, and replay envelope are explicit artifacts. Provenance manifest contract v0.2 now re-runs schema coverage over the exact schema/record/map bytes and refuses to emit a manifest unless the supplied coverage report says `schema_coverage_complete` and exactly matches a fresh audit. That coverage evidence then becomes an explicit input to the adapter-projection activity.

A second new qualification axis is **schema-complete mapping**, not merely per-record accounting. A sample-record audit can miss an optional field that did not occur in that sample. The new `audit-adapter-schema-coverage` command derives every leaf path in a strict, closed-object schema subset and requires exactly one adapter mapping with matching required/optional semantics. Missing optional mappings are blockers even when the sample record omits the field. Schemas with open objects, references, composition/conditional constructs or other unsupported field-inventory semantics are rejected rather than approximated. This is field-path coverage—not full JSON Schema validation, vendor conformance or proof of semantic equivalence.

The standards-research direction is to make the schema and conformance assets executable and reviewable, not to treat a prose standard as the entire interoperability artifact. NIST's [machine-readable manufacturing standards production method](https://www.nist.gov/publications/production-method-machine-readable-manufacturing-data-standards) advocates a digital standards workflow; NIST's [MTConnect–OPC UA companion-specification test](https://www.nist.gov/publications/testing-mtconnect-opc-ua-companion-specification) validates exchanges by sending varied structures through the system and checking accurate receipt. OMSI's current JSON inventory is a pilot-sized first step—not a replacement for XSD/QIF or vendor-specific conformance suites.

The next steps should avoid inventing an isolated proprietary trust format. Keep OMSI's semantic profile deterministic and offline; map provenance artifacts to [W3C PROV](https://www.w3.org/TR/2013/REC-prov-dm-20130430/) entity/activity relations and package authenticated step attestations using a standard such as [in-toto](https://github.com/in-toto/attestation/blob/main/spec/predicates/link.md). In parallel, test real interoperability against manufacturing-quality schemas and conformance vectors rather than claiming QIF/STEP/vendor conformance from synthetic JSON fixtures. NIST's Digital Thread for Manufacturing explicitly identifies semantic conformance testing and data-asset traceability as gaps, and ISO confirms that QIF (ISO 23952:2020) remains current as of its 2026 review. The architectural opportunity is to make meaning-preservation and evidence lineage machine-checkable at every boundary, then add signer identity and independent verifiers as separate trust layers.

## Real-standards qualification matrix

The new [Interoperability Targets 0.1](./interoperability-targets-0.1.md) and its [machine-readable contract](./interoperability-targets-0.1.json) define ten future qualification targets separately from current capabilities: ISO 8000-110/120/130/140 for characteristic-data exchange, provenance, accuracy and completeness; ISO 8000-66 data-quality indicators; ISO 8000-210/220 sensor-data quality; ISO/TS 8000-230 sensor cleansing; ISO 23952 QIF; and MTConnect/OPC UA device interoperability. ISO 8000-110 explicitly excludes provenance, accuracy and completeness information, referring to separate parts for those concerns; do not flatten these into one general data-validity flag. Each target is labeled not implemented/not assessed; a synthetic JSON fixture is not conformance evidence.

The next cross-industry milestone is to pin one target's exact standard/schema version, acquire its authoritative machine-readable assets, select an independent validator, and establish positive/negative source fixtures. For QIF, the intended test path is XML/XSD plus semantic field accounting; for MTConnect/OPC UA, it is a simulator or server/client integration path checking accurate receipt of varied observations. These gates should remain separate from local OMSI evaluation and its performance timings.

## Proposed benchmarks and release gates

| Area | Proposed target | Gate type | Rationale |
|---|---|---|---|
| Semantic conformance | 100% of expected versioned fields match every committed vector | Hard | No silent semantic drift |
| Negative controls | Zero unexpected acceptances in cross-tenant, malformed, incompatible-unit and contradiction suites | Hard | False acceptance matters more than speed |
| Independent schema checks | 100% of schemas and representative outputs validate with an independent Draft 2020-12 engine; mutated invalid reports are rejected | Hard | Parsing JSON is not schema validation |
| Replay tamper suite | 100% detection for every named mutation to input, digest, envelope contract, report and limitations | Hard | Digest and report claims must be checked, not trusted |
| Provenance-chain reconstruction | 100% of valid schema/record/map/coverage/projection/input/report/envelope fixtures produce and verify manifests; inconsistent inputs, a non-complete/stale coverage report, artifact drift, or a mutated manifest digest are rejected | Hard | Schema qualification must be linked to the projection and semantic result, not merely run somewhere earlier in the pipeline |
| Repeatability | 100% semantic report equality for same bytes/profile/rule/build | Hard | Reproducible interpretation |
| Property testing | At least 100,000 deterministic generated cases per profile version | Release | Boundary and metamorphic coverage |
| Coverage-guided fuzzing | At least 24 hours per release candidate, with crash corpus retained | Release | Parser/resource robustness |
| Semantic latency | p95 ≤ 5 ms for one input ≤ 32 KiB | Provisional | Local software target; measure on pinned reference host |
| Replay verification latency | p95 ≤ 20 ms for one input ≤ 64 KiB | Provisional | Includes fresh evaluation, parsing and digest |
| Semantic throughput | ≥ 500 observations/s over ≥ 1,000 representative cases | Provisional | Compare only matching workloads and environments |
| Memory | Peak RSS ≤ 64 MiB for one bounded input/replay process | Provisional | Must be measured externally, not inferred from timing |
| Adapter fidelity | 100% of observed leaf paths accounted exactly once across at least two differently shaped adapters; zero silent loss | Hard | Missing/duplicate field classifications fail; explicit drops and unmapped fields remain visible and do not count as preserved |
| Source-schema coverage | 100% of leaf paths in the supported closed-object inventory have one mapping with matching required/optional presence; zero mappings outside schema; unsupported schema constructs fail closed | Hard | Prevents absent optional fields from escaping review merely because they do not occur in a sample record |
| Independent artifact/graph verifier | 100% verification of exact-byte bindings, fixed provenance activities/edges, source-schema field inventory, required/optional mapping agreement, and reported absent optional paths; forged summaries must be rejected even after hashes are recomputed | Hard for local evidence-chain qualification | A self-consistent hash does not prove a self-reported coverage count is true |
| Independent semantic verifier | 100% agreement with the public semantic-vector/mutation catalogue from a separately implemented evaluator | Release | Artifact binding is not the same as independently reproducing a manufacturing decision |
| Public receipt privacy | Zero prohibited raw telemetry, confidential parameters, operator PII or unauthorized tenant identifiers | Hard | Public DHT records require a conservative disclosure boundary |
| Multi-agent behavior | At least two Holochain agents exercise receipt publication, resolution, disputes and revocation observations | Release | Unit tests do not qualify DHT/bridge semantics |

These are engineering targets proposed for the pilot—not sourced industrial benchmarks, measured current performance, or declarations of regulatory compliance. Adjust them only after capturing a baseline and documenting why.

## Corpus growth targets

The current eight conformance vectors are too small to represent full interop and security behavior. Source-schema coverage now rejects mappings outside the declared inventory, missing optional mappings, presence mismatches, invalid required-child structure, and unsupported schema constructs; the independent verifier separately re-derives the field inventory and checks reported path coverage. It does not yet implement the full semantic evaluator, validate constraints inside array items, authenticate signatures, or prove vendor conformance. Grow the corpus by behavior and semantic risk, not by duplicating records:

1. **Core semantic:** lower/upper bound minus, at and plus one representable decimal step; precision limits; equivalent decimal spellings; incompatible units; missing required units; malformed values.
2. **Time and snapshots:** equivalent explicit offsets; leap/invalid dates; naive time; reversed calibration interval; non-atomic snapshot; stale export with a new source revision and its converse.
3. **Scope and provenance:** tenant mismatch at each object reference; source-instance mismatch; missing/unknown revision; export time incorrectly offered as revision; conflicting source and derived values.
4. **Replay:** input-byte mutation, whitespace mutation, digest mutation, altered report field, dropped limitations, unknown contract version, unsupported digest metadata.
5. **Federation and adapters:** unknown source fields; lossy adapter conversion; pagination truncation; missing/duplicate records; partial read; schema downgrade; credential expired/revoked after a recorded check; wrong issuer purpose; duplicate or conflicting receipt.
6. **Privacy:** identifiers that reveal cross-tenant linkage; raw sensor values; confidential recipes/design values; operator personal information; overlarge or high-rate evidence payload attempts.

Every discovered defect becomes a minimal regression case, and the suite preserves negative controls. Fuzz-generated cases should be minimized and promoted into deterministic tests rather than kept only in opaque fuzz artifacts.

## Performance protocol

For an initial comparable baseline:

1. Build release mode; record git commit SHA, `rustc --version`, Cargo lockfile digest, OS/kernel, CPU/model, memory, compiler flags and power profile.
2. Use the same corpus-file digest, the same input-size distribution and at least 1,000 behaviorally distinct cases. Until corpus expansion is complete, label results **microbenchmark only**.
3. Set `RUSTC_VERSION="$(rustc --version)"` and the optional anonymized `OMSI_BENCHMARK_HOST_LABEL` / `OMSI_BENCHMARK_CPU_LABEL` variables. The report must never silently infer a hostname or expose it by default.
4. Run one warm-up process followed by at least 10 measured repetitions; report p50/p95/p99 and variance across runs, not only a best-case run.
5. Separate local evaluator timings from source-adapter I/O, Holochain DHT latency, credential status checks and any external service. Measure them separately, then measure an end-to-end synthetic receipt flow.
6. Compare both latency quantiles and `wall_clock_operations_per_second` / `wall_clock_serialized_input_mib_per_second` only across equivalent semantic rule/profile versions, corpus digests, input-size distributions and toolchain environments. Keep latency-derived rates labeled as such.
7. Do not fail shared hosted PR CI on wall-clock thresholds until a stable reference host and measurement procedure have been established. CI should first verify that the harness runs and emits structurally valid metrics.
8. Do not optimize by skipping validation, treating unknown as accepted, removing negative controls or reducing evidence retention.

## Mycelix integration benchmark additions

After a read-only adapter exists, benchmark these separately from OMSI's local evaluation:

- credential verification latency and cache policy, including revoked/expired and issuer-purpose negative cases;
- receipt creation/resolution cost in the local and multi-agent Holochain paths;
- DHT dependency-unresolved rate and retry completion under intentionally delayed dependencies;
- cross-cluster route rejection rate for unauthorized calls;
- semantic retention: percent of source fields preserved, explicitly transformed, or enumerated as unmapped;
- receipt verification agreement between two agents and independent implementations;
- disclosure test leakage: prohibited field count must stay zero;
- offline operation: OMSI evaluation/replay should still work when Mycelix or network access is unavailable.

Do not make Mycelix reputation or consensus a performance/correctness substitute for metrology. An authenticated issuer can provide a false reading; a quorum can agree on an incorrect reading.

## Research basis

- [ISO 8000-110:2021](https://www.iso.org/standard/78501.html) specifies computer-checkable characteristic-data message syntax, semantic encoding and conformance to a data specification. Its published scope explicitly excludes provenance, accuracy and completeness.
- [ISO 8000-120:2016](https://www.iso.org/standard/62393.html), [ISO 8000-130:2016](https://www.iso.org/standard/62394.html), and [ISO 8000-140:2016](https://www.iso.org/standard/62395.html) address provenance, accuracy assertions and completeness assertions as separate concepts. These are not proof of physical truth or universal requirements.
- [ISO 8000-66:2021](https://www.iso.org/standard/76390.html) describes assessment indicators for data-quality management in manufacturing operations.
- [ISO 8000-210:2024](https://www.iso.org/standard/84274.html) specifies sensor-data quality characteristics; [ISO 8000-220:2025](https://www.iso.org/standard/86062.html) defines corresponding measures; [ISO/TS 8000-230:2026](https://www.iso.org/standard/89681.html) provides sensor-cleansing guidelines and does not specify real-time cleansing algorithms.
- [ISO 23952:2020 QIF](https://www.iso.org/standard/77461.html) remains current after its 2026 review. Its scope is an interoperability reference, not evidence that OMSI conforms.
- [NIST testing of the MTConnect–OPC UA Companion Specification](https://www.nist.gov/publications/testing-mtconnect-opc-ua-companion-specification) demonstrates an integration-test approach that sends varying data structures through a server/client path and checks accurate receipt.
- NIST's [Digital Thread for Manufacturing](https://www.nist.gov/programs-projects/digital-thread-manufacturing) and [machine-readable manufacturing standards production method](https://www.nist.gov/publications/production-method-machine-readable-manufacturing-data-standards) support executable standards artifacts and traceable evidence chains.
- [ILAC G8/G17 guidance](https://ilac.org/publications-and-resources/ilac-guidance-series/) reinforces the distinction between a measurement result, uncertainty and a named decision rule. This is why the current nominal-only rule must not be silently reinterpreted.
- [Holochain validation guidance](https://developer.holochain.org/build/validation/) describes determinism, addressable DHT dependencies and single/multi-agent testing, supporting separation of deterministic integrity checks from dynamic trust/status lookups.

## Current evidence status

The scorecard is deliberately set to `targets_not_measured`. Current semantic benchmark corpus count is eight. The benchmark harness, 100,000-case assurance sweep, adapter projections, transform vectors, independent Draft 2020-12 schema tests, source-schema coverage audit, schema-bound provenance v0.2 chain, and the separate Rust artifact/graph verifier are authored but have not been locally compiled or executed in this environment. The exact-head workflow must confirm compilation, schema/meta-validation, emitted-report validation, source-schema mutation controls, forged-coverage-summary rejection, and reconstruction of the eight-artifact chain. The independent verifier rechecks byte bindings, graph structure and the restricted schema leaf inventory; it does not execute transforms or recompute the semantic decision, so independent semantic equivalence remains an open gate. No reference hardware has been pinned, and the Mycelix runtime adapter is not implemented. The next honest milestone is exact-head execution evidence, followed by a controlled baseline report, wider source-schema/vendor fixtures, and expansion of the performance corpus.
