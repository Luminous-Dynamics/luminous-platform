# PF-HVDC-6K — 800 VDC to 50 V DCX research plan v0.1

**Snapshot:** 2026-11-10  
**Parent:** [Luminous Power Fabric #364](https://github.com/Luminous-Dynamics/luminous-platform/issues/364)  
**Status:** literature reproduction and system-design plan; no high-voltage circuit or prototype is claimed.  
**Safety class:** high-energy 800 VDC system; no hardware work or energization authorized by this document.

## 1. Why this is a separate programme

The 500 W PEBB-LV-01 work targets a low-voltage, isolated, bidirectional converter. The PF-HVDC-6K
research target is a very different device: a high-frequency, isolated, unregulated or tightly bounded
800 VDC-to-50 V DC transformer (DCX) for AI/data-centre power delivery. These architectures may share
mechanical, evidence, control and lifecycle contracts, but they must not share unreviewed power stages,
isolation assumptions, component ratings or safety procedures.

**Power level:** 6 kW. At 50 V output this corresponds to 120 A continuous output current. The very high
secondary current makes transformer termination, copper/PCB/busbar resistance, current sharing,
rectifier conduction and output interconnect major design concerns.

**Important distinction:** the published 800 V-to-50 V LLC/DCX prototypes are primarily unidirectional
fixed-ratio / unregulated converters. They are not bidirectional battery interfaces. A future
bidirectional 800 V-to-50 V product needs a separately justified topology and control design (for
example DAB/CLLLC with a true bidirectional active stage) and its own benchmark / safety case.

## 2. Literature frontier

| Reference | Published configuration / result | What to reproduce |
|---|---|---|
| Prakash & Li, APEC 2026, DOI 10.1109/APEC51134.2026.11517167 | 6 kW GaN LLC DCX, 800 V / 50 V, 98.75% peak, 2,070 W/in³ | Exact resonant topology, stacked / bridge structure, transformer construction, cooling, peak/full-load test boundary, full efficiency curve and density definition |
| CPSS TPEA 2026, DOI 10.24295/CPSSTPEA.2026.00003 | 800 V-to-50 V input-series/output-parallel half-bridge LLC, GaN and matrix transformer; 98.1% full-load, peak >98.4% | ISOP cell arrangement, voltage sharing, transformer design, regulation method, cooling and measurement setup |
| ST APEC 2026 technical presentation | 12 kW, 800 V-to-50 V, 16:1, 1 MHz GaN stages; ≥98% full-load target, liquid cooling | Component and architecture trade-offs; treat vendor performance target as a target, not independent qualification |
| OCP LVDC distribution v1.1.0, September 2026 | System architecture for ≤1,500 VDC distribution, storage, rack interface, startup/hot-swap, hold-up, system control/monitoring and EMC | Requirements and interoperability contract, not an individual converter topology |

Sources are listed in [SOURCES.md](./SOURCES.md). No single reference is a complete matched baseline. The
Prakash–Li abstract reports peak efficiency and power density; CPSS reports a separate full-load
figure; ST provides a high-power architecture target. Do not merge their results into one imaginary
competitor.

## 3. Research hypotheses

1. **Modular ISOP can improve voltage sharing and serviceability:** multiple lower-voltage cells may
   distribute primary voltage stress and enable fault isolation. The costs are cell current/voltage
   sharing, duplicated drivers/controls, isolation coordination and more failure modes.
2. **Matrix magnetics can raise power density:** an integrated multi-leg / multi-cell transformer
   can improve winding utilization and shorten secondary current paths. Termination loss, leakage,
   common-mode capacitance, insulation, thermal gradients and manufacturability become critical.
3. **High-frequency GaN enables smaller magnetics, not automatically lower total loss:** switching,
   gate-drive, dead-time, Coss, AC copper, core and termination losses must be optimized together.
4. **The output side should be designed around 120 A, not around a 50 V label:** busbar, connector,
   current-sharing and PCB/termination loss may dominate a very compact power stage.
5. **DCX and regulation should be separated deliberately:** an unregulated high-efficiency DCX plus
   downstream point-of-load regulation may outperform a fully regulated isolated stage in system
   efficiency, but only after the complete chain and dynamic load are measured.
6. **A serviceable multi-cell shelf could be a stronger differentiator than peak efficiency:** replaceable
   cells, independent monitoring, fault containment, offline evidence verification and rapid
   requalification could reduce downtime and lifetime cost.

## 4. Candidate architecture comparison

| Candidate | Primary advantage | Main engineering risk | First study |
|---|---|---|---|
| Single-stage half-bridge LLC DCX | Fewer conversion stages, compact resonant design | High step-down ratio, transformer/capacitor stress, limited regulation / gain range | Reproduce APEC paper equations, magnetics and efficiency boundary |
| Multi-cell ISOP LLC | Divides high-voltage input and enables modular scaling | Input voltage sharing, output current sharing, circulating currents, cell control and fault coordination | Reproduce CPSS paper and derive cell mismatch/fault model |
| Three-level / stacked LLC | Reduces per-device voltage stress and may improve switching-loss trade-offs | More switch nodes, gate-drive and control complexity, midpoint imbalance | Compare device stress and total losses at matched conditions |
| DAB / CLLLC bidirectional | True reverse power, active control and wide operating range | More circulating current, bridge/device count and control/soft-switching complexity | Keep as a separate future bidirectional product family, not a DCX benchmark |
| Hybrid / mechanical isolation or protection | May reduce conduction loss and allow physical isolation | More components and coordination; mechanical wear | Evaluate as system-level protection, not as converter topology replacement |

No candidate is selected by this plan. Topology down-selection requires matched non-ideal models and full
loss / thermal / protection trade-offs.

## 5. Requirements to freeze before detailed design

Do not freeze numeric electrical limits until the full papers and the OCP v1.1.0 document have been
read and the target facility / rack interface is known. The requirement register must include:

- Nominal input voltage, minimum/maximum steady-state voltage, transients, polarity and grounding.
- Output voltage tolerance / droop, current rating, transient response, ripple and load-step profile.
- Continuous output 6 kW and any overload duration; output current sharing across cells.
- Isolation working voltage, test voltage / impulse, partial discharge, creepage/clearance, altitude,
  pollution degree and insulation material group, derived from actual installation conditions.
- Primary / secondary common-mode capacitance, leakage current, dv/dt and EMI limits.
- Startup, pre-charge, output hold-up, discharge, restart and brownout behavior.
- Short circuit / overload, reverse current, cell fault, cell isolation and system protection coordination.
- Cooling medium, inlet temperature, flow / pressure envelope, leakage detection and coolant parasitics.
- Mechanical envelope, mass, connectors, busbar, vibration and service access.
- Firmware / monitoring, fault telemetry, communications loss and deterministic local control.
- Manufacturing yield, component second sources, obsolescence, repair and post-repair requalification.
- Jurisdiction, installation category, standards and facility/OCP interface requirements.

## 6. Benchmark protocol

### Efficiency map

Measure simultaneous input/output power with calibrated, synchronized instruments and a declared
measurement boundary. Include controller, gate drive, pump/fan, sensors and auxiliary supplies in
system efficiency. At minimum characterize the declared load range (e.g. 10%, 25%, 50%, 75%, 100%)
at low/nominal/high input voltage and cooling boundaries, with cold start and thermal equilibrium
separated. Do not publish peak efficiency without its input voltage, load, ambient, cooling, sample
count and uncertainty.

**Published references:**
- APEC 2026 6 kW 800 V-to-50 V LLC: 98.75% peak, 2,070 W/in³.
- CPSS 2026 800 V-to-50 V ISOP LLC: 98.1% full load, peak >98.4%.
- EPC91134 11 kW 800 V-to-50 V ISOP LLC: 98.0% at 220 A and 98.5% peak under forced-air cooling; approximately 4,800 mm² footprint (436 mm²/kW), as reported from the EPC presentation.

See [HVDC benchmark update, 10 November 2026](./HVDC_BENCHMARK_UPDATE_2026-11-10.md) for boundaries and source caveats. The EPC footprint metric must not be compared directly with APEC's volumetric W/in³.

**Revised proposed internal stretch objective (not an achieved result or frozen requirement):** at matched and fully documented conditions, aim for at least 98.5% at full load and 99.0% peak. First reproduce the selected 6 kW and 11 kW references and extract their full curves, cooling and measurement uncertainty. A better-than-baseline claim requires the difference to exceed combined expanded measurement uncertainty and independent reproduction. If a matched baseline cannot be reproduced, report "not comparable" rather than an SOTA ranking.

### Power density

Report separately:
- Bare converter assembly W/in³ and kW/L.
- Converter plus required cooling / enclosure W/in³ and kW/L.
- Complete serviceable shelf including connectors, protection and coolant manifold.
- Mass density W/kg at each of the same boundaries.

Never compare a bare-board research number to a shelf with coolant / busbar / protection included.

### Thermal and reliability

Measure semiconductor case/junction estimate, transformer winding/core, capacitor, termination,
connector and busbar temperatures. Report actual ambient/coolant, flow, orientation, thermal
equilibrium criterion, measurement uncertainty and hot-spot location. Then plan power cycling,
thermal cycling, vibration, humidity, connector mating, coolant leak and service tests on a
mission-profile basis. Short-run efficiency is not a lifetime claim.

### Protection and interoperability

The qualified laboratory and system design must separately assess:
- Pre-charge / discharge and residual energy.
- Input and output DC fault energy and let-through current.
- Cell current/voltage sharing, isolation and backfeed.
- Selective protection and fault containment across parallel cells.
- Grounding / polarity / insulation monitoring.
- Startup sequence, hold-up and load transfer.
- EMC and common-mode behavior.
- Controller / communications failure and local safe-state behavior.

The DCX must not rely on AI, cloud, network or supervisory controller for safe interruption.
No hot-swap, fault injection, or energized repair is permitted without a separate engineered and
reviewed procedure in an appropriately equipped high-voltage facility.

## 7. Luminous system integration

| System | Responsibility |
|---|---|
| Symthaea | Reproduce literature, rank topology/semiconductor/magnetic alternatives, propose experiments |
| Symtropy | Switching, electrothermal, magnetic, control, cell-sharing and fault simulation; HIL plan |
| Sol-Atlas | Cell/shelf geometry, matrix transformer package, busbars, cooling manifold, assembly/service and BOM revision |
| Mycelix | Device/lot provenance, hardware/software identity, calibration, test records, commissioning and repair evidence |
| Embedded control | Deterministic local regulation, current sharing, startup and cell state machine |
| Independent protection | Hardware limits, pre-charge/discharge, cell isolation, fault interruption and safe de-energization |

Do not build a second fleet manager, telemetry database, component ledger or lifecycle system. Reuse
existing Luminous platform services and preserve clear authority separation.

## 8. Work packages

- HV-001 — retrieve and extract full APEC 6 kW paper and CPSS ISOP paper; build an evidence table.
- HV-002 — reproduce the APEC topology and efficiency map boundary from published design details.
- HV-003 — reproduce the CPSS ISOP cell model, voltage/current sharing and matrix magnetics.
- HV-004 — create matched non-ideal switching/loss models for both candidates.
- HV-005 — build transformer / termination / busbar electrothermal models.
- HV-006 — model pre-charge, startup, output hold-up, reverse current and DC fault containment.
- HV-007 — compare GaN, silicon and candidate magnetic / capacitor / interconnect choices using
  actual source data and derating.
- HV-008 — create package and serviceable shelf digital thread / BOM in Sol-Atlas.
- HV-009 — define OCP v1.1.0 interface / interoperability test matrix.
- HV-010 — prepare an accredited-lab and high-voltage facility partner brief.

## 9. Explicit exclusions and claim ceiling

- No 800 VDC circuit, PCB, firmware, magnetic part or prototype is currently built or approved.
- No high-voltage or high-energy hardware test is authorized by this plan.
- No bidirectional capability is implied by the unidirectional LLC/DCX reference designs.
- No published result is claimed by Luminous.
- Simulation, analytical screens, paper reproduction and software checks remain separate from
  physical measurements, accredited testing and deployment approval.
- A prototype is not a certified rack supply, data-centre product or utility interconnection device.
