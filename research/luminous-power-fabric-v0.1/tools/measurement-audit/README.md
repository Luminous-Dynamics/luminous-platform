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

## Constant-import storage buffer sizing screen

The crate exposes `storage_buffer::size_constant_import_buffer` for an ideal buffer that holds
facility import at a declared constant target against a time-series load profile.

The model treats positive `load_power - grid_import_target` as storage discharge and negative
values as charging surplus. It integrates linearly interpolated load power over each sample
interval and splits intervals at exact crossings of the grid-import target, so charge and discharge
energy in the same interval are not incorrectly cancelled.

It reports load / grid energy, storage charge-input and discharge-delivered energy, charge /
discharge conversion loss, minimum required usable energy capacity, minimum initial stored energy,
terminal state, actual min/max state, peak charge/discharge power, sample gaps and limit statuses.

The model assumes instantaneous power tracking, fixed round-trip directions / efficiencies and
no SOC, voltage, current, temperature, aging, ramp, inverter, grid-forming or communications
dynamics. It is an ideal sizing lower bound, not a battery dispatch controller or grid-interactive
system qualification. Capacitors, EDLC, batteries and grid-forming storage operate at different
timescales; do not infer that one technology satisfies every layer.

The interchange draft is `schemas/facility-power-profile-v0.1.schema.json`, with a synthetic
example in `schemas/facility-power-profile.example.json`. Both JSON files were parsed and basic
example invariants (strict timestamp order, sample quality and maximum gap) were checked. JSON
Schema validation and a runtime JSON parser have not been run or implemented.

See [multi-timescale storage research](../../MULTI_TIMESCALE_STORAGE_AND_GRID_DYNAMICS_2026-11-10.md).

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


## Compute-preserving grid-to-chip benchmark

The library exposes compute_power_benchmark:

- audit_compute_power_trace integrates PCC active energy and useful-work rate on a time-aligned trace. It corrects input energy for initial/final stored-energy change, and reports time-weighted RMS power fluctuation, RMS tracking error against a declared reference, peak deviation and up/down ramp.
- compare_compute_power_traces requires the same workload, boundary, work unit, PCC reference, initial stored energy, sample count and exact timestamps. It reports useful-work retention separately from energy-per-work and PCC power-shaping metrics. A smoother trace is not an unqualified improvement if it misses the useful-work budget.
- audit_multisite_load_correlation requires synchronous valid site traces with identical timestamp vectors. It reports time-weighted site variance/covariance and compares aggregate variability against the zero-covariance independent reference.

The multi-site v0.1 calculation is zero-lag only. It does not estimate lagged correlation, spectral coherence, grid frequency response or network stability. The trace audit is read-only and does not control workload, converter, storage or protection systems.

**Validation:** tests are authored but not executed. No Rust compilation, formatting, Clippy, schema validation, EMT, HIL/PHIL, physical measurement or certification is claimed. See [COMPUTE_PRESERVING_GRID_TO_CHIP_BENCHMARK_2026-11-10.md](../../COMPUTE_PRESERVING_GRID_TO_CHIP_BENCHMARK_2026-11-10.md).


## Frequency-dependent PDN target and impedance audit

The library exposes pdn_frequency::audit_pdn_frequency_response for an explicit frequency-grid comparison between processor current-ripple RMS bins and supplied complex self-impedance points. It derives Z_target(f) from a per-bin voltage-ripple allocation, verifies the RSS allocation against the available AC ripple budget, applies a caller-declared conservative uncertainty bound to impedance magnitude, reports the worst frequency/ratio and flags sampled local impedance peaks as anti-resonance candidates.

The caller must provide a valid, orthogonal/non-overlapping RMS spectral decomposition and impedance data at the same reference plane. The frequency vectors must match exactly; v0.1 does not interpolate, extrapolate, smooth, de-embed or parse S-parameters. Evidence class, calibration-reference presence, instrument bandwidth, configuration and de-embedding state remain explicit. A numerical screen is not loop-stability, package or product qualification.

**Validation:** nine new Rust tests are authored but not executed. No compile, format, Clippy, field solver, Touchstone parser, de-embedding, physical measurement or certification is claimed. See [PDN_FREQUENCY_DOMAIN_TARGET_AND_ANTIRESONANCE_2026-11-10.md](../../PDN_FREQUENCY_DOMAIN_TARGET_AND_ANTIRESONANCE_2026-11-10.md).


## Multiresolution sampling adequacy

The library exposes sampling_adequacy::audit_sampling_adequacy for a timestamp vector, declared measurement bandwidth, claimed signal bandwidth, samples-per-cycle requirement and anti-alias filter contract. It uses the largest observed timestamp gap to calculate a conservative equivalent sample rate and Nyquist frequency, reports maximum interval jitter and explicitly rejects claims that exceed sample density, instrument bandwidth or declared anti-alias filter limits.

This is a sampling/bandwidth metadata screen. It does not validate a physical sensor, calibration, probe loading or actual analog-filter response. The 50 Hz AI workload trace fixtures deliberately fail when used to support a sub-microsecond VRM transient claim.

**Validation:** eight Rust tests are authored but not executed. No workload dataset adapter, archive parsing, field measurement, physical test or certification is claimed. See [MULTIRESOLUTION_AI_WORKLOAD_TRACE_AND_SAMPLING_ADEQUACY_2026-11-10.md](../../MULTIRESOLUTION_AI_WORKLOAD_TRACE_AND_SAMPLING_ADEQUACY_2026-11-10.md).


## Configuration-bound evidence manifests and requalification

The library exposes evidence_manifest::validate_evidence_manifest for typed, evidence-class-aware metadata checks; compare_computed_artifact_digest for comparing an externally calculated SHA-256 value against a declared digest; validate_evidence_graph for dependency-reference/cycle checks; and requalification_closure for finding the transitive evidence set affected by a configuration revision change.

Physical-measurement records require calibration, instrument IDs/bandwidth, reference plane, procedure and clock metadata. Simulation records require model, solver, parameter-set and boundary-condition identity. Independent-lab and certification records require an explicit external attestation reference.

This module does not parse JSON, open/hash artifact files, verify signatures, authenticate issuers, validate calibration certificates or qualify equipment. It intentionally does not implement cryptography. See [EVIDENCE_MANIFEST_AND_REQUALIFICATION_GRAPH_2026-11-10.md](../../EVIDENCE_MANIFEST_AND_REQUALIFICATION_GRAPH_2026-11-10.md) and [evidence-manifest-v0.1.schema.json](../../schemas/evidence-manifest-v0.1.schema.json).

**Validation:** nine Rust unit tests are authored but not executed. JSON Schema validation, compilation, formatting and Clippy are not claimed.


## License-aware AI workload dataset registry

The library exposes dataset_registry::validate_dataset_registry and DatasetRegistryEntry::decide_use. Registry entries carry source/version/availability/evidence class, physical boundary, file metadata, sample interval, sensor bandwidth, anti-alias verification, supported analysis uses and explicit dataset licence permissions. A permission is not inferred from the existence of a download link.

Use decisions are fail-closed:
- unverified licence permissions return Unknown;
- pending/planned datasets return Unavailable;
- access-controlled datasets require a separate access grant;
- explicit CC BY-NC-ND restrictions deny commercial use and derivative creation;
- public CC BY datasets permit declared use only with the recorded attribution/notices.

Fast transient eligibility requires physical measurement evidence, known sample interval, verified timestamp-jitter and clock synchronization, verified sensor bandwidth, verified anti-alias filter and a declared signal bandwidth within both max-gap-derived Nyquist and sensor limits. The max-gap-derived Nyquist value is a uniform-equivalent metadata screen, not a general theorem for arbitrary nonuniform sampling. A phrase such as "high-rate trace" does not qualify a dataset.

The registry example separates the NLR measured workload traces from its model-generated facility profiles, splits the Elsayed node and single-machine sampling paths, and records ELF, ORNL Frontier, LLM-PowerTrace and Argonne's planned flexibility corpus with their respective release and licence states.

**Validation:** twelve Rust tests are authored but not executed. The registry JSON and schema are present; JSON parsing is not JSON Schema validation. No dataset archive was downloaded or parsed, and no source-specific adapter is claimed. See [DATASET_REGISTRY_AND_POWER_FLEXIBILITY_BENCHMARK_2026-11-10.md](../../DATASET_REGISTRY_AND_POWER_FLEXIBILITY_BENCHMARK_2026-11-10.md).


## QoS-constrained power-flexibility Pareto audit

The library exposes power_flexibility::evaluate_power_flexibility_sweep for offline comparison of a frozen baseline with controlled power-cap/DVFS/workload operating points. Every candidate must match the baseline workload contract, request mix, quality contract, PCC boundary, time window and initial stored energy. Mean PCC power is derived from integrated PCC energy over the exact window, avoiding a second inconsistent mean-power input. PCC energy is corrected for the initial/final stored-energy change.

The result reports power shed, useful-work retention, P99 latency/TTFT changes, SLO violation fraction, energy per work change, P99 ramp, response latency to target, power overshoot, recovery settling and recovery rebound. Service-contract and minimum-power-shed gates are evaluated separately. The Pareto frontier contains only non-dominated candidates that pass the declared useful-work, QoS, energy/work, sustained-duration and rebound constraints.

This is a read-only analysis function. It does not send GPU power-cap, DVFS, request admission, workload scheduling, storage or grid-dispatch commands. It assumes the input measurements have already passed the upstream measurement contract; the evidence IDs are references, not independently authenticated by this module.

**Validation:** twelve Rust tests are authored but not executed. No Rust compilation, format, Clippy, live GPU experiment, workload control, PCC measurement, EMT/HIL/PHIL or grid-service qualification is claimed. See [POWER_FLEXIBILITY_VS_QOS_PARETO_BENCHMARK_2026-11-10.md](../../POWER_FLEXIBILITY_VS_QOS_PARETO_BENCHMARK_2026-11-10.md).


## Fleet-capacity-aware Joule Point audit

The library exposes fleet_capacity::evaluate_fleet_capacity_sweep to compare fixed-fleet and capacity-scaled scenarios from matched per-GPU operating points. Capacity-scaled GPU count is rounded up to an integer using offered goodput plus declared headroom. It reports GPU-board power separately from total-system power; total-system calculations require explicit per-GPU support plus separate fixed-fleet and capacity-scaled fixed overhead with measured/modelled evidence.

The evaluator separates capacity feasibility, completed-useful-work feasibility and QoS feasibility. Storage correction applies to the declared total-system energy boundary, not to GPU-board component energy. Optional capex requires a currency and cost-source reference; unknown cost remains unknown. Capacity-scaled goodput is a linear projection, not measured multi-GPU performance.

**Validation:** eleven Rust tests are authored but not executed. No compilation, formatting, Clippy, multi-GPU physical experiment, fleet scheduler or grid qualification is claimed. See [FLEET_CAPACITY_AWARE_JOULE_POINT_BENCHMARK_2026-11-10.md](../../FLEET_CAPACITY_AWARE_JOULE_POINT_BENCHMARK_2026-11-10.md).

## Repeated-event grid-interactive compute qualification

The library exposes grid_service::evaluate_grid_service_sweep for a preregistered, location-specific compute-derived demand-response contract. It keeps market-serving, grid-serving and system-serving demand response distinct; it does not qualify inverter services or grid-code compliance.

For each event, the evaluator computes site/PCC power reduction, subtracts independently metered non-compute load reduction and subtracts conservative absolute uncertainty bounds. It then applies response, sustain, useful-work, P99 latency/TTFT, SLO, recovery-observation and rebound gates. Events failing a service gate receive zero credited capacity.

Each report retains the frozen service contract, baseline and event evidence IDs, and exact event/recovery nanosecond timestamps. Recovery begins exactly at dispatch end (no unobserved gap), and dispatch/recovery intervals for one resource cannot overlap. The sweep reports event success fraction, nearest-rank P10 credited compute capacity and a one-sided Wilson lower confidence bound when an independent-trial assumption is explicitly declared (z=1.2815515655 for one-sided 90%). The Wilson bound remains conditional on a representative, independent event design; the declaration is metadata, not proof. The calculation rejects confidence parameters or derived arithmetic that overflow. A single event, post-hoc baseline, missing rebound data, unmeasured non-compute contribution or inadequate event count cannot qualify a resource commitment.

See [GRID_INTERACTIVE_COMPUTE_RESOURCE_QUALIFICATION_2026-11-11.md](../../GRID_INTERACTIVE_COMPUTE_RESOURCE_QUALIFICATION_2026-11-11.md) for the attribution equations, literature comparison, test targets and limitations.

**Validation:** nineteen synthetic Rust tests are authored but not executed. No compilation, formatting, Clippy, live dispatch, site measurement, grid participation, market capacity accreditation or certification is claimed.

## Deterministic workload replay and counterfactual flexibility

The library exposes workload_replay::evaluate_workload_replay. It compares the same normalized jobs under a full-capacity baseline and a time-varying dispatch admission-capacity profile. Each job declares CPU milli-core, GPU unit and memory-MiB demand, release/latest-start/completion timestamps, a non-preemptive runtime, a useful-work count, and source evidence. The replay contract preserves dataset version/evidence, a use-authorization reference, normalization revision, scheduling-policy revision and work unit.

The v0.1 policy is deterministic strict priority: non-deferrable before deferrable, then earliest latest start, completion deadline, release time and stable job ID. It is no-backfill, non-preemptive, event-driven, and dispatch windows constrain new admissions only. Ordered release, ready and completion sets plus incremental resource accounting avoid full-workload scans at each event.

Outputs include per-job baseline/candidate schedules, completion and miss counts by deferrability class, useful-work retention, delay percentiles, recovery tail, and CPU/GPU/memory resource-time during the same dispatch windows. The v0.2 report also emits exact half-open occupancy segments and timestamped useful-work completion events for both runs, preserving clock ID/origin, workload lineage and a caller-supplied schedule-digest reference. Segment resource-time must reconcile with direct per-job resource-time; completion-event work sums must reconcile with run summaries. The capacity-restriction tensor reports CPU-millicore-ns, GPU-unit-ns and memory-MiB-ns separately. None of these units are watts, joules or grid capacity. The schedule event stream is intended to feed #382; it does not duplicate PCC energy integration. The module does not parse source trace files, resolve DAG dependencies, model per-job fragments or infer power.

Twenty synthetic Rust tests are authored but not executed, including deterministic replay, deadline cliffs, in-flight continuity, exact half-open boundary ties, multi-resource capacity restrictions, occupancy/completion conservation negative controls, provenance fail-closed checks, overflow and a 10,000-job fixture.

## Calibrated workload-state power envelopes (issue #401)

The `power_state` module fits empirical state-conditioned and ordered one-step-transition power envelopes from measured traces. Its calibration contract binds an exact hardware configuration fingerprint, physical boundary, sensor/calibration evidence, dataset rights reference, workload contract, state mapper, clock-alignment method and sample period. It splits complete acquisition sessions into training and held-out sets; the fitter records held-out IDs but never adds held-out samples to the fitted distributions.

Prediction accepts an independently evidenced, time-aligned state sequence. It emits the training envelope's median and p10–p90 range in the exact input order, widened by declared sensor uncertainty. Every subsequent sample requires a supported previous-state/current-state pair. Unknown configuration, state or transition support fails closed. Missing, saturated, unsynchronized, calibration-invalid, irregular or out-of-order samples are rejected rather than silently removed. The module does not infer workload phase from occupancy.

The held-out evaluator requires exact run, timestamp, cadence, state and sample-count alignment and reports integrated power-time error, mean/peak power error, signed-ramp error, lag-one autocorrelation mismatch, interval coverage and interval width. This is a model-validation layer only; predicted output remains MODELLED, measured input remains MEASURED, and issue #382 remains the sole PCC energy auditor.

See [the benchmark contract](../../CALIBRATED_WORKLOAD_STATE_POWER_MODEL_BENCHMARK_2026-11-11.md). Ten synthetic Rust tests are authored but not executed. No physical sensor data, licensed trace corpus, compilation, lint or runtime benchmark is claimed.

The current implementation also rejects attempts to predict a training run and independently verifies that a measured run is listed in the prediction trace's held-out evidence manifest before computing validation metrics. The mapped prediction trace must carry the exact clock-alignment method ID from the calibration contract. These prevent caller-side relabelling from bypassing the declared split or alignment method; evidence references themselves are still references, not authenticated signatures.

Each `PredictedPowerTrace` now includes the mapped source trace's own state-mapping evidence reference as well as calibration-run mapping/time-alignment evidence. This distinguishes evidence that trained the model from evidence that assigned states to the particular sequence being predicted.

The model now exposes distinct prediction APIs for held-out validation and new operational scenarios. `predict_power_trace` only accepts a run in the held-out session set. `predict_operational_scenario` accepts a new run ID not present in either training or holdout sets. Each output records a `PredictionScope` and distinct claim ceiling; held-out scoring rejects operational-scenario traces. This keeps calibration leakage checks intact while allowing #400/#403/#404 to use the calibrated model for new MODELLED counterfactual traces.

Fifteen synthetic Rust tests are authored but not executed. No held-out or scenario prediction has been run on a physical power trace.


## Regime-conditioned session conformal forecast calibration

The forecast_calibration module adds a regime-specific split-conformal correction on top of #401 predictions. Its exchangeability unit is a complete, fixed-exposure acquisition session, not a sample row. Each session contributes the maximum interval violation over every sample/trace in that session, allowing arbitrary temporal dependence inside the session.

The exact complete PowerModelContract, model revision, hardware, boundary, workload, state mapper, clock method and cadence are bound to every calibration. Calibration accepts only new OperationalScenario predictions, never #401 held-out validation traces. Calibration and monitoring session/run/trace IDs must be disjoint. Unknown regimes are OUT_OF_DOMAIN and receive no borrowed interval correction.

The fitted correction targets whole-session coverage: every sample in a session must lie inside the corrected interval. The finite-sample conformal order statistic is ceil((n+1) × target_coverage); a 90% target first has an observable rank at nine exchangeable sessions per regime. That is a mathematical minimum, not a recommended field sample size. Fixed session duration, trace count, samples per trace and minimum separation are required so session maxima have comparable exposure.

A separate monitor reports whole-session coverage, sample-level coverage as a non-independent diagnostic, per-regime Wilson lower bounds (only with a separately evidenced monitor-session independence design) and NOMINAL / DEGRADED / INSUFFICIENT_SUPPORT / SAMPLING_DESIGN_UNQUALIFIED / OUT_OF_DOMAIN states. Monitoring never mutates the frozen calibrator; recalibration requires a new version and a new calibration set.

See [REGIME_CONDITIONED_SESSION_CONFORMAL_FORECAST_CALIBRATION_2026-11-11.md](../../REGIME_CONDITIONED_SESSION_CONFORMAL_FORECAST_CALIBRATION_2026-11-11.md) for the statistical contract and reproduction plan for Yao et al. (6 November 2026).

**Validation:** twenty-four synthetic Rust tests are authored but not executed. No compile, Cargo test, formatter, Clippy, physical trace, dataset reproduction or runtime benchmark is claimed.


## Dependence-preserving probabilistic power scenarios and reserve sizing

The registered temporal_scenarios module adds three matched scenario arms: independent marginal-quantile sampling as a temporal-dependence negative control, complete empirical residual-trajectory bootstrap with replacement, and Schaake-shuffle rank transfer.

The residual contract binds the exact model/configuration/regime/horizon, common clock identity/time origin, historical forecast origin and complete target window, forecast/measurement/residual/state/clock evidence, source timeline, exact sample timestamps and per-sample evidence IDs. It rejects future residual windows, duplicate origins/trajectory IDs, raw sample-evidence reuse across blocks, and residual origin gaps shorter than a full target horizon. This avoids direct target-window overlap; it does not prove stationarity or independence.

The evaluator computes multivariate energy and variogram scores, peak/ramp/energy distributions and PIT diagnostics, then sizes horizon-wide reserve using the declared c_R/c_S critical fractile of maximum positive deviation from the scheduled median path. Energy is integrated from interval-average power and frozen forecast-step duration (mW·ns in audit units). This remains offline scenario analysis, not physical reserve, PCC qualification or grid accreditation.

See TEMPORAL_SCENARIOS_AND_HORIZON_RESERVE_BENCHMARK_2026-11-11.md for the 6 November 2026 Yao et al. reproduction protocol, industry comparators and MIT dataset rights gate.

Validation: 34 synthetic Rust tests are authored but not executed. No compiler, Cargo test, formatting, Clippy, runtime benchmark, dataset download or physical measurement is claimed.


**Gaussian-copula arm:** a rank-normal residual dependence estimator, frozen correlation shrinkage, deterministic Cholesky sampling and current-marginal quantile mapping are now implemented. It requires at least max(30, horizon_steps + 1) non-overlapping residual blocks, records raw residual vectors, recomputable rank scores, exact per-residual model/regime/clock/time-origin contracts, correlation/regularization/factor evidence and rejects constant dimensions. Thirty-six synthetic tests are authored but not executed. This completes the four method arms structurally; no paper reproduction is claimed.


## Paired, cluster-aware scenario decision tournament

The new \`scenario_tournament\` module compares the four scenario generators from #407/#409 on exact paired episodes. Each episode must have one report per method and all four must use the same forecast origin and timestamp, full model contract, physical measurement run, measured-sample evidence, horizon, scenario count, deterministic seed and reserve/shortfall cost ratio. The forecast origin must lie inside its preregistered episode window.

Bootstrap and Schaake carry the complete selected historical residual records. The tournament checks those records field-for-field against the same 200-row historical pool retained by the Gaussian arm; a shared pool ID cannot conceal different residual data. Bootstrap may resample pool rows with replacement; Schaake requires unique selected historical rank rows.

The independent-marginal generator is the negative-control baseline. Gaussian copula, empirical residual bootstrap and Schaake shuffle are compared against that same episode-level baseline. The evaluator checks each method's reserve/shortfall arithmetic and freezes the exact generator ID, revision, model, regime and clock contracts across the experiment.

Episodes are grouped into fixed-exposure clusters. All episodes inside one cluster may be dependent; the cluster mean cost is one statistical unit. A candidate cluster succeeds only if it achieves the preregistered relative cost saving. The tournament computes an exact one-sided Clopper–Pearson lower confidence bound over independent clusters, not episode rows.

Every required stratum is qualified separately. Confidence is Bonferroni-adjusted across the required strata; the overall pooled cost summary is descriptive and publishes no pooled binomial confidence bound. Candidate qualification requires adequate cluster support, the target mean cost saving, the same frozen shortfall ceiling and the cluster-success lower bound in every required stratum.

The initial target is at least 7.4% lower normalized reserve cost than independent marginal sampling, at the same preregistered shortfall ceiling. This is a research target motivated by the study-specific 7.4–10.6% saving reported by Yao et al.; it is not a Luminous result or a physical reserve claim.

See [SCENARIO_DECISION_TOURNAMENT_2026-11-11.md](../../SCENARIO_DECISION_TOURNAMENT_2026-11-11.md).

**Validation:** twenty-one synthetic Rust tests are authored but not executed. No compile, Cargo test, formatting, Clippy, runtime benchmark, physical measurement, grid-service qualification or accreditation is claimed.
