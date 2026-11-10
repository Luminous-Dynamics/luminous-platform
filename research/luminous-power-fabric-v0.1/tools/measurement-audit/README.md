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
