# Civic services and OpenStart: Luminous Platform integration proposal

**Status:** Research/design proposal for review (2026-10-13). This is not an approved architecture, a production implementation, or an official government integration.

## Executive decision

Use **Luminous Platform as the secure deployment, operations, and evidence foundation** for a new, separately deployable civic-services application. Do not turn the platform repository into the citizen-facing product, and do not couple the product to a single government's APIs.

Build two experiences over shared workflow infrastructure:

1. **Open Civic Services** — a plain-language, accessible guide that maps a person's goal to the official services and steps needed to complete it.
2. **OpenStart** — an independent-business launch workflow, beginning with a Johannesburg food-service business and later reusable for other business types and jurisdictions. It combines public-service guidance with optional operational tools (startup budget, recipe/menu costing, inventory, supplier comparison, and cash-flow scenarios).

The first release should be a **source-grounded navigator and preparation tool**, not a proxy government, legal adviser, permit issuer, identity provider, or autonomous agent that submits consequential forms.

## Why this is worth testing

Cross-agency journeys are organized around people's goals, while government services are often organized around departments. A person opening a restaurant needs a coherent sequence that may involve company registration, tax, premises suitability, local business licensing, food-safety requirements, and operating costs. The specific requirements depend on the business model and location, so the product must expose jurisdiction and source evidence rather than flattening all rules into one universal checklist.

Existing services are important integration points, not competitors to copy. South Africa's BizPortal already provides company registration and related services through CIPC and participating agencies. The City of Johannesburg publishes its business-licensing requirements, including the requirement for a licence for food provision businesses covered by the relevant schedule. South Africa's food-premises hygiene regulations establish the Certificate of Acceptability framework. The US FDA's 2026 Food Code is a model offered for adoption by relevant jurisdictions; it is not a single nationwide restaurant permit rule.

## Product and repository boundaries

### Citizen-facing application (new, separate product repository)

Owns:
- responsive user interface, accessibility, plain-language content, localization, and low-bandwidth behavior;
- resumable checklists and user-controlled case progress;
- jurisdiction selection and service discovery;
- source references displayed beside requirements;
- links to official portals and clearly labeled future integrations;
- optional OpenStart planning tools, kept separate from official obligations.

The interface must make its status unmistakable: unless a government agency has formally authorized an integration, this is an independent guidance tool and not an official government service.

### Luminous Platform

Use for:
- reproducible, version-pinned deployment and configuration;
- deployment provenance, release records, and rollback/recovery procedures;
- secret management and service-to-service identity;
- operational health, audit event export, backup/restore, and incident response;
- constrained host operations through existing platform contracts;
- a common way to run the web/API/workflow services without requiring the cognitive stack to be available for basic service continuity.

Keep the product runtime independent from desktop boot UX and host-management internals. In particular, the platform's host-level fail-open/recovery principle **must not** become a fail-open rule for application authorization or policy checks. If a rule is stale, evidence is missing, or a protected integration is unavailable, preserve the draft and provide safe guidance, but do not mark a requirement as satisfied or submit an action.

### Symthaea

Potential role:
- retrieval-grounded explanation of official sources;
- converting a user's goal into candidate next steps, subject to the deterministic workflow rules;
- asking for missing non-sensitive context;
- explaining why a step applies, identifying uncertainty, and preparing document checklists;
- scenario analysis for optional restaurant costs, margins, purchasing, and cash flow.

Hard boundary: Symthaea-generated prose is advisory. It cannot create a legal requirement, certify compliance, decide official eligibility, approve a permit, or override the workflow's evidence and freshness gates. Any statement about a requirement must point to the versioned source record that supports it. Uncertain/conflicting cases route to an explicit "verify with the issuing authority" state.

### Mycelix

Potential role after the simplest MVP:
- consent-bound coordination across participating suppliers, business-support organizations, and agencies;
- auditable acknowledgements and referrals between parties;
- tamper-evident references to workflow events where this is justified by an actual trust requirement.

Do not store identity-document images, national ID numbers, addresses, application contents, or sensitive personal records on a public or replicated ledger. Keep personal data in a minimal, access-controlled service store with retention, correction, deletion, and access policies. Use off-ledger references and signed attestations only where necessary and lawful. Mycelix coordination does not itself prove a person's civil identity or make a private assertion an official decision.

### Symtropy and Sol-Atlas

Leave these as optional future modules, not MVP dependencies. They may later support explicit optimization or public-infrastructure workflows, but they should be introduced only where a measurable use case and a well-defined input/output contract exist. Municipal infrastructure control is outside the first restaurant/civic guidance pilot.

## Reference architecture

~~~text
Citizen / business owner
        |
Accessible web app (Open Civic Services / OpenStart)
        |
Goal-oriented workflow API ---- Jurisdiction service catalog
        |                               |
Deterministic rules engine       Versioned source/evidence records
        |                               |
Case progress store <----------- Review + freshness process
        |
        +---- Official portal links (MVP)
        +---- Approved APIs / SSO / payments (only after agreement)
        +---- Optional, consent-bound Mycelix coordination
        |
Grounded Symthaea assistance (advisory sidecar; never policy authority)

Luminous Platform: reproducible deployment, secrets, operations,
release provenance, audit export, backup/recovery, access boundaries
~~~

The cognitive component must be optional for core checklists and official links to function. A user should still be able to review saved progress when Symthaea is unavailable. No user-facing workflow should require a mesh quorum to display public information.

## Jurisdiction packs and rule provenance

Never hard-code a claim as "the law" without its scope and evidence. Each jurisdiction pack should be versioned data, reviewed separately from UI code.

Minimum fields for each requirement:
- stable requirement ID and short, plain-language explanation;
- jurisdiction and applicable business/service types;
- source authority, source URL, source title, relevant section, and date checked;
- effective/published date when stated by the authority;
- applicability conditions, exceptions, and required next action;
- official submission portal, contact, or office where known;
- source/version fingerprint when feasible and legally appropriate;
- review status: **draft**, **verified**, **stale**, **disputed**, or **withdrawn**;
- reviewer identity/role, reviewed-at timestamp, and next review date.

A crawler or diff checker can alert on source changes; it cannot automatically certify that a changed web page leaves the rule valid. Human review is required before publishing material legal guidance. An expired review date, unresolved conflict, or unavailable source should visibly downgrade the claim and block any assertion that the user's requirements are complete.

Keep a clear separation among:
1. **Official fact** — directly supported by an authoritative source.
2. **Derived workflow** — a transparent, testable sequencing or applicability rule.
3. **AI explanation** — generated language linked to supporting official facts.
4. **User-provided information** — not independently verified.
5. **Official outcome** — only reported after a trustworthy receipt or confirmation from the responsible authority.

## First workflow: Johannesburg food-service business

The initial pack should begin small and make uncertainty visible. It must be reviewed against live official sources before it is presented as current advice.

1. Ask the user what type of food business they intend to run (for example, sit-down restaurant, takeaway, mobile vendor, or another model) and which premises/location they are considering.
2. Check the relevant City of Johannesburg business-licensing information. The city states that food-provision businesses selling or supplying meals, takeaways, or perishable food may fall within licence requirements; confirm applicability against the current official schedule and the actual business model.
3. Explain the Certificate of Acceptability process under the national food-premises hygiene regulations, and direct the user to the appropriate local authority. Do not infer approval from a completed checklist.
4. Surface other potential dependencies—such as zoning/land use, building use, fire safety, landlord permission, waste, signage, or environmental-health steps—only when supported by a source for the selected situation.
5. Link to CIPC BizPortal for relevant company-registration and related services, rather than reimplementing their forms without authorization.
6. Produce a separate, explicitly non-regulatory startup worksheet: rent/deposit, fit-out, equipment, utilities, ingredients, labor, insurance, payment fees, contingency, working capital, and conservative sales scenarios.
7. End with a cited list of unresolved questions and the office/official channel best placed to answer them.

Do not copy the currently displayed price or tariff from an old municipal page into a calculator without a date and current confirmation. Fees and requirements must be reviewed for recency before use.

## US expansion

Do not create one "US rules" pack. Select a named city/county and state, then verify local health department, zoning, fire/building, state tax/business registration, and other applicable requirements.

The FDA's 2026 Food Code is a current model code, but adoption and implementation vary by jurisdiction. Use it as a source for context, not as proof of what a particular county has adopted. Integrate with official identity providers such as Login.gov only when the service is eligible, the responsible agency authorizes the integration, and the security/privacy requirements are met. Never make a login integration a prerequisite for reading public guidance.

Use USWDS patterns/components for the relevant US federal design context, and align accessibility work to the applicable legal standard (including Section 508 for covered federal services). For South Africa, map interface and service decisions to the DPSA Digital Public Services Standard and relevant obligations such as POPIA after jurisdiction-specific review. Neither design systems nor this proposal alone certifies legal compliance.

## Security, privacy, and decision rights

- Data minimization: collect only what's needed for the current step; no national ID, passport scan, payment card, or sensitive application data in the prototype.
- Consent and purpose limitation: explain who can see a field, why it is needed, how long it is retained, and how a user can request access/correction/deletion as applicable.
- Encryption in transit and at rest; role-scoped access; secrets outside source control; audit access to records without copying sensitive payloads into logs.
- Separate public source evidence, personal draft data, and official response receipts into distinct storage and access policies.
- Never label a form "submitted", a permit "approved", or a user "eligible" without a verifiable official response that supports that exact state.
- No automated filing, payment, eligibility decision, or municipal actuation in the MVP.
- Protect against source spoofing, prompt injection in retrieved documents, stale regulations, cross-jurisdiction rule leakage, malicious uploads, and misleading imitation of an official portal.
- Avoid vendor lock-in in user data and workflow definitions: export a machine-readable case summary and allow users to leave the service.
- Preserve an assisted/non-digital path for people who lack a suitable device, connectivity, literacy, language access, or required identity credentials.

## MVP acceptance criteria

The first milestone is a demonstrator, not a claim of government readiness.

- One jurisdiction (City of Johannesburg) and one workflow (starting a food-service business).
- Every published requirement is linked to a reviewed, authoritative source record with a date checked and applicability scope.
- No generated legal requirement can be published without the rule/evidence record that supports it.
- Source-staleness or conflict tests force an explicit review/verification state; they never silently preserve "complete".
- Deterministic workflow and state-transition tests; representative end-to-end tests with synthetic data; reproducible screenshots/artifact hashes for review.
- Keyboard-only and screen-reader checks plus automated accessibility checks; plain-language and mobile/low-bandwidth review.
- No real citizen PII, official credentials, live payments, or unapproved government API calls in the prototype.
- Open portal links are distinguished from integrated application submission.
- The restaurant financial worksheet labels assumptions, allows scenario changes, and never represents forecast output as a guarantee.
- Operational deployment can be rolled back without losing a user's saved draft; backup/restore behavior is tested.

## Phased delivery

**P0 — Design contract:** approve repository boundaries, service catalog schema, rule lifecycle, security/privacy boundaries, accessibility targets, and license decisions.

**P1 — Static demonstrator:** one Johannesburg restaurant flow, sourced requirement cards, progress checklist, official links, and a non-binding cost workbook using synthetic/sample values.

**P2 — Verifiable workflow engine:** deterministic state transitions, source-freshness gate, exportable case summary, audit evidence, accessible tests, and deployment via Luminous Platform.

**P3 — Assisted pilot:** limited-scope usability review with consent; no government-system claims or live personal data unless operational controls and approvals exist. Measure completion of next steps, missing-document rate, user understanding, time-to-prepare, and error reports.

**P4 — Authorized integration:** only after a named agency/business partner confirms a supported workflow, legal basis, API/identity access, operational owner, incident process, and service-level expectations.

**P5 — Reuse:** add one new municipality or one named US locality using the same engine but a separately reviewed pack. Expand only after the first pack shows reliable source maintenance.

## Implementation contracts added for review

The architecture proposal now has versioned review artifacts alongside this design:

- [Jurisdiction-pack schema v0.1.0](../../schemas/jurisdiction-pack-v0.1.0.schema.json) defines the portable pack shape; JSON Schema shape checks must be supplemented by semantic validation.
- [Johannesburg food-service pack v0.1.0](../../examples/jurisdiction-packs/za-city-of-johannesburg-food-service-0.1.0.json) is a deliberately unpublished fixture. Its sources and requirements are marked unreviewed.
- [Deterministic workflow-engine contract v0.1](jurisdiction-pack-engine-contract-v0.1.md) separates user progress from evidence review and official authority outcomes, and specifies adversarial tests.
- [ADR-0001: licensing and content rights](adr/0001-licensing-and-content-rights.md) proposes a license split for the future public product and explains that this ADR does not itself relicense any existing files.

These are design contracts and fixtures, not an implementation of the engine, a verified legal checklist, or evidence that CI has passed.

## Decisions still required before production

- Create the separate public-facing repository only after reviewing ADR-0001, then add and verify the actual license files, notices, and dependency/source-rights inventory before the first public release.
- Pick the first US locality only after the Johannesburg demonstrator proves the workflow engine.
- Define who reviews and signs off source updates, and who owns responses to user-reported errors.
- Decide hosting, retention windows, data residency, incident response, and accessibility conformance evidence for any real pilot.
- Determine whether optional Mycelix coordination solves a demonstrated multi-party trust problem; keep it out of the critical path until it does.

## Sources reviewed

- South Africa DPSA, [Digital transformation policy updates and Digital Public Services Standard](https://www.dpsa.gov.za/policy-updates/e-gov/digital-transformation).
- CIPC, [BizPortal](https://www.bizportal.gov.za/) and [About BizPortal](https://www.bizportal.gov.za/about_bizportal.aspx).
- City of Johannesburg, [Business Licences](https://joburg.org.za/work_/InvestInJoburg/Pages/Business-Licences.aspx) and [licensing policy](https://joburg.org.za/work_/BusinessLicences/Pages/The-City%27s-policy-on-business-licences.aspx).
- South African Government, [Foodstuffs, Cosmetics and Disinfectants Act regulations: general hygiene requirements for food premises](https://www.gov.za/documents/notices/foodstuffs-cosmetics-and-disinfectants-act-regulations-general-hygiene-0).
- US FDA, [Food Code 2026](https://www.fda.gov/food/fda-food-code/food-code-2026) and [How to Start a Food Business](https://www.fda.gov/food/food-industry/how-start-food-business).
- USWDS, [Design system](https://designsystem.digital.gov/) and [accessibility guidance](https://designsystem.digital.gov/documentation/accessibility/).
- Section508.gov, [Accessible design using USWDS](https://www.section508.gov/develop/accessible-design-using-uswds/).

**Evidence note:** this is an architecture/research synthesis, not a verified legal checklist. Official web content and adopted regulations can change; all operational rule packs require dated source review before publication.
