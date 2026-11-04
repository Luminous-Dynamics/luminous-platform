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


## Evaluator-owned causal-breakpoint analysis

The additive \`src/breakpoint.rs\` module evaluates an immutable event trace against evaluator-owned truth and a versioned constraint contract. It is designed to answer two different questions:

- **First critical violation:** when did the agent first breach a declared evidence, causal, authority, workload, or recovery constraint?
- **First irrecoverable breakpoint:** when did the trace first cross into an irreversible external effect or committed state change?

The evaluator binds decisions to the evidence IDs actually observed before the decision, rejects stale evidence and topology epochs, checks that the hypothesis matches evaluator-known causes and evidence support, and binds later mutation attempts to that decision. It separates blocked unauthorized attempts from unauthorized participant commits and detects duplicate logical effects.

A recovery claim is credited only when a participant commit for the same action/service precedes an independent, complete, workload-matching service postcondition for that service, and that postcondition precedes the claim. Missing or incomplete observer evidence makes attribution ambiguous; a complete independent observation that still shows failure supports agent attribution. Demand under-delivery is attributed to the agent only when the trace records agent-controlled shedding, to the environment when the load source identifies an external shortfall, and otherwise remains ambiguous.

The report preserves agent/environment/harness/ambiguous cause classes, exact event sequence, related IDs and evidence IDs. Harness retention regressions are only attributed when they occur on a held-out task after a passed contamination scan.

\`assess_counterfactual\` compares two independently produced reports: the original trace and a separately replayed trace with one event removed or neutralized. It reports material, non-material or inconclusive effects on the first irreversible breakpoint. It does **not** create an infrastructure simulator or synthesize the counterfactual environment itself.

This is an E0 deterministic evaluator reference, not an LLM judge, production event collector, authenticated evidence pipeline, or formal proof of causal sufficiency. Evaluator truth annotations, participant receipts, observer independence, holdout integrity and counterfactual replay validity must be established by separate adapters and audit evidence.


## Operational evidence debt v0.1

The additive `src/evidence_debt.rs` module measures how long an external
effect, recovery claim or observer obligation remains unresolved. It is a
time-integrated **evidence debt** metric, not a trust score, causal judge or
replacement for the participant attempt ledger.

For each operation (j), the evaluator receives a sequence of evaluator-owned
state transitions. The unresolved states are:

- `PendingReconciliation`: an effect or claim exists but has no authoritative
  terminal result;
- `ObserverGap`: the required independent observer is unavailable or has a
  coverage gap;
- `Contradicted`: available evidence conflicts with the current claim/state;
- `NotObserved`: the expected receipt or observation has not arrived;
- `Superseded`: a later operation replaced this one, but the original result
  has not yet been reconciled.

These states **do not close the debt**. The only terminal states are
`Confirmed` (requires evidence references) and
`ResolvedWithoutConfirmation` (requires a participant/evaluator resolution
receipt, such as a rejection or compensation record). A terminal item cannot
be reopened. Raw bearer material must never be placed in evidence references.

For run horizon (T), the evaluator reports item-time exposure

[
D(T) = \sum_j \int_0^T \mathbf{1}[j\text{ unresolved at }t] \,dt
]

and preregistered consequence-weighted exposure

[
D_w(T) = \sum_j w_j \int_0^T
\mathbf{1}[j\text{ unresolved at }t] \,dt.
]

The implementation uses exact integer milliseconds and fixed-point
`consequence_weight_milli` (1,000 = 1x), not floating point or model
confidence. Concurrent operations add to item-time. The report retains:
- total unweighted and weighted exposure;
- exposure by operation kind (participant effect, recovery claim, observer
  coverage) and unresolved state;
- unsubstantiated recovery-claim exposure separately;
- peak concurrent unresolved items and longest unresolved item;
- confirmed, resolved-without-confirmation and still-open counts;
- open-item state and last evidence references at run closeout.

The event sequence is the immutable evaluator order. Event times must be
monotonic in that order and each event must fall within the frozen run horizon.
Operation kind and consequence weight cannot change mid-lifecycle. Duplicate
event/sequence IDs, post-terminal events, oversized references, invalid
weights and out-of-horizon events fail closed. The evaluator bounds item and
event cardinality before processing.

### Interpretation and safety

- A negative terminal receipt closes uncertainty but is **not** counted as a
  successful confirmation.
- A superseded or not-observed effect continues to accrue exposure; it cannot
  disappear through relabeling.
- Recovery claims remain exposed until a qualifying independent confirmation
  or explicit resolution receipt is recorded.
- Report unweighted duration alongside weighted exposure; never let a low
  consequence count hide a critical unresolved effect in an aggregate.
- Unauthorized, duplicate or out-of-scope committed effects remain
  non-compensatory hard failures in the causal-breakpoint/safety report.
  Evidence-debt exposure is diagnostic and cannot offset those failures.

### Claim ceiling and integration

This is an E0 deterministic integration over supplied evaluator-owned lifecycle
records. It does not authenticate those records, collect production telemetry,
establish that an observer is independent, infer consequence weights, or
replace participant-ledger / broker / Nixward authority. The caller must bind
each operation and evidence reference to the canonical event/receipt IDs from
those owners. The evidence-debt score is never derived from agent self-report.

The module includes eleven conformance/adversarial test sources for exact
weighted integration, open exposure through the horizon, unresolved
supersession/observer gaps, negative resolution, concurrent items, shuffled
input order, identity/sequence/weight mutation, missing terminal evidence,
reopening, resource bounds, and event-time regression. These are evaluator
tests, not benchmark results.
