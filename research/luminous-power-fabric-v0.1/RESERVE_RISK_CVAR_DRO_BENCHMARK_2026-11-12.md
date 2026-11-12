# Reserve-risk optimisation: empirical CVaR and finite-support TV-DRO

**Date:** 12 November 2026  
**Implementation:** \`tools/measurement-audit/src/reserve_risk.rs\`  
**Parent evidence:** [PR #410 — paired, cluster-aware scenario decision tournament](https://github.com/Luminous-Dynamics/luminous-platform/pull/410)  
**Branch:** \`research/luminous-power-fabric-cvar-dro-v0.27\`

## 1. Research question

The previous tournament asks which scenario generator produces better reserve decisions. This increment asks a different question:

> Given the same validated scenario trajectories, which reserve policy offers the best cost-risk trade-off without concealing shortage probability or tail severity?

The new layer compares three policies on the same validated \`ForecastScenarioSet\`:

1. **Empirical critical-fractile baseline.** Minimise empirical expected reserve-plus-shortfall cost on a frozen discrete reserve grid.
2. **Empirical CVaR-constrained policy.** Minimise empirical expected cost subject to empirical CVaR and shortfall-probability limits.
3. **Finite-support total-variation DRO/CVaR.** Minimise worst-case expected cost over a reweighted empirical distribution, subject to worst-case CVaR and shortfall-probability limits.

All methods use the same forecast ID and origin, complete \`ScenarioGenerationContract\`, model/regime/clock/time-origin contract, source timeline, scenario paths, reserve-cost ratio and reserve grid. The risk evaluator calls the parent scenario-set validator before deriving any demand sample. It does not accept a caller-supplied unbound list of summary values.

## 2. Loss construction

For scenario path \(i\), the reserve demand is the maximum positive deviation from the scheduled median path over the full horizon:

\[
D_i=\max_t\left(P_{i,t}-P_t^{\mathrm{median}}\right)_+.
\]

For reserve \(R\), shortfall is:

\[
L_i(R)=(D_i-R)_+.
\]

With the shortfall cost normalised to one and reserve cost ratio \(r=c_R/c_S\), realised modelled cost is:

\[
C_i(R)=rR+L_i(R).
\]

The empirical baseline minimises:

\[
\widehat{\mathbb E}[C(R)]
=rR+\frac{1}{N}\sum_{i=1}^N L_i(R).
\]

The reserve candidates are an explicitly frozen, inclusive integer-MW grid. A coarse grid is a modelling choice and must be held identical across all policies. The critical-fractile baseline is therefore the minimum empirical-cost decision on that grid, rather than an unreported continuous optimum.

## 3. Empirical CVaR

For confidence \(\alpha\), empirical upper-tail CVaR of shortfall is computed with fractional weighting of the final order statistic:

\[
\widehat{\operatorname{CVaR}}_\alpha(L)
=\frac{1}{(1-\alpha)N}
\left(\sum_{j=1}^{k}L_{(N-j+1)}
+\delta L_{(N-k)}\right),
\]

where \((1-\alpha)N=k+\delta\), \(k\) is an integer and \(0\leq\delta<1\). This avoids silently rounding a non-integer tail count.

The implementation records both:
- shortfall probability \(\widehat P(L>0)\); and
- empirical CVaR of shortfall in MW.

These are separate constraints. A lower expected cost cannot compensate for a violated tail-severity or shortage-probability limit.

## 4. Finite-support total-variation ambiguity

The robust arm defines the ambiguity set over the observed scenario support only:

\[
\mathcal Q_\rho=
\left\{q\in\Delta_N:
\frac{1}{2}\sum_{i=1}^{N}\left|q_i-\frac{1}{N}\right|
\leq\rho\right\}.
\]

For a vector of path losses \(z_i\), the finite-support worst-case expectation is:

\[
\sup_{q\in\mathcal Q_\rho}\sum_i q_i z_i.
\]

The implementation solves this exactly for the uniform empirical reference distribution by transferring probability mass from the lowest-valued paths to a highest-valued path until the TV budget or donor mass is exhausted.

Worst-case CVaR is evaluated through the finite-support representation:

\[
\sup_{q\in\mathcal Q_\rho}\operatorname{CVaR}_\alpha^q(L)
=
\min_{\eta}
\left[
\eta+
\frac{
\sup_{q\in\mathcal Q_\rho}
\mathbb E_q[(L-\eta)_+]
}{1-\alpha}
\right].
\]

The implementation checks the candidate breakpoints at the observed loss values. The robust policy minimises worst-case expected reserve-plus-shortfall cost over the reserve grid, subject to both worst-case CVaR and worst-case shortage-probability gates.

**Important scope boundary:** this is a finite-support reweighting ambiguity set. It cannot assign probability to an event absent from the scenario pool, and a selected radius is not automatically a confidence region for the unknown real-world distribution. It is a controlled stress envelope over observed support. Radius selection must be calibrated on a separate training/calibration partition, frozen before the confirmatory evaluation, and sensitivity-tested. A future Wasserstein or moment-based ambiguity model may provide a broader support treatment, but requires a separate derivation and benchmark.

## 5. What is now implemented

The registered Rust module \`reserve_risk\`:

- reuses the validated temporal scenario-set boundary;
- derives one horizon-maximum surplus sample from each complete trajectory;
- binds the report to forecast ID/origin/time, forecast evidence, clock, time-origin, state mapping, time alignment, timeline digest and complete scenario-generation contract;
- computes empirical expected cost, empirical shortage probability, worst-case shortage probability, empirical mean shortfall, worst-case mean shortfall, empirical CVaR and worst-case CVaR for every reserve candidate;
- selects the empirical critical-fractile baseline, empirical CVaR-constrained policy and TV-DRO/CVaR policy;
- replays each available policy against the exact subsequently measured horizon, after deterministic recomputation of the full risk report; it retains the measurement-run/evidence IDs, timestamp alignment, source timeline and each measured sample ID;
- keeps the critical-fractile baseline visible even when it violates the declared empirical or robust risk gates; its qualification flag is false rather than silently treating the baseline as safe;
- emits a three-objective Pareto frontier over worst-case expected cost, worst-case CVaR and worst-case shortage probability;
- reports a constrained policy as absent when no reserve candidate meets its frozen gates, rather than silently weakening a threshold;
- rejects insufficient scenario support, insufficient effective tail-path mass, invalid reserve grids, blank decision evidence and invalid probability/risk parameters.

Twenty-three synthetic Rust tests are authored for fractional-tail calculations, TV reweighting, robust-CVaR ordering, shortage-probability stress, horizon-surplus derivation, support checks, grid validation, Pareto/frontier generation, infeasibility and policy selection.

**Tests are authored but not executed.** No Rust compilation, Cargo test, rustfmt, Clippy, runtime benchmark or measured performance result is claimed.

## 6. Benchmark protocol and target ladder

### Paired experiment design

Use the four scenario generators from #410 (independent marginal, Gaussian copula, empirical trajectory bootstrap and Schaake shuffle). For every forecast origin, evaluate all three reserve policies on the exact same scenario set and measured horizon. Keep generator, seed, horizon, workload/model contract, reserve cost ratio, reserve grid, SLO constraints and measurement boundaries frozen.

| Dimension | Confirmatory design |
|---|---|
| Scenario count | 200 minimum parity track; 500 preferred for CVaR95; 1,000 for a useful CVaR99 sensitivity track |
| CVaR tail support | At least 10 effective paths in the declared tail; 200 paths at 95% gives 10 |
| Independent operating clusters | At least 30 per required stratum; 50+ preferred |
| Exposure | Fixed episodes per cluster; episode dependence allowed only inside the cluster; disjoint cluster design reviewed independently |
| Ambiguity radius | Pre-register 0, 0.01, 0.05 and 0.10 sensitivity tracks; tune/freeze any selected production radius on calibration data only |
| Holdout | Chronologically later, disjoint episodes and clusters; no reuse of forecast origins, residual blocks or calibration records |
| Strata | Require every predeclared workload, thermal, season and operating regime to pass; no pooled rescue of a weak stratum |
| Evidence | 100% paired forecast/scenario/measurement/clock/sample/cluster lineage |

At \(\alpha=0.95\), 200 paths contribute ten effective tail paths; 500 contribute 25; 1,000 contribute 50. At \(\alpha=0.99\), 1,000 paths contribute ten. These are arithmetic support requirements, not guarantees that the tail is well estimated. Report sensitivity to scenario count and resampling uncertainty.

### Metrics that must be published together

1. Empirical expected normalized cost and worst-case expected normalized cost.
2. Empirical and worst-case shortfall probability.
3. Empirical and worst-case CVaR95 shortfall in MW; CVaR99 as a sensitivity track.
4. Mean shortfall, total shortfall and worst observed shortfall.
5. Reserve MW, unused reserve and reserve utilisation.
6. Cost-risk Pareto frontier, not just one selected operating point.
7. Results by workload, thermal, season and regime strata.
8. Decision computation time and memory at 200/500/1,000 scenarios and 100/500/1,000 reserve candidates.
9. Out-of-distribution degradation and recovery after a regime change.
10. Evidence completeness, missingness, rejected inputs and qualification claim ceiling.

### Internal research targets (not achieved results)

- Reproduce the #410 temporal-dependence target of at least 7.4% lower normalized reserve cost than independent marginal sampling while preserving the same shortfall ceiling and cluster-reliability gate. The 7.4% figure comes from a study-specific external result; it is not a Luminous result.
- Test whether the TV-DRO policy can reduce worst-case expected cost by at least 5% relative to the empirical critical-fractile baseline **without increasing** the predeclared worst-case CVaR or worst-case shortfall-probability limits. This is a research target, not a guaranteed optimizer property.
- Achieve zero hidden constraint violations: a policy is feasible only when every declared cost, empirical-tail and robust-tail constraint passes.
- Achieve 100% source-contract and sample-lineage completeness in the confirmatory evaluation.
- Target sub-second decision computation for 200–500 scenarios and up to 500 reserve candidates on a declared reference host; publish measured p50/p95 latency and host specification before making any speed claim.
- Require no statistically supported degradation in any required stratum. If one regime fails, expose it and lower the claim ceiling rather than average it away.

## 7. Industry and state-of-the-art comparison

| Reference | What it contributes | How the Power Fabric track should compete |
|---|---|---|
| Yao et al., 6 Nov 2026 | Four scenario-generation families and a GPU-IT forecast-to-reserve comparison; study reports 7.4–10.6% reserve-cost reductions from temporal dependence reconstruction | Reproduce the scenario tournament on licensed, disjoint evidence, then add constrained empirical/robust tail-risk policies and cluster-level uncertainty |
| Wang et al., September 2026, Bit-Watt | SLA-constrained joint workload, cooling, multi-energy and reserve co-optimization with moment-based DRO and worst-case CVaR | Do not claim equivalence: current code is a finite-support reserve-policy subsystem. The next major step is matched joint workload/cooling/storage/compute optimization with explicit SLA and equipment constraints |
| IEEE 2026 decision-focused learning benchmark | Makes cost reduction, forecast quality and decision speed joint comparison dimensions | Add tail constraints, policy feasibility, cluster reliability, out-of-distribution recovery and evidence completeness to the scorecard |
| ASHRAE 2026 grid-interactive data-centre framework | Separates load flexibility/demand response from inverter-based grid support and emphasizes uptime, controls and verifiable response | Preserve separate workload, storage and inverter control claims; only credit grid response at a physically measured PCC boundary |
| IEA 4E 2026 flexibility report | Distinguishes market-, grid- and system-serving flexibility and documents deployment barriers | Treat operational permissions, workload contracts, local grid constraints and economics as first-class benchmark fields |
| NIST transactive-services work, September 2026 | Defines interoperable messaging for local energy markets and flexible edge assets | Add a future protocol adapter, but never give an external market or utility direct authority over protected workload or facility controls |

The current literature already contains more ambitious full-system DRO/CVaR formulations. The differentiator cannot simply be “Power Fabric has CVaR.” The defensible differentiator is a reproducible chain from versioned compute workload through forecast/scenario evidence, reserve choice, constraint checks, physical measurement and independent qualification.

## 8. Next paradigm shifts

### A. From a single reserve number to a capability envelope

Represent flexibility as a time-indexed envelope with upward/downward MW, duration, ramp, recovery, rebound, minimum notice, confidence, thermal state, workload/SLO state and equipment constraints. A point estimate of “available MW” hides the most important operational limits.

### B. From independent subsystems to a coupled digital twin

The next optimizer should co-schedule:
- deferrable and migratable compute jobs with deadlines and tenant SLOs;
- GPU power caps, DVFS, batch size and concurrency;
- cooling and thermal storage with temperature, water and equipment constraints;
- batteries and on-site generation with state-of-charge, reserve and degradation;
- interconnection/PCC limits and any applicable market/service envelope.

Every constraint must be bound to its evidence source. A solver's feasible answer is a model result until hardware and PCC measurements verify it.

### C. From risk scores to a recovery-aware service contract

Add post-event recovery and rebound constraints. A data centre that reduces 10 MW for an event but creates a coincident 12 MW recovery spike may transfer rather than solve the grid problem. The benchmark should score the event and its recovery window as one service episode.

### D. From average performance to common-cause resilience

Stress correlated loss of grid supply, cooling, UPS strings, generators, network links and workload orchestration. A reserve contribution must be derated when the same common cause can disable both the facility load response and the resource claimed to support it.

### E. From opaque optimizer outputs to proof-carrying decisions

Every reserve commitment should include the exact scenario set, risk contract, constraints, solver/version, optimization gap, selected action, fallback, measurement boundary, authority and expiry. Unknown execution outcome should block a new mutation until reconciled.

## 9. Next qualification step

The replay evaluates a selected reserve against one measured horizon and reports the corresponding modelled cost and shortfall. The next validation increment must aggregate these per-episode outcomes by independently reviewed operating clusters, using the fixed-exposure design in #410, before making a policy-level reliability claim.

## 10. Explicit non-claims

This increment does **not** demonstrate:
- a statistically calibrated TV ambiguity radius;
- coverage of events outside the supplied scenario support;
- a full Wasserstein/moment-based DRO model;
- joint optimal workload/cooling/battery/generation dispatch;
- measured facility load response or PCC reserve;
- market participation, grid-service accreditation or firm capacity;
- execution, compile, test, format, Clippy or runtime success.

It is a new offline, modelled reserve-risk decision benchmark and an engineering foundation for the larger optimisation and physical qualification programme.
