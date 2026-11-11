# Calibrated Workload-State Power Model and Benchmark v0.1
**Date:** 11 November 2026  
**Implementation branch:** `research/luminous-power-fabric-power-state-model-v0.18`  
**Parent:** [Issue #401](https://github.com/Luminous-Dynamics/luminous-platform/issues/401)  
**Upstream workload evidence:** [PR #400](https://github.com/Luminous-Dynamics/luminous-platform/pull/400)  
**Electrical measurement authority:** [Issue #382](https://github.com/Luminous-Dynamics/luminous-platform/issues/382)

## 1. Purpose and claim ceiling

This increment establishes a typed, offline bridge from independently mapped workload states to measured, configuration-specific power distributions. It is deliberately narrower than a full data-centre digital twin or a Microsoft-style compositional trace generator.

The implementation fits:
- an empirical power envelope conditioned on each workload state; and
- an empirical destination-power envelope conditioned on each ordered one-step state transition (s_{t-1} \rightarrow s_t).

For a supplied state sequence, it emits deterministic median predictions and p10–p90 envelopes in the original sample order. It does not independently shuffle samples, smooth away bursts, fill missing measurements, extrapolate to unseen configurations, or infer workload phase from CPU/GPU occupancy alone.

**Claim ceiling:** calibrated model output, not measurement; held-out error metrics, not independent reproduction; no physical power control, site qualification, grid-service accreditation or carbon claim. Issue #382 remains the sole owner of measured PCC energy, storage correction, RMS/peak/ramp and variability audit.

## 2. Calibration contract

Every model binds to the exact:
- hardware configuration fingerprint (accelerator SKU, board/server topology, CPU and memory configuration, driver, firmware, runtime, model/software build, clocks, power cap and cooling envelope);
- physical measurement boundary and sensor identity;
- sensor calibration evidence and uncertainty in milliwatts;
- dataset/version/evidence and use-authorization reference;
- workload contract, normalization revision and state-mapper revision;
- sample period and clock-alignment method.

The fingerprint is supplied by the measurement/evidence pipeline. This crate checks exact equality but does not authenticate the fingerprint, sensor, signature, calibration certificate or data-use authorization.

Calibration samples are accepted only with `GOOD` quality. Missing, saturated, clock-unsynchronized, calibration-invalid, duplicate, irregular or out-of-order samples fail the run. The initial implementation rejects irregular cadence rather than silently imputing or dropping samples.

## 3. Leakage-resistant fitting

A complete acquisition session is assigned to either training or held-out evaluation. A session cannot appear in both splits, even if its samples have different run IDs. All runs must match the calibration contract exactly.

The fitter validates held-out runs and records their run/session IDs, but only training runs enter the state and transition distributions. No random row-level split is permitted. A later cross-validation benchmark must group by complete acquisition session and, when sample count permits, add leave-one-hardware-configuration-out evaluation as a separate test of transferability. It must not pool configurations to make a stronger headline.

## 4. State and transition semantics

The model maintains two separate empirical objects:

1. **State envelope:** observed power distribution for (P(P_t \mid s_t)).
2. **Transition envelope:** observed destination-sample power distribution for (P(P_t \mid s_{t-1}, s_t)).

For the first sample, prediction uses the state envelope. Every subsequent sample uses the exact ordered state-pair envelope. The output retains every timestamp, source-event reference and state label from the mapped trace. The prediction interval is widened by the calibration uncertainty recorded in the contract.

This captures first-order transition conditioning and preserves the supplied state chronology. It does **not** yet model long-memory latent states, transition-duration distributions, device-to-rack synchronization, multi-device aggregation, or a generative stochastic trace. Those are explicit future increments, not implied capabilities.

The state mapper must be backed by workload-phase or utilization telemetry. A job's abstract GPU-unit-nanoseconds, CPU occupancy or memory allocation does not by itself identify prefill, decode, communication wait, memory stall, idle, training phase or power state.

## 5. Held-out validation metrics

The evaluator requires exact run identity, sample count, timestamps, state sequence, sample cadence and GOOD-quality measured samples. It reports:
- integrated power-time relative error (energy error; common sample period and unit factors cancel);
- mean and peak power relative error;
- mean absolute signed-ramp-profile error in mW;
- lag-one autocorrelation absolute error, when both traces have defined variance;
- prediction-interval coverage and mean interval width.

Coverage without width is insufficient: an arbitrarily wide interval can achieve nominal coverage while providing no useful information. Report both, per state/transition and across the complete held-out run.

## 6. Baselines and head-to-head comparison

For every eligible configuration, compare the exact same held-out runs against:
1. nameplate/TDP-only estimate (diagnostic upper-bound baseline only; never a measured-power substitute);
2. one global training-set mean;
3. state-conditioned empirical means/medians without transition conditioning;
4. simple utilization regression, only where utilization telemetry and sample boundaries match;
5. this state-plus-one-step-transition envelope;
6. Microsoft's compositional trace method or a faithful reproduction when its licensed data, model details and supported configuration are available.

For scheduling/flexibility evaluation, reuse #400's same workload arrivals and schedule timelines across:
- FCFS;
- EDF;
- EDF with backfilling;
- the current strict-priority/no-backfill reference policy;
- an independently implemented LAD-Flex reproduction;
- a matched MILP/optimization oracle for tractable subsets.

The comparison must hold workload, useful work, QoS, deadlines, hardware, policy constraints, power boundary and sample resolution constant. If a baseline cannot run on the same case, label the result non-comparable instead of filling the gap with a proxy.

## 7. Proposed targets (research gates, not standards or achieved results)

| Dimension | Initial target | Required disclosure |
|---|---:|---|
| Held-out integrated energy | median absolute error <= 5% on a matched, supported cohort | P50, P90, worst configuration and support coverage |
| Held-out mean power | report relative error | P50/P90/worst, by state |
| Held-out peak power | <= 10% relative error | Peak definition and common sample bandwidth |
| Nominal 90% interval | empirical coverage 85–95% | Mean/median interval width and per-state coverage |
| Lag-one autocorrelation | absolute error <= 0.05 | Per-run result and sample length |
| Ramp profile | report mean absolute ramp error | Sampling bandwidth, peak ramp error and sign |
| Unsupported support | 0 extrapolated predictions | Unknown state/transition/configuration remains UNKNOWN |
| Missing/invalid measurements | 0 silently dropped samples | Exact missing count and fail status |
| Evidence lineage | 100% of outputs bound | Model, config, boundary, data, mapper, time alignment and source-event IDs |
| Reproducibility | deterministic output for frozen inputs and revision | Canonical input/output digests produced by the evidence pipeline |

The <=5% goal is motivated by Microsoft's published results for most tested configurations. It is not a claim that the current implementation matches that method or result. The 85–95% interval-coverage band is a proposed calibration check, not a published industry threshold.

## 8. Industry position and route beyond SOTA

Microsoft Research's March 2026 work composes workload-state transitions with configuration-specific conditional power distributions, preserves temporal autocorrelation, and reports median absolute energy error below 5% for most tested configurations. Our v0.1 module deliberately covers the first reproducible subset: state and one-step transition envelopes for an already mapped, exact-configuration sequence. The gap is trace synthesis, longer temporal dependence, multi-device composition and workload/configuration generalization.

MLPerf Power provides the right discipline for energy efficiency: measured samples/joule or tokens/joule, with explicit workload and instrumentation. It is not a grid-flexibility or deadline-preservation benchmark. Comparisons must match model/output quality, system configuration, software stack, power boundary and measurement protocol.

OpenDC's task/fragment schema shows why future workload import must preserve time-varying demand and task dependencies. The present replay's constant-duration jobs are a declared subset. Do not average away fragments before replay.

A stronger programme should publish a multi-axis capability surface, not one flexibility percentage:
- useful work retained and QoS/deadline compliance;
- measured PCC kW reduction and kWh movement;
- response time, sustainable duration, ramp/rebound and recovery tail;
- uncertainty, confidence bounds and out-of-support coverage;
- workload and hardware configuration;
- location, price, carbon intensity and grid-service class.

Market-serving, grid-serving and system-serving flexibility remain separate products. Lower electricity cost or lower peak load does not establish lower emissions; the carbon calculation must use the appropriate regional and temporal marginal-emissions evidence, and must be independently audited.

## 9. Integration sequence

`#400 workload replay/timeline -> independently evidenced state mapper -> #401 calibrated power-state model -> #382 measured PCC/work audit -> #396 repeated-event qualification`

The state-model output is tagged MODELLED. Only #382's matched physical data are MEASURED. #396 can only consume evidence that has passed its own repeated-event, useful-work, uncertainty, response, sustain and recovery gates.

## 10. Limitations at this revision

- Synthetic Rust unit tests are authored but have not been executed.
- No physical power trace, licensed trace corpus, sensor calibration certificate or hardware configuration has been imported.
- No Rust compile, formatter, linter or runtime benchmark is claimed.
- State labels and clock-alignment evidence are supplied by callers and are not authenticated by this module.
- This is not a replication of Microsoft's compositional generator.
- No device/rack/facility aggregation, long-memory state model, power-control action, grid bid, carbon accounting or hardware qualification is implemented.

## 11. Additional leakage and clock-identity hardening

Prediction now only accepts a run explicitly declared in the fitted model's held-out run set. A prediction cannot target a training run. The held-out evaluator independently checks that the run being scored is present in the prediction trace's carried held-out evidence manifest; a caller cannot relabel a prediction run and bypass the split merely by supplying a matching string.

The calibration run and mapped prediction trace must also declare the exact same clock-alignment method ID as the model contract. A PTP-labelled calibration cannot be joined to an NTP- or host-clock-mapped trace without a new explicitly validated alignment contract. These are implemented fail-closed checks; the clock evidence itself still depends on the supplied evidence references and is not cryptographically authenticated by this module.

The output trace also carries the specific mapped-input `state_mapping_evidence_id`, in addition to the model's calibration-run mapping evidence, clock-alignment evidence and source timeline digest reference. This makes the per-prediction input mapping provenance explicit in the output object.
