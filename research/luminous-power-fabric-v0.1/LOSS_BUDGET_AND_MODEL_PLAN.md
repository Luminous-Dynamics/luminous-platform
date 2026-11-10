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
Machine-readable interchange uses [loss-budget-v0.1.schema.json](./schemas/loss-budget-v0.1.schema.json) and the explicitly synthetic [loss-budget.example.json](./schemas/loss-budget.example.json). The `budget.output_power_w` field is the single canonical output-power value; do not duplicate it in the operating-point object.


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

## 3. Ideal DAB SPS transfer screening model (implemented)

The Rust library now exposes `dab_sps::screen_dab_sps`. It inverts the ideal single-phase-shift
power-transfer equation for a requested signed transfer, and reports the ideal maximum transfer,
phase shift and requested-power utilization.

Using secondary voltage and leakage inductance both referred to the primary side:

`P = Va × Vb_ref × φ × (π - |φ|) / (2π² × fs × L)`

`|P|max = Va × Vb_ref / (8 × fs × L)`, at `|φ| = π/2`.

The sign of φ follows the declared power direction: positive Port A→Port B, negative Port B→Port A.
The API uses a referred secondary voltage directly to avoid a hidden transformer-ratio convention.

The companion `leakage_inductance_for_power_and_phase` helper performs the inverse design-space
calculation for a chosen transfer and phase-shift magnitude. At 48 V / 48 V referred, 100 kHz and
500 W, an ideal 45° target gives 4.32 µH. This is only a starting equation: the actual transformer
leakage, added series inductance, turns ratio and voltage-gain map must be derived from the selected
physical design.


### Screening arithmetic for a 48 V, 100 kHz design

With equal 48 V referred ports:
- At 10 µH, the ideal SPS maximum is 288 W; a 500 W request is infeasible before losses.
- At 5.76 µH, the ideal SPS maximum is exactly 500 W; that leaves no phase-shift headroom at rated power.
- At 3.84 µH, the ideal SPS maximum is 750 W; 500 W requires about 38.05° phase shift in this ideal model.

These are ideal equation results, not a proposed magnetic design. They do not include tolerance,
winding loss, magnetizing current, semiconductor limits, current stress, dead-time, ZVS or temperature.
Maximum theoretical power at 90° is not a sensible design target by itself.

The 2026 DAB HIL paper by Nkembi et al. uses a 100 kHz, 10 µH, 1:1 example and studies equal-voltage
10/20/30 V conditions plus 30-to-42 V. Its abstract and discussion emphasize SPS ZVS degradation,
circulating current and stress away from unity conversion ratio / at light load. Those conditions
are a valuable reference for the model and control studies, but its 10 µH setting must not be copied
as a 500 W / 48 V design. The actual paper reports the equation and explicit limitations in its
operating-principle section.

**Claim ceiling:** this function is ideal steady-state SPS power-transfer screening only. It does not
derive device RMS/peak current, prove ZVS, model losses, establish stable control, evaluate EPS/TPS,
or approve energization. The model is suitable for early infeasibility screening, not topology
qualification.

## 4. Avoid double-counting

This is one of the major ways analytical converter models produce unrealistic efficiency.

- A datasheet `E_on/E_off` curve may include device output-capacitance or reverse-recovery effects under the vendor test circuit; do not add the same energy again as an independent C_oss / Q_rr term without defining the boundary.
- MOSFET `R_DS(on)` conduction loss and body-diode dead-time loss are distinct only when the intervals and current paths are separated.
- Transformer copper loss and leakage-inductor copper loss can be the same physical winding; do not count one twice.
- Core-loss model coefficients must correspond to material, waveform, frequency, temperature and flux-density range.
- Fan and auxiliary supply draw belong in system efficiency; do not count an internal fan twice if it is already inside an instrument's measured input boundary.
- If input/output power is directly measured for a whole converter, the total measured loss is the balance residual. Component-loss estimates are diagnostic allocations, not an additional loss to add on top of the measured total.

Every loss term must declare exactly which physical boundary it belongs to.

## 5. Model hierarchy: do not skip fidelity levels

1. **Analytical screening:** fundamental-harmonic approximations for CLLC gain and resonant design; DAB single-phase / extended-phase-shift power-flow and current-stress equations. Explicitly record their assumptions.
2. **Non-ideal switching model:** real semiconductor conduction/switching behavior, transformer and inductor parasitics, ESR, dead time, gate drive, snubbers, interconnect and control implementation.
3. **Electrothermal coupling:** temperature-dependent device resistance, core/copper loss, cooling, thermal capacitance and transient temperature.
4. **Tolerance / uncertainty sweep:** component tolerances, parameter drift, temperature, source/load range and model uncertainty.
5. **Controller-HIL / switching HIL:** validate control and timing boundaries.
6. **Physical prototype correlation:** instrumented efficiency maps, waveforms, temperature maps and discrepancy analysis.

A model cannot be promoted merely because its numerical solver converges. It must preserve energy accounting, satisfy physical bounds and eventually match a declared hardware revision.

## 6. Topology-specific modelling tasks

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

## 7. CLLC FHA gain screening and the next state-constraint layer

The Rust crate now exposes:
- `cllc_fha::evaluate_cllc_fha_gain` — symmetric CLLC FHA normalized gain for a declared
  magnetizing/resonant inductance ratio, direction-specific equivalent quality factor and normalized
  switching frequency.
- `cllc_fha::scan_cllc_fha_gain_curve` — evaluates a strictly increasing list of normalized
  frequencies and classifies the **sampled** gain trend.

For a symmetric tank, the 2026 Tao et al. FHA expression is:

`M = 1 / sqrt(A² + B²)`

`A = 1 + 1/k_L - 1/(k_L * omega_n²)`

`B = (Q/k_L) * ((2k_L+1)omega_n - (2k_L+2)/omega_n + 1/omega_n³)`

where `k_L = L_m / L_rp`, `omega_n = omega_s / omega_r1`, and Q is specific to the
charging or discharging equivalent AC load. The symmetric expression has the same functional form
in each direction, but charging and discharging do not generally have the same Q.

The direction-specific Q values must be derived from the actual equivalent DC loads and referred tank:
- Charging equivalent AC load: `R_eq,ch = 8 N² R_bat / π²`.
- Discharging equivalent AC load: `R_eq,dis = 8 R_dc / (π² N²)`.
- `Q_ch = sqrt(L_rp / C_rp) / R_eq,ch`.
- `Q_dis = sqrt(L_rs / C_rs) / R_eq,dis`.
- For the symmetric referred tank, `L_rp = N² L_rs` and `C_rp = C_rs / N²`.

These definitions follow Tao et al.'s Eqs. (3)–(5). The caller to the Rust evaluator supplies the
correct Q for the selected direction and load point; the function does not derive battery/DC load
models or infer Q from a nominal power command. A wrong Q can produce a plausible-looking but
invalid gain curve.

At resonance (`omega_n = 1`), the ideal FHA normalized gain is unity, independent of Q and k_L.
For the *illustrative equation inputs* `k_L=3.51`, `Q=0.3`, the formula returns approximately
1.14194 at `omega_n=0.8` and 0.90007 at `omega_n=1.2`. These are formula test vectors, not a
PEBB-LV-01 tank prediction. The sampled scan can flag a turning point, but cannot prove continuous
monotonicity between sampled frequencies.

### Generic T-network phasor evaluator (implemented)

The Rust crate now exposes `cllc_t_network::evaluate_cllc_t_network`. It accepts the signed branch
reactances `X1/X2/X3` and a secondary equivalent complex impedance `Zac = Req + jXeq`, and
calculates the T-network admittance parameters, secondary-to-primary voltage phasor ratio, and
primary input admittance / impedance.

With `D = X1*X2 + X1*X3 + X2*X3` and branch impedances `Zi = jXi`, the implemented paper
relationships are:

`Y1 = -j*(X2 + X3)/D`

`Y2 =  j*X3/D`

`Y3 = -j*(X1 + X3)/D`

Using the paper's two-port current convention
`I1 = Y1*U1 + Y2*U2`,
`I2 = -Y2*U1 + Y3*U2 = Yeq*U2`:
- `Yeq = 1/Zac`
- `U2/U1 = Y2/(Y3 - Yeq)`
- `Yin,1 = Y1 + Y2²/(Y3 - Yeq)`
- `Zin,1 = 1/Yin,1`

The paper's variable-frequency triple-phase-shift (VF-TPS) example also has a specific
secondary bridge equivalent (Eq. 1):

`Zac,eq = [8 * Rload * cos²(theta2) * cos²(phi2) / (n² * pi²)] * exp(j * phi2)`

The separate `secondary_vftps_equivalent_impedance` helper implements this expression, and
`evaluate_cllc_vftps_t_network` composes it with the T network. This exact bridge equivalent is
modulation-specific: theta2 is the secondary bridge phase shift and phi2 is the fundamental
secondary AC voltage/current phase difference. It must not be reused for SPS, EPS or another
TPS convention without a fresh derivation.

The crate also implements `screen_cllc_low_frequency_boost_window`, a screening function for
the paper's selected low-frequency boost state (Eq. 10):

`max(fr1/sqrt(1+k), fr2/sqrt(1+2k)) < fs < min(fr1, fr2)`

It intersects the paper-derived open interval with the controller's declared min/max switching
frequency. It reports an empty intersection as infeasible rather than forcing a frequency. This is
a specific state-window screen under the paper's symmetric inductance-ratio assumption; it does not
evaluate the remaining constant-power ZVS, unity-gain ZVS or maximum-gain constraints and does not
prove ZVS.

This module is a generic small-signal phasor network kernel. The caller must supply branch
reactances using a reviewed circuit orientation, sign and turns-ratio convention. It intentionally
does **not** map a selected CLLC schematic to X1/X2/X3, derive the phase-modulated secondary
impedance from bridge angles, classify the paper's operating states, or infer ZVS from the input
impedance alone. Those mappings are exactly where convention errors can invalidate a seemingly
reasonable simulation.

### Required state-constrained extension

Huang et al. (IEICE Electronics Express, online 30 October 2026, DOI 10.1587/elex.23.20260463)
propose mapping a bidirectional CLLC tank to a T-type impedance network and including the secondary
active bridge as a complex AC impedance. Their design region adds internal-state constraints linked
to a designated low-frequency boost mode and resonant-inductance ratio to the FHA gain requirement.
Their 2 kW prototype verifies forward/reverse ZVS for their own configuration and reports 95.7% maximum forward efficiency (95.2% at rated forward load) and 95.4% reverse. Its 150–300 V input / 75–167 V output configuration is not a 48 V / 500 W efficiency comparator.

The generic complex T network and the paper-specific VF-TPS secondary equivalent are now implemented.
The next sequence is:
1. Independently review the exact circuit, current orientation and turns-ratio referral against the
   paper before mapping physical CLLC components into signed X1/X2/X3.
2. Derive and implement the paper's state inequalities and admissible switching-frequency region.
3. Evaluate the selected low-frequency boost mode over voltage/load/frequency points in both
   directions; keep state classification separate from the FHA gain trend.
4. Add constant-power ZVS, unity-gain ZVS and maximum-gain constraints before selecting k and Lr1.
5. Add device Coss/dead-time energy, peak/RMS current and thermal constraints.
6. Sweep device, resonant component, transformer and digital PFM timing tolerances.
7. Cross-check selected points against a time-domain switching model and then independent bench data.

Do not approximate missing circuit states as “inductive” or infer ZVS from FHA gain alone. Until
these steps exist, the Rust CLLC functions remain **FHA gain screening only**.

### Controller timing and frequency quantization

Tao et al. (Electronics 2026, 15(21), 4918, DOI 10.3390/electronics15214918) report a 4.8 kW
bidirectional CLLC prototype combining PFM, FHA gain constraints, monotonicity and dead-time
commutation design with high-resolution PWM. The study describes how ordinary timer counts create
coarse frequency steps at high switching frequency and uses synchronized high-resolution period
updates. This is a strong design lesson for later hardware: achievable PFM resolution, command-update
timing and jitter must be included in the model and measurement contract. Their specific DSP,
frequency range and 4.8 kW component values are not transferable to our 500 W design without
re-derivation.

## 8. Model-to-bench correlation gate

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

## 9. Recommended next experiment sequence

1. Reproduce the 500 W CLLC paper's own operating point from its full paper and supplement before using its 97.6% result as a matched comparator.
2. Reproduce a DAB SPS/EPS loss benchmark from published equations and parameter tables.
3. Create one non-ideal model for each topology with the same port envelope and power boundary.
4. Run both directions and all declared voltage/load points.
5. Compare loss budgets category-by-category; preserve missing parameters as unknown rather than zero.
6. Sweep resonant components, turns ratio, switching frequency and semiconductor choice under hard ZVS/current/thermal constraints.
7. Only after independent review, select a prototype topology and create its controlled schematic/BOM package.

## 10. Explicit claim ceiling

The Rust module is a loss-budget arithmetic and provenance layer. It is not a converter solver, a DAB or CLLC model, a waveform simulator, an optimizer, a thermal model, a safety function or a physical qualification tool. The current test cases are authored but not run in this environment. No Luminous loss values or topology ranking have been produced.
