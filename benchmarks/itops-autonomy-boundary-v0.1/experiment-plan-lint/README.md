# ITOPS preregistered campaign plan

This directory contains a versioned experiment-plan schema and example for
ITOPS Autonomy Boundary v0.1. The plan is intended to be frozen before any
agent campaign is executed.

## Files

- experiment-plan.schema.json: Draft 2020-12 contract for estimand, strata,
  sampling, reset, dependence, experiment arms, paired comparison, stopping,
  inference and reporting.
- experiment-plan.example.json: a proposed 12,500-episode-per-arm negative-control
  campaign (37,500 total arm executions) with three declared strata and
  three paired experiment arms.
- experiment-plan-lint/: a Rust schema and semantic auditor. It validates
  Draft 2020-12 schema/meta-schema constraints and checks cross-field
  invariants that JSON Schema alone cannot express.

## Run

From the linter crate directory:

    cargo test
    cargo run --release

The linter checks that stratum minimum sample counts per arm sum to the target sample count per arm,
population weights sum to one, each arm role occurs exactly once, the
ungated arm is synthetic-only, the inference unit matches the dependence
unit, the stratified zero-event confidence-bound method matches the declared unit,
agent-state reset and cross-episode memory policies agree, and paired blocks
include scenario-variant and stratum identity.

## Interpretation

The example plans 12,500 episode blocks per arm (37,500 total arm executions across three arms), allocated 6,250 / 3,125 / 3,125 across three strata. The safety interval is computed and reported separately for each pinned system arm. It is a preregistration template, not an executed campaign.

For an episode-independent fixed-stratum design with H strata, the example uses a one-sided exact Clopper–Pearson bound in each stratum with Bonferroni alpha 0.05/H. The weighted sum of those simultaneous upper bounds is a conservative bound for the preregistered population mixture. If every stratum has zero violations, this plan's illustrative bound is approximately 0.09821% per arm. This is a design calculation, not an observation or a universal production-risk guarantee.

The familiar 0.0998% figure from 3,000 trials applies to a single i.i.d. Bernoulli cohort, not to this fixed-quota stratified sample. A new episode ID does not prove independence: reset, randomization, memory, environment and failure-domain assumptions must support the declared unit. Shared state or failure domains must be represented as clusters and analyzed with a cluster-conservative method.

The sample weights, task strata, arm profiles, seed-manifest digests and
analysis-code digest are illustrative placeholders and must be replaced and
frozen before execution. The example must not be used as a benchmark result.
