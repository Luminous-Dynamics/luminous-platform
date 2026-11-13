# Uncertainty-bounded timebase evidence and causal event ordering — v0.1

**Tracking:** [Issue #468](https://github.com/Luminous-Dynamics/luminous-platform/issues/468) · [Command-intent PR #460](https://github.com/Luminous-Dynamics/luminous-platform/pull/460) · [Benchmark passport PR #430](https://github.com/Luminous-Dynamics/luminous-platform/pull/430)

**Status:** synthetic/offline temporal qualification evaluator. It does not implement or configure PTP and does not prove physical time traceability.

## Standards context (2026-11-13)

- IEEE 1588-2019 is active and supports precise network clock synchronization in sub-microsecond regimes when a suitable profile/network is engineered; the IEEE catalogue records multiple amendments, including 1588h-2026: https://standards.ieee.org/ieee/1588/6825/
- IEC/IEEE 61850-9-3:2016 remains the published utility PTP profile in the IEEE catalogue. Edition 2 is still a revision project: IEEE lists an active PAR; the IEC national mirror records the CD approved for registration as DIS on 2026-10-29. Do not cite Edition 2 as final/published: https://standards.ieee.org/ieee/61850-9-3/6167/ · https://standards.ieee.org/ieee/61850-9-3/10586/ · https://dps.gov.al/sq/project/show/iec%3Aproj%3A142427
- IEEE C37.118.1 synchrophasor timing depends on a common UTC time base. Its PMU-specific accuracy requirements must not be indiscriminately imposed on every Power Fabric sensor: https://www.ons.org.br/AnexosMPO/A5_C37.118.1-2011.pdf

## Paradigm

Every event is represented by an uncertainty-bounded interval on TAI. A is provably before B only if upper(A) is strictly less than lower(B). Overlapping intervals are indeterminate, even if their point timestamps sort neatly. This prevents event logs from inventing causal order or reporting a response-time win smaller than the combined time uncertainty.

The event interval includes clock error at last synchronization, drift growth since that sync, timestamp quantization and a conservative physical-capture delay bound. UTC events additionally require a current UTC-to-TAI offset record valid at the event time. Holdover is permitted only inside a declared maximum age and its drift bound grows with elapsed time.

Events from different clock domains or epochs require a current directional clock-relation receipt with offset uncertainty and relative-drift bound. An expired mapping, unknown clock state or lost sync fails qualification. Same-domain events must use the same clock configuration and maintain consistency with monotonic local sequence.

## Duration claims

For start interval S and end interval E:
- minimum duration = max(0, lower(E) - upper(S));
- maximum duration = upper(E) - lower(S).

A positive response-time claim is established only when the minimum duration is strictly positive. If intervals overlap, minimum duration is zero and the response-time claim is not established.

## Synthetic corpus

Ten fixtures cover disjoint order, overlapping interval, holdover drift, lost clock, missing cross-domain map, expired mapping, UTC conversion, sequence contradiction, overlapping response duration and synthetic evidence. Rust tests use deterministic synthetic clocks and nanosecond bounds.

## Ownership and claim ceiling

Platform evaluates typed temporal intervals. Mycelix verifies signed timebase/mapping receipts, profile identity and revocation. Symtropy provides controller/plant event histories. Sol-Atlas owns instrument/device identity and network topology. Qualified metrology and lab owners establish time traceability, channel latency, mapping bounds and uncertainty. No tool here adjusts a clock or controls equipment.

`TEMPORAL_EVIDENCE_QUALIFICATION_ONLY_NOT_CLOCK_SYNC_OR_PHYSICAL_RESPONSE_CLAIM`


## Validation status and claim ceiling

- 12 Rust unit tests are authored, not executed.
- JSON schema and synthetic fixture parse in the authoring runtime; fixture case IDs and count were checked.
- No Rust compiler/Cargo, formatting, Clippy or exact-head workflow pass is claimed.
- This module performs declared-bound arithmetic only. It does not query PTP, verify grandmaster identity, validate hardware timestamping, retrieve leap-second bulletins, calibrate instrument latency, or prove time traceability.
- It does not adjust clocks or issue equipment commands. No physical response-time, PMU accuracy, HIL or IEC/IEEE conformity claim is made.

Claim ceiling: `TEMPORAL_EVIDENCE_QUALIFICATION_ONLY_NOT_CLOCK_SYNC_OR_PHYSICAL_RESPONSE_CLAIM`.


## Validation status

- 12 Rust unit tests are authored, not executed.
- The JSON schema and ten-case synthetic fixture parse in the authoring runtime; case IDs and required schema fields were checked.
- No Rust compiler/Cargo, formatting, Clippy or exact-head workflow pass is claimed.
- No PTP packets, clock configuration, grandmaster, leap-second source, PMU, HIL or physical instrument was accessed. Clock evidence values and time mappings remain declared test inputs.
- The evaluator does not authenticate profile identity, verify clock traceability, measure path asymmetry, calibrate instrument delay or prove independent clocks.

Claim ceiling: `TEMPORAL_EVIDENCE_QUALIFICATION_ONLY_NOT_CLOCK_SYNC_OR_PHYSICAL_RESPONSE_CLAIM`.
