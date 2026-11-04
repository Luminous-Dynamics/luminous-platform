# Operational Model-Assumption Monitor v0.1

Status: research reference implementation; no field-model qualification or SRE performance result is claimed.

Related work: issue #254; predictive safety filter #250 / PR #252; closed-loop plant #237 / PR #243; demand fidelity #184; active evidence acquisition #185; sustained recovery #181; temporal topology #214 / PR #241.

## 1. Purpose

A safety filter is only as sound as the state, model-parameter and disturbance sets supplied to it. A model can be correctly implemented and still be wrong for the current plant, workload, topology or operating regime.

This monitor checks whether a frozen predictive model's declared intervals continue to agree with independently observed outcomes. It produces evidence status, not mutation authority.

## 2. Scope and identity

Every qualification window is scoped to:

- run, scenario, arm and dependence cluster;
- model revision and digest;
- plant revision and digest;
- workload-profile digest;
- topology epoch and digest;
- model producer identity/control domain;
- independent observer identity/control domain and adapter digest.

Forecasts and observations must repeat the exact model, plant, workload and topology bindings. A change starts a new qualification window. Previous status must not be reused across model or plant changes.

## 3. Forecast/observation protocol

A forecast is immutable and must be issued before its target time. It binds a metric, units, source/target ticks, target timestamp, lower/point/upper predictions and the complete identity scope.

An independent observation binds one forecast ID to the actual metric value, exact target tick, observation/ingest times, clock uncertainty, source sequence, evidence digest and completeness state.

The reference requires:
- distinct model-producer and observer identities/control domains;
- unique forecast and observation IDs;
- unique evidence digests;
- contiguous source sequences in supplied authoritative order;
- increasing target ticks per metric;
- exact metric, unit, forecast and identity binding;
- bounded observer ingest age and target-time skew;
- complete observations and no missing forecast outcomes.

Any missing, stale, reordered, duplicated, cross-run or cross-epoch evidence makes the report indeterminate. Missing data are never converted into zero residuals or clean coverage.

## 4. v0.1 monitor logic

The Rust evaluator uses integer/fixed-point arithmetic and no external dependencies.

1. **Envelope coverage:** an actual value outside its pre-issued prediction interval immediately quarantines this model scope.
2. **Sharpness:** intervals wider than the frozen per-metric maximum prevent qualification. An arbitrarily wide interval cannot win by covering everything.
3. **Directional residual drift:** per-metric positive and negative one-sided CUSUM accumulators use the signed point-forecast residual divided by a pinned integer scale. A warning threshold yields DRIFT_SUSPECTED; a hard threshold yields QUARANTINED.
4. **Sufficiency:** every metric needs the preregistered minimum number of matched independent observations.
5. **Identity invalidation:** model, plant, workload, topology or observer changes require a new qualification epoch.

Disposition meanings:
- UNQUALIFIED: valid evidence is insufficient or forecast intervals fail the sharpness limit.
- QUALIFIED: the fixed screening criteria passed for this exact finite window and identity.
- DRIFT_SUSPECTED: sequential residual drift crossed the warning threshold.
- QUARANTINED: a forecast interval was violated or hard drift threshold crossed.
- INDETERMINATE: integrity, source separation, freshness, completeness or lineage checks failed.

The monitor uses fixed thresholds. It does not adapt interval bounds after seeing confirmatory outcomes. The thresholds are engineering screening rules, not an online conformal guarantee.

## 5. Research comparators

Future calibration work should compare against pinned implementations and explicit assumptions:
- Adaptive/online conformal inference for arbitrary shifts (Gibbs & Candès, JMLR 2024).
- Online conformal prediction via online optimization (Areces et al., ICML 2025).
- Time-series conformal prediction with change points (Sun & Yu, NeurIPS 2025).
- Error-quantified conformal inference for time series (Wu et al., ICLR 2025).

Do not label the v0.1 CUSUM monitor as conformal prediction. Exchangeability, dependence, covariate shift, conditional coverage and online-selection effects require separate treatment.

## 6. Claim boundary

QUALIFIED means only that the frozen rules passed for this declared finite set of independent observations and identity. It does not prove that the observer measures the true plant, that future errors are exchangeable, that the plant will remain stationary, or that the uncertainty envelope contains all future disturbances.

The safety-filter caller should continue only when the model monitor is QUALIFIED and the appraisal is fresh and bound to the same exact model/plant/workload/topology identity. This monitor cannot grant authority, dispatch a fallback, mutate the plant or bypass the effect gate and participant CAS.
