# PEBB-LV-01 design basis v0.1

**Parent:** [Luminous Power Fabric #364](https://github.com/Luminous-Dynamics/luminous-platform/issues/364)  
**Date:** 2026-11-10  
**Status:** candidate design basis; requirements are not frozen

## 1. Use case

A serviceable, isolated, bidirectional DC/DC building block for connecting two nominally 48 V DC
domains, such as a low-voltage storage emulator and a local DC bus. The first demonstrator is a
laboratory platform, not a residential product, charger, UPS, PV inverter or grid-connected device.

The v0.1 objective is to establish a repeatable design and measurement process and to learn which
topology gives the best combined efficiency, dynamic range, thermal behavior, safety boundary,
cost and repairability.

## 2. Candidate operating envelope

| Requirement | Candidate value | Qualification note |
|---|---:|---|
| Port A nominal | 48 V DC | Configuration value |
| Port B nominal | 48 V DC | Configuration value |
| Port operating range | 36–60 V DC | Verify every topology at both ends |
| Continuous transferred power | 500 W | Thermal equilibrium required |
| Power flow | A→B and B→A | Separate full maps and transient tests |
| Galvanic isolation | Yes | Working voltage, impulse withstand, creepage, clearance and test method must be derived from the applicable use case/standard |
| Ambient | 25 °C nominal lab baseline | Add hotter/cooler mission profiles later |
| Cooling | Passive and forced-air candidates | Include fan/auxiliary power in system efficiency |
| Control | Local deterministic controller | Supervisory AI excluded from real-time actuation |
| Protection | Independent hardware limits and DC-rated disconnect/fuse | Exact device selection follows fault/energy calculations |

These values are provisional engineering requirements, not certified ratings or claims that a prototype
already operates over this envelope. The 36–60 V DC class can carry hazardous current and stored
energy. Current limiting, overcurrent protection, guarded enclosures, safe discharge and controlled
test procedures remain mandatory.

## 3. Topology candidates

### Candidate A — dual-active bridge (DAB)

Two active bridges exchange energy through a high-frequency transformer. Phase shift controls
transferred power and the topology naturally supports bidirectional flow and galvanic isolation.

**Investigate**
- Single-, dual- and extended-phase-shift modulation.
- Transformer turns ratio, leakage inductance and magnetizing current.
- ZVS range and circulating-current penalty at light load.
- RMS current, conduction loss and switching loss in both directions.
- Dead-time sensitivity, reverse recovery / output capacitance effects and gate-drive margins.
- Transformer core loss, winding loss, proximity/skin effects and insulation.
- Start-up, pre-charge, current ramp and reversal behavior.

**Risks / trade-offs**
- Light-load circulating current can erase peak-efficiency gains.
- Magnetics and leakage-inductance design are central to performance.
- Current stress can be high at low-voltage ports.
- More switches, gate drivers and control states increase failure and validation surface.

**Why retain it**
DAB is a natural reference for modular isolated bidirectional conversion and has strong relevance
to modular SST DC/DC building blocks.

### Candidate B — bidirectional CLLC / resonant converter

A resonant isolated topology may improve soft switching and efficiency over a selected operating
range, but control, component stresses and bidirectional symmetry require careful analysis.

**Investigate**
- Frequency range required to cover 36–60 V ports.
- Resonant tank sensitivity to tolerance and temperature.
- Soft-switching region in forward and reverse power flow.
- Circulating current, magnetizing current and light-load losses.
- Start-up, burst/skip operation, reversal and fault interruption behavior.
- Magnetic and capacitor stress under worst-case tolerance.

**Risks / trade-offs**
- Frequency-control range may be wide.
- Parameter drift and tolerances can move the converter outside the desired soft-switching region.
- Control and fault behavior may be less intuitive than phase-shift DAB.
- A high headline efficiency at one point may hide poor corners.

### Reference Candidate C — four-switch non-isolated buck-boost

Use as a reference for efficiency, component count, BOM, control complexity, size and repairability.
It does **not** satisfy the isolation requirement and must never be presented as the selected isolated
product if the requirement remains.

## 4. Fair topology comparison

All candidates must use the same:
- Port-voltage operating envelope and output-power boundary.
- Forward and reverse operating points.
- Switching/thermal ambient assumptions and cooling boundary.
- Semiconductor device availability and realistic gate-drive assumptions.
- Transformer or inductor loss models with source and uncertainty.
- Measurement boundary, including control, gate-drive, fan and auxiliary consumption.
- Fault and transient requirements.
- Volume, mass, service and BOM-cost accounting rules.

Do not compare a simulated ideal topology against a measured commercial product without labelling
the evidence mismatch. A model with ideal switches, ideal magnetics or zero parasitics is a
screening model, not a competitive result.

## 5. Functional architecture

1. **Port A interface:** DC-rated connector, polarity/keying, local voltage/current sensing, fuse or
   coordinated protection, controlled connection and safe discharge.
2. **Port B interface:** same functions, sized for the declared direction and fault envelope.
3. **Power stage:** removable subassembly with revisioned semiconductor, magnetics, capacitor and
   thermal configuration.
4. **Gate drive:** hardware enable/disable, desaturation/overcurrent response as appropriate to the
   selected device, gate-voltage monitoring and safe defaults.
5. **Local controller:** current/voltage loop, soft start, bounded reference handling, watchdog and
   state machine. The controller cannot defeat hard hardware limits.
6. **Independent protection:** separate analog/hardware path where practicable; hardware gate
   inhibit; DC-rated interrupting means; fault latch and manual/explicitly authorized reset.
7. **Sensing:** calibrated voltage/current and temperature channels with range, bandwidth, offset,
   uncertainty and calibration state attached.
8. **Thermal system:** conduction path from semiconductor to baseplate/heatsink; sensor locations
   near identified hot spots; passive/forced-air comparison.
9. **Service interface:** keyed replaceable power stage, accessible fasteners, replaceable fans and
   control board, clear discharge state and post-repair test requirements.
10. **Evidence interface:** immutable build identity, BOM and lot references, firmware digest,
    calibration records, test-run IDs and repair events.

## 6. Safe-state model

The default safe state is **power transfer inhibited, gate drives disabled, energy path isolated
where the hardware permits, and stored energy discharged through a designed path**. Exact behavior
depends on topology and must be specified before circuit design.

Required invariants:
- Main controller reset, crash, watchdog expiry or communications loss cannot hold gates enabled.
- Loss or corruption of a sensor value cannot be interpreted as permission to increase power.
- Overcurrent, overvoltage, overtemperature, desaturation or gate-driver fault causes a hardware
  inhibit and a latched fault according to the reviewed hazard analysis.
- A reversal command passes through a bounded ramp/zero-transfer state; it does not instantaneously
  swap sign at full power.
- Unknown post-fault state blocks re-energization until reconciliation and an authorized reset.
- Firmware cannot change protection thresholds outside the reviewed signed configuration envelope.
- Telemetry loss cannot disable local protection or create a remote bypass.

This is a design intent, not evidence that any circuit currently implements it.

## 7. Mechanical, thermal and manufacturing rules

- Treat the complete module (power stage, magnetics, capacitor bank, gate drive, control, cooling,
  enclosure and connectors) as the unit of efficiency and lifecycle accounting.
- Keep high-current loops short and symmetric; use layout parasitics in switching and EMI analysis.
- Route low-level sensing away from switching nodes and high di/dt loops.
- Make thermal interfaces measurable and serviceable; avoid permanent potting unless required by
  a documented environmental/safety case.
- Document conductor material, plating, torque, creepage/clearance and enclosure materials.
- Avoid single-source end-of-life components for non-specialized parts where practical.
- For each replaceable unit, record exact part number, manufacturer, revision, lot/date code,
  approved alternate and the tests required after replacement.
- Repair does not automatically preserve qualification: changes to magnetics, semiconductors,
  sensors, insulation, thermal interfaces or firmware invalidate affected claims until re-tested.

## 8. Open design questions

1. Is 500 W sufficient for the first demonstrator, or does the learning justify a lower-power
   bring-up board before the full unit?
2. Is the isolation requirement necessary for the first experiment, or should a cheap non-isolated
   baseline be built first while the isolated design remains the product target?
3. Which semiconductor families (low-voltage silicon MOSFET, SiC, GaN) actually minimize total
   losses/cost at this voltage and power? Do not assume SiC or GaN wins at low voltage.
4. What control/measurement hardware provides sufficiently fast, deterministic and observable
   protection without excessive standby draw?
5. Which magnetic materials and winding structures offer the best combined loss, temperature,
   partial-discharge/insulation margin and repairability?
6. Which local suppliers and second sources exist for semiconductors, magnetics, connectors,
   capacitors, cooling and test services in South Africa and internationally?
7. Which standards apply to the exact eventual product category and jurisdiction? A general power
   converter standard is not a substitute for the product-specific PV, UPS, EV or installation
   requirements.

## 9. Gate before schematic freeze

Do not freeze a schematic until the team has:
- A controlled requirement-to-test traceability table.
- At least two isolated topology options screened with non-ideal models.
- A component list with manufacturer datasheets, lifecycle, sourcing and derating.
- A preliminary switching/thermal/magnetic model with assumptions documented.
- A reviewed protection/safe-state architecture and energy/fault calculations.
- A preliminary insulation coordination and creepage/clearance review.
- A physical packaging and service concept.
- A measurement plan and uncertainty budget.
- A qualified engineer's review of the proposed laboratory procedure.

## 10. Explicit phase-0 exclusions

No connection to household mains, utility AC, a public DC network, grid-tied inverter, medium
voltage, high-voltage battery pack, EV traction battery or utility switchgear. No field installation.
No intentionally uncontrolled short circuit. Any later test that increases stored energy or fault
current requires a separate reviewed procedure, suitable enclosure, DC-rated disconnecting means,
and competent laboratory supervision.
