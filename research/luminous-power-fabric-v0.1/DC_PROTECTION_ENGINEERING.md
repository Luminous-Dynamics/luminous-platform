# DC protection engineering and energy-envelope research v0.2

**Snapshot:** 2026-11-10  
**Parent:** [Luminous Power Fabric #364](https://github.com/Luminous-Dynamics/luminous-platform/issues/364)  
**Implementation branch:** `research/luminous-power-fabric-protection-v0.2`  
**Status:** desk research plus offline analytical screening code; not a protection design or safety qualification.  
**High-energy boundary:** no 800 VDC hardware, PCB, switching circuit, prototype, lab procedure, or energization is specified or authorized here.

## 1. Why this work is the next critical path

The industry transition is not just a change in converter input voltage. Open Compute Project (OCP) describes a 800 VDC ecosystem being aligned by Google, Microsoft, NVIDIA and other participants. It explicitly identifies two deployment paths:

1. A rack-side power rack converting existing 480 VAC to either +/-400 VDC or 0–800 VDC.
2. A longer-term direct MVAC-to-800 VDC facility architecture using high-density conversion blocks.

OCP describes the 800 VDC backbone as a possible integration point for battery energy storage, DC UPS functionality and future DC microgrids. Its Power Distribution programme published LVDC white paper v1.1.0 in September 2026, and Protection and Safety is a distinct workstream. This is an industry architecture and requirements convergence—not evidence that there is one finished, universally adopted 800 VDC implementation.

**Architecture implication:** the Power Fabric must model grounding/reference and polarity explicitly. +/-400 V and 0–800 V are not interchangeable labels. The port contract must identify the voltage between conductors, each conductor's relationship to protective earth, insulation/monitoring assumptions, and the applicable disconnect/protection arrangement. Never infer this from a nominal "800 V" label.

References:
- OCP, [Powering the Next Era of AI](https://www.opencompute.org/news/powering-the-next-era-of-ai-how-google-microsoft-and-nvidia-are-standardizing-and-accelerating-the-industry-transition-to-lvdc/).
- OCP, [Power Distribution](https://www.opencompute.org/projects/data-center-facility/power-distribution/).
- OCP, [Protection and Safety workstream](https://www.opencompute.org/projects/data-center-facility/power-distribution/protection-and-safety/).

## 2. 800 VDC is not a "safe low-voltage" product category

OCP uses LVDC as a system-family label for DC distribution up to 1,500 VDC. That nomenclature does not make an 800 V bus touch-safe or low-energy.

Useful scale checks:

- 6 kW at 800 V is 7.5 A at the nominal bus, before losses.
- 6 kW at 50 V is 120 A, before losses.
- A 1 mF capacitor bank charged to 800 V stores 320 J.
- The same 1 mF at 50 V stores 1.25 J: stored energy scales with voltage squared.

The capacitor example is an energy arithmetic check, not an incident-energy or arc-flash calculation. A real fault boundary can also include converter-fed source current, batteries, cable/bus inductance, parallel capacitors, rotating equipment, upstream supply and other coupled sources.

Schneider Electric's July 31, 2026 white paper studies DC arc-flash analysis for two 800 VDC AI data-centre architectures: rack-level sidecar and centralized distribution. Its 2026 technical commentary highlights that conventional arc-flash calculation methods were primarily developed for AC and do not fully capture time-dependent, converter-limited DC fault behavior. Arc-flash incident-energy analysis therefore remains a separate specialist workstream; the Rust ledger in this branch is not a substitute.

References:
- Schneider Electric, [DC Arc Flash Analysis: A Practical Study on 800 VDC AI Data Centers](https://www.se.com/us/en/download/document/SPD_WP219_EN/), 31 July 2026.
- Schneider Electric, [800 VDC safety in AI data centers: Understanding the real arc-flash risk](https://www.se.com/ww/en/insights/ai-and-technology/artificial-intelligence/800-vdc-safety-in-ai-data-centers-understanding-the-real-arc-flash-risk/).

## 3. Offline implementation in this branch

The new Rust module `dc_protection.rs` adds two read-only functions.

### 3.1 DC fault-energy ledger

For one declared boundary it reports these terms separately:

```
E_capacitor = 0.5 * C_max * V_max^2
E_cable     = 0.5 * L_max * I_max^2
E_source    = V_source,max * I_source,max * t_isolation,max
```

The source term is a rectangular upper bound, not a simulated waveform. The sum of the selected maxima is intentionally conservative and can over-count if energy paths overlap. The report preserves the terms separately so that a reviewer can see where the estimate comes from.

### 3.2 First-order DC-link discharge screen

For a constant-R, constant-C discharge model:

```
t_to_threshold = R_max * C_max * ln(V_initial,max / V_threshold)
I_initial,max  = V_initial,max / R_min
P_initial,max  = V_initial,max^2 / R_min
E_dissipated   = 0.5 * C_max * (V_initial,max^2 - V_threshold^2)
```

The code reports the calculated time, current, initial resistor power, energy to dissipate, and numeric margins against user-supplied limits. Resistance bounds are expected to already include declared tolerance and temperature effects.

**Competitive benchmark update:** a separate 11 kW EPC ISOP 800 V-to-50 V architecture is now recorded in [HVDC_BENCHMARK_UPDATE_2026-11-10.md](./HVDC_BENCHMARK_UPDATE_2026-11-10.md). The 98.0% at 220 A / 98.5% peak figures are from an EPC presentation reported by an industry article; they are not a Luminous measurement or independent product qualification.

**The two numeric booleans are not a safety verdict.** A resistor pulse-energy rating must be interpreted against its actual pulse-duration, repetition, initial-temperature and mounting derating curves. The RC model omits voltage-dependent capacitance, parasitic inductance, switching, contactor sequencing, leakage, parallel discharge paths, failed-open/failed-short behavior and source re-energization.

### 3.3 Deliberate non-goals

This code does not calculate or qualify:

- Arc-flash incident energy, arc duration, approach boundaries or PPE.
- Fault-current waveforms, DC breaker interrupting capacity, selectivity, let-through I²t or fuse coordination.
- Semiconductor short-circuit withstand, desaturation response or solid-state breaker commutation.
- Insulation coordination, creepage/clearance, partial discharge, grounding, touch voltage or protective-earth continuity.
- Pre-charge design, contactor selection, hot-swap sequence, safe service time or lockout/tagout adequacy.
- A safe discharge design, a safe operating state, or authorization to handle/energize equipment.

Unknown source current, clearing time, capacitance, residual voltage or resistance is not zero. It is missing evidence and blocks a complete engineering disposition.

## 4. Protection co-design architecture

The target is an evidence-backed sequence of independent protection layers, not an AI-controlled trip system:

1. **Constrain:** choose a declared bus/grounding architecture, source envelope, capacitance, cable inductance and module fault boundary.
2. **Detect locally:** use appropriately rated local sensing and independent hardware trip paths for the hazards that require them.
3. **Interrupt:** coordinate semiconductor current limiting/interruption with a qualified DC-rated protective device and an independent means of isolation where required.
4. **Contain:** segment the bus and prevent a failed module or branch from propagating faults or backfeeding healthy sections.
5. **Discharge:** account for every energy source and path; define the residual-voltage criterion and independent verification.
6. **Prove safe state:** require local measurement and operator-visible evidence before service or re-energization.
7. **Reconcile:** after an unknown actuation or measurement outcome, do not issue a new energization command until the physical state is independently reconciled.

Symthaea is allowed to propose designs, compare models, detect anomalies and prepare evidence. It must not participate in the fast protective loop, directly command gate timing, or override a local protection invariant. Cloud availability, network communications and model confidence are not safety functions.

## 5. Standards and regulatory routing to investigate

This is a research index, not a complete applicability determination. The exact product category, system boundary, market and jurisdiction determine the required standards and certification route.

| Reference | Confirmed scope / relevance | Luminous action |
|---|---|---|
| IEC 62477-1:2022 | General safety requirements for power-electronic converter systems/equipment, including control, protection, monitoring and measurement; scope extends to 1,500 V DC | Use as a starting point for converter safety mapping; identify any more specific product standard and amendments |
| IEC 60947-2:2024 | Circuit-breakers with rated voltage up to 1,500 V DC (and 1,000 V AC) | Map protective-device selection, isolation suitability, DC test conditions and manufacturer application limits |
| OCP LVDC white paper v1.1.0 (September 2026) | Data-centre LVDC system architecture and interfaces | Reproduce the current system requirements and track version changes |
| OCP Protection and Safety workstream | Dedicated workstream under Power Distribution; detailed scope is still being developed on its public page | Participate in open work; do not claim OCP compliance from this research |
| Schneider Electric 800 VDC arc-flash study (July 2026) | Scenario analysis for sidecar and centralized 800 VDC architectures | Use to build a separate arc-flash questions register for qualified specialists |

Official references:
- [IEC 62477-1:2022](https://webstore.iec.ch/en/publication/28936).
- [IEC 60947-2:2024](https://webstore.iec.ch/en/publication/66277).
- [OCP Power Distribution](https://www.opencompute.org/projects/data-center-facility/power-distribution/).
- [OCP Protection and Safety](https://www.opencompute.org/projects/data-center-facility/power-distribution/protection-and-safety/).

## 6. Benchmark changes: compare protection outcomes, not trip-speed slogans

For any future protection assembly, the benchmark record must include the same source, bus, grounding, conductor, capacitance, inductance, load, fault location and boundary for every candidate.

| Benchmark dimension | Required measurement / evidence |
|---|---|
| Fault-energy boundary | C, L, source current/voltage envelope, source contribution duration, parallel sources and uncertainty |
| Detection | Fault class, sensor bandwidth, threshold tolerance, latency distribution, missed-detection and nuisance-trip rates |
| Interruption | Measured current and voltage waveforms, interruption time, peak let-through current, let-through energy, restrike/re-ignition observation |
| Isolation | Verified open-circuit state, dielectric suitability, polarity dependence, leakage and backfeed paths |
| Discharge | Residual voltage versus time, worst-case capacitance/tolerance, discharge-path faults, independent voltage verification |
| Selectivity | Healthy branches retained or de-energized as intended; coordination across source, feeder, shelf and module |
| Thermal | Device, busbar, connector and termination hot spots before/during/after fault interruption |
| Service | Safe-state evidence, replacement time, repair scope and requalification coverage |
| Resilience | Repeatability, environmental corners, component aging, sensor faults and controller/communications loss |

A solid-state interrupter is not automatically superior to a hybrid interrupter. Compare conduction loss, interruption behavior, galvanic isolation, leakage, thermal burden, serviceability, failure modes and total lifecycle cost together.

## 7. Topology research updates to preserve

- **500 W bidirectional branch:** retain DAB, conventional CLLC and the fixed-frequency LLC with actively controlled bridge-leg voltage as distinct candidates. The May 2026 500 W paper reports 96.8% forward and 97.0% reverse peak efficiency on a 40–60 V / 400 V prototype. It is a topology reference, not a matched PEBB-LV-01 baseline.
- **CLLC state-constrained design:** the October 30, 2026 T-network letter explicitly includes secondary active-bridge equivalent reactance in its T-network state constraints; its prototype reports 95.7% maximum forward, 95.2% rated-forward and 95.4% reverse efficiency. Its test prototype is 2 kW, 150–300 V input and 75–167 V output. The method is valuable; its efficiency is not a 500 W competitor benchmark.
- **800 V-to-50 V DCX branch:** reproduce the APEC 6 kW GaN LLC DCX (98.75% reported peak, 2,070 W/in^3), the CPSS ISOP LLC / matrix-magnetics design (98.1% full load, peak above 98.4%), and EPC's 11 kW ISOP LLC presentation (98.0% at 220 A, 98.5% peak under forced air; approx. 436 mm^2/kW). Their power levels, cooling and density boundaries differ. See [HVDC benchmark update](./HVDC_BENCHMARK_UPDATE_2026-11-10.md); do not rank them as a matched head-to-head.

The CLLC paper's critical lesson is that an FHA gain curve alone is not enough. The feasible parameter region must satisfy the selected T-network branch-admittance state, the secondary equivalent-reactance sign, frequency window, constant-power ZVS, unity-gain ZVS and maximum-gain constraints together. The paper's prototype shows ZVS waveforms at the tested light, medium and full-load points; that is not a universal ZVS guarantee outside its demonstrated envelope.

## 8. Next execution gates

1. Add property-based and boundary tests for both new analytical functions; run tests, formatting and clippy in a Rust-capable environment.
2. Independently recompute the 800 V / 1 mF example and all source/cable terms; verify units and worst-case conventions.
3. Add uncertainty propagation and explicit evidence provenance for C, L, current, source-current limit, isolation time, voltage threshold and resistor pulse curves.
4. Build an architecture-specific fault tree and protection coordination register for each distinct grounding arrangement and module boundary.
5. Commission an independent electrical-safety specialist to review the protection assumptions and standards applicability before any hardware activity.
6. Treat switching simulation, hardware-in-the-loop, physical interruption, arc-flash analysis and certification as separate future evidence gates.

No tests, simulations, HIL, physical experiments or certifications are claimed by this branch.
