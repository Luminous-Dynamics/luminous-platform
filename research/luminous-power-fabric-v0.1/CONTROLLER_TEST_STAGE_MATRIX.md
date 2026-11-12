# Power Fabric microgrid controller test-stage matrix — v0.1

**Tracking:** [#445 — staged HIL/P-HIL qualification and test-environment evidence](https://github.com/Luminous-Dynamics/luminous-platform/issues/445)  
**Controller protocol:** [#432](https://github.com/Luminous-Dynamics/luminous-platform/issues/432) · [draft PR #433](https://github.com/Luminous-Dynamics/luminous-platform/pull/433)

**Status:** research crosswalk and planned test coverage only. No HIL, power-HIL, commissioning, periodic or field test has been executed. This is not a claim of conformity to IEC TS 62898-3-5:2026 or IEEE 2030.7/2030.8.

## 1. Why this matrix exists

The controller benchmark in PR #433 compares dispatch policies across full mission trajectories. That is only one part of microgrid-system qualification. A deployable system also needs evidence for forecast input quality, demand-side integration, flexible-resource management, mode switching, protection interlocks, electrical control, islanding and black start.

IEC TS 62898-3-5:2026 is the current directly relevant IEC test-method anchor. The IEC publisher states that it covers HIL, commissioning and periodic testing for microgrid monitoring, control and energy-management systems in grid-connected and isolated microgrids. Its public contents preview identifies separate topics for MEMS dispatch optimization, forecast, demand-side integration and flexible-resource management, and MMCS anti-maloperation, voltage/frequency regulation, device switching, operating-mode transition, active/reactive power, islanding detection, black start and end-to-end testing.

This matrix maps those public topic headings into a Luminous research coverage plan. It does not reproduce normative clauses, acceptance limits or test setup requirements. The licensed IEC specification must be consulted for formal applicability and exact procedures.

## 2. Standards crosswalk

| Reference | Published role | How Power Fabric uses it |
|---|---|---|
| IEC TS 62898-3-5:2026 | HIL, commissioning and periodic testing of MMCS/MEMS | Primary controller test-stage crosswalk |
| IEC TS 62898-3-2 | Microgrid energy-management systems | Interface and function dependency to review in licensed text |
| IEC TS 62898-3-4 | Microgrid monitoring and control systems | Monitoring/control interface dependency to review in licensed text |
| IEEE 2030.7-2017 | Common microgrid-controller functional specification | Functional decomposition and operating-mode crosswalk |
| IEEE 2030.8-2018 | Microgrid-controller test procedures and comparison metrics | Controller test design and metric crosswalk |
| P2030.7 / P2030.8 | Active IEEE revision projects in this snapshot | Track separately; do not treat PAR status as a published replacement |
| IEEE 2800.2-2026 | Plant-level conformity assessment for applicable bulk-system IBR plants | Separate grid-interconnection/IBR path, not a substitute for controller HIL tests |
| P2800.1 | Active project on grid-forming IBR/CBR functional capabilities and performance | Future GFM-specific extension when product scope warrants it |

Sources: [IEC TS 62898-3-5:2026](https://webstore.iec.ch/en/publication/84175), [IEC TS 62898-3-2](https://webstore.iec.ch/en/publication/77331), [IEC TS 62898-3-4](https://webstore.iec.ch/en/publication/77333), [IEEE 2030.7-2017](https://standards.ieee.org/ieee/2030.7/5941/), [IEEE 2030.8-2018](https://standards.ieee.org/ieee/2030.8/6169/), [P2030.8](https://standards.ieee.org/ieee/2030.8/12543/), [IEEE 2800.2-2026](https://standards.ieee.org/ieee/2800.2/10616/), [P2800.1](https://standards.ieee.org/ieee/2800.1/12387/).

## 3. Functional coverage matrix

The machine-readable research matrix is [controller-test-stage-matrix-v0.1.json](./schemas/controller-test-stage-matrix-v0.1.json), governed by [controller-test-stage-matrix-v0.1.schema.json](./schemas/controller-test-stage-matrix-v0.1.schema.json).

| Test ID | System layer | Function | Primary evidence question |
|---|---|---|---|
| MEMS-01 | MEMS | Dispatch optimization | Are schedules feasible, service-preserving and reserve-aware? |
| MEMS-02 | MEMS | Forecast | Does forecast error, bias and staleness remain visible and bounded? |
| MEMS-03 | MEMS | Demand-side integration | Are demand-response obligations and customer/load constraints honoured? |
| MEMS-04 | MEMS | Flexible-resource management | Are flexible resources dispatched within qualified availability and energy limits? |
| MMCS-01 | MMCS | Anti-maloperation lock/alarm | Are forbidden commands blocked and alarmed independently of advisory policy? |
| MMCS-02 | MMCS | Islanded voltage/frequency regulation | Are voltage/frequency and recovery behaviors within the product-specific acceptance envelope? |
| MMCS-03 | MMCS | Device switching | Are switching sequences, interlocks and feedback coherent under faults? |
| MMCS-04 | MMCS | Operating-mode transition | Are grid-connected, islanded and reconnection transitions safe and recoverable? |
| MMCS-05 | MMCS | Active/reactive power control | Are P/Q requests tracked without violating current, voltage or reserve constraints? |
| MMCS-06 | MMCS | Islanding detection | Are intentional/unintentional island conditions distinguished within the defined requirements? |
| MMCS-07 | MMCS | Black start | Can the declared black-start sequence form and recover the intended island safely? |
| SYS-01 | System | End-to-end pre-commissioning | Do controller, model, protection, comms, DER and operator interfaces work as one configured system? |

Each entry specifies planned HIL/commissioning/periodic stages, admissible execution boundaries, positive and negative controls, required observables, hard gates and a claim ceiling. The exact test set must be narrowed to the actual product and microgrid boundary. A test that is not applicable should be marked not applicable with a documented rationale—not silently omitted.

## 4. Execution stages and physical boundaries

A label like "HIL passed" is not sufficient. The passport must record which components were physical and which were simulated/emulated.

| Execution boundary | What is physically present | What it can support | What it cannot establish alone |
|---|---|---|---|
| Model-only simulation | No physical controller/power plant | Model-conditioned algorithm and scenario analysis | Physical controller I/O, protection, power-stage or site behavior |
| Controller-HIL | Physical controller and measured I/O; plant/network model executes in real time | Firmware, communications, control logic, timing and closed-loop controller behavior conditional on model validity | Physical plant power dynamics, battery thermal behavior or actual site availability where those elements are simulated |
| Power-HIL | Declared physical power interface/device plus real-time emulated system | Interface/power-device behavior under declared coupling and real-time model boundary | Full installed-site and lifecycle behavior |
| Controlled physical laboratory | Declared physical DER, loads, protection and instruments | Measured behavior of the actual laboratory configuration | Other environments, seasons, configurations and long-term site operation |
| Commissioning | Exact installed configuration and site boundary | Initial installed-system function and acceptance under the commissioning procedure | Future availability or full lifecycle reliability |
| Periodic/site operation | Installed system, operational telemetry and histories over a declared interval | Degradation, availability, dispatch, maintenance and site performance over sampled regimes | Unobserved regimes or fleet-wide reliability without a suitable sampling design |

These boundaries are deliberately descriptive. Formal HIL/P-HIL definitions, interface requirements and acceptance procedures must come from the licensed standards and test-lab procedure.

## 5. Test-environment passport requirements

Every executed test should bind:
- controller, firmware, policy, configuration, communication-stack and DER revisions;
- real-time simulator, plant model, network model, timestep, solver and real-time execution statistics;
- each physical, simulated and emulated component/channel;
- physical interface topology, coupling and relevant impedance/transformer/inductor parameters;
- communication protocol, latency/jitter, packet loss, ordering, reconnect/failover and clock synchronization;
- scenario, seed, initial conditions, disturbance timing and available forecast/observation set;
- raw controller commands/acknowledgements, telemetry, waveforms and independent protection outputs;
- instrument identity, calibration scope, timebase, measurement uncertainty and data rights;
- missed deadlines, timestep overruns, dropped/reordered samples, aborted runs and exclusions;
- test procedure revision, operator, acceptance gates, analysis build, input root and output digest.

A real-time simulator deadline miss, unreported clock discontinuity, or dropped control/measurement sample is a test-integrity failure unless the procedure explicitly defines a safe, recorded non-qualifying outcome. Do not silently stretch a simulation step or drop late data and still call the controller real-time qualified.

## 6. Model validity and uncertainty

Simulation sophistication is not model validation. For each modeled subsystem, bind:
- intended operating domain, topology, bandwidth and boundary conditions;
- parameter source and calibration basis;
- comparison against independent physical observations where available;
- residuals per operating regime and metric, including tails and transient response;
- out-of-domain detection, model discrepancy and fallback;
- model version and invalidation when parameters, topology or calibration evidence changes.

A model that is not validated for a test's critical regime cannot support a physical-performance conclusion for that regime. Retain the simulation result, but lower the claim ceiling to model-conditioned evidence or mark it inconclusive.

## 7. Competitive benchmark targets

Targets below are internal research aims, not standard limits or achieved results.

**Coverage and safety:** 100% of applicable functional test rows have positive, negative and recovery controls. Zero false acceptance of a hard interlock violation, unqualified dispatch or critical service failure.

**Timing and determinism:** zero unreported real-time overruns or clock discontinuities in qualifying runs. Report controller decision latency and simulator real-time factor distributions, not only mean values.

**Model assurance:** disclose regime-specific model discrepancy and out-of-domain rejection. Do not accept a single global model-fit score as a substitute for regime coverage.

**Interoperability:** test every supported protocol/interface combination against exact message ordering, units, time semantics, quality flags, timeout, reconnect and stale-data behavior. Do not claim vendor-neutral interoperability from one successful Modbus or IEC 61850 connection.

**Lifecycle:** periodic testing must detect configuration drift, calibration expiry, changed device firmware, degradation and maintenance/requalification conditions. A commissioning pass must not remain valid indefinitely after relevant configuration/evidence changes.

## 8. Industry benchmark positioning

The public NLR literature provides concrete CHIL/PHIL comparators: controller-HIL evaluation of a microgrid controller with multiple grid-forming inverters (2024), and PHIL tests of a grid-forming battery inverter in the Borrego Springs microgrid context (2025). A 2026 open-access study reports a forecast–MILP–real-time hybrid EMS validated through PHIL on a 300 kVA platform.

These are different test environments and claims. Power Fabric should compare only matched functions, plant boundaries, model fidelity, physical hardware and observables. It is not valid to rank an offline synthetic dispatch fixture against a 300 kVA PHIL experiment.

Further reading:
- [NLR CHIL multi-GFM microgrid controller evaluation](https://research-hub.nlr.gov/en/publications/controller-hardware-in-the-loop-evaluation-of-a-microgrid-control-2/).
- [NLR PHIL grid-forming battery inverter experiments](https://research-hub.nlr.gov/en/publications/power-hardware-in-the-loop-experiments-of-a-microgrid-with-a-grid-2/).
- [2026 hybrid EMS PHIL study](https://www.sciencedirect.com/science/article/pii/S2666790825002484).
- [2026 GFM testing and validation review](https://www.mdpi.com/1996-1073/19/20/4966).

## 9. Claim ceiling

The matrix is a research crosswalk and test plan. It does not implement controller execution, simulate a power plant, certify hardware, authorize energization, establish HIL/P-HIL performance, or demonstrate IEC/IEEE conformity. The exact licensed-standard clauses, product boundary and local conformity route require qualified review before any acceptance test or deployment.
