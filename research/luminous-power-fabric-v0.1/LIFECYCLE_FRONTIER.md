# Uncertainty-qualified lifecycle service frontier — v0.1

**Tracking:** [Issue #461](https://github.com/Luminous-Dynamics/luminous-platform/issues/461)  
**Lifecycle accountant:** [draft PR #428](https://github.com/Luminous-Dynamics/luminous-platform/pull/428)  
**Benchmark passport:** [draft PR #430](https://github.com/Luminous-Dynamics/luminous-platform/pull/430)

**Status:** typed offline decision layer over the existing lifecycle accounting report. No field measurements, battery ageing forecast, reliability probability or SOTA result is claimed.

## Paradigm

A scalar weighted score can hide a missed critical interval. This module first applies a non-compensatory feasibility gate, then reports central and uncertainty-robust Pareto frontiers. It never trades a critical-service failure against lower cost.

The lifecycle accountant remains responsible for mission/cost/repair/downtime/observed-retention arithmetic. This module does not duplicate it; it consumes `LifecycleScenarioReport` and checks that supplied uncertainty point estimates bind exactly to that report.

## Matching contract

All candidates in one frontier must share comparator set, currency/minor-unit scale, service boundary, site profile, mission-set digest, scenario-manifest digest, initial/terminal obligation digest, storage scope and confidence level. Product configuration digests may differ; the workload and service obligation may not.

## Hard feasibility and evidence eligibility

A candidate is service-feasible only when it has mission records, all missions pass, the critical-service pass rate is 100%, unserved critical energy is zero within the accountant tolerance, and configuration/lifecycle accounting eligibility pass. Cheap-but-failed candidates are excluded before Pareto ranking.

Evidence eligibility is a separate gate requiring the underlying lifecycle report's physical-claim, current/actual cost, current/physical mission, current/actual repair and (when storage is in scope) complete observed storage-health evidence. Synthetic or incomplete candidates may be useful for fixture testing, but are not eligible for a physical lifecycle claim.

## Uncertainty intervals and Pareto dominance

Each numeric axis has a point estimate, lower and upper bounds, a common declared confidence level, current evidence status, uncertainty basis and evidence reference. The point estimate must reconcile with #428's report; intervals are not generated or statistically re-estimated here. They are supplied by the responsible metrology/model/statistics owner.

Central Pareto membership compares point estimates. Robust Pareto membership is more conservative: for a lower-is-better metric, candidate A is no worse than B only when A's upper bound is no greater than B's lower bound; for the higher-is-better storage-retention metric, A's lower bound must be no less than B's upper bound. Replacement count is an exact integer. A robust dominator must be no worse on every included axis and strictly better on at least one.

Axes: lifecycle cost per delivered critical kWh, unserved critical Wh, system-unavailable hours, replacement count, and (when storage is in scope for every candidate) minimum observed storage-capacity retention. No arbitrary weights or aggregate score exist.

The robust frontier may retain candidates whose intervals overlap. That is intentional: uncertainty means the data do not establish dominance. It does not mean the candidates are equal.

## Synthetic oracle fixture

The fixture has three feasible synthetic candidates and one cheaper candidate that fails the critical-service gate. Candidate A has lower cost but more downtime/replacements than B; A and B remain non-dominated because they represent different trade-offs. C is robustly dominated by A across the declared intervals. CHEAP-FAIL is excluded regardless of its low cost.

All four fixture rows and intervals are synthetic, not measured or accredited.

## Research targets

- 100% hard feasibility for every critical mission and terminal reserve obligation before ranking.
- Zero weighted-score compensation for critical-service or protection failure.
- 100% matched workload/site/initial-terminal/scenario digest identity.
- 100% uncertainty intervals trace to current owner evidence and reconcile to the lifecycle report point estimates.
- Publish both central and robust frontiers; explain every exclusion and domination.
- Only assert superiority where a preregistered paired comparison margin exceeds combined measurement uncertainty and run-to-run variance.
- Add cluster-level paired bootstrap confidence intervals in a later slice; resample independent mission/site/event clusters, never individual autocorrelated telemetry samples.

## Scope and limitations

The current module validates declared uncertainty intervals and computes interval-robust Pareto dominance. It does not calculate confidence intervals, bootstrap replicate data, estimate statistical independence, predict battery RUL, fit degradation laws, forecast costs, or produce reliability probabilities. Evidence references and confidence intervals remain owner-supplied and must be independently reviewed.

`UNCERTAINTY_QUALIFIED_LIFECYCLE_PARETO_SCREEN_ONLY_NOT_LIFETIME_FORECAST_OR_SOTA_CLAIM`
