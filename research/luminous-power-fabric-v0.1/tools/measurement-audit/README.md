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
