# OMSI Positioning, Navigation and Timing Resilience Profile 0.1

**Profile ID:** `omsi.pnt-resilience`  
**Version:** `0.1.0-draft`  
**Status:** Proposed profile; not an adopted standard  
**Parent:** [OMSI-INDUSTRY-001 #154](https://github.com/Luminous-Dynamics/luminous-platform/issues/154)  
**Profile issue:** [OMSI-PNT-001 #297](https://github.com/Luminous-Dynamics/luminous-platform/issues/297)  
**Claim ceiling:** Defines profile semantics, evidence requirements and benchmark rules. It does not qualify a receiver, detector, navigation solution, operator, service, deployment or safety case.

## 1. Purpose

The PNT Resilience Profile defines how Luminous Platform and OMSI exchange and evaluate evidence about positioning, navigation and timing services when their input sources may be degraded, correlated, stale, uncertain or unavailable.

The profile is designed to serve multiple sectors—telecommunications, ports and maritime, rail and road transport, industrial automation, utilities, emergency services, data centres and autonomous systems—without pretending that their operational risk tolerances or performance thresholds are interchangeable.

The central question is not merely whether a receiver reports an anomaly. It is:

> For this declared use, what PNT/time service can still be relied upon, what evidence supports that claim, what weaker service modes are permitted, and what must be requalified before normal service resumes?

## 2. Design decisions

### 2.1 Profile-first, not a universal PNT ontology

OMSI supplies the shared interchange/evidence envelope and consequential-operation contract. This profile supplies PNT-specific semantics, units, reference systems, time bases, service requirements, hazards, baselines and claim ceilings.

Do not create a second:
- positioning or navigation mathematics library;
- sensor identity, calibration or revocation authority;
- clock/time-scale model;
- generic provenance or common-mode graph;
- evidence envelope, run-receipt format or transparency ledger;
- host/industrial transaction journal, authorization broker or service workflow engine;
- universal PNT health score or weighted industry leaderboard.

### 2.2 Separate predicates

The following do not imply one another:

- authenticated source != physically accurate measurement;
- valid signature != valid calibration;
- receiver-reported health != service integrity;
- recent observation != sufficiently low uncertainty;
- peer count != independent source count;
- no declared common dependency != proven physical independence;
- anomaly detection != cause determination;
- cause hypothesis != actor attribution;
- predicted position/time != fresh observation;
- model recommendation != authority;
- command acknowledgement != achieved service postcondition;
- anomaly disappearance != recovery qualification.

### 2.3 Availability-resilient, authority-fail-closed

Optional platform services must not be required for basic host boot or declared local availability. Missing identity, authorization, policy, currentness or participant-effect evidence must never grant a protected operation. An unknown operation outcome is reconciled before a new mutation is attempted.

Reuse the platform failure-semantics contract in [#283](https://github.com/Luminous-Dynamics/luminous-platform/issues/283) and the agent effect boundary in [#168](https://github.com/Luminous-Dynamics/luminous-platform/issues/168).

## 3. Ownership and composition

| Layer | Owner | PNT profile responsibility |
|---|---|---|
| Cross-industry profile and operations interchange | Luminous Platform / OMSI | Profile registry, source adapters, operator workflow, lifecycle, outcome and claim-ceiling semantics |
| Cognitive assessment | Symthaea | Anomaly/hypothesis assessment, calibrated uncertainty, abstention, diagnostic selection and evaluation |
| Spatial/time/fusion evidence | Mycelix Position | Spatial reference systems, time scales, clock correlation, measurement provenance, fusion, source independence and navigation health |
| Physical/service consequence | Symtropy | Deterministic scenario replay and model-bounded service/recovery consequences |
| Read-only spatial/dependency projection | Sol-Atlas | Asset, site, service and dependency context with source lineage and freshness |
| Mobile sensor adapter | Symthaea SOMA | Device/receiver input mapping to existing Position semantics; no independent PNT authority |
| Host transaction authority | Nixward / platform participant boundary | Host-side authorization, effect commit and recovery; no bypass through the cognitive model |
| Durable source and trust relationships | Mycelix | Identity, delegation, source provenance, revocation, calibration/time evidence and common-mode relationships |

### 3.1 Existing work to reuse

- Symthaea EMSR programme: [#10775](https://github.com/Luminous-Dynamics/symthaea/issues/10775)
- Dataset/split/run receipt contract: [#10814](https://github.com/Luminous-Dynamics/symthaea/issues/10814), draft [PR #10875](https://github.com/Luminous-Dynamics/symthaea/pull/10875)
- SOMA positioning adapter: [#10824](https://github.com/Luminous-Dynamics/symthaea/issues/10824)
- Platform edge sensor node profile: [#300](https://github.com/Luminous-Dynamics/luminous-platform/issues/300)
- First deployable platform product slice: [#301](https://github.com/Luminous-Dynamics/luminous-platform/issues/301)
- Ground-truth/oracle independence: [#10826](https://github.com/Luminous-Dynamics/symthaea/issues/10826)
- Mycelix spatial truth: [#413](https://github.com/Luminous-Dynamics/mycelix/issues/413)
- Fusion admission versus mathematical precision: [#417](https://github.com/Luminous-Dynamics/mycelix/issues/417)
- Time scales and clock domains: [#435](https://github.com/Luminous-Dynamics/mycelix/issues/435)
- Navigation health/freshness/failover: [#458](https://github.com/Luminous-Dynamics/mycelix/issues/458)
- Geometry/solver qualification: [#433](https://github.com/Luminous-Dynamics/mycelix/issues/433), [#447](https://github.com/Luminous-Dynamics/mycelix/issues/447)
- Sensor provenance and common-mode evidence: [#5899](https://github.com/Luminous-Dynamics/mycelix/issues/5899), [#5904](https://github.com/Luminous-Dynamics/mycelix/issues/5904)
- Symtropy degraded service and recovery scenarios: [#3909](https://github.com/Luminous-Dynamics/symtropy/issues/3909)
- Sol-Atlas dependency twin: [#584](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/584)

This table is an ownership map, not proof that any linked item is complete or qualified.

## 4. Profile object model

The profile projects domain-specific content into OMSI's existing envelope. The following are semantic groups, not a mandate to create one monolithic serialized object.

### 4.1 Declared PNT service profile

A PNT service profile must bind:
- immutable profile ID and version;
- domain/use and intended consumer;
- subject, asset, site and tenant references as applicable;
- applicable operating environment and coverage region;
- required horizontal/vertical accuracy or time error where relevant;
- alert limit / protection requirement and its domain authority;
- freshness, continuity, availability and maximum propagation requirements;
- permitted degraded modes and required evidence for each mode;
- escalation owner, human approval boundary and recovery criteria.

Numeric limits are selected by the relevant sector/operator/regulator and exact standard version. No universal `10 m`, `1 s`, false-alarm or outage threshold is implied here.

### 4.2 Observation and time evidence

Every admitted observation must preserve, where available:
- stable source/receiver identity and source class;
- measurement modality and exact adapter/profile version;
- receiver, antenna, firmware and material configuration identity;
- calibration identity, validity interval and uncertainty;
- units, value, covariance/uncertainty and reference frame;
- raw observation time, explicit time scale or clock-domain/epoch identity, and time uncertainty;
- receipt/ingestion time separately from observation time;
- source artifact or acquisition reference and exact digest where retention policy permits;
- evidence class (measured, reported, derived, simulated, inferred, nominal or unknown);
- validity/currentness and explicit missing/conflicting fields;
- privacy, data-rights and retention class.

Local monotonic time is not automatically UTC, GPS time or cross-device comparable. A conversion is a derivation and retains its source-clock reading, correlation profile, valid interval and uncertainty.

### 4.3 Source independence and common-mode context

The profile must preserve failure-domain references for, as applicable:
- receiver, antenna and installation/site;
- oscillator, reference clock and time-distribution service;
- GNSS constellation/signal/service or correction service;
- power supply, network, backhaul and transport;
- firmware/software lineage and configuration management;
- monitoring, logging and data-transformation pipeline;
- operator, maintenance process and shared control plane.

The pairwise relation is one of:
- `shared_source`;
- `shared_failure_domain`;
- `unknown` when inventory is incomplete or stale;
- `no_declared_common_domain` only when both scoped inventories are explicitly complete.

No declared common domain does not prove physical independence. Source count, signature count, network route count or device-ID count are not independence measures.

The dependency graph must bind one immutable snapshot ID and one valid as-of time.

### 4.4 Assessment and hypothesis evidence

An assessment must distinguish:
- observed symptom;
- receiver/service integrity facet;
- cause hypothesis and alternatives;
- calibrated uncertainty and abstention state;
- evidence used and evidence missing;
- applicability profile and model/configuration identity;
- service impact assessment;
- allowed decision options and explicit exclusions.

Benign alternatives include multipath, obstruction, receiver/antenna/cabling faults, configuration drift, power/network faults, timing faults, stale corrections and data-pipeline faults. An anomaly score does not establish malicious intent or an actor.

### 4.5 Service consequence, action and recovery

Service consequence is a separate claim from receiver anomaly. It binds:
- affected service/consumer and declared requirement;
- time interval and service impact;
- actual observations versus modelled/simulated estimates;
- selected degraded-mode policy and responsible decision owner;
- approval/authority artifact if an operation is requested;
- exact participant attempt/effect reference;
- independent postcondition evidence;
- recovery evidence and requalification result.

Sol-Atlas display state, Symtropy simulation output, Symthaea recommendation and OMSI profile conformance cannot mint an operation permission or satisfy a live postcondition.

## 5. Operational lifecycle

The interoperable lifecycle should preserve distinct events:

1. **Observe** — collect receiver, timing and service observations with source/time/configuration identity.
2. **Validate** — check numeric, calibration, time, frame, freshness and provenance constraints.
3. **Assess** — derive integrity facets and competing hypotheses; abstain when evidence is insufficient.
4. **Determine service mode** — apply the declared operational profile to the qualified facets and downstream service requirements.
5. **Decide** — operator or authorized policy owner selects a permitted response.
6. **Authorize** — bind exact tenant, target, operation, plan, request digest, authority grant and validity window.
7. **Execute** — the authoritative participant records the attempt and effect.
8. **Verify** — independent fresh observations establish the postcondition.
9. **Recover** — requalify sources, time, calibration, model state and service requirements before returning to a stronger mode.

A model cannot move directly from assessment to physical action. Missing authority or evidence is fail-closed for mutation, while local observation and declared safe-baseline service may continue.

### 5.1 Decision-aware evidence acquisition

A cognitive system should propose the **next observation most likely to change a safe service decision**, not simply the observation with the largest entropy reduction.

Compose the existing decision-aware investigation and bounded read-only probe contracts:
- Symthaea #10109 supplies hypotheses, uncertainty and decision-aware EVSI/minimax-regret proposals.
- Luminous Platform #299 maps those proposals to the existing probe catalogue, checks eligibility and cumulative budgets, and records read-only acquisition and probe-impact evidence.
- Mycelix #435/#5904 supply time-domain and common-cause semantics.
- The selected sector profile supplies the decision-loss/utility scope and acceptable impact envelope; the model must not invent them.

For a PNT use case, candidate observations may include receiver-reported integrity/status, calibration-currentness, clock-correlation validity, independent timing-source state, source-dependency evidence, or a bounded read-only service/asset health observation. This profile does not grant the model unrestricted access to raw receiver streams or customer/production infrastructure.

The handoff must preserve:
- exact hypothesis/scenario, service profile and decision-loss digests;
- predicted decision value and its uncertainty, separate from source reliability;
- probe ID, exact read-only operation/schema, authorized target/scope and collector;
- per-call and cumulative query/byte/concurrency/time budgets;
- source identity, failure-domain and freshness references;
- acquisition privacy cost and expected operational externality;
- independent observation receipt and separate probe-impact receipt;
- predicted versus realized decision-risk reduction;
- explicit stop, abstain and human-escalation conditions.

A positive EVSI estimate is not permission to probe. A probe returning no data is not proof that acquisition stopped; missing impact evidence is unknown, not zero. Only catalogued read-only probes are eligible in v0.1; write-capable, mixed, unknown or unbounded operations are rejected. A probe result cannot authorize remediation.

#### Decision-aware acquisition benchmark

Compare, on the same frozen scenarios and resource budgets:
1. random eligible probe;
2. cheapest-first;
3. entropy-per-cost / point-EIG;
4. Platform #185 risk-adjusted EVSI reference;
5. Symthaea expected EVSI;
6. Symthaea lower-tail/worst-case EVSI or minimax regret;
7. decision-aware selection with independently measured probe-impact penalty;
8. no-probe and fixed-runbook controls.

Report predicted EVSI calibration, realized decision-risk reduction, worst-case/lower-tail regret, useful-observation yield, duplicate/correlated evidence over-credit, probe latency/cost/privacy, probe-induced service degradation, budget overruns, timeout reconciliation, and correct stop/abstain/escalate decisions separately. No composite score may trade a probe-safety or authority failure against decision value.

## 6. Separate industry tracks

These are distinct application profiles, not one leaderboard.

| Track | Key profile concerns | Example output |
|---|---|---|
| Telecom timing | PTP/GNSS time distribution, holdover, phase/time error, sync hierarchy and service continuity | Timing-service integrity state |
| Maritime / ports | Receiver integrity, vessel/port context, coverage and navigation service requirement | Navigation availability and uncertainty report |
| Road / rail / fleet | Position/time freshness, tunnel/urban transitions, odometry and service-specific fallback | Profile-qualified positioning state |
| Industrial / warehouse | UWB/Wi-Fi RTT/GNSS/IMU, local frame, anchor survey/calibration and NLOS | Local positioning qualification |
| Utilities / emergency | Shared timing dependencies, service continuity, fallback and operator response | Critical-service timing/positioning assessment |
| Aviation | Domain authority, exact aviation standard, protection levels, approved equipment and formal safety case | Out of scope for initial profile; requires separate admission |

The initial profile may define mappings for all tracks, but an implementation or benchmark claim must name the exact track and applicability boundary.

## 7. Benchmark protocol

The profile requires matched tasks, data, splits, hardware, operating conditions and false-alarm budgets. It does not permit one weighted PNT score.

### 7.1 Evaluation tracks

| Track | Measures |
|---|---|
| Detection | Precision/recall, PR-AUC, detection probability at fixed false-alarm budget, false alarms per receiver-hour, detection delay |
| Domain transfer | Held-out receiver, campaign, site/area, date, scenario family and dataset; worst-domain and macro metrics |
| Epistemic quality | Brier/log loss, reliability diagrams, calibration, selective risk/coverage, abstention correctness |
| Source independence | False-independent-corroboration rate, shared-domain conflict recall, unknown-dependency abstention |
| PNT integrity | Integrity alert latency, misleading-solution exposure duration, false exclusion and continuity |
| Service continuity | Permitted degraded-mode transition, service availability and downstream consequence |
| Recovery | Premature re-entry count, time to requalify, calibration/time currentness failures and evidence completeness |
| Assurance | Exact input/label/source binding, offline reconstruction, tamper detection and independent verifier success |
| Deployability | P50/P95/P99 latency, throughput, CPU/RAM/energy and cost on named hardware |

### 7.2 Baseline ladder

Use, when the exact input type permits:
1. receiver-native status, alerts, authentication and protection outputs;
2. conventional RAIM/ARAIM or timing-monitoring baseline appropriate to the selected sector;
3. simple status/residual/threshold baseline;
4. statistical raw-IF or signal-feature baseline;
5. conventional supervised ML baseline;
6. published cross-domain, meta-learning or spatio-temporal baseline where faithful reproduction is possible;
7. collaborative integrity baseline with explicit common-mode testing;
8. matched human/operator workflow;
9. Symthaea assessment and ablations;
10. integrated OMSI evidence-qualified service workflow.

#### 7.2.1 Reproducible 2026 comparator set

The Symthaea EMSR benchmark charter maintains the complete paper-level reproduction plan. The PNT profile calls out the following published results so no mixed-split score is mistaken for transfer performance:

- IEEE Access (2026): >90% average in-domain accuracy but <=66% average cross-dataset performance across the reported study conditions. https://doi.org/10.1109/ACCESS.2026.3693081
- NAVITEC (2026): CNN versus MAML cross-testing over TEXBAT/OAKBAT/FGI-SpoofRepo; MAML with post-correlation Doppler features is the reported strongest cross-testing method. https://doi.org/10.1109/NAVITEC202669090.2026.11692526
- ION GNSS+ (2026): 96.4% detection and 2.26% false alarms on a mixed TEXBAT split, versus 58.6% detection in the reported harder cross-scenario evaluation. https://doi.org/10.33012/2026.20700
- ION GNSS+ (2026) DANN: improved cross-scene simulation reports 95.90% detection at 1.38% false alarms; cross-scene simulation only. https://doi.org/10.33012/2026.20876
- ION GNSS+ (2026) lightweight multi-domain feature fusion: reports 94.59% satellite-wise TPR at 1% FPR and 0.173 ms/sample on its scenario-diverse evaluation; exact feature/hardware/split reproduction required. https://doi.org/10.33012/2026.20831
- Explainable multimodal classifier (May 2026): 99.33% on simulated test data; not a field-performance comparator. https://doi.org/10.1007/s10207-026-01267-5
- RAIM-driven semi-supervised maritime method (July 2026): candidate maritime comparator, not a certified integrity method. https://doi.org/10.1016/j.asr.2026.07.097

Required interpretation: reproduce the exact task, feature pipeline, dataset rights, receiver, scenario, split and false-alarm budget before comparing. Keep mixed-split, cross-dataset, cross-receiver and cross-scenario results in separate result lanes. The hardest scenario-family holdout remains a primary metric.

Do not compare a simulated-data headline with field-campaign data or a receiver-native protection level with an ML anomaly score as though they measured the same construct.

### 7.3 Ground-truth independence

Each run binds an exact ground-truth/adjudication manifest. The manifest records event-label issuer, method, event-onset time source/basis/uncertainty, receiver/reference source, detector-input sources, shared dependencies, adjudication disagreements and ambiguous states.

Physical-event truth, receiver-integrity truth and service-consequence truth remain distinct. Model-inferred labels are never silently promoted to independent oracle truth.

### 7.4 Admission and comparison rule

Missing, ambiguous or incomparable fields produce `unknown`, `indeterminate` or `not_comparable`, not zero or a guessed score. A better-than-SOTA claim requires a preregistered matched comparison, exact source/version and split, equivalent resource budget, uncertainty, per-domain results and independent verification.

## 8. Data-plane and privacy boundary

Raw receiver/RF/time-series telemetry remains in the authorized edge or time-series data plane unless a separate legal, security and operational review authorizes retention elsewhere.

Holochain/Mycelix and transparency/evidence services carry significant events, source/authority/provenance commitments and receipts—not a global coordinate or raw-IQ firehose.

Location precision, target identity, vessel/vehicle/asset trajectory, receiver metadata and critical-infrastructure dependency information are privacy/security-sensitive. Read-only projections may be delayed, approximate, region-only or redacted. A public map is not permission to expose people, cargo or sensitive infrastructure state.

## 8.1 Edge intelligence and embedded sensor nodes

Luminous Platform may deploy low-power edge nodes for auxiliary equipment/environmental telemetry and bounded local inference. These nodes are subordinate instruments, not independent PNT receivers or RF acquisition systems.

Candidate Espressif families for exact-board evaluation include ESP32-S3, ESP32-P4 and ESP32-S31. Chip-level capability is not board qualification. For PNT-adjacent use, a node may report temperature, power-rail condition, vibration, enclosure/tamper state, network/clock status and receiver-reported health over an authorized interface. It must not be described as a GNSS raw-IF digitizer, spoofing detector or PNT integrity authority unless a separate qualified receiver/front-end and acquisition chain provide that evidence.

The edge measurement envelope binds exact board/revision, sensor part/revision, calibration, units, uncertainty, local clock/boot epoch, firmware/model/configuration digests, retention and privacy policy. Local inference is advisory, uncertainty-bearing and abstaining; it cannot authorize a protected action or promote a service state by itself.

Implementation ownership and benchmarks are tracked in [OMSI-EDGE-SENSOR-001 #300](https://github.com/Luminous-Dynamics/luminous-platform/issues/300). The first deployable platform slice is tracked in [LP-PRODUCT-001 #301](https://github.com/Luminous-Dynamics/luminous-platform/issues/301).

## 8.2 Open-set RF awareness and edge AI boundary

General receive-side RF signal-family awareness is an adjacent capability, not a replacement for PNT integrity monitoring. Symthaea tracks it separately in EMSR-RF-OPENSET-001 #10893, with RF-RPL (2026) as a scarce-label/open-set comparator and IriSig-Spoof (2026 preprint) as a candidate temporal/open-set satellite-RF benchmark. IriSig-Spoof reports 5.17 million Iridium messages from 66 satellites over 32 days, 97.75% best cross-day accuracy and 0.9715 open-set AUROC; these are preprint-reported results, not independent Luminous reproduction. Source-level rights review remains mandatory and no payload is imported. A model output must distinguish known-class, unknown, indeterminate and degraded acquisition; unknown must not be converted into a malicious-emitter, identity, location or cause label. The open-set RF benchmark does not establish the validity of a GNSS position/time solution.

The platform must distinguish:
- ESP32-class auxiliary edge nodes (#300);
- a separately qualified RF acquisition front end and sample-clock/calibration chain;
- AI applied to recorded RF features/windows (#10893);
- ESP (extrasensory perception) as a separate, unestablished scientific hypothesis (#10843).

No AIA/ESP output may be used as PNT evidence, safety input or operational authority. ESP32 edge nodes are not assumed to provide GNSS raw-IF/IQ capture.

## 8.3 PNT platform-level comparators

PNT comparison must include platform/service offerings, not only receiver hardware:

- **SESAR real-time GNSS monitoring/interference solutions** provide area/route monitoring, receiver/sentinel feeds, GNSS performance indicators, A-RAIM protection-level estimates and API/SWIM integration. This is a direct service-monitoring/API comparator: https://sesarju.paddlecms.net/sesar-solutions/real-time-gnss-monitoring-and-interference
- **CISA PNT Integrity Library** is a modular GNSS PNT manipulation-detection framework designed for integration into receiver and timing-server OEM products: https://github.com/cisagov/PNT-Integrity

The current development README describes checks covering multi-antenna / angle-of-arrival and range-position consistency, receiver AGC/acquisition behavior, navigation-data validity, static-position and position-jump detection, position/velocity consistency, clock jumps and C/N0 consistency, combined through weighted assurance-level blending. Use this as a receiver-side algorithm comparator where the exact compatible receiver fields are available. The benchmark must preserve the individual check outputs, applicability, missing-data behavior and aggregate result separately; a weighted assurance level does not establish independent physical truth, source independence, downstream service impact or verified recovery. Compare on identical receiver inputs, event labels, profile thresholds and compute budgets. Do not compare unlike receiver capabilities or treat a missing check as a successful check. Source: https://github.com/cisagov/PNT-Integrity/blob/develop/pnt_integrity/pnt_integrity/README.md
- **PNTMAP** provides event/API/webhook intelligence and explicitly distinguishes ADS-B-derived integrity anomaly monitoring from direct RF sensing and live validation: https://pntmap.co.uk/api-access
- **Receiver-native outputs** from compatible GNSS/PNT equipment remain the strongest hardware-specific operational baseline where accessible.

Luminous must not claim superiority by offering a dashboard or API alone. Its proposed differentiation is cross-vendor source provenance, clock/calibration currentness, common-mode and oracle-independence analysis, service-dependency impact, domain-specific degraded operation and independently evidenced recovery. Head-to-head superiority remains unproven until matched data, equipment, baselines and sector thresholds are evaluated.

## 9. Standards and external references

The profile maintains a version-pinned reference registry per industry track. Candidate reference families include:
- NIST AI RMF and its developing Trustworthy AI in Critical Infrastructure profile (ongoing profile work, not a completed PNT standard): https://www.nist.gov/programs-projects/concept-note-ai-rmf-profile-trustworthy-ai-critical-infrastructure
- NISTIR 8620 (2026) on digital-twin interoperability, VVUQ, cybersecurity and workforce challenges: https://doi.org/10.6028/NIST.IR.8620
- IDTA Asset Administration Shell Release 26-01 (including API v3.2.0) for industrial asset/twin information exchange where applicable: https://industrialdigitaltwin.org/en/content-hub/aasspecifications
- ISO 23247-1:2021, the published manufacturing digital-twin framework; applicability is sector/profile-specific: https://www.iso.org/standard/75066.html
- IETF RFC 9943 / SCITT for signed-statement transparency and independently verifiable receipts: https://www.rfc-editor.org/rfc/rfc9943
- W3C Verifiable Credentials Data Model 2.0 for issuer/holder/verifier credential exchange where suitable: https://www.w3.org/TR/vc-data-model-2.0/
- OpenTelemetry semantic conventions for telemetry naming, resource identity and event correlation, not as a substitute for measurement or evidence semantics: https://opentelemetry.io/docs/specs/otel/semantic-conventions/
### 9.1 Telecom timing baseline — current references checked 2026-11-07

Treat telecom PTP as a concrete, profile-specific benchmark family rather than a universal PNT threshold:

- **ITU-T G.8275 (01/2026)** — common aspects of PTP profiles for phase/time synchronization. Its optional Annex G defines synchronization-service availability performance metrics (SyncaaS), including node-side per-second available/unavailable service accounting and an NMS aggregation path. Applicability and optional-feature support must be recorded per implementation; do not assume every deployed clock exposes the dataset. https://www.itu.int/epublications/publication/itu-t-g-8275-2026-01-common-aspects-of-ptp-profiles-for-phase-time-synchronization
- **ITU-T G.8275.1/Y.1369.1 (02/2026), Amendment 1 (08/2026)** — current full-timing-support telecom PTP profile; the ITU recommendation database lists the base edition and Amendment 1 as in force (Amendment 1 prepublished at the time checked). Pin both exact editions in each interoperability manifest. https://www.itu.int/rec/T-REC-G.8275.1/en
- **ITU-T G.8275.2 (02/2026)** — partial-timing-support profile; keep it as a separate deployment profile, not a drop-in equivalent to G.8275.1. https://www.itu.int/rec/T-REC-G.8275.2/en
- **ITU-T G.8273.2 (06/2023)** — timing characteristics of telecom boundary clocks and time synchronous clocks under full timing support; relevant to equipment requirements and holdover behaviour, subject to the applicable device class. https://www.itu.int/epublications/publication/itu-t-g-8273-2-y-1368-2-2023-06-timing-characteristics-of-telecom-boundary-clocks-and-telecom-time-synchronous-clocks-for-use-with-full-timing-support-from-the-network
- **linuxptp** — open implementation/configuration comparator for G.8275.1 and G.8275.2; compare observed state and counters with native `ptp4l`/`pmc` outputs, not by assuming protocol implementation equals service assurance. https://www.linuxptp.org/documentation/configs/
- **External timing measurement** — use an independently calibrated phase/time reference or timing analyzer for reference validation where hardware is available. Device-reported PTP counters alone cannot establish physical output time error; record reference accuracy, calibration, measurement point, cabling/delay correction and time alignment.

Telecom timing benchmark observations should preserve at least: PTP profile/domain/version, clock identity and role, parent/grandmaster identity and quality, traceability/time-properties flags, port state, `synchronizationUncertain` where supported, parent/alternate-BTCA changes, path-trace/dependency data where supported, message counters, offset/path-delay performance records, SyncaaS availability counters where implemented, local reference state, holdover state and independently measured output time error where available.

Keep these dimensions separate:
- synchronization service availability (available/unavailable seconds and measurement validity/completeness);
- phase/time error and its distribution against a qualified reference;
- frequency stability / holdover performance under the relevant equipment profile;
- PTP topology, source traceability and common-mode dependencies;
- receiver/native source status and GNSS-derived time evidence;
- communications reachability and management-plane visibility.

Do not treat a PTP clock class, grandmaster identity, `timeTraceable` flag, source switch or service-availability counter as proof of physical time accuracy. A PTP monitoring implementation should report unavailable or incomplete measurement intervals explicitly, never as zero error or zero outage.

Useful interoperability and test references:
- EANTC 2026 PTP performance-monitoring interop report: a cascaded T-GM / seven-T-BC test setup compared PTP performance-monitoring metrics with Calnex and Keysight timing analyzers; treat as a test-method comparator, not a universal accuracy guarantee. https://wiki.eantc.de/wiki/publicreports/view/Main/EANTC%20Transport%20%26%20Cloud%20Networks%20Interop%20Test%20Report%202026/Time%20Synchronization/PTP%20Performance%20Monitoring
- Calnex PTP/time-error measurement guidance: independent reference measurements and controlled negative-condition testing. Vendor technical guidance, not a normative standard. https://www.calnexsol.cn/docman/techlib/timing-and-sync-lab/191-mifid-ii-time-compliance

- applicable GNSS receiver, telecom timing, maritime, transport and sector-specific requirements selected and reviewed by the relevant profile authority.

The profile must not use a draft concept note, standard mapping or schema as evidence of regulatory compliance or certification.

## 10. Evidence-promotion stage

Use the existing OMSI E0–E6 portfolio vocabulary **per capability and claim**, not one blanket stage for the entire PNT profile:

- E0 — Concept: scope, owner, intended decision and assumptions.
- E1 — Executable contract: validator, schemas, synthetic fixtures and negative controls executed.
- E2 — Reference validated: matched analytical/independent reference model and discrepancy analysis.
- E3 — External-data validated: rights-reviewed, held-out external dataset and uncertainty/calibration reporting.
- E4 — Operationally integrated: authorized source system/instrument, security/privacy review and operational owner.
- E5 — Shadow/controlled pilot: prospective operator comparison with human override and reconciled outcomes.
- E6 — Production-assured: limited scope with accountable owner and ongoing security, resilience, recovery and monitoring assurance.

These are internal portfolio stages, not external certification.

## 11. Initial acceptance gates

The v0.1 profile is ready for review when:
- sector/use profiles define their own requirements and thresholds;
- Mycelix, Symthaea, Symtropy and Sol-Atlas ownership is preserved;
- source/time/calibration/failure-domain evidence remains typed and separately addressable;
- missing dependencies and unknown oracle provenance remain explicit;
- synthetic common-mode, clock, stale-calibration, receiver-fault, weak-geometry, false-degradation and recovery controls exist;
- run receipts bind exact source, split, label, code/model/configuration and environment artifacts;
- read-only adapters precede any consequential-operation integration;
- no claim is promoted beyond the actual evidence stage.

## 11.1 Frozen synthetic conformance vector design

A machine-readable, design-only PNT vector pack is maintained at [pnt-conformance-vectors-0.1.json](./pnt-conformance-vectors-0.1.json). It contains 16 synthetic positive and negative cases for source/calibration/time validity, common-mode dependencies, oracle independence, epistemic abstention, open-set RF boundaries, degraded capture, communication partitions, recovery, authority and probe-impact evidence.

**Status:** `FROZEN_DESIGN_NOT_EXECUTED`. These are expected outcomes and invariant assertions, not test results. Issue #307 owns integration into the existing OMSI validator tooling after the shared validator path is available. The vector pack does not define sector thresholds or prove receiver, service, field or safety qualification.

## 12. Non-goals

- No global PNT health score or universal alert limit.
- No replacement of receiver anti-jam/anti-spoof hardware or vendor-certified protection.
- No offensive RF, transmitter construction, waveform optimization, emitter targeting or weapon/kinetic interface.
- No automatic service actuation from an anomaly score.
- No aviation/maritime/telecom certification claim.
- No raw telemetry firehose in a DHT.
- No inference of actor/intent from receiver anomaly.
- No universal claim that two sources without an identified shared dependency are physically independent.
