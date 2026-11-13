# Conservative metrology uncertainty bounds — v0.1

**Tracking:** [Issue #464](https://github.com/Luminous-Dynamics/luminous-platform/issues/464)  
**Benchmark passport:** [PR #430](https://github.com/Luminous-Dynamics/luminous-platform/pull/430)  
**Lifecycle frontier:** [PR #462](https://github.com/Luminous-Dynamics/luminous-platform/pull/462)

**Status:** read-only conservative interval propagation over typed measurement and calibration inputs. This is not a GUM evaluation, accredited calibration process, conformity decision or physical performance result.

## Why this exists

PR #462 accepts uncertainty intervals from measurement/model owners, but did not derive them from the meter, calibration, timebase and integration evidence. This module fills that gap for a narrow first slice.

BIPM lists JCGM 100:2008 with Amendment 1:2026 addressing nonlinearity in measurement models, GUMsuite:2026 and JCGM GUM-5:2026. JCGM 101:2008 covers propagation of distributions using Monte Carlo. These are the relevant metrology framework anchors; this v0.1 implementation does not claim conformance to them. See https://www.bipm.org/en/committees/jc/jcgm/publications/, https://doi.org/10.59161/JCGM100-2008E and https://doi.org/10.59161/JCGM101-2008.

## Conservative bound contract

Each measurement binds:
- comparison set, measurand and exact POC/boundary;
- instrument and channel identities;
- scope and instrument/configuration digests;
- observed and evaluation time;
- calibration identity, certificate/evidence reference, status and validity;
- timebase identity and evidence reference;
- overall evidence class/status and raw evidence reference;
- explicit uncertainty components, each with kind, unit, current evidence and reference.

Each uncertainty component is an **absolute maximum error bound in the same unit as the measurement**, not a standard deviation, standard uncertainty or confidence interval. v0.1 adds all component bounds linearly (triangle-inequality conservative bound). It does not assume statistical independence and does not apply RSS or covariance cancellation. Duplicate components, stale evidence, unit mismatch, expired calibration, invalid timebase or missing references are rejected.

The measurement result is bounded as `[max(0, x - U), x + U]`, where `U` is the sum of declared absolute component bounds.

## Propagation

- Positive ratio (e.g. output energy / input energy): lower = numerator lower / denominator upper; upper = numerator upper / denominator lower. Inputs must share a comparison-set and scope digest, use compatible units, and have a strictly positive denominator lower bound. Their POCs may differ and are both retained.
- Paired difference (candidate - reference): lower = candidate lower - reference upper; upper = candidate upper - reference lower. Candidate/reference must match comparison set, scope, measurand, POC and unit. No common-mode error is cancelled.
- Decision margin: lower-is-better clears only if the entire difference interval is below negative margin; higher-is-better clears only if the entire interval is above positive margin. Otherwise the result does not clear or overlaps the decision boundary.

A numeric margin clearance is not a physical superiority claim unless both inputs are independently physically measured and current. Synthetic, assumed, modelled or vendor evidence retains its class; it cannot be upgraded by this calculation.

## GUM follow-on

A future implementation may support Type A/Type B distributions, sensitivity-coefficient propagation with validated covariance, and Monte Carlo propagation following JCGM 101. It must separately handle nonlinear models and coverage decisions. Common-mode cancellation requires explicit shared calibration covariance; different meter IDs alone do not prove independence.

## Synthetic example

The fixture includes candidate output energy at a customer POC, input energy at a source POC, and candidate/reference cost readings at the same customer boundary. It exercises energy-ratio propagation, paired-difference intervals and a margin clearance. All inputs are synthetic. The numeric margin can clear while physical superiority eligibility remains false.

Claim ceiling: `CONSERVATIVE_METROLOGY_INTERVAL_SCREEN_ONLY_NOT_GUM_EVALUATION_OR_CONFORMITY_DECISION`.
