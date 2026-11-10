# Luminous Power Fabric product-family architecture v0.1

**Snapshot:** 2026-11-10  
**Status:** strategic architecture proposal; no product hardware or system rating is claimed.  
**Parent:** [Luminous Power Fabric #364](https://github.com/Luminous-Dynamics/luminous-platform/issues/364)

## 1. Strategic thesis

Do not turn PEBB-LV-01 into a one-off 500 W converter. Establish a shared engineering and evidence
platform for several physically distinct power-conversion families:

1. Low-voltage isolated / non-isolated DC conversion.
2. High-voltage DC distribution conversion for data-centre and commercial facilities.
3. Bidirectional grid-connected conversion and microgrid interfaces.
4. DC protection, isolation and energy-routing subsystems.
5. Later medium-voltage AC/DC and solid-state-transformer systems, with specialist partners.

The commonality is the **module contract, configuration identity, test evidence, protection coordination,
service and lifecycle model**—not a presumption that one semiconductor bridge or PCB scales across
all voltage classes.

## 2. Product family map

| Family | Candidate product | Interface / application | Engineering maturity target |
|---|---|---|---|
| PF-LV | PEBB-LV-01 | 36–60 V-class, isolated, bidirectional, 500 W laboratory DC/DC building block | First design and measurement programme |
| PF-48 | 48 V power conversion module | Data-centre / telecom / industrial 48 V buses; fixed-ratio, regulated or bidirectional variants | Product discovery and benchmark study |
| PF-HVDC | 400–800 VDC to 48 V conversion building block | Facility sidecar, rack power shelf, high-density compute | Research and partner architecture; separate high-voltage qualification |
| PF-ENERGY | Multiport storage / source / load interface | DC microgrids, storage emulator, PV and distributed generation | System simulation and HIL research |
| PF-GRID | Grid-connected bidirectional converter / grid-forming inverter | Commercial microgrids, storage, ancillary services | Grid-code and standards mapping before hardware product design |
| PF-PROTECT | DC breaker, hybrid breaker, pre-charge, isolation and sectionalizer | DC buses, module branches and rack distribution | Dedicated fault-energy and protection programme |
| PF-SST | Medium-voltage AC to LVDC solid-state transformer | Data hall and utility / campus DC supply | Long-term R&D; consortium and accredited-lab partnership |

### PF-HVDC-6K engineering entry point

The specific literature-reproduction and qualification work for an 800 V-to-50 V, 6 kW unidirectional
DCX is tracked separately in [PF-HVDC-6KW_RESEARCH_PLAN.md](./PF-HVDC-6KW_RESEARCH_PLAN.md).
Do not treat those LLC/DCX prototypes as bidirectional storage converters. A future bidirectional
800 V product requires a separate power-stage architecture, protection review and qualification.

## 3. Why 800 VDC is a distinct strategic opportunity

The Open Compute Project's Power Distribution sub-project defines its LVDC scope as systems up to
1,500 VDC. Its stated work includes backward-compatible LVDC distribution evolution, ESS, rack power
architecture, hot-swap / startup protection interface and hold-up requirements. It also states that
detailed component specifications are assigned to separate workstreams. The OCP Protection and Safety
workstream is now a distinct part of that programme.

NVIDIA describes a transition path from AC-fed racks toward 800 VDC, with rack-level power racks,
cluster-level power centres and data-hall-level DC power blocks. Schneider Electric's March 2026
white paper emphasizes architecture selection, ecosystem readiness, protection / grounding
coordination, storage integration and operational execution. These are credible industry directions,
not a frozen universal standard or one mandatory topology.

**Luminous opportunity:** offer an open, evidence-bearing implementation and integration layer that
can work with multiple suppliers' converters, busbars, storage and protective devices. The advantage
should be interoperable configuration, commissioning, protection coordination, service and evidence,
not a proprietary 800 V connector or a claim to replace OCP.

### PF-HVDC reference architecture (conceptual)

- Facility AC / MV intake and transformer / rectifier or approved SST.
- Protected LVDC distribution backbone (e.g. 800 V nominal architecture selected by the facility).
- Segmented DC bus with selective protection and independently verifiable isolation.
- Replaceable high-voltage DC/DC power blocks converting to a lower-voltage intermediate bus.
- Rack-level 48 V conversion and point-of-load conversion close to compute.
- Storage / ride-through interface with defined current limits and fault containment.
- Independent grounding, insulation monitoring, pre-charge, discharge, disconnect and emergency
  isolation design.
- Digital configuration and evidence plane that does not participate in the fast protection loop.

This is a reference decomposition only. It does not specify a safe working voltage, conductor system,
grounding topology, protection device, fault current, connector, or installation procedure. These
must be defined for the specific facility and jurisdiction by qualified engineers.

## 4. Power-fabric paradigm shifts

### 4.1 Standardize contracts, not circuit boards

Define electrical-port, mechanical, thermal, control, protection, telemetry, service and evidence
contracts. A 48 V converter, 800 V isolating stage and microgrid inverter can share lifecycle and
qualification semantics without sharing hardware.

### 4.2 Move from converter efficiency to infrastructure efficiency

Measure AC-in to IT-load or source-to-load delivered energy, including every conversion stage,
transformers, busbars, cooling, standby, storage round-trip loss, power factor and availability.
Do not assume that replacing AC distribution with DC always saves energy: compare the complete
facility and mission profile.

### 4.3 Move from monolithic power supplies to replaceable power cells

Separate power-stage, magnetics, control, sensing, cooling and protection where practical.
A failed module should be isolatable and replaceable without scrapping healthy assemblies. The
system must prove that redundancy and parallel operation do not create unsafe backfeed paths.

### 4.4 Make protection a coordinated system property

DC protection is not just a faster fuse. Coordinate source capacitance, cable inductance, bus
segmentation, grounding, fault detection, semiconductor interruption, mechanical isolation,
pre-charge, discharge and backup energy. Test worst-case fault energy and let-through at the
system level. AI, cloud and communications are not required for safe interruption.

### 4.5 Separate power control from supervisory intelligence

Local deterministic control regulates converter dynamics. Independent hardware and qualified
protection constrain hazardous states. Symthaea can optimize engineering, identify faults and
propose new configurations, but cannot directly command gate timing or override protective limits.

### 4.6 Make equipment evidence-native

Every installed unit should bind component lots, configuration, firmware, calibration, commissioning,
fault events, repairs and requalification to a stable product identity. Evidence reuse must be scoped:
changing semiconductors, magnetics, insulation, firmware or thermal interfaces may invalidate
different classes of prior evidence.

### 4.7 Design for second sources and lifecycle continuity

Track semiconductor / magnetic component obsolescence, approved alternates, repairability, spare
availability, local manufacturing capability and end-of-life recovery. A module is not future-proof
because its communication protocol is open if its critical power devices cannot be sourced.

## 5. Competitive position and product wedge

Do not start by competing head-to-head with utility-scale SSTs or 800 V rack power shelves. Those
markets require capital, suppliers, certification and facilities beyond a first low-voltage prototype.

A more credible entry sequence is:

1. Build the engineering/evidence tooling and prove a small isolated bidirectional module design
   through independent low-voltage bench measurements.
2. Compare a modular 48 V family with current commercial references in matched operating conditions.
3. Partner with a facility or power-systems integrator to validate a rack/sidecar subsystem at a
   properly equipped laboratory.
4. Extend the evidence, protection-coordination and lifecycle platform to higher-voltage assemblies.
5. Only later undertake an SST demonstrator with semiconductor, magnetics, utility and accredited
   laboratory partners.

This is a way to earn access to the larger market rather than claiming to compete with a multi-billion
dollar industrial portfolio on day one.

## 6. Standards and jurisdictional routing

The product category determines the standards route:

- Bidirectional grid-connected converter: assess IEC 62909-1:2025 where its scope fits.
- PV power conversion: assess IEC 62109 series; IEC 62909-1 explicitly excludes PV conversion equipment.
- UPS: assess IEC 62040 series; IEC 62909-1 explicitly excludes UPS systems.
- EV charging / battery energy transfer: assess IEC 61851 series; IEC 62909-1 excludes EV battery charge/discharge converters.
- DC distribution and power electronics: assess applicable IEC 62477-1 and product-specific requirements,
  plus installation, wiring, EMC, insulation and local rules.
- South African SSEG / grid-connected generation: determine the applicable NRS 097 requirements,
  local distributor rules and accredited type-test / registration pathway for the actual installation.

This is a routing list, not a complete compliance matrix. Obtain the applicable standards in full and
use a competent conformity-assessment specialist before product design freeze.

## 7. Luminous system roles

| System | Role |
|---|---|
| Symthaea | Source-backed design hypotheses, topology and part trade-offs, mission-profile optimization |
| Symtropy | Electrical / thermal / magnetic / mechanical models, fault replay, HIL scenario and model correlation |
| Sol-Atlas | Product geometry, PCB and enclosure revision, wiring, clearances, BOM, assembly / service digital thread |
| Mycelix | Component and lot provenance, authorization, instrument calibration, test evidence and repair records |
| Embedded control | Deterministic converter regulation and bounded operating state |
| Independent protection | Hardware current / voltage / thermal constraints, isolation, safe discharge and fault interruption |

Reuse existing platform identity, lifecycle, evidence and authorization infrastructure. Do not build a
second fleet manager or general-purpose evidence ledger for this programme.

## 8. Non-goals and safety gates

- No 400/800 VDC hardware prototype is authorized by this architecture document.
- No public mains, medium-voltage, utility, live rack or field testing.
- No energized hot-swap, dynamic bus reconfiguration or autonomous fault clearing.
- No assumption that DC voltage class alone determines shock/arc safety.
- No certification or grid-code compliance claim.
- No universal "solid-state is better" claim; hybrid switching, conventional transformers and
  mechanical isolation remain eligible when lifecycle and protection evidence favour them.
