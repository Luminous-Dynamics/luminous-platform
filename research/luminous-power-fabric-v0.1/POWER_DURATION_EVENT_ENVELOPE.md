# Power-duration event envelope: transient service qualification

**Snapshot:** 12 November 2026  
**Tracking:** [#425 — qualified power-duration event envelope](https://github.com/Luminous-Dynamics/luminous-platform/issues/425)  
**Parent:** [mission-energy envelope #422](https://github.com/Luminous-Dynamics/luminous-platform/issues/422) · [draft PR #423](https://github.com/Luminous-Dynamics/luminous-platform/pull/423)  
**Evidence status:** deterministic event arithmetic and synthetic fixtures only. No motor-start, ride-through, power-quality or physical qualification is claimed.

## 1. Why average power is not enough

A mission can satisfy every hourly energy budget and still fail at a two-second pump start. The generator may have enough average kW but inadequate step response; an inverter may current-limit; a battery may have enough kWh but inadequate transient kW; or the shared output path may cap the combined source-plus-storage contribution.

A generic surge multiple is incomplete without a declared service boundary, guaranteed duration, source ramp/start/transfer behaviour, event-time battery SOC/SOH/temperature, usable event energy, common bus/converter/cable limits, uncertainty and exact configuration.

Power Fabric represents this as a qualified power-duration event envelope. The source value is the guaranteed minimum over the complete event window, not static nameplate power. Optional storage is qualified for the exact event duration and state. A separately qualified common output-path limit caps their sum.

## 2. Ownership and common service boundary

- **Symtropy** owns generator, inverter and plant dynamic response. Its event source value must already account for ramp, start/transfer delay, fuel/control derating and availability over the window.
- **Mycelix** owns channel/timebase/calibration, SOC/SOH/temperature provenance, uncertainty and evidence invalidation. Storage capability snapshots are tied to the event-time state and operating conditions.
- **Sol-Atlas** owns site environment, layout, cable route, elevation, thermal zone, source accessibility and common-cause context.
- **Symthaea** ranks informative motor-start, source-loss and thermal tests; it cannot relax hard gates.
- **Luminous Platform** composes owner-qualified source, storage and common-path envelopes; it does not replace owner-native models.

All power values must refer to the same useful-service boundary. Source capability, storage capability and common-path capability from different electrical boundaries must not be added without explicit conversion/loss accounting.

## 3. Event equations

For event duration Δt_h, required peak load power P_d, source guaranteed minimum P_s, source measurement uncertainty U_m and source model uncertainty U_r:

P_s,conservative = max(0, P_s - U_m - U_r)

The optional storage snapshot supplies qualified output power P_b, maximum supported duration T_b, battery-side usable energy E_b - U_E and discharge efficiency η_b. Storage is eligible only if exact configuration matches, evidence/calibration/sensor states are current, protection is Ready and T_b covers the event.

P_b,conservative = min(P_b, (E_b - U_E) × η_b / Δt_h)

The common output path (shared bus, inverter, converter, cables, connectors and output stage) has its own qualified event power P_p and uncertainty U_p:

P_p,conservative = max(0, P_p - U_p)

P_support = min(P_s,conservative + P_b,conservative, P_p,conservative)

P_shortfall = max(0, P_d - P_support)

E_event,unserved = P_shortfall × Δt_h

Each event has its own maximum unserved-energy allowance. A failed critical event cannot be compensated by a later event or surplus elsewhere.

## 4. Relationship to the aggregate mission energy ledger

This event check is a power-duration overlay, not a second energy source or load. The motor-start/load-step energy must already appear exactly once in the ordinary mission demand trace and SOC trajectory in #422.

The event evaluator's event-unserved-energy value is diagnostic. Do not add it to aggregate mission unserved energy a second time, and do not subtract the event storage energy again from the mission SOC ledger. The event storage snapshot must already reflect SOC and all prior mission/event commitments at the event time.

v0.1 rejects overlapping event windows. Simultaneous loads must be represented as one composite event. For sequential events, each later storage snapshot must reflect earlier event commitments; otherwise the same battery energy could be reused in multiple event claims.

## 5. Implementation boundary

The dependency-free Rust module in tools/measurement-audit/src/power_event_envelope.rs:
- validates mission/event timing, unique IDs and non-overlapping windows;
- requires qualified/observed source and common-path digests to match;
- gates source and common-path power on current evidence and qualification expiry;
- subtracts conservative uncertainty;
- validates optional storage configuration, capability/calibration/sensor evidence, protection readiness, duration, power, usable energy, efficiency, SOC, SOH and temperature;
- caps storage power by both qualified power and conservative event energy;
- caps source-plus-storage by the qualified shared output path;
- enforces a non-compensatory event-local unserved-energy limit;
- retains source, common-path and storage evidence/configuration snapshots.

It is not a waveform or electromagnetic-transient simulator, induction-motor model, inverter current-limit model, BMS, dispatch controller, relay, breaker, isolation or protection coordination function.

A software event pass does not prove motor acceleration, voltage/frequency quality, DC-bus ripple, THD, fault clearing, interruption or safe isolation. Physical acceptance needs synchronized, calibrated waveform captures and process/load outcomes; protection remains a separate independent gate.

## 6. Benchmark programme

| Event family | Sweep dimensions | Primary outcomes |
|---|---|---|
| Motor start | Pump/compressor/motor; cold/hot; direct-on-line vs VFD/soft starter; load torque | Peak kW/kVA, RMS current, minimum bus voltage, frequency nadir, acceleration time, failed starts |
| Load step/rejection | Positive/negative steps; amplitude, duration, source/SOC state | Voltage/frequency excursion, overshoot, settling time, recovery, current limit |
| Source loss/transfer | High/low load; generator start delay; source return; transfer logic | Ride-through duration, critical-service interruption, restoration time |
| Storage burst | SOC × SOH × temperature × duration × power | Delivered peak, usable event energy, voltage/current limit and thermal response |
| Repeated events | Event spacing, starts/hour, thermal soak/recovery | Within-shift capability fade, temperature accumulation, protection trips |
| Common path | Bus/inverter/cable/connector, source+BESS concurrency | Output bottleneck, voltage drop, connector/bus temperature, common-path derating |

Record synchronized peak/RMS voltage and current, voltage/frequency nadir, DC-bus minimum/overshoot, recovery/settling time, active/reactive/apparent power, power factor, harmonics where relevant, motor speed/acceleration or process completion, source trajectory and transfer delay, event-time storage SOC/SOH/temperature, protection pickup/trip/I²t/interruption/isolation, exact BOM/firmware/settings/layout, calibration/uncertainty, environment and raw-data digest.

## 7. Standards applicability

- **IEC 61000-4-11:2020, Edition 3:** voltage-dip, short-interruption and voltage-variation immunity methods for equipment with rated mains current up to 16 A per phase on 50/60 Hz low-voltage AC networks.
- **IEC 61000-4-34:2005+AMD1:2009+AMD2:2025, consolidated Edition 1.2:** corresponding immunity methods for equipment with mains current above 16 A per phase. The 2025 amendment changes the wording from input current to mains current.
- **IEC 61000-4-4:2012, Edition 3:** electrical fast transient/burst immunity tests on supply, signal, control and earth ports.
- **ISO 8528-5:2025, Edition 6:** design and performance criteria for an RIC engine and AC generator operating as a generating set.
- **IEC TS 62933-2-2:2022:** application-specific EES duty cycles and performance testing.

IEC 61000 immunity methods test equipment against specified disturbances. They do not certify generator load acceptance, motor-start torque, an inverter overload rating, source ride-through or combined hybrid-plant performance. Product and installation scope, including South African SANS/NRCS applicability, must be determined separately.

Publisher sources:
- [IEC 61000-4-11:2020](https://webstore.iec.ch/en/publication/63503)
- [IEC 61000-4-34 consolidated 2025](https://webstore.iec.ch/en/publication/109082)
- [IEC 61000-4-4:2012](https://webstore.iec.ch/en/publication/4222)
- [ISO 8528-5:2025](https://www.iso.org/standard/85960.html)
- [IEC TS 62933-2-2:2022](https://webstore.iec.ch/en/publication/64570)

## 8. Claim rules

1. Source, storage and common-path capability are configuration-bound, current, duration-matched and conservative after uncertainty.
2. Storage support is qualified at the event's SOC/SOH/temperature and includes usable-energy bounds.
3. Source-plus-storage is capped by the separately qualified common output path.
4. Every critical event meets its own service allowance.
5. The aggregate mission ledger includes event load and battery energy exactly once.
6. The software screen is paired with synchronized calibrated waveforms and process/load outcomes before a physical event claim.
7. Fault clearing, interruption, isolation and safe operating area pass independent protection evidence.
8. Publish failed events, exclusions, instrument bandwidth, expanded uncertainty, exact hardware/firmware and data rights.

No percentage superiority target is set until the same event suite is run on a relevant incumbent and the exact Power Fabric configuration.
