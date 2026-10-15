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

It emits a versioned JSON report with case IDs, corpus digest scope, OS/architecture, iteration and warm-up counts, sample count, min/p50/p95/p99/max/mean latency, observations per second and serialized-input throughput. The report explicitly states `performance_observation_only` and `measured_no_acceptance_claim`.

### Reproducibility contract

- The corpus digest is SHA-256 over the **exact bytes of the vector document**; it does not claim canonical JSON semantics.
- Benchmark inputs are the fixture input objects serialized by the current `serde_json` build. They are a stable per-build workload, not a claim that every JSON implementation produces the same bytes.
- The harness warms each case, uses `black_box`, records per-operation samples, and reports quantiles by nearest-rank selection.
- Results vary with CPU, compiler version/profile, host scheduling and background load. This CLI is a reproducible workload/report format, not a laboratory-grade isolation system.
- Timing thresholds are intentionally not enforced by shared hosted CI. The current committed vector set contains eight cases and is useful as a microbenchmark smoke corpus, **not** yet large or representative enough to establish a production SLO.
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
| Adapter fidelity | Zero silent field loss across at least two differently shaped synthetic source adapters | Hard | Fields must be preserved, explicitly transformed or recorded as lost/unmapped |
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
3. Run one warm-up process followed by at least 10 measured repetitions; report p50/p95/p99 and variance across runs, not only a best-case run.
4. Separate local evaluator timings from source-adapter I/O, Holochain DHT latency, credential status checks and any external service. Measure them separately, then measure an end-to-end synthetic receipt flow.
5. Compare only equivalent semantic rule/profile versions. Treat different corpus digests or toolchain environments as different benchmark configurations.
6. Do not fail shared hosted PR CI on wall-clock thresholds until a stable reference host and measurement procedure have been established. CI should first verify that the harness runs and emits structurally valid metrics.
7. Do not optimize by skipping validation, treating unknown as accepted, removing negative controls or reducing evidence retention.

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
