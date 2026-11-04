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

The initial implementation replays the already-issued action prefix at each tick to derive the next observation. This is **O(T²)** and intentionally prioritizes a clear deterministic information boundary for small conformance episodes. It is not the performance runner for large campaigns. The next optimization should share a stepwise plant transition kernel between replay and streaming execution, then prove byte-for-byte trace equivalence before using it for throughput comparisons.

Two controller-loop tests prove that a threshold policy reacts only after the fault observation arrives and improves the paired no-action synthetic trace, and that the controller is never invoked for the no-intervention tail.

The controller trait is an in-process policy seam, not a live LLM integration. Model/provider, prompt, tool, memory, policy and action budgets still need to be supplied by the external runner manifest and hashed into each arm.


The controller harness also accepts multiple policies in one run. Each declares a stable unique controller ID and controller class; policies are sorted by ID before each tick and receive the same permitted observation. Their actions are then arbitrated by the evaluator-owned precedence rule. The resulting action and decision traces are invariant to the order in which the caller supplies the controller slice. This supports paired competition between an agent policy, deterministic runbook, and autoscaler baseline without giving any policy access to evaluator truth.


## Noise-thrashing baseline policies

The reference plant includes two deterministic autoscaler policies:
- `BangBangCapacityController`: one threshold switches between normal and elevated capacity.
- `HysteresisCapacityController`: separate scale-up and scale-down thresholds create a deadband.

Both consume the same `PlantObservation` stream and share the same action budget/delay/arbitration. The conformance fixture alternates noisy failure-rate observations while the actual plant remains healthy: the bang-bang policy repeatedly reverses its setpoint, whereas the hysteresis policy scales once and holds. Both must produce the same zero SLO-violation integral and healthy no-intervention tail. This isolates reduced action churn from a service-quality trade-off; it is not yet a live cloud autoscaling benchmark.
