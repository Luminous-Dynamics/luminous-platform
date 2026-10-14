# Public Source Catalogue V1

This directory defines the machine-readable source inventory and observation contract for the proposed South African public-service operations profile.

**Current state: offline contract and validator implementation under review.** It does not implement connectors, make network requests, cache government records, or authorise use of private municipal data. All source entries remain catalogue_only with ingestion disabled.

## Contents
- schema.json — JSON Schema Draft 2020-12 for catalogue documents.
- observation.schema.json — JSON Schema Draft 2020-12 for observations.
- examples/catalogue.json — initial public-source inventory.
- examples/observation-v1.json — synthetic provenance fixture.
- CONFORMANCE.md — future adapter and contract test requirements.
- rust/ — strict offline validator and CLI, authored in Rust.

## Rust-first / no JavaScript policy
The validator and first-party domain logic are Rust. This slice adds no JavaScript, TypeScript, Node.js, npm runtime, or browser-side business logic. Follow the platform language/runtime policy at docs/LANGUAGE_AND_RUNTIME_POLICY.md. Any future civic UI must preserve essential workflows with JavaScript disabled; prefer server-rendered HTML and ordinary HTTP forms.

## Sources selected
The metadata-only catalogue now includes eight source families:

1. National Treasury Municipal Finance Data: repairs and maintenance, capital acquisition and expenditure measures.
2. National Treasury Transparency Portal / eTender published OCDS data.
3. Johannesburg Water published service standards and public notices.
4. City of Johannesburg IDP, SDBIP and Service-Level Standards index.
5. City/Johannesburg Roads Agency Operation Restore maintenance announcements.
6. Pikitup public-service, waste-collection disruption and illegal-dumping notices.
7. City Comprehensive Integrated Transport Plan public updates.
8. City Food Resilience Policy and urban-agriculture programme announcements.

Research snapshot: 2026-10-14. Every source remains catalogue-only with ingestion disabled. Most city/utility entries are manual-review pointers, not APIs. A URL is a discovery pointer, not proof that retrieval, licensing, completeness, unique incident identity or outcome verification has been tested. For each use, pin the exact notice or policy version rather than treating a mutable webpage as a dataset.

## Critical source semantics
The Municipal Finance API overview links to an HTTPS API base, while the API guide's 'Using the API' section says the endpoint is HTTP. Do not downgrade transport silently. Confirm an HTTPS-compatible endpoint with the publisher before implementing an adapter; otherwise do not connect. The same guide states that returned null and zero can represent unfilled municipal values and says null should be treated as zero, while a query with no returned row means that data was not supplied to Treasury in an acceptable form. If that source-specific interpretation is applied, retain raw null and record an explicit, versioned normalization. Never convert a missing row into zero.

The eTender Transparency Portal documents CC BY 4.0 for published data, but states that the portal covers only a subset of eTender data shared with National Treasury. Missing records cannot support conclusions about non-existence. Anomaly flags are review leads, not accusations.

## Validation
The Rust validator enforces strict field sets, unique source IDs, HTTPS URL syntax, catalogue-only ingestion, timestamp shape, observation-state consistency, explicit normalization identity and source snapshot digest requirements.

Run from the repository root:

    cargo test --manifest-path contracts/public-source-catalogue-v1/rust/Cargo.toml --all-targets
    cargo run --manifest-path contracts/public-source-catalogue-v1/rust/Cargo.toml --bin validate-public-source -- catalogue contracts/public-source-catalogue-v1/examples/catalogue.json
    cargo run --manifest-path contracts/public-source-catalogue-v1/rust/Cargo.toml --bin validate-public-source -- observation contracts/public-source-catalogue-v1/examples/observation-v1.json

These commands describe the intended workflow. They have not yet been reported as executed on an exact GitHub commit in this PR. The Rust implementation is a strict domain validator, not a general JSON Schema Draft 2020-12 implementation.
