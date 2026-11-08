# SyncaaS Annex G / profile clockClass execution notes 0.2

**Status:** executable synthetic base-edition reference; not equipment conformance.

## Standards boundary

- ITU-T G.8275 (01/2026) Annex G defines the common N1S/D1S → AS/UAS state machine.
- ITU-T G.8275.1 (02/2026) Annex M specifies the FTS PTP criterion using `parentDS.grandmasterClockQuality.clockClass` and `defaultDS.clockclassAvailabilityThreshold`.
- ITU-T G.8275.2 (02/2026) Annex G specifies the PTS PTP criterion using the same clockClass/Cth comparison.
- ClockClass hierarchy is inverse numeric order: smaller numeric value is better. The default Cth is 6. N1S requires the threshold to be met for the entire one-second interval; any observation worse than Cth makes that second D1S.
- The common state machine changes state at the onset of ten consecutive D1S or N1S, pauses successive one-second evaluation during maintenance, and carries the sliding window across PM-period boundaries.
- The PTP AS/UAS dataset contains UInteger32 counters with `measurementValid`, `periodComplete`, `PMTime`, `AS` and `UAS`. This fixture pack uses complete 15-minute synthetic PM periods (900 seconds).

## Exact limitations

The 08/2026 amendments to G.8275, G.8275.1 and G.8275.2 are listed as in force/prepublished, but their changes are not integrated into this benchmark contract. In particular, G.8275.1 Amendment 1 states that Annex M is enhanced. This 0.2 fixture pack therefore pins the February/January 2026 base editions only and explicitly does **not** claim Amendment 1 conformance.

G.8275.1 specifies a separate frequency-layer criterion as further study; this prototype does not score it. G.8275.2 notes the announce message rate below one message per second as further study; the prototype does not resolve that case. Missing clockClass observations are a local UNKNOWN extension, never silently mapped to D1S or N1S.

## Synthetic suite

The 11 cases cover nominal service, transient D1S run shorter than the threshold, outage and recovery, maintenance pause, a transition crossing a PM-period boundary, a missing clockClass second, disabled/unsupported optional dataset, a counter mismatch, incomplete measurement record, and a separate PTS profile case.

The primary evaluator classifies each second with an `all(values <= Cth)` rule. The reference path independently derives the same N1S/D1S result from the maximum observed clockClass. They then use distinct streaming and maximal-run state-machine algorithms. The verifier is algorithmically distinct within the same Rust crate; it is not cross-language, organizational or physical independence.

## Non-claims

No physical time-error measurement, no GM/source/receiver hardware qualification, no proof of synchronization quality, no PTP profile certification, no live NMS integration, no Amendment 1 conformance, no frequency-layer availability, no operator or regulatory approval.
