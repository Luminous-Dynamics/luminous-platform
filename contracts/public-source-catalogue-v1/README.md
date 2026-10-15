# Public Source Catalogue V1

This directory defines the machine-readable source inventory and observation contract for the proposed South African public-service operations profile.

**Current state: offline contract and validator implementation under review.** It does not implement connectors, make network requests, cache government records, or authorise use of private municipal data. All source entries remain catalogue_only with ingestion disabled.

## Contents
- schema.json — JSON Schema Draft 2020-12 for catalogue documents.
- observation.schema.json — JSON Schema Draft 2020-12 for observations.
- examples/catalogue.json — initial public-source inventory.
- examples/observation-v1.json — synthetic Treasury null/zero provenance fixture.
- examples/johannesburg-power-water-statement-v1.synthetic.json — synthetic source-claim fixture distinguishing a published cross-utility statement from live status, incident-level causality and verified outcomes.
- Three dated City project fixtures — a water depot close-out, Braamfontein traffic-signal/accessibility upgrades, and Malvern Clinic refurbishment; each preserves publisher claims without asserting live status or independently verified outcomes.
- CONFORMANCE.md — future adapter and contract test requirements.
- rust/ — strict offline validator and CLI, authored in Rust.

## Rust-first / no JavaScript policy
The validator and first-party domain logic are Rust. This slice adds no JavaScript, TypeScript, Node.js, npm runtime, or browser-side business logic. Follow the platform language/runtime policy at docs/LANGUAGE_AND_RUNTIME_POLICY.md. Any future civic UI must preserve essential workflows with JavaScript disabled; prefer server-rendered HTML and ordinary HTTP forms.

## Sources selected
The metadata-only catalogue now includes eleven source families:

1. National Treasury Municipal Finance Data: repairs and maintenance, capital acquisition and expenditure measures.
2. National Treasury Transparency Portal / eTender published OCDS data.
3. Johannesburg Water published service standards and public notices.
4. City of Johannesburg IDP, SDBIP and Service-Level Standards index.
5. City Power public service, fault-logging and dated interruption/restoration notices.
6. City/Johannesburg Roads Agency Operation Restore maintenance announcements.
7. Pikitup public-service, waste-collection disruption and illegal-dumping notices.
8. City Comprehensive Integrated Transport Plan public updates.
9. City Food Resilience Policy and urban-agriculture programme announcements.
10. South African Government's dated joint statement on electricity reliability and water-supply disruptions.
11. City of Johannesburg's dated newsroom announcements about infrastructure milestones, commissioning/close-out, accessibility and public-facility refurbishment.

Research snapshot: 2026-10-15. Every source remains catalogue-only with ingestion disabled. Most city/utility entries are manual-review pointers, not APIs. A URL is a discovery pointer, not proof that retrieval, licensing, completeness, unique incident identity or outcome verification has been tested. For each use, pin the exact notice or policy version rather than treating a mutable webpage as a dataset.

## Critical source semantics
The Municipal Finance API overview links to an HTTPS API base, while the API guide's 'Using the API' section says the endpoint is HTTP. Do not downgrade transport silently. Confirm an HTTPS-compatible endpoint with the publisher before implementing an adapter; otherwise do not connect. The same guide states that returned null and zero can represent unfilled municipal values and says null should be treated as zero, while a query with no returned row means that data was not supplied to Treasury in an acceptable form. If that source-specific interpretation is applied, retain raw null and record an explicit, versioned normalization. Never convert a missing row into zero.

The eTender Transparency Portal documents CC BY 4.0 for published data, but states that the portal covers only a subset of eTender data shared with National Treasury. Missing records cannot support conclusions about non-existence. Anomaly flags are review leads, not accusations.

## Validation
The Rust validator enforces strict field sets, unique source IDs, HTTPS URL syntax, catalogue-only ingestion, observation-state consistency, explicit normalization identity and source snapshot digest requirements. For `fixture_only: true`, `retrieved_at` and `source_snapshot_sha256` must both be explicit JSON `null`; synthetic test data must not fabricate a retrieval timestamp. Captured observations (`fixture_only: false`) require an RFC3339 timestamp with an explicit timezone and a valid `sha256:<64 lowercase hex digits>` source snapshot digest. The Rust contract rejects both a timestamp on a fixture and missing retrieval provenance on a non-fixture record, and the JSON Schema carries the same conditional rule. Conformance tests pin all thirteen catalogue entries and verify that the synthetic Johannesburg power/water statement plus three dated City project fixtures are not promoted from publisher claims into live service status or independently verified outcomes.

Run from the repository root:

    cargo test --manifest-path contracts/public-source-catalogue-v1/rust/Cargo.toml --all-targets
    cargo run --manifest-path contracts/public-source-catalogue-v1/rust/Cargo.toml --bin validate-public-source -- catalogue contracts/public-source-catalogue-v1/examples/catalogue.json
    cargo run --manifest-path contracts/public-source-catalogue-v1/rust/Cargo.toml --bin validate-public-source -- observation contracts/public-source-catalogue-v1/examples/observation-v1.json
    cargo run --manifest-path contracts/public-source-catalogue-v1/rust/Cargo.toml --bin validate-public-source -- observation contracts/public-source-catalogue-v1/examples/johannesburg-power-water-statement-v1.synthetic.json

These commands describe the intended workflow. They have not yet been reported as executed on an exact GitHub commit in this PR. The Rust implementation is a strict domain validator, not a general JSON Schema Draft 2020-12 implementation.

## Water-service clock semantics and national standards

The catalogue keeps Johannesburg Water's `Services` page separate from its [Daily Water Notices page](https://www.johannesburgwater.co.za/daily-water-notices/). On the 2026-10-15 review, the Daily Water Notices summary listed sewer 48 hours, water outage 24 hours, leaking meter 72 hours, burst meter 24 hours, water leak 48 hours and major burst pipe 48 hours. The summary has no visible publication/effective date; these values are a dated page observation and must be rechecked before reuse. The `Services` page separately says it endeavours to respond within 12 hours to unplanned supply emergencies. Preserve both as distinct source claims; do not collapse response and category-turnaround measures into a single SLA without source-defined clock start/stop, exclusions and applicability.

The national standards entry is metadata-only. Its linked Gazette PDF identifies Government Gazette No. 52814, Notice 6292, dated 6 June 2025. This identifies the published instrument, but the catalogue does not establish current legal status, effective dates, amendments or applicability; qualified review remains required. Ingestion remains disabled for every source.
