# Correlation-Aware Multi-Site AI Data-Centre Power Aggregation v0.1
**Date:** 11 November 2026  
**Issue:** [#403 — correlation-aware multi-site aggregation](https://github.com/Luminous-Dynamics/luminous-platform/issues/403)  
**Calibration model:** [PR #401](https://github.com/Luminous-Dynamics/luminous-platform/pull/401)  
**Workload timeline:** [PR #400](https://github.com/Luminous-Dynamics/luminous-platform/pull/400)  
**Measured site/PCC audit:** [Issue #382](https://github.com/Luminous-Dynamics/luminous-platform/issues/382)  
**Repeated-event qualification:** [Issue #396](https://github.com/Luminous-Dynamics/luminous-platform/issues/396)

## 1. Research premise

Fleet flexibility is not necessarily the sum of independent site flexibility. AI workload orchestration, common model releases, request bursts, thermal control and power-electronic interactions can produce synchronized load dynamics across servers and sites.

A 31 August 2026 IEEE Transactions on Power Systems early-access paper, *Spatially Correlated Load Dynamics in AI Data-Center-Dominated Grids*, reports strong temporal non-stationarity and spatial correlation in its studied AI-data-centre scenarios. Its case analysis reports active-power variability around seven times the independent-load reference and frequency standard deviation around eleven times the independent reference. These are scenario-specific results, not fleet constants, universal multipliers or values to hard-code into this programme.

The research question is: **How much does a dependence-aware model change aggregate peak, ramp, reserve and flexibility estimates compared with a matched independent-load assumption?**

## 2. Scope and ownership

This is a trace composition and dependence-validation layer:
- #401 provides configuration-specific MODELLED state/transition power envelopes for explicitly mapped traces.
- #382 remains the owner of MEASURED PCC active/reactive power, energy, uncertainty, storage correction, ramp and variability.
- #396 remains the owner of repeated-event compute attribution and grid-service qualification.
- #400 remains the source of workload occupancy and useful-work event timelines.
- #391 and #387/#388 remain the authorities for dataset rights, source evidence and sampling adequacy.

This work must not construct a second PCC integrator, infer energy from abstract GPU units, or label predicted regional aggregation as measured.

## 3. Hierarchical aggregation contract

Represent the topology explicitly:
`device -> server -> rack -> row -> site/PCC -> region`

Every time series binds to:
- physical or modelled source ID and parent topology ID;
- exact hardware/configuration fingerprint and software/power-cap state;
- workload/model/request-mix contract and state-mapper revision;
- measurement boundary (or explicit modelled boundary);
- sensor identity, calibration/uncertainty and sample quality when MEASURED;
- clock ID, time origin, alignment method, offset estimate and uncertainty;
- region/grid node/PCC identity where known;
- source dataset version, use authorization, evidence IDs and digest references.

Aggregation may only combine compatible power quantities at a common declared boundary and cadence. If child boundaries omit cooling, conversion loss, UPS, storage, facility load or non-compute loads, the aggregate must label those omissions; it cannot quietly claim facility or PCC power.

## 4. Dependence model ladder

Compare several increasingly expressive models on the same held-out multi-device/site episodes:

1. **Marginal-sum baseline:** sum device/site means or distributions while assuming independence. This is a deliberately weak comparator.
2. **Independent-session composition:** combine separate acquisition sessions matched by operating regime while preserving each source session's own time autocorrelation. No arbitrary sample-row shuffling.
3. **Lagged cross-correlation:** estimate contemporaneous and lagged cross-correlation/covariance on aligned traces.
4. **State-conditioned dependence:** condition cross-site dependence on workload phase, transition events, model/request mix, orchestration domain and shared control state.
5. **Hierarchical temporal model:** preserve device/server/rack/site aggregation and common-mode latent factors; model transition synchrony, duration and burst persistence.
6. **Measured aggregation:** directly measured parent-boundary trace used as an independent reference, where instrumentation and boundary definitions permit it.

A synthetic independence null must be constructed from a declared, defensible procedure. Examples include independent sessions matched by time-of-day, workload class, configuration and season, or a validated conditional model. Circular shifts and phase randomization can be diagnostic controls, but are not automatically a physically valid independent baseline.

## 5. Dependence and grid-facing metrics

Do not reduce the dependence structure to one average correlation coefficient. Report:
- cross-site Pearson/Spearman correlation with validity conditions;
- lagged cross-correlation and lag distributions;
- covariance and variance contributions, including common-mode factors;
- spectral coherence/band-limited correlation at a declared sample bandwidth;
- simultaneous peak and tail co-exceedance probabilities;
- aggregate P50/P90/P95/P99/max, ramps, variance and duration above thresholds;
- uncertainty/coverage by configuration and aggregation level;
- independent-null versus observed/modelled aggregate variance and peak differences;
- conditional deliverability and response/recovery under synchronized dispatch episodes.

A useful diagnostic is:
`F_var = Var(P_aggregate_observed) / Var(P_aggregate_independent_reference)`
It must be labelled as a scenario-specific diagnostic. It is not a universal diversity factor or a reliability credit by itself.

## 6. Validation protocol

- Use session/day-blocked train/validation/test splits; never split adjacent telemetry rows randomly.
- When enough sites exist, perform leave-one-site-out and leave-one-region-out evaluation.
- Quantify clock offset/drift and propagate alignment uncertainty. If alignment uncertainty is too large relative to the phenomenon under test, reject the correlation claim.
- Compare summed child traces against measured parent-boundary traces with energy conservation residuals and combined uncertainty.
- Preserve measured/modelled labels at every aggregation level.
- Include negative controls: arbitrary row shuffle, stale/offset site clock, dropped site, missing channels, shared model release, synchronized burst, thermal control coincidence, common power-cap change, network partition, sensor saturation and mismatched topology.
- Use day/session-block bootstrap or another dependence-aware inference procedure for uncertainty. Treating adjacent time samples as independent replicates is invalid.

## 7. Proposed benchmark gates (internal research targets, not standards or achieved results)

| Dimension | Proposed initial gate |
|---|---|
| Per-site state-power model | Reproduce #401 matched-cohort targets; report P50/P90/worst configuration and interval width |
| Aggregate energy | Residual between summed child and measured parent traces within independently established combined measurement uncertainty |
| Aggregate peak | <=10% held-out relative error on a matched supported cohort; report P50/P90/worst site/region |
| Dependence | Report contemporaneous/lagged structure, spectral bands and tail co-exceedance; do not use one scalar as sole acceptance |
| Interval quality | Nominal 90% predictive interval target coverage 85–95%, with width and group-conditioned coverage |
| Clock support | 100% of used site pairs have declared offset/drift uncertainty and common time support |
| Extrapolation | Zero silent transfer to unseen configuration, site boundary, topology or workload regimes |
| Independence error | Quantify held-out peak/ramp/variance bias from the independent assumption; no universal threshold until corpus and use case are preregistered |
| Fleet credit | No capacity/diversity credit until at least one independently reproduced multi-site sequence and relevant #396 repeated-event gates are passed |

## 8. Why this can move beyond the state of the art

The differentiator should not be “we calculate correlation.” That is established statistical practice. The opportunity is a reproducible **evidence-carrying, configuration- and state-conditioned dependence model** connected end-to-end to:
- task-level accepted useful work (#400);
- calibrated device/server state power (#401);
- measured electrical boundaries and storage correction (#382);
- repeated-event conservative site capacity (#396);
- and policy comparisons under identical correlated and independent scenarios (#402).
- cross-site data readiness and command-to-source-relief response evidence (#404).

This lets operators see not just each site's marginal flexibility, but the probability that the fleet can deliver simultaneously when called, under a shared workload and control environment. It should also reveal regimes where coordination improves diversity and where synchronized actions amplify risk.

## 9. Implementation sequence

1. Freeze topology/time-alignment and cross-site trace contracts.
2. Implement exact timestamp/cadence alignment with explicit uncertainty and rejection.
3. Build dependence statistics and a defensible independent-session null.
4. Add held-out hierarchical and state-conditioned dependence models.
5. Reconcile child aggregation against measured parent traces.
6. Feed correlation-aware dispatch trials into #396 only after site/PCC measurement gates.
7. Compare policy tournament #402 under asynchronous, correlated and common-mode workload episodes.
8. Pair the correlated scenarios with #404 data-path/pre-launch migration trials; preserve source relief and destination added load separately.

## 10. Claim ceiling

This document is a research specification. No multi-site telemetry corpus, regional trace model, correlation metric implementation, field dispatch or reserve/diversity credit has been produced by this revision. A predicted aggregate is MODELLED; a measured parent-boundary trace is MEASURED; grid qualification remains separate.

## 11. v0.1 Rust implementation status

The first offline implementation is registered as `tools/measurement-audit/src/multi_site_power.rs`.

Implemented in this increment:
- at least two child traces with exact parent-topology and region membership;
- distinct source clock/time-origin IDs mapped by caller-supplied signed constant offsets into an explicit common clock and time origin;
- exact declared sample cadence and common timestamp sequence; no interpolation or row shuffling;
- maximum clock-alignment uncertainty, method ID and evidence ID checks;
- fail-closed rejection of missing, saturated, unsynchronized or calibration-invalid samples;
- no mixing MODELLED and MEASURED traces in one aggregate;
- deterministic canonical ordering by site ID;
- aggregate power samples and mean/variance/P99/peak;
- pairwise contemporaneous Pearson and tie-aware Spearman correlation, covariance and P90 tail co-exceedance;
- lagged Pearson correlations and sample counts for each configured lag;
- Welch magnitude-squared spectral coherence with Hann windows, 50% overlap and configured power-of-two segment size; it is a descriptive time-series statistic, not an EMT or grid-dynamics simulation;
- Welch magnitude-squared spectral coherence over Hann-windowed, 50%-overlapped, power-of-two segments, with frequency bins and number of segments; this is a descriptive trace statistic, not a network/EMT stability simulation;
- independent-reference variance as the sum of child population variances, variance amplification ratio and covariance-decomposition residual;
- source site/run/session/trace/clock-alignment lineage.
- complete per-site provenance snapshots containing dataset/version/evidence, use authorization reference, hardware-configuration fingerprint, physical measurement boundary, workload/orchestration identity, state mapper, source/aligned clocks, offset, uncertainty and cadence.
- every aggregate timestamp carries its exact per-site contribution vector (site ID, power, state and source sample evidence ID), enabling independent row-level reconstruction and sum checking.

Not implemented yet: time-varying clock-drift estimation/correction, state-conditioned dependence models, long-memory/common-factor model, physically matched independent-session trace synthesis, measured parent-boundary reconciliation, energy integration, causal attribution or region-level grid simulation. The current implementation only applies caller-declared constant offsets, enforces a conservative pairwise sum of clock-alignment uncertainty bounds, and retains topology/state-mapping evidence; it does not estimate time-varying clock drift.

Twenty-two synthetic Rust tests are authored but not executed. P90 tail co-exceedance is suppressed unless at least 100 common aligned samples are available; Welch coherence is suppressed unless at least eight complete 50%-overlapped segments are available (segment length capped at 65,536); smaller fixtures still exercise exact alignment and aggregate arithmetic without claiming supported tail statistics. The implementation applies a fixed caller-declared offset, requires mapped common clock/time-origin IDs to match the aggregation contract, and enforces a conservative pairwise clock-uncertainty sum. Parent-topology evidence and per-site state-mapping evidence are carried into the report. No time-varying clock-drift estimate/correction is implemented. No multi-site measured or licensed trace data were imported. No compile, formatter, linter, runtime benchmark or field result is claimed.
