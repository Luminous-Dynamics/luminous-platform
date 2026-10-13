# ADR-0001: Licensing and content rights for Open Civic Services

**Status:** Proposed for review  
**Date:** 2026-10-13  
**Scope:** Future citizen-facing application, deterministic workflow engine, jurisdiction-pack specification, and authored guidance.

## Context

The product direction is to keep public-service guidance freely accessible and portable across governments. Sustainable revenue should pay for hosting, operations, integrations, training, support, and maintenance—not a franchise fee, a royalty on each restaurant sale, or a charge for reaching a civic outcome.

The existing Luminous Platform README declares AGPL-3.0-or-later and points to a separate commercial-licensing contact. That is useful precedent, but a public-facing product needs explicit file-level and repository-level licensing; it must not inherit assumptions accidentally.

## Decision proposed

### 1. Application and deterministic workflow engine: AGPL-3.0-or-later

Use **AGPL-3.0-or-later** for the citizen-facing application and first-party workflow engine.

Why:
- It permits use, self-hosting, modification, and commercial operation.
- Its network-interaction provision is designed to make the corresponding source of a modified network service available to remote users of that service. This reduces the risk that community-developed civic infrastructure becomes a closed hosted fork.
- It is consistent with the license declared by Luminous Platform today.

Operational implication: provide a visible **Source code** link in the live service that points to the source corresponding to that deployed release, including the changes required by the license. Keep deployment manifests, version identifiers, and source references traceable.

This is a proposal for the new application; it does not change any existing repository's license and is not a substitute for license review before a public release.

### 2. Pack schema and interoperability specification: Apache-2.0

License the standalone jurisdiction-pack schema, protocol definitions, and conformance fixtures under **Apache-2.0**, with a clear SPDX declaration and license text in the repository that publishes them.

The schema should be usable by an agency or implementer without requiring them to embed or link the AGPL application. This is an intentional interoperability boundary, not a relicensing of the engine.

### 3. Original explanatory text and pack annotations: CC BY 4.0

License original explanatory prose, editorial summaries, taxonomy labels, and original workflow annotations under **CC BY 4.0**, where Luminous Dynamics has the right to grant that license. This allows reuse, adaptation, and commercial use with attribution rather than imposing a non-commercial restriction.

Do **not** imply that this license covers an upstream law, agency page, logo, form, dataset, or third-party work merely because a link or citation appears in a pack. Each source record must independently state what reuse rights are known. When rights are not established, store metadata and links, use only appropriately limited quotations where permitted, and do not redistribute the source document or substantial copied text.

### 4. Public sources, factual claims, and provenance

Keep the following separate:
- official source material and its own reuse terms;
- our concise, attributable explanation of that source;
- the deterministic applicability/sequence rule derived from it;
- AI-generated wording, which is advisory and must cite supporting source records;
- user assertions and user-marked progress;
- the issuing authority's actual response or decision.

Every source needs a rights assessment (not_assessed, link_only, public_domain, licensed, or permission_granted) plus attribution/restriction notes. Do not treat a URL, government ownership, or public accessibility as proof of redistribution permission. Do not license someone else's work on their behalf.

### 5. Contributions and sustainable services

Use a Developer Certificate of Origin (DCO)-style sign-off rather than copyright assignment as the default contribution model, subject to repository-owner review. Each repository must plainly state which license applies to code, schema, and content, and the contribution agreement must match those declarations.

Permitted service revenue includes:
- hosting and managed operations;
- paid deployment, migration, integration, and data import/export work;
- staff training, onboarding, and support;
- optional service-level agreements, monitoring, backup, and recovery;
- grant-funded implementation and maintenance.

The public application source, pack schema, and published pack data should remain available without a paywall. Avoid per-transaction royalties, exclusivity over public requirements, and license terms that make citizens pay to read public guidance. Any paid service must be transparently optional unless procured by an agency as its chosen delivery channel.

## Alternatives considered

| Alternative | Benefit | Cost / reason not selected as default |
|---|---|---|
| Apache-2.0 for everything | Maximum downstream license flexibility and simple reuse | A hosted operator could keep changes to the application/engine closed; this does not match the goal of returning improvements to the civic commons. |
| AGPL-3.0-or-later for everything, including schema and all content | One strong copyleft boundary | Adds avoidable friction to independent implementations of the interchange schema and to reuse of original explanatory content. Separate those artifacts by role. |
| Non-commercial or “free for government” custom license | May appear to reserve commercial value | It restricts lawful commercial reuse, is not an OSI-approved open-source license, and creates ambiguity for contractors, social enterprises, and community operators. Do not use it for the common core. |
| EUPL-1.2 | Established public-sector-oriented open-source license with official EU publication | Worth considering for an EU-led deployment, but introduces another compatibility decision for the initial South Africa/US-oriented product. Revisit only if a specific adopting authority requires it. |

## Release gates

Before publishing a separate application repository:
1. Place the actual license text and SPDX headers/notices in the correct repositories and paths; do not rely on this ADR as a license grant.
2. Review dependencies, fonts, icons, maps, translations, official content, and sample data for compatible rights.
3. Publish a short reuse guide covering code, schemas, pack annotations, government sources, trademarks, and attribution.
4. Provide an in-product source link and ensure the source corresponds to the deployed build.
5. Have qualified counsel or a responsible license steward review the final arrangement, especially any pack containing third-party text or databases.

## Evidence and reference material

- Open Source Initiative, [GNU Affero General Public License v3](https://opensource.org/license/agpl-3-0): describes the license's network-service source-availability purpose.
- Open Source Initiative, [OSI-approved licenses](https://opensource.org/licenses): open-source licenses allow free use, modification, and sharing.
- Creative Commons, [CC BY 4.0 deed](https://creativecommons.org/licenses/by/4.0/): permits sharing and adaptation, including commercial use, subject to attribution and related conditions.
- Creative Commons, [CC BY 4.0 legal code](https://creativecommons.org/licenses/by/4.0/legalcode.en): licensors must have the rights needed to grant the license; material not covered by the license should be identified.
- European Commission, [EUPL-1.2 text](https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12).

**Decision requested:** approve or amend this as the default licensing policy before creating the public-facing application repository. This document is a technical/product recommendation, not legal advice.
