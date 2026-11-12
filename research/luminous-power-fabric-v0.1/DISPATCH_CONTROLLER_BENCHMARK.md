# Power Fabric dispatch-controller benchmark — v0.1

**Parent:** [Issue #432 — matched mission dispatch-controller benchmark and adversarial scenario corpus](https://github.com/Luminous-Dynamics/luminous-platform/issues/432)  
**Builds on:** [mission-energy PR #423](https://github.com/Luminous-Dynamics/luminous-platform/pull/423), [power-duration PR #426](https://github.com/Luminous-Dynamics/luminous-platform/pull/426), [benchmark passport PR #430](https://github.com/Luminous-Dynamics/luminous-platform/pull/430), [lifecycle service PR #428](https://github.com/Luminous-Dynamics/luminous-platform/pull/428)

**Status:** protocol and synthetic corpus only. No controller implementation, physical measurement, HIL result, field result, standard conformity, or competitive superiority is claimed.

## 1. The missing benchmark layer

The existing Power Fabric stack evaluates whether a source/storage configuration can supply a mission's energy and event-power obligations, whether two runs are structurally comparable, and how repeated service histories affect lifecycle cost.

It does not yet qualify the **policy that decides when and how to dispatch** those resources.

That is a separate question. Two systems with the same generator, battery, converter and load can produce very different customer outcomes because one controller:
- consumes reserve too early;
- starts a generator too late or too often;
- trusts a biased forecast;
- ignores auxiliary/cooling loads;
- hunts between sources or operating modes;
- fails to preserve a terminal reserve;
- cannot recover after a disturbance;
- dispatches from stale or contradictory telemetry.

The unit of controller evaluation is therefore a **complete mission trajectory**, including disturbances and recovery, not one operating point or a scalar efficiency result.

## 2. Standards and external method anchors

Use the standards as scoped references, not as a claim of conformity:

- [IEEE 2030.7-2017](https://standards.ieee.org/ieee/2030.7/5941/) specifies common microgrid-controller functions and operating modes.
- [IEEE 2030.8-2018](https://standards.ieee.org/ieee/2030.8/6169/) provides procedures and metrics to test and compare microgrid-controller functions.
- Revision projects [P2030.7](https://standards.ieee.org/ieee/2030.7/12022/) and [P2030.8](https://standards.ieee.org/ieee/2030.8/12543/) are active. At the 12 November 2026 research snapshot, their project status must be tracked separately from the published 2017/2018 editions.
- [IEC TS 62933-2-2:2022](https://webstore.iec.ch/en/publication/64570) supplies application-specific EES duty-cycle and performance-test context.
- [IEC TS 62933-2-3:2025](https://webstore.iec.ch/en/publication/77334) supplies post-commissioning site-operation assessment context.


- [IEC TS 62898-3-5:2026](https://webstore.iec.ch/en/publication/84175) is the direct, newly published microgrid MMCS/MEMS test anchor for hardware-in-the-loop, commissioning and periodic testing of grid-connected and isolated microgrid monitoring, control and energy-management systems. It was published 11 June 2026. Use the licensed text to map exact HIL setup, test conditions and acceptance requirements; a reference to the specification is not a conformity claim.

The intended relationship is complementary: IEEE 2030.7-2017 specifies common controller functions; IEEE 2030.8-2018 provides controller test procedures and comparison metrics; IEC TS 62898-3-5:2026 specifically covers HIL, commissioning and periodic tests for MMCS/MEMS. Revision projects P2030.7 and P2030.8 remain active PARs, not published replacements.
The controller protocol does not replace electrical waveform, power-quality, protection, generator, battery-safety, grid-interconnection or product-conformity testing. The power-duration and evidence-passport workstreams remain separate evidence obligations.

## 3. Benchmark contract

### Inputs consumed, not re-modelled

The controller benchmark consumes versioned, evidence-qualified inputs from their owners:

| Input | Required content | Owner boundary |
|---|---|---|
| Mission and load | Time series, criticality, service windows, deadlines and allowed shortfall | Platform mission contract |
| Source capability | Available power by duration, ramp, start delay, fuel/quality and outage state | Symtropy / source specialist |
| Storage capability | Usable energy and power at the observed state, SOC/SOH/temperature limits, charge limits and reserve | Battery owner / Symtropy model; Mycelix evidence |
| Conversion and common path | Directional limits, derating, protection readiness and qualified event envelope | Power-electronics owner / Platform composition |
| Auxiliary demand | Controls, cooling, pumps, fans, standby and separately metered POC boundary | Product configuration and measured evidence |
| Environment and site | Ambient/thermal profile, elevation, site constraints and dependency context | Sol-Atlas |
| Forecasts | Observed forecasts, issue timestamps, horizon, model revision and error distribution | Forecast producer; independent evaluation |
| Evidence | Configuration, instrument, calibration, timebase, raw samples and verification receipts | Mycelix |

Do not create a second generator model, battery ageing law, inverter/waveform simulator, protection function, metrology registry or trust ledger in Platform. Unknown or expired capability is not silently replaced with a nominal rating.

### Matched policy arms

A first evaluation should compare distinctly identified policy arms:

1. Fixed schedule / static-threshold dispatch.
2. Transparent deterministic rule-based dispatch.
3. Greedy fuel/cost dispatch with explicit feasibility checks.
4. Constrained optimisation reference (MILP/CP-SAT where tractable).
5. Receding-horizon model predictive control (MPC).
6. Uncertainty-aware or robust MPC.
7. Future Symthaea advisory policy, only after its interface, information budget and abstention semantics are frozen.

The optimisation reference is a bound, not automatically a deployable policy. Record its formulation, solver version, optimality gap and timeout. A clairvoyant solution with future knowledge must be labelled as such and must not be compared as though it had the same information as an operational controller.

For operational policy arms, match:
- observed inputs and forecast issue times;
- update cadence and decision deadline;
- compute/runtime budget;
- action set and actuator/capability constraints;
- initial state and final reserve/service obligations;
- mission, site, plant model, auxiliary boundary and scenario seed.

Policy code and parameters are expected to differ. The conditions under which the policies operate are not.

## 4. Scenario programme

The initial synthetic corpus is at [schemas/dispatch-controller-scenarios.synthetic.json](./schemas/dispatch-controller-scenarios.synthetic.json), governed by [schemas/dispatch-controller-scenario-corpus-v0.1.schema.json](./schemas/dispatch-controller-scenario-corpus-v0.1.schema.json).

Required scenario families:

| Family | Disturbance | Question |
|---|---|---|
| Baseline | Stable load and source conditions | Does each policy satisfy the nominal mission and terminal reserve? |
| Forecast uncertainty | Bias, delayed forecast, ramp or out-of-distribution demand | Does the policy preserve service and reserve without future-information leakage? |
| Source contingency | Grid/source loss, generator delay or failed start, fuel constraint | Does dispatch transition safely and account for service loss/recovery? |
| Storage contingency | Temperature or SOH derating, SOC estimation error, power/energy reduction | Does the policy respect the qualified state-dependent envelope? |
| Auxiliary/common cause | Cooling degradation, fan/pump increase and shared control/cooling dependency | Does it account for parasitic demand and correlated failure? |
| Telemetry integrity | Stale/missing/biased sensor, timebase drift, comms partition | Does it distinguish unknown from fresh/healthy and enter bounded fallback? |
| Recovery | Source restoration, recharge, thermal recovery, reserve restoration | Does it avoid declaring recovery before the terminal obligation is met? |
| Pairing integrity | Mismatched initial state, reserve, auxiliary boundary or information budget | Does the comparator reject the pair before scoring? |

Each disturbance records its deterministic time, duration, domain, severity, parameter digest and optional common-cause group. Each case records the seed, policy pair, matched-condition digests, expected gate and invariants. The synthetic corpus is a contract/fixture, not a claim that the disturbances reproduce real plant dynamics.

A future modelled or hardware-backed corpus must preserve the same scenario identity while changing the evidence class and binding the exact model/hardware configuration. A scenario does not become physical evidence merely because its schema contains physical-looking fields.

## 5. Non-compensatory scoreboards

Never collapse these dimensions into one weighted score.

### A. Customer service
- critical interval pass/fail;
- unserved critical energy and duration;
- useful non-critical work and deferral/deadline outcome;
- mission completion and workload lost;
- service delivered at the same customer POC.

### B. Feasibility and safety
- source, storage, converter and common-path constraint violations;
- minimum reserve and end-of-mission reserve;
- protection-ready / evidence-current state;
- dispatch during unknown, stale or contradictory telemetry;
- action outside the qualified capability envelope.

Any critical-service miss, unsafe dispatch, reserve breach or use of an unqualified capability is a hard failure. No fuel or efficiency saving compensates for it.

### C. Resilience and recovery
- disturbance detection and safe-state latency;
- source-transfer and restoration time;
- critical service lost during outage/recovery;
- time to restore storage and terminal reserve;
- repeated-event performance and recovery-tail energy;
- repair/requalification downtime where part of the episode.

### D. Resource and lifecycle
- primary energy and fuel per delivered critical service;
- customer-POC energy and auxiliary energy, with each flow counted once;
- conversion/storage losses and curtailed energy;
- generator starts, stops, low-load dwell and runtime;
- battery throughput, SOC trajectory and temperature exposure;
- lifecycle cost only where price, cost-allocation and degradation evidence are qualified.

### E. Controller quality
- decision latency and missed decision deadlines;
- solver runtime, optimality gap and infeasible/timeout rate;
- constraint slack and reserve trajectory;
- policy switching/hunting and avoidable start-stop cycling;
- forecast sensitivity and robustness;
- fallback entry, duration, exit criteria and recovery.

### F. Evidence and replay
- exact policy source/build and dependency closure;
- complete input, forecast, scenario, seed and configuration digests;
- exact event/decision trace and output digest;
- analysis build and replay receipt;
- evidence class and claim ceiling.

Fast waveform quality, THD, voltage/frequency nadir, protection let-through and isolation behavior remain separate instrumented electrical tests. A high-level dispatch result cannot establish them.

## 6. Fairness, statistics and SOTA comparison

- Preregister primary outcomes, policy arms, exclusion rules, guard bands and improvement margins before confirmatory runs.
- Use paired seeds and randomized policy-arm order.
- Hold out entire sites/assets/seasons or operating clusters. Do not split adjacent intervals from one trajectory across training and test.
- Preserve every failed, aborted, infeasible, timed-out and indeterminate episode in the report.
- Give each operational policy the same observed information and declared compute budget; disclose every exception.
- Publish scenario-level results and uncertainty, then stratified summaries. Do not let a high-volume easy stratum hide a weak critical stratum.
- For tractable optimisation cases, report a separately formulated bound, solver gap and timeout. A heuristic is not optimal merely because it completed.
- A superiority claim requires repeated matched episodes, independent evidence where relevant, and a difference exceeding combined measurement uncertainty and run-to-run variability.

The first competitive objective is not a universal percentage saving. It is to demonstrate a Pareto improvement over the transparent rule-based baseline on at least one declared mission class **without regressing any hard gate**. Publish where simpler policies win.

## 7. Proposed research targets

These are internal targets, not achieved results or standards limits:

| Target | Initial gate |
|---|---|
| Critical service | Zero missed critical intervals in the frozen positive-control suite; every designed unavoidable shortage remains visible as a failure |
| Feasibility | Zero dispatches outside supplied qualified envelopes |
| Reserve | Zero false passes against initial/final state and reserve obligations |
| Data integrity | Zero use of stale, missing or contradictory telemetry as if it were current |
| Pair matching | Zero acceptance of deliberate initial-state, auxiliary-boundary, terminal-reserve or information-budget mismatches |
| Replay | Byte-identical normalized outputs for identical inputs, policy build, dependency closure and seed, or explicit disclosure of nondeterminism |
| Baseline comparison | At least one Pareto improvement over rule-based dispatch without hard-gate regression before claiming an advantage |

No fixed percentage advantage is frozen until a real incumbent, a validated model domain and a physical measurement budget exist.

## 8. Promotion path and evidence boundaries

Keep **evidence class** separate from **test environment**:

- Synthetic fixture: schema, arithmetic and evaluator behavior only.
- Simulation: modelled plant/controller behavior within a declared model domain.
- Controller-HIL: a real controller may be measured while the plant remains modelled; the overall physical-service claim cannot exceed the modelled plant evidence.
- Power-HIL / controlled laboratory: measured hardware behavior under declared emulator, instrumentation and laboratory boundaries; it does not automatically prove site operation.
- Site operation / field mission: operational performance under the specific site, configuration, climate, dispatch and maintenance history.

Every promotion requires new evidence and a fresh eligibility assessment. A Mycelix verification receipt authenticates the declared evidence lineage; it does not by itself prove model validity, physical accuracy or safety.

The v0.1 schema currently labels every checked-in case as synthetic. There is no live controller, HIL runner or production action path in this tranche.

## 9. Ownership and claim ceiling

| Owner | Responsibility |
|---|---|
| Platform | Scenario contract, matched policy conditions, result vectors and hard-gate composition |
| Symtropy | Source/plant model execution, event replay and model validity limits |
| Mycelix | Calibration, timebase, uncertainty, content integrity, signatures, revocation and verification receipts |
| Sol-Atlas | Site, environment, topology and deployment context |
| Symthaea | Bounded advisory-policy research and high-information experiment selection; not evidence authority or a safety interlock |
| Specialist hardware owners | Qualified component, protection, thermal, storage and lifecycle limits |

Claim ceiling: **OFFLINE_CONTROLLER_BENCHMARK_PROTOCOL_ONLY_NOT_PHYSICAL_OR_CONFORMITY_CLAIM**.

This document does not authorize dispatch, connect to hardware, energize a battery or grid, participate in energy markets, certify a microgrid controller, or establish product conformity.
