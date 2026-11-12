# Waveform and power-quality evidence contract v0.1

**Parent:** [Issue #438](https://github.com/Luminous-Dynamics/luminous-platform/issues/438)  
**Stack:** [Controller trace PR #436](https://github.com/Luminous-Dynamics/luminous-platform/pull/436)  
**Status:** versioned metadata schema and synthetic fixtures only. No raw waveform DSP, PQ algorithm execution, Rust execution or physical measurement is claimed.

## Two distinct record types

In-situ power-quality assessment and high-speed transient capture are different measurement products. They share event identity and evidence lineage, but have different record structures and time aggregation.

- **PQ assessment** records IEC 61000-4-30 Class A/S, nominal frequency, aggregation interval, assessment window, source dataset and processed-result digest, instrument/channel/calibration/timebase lineage, data coverage, missing aggregation intervals and precomputed parameter results.
- **High-speed transient** records raw waveform digest, sample rate/count, analog bandwidth, anti-alias filter, trigger/pre/post window, channel skew and sample-quality counters.

A Class A/S PQ meter result is not a long oscilloscope waveform. Correlate the two by event; do not conflate them in the schema.

## Standards snapshot

- [IEC 61000-4-30:2025, corrected version 2026-07](https://webstore.iec.ch/en/publication/71611), Edition 4.0: in-situ PQ methods for 50/60 Hz AC; Class A and Class S. Covers frequency, voltage, flicker, dips/swells, interruptions, transients, unbalance, harmonics/interharmonics, RVC and current parameters; Annex C/D includes 2–150 kHz conducted emissions.
- [IEC 61000-4-7:2002+AMD1:2008](https://webstore.iec.ch/en/publication/4228): harmonic/interharmonic instrumentation through 9 kHz; Edition 3 is under development.
- [IEEE 519-2022](https://standards.ieee.org/ieee/519/10677/): steady-state harmonic goals at PCC, not transient-event acceptance.
- [IEEE 1547-2018](https://standards.ieee.org/ieee/1547/5915/) / [IEEE 1547.1-2020](https://sagroups.ieee.org/scc21/standards/standards-1547-1/): DER interconnection/test procedures; product/jurisdiction-specific. The 60 Hz-based 2018 standard is not a blanket 50 Hz South African route.

IEC 61000-4-30 is an in-situ PQ method, not a substitute for high-speed inrush or switching capture. CT/PT/probe transfer functions, phase error, bandwidth, saturation and channel skew need separate evidence.

## Evidence planes

| Plane | Measures |
|---|---|
| Customer service | process/motor outcome, critical continuity, unserved service |
| In-situ PQ | frequency, voltage, dips/swells/interruption, RVC, flicker, unbalance, harmonics |
| Harmonic spectrum | voltage/current harmonics and interharmonics |
| High-speed transient | pre/post-trigger voltage/current around start, step, transfer, island and protection |
| Protection | trip/no-trip, interruption, isolation and let-through/I²t |
| Energy | real input/output, auxiliaries and storage delta at matched POCs |

## Fixtures and claim ceiling

Six synthetic examples: valid high-speed transient, valid PQ Class A, valid PQ Class S, channel-skew failure, clipped transient and expired calibration. PQ results use a distinct aggregation/measurement record and processed-result digest; transient records use the high-speed sampling/trigger capture structure.

The Draft 2020-12 schema is an interchange contract; no full validator or DSP algorithm is included in this PR. A synthetic Class A/S label is not an IEC-compliant meter or assessment.

No raw waveform DSP, RMS/FFT/THD/flicker algorithms, Mycelix receipt verification, protection let-through, physical measurement or conformity claim is implemented. No energization or fault injection is authorized.
