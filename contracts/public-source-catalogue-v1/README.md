# Public Source Catalogue V1

This directory defines a machine-readable source inventory for the proposed South African public-service operations profile.

**Current state: catalogue and contract only.** It does not implement connectors, make network requests, cache government records, or authorise use of private municipal data.

## Contents

- `schema.json` — JSON Schema Draft 2020-12 for the catalogue.
- `examples/catalogue.json` — three official-source entries for initial discovery.
- `examples/observation-v1.json` — synthetic observation fixture showing how provenance and missing-value semantics should be retained.
- `CONFORMANCE.md` — requirements for future implementations and their tests.

## Sources selected

1. National Treasury Municipal Finance Data: repairs and maintenance, capital acquisition and expenditure measures.
2. National Treasury Transparency Portal / eTender published OCDS data.
3. Johannesburg Water published service information, catalogued as reviewed webpages rather than an assumed API.

Official source documentation was reviewed on 2026-10-12. Entries deliberately disable ingestion. A URL is a discovery pointer, not proof that retrieval, licensing, completeness or API compatibility has been tested.

## Critical source semantics

The Municipal Finance API overview links to an HTTPS API base, while the API guide's 'Using the API' section lists an HTTP endpoint. Implementations must not silently downgrade to cleartext HTTP: verify HTTPS with the publisher and fail closed if it cannot be established. The same documentation says null values may represent data not filled in, while an absent query result can mean a submission was not accepted. Do not collapse raw null, numerical zero, absent row, invalid value and stale data into one representation. Source-specific interpretation must be explicit, versioned and tested.

The eTender Transparency Portal documents CC BY 4.0 for published data, but it also explains that the portal covers only a subset of eTender data shared with National Treasury. Missing records cannot support conclusions about non-existence. Anomaly flags are review leads, not accusations.

## Validation

Validate the catalogue against `schema.json` using a Draft 2020-12-compliant JSON Schema validator. Validate that `source_id` is unique across the catalogue; JSON Schema uniqueItems cannot enforce uniqueness of just one property. A future production contract crate should be Rust-first and integrated into the repository's qualified contract test workflow. No runtime test pass is claimed in this documentation change.
