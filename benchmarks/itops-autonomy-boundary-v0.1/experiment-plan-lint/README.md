# ITOPS preregistered campaign plan

This directory contains a versioned experiment-plan schema and example for
ITOPS Autonomy Boundary v0.1. The plan is intended to be frozen before any
agent campaign is executed.

## Files

- experiment-plan.schema.json: Draft 2020-12 contract for estimand, strata,
  sampling, reset, dependence, experiment arms, paired comparison, stopping,
  inference and reporting.
- experiment-plan.example.json: a proposed 3,000-episode negative-control
  campaign with three declared strata and three experiment arms.
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
unit, the zero-event confidence-bound method matches the declared unit,
agent-state reset and cross-episode memory policies agree, and paired blocks
include scenario-variant and stratum identity.

## Interpretation

The example plans 3,000 episode blocks per arm (9,000 total arm executions across three arms); the safety interval is computed and reported separately for each pinned system arm. It is a preregistration template, not an executed campaign. The
3,000-episode target is not a claim of 3,000 independent observations. An
episode counts as independent only when the reset, randomization, memory,
environment and failure-domain assumptions support that unit. Shared state
or shared failure domains must be represented as clusters and analyzed with a
cluster-conservative method.

The sample weights, task strata, arm profiles, seed-manifest digests and
analysis-code digest are illustrative placeholders and must be replaced and
frozen before execution. The example must not be used as a benchmark result.
