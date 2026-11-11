# Paired, Cluster-Aware Scenario Decision Tournament v0.1

**Date:** 11 November 2026  
**Implementation:** \`tools/measurement-audit/src/scenario_tournament.rs\`  
**Parent scenario methods:** [PR #407 — temporal scenarios and horizon reserve](https://github.com/Luminous-Dynamics/luminous-platform/pull/407) and [PR #409 — Gaussian-copula residual dependence](https://github.com/Luminous-Dynamics/luminous-platform/pull/409)  
**Forecast calibration:** [PR #406 — regime-conditioned session conformal calibration](https://github.com/Luminous-Dynamics/luminous-platform/pull/406)

## 1. Research problem: forecast scores do not certify a good reserve decision

Energy score, variogram score, interval coverage, PIT and ramp diagnostics are necessary for probabilistic forecast evaluation. They do not, by themselves, establish that a forecast produces a better reserve decision under a declared cost and shortfall constraint.

Decision-focused learning research in power systems makes this distinction explicit: statistical forecast accuracy and operational decision quality can rank methods differently. A 2026 systematic review of probabilistic forecasting-to-optimization workflows likewise identifies the limited coupling between sophisticated forecasts and downstream optimization objectives as a major research gap.

This tournament creates a reproducible **forecast-to-decision comparison** for the four scenario generators in #407/#409. Each method sees the exact same forecast, measured horizon, workload/model contract, clock/time origin, scenario count and reserve/shortfall cost ratio.

## 2. Four paired arms

Every episode requires exactly one report from each generator:
1. Independent marginal-quantile sampling — negative control for lost temporal dependence.
2. Gaussian copula — rank-normal dependence, regularized correlation and current forecast marginals.
3. Empirical residual trajectory bootstrap — complete historical error vectors sampled with replacement.
4. Schaake shuffle — historical rank transfer over the current forecast marginal sample set.

The baseline is always the independent-marginal arm. The candidate arms are compared to that same baseline, not to independently selected episodes or a different forecast.

The tournament rejects an episode unless all four reports agree on:
- forecast ID and origin ID;
- complete \`PowerModelContract\`;
- forecast evidence, regime/clock/time-origin and state/clock mapping references;
- physical measurement run, measurement evidence, time alignment and source timeline;
- exact measured source-sample evidence IDs;
- scenario count, horizon length and reserve-cost ratio / critical fractile;
- actual horizon energy, peak, ramp-up/down and maximum positive deviation.

It also checks that every report's shortfall quantity and realized reserve cost are algebraically consistent with its reserve and the frozen cost ratio.

## 3. Reserve decision and cost

The parent scenario evaluator defines the horizon-wide demand for reserve as the maximum positive deviation from the scheduled median path:

\[
D=\max_{t \in \mathrm{horizon}}\left(P_t^{\mathrm{actual}}-P_t^{\mathrm{median}}\right)_+
\]

For each scenario, it computes the same horizon-wide maximum deviation. Given the declared reserve/shortfall cost ratio \(c_R/c_S\), reserve is selected at the empirical critical fractile:

\[
q^*=1-\frac{c_R}{c_S}
\]

The realized normalized cost for one measured horizon is:

\[
C=c_R R+c_S(D-R)_+
\]

The current evaluator normalizes \(c_S=1\), so the reported cost units are \(C=(c_R/c_S)R+(D-R)_+\). The tournament requires an identical ratio and measured horizon across all four methods.

This is an offline decision proxy for scenario comparison. It is not a market-cleared reserve offer, grid-service delivery measurement, firm capacity commitment or accreditation.

## 4. Independent statistical unit and pairing

Episodes from one operating period may share weather, thermal state, workload bursts, deployment versions or common-cause conditions. The tournament therefore requires predeclared fixed-exposure operating clusters. All episodes in a cluster may be dependent.

The frozen contract defines:
- required strata (for example, season, temperature regime or workload regime);
- exact episode count per cluster;
- exact episode-window and cluster-window durations;
- non-overlapping episode windows within a cluster;
- minimum separation between clusters;
- minimum independent cluster count per stratum;
- cluster-independence review evidence;
- reserve/shortfall cost ratio, target savings and maximum shortfall fraction.

All four methods are paired within every episode. Each cluster is then reduced to one conservative comparison per candidate: average reserve cost over its fixed episode count, compared against the average independent-marginal baseline cost over the same episodes.

The cluster is a target-saving success only when its candidate's relative cost saving meets the preregistered threshold:

\[
S_c=\frac{\bar C_{c,\mathrm{independent}}-\bar C_{c,\mathrm{candidate}}}
{\bar C_{c,\mathrm{independent}}}
\ge s_{\mathrm{target}}
\]

A zero baseline cost does not count as a target-saving success; the relative saving is undefined.

## 5. Exact cluster-level confidence and multiple strata

For a candidate, the tournament reports the number of independent clusters whose average relative savings meet the target. It computes an exact one-sided Clopper–Pearson lower confidence bound for the probability that an independent cluster achieves at least that target.

The binomial trial count is the number of independent operating clusters—not the number of episode rows, scenario paths or time samples.

Each required stratum is evaluated separately. To make the set of stratum-specific lower bounds simultaneous at the contract's familywise confidence, this implementation uses a Bonferroni-adjusted one-sided confidence level:

\[
\gamma_{\mathrm{stratum}}
=1-\frac{1-\gamma_{\mathrm{family}}}{K}
\]

where \(K\) is the number of required strata. The overall summary aggregates costs and shortfalls descriptively, but intentionally does **not** publish a pooled binomial confidence bound across different strata. Overall qualification is the conjunction of all required stratum qualifications.

The exact lower bound is an interval for a cluster-level success probability under the declared independent-cluster sampling design. It is not a guarantee of future grid delivery. The review reference, fixed exposure and separation are auditable declarations; they cannot prove independence without independent statistical review.

## 6. Proposed research gate

The following values are internal confirmatory targets, not grid-code limits, regulatory standards or achieved results:

| Gate | Initial target |
|---|---:|
| Scenario methods | All four arms on identical paired episodes |
| Scenarios per forecast | 200 for direct Yao et al. method parity; 100/200/500 sensitivity track |
| Independent clusters | At least 30 per required stratum; 50+ preferred |
| Episodes per cluster | Fixed and preregistered; identical exposure in all clusters |
| Mean normalized reserve-cost reduction | At least 7.4% versus independent marginal sampling |
| Target-saving reliability | Exact one-sided cluster-success lower bound above the preregistered probability threshold |
| Shortfall constraint | Candidate shortfall-episode fraction at or below the same preregistered ceiling for every method |
| Evidence completeness | 100% paired forecast, model, measurement, clock, sample and cluster lineage |
| Regime robustness | Every required stratum qualifies separately; no pooled rescue of a weak stratum |

The 7.4% target is motivated by Yao et al.'s reported 7.4–10.6% reserve-cost reduction from temporal dependence reconstruction in its three studied tasks. It is a research target, not a predicted or achieved Luminous improvement. The paper's target was a GPU-IT forecasting benchmark; this tournament must not be presented as the same physical facility/PCC experiment.

A candidate must satisfy all of the following to qualify under the declared benchmark:
1. enough independent clusters in every required stratum;
2. overall mean cost reduction at or above the frozen target;
3. candidate shortfall fraction at or below the frozen ceiling;
4. exact cluster-level lower confidence bound at or above the preregistered target-saving reliability probability in every required stratum.

The independent arm's own shortfall rate and total shortfall magnitude remain visible. Candidate success cannot be claimed by hiding a worse shortfall outcome behind lower reserve procurement cost.

## 7. Industry and SOTA comparison

- **Decision-focused learning for power systems (IEEE Transactions on Power Systems, 2026):** provides a decision-focused taxonomy and open comparative benchmark with cost reduction, forecast accuracy and decision speed. The Power Fabric tournament adds exact paired scenario lineage, cluster-level reliability and an explicit shortfall gate.
- **Probabilistic day-ahead system-level demand and renewable forecasting (Nature Communications, 2026):** uses joint distributions and multivariate proper scores for reserve allocation. The Power Fabric track extends the decision comparison to compute-derived reserve scenarios, while keeping GPU-IT and PCC boundaries distinct.
- **Tail-risk-aware Bit-Watt co-optimization (IJEPES, September 2026):** combines workload, cooling, reserve and joint forecast uncertainty under SLA constraints, using a moment-based distributionally robust model with worst-case CVaR. This is a useful next optimizer comparator beyond the current empirical critical-fractile reserve decision.
- **Yao et al. (6 November 2026):** supplies the four scenario generator methods and reports study-specific reserve-cost improvements. Its 7.4–10.6% result is the reproduction motivation for the internal target, not a result from Luminous.

The next optimization frontier should add matched stochastic-programming and distributionally robust reserve policies (including CVaR), with identical scenario inputs, workload and SLO constraints. The comparison should include objective cost, decision latency, reserve shortfall, tail loss and out-of-distribution behavior. Do not compare different solvers on unmatched scenario distributions.

## 8. Implementation and validation status

The new Rust \`scenario_tournament\` module:
- accepts the four exact \`ScenarioEvaluationReport\` arms per episode;
- rejects mismatched forecast, model, measurement, horizon, clock, sample evidence, scenario count or reserve-cost contracts;
- checks the cost/shortfall algebra before comparison;
- freezes generator IDs and revisions across the tournament;
- validates exact per-episode and per-cluster exposure, separation and per-stratum support;
- compares candidate cluster-mean costs against the same independent-marginal baseline;
- calculates exact one-sided Clopper–Pearson lower bounds using clusters as trials;
- adjusts confidence across required strata and withholds a pooled overall confidence bound;
- requires bootstrap/Schaake selected historical residual records to match the same complete Gaussian-copula residual pool, including raw residual values, origin/target windows, model/regime/clock/time-origin and sample evidence;
- retains cluster design, episode, forecast, measurement-run and measured-sample evidence references;
- marks candidate qualification only when mean savings, shortfall ceiling, cluster reliability and every stratum's support pass.

Twenty-one synthetic Rust tests are authored. They cover exact binomial bounds, successful paired comparisons, no-improvement rejection, missing or duplicate arms, mismatched measurement, generator revision drift, cluster exposure, episode overlap, duplicate forecast origins, evidence reuse, missing strata and familywise multi-stratum confidence.

**Tests are authored but not executed.** No Rust compilation, Cargo test, formatting, Clippy, runtime benchmark, public dataset import, physical reserve delivery, PCC test, grid-service qualification, market participation or accreditation is claimed.

## Sources

- [Decision-Focused Learning for Power System Decision-Making Under Uncertainty](https://doi.org/10.1109/TPWRS.2025.3597806), *IEEE Transactions on Power Systems*, 41(1), January 2026. Decision-focused benchmark dimensions include cost reduction, forecast accuracy and decision speed.
- [From Probabilistic Forecasting to Optimization Decisions in Renewable-Integrated Power Systems: A Systematic Review and Quantitative Taxonomy of the Decision Chain](https://www.sciencedirect.com/science/article/pii/S136403212600818X), *Renewable and Sustainable Energy Reviews*, 2026. Highlights the limited integration of advanced probabilistic forecasting with downstream optimization objectives and identifies decision-focused learning and conformal + stochastic/robust optimization as research priorities.
- Wang et al., [Tail-risk-aware Bit-Watt co-optimization for data center integrated energy systems under joint forecast uncertainty](https://doi.org/10.1016/j.ijepes.2026.112227), *International Journal of Electrical Power & Energy Systems*, September 2026. SLA-constrained workload/cooling/reserve optimization under joint uncertainty with worst-case CVaR.
- Terrén-Serrano et al., [Probabilistic day-ahead forecasting of system-level renewable energy and electricity demand](https://doi.org/10.1038/s41467-026-69015-w), *Nature Communications*, 2026. Joint probabilistic forecasts and multivariate scoring for operational reserve allocation.
- Yao et al., [Grid-Oriented Probabilistic Load Forecasting for AI Data Centers](https://arxiv.org/abs/2611.08929), arXiv:2611.08929, submitted 6 November 2026. Four temporal-scenario arms and study-specific 7.4–10.6% reserve-cost reduction from temporal-dependence reconstruction.
