# Home and SMB Operations Product-Line Decision

**Status:** Proposed product/architecture decision for review. This document is planning, not proof of a shipped product, tested adapter, customer commitment, safety qualification, or commercial result.

**Reviewed:** 2026-10-15  
**Scope:** Public Luminous Platform architecture. It does not move funding material into a public repository.

## Decision

Build **one small, auditable operations kernel with separately governed vertical profiles**. Start commercial validation with small-business IT operations, while preserving a clear path to the Hearth Node / smart-home family. Do not build two parallel platforms, and do not market a universal agent that can safely administer every system.

The first SMB product should be a **Verified Operations Companion**: observe infrastructure, organize evidence, highlight prioritized risks, connect findings to existing work cases, and prove whether an explicitly approved remediation actually worked. Start read-only. Integrate with incumbents before replacing them.

The first home product should be a **Home Resilience Companion** built around Home Assistant and interoperable device protocols. Begin with local availability, device health, backups/update visibility, and measured electricity/water observability. Treat energy optimization, irrigation, locks, high-power loads, and robotics as separate maturity tracks with explicit limits.

These are distinct products over shared infrastructure, not a claim that every adapter or capability already exists.

## Why this is the best next move

Luminous Platform already has public work on a shared sovereign-operations architecture and a native work-case lifecycle ([PR #4](https://github.com/Luminous-Dynamics/luminous-platform/pull/4), [PR #8](https://github.com/Luminous-Dynamics/luminous-platform/pull/8)). The public repository also tracks a vendor-neutral hardware and firmware adapter contract ([issue #23](https://github.com/Luminous-Dynamics/luminous-platform/issues/23)), a read-only Synology monitoring candidate ([issue #37](https://github.com/Luminous-Dynamics/luminous-platform/issues/37)), and a proposed open home-systems architecture ([PR #50](https://github.com/Luminous-Dynamics/luminous-platform/pull/50)). This document connects those lines of work; it does not replace their individual contracts.

The market already has substantial RMM/PSA and home-automation capability. Tactical RMM advertises monitoring, asset inventory, patching, scripting, and remote access ([features](https://tacticalrmm.com/features/)). ConnectWise announced an AI-oriented platform unifying PSA, RMM, security, automation, and orchestration on 8 June 2026 ([announcement](https://www.connectwise.com/company/press/releases/connectwise-unveils-the-industrys-first-predictive-intelligence-platform-to-lead-the-era-of-predictive-it)). Therefore “we use AI” or “we have a dashboard” is not sufficient differentiation.

The proposed differentiation is narrower: **source-bound evidence, customer-controlled deployment, explicit authority, safe handling of uncertain effects, and independent verification across integrations**. These must be demonstrated through tests and pilots; they are not competitive claims we can make before measurement.

For the SMB problem baseline, use NIST's Cybersecurity Framework 2.0 Small Business Quick-Start Guide as an optional control-mapping reference—not as a claim of certification or a substitute for the customer's applicable law and obligations ([NIST SP 1300](https://csrc.nist.gov/pubs/sp/1300/final)). Asset awareness, MFA, patching, backup protection, and tested recovery are useful outcomes to map to; a green dashboard alone does not prove that they are implemented.

## Product boundaries

| Capability | Verified Operations Companion (SMB / MSP) | Home Resilience Companion (household) |
|---|---|---|
| First user | Small internal IT team or a service provider piloting with one customer | Owner/operator of a Home Assistant-based home |
| Primary outcome | Fewer blind spots and more trustworthy incident/recovery evidence | Greater local control, visibility, and resilience during cloud or WAN outages |
| Initial observations | Assets, device/service health, patch/security posture, backup status, connector health | Home Assistant health, devices/sensors, service/backup/update state, energy and water measurements |
| Workflow | Finding → evidence → work case → approved plan → effect journal → independent verification | Observation → explanation → optional bounded proposal → owner authorization where required → verified outcome |
| Existing systems to reuse | Current PSA/ticketing, RMM/security/backup tools, supported vendor APIs and protocols | Home Assistant, Matter where supported by the exact device/integration, Zigbee, Z-Wave, ESPHome and vendor-native local integrations where appropriate |
| AI role | Correlate observations, rank hypotheses, explain confidence and recommend next checks | Explain patterns, forecast within measured-data limits, propose schedules and help diagnose anomalies |
| Authority boundary | Backend policy and authenticated operator approval; no shell or device mutation in the first MVP | Home Assistant/device capabilities remain bounded; cognitive recommendations never become a raw device-command channel |
| Data default | Tenant-scoped, customer-controlled storage and secret management | Local storage and processing where practical, clear household consent, export and deletion |
| Commercial hypothesis | Recurring service for operational assurance and evidence-backed support | One-time setup/support plus an optional managed service; validate willingness to pay before committing to a model |

An MSP-facing deployment may eventually manage multiple customers, but **the first pilot should be one tenant and one site** until tenant isolation, authorization, export/deletion, and cross-tenant negative tests have passed. Multi-tenancy is a security property, not simply a navigation option in the UI.

## Shared kernel: what deserves to be built once

Keep the domain model separate from any one UI or provider. The stable core should cover:

1. **Identity and scope:** tenant or household, site, asset/resource, operator, provider connection, and narrowly scoped permissions.
2. **Observations:** typed value and units, source identity, source event time, local receipt time, collection method, profile/version, freshness, confidence/quality, and explicit unknown/error states.
3. **Resource and dependency graph:** a service can depend on a host, network path, storage system, identity provider, power supply, or device. Edges need provenance and must distinguish confirmed dependencies from inferred ones.
4. **Findings and work cases:** deduplicate related symptoms without deleting original evidence; preserve severity rationale, state transitions, ownership, external ticket mappings, and audit history.
5. **Plans and authority:** represent a proposed change separately from an authorized command. Bind any later approval to the exact target, pre-state, intended effect, policy revision, actor, expiry and action digest.
6. **Effect journal and recovery:** distinguish prepared, dispatched, applied, verified, failed, partial, and indeterminate outcomes. A timeout after dispatch is not evidence that nothing happened and must not blindly replay an unsafe operation.
7. **Independent verification:** the executor does not certify itself. A separate verifier checks the target and postconditions, reports partial evidence honestly, and retains negative results.
8. **Portable evidence:** store customer-sensitive payloads in an appropriate protected domain. Use redacted references, digests and minimal receipts where sharing is necessary; a digest does not prove that the source observation was true.
9. **Stable external contracts:** retain the repository's established versioned HTTP/event/CloudEvents and work-case direction. Do not make Leptos UI state, Symthaea internal representations, or a Holochain availability assumption the product API.

Use the existing work-case, event, adapter, recovery and receipt contracts as dependencies rather than forking near-duplicate schemas: [PR #4](https://github.com/Luminous-Dynamics/luminous-platform/pull/4), [PR #8](https://github.com/Luminous-Dynamics/luminous-platform/pull/8), [issue #23](https://github.com/Luminous-Dynamics/luminous-platform/issues/23), [issue #42](https://github.com/Luminous-Dynamics/luminous-platform/issues/42), [issue #43](https://github.com/Luminous-Dynamics/luminous-platform/issues/43), and [issue #44](https://github.com/Luminous-Dynamics/luminous-platform/issues/44). Their exact implementation and qualification state must still be checked at the relevant head; links to open work are not proof of completion.

## Responsibility boundaries between Luminous projects

- **Luminous Platform** owns product APIs, adapters, normalized operational state, work-case orchestration, policy enforcement, effect journaling, verification and operator/customer views.
- **Symthaea** provides bounded interpretation, forecasting, anomaly hypotheses, trade-off analysis and candidate plans. It must expose supporting evidence and uncertainty. It is not an identity provider, policy bypass, privileged command channel, or safety interlock.
- **Mycelix** can support explicit delegation, consent, peer/witness provenance and selected verifiable receipts when multi-party trust is genuinely useful. It must not be required for local safety, device control, or real-time authorization; raw household telemetry, secrets, and unredacted customer topology do not belong in a public DHT.
- **Nixward** manages the NixOS hosts and declarative configuration that it explicitly supports. It is not a universal agent for Windows, macOS, network appliances, DSM, or smart-home radios.
- **Home Assistant** remains the device integration and home-automation system of record for the home product. Luminous should add operations, reliability, evidence and carefully bounded optimization—not duplicate its integration catalogue.
- **External PSA/RMM/backup systems** remain authoritative for their existing records until an integration contract and migration plan deliberately says otherwise. No “replacement” claim is needed to sell an evidence-focused first service.

## First commercial slice: verified operations and recovery

Do not attempt full feature parity with enterprise RMM/PSA products. The pilot should answer a small set of questions reliably:

- What assets and critical dependencies have actually been observed, from which sources, and how stale is the view?
- Which critical devices or services are unhealthy or unsupported, and what evidence supports the finding?
- What do the available records say about backup freshness and job outcome?
- Is there a recent, attributable test that restored data or a service to a known target? If not, say **recovery not recently demonstrated**—never infer recoverability from a successful backup job.
- Which work item tracks the finding, who is allowed to approve a proposed action, and what postcondition would establish success?
- Which data is missing, contradicted, stale, or unavailable? Missing telemetry must not silently become healthy.

Phase 1 is read-only and should be able to generate work cases without live execution permissions. Synology SNMPv3 is a candidate for bounded NAS health observations under [issue #37](https://github.com/Luminous-Dynamics/luminous-platform/issues/37); it cannot by itself prove backup completion or successful restore. Choose a backup adapter only after confirming the pilot customer's real product, API/protocol availability, version, and ability to expose suitable evidence. Keep the customer’s current PSA as the system of record; only enable live ticket events after durable inbox/outbox and tenant checks are qualified.

Use a customer lab or synthetic fixture to exercise malformed data, timeouts, stale observations, duplicate/reordered events, revoked credentials, provider version changes, contradictory claims, permission denial, partial outcomes and unavailable verifiers. Do not start with live remote shell, unattended patch deployment, firewall edits, firmware flashing, or automated credential changes.

## Home slice: local resilience before autonomy

Home Assistant already provides an energy dashboard for electricity, gas, water, production, storage, and individual-device telemetry ([official guide](https://www.home-assistant.io/docs/energy/)). The opportunity is not to rebuild those charts. Add a coherent operations view: whether measurements are fresh and correctly typed, whether critical integrations are reachable, whether local services/backups are healthy, what changed, and which next action is safe.

Matter is valuable but must be treated as a versioned capability, not a promise that all feature clusters exist in every deployed controller and device. Matter 1.5, announced on 20 November 2025, added or expanded areas including soil sensing and energy-management data for tariffs, carbon intensity, metering, grid limits and EV charging ([Connectivity Standards Alliance](https://csa-iot.org/newsroom/matter-1-5-introduces-cameras-closures-and-enhanced-energy-management-capabilities/)). Home Assistant documents local Matter control, but the actual supported behavior still depends on the deployed server, integration and device ([Matter integration](https://www.home-assistant.io/integrations/matter/)).

The first home slice should therefore:

- retain local operation and manual control during WAN loss wherever the device/protocol permits;
- measure and label **power (W/kW)** separately from **energy (Wh/kWh)**, and measured values separately from estimates;
- handle missing, implausible, stale and contradictory measurements explicitly;
- provide energy/water visibility before claiming optimization savings;
- forecast from measured history and declared tariff/weather inputs, stating uncertainty and data gaps;
- keep locks, garage doors, heaters, high-power electrical equipment, batteries, water valves and irrigation under exact device-specific safety profiles;
- start high-consequence actions in recommendation-only mode; any later automation needs constrained setpoints, rate limits, expiry, manual override, safe defaults and failure/recovery tests.

Food-growing support can be an additional data pack—soil moisture/temperature, irrigation, light, growing environment and harvest observations—without turning a home controller into an unvalidated agronomy authority. Nutrient or crop recommendations must state the crop/context, evidence quality and uncertainty; sensor readings alone do not establish soil nutrient composition or food nutritional content. First integrate observation and user feedback, then test bounded irrigation schedules against water use and plant-health measurements before automating valves.

## Safety, privacy and failure semantics

The common rule is **observe → interpret → propose → authorize → execute → independently verify**. The runtime must preserve a hard distinction between every step.

- Default to read-only integrations and least-privilege credentials.
- Store secrets in the approved secret provider; never place them in prompts, command-line arguments, receipts, telemetry, fixtures or logs.
- Bind any authorization to an exact target and action, fresh pre-state, policy revision and expiry; stale state invalidates it.
- Keep retries idempotent where possible. For uncertain non-idempotent operations, reconcile state before retrying.
- Do not infer safety, health, backup recoverability, device identity or permission from missing fields or a signed but unverified claim.
- Preserve local manual operation if Symthaea, Mycelix, an optional cloud service, or the control-plane UI becomes unavailable.
- Minimize collection and retention. Home video, occupancy, access credentials, personal routines and business topology are especially sensitive and must not be ingested by default merely because an integration exposes them.
- Provide an export and a deletion/offboarding path. Revoking an integration credential must prevent new collection and authorized actions.
- Separate jurisdiction-specific requirements from the global product contract. NIST CSF is a control-mapping baseline, not a universal legal profile.
- Publish deployment maturity honestly: research-only, synthetic-contract tested, read-only lab tested, independently verified, controlled-action tested, and field qualified are different stages.

## Roadmap and exit gates

### Gate 0 — contract and customer discovery

- Confirm one SMB pilot profile: business type, size, critical services, existing PSA/RMM/backup, accepted deployment model, data residency, operator roles and recovery expectations.
- Select one home reference profile separately; do not let it silently expand the SMB MVP.
- Map every proposed feature to an existing repository contract, provider capability or explicit gap.
- Record competitor alternatives, installation burden, operating cost and why an integration/reuse is preferable to a rewrite.

**Exit:** named pilot criteria and a bounded profile; no production-readiness claim.

### Gate 1 — synthetic, read-only evidence loop

- Ingest deterministic fixtures from a small number of deliberately chosen adapters.
- Preserve source identity, timestamp, freshness, profile revision and failure reason.
- Normalize findings into an existing work-case contract, including deduplication and conflict/dead-letter paths.
- Demonstrate tenant/site scoping and negative tests for scope confusion.
- Show that missing data yields unknown/indeterminate rather than healthy.

**Exit:** reproducible conformance evidence tied to an exact commit; a queued or absent CI run does not count.

### Gate 2 — one controlled lab deployment

- Use lab or customer-approved read-only credentials only.
- Verify bounded collection, revocation, restart, migration, data export/deletion and log redaction.
- Test backup evidence against a separately recorded restore exercise. Do not claim restoration was tested unless an actual restore target and outcome are recorded.
- Run an independent verifier against captured receipts and negative controls.
- Establish baseline measurements: discovery coverage, stale-data rates, false-positive rate, analyst time per finding, onboarding time, and unresolved/indeterminate counts.

**Exit:** repeatable lab result plus reviewer-visible limitations; no automatic production changes.

### Gate 3 — limited customer pilot

- Start with one tenant/site and named operator approvals.
- Obtain explicit authorization for any action beyond read-only observation; start with low-risk, reversible actions only after Gate 2 passes.
- Track outcomes, failed/partial actions, rollback/recovery, operational hours, customer-reported value, support load and willingness to pay.
- Keep current tools in place until equivalent behavior and migration/exit conditions are proven.

**Exit:** measured user value and security review, not feature count.

### Gate 4 — home energy/water and bounded automation

- Qualify each exact controller/device/protocol/version combination separately.
- Validate unit normalization, measurement freshness, time-series gaps, tariff/forecast source provenance, and physical constraints before optimization.
- Compare optimization against a baseline over comparable periods; distinguish causal savings from weather, occupancy, tariff, and measurement changes.
- Test WAN loss, power loss, controller restart, manual override, sensor failure and actuator feedback mismatch.
- Enable one low-risk action class at a time; retain an accessible stop/disable path.

**Exit:** measured repeatability on the target profile and explicit owner acceptance. Energy savings, resilience, and safety are claims to measure, not infer from a simulation.

## Pilot scorecard

Record a baseline before setting numerical service-level targets.

| Measure | Definition | Guard against |
|---|---|---|
| Asset/source coverage | Observed in-scope assets divided by an independently confirmed pilot inventory | Do not use discovered assets as the denominator |
| Data freshness | Fraction of observations within each source-specific freshness bound | Never convert missing observations into success |
| Finding precision | Confirmed actionable findings divided by reviewed findings | Keep dismissed/false-positive examples |
| Evidence completeness | Findings with source, timestamp, profile, reason and validation outcome | A signed receipt alone does not prove truth |
| Recovery assurance | Critical systems with a recent, recorded restore test and known result | A completed backup job is not a restore test |
| Operator effort | Human minutes per reviewed/remediated finding and per customer per week | Include setup and exception handling |
| Action safety | Authorized actions independently verified; partial/indeterminate/rollback counts | Do not report dispatched commands as successful fixes |
| Household resource outcome | Measured consumption/cost/water use across comparable periods | Report weather, tariffs, occupancy and measurement gaps |

## Out of scope for the initial releases

- Universal unattended administration, arbitrary model-generated shell commands, or unsupervised high-impact actions.
- Replacing Home Assistant, every PSA/RMM, all backup products, or every vendor's native management plane.
- Treating Matter 1.5 release notes as proof that a deployed controller supports every new capability.
- Treating Holochain/DHT availability as a hard real-time safety dependency or storing sensitive raw telemetry in a public DHT.
- Claiming compliance, certification, production qualification, customer savings, reduced incidents, or universal hardware support without the corresponding evidence.
- Building proprietary device integrations where a supported upstream interface can meet the requirement.

## Source notes

1. [Home Assistant — Energy management](https://www.home-assistant.io/docs/energy/), reviewed 2026-10-15.
2. [Home Assistant — Matter integration](https://www.home-assistant.io/integrations/matter/), reviewed 2026-10-15.
3. [Connectivity Standards Alliance — Matter 1.5 announcement, 2025-11-20](https://csa-iot.org/newsroom/matter-1-5-introduces-cameras-closures-and-enhanced-energy-management-capabilities/).
4. [NIST SP 1300 — Cybersecurity Framework 2.0 Small Business Quick-Start Guide](https://csrc.nist.gov/pubs/sp/1300/final), 2024-02-26.
5. [Tactical RMM — feature list](https://tacticalrmm.com/features/), reviewed 2026-10-15.
6. [ConnectWise — Predictive Intelligence Platform announcement, 2026-06-08](https://www.connectwise.com/company/press/releases/connectwise-unveils-the-industrys-first-predictive-intelligence-platform-to-lead-the-era-of-predictive-it).

Competitive update (15 October 2026): SolarWinds announced Avior on 14 October 2026, with goal-oriented agents, deterministic execution and risk-gated actions. This means “AI that pursues goals safely” is not, by itself, a defensible differentiator. The proposed distinction is an independent cross-vendor assurance layer with explicit outcome contracts, visible evidence coverage, met/missed/indeterminate outcomes, and measured net operator toil. See [SolarWinds Avior announcement](https://www.solarwinds.com/company/newsroom/press-releases/solarwinds-unveils-avior-bringing-deterministic-agentic-ai-to-enterprise-it) and [the outcome-assurance paradigm note](https://github.com/Luminous-Dynamics/luminous-platform/blob/docs/home-smb-operations-portfolio-2026-10-15/docs/OUTCOME_ASSURANCE_PARADIGM.md).

External references provide context, not verification that Luminous has implemented or qualified any of the described capabilities.
