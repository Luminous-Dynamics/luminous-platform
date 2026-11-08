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


## Deterministic watch-to-probe episode v0.1

The additive `src/watch_probe.rs` module provides a small scripted reference episode that composes:
- a bounded watch bound to tenant, target, purpose, source and expiry;
- an authority snapshot rechecked at event delivery and probe admission;
- source-sequenced events with duplicate, event-ID collision, gap, future-delivery, watch-start and identity handling;
- a policy that proposes only an existing catalog entry ID;
- exact-scope, read-only probe admission;
- a collector-owned observation (which may come from a different source than the watch event) and a separate impact observation;
- observation/impact receipts bound to exact probe, operation, tenant, target, purpose and time window, with observation freshness constrained to after the triggering wake; explicit unknown for a collector timeout with continuing work or a missing impact record;
- evaluator-only pre/post decision loss, scored after evidence acquisition.

The module includes eighteen deterministic unit tests for a valid positive episode, revocation during dormancy, duplicate delivery, sequence gaps, write-capable probe rejection, unknown impact, independence-domain collision, timeout with continuing work, quiet expiry/no-match behavior, and wrong-tenant events.

**Claim ceiling:** this is a one-probe, in-process scripted conformance reference. It does not consume the production CloudEvents/inbox/outbox path, verify cryptographic source authenticity, enforce live Xenia revocation, execute a real collector, establish process isolation between policy and evaluator, provide multi-probe stopping optimization, or establish production event-delivery guarantees. It does not replace Platform #309; it is the first small code seam for it. The evaluator's hidden truth is supplied to the harness and is not passed as a policy method argument, but a future process-separated runner is required to enforce a true information-flow boundary.

Run the complete crate locally:

```sh
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
```

No local or CI result is claimed by this change. Exact-head qualification is required before the code is treated as a qualified implementation slice.

## Paired attention experiment v0.1

The additive `src/paired_attention.rs` module reuses the existing deterministic stability plant and compares two policies on the **same immutable plant configuration and noise vectors**:

- **Fixed polling:** invokes the policy at a configured cadence, including quiet polls.
- **Event-triggered:** runs a lightweight observation monitor each actionable tick but invokes the policy only on a rising edge of the watched condition.
- **No-action counterfactual:** runs the same plant with no controller actions.

Both controller arms receive only the delayed/noisy `PlantObservation` and the eligible read-only probe identifier set produced by the shared `eligible_probe_ids_for_watch` filter. An out-of-set policy proposal is denied. Watch wakeups and probe selections have separate budgets. A selected read-only probe ID is only a proposal: it does not create a plant control action or authorize mutation. Both attention arms run the same no-action plant, so plant outcomes must match; remediation belongs in a later separately-authorized experiment. The evaluator compares wake records against actual plant samples only after each arm to report false wakes, missed fault episodes and event-to-wake latency.

The module includes eight paired-run tests covering invocation reduction, detection latency, no-action comparison, watch activation during a persistent fault, watch expiry during a persistent fault, authority expiry, out-of-set probe rejection, and a zero-probe-budget negative control.

**Metric boundary:** `monitor_checks` are deterministic monitor callbacks, not LLM calls. `cognitive_invocations` count scheduled policy invocations. `probe_selection_calls` and `admitted_probe_selections` count selection proposals, not actual remote collector executions. The module does not yet consume independent source/impact receipts for every paired wake; that is the next #320 integration increment. A synthetic plant pass is not a live SRE benchmark result.

**Claim ceiling:** the experiment is an offline paired controller comparison on a deterministic synthetic service model. It does not prove production event-delivery guarantees, model superiority, live Xenia authority enforcement, actual probe resource externality, independent observer authenticity or performance on SREGym/SRE-Marathon.

## Durable, ephemeral and concurrent event schedule v0.1

The additive `src/event_schedule.rs` module evaluates a frozen, source-qualified schedule using the existing `WatchSpec` and `AuthoritySnapshot` contracts. It is an offline inbox/event-schedule evaluator; it does not create or run another broker, subscription service, timer, inbox, outbox or acknowledgement path.

It models:
- source ID, source epoch, event ID, payload digest and monotonically ordered source sequence;
- incident ID, tenant, target, purpose and event type;
- event time, valid-until time, delivery time, processing time and response deadline;
- persistent versus ephemeral condition records;
- explicit source epoch restart markers;
- duplicate suppression, event-ID/sequence identity collision, out-of-order buffering and gap timeout;
- exact watch scope and authority checks at delivery and processing;
- per-event wake records with safe same-tick coalescing within one source epoch that preserves every distinct incident ID;
- reserved critical wake capacity so normal-priority wake batches cannot consume all wake slots;
- explicit stale, late, out-of-scope, duplicate, buffered, woken and budget-suppressed dispositions.

The module includes 23 deterministic tests: quiet schedule, durable wake, same-tick multi-incident coalescing, distinct wake batches across ticks and source epochs, pre-watch event rejection, ephemeral wake independent of a persistent level sample, duplicate delivery, event-ID and sequence collisions, out-of-order repair, late gap repair rejection, unresolved gap escalation, in-deadline delayed delivery with latency, missed deadline, stale ephemeral record, revocation while dormant, valid/invalid source restart, old-epoch replay deduplication, critical-budget reservation, wrong-tenant sequence consumption and future-record invisibility.

**Important limitation:** this module evaluates a supplied frozen schedule. It does not authenticate source publishers, validate the truth of the payload digest, execute #306's live CloudEvents/inbox/outbox path, implement durable cursor persistence across process restarts, or execute a collector. It does not observe plant health directly; event payload condition and incident identity are source-provided inputs, while evaluator-only plant truth remains outside the event policy. A source restart is accepted only as an explicit next-epoch marker with no unresolved cursor gap. Wake coalescing never crosses source epochs, and events that occurred before the watch start are not retroactively matched.

**Claim ceiling:** this is a deterministic event schedule semantics evaluator, not a production event delivery guarantee or benchmark result. It should be paired with #321's fixed-poll/event-threshold micro-experiment and the #306 watch contract; it does not yet perform model reasoning or actual probe/impact acquisition.
