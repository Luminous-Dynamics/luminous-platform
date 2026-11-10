# Processor-rail transient and vertical power delivery research

**Snapshot:** 10 November 2026  
**Parent:** [Luminous Power Fabric #364](https://github.com/Luminous-Dynamics/luminous-platform/issues/364)  
**Implementation branch:** `research/luminous-power-fabric-transient-v0.5`  
**Status:** literature research and a sampled-event audit screen. No Luminous hardware, VRM, package power-delivery network or physical result is claimed.

## 1. The competitive boundary has moved closer to silicon

The grid-to-chip benchmark is incomplete if it ends at a 6 V, 6.25 V, 12.5 V or 50 V connector. Processor-rail power delivery has its own severe constraints: current slew, undershoot/overshoot, load-line behavior, multiphase sharing, output impedance, package inductance, decoupling, die power integrity, thermal and mechanical integration.

A November 4, 2026 industry article describes a 16-phase regulator transient evaluation from 40 A to 360 A at 800 A/µs. The implied 320 A change over a 0.4 µs ramp is a useful published stress point. Analog Devices' technical article describes the underlying MAX16602 / MAX20790 / coupled-inductor demonstration and explains active voltage positioning / load-line control and the coupled-inductor strategy. These are vendor evaluation conditions and a reported design result, not a universal processor rail specification or an independently matched comparison.

References:
- Aharon Etengoff, [How do multiphase VRMs support AI accelerator transient loads?](https://www.eeworldonline.com/how-do-multiphase-vrms-support-ai-accelerator-transient-loads/), 4 November 2026.
- Analog Devices, [Impacts of Transients on AI Accelerator Card Power Delivery](https://www.analog.com/en/resources/technical-articles/impacts-of-transients-on-ai-accelerator-card-power-delivery.html).

## 2. New scaling boundary: 10 kA-class accelerator power delivery

Google Research lists a 2026 IEEE Transactions on Components, Packaging and Manufacturing Technology paper, [Towards 10kAmp AI Chip: A System Perspective from Lateral to Vertical Power Delivery](https://research.google/pubs/towards-10kamp-ai-chip-a-system-perspective-from-lateral-to-vertical-power-delivery/). Its abstract states that accelerator current demands are rapidly exceeding 10,000 A and examines lateral versus vertical power delivery. It specifically identifies power integrity, signal integrity, thermal management, mechanical design, input-voltage choices, module granularity, warpage and manufacturability as coupled design constraints.

This is a research frontier, not a statement that every current GPU draws 10 kA. It means future power-delivery design cannot treat the VRM output connector as the final boundary. The package, interposer, substrate, die, thermal stack and current-extraction geometry must be included.

**Luminous opportunity:** extend the digital thread and evidence model from converter → shelf → board → package → die interface. Sol-Atlas can hold geometric / stack-up / current-path configuration; Symtropy can couple electrical and electrothermal models; the measurement auditor can preserve the exact rail transient and current waveform evidence.

Reference:
- Gan et al., [Towards 10kAmp AI Chip: A System Perspective from Lateral to Vertical Power Delivery](https://research.google/pubs/towards-10kamp-ai-chip-a-system-perspective-from-lateral-to-vertical-power-delivery/), IEEE Transactions on Components, Packaging and Manufacturing Technology (2026).

## 3. What the new Luminous transient auditor measures

The new Rust module `transient_audit.rs` accepts an explicitly declared processor-rail event with:
- Exact pre-step baseline, step-start, steady-state current window and evaluation-end sample timestamps.
- Rail target and voltage tolerance fraction.
- Expected post-step current and absolute current tolerance.
- Settling dwell requirement.
- Maximum sample gap and clock synchronization budget.
- Per-sample rail voltage, load current and quality flag.

It calculates:
- Measured pre- and post-step load current and current-step amplitude.
- Maximum adjacent-sample upward and downward current slew in A/µs.
- Minimum / maximum rail voltage, maximum undershoot and overshoot.
- First settling time where the rail enters the tolerance band and remains there through the observation end for the declared dwell.
- Maximum sample gap and synchronization-budget status.

The code rejects invalid quality, duplicate/out-of-order timestamps, absent exact event boundaries, non-finite values and sample gaps above the declared limit. It does not resample, interpolate, filter or infer an unobserved edge.

### Synthetic reference fixture

The authored fixture uses 40 A to 360 A over four 100 ns increments:
- Sampled upward slew: 800 A/µs.
- Minimum voltage: 0.87 V against a 0.90 V nominal rail.
- Maximum undershoot: 30 mV (3.33%).
- Declared tolerance band: ±2%.
- Settling: 300 ns after the declared step, with the required dwell observed.

This fixture proves only the arithmetic expected of the audit function. It is not an ADI waveform, not a reproduced vendor result and not a processor voltage-limit requirement. The 30 mV dip deliberately fails the ±2% band so the test can inspect the recovery metric.

## 4. Benchmark protocol for processor rails

A real benchmark must bind the exact load step to the electrical boundary and measurement bandwidth. For each architecture report:

| Dimension | Measurement |
|---|---|
| Load step | Initial and final current, rise / fall time, A/µs, repetition and phase alignment |
| Rail response | Minimum / maximum voltage, overshoot, undershoot and time outside tolerance |
| Settling | Entry into declared band and continuous dwell; include the full observation window |
| Output impedance | Frequency-resolved impedance / PDN target, not inferred from one transient |
| Phase behavior | Per-phase current, imbalance, phase shedding / enable state, interleaving and ripple |
| Decoupling | Capacitance, ESR/ESL, placement, tolerance, voltage and temperature dependence |
| Package boundary | Board, connector, substrate/interposer, package and die-side sensing locations |
| Thermal | Inductor, power stage, capacitor, package and cooling temperature / gradients |
| Efficiency | Full workload and rail map including gate drive / controller / bias and cooling auxiliaries |
| Repeatability | Repeated runs, measurement bandwidth, clock alignment, calibration and uncertainty |

**Do not compare only the smallest observed voltage dip.** Load-line / active voltage positioning, package-level capacitance, coupled inductors and current slew shape the response. The delivered processor workload and voltage tolerance must be identical for candidate and baseline.

## 5. Paradigm-shift research hypotheses

### 5.1 Co-design lateral and vertical power delivery

Compare lateral multiphase conversion against vertical power delivery and package-level power conversion under one electrical / thermal / mechanical boundary. Optimize current path, magnetic placement, decoupling, package warpage, thermal extraction, assembly yield and repair scope together.

### 5.2 Optimize the PDN as an impedance surface

Move beyond a single transient number. Specify target impedance versus frequency and map the contribution of source converter, board planes, package, interposer, decoupling and die. Use frequency-domain impedance measurement and time-domain load steps as complementary evidence.

### 5.3 Treat load shaping as a contractual interface, not a hidden workaround

Workload ramp-rate control and power capping can reduce upstream disturbances, while local capacitance / VRM control addresses the faster processor-rail event. The system benchmark should distinguish performance lost to workload shaping from power-quality improvement. A controller must not silently lower useful compute throughput to win an efficiency comparison.

### 5.4 Coordinate energy storage over multiple timescales

Capacitors at the die / board support very fast rail transients; EDLC / capacitor banks can address short bus hold-up; batteries and grid-forming storage can handle longer facility fluctuations. Their control and energy boundaries must remain separate. Storage cannot be counted as converter efficiency.

### 5.5 Preserve safety authority boundaries

Fast processor-rail control can be optimized locally, but input isolation / HVDC protection remains independent and deterministic. Supervisory AI may propose setpoints or workload envelopes within reviewed bounds; it does not replace local gate protection, overcurrent hardware or qualified grid-forming control.

## 6. Grid interaction research frontier

Recent 2026 papers report that AI workloads can create correlated and oscillatory large-load behavior at the grid boundary:
- An IEEE Transactions on Power Systems paper reports correlated AI data-centre loading can amplify aggregate active-power variability about seven times above an independent-load reference and raise frequency standard deviation eleven times in its studied system.
- A PNNL / IEEE Access paper published 16 October 2026 models UPS, co-generation, cooling and grid-forming storage with converter/control dynamics; its electromagnetic-transient studies report improved frequency nadir, overshoot, settling and generator shaft-torque response under selected load fluctuations.
- An IEEE PESGM paper compares point-of-common-coupling feedback and coordinated plant dispatch for grid-forming BESS, including communication delay effects.

These are model and system studies; their exact network, workload and controller assumptions are not universal site design values. They motivate a separate upstream benchmark: source power ramp rate, fluctuation spectrum, correlation across sites, PCC frequency / voltage response, storage dispatch latency and recovery.

References:
- [Spatially Correlated Load Dynamics in AI Data-Center-Dominated Grids](https://ieeexplore.ieee.org/abstract/document/11673204/), IEEE Transactions on Power Systems, published 31 August 2026.
- Ross et al., [Using Grid-Forming Energy Storage Systems to Provide Dynamic Active Power Support for Hyperscale Data Centers](https://www.pnnl.gov/publications/using-grid-forming-energy-storage-systems-provide-dynamic-active-power-support), 16 October 2026.
- [Mitigating Rapid AI Data Center Power Fluctuations with Grid-Forming Battery Storage: Accounting for Communication Delays](https://ieeexplore.ieee.org/abstract/document/11693692), IEEE PESGM 2026.

## 7. New engineering work and boundaries

The transient auditor is a sampled-event measurement tool only. Its slew is based on adjacent samples and therefore depends on sample interval and measurement bandwidth. Settling is conditional on the exact observation end and dwell. It is not a loop stability test, target-impedance scan, package PDN solver or processor compliance certification.

No Luminous VRM or package design is implemented or validated by this increment. No physical processor-rail test, 800 V energization, HIL, switching simulation or certification is claimed. The Rust tests are authored but not executed in this environment.

## 8. Next programme

Open a dedicated package-level power delivery issue to:
1. Define the package / board / interposer / die measurement boundaries.
2. Capture current waveform and voltage response with calibrated synchronized instrumentation.
3. Compare lateral multiphase, coupled-inductor and vertical delivery.
4. Build target-impedance and transient response evidence.
5. Couple electrical losses to package thermal and mechanical warpage.
6. Include assembly yield, reworkability, repair and requalification.
