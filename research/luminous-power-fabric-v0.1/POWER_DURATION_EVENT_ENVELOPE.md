# Power-duration event envelope: transient service qualification

**Snapshot:** 12 November 2026  
**Tracking:** [#425 — qualified power-duration event envelope](https://github.com/Luminous-Dynamics/luminous-platform/issues/425)  
**Parent:** [mission-energy envelope #422](https://github.com/Luminous-Dynamics/luminous-platform/issues/422) · [draft PR #423](https://github.com/Luminous-Dynamics/luminous-platform/pull/423)  
**Evidence status:** deterministic event arithmetic and synthetic fixtures only. No motor-start, ride-through, power-quality or physical qualification is claimed.

## 1. Why average power is not enough

A mission can satisfy all hourly energy budgets and still fail at a two-second pump start. A generator may have sufficient average kW but insufficient step response; an inverter may current-limit; a battery may have adequate kWh but inadequate transient power; or the shared output path may cap the combined source-plus-storage contribution.

A generic surge multiple is incomplete unless it states event power at a defined useful-service boundary, the duration for which that power is guaranteed, source ramp/start/transfer response, battery state of charge/health/temperature and available energy at that event time, the converter/bus/cable/connector limit, measurement/model uncertainty and exact configuration.

Power Fabric represents this as a qualified power-duration event envelope. The event is a time window with a peak service requirement; source and optional storage capability are qualified for at least that duration; a common output-path limit bounds their sum.

## 2. Ownership and common service boundary

- **Symtropy** owns generator, inverter and plant dynamic response. Its event source value is the guaranteed minimum power over the entire event window after source start delay, ramp limits, fuel/control derating and source availability are accounted for.
- **Mycelix** owns instrument channels, timebase, calibration, SOC/SOH/temperature measurement provenance, uncertainty and evidence invalidation. Storage usable-energy/power snapshots must be bound to the event's state and operating condition.
- **Sol-Atlas** owns site environment, layout, cable route, elevation, thermal zone, source accessibility and common-cause context.
- **Symthaea** can choose the most informative motor-start, source-loss and thermal test; it cannot relax an event, evidence or protection gate.
- **Luminous Platform** composes the owner-qualified source, storage and common-path envelopes and reports the event result. It does not replace their models or authority.

All power values in the event contract are at the same declared useful-service boundary (e.g. a 48 V DC service bus or 400 V AC load bus). Source capability, storage capability and common-path capability cannot be taken from different electrical boundaries without an explicit upstream conversion model and loss accounting.

## 3. Event envelope equations

For event duration Δt_h, required peak load power P_d, source guaranteed minimum P_s, source measurement uncertainty U_m and source model uncertainty U_r:

P_s,conservative = max(0, P_s - U_m - U_r)

The optional storage snapshot supplies a qualified output power P_b, maximum supported duration T_b, conservative battery-side usable energy E_b - U_E, and qualified discharge efficiency η_b. It must also carry its event-time SOC, SOH, temperature, configuration and evidence. Storage support is eligible only when the exact configuration matches, evidence and calibration are current, protection is Ready and T_b covers the event duration.

P_b,conservative = min(P_b, (E_b - U_E) × η_b / Δt_h)

This prevents a battery with adequate peak kW but insufficient usable event energy from appearing capable. The energy snapshot must already account for all previous mission/event commitments and the event-time SOC; it is not inferred from nameplate kWh.

The source-plus-storage sum is still not the final output. A common path—shared DC bus, inverter, converter, cables, connectors or output stage—may be the actual bottleneck. Its own qualified event power P_p and uncertainty U_p cap the sum:

P_p,conservative = max(0, P_p - U_p)

P_support = min(P_s,conservative + P_b,conservative, P_p,conservative)

P_shortfall = max(0, P_d - P_support)

E_event,unserved = P_shortfall × Δt_h

Every event carries its own maximum permitted unserved energy. A failed critical event cannot be compensated by a later event or by excess energy elsewhere.

## 4. Event and energy-ledger relationship

The event check is a power-duration overlay on the mission energy profile, not an extra energy source or an extra customer load. The motor-start or load-step energy is already included exactly once in the ordinary mission demand/load trace and the mission storage trajectory in #422.

The evaluator's event_unserved_energy_wh is a diagnostic derived from peak-power shortfall over that window. Do not add it to aggregate mission unserved energy a second time. Conversely, do not subtract event storage energy again from the mission SOC ledger: the event snapshot's usable energy must already reflect SOC and energy commitments at that point in the mission.

v0.1 rejects overlapping events. If two loads start together, represent them as one composite event with their combined load and common path requirement. For sequential events, the storage snapshot for each later event must reflect energy consumed by earlier events. This prevents reusing the same battery energy in several separate event claims.

## 5. Implemented software boundary

The Rust module in tools/measurement-audit/src/power_event_envelope.rs provides a dependency-free, deterministic, read-only evaluator. Each event validates mission and event timing; requires qualified/observed source and common-path configuration digests to match; requires current source/common-path evidence and qualification validity; subtracts conservative source and common-path uncertainty; optionally validates storage configuration, capability/calibration/sensor evidence, protection state, event duration, usable energy, efficiency and state snapshot; caps storage output by both qualified power and conservative usable event energy; caps source-plus-storage by the common output path; calculates peak power shortfall and event-window diagnostic unserved energy; enforces the event-local unserved energy allowance; and retains source, common-path and storage evidence/configuration snapshots.

The API sorts non-overlapping events by event start and ID for deterministic output and rejects duplicate or overlapping windows.

## 6. What a pass does not mean

This evaluator is not a sampled waveform or electromagnetic transient simulation; an induction-motor equivalent circuit, acceleration/load-torque or VFD/soft-starter model; an inverter semiconductor current-limit/foldback model; an AC voltage/frequency, DC-bus ripple, THD, sag or recovery measurement; a relay, fuse, eFuse, SSPC, breaker, isolation or protection coordination function; a power-HIL/controller-HIL result, product type test or standards conformity certificate.

A power-duration event pass only screens a declared, evidence-qualified support envelope. It does not establish voltage/frequency quality or that the motor accelerates successfully. A physical acceptance test must pair calibrated current/voltage waveform captures with torque/speed or load/process evidence where applicable; protection and safe isolation are separate independent gates.

## 7. Benchmark programme

Run matched events on a current-limited, contained bench with exact configuration and a safe test procedure, then progress only through reviewed power levels and qualified test facilities.

| Event family | Sweep dimensions | Primary outcomes |
|---|---|---|
| Motor start | Pump/compressor/motor; cold/hot; direct-on-line vs VFD/soft starter; load torque; start interval | Peak kW/kVA, RMS current, minimum bus voltage, frequency nadir, acceleration time, failed starts |
| Load step | Positive and negative steps; event amplitude and duration; source/SOC state | Voltage/frequency excursion, overshoot, settling time, recovery, current limit |
| Source loss/transfer | Loss at high/low load; generator start delay; source return; transfer logic | Ride-through duration, critical-service interruption, restoration time |
| Storage burst | SOC × SOH × temperature × event duration × power | Delivered peak, usable event energy, voltage/current limit and thermal response |
| Repeated events | Event spacing, start count/hour, thermal soak and recovery | Capability fade within the shift, temperature accumulation, protection trips |
| Shared path | Common bus/inverter/cable/connector limits and source+BESS coexistence | Output bottleneck, connector/bus temperature, voltage drop and common-path derating |

Record at minimum peak and RMS voltage/current with synchronized timebase; voltage and frequency nadir; DC-bus minimum/overshoot; recovery/settling time; active/reactive/apparent power and power factor; harmonics where applicable; motor speed/acceleration or process completion; source power trajectory and transfer delay; storage SOC/SOH/temperature and energy before/after; protection pickup/trip/no-trip, let-through/I²t, interruption, fail-short handling and isolation state; exact BOM, firmware, settings, cable/connector layout, calibration, measurement uncertainty, environment and raw-data digest.

## 8. Standards map (scope must fit the equipment)

- **IEC 61000-4-11:2020, Edition 3:** immunity test methods for voltage dips, short interruptions and voltage variations for equipment with rated mains current up to 16 A per phase on 50/60 Hz low-voltage AC networks.
- **IEC 61000-4-34:2005+AMD1:2009+AMD2:2025, consolidated Edition 1.2:** voltage-dip/short-interruption/voltage-variation immunity test methods for equipment with mains current above 16 A per phase. The 2025 amendment updates the wording from input current to mains current.
- **IEC 61000-4-4:2012, Edition 3:** electrical fast transient/burst immunity tests on supply, signal, control and earth ports.
- **ISO 8528-5:2025, Edition 6:** design and performance criteria for the combination of RIC engine and AC generator when operating as a generating set.
- **IEC TS 62933-2-2:2022:** EES application-specific duty cycles and performance testing.

These IEC 61000 immunity methods test equipment response to specified disturbances. They do not certify generator load acceptance, motor-start torque, an inverter's overload rating, ride-through from a source or the performance of a combined hybrid plant. Apply product-family and installation standards separately; determine South African SANS/NRCS requirements from the final product classification.

Publisher sources:
- [IEC 61000-4-11:2020](https://webstore.iec.ch/en/publication/63503)
- [IEC 61000-4-34 consolidated 2025](https://webstore.iec.ch/en/publication/109082)
- [IEC 61000-4-4:2012](https://webstore.iec.ch/en/publication/4222)
- [ISO 8528-5:2025](https://www.iso.org/standard/85960.html)
- [IEC TS 62933-2-2:2022](https://webstore.iec.ch/en/publication/64570)

## 9. Acceptance and claim rules

1. Every source and common-path event capability is configuration-bound, current, duration-matched and conservative after uncertainty.
2. Storage event support is qualified at the event's SOC/SOH/temperature and includes a usable energy limit, not only peak kW.
3. The source-plus-storage sum is capped by the separately qualified common output path.
4. Critical events meet their own maximum-unserved-energy limit; no across-event averaging can compensate for a failure.
5. The aggregate mission energy/SOC ledger includes the event load and battery energy exactly once.
6. The software event screen is paired with a synchronized, calibrated waveform and process/load result before any physical event-performance claim.
7. Fault clearing, interruption, isolation and safe operating area are passed by independent protection evidence. Energy or service benefits cannot compensate for a protection failure.
8. Publish failed starts/events, exclusions, instrument bandwidth, expanded uncertainty, exact hardware/firmware and source data rights.

No percentage superiority target is set until the same event suite is run on a relevant incumbent and Power Fabric configuration.
