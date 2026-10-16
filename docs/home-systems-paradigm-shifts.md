# Paradigm Shifts for Open Home Systems

**Status:** Research and design direction; proposals below are not implementation claims.  
**Purpose:** Define how Luminous can compete at the level of system architecture and ownership economics, not merely product features.

## Thesis: stop building another smart-home box

The category to pursue is a **Sovereign Home Operating Environment**: a household-owned, locally operable computing foundation that hosts automation, storage, entertainment, optional cognition, and later community compute without making any one service or vendor the source of authority.

The competitive unit is not the appliance. It is the complete lifecycle: install, understand, operate offline, recover, update, repair, replace components, export data, change providers, and leave without asking the original vendor for permission.

“Better than everything” is not defensible before comparative measurement. The credible ambition is to make owner control, graceful degradation, reproducibility, repairability, and honest evidence first-class properties—and then publish head-to-head measurements.

## 1. Paradigm shifts

| Current industry default | Proposed shift | Measurable engineering consequence |
|---|---|---|
| A smart-home hub is the product | A household operating environment is the product | Multiple replaceable services; component upgrades do not erase local authority |
| Cloud account is the connective tissue | Local control is the baseline; remote connectivity is an optional adapter | Essential flows pass tests with WAN and public DNS unavailable |
| Integration count tells the compatibility story | Exact capability and version contracts tell the compatibility story | Each integration records device/model, firmware, protocol, software version, tested scope, limitations, and evidence freshness |
| An app's permissions are implicit | Every service declares required capabilities before enablement | No service starts without a reviewed permission/data/network/resource manifest |
| A sent command equals success | Dispatch and verified effect are separate states | Missing or contradictory observations produce “unknown / needs reconciliation” |
| AI is promoted as the user-facing brain | Cognition is an optional advisor behind a narrow policy boundary | Host boot, manual controls, and essential automations work with inference removed |
| Backup is a checkbox | Recovery is an executable, timed property | Restore runs use fresh virtual disks and identities; recovery time and data-loss windows are measured |
| Security is a release checklist | Security maintenance is a product lifecycle | Supported-version windows, update ownership, disclosure, revocation, and end-of-life export are published |
| “Open” is a logo or repository | Openness is a component-level bill of rights | Source, firmware, blobs, BOM, licences, repair procedure, and reproducibility limits are individually declared |
| Device value is measured at purchase | Lifecycle quality is measured over years | Measure energy, repair time, parts availability, support lifetime, lifecycle cost, and disposal |
| One appliance should do everything | Capability tiers and interoperable roles | Low-power control/storage roles need not inherit gaming GPU power draw or robotics risk |
| Centralized orchestration is assumed superior | Small local authorities compose through bounded adapters | A failed coordinator or cognitive component cannot seize direct control or take the local system offline |
| Peer compute is a hidden utilization opportunity | Community compute is a voluntary, revocable lease of a resource budget | Owner sets allowed jobs, data classes, resource ceilings, network access, quiet hours, and cancellation |
| A single score certifies quality | A vector of independent claims with explicit ceilings | Critical privacy/safety failures cannot be averaged away by performance or convenience |

These are design hypotheses. They become meaningful only when tests and user studies demonstrate better outcomes than alternatives.

## 2. From services to capability contracts

Before a workload, integration, extension, model, or peer job is enabled, require a **capability contract** describing:

- **Identity and subject:** exact package/service/provider version and digest where available.
- **Purpose and owner:** why it is present, who enabled it, who can revoke it, and how the owner inspects it.
- **Permissions:** explicit files, devices, APIs, privileged operations, and physical effects it may request.
- **Data contract:** input/output categories, storage, retention, export, deletion, and whether data can leave the home.
- **Network contract:** allowed destinations/protocols, ingress policy, DNS dependency, and offline behaviour.
- **Resource contract:** CPU, memory, GPU/VRAM, storage, bandwidth, runtime, and priority ceilings.
- **Failure contract:** timeout, restart, retry, cancellation, partial work, recovery, and which other capabilities must continue if this one fails.
- **Authority contract:** proposal versus authorization versus dispatch; reversibility, required consent, and independent outcome observation.
- **Evidence contract:** applicable tests, evidence class, freshness/revalidation conditions, claim ceiling, and stable issue/report reference.

Default-deny should be an executable admission rule, not a UI convention. A contract requesting broader permissions than the owner granted must fail to start or run in a restricted mode. Unknown capabilities stay unavailable; they do not acquire authority from model confidence, a vendor label, a signature alone, or another service's reputation.

This becomes an interoperability layer: Home Assistant, local storage, media, inference, game runtimes, and eventually Mycelix peers compose through narrow, inspectable contracts instead of receiving ambient trust because they share a host.

## 3. Cognition proposes; policy decides; mechanisms enforce

Separate the architecture into six planes:

1. **Observation:** collects host/device facts with provenance, timestamps, and uncertainty.
2. **Proposal:** rules, user interfaces, and Symthaea may suggest plans but cannot perform privileged effects directly.
3. **Policy and consent:** decides whether a subject, requester, operation, scope, time, and context are permitted.
4. **Execution:** narrow adapters dispatch only admitted operations and enforce range, duration, rate, and idempotency bounds.
5. **Reconciliation:** compares independently observed outcomes to intended postconditions and records success, failure, or unknown.
6. **Recovery:** offline, minimal, and separate from optional dashboards, inference, overlays, and peer networks.

For safety-relevant actions, software permission never replaces suitable physical interlocks, manual control, or an emergency stop. A successful HTTP response or AI-generated explanation is not proof of a physical outcome.

This design makes AI more useful by giving it high-leverage advisory work without unrestricted authority. The same architecture remains usable with no AI at all.

## 4. Shift reliability from uptime to graceful degradation

Traditional availability numbers can hide unacceptable failure modes. Define essential owner journeys and inject faults across their dependencies:

- WAN and public DNS absent.
- Inference process crashed or removed.
- Dashboard unavailable.
- Local automation provider restarted.
- Disk or log partition full.
- Update interrupted or signature rejected.
- Backup target unavailable, then restored.
- Administrator credential revoked.
- Peer/network coordinator unavailable.

For each scenario declare what **must continue**, what may degrade, what must stop, what must be observable, and how a human can recover. Test actual service dependencies, not only whether a VM boots.

A key demonstration should be the **Home Continuity Drill**: in a disposable environment, break the network, crash optional cognition, consume a bounded resource, interrupt a test update, and restore from backup. Publish the exact revision, injected faults, assertions, logs, timings, and failures. Do not collapse these results into one blended score: a critical invariant violation blocks advancement regardless of the other results.

## 5. Make owner sovereignty observable and enforceable

“Local-first” is not the same as sovereignty. A local service may still be impossible to migrate, audit, disable, or use without a vendor account.

Require visible owner controls for:
- Running services and their effective permissions.
- Data leaving the home and the reason for it.
- Device/account enrollment, owner transfer, and revocation.
- Remote access, disabled until explicitly configured.
- Compute contribution, including immediate local disable and cleanup.
- Export, deletion, backup retention, and key recovery.
- Updates: source, version, signing identity, changes, support window, and rollback limitations.
- A tested end-of-life path that preserves user data and, where practical, lets the owner install another OS or host.

Add a recurring **exit test**: migrate an owner's configuration and data to a clean replacement VM without contacting the original service vendor. Track data loss, manual work, and undocumented formats. Make exit a supported workflow, not a threat to retention.

## 6. Treat evidence as a graph with expiration, not a green badge

Every capability claim should link to:
- Exact source tree, profile, artifact, and dependency versions.
- Test specification, method, parameters, environment, and input digests.
- Per-test outcomes, expected and observed values, logs, and raw artifacts.
- Evidence class: static, unit, VM/simulation, physical bench, field, or independent review.
- Claim owner, limitations, and next review/retest conditions.

Revalidation triggers include relevant changes to the kernel, driver, firmware, hypervisor, isolation policy, data schema, service version, test harness, trust key, or physical subject. Not every change invalidates everything; affected claims must become stale until an explicit equivalence or retest policy resolves them.

Cryptographic signatures authenticate an evidence producer and protect integrity within a trust chain. They do not prove an experiment happened, a sensor was truthful, or a conclusion followed. A verifier must validate evidence contents and policy while preserving unknown and incomplete outcomes.

The first implementation seam is now explicit: [home-capability-contract.schema.json](../schemas/home-capability-contract.schema.json) describes a workload's request, and the separate Rust capability-contract CLI applies structural and semantic checks. The CLI deliberately rejects self-declared qualified status until independent evidence verification exists. The schema and CLI still do **not** enforce requested permissions against a running host; a separate owner-controlled grant and runtime enforcement layer must do that before this can be called an admission-control system.

The cross-project boundary is specified in [the Mycelix Hearth integration contract](home-systems-myelix-hearth-integration.md) and [Home Effect Request Envelope schema](../schemas/home-effect-envelope.schema.json). It treats the home workload declaration as a request only and routes effects through Hearth's exact adapter-admission path; it does not introduce a second authority source.

The existing product-profile validator and this capability validator are foundations for the shift, not a complete evidence verifier or runtime reference monitor.

## 7. Optimize lifetime serviceability, not one-time openness claims

Publish a component-level manifest: manufacturer/model, board revision, firmware, drivers, binary blobs, licence, supply/support status, replacement options, tool requirements, and repair instructions.

Measure:
- Time and skill needed to replace common failure items.
- Data preservation during component/host replacement.
- Parts availability and supported firmware lifetime.
- Time from upstream security fix to tested project release.
- Ability to rebuild after an upstream disappears.
- Total cost of ownership, idle energy, energy per task, noise, and thermal margin on the exact physical configuration.

Published CAD alone does not prove repairability. A reproducible software build does not prove that a third party can manufacture a safe physical device. Keep these claims separate.

## 8. Product strategy: small sovereign roles, shared contracts

Do not force every product into one hardware profile.

- **Hearth Node:** low-power, quiet, reliable, easy to back up, independently recoverable. Start here.
- **Arc Console:** may require a discrete GPU and higher cooling; maintain game-specific compatibility profiles without letting console demands compromise Hearth essentials.
- **Commons Compute:** opt-in and isolated, with workload-specific validation and local-service priority.
- **Symthaea Robotics:** progress through software-in-the-loop and a single instrumented joint, with cognition above bounded deterministic control and an independent physical stop.

Share identity, consent, capability contracts, update/recovery policy, and evidence semantics. Do not share a universal authority plane or imply qualification of one role qualifies the others.

## 9. What to reuse rather than reinvent

- **Home Assistant:** preserve mature local device integrations. Its docs cover local data, local protocols, and optional cloud functions; evaluate exact integrations rather than assume every device is offline-capable.
- **NIST IR 8259 Rev. 1 (final, April 20, 2026):** use the updated IoT manufacturer guidance as a lifecycle baseline for customer needs, risk assessment, securability, maintenance, and communications. Tailor it to each product's threat and impact profile.
- **NixOS test framework and QEMU:** use declarative multi-VM tests and fault injection; simulated results must remain distinct from physical evidence.
- **SLSA build provenance / in-toto attestations:** evaluate standard provenance formats rather than inventing a non-interoperable receipt protocol. Provenance is one input to qualification, not proof of semantic correctness.
- **CISA Secure by Design:** make product security and maintenance the manufacturer's responsibility; publish progress and limitations rather than transferring the burden to household owners.
- **OSHWA / CERN-OHL:** use modifiable design documentation and precise, component-scoped licensing for genuine open-hardware claims.
- **Steam/Proton and BOINC:** evaluate mature compatibility and volunteer-compute machinery before building replacements; add adapters only where requirements cannot be met safely.

Primary references:
- NIST IR 8259 Rev. 1: https://csrc.nist.gov/pubs/ir/8259/r1/final
- NIST 2026 announcement: https://www.nist.gov/news-events/news/2026/04/sharpening-focus-product-requirements-and-cybersecurity-risks-updated
- Home Assistant privacy: https://www.home-assistant.io/faq/is-my-data-private/
- Home Assistant security: https://www.home-assistant.io/docs/configuration/securing/
- NixOS test framework: https://nixos.org/manual/nixos/stable/#sec-call-nixos-test-outside-nixos
- QEMU security architecture: https://www.qemu.org/docs/master/system/security.html
- SLSA build provenance: https://slsa.dev/spec/v1.0/provenance
- CISA Secure by Design: https://www.cisa.gov/securebydesign
- OSHWA definition: https://oshwa.org/definition/
- ROS 2 security enclaves: https://design.ros2.org/articles/ros2_security_enclaves.html

## 10. Prioritized implementation path

### P0 — make current claims defensible
1. Obtain passing Rust CI on the exact current head.
2. Obtain a passing pinned QEMU sentinel run and inspect its logs/manifest.
3. Fix actual failures before adding scope.
4. Preserve negative controls and claim ceilings.

### P1 — capability contracts become an admission seam
1. Design a versioned machine-readable capability manifest based on Section 2.
2. Add positive/adversarial fixtures for undeclared egress, broader-than-owner permissions, missing failure semantics, sensitive outputs marked public, and stale evidence.
3. Validate manifests before enabling services; test enforcement, not only schema conformance.
4. Treat the new manifest as a reviewed API/contract, not dashboard metadata.

### P2 — replace sentinels with a useful Hearth instance
1. Create the minimal real NixOS module, with a local essential service and genuinely optional inference adapter.
2. Add Home Continuity Drill scenarios for offline operation, service failure, resource pressure, and recovery.
3. Implement clean-VM restore, update interruption/rollback, and owner export/exit tests.
4. Publish per-test VM evidence with honest limitations.

### P3 — compare before custom hardware
1. Shortlist two or three commodity candidates only after acceptance thresholds are set.
2. Measure idle/workload power, noise, sustained thermal margin, repair time, parts support, and lifecycle cost.
3. Keep a standard x86-64/ARM64 machine as the reference baseline.
4. Freeze no BOM until measured results justify it.

### P4 — expand as separate qualification programmes
Add Arc Console compatibility measurements, then a two-node Commons Compute hostile-worker exercise. Keep physical robotics until simulation, software contracts, and lab hazards are reviewed. Do not transfer qualification between products.

## 11. Definition of a meaningful revolution

A credible first milestone is not “a smarter AI controls the whole house.” It is:

> A household can run essential local services without a vendor cloud; inspect every enabled workload's permissions; let optional cognition fail without losing core function; restore to a clean replacement; revoke access and export its data; and prove these claims with exact, inspectable evidence.

If we can repeat that test, publish the method, invite external reproduction, and outperform a commodity baseline on measured lifecycle criteria without sacrificing owner control, we will have a meaningful contribution to the industry. Until then, this remains a research-led proposal rather than proof of superiority.
