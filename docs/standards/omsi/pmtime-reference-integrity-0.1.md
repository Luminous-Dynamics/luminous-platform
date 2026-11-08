# OMSI PMTime reference-integrity profile 0.1

**Status:** synthetic software conformance profile; not a physical timing qualification  
**Owner:** OMSI / Luminous Platform  
**Parent work:** [PMTime reference integrity #335](https://github.com/Luminous-Dynamics/luminous-platform/issues/335)  
**Cross-domain benchmark:** [OMSI-EVAL-001 #325](https://github.com/Luminous-Dynamics/luminous-platform/issues/325)  
**Upstream timing profile:** [SyncaaS Annex G PR #329](https://github.com/Luminous-Dynamics/luminous-platform/pull/329)  
**Dependency owner:** [Mycelix temporal dependency hypergraph #5929](https://github.com/Luminous-Dynamics/mycelix/pull/5929)

## Research objective

Evaluate whether the clock and source used to timestamp a PTP performance-monitoring record are sufficiently characterized and dependency-qualified for the declared PMTime decision context.

ITU-T G.8275 (01/2026) Annex F is an optional performance-monitoring annex. Its Note 2 states that the time-of-day clock used to build the record should be reliable and preferably based on a timing reference not impacted by the same type of failures that can impact the recovered PTP clock. The current evaluator turns that qualitative engineering recommendation into an executable synthetic evidence contract; it does not invent a universal timing threshold or claim conformance to the August 2026 amendment.

Official source: [ITU-T G.8275 (01/2026), Annex F](https://www.itu.int/epublications/publication/itu-t-g-8275-2026-01-common-aspects-of-ptp-profiles-for-phase-time-synchronization).

## Why this is separate from SyncaaS availability

The existing SyncaaS profile evaluates Annex G availability-state reconstruction and profile-specific clockClass-to-N1S/D1S classification. It does not by itself establish that the time-of-day reference used to timestamp those records is independent of the recovered PTP clock.

The PMTime profile is a separate evidence axis:
- Annex G AS/UAS describes service availability from the declared profile inputs.
- PMTime integrity describes whether record time attribution, calibration, source lineage and dependency evidence are adequate for a declared timing decision.
- Physical phase/time error remains a separate quantity measured against an independent traceable analyzer.
- A correct PMTime report is not proof that the PTP service met its accuracy, frequency, holdover or availability requirements.

The two validators must not be collapsed into one status or one percentage.

## Input contract

Each synthetic case contains:

- One or more PMTime records with an increasing record index, event timestamp in TAI nanoseconds, explicit half-open measurement period, record-receive time, source identity/type, clock-domain identity, PTP clock instance, timestamp method, configuration revision and adapter mapping version.
- Exact record-content and uncertainty-budget digest references.
- A calibration certificate digest, validity interval and traceability reference.
- A declared timestamp uncertainty and an explicit synthetic profile limit. The 1,000 ns limit in the initial fixtures is only a software-test value; it is not an ITU-T performance threshold or a universal metrology limit.
- Typed dependency assessments from Mycelix for the exact PMTime source and PTP clock instance, bound to the record's event time, inventory snapshot and evidence frontier.
- The upstream Mycelix report contract and reference-verifier receipt, including the expected five reference checks.

The evaluator requires core domains `clock`, `oscillator` and `gnss_reference`. Cases can add `power`, `network`, `collector`, `data_pipeline`, `correction_provider`, `firmware_lineage` and `antenna_site` where those domains are relevant to the threat model. Domain types remain separate: shared power does not imply shared clock.

## Decision outcomes

- `TIMESTAMP_QUALIFIED_FOR_DECLARED_SCOPE`: record sequence, event/period attribution, time scale, calibration and uncertainty evidence pass the declared profile, and every required Mycelix dependency query returns a verifier-checked `NO_DECLARED_COMMON_DOMAIN`.
- `TIMESTAMP_SHARED_FAILURE_DOMAIN`: direct source identity or an upstream Mycelix assessment identifies a shared failure domain. This is a common-cause finding, not an estimate of failure probability.
- `TIMESTAMP_INDETERMINATE`: evidence is missing, stale, incomplete, conflicting, unverified or outside the supported time-scale/uncertainty profile.
- `TIMESTAMP_INVALID`: structural contradictions such as a PMTime outside its period, non-increasing record index/time, or overlapping measurement periods.

`NO_DECLARED_COMMON_DOMAIN` is scoped to the exact complete declared inventory and evidence frontier. It does not prove physical or statistical independence or the absence of undiscovered common causes.

## Temporal and acquisition semantics

1. PMTime event time and collector receive time are distinct. A late management-plane delivery does not move the measurement into a later period and does not by itself create a local service outage.
2. Period intervals are half-open `[start, end)`. Overlap is invalid; a gap in a supposedly continuous sequence is indeterminate.
3. Record indices and PMTime event timestamps must increase. A missing record index is indeterminate; a repeated or decreasing index/time is invalid.
4. A timestamp source, clock-domain, PTP instance, time scale, configuration revision or adapter version change requires a configuration-change receipt. Without it, the record chain is indeterminate.
5. Calibration must cover the full measurement period. A current label without a certificate digest, validity interval and traceability reference is not enough.
6. The supported time scale in 0.1 is TAI. UTC/GPS or other scales without a separately implemented and evidenced conversion path remain indeterminate.
7. A valid signature or record digest does not upgrade stale calibration or missing dependency lineage.

## Composition with Mycelix

The evaluator does **not** implement another dependency graph. For each domain, it binds the supplied Mycelix report to:
- the expected domain;
- the sorted exact pair of PMTime source and PTP clock instance;
- the event time of the latest record in the case;
- the inventory snapshot ID and evidence frontier;
- the report's disposition and input digest;
- the Mycelix reference verifier contract, `verified` status, authority and five required checks.

The report and verifier artifact SHA-256 values are preserved as receipt references. This profile verifies their shapes and structured bindings; it does not authenticate Mycelix signatures or custody, and it does not independently rerun the graph. That remains the Mycelix evidence owner's responsibility.

## Frozen challenge suite

The current public suite has 17 synthetic cases:

1. Fully qualified reference and complete dependency inventory.
2. Shared clock.
3. Shared oscillator.
4. Shared GNSS reference.
5. Missing required domain query.
6. Stale calibration.
7. Timestamp-source switch without a new configuration receipt.
8. PMTime rollback.
9. Overlapping measurement periods.
10. Incomplete dependency inventory.
11. Valid record signature with stale dependency lineage.
12. Unsupported/unmapped time scale.
13. PMTime and PTP service using the same clock instance.
14. Shared network delivery dependency.
15. Delayed management-plane ingestion without changing event-time period attribution.
16. Period gap.
17. Timestamp-source switch with a bound new configuration receipt.

The fixture pack is public synthetic conformance, not blind evaluation. Blind generalization belongs to the evaluator-sealed protocol in [#340](https://github.com/Luminous-Dynamics/luminous-platform/issues/340).

## Industry/SOTA positioning

The contribution is not a replacement PTP servo, a new synchronization protocol or a claim of lower physical time error. It formalizes a gap between:
- PTP's own service/clock quality monitoring;
- the integrity of the time source used to timestamp monitoring records;
- dependency-aware evidence that the monitoring source does not share a declared failure cause with the service being monitored;
- independent physical time/phase measurement.

This is an evidence-provenance and common-cause assurance contribution. To compare with deployed telecom equipment, the next tier must use exact hardware/firmware revisions, a separately traceable time analyzer, profile-specific G.8275.1 or G.8275.2 thresholds, measured phase/time error and frequency/holdover, and controlled failure injection. The present synthetic suite cannot establish those results.

## Claim ceiling

This implementation validates synthetic PMTime record semantics and structurally composes declared Mycelix dependency-verification outputs. It does not authenticate upstream receipts, measure absolute physical time error, prove physical independence, validate a live PTP clock or SyncaaS service, establish telecom SLA, or claim G.8275 Amendment 1 conformance.
