# Dependence-Preserving Forecast Scenarios and Horizon-Wide Reserve Sizing v0.1

**Date:** 11 November 2026  
**Parent forecast model:** [PR #401 — calibrated workload-state power envelopes](https://github.com/Luminous-Dynamics/luminous-platform/pull/401)  
**Regime calibration:** [PR #406 — session conformal forecast calibration](https://github.com/Luminous-Dynamics/luminous-platform/pull/406)  
**Related measurement:** [Issue #382 — compute-preserving grid-to-chip PCC M&V](https://github.com/Luminous-Dynamics/luminous-platform/issues/382)  
**Direct research comparator:** Yao et al., [Grid-Oriented Probabilistic Load Forecasting for AI Data Centers](https://arxiv.org/abs/2611.08929), submitted 6 November 2026

## 1. Why temporal scenarios are a separate capability

A probabilistic forecast can have well-calibrated marginal intervals and still be operationally wrong. Independent sampling at each forecast horizon step destroys serial dependence: it can create ramps, peaks, dwell durations and recovery patterns that were not present in the underlying load process.

Yao et al. show this directly on their 448-GPU MIT TX-Gaia benchmark. Consecutive forecast errors had lag-one correlations around 0.84–0.90 across the studied tasks. Independent scenario sampling distorted ramp distributions and reserve procurement. Reconstructing temporal dependence from historical forecast errors reduced realized reserve cost by 7.4–10.6% across the paper's three tasks. These are study-specific results, not Luminous results.

This increment starts the temporal-scenario layer after #401/#406. It keeps base probabilistic prediction, marginal calibration, temporal dependence, reserve sizing and physical measurement as separate components.

## 2. Exact input contract

ProbabilisticPowerForecast binds:
- Exact full PowerModelContract from #401 (model revision, hardware, sensor/calibration, dataset/right-to-use, normalization, measurement boundary, workload, state mapper, clock method and cadence), plus an exact common clock identity and time-origin ID shared by forecast, residual blocks and measured evaluation.
- Forecast ID and origin ID/timestamp.
- Predeclared operating regime.
- Forecast, state-mapping and time-alignment evidence references plus source-timeline digest.
- Exact equally spaced horizon timestamps and per-step quantile distributions.

Each step's power value represents **interval-average power over the interval ending at that timestamp**, not an instantaneous power sample. The timestamp vector must equal origin + (step_index + 1) × step_period. Quantile probabilities must be strictly increasing, values non-crossing, and an exact median at 0.5 must be present.

ForecastResidualTrajectory contains the full historical measured-minus-median-forecast error vector over the same horizon. It is tied to the exact model/configuration/regime and includes the historical forecast origin, target window end, forecast/measurement/residual-generation/state-mapping/clock evidence, source timeline digest, exact source sample timestamps, and per-sample evidence IDs.

Residual target windows must end no later than the current forecast origin. The residual pool rejects duplicate origins, duplicate trajectory IDs, reuse of a source measurement-sample evidence ID across blocks, and origin separations shorter than the frozen minimum. For dependence-preserving methods, the minimum origin separation must be at least one full target horizon, so sampled target windows do not overlap. This reduces direct overlap; it does **not** prove statistical independence or stationarity.

## 3. Three matched scenario-generation arms

### A. Independent marginal-quantile negative control

For every horizon step, the generator takes a deterministic stratified grid of quantile probabilities and independently permutes the marginal samples across scenario rows. Each horizon retains the same discrete marginal sample set, but temporal rank association is deliberately destroyed.

This is a negative control for the failure mode identified in the 6 November paper. It is not the production recommendation.

### B. Empirical residual-trajectory bootstrap

The generator samples **complete historical residual trajectories with replacement**, then adds every selected residual vector to the current forecast's median path. Whole vectors are kept together, preserving their within-horizon ramp, persistence and recovery structure.

Sampling with replacement is essential: sampling without replacement is a finite permutation, not a bootstrap. Repeated selected residual IDs are therefore expected and retained in the report. A bootstrap draw never resamples individual horizon rows.

Residual floor clamping at zero power is reported explicitly. The source residual ID, residual measurement evidence and every original sample evidence/timestamp are retained for each generated scenario.

### C. Schaake-shuffle rank transfer

The generator selects historical trajectory rows without replacement, ranks only that selected subset independently at each horizon step, sorts the current forecast's marginal scenario samples, and assigns each marginal sample exactly once according to the selected rows' rank order.

This transfers the selected historical temporal dependence pattern while preserving the forecast's discrete marginal sample set exactly at every horizon step, even when the residual pool is larger than the requested ensemble. The temporal dependence remains an empirical finite-ensemble approximation and must be evaluated with proper multivariate scores and ramp/peak diagnostics.

The selected historical trajectory and its source sample evidence are retained for every scenario.

## 4. Horizon-wide reserve sizing

For scenario s, define the positive deviation from the scheduled median path over the complete horizon:

\[
D_s = \max_{h=1,\ldots,H}
\left(P_{s,h} - \widetilde P_h\right)_+
\]

For reserve cost c_R per unit and shortfall cost c_S per unit, the critical fractile is:

\[
q^* = 1 - \frac{c_R}{c_S}
\]

The reserve decision is the empirical nearest-rank q* quantile of the scenario-level horizon maxima. The measured horizon is then scored against this decision:

\[
\text{shortfall} = (D_{\mathrm{actual}} - R)_+
\]

\[
\text{realized cost} = c_R R + c_S \text{shortfall}
\]

The evaluator records the declared cost ratio, critical fractile, reserve quantity, actual horizon maximum deviation, realized shortfall and normalized cost. It does not connect to a market, submit a bid, issue a dispatch command or accredit capacity.

Energy uses interval-average power multiplied by the frozen forecast-step duration. It is reported as mW·ns in the core audit to avoid silently changing units; conversion to joules is 10^-12 J per mW·ns. Peak, maximum ramp-up/down, horizon energy, PIT diagnostics, energy score and variogram score are reported independently.

## 5. Evaluation and comparison protocol

Every generator arm must receive the exact same:
- probabilistic forecast and origin;
- full model/configuration/workload/state/clock contract;
- regime and forecast horizon;
- historical residual pool (where relevant);
- scenario count and frozen random seed;
- measured horizon and measurement evidence;
- reserve/shortfall cost ratio.

Primary metrics:
- Pinball loss / CRPS and interval coverage from the upstream probabilistic forecast track.
- Energy score and variogram score for the joint horizon distribution.
- Peak, horizon energy, maximum ramp-up/down and excursion-duration distributions.
- PIT/rank diagnostics for peak, energy and ramp quantities.
- Horizon-wide reserve MW, shortfall rate and realized cost across a preregistered range of c_R/c_S ratios.
- Residual source, trajectory, sample, forecast and measurement evidence completeness.

The energy score is a strictly proper multivariate score, but can be relatively insensitive to dependence errors. The variogram score is a complementary proper score with greater sensitivity to dependence/correlation structure. Both are required; neither replaces direct ramp/peak/reserve diagnostics.

## 6. Direct reproduction track: Yao et al. (6 November 2026)

The paper's public benchmark uses measured GPU IT power from 224 TX-Gaia nodes with two NVIDIA V100 GPUs each (448 GPUs), plus Slurm scheduler data. It is not a whole-facility/PCC data set.

Paper benchmark dimensions:
- Chronological training 1 March–30 June 2021; validation July; test 1 August–30 September; target windows remain within their split.
- 5-minute resolution / 1-hour horizon; 15-minute / 4-hour; 1-hour / 24-hour.
- Nine quantiles from 0.025 to 0.975, persistence and seasonal-naive baselines, quantile LightGBM and temporal convolutional network.
- Regime-conditioned online calibration using recent residuals.
- Independent sampling, Gaussian copula, residual trajectory bootstrap and Schaake shuffle, evaluated with 200 scenarios.
- Reserve sizing from the horizon maximum-deviation distribution and declared reserve/shortfall cost ratios.

Reported results include nominal-90% LightGBM interval coverage falling to 63.4% on the 1-hour / 24-hour task after utilization change, then reaching 87.3% under the paper's online regime calibration. Temporal dependence reconstruction reduced realized reserve cost by 7.4–10.6% over its three tasks. These figures are comparison targets, not Luminous results.

### Rights gate

The MIT Supercloud Dataset Registry lists CC BY-NC-ND 4.0 for the underlying dataset. Do not download, transform, redistribute or use the dataset for commercial/derivative product development without a separate rights review and appropriate permission. The public evaluation code and the dataset are separate licensed artifacts; code availability does not authorize dataset use. No MIT telemetry was imported for this PR.

A valid reproduction manifest must bind the exact upstream code commit/license, dataset release/license/rights decision, extraction/filter counts, exact split/horizon settings, idle-power assumptions, regime derivation, model settings, scenario generator and scoring code. If dataset rights are not granted, only a separately labelled methodological reproduction using permissibly licensed data or synthetic fixtures may proceed.

## 7. Proposed research gates

These are internal research targets, not standards or achieved results.

- Use 200 scenarios for initial parity with the paper; test 100, 200 and 500 as a separate ensemble-size sensitivity.
- Reproduce all paper tasks and scenario generators before making a superiority claim.
- Match or improve joint energy score and variogram score against independent sampling, residual bootstrap and Schaake shuffle on identical forecasts and observations.
- Demonstrate calibrated peak/ramp/energy diagnostics without merely widening forecast intervals.
- Target at least 7.4% lower realized reserve cost than the independent-marginal negative control at the **same shortfall-rate constraint**, then verify across all task horizons and cost ratios. This is a research target motivated by the paper's observed range, not an expected guaranteed result.
- Report cost/shortfall Pareto frontiers; no single reserve-cost operating point may hide more shortfalls.
- Preserve complete event/session residual blocks and exact source evidence; zero use of future residuals.
- Do not infer PCC or facility reserve from GPU IT-only scenarios. #382 remains the physical PCC/useful-work measurement authority; #396/#405 remain the repeated-event and clustered reliability layers.

## 8. Rust implementation and validation status

The new temporal_scenarios module:
- provides independent marginal quantile sampling, residual trajectory bootstrap with replacement and Schaake-shuffle rank transfer;
- pins the exact PowerModelContract, operating regime, forecast origin, horizon, cadence and residual-pool rights evidence;
- rejects future residual windows, overlapping historical target blocks, duplicate origins/trajectory IDs, duplicate source-sample IDs and timestamp/cadence mismatches;
- retains full forecast, residual, sample, measurement and time-alignment lineage;
- evaluates scenario energy score, variogram score, peak/ramp/energy diagnostics, PIT and horizon-wide reserve critical-fractile outcomes;
- labels results as modelled scenario/reserve economics, not measured or accredited capacity.

Twenty-six synthetic Rust tests are authored. They cover deterministic independent-marginal sampling, with-replacement residual bootstrap, source trajectory integrity, Schaake marginal sample preservation, residual overlap/future leakage, quantile crossing, timestamp alignment, insufficient support, reserve critical fractile, energy/variogram scoring, invalid measurements, residual identity, full provenance carry-through, common clock/time-origin mismatch and claim-ceiling tampering.

**Tests are authored but not executed.** No Rust compilation, cargo test, formatting, Clippy, runtime benchmark, MIT dataset download, paper result reproduction, physical PCC measurement, grid dispatch, reserve delivery or grid accreditation is claimed.

## Sources

- Yao, Lin, Zhao, Yue and Wang, [Grid-Oriented Probabilistic Load Forecasting for AI Data Centers](https://arxiv.org/abs/2611.08929), arXiv:2611.08929, submitted 6 November 2026.
- Clark, Gangopadhyay, Hay, Rajagopalan and Wilby, [The Schaake Shuffle: A Method for Reconstructing Space-Time Variability in Forecasted Precipitation and Temperature Fields](https://doi.org/10.1175/1525-7541(2004)005<0243:TSSAMF>2.0.CO;2), Journal of Hydrometeorology, 2004.
- Gneiting and Raftery, [Strictly Proper Scoring Rules, Prediction, and Estimation](https://doi.org/10.1198/016214506000001437), Journal of the American Statistical Association, 2007.
- Scheuerer and Hamill, [Variogram-Based Proper Scoring Rules for Probabilistic Forecasts of Multivariate Quantities](https://doi.org/10.1175/MWR-D-14-00269.1), Monthly Weather Review, 2015.
- [MIT Supercloud Dataset](https://github.com/MIT-AI-Accelerator/MIT-Supercloud-Dataset), [MIT Datacenter Challenge data page](https://dcc.mit.edu/data/) and [AWS Registry of Open Data](https://registry.opendata.aws/dcc/). Dataset right-to-use gate: CC BY-NC-ND 4.0.


**Parity limitation:** Yao et al. also compare a Gaussian-copula scenario generator. That method is not implemented in this v0.1 module. [Issue #408](https://github.com/Luminous-Dynamics/luminous-platform/issues/408) owns the rank-normal residual dependence estimator, regularized correlation factorization, deterministic copula sampling and the missing four-arm parity tournament. Do not describe PR #407 as a complete reproduction of the paper's scenario suite.
