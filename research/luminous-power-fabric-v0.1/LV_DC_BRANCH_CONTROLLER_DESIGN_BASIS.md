# 48 V DC branch power controller — design basis and architecture study

**Snapshot:** 2026-11-12  
**Status:** research / design-basis only; no schematic, PCB, prototype, hardware rating or qualification is claimed  
**Parent component research:** [#415 — 48 V DC solid-state branch power controller and hybrid protection](https://github.com/Luminous-Dynamics/luminous-platform/issues/415)  
**Field system:** [#413 — portable industrial field-power router](https://github.com/Luminous-Dynamics/luminous-platform/issues/413)  
**Parent platform:** [#364 — Luminous Power Fabric](https://github.com/Luminous-Dynamics/luminous-platform/issues/364)

## 1. Product concept

A modular **48 V DC branch power controller** for bounded auxiliary and service loads in portable industrial equipment:

- Instrumentation and control panels.
- Communications and edge-compute nodes.
- DC lighting.
- Low-power actuators and accessories.
- Other explicitly characterized DC loads.

This is not a general-purpose DC circuit breaker, battery-management system, motor drive, or certified isolator. It must not be placed in a high-energy branch until the complete source, cable, connector, load, fault and protection envelope is specified.

The key functional distinction:

> Semiconductor switching can control and interrupt a branch quickly; a commanded OFF state does not prove galvanic isolation or safe maintenance state.

The system therefore needs a separately justified interruption/isolation chain, appropriate upstream protection and a deterministic recovery rule.

## 2. Existing industry baseline

### Texas Instruments TIDA-050090

TI's official page specifies a 54 V / 5 kW input power-path protection reference design using one TPS1689 and five TPS1685 eFuses in parallel. TI reports 100 A RMS at 55 °C ambient without external airflow, with PMBus monitoring/configuration, EEPROM fault data and over-/under-voltage, overcurrent, short-circuit and overtemperature protection. The reference board is described as developed for testing and performance validation, not for sale.

Source: https://www.ti.com/tool/TIDA-050090

This is a useful same-voltage-class, high-current power-path reference. It is not an independent test, not a field-qualified breaker, not proof that a different PCB/layout will deliver the same thermal performance, and not a complete construction-site isolation or protection scheme.

### Candidate device families

| Candidate | Vendor-published envelope / features | Research implication |
|---|---|---|
| TI TPS1685 / TPS1689 | 9–80 V, 20 A stackable eFuse family, 3.5 mΩ stated RON; TPS1689 includes PMBus telemetry | Strong first evaluation comparator for a 48 V DC branch; compare exact datasheet conditions and reference-design thermal implementation |
| MPS MP5048 | 24–60 V input, 15 A continuous, 7 mΩ integrated MOSFET, current/temperature sensing and short-circuit / thermal protection; identifies gate/source/drain short diagnostics | Useful alternative architecture and fail-short diagnostic comparator |
| External-FET hot-swap / eFuse controllers | Controller + separately selected FET bank, shunt/sensor and protection network | Better customization potential, with larger SOA, layout, gate-drive, thermal and validation burden |
| Fuse + DC-rated mechanical disconnect | Conventional passive / mechanical baseline | Must remain a comparator; may be simpler and more serviceable for some branch classes |
| Hybrid semiconductor + mechanical interruption | Semiconductor path for fast current limitation/interruption followed by mechanical isolation | Candidate for branches where fault response and safe service isolation both justify added complexity |

Sources:
- TI eFuse family and reference-design listings: https://www.ti.com/en-us/product-category/power-management/power-protection-switches-controllers/efuses
- MPS MP5048 official page: https://www.monolithicpower.com/en/products/power-management/power-protection/mp5048.html
- TI TIDA-050090: https://www.ti.com/tool/TIDA-050090

## 3. Nominal power and conduction-loss arithmetic

The following are dimensional checks, not design ratings or thermal predictions.

At 48 V nominal:
- 5 A corresponds to 240 W.
- 10 A corresponds to 480 W.
- 20 A corresponds to 960 W.
- 100 A corresponds to 4.8 kW.

At 54 V nominal:
- 20 A corresponds to 1.08 kW.
- 100 A corresponds to 5.4 kW.

The idealized resistive loss equation is (P_{cond}=I^2R). Using only vendor-stated nominal ON resistance as an arithmetic illustration:
- 20 A through 3.5 mΩ gives 1.4 W.
- 15 A through 7 mΩ gives 1.575 W.

These calculations are not package thermal performance. Actual device resistance varies with junction temperature, gate voltage, current and operating condition; board copper, vias, connectors, busbars, parallel current sharing, switching, transients and cooling add loss. Use complete datasheet curves and measured hardware before any rating or derating claim.

The first product study should prioritize individually protected branches (for example, 5 A / 10 A / 20 A candidate classes) and a separate trunk path. These are comparison bins only; actual ratings must come from the load mission profile and fault study. Do not casually parallel low-current eFuse channels or extrapolate from TI's specific 5 kW board.

## 4. Functional architecture to compare

### Option A — integrated eFuse branch

- Commercial eFuse device(s).
- Bounded voltage/current/temperature envelope.
- Defined reverse-current and reverse-polarity behavior.
- Hardware enable and fault output.
- Latching / retry behavior selected for the declared hazard and load.
- Upstream DC-rated fuse and independently verified maintenance isolation.

Advantages: compactness, integration and documented device protection functions.

Risks: internal FET failure mode, thermal concentration, fixed device limits, and possible confusion between electronic interruption and physical isolation.

### Option B — external-FET branch controller

- Dedicated hot-swap / eFuse controller.
- MOSFET bank selected by voltage, current, SOA and thermal mission.
- Independent current/voltage sensing where needed.
- Gate-drive fault and safe-default behavior.
- Independent hardware inhibit and upstream protection.
- Separate maintenance isolation.

Advantages: configurable FET selection, current sharing and layout/thermal architecture.

Risks: larger verification burden; gate-drive and SOA errors; dynamic current sharing; parasitic inductance; thermal imbalance; and more ways to fail.

### Option C — hybrid branch interruption

- Fast semiconductor-controlled current limitation/interruption.
- A separately rated mechanical device provides isolation after current has been driven to a suitable condition.
- Fuse or upstream protective device remains coordinated as backup.
- Interlocks and auxiliary contacts verify state; sensing disagreement gives UNKNOWN, not SAFE.

Advantages: potentially combines electronic response with a physical isolation function.

Risks: contact welding, failed-short semiconductor, transfer/commutation behavior, extra volume and maintenance burden.

### Common rules

- OFF command ≠ isolated.
- No network or PMBus availability requirement for local protection.
- No MCU-only trip path.
- No application-software bypass of the hardware limit.
- Unknown current/switch/sensor state blocks new energization.
- No automatic reclose after unresolved fault or uncertain actuation outcome.
- A protection component is not a breaker/isolator product unless the complete assembly is designed, tested and assessed as such.

## 5. Port and branch contract

A qualified branch instance should declare:
- Exact hardware and PCB revision, BOM and device lot.
- Nominal and allowed voltage range plus transient/surge envelope.
- Continuous, overload and inrush current limits.
- Supported power-flow direction and reverse-current behavior.
- Cable, connector, busbar and terminal ratings.
- Current measurement location, bandwidth, accuracy and calibration.
- Hardware trip thresholds, tolerance, delay and latching policy.
- FET/controller SOA constraints and thermal derating curves.
- Upstream backup protection and fault-clearing coordination.
- Maintenance disconnect and proof-of-isolation method.
- Firmware/configuration digest and allowed compatibility matrix.
- Operating ambient, cooling, enclosure and installation conditions.
- Revalidation scope after repair or component substitution.

The field-mission-profile schema added in this branch is the proposed input contract for deriving branch loads and mission duty. It is not a substitute for waveform, fault-current or installation data.

## 6. Qualification experiment matrix

No physical test should begin until a qualified engineer has reviewed the selected parts, schematic, fault energy, test fixture, source limits and procedure.

| Experiment | Observables | Negative control / fail condition |
|---|---|---|
| Normal conduction map | Branch voltage drop, current, device temperature, PCB/connector hot spots, auxiliary power | Thermal or current limit is exceeded; unexplained loss |
| Inrush/capacitive load | Peak current, source droop, ramp time, SOA trajectory | Uncontrolled current overshoot or repeated restart |
| Programmable current limit | Threshold accuracy, tolerance, trip latency and waveform | Sensor offset/polarity error causes higher allowed current |
| Short-circuit response | Peak let-through current, I²t, energy, upstream-clearing behavior | Protection does not clear or backup coordination fails |
| FET failure short | Actual electrical continuity after switch-off, backup interruption, verified isolation | Controller reports OFF while output remains energized |
| FET/controller open | Loss of branch service, diagnostics and safe restart state | Unintended repeated retry or unstable reconnect |
| Sensor open/short/drift | Independent comparator response and plausibility diagnostics | Missing data is interpreted as zero current / safe |
| MCU reset/watchdog | Gate defaults, hardware inhibit, state retention | Uncommanded pulse or uncontrolled reclose |
| Gate-driver UVLO/brownout | Gate waveform, fault latch and safe output state | Partial enhancement or oscillatory switching |
| Communications/telemetry loss | Local trip behavior and retained safety state | Protection depends on bus/cloud state |
| Reverse-current / polarity | Current path, blocking, thermal and trip behavior | Undeclared backfeed or false healthy indication |
| Thermal soak / blocked airflow | Hot-spot temperatures, resistance drift and derating | Component exceeds reviewed limits |
| Repair/replacement | Hardware identity, configuration, recalibration and retest | Prior evidence is silently inherited across changed hardware |

## 7. Comparison outputs

Publish a multi-dimensional comparison, not a single score:
- Continuous conduction loss at each branch current and ambient.
- Maximum current and SOA margin.
- Short-circuit let-through peak current and I²t.
- Fault-clearing/interrupt time with bandwidth and uncertainty.
- Failure-short response and verified isolation.
- Reverse-current and bidirectional compatibility.
- Inrush performance.
- Thermal rise and derating.
- Component/BOM cost and supply risk.
- PCB area, mass and service/replaceability.
- Diagnostic coverage and fault-state observability.
- Requalification scope after repair.

Safety, fault containment, isolation semantics and electrical compatibility are hard gates. A lower RON, lower BOM or faster nominal trip cannot compensate for a failed hard gate.

## 8. Near-term decision

First compare the TI reference architecture and MPS device family against a discrete external-FET and hybrid candidate using the same declared low-voltage branch profile. Select the smallest useful branch class first. Keep a separate high-current trunk path as a distinct design and qualification problem.

The design review should decide whether Luminous should:
1. Integrate an existing eFuse device into a repairable Luminous branch module;
2. Design an external-FET SSPC for a specific branch class; or
3. Use a hybrid interruption arrangement for the required fault/isolation semantics.

Do not build custom silicon. The initial differentiated value is system architecture, thermal/layout engineering, failure-state design, configuration/evidence integrity and field serviceability—not manufacturing a MOSFET.

## 9. Claim boundary

This is a literature-backed design basis. It contains no schematic, PCB layout, simulated switching waveform, bench measurement, fault-test result, certified protective-device rating or permission to energize a prototype. All ratings and safety claims remain open until a mission profile, design review and appropriate physical qualification exist.
