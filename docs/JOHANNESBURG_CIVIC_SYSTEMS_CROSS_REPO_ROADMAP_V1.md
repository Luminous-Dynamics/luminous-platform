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

The public Platform PR branch now includes a separate contract in the [civic-record-crosswalk-v1 directory](https://github.com/Luminous-Dynamics/luminous-platform/tree/docs/public-service-operations-profile-rfc-001/contracts/civic-record-crosswalk-v1/). It pins exact audited Mycelix service-journey and applicability schemas, the Mycelix service-journey evaluation corpus v0.4.0, Sol-Atlas Rust civic-case types, the Platform public-observation schema and Symthaea's life/civic evaluation corpus v0.2.0. The Mycelix/Symthaea corpora now share an explicit semantic-overlap mapping for the power–water causality negative control. Their case IDs, evidence structure and expected outputs remain distinct; the mapping is `preserve_separately`, not a runtime adapter or claim of equivalent execution.

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

The model must keep these states distinct: **zero, null, missing row, not published, stale, invalid, conflicting and not checked**. An unavailable source is not a negative finding. A source's stated target is not the measured outcome. A model score is not ground truth. A repair claim is not independently verified restoration. Synthetic test data has its own hard provenance rule: `fixture_only=true` requires `retrieved_at=null` and no captured-source digest; a captured observation must carry an RFC3339 retrieval time with an explicit timezone and a valid digest of the captured source snapshot. A human-authored placeholder time must never be placed in the retrieval field. The draft observation schema and Rust validator now enforce the same distinction, but are marked unqualified pending exact-head CI.

### Spatial/privacy rules

- Prefer public region/ward-level analysis; do not publish household coordinates, private contact details, account numbers, unique vulnerabilities or asset locations that could expose critical infrastructure.
- Verify the meaning and allowed granularity of every geographic code with a responsible data steward; a code format alone does not prove its geography.
- Preserve an internal record only under documented authority and purpose. Public projection must be an explicit allowlisted view, not serialisation of the internal record with a few fields hidden in the UI.
- Aggregate small cells or suppress them when publishing could re-identify people, expose a vulnerable household or reveal sensitive infrastructure. Preserve a documented suppression reason.
- Do not use report volume or smartphone access as a proxy for need; model under-reporting and missingness as first-class risks.

## Fresh official examples for the first scenario corpus (reviewed 15 October 2026)

Three City announcements published in October make the cross-service case for the first vertical slice unusually concrete:

- **Johannesburg Water depot, Randburg (12 October):** the City/JDA says the project is above 89% progress and in final close-out, while testing/commissioning and required Certificates of Compliance are still being finalised. This is a strong regression case for “progress percentage ≠ operational acceptance”. The reported service region includes Region C and parts of Regions B, D and F; that is project context, not a live performance result. Source: https://joburg.org.za/media_/Newsroom/Pages/2026-News-Articles/Joburg-Water-depot-upgrade-to-strengthen-service-delivery.aspx
- **Braamfontein traffic signals and accessibility (2 October):** the City/JRA statement describes upgrades at eight intersections, backup power with Wits as partner, countdown timers, audible pushbuttons and widened pavements. It reports a R19 million project and delivered upgrades, but does not provide independent signal-uptime or safety-impact measurements. Source: https://joburg.org.za/media_/Newsroom/Pages/2026-News-Articles/Braamfontein-traffic-upgrades-set-to-make-journeys-safer-and-smoother.aspx
- **Malvern Clinic (14 October):** the City says the refurbished clinic reopened after addressing electricity problems and connecting to water reticulation, noting earlier water shortages sometimes forced closure. That is a real cross-dependency test between municipal water, electricity and access to healthcare, but it is not evidence of today's staffing, availability or clinical outcomes. Source: https://joburg.org.za/media_/Newsroom/Pages/2026-News-Articles/Refurbished-Malvern-Clinic-restores-access-to-healthcare.aspx

Johannesburg Water's 8 October Customer Notice page lists four image attachments rather than a text-normalised incident table. The first public demonstrator should therefore treat article pages as discovery pointers, use human-reviewed per-article observations, preserve the exact URL and publication date, and require capture time/content digest only after an actual source capture. It must not claim API access, OCR correctness or current service status from the archive. Source: https://www.johannesburgwater.co.za/blog/2026/10/08/customer-notice-8-october-2026/

The source-discovery rule remains unchanged: public announcements seed *attributed claim fixtures*, not authoritative live work-order states. Any future extraction must retain source content and provenance, record who reviewed transcription, and keep the original claim separate from subsequent inspection or independent verification.

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

## 9. Previous qualification snapshot — 15 October 2026 (superseded by section 9A)

This is a time-stamped exact-head check, not a prediction. A workflow that is queued, pending, skipped, absent or failed is not a pass. Where a check is a narrow contract or fixture workflow, its result qualifies only that scope.

| Track / exact head at review time | Observed workflow evidence | Implication |
|---|---|---|
| Luminous Platform PR #34 — pre-refresh head `70525f66bebee067abcd686a894453a1aaf7e1ff` | Public Source and Civic Crosswalk Contracts [#38691314398](https://github.com/Luminous-Dynamics/luminous-platform/actions/runs/38691314398) queued. Prior run [#38633248000](https://github.com/Luminous-Dynamics/luminous-platform/actions/runs/38633248000) failed before tests because Rust 1.96.0 had no `cargo-fmt` component. The workflow now explicitly installs `rustfmt`; this roadmap/workflow commit creates a newer exact head and requires a fresh run. | No pass established for the new head. The new workflow additionally checks formatting of both Rust contract crates before test and fixture validation. |
| Mycelix PR #5016 — `097aabfc426d9a8b06a5e46fac584bf34ed31298` | Johannesburg Water Workforce Pilot Qualification [#38690807437](https://github.com/Luminous-Dynamics/mycelix/actions/runs/38690807437) queued. | No pass established; the PR targets `fix/d6s-canonical-workflow-path-scope`, so its base/stack ancestry must be reconciled before merge. |
| Mycelix PR #5019 — `227e0b174c58044c75f1a56f6b78730fdf7fa4d6` | Life/civic reference workflow [#38689970670](https://github.com/Luminous-Dynamics/mycelix/actions/runs/38689970670) queued; general Mycelix CI [#38689970659](https://github.com/Luminous-Dynamics/mycelix/actions/runs/38689970659) skipped. | No complete qualification established. |
| Symthaea PR #7443 — `e78717f569d7aa5956e2f70c891f257417c2429f` | Life/civic scenarios [#38689702336](https://github.com/Luminous-Dynamics/symthaea/actions/runs/38689702336), crime-economics reference [#38689702298](https://github.com/Luminous-Dynamics/symthaea/actions/runs/38689702298) and synthetic evaluator [#38689702259](https://github.com/Luminous-Dynamics/symthaea/actions/runs/38689702259) queued; broad CI, benchmark and governance workflows skipped. | No pass established for the civic reference/evaluator suite. |
| Symthaea PR #7414 — `6ac47344fe854386df42b63ce2a05cdc4e28fc00` | Water Leak Satellite Offline Tests [#38577467963](https://github.com/Luminous-Dynamics/symthaea/actions/runs/38577467963) succeeded; broad CI, benchmark and governance workflows skipped. | Narrow offline test success only; no proven leak-detection accuracy, satellite detection or field qualification. |
| Symthaea PR #7425 — `c6c3a252c1fec946be2104367d080013e6c06d82` | Energy Telemetry Privacy Contract [#38619740708](https://github.com/Luminous-Dynamics/symthaea/actions/runs/38619740708) succeeded; broad CI, benchmark and governance workflows skipped. | Narrow privacy-contract success only; no overall Symthaea qualification or actual satellite-field qualification. |
| Sol-Atlas PR #104 — `b561102f77df76f779b4b5c30a2cccb361074afe` | Check workflow [#38509723929](https://github.com/Luminous-Dynamics/sol-atlas-leptos/actions/runs/38509723929) succeeded. | Narrow workflow evidence only; not a live data integration or field result. |
| Sol-Atlas PR #109 — `e5b68e0fd30667ac9712a839faadc951e939c42c` | Check [#38628758077](https://github.com/Luminous-Dynamics/sol-atlas-leptos/actions/runs/38628758077) completed successfully on this exact head. | This is the strongest current application-level check among the listed civic tracks, but only qualifies the Check workflow's native/WASM/format scope; it does not qualify municipal data access or operational outcomes. |
| Sol-Atlas PR #108 — `00189b8eda44bdc3a6ab1d7d7ddcfd5bd4106eaa` | Check [#38691734943](https://github.com/Luminous-Dynamics/sol-atlas-leptos/actions/runs/38691734943) and Earth systems catalog validation [#38691734820](https://github.com/Luminous-Dynamics/sol-atlas-leptos/actions/runs/38691734820) queued following a test/profile repair. The prior Earth systems catalog run [#38618014568](https://github.com/Luminous-Dynamics/sol-atlas-leptos/actions/runs/38618014568) failed six agroclimate tests. | New head unqualified. Do not weaken the source/manifest guards to make the tests pass; verify this patch removes assertion defects while preserving the evidence requirements. The PR remains stacked on the Earth Systems Intelligence branch. |
| Symtropy PR #1602 — `3d7d9ce4e8fc1039c5dc38c67f3b24785ed2d3b5` | CI [#38690366367](https://github.com/Luminous-Dynamics/symtropy/actions/runs/38690366367) queued. | No pass established for the modular transport model. |
| Symtropy PR #1610 — `4d3267e0fb395a9d67bbf99571ba2fb8edf09ca0` | No workflow run was returned for the exact head during this review. | No pass established; absence of a run is not qualification. |
| Nixward PR #58 — `34ef8807cedd917c8116218c366d87642d824c7a` | Standalone qualification [#38590820715](https://github.com/Luminous-Dynamics/nixward/actions/runs/38590820715) failed. Earlier listed attempts were skipped. | No full deployment/recovery qualification. |
| Xenia peer PR #451 — `208423e7786e18364679983b05865162e28e840a` | Validation [#38354999757](https://github.com/Luminous-Dynamics/xenia-peer/actions/runs/38354999757), witness frontier [#38354999852](https://github.com/Luminous-Dynamics/xenia-peer/actions/runs/38354999852), runtime smoke [#38354999808](https://github.com/Luminous-Dynamics/xenia-peer/actions/runs/38354999808), durable frontier [#38354999908](https://github.com/Luminous-Dynamics/xenia-peer/actions/runs/38354999908), persistence [#38354999892](https://github.com/Luminous-Dynamics/xenia-peer/actions/runs/38354999892) and CI [#38354999817](https://github.com/Luminous-Dynamics/xenia-peer/actions/runs/38354999817) failed; artifact and WASM checks succeeded. | Not qualified for privileged civic use. |
| Luminous Platform PR #4 — `b9b8ae1483e8b924d49364f9d972ccdeb58115f3` | Operations Event and Holochain Attestation Contracts [#38618166253](https://github.com/Luminous-Dynamics/luminous-platform/actions/runs/38618166253) failed. The workflow must be inspected for the current exact failure before reuse. | Keep adapters involving Operations Event V1 blocked until corrected exact-head qualification. |
| Luminous Platform PR #8 — `7ec5588d9844b10006b3264e34f30bd4aeba48ec` | An ancestry-preserving merge now includes the two-file parent fix from PR #4; the new combined workflow [#38693004364](https://github.com/Luminous-Dynamics/luminous-platform/actions/runs/38693004364) is queued. The prior head's workflow [#38618946724](https://github.com/Luminous-Dynamics/luminous-platform/actions/runs/38618946724) failed on the inherited invalid `observedat` test. | Parent ancestry and narrow fixes are reconciled; no current exact-head pass is established. Keep Work Case mappings blocked pending the new workflow and separate Work Case qualification. |
| Luminous Platform PR #28 — `c33dc0cd12223fe40d66787aea357577591c15da` | Deployment-profile validator [#38309301818](https://github.com/Luminous-Dynamics/luminous-platform/actions/runs/38309301818) failed. | Do not treat the workload/deployment contract as qualified. |

The practical dependency rule is: **the public-data demonstrator must remain independent of unqualified write, identity, and deployment paths.** Narrow successful contract checks can support only the claim they directly tested. They do not establish runtime interoperability, model accuracy, legal applicability, operational safety, municipal endorsement or field impact.

**Snapshot caveat:** the table pins the exact heads observed before the latest roadmap/workflow commit. That commit creates a new head for PR #34; its own CI result must be fetched and recorded separately. Queued, skipped, failed or absent runs never qualify a newer revision.

## 9A. Latest exact-head qualification refresh — 15 October 2026

The table in section 9 is retained as a historical snapshot. The following is the latest cross-check performed before the current source-pin refresh commit. After the refresh changes PR #34's head, the new exact-head workflow must be checked separately; no prior or queued result transfers automatically.

| Track / exact head at refresh | Latest observed workflow evidence | Current interpretation |
|---|---|---|
| Luminous Platform PR #34 — predecessor head `f04951f8f1fed8f0ea1d244824e1d39d8f71089d` | The combined Public Source and Civic Crosswalk Contracts run [#38703017164](https://github.com/Luminous-Dynamics/luminous-platform/actions/runs/38703017164) was queued after formatter corrections. Before that, exact head `4a1448efa630d350cd507753cf447bb7a2ecdeaa` failed only at public-source catalogue `cargo fmt --check`; tests were skipped. This refresh adds strict calendar validation for `review_snapshot`, tests impossible dates and leap years, refreshes source pins and records current upstream qualification. The follow-on integrity hardening adds an HTTPS read-only verifier that retrieves each pinned public artifact at its exact commit and recomputes the Git blob SHA-1; this new exact-head run must pass before the verification claim is relied on. | No pass on the refreshed head. New check must finish formatting, tests and fixture validation before this crosswalk is relied upon. |
| Mycelix PR #5016 — `22abffce5796ce9bd2fd371e1d2f1beb496d13b1` | Johannesburg Water Workforce Pilot Qualification [#38702636295](https://github.com/Luminous-Dynamics/mycelix/actions/runs/38702636295) queued. | No pass established. It targets a separate workflow-fix branch; resolve ancestry and exact-head qualification before merge. |
| Mycelix PR #5019 — `4f74e4338d44e3523f6f9f629455beabd834b1ae` | Life/civic reference workflow [#38701461835](https://github.com/Luminous-Dynamics/mycelix/actions/runs/38701461835) queued; general CI skipped. | Keep service/applicability and corpus artifacts `unqualified_draft`; no adapter qualification. |
| Symthaea PR #7443 — `18697d2737af4164ef0ffa54cf7293c8e97bc792` | Life/civic scenarios [#38695477970](https://github.com/Luminous-Dynamics/symthaea/actions/runs/38695477970), crime-prevention reference [#38695477851](https://github.com/Luminous-Dynamics/symthaea/actions/runs/38695477851), and crime-economics synthetic evaluator [#38695478067](https://github.com/Luminous-Dynamics/symthaea/actions/runs/38695478067) succeeded. Broad CI, benchmark, showroom integrity and governance workflows were skipped. | Narrow corpus/evaluator success only. The corpus pin is refreshed to blob `574f6d3ba235c68e306f84f5ee761d399cc165b6`; no overall model, service recommendation or field-performance qualification is implied. |
| Symthaea PR #7414 — `6ac47344fe854386df42b63ce2a05cdc4e28fc00` | Water Leak Satellite Offline Tests [#38577467963](https://github.com/Luminous-Dynamics/symthaea/actions/runs/38577467963) succeeded; broad CI, benchmark and governance workflows skipped. | Narrow offline source/catalogue test only; no live leak detection or field accuracy result. |
| Symthaea PR #7425 — `c6c3a252c1fec946be2104367d080013e6c06d82` | Energy Telemetry Privacy Contract [#38619740708](https://github.com/Luminous-Dynamics/symthaea/actions/runs/38619740708) succeeded; broad CI, benchmark and governance workflows skipped. | Narrow privacy-contract result only. |
| Sol-Atlas PR #104 — `b561102f77df76f779b4b5c30a2cccb361074afe` | Check [#38509723929](https://github.com/Luminous-Dynamics/sol-atlas-leptos/actions/runs/38509723929) succeeded. | Narrow contract/native/WASM check only; no live source connector or satellite-derived fault proof. |
| Sol-Atlas PR #109 — `e5b68e0fd30667ac9712a839faadc951e939c42c` | Check [#38628758077](https://github.com/Luminous-Dynamics/sol-atlas-leptos/actions/runs/38628758077) succeeded. | Case-ledger source has a narrow repo Check success; cross-repository interoperability remains unqualified. |
| Sol-Atlas PR #108 — `00189b8eda44bdc3a6ab1d7d7ddcfd5bd4106eaa` | Check [#38691734943](https://github.com/Luminous-Dynamics/sol-atlas-leptos/actions/runs/38691734943) and Earth-systems catalogue validation [#38691734820](https://github.com/Luminous-Dynamics/sol-atlas-leptos/actions/runs/38691734820) succeeded after profile/test repair. | Narrow profile/catalogue checks only; no measured crop yield, farm outcome or operational food-resilience result. |
| Symtropy PR #1602 — `3d7d9ce4e8fc1039c5dc38c67f3b24785ed2d3b5` | CI [#38690366367](https://github.com/Luminous-Dynamics/symtropy/actions/runs/38690366367) succeeded. | Narrow deterministic transport model check; no live dispatch or measured route improvement. |
| Symtropy PR #1610 — `93f937f7260abb3f4cd971e7ff8476e4d0772274` | No workflow run returned for the exact head. | No pass established. |
| Nixward PR #58 — `34ef8807cedd917c8116218c366d87642d824c7a` | Standalone qualification [#38590820715](https://github.com/Luminous-Dynamics/nixward/actions/runs/38590820715) failed. | Deployment/rollback/recovery stack is not qualified for civic use. |
| Xenia peer PR #451 — `208423e7786e18364679983b05865162e28e840a` | Validation [#38354999757](https://github.com/Luminous-Dynamics/xenia-peer/actions/runs/38354999757), witness frontier [#38354999852](https://github.com/Luminous-Dynamics/xenia-peer/actions/runs/38354999852), runtime smoke [#38354999808](https://github.com/Luminous-Dynamics/xenia-peer/actions/runs/38354999808), durable frontier [#38354999908](https://github.com/Luminous-Dynamics/xenia-peer/actions/runs/38354999908), persistence [#38354999892](https://github.com/Luminous-Dynamics/xenia-peer/actions/runs/38354999892) and CI [#38354999817](https://github.com/Luminous-Dynamics/xenia-peer/actions/runs/38354999817) failed. Artifact and WASM checks succeeded. | Not qualified for privileged authority, evidence restore or civic mutations. |
| Luminous Platform PR #4 — `c43a68adc9543fee67d67debfd7bcf6e1387395d` | Operations Event and Holochain Attestation Contracts [#38692588693](https://github.com/Luminous-Dynamics/luminous-platform/actions/runs/38692588693) succeeded. | Narrow schema/contract check only; runtime event delivery remains unqualified. |
| Luminous Platform PR #8 — `662aaf2bd2a3f873fd8f6fc0eb4181f061043794` | Operations Event, Work Case and Holochain Attestation Contracts [#38694337384](https://github.com/Luminous-Dynamics/luminous-platform/actions/runs/38694337384) succeeded. | Narrow contract success; the crosswalk leaves state translations blocked until independent runtime checks pass. |
| Luminous Platform PR #28 — `c33dc0cd12223fe40d66787aea357577591c15da` | Deployment-profile validator [#38309301818](https://github.com/Luminous-Dynamics/luminous-platform/actions/runs/38309301818) failed. | Do not claim workload packaging or deployment qualification. |

## 10. Immediate next actions, in order

1. **Finish the public-data contract slice.** Run the latest exact-head source-catalogue and crosswalk workflow with Rust formatting, conformance tests, catalogue validation and four synthetic Johannesburg claim fixtures. Keep ingestion disabled; no captured-source timestamps or digests are fabricated.
2. **Validate the agroclimate test repair.** The current Sol-Atlas #108 patch fixes error-path assertions and a missing synthetic private-artifact manifest, restores explicit regional-supply and boundary-withdrawal language, and updates the review date. Wait for exact-head check + Earth-systems validation; keep the profile design-only.
3. **Keep the source-case crosswalk descriptive.** Review all mappings against pinned blobs. Do not write runtime adapters for Work Case V1 or Operations Event V1 until PR #8 / #4 exact-head dependencies are qualified and their stack ancestry is reconciled.
4. **Use official City notices as attributed claims, not operational feeds.** The public scenario set now spans water depot readiness, Braamfontein traffic-signal accessibility and Malvern Clinic reopening; manually capture each source only when actual source review happens, and require separate operational/independent evidence before upgrading claim status.
5. **Freeze the water-workflow benchmark.** Use Mycelix and Symthaea scenario corpora to separate a published cross-utility statement from a particular live outage, causal proof, repair and independently verified restoration. Record false-positive costs and abstention cases before any model comparison.
6. **Repair non-civic prerequisite failures without widening authority.** Review Xenia #451, Nixward #58 and Platform #4/#8/#28 current failures. Until those dependencies qualify, the civic proof stays read-only, synthetic and reproducible.
7. **Only then request partner review.** Propose one named owner and read-only shadow phase with a specified dataset, retention limits, security controls, baseline, equity review and stop rules. No real municipal feed, resident records or live dispatch is assumed.

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
