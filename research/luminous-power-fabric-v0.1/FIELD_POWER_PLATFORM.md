# Portable Field Power Fabric — product architecture and qualification brief

**Snapshot:** 2026-11-12  
**Status:** systems research proposal; no field unit, power rating, certification or performance result is claimed  
**Parent programme:** [Power Fabric #364](https://github.com/Luminous-Dynamics/luminous-platform/issues/364)  
**Field-system work:** [#413 — portable industrial field-power router](https://github.com/Luminous-Dynamics/luminous-platform/issues/413)  
**Converter foundation:** [draft architecture PR #365](https://github.com/Luminous-Dynamics/luminous-platform/pull/365)

## 1. Product thesis

Portable industrial power should be treated as a **field-deployable, modular power system**, not as a battery with an inverter attached.

The system must coordinate energy sources, conversion, storage, distribution, loads, local protection, thermal behavior, physical deployment and maintenance. A good nominal kW/kWh specification says very little about whether the system can start a motor, ride through a source change, isolate a DC fault, survive dust and vibration, or be safely repaired at a remote site.

Luminous should pursue a repairable **power router** whose distinctive value is:

1. Interoperable converter and protection building blocks.
2. Source-to-load energy accounting and useful-service measurement.
3. Protection coordination and independently verified safe isolation.
4. Mission-profile reliability and thermal derating.
5. Repairable modules and configuration-bound qualification.
6. Offline-capable local operation with an auditable evidence plane.

The first product should be narrower than the long-term platform. It should integrate proven, appropriately qualified batteries, generator sets, inverters and distribution equipment where doing so reduces risk and development cost. Luminous can differentiate through its own modular interfaces, low-voltage conversion/protection building blocks, control contracts, instrumentation, diagnostics, service architecture and evidence.

## 2. Industry baseline and what it means

Two commercial product families make the power-and-energy scale concrete:

| Comparator | Published specification | Implication for Luminous |
|---|---|---|
| Caterpillar XES120, 50 Hz | 72 kW / 90 kVA; 127.9 kWh; IP55; 400 V-class mobile ESS; designed to hybridize with generators and renewable sources | A portable field node must be benchmarked against actual hybrid generator operation and the load boundary—not just battery capacity |
| Atlas Copco ZBC family | Published range includes 250–1,000 kVA and 246–1,200 kWh; 400 V AC; LiFePO4 container systems | Large construction/industrial BESS is an established product category; competing on containerized kWh alone is not a defensible entry point |

Sources:
- https://www.cat.com/en_ZA/products/new/power-systems/electric-power/battery-energy-storage-systems/119844.html
- https://www.atlascopco.com/en-za/construction-equipment/products/energy-storage-systems/medium

Solid-state protection is also a commercial market, not a blank slate. ABB's SACE Infinitus combines semiconductor interruption with electromechanical isolation and publishes ratings up to 2,500 A / 1,250 V DC with interruption below 25 μs. Siemens announced semiconductor breaker and solid-state switching products for DC distribution in April 2026.

Sources:
- https://www.abb.com/global/en/areas/electrification/low-voltage/circuit-breakers/solid-state-circuit-breakers/sace-infinitus
- https://press.siemens.com/global/en/pressrelease/siemens-launches-groundbreaking-portfolio-era-direct-current-technology

**Competitive conclusion:** a new Luminous field-power product should not be positioned as “faster breaker”, “more efficient inverter” or “more battery”. The differentiator to test is a modular system that can demonstrate lower lifecycle cost per verified useful kWh, lower fuel consumption on a matched duty cycle, better recoverability, faster safe repair and stronger evidence quality without weakening any electrical safety or quality requirement.

## 3. Product archetypes

These are discovery envelopes, not frozen ratings or launch commitments.

### PF-FIELD-PORTABLE — service and instrument node

Target setting:
- Commissioning, diagnostics, temporary control rooms, remote instrumentation, communications, lighting and modest tools.
- Cart, case or small skid deployment by a small crew.
- Low-voltage DC ports for auxiliary equipment; AC output is a separately qualified interface.

Potential product boundary:
- A 48 V-class, isolated auxiliary conversion module based initially on PEBB-LV-01.
- Independently protected output branches with current, voltage and temperature observability.
- Replaceable power, sensing, cooling and control subassemblies.
- External, qualified source or battery/inverter integration for the first system prototype.

This is the most credible first learning platform because it can exercise the PEBB contract at low energy before a product takes on mains, high-energy storage or site-interconnection risk. A 500 W module is not a substitute for a full worksite supply.

### PF-FIELD-MOBILE — towable or forkliftable hybrid node

Target setting:
- Construction and civil-engineering sites.
- Industrial shutdowns and maintenance.
- Pumps, lighting, tools, smaller process loads and controlled motor-start applications.
- Temporary supplies where generator-only operation is fuel-inefficient or noisy.

System building blocks:
- Qualified battery subsystem selected from existing industrial products.
- Generator input with explicit synchronization / transfer design and a bounded operating envelope.
- Optional PV or other source input after source-specific compatibility and safety review.
- Modular AC/DC and DC/DC conversion.
- Construction-site assembly and protected branch distribution.
- Local source/load dispatch, power-quality monitoring and event logging.

Do not assume that a 48 V module can be scaled to tens of kW by paralleling boards. At 48 V, 5 kW implies about 104 A before losses; 20 kW implies about 417 A. Current sharing, busbar impedance, connectors, thermal paths, fault energy and protection coordination become first-order design problems. Any module scaling or parallel operation requires separate qualification.

### PF-FIELD-PLANT — temporary plant support and outage node

Target setting:
- Temporary auxiliary supply, commissioning and maintenance loads.
- Non-safety-critical pumps, fans, lighting, workshop supply, instrumentation and communications where permitted by the plant owner.
- Remote or constrained sites requiring planned temporary power.

This is a later integration product, not a starter prototype. Plant auxiliary networks, emergency systems, safety instrumented systems, switchyards, protection relays and hazardous-area equipment have different engineering and permit requirements. No interface to a plant protection system, switchyard or safety-critical bus is implied. No hazardous-area use is in scope without the complete applicable Ex design and certification.

## 4. Reference architecture

```text
                 ENERGY SOURCES
       Grid / Generator / PV / Qualified BESS
                       |
           Source-specific protection
                       |
        +--------------+---------------+
        |      POWER CONVERSION         |
        |  AC/DC  | DC/DC | Inverter    |
        |  isolated and non-isolated    |
        +--------------+---------------+
                       |
             Segmented DC / AC buses
                       |
          Protection + verified isolation
                       |
         +-------------+--------------+
         |             |              |
       Branch A      Branch B       Branch C
       motor         tools          controls
       / pump        / lighting     / comms
         |             |              |
       Load-side measurement and quality
                       |
         Independent measurement / evidence

 Supervisory planning, HMI and remote telemetry
       (not in the local protection loop)

 Local deterministic controls + hardware trips
       (remain safe with network/AI loss)
```

A product configuration must explicitly define:
- Source voltage, frequency, fault level, grounding and transfer behavior.
- Bus voltage class, polarity, grounding and insulation monitoring.
- Direction of power flow and any reverse-energy paths.
- Each port's continuous, overload and transient limits.
- Pre-charge, discharge, disconnect and maintenance states.
- Protection zones, trip selectivity and backup interrupting means.
- Thermal, acoustic, enclosure and service envelopes.
- What happens when a source, controller, sensor, fan, network or power module fails.

The digital supervisor observes, schedules and recommends. Local deterministic controllers regulate within a reviewed mode. Independent hardware protection and appropriately rated interrupting/isolation equipment constrain hazards. No AI, cloud or network function may be required to trip, isolate or reach a safe state.

## 5. Power-electronics component roadmap

| Component family | First engineering question | Candidate technology direction | Qualification boundary |
|---|---|---|---|
| Isolated LV DC/DC | Which topology offers the best two-way efficiency and full-envelope thermal behavior? | DAB vs bidirectional CLLC; silicon and GaN comparison; use the existing PEBB-LV-01 study | 36–60 V, 500 W current-limited lab demonstrator only |
| LV DC solid-state power controller | Can a protected branch switch detect and contain a declared fault without unsafe failure modes? | Silicon MOSFET / GaN where appropriate; compare semiconductor-only against hybrid interruption plus mechanical isolation | Separate fault-energy, fail-short, thermal and isolation study before any prototype |
| DC bus protection and isolation | How are source capacitance, wiring inductance, fault current and branch selectivity coordinated? | Fuse, contactor, hybrid breaker and commercial SSCB baselines | System protection study and specialist review; no breaker rating from desk research |
| AC/DC active front end | Can a source interface control power factor and harmonics while meeting site requirements? | Commercial qualified inverter/rectifier first; later SiC/GaN candidate comparison | Product-specific grid / installation standards and accredited testing |
| Bidirectional inverter / grid interface | Which functions are genuinely needed: grid-following, islanding, black start or grid-forming? | Buy qualified products for early field system; Luminous control/evidence adapter first | Site owner / utility permissions and applicable grid-code qualification |
| Thermal and magnetic assemblies | Can magnetics, cooling and packaging be repaired and independently requalified? | Modular transformer / inductor / cold-plate / fan assemblies | Material, insulation, thermal-cycle, vibration and manufacturing-process evidence |
| Protection coordination controller | Can the system maintain deterministic state and safe recovery across source and module failures? | Simple local finite-state control; supervisory optimization outside protection loop | Hardware-in-loop and physical protection tests; software cannot replace protective devices |

**Semiconductor selection is voltage- and mission-profile-specific.** Low-voltage silicon MOSFETs and GaN deserve serious comparison for the 48 V-class work. SiC is more naturally investigated for higher-voltage, higher-power conversion stages. Do not select a device by material label or headline switching frequency; compare conduction and switching losses, dynamic RDS(on), Coss, dead time, parasitics, thermal impedance, package reliability, gate drive and availability.

A fully solid-state device may have very fast interruption but incur conduction and thermal loss, and semiconductor failure can be short-circuit. Hybrid architectures can retain mechanical isolation and lower conduction losses. Luminous should make hybrid protection a first-class comparator, not a fallback.

## 6. Product-level differentiators worth testing

### 6.1 Capability-bounded power ports

Each source, converter, bus and load port should publish a versioned capability contract:
- Voltage and frequency range.
- Continuous and time-limited power/current.
- Power-flow direction.
- Ramp rate, response time and duration.
- Inrush / overload envelope.
- Dynamic behavior and allowed source impedance.
- Grounding / isolation / fault-current assumptions.
- Thermal derating and environmental limits.
- Protection and isolation semantics.
- Measurement quality, configuration identity and evidence expiry.

The system composes only compatible ports. Unknown or stale capability evidence means the port is unavailable for new dispatch.

### 6.2 Protection coordination as a system property

For each fault zone, record the upstream and downstream protective devices, current/energy contribution from each source, detection path, interruption path, backup device, isolation state and recovery authority.

Test:
- Bolted and resistive faults within a declared test envelope.
- Faults in both directions on bidirectional buses.
- Switch failure short and failure open.
- Sensor open, short, polarity reversal and drift.
- Loss of control power, communications and cooling.
- Source transfers and load rejection.
- Residual capacitor / battery / cable energy after shutdown.

Microsecond switching time is not a complete safety metric. The comparison must include peak let-through current, I²t, voltage overshoot, semiconductor stress, backup-clearing behavior, isolation state and safe maintenance evidence.

### 6.3 Workload and source-aware dispatch

Dispatch should minimize lifecycle cost and fuel/emissions while preserving:
- Required load service and priority.
- Source operating envelopes.
- Battery limits and degradation budget.
- Generator minimum loading and start/stop constraints.
- Power-quality requirements.
- Recovery reserve and future mission obligations.

Do not report fuel savings unless generator-only and hybrid cases use matched loads, timing, ambient conditions, fuel measurement and energy delivered at the same load boundary.

### 6.4 Repair-first design

Design around field-replaceable functional assemblies:
- Power stage.
- Magnetics.
- Cooling.
- Sensors.
- Control.
- Protection / isolation.
- Connectors and busbar assembly.

Record serial identity, BOM, supplier/lot, firmware, configuration, calibration, repair parts, torque/process records and retest evidence. A repair invalidates only the claims affected by the changed assembly if the dependency graph can justify that boundary; otherwise requalify the larger system.

### 6.5 Offline-first operation

The local unit must retain its safe operating mode and protection if cellular, Wi-Fi, telemetry, cloud, supervisory AI or fleet management is unavailable. Remote commands are bounded requests, not protection authority. The equipment must have a local, understandable state indication and a documented safe shutdown/maintenance process.

## 7. Benchmark programme

All results must be measured on a frozen configuration and complete boundary. The following is a test matrix, not a claim that any Luminous system has passed it.

| Benchmark | Measurement protocol | Required reporting |
|---|---|---|
| Converter efficiency | 10 / 25 / 50 / 75 / 100% load; both directions where applicable; full input-voltage range; declared ambient and cooling; calibrated simultaneous power measurements | Efficiency map, uncertainty, standby/auxiliary power, thermal equilibrium, losses by component |
| System efficiency | Source terminal to load terminal; include inverter, conversion, busbar, cooling, controls and storage cycling | Net kWh in/out, storage SOC at start/end, energy delivered and unserved |
| Generator hybridization | Paired generator-only and hybrid runs with identical load traces and environmental conditions | Litres per delivered kWh, fuel total, engine run hours, start/stop count, emissions method and uncertainty |
| Power quality | Declared resistive, nonlinear, motor and mixed loads | Voltage/frequency deviation, THD, PF, unbalance, transient recovery, motor-start success |
| Dynamic response | Load step, motor inrush, load rejection, source loss and transfer | Time-aligned waveforms, overshoot/undershoot, recovery time, unserved energy |
| Protection | Fault-injection matrix with rated test fixtures / facilities selected by specialist | Detection and interruption time, peak current, I²t, voltage stress, selectivity, backup clearing, isolation verification |
| Thermal/environmental | Hot/cold soak, dust/water ingress, vibration, shock, humidity, corrosion, blocked airflow and altitude as applicable | Derating map, hot-spot temperatures, ingress/vibration evidence, post-stress functional retest |
| Reliability | Repeated start-stop, load and ambient cycles; mission-profile semiconductor power cycling | Failure modes, confidence bounds, availability, repair frequency; no unsupported lifetime extrapolation |
| Field service | Controlled technician task studies on de-energized equipment | Safe isolation success, MTTR, module replacement time, tools/spares, error rate, requalification scope |
| Lifecycle economics | Frozen utilization, energy prices, fuel, storage replacement and maintenance assumptions | TCO per verified useful kWh, sensitivity range, embodied material and end-of-life treatment |
| Evidence integrity | Configuration and instrument records bound to raw time-series data | 100% source, serial/BOM, firmware, calibration, timebase, procedure and result lineage |

Suggested initial qualification gates:
- No unsafe state is reachable by any predeclared single fault or negative control in the reviewed Stage 1 bench model.
- 100% of performance runs have a known hardware revision, firmware/configuration identity, calibrated measurement chain and raw-data reference.
- Measured efficiency comparisons target combined uncertainty no worse than 0.2 percentage points where a 0.5–1.0 point difference is being interpreted. Otherwise disclose the actual uncertainty and do not rank close results.
- Every safety and power-quality gate passes independently; average scores cannot compensate for a failed hard gate.
- All failed, aborted, unknown, skipped and unrun tests remain visible in the results.

Do not select an efficiency uplift, runtime, MTTR or fuel-saving percentage until the anchor duty cycle and matched incumbent are frozen. Once frozen, preregister a minimum detectable improvement and confidence method before physical runs.

## 8. Standards and test-facility map

Applicability must be checked by product configuration and jurisdiction. Obtain licensed standards and qualified interpretation; this summary is not a compliance determination.

- **IEC 61439-4:2023** — assemblies for construction sites, including transportable/mobile low-voltage assemblies.
- **IEC 60364-7-704:2017** — construction and demolition site electrical installations.
- **IEC 61439-2:2020** — power switchgear and controlgear assemblies.
- **IEC 62477-1:2022** — safety requirements for power electronic converter systems up to 1,000 V AC / 1,500 V DC.
- **IEC 62619:2022** — industrial lithium cells and batteries if an integrated battery is selected.
- **IEC 62933-5-1:2024** and **IEC 62933-5-4:2026** — grid-integrated electrical energy storage safety and lithium-ion ESS test methods, where applicable.
- **IEC 60529** — enclosure ingress protection.
- **IEC 60068 series**, including relevant temperature, vibration, shock, humidity and corrosion methods. IEC 60068-2-64:2026 is the current random-vibration revision identified in this research snapshot.
- **IEEE 2800 / P2800.1** — relevant transmission-interconnected inverter capabilities; P2800.1 is a standards development project, not a product certification.
- **South Africa:** verify current SANS adoptions, OHS/Electrical Installation requirements, NRCS compulsory specifications and Letter of Authority applicability, accredited-laboratory routes, and site/utility/plant-owner approvals before sale or installation.

Primary references:
- IEC 61439-4:2023: https://webstore.iec.ch/en/publication/69748
- IEC 60364-7-704:2017: https://webstore.iec.ch/en/publication/33784
- IEC 61439-2:2020: https://webstore.iec.ch/en/publication/30043
- IEC 62477-1:2022: https://webstore.iec.ch/en/publication/28936
- IEC 62619:2022: https://webstore.iec.ch/en/publication/64073
- IEC 62933-5-4:2026: https://webstore.iec.ch/en/publication/67442
- IEC 60068-2-64:2026: https://webstore.iec.ch/en/publication/104354
- IEEE P2800.1: https://standards.ieee.org/ieee/2800.1/12387/
- South African NRCS LOA administration reference: https://www.nrcs.org.za/Documents/Electrotech/LOA%20Admin%20Procedure/LOA%20ADMIN%20Revised%2012_06%20May%202019.pdf

## 9. Manufacturing and supplier strategy

Do not attempt custom semiconductor fabrication or battery-cell manufacturing in the first cycle.

Use a staged make/buy split:
- **Buy initially:** qualified battery system, generator set, AC inverter/charger, grid interface, high-energy disconnects, enclosure, certified site distribution and safety-critical connectors.
- **Design / integrate:** low-voltage isolated DC/DC module, bounded local supervisory controller, measurement and evidence plane, modular diagnostics, port capability contracts and service/repair records.
- **Partner before scaling:** power module packaging, magnetics, thermal assembly, busbar fabrication, EMC and dielectric testing, accredited safety testing and contract manufacturing.
- **Consider in-house later:** repetitive low-voltage board assembly or modular enclosure work only when manufacturing process capability, traceability, test fixtures, repair economics and quality-system requirements are demonstrated.

Manufacturing release requires:
- Revision-controlled schematic, PCB, mechanical drawings, wiring and BOM.
- Second-source and obsolescence plan for critical parts.
- Supplier qualification and incoming inspection.
- Controlled assembly operations (e.g., torque, crimp, coating, insulation and thermal-interface process).
- End-of-line functional and dielectric tests appropriate to the product.
- Calibration and serialized results.
- Repair instructions and configuration compatibility matrix.
- Manufacturing yield and failure-analysis loop.

Design for repair must not undermine safety: no exposed live service access, no unreviewed hot swapping, no bypassable interlocks and no automatic restart after unresolved faults.

## 10. Integration into the Luminous engineering stack

| System | Role | Must not become |
|---|---|---|
| Symthaea | Literature synthesis, candidate architecture and decision-value-driven test selection; compose [SE-OPT-BENCH-001 #7803](https://github.com/Luminous-Dynamics/symthaea/issues/7803) | Safety certifier or real-time protection authority |
| Symtropy | Compose [POWER-SYSTEM-DIGITAL-TWIN-001 #3400](https://github.com/Luminous-Dynamics/symtropy/issues/3400), [POWER-SYSTEM-RESILIENCE-001 #3418](https://github.com/Luminous-Dynamics/symtropy/issues/3418), topology #3409 and matched-service FFER #3417 for generator/storage/load dynamics, fault replay, islanding, black-start and restoration | A substitute for calibrated physical tests |
| Sol-Atlas | Compose [ENERGY-GEN-PORTFOLIO-001 #293](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/293), [ENERGY-RESOURCE-SITE-001 #662](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/662) and [GMD-RESILIENCE-001 #692](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/692) for site, source, spatial exposure and dependency context; bind equipment/cable/busbar/connector and enclosure revisions | Proof that physical insulation or thermal design is qualified |
| Mycelix | Compose [MYC-BUILD-METROLOGY-002 #6034](https://github.com/Luminous-Dynamics/mycelix/issues/6034), [MYC-BUILD-UQ-001 #6048](https://github.com/Luminous-Dynamics/mycelix/issues/6048) and [MYC-BUILD-METROLOGY-003 #6042](https://github.com/Luminous-Dynamics/mycelix/issues/6042) for instrument capability, calibration, correlated uncertainty and dependency invalidation | An oracle of physical truth or a source of actuation authority |
| Local controller | Deterministic source/load state machine and bounded control loops | A network-dependent or AI-only protection system |
| Independent electrical protection | Hardware current/voltage/temperature limits and rated protective isolation | A function implemented only in application software |

The engineering loop should be:

`Mission profile → requirements → topology → model → design review → controlled prototype → calibrated test → independent review → qualified configuration → field monitoring → repair/requalification`

Each transition must preserve the evidence that supports the next claim.

## 11. Immediate research experiments

1. **Anchor-use-case tournament:** compare portable commissioning/service power, construction mobile power and plant-support power using safety as a hard gate and customer duty-cycle economics as the decision criteria.
2. **48 V current/thermal study:** quantify copper, connector, busbar and device losses at 0.5 / 1 / 2 / 5 / 10 kW. This will expose the scale boundary of the LV demonstrator rather than assuming it.
3. **Hybrid-source replay:** build matched profiles for a generator-only reference and a battery-generator hybrid; include low-load dwell, motor starts, step loads, idle and recovery.
4. **Protection topology tournament:** compare fuse + contactor, semiconductor branch controller, and hybrid breaker on declared fault-energy cases; no live high-energy test before specialist review.
5. **Field reliability mission profile:** translate dust, vibration, heat, humidity, transport, repeated connections and technician repair into explicit environmental and lifecycle test profiles.

## 12. Claim and safety boundary

This document is a systems research proposal. It does not authorize energizing hardware, building a mains or grid-connected prototype, connecting a battery pack, entering a power plant, or installing equipment at a construction/industrial site.

Evidence must be labelled as one of:
- Literature-derived.
- Analytical / simulated.
- Software-checked.
- Bench-measured under a reviewed low-energy test plan.
- Independently reproduced.
- Accredited / certified.
- Supervised field-validated.

Do not promote a claim up the evidence ladder merely because code is committed, CI is green, a model converges, or a vendor publishes a specification.
