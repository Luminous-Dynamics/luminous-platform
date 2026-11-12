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



## Common-cause-aware multi-path capability

The crate exposes `multi_port_capability::evaluate_multi_path_capability`, a deterministic,
read-only composition of already-qualified parallel source-to-service paths. All route powers must
terminate at one exact useful-service boundary. Each path retains its configuration SHA-256 and
upstream capability-evidence reference.

The evaluator:
- automatically evaluates every declared component and shared failure-domain loss;
- supports explicit compound contingencies;
- removes all routes that depend on a failed, unknown or otherwise unavailable component/domain;
- applies shared throughput ceilings for declared buses, feeders, cooling resources or other common
  bottlenecks;
- recomputes shared capacity after each contingency; and
- rejects partially overlapping non-identical shared-capacity path groups rather than applying an
  unsafe arithmetic approximation.

The robust capability is the minimum surviving capacity across the baseline and all enumerated
scenarios. Shared resources retain their configuration and evidence references in the result.

This is a declared-contingency and shared-capacity arithmetic screen, not an electrical network
solver, fault-current model, protection coordination test, reliability-probability estimator or
minimal-cut-set discovery engine. It does not authenticate path evidence, infer missing dependencies,
calculate serial-component bottlenecks or authorize switching. Each route's input power must already
be bounded by the upstream capability and engineering assessment. See
`multi_port_capability.rs`, the `schemas/multi-path-capability-*` contracts and
[MULTI_PORT_CAPABILITY_DESIGN.md](../../MULTI_PORT_CAPABILITY_DESIGN.md).

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

## Mission energy and storage reserve envelope

The crate exposes `mission_energy::evaluate_mission_energy`, a deterministic, read-only interval-energy
accounting screen. It consumes contiguous mission segments with a required service power and an
owner-qualified interval-average source-power envelope (including source ramp/start constraints
already modelled upstream). It subtracts explicit source measurement/model uncertainty bounds, gives
source energy priority for direct service, then charges an aggregate storage bank from surplus or
discharges it to cover a source deficit.

The evaluator enforces both a mission-wide unserved-energy budget and a non-compensatory
per-segment limit (set zero for a segment that must be fully served). It limits storage by
conservative capacity, initial-SOC uncertainty, charge/discharge power, charge/discharge efficiency,
minimum SOC and an explicit end-of-mission reserve. It retains
qualified/observed source digests, source-specific expiry, evidence status and uncertainty bounds per
segment, plus the exact storage evidence/configuration/limit snapshot. Global system mismatch or expired mission qualification blocks all capability; a stale
source segment is excluded rather than trusted. If a storage bank is configured but is ineligible,
the mission cannot claim a pass because its final SOC/reserve is unknown. For a true generator-only
comparison, omit the storage object.

This is interval-energy bookkeeping only, not a dispatch optimizer/controller, BMS, electrochemical
or thermal model, generator/inverter transient simulator, AC/DC network solver, fuel calculation,
protection function or hardware qualification. The energy boundary must be identical across all
inputs. Transient motor-start, power-quality, protection and fault response remain separate tests.
See [MISSION_ENERGY_ENVELOPE.md](../../MISSION_ENERGY_ENVELOPE.md), the
`schemas/mission-energy-envelope-v0.1.schema.json` contract and synthetic fixture.

## Power-duration event envelope

The crate exposes `power_event_envelope::evaluate_power_event_envelope`, a deterministic, read-only
screen for duration-bounded transient service. Each event carries its required peak power and window,
source's guaranteed minimum power over that window, independent measurement/model uncertainty,
qualified/observed source digests and expiry, plus a separately qualified common output-path power
limit and evidence. Optional storage support includes event-time SOC/SOH/temperature, usable
battery-side energy and uncertainty, qualified event power/duration, discharge efficiency and
calibration/sensor/protection evidence.

The evaluator subtracts conservative source/common-path uncertainty, caps storage power by both its
qualified power and usable energy over the event duration, then caps the source-plus-storage sum by
the common output path. Events must be non-overlapping; critical event shortfall is tested against a
local unserved-energy limit. The storage energy snapshot must already account for earlier commitments.

This is a screening contract, not a waveform/motor/inverter model, BMS, dispatch controller,
protection function or ride-through qualification. Event energy is part of the aggregate mission load
exactly once; the event-unserved output is diagnostic only. See
[POWER_DURATION_EVENT_ENVELOPE.md](../../POWER_DURATION_EVENT_ENVELOPE.md), the
`schemas/power-event-envelope-v0.1.schema.json` and its synthetic fixture.

## Replayable benchmark evidence comparison

The crate exposes `benchmark_comparison::compare_matched_benchmark_runs`, a dependency-free,
read-only eligibility gate for two typed benchmark run records. It checks identical comparison
context (test protocol, primary/auxiliary POC, useful-service/load/event/environment/source/fuel
conditions, initial storage/reserve, reporting window, sample policy, data processing and timebase),
matching metric units/method/measurand scope, current calibration/timebase status and expiry, and
raw-data digest presence for physical measurement runs.

Each metric requires instrument references; the instrument calibration scope must cover that
measurand, and the calibration method must match across the pair. Site-operational assessments
require at least three declared representative sample events and fault/maintenance history
digests; the sample-selection and time-series preprocessing policies are bound by digest. The
result retains both run evidence-bundle and raw/history digests, matched metric deltas, and a
conservative sum of absolute uncertainty bounds.

Three gates are separate: structural matching, physical measurement eligibility and independent
claim-evidence eligibility. Synthetic/modelled/vendor runs never become physical evidence merely
because their scenario keys match. This is not a performance score, authenticity verifier, schema
parser, certificate validator, uncertainty budget or SOTA/significance test. The 0.1 JSON contract
and fixtures are under `schemas/benchmark-evidence-*`; see
[BENCHMARK_EVIDENCE_PASSPORT.md](../../BENCHMARK_EVIDENCE_PASSPORT.md).

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
- The CSV auditor does not verify file digests or certificates. The separate benchmark comparator
  checks declared SHA-256 string shape and evidence status only; it does not hash/authenticate the
  underlying files or validate calibration certificates.
- The JSON Schema is a separate external data contract; the typed Rust comparator does not parse or
  serialize JSON.
- The tool does not determine whether the capture is truly steady-state or whether sampling
  bandwidth/resolution was sufficient.
- The CSV energy auditor does not estimate measurement uncertainty; the mission-energy evaluator
  only consumes caller-supplied conservative bounds and does not calculate the uncertainty budget.
- It does not qualify thermal equilibrium, ripple, transients, protection, insulation, EMC, reliability
  or grid behavior.
- CSV parsing intentionally accepts only the simple fixed-column format; it is not a general RFC 4180
  parser.
- No physical measurement is included in this repository.

Every accepted output must be combined with the exact manifest, raw-file digest, calibration
evidence, analysis build identity, test-procedure revision and independent review before it can
support a declared bench claim.


## Topology-aware overlapping shared-capacity graph

The crate exposes `topology_capacity_graph::evaluate_topology_capacity_graph` for a read-only,
deterministic max-flow/min-cut calculation over an owner-qualified service-side capacity graph.
Unlike the v0.1 parallel-path arithmetic, a shared feeder/cooling/control resource is represented
once in the graph, so partially overlapping path dependencies are supported without summing the
same shared capacity more than once.

The graph uses integer watts normalized at one exact customer POC. Each node and arc retains a
configuration digest, evidence state, provenance reference and failure-domain labels. Missing,
expired, invalid or unverified components/arcs contribute no flow. A scenario can withdraw nodes,
components, arcs or shared failure domains. The report returns maximum service flow, an active
minimum-cut arc witness whose capacity sum must equal the flow, all blocked arcs, and the residual
source-side set.

This remains a capacity abstraction, not AC/DC power flow, EMT, voltage/current/impedance modeling,
reactive-power calculation, protection coordination, thermal coupling, or reliability probability.
The max-flow result is an upper bound under supplied, already-normalized capacity evidence. See
`topology_capacity_graph.rs` and [OVERLAPPING_CAPACITY_GRAPH.md](../../OVERLAPPING_CAPACITY_GRAPH.md).
