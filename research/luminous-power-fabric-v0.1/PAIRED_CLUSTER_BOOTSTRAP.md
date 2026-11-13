# Paired cluster bootstrap for lifecycle comparisons — v0.1

**Tracking:** [Issue #461](https://github.com/Luminous-Dynamics/luminous-platform/issues/461) · [Lifecycle frontier PR #462](https://github.com/Luminous-Dynamics/luminous-platform/pull/462)

**Status:** deterministic offline paired percentile bootstrap over pre-aggregated mission/site/event clusters. This is a statistical-analysis implementation, not a physical performance result.

## Method

The reference and candidate must share the exact scenario-manifest digest, cluster IDs, cluster level and scenario seed digest for every pair. All candidates use the same frozen cluster set. The implementation resamples whole cluster indices with replacement and applies the same sampled indices to both arms and all metrics, preserving pairing and within-cluster metric covariance.

Each cluster retains mission count, completed-mission count, cost, required/delivered/unserved critical energy, system-unavailable time, evidence status/class and an evidence reference. The validator requires current evidence and verifies that required service reconciles to delivered plus unserved service. Failed missions are **not removed**. Their counts and unserved service remain in the cluster and in each paired resample. A draw with zero total delivered critical service is rejected rather than silently censored.

The tool reports candidate-minus-reference deltas for lifecycle cost per delivered critical-service kWh, unserved critical-service Wh and system-unavailable hours. Cost is calculated from aggregate costs divided by aggregate delivered energy for each sampled draw; the other two axes are cluster means.

A deterministic SplitMix64 PRNG uses a declared SHA-256 seed and candidate identity. The percentile interval and full-sample paired mean are output at the declared confidence level.

## Sampling and independence

v0.1 requires at least 30 predeclared clusters and 1,000 bootstrap iterations. This is an input guard, not evidence of statistical power or a universal sample-size prescription. For serious multi-site superiority work, target at least 100 independent site/mission/event clusters where feasible, with a preregistered power and precision analysis.

The caller supplies a cluster-independence design reference and cluster level. Every cluster also retains current evidence status, evidence class and a provenance reference. These are lineage checks, not proof of statistical independence or physical measurement accuracy. A data owner must justify independence and account for shared site, weather, feeder, maintenance, source/fuel, firmware and operator effects. Where clusters are nested or correlated, use a hierarchical or block bootstrap in a later version; do not pretend that records from one site are independent merely because IDs or seeds differ.

## Bias and claim controls

- Exact candidate/reference cluster and seed matching is mandatory.
- Failed mission records remain in the paired sample; no survivorship filtering. The report counts failed missions in each arm.
- All metrics use the same sampled cluster indices.
- Zero-delivery bootstrap draws fail closed; they are not dropped to improve the estimate.
- The output is a percentile interval, not a BCa/studentized interval and not a multiple-comparison-adjusted interval.
- This module does not prove independence, power, causal attribution or measurement uncertainty.
- Report metrology uncertainty from PR #462 alongside bootstrap sampling uncertainty; these are distinct uncertainty sources.

## Research targets

- 100% paired cluster/seed identity.
- Zero excluded failed missions or silently censored bootstrap draws.
- Reproducible report for identical inputs, algorithm version and seed.
- Report paired mean deltas and confidence bounds for cost, unserved service and downtime separately.
- Pre-register minimum meaningful difference, confidence level, stopping rule and comparison family before observing results.
- Use independent cluster design and a power/precision calculation before a real superiority claim.

Claim ceiling: `PAIRED_CLUSTER_BOOTSTRAP_SUMMARY_ONLY_NOT_INDEPENDENCE_PROOF_OR_FIELD_SUPERIORITY`.
