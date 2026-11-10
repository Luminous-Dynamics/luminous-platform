# High-voltage DCX benchmark update — 10 November 2026

**Parent programme:** [Luminous Power Fabric #364](https://github.com/Luminous-Dynamics/luminous-platform/issues/364)  
**Research base:** [PR #365](https://github.com/Luminous-Dynamics/luminous-platform/pull/365), branch `research/luminous-power-fabric-v0.1`  
**Status:** literature / vendor-presentation benchmark update. No Luminous HV hardware, matched test, or independent reproduction is claimed.

## 1. A new 11 kW comparator changes the benchmark

The earlier PF-HVDC-6K plan correctly recorded the APEC 2026 6 kW GaN LLC reference and the CPSS ISOP LLC result. A further comparator should now be added: EPC's EPC91134 800 V-to-50 V ISOP platform.

EPC's October 19, 2026 announcement identifies an 11 kW 800 V-to-50 V converter built around an input-series, output-parallel (ISOP) architecture. A September 24, 2026 technical article reporting EPC's presentation gives the following detail:

| Item | EPC91134 reported result |
|---|---:|
| Input / output | 800 VDC / 50 VDC |
| Maximum output | 11 kW / 220 A |
| Module organization | Four input-series / output-parallel modules |
| Per-module operating point | 200 V to 50 V, 2.75 kW |
| Switching frequency | 1.1 MHz |
| Integrated magnetics | Two ISOP modules share a magnetic core / combined secondary structure |
| Reported footprint | approximately 4,800 mm²; 436 mm²/kW |
| Cooling in reported test | forced air |
| Reported efficiency | 98.5% peak; 98.0% at 220 A output |

The 11 kW output current is internally consistent: 50 V × 220 A = 11 kW. These figures are reported from an EPC presentation by a technical industry article whose images are credited to EPC. EPC's own news page independently confirms the 11 kW architecture, but does not publish the complete efficiency curve on the page.

**Benchmark status:** vendor-demonstrated / presentation-reported result. It is not an independent certification and it is not a matched comparison against Luminous. Preserve the source boundary, cooling and test-condition caveats.

Sources:
- EPC, [High-Density 800 V-to-50 V, 12 V, and 6 V Conversion](https://epc-co.com/epc/about-epc/events-and-news/news/artmid/1627/articleid/3381/high-density-800-v-to-50-v-12-v-and-6-v-conversion), 19 October 2026.
- Maurizio Di Paolo Emilio / EPC, [How Low-Voltage GaN FETs Are Enabling Multi-kW 800 V Isolated Converters](https://www.allaboutcircuits.com/industry-articles/how-low-voltage-gan-fets-are-enabling-multi-kw-800-v-isolated-converters/), 24 September 2026.

## 2. Keep the three high-voltage references separate

| Reference | Conversion | Published headline result | Proper interpretation |
|---|---|---|---|
| Prakash & Li, APEC 2026 | 6 kW, 800 V to 50 V, GaN LLC DCX | 98.75% peak; 2,070 W/in³ | Peer-reviewed conference demonstrator; peak and volumetric density require exact boundary extraction |
| CPSS Transactions on Power Electronics and Applications, 2026 | 800 V to 50 V, ISOP half-bridge LLC, GaN and matrix transformer | 98.1% full load; peak >98.4% | Separate paper and implementation; useful full-load comparator |
| EPC91134, 2026 presentation | 11 kW, 800 V to 50 V, multi-cell ISOP LLC with integrated magnetics | 98.0% at 220 A; 98.5% peak; approximately 436 mm²/kW | Higher-power vendor demonstrator; forced-air condition stated; area metric is not volumetric power density |

Do not merge these into one "industry SOTA" number. In particular:

- 2,070 W/in³ is a volumetric metric; 436 mm²/kW is an area-normalized footprint metric. They cannot be compared directly.
- The power ratings, magnetic integration, cell count, cooling boundary, regulation and efficiency measurement procedures differ.
- An unregulated/fixed-ratio DCX is not a regulated isolated converter and is not a bidirectional storage converter.
- The EPC article describes regulation options but does not establish that every presented variant retains the reported fixed-ratio efficiency.

## 3. Architecture opportunity: treat the DCX as a cell-composition and regulation-placement problem

The most interesting lesson is not simply "use GaN at 1.1 MHz." The ISOP architecture divides the high-voltage input among series-connected cells and combines their outputs in parallel. The EPC example reports integrated magnetics across pairs of cells and current sharing between magnetically coupled sections.

This suggests a new research experiment: compare *where regulation is placed* rather than assuming every cell must be fully regulated.

Candidate configurations to model on a common boundary:

1. **Fixed-ratio DCX cells:** maximum-efficiency baseline for a known, bounded bus and load.
2. **One regulated cell / remaining fixed-ratio cells:** a regulated pre-stage or one adjustable module accommodates a limited bus/output range while the remaining cells stay near their high-efficiency operating point.
3. **One regulated isolated cell / remaining LLC cells:** replace one cell with DAB or phase-shift regulation; measure the cost in device count, magnetic utilization, controls and density.
4. **Regulation distributed across all cells:** wider regulation and sharing flexibility, with additional switching/control loss and cell mismatch questions.
5. **DCX plus downstream point-of-load regulation:** optimize 800 V-to-50 V and 50 V-to-processor rails as a complete chain, not as isolated stages.

For every option, evaluate efficiency map, current sharing, cell input-voltage balance, output ripple, transient response, regulation range, thermal distribution, fault isolation, degraded operation, service procedure and total system losses. The power fabric must treat module insertion/removal and series/parallel reconfiguration as separately designed and qualified operations, not a software-only runtime convenience.

### Offline engineering increment: static output-current sharing

The Rust library now includes `isop_sharing.rs`, a linear Thevenin/droop screen for output-parallel cells:

```
I_i = (E_i - V_bus) / R_i
V_bus = (sum(E_i / R_i) - I_load) / sum(1 / R_i)
```

It reports cell currents and DC output powers, bus voltage, maximum share deviation, peak-to-peak current spread, current-balance residual and cells for which the model predicts reverse current/backfeed. Seven unit tests are authored for equal sharing, resistance mismatch, setpoint mismatch at zero load, conservation, input validation and underflow.

The model's important use is to expose current-sharing stress and backfeed sensitivity before attempting a high-fidelity multi-cell simulation. It assumes linear positive-slope output characteristics. It does **not** model input-series voltage balance, control-loop interaction, negative incremental impedance, switching ripple, transient load steps, current limiting, cell dropout, protection coordination or fault dynamics. A predicted backfeed current is a review flag, not proof that a particular converter can absorb reverse energy.

## 4. Target ladder — provisional, not a product specification

### PF-HVDC-6K, 800 V to 50 V

The previous internal proposal of ≥98.2% full-load and ≥98.8% peak should be revised upward as a *research stretch*, because the newly recorded references show 98.0% full-load / 98.5% peak at 11 kW and 98.1% full-load / >98.4% peak in a separate 800 V-to-50 V ISOP paper.

| Level | Efficiency objective | Qualification condition |
|---|---:|---|
| Reproduction gate | Reproduce each selected source within its stated measurement uncertainty and exact test boundary | Obtain full paper / test curve, operating point, cooling, instrumentation and efficiency boundary first |
| Competitive research target | ≥98.0% full-load and ≥98.5% peak | Matched voltage, output current, thermal boundary and measurement method; independent review |
| Stretch target | ≥98.5% full-load and ≥99.0% peak | Full operating map, explicit uncertainty budget, thermal margin, fault/protection review and repeatability evidence |

These values are **aspirational internal targets**, not demonstrated performance, contractual claims or frozen requirements. A design that reaches 99% peak but fails thermal, fault, service or manufacturing constraints is not a winning product.

For density, reproduce the APEC paper's 2,070 W/in³ only after resolving exactly which volume is counted (active power stage, complete converter, connectors, cooling and enclosure). Separately record footprint mm²/kW, stack height, cooling volume and mass. Do not substitute a smaller active-board boundary for a complete-system boundary.

### PEBB-LV-01, 500 W bidirectional

Keep ≥97% as the functional Rev-A objective and ≥98% as a stretch objective, not a superiority claim. The 500 W CLLC paper's 97.6% is a reported peak-forward result; the fixed-frequency LLC plus bridge-leg voltage-control prototype reports 96.8% forward / 97.0% reverse, but operates between 40–60 V and 400 V. ELSİS advertises >97% for a 500 W bidirectional buck-boost product, while its public material does not establish galvanic isolation. These are distinct functions and boundaries.

To claim a competitive result, publish both-direction maps over the whole agreed input/output/load envelope, light-load and standby behavior, uncertainty, thermal conditions, power density and isolation qualification. A peak-only result is insufficient.

## 5. A better benchmark than peak efficiency

Create a multi-objective scorecard, with hard safety / interface constraints separated from optimizable objectives.

**Hard constraints:** voltage/current ratings, insulation coordination, protection coordination, thermal limits, isolation, pre-charge/discharge, fault containment, grounding compatibility and service safety.

**Measured performance vector:** full efficiency surface; standby / auxiliary power; system energy per useful delivered kWh; W/in³ and mm²/kW with declared boundaries; mass; output ripple; dynamic response; temperature rise and hotspot margin; current-sharing error; noise / EMI; lifetime stress; service time; spare-part availability; requalification burden; and cost per delivered kWh over a declared mission profile.

A candidate is superior only if it improves a declared Pareto frontier without violating any hard constraint. Publish dominated candidates and negative results, not only the winner.

## 6. Immediate follow-on work

1. Download and extract the EPC presentation curves and the full APEC / CPSS test boundaries.
2. Reproduce the EPC ISOP cell and integrated-magnetics loss/thermal model as a distinct branch; do not adapt the 500 W DAB/CLLC model by scaling power.
3. Implement a cell-balance and current-sharing model with explicit cell mismatch, sensor offsets, unequal impedances and an open/disabled-cell state.
4. Add regulation-placement comparisons to the whole-chain energy auditor.
5. Extend the protection register to input-series cell voltage imbalance, output-parallel current sharing, cell bypass/failure, reverse energy paths and module extraction.
6. Obtain independent review of the high-voltage fault, arc-flash, insulation and service assumptions before any hardware work.

No simulation, HIL, hardware testing, certification or matched Luminous/competitor comparison is claimed by this update.
