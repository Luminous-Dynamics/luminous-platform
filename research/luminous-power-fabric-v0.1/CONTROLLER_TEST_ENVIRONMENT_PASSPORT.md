# Power Fabric controller test-environment passport — v0.1

**Tracking:** [#445 — staged HIL/P-HIL qualification and test-environment evidence](https://github.com/Luminous-Dynamics/luminous-platform/issues/445)  
**Functional matrix:** [#446 — controller HIL / commissioning / periodic coverage](https://github.com/Luminous-Dynamics/luminous-platform/pull/446)  
**Sampling adequacy:** [#434 — metric-specific representativeness and cluster-aware confidence](https://github.com/Luminous-Dynamics/luminous-platform/issues/434)

**Status:** schema and synthetic planned fixture only. No physical controller, simulator, network, instrument or power interface is represented as having been tested.

## 1. Purpose

A test result labelled "HIL" is incomplete unless a reviewer can reconstruct exactly what was physical, simulated, emulated or absent. Controller-HIL, power-HIL, laboratory, commissioning and site-operation evidence support different claims.

The v0.1 passport records the test-stage and planned execution mode, per-component realization, simulator/model identity, communications, timebase, real-time execution observations, model-validation state and physical I/O interfaces. It binds the record to specific functional test-case IDs from the controller matrix.

The sample at [controller-test-environment-passport.synthetic.json](./schemas/controller-test-environment-passport.synthetic.json) is explicitly a synthetic *plan fixture*. Its "physical" components indicate what a future test intends to place in the loop; they are not observations that a test has occurred.

## 2. Physical/model boundary

Every component in the test configuration must be classified independently:

- **Physical:** actual device or controller hardware participating in the test.
- **Simulated:** represented by a software model.
- **Emulated:** represented by a dedicated interface/equipment emulator rather than the final production device.
- **Not in test:** explicitly excluded from this test boundary.

Do not describe an entire testbed as physical because its controller is physical. A controller-HIL run with a simulated battery and inverter says nothing direct about their thermal, cell, switching, protection or power-stage behavior. A power-HIL run only supports claims tied to the declared physical power interface and coupling boundary.

## 3. Minimum passport contents

| Area | Required record |
|---|---|
| Identity | Passport, scenario, test case, stage, procedure, policy and configuration IDs |
| Component boundary | Component role, physical/simulated/emulated status, configuration digest and evidence reference |
| Simulator | Platform identity, model bundle, solver, fixed step, real-time deadline and real-time factor |
| Communications | Protocol, physical transport, latency/jitter, packet loss, reordering and reconnect evidence |
| Timebase | Clock identity, synchronization method, offset/skew evidence and validity |
| Real-time integrity | Missed deadlines, overruns, dropped/out-of-order samples and run status |
| Model validity | Validation domain, independent comparison evidence, residuals, out-of-domain state and invalidation |
| Physical channels | Direction, unit, source/destination, instrument, calibration and raw-data reference |
| Data closure | Exact input/model/configuration root, command, outputs and replay receipt from Mycelix once available |

A missing timing or model-validation value is unknown—not zero. A planned test fixture uses nulls for values that cannot exist before execution.

## 4. Qualifying-run gates

A future executed run must not be eligible for a closed-loop HIL claim if:
- the real-time simulator misses declared execution deadlines without an explicit, non-qualifying disposition;
- control or measurement samples are dropped, reordered or silently interpolated;
- clocks are unsynchronized beyond the predeclared test tolerance;
- the simulator changes timestep or solver behavior without a new configuration identity;
- a component's physical/simulated/emulated status is ambiguous;
- the tested operating regime lies outside the model's validated domain and the result is still presented as a physical system claim;
- the raw I/O, model/configuration or analysis output cannot be reconciled.

The tolerances must be determined from the test protocol, product/controller requirements and licensed standard—not invented as universal values in this schema.

## 5. Model validity and uncertainty

For each model, retain independent reference observations and discrepancy by regime. Do not report only a global RMSE: a model can fit steady state while misrepresenting a transient, current limit, mode transition or thermal boundary.

Record:
- model version and exact parameters;
- validation asset/site and configuration;
- covered load, temperature, voltage, frequency, topology and transient regimes;
- residual and uncertainty per output/operating stratum;
- out-of-domain detection and behavior;
- source, calibration and evidence invalidation dependencies.

Unknown model validity prevents promotion to a physical-performance claim. It does not erase a useful model-only result.

## 6. Commissioning and periodic evidence

Commissioning records must bind the exact installed controller, DER, protection, communications and wiring configuration. Periodic tests must record what changed since commissioning and whether that change invalidates prior qualification.

Periodic/site-operation sample sufficiency is metric-specific. A minimum count of sample events is a structural evidence floor, not a universal reliability sample size. Use [issue #434](https://github.com/Luminous-Dynamics/luminous-platform/issues/434) for coverage, clustering, dependence, precision and claim-specific sample adequacy.

## 7. Schema and claim boundary

The strict schema is [controller-test-environment-passport-v0.1.schema.json](./schemas/controller-test-environment-passport-v0.1.schema.json). The synthetic fixture is [controller-test-environment-passport.synthetic.json](./schemas/controller-test-environment-passport.synthetic.json).

Claim ceiling: **SYNTHETIC_TEST_ENVIRONMENT_PASSPORT_FIXTURE_ONLY_NOT_EXECUTED_HIL_OR_PHYSICAL_CLAIM**.

This passport does not execute a controller, run a simulator, verify an instrument certificate, establish physical behavior, authorize energization, or certify standards conformity. A production evidence record must be produced by the test owner, independently verified by the responsible evidence authority and bound to Mycelix receipts when its native verification interface is agreed.
