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


## Evidence-gated capability envelope

The crate exposes `capability_envelope::evaluate_capability_envelope`, a read-only evaluator
for configuration-bound, direction-specific continuous power capability. It requires current
qualification evidence, exact configuration digests, current calibration/sensor evidence, a ready
independent-protection state, fresh telemetry and non-expired qualification. It applies a declared
linear thermal derating interval and an external derating factor, then subtracts explicit conservative
measurement/model uncertainty bounds. Any hard blocker withdraws both directional capabilities.

This is an evidence-gated software screen only. It does not authenticate the supplied records,
measure junction temperature, model transient SOA, coordinate cables/busbars, operate a switch,
authorize dispatch, replace physical protection or qualify a device. Linear thermal derating is a
declared first-order policy assumption, not a universal component thermal law. See
`capability_envelope.rs` and [FIELD_POWER_PLATFORM.md](../../FIELD_POWER_PLATFORM.md).


## Evidence-gated capability envelope

The crate exposes `capability_envelope::evaluate_capability_envelope`, a read-only screen for
configuration-bound, direction-specific continuous power capability. It requires exact
configuration identity, current qualification/calibration/sensor evidence, protection readiness,
fresh telemetry and an unexpired qualification. It applies a declared linear thermal derating law
and an external derating factor, then subtracts conservative absolute measurement/model uncertainty
bounds. A failed hard gate withdraws both directional capability outputs.

The report retains configuration digests, evaluation/expiry/telemetry times and the uncertainty,
external-derating and thermal evidence references. The v0.1 JSON contract and synthetic fixture are
in `schemas/capability-envelope-v0.1.schema.json` and `schemas/capability-envelope.synthetic.json`.
The research design and benchmark programme are in [CAPABILITY_ENVELOPE_DESIGN.md](../../CAPABILITY_ENVELOPE_DESIGN.md).

This evaluator does not authenticate supplied records, estimate junction temperature, model
transient SOA, coordinate cables/busbars, operate a switch, authorize dispatch, replace physical
protection or qualify hardware. Linear thermal derating is a transparent first-order screening
assumption, not a universal component thermal law.

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
