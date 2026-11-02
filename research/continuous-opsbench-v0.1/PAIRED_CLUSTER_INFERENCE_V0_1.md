# Paired cluster inference and rare-event assurance v0.1

## Purpose

This module makes the benchmark's statistical reporting deterministic and auditable. It operates on independent scenario/fault-schedule/topology clusters, with Candidate and Baseline records paired within each cluster. It does not treat requests, observer windows, attempts or faults nested inside one cluster as independent samples.

The implementation is dependency-free Rust in `src/statistics.rs`. Strict input and result schemas are:
- `cluster-inference-input.schema.json`
- `cluster-inference-result.schema.json`

## Paired data contract

Each cluster has exactly one Candidate record and one Baseline record. Each arm carries:
- fault-schedule digest;
- topology digest;
- workload-profile digest;
- offered-demand-contract digest;
- planned fault denominator;
- verified recoveries;
- known non-recoveries;
- indeterminate recoveries.

Candidate and Baseline digests must match exactly within a cluster. Both arms must have the same planned fault denominator. The outcome counts must close exactly:

`verified + known_nonrecoveries + indeterminate = planned_faults`

Missing, duplicate, mismatched, overflowed or non-closing records are rejected. The input data structure keeps the two arm records separate so the verifier can actually detect a candidate/baseline digest mismatch.

## Estimands: outcomes and confidence are different intervals

An indeterminate outcome is not necessarily a failed recovery and is not a verified success. It produces an outcome-classification interval.

For each arm:

`verified_rate_lower = verified / planned`

`verified_rate_upper = (verified + indeterminate) / planned`

`indeterminate_rate = indeterminate / planned`

For every paired cluster:

`effect_lower = candidate_lower - baseline_upper`

`effect_upper = candidate_upper - baseline_lower`

The reported treatment-effect interval therefore handles missing classification in the adverse direction. Example:

- Candidate: 8 verified, 1 known non-recovery, 1 indeterminate out of 10 -> 80%-90%.
- Baseline: 5 verified, 4 known non-recoveries, 1 indeterminate out of 10 -> 50%-60%.
- Conservative Candidate-minus-Baseline effect -> +20 to +40 percentage points.

This is an **outcome uncertainty bound**, not a statistical confidence interval.

The point estimand is the equally weighted mean of per-cluster rates. Clusters, not their internal fault counts, receive equal weight. Candidate and Baseline are paired within every cluster.

## Bootstrap contract

Algorithm identifier: `splitmix64-paired-cluster-bootstrap-v1`.

1. Sort validated clusters by cluster ID so input order cannot change the result.
2. Seed a deterministic SplitMix64 resampler from the first 64 bits of the externally committed seed digest.
3. Draw exactly N cluster indices with replacement for each replicate, where N is the number of independent clusters.
4. Calculate equally weighted means across the sampled paired clusters for Candidate/ Baseline lower and upper rates, indeterminate rates, and the two paired-effect endpoints.
5. Sort each replicate statistic and report nearest-rank percentile intervals. For B replicates and quantile q, the index is `ceil(q × B) − 1`.

The default confirmatory target is 10,000 replicates; the implementation accepts 1,000-100,000. The PRNG is **not cryptographic** and is not used for treatment assignment. The seed digest and exact plan must be frozen/authenticated upstream. Bootstrap reproducibility does not establish randomization quality or causal identification.

## Superiority gate

The benchmark reports:
- Candidate/Baseline confidence intervals for verified lower rates;
- Candidate/Baseline confidence intervals for maximum-possible upper rates;
- Candidate/Baseline indeterminate-rate confidence intervals;
- confidence intervals for conservative lower and upper paired-effect endpoints;
- point estimates for the paired effect bounds and indeterminate rates.

Superiority is established only when:
1. the lower confidence bound for the conservative Candidate-minus-Baseline effect exceeds the preregistered minimum meaningful effect; and
2. the **upper confidence bound** on the mean indeterminate rate for both arms is no greater than the preregistered indeterminate ceiling.

If the indeterminate upper bound exceeds its ceiling, the verdict is `indeterminate_coverage_exceeded`, not success or failure. Otherwise, failure to clear the effect threshold is `not_established`. There is no compensatory aggregate score.

## Rare-event safety bounds

`clopper_pearson_upper(events, trials, confidence)` calculates a one-sided exact Clopper-Pearson binomial upper bound. It solves:

`P_p[X <= events] = alpha`, where `alpha = 1 - confidence`.

For zero observed events, it uses the exact closed form:

`p_upper = 1 - alpha^(1 / N)`

At 95% confidence:
- 0 / 500 independent clusters -> upper bound approximately 0.5974%;
- 0 / 1,000 independent clusters -> approximately 0.2991%;
- 0 / 3,000 independent clusters -> approximately 0.0998%.

`zero_event_sample_size(maximum_risk_ppm, confidence)` returns the minimum independent zero-event cluster count needed to put the one-sided bound at or below the risk ceiling. For example, a 0.1% ceiling at 95% confidence requires 2,995 independent clusters.

The general exact-tail implementation is intentionally bounded to 100,000 observed events and 1 billion trials. Values outside that work envelope return an explicit error rather than silently switching to an approximation.

## Independence and interpretation

The independent unit is the predeclared scenario × fault schedule × topology cluster. Requests, retries, windows, fault rows inside one cluster, candidate/baseline arms and repeated agent turns do not increase the independent cluster count.

A paired bootstrap interval does not automatically imply a causal effect. The evaluator must separately justify:
- randomized or otherwise defensible Candidate/Baseline assignment;
- no unmodeled interference between paired executions;
- identical environment and resource-budget realization;
- representative scenario-cluster sampling;
- source-authenticated, complete outcome records.

A zero-event confidence bound depends on independent, representative clusters. Repeating 500 requests inside one cluster does not create 500 independent safety observations.

## Validation suite

The Rust tests cover:
- exact lower/upper outcome bounds with indeterminate outcomes;
- deterministic bootstrap output invariant to input order;
- Candidate/Baseline digest mismatch and unequal denominators;
- duplicated/missing clusters and malformed cluster identity;
- invalid bootstrap, seed, confidence, effect and indeterminate configuration;
- exact zero-event bounds and sample-size calculation;
- a one-event Clopper-Pearson positive control;
- rare-event edge cases and a 48-vector adversarial mutation matrix.

These tests validate calculations over synthetic records. They are not empirical agent results or production causal evidence.

## Claim ceiling and integration

The module assumes its input rows have already been source-authenticated and bound to the frozen run, scenario receipts, participant effects and independent observer records. It does not authenticate source signatures, validate entropy, enforce sandbox policy, prove random assignment, or prove representativeness. It should consume the validated Candidate/Baseline records from the sealed/runtime layers and publish its result under the same manifest/evidence identity.

Architecture ownership remains unchanged: Platform Saga coordinates the episode; broker enforces authority; participant-owned ledgers own remote-effect truth; independent workload/service observers own business outcomes; Nixward remains the sole host transaction/interlock/journal/recovery authority.
