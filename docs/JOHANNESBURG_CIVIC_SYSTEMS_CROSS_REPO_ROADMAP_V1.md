# Johannesburg Civic Systems — Cross-Repository Delivery Roadmap v1

**Status:** Coordination proposal; architecture and research only, not municipal endorsement or deployment authorisation  
**Snapshot:** 14 October 2026  
**Initial geography:** Johannesburg / Gauteng, South Africa  
**Scope:** Public Luminous Dynamics repositories only. Private funding materials and the private monorepo are out of scope.

## Executive decision

Treat the civic work as one coordinated programme implemented by specialised repositories, not as a giant Sol-Atlas feature or a new monolith. The first useful outcome should be a read-only **Joburg Service Reliability Evidence Pack** and a synthetic, replayable **water fault-to-repair workflow**. This gives us an auditable way to improve everyday service reliability while avoiding claims of access to municipal systems.

Run two early tracks in parallel:

1. **Public service observability:** connect published budgets, service standards, service notices and open procurement data to reproducible observations with source coverage and limitations visible.
2. **Water reliability / paid-work pilot design:** model the report → acknowledge → inspect → repair claim → independent verification lifecycle with synthetic cases, documented service-standard semantics and a workforce pathway. Keep any real utility data in a later, authorised shadow phase.

Then add geographic context, environmental evidence and simulation as the contracts and evaluation gates mature. Do not build separate apps for water, waste, roads, food, transport and housing before the shared interfaces have proved useful.

The decision this roadmap seeks is agreement on repository ownership, contract boundaries and the phase gates below. It does **not** assert that any current PR is merged, qualified in full, or integrated with another repository.

## 1. Programme objective and first proof

The product should help answer five questions, with traceable evidence:

- What need, interruption, hazard, commitment or service claim has been recorded?
- Which published source or authorised actor supports each fact, and how current and complete is that source?
- What is the next valid step, who has authority to take it, and what remains unknown?
- Did an intervention happen, and is there independent evidence that the service outcome improved?
- Did the outcome improve fairly across different neighbourhoods and access conditions, including residents who do not use a smartphone?

### Initial proof package

Publish a small, reproducible pack containing:

- a source catalogue with owner, terms/licence, access method, update cadence, coverage, time period and limitations;
- versioned observations preserving source values, units, timestamps, source snapshots and normalisation identity;
- synthetic service cases that exercise duplicates, conflicting notices, missing/stale data, ambiguous asset matches, delayed repair parts, reopened faults, outages and corrections;
- a deterministic baseline for priority and work-queue choices;
- a Symtropy comparison of candidate policies under explicit, bounded assumptions;
- an evidence report separating observed results, source claims, model hypotheses and unknowns;
- a public explanation of what the system cannot infer or authorise.

The demonstrator remains useful if every AI component is switched off. No resident-identifying data is needed for this proof.

## 2. Repository ownership: one system, separate responsibilities

| Repository / active track | Owns | Must not become |
|---|---|---|
| [Luminous Platform — public-service profile PR #34](https://github.com/Luminous-Dynamics/luminous-platform/pull/34) | System boundary, source catalogue, operating profile, integration policy and rollout gates | A competing municipal master database or replacement for a city's official channels |
| [Luminous Platform — workload contracts PR #28](https://github.com/Luminous-Dynamics/luminous-platform/pull/28) | Reproducible deployment/workload contract and validator, once qualified | A claim that the civic workload is production-ready merely because a schema exists |
| [Mycelix — Johannesburg water/workforce PR #5016](https://github.com/Luminous-Dynamics/mycelix/pull/5016) | Pilot protocol, human/community coordination, job-task pathway, service journey and outcome governance | A direct utility connector or an autonomous dispatcher |
| [Mycelix — standards/evidence PR #5019](https://github.com/Luminous-Dynamics/mycelix/pull/5019) | Versioned instrument/standard records, jurisdictional applicability, service journey reference and cross-domain semantics | A generic trust score that decides truth, legal applicability or eligibility |
| [Symthaea — civic mastery PR #7443](https://github.com/Luminous-Dynamics/symthaea/pull/7443) | Advisory reasoning, source conflict handling, uncertainty/abstention and frozen reference scenarios | The authoritative case ledger, a legal decision-maker or a privileged action executor |
| [Symthaea — Johannesburg water research PR #7414](https://github.com/Luminous-Dynamics/symthaea/pull/7414) | Source inventory and reproducible research path for leak-detection transfer; controlled sensor evaluation before field claims | A claim that satellite or acoustic data has detected a live municipal leak without independently labelled ground truth |
| [Symthaea — privacy-preserving telemetry PR #7425](https://github.com/Luminous-Dynamics/symthaea/pull/7425) | Privacy and delay-tolerant observation transport research for weak-connectivity contexts | A route for silently uploading sensitive or identifiable observations |
| [Sol-Atlas — utility telemetry PR #104](https://github.com/Luminous-Dynamics/sol-atlas-leptos/pull/104) | Spatial/environmental observations, satellite product/quality metadata, read-only utility-observation contract | SCADA, asset-owner authority or proof of a service fault from a remote-sensing index alone |
| [Sol-Atlas — civic portfolio/case ledger PR #109](https://github.com/Luminous-Dynamics/sol-atlas-leptos/pull/109) | Public-facing geographic context, intervention portfolio and sanitised projection contract with authored tests | The canonical operational Work Case or a source of private municipal work-order state; current-head CI still governs qualification |
| [Sol-Atlas — Highveld food resilience PR #108](https://github.com/Luminous-Dynamics/sol-atlas-leptos/pull/108) | Climate/food resilience profile and a future second domain pilot | An agricultural recommendation service until local assumptions and outcomes are validated |
| [Symtropy — transport optimisation PR #1602](https://github.com/Luminous-Dynamics/symtropy/pull/1602) | Deterministic comparison of routes, costs, service/workload policies and trade-offs | A live crew dispatch system or a simulator whose assumptions are presented as measured City outcomes |
| [Nixward — full-stack qualification PR #58](https://github.com/Luminous-Dynamics/nixward/pull/58) | Reproducible NixOS deployment, host/service hardening, rollback, offline operation and qualification | A reason to deploy before the application and workload contracts qualify |
| [Xenia peer — verified restore PR #451](https://github.com/Luminous-Dynamics/xenia-peer/pull/451) and [Xenia wire](https://github.com/Luminous-Dynamics/xenia-wire) | Authenticated identity/session/consent and tamper-aware evidence/ledger foundations, only after their own boundaries qualify | Civic workflow policy, municipal authorization or an unqualified single source of truth |

Mycelix Prism can remain an optional, separately qualified presentation/privacy component; it is not on the first-pilot critical path.


## 2A. First executable cross-repository crosswalk (added 14 October 2026)

The public Platform PR branch now includes a separate contract in the [civic-record-crosswalk-v1 directory](https://github.com/Luminous-Dynamics/luminous-platform/tree/docs/public-service-operations-profile-rfc-001/contracts/civic-record-crosswalk-v1/). It pins the exact audited Mycelix service-journey and applicability schemas, Sol-Atlas Rust civic-case types, the Platform public-observation schema and Symthaea's life/civic evaluation corpus.

The crosswalk is intentionally metadata-only. It forbids automatic state-machine conversion, keeps evidence independence separate from review disposition, rejects reuse of source-snapshot digests as target evidence-byte digests, and prevents applicability assessments or evaluation expectations from becoming action authority. A deeper public-repository audit found the actual Work Case V1 schema in draft [Platform PR #8](https://github.com/Luminous-Dynamics/luminous-platform/pull/8) at exact head 384a60d9668e77c40561b46d5511593ea5198c23 and Operations Event V1 schema in draft [Platform PR #4](https://github.com/Luminous-Dynamics/luminous-platform/pull/4) at exact head b4b32d90742e9a218e4d6919f15d74d904f5beca. They were absent from the audited PR #34 branch tree itself, so the original snapshot finding remains accurate; the crosswalk now pins the separately located schemas and labels both `unqualified_draft`. PR #4's exact-head qualification workflow failed (SQLx 0.9 dynamic-query compilation and a malformed research-only test definition), and no workflow run was returned for PR #8's pinned head. Mappings into these contracts therefore remain blocked until qualification and stacked branch ancestry are reconciled.

The Rust 1.96 checker rejects unknown fields, requires qualification state for every pinned contract and tests conservative invariants. The workflow formats, tests and validates both public-source and civic-crosswalk contracts at the exact event revision. Passing that workflow qualifies the validators and fixtures only, not the separate Work Case/Operations Event PRs, runtime interoperation or municipal deployment.

## 3. The most important architectural decision: do not merge three different lifecycles

Several current tracks contain useful case-like or journey-like records. We should reconcile their purposes before extending any one of them into a production schema.

1. **Operational Work Case (Luminous Platform):** who owns a task, what state the work is in, what command is allowed, and what evidence is required for the next operational transition.
2. **Service Journey / standards applicability (Mycelix):** the person's or community's path through options, requirements, referrals, acknowledgement, remedy and review. It explains applicable rules; it must not itself grant authority.
3. **Public civic observation or mapped issue (Sol-Atlas):** a privacy-minimised representation of a reported/published condition and its spatial context. It may link to an official reference but must not expose internal identifiers or imply access to the official work-order state.

**Crosswalk v1 now exists** at [contracts/civic-record-crosswalk-v1](https://github.com/Luminous-Dynamics/luminous-platform/tree/docs/public-service-operations-profile-rfc-001/contracts/civic-record-crosswalk-v1/). It pins seven source artifacts and records 20 explicit semantic relationships, including prohibited lossy conversions and blocked adapters. Next, review the field paths against each pinned source revision, resolve the Platform PR #8/#4 draft ancestry/qualification issues, and rerun the exact-head crosswalk workflow. Do not add a fourth “universal case schema.” Prefer explicit adapters and stable, purpose-specific identifiers. Preserve distinct state machines.

Before any service adapter is write-capable, the shared Work Case and event contracts must pass their own exact-head qualification. An observation, Symthaea recommendation, public report, signed receipt or simulated improvement is never authorisation to mutate a utility system.

## 4. Shared evidence envelope and semantic rules

Reuse compatible existing schemas where possible. If a missing common envelope is confirmed by the crosswalk, propose it as a separately versioned contract before implementing multiple producer-specific variants.

At minimum, every normalised observation should preserve:

- schema name/version and stable observation identifier;
- source identifier, source URL/reference, owner, licence/terms and snapshot digest when lawful and available;
- source time, capture time, clock domain, retrieval method and retrieval status separately;
- jurisdiction, location/asset reference and the granularity permitted for the evidence's visibility class;
- service domain, observed variable, value, unit and method/version of normalisation;
- quality, completeness, freshness, uncertainty, known bias and known coverage gap;
- access/visibility classification, privacy rationale and applicable retention rule;
- relation to earlier/corrected observations without erasing history.

The model must keep these states distinct: **zero, null, missing row, not published, stale, invalid, conflicting and not checked**. An unavailable source is not a negative finding. A source's stated target is not the measured outcome. A model score is not ground truth. A repair claim is not independently verified restoration.

### Spatial/privacy rules

- Prefer public region/ward-level analysis; do not publish household coordinates, private contact details, account numbers, unique vulnerabilities or asset locations that could expose critical infrastructure.
- Verify the meaning and allowed granularity of every geographic code with a responsible data steward; a code format alone does not prove its geography.
- Preserve an internal record only under documented authority and purpose. Public projection must be an explicit allowlisted view, not serialisation of the internal record with a few fields hidden in the UI.
- Aggregate small cells or suppress them when publishing could re-identify people, expose a vulnerable household or reveal sensitive infrastructure. Preserve a documented suppression reason.
- Do not use report volume or smartphone access as a proxy for need; model under-reporting and missingness as first-class risks.

## 5. Priority use cases and why

### P0 — Public service reliability and evidence ledger

Begin with published service notices and aggregate indicators, plus synthetic cases. Display the source, date, period, update state, unit and coverage directly. Preserve official case/ticket references only when supplied lawfully and keep internal identifiers private. The ledger should distinguish reported, acknowledged, assigned, attended, resolution claimed, independently verified, dismissed and reopened states.

Starter measures: source freshness/completeness, unresolved conflicts, acknowledgement delay where evidenced, duplicate-link precision, evidence completeness, repeat incidents and independently verified outcomes. Do not invent case-level SLA timestamps from annual indicators.

### P0 — Water reliability and paid maintenance pathways

This is the best operational pilot candidate because the current work already covers service-standard semantics, a water/workforce pilot, leak research, telemetry contracts and privacy-preserving delayed observations.

The pilot is not “AI replaces water crews.” It is a bounded workflow to reduce lost context between report, inspection, repair and verification; test whether maintenance and logistics tasks can become paid, supervised work; and measure incremental value against current practice.

The research basis is now stronger than an isolated water-sector argument: the South African Government published a joint statement on 10 September 2026 describing an intergovernmental meeting convened to address water-supply disruptions associated with intermittent power trips, with Johannesburg Water, City Power, Rand Water, Eskom and national/provincial authorities represented. This justifies a **cross-utility dependency hypothesis to test**, not a presumption that any specific water fault was caused by electricity. Source: https://www.gov.za/news/media-statements/government-measures-mitigate-water-supply-disruptions-resulting-power-trips. The expanded public source catalogue now includes separate metadata-only references for City Power notices and that dated government statement; neither is a live outage feed.

Begin with synthetic records and public notices. A later shadow phase may use an approved historical export or read-only feed with a named data owner. No real dispatch, crew assignment or live sensor access until permission, safety, security, support and rollback requirements are documented. Verify current smart-controller/noise-logger installation status with Johannesburg Water; the July 2026 announcement's target dates are not proof of completion.

Candidate outcomes: time from report to acknowledgement and inspection; repeat-visit rate; share of claims with independent repair evidence; water-loss measure only where source definition and denominator are confirmed; false positive workload; cost per verified resolution; paid task completion and worker safety; and resident correction/reopen rate.

### P1 — Waste, roads and stormwater

Reuse the same evidence and case mapping rather than creating separate products. Represent waste hot spots, missed collection, potholes, traffic signals, pavement hazards and blocked drains as different domain profiles. Model public-land ownership and responsible operator explicitly; a visual hazard at a coordinate does not automatically identify the responsible agency.

Start after the common event/case crosswalk and source-quality tests are in place. Later use Symtropy to compare inspection and repair queues; do not optimise exclusively for the most-reported or most-connected places.

### P1 — Food resilience, heat and water availability

Sol-Atlas's Highveld agriculture profile can become a second, distinct validation exercise: combine licensed climate/land-use/water context with clearly labelled assumptions, local agronomic input and measured growing outcomes. Do not turn remotely sensed greenness into a direct claim of yield, irrigation need, land tenure or food availability. Keep seasonal scenarios and locally sourced evidence separate.

### P2 — Budget, procurement, transport, health, housing and local economic pathways

Build the catalogue and evidence semantics once, then add domains according to source readiness, a named human owner and the ability to measure an outcome. Public procurement anomalies remain human-review leads, never public accusations. Clinic/health workflows require a separate health-specific privacy and safety review; no medical records enter the first demonstrator. Housing/benefit eligibility and other decisions with substantial effects on individuals must not be automated by Symthaea.

## 6. Standards and authoritative reference map

Treat each standard or rule as a versioned source with issuer, jurisdiction, subject, mandatory/contractual/voluntary status, effective dates, source digest and a human-reviewed applicability assessment.

| Source / body | Use in this programme | Boundary |
|---|---|---|
| City of Johannesburg IDP, SDBIP, final business plans and Service-Level Standards Charter | The City's own stated outcomes, measures and local targets | The published index is a discovery source; validate document version, measure definition, target period and accountable department before comparison |
| Johannesburg Water service standards, notices and active leak-detection programme | Published service semantics, fault-report channels and research baseline | Utility publications are source claims; they do not prove that a particular sensor is installed or a particular repair was completed |
| National Treasury Municipal Money / Municipal Finance API; municipal Standard Chart of Accounts (mSCOA) | Budget, capital acquisition, repairs and maintenance and financial time series | Treasury notes completeness depends on municipal submissions. Its API documentation describes an HTTP endpoint despite the HTTPS base listed elsewhere; confirm safe HTTPS access before writing a network adapter. Preserve the submitted period and data status |
| National Treasury eTender Transparency Portal / Open Contracting Data Standard (OCDS) | Published procurement records and contract-flow provenance | The portal explicitly covers only a subset of procurement data shared with Treasury. Missing entries do not prove that no tender or contract exists |
| South African Bureau of Standards (SABS) / South African National Standards (SANS) | Standard catalogue and jurisdiction-specific standards discovery | Publication of a SANS is not by itself proof that a requirement applies to this project or a specific decision; check legislation, incorporation, contract and current edition |
| ISO/TC 268 city indicators: ISO 37120, ISO 37122, ISO 37123 and guidance in ISO 37124 | Comparable city-service, smart-city and resilience indicators | Use as measurement references, not as automatic City targets. The ISO site lists the 2018 edition of ISO 37120 and an edition-3 working draft under development; pin the edition actually used |
| Local Government: Municipal Systems Act 32 of 2000 and deployment-specific South African privacy/accessibility/administrative-law review | Municipal service and community-participation context; privacy and review obligations | This roadmap is not legal advice, accreditation or a determination of authority. Obtain qualified review before person-level or regulated processing |
| MyMzansi Digital Transformation Roadmap | Interoperability context and future government engagement pathway | Government roadmap milestones for selected non-government access are indicative, not an entitlement, approval or guaranteed data-sharing route |

Official reference URLs:
- Johannesburg SDBIP / IDP / SLS index: https://joburg.org.za/documents_/Pages/Key%20Documents/SDBIP%20IDP%20and%20Budget/SDBIP-IDP-And-Budget.aspx
- Johannesburg Water active leak detection: https://www.johannesburgwater.co.za/blog/2019/07/17/active-leak-detection-programme/
- Johannesburg Water smart technology announcement (23 July 2026): https://www.johannesburgwater.co.za/blog/2026/07/23/johannesburg-water-accelerates-smart-technology-rollout-to-reduce-water-losses/
- National Treasury Municipal Money API: https://municipaldata.treasury.gov.za/docs
- eTender Transparency Portal: https://data.etenders.gov.za/
- SABS: https://www.sabs.co.za/
- ISO 37120: https://www.iso.org/standard/68498.html
- ISO 37123: https://www.iso.org/standard/70428.html
- ISO 37124: https://www.iso.org/obp/ui?_escaped_fragment_=iso%3Astd%3Aiso%3A37124%3Aed-1%3Av1%3Aen
- Municipal Systems Act: https://www.gov.za/documents/local-government-municipal-systems-act
- MyMzansi data exchange roadmap: https://www.mymzansi.gov.za/roadmap/initiative-2

## 7. Delivery sequence: 90-day plan, gated by evidence

Timeboxes are planning windows, not promises; qualification and permissions override dates.

### Gate 0 — Align contracts (weeks 1–2)

**Work**
- Create a field-by-field crosswalk between Luminous Platform Work Case / Operations Event, Mycelix service-journey and applicability schemas, and Sol-Atlas civic observation/case records.
- Identify which base contracts are actually qualified at their current exact heads; list required, optional, skipped and unrelated CI checks separately.
- Freeze one shared synthetic scenario manifest format and assign unique case/evidence IDs with no person-level data.
- Write source and data-rights metadata before adding any connector.

**Exit:** the models have non-overlapping responsibility statements, every adapter maps explicit fields, and no work depends on an assumed shared schema or unqualified privileged mutation.

### Gate 1 — Public-data vertical slice (weeks 2–4)

**Work**
- Validate public-source inventory and observation contracts in Luminous Platform.
- Select a small public dataset with a confirmed, safe retrieval method, coverage explanation and licence; if the Treasury endpoint mismatch is unresolved, use an approved static published file or synthetic fixture rather than downgrade transport security.
- Add an offline case/evidence replay in Sol-Atlas and publish its limitations.
- Add source conflict, stale data, missing-data and correction scenarios to Mycelix / Symthaea reference corpora.

**Exit:** a clean-room reproduction can regenerate the same evidence pack; source gaps and normalisation are visible; no private records/secrets/network access are required.

### Gate 2 — Water workflow shadow model (weeks 4–8)

**Work**
- Refine the pilot protocol and service-standard clocks with Mycelix; preserve differing standard versions and distinguish service target from actual performance.
- Run leak-detection bench experiments separately for pipe-coupled contact sensing and stand-off/exposed plumbing. Use controlled, labelled conditions; compare simple signal-processing baselines against Symthaea. Keep satellite indicators as supporting context, not leak ground truth.
- Produce synthetic water/power dependency cases and replay delayed, duplicated, out-of-order and conflicting observations.
- Define workforce task descriptions, training/supervision, paid task measures and worker-safety requirements with prospective partners; no labour outcomes are claimed before observations exist.

**Exit:** frozen test fixtures, independent labels, defined false-positive/false-negative costs, source-to-case provenance and a clear list of conditions where the model must abstain.

### Gate 3 — Simulation, fairness and deployability (weeks 6–10; parallel)

**Work**
- Use Symtropy to compare a deterministic first-come/priority baseline with alternative inspection, crew-routing and queue policies. Vary travel times, stockouts, staffing, outage duration, source delay, under-reporting and uncertainty.
- Measure outcome quality and geographic equity as separate objectives; do not hide trade-offs in one score.
- Package a fully synthetic replay in Nixward's reproducible environment, with documented backup/restore, logs, resource budget and an offline/manual fallback.
- Add Xenia identity/authority controls only if the exact trust and restore chain is qualified. Otherwise, keep privileged paths absent and test only local simulation.

**Exit:** every simulation result names its assumptions and input digest; a simple independent checker recomputes summary metrics; the system remains usable with AI and network access disabled; no privileged operation is reachable from advisory output.

### Gate 4 — Partner discovery and authorised shadow mode (weeks 8–12 or later)

**Work**
- Seek an explicit sponsor and data owner among the relevant service operator, ward/community organisations, university/lab or maintenance/workforce partner.
- Submit a narrow data-use request describing fields, purpose, retention, access roles, security, evaluation, human oversight, deletion and publication constraints.
- Prefer an authorised historical export first; define how it is redacted and verified before using it.
- Preregister an evaluation baseline, success/failure criteria, subgroup analysis and stop rules before staff see recommendations.
- Start read-only shadow comparison only after permission, threat model, support plan and exit criteria are signed off.

**Exit:** written authority, named accountable owner, tested rollback/outage paths, evaluation protocol and a human route for correction. If permission or safe data access is unavailable, stay at Gate 1–3; the synthetic/public work remains a valid deliverable.

## 8. Shared evaluation scorecard

Do not define ungrounded numeric targets. Agree the baseline, denominator, data source, owner and acceptance threshold before looking at comparative results.

| Dimension | Measures |
|---|---|
| Data quality | Source coverage; freshness; missing/stale/contradictory share; normalisation errors; correction frequency |
| Case integrity | Provenance completeness; false-duplicate and false-join rates; replay rejection; valid state transitions; resolution-proof completeness |
| Service outcome | Acknowledgement/inspection/repair intervals where evidenced; repeat-incident rate; independently verified resolution rate; reopen/appeal rate |
| Operations | Staff time per case; repeat visits; travel and queue time; delayed-part sensitivity; cost per verified outcome |
| Workforce / community | Paid tasks completed; training/supervision; worker safety; fair access; resident-reported burden and correction success |
| Equity / access | Outcomes by ward/region and data coverage; under-reporting indicators; language/offline accessibility; no displacement of underserved areas |
| Model quality | Precision/recall on labelled synthetic or consented data; calibration; abstention; counterfactual stability; improvement over deterministic baseline |
| Safety / governance | Unauthorized-action attempts rejected; least privilege; rollback/restore evidence; privacy leaks; AI-offline/manual fallback tests |

Every performance headline must label whether it is a published source claim, synthetic benchmark result, simulation result, independently measured field outcome or unverified hypothesis.

## 9. Qualification snapshot — 14 October 2026

This is a time-stamped engineering snapshot only. Re-query each exact head before relying on it; a workflow that did not trigger, was skipped or remains queued is not a pass.

| Track / exact head | Observed result | Implication |
|---|---|---|
| Previous Luminous Platform PR #34 head — a4736a4decdf00d3fd001e90f96a3da08a34c376 | Public Source and Civic Crosswalk Contracts workflow #38616710257 was queued when checked | Queued was not a pass; the next crosswalk commit requires a new exact-head run |
| Mycelix PR #5016 — e53d245945e2dc24a70498e4441cc4a5bb1c6807 | Water Workforce Pilot Qualification queued | No pass established |
| Mycelix PR #5019 — 4f29a02acb591285e774724fd764e439e2881c31 | Life/civic contract queued; D6S qualification pending; general CI skipped | No complete qualification established |
| Symthaea PR #7443 — db537f003a086f9bcabf00b3277e61028a707440 | Life/civic reference and crime-economics workflows queued; other workflows skipped | No pass established for the reference suite |
| Symthaea PR #7414 — 6ac47344fe854386df42b63ce2a05cdc4e28fc00 | Water Leak Satellite Offline Tests completed successfully; several broad project workflows skipped | Narrow offline-test success only, not full Symthaea or field qualification |
| Symthaea PR #7425 — 3f7c3b08ffae3718f881dfa350ceb0f06b424d34 | Energy Telemetry Privacy Contract workflow failed; broad workflows skipped | Needs failure review before reuse |
| Sol-Atlas PR #104 — b561102f77df76f779b4b5c30a2cccb361074afe | Check workflow completed successfully | Qualification evidence for this workflow only |
| Sol-Atlas PR #109 — 1c84081f05abdbb6b824f83284dcaab7f62fca81 | current Check run queued; prior head 768b20d5dab207cbd53b37d1bd56fc53632ff3b8 had native tests and WASM check succeed but formatting fail | No pass established on the current exact head; prior fmt corrections were applied and need exact-head requalification |
| Symtropy PR #1602 — a46ef8d348394ba3dca41d85f237c1e3ef76b700 | CI queued | No pass established |
| Nixward PR #58 — 34ef8807cedd917c8116218c366d87642d824c7a | standalone qualification queued; previous listed attempts skipped | No pass established |
| Xenia peer PR #451 — 208423e7786e18364679983b05865162e28e840a | validation, ledger format, restore/runtime and durable-frontier checks failed; two listed checks succeeded | Not qualified for privileged civic use |
| Luminous Platform PR #4 — b4b32d90742e9a218e4d6919f15d74d904f5beca | Operations Event and Holochain Attestation Contracts workflow #38043424999 failed; Rust qualification stopped on SQLx 0.9 dynamic SQL compilation and the research-only Python suite failed import on a missing colon | Schema is pinned but unqualified; repair the observed failures and rerun exact-head qualification before adapters | 
| Luminous Platform PR #8 — 384a60d9668e77c40561b46d5511593ea5198c23 | No workflow runs were returned for this exact head during this audit | Schema is pinned but unqualified; do not infer a pass from absence of runs |
| Luminous Platform PR #28 — c33dc0cd12223fe40d66787aea357577591c15da | deployment-profile-validator failed | Do not treat the service deployment contract as qualified |

The snapshot suggests a practical rule: **build the civic demonstrator so it does not depend on unqualified write, identity or deployment paths.** Use each narrow, already successful check only for the property it actually checked. Do not try to “integrate everything” by stacking more code on top of failing or unqualified dependencies.

## 10. Immediate next actions, in order

1. **Keep the public-data demonstrator moving independently:** source catalogue, safe source retrieval, synthetic evidence/case replay, deterministic checker and clear limitations.
2. **Fix the Sol-Atlas #109 formatting failure** and confirm the rerun against its current exact head. Earlier native/WASM successes are historical evidence, not a green aggregate.
3. **Repair the confirmed Platform PR #4 blockers** (SQLx 0.9 dynamic-query compile error and malformed research-only test definition); then rerun exact-head qualification. Check PR #8's stale base/stack ancestry and qualify its Work Case contract separately.
4. **Review the Symthaea #7425 privacy-contract failure** and the Luminous Platform #28 validator failure. Do not rely on those paths while they remain unqualified.
5. **Review the 20 crosswalk rules** against pinned source blobs and add adapter conformance fixtures only once the source contracts are qualified.
6. **Freeze and preregister the water workflow benchmark** and lab protocol before experimenting with model-based detection or queue optimization.
7. **Request partner review only with a bounded proposal:** read-only shadow evaluation, named service owner, specified data fields, no autonomous action and measurable stop conditions.

## 11. Non-goals and stop conditions

The first programme does not authorise live telemetry/SCADA access, crew dispatch, a new resident registry, identity or eligibility decisions, clinical workflows, facial recognition, individual risk scores, supplier accusations, enforcement, disconnections or automated adverse decisions. It does not replace municipal complaint channels, MyMzansi, official standards bodies or legally accountable service operators.

Stop or remain in the current phase if source terms, authority, freshness, reference accuracy, privacy protection, reproducibility, field labels, manual fallback or independent verification are inadequate. More code is not the remedy for missing authority or ground truth.

## 12. Reference sources

- City of Johannesburg SDBIP, IDP and SLS index: https://joburg.org.za/documents_/Pages/Key%20Documents/SDBIP%20IDP%20and%20Budget/SDBIP-IDP-And-Budget.aspx
- Johannesburg Water active leak detection programme: https://www.johannesburgwater.co.za/blog/2019/07/17/active-leak-detection-programme/
- Johannesburg Water smart technology announcement, 23 July 2026: https://www.johannesburgwater.co.za/blog/2026/07/23/johannesburg-water-accelerates-smart-technology-rollout-to-reduce-water-losses/
- National Treasury Municipal Money API: https://municipaldata.treasury.gov.za/docs
- National Treasury eTender Transparency Portal / OCDS data: https://data.etenders.gov.za/
- SABS: https://www.sabs.co.za/
- ISO 37120: https://www.iso.org/standard/68498.html
- ISO 37123: https://www.iso.org/standard/70428.html
- ISO 37124: https://www.iso.org/obp/ui?_escaped_fragment_=iso%3Astd%3Aiso%3A37124%3Aed-1%3Av1%3Aen
- Municipal Systems Act 32 of 2000: https://www.gov.za/documents/local-government-municipal-systems-act
- MyMzansi roadmap phases: https://www.mymzansi.gov.za/roadmap
- MyMzansi data exchange initiative: https://www.mymzansi.gov.za/roadmap/initiative-2

**Qualification caveat:** the GitHub workflow snapshot above is reproduced for coordination, not as a permanent source of truth. Before merging or reusing any component, inspect the current PR head and checks and record the exact revision and completed job outcomes.
