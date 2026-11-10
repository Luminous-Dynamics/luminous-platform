# Power Fabric measurement audit tool

Dependency-free Rust CLI and library for an explicitly narrow v0.1 task: parse the agreed six-column
capture format, check numeric/time-series integrity, integrate port energy using the trapezoidal
rule, validate the declared energy-flow direction and calculate the observed port-to-port efficiency.

This is a **read-only data auditor**, not a converter controller, device driver, protection relay,
safety tool, calibration service or qualification authority.

## Run

From this directory:

```sh
cargo test --locked
cargo run --locked -- capture.csv A_TO_B
```

The CLI prints `PARSE_AND_ENERGY_CHECKS_ONLY_NOT_QUALIFICATION` on success. This means only that
the structural, timestamp, finite-value, direction and energy-arithmetic checks passed. It does not
mean the measurements are accurate, the instruments calibrated, the device efficient or safe, or
the test procedure valid.

## CSV contract

The first row must match exactly:

```csv
time_s,port_a_voltage_v,port_a_current_a,port_b_voltage_v,port_b_current_a,case_temp_c
```

Every subsequent row must contain six plain finite numeric fields. Quoted fields, blank rows,
missing fields, extra fields, non-finite values, non-positive port voltages and non-increasing
timestamps are rejected. At least two samples are required.

**Current sign convention:** both currents are positive into the converter.
- `A_TO_B`: integrated Port A energy is input; negative integrated Port B energy is output.
- `B_TO_A`: integrated Port B energy is input; negative integrated Port A energy is output.

The tool integrates power over timestamps and does not assume a fixed sample rate. It rejects
non-positive net input/output energy and apparent efficiency above 100%. It does not remove
outliers, fill missing data, infer the declared run direction from the data, or silently repair
measurements.

## Loss-budget library

The library also exposes `loss_budget::evaluate_loss_budget`. It accepts explicit loss terms with
category, watts, standard uncertainty, evidence kind and provenance reference. It returns total loss,
derived efficiency, target loss headroom, term shares, RSS standard uncertainty and a conservative
linear uncertainty bound.

This is accounting over supplied values, **not** an electrical topology solver. The upstream model
must still derive waveforms, RMS currents, switching energies, magnetic losses and thermal states.
The code does not promote datasheet/model/simulation inputs to measurement evidence.

See [LOSS_BUDGET_AND_MODEL_PLAN.md](../../LOSS_BUDGET_AND_MODEL_PLAN.md) for equations, categories,
anti-double-counting rules and the DAB/CLLC model-to-bench plan.

## Ideal DAB SPS screen

The library also exposes `dab_sps::screen_dab_sps` and
`dab_sps::leakage_inductance_for_power_and_phase`. These functions implement the ideal steady-state
SPS power-transfer equation using a secondary voltage and leakage inductance already referred to
the primary side. Positive signed power is A-to-B; negative is B-to-A. The result is a mathematical
power/phase/inductance screen only, not a loss, RMS-current, ZVS, controller, thermal or safety model.

The equations and reference conditions are documented in
[LOSS_BUDGET_AND_MODEL_PLAN.md](../../LOSS_BUDGET_AND_MODEL_PLAN.md).

## Symmetric CLLC FHA gain screen

The crate exposes `cllc_fha::evaluate_cllc_fha_gain` for the ideal symmetric CLLC FHA normalized
gain formula using k_L, normalized switching frequency and a direction-specific Q. The Q value for
charging and discharging generally differs because the equivalent AC load resistance differs.
`cllc_fha::scan_cllc_fha_gain_curve` evaluates a strictly increasing discrete frequency grid and
classifies the **sampled** trend.

These are FHA gain screens only. They do not calculate T-network internal states, complex input
impedance, inductive/ZVS feasibility, resonant currents, Coss/dead-time commutation, losses, thermal
performance or continuous-band monotonicity. See
[LOSS_BUDGET_AND_MODEL_PLAN.md](../../LOSS_BUDGET_AND_MODEL_PLAN.md).

## Generic CLLC T-network phasor evaluator

The crate exposes `cllc_t_network::evaluate_cllc_t_network`. It takes three signed branch reactances
and the secondary bridge/load complex impedance and evaluates the two-port Y parameters, the
secondary-to-primary voltage phasor ratio and primary input impedance.

This is a generic FHA T-network calculation, not yet a complete CLLC state/soft-switching evaluator.
The caller must supply the correct signed reactances and transformer referral convention for the
exact schematic. It does not calculate secondary modulation impedance from TPS/PFM command variables,
classify PNNN/other paper states, evaluate device Coss/dead-time, or qualify ZVS/ZCS.

For Huang et al.'s specific variable-frequency triple-phase-shift (VF-TPS) method, the crate also
provides `secondary_vftps_equivalent_impedance` and `evaluate_cllc_vftps_t_network`. These use the
paper's secondary bridge / load equivalent complex impedance formula and compose it with the generic
T-network calculation. Do not apply that equivalent to SPS, EPS or other TPS controllers without
re-deriving the modulation-dependent impedance.

The T-network module also exposes `screen_cllc_low_frequency_boost_window`, which screens the
paper's selected low-frequency boost-mode interval and intersects it with controller frequency
limits. It does not classify all topology states or evaluate the paper's separate constant-power
ZVS, unity-gain ZVS and maximum-gain constraints.

The T-network module now also maps a declared physical tank (`Lr1, Cr1, Lr2, Cr2, Lm, n, fs`)
to the paper's signed reactances, reports both resonant frequencies and both magnetizing ratios, and
classifies the sixteen P/N impedance states. Near-zero values return an unknown state instead of
being forced positive or negative. PNNN is the paper's selected low-frequency boost state, but is
not a standalone ZVS proof or design qualification.

## DAB SPS exact-current and resistive-periodic screens

The crate now exposes:
- `dab_sps_physics::evaluate_dab_sps_physics` for the published lossless piecewise SPS power,
  RMS/peak current, FHA error and analytic switching-current-polarity boundary.
- `dab_sps_physics::screen_dab_sps_lumped_thermal` for a declared one-node thermal estimate using
  user-supplied equivalent resistance, hard-switching loss, residual ZVS factor and thermal path.
- `dab_sps_resistive::evaluate_dab_sps_resistive` for exact four-segment RL current integration
  and one-period steady-state periodicity with a lumped nonzero series resistance.

The resistive solver checks periodic current closure and energy balance. It is a bridge-square-wave
RL model, not a semiconductor switching or magnetic model. Both modules are read-only calculation
tools; neither commands hardware nor establishes a real-device ZVS condition.

## CLLC physical tank and PNNN screen

The crate exposes `map_cllc_tank_reactances`, `classify_cllc_operating_state`, and
`screen_cllc_pnnn_low_frequency_boost_state`. Together these map the physical resonant tank to the
paper's T-network, classify all sixteen P/N sign states, check equal-`fr` / equal-`k` symmetry within
a declared tolerance and evaluate the actual switching frequency against the controller-intersected
low-frequency boost window.

A PNNN/window pass is only this paper-specific operating-state screen. It does not test FHA gain,
constant-power or unity-gain ZVS boundaries, maximum gain, device commutation energy, losses, thermal
limits or hardware safety.

## DAB mission profile and Pareto sweep

The crate exposes `dab_mission_sweep::evaluate_dab_design_candidate` and
`evaluate_dab_design_set`. It evaluates every frozen, weighted operating point using the ideal SPS
phase screen, lossless exact-current formulas and resistive periodic RL model. It records delivered
power at the correct receiving port in both directions, efficiency, loss, current, FHA error,
analytic ZVS-polarity coverage and a one-node thermal estimate. Infeasible points are retained with
explicit failure reasons.

A Pareto frontier is returned only among candidates feasible at every declared mission point. It
keeps mission efficiency, power-delivery ratio, analytic ZVS-polarity coverage, peak current and
thermal margin as separate objectives. This is not a product optimizer: the models remain lumped,
idealized and uncorrelated with real hardware.

## CLLC parameter-design constraint evaluator

The crate exposes `cllc_design::evaluate_cllc_parameter_design`. It calculates the paper-derived
symmetry precondition, low-frequency boost window, constant-power quality factor, critical-Q
boundary, constant-power inductance limit, unity-gain ZVS inductance limit and maximum-gain boundary
from Eq. (10), (12)–(16), (17)–(18), (20), and (22)–(24).

It rejects inputs that violate the symmetric resonant-frequency / magnetizing-ratio assumptions
instead of forcing them into the paper's model. Passing the resulting screen is **not** a physical
ZVS or hardware qualification: device Coss, dead-time, current waveforms, parasitics, thermal limits
and time-domain switching still need separate analysis and measurement.

See [LOSS_BUDGET_AND_MODEL_PLAN.md](../../LOSS_BUDGET_AND_MODEL_PLAN.md) for equations and the
synthetic arithmetic fixture.

## End-to-end conversion-chain energy auditor

The crate exposes `energy_chain::evaluate_energy_chain` for serial facility/source-to-load
conversion chains. A scenario supplies ordered stages, a complete 8,760-hour piecewise load profile,
and the exact per-stage efficiency / input-side standby value for each operating segment.

The evaluator works backwards from delivered load power:

`P_stage,in = P_stage,out / eta_stage + P_stage,standby,input`

It then reports facility input energy, delivered load energy, total chain losses, stage-by-stage
energy balances, annual cost in an explicit three-letter currency, and annual grid emissions using
a caller-supplied carbon-intensity factor. It retains separate source references for every efficiency,
standby, tariff and emissions input. It supports an explicit idle segment with zero delivered load
and nonzero equipment standby draw; unrepresented annual hours are rejected.

`energy_chain::compare_energy_chain_reports` compares baseline and candidate reports only when
annual hours, useful delivered energy, tariff/currency and grid-carbon intensity match. It returns
positive savings when the candidate needs less source energy, costs less or emits less. This prevents
comparing different load profiles or tariffs as though they were a technology improvement.

The tool does not infer or interpolate efficiency maps, does not automatically add busbar/cable,
cooling/storage/availability losses, and does not propagate uncertainty. Add these as separate,
provenance-bearing stages or separate validated models. It is an architecture-comparison aid, not a
site measurement, data-centre guarantee or OCP compliance test. See the synthetic
[energy-chain example](../../schemas/energy-chain.example.json).

## ISOP output-parallel current-sharing screen

The crate exposes `isop_sharing::evaluate_isop_output_sharing` for a static, linear Thevenin/droop
approximation of output-parallel cells. Each cell supplies a no-load voltage setpoint and an effective
positive output resistance. The solver returns the common bus voltage and signed per-cell current:

```
I_i = (E_i - V_bus) / R_i
V_bus = (sum(E_i / R_i) - I_load) / sum(1 / R_i)
```

It reports current-share deviation, peak-to-peak current spread, current-balance residual and any
cell whose modeled current is negative (backfeed into that cell). At zero load it reports circulating
current in amperes and leaves relative share deviation undefined.

This is an algebraic steady-state screen only. It does not model the input-series voltage split,
closed-loop interaction, negative incremental impedance, switching ripple, load transients,
current limiting, cell dropout, fault propagation or protection. The positive output resistance is
a declared static droop approximation, not a measured dynamic impedance. A backfeed flag is a
review trigger, not evidence that a unidirectional cell can safely absorb reverse power.

## DC fault-energy and discharge screens

The crate exposes `dc_protection::evaluate_dc_fault_energy` to keep selected DC fault-boundary
energy terms separate: capacitor energy (0.5 C V²), cable magnetic energy (0.5 L I²), and a
rectangular source-contribution bound (V I t). The sum may over-count overlapping energy paths and
is not a fault-current waveform.

`dc_protection::screen_dc_link_discharge` evaluates a first-order constant-R / constant-C
discharge screen. It reports energy to dissipate, maximum initial current and resistor power,
worst-case time to a caller-declared residual-voltage threshold, and numeric margins against
caller-supplied limits. It rejects partial source bounds, invalid domains, non-finite values and
positive inputs that underflow to zero.

These functions do not calculate arc-flash incident energy, breaker/fuse coordination, selectivity,
interrupting capacity, safe discharge, touch voltage, insulation coordination or service authorization.
Resistor pulse ratings must be evaluated against the actual pulse curve and applicable derating.
They are not protection relays or hardware controls.

See [DC_PROTECTION_ENGINEERING.md](../../DC_PROTECTION_ENGINEERING.md) and
[HVDC_BENCHMARK_UPDATE_2026-11-10.md](../../HVDC_BENCHMARK_UPDATE_2026-11-10.md).

## ISOP input-series voltage-allocation screen

The crate exposes `isop_input_balance::evaluate_isop_input_voltage_balance` for a static
energy-balance allocation of input voltage in an input-series string.

All active cells share the same input current. For each active cell:

```
P_in,i = P_out,i / eta_i + P_quiescent,i
V_in,i = P_in,i / I_series
```

The function solves the common series current so active-cell input-voltage drops plus explicitly
declared bypass drops sum to the total bus voltage. It preserves per-cell input voltage, power,
limits, chain efficiency and conservation residuals. Bypassed cells must be explicitly marked and
must declare zero output power; their bypass drop is a caller-supplied parameter.

A voltage-limit violation is returned as a visible false status rather than clipping the calculated
voltage. A bypassed-cell result assumes an external, separately qualified bypass path; the code
does not design or validate that path.

This is a static allocation screen, not a cell-voltage balancing controller, stability model, or
protection study. It assumes per-cell output powers are already known and feasible. It does not
model control-loop coupling, input capacitance dynamics, constant-power instability, switching
ripple, input source impedance, transients, or physical fault behavior.

## Phase-interleaved ripple harmonic screen

The crate exposes `interleaved_ripple::evaluate_interleaved_ripple`. It accepts phase offsets and
per-phase peak ripple-current amplitudes for declared switching harmonics. For harmonic h, it sums
the phase phasors using h times the fundamental phase offset. It reports the combined peak and RMS
amplitude for each harmonic, total RMS by root-sum-square and a conservative sum-of-harmonic peak
envelope.

The screen represents ideal cancellation and first-order sensitivity to phase timing and amplitude
mismatch. It does not reconstruct actual switch waveforms or model dead time, PWM / resonant
modulation, parasitic ringing, ESR / ESL, capacitor thermal stress, EMI or layout. Do not treat its
RMS result as a capacitor rating or qualification result.

## DC rail conductor-loss screen

The crate exposes `dc_rail_path::evaluate_dc_rail_path` for steady two-wire resistive distribution
losses at a constant-power load. It returns current, source voltage, conductor drop, I²R loss,
path efficiency and maximum loop-resistance budgets for declared voltage-drop and loss limits.

Use a separate measured or justified loop resistance for each rail architecture. The demonstration
that 6 kW corresponds to 120 A at 50 V, 480 A at 12.5 V and 960 A at 6.25 V does not imply equal
conductor geometry or equal loop resistance. This is a DC resistive screen, not a thermal, EMI,
transient or installation qualification.

See [GRID_TO_CHIP_BENCHMARKS_2026-11-10.md](../../GRID_TO_CHIP_BENCHMARKS_2026-11-10.md).

## Grid-to-chip time-aligned workload trace contract

The research branch adds `schemas/grid-to-chip-trace-v0.1.schema.json` and a deliberately synthetic
example. The contract binds one architecture and workload identity to source/load boundaries, a
clock domain, nominal sample interval, synchronization uncertainty, channel evidence kind,
measurement uncertainty, and time-stamped samples across idle / steady / burst / transition /
recovery states.

The trace carries source-bus voltage/current and processor-rail voltage/current so a future auditor
can integrate the entire declared boundary using synchronized samples. Physical-measurement channels
require a calibration reference and measurement bandwidth in the schema. The example is explicitly
marked `SYNTHETIC_EXAMPLE`; it is not a measurement or product claim.

JSON parsing of the schema and example and basic example invariants (unique channel IDs and
strictly increasing timestamps) were checked. A JSON Schema validator and a runtime trace auditor
have not yet been run or implemented. Schema validation alone cannot prove timestamp monotonicity,
clock synchronization, calibration authenticity, or measurement bandwidth sufficiency.

See [GRID_TO_CHIP_BENCHMARKS_2026-11-10.md](../../GRID_TO_CHIP_BENCHMARKS_2026-11-10.md).

## Time-aligned grid-to-chip trace auditor

The crate exposes `power_trace::audit_grid_to_chip_trace` over a typed in-memory trace. It
integrates source-bus input power and processor-rail delivered power with the trapezoidal rule over
strictly increasing timestamps, including irregular sample intervals. It reports total input and
processor energy, maximum sample gap, interval deviation, workload-state durations, source/load
peak power and processor-rail voltage excursion.

The initial/final stored-energy values are mandatory metadata. The auditor computes:

```
delta_E_stored = E_stored,final - E_stored,initial
E_available    = E_source - delta_E_stored
E_loss         = E_available - E_processor
eta_adjusted   = E_processor / E_available
```

A depleted hold-up store can produce a raw processor-energy / source-energy ratio above 100%.
That raw ratio is reported separately; the storage-adjusted efficiency is the comparison value.
The auditor rejects negative energy balance beyond a small numerical tolerance, non-monotonic
timestamps, invalid-quality samples, non-finite / wrong-sign values and sample gaps beyond the
declared limit. It never inserts missing samples or interpolates gaps. A synchronization-budget or
processor-rail tolerance violation remains visible in the report rather than being hidden.

This is **typed in-memory analysis**, not yet a JSON ingestion parser. The JSON interchange draft is
`schemas/grid-to-chip-trace-v0.1.schema.json`; its schema / example have been JSON-parsed and basic
sample invariants checked, but JSON Schema validation and calibration authenticity checks remain
unperformed. The trace auditor does not model switching dynamics, converter control loops, thermal
response, capacitor ESR/ESL, protection or workload scheduling.

## Processor-rail transient auditor

The crate exposes `transient_audit::audit_rail_transient` for a declared load-step event at a
processor rail. The caller supplies exact sample timestamps for a pre-step baseline window, step
start, steady-state current window and evaluation end, plus the declared rail target, voltage
tolerance band, expected post-step current, current tolerance, settling dwell, sample-gap limit
and clock-synchronization budget.

The auditor reports:
- Pre-step and post-step average load current and the measured current-step amplitude.
- A first-order scalar target-impedance budget derived from allowed voltage deviation divided by declared current-step amplitude; this is not a measured or frequency-dependent impedance curve.
- Post-step current error against the declared target.
- Maximum sampled upward and downward current slew (A/µs).
- Minimum / maximum post-step rail voltage, undershoot, overshoot and maximum deviation.
- Settling time only where the rail enters the declared band and remains there through the
  evaluation end for at least the declared dwell.
- Maximum sample gap and clock-synchronization budget status.

No interpolation or filtering is performed. Slew is the largest adjacent-sample difference, not
an estimate of an unobserved edge between samples. A settling result is therefore specific to
the declared sample bandwidth, timing, tolerance and dwell.

The November 2026 industry reference discussed in
[VRM transient and vertical power benchmarks](../../VRM_TRANSIENT_AND_VERTICAL_POWER_BENCHMARKS_2026-11-10.md)
is a 40 A to 360 A load step at 800 A/µs on a 16-phase regulator. This is a published example,
not a universal CPU/GPU rail requirement and not a Luminous result.

This screen is not a control-loop stability analysis, impedance scan, full-bandwidth power-integrity
qualification, VRM vendor comparison, or processor compliance test.

## Explicit limitations

- The CLI direction is supplied as an argument; it is not yet bound to or cross-checked against the
  JSON run manifest.
- SHA-256 digest verification, JSON Schema validation, calibration verification and instrument
  provenance are not implemented in this first slice.
- The tool does not determine whether the capture is truly steady-state or whether sampling
  bandwidth/resolution was sufficient.
- It does not estimate measurement uncertainty.
- It does not qualify thermal equilibrium, ripple, transients, protection, insulation, EMC, reliability
  or grid behavior.
- CSV parsing intentionally accepts only the simple fixed-column format; it is not a general RFC 4180
  parser.
- No physical measurement is included in this repository.

Every accepted output must be combined with the exact manifest, raw-file digest, calibration
evidence, analysis build identity, test-procedure revision and independent review before it can
support a declared bench claim.
