# OMSI Telecom PTP Timing-Service Benchmark 0.1

**Status:** Research/benchmark design; not executed  
**Owner:** Luminous Platform / OMSI  
**Implementation issue:** [OMSI-PNT-TELECOM-001 #312](https://github.com/Luminous-Dynamics/luminous-platform/issues/312)  
**Scenario owner:** [Symtropy PNT-TELECOM-001 #4079](https://github.com/Luminous-Dynamics/symtropy/issues/4079)  
**Parent profile:** [OMSI PNT resilience #297](https://github.com/Luminous-Dynamics/luminous-platform/issues/297) / [draft PR #298](https://github.com/Luminous-Dynamics/luminous-platform/pull/298)

## 1. Research question

Can a vendor-neutral assurance layer reconstruct the state and service consequence of a telecom PTP timing network from source-qualified evidence, distinguish availability from physical time accuracy, detect shared failure dependencies, and require evidence-based requalification—without implementing or controlling the PTP timing system?

The intended output is not a second PTP monitor. It is a read-only, evidence-qualified service assessment that composes device-native monitoring, operator NMS, PTP telemetry, independent measurement and existing Mycelix time/dependency semantics.

## 2. Normative reference baseline

Version checks below were made 2026-11-07. The exact standard edition and supported optional feature set must be pinned in every test manifest.

| Reference | Current edition checked | Benchmark role |
|---|---|---|
| ITU-T G.8275 | 01/2026 | Common aspects for PTP phase/time profiles; optional Annex G defines synchronization-service availability metrics (SyncaaS). |
| ITU-T G.8275.1/Y.1369.1 | 02/2026 + Amendment 1 (08/2026) | Full-timing-support (FTS) telecom PTP profile. ITU lists Amendment 1 as in force/prepublished at the check date. |
| ITU-T G.8275.2 | 02/2026 | Partial-timing-support (PTS) profile; keep evaluation separate from FTS. |
| ITU-T G.8273.2 | 06/2023 | Relevant clock timing characteristics and holdover requirements for the applicable device class/profile. |
| IEEE 1588 | Exact edition per device profile | Underlying PTP protocol version and semantics; record the edition actually implemented. |

Official references:
- G.8275: https://www.itu.int/epublications/publication/itu-t-g-8275-2026-01-common-aspects-of-ptp-profiles-for-phase-time-synchronization
- G.8275.1 edition and amendment register: https://www.itu.int/rec/T-REC-G.8275.1/en
- G.8275.2: https://www.itu.int/rec/T-REC-G.8275.2/en
- G.8273.2: https://www.itu.int/epublications/publication/itu-t-g-8273-2-y-1368-2-2023-06-timing-characteristics-of-telecom-boundary-clocks-and-telecom-time-synchronous-clocks-for-use-with-full-timing-support-from-the-network

**Optional-feature rule:** SyncaaS availability datasets and PTP performance-monitoring extensions must be represented with an explicit capability/support state (`supported_and_enabled`, `supported_but_disabled`, `not_supported`, `unknown`). An absent record is not zero unavailable seconds, zero time error or proof of healthy service.

## 3. Product and technical boundary

### Platform owns
- Profile registry and exact standard/version/capability manifest.
- Read-only adapters and preservation of native telemetry semantics.
- Evidence admission, operator workflow, service-impact mapping and claim ceiling.
- Portable reports and integration with existing OMSI/EMSR receipts and independent verification.

### Existing projects own
- **Mycelix Position:** time scales, clock domains/correlation, temporal uncertainty, source provenance, failure-domain graph and operational navigation/time health (#435, #458, #5904).
- **Symthaea:** competing hypotheses, calibrated uncertainty, abstention and bounded decision-aware evidence acquisition (#10109, #10824).
- **Symtropy:** deterministic service transition, continuity and recovery scenarios (#3909, #4079).
- **Nixward / participant boundary:** any host/configuration mutation authority; no AI-created authority.

### Explicit non-goals
- No new PTP servo, Best TimeTransmitter Clock Algorithm (BTCA/BMCA), grandmaster selection policy, clock implementation or frequency/phase control.
- No live changes to grandmaster priorities, PTP domain, network configuration or service mode.
- No raw RF collection, RF transmission or emitter/actor attribution.
- No claim that protocol-level PTP fields prove physical output accuracy.

## 4. Evidence contract

Every adapter run should preserve, when exposed:
- standard/profile and edition, IEEE 1588 edition, PTP domain and transport;
- clock identity, role/type, port identity/state and configuration digest;
- parent/timeTransmitter identity, grandmaster identity and clock-quality/priority fields;
- `stepsRemoved`, path-trace/topology and parent-change events where supported;
- `timePropertiesDS` fields and traceability/time-source flags with their source meaning;
- `synchronizationUncertain` state and whether propagation is configured/supported end-to-end;
- announce/sync/follow-up/delay message counters and loss/timeout observations;
- mean path delay, offset-from-time-transmitter statistics and record interval where available;
- SyncaaS AS/UAS counters, `measurementValid`, `periodComplete`, interval and collection coverage where supported;
- clock state such as acquiring, locked, holdover or free-run only when the exact source reports a documented state;
- calibration, firmware, hardware revision, local/external reference, clock domain and time-correlation lineage;
- management-plane reachability separate from synchronization evidence;
- independent time-error/phase/frequency measurements with instrument calibration, measurement point and reference provenance where available.

Record native values before any normalized projection. Unknown enum values and unsupported optional datasets remain unknown; never map them to a favorable default.

## 5. Separate benchmark tracks

| Track | Question | Measurements |
|---|---|---|
| PTP-ADAPT | Does the adapter preserve source semantics? | Field coverage, enum/value fidelity, unknown-field handling, timestamp/counter interpretation, exact profile/version identity. |
| PTP-SVC | Was the synchronization service available? | Available/unavailable seconds, measurement validity, period completeness, missed intervals, NMS aggregation agreement. |
| PTP-ERR | Was delivered time accurate? | Absolute time error, phase error, frequency offset/stability, p50/p95/p99/max and time outside the operator-defined limit against an independent qualified reference. |
| PTP-HOLD | Did holdover remain within its declared equipment/profile envelope? | Entry/exit time, holdover duration, error/uncertainty trajectory and limit crossing against referenced equipment data or measured trace. |
| PTP-TOPO | Did topology/source transitions behave as declared? | Parent changes, reparent/failover duration, flapping, path changes and stabilization. |
| PTP-COR | Were sources actually independent? | False independent corroboration, shared failure-domain recall, incomplete inventory abstention and positive independent-source recognition. |
| PTP-REC | Was service requalified correctly? | Premature re-entry, requalification time, stale calibration/configuration rejection and recovery evidence completeness. |
| PTP-VOI | Did the next diagnostic observation improve the decision? | Realized decision-risk reduction, prediction error, probe latency/cost/privacy and separately measured service/resource interference. |
| PTP-AUD | Can an independent party reproduce the result? | Exact-byte receipt completeness, offline reconstruction, independent verifier agreement and missing-evidence behavior. |

Do not collapse availability and accuracy. An implementation can report PTP service available while time error exceeds a selected engineering limit; it can also lose management visibility while local timing remains within its declared profile.

## 6. Scenario families

Start with deterministic synthetic/replay cases. Exact scenarios are owned by Symtropy #4079 and should be bound here by manifest digest when materialized.

1. Stable G.8275.1 FTS topology and baseline counters.
2. Stable G.8275.2 PTS topology, evaluated on a separate split/scorecard.
3. Primary grandmaster loss with successful alternate source reparenting.
4. Grandmaster cycling/flapping and stabilization.
5. PTP path change, congestion or delay variation while connectivity remains up.
6. Holdover entry, bounded propagation, limit crossing and reacquisition under the exact clock-class profile.
7. `synchronizationUncertain` set, cleared, unsupported or not propagated end-to-end.
8. SyncaaS data collection disabled, unsupported, delayed, incomplete or invalid.
9. Several clocks share GNSS, oscillator, power, network, correction service or telemetry collector.
10. Incomplete dependency graph or inconsistent dependency snapshot.
11. Management partition with local PTP service still evidenced.
12. PTP transport loss with management still reachable.
13. Device-reported healthy status conflicts with independent reference time-error measurement.
14. Parent recovers but calibration, source traceability or independent time-error evidence is stale/missing.
15. Benign management/telemetry gap that must not be called a timing outage.
16. Diagnostic probe is labelled read-only but independently measured CPU/network/queue impact is non-zero or unknown.

Every case must state exogenous events, source/profile identity, time domain, declared limits, data origin, dependencies, ground-truth/reference lineage and expected state/reason codes.

## 7. Matched baselines and comparison design

Compare on the same topology, device/profile, input observations, reference instrument, scenario, event schedule and observation budget:

A. Native PTP clock/device status and vendor NMS alarm handling.
B. linuxptp `ptp4l` / `pmc` telemetry and configuration where applicable: https://www.linuxptp.org/documentation/configs/.
C. Fixed threshold/runbook baseline using the same operator-defined service profile.
D. Human timing/network operator triage on matched incidents.
E. Luminous deterministic evidence policy without Symthaea.
F. Luminous evidence policy with Symthaea decision-aware assessment/abstention.
G. Evaluator oracle as an upper bound only; it never sees its answer deck in the system-under-test arm.

Where available, independently compare protocol-level performance records with calibrated hardware timing analyzers. EANTC's 2026 PTP performance-monitoring interop test is a relevant method comparator: it used one telecom grandmaster, seven boundary clocks and Calnex/Keysight analyzers. It is evidence that native performance-monitoring records can be cross-checked in a lab, not proof that all vendors or deployments produce equivalent accuracy. https://wiki.eantc.de/wiki/publicreports/view/Main/EANTC%20Transport%20%26%20Cloud%20Networks%20Interop%20Test%20Report%202026/Time%20Synchronization/PTP%20Performance%20Monitoring

Calnex test guidance is useful for the independent-reference and negative-condition design; treat vendor guidance as a test-method source, not normative requirements: https://www.calnexsol.cn/docman/techlib/timing-and-sync-lab/191-mifid-ii-time-compliance

## 8. Statistical and assurance protocol

- Freeze scenario IDs, input bytes, profile editions, reference calibration, source topology and evaluator-owned expected results before confirmatory evaluation.
- Split by topology, clock model, vendor, firmware, campaign and time period; do not randomly split adjacent samples from one timing trace across train/test.
- Use paired scenarios/seeds for policy comparisons and report cluster-aware uncertainty intervals.
- Preserve all started, failed, timed-out and indeterminate runs in the denominator.
- A missing SyncaaS or independent-reference record is not a successful observation.
- Keep evaluator labels and reference measurement provenance independent from the model's input path where the claim requires independence.
- Separate discovery/tuning scenarios from sealed confirmation scenarios.
- Bind exact bytes, model/configuration, adapters, software revisions, environment and independent report digests in existing EMSR/OMSI evidence contracts.

## 9. Proposed gates (research targets, not standards)

- 100% expected outcome/reason-code agreement on the frozen synthetic suite.
- Zero false continuity on canonical cases where the independent reference is outside the declared service envelope.
- Zero false-independence claims on complete common-domain negative controls; recognize valid independent-source positive controls.
- Zero premature NORMAL re-entry without the profile's required fresh evidence.
- 100% deterministic replay and offline reconstruction for qualified synthetic runs.
- 100% accounting of supported adapter fields as preserved, transformed under a registered rule, explicitly dropped with reason, or unsupported/unknown.
- Calibration, detection delay, availability and time-error thresholds must be set by the specific operator/profile owner; do not invent universal thresholds.
- Performance targets are set only after a named host, topology, input rate and reference baseline are measured.

No composite score may allow a provenance, integrity, false-continuity or premature-recovery failure to be offset by accuracy or throughput.

## 10. Current industry comparison

- Receiver-native products (e.g. Septentrio AIM+ and NovAtel GRIT) own receiver-level RF/PNT observations and mitigation. Consume their output; do not claim to replace their hardware/firmware.
- Telecom clock/NMS vendors expose clock state and monitoring data. Preserve native semantics and interoperate rather than duplicating their servo or network-management plane.
- Timing analyzers (e.g. Calnex and Keysight) provide independent measurement baselines. Their physical measurements are not replaceable by PTP self-reported counters or AI confidence.
- linuxptp is a transparent open implementation baseline for compatible protocol behavior, not a physical reference clock.

Luminous's hypothesis is cross-vendor service assurance: source/time/calibration provenance, dependency-aware corroboration, separate service availability versus time accuracy, decision-aware diagnostics and evidence-qualified recovery. This is a differentiated product hypothesis, not demonstrated superiority.

## 11. Claim ceiling and promotion

This document is a benchmark design. It does not establish an implemented adapter, executed scenario suite, measured PTP performance, equipment conformance, physical time accuracy, telecom service assurance, customer deployment or regulatory certification. A later result must name the exact profile editions, devices, firmware, topology, measurement reference, operator limit, data rights, split and independent verifier. Promotion follows the OMSI E0-E6 evidence stages per claim.

## 12. Primary external references

- ITU-T G.8275 (01/2026): https://www.itu.int/rec/T-REC-G.8275/en
- ITU-T G.8275.1 (02/2026) + Amendment 1 (08/2026): https://www.itu.int/rec/T-REC-G.8275.1/en
- ITU-T G.8275.2 (02/2026): https://www.itu.int/rec/T-REC-G.8275.2/en
- ITU-T G.8273.2: https://www.itu.int/rec/T-REC-G.8273.2/en
- linuxptp configurations: https://www.linuxptp.org/documentation/configs/
- EANTC PTP performance monitoring interoperability test 2026: https://wiki.eantc.de/wiki/publicreports/view/Main/EANTC%20Transport%20%26%20Cloud%20Networks%20Interop%20Test%20Report%202026/Time%20Synchronization/PTP%20Performance%20Monitoring
- Calnex independent timing measurement guidance: https://www.calnexsol.cn/docman/techlib/timing-and-sync-lab/191-mifid-ii-time-compliance
