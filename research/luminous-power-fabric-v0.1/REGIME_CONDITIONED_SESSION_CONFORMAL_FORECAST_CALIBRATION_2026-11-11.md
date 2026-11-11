# Regime-Conditioned Session Conformal Forecast Calibration v0.1

**Date:** 11 November 2026  
**Parent model:** [PR #401 — calibrated workload-state power envelopes](https://github.com/Luminous-Dynamics/luminous-platform/pull/401)  
**Research baseline:** Yao et al., [Grid-Oriented Probabilistic Load Forecasting for AI Data Centers](https://arxiv.org/abs/2611.08929), submitted 6 November 2026  
**Public processing/evaluation code:** [dinglin613/Data_center_forecasting](https://github.com/dinglin613/Data_center_forecasting)

## 1. Why this increment exists

PR #401 fits exact-configuration state and one-step transition power envelopes, preserves whole-session training/held-out separation, and exposes a separate operational-scenario prediction API. It reports interval coverage and interval width, but its current prediction interval is empirical p10–p90 (a central 80% interval before additional uncertainty widening). Its benchmark target table separately describes a nominal 90% interval. Those statements are not interchangeable.

The 6 November 2026 arXiv benchmark by Yao et al. demonstrates why this is important. On a 448-GPU production cluster, nominal 90% LightGBM intervals covered only 63.4% of outcomes at the 1-hour / 24-hour horizon after a utilization shift. Regime-conditioned online calibration raised coverage to 87.3%. A temporal convolutional network was already closer to nominal without correction, illustrating that calibration and interval width must be compared together rather than treating recalibration as an automatic improvement.

That paper also shows that marginal interval calibration is not sufficient: independently sampled scenarios destroyed serial dependence and distorted ramp distributions. Historical error-trajectory methods reduced realized reserve cost by 7.4–10.6% over its three tasks. These are paper results, not Luminous results.

## 2. The new statistical unit: an acquisition session

The v0.1 Rust implementation uses **split conformal calibration over complete acquisition sessions**, conditioned on a predeclared operating-regime ID.

For each calibration session, it computes one nonconformity score:

s_j = max over t in session j of max{L_jt - y_jt, 0, y_jt - U_jt}

where L and U are the base lower/upper prediction bounds and y is the measured power.

The regime-specific additive correction is the finite-sample split-conformal order statistic:

k = ceil((n + 1) gamma), q_gamma = s_(k)

where n is the number of calibration sessions in the regime and gamma is the target **whole-session** coverage. The corrected interval is:

[L_t - q_gamma, U_t + q_gamma]

The maximum score makes a session successful only when every sample in every trace covered by that session is inside its corrected interval. Arbitrary temporal dependence within a calibration session is therefore not treated as independent sample evidence. Fixed session duration, fixed trace count, fixed samples per trace and a declared separation rule keep the calibration exposure comparable.

For a 90% whole-session target, the finite-sample rank is first observable with 9 exchangeable calibration sessions; for 95%, it is first observable with 19. Those are mathematical minimums, not recommended field sample sizes. The research programme should target at least 30 and preferably 50+ separately sampled sessions per regime for confirmatory assessment.

**Assumption:** finite-sample split-conformal coverage depends on exchangeability of the calibration and future session scores within the declared regime. The implementation requires an exchangeability-review reference, but cannot prove exchangeability or regime correctness from identifiers and timestamps.

## 3. Frozen contract and leakage controls

Every calibrator binds to:
- The exact full PowerModelContract snapshot from #401, including model revision, hardware fingerprint, measurement boundary, sensor and calibration evidence, dataset/version/rights, normalization, workload contract, state mapper, clock method, sample cadence and support thresholds.
- A versioned calibrator ID/revision, target session coverage and explicit required-regime list.
- Exchangeability-review evidence and an explicit declared assumption.
- Fixed session window duration, minimum separation, trace count and sample count.

Calibration uses only OperationalScenario predictions whose run IDs are absent from both the source model's training and held-out evidence. Every calibration session and run ID is frozen into the calibrator. Monitoring sessions cannot reuse those session/run/trace IDs.

The calibrator does **not** use the #401 held-out validation partition to tune its correction. The source model's held-out evaluation remains a separate measurement of base-model performance.

## 4. Operating-regime behavior

A regime label is an upstream, evidence-backed input. The module does not infer workload state, thermal state, hardware configuration or regime from a power scalar.

Each required regime receives its own correction. A regime absent from the frozen calibration contract is **OUT_OF_DOMAIN**; no interval correction is borrowed from another regime, and the API refuses to emit a calibrated trace.

The initial implementation does not silently update its correction from monitoring outcomes. A recalibration requires a new revision and a newly frozen calibration-session set. This avoids accidental reuse of monitoring/test outcomes as training data.

## 5. Monitoring and safe degradation

The monitor evaluates later, disjoint sessions using the frozen corrections and reports:
- Whole-session coverage (every sample covered) as the primary statistical unit.
- Sample-level coverage separately as a diagnostic only; individual samples are not treated as independent trials.
- A one-sided Wilson lower bound over monitored sessions.
- Regime health as NOMINAL, DEGRADED, INSUFFICIENT_SUPPORT or OUT_OF_DOMAIN.
- Exact session, run, trace, timeline, event and measured-sample evidence references.

The monitor never turns a failed interval into a pass, changes the calibrator or claims that future coverage is guaranteed. A degraded or out-of-domain status should reduce the forecast claim ceiling and trigger review or a newly qualified calibration revision.

## 6. Direct reproduction track: Yao et al. (6 November 2026)

The paper reports an open benchmark and links its processing/evaluation code. It uses MIT Supercloud TX-Gaia:
- 224 nodes with two NVIDIA V100 GPUs each (448 GPUs).
- Public GPU power telemetry at 100 ms and Slurm scheduler logs.
- Target is aggregated **GPU IT power**, not CPU, cooling, total facility or PCC demand.
- Unassigned GPUs are assigned an assumed 40 W idle draw; this is a modelling assumption, not measured idle power.
- The underlying MIT Supercloud source collection spans 5 February–1 October 2021.
- The paper uses training 1 March–30 June, validation July and test 1 August–30 September 2021; each forecast target window must lie completely within its split.
- Horizons: 5-minute resolution / 1-hour horizon; 15-minute resolution / 4-hour horizon; 1-hour resolution / 24-hour horizon.
- Four workload regimes: low, calm, active and volatile, using only information available at forecast time.

Its four-stage benchmark is:
1. Probabilistic prediction: persistence, seasonal naive, quantile LightGBM and temporal convolutional network; nine quantiles from 0.025 to 0.975.
2. Online uncertainty calibration: regime-conditioned recent residual corrections, updated hourly, with a 14-day window and a documented fallback for low regime support.
3. Temporal scenario generation: independent sampling, Gaussian copula, whole-trajectory residual bootstrap and Schaake shuffle; 200 scenarios.
4. Horizon-wide reserve sizing: reserve based on the maximum positive deviation across the forecast horizon, with explicit reserve/shortfall cost ratio.

Native metrics include pinball loss / CRPS, MAE, interval coverage by regime, minimum regime coverage, Winkler interval score, energy score, variogram score, PIT/KS diagnostics for peak/ramp/energy/duration distributions, realized reserve cost and shortfall rate.

**Critical boundary:** this is an open research reproduction of GPU IT load forecasting. It does not establish whole-facility power, cooling, PCC response, compute-derived grid-service attribution or repeated-event capacity. Do not compare its GPU IT results directly against #382 PCC measurements.

### Dataset rights gate

The MIT Supercloud Dataset Registry lists the dataset license as Creative Commons Attribution–NonCommercial–NoDerivatives 4.0. The public source describes a roughly 2 TB dataset. Do not download, transform, redistribute or use it in commercial product development without a rights review and an appropriate separate permission. The published benchmark paper links its own processing/evaluation code; review that code's license separately from the dataset's license.

The reproduction manifest must record:
- Exact upstream code commit and code license.
- Exact source dataset release and data license.
- Rights decision and permitted purpose.
- Raw archive manifest/digests where lawful.
- Extraction/filtering counts and timestamp spans.
- The paper's idle-power assumption and treatment of missing GPU telemetry.
- Exact split boundaries, horizon/target windows and regime derivation.
- The model, quantile, calibration and scenario-generator settings.
- Evaluation scripts and raw outputs.

If the rights gate cannot be satisfied, reproduce only the algorithmic protocol using independently licensed or synthetic fixtures and label it **methodological reproduction**, not a dataset reproduction.

## 7. Competitive benchmark plan

| Dimension | Direct paper comparator | Luminous research target |
|---|---|---|
| Base probabilistic forecast | Persistence, seasonal naive, LightGBM, TCN | Reproduce native baselines; add #401 state/transition model as a separate arm |
| Regime calibration | 14-day rolling regime-conditioned residual correction | Compare to session-max conformal, same-regime adaptive calibration and strongly-adaptive baselines |
| Whole-session coverage | Paper reports stepwise central-90% coverage | Additionally report all-samples-covered session coverage, by regime |
| Interval sharpness | Winkler score and width | Report interval score, median/P95 width and correction size |
| Temporal dependence | Copula, residual bootstrap, Schaake shuffle | Exact empirical residual-block replay with no independent row shuffling |
| Ramp/peak | PIT/KS of maximum ramp and peak | Match observed ramp, peak, duration and energy distributions |
| Reserve sizing | Horizon maximum-deviation critical fractile | Paired reserve cost/shortfall frontier at identical cost ratios |
| Shift robustness | Low/calm/active/volatile regimes | Explicit unseen-regime abstention and hidden regime-shift negative controls |
| Leakage | Chronological whole-target-window splits | Whole-session splits; calibration sessions disjoint from training, held-out and monitoring |
| Evidence | Public code/data processing | Exact model/data/code/rights/clock/boundary/source evidence for every run |

Proposed confirmatory goals (not standards or achieved results):
- Nominal 90% whole-session coverage: empirical 85–95% on disjoint confirmatory sessions, overall and separately for every required regime.
- No calibrated output for unknown regimes; 100% correct out-of-domain abstention in negative controls.
- Every calibration/monitoring sample has exact model, configuration, boundary, clock, state, session and source evidence.
- At least 30 independent calibration sessions per regime for initial confirmatory work; 50+ preferred.
- Report interval width and interval score alongside coverage; arbitrarily wide intervals are not a win.
- Joint scenario generator: reproduce lagged-error correlation, ramp/peak/duration/energy PIT and energy/variogram scores.
- Reserve sizing: paired realized cost improvement at the same shortfall constraint; compare the full reserve-cost frontier, not one tuned operating point.
- Preserve accepted useful work and QoS in downstream #402/#382 experiments.

## 8. Implementation and validation status

The registered Rust forecast_calibration module provides:
- Exact full-model-contract binding and operational-scenario-only calibration.
- Session-level maximum nonconformity scores and regime-specific finite-sample additive corrections.
- Exact calibration/monitoring session exposure and separation checks.
- Run/session/trace disjointness, source-evidence carry-through and exact measured/predicted timestamp/state joins.
- Unknown-regime abstention.
- Per-regime online health monitoring using session-level coverage and Wilson lower bounds.
- Explicit claim ceilings and no silent online mutation of the frozen calibrator.

Seventeen synthetic Rust tests are authored. They cover conformal rank support, session-maximum fitting, insufficient/missing/undeclared regimes, held-out-data rejection, model revision mismatch, timestamp/state mismatch, invalid sample quality, duplicate sessions, unequal exposure, out-of-domain abstention, interval expansion, degraded monitoring, insufficient monitor support and calibrator immutability.

**Tests are authored but not executed.** No Rust compilation, cargo test, formatting, Clippy, performance benchmark, public dataset import, model reproduction, physical measurement, scenario reserve savings or grid-service qualification is claimed.

The next major increment is not another pointwise calibrator: it is dependence-preserving multi-horizon scenario generation with residual-block / Schaake-shuffle comparators and paired horizon-wide reserve sizing, using #382 only when physical PCC measurements are actually available.

## Sources

- Yao, Lin, Zhao, Yue and Wang, [Grid-Oriented Probabilistic Load Forecasting for AI Data Centers](https://arxiv.org/abs/2611.08929), arXiv:2611.08929, submitted 6 November 2026. Public code: [Data_center_forecasting](https://github.com/dinglin613/Data_center_forecasting).
- [MIT Supercloud Dataset](https://github.com/MIT-AI-Accelerator/MIT-Supercloud-Dataset) and [MIT Datacenter Challenge data page](https://dcc.mit.edu/data/). The Registry of Open Data lists CC BY-NC-ND 4.0; rights review required before commercial or derivative use.
- Amiri Shahbazi and Baheri, [Heterogeneous-Horizon Conformal Ensembles for Online Prediction Under Distribution Shift](https://doi.org/10.3390/forecast8050077), Forecasting, 1 September 2026.
- Sun and Yu, [Conformal Prediction for Time-series Forecasting with Change Points](https://proceedings.neurips.cc/paper_files/paper/2025/hash/12271b64c483ad8f6192eb6aaa102044-Abstract-Conference.html), NeurIPS 2025.
- Gibbs and Candès, [Conformal Inference for Online Prediction with Arbitrary Distribution Shifts](https://www.jmlr.org/beta/papers/v25/22-1218.html), JMLR, 2024.
