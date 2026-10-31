# OpsBench paired scenario-cluster power and MDE planner v0.1

**Status:** screening-only planning tool. The checked-in pilot is synthetic and example-only. This is not a confirmatory result, SOTA claim, population guarantee or safety certification.

## Why this is separate from the evaluator

A target of 500 scenario clusters is a floor, not a power calculation. The required sample depends on the paired scenario-level variance, the effect worth detecting, desired power and the number of candidate comparisons. Repeated seeds and variants within one scenario do not create new independent incident families.

This tool consumes only a disjoint preregistered screening pilot. It rejects confirmatory inputs and requires the pilot scenario IDs to exactly match the preregistered screening split. It never reads or selects confirmatory scenario identities.

## Method

For each matched scenario cluster, the pilot provides one paired candidate-minus-baseline difference in strict independently verified service success. The planner computes the sample mean, sample standard deviation and p10/p25/p50/p75/p90 quantiles across these independent scenario clusters.

The normal-approximation planning count is:

\[
n = \left\lceil \left(\frac{(z_{1-\alpha/(2m)} + z_{1-\beta})s}{\delta}\right)^2 \right\rceil
\]

where:
- \(s\) is the screening pilot's sample standard deviation of paired scenario-cluster effects;
- \(\delta\) is the target minimum detectable effect;
- \(1-\beta\) is target power;
- \(\alpha\) is the family-wise two-sided type-I error rate;
- \(m\) is the preregistered number of candidate comparisons, using Bonferroni adjustment.

If pilot variance is zero, the planner does not infer certainty. It applies a conservative 5 percentage-point SD floor, flags the degeneracy, and calculates a finite sample size. This floor is a planning convention and must be challenged in sensitivity analysis.

A separate empirical centered-residual resampling simulation estimates power at the recommended count. It preserves the observed screening-cluster residual shape; when pilot variance is zero, it uses the disclosed symmetric variance-floor residual. It is explicitly a planning diagnostic, not the confirmatory analysis. The primary confirmatory comparison remains the paired scenario-cluster bootstrap in the assurance analyzer.

The recommendation is the maximum of:
1. the preregistered minimum confirmatory scenario target;
2. the power-derived count after the preregistered safety margin for unmatched/incomplete clusters.

The sensitivity grid varies target MDE (5/10/15 percentage points), power (80/90/95%), and comparison-family size (1/2/4).

## Missing and unmatched pilot data

Every screening scenario must be either:
- represented by one paired scenario-cluster effect; or
- explicitly named as unmatched on the candidate or baseline side.

Unmatched clusters are counted and reported. They are not silently dropped and never converted to zero effects. Only paired clusters contribute to the paired mean and variance. A larger unmatched fraction should trigger investigation of runner/data quality before using the planner's recommendation.

## Run

From the repository root:

    python3 tools/opsbench-power-mde-v0.1/plan_power.py \
      --pilot benchmarks/opsbench-power-mde-v0.1/pilot.example.json \
      --preregistration benchmarks/opsbench-common-core-v0.1/preregistration.example.json \
      --output /tmp/opsbench-power-plan.json

Run tests:

    python3 -m unittest discover \
      -s tools/opsbench-power-mde-v0.1 \
      -p 'test_plan_power.py' -v

## Interpretation limits

- Five screening scenarios are far too few to estimate a stable variance; the fixture exists to test the tool, not to recommend an actual sample size.
- Pilot representativeness is an assumption, not something the planner can verify.
- The normal approximation may be poor for bounded, skewed, heavy-tailed or multimodal scenario effects.
- Residual-resampling simulation is a second planning lens, not a substitute for the preregistered cluster bootstrap.
- Bonferroni is conservative; any alternative multiplicity procedure must be declared before confirmatory trials.
- A larger sample does not repair a biased oracle, a contaminated holdout, a non-independent safety assessor or unfair arm permissions.
- This is not an assurance guarantee. Safety remains a separate non-compensatory gate.
