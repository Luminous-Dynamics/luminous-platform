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

Two differently shaped synthetic examples are checked by CI: [flat MES-like mapping](./adapters/0.1/examples/mes-flat-map.json) and [nested QMS-like mapping](./adapters/0.1/examples/qms-nested-map.json). This is field-accounting evidence only: it does not execute transformations, prove cross-system semantic equivalence, or cover optional fields absent from the supplied fixture.

### Reproducibility contract

- The corpus digest is SHA-256 over the **exact bytes of the vector document**; it does not claim canonical JSON semantics.
- Benchmark inputs are the fixture input objects serialized by the current `serde_json` build. They are a stable per-build workload, not a claim that every JSON implementation produces the same bytes.
- The harness warms each case, uses `black_box`, records per-operation samples, and reports quantiles by nearest-rank selection.
- Resource bounds are explicit: at most 10,000 vector cases and 1,000,000 samples per phase; checked multiplication and rejection happen before per-case preparation or sample-buffer allocation. The iteration-per-case cap remains 100,000.
- Each phase uses one pass with per-operation timers for latency quantiles and a separate batch pass for wall-clock throughput. The throughput pass omits per-operation timers and sample aggregation, while still including loop/dispatch and `black_box` overhead; it excludes process startup. The `*_from_sampled_time` rates are inverse-mean-latency equivalents, **not** throughput. The report names this separation with `throughput_measurement_mode`.
- Results vary with CPU, compiler version/profile, host scheduling and background load. This CLI is a reproducible workload/report format, not a laboratory-grade isolation system.
- Timing thresholds are intentionally not enforced by shared hosted CI. The benchmark corpus still contains eight committed vectors and is a microbenchmark smoke corpus, **not** large or representative enough to establish a production SLO.
- A separate deterministic assurance sweep has now been authored for **100,000 generated cases** in the test suite: 20,000 each for nominal numeric-range behavior (including boundary-adjacent values), equivalent decimal spellings, equivalent explicit timestamp offsets, missing-unit non-evaluability, and tenant-mismatch rejection. It is not counted as part of the eight-vector timing corpus, and it has not yet been compiled or executed in this environment.
- The benchmark command has been authored but not compiled or run locally in this environment. A current exact-head CI conclusion is not available. Do not quote performance numbers until an actual report is captured with the report and environment pinned.

## Proposed benchmarks and release gates

| Area | Proposed target | Gate type | Rationale |
|---|---|---|---|
| Semantic conformance | 100% of expected versioned fields match every committed vector | Hard | No silent semantic drift |
| Negative controls | Zero unexpected acceptances in cross-tenant, malformed, incompatible-unit and contradiction suites | Hard | False acceptance matters more than speed |
| Independent schema checks | 100% valid reports; mutated schema-invalid reports rejected by an independent Draft 2020-12 engine | Hard | Parsing JSON is not schema validation |
| Replay tamper suite | 100% detection for every named mutation to input, digest, envelope contract, report and limitations | Hard | Digest and report claims must be checked, not trusted |
| Repeatability | 100% semantic report equality for same bytes/profile/rule/build | Hard | Reproducible interpretation |
| Property testing | At least 100,000 deterministic generated cases per profile version | Release | Boundary and metamorphic coverage |
| Coverage-guided fuzzing | At least 24 hours per release candidate, with crash corpus retained | Release | Parser/resource robustness |
| Semantic latency | p95 ≤ 5 ms for one input ≤ 32 KiB | Provisional | Local software target; measure on pinned reference host |
| Replay verification latency | p95 ≤ 20 ms for one input ≤ 64 KiB | Provisional | Includes fresh evaluation, parsing and digest |
| Semantic throughput | ≥ 500 observations/s over ≥ 1,000 representative cases | Provisional | Compare only matching workloads and environments |
| Memory | Peak RSS ≤ 64 MiB for one bounded input/replay process | Provisional | Must be measured externally, not inferred from timing |
| Adapter fidelity | 100% of observed leaf paths accounted exactly once across at least two differently shaped adapters; zero silent loss | Hard | Missing/duplicate field classifications fail; explicit drops and unmapped fields remain visible and do not count as preserved |
| Independent verifier | 100% agreement with the public vector/mutation catalogue | Release | A single implementation must not define and attest its own correctness |
| Public receipt privacy | Zero prohibited raw telemetry, confidential parameters, operator PII or unauthorized tenant identifiers | Hard | Public DHT records require a conservative disclosure boundary |
| Multi-agent behavior | At least two Holochain agents exercise receipt publication, resolution, disputes and revocation observations | Release | Unit tests do not qualify DHT/bridge semantics |

These are engineering targets proposed for the pilot—not sourced industrial benchmarks, measured current performance, or declarations of regulatory compliance. Adjust them only after capturing a baseline and documenting why.

## Corpus growth targets

The current eight conformance vectors are too small to represent full interop and security behavior. Grow the corpus by behavior and semantic risk, not by duplicating records:

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

- NIST's [Digital Thread for Manufacturing](https://www.nist.gov/programs-projects/digital-thread-manufacturing) emphasizes product-lifecycle standards, conformance tooling and trust in data assets.
- [ISO 23952:2020 QIF](https://www.iso.org/standard/77461.html) is a manufacturing-quality information model and was confirmed current by ISO in 2026. Its scope is a useful interoperability reference, not evidence that OMSI is QIF-conformant.
- [ILAC G8 and G17 guidance](https://ilac.org/publications-and-resources/ilac-guidance-series/) reinforces the distinction between a measurement result, uncertainty and a named decision rule. This is why the existing nominal-only rule is not to be silently reinterpreted.
- [Holochain validation guidance](https://developer.holochain.org/build/validation/) describes determinism, addressable DHT dependencies and the need for single- and multi-agent tests. It supports separating deterministic integrity checks from dynamic trust/status lookups.

## Current evidence status

The scorecard is deliberately set to `targets_not_measured`. Current checked-in vector count is eight. The new benchmark harness, its tests and CI smoke step are implementation work but have not been locally compiled or executed here. No reference hardware has been pinned, no independent JSON Schema engine has been confirmed running, no independent cross-implementation verifier is complete, and the Mycelix adapter is not implemented. The next honest milestone is to get the current branch tested, capture the first baseline report, then expand the corpus before claiming the targets above are met.
