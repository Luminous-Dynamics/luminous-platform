# Luminous Platform Product Architecture 0.1

**Status:** Draft product architecture for review  
**Date:** 2026-11-07  
**Owner:** Luminous Platform / OMSI  
**Tracker:** [LP-PRODUCT-001 #301](https://github.com/Luminous-Dynamics/luminous-platform/issues/301)  
**Portfolio:** [OMSI industry-profile programme #154](https://github.com/Luminous-Dynamics/luminous-platform/issues/154)

## 1. Executive product thesis

Luminous Platform is the **sovereign, local-first edge operations and verified lifecycle product** for deploying, observing, maintaining and recovering distributed technical systems.

Its differentiated promise is not that it contains the largest AI model, replaces NixOS tooling, or centralizes every device's raw telemetry. Its promise is that operators can answer, with evidence:

1. Which devices and software revisions are actually present?
2. Which observations are current, calibrated, trusted for this use, stale, inferred or unknown?
3. What may explain a degradation, and what evidence would distinguish the hypotheses?
4. Which maintenance or update plan is permitted, bounded and reversible?
5. Did the intended change actually occur?
6. Did the system recover to the required postcondition?
7. Can an independent reviewer reconstruct the decision and outcome when the site is offline?

The core product loop is:

```text
observe
  -> qualify evidence
  -> diagnose with uncertainty
  -> propose a bounded plan
  -> obtain operator / owner authorization
  -> execute through an existing effect boundary
  -> independently verify the postcondition
  -> retain a portable receipt
  -> learn without promoting unsupported claims
```

**First product wedge:** evidence-bound site/device operations and verified recovery for small distributed technical or industrial sites. PNT resilience is a later sector-specific profile, not the first general product claim.

## 2. Product boundary and non-goals

Luminous Platform packages existing engines, deployment primitives, device adapters and operator workflows into a coherent product. It must not create a second implementation of:

- NixOS configuration/deployment;
- generic embedded Linux OTA update;
- cryptographic identity, provenance or durable trust;
- generic telemetry storage or global event firehose;
- AI inference or causal reasoning;
- spatial truth or navigation;
- physical simulation;
- authorization and tool-effect control.

The Platform owns product packaging, deployment composition, site experience, adapter admission, operator workflow, cross-component integration, profile mapping, recovery coordination and evidence presentation.

Domain-specific semantics remain with the existing owners.

## 3. System context and ownership

| Component | Product responsibility | Boundary |
|---|---|---|
| Luminous Platform / OMSI | Product packaging, site workflow, profile admission, operator experience, adapter integration and lifecycle orchestration | Does not become a second engine or evidence/authority ledger |
| Nixward | NixOS host management, machine contracts, diagnosis and deployment state | Existing NixOS management owner |
| luminous-edge | Reproducible composition of platform NixOS modules | Meta-flake / deployment composition |
| sovereign-boot | Optional boot/recovery state machine | Disabled by default; never required for host boot |
| Sovereign Ops | Fleet intent and operations console | Intent and approval UX; does not directly mutate host configuration |
| Xenia | Authenticated peer/site communications | Transport, not truth or authority |
| Symthaea | AI diagnostic hypotheses, anomaly ranking, uncertainty, abstention and evidence-aware assistance | Recommendations do not authorize actions |
| Mycelix | Source identity, provenance, trust-root and authority/evidence integrations where already implemented | No new parallel platform trust score |
| Symtropy | Deterministic consequence simulation where a validated model/profile exists | Simulation is not observed field truth |
| Sol-Atlas | Read-only spatial, asset and dependency projection | Visualization does not grant mutation authority |
| OMSI | Cross-industry profile, operation lifecycle and effect/verification contract | Domain profiles retain their own physics, units, hazards and acceptance criteria |

The Platform should expose stable, versioned integration contracts rather than import internal implementation details from every engine.

## 4. Logical architecture

### 4.1 Edge and device layer

Devices may include NixOS hosts, embedded Linux gateways, industrial controllers, receiver appliances and optional microcontroller sensor nodes.

Each device has an exact capability manifest bound to:
- manufacturer, model, board and hardware revision;
- firmware, bootloader and configuration revision;
- available sensors, interfaces, memory and radio capabilities;
- supported update/recovery method;
- source time basis and local clock/boot epoch;
- calibration and repair lifecycle where applicable.

A device identifier or signature authenticates a claim's origin; it does not prove sensor accuracy or physical truth.

### 4.2 Local site plane

The local site plane remains useful during WAN, DNS, cloud, identity-service or central-console outages.

It provides:
- local observation and bounded evidence buffering;
- local health and recovery status;
- device inventory and software/configuration state;
- read-only diagnostics by default;
- explicit local policy and operator approval for any separately authorized mutation;
- store-and-forward synchronization after connectivity returns.

Loss of central connectivity must not convert unknown state into healthy state. It also must not automatically make a locally functioning site unavailable.

### 4.3 Evidence and adapter plane

Adapters normalize vendor-specific observations without erasing:
- source identity and source artifact reference;
- observed, received and authored times as distinct fields;
- clock domain / time scale and uncertainty;
- units, calibration, sensor/receiver/firmware revision;
- raw, normalized, derived, inferred, simulated and unknown status;
- loss, transformation and mapping limitations.

Raw high-rate telemetry remains local or in an explicitly selected time-series system by default. Durable provenance systems receive bounded summaries, important state transitions and evidence commitments when policy permits; they are not used as a global raw-telemetry bus.

### 4.4 Cognitive assistance plane

Symthaea may:
- rank anomaly and fault hypotheses;
- identify contradictory or missing evidence;
- propose the next diagnostic observation;
- summarize logs and current state with exact evidence references;
- draft maintenance or update plans;
- estimate uncertainty and explicitly abstain.

It may not:
- turn an inferred condition into a measured fact;
- turn a signature into physical truth;
- create or expand its own permissions;
- invoke a physical mutation without the existing authorization path;
- declare success without an independently observed postcondition.

### 4.5 Consequence and spatial context

Symtropy may replay a proposed change or degradation scenario where a validated model and operating envelope exist. Sol-Atlas may project assets, dependencies and potential impact read-only.

A simulation result remains simulated. A dependency map remains a model until its edges are supported by current evidence. Unknown dependencies must remain visible.

### 4.6 Operations, authority and verification

The operator experience coordinates:
1. current-state evidence;
2. causal hypotheses and uncertainty;
3. proposed action and exact target scope;
4. required human/owner approval;
5. bounded execution attempt;
6. observed participant effect;
7. independent postcondition check;
8. rollback / recovery state;
9. immutable or tamper-evident receipt.

The actor performing an action must not be the sole verifier of its outcome. A command accepted by an agent is not proof that the intended state was achieved.

## 5. Build-versus-compose policy

| Capability | Initial approach | Do not build |
|---|---|---|
| NixOS deployment | Evaluate existing tools such as Bento, Colmena, comin, deploy-rs, Morph and native Nix workflows against the exact platform needs | Another generic NixOS deployment engine |
| Embedded Linux OTA | Integrate a qualified updater such as RAUC or Mender where suitable; preserve signing, compatibility, A/B/recovery and post-boot confirmation | Another generic OTA client |
| Update trust | Map TUF/Uptane principles and metadata lifecycle where relevant | A new root-of-trust or signing framework |
| Observability | Consume OpenTelemetry, Prometheus, Zabbix, NetBox and vendor telemetry through source adapters | A new universal metrics and inventory ecosystem |
| Communications | Use Xenia and existing transport contracts | A second transport/security protocol |
| AI diagnosis | Use Symthaea and existing reasoning components | A duplicate model server / agent runtime |
| Spatial/digital twin context | Use Mycelix Position, Symtropy and Sol-Atlas owners | A second spatial, fusion or simulation engine |

Reference material:
- NixOS operations/deployment tools: https://wiki.nixos.org/wiki/Applications
- RAUC: https://github.com/rauc/rauc
- Mender: https://github.com/mendersoftware/mender
- The Update Framework: https://theupdateframework.io/
- OpenTelemetry: https://opentelemetry.io/docs/specs/otel/

The integration decision must be recorded per exact device, version, deployment mode and recovery method. Tool availability alone does not qualify a device lifecycle.

## 5.1 Competitive boundary and differentiation

The product must not claim that local edge execution, offline operation, fleet updates or rollback are unique Luminous capabilities.

| Existing category / product | Existing capability to respect | Luminous response |
|---|---|---|
| NixOS deployment tools (Bento, Colmena, comin, deploy-rs, Morph, etc.) | Reproducible host realization and deployment across NixOS fleets | Choose the simplest compatible tool for each target; differentiate in lifecycle evidence, cross-vendor operations and verified recovery, not a new deploy engine |
| Mender | Embedded Linux OTA, A/B rootfs updates, rollback after failed boot and standalone deployment without a server | Compose where board/partition integration is suitable; add source/board/firmware evidence and independent application/site postcondition verification |
| RAUC | Signed bundle/update flows and bootloader-integrated slot handling for embedded Linux | Reuse where target integration fits; avoid duplicating update signing and slot/rollback logic |
| AWS IoT Greengrass v2 | Local software components, messaging, local data processing, fleet management, and operation with intermittent connectivity | Treat as a capable cloud-connected alternative; differentiate through sovereign/local-first deployment composition and vendor-neutral evidence/authority/recovery contracts |
| Monitoring/inventory products | Mature metrics, alerts, device inventory and topology views | Ingest their data with loss-aware adapters; do not replace them without a matched migration case |

Reference facts as of 7 November 2026:
- AWS says Greengrass v1 support ended 1 June 2026 and recommends v2 migration; Greengrass v2 is its current runtime: https://aws.amazon.com/greengrass/ and https://docs.aws.amazon.com/greengrass/.
- AWS Greengrass v2 supports local processing and device messaging with intermittent or absent cloud connectivity: https://docs.aws.amazon.com/greengrass/latest/developerguide/what-is-iot-greengrass.html.
- Mender describes dual A/B root filesystem updates and rollback if the updated image fails to boot: https://mender.io/engineers/how-mender-works.
- Mender supports standalone deployments without a Mender Server, including local/USB-delivered artifacts: https://docs.mender.io/artifact-creation/standalone-deployment.
- NixOS deployment tooling remains a plural ecosystem; the official wiki lists Bento, Colmena, comin, deploy-rs and others, and labels NixOps unmaintained: https://wiki.nixos.org/wiki/Applications.

### What Luminous should own

The defensible differentiator is **evidence-qualified cross-tool lifecycle assurance**:
1. Preserve exactly which device, board revision, firmware, configuration, sensor calibration and source observations justify a diagnosis.
2. Distinguish measured, reported, inferred, simulated, stale and unknown state across providers.
3. Keep AI diagnosis separate from operator authority and effect execution.
4. Verify application/service postconditions independently after an update, repair or recovery; a successful boot or updater status alone does not prove the service is healthy.
5. Preserve local continuity and auditable receipts during network/platform outages, then reconcile without silently rewriting source chronology.
6. Quantify operator effort and verified recovery cost against the existing toolchain.

Do not claim “offline-first” or “rollback” superiority without a matched comparison against Greengrass v2, Mender/RAUC and the chosen NixOS deployment tool on the same hardware and failure scenarios.

## 6. First demonstrator and scope

Use one isolated lab site with:
- one NixOS host managed through Nixward;
- one embedded Linux target, initially read-only if the update/recovery path is not yet qualified;
- optionally one low-power auxiliary sensor node from [OMSI-EDGE-SENSOR-001 #300](https://github.com/Luminous-Dynamics/luminous-platform/issues/300);
- a local-only observer and bounded evidence store;
- one operator workflow for a seeded, reversible fault;
- one human-approved maintenance/update exercise on a disposable target;
- a network partition and power-loss test;
- an independent postcondition verifier and portable receipt.

Do not begin with live production OT mutation, autonomous fleet-wide rollout, or a certified PNT/aviation/maritime claim.

### Initial user/customer hypothesis

Interview operators of small distributed technical/industrial sites who have mixed NixOS/Linux systems, intermittent connectivity, limited local IT support and a need to prove that maintenance or updates actually succeeded.

Potential discovery settings include engineering labs, remote operations sites, small utilities, warehouses and port-adjacent facilities. These are hypotheses, not validated market segments. Pricing, buyer and ROI claims require customer evidence.

## 7. Benchmarks and product metrics

Establish the current manual/tooling baseline before setting numerical targets. Measure separately:

| Dimension | Measurement |
|---|---|
| Installation | Time to install and configure a site; reproducibility of host/build identity |
| Inventory | Exact model, revision, firmware and configuration coverage; unknown-field rate |
| Diagnosis | Time to identify a seeded fault; hypothesis calibration; evidence traceability |
| AI quality | False diagnosis/recommendation, selective risk, abstention, operator correction rate |
| Update/recovery | Update success, rollback success, power-loss recovery, false success |
| Authority | Unauthorized effect count; exact grant/target/action binding; approval burden |
| Verification | Independent postcondition success and receipt reconstruction rate |
| Offline continuity | Local observation, buffering and safe status during network/platform outage |
| Economics | Operator minutes/device-month; cost per verified recovery; support effort and failed-attempt cost |

Do not collapse these into a single composite score. A safety/authority failure cannot be compensated by better inference latency or a high AI score.

## 8. Release and evidence stages

Use explicit internal evidence stages. They are not external certification:

- **E0 — Conceptual:** architecture and requirements only.
- **E1 — Synthetic:** deterministic fixtures, failure injection and contract tests.
- **E2 — Reference validated:** exact hardware/software build, calibrated lab inputs, resource and recovery measurements, independent replay.
- **E3 — Held-out external validation:** unseen device/site/data and independent evaluation.
- **E4 — Shadow pilot:** authorized read-only operation in a real site, with no autonomous effects.
- **E5 — Production profile:** only after domain owner, security, operational recovery and relevant regulatory/sector requirements are separately satisfied.

The current proposal is not yet a production or field-validated product.

## 9. Security, privacy and lifecycle requirements

- Secure boot, signed artifact verification, update metadata expiry and rollback / anti-rollback policy where supported.
- Tested recovery paths for power loss during update, storage write and evidence commit.
- Read-only by default; all mutations route through the existing effect/authorization boundary.
- Model outputs do not hold direct device or host mutation capabilities.
- Raw measurements are minimized and retained locally unless explicitly authorized for export.
- Offline logs are bounded, integrity checked and privacy controlled.
- Repair or component replacement triggers the relevant calibration, firmware and profile requalification.
- Device and firmware revocation / currentness state is explicit.
- Public or customer-facing views distinguish observation, inference, simulation, recommendation, authorization, execution and verified outcome.

## 10. Standards and interoperability context

Potential mappings include:
- NISTIR 8620 (2026) for digital-twin interoperability, verification/validation/uncertainty quantification and cybersecurity research priorities: https://doi.org/10.6028/NIST.IR.8620
- NIST AI RMF and the developing critical-infrastructure AI profile: https://www.nist.gov/programs-projects/concept-note-ai-rmf-profile-trustworthy-ai-critical-infrastructure
- TUF/Uptane for update trust and metadata lifecycle.
- OpenTelemetry semantic conventions for telemetry naming and correlation, not as a substitute for evidence semantics.

A mapping exercise is not compliance or certification.

## 11. Related implementation owners

- Product and packaging: [LP-PRODUCT-001 #301](https://github.com/Luminous-Dynamics/luminous-platform/issues/301)
- Edge sensors and local inference: [OMSI-EDGE-SENSOR-001 #300](https://github.com/Luminous-Dynamics/luminous-platform/issues/300)
- PNT sector profile: [OMSI-PNT-001 #297](https://github.com/Luminous-Dynamics/luminous-platform/issues/297)
- Industry admission: [OMSI-INDUSTRY-001 #154](https://github.com/Luminous-Dynamics/luminous-platform/issues/154)
- Cross-industry effect boundary: #168
- Vendor/firmware lifecycle: #23
- Repair-aware device qualification: #131
- Open-hardware gateway: #71

## 12. Claim ceiling

This document is a proposed product architecture. It does not establish production readiness, customer deployment, update-system superiority, industrial interoperability, security certification, PNT integrity, airworthiness, maritime qualification or autonomous safe operation. Every consequential effect remains within the existing participant and authority boundary.

A product release requires exact-head builds, independent verification, matched external comparisons, operational recovery evidence and customer validation.
