# Time-dependent mission energy and storage reserve envelope

**Snapshot:** 12 November 2026  
**Owner:** Luminous Power Fabric / measurement-audit  
**Tracking:** [#422 — Time-dependent mission energy and end-of-mission reserve envelope](https://github.com/Luminous-Dynamics/luminous-platform/issues/422)  
**Parent architecture:** [draft PR #414](https://github.com/Luminous-Dynamics/luminous-platform/pull/414)  
**Evidence status:** deterministic software design and synthetic arithmetic only; no physical product or field qualification is claimed.

## 1. Paradigm shift: qualify the mission, not the nameplate

A power system should not pass because its catalogue says "100 kW / 200 kWh". It should pass only when a specified system configuration can deliver the required service over the entire time-indexed mission, while respecting source availability, storage power and energy limits, losses, uncertainty and the end-of-mission reserve.

This changes the comparison unit from device ratings to a **mission service envelope**:

- same useful-service boundary and time-indexed demand;
- same initial storage state and minimum final reserve;
- same source outages, ambient conditions and recovery obligations;
- owner-qualified source capability for each interval, including source ramp/start response;
- storage power and energy constraints, with conservative efficiency and uncertainty bounds;
- per-interval energy and evidence lineage.

This prevents three common comparison errors: crediting surplus generation as useful service, treating nominal battery capacity as fully dispatchable energy, and letting a hybrid system consume its reserve to appear superior during a short test.

## 2. Ownership boundary

The evaluator deliberately does not recreate the plant models already owned elsewhere:

| System | Owns | Platform consumes |
|---|---|---|
| Symtropy | Time-dependent source/plant physics, generator and inverter behaviour, ramp/start behaviour, co-simulation and matched-service fuel accounting | A qualified interval-average source capability and its evidence/configuration lineage |
| Mycelix | Instrument/channel identity, calibration, timebase, uncertainty, evidence validity and invalidation | Evidence statuses and references; no second calibration registry |
| Sol-Atlas | Site geometry, location, environmental exposure and spatially coupled resource constraints | Site/ambient and deployment context |
| Symthaea | Experiment selection and information-gain ranking | Candidate experiments only; no safety, evidence or dispatch authority |
| Luminous Platform | Mission profile, capability composition, cross-system configuration identity and auditable service result | Owner-native contracts and evidence references |

The key source interface is **qualified_source_average_power_w**, bound to a qualified/observed source configuration pair and source-specific expiry. Symtropy must calculate the interval average from its time-dependent source model, including any ramp, start delay, fuel/control derating and source availability represented in that model. This evaluator does not interpolate generator physics from a static nameplate rating.

## 3. Model boundary and units

One mission comprises contiguous, piecewise-constant intervals. Required service power, qualified source-average power and the storage charge/discharge power limits are referred to one declared useful-service boundary for the entire comparison: for example, the 48 V DC service bus or the AC customer outlet. Storage capacity and SOC are battery-side energy state; charge efficiency maps useful-boundary input energy to battery-side stored energy, while discharge efficiency maps battery-side withdrawn energy to useful-boundary output. Do not mix generator terminals, battery DC terminals and delivered AC energy without explicitly accounting for intervening conversion losses upstream.

For interval duration Δt_h, required average service power P_d, qualified interval-average source power P_s, measurement uncertainty bound U_m, and model uncertainty bound U_r:

\[
E_d = P_d \Delta t_h
\]

\[
P_{s,\mathrm{conservative}} =
\max(0, P_s-U_m-U_r)
\]

\[
E_s = P_{s,\mathrm{conservative}}\Delta t_h
\]

The uncertainty bounds are absolute watts. Their conservative sum is subtracted; this v0.1 evaluator does not assume statistical independence or combine covariance. A non-current source evidence status excludes that source interval completely.

### Direct source service and storage discharge

\[
E_{\mathrm{direct}} = \min(E_d,E_s)
\]

\[
E_{\mathrm{deficit}} = \max(0,E_d-E_{\mathrm{direct}})
\]

For effective capacity C_eff = C - U_C, current conservative stored energy S, minimum SOC fraction q_min, qualified discharge efficiency η_d, maximum discharge power P_d,max, and interval duration Δt_h:

\[
E_{\mathrm{store\rightarrow load}} =
\min\left(
E_{\mathrm{deficit}},
P_{d,\max}\Delta t_h,
\max(0,S-q_{\min}C_{\mathrm{eff}})\eta_d
\right)
\]

Battery-side energy withdrawn is E_store→load / η_d. Remaining deficit is unserved energy. Storage discharge is therefore constrained by both power and available energy.

### Source surplus and storage charge

\[
E_{\mathrm{surplus}}=\max(0,E_s-E_{\mathrm{direct}})
\]

\[
E_{\mathrm{charge,in}} =
\min\left(
E_{\mathrm{surplus}},
P_{c,\max}\Delta t_h,
\frac{\max(0,q_{\max}C_{\mathrm{eff}}-S)}{\eta_c}
\right)
\]

Battery-side energy added is E_charge,in η_c. The remaining source surplus is curtailed, not credited as delivered service. The algorithm never charges and discharges the aggregate store in the same interval.

### Conservative initial and end state

\[
C_{\mathrm{eff}}=C_{\mathrm{declared}}-U_C
\]

\[
S_0=\max(0,q_0-U_{q_0})C_{\mathrm{eff}}
\]

The SOC uncertainty is an absolute fraction (0.01 means one percentage point). The mission reserve passes only when final conservative stored energy is at least:

\[
S_{\mathrm{final}}\ge q_{\mathrm{reserve}}C_{\mathrm{eff}}
\]

Each segment also carries its own maximum permitted unserved energy. The segment must pass its local service gate, and the mission must pass its aggregate unserved-energy budget. A generous aggregate allowance cannot compensate for a critical segment shortfall. Set a critical segment's maximum to zero.
 
A service-complete run that misses its final reserve is a **mission failure**. This is intentional: it cannot borrow resilience from the next shift or emergency.

## 4. Evidence and fail-closed rules

- The exact top-level qualified and observed system configuration SHA-256 digests must match.
- The mission qualification must be current at evaluation time; mismatch or expiry withdraws all source and storage capability.
- Each segment has qualified and observed source configuration SHA-256 digests that must match, a source-specific qualification expiry later than evaluation time, a source evidence status of Current and an evidence reference. Any mismatch, expiry or non-current status excludes that segment's source power.
- If a storage bank is included in the evaluated mission, it must be evidence-eligible and its final reserve must be verifiable for the mission to pass. A deliberately generator-only comparator should omit the storage object rather than attach an unqualified store.
- Storage is eligible only when its configuration digests match, its qualification is current, capability/calibration/sensor statuses are Current, and independent protection is Ready.
- Storage capacity and initial SOC uncertainty reduce the available energy. Power and efficiency inputs must be conservative qualified values, not optimistic nominal maxima.
- Missing, invalid, expired, unverified or non-ready evidence cannot increase capability.
- All inputs are checked for finite values, valid units/ranges, unique segment IDs and a contiguous timeline. Numeric overflow is an error.
- The report retains source configuration/evidence lineage per interval, storage passport reference, SOC trajectory, source energy, charge/discharge, curtailed energy and unserved energy.

A component excluded for stale evidence does not automatically invalidate a mission that can demonstrably meet service without that component. A global system configuration mismatch or expired mission qualification is different: it blocks the whole evaluated configuration.

## 5. What this evaluator does not prove

This is deterministic interval-energy accounting. It is not:

- a dispatch optimiser, model-predictive controller, BMS, inverter controller or protective function;
- a generator, converter, battery-cell, thermal, degradation or electrochemical model;
- an AC/DC transient, harmonics, voltage/frequency, fault-current, motor-inrush or black-start simulation;
- a network-flow solver, N-1 reliability assessment or probabilistic availability model;
- a fuel-consumption model or an independent measurement/calibration service;
- a safety assessment, standards conformity test, type test or field qualification.

Averaged intervals cannot establish a sub-second motor-start or fault-response pass. Those require owner-native dynamic models and synchronized, instrumented physical tests. A source interval must not silently flatten a transient into a static rating.

## 6. Benchmark programme and target gates

### Tier 0 — arithmetic and evidence oracle

- Run generator-only, storage-only and generator-plus-storage scenarios against identical synthetic service demand.
- Prove source-energy conservation: source energy available equals source-to-load plus charge input plus curtailment.
- Prove service-energy conservation: required energy equals source-to-load plus storage-to-load plus unserved energy.
- Prove charge/discharge efficiency losses and capacity/power/SOC limits.
- Prove a service-complete but reserve-deficient mission fails.
- Require deterministic output under repeat evaluation and reject invalid digests, gaps, duplicate segments, non-finite values and stale evidence.

### Tier 1 — module and source-to-load bench

Capture synchronized input/output energy at every declared conversion boundary; measure continuous and time-windowed power, round-trip efficiency, standby consumption, thermal derating, charge/discharge response, ramp, transient voltage/frequency/THD and safe interruption independently. Use calibration and uncertainty from Mycelix, and keep test fixture/software versions bound to the hardware configuration.

### Tier 2 — matched useful-service mission

Compare generator-only, commercial generator+BESS, commercial BESS-only where feasible, and Luminous-integrated systems with exactly the same:
- load trace and useful-service boundary;
- source/fuel and environmental conditions;
- initial SOC and end-of-mission reserve;
- outage and recovery sequence;
- mission duration and duty cycle;
- measurement methods, uncertainty treatment and maintenance state.

Primary metrics: delivered useful kWh, unserved Wh and duration, fuel per delivered useful kWh, battery input/output energy, SOC trajectory, final reserve, charge/discharge cycles, source run-hours and starts, standby/cooling losses, service recovery time, noise and total cost per delivered useful kWh. Surplus energy is never counted as useful service.

### Tier 3 — resilience, degradation and lifecycle

Repeat matched missions across high/low ambient, altitude, dust/moisture exposure, source loss/return, failed start, reduced battery capability, cooling degradation, sensor invalidation, protection trip, module replacement and requalification. Record repair time, replacement mass/cost, unavailable hours, capacity fade, energy throughput and evidence completeness.

### Non-negotiable gates before any SOTA claim

1. Zero unexplained energy creation in the independent ledger; measured residual must be inside the agreed expanded measurement uncertainty.
2. 100% of required critical-service intervals meet the customer-defined limit, or every deficit is explicitly recorded and the mission is marked failed.
3. The final SOC reserve obligation is met in every claimed successful mission.
4. Exact same service, initial/final state, environmental conditions and measurement uncertainty for all comparators.
5. Protection, isolation and power-quality gates pass independently; they cannot be compensated by fuel or efficiency gains.
6. Publish all test exclusions, failed runs, measurement uncertainty, hardware/firmware/BOM identity and source data rights.

No arbitrary percentage "better than SOTA" is frozen before a baseline is measured. Once a matched baseline exists, set a pre-registered improvement margin that exceeds combined measurement uncertainty and test-to-test variance. For the present 500 W PEBB-LV-01 research candidate, the first fair comparisons are component-level DC conversion/protection and a small service-node mission—not a direct nameplate competition with containerized commercial BESS.

## 7. Standards and external benchmark map

The standards below inform test design; listing them is not a conformity claim and does not decide South African product classification or legal market access.

- **ISO 8528-5:2025 (Edition 6):** design/performance criteria for reciprocating-engine-driven AC generator sets as a unit. It is the current Part 5 edition and replaces the withdrawn 2022 edition. Apply it to the generator-set subsystem, not to the entire hybrid mission evaluator.
- **IEC TS 62933-2-2:2022:** application and performance testing for EES systems, including application-specific duty cycles and test methods. This is directly relevant to matched mission profiles.
- **IEC TS 62933-2-3:2025:** EES performance assessment tests during site operation, useful for post-commissioning validation.
- **IEC 62933-2-1:2017:** general EES unit parameters and testing methods.
- **IEC 62619:2022:** industrial lithium-cell/battery safety requirements and tests; it is a battery-safety reference, not an energy-performance score.
- **IEEE 2030.3-2016:** active standard providing test procedures for electric energy storage equipment/systems used in electric power systems.
- **IEEE 1679-2020:** characterization/evaluation guidance for stationary storage technologies, emphasizing conditions and limitations needed for valid comparison.

Sources:
- [ISO 8528-5:2025](https://www.iso.org/standard/85960.html)
- [IEC TS 62933-2-2:2022](https://webstore.iec.ch/en/publication/64570)
- [IEC TS 62933-2-3:2025](https://webstore.iec.ch/en/publication/77334)
- [IEC 62933-2-1:2017](https://webstore.iec.ch/en/publication/27124)
- [IEC 62619:2022](https://webstore.iec.ch/en/publication/64073)
- [IEEE 2030.3-2016](https://standards.ieee.org/ieee/2030.3/5076/)
- [IEEE 1679-2020](https://standards.ieee.org/ieee/1679/7716/)

## 8. Synthetic arithmetic fixture

The file schemas/mission-energy-envelope.synthetic.json contains a two-segment, one-hour-per-segment hybrid illustration:

| Quantity | Synthetic value |
|---|---:|
| Segment 1 demand / source | 1,000 W / 2,000 W |
| Segment 1 source-to-load / charging | 1,000 Wh / 1,000 Wh input |
| Charge efficiency / stored increment | 90% / 900 Wh |
| Segment 2 demand / source | 800 W / 0 W |
| Segment 2 storage-to-load | 800 Wh |
| Battery-side withdrawal at 90% discharge efficiency | 888.89 Wh |
| Total mission demand / unserved | 1,800 Wh / 0 Wh |
| End SOC (conservative) | approximately 20.22% |
| End reserve threshold | 20% |

All values are invented arithmetic controls. They are not measured efficiencies, battery performance, generator performance or a product rating.
