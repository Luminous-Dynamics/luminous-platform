# Cluster-Aware Repeated-Event Power-Flexibility Qualification v0.1

**Date:** 11 November 2026  
**Parent evaluator:** [PR #396 — repeated-event grid-interactive compute resource qualification](https://github.com/Luminous-Dynamics/luminous-platform/pull/396)  
**Parent research issue:** [#395 — counterfactual M&V and repeated-event qualification](https://github.com/Luminous-Dynamics/luminous-platform/issues/395)  
**Related programme increments:** [#401 power-state model](https://github.com/Luminous-Dynamics/luminous-platform/pull/401), [#402 policy tournament](https://github.com/Luminous-Dynamics/luminous-platform/pull/402), [#403 fleet dependence](https://github.com/Luminous-Dynamics/luminous-platform/pull/403), [#404 migration response](https://github.com/Luminous-Dynamics/luminous-platform/pull/404)

## 1. The missing statistical unit

PR #396 already retains failed dispatches, computes conservative compute-attributed capacity, checks response/sustain/QoS/recovery/rebound and reports an event-level P10 plus a conditional Wilson lower bound. Its own limitation is important: the Wilson binomial interpretation assumes representative independent trials. A boolean declaration and a sampling-design identifier cannot establish that repeated events are independent.

That matters for a data centre. Thirty dispatches across three hot afternoons, one model-release incident or one synchronized workload burst are not thirty independent observations. Treating every event as independent can make a confidence interval look much stronger than the evidence supports.

This increment changes the statistical unit from **dispatch event** to **predeclared independent operating cluster** (for example, an appropriately separated site-day or independently randomized operating episode). Dependence within a cluster is unrestricted. Only clusters, not event rows, contribute independent statistical trials.

## 2. Cluster and event contracts

The evaluator consumes the exact GridServiceQualificationReport from grid_service::evaluate_grid_service_sweep; it does not recalculate the PCC baseline, non-compute attribution, uncertainty, service gates or credited event capacity.

A frozen ClusteredReliabilityContract binds:
- the exact source service-contract ID and event-sampling-design ID;
- a source-report evidence reference;
- cluster-sampling and event-assignment design IDs/evidence;
- the complete list of required operating strata;
- the lower-tail quantile and one-sided confidence target;
- an explicit independence-review evidence reference and minimum cluster separation.

Each IndependenceCluster has a stable ID, one declared operating stratum, a half-open sampling window and design evidence. The contract freezes one common window duration and an exact event count per cluster, so cluster minima do not mix different observation exposures or different event counts. Every source dispatch event must map to exactly one cluster with an assignment evidence reference. The complete dispatch and recovery interval must fit inside the cluster window. Cluster windows cannot overlap; when independence is claimed, the declared minimum separation is enforced.

The independence review must justify the unit and separation from the operating process: site, season/thermal state, workload regime, dispatch cadence, common-cause events, maintenance and recovery. The implementation checks the declared window and evidence structure; it cannot prove independence from identifiers or timestamps alone.

## 3. Conservative cluster outcomes

For a cluster c, define:

S_c = product over events e in c of 1[event e passes the full commitment]

C_c = minimum event-level credited capacity across events e in c

A cluster succeeds only when **every** event in it succeeds. Its credited capacity is the minimum event-level capacity. A failed event therefore fails the entire cluster and contributes zero capacity through the source evaluator. These worst-case block summaries allow arbitrary dependence among events within the cluster.

Every required operating stratum is evaluated separately. The overall result is the weakest supported stratum, not a pooled average that can hide winter, hot-weather, workload or destination-headroom failure.

## 4. Distribution-free lower-tail capacity support

For each required stratum, the evaluator reports the empirical nearest-rank lower quantile of cluster-minimum capacities. It also computes a conservative one-sided lower confidence bound for the population lower quantile.

Let q be the target lower-tail probability (0.10 for P10), n the number of independent clusters and M_n the sample minimum. For population quantile Q_q:

P(M_n <= Q_q) >= 1 - (1 - q)^n

Therefore the sample minimum is a distribution-free one-sided lower confidence bound once:

n >= ceil( ln(1 - gamma) / ln(1 - q) )

where gamma is the requested confidence. This is intentionally conservative: it avoids assuming a parametric tail distribution or selecting a bootstrap block length for the primary capacity gate.

| Target | Required independent clusters per stratum |
|---|---:|
| P10 lower bound, 90% one-sided confidence | 22 |
| P10 lower bound, 95% one-sided confidence | 29 |
| P05 lower bound, 90% one-sided confidence | 45 |
| P05 lower bound, 95% one-sided confidence | 59 |

These counts apply to independent clusters **within each declared stratum**. Thirty dispatches in three clusters do not satisfy a 22-cluster P10/90% requirement.

The cluster-level success probability is reported separately using a one-sided Wilson score lower bound with the number of independent clusters as its denominator and the success definition S_c=1. Its z-score is frozen explicitly in the clustered contract, separate from the confidence level used for the distribution-free capacity bound. This is a block-level “all events succeeded” reliability statement, not the per-event success probability. Wilson's normal-score interval is approximate.

## 5. Why this is not just a bootstrap

Moving/stationary block bootstrap methods are valuable for estimation under suitable weakly dependent or stationary processes, but their validity depends on dependence and block-length assumptions. Künsch (1989) and Politis & Romano (1994) provide foundational methods for dependent observations.

For a capacity commitment, the primary gate here instead uses predeclared operational clusters, worst-case within-cluster outcomes and a distribution-free order-statistic lower bound. A future exploratory block-bootstrap interval can be reported alongside this gate when the dependence model, stationarity assumptions, block-length selection and sensitivity analysis are preregistered. It must not replace the conservative commitment gate after observing outcomes.

## 6. Benchmark and comparator programme

| Comparator / layer | What it establishes | Required Luminous comparison |
|---|---|---|
| PR #396 event-level P10 + Wilson | Repeated dispatch qualification under an independent-event assumption | Run event-level and cluster-level reports on the exact same source report; publish the difference and why |
| Künsch / Politis–Romano block resampling | Inference methods for dependent stationary observations | Optional sensitivity arm, not the primary capacity gate |
| 2026 Microsoft compositional power traces | Configuration- and state-conditioned workload power traces; <5% median absolute energy error in most tested configurations | Only compare after matching measured hardware/configuration, state definitions and electrical boundary |
| MLPerf Power 2026 | Shared energy metrics such as samples/joule and tokens/joule across systems | Preserve accepted useful work and measured boundary; do not equate replayed resource time with energy |
| LAD-Flex (Caprara et al., 2026) | Up to 22% modelled load deferral in its one-week, >1M-task Alibaba-trace case | Reproduce on permitted matched traces; report clustered site-delivery reliability separately from modelled deferral |
| PR #403 fleet aggregation | Cross-site contemporaneous/lagged dependence and common-mode scenarios | Cluster dispatch experiments across synchronized/common-mode site periods; no independence credit from site count alone |

The 22%, <5% and MLPerf figures above are external study results, not Luminous measurements or achieved improvements. Comparisons require the same workloads, boundaries, trial populations and claim tiers.

## 7. Proposed gates

These are internal research targets, not grid-code limits, market rules, standards or achieved results.

- Retain the source evaluator's minimum total dispatch-event count and all event-level eligibility gates.
- At least 22 independent operating clusters per required stratum for a P10/90% distribution-free lower bound; use 29 per stratum for P10/95%.
- Zero pooled qualification when any required stratum lacks independent-cluster support.
- A cluster fails when any contained dispatch fails; no event removal after observing its outcome.
- 100% one-to-one assignment of source events to predeclared clusters, with complete dispatch and recovery containment.
- Report event-level and cluster-level denominators, per-stratum support, all failed events, empirical P10, lower confidence bound and Wilson block-success bound separately.
- Keep site/POI, workload, control action, baseline, quality contract and measurement boundary frozen through the source contract.
- Do not extrapolate cluster results across sites, grid zones, hardware configurations, seasons or service classes.

## 8. Rust implementation and validation status

The new clustered_reliability module:
- consumes the existing grid-service report and preserves its event-level source evidence;
- validates source-report consistency, exact event assignment, cluster windows, recovery containment, declared separation and required strata;
- computes worst-event capacity and all-events-success per cluster;
- computes the distribution-free lower-tail capacity support requirement and stratified cluster-level Wilson bounds;
- withholds inferential outputs when cluster independence is not declared or support is inadequate.

Nineteen synthetic Rust unit tests are authored. They cover 22/29-cluster thresholds, valid 30-event/30-cluster qualification, 30 events grouped into three clusters, within-cluster failure, per-stratum support, undeclared independence, assignment omissions/duplicates/unknown IDs, unequal cluster exposure, minimum event-count rejection, cluster separation, event/recovery containment, source-contract mismatch, inconsistent source outcomes and unused clusters.

**Tests are authored but not executed.** No Rust compilation, cargo test, formatting, Clippy, runtime benchmark, physical measurement, field dispatch, grid-service qualification, market participation or capacity accreditation is claimed. The module is read-only and cannot authenticate the referenced evidence or prove statistical independence. An independent reviewer must approve the cluster design before inferential use.

## Sources

- H. R. Künsch, [The Jackknife and the Bootstrap for General Stationary Observations](https://doi.org/10.1214/aos/1176347265), *The Annals of Statistics*, 1989.
- D. N. Politis and J. P. Romano, [The Stationary Bootstrap](https://doi.org/10.1080/01621459.1994.10476870), *Journal of the American Statistical Association*, 1994.
- NIST/SEMATECH, [Wilson confidence intervals for proportions](https://www.itl.nist.gov/div898/handbook/prc/section2/prc241.htm).
- Wilkins, Kazhamiaka and Rajagopal, [From Servers to Sites: Compositional Power Trace Generation of LLM Inference for Infrastructure Planning](https://www.microsoft.com/en-us/research/publication/from-servers-to-sites-compositional-power-trace-generation-of-llm-inference-for-infrastructure-planning/), Microsoft Research, March 2026.
- [MLPerf Power: Addressing the Artificial Intelligence Power Wall](https://ieeexplore.ieee.org/abstract/document/11551311), *IEEE Micro*, September–October 2026.
- Caprara et al., [Data center workload flexibility for power system demand response: Evidence from Alibaba traces](https://doi.org/10.1016/j.ijepes.2026.111940), *International Journal of Electrical Power & Energy Systems*, May 2026.
