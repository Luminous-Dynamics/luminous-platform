# Multi-timescale storage and grid dynamics research

**Snapshot:** 10 November 2026  
**Parent:** [Luminous Power Fabric #364](https://github.com/Luminous-Dynamics/luminous-platform/issues/364)  
**Implementation branch:** `research/luminous-power-fabric-buffer-v0.6`  
**Status:** current literature synthesis and an ideal constant-import storage sizing screen. No Luminous battery, EDLC, UPS, grid-forming inverter or facility result is claimed.

## 1. One buffer cannot solve every timescale

The power-delivery chain now needs to distinguish energy buffers by their location and response domain:

| Layer | Primary disturbance | Candidate buffer / control | Evidence needed |
|---|---|---|---|
| Die / package | Very fast current edge, PDN impedance | Package / on-die decoupling, vertical delivery, local VRM control | Target impedance, waveform, package/die voltage |
| Board / tray | Microsecond-to-millisecond load steps | Multiphase VRM, board capacitors, coupled inductors | Current sharing, undershoot/overshoot, ESR/ESL, thermal |
| Rack / DC bus | Millisecond-to-second bursts and hold-up | Bus capacitors, EDLC / capacitor bank, DC UPS | Bus ripple, hold-up, charge/discharge power and stored energy |
| Facility | Seconds-to-minutes load ramps, generator / UPS transitions | Battery ESS, UPS, co-generation, grid-forming inverter | PCC power/frequency/voltage dynamics, SOC, delay, fault recovery |
| Grid / fleet | Minutes-to-hours variability and spatial correlation | Coordinated storage, grid operations, workload scheduling | Cross-site correlation, reserve, spectra, interconnection studies |

The table is a research decomposition, not a statement that the time bands are universally fixed. Real systems overlap, and storage technology capabilities depend on the device, converter, controls and installation.

**Paradigm shift:** optimize the energy and control allocation across timescales instead of assigning every fluctuation to the central battery or the local capacitor bank.

## 2. Industry research shows the grid-facing problem is dynamic

### 2.1 Spatial correlation changes reserve requirements

An August 31, 2026 IEEE Transactions on Power Systems paper, *Spatially Correlated Load Dynamics in AI Data-Center-Dominated Grids*, combines time-domain statistical measures and frequency-resolved spectral analysis with EMT simulation on IEEE 39-bus. In its studied cases, AI data-centre loading amplified aggregate active-power variability about seven times versus its independent-load reference and produced frequency standard deviation eleven times larger. It attributes low-frequency alignment to workload orchestration / thermal management and higher-frequency alignment to converter-control interactions and network impedance.

These are results for the paper's network, workload model and reference case. They are not universal planning multipliers. The important research implication is that independent site traces can understate aggregate variability when training workloads are synchronized.

Reference:
- [Spatially Correlated Load Dynamics in AI Data-Center-Dominated Grids](https://ieeexplore.ieee.org/abstract/document/11673204/), IEEE Transactions on Power Systems, published 31 August 2026.

### 2.2 Grid-forming storage is an active power-system subsystem

PNNL's October 16, 2026 article describes an IEEE Access study with a dynamic data-centre model including UPS, co-generation, cooling and grid-forming energy storage. EMT scenarios include low-voltage faults and fast AI training fluctuations. The study reports improvements in frequency overshoot, nadir, settling, post-fault recovery, active-power oscillation and generator shaft-torque oscillation under selected control conditions.

A 2026 IEEE PESGM paper comparing PCC-feedback broadcast and coordinated plant dispatch also reports communication-delay sensitivity: the better-performing strategy depends on delay, and large delay can destabilize the PCC-feedback approach.

References:
- Ross et al., [Using Grid-Forming Energy Storage Systems to Provide Dynamic Active Power Support for Hyperscale Data Centers](https://www.pnnl.gov/publications/using-grid-forming-energy-storage-systems-provide-dynamic-active-power-support), 16 October 2026.
- [Mitigating Rapid AI Data Center Power Fluctuations with Grid-Forming Battery Storage: Accounting for Communication Delays](https://ieeexplore.ieee.org/abstract/document/11693692), IEEE PESGM 2026.

### 2.3 Load shaping has a cost

IEEE's 2026 review of AI data-centre power-quality mitigation discusses artificial load modulation, GPU ramp-rate tuning, power capping, multi-level energy storage and system-level coordination. Workload shaping can reduce upstream power fluctuations, but may affect time-to-train, throughput, latency or utilization.

The benchmark must therefore count useful compute delivered as well as energy consumed. A controller must not quietly reduce accelerator throughput and claim that the facility became more efficient.

Reference:
- Maheshwari et al., [AI Data Centers: A Review of Power Quality Challenges and Mitigation Strategies](https://doi.org/10.1109/PECI70026.2026.11516457), IEEE Power and Energy Conference at Illinois 2026.

## 3. Ideal constant-import buffer sizing

The new Rust module `storage_buffer.rs` estimates the power and usable-energy lower bound needed to maintain a declared constant grid-import target against a sampled facility load profile.

For load power (P_L(t)) and grid-import target (P_G):

```
P_discharge(t) = max(P_L(t) - P_G, 0)
P_charge(t)    = max(P_G - P_L(t), 0)
```

For fixed directional efficiencies:

```
dE_storage/dt = eta_charge * P_charge
                - P_discharge / eta_discharge
```

The integrator linearly interpolates between adjacent load samples. When a sample interval crosses the grid-import target, it splits the interval at the exact crossing. This preserves charge and discharge energy separately rather than allowing opposite-sign contributions to cancel before efficiency is applied.

The facility-profile interchange draft is `schemas/facility-power-profile-v0.1.schema.json`, with a synthetic sample in `schemas/facility-power-profile.example.json`. The files parse as JSON and the synthetic example has strictly increasing timestamps, valid sample flags and gaps within its declared bound. JSON Schema validation and a runtime JSON parser have not been executed or implemented.

The result reports:
- Load and target grid-import energy.
- Charge input and discharge delivered energy.
- Internal storage-energy change and conversion losses.
- Minimum usable-energy capacity and minimum initial stored energy.
- Actual terminal / minimum / maximum stored energy for the supplied initial state.
- Peak charge and discharge power.
- Energy-capacity, charge-power and discharge-power limit statuses.
- Bus-side energy-balance residual and maximum sample gap.

### Synthetic fixtures

**A 1 kW base load, 2 kW one-second pulse, then 1 kW base load**, with a constant 1 kW grid-import target:
- Discharge delivered to the load: 1,000 J.
- At 95% discharge efficiency, internal energy depletion: 1,052.63 J.
- Peak discharge power: 1,000 W.
- Minimum usable storage energy for the trace: 1,052.63 J.

**A 1 kW target with a 0 W one-second trough between 1 kW intervals**, with 90% charge efficiency:
- Grid surplus accepted for charging: 1,000 J.
- Stored energy gained: 900 J.
- Peak charge power: 1,000 W.

These are analytical fixtures, not selected cell ratings, site sizing recommendations or storage-controller test results.

## 4. Why power and energy must be sized separately

A buffer can have sufficient kWh but insufficient kW, or sufficient kW but insufficient usable kWh. The new screen returns both.

A 1 MW deficit lasting 100 ms requires 100 kJ of delivered energy before accounting for discharge losses. A 100 kW deficit lasting 10 seconds also requires 1 MJ. The lower peak power does not imply the lower energy requirement.

For each candidate technology record its actual:
- Power rating, charge and discharge.
- Usable energy and state-of-energy window.
- Charge / discharge efficiency versus power and temperature.
- Ramp / response limits and converter current limits.
- Thermal and aging / cycle-life derating.
- Fault-current, isolation, protection and service boundaries.

Do not infer energy capacity from power rating or vice versa.

## 5. Luminous multi-timescale architecture

The longer-term architecture should make timescale and authority explicit:

1. **Local rail:** package / board capacitors and VRM regulate fast processor load changes. The transient auditor reports sampled step response; a future PDN solver / impedance analyzer must handle frequency-domain behaviour.
2. **Rack / tray:** EDLC / capacitor banks provide bounded DC bus hold-up and ramp support. They do not replace longer-duration batteries.
3. **UPS / battery:** supports longer facility disturbances within declared SOC, inverter and thermal constraints.
4. **Grid-forming interface:** local inverter controls provide fast, deterministic voltage/frequency support; slower plant coordination restores SOC and dispatch targets.
5. **Workload orchestrator:** may schedule or ramp workloads within explicit throughput / latency contracts. It is not part of the electrical protection loop.

Fast local control should not depend on cloud connectivity. Coordinated dispatch needs explicit communication-delay limits and safe fallback to local control.

## 6. Better benchmarks than peak efficiency

The storage and grid benchmark should add:

| Dimension | Metric |
|---|---|
| Load variability | Max up/down ramp (MW/s), RMS fluctuation, percentiles and event duration |
| Spectral structure | Power spectral density / band power, dominant frequencies and non-stationarity |
| Multi-site effects | Cross-correlation, synchronized burst fraction, aggregate ramp and reserve |
| Buffer sizing | Peak kW, usable kWh, SOC swing, terminal SOC, efficiency and energy residual |
| Grid interaction | PCC voltage / frequency nadir, overshoot, settling, oscillation damping and power quality |
| Control robustness | Local response latency, supervisory delay/jitter, packet loss, fallback time and stability envelope |
| Compute preservation | Delivered useful work, throughput, time-to-train and service-level compliance |
| Lifecycle | Cycle throughput, capacity fade, thermal exposure, replacement and service cost |

All comparisons must use identical workload traces, initial state, boundary, grid conditions and useful compute. Simulated EMT results, measured site results and synthetic traces remain separate evidence kinds.

## 7. Limitations and next qualification gates

The Rust screen assumes ideal instantaneous buffer power tracking and fixed directional efficiencies. It does not model:
- Battery chemistry, cell voltage, SOC nonlinearities, aging, temperature, current and voltage limits.
- EDLC equivalent circuits, ESR/ESL, leakage or thermal state.
- Converter current loops, DC bus impedance, inverter control, grid strength or fault ride-through.
- Communication delay, jitter, packet loss or control stability.
- Generator shaft dynamics, UPS transfer sequences or real facility operation.

Next steps:
1. Add time-varying efficiency maps and explicit power limits by technology.
2. Add measured / simulated PCC traces with timestamp and calibration provenance.
3. Model two-stage storage dispatch: local fast response plus slower SOC restoration.
4. Compare independent versus correlated workload traces.
5. Validate with a declared EMT model and independent power-systems review.
6. Keep hardware and grid-interconnection claims behind accredited laboratory and utility approval.

No Luminous battery, EDLC, UPS, grid-forming inverter or data-centre system is built or qualified by this research.
