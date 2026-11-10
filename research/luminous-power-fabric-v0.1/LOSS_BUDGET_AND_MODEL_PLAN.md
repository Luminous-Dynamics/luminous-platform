# PEBB-LV-01 loss budget and topology-model plan v0.1

**Snapshot:** 2026-11-10  
**Parent:** [Issue #364](https://github.com/Luminous-Dynamics/luminous-platform/issues/364)  
**Status:** equation and data-contract proposal; no DAB/CLLC waveform model or physical loss result is implemented here.

## 1. Why watts of loss are the next engineering unit

A peak efficiency percentage does not identify which design change is useful. At 500 W output, the loss budget is small enough that a few watts from magnetics, switching, dead time, gate drive, interconnect or cooling materially change the result.

For a converter measured at one declared power boundary:

`P_in = P_out + ΣP_loss,i`

`η = P_out / P_in`

The maximum permitted loss for a target efficiency is:

`P_loss,max = P_out(1/η_target - 1)`

For 500 W output:

| Efficiency | Maximum total loss | Engineering interpretation |
|---:|---:|---|
| 97.0% | 15.464 W | Functional Rev-A floor proposal |
| 97.6% | 12.295 W | Loss equivalent of the June 2026 500 W CLLC paper's reported peak forward efficiency |
| 98.0% | 10.204 W | Initial competitive objective at matched operating points |
| 98.5% | 7.614 W | Stretch target; requires strong thermal/magnetic/switching performance |

These are arithmetic conversions of percentages into allowable watts, not predicted Luminous losses. The paper's 97.6% is a peak forward-transfer result; it does not establish the same result in reverse or over our proposed 36–60 V envelope.

Moving from 97.6% to 98.0% at 500 W means reducing the permitted loss budget by about 2.09 W, or roughly 17% of the 97.6%-equivalent loss. That is a meaningful engineering challenge, not an automatic consequence of changing topology.

## 2. Loss-budget accounting engine

The new Rust `loss_budget` module is a read-only accounting layer. It accepts explicit loss terms with category, watts, uncertainty, evidence kind and provenance reference. It calculates:
- Total converter loss.
- Derived input power and efficiency.
- Maximum loss allowed for a target efficiency.
- Remaining / exceeded target loss headroom.
- Each term's percentage contribution to total loss.
- Root-sum-square standard uncertainty (only interpretable when the terms are sufficiently independent).
- A conservative linear uncertainty bound when correlation is unknown.

The tool rejects empty budgets, zero-total-loss idealizations, negative/non-finite values, invalid targets and missing provenance. It retains the evidence kind supplied by the author. It does not turn a datasheet estimate into a measurement or a simulation into a physical qualification.

### Loss categories to populate

| Category | Required data from the upstream model or instrument | Typical calculation / origin |
|---|---|---|
| Semiconductor conduction | Per-device RMS current, on-state characteristic at junction temperature and gate voltage, conduction interval | Sum of device conduction losses; for a MOSFET screening approximation use I_RMS² × R_DS(on)(T,V_GS), with actual switching intervals and parallel-device sharing |
| Semiconductor switching | Device current and voltage at transitions, switching energy at relevant gate resistance/temperature, switching frequency and commutation state | Sum of E_on + E_off per event × event rate; datasheet test-point scaling must be justified |
| Dead-time / reverse conduction | Dead-time, reverse current, body-diode or reverse-channel voltage, commutation count | Integrate actual current × voltage over dead-time intervals |
| Gate drive | Total gate charge at actual gate voltage, driver supply voltage/current, number of devices, frequency, driver quiescent power | First-order gate-charge estimate Q_g × V_drive × f_sw × device count, plus driver losses |
| Transformer copper | Each winding RMS current, AC resistance at switching frequency and operating temperature | Sum of I_RMS² × R_AC(f,T), including proximity / skin effects and terminations |
| Transformer core | Core material, geometry, flux density waveform, frequency and temperature | Validated material-loss model or calorimetric/bench result; do not substitute generic Steinmetz coefficients without matching waveform and material |
| Resonant / leakage inductors | RMS current, AC winding resistance, core loss and saturation margin | Copper plus core loss, preserving the waveform and temperature basis |
| Capacitors | Ripple RMS current, ESR versus frequency/temperature, dielectric loss where relevant | I_ripple,RMS² × ESR(f,T), plus any justified dielectric term |
| Interconnect / PCB / busbar | Current waveform, measured or extracted parasitic resistance, temperature | I_RMS² × R(T), including connector/contact/joint contributions |
| Snubber / clamp | Energy dissipated per event and event rate, or measured dissipation | E_event × f_event, with repeated-energy paths not double-counted |
| Cooling / auxiliary | Fan/pump/control-supply input power at the measurement boundary | Physical input power to all required auxiliaries |
| Control / sensing | Controller and measurement supply input power, including isolated bias rails | Measured or datasheet-backed rail draw plus conversion loss |

This table is not a ready-made DAB/CLLC formula set. Current waveforms, RMS values, switching events, magnetic flux and resonant currents must come from an upstream topology model and later be correlated with measurements.

## 3. Avoid double-counting

This is one of the major ways analytical converter models produce unrealistic efficiency.

- A datasheet `E_on/E_off` curve may include device output-capacitance or reverse-recovery effects under the vendor test circuit; do not add the same energy again as an independent C_oss / Q_rr term without defining the boundary.
- MOSFET `R_DS(on)` conduction loss and body-diode dead-time loss are distinct only when the intervals and current paths are separated.
- Transformer copper loss and leakage-inductor copper loss can be the same physical winding; do not count one twice.
- Core-loss model coefficients must correspond to material, waveform, frequency, temperature and flux-density range.
- Fan and auxiliary supply draw belong in system efficiency; do not count an internal fan twice if it is already inside an instrument's measured input boundary.
- If input/output power is directly measured for a whole converter, the total measured loss is the balance residual. Component-loss estimates are diagnostic allocations, not an additional loss to add on top of the measured total.

Every loss term must declare exactly which physical boundary it belongs to.

## 4. Model hierarchy: do not skip fidelity levels

1. **Analytical screening:** fundamental-harmonic approximations for CLLC gain and resonant design; DAB single-phase / extended-phase-shift power-flow and current-stress equations. Explicitly record their assumptions.
2. **Non-ideal switching model:** real semiconductor conduction/switching behavior, transformer and inductor parasitics, ESR, dead time, gate drive, snubbers, interconnect and control implementation.
3. **Electrothermal coupling:** temperature-dependent device resistance, core/copper loss, cooling, thermal capacitance and transient temperature.
4. **Tolerance / uncertainty sweep:** component tolerances, parameter drift, temperature, source/load range and model uncertainty.
5. **Controller-HIL / switching HIL:** validate control and timing boundaries.
6. **Physical prototype correlation:** instrumented efficiency maps, waveforms, temperature maps and discrepancy analysis.

A model cannot be promoted merely because its numerical solver converges. It must preserve energy accounting, satisfy physical bounds and eventually match a declared hardware revision.

## 5. Topology-specific modelling tasks

### DAB

- Derive SPS and EPS or TPS operating boundaries for the chosen voltage ratio.
- Calculate bridge current waveforms and RMS / peak device current.
- Map ZVS conditions for each bridge and direction.
- Quantify circulating current and reactive power at light load.
- Compare fixed-frequency and constrained variable-frequency operation.
- Include transformer leakage, magnetizing current, turns ratio, core and winding loss.
- Record current stress and ZVS margin separately from efficiency.

### CLLC

- Begin with FHA for gain and soft-switching screening, explicitly identifying harmonic limitations.
- Parameterize L_r1, L_r2, C_r1, C_r2, L_m and turns ratio with tolerances.
- Map frequency-control range and gain in both directions.
- Derive the ZVS constraints before optimizing the resonant tank.
- Include current stress, resonant tank RMS current, capacitor ripple, magnetic loss and frequency-dependent switching loss.
- Investigate synchronous rectification and variable-frequency control as separate design variants.
- Compare the model with time-domain switching simulation before selecting a design.

### Cross-topology selection

Use a Pareto surface rather than a weighted scalar score. At minimum retain separate axes for:
- Full operating-envelope efficiency.
- Peak and RMS semiconductor current.
- ZVS/ZCS coverage and commutation margin.
- Transformer / magnetic volume and temperature.
- Device and magnetics BOM cost / availability.
- Control complexity and transient response.
- Fault containment and safe-state behavior.
- Serviceability and replacement cost.
- Uncertainty and model-to-bench residual.

A candidate that improves peak efficiency but loses ZVS at cold start, has poor reverse operation or materially worsens thermal cycling should not win by a scalar average.

## 6. Model-to-bench correlation gate

A 2025 IEEE DAB loss-model study reported experimental/model loss deviations below 5 W across its tested 80–180 kHz range and SPS/EPS scenarios. This is a useful published example of the kind of model-correlation metric we should report, not a universal tolerance. Its voltage, power and hardware conditions are not a PEBB-LV-01 match.

For our future prototype, preregister:
- Total loss residual in watts and percentage of output power.
- Efficiency residual at every operating point, not only at the peak.
- Device-current RMS and peak residual.
- Transformer copper/core loss residual where independently measurable.
- Temperature residual at named sensor locations and thermal equilibrium.
- Reverse-flow residual separately.
- Uncertainty interval and model parameter revision.

Target model-loss error only after the instrument uncertainty and physical test design are defined. A numerical target such as ±5 W would be meaningless if the total measurement uncertainty is itself several watts.

## 7. Recommended next experiment sequence

1. Reproduce the 500 W CLLC paper's own operating point from its full paper and supplement before using its 97.6% result as a matched comparator.
2. Reproduce a DAB SPS/EPS loss benchmark from published equations and parameter tables.
3. Create one non-ideal model for each topology with the same port envelope and power boundary.
4. Run both directions and all declared voltage/load points.
5. Compare loss budgets category-by-category; preserve missing parameters as unknown rather than zero.
6. Sweep resonant components, turns ratio, switching frequency and semiconductor choice under hard ZVS/current/thermal constraints.
7. Only after independent review, select a prototype topology and create its controlled schematic/BOM package.

## 8. Explicit claim ceiling

The Rust module is a loss-budget arithmetic and provenance layer. It is not a converter solver, a DAB or CLLC model, a waveform simulator, an optimizer, a thermal model, a safety function or a physical qualification tool. The current test cases are authored but not run in this environment. No Luminous loss values or topology ranking have been produced.
