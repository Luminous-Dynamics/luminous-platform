# Grid-to-chip power delivery: benchmark and architecture research

**Snapshot:** 10 November 2026  
**Parent:** [Luminous Power Fabric #364](https://github.com/Luminous-Dynamics/luminous-platform/issues/364)  
**Implementation branch:** `research/luminous-power-fabric-isop-v0.3`  
**Status:** literature synthesis, offline steady-state screening code and proposed benchmark protocol. No Luminous converter or facility result is claimed.

## 1. Strategic update: the competitive unit is the entire grid-to-chip power tree

The August 2026 IEEE Open Journal of Power Electronics review, *State-of-the-Art Power Electronics in AI Data Centers*, benchmarks four stages from the grid to the processor die: PSU, high-voltage intermediate bus converter, on-board low-voltage intermediate bus converter / VRM, and enabling wide-bandgap devices. It compares conventional 3–12 kW AC-distributed power electronics against emerging 800 VDC architectures, including ISOP GaN LLC designs. Its abstract reports ISOP power densities exceeding 20 times those of conventional PSUs at comparable efficiency, and separately evaluates board-level LV IBC, VRM and direct 48 V-to-PoL architectures on efficiency, density, transient response and thermal performance.

This changes the Luminous benchmark strategy: a single 800 V-to-50 V converter efficiency is no longer the dominant comparison. The measurable competitive unit is the power path from the facility DC boundary to useful processor-rail power, with protection, hold-up, cooling and interconnect losses explicitly included.

Reference:
- Surya Elangovan et al., IEEE Open Journal of Power Electronics, [State-of-the-Art Power Electronics in AI Data Centers](https://doi.org/10.1109/OJPEL.2026.3720942), published 6 August 2026, vol. 7, pp. 2434–2463.

## 2. Competitive architecture evidence

### 2.1 Two-stage 800 V-to-processor-core path

Texas Instruments announced a complete 800 VDC architecture with NVIDIA on 16 March 2026. TI describes a two-stage path:
1. 800 VDC to 6 V isolated bus conversion.
2. 6 V to less-than-1 V multiphase buck conversion near the processor.

TI's announced 800 V-to-6 V GaN bus converter reports 97.6% peak efficiency and more than 2,000 W/in³ power density. The solution also includes an 800 V hot-swap controller, 30 kW AC/DC supply and an 800 V capacitor-bank unit with EDLC cells. These are vendor-announced reference-design figures; they are not independent measurements of a complete data-hall-to-GPU path, and the public release does not state a full efficiency map or full-load efficiency.

Reference:
- TI, [Complete 800 VDC power architecture for future generation AI data centers with NVIDIA](https://www.ti.com/en-us/about-ti/newsroom/news-releases/2026/2026-03-16-ti-unveils-complete-800-vdc-power-architecture-for-future-generation-ai-data-centers-with-nvidia), 16 March 2026.

### 2.2 EPC multi-rail ISOP designs

EPC's 2026 demonstrations describe separate conversion boundaries:

| Rail | Reported design point | Reported efficiency | Engineering implication |
|---|---|---|---|
| 800 V → 50 V | 11 kW / up to 220 A | 98.0% at 220 A; 98.5% peak in detailed October/November reporting | Intermediate rack bus retaining conventional 48/50 V distribution practices |
| 800 V → 12.5 V | 6 kW / up to 480 A | Above 98% peak; about 97% full-load in November reporting | Direct high-current compute-tray rail |
| 800 V → 6.25 V | 6 kW / up to 960 A | Above 98% peak; about 97% full-load in November reporting | Very high-current rail that can improve downstream PoL density but imposes severe current-extraction requirements |

EPC's April 2026 webinar describes an eight-interleaved-module 800 V (±400 V)-to-12.5 V ISOP converter and says the architecture reduces ripple current, capacitance and thermal concentration. The October 2026 EPC article describes its 11 kW 800 V-to-50 V converter and 6 kW 800 V-to-12 V design. The November article gives further details on 50 V, 12.5 V and 6.25 V rails.

The EPC figures are vendor-demonstrated results and technical presentations, not independent matched comparisons against Luminous. Exact source revisions, operating points and cell boundaries must be reconciled before a replication model is frozen.

References:
- EPC, [800 V Server Power Supplies](https://epc-co.com/epc/design-support/technical-publications/articles/artmid/9702/articleid/3289/800-v-server-power-supplies), 6 April 2026.
- EPC, [High-Density 800 V-to-50 V, 12 V, and 6 V Conversion](https://epc-co.com/epc/design-support/technical-publications/articles/artmid/9702/articleid/3381/high-density-800-v-to-50-v-12-v-and-6-v-conversion), 19 October 2026.
- EPC / EEPower, [800 VDC to the AI Server: A Modular GaN Route To Low-Voltage Rails](https://eepower.com/technical-articles/800-vdc-to-the-ai-server-a-modular-gan-route-to-low-voltage-rails/), November 2026.

### 2.3 OCP architecture context

OCP's Power Distribution sub-project defines LVDC distribution as up to 1,500 VDC, and its scope includes system architecture, ESS, rack power, protection interfaces, hot-swap/startup and hold-up. The project page lists white paper v1.1.0 as released September 2026 and has distinct sub-workstreams for Protection and Safety, Polarity and System Earthing, System Architecture, Rack and Power, Voltage Levels and Power Quality, Main Components, and Codes and Standards.

This is not a single frozen end-to-end product specification. It is a standards and interoperability convergence programme. Luminous should design for documented interfaces and versioned requirements, not claim compliance merely because a design uses an 800 V nominal bus.

References:
- OCP, [Power Distribution](https://www.opencompute.org/projects/data-center-facility/power-distribution/).
- OCP, [Protection and Safety](https://www.opencompute.org/projects/data-center-facility/power-distribution/protection-and-safety/).

## 3. The rail-current trade-off is not optional

At 6 kW useful load power:

| Load rail | Current required |
|---|---:|
| 50 V | 120 A |
| 12.5 V | 480 A |
| 6.25 V | 960 A |
| 6 V | 1,000 A |
| 0.9 V | 6,667 A |

For a simple two-wire path with loop resistance R and a constant-power load at voltage V:

```
I_load = P_load / V
P_copper = I_load^2 * R
Delta_V = I_load * R
```

For **the same hypothetical loop resistance**, relative copper losses at 50 V, 12.5 V and 6.25 V are 1:16:64. This is a normalization thought experiment, not a claim that the three architectures have the same conductor geometry. Direct low-voltage architectures require dramatically shorter, wider and more distributed current-extraction paths, as illustrated by EPC's matrix-transformer and perimeter-extraction designs.

The new Rust `dc_rail_path` screen calculates each architecture using its own supplied loop resistance and exposes voltage-drop and conduction-loss resistance budgets. It deliberately does not assume the same R for all rail options.

## 4. The real optimization problem: place conversion where the complete path wins

Compare at equal delivered processor power and equal time-varying workload:

A. **50 V intermediate distribution**
- 800 V → 50 V DCX.
- 50 V distribution bus and connectors.
- 50 V → 6 V / 12 V board conversion.
- Final multiphase buck / VRM to processor voltage.

B. **Direct 12.5 V path**
- 800 V → 12.5 V ISOP.
- High-current tray distribution.
- Final PoL conversion.

C. **Direct 6–6.25 V path**
- 800 V → 6–6.25 V ISOP.
- Very high-current but short board-level interconnect.
- Final multiphase buck / VRM.

D. **Hybrid path**
- 800 V to a bus selected by rack power, tray topology and service envelope.
- Locally regulated / fixed-ratio cells selected by rail, with redundancy and fault domains designed explicitly.

The winning path is not necessarily the one with the fewest stages. It must minimize total energy and cost while satisfying hard constraints for transient response, thermal margin, protection, availability and repair.

## 5. Benchmark protocol: four levels, one workload

### Level 1 — converter / DCX
- Full efficiency surface at low / nominal / high input voltage and 10 / 25 / 50 / 75 / 100% load.
- Both directions where the product is bidirectional.
- Standby, quiescent and auxiliary losses.
- Full-load and peak efficiency with measurement uncertainty.
- Bare-board, enclosed, and complete-cooling power density.
- Component temperature and hot-spot margin.

### Level 2 — power shelf / tray
- Converter plus hot-swap, pre-charge, isolation, protection and output distribution.
- Cell voltage balance, output current sharing, phase ripple, capacitor RMS current.
- Fault let-through, selective isolation, backfeed and degraded-mode response.
- Service time, replacement scope and requalification coverage.

### Level 3 — processor board
- 50/48 V or 12.5 V or 6/6.25 V input to the same processor rails.
- Dynamic load step: overshoot, undershoot, settling time and rail impedance versus frequency.
- Multiphase current balance, output ripple, capacitor RMS/peak current and thermal distribution.
- VRM / PoL efficiency across realistic processor utilization.

### Level 4 — facility to processor
- Same facility input and same useful processor energy.
- Include every conversion stage, transformer, conductor, connector, cooling system, fan/pump, standby, storage round-trip loss and hold-up operation.
- Compare annual energy, energy cost, grid emissions, peak power, thermal rejection and service downtime.
- Preserve source assumptions, tariff, carbon intensity and workload trace.

**One workload rule:** use identical time-aligned load traces at the processor rail. A steady 100% load point is insufficient to select an architecture for bursty accelerator workloads. Include repeated load steps, long idle intervals, synchronized multi-accelerator bursts and representative compute traces. Publish the exact trace, sampling interval and filtering method.

### Hard constraints are not weighted objectives

Voltage and current limits, insulation, isolation, protection, fault containment, grounding, thermal limits, transient response and safe service are pass/fail gates. Efficiency, density, cost, acoustic noise and maintenance can be optimized only among candidates that satisfy the hard gates.

## 6. Proposed research targets

The following are internal aspirations, not product specifications or achieved results.

| Product track | Competitive research objective | Stretch objective |
|---|---|---|
| PEBB-LV-01, 500 W bidirectional | ≥97% functional efficiency across the declared useful operating map | ≥98% at selected matched operating points, both directions |
| PF-HVDC-6K, 800→50 V | ≥98.0% full-load, ≥98.5% peak | ≥98.5% full-load, ≥99.0% peak |
| PF-HVDC direct 12.5 V / 6.25 V | Reproduce the EPC / CPSS full-load curves and matched density boundary | Improve full-load efficiency and complete-board loss without increasing thermal or fault risk |
| PF-HVDC direct 6 V | Reproduce TI's 97.6% peak, >2,000 W/in³ reference boundary | ≥98% full-load and ≥98.5% peak under matched system boundary |
| Grid-to-chip system | Establish a measured baseline for each of the 50 V, 12.5 V and 6 V paths | ≥0.5 percentage-point improvement in annual energy-weighted end-to-end efficiency against a reproducible baseline, with equal useful processor energy |

A system-level 0.5 percentage-point improvement is a proposed research threshold, not a published industry standard. It should be accepted only where the measured difference exceeds combined expanded uncertainty and is repeatable across workload traces and thermal conditions.

For the direct 6 V path, TI's public release supplies peak efficiency and a density figure but not a full-load map. The first gate is source reproduction and boundary extraction, not claiming a numerical lead from a peak-only figure.

## 7. Paradigm shifts worth pursuing

### 7.1 Grid-to-chip co-design instead of stage-by-stage silos
The utility interface, 800 V backbone, rack distribution, tray DCX, PoL and processor load should share a common time-aligned energy and thermal model. The energy-chain auditor is the first accounting layer; dynamic trace integration is the next.

### 7.2 Current extraction becomes a first-class design domain
At 6 kW, reducing a rail from 50 V to 6.25 V multiplies the delivered current by eight. Output copper, connectors, matrix-transformer placement and board perimeter extraction can dominate the final architecture. Track path resistance and current density as first-order design variables.

### 7.3 Treat power quality and workload dynamics as product features
A converter with a higher peak efficiency may be worse on synchronized accelerator steps, transient droop, bus impedance, hold-up, cooling and service. Compare complete dynamic response under the same load trace, not isolated step tests chosen by each vendor.

### 7.4 Storage is a multi-timescale subsystem
Separate short-duration capacitor / EDLC hold-up from longer-duration batteries and facility storage. Measure energy, power, duration, conversion round-trip loss, transient support and fault isolation separately. Do not treat a capacitor-bank unit as a battery replacement.

### 7.5 Local protection, supervisory intelligence
Local hardware protection and deterministic controller limits remain authoritative. Symthaea may optimize architecture, estimate workload demand, identify anomalies and propose maintenance; it must not be the sole protective relay or directly override gate-level protection.

## 8. Implementation added in v0.3

The Rust measurement-audit library now adds:

- `isop_input_balance`: static input-series voltage allocation from declared per-cell output power, efficiency and quiescent draw; explicit bypass-cell model and voltage-limit results.
- `interleaved_ripple`: harmonic phasor superposition with actual phase offsets and per-phase harmonic amplitudes; returns harmonic peak/RMS residuals and an aggregate RMS screen.
- `dc_rail_path`: steady constant-power load and two-wire loop-resistance calculation for current, voltage drop, I²R loss and resistance budgets.

The modules are analytical screens only. They do not model closed-loop dynamics, switch waveforms, parasitics, capacitor ESR/ESL, thermal behavior, arc flash, protection selectivity or hardware.

## 9. Validation status and next gates

- The new Rust tests are authored but not executed.
- `cargo test`, `cargo fmt`, `cargo clippy`, switching simulation, HIL, physical tests and certification are not claimed.
- Independently recompute the synthetic fixtures and conservation residuals before relying on the screens.
- Next: build a timestamped processor-load trace contract and an end-to-end dynamic energy / thermal replay tool; correlate it against a declared reference converter and measured load waveform before using it to select a product architecture.

No Luminous-vs-industry performance superiority is claimed by this document.
