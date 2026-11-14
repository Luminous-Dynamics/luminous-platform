# Paired cluster bootstrap for lifecycle comparisons — v0.1

**Tracking:** [Issue #461](https://github.com/Luminous-Dynamics/luminous-platform/issues/461) · [Lifecycle frontier PR #462](https://github.com/Luminous-Dynamics/luminous-platform/pull/462)

**Status:** deterministic offline paired percentile bootstrap over pre-aggregated mission/site/event clusters. This is a statistical-analysis implementation, not a physical performance result.

## Method

The reference and candidate must share the exact scenario-manifest digest, cluster IDs, cluster level and scenario seed digest for every pair. All candidates use the same frozen cluster set. The implementation resamples whole cluster indices with replacement and applies the same sampled indices to both arms and all metrics, preserving pairing and within-cluster metric covariance.

Each cluster retains mission count, completed-mission count, cost, required/delivered/unserved critical energy, system-unavailable time, evidence status/class and an evidence reference. The validator requires current evidence and verifies that required service reconciles to delivered plus unserved service. Failed missions are **not removed**. Their counts and unserved service remain in the cluster and in each paired resample. A draw with zero total delivered critical service is rejected rather than silently censored.

The tool reports candidate-minus-reference deltas for lifecycle cost per delivered critical-service kWh, unserved critical-service Wh and system-unavailable hours. Cost is calculated from aggregate costs divided by aggregate delivered energy for each sampled draw; the other two axes are cluster means.

A deterministic SplitMix64 PRNG uses a declared SHA-256 seed and candidate identity. The pointwise percentile interval and full-sample paired mean are output at the declared confidence level. A second interval is calculated from a single shared resample stream across all candidates and endpoints: for each bootstrap replicate, the maximum absolute standardized error across the entire comparison family is retained, and its declared confidence quantile sets simultaneous bounds. The pointwise interval is descriptive; the simultaneous interval is used for preregistered practical-effect findings.

## Familywise comparison and practical-effect decisions

A multi-candidate benchmark creates a family of hypotheses: each candidate is compared with the reference on cost, unserved critical service and downtime. Treating every 95% interval independently increases false-positive risk as the family grows. v0.2 uses one shared resampling stream across every candidate and endpoint and computes the maximum absolute standardized bootstrap error on each replicate. The confidence quantile of that maximum defines simultaneous bounds over the full family. The method label is `MAX_ABS_STANDARDIZED_BOOTSTRAP_ERROR_V0_1`; this is not per-replicate studentized max-t and should receive independent statistical review and coverage simulation before high-stakes use.

The preregistered protocol digest and three positive minimum-practical-benefit thresholds are required inputs. For each endpoint independently: practical superiority is supported only when the simultaneous upper bound is below the negative benefit threshold; practical harm is supported only when the simultaneous lower bound exceeds the positive threshold; otherwise the result is `NotEstablished`. No aggregate winner is manufactured across unlike metrics.

## Sampling and independence

v0.1 requires at least 30 predeclared clusters and 1,000 bootstrap iterations. This is an input guard, not evidence of statistical power or a universal sample-size prescription. For serious multi-site superiority work, target at least 100 independent site/mission/event clusters where feasible, with a preregistered power and precision analysis.

The caller supplies a cluster-independence design reference and cluster level. Every cluster also retains current evidence status, evidence class and a provenance reference. Assumption-only records are rejected; synthetic fixtures remain explicitly synthetic. These are lineage checks, not proof of statistical independence or physical measurement accuracy. A data owner must justify independence and account for shared site, weather, feeder, maintenance, source/fuel, firmware and operator effects. Where clusters are nested or correlated, use a hierarchical or block bootstrap in a later version; do not pretend that records from one site are independent merely because IDs or seeds differ.

## Bias and claim controls

- Exact candidate/reference cluster and seed matching is mandatory. Each paired cluster must also have equal mission count and required critical-service demand, preventing a candidate from being evaluated against an easier workload.
- Failed mission records remain in the paired sample; no survivorship filtering. The report counts failed missions in each arm.
- All metrics use the same sampled cluster indices.
- Zero-delivery bootstrap draws fail closed; they are not dropped to improve the estimate.
- The output includes a pointwise percentile interval and a simultaneous max-absolute-standardized-bootstrap-error interval across every candidate/endpoint hypothesis. The latter is intended to address familywise multiplicity; it is not a BCa interval, does not use per-replicate studentization, and needs simulation/independent statistical review before high-stakes claims.
- This module does not prove independence, power, causal attribution or measurement uncertainty.
- Report metrology uncertainty from PR #462 alongside bootstrap sampling uncertainty; these are distinct uncertainty sources.

## Research targets

- 100% paired cluster/seed identity.
- Zero excluded failed missions or silently censored bootstrap draws.
- Reproducible report for identical inputs, algorithm version and seed.
- Use one shared resample stream across all candidates and endpoints; report familywise simultaneous bounds over the complete preregistered hypothesis family.
- Declare minimum practically meaningful effects for cost, unserved service and downtime before results are examined.
- Only label practical superiority when the simultaneous upper bound is below the negative minimum-benefit threshold; label harm only when the lower bound exceeds the positive threshold; otherwise report not established.
- Keep endpoint findings separate: do not turn three endpoints into a scalar overall winner.
- Pre-register minimum meaningful difference, confidence level, stopping rule and comparison family before observing results.
- Use independent cluster design and a power/precision calculation before a real superiority claim.

The protocol digest binds the preregistered endpoints, confidence level and minimum-practical-benefit thresholds. The evaluator validates digest shape but does not reconstruct protocol bytes. It does not prove cluster independence, power, causal attribution or measurement uncertainty.

Claim ceiling: `PAIRED_CLUSTER_BOOTSTRAP_SUMMARY_ONLY_NOT_INDEPENDENCE_PROOF_OR_FIELD_SUPERIORITY`.
