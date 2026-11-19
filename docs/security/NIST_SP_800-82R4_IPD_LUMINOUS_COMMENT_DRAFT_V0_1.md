# Luminous technical comment draft — NIST SP 800-82 Rev. 4 IPD

**Status:** Internal review draft only — not submitted to NIST  
**Reviewed:** 2026-11-19  
**Document:** NIST SP 800-82 Rev. 4, Initial Public Draft (September 2026)  
**Comment deadline shown on NIST page:** November 30, 2026  
**Official publication page:** https://csrc.nist.gov/pubs/sp/800/82/r4/ipd  
**Official draft DOI:** https://doi.org/10.6028/NIST.SP.800-82r4.ipd

## 1. Scope and claim ceiling

This draft is prepared as a technical review aid for Luminous Dynamics. It is not an official organizational position, has not been approved by the user or any legal/compliance authority, and has not been submitted. Any external submission requires owner review, correction of organizational attribution, and approval of the final wording.

This review considered the IPD's discussion of OT asset discovery and baselines (Sec. 4.1.1–4.1.2), data in transit (Sec. 4.2.2.4), emerging AI/ML/digital-twin detection (Sec. 4.3.2.4), OT cybersecurity architecture (Sec. 5), management network architecture (Sec. 5.2.1), zero-trust integration (Sec. 5.3), and the associated examples. This is not a line-by-line review of all 321 pages.

## 2. Overall assessment

The draft's direction is valuable. In particular, it:
- treats OT availability, reliability and safety as distinct requirements;
- expands coverage to BACS, food/agriculture, freight rail, maritime, WWS, IIoT and cloud convergence;
- organizes guidance around NIST CSF 2.0;
- strengthens asset management and network monitoring guidance;
- distinguishes management and operational network functions;
- discusses zero trust in an OT-aware way rather than assuming every controller can run a conventional IT agent;
- recognizes prompt injection, compromised training data, non-deterministic AI behavior, and digital-twin synchronization/drift as OT risks.

The following comments aim to make selected recommendations easier to operationalize and evaluate across mixed-vendor and legacy environments.

## 3. Suggested technical comments

### Comment 1 — Make asset-discovery safety and provenance explicit

**Relevant sections:** 4.1.1.1, 4.1.2.

The draft appropriately recognizes several asset-discovery approaches, including vendor management tools, passive monitoring, OT-aware scanning, independent active scanning and manual inspection. It also notes that active scanning may affect availability and should be tested before production use.

**Suggested clarification:** recommend that each asset-inventory record preserve the discovery method, tool/version, collection point, time, asset identity confidence, scope, coverage limitations and approval basis. For active discovery in OT, require a documented device/vendor compatibility review and a non-production test or explicitly approved planned maintenance window before production use. Passive discovery should record sensor placement and known blind spots, including isolated switch segments and non-IP networks. Manual inventory should remain a valid first-class method for isolated or unsupported devices.

**Rationale:** a discovered-asset count without a method and coverage statement can overstate completeness. An active scan that disrupts a controller is not a security improvement. Inventory completeness should be a qualified claim, not inferred from one tool's output.

**Potential evaluation:** discovery recall by device class; false identification rate; unobserved-segment rate; scanner-induced service impact; percentage of assets with exact owner/model/firmware/criticality; percentage of unknown assets and connections left unresolved.

### Comment 2 — Treat management, operational and security-monitoring networks as distinct functions

**Relevant sections:** 5.1 and 5.2.1.

The draft's distinction between management traffic and operational process traffic is important. It also describes separating external vendor access and isolating security-monitoring infrastructure.

**Suggested clarification:** provide a consistent network-zone/conduit inventory that records, at minimum, management, operational, enterprise/IT, OT DMZ, external vendor access and security-monitoring functions. Where practical, security-monitoring infrastructure should receive telemetry over one-way or otherwise tightly constrained paths and should not share the same administrative reachability as the systems it monitors. Any bidirectional exception should identify its purpose, owner, enforcement point, credential domain and compensating controls.

**Rationale:** network labels and VLAN IDs do not prove isolation. Shared switches, credentials, management APIs, identity infrastructure or routing can collapse an apparently separated design into one common failure domain.

**Potential evaluation:** cross-zone reachability; vendor-path credential independence; management-to-operational route tests; monitoring-zone ingress/egress tests; one-way telemetry verification; shared infrastructure and common-cause inventory.

### Comment 3 — Apply zero trust to compatible OT resources without requiring unsupported agents on every controller

**Relevant section:** 5.3 and 5.3.1.

The draft correctly notes that many PLCs, controllers and HMIs may not support the mechanisms required for a full zero-trust implementation.

**Suggested clarification:** explicitly describe gateway-mediated enforcement as a valid compensating pattern for legacy devices. A policy enforcement point can authenticate the human/workload, validate the exact target, command, protocol function and operating window, and then mediate a narrowly allowed request to a legacy endpoint. The final local controller/safety interlock remains authoritative for physical safety. Device/network location alone must not become permission.

**Rationale:** forcing agents or modern identity stacks onto unsupported controllers can create availability, warranty and safety risks. A gateway can improve control only if its own scope is constrained, the controller remains protected, and bypass routes are inventoried.

**Potential evaluation:** gateway bypass attempts; exact function/parameter authorization; replay and duplicate-command handling; local-controller refusal; legacy-device behavior under gateway loss; positive controls for necessary local process traffic.

### Comment 4 — Strengthen AI/ML and digital-twin assurance around authority and physical truth

**Relevant sections:** 4.3.2.4 and the AI-related threat/vulnerability discussion.

The IPD already identifies prompt injection, compromised training data, sensitive-data disclosure, probabilistic model behavior and digital-twin drift/bidirectional connectivity as relevant concerns.

**Suggested clarification:** complement these threat descriptions with an explicit boundary rule: AI/ML and digital-twin outputs may provide advisory detections or proposals, but they do not independently authorize physical actuation. Consequential operations should require exact, current authority at the participant/controller boundary, deterministic precondition checks and independent postcondition/physical observation. Model version, training-data provenance, sensor lineage, calibration, model drift, uncertainty and twin/physical divergence should be recorded for the claimed scope.

For bidirectional twins, document separate read/synchronize/write capabilities, the physical destinations reachable through each connection, and the independent safety mechanism that prevents a compromised twin from changing a physical process outside its allowed envelope.

**Rationale:** model accuracy or twin synchronization is not equivalent to verified physical state. A compromised baseline can make an anomaly detector confidently wrong; a stale twin can suppress a real fault; a bidirectional integration can turn a monitoring system into a control path.

**Potential evaluation:** prompt-injection success; training-data poisoning sensitivity; model-version reproducibility; false negative/false positive rate; calibration and abstention; twin drift detection; compromised-twin command rejection; simulated-versus-measured outcome disagreement; local safety-controller refusal.

### Comment 5 — State the availability and latency budget for cryptographic protection

**Relevant section:** 4.2.2.4.

The draft recognizes that encryption and authentication may affect OT communication latency.

**Suggested clarification:** encourage measurement against the actual control application's timing, jitter, failover and recovery requirements. A cryptographic tunnel or protocol should not be called suitable solely because it encrypts traffic; it should be qualified under the exact network topology, device capability and failure conditions. Legacy protocols without native cryptography may require a compensating boundary, but the gateway's availability and recovery characteristics must also be considered.

**Potential evaluation:** p50/p95/p99 latency and jitter; packet loss/retransmission; controller scan-cycle impact; timeout behavior; rekey/credential-expiry outage; fallback semantics; safe process behavior during loss of the security gateway.

### Comment 6 — Distinguish monitoring evidence from process-state truth

**Relevant sections:** 4.3.2 and 5.

**Suggested clarification:** telemetry, alarms and digital-twin predictions should retain source identity, sensor calibration, timestamp/time-quality, collection path, transformation history and known dependencies. Multiple dashboards or relays that derive from one sensor/collector should not be treated as independent corroboration. For safety-relevant outcomes, use an independent measurement or local controller observation where feasible.

**Rationale:** a cryptographically protected log may be authentic while reflecting a compromised sensor, an unrepresentative model baseline or a shared collection failure.

**Potential evaluation:** stale/replayed sensor data detection; common-source independence errors; timestamp drift; dropped monitoring segments; observer outage; reconciliation against independent instrument measurements.

## 4. Candidate cross-cutting test profile

A useful evaluation episode could record the following states separately:

1. Asset and firmware identity/criticality.
2. Discovery method and discovery-safety approval.
3. Declared network zones, conduits, sensor coverage and unknown routes.
4. Human/workload/device identity and current authority.
5. AI/twin proposal, model and data provenance, uncertainty.
6. Policy decision and exact authorized operation.
7. Independent local-controller acceptance or refusal.
8. Measured physical postcondition and observer dependencies.
9. Recovery, safe-state continuity and human impact.

The evaluator should own hidden asset state, expected outcomes and common-cause dependencies. A system should not gain assurance credit by refusing all legitimate operations; positive safe-operation controls and benign false denial should be reported beside attack prevention.

## 5. Luminous internal work that motivated these comments

These are internal research artifacts, not claims of conformance or deployment:
- [OSI/XAP profile PR #498](https://github.com/Luminous-Dynamics/luminous-platform/pull/498)
- [L2/L3 deployment assurance issue #499](https://github.com/Luminous-Dynamics/luminous-platform/issues/499)
- [Machine-readable OSI/XAP control and scenario files](https://github.com/Luminous-Dynamics/luminous-platform/tree/research/osi-cross-layer-assurance-v0-1/docs/security)
- Existing hardware/firmware and edge qualification: Platform #23 and #300/#349.
- Existing cross-domain evidence benchmark issue: Platform #325.

## 6. Submission checklist — not yet performed

- [ ] Review every comment against the complete current IPD and its comment template.
- [ ] Separate editorial suggestions from technical recommendations.
- [ ] Confirm that each recommendation is supported by OT domain experts and exact use cases.
- [ ] Check intellectual-property/patent notice and NIST submission instructions.
- [ ] Confirm organization/author attribution and authorization.
- [ ] Obtain explicit approval before submission.
- [ ] Submit to the NIST-designated address before the published deadline, if approved.

**No comment has been sent.**
