# Continuous OpsBench v0.1

Research-only deterministic evaluator for issue #188. This is the continuous-operation companion to episodic OpsBench, not a replacement for it.

## What is evaluated

The evaluator consumes separate planes:
- evaluator-owned fault truth and active/revert intervals;
- agent-produced incident reports and proposed/executed intervention references;
- participant-owned attempt/effect receipts;
- independent service observations bound to a frozen offered-demand workload profile;
- a no-action twin using the same scenario schedule and workload identity.

It derives per-fault correlation, exact service/artifact localization, repair before evaluator cleanup, sustained workload-conditioned recovery, false repair claims, intervention safety, duplicate committed effects, unauthorized committed effects, and demand-weighted degraded request-time.

Every injected fault remains in the denominator, including faults that disappear before an agent invocation. No-action observations are compared only when bound to the same scenario and workload profile. Metrics remain separate; there is no compensatory composite score.

## Run

```sh
cargo test --locked
```

No external crates or live infrastructure are used.

## Claim ceiling

This is a typed offline evaluator over supplied records. It does not generate Kubernetes faults, run an agent, authenticate publishers, verify cryptographic signatures, prove the independence of declared observer sources, or establish production representativeness. A committed participant receipt is trusted as participant-owned input. A production adapter must bind exact episode/run/arm and evidence digests, use a source-authenticated participant ledger and observer, and integrate through Platform Saga and the existing effect broker. Nixward remains the sole host transaction and recovery authority.

The synthetic fixture is not a benchmark result and cannot support SOTA or production-safety claims.


## Temporal topology and causal-model freshness v0.1

The additive `src/temporal.rs` evaluator separates evaluator-owned topology truth from agent-visible topology observations. It binds causal hypotheses to exact topology epochs/digests and reports stale-model action attempts, stale committed effects, stale or missing observations, unknown truth, sequence gaps, and unknown edges. Event time, ingest time, processing time, and valid time remain separate. See `TEMPORAL_TOPOLOGY_V0_1.md`, `schemas/temporal-topology-bundle.schema.json`, and the transition fixture.

This is a structural synthetic evaluator. It does not authenticate topology sources or enforce action preconditions at the participant boundary. A production adapter must bind the epoch into a conditional participant operation to address time-of-check/time-of-use races.


## Closed-loop operational stability v0.1

The additive \`src/stability.rs\` module provides a deterministic discrete-time synthetic plant and evaluator for issue #237. It includes delayed actuation, delayed/noisy observations with explicit source ticks, critical/bulk workload priority, retry amplification, API/database capacity mismatch, action churn/reversals, request-time SLO violation integral, queue/latency maxima, post-fault settling, and a reserved no-intervention recovery tail. Settling time is the first post-fault tick after which every remaining sample stays healthy for the rest of the run; a temporary healthy streak followed by a regression does not count as settlement.

Controller actions are supplied as a trace; this module does not implement an agent, scheduler, autoscaler or policy gate. The evaluator reports critical and bulk failures separately and preserves the plant trace separately from delayed observations.

**Claim ceiling:** this is a small synthetic plant for deterministic scenario construction and metric validation. It does not establish production representativeness, formal/Lyapunov stability, or convergence for arbitrary systems. Stability thresholds must be calibrated and preregistered against a pinned plant/workload before candidate-agent evaluation. The initial synthetic tests are conformance tests, not agent benchmark results.


## Competing controller actions

The stability reference applies an evaluator-owned precedence rule to simultaneous mutations of the same resource in the same effect tick: Operator > Runbook > Agent > Autoscaler. Equal-precedence proposals are resolved by stable action ID order. Every proposal gets an explicit `ActionOutcome` (Applied or Overridden with the winning action ID); conflict and override counts are reported separately from action churn. The resolver is input-order independent, so shuffling an otherwise identical action log cannot change the simulated plant. This is a benchmark arbitration fixture, not a recommended universal production policy.


The workload model reports critical and bulk failure rates, latency envelopes, and request-time SLO violation integrals independently. Aggregate SLO compliance and persistent recovery require **both** workload classes to pass their declared failure-rate and latency limits; bulk or critical degradation cannot be hidden by a healthy aggregate. The aggregate request-time violation integral is the sum of the two class-specific integrals.


## Observation-driven closed-loop controller runner

The additive `run_stability_closed_loop` function calls a `StabilityController` once per actionable tick. Its only input is the current delayed/noisy `PlantObservation`; evaluator fault truth and actual plant state are not passed through the policy interface. The runner records the observation source tick, measurement values, proposed action IDs, issue time and applied/overridden outcomes. It stops invoking the controller early enough that every proposed actuation lands before the reserved no-intervention tail.

The default controller runner is incremental **O(T + A log A)**, where T is the number of plant ticks and A is the number of proposed actions. It advances the plant once per tick and feeds only the observation delivered at that tick to each policy. A separate **O(T²)** prefix-replay runner remains as an independent orchestration reference. Before returning, the streaming path replays the complete action trace through the offline scorer and requires exact equality of plant samples, delayed/noisy observations and action outcomes. A divergence is a hard error, not a score.

Discrete-time semantics are causal: a controller receives its observation at tick t, proposes an action, and the earliest possible effect is the next plant transition. Thus zero additional actuation delay means an effect at t+1; it never changes the sample that generated the decision. The no-intervention tail gate uses the same due-tick function, so no delayed action can enter the tail.

Each ControllerDecision preserves the complete observation and every proposed ControlAction, in addition to its decision tick, source tick and action IDs. validate_closed_loop_evidence checks that the observation exactly matches the independent scorer's observation at that decision tick, all actions are bound to that controller/tick/source observation, the decision action list is complete, and concatenating those proposals exactly reproduces the scored action trace. It replays the scorer and rejects tampered observations, substituted operations, orphan actions and missing action rows.

The controller trait is an in-process policy seam, not a live LLM integration. Model/provider, prompt, tool, memory, policy and action budgets still need to be supplied by the external runner manifest and hashed into each arm.


The controller harness also accepts multiple policies in one run. Each declares a stable unique controller ID and controller class; policies are sorted by ID before each tick and receive the same permitted observation. Their actions are then arbitrated by the evaluator-owned precedence rule. The resulting action and decision traces are invariant to the order in which the caller supplies the controller slice. This supports paired competition between an agent policy, deterministic runbook, and autoscaler baseline without giving any policy access to evaluator truth.


## Deterministic autoscaling baselines

The reference plant now includes three deterministic policies:

- `BangBangCapacityController`: one threshold switches between normal and elevated capacity.
- `HysteresisCapacityController`: separate scale-up and scale-down thresholds create a deadband.
- `PersistentHysteresisCapacityController`: queue-aware debounce; overload must persist for N delivered observations before scale-up, while both failure and queue signals must remain healthy for M observations before scale-down. An intermediate observation breaks the streak.

All policies consume the same `PlantObservation` stream and share the same action budget, causal actuation delay, and deterministic arbitration. The paired noise fixture makes bang-bang repeatedly reverse its setpoint, while two-threshold hysteresis scales once and holds. The persistence-filtered controller additionally refuses to act on alternating single-sample spikes. Tests assert that all these differences are evaluated against the same plant truth and SLO contract; these are synthetic policy comparisons, not production autoscaling results.


## Predictive operational safety filter v0.1

The additive \`src/safety_filter.rs\` module implements a pure one-step appraisal over explicitly supplied finite state-belief, capacity-uncertainty and demand-disturbance sets. It checks critical and bulk queue bounds, aggregate queue bounds, API/database capacity limits, retry amplification and a minimum spare-capacity floor. Retry-amplified demand is rounded upward; arithmetic overflow, stale/future observations, missing or duplicate set members, and enumeration above the fixed cap fail closed.

The filter returns \`AdmitForAuthorization\`, \`RejectUnsafe\`, or \`DeferForEvidence\`. Its result binds the exact candidate-action digest, tenant, participant, target, topology epoch/digest, observation, model, uncertainty set, disturbance set and envelope. It does not authenticate publishers or digests, issue a grant, dispatch an action, write a ledger, or mutate the plant. An admission is only permission to continue to the existing effect gate; participant topology CAS remains the final race-safe commit check.

The v0.1 horizon is **exactly one transition**. The supplied bounded state/model/disturbance set is exhaustively enumerated. An independently written test oracle checks a small action matrix without calling the filter's transition helper. This is not yet a multi-step viability-kernel implementation and does not solve degraded-state liveness. The model is a narrow two-workload reference and is not yet an adapter over PR #243's plant implementation.

Wire contracts and fixtures:
- \`predictive-safety-filter-request.schema.json\`
- \`predictive-safety-filter-appraisal.schema.json\`
- \`examples/predictive-safety-filter-request.example.json\`
- \`examples/predictive-safety-filter-appraisal.example.json\`

CI meta-validates both Draft 2020-12 schemas and validates both example fixtures before running the Rust suite. The schemas define a hex digest wire encoding; a real adapter must perform canonical serialization and recompute/authenticate digests rather than accepting caller-supplied strings as proof.


## Operational model-assumption monitor v0.1

The additive `src/model_assumption.rs` module evaluates immutable pre-issued forecast intervals against independently sourced observations. It binds every forecast and observation to the exact model revision/digest, plant revision/digest, workload profile and topology epoch/digest. It rejects cross-domain observer aliasing, duplicate IDs/evidence digests, sequence gaps/regressions, missing outcomes, stale or time-misaligned observations, and incomplete records.

The monitor enforces three separate checks: interval coverage (a miss quarantines the model scope), interval sharpness (wide intervals cannot qualify), and per-metric positive/negative CUSUM on fixed-point point-forecast residuals (warning or hard quarantine thresholds). Every metric must meet a preregistered minimum observation count. All thresholds are frozen inputs; the monitor does not recalibrate on confirmatory outcomes.

Dispositions are `Unqualified`, `Qualified`, `DriftSuspected`, `Quarantined` and `Indeterminate`. Only `Qualified` permits a caller to continue to the separately implemented predictive safety filter. This module does not authorize or execute any action and does not claim a distribution-free future coverage guarantee. It is not an implementation of conformal prediction.

Wire contracts/fixtures:
- `model-assumption-monitor-input.schema.json`
- `model-assumption-monitor-report.schema.json`
- `examples/model-assumption-monitor-input.example.json`
- `examples/model-assumption-monitor-report.example.json`

CI meta-validates both Draft 2020-12 schemas and validates both example fixtures. See `MODEL-ASSUMPTION-MONITOR-V0.1.md` for identity, logic and claim boundaries.


### Composed model-assurance entry point

The safety-filter module now exposes `appraise_with_model_assurance`. It requires a Qualified monitor report whose run/scenario/arm, model revision/digest, plant revision/digest, workload profile, metric-policy digest and topology epoch/digest exactly match the candidate appraisal. It also requires distinct producer/observer identities and control domains, a complete finding-free report, and a non-expired validity window within the configured maximum report age.

The resulting safety appraisal records the exact model-monitor evaluation and expiry times. An unqualified report, scope substitution, stale report, or missing freshness evidence returns DEFER_FOR_EVIDENCE. The low-level `appraise_one_step` remains available for filter-only differential tests; the composed entry point is the intended full research path.

Model-monitor reports are explicitly time-limited (maximum 60 seconds in v0.1). A new model, plant, workload, topology or metric-policy identity requires a new report. Publisher authentication remains the caller's responsibility; these typed reference records are not signed evidence by themselves.
