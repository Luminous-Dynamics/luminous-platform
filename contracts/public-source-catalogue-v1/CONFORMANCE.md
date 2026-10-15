# Public Source Catalogue V1 — Conformance Requirements

## Implementation language
Production/domain logic is Rust-first. No handwritten JavaScript/TypeScript, Node.js backend, npm-based business logic or browser-side policy/authority logic is permitted for this civic profile. The civic core UI must remain usable with JavaScript disabled, using server-rendered HTML and ordinary HTTP forms. WebAssembly is an optional enhancement only; server-side validation remains authoritative.

## Required ingestion safeguards
- Catalogue entries are metadata, not grants of access. All entries are catalogue_only with ingestion disabled.
- Before enabling a connector, record the intended dataset, exact publisher-controlled endpoint, source terms/licence, legal purpose, retrieval budget, allowed fields and retention policy.
- HTTPS is mandatory. Never follow a downgrade or arbitrary redirect to HTTP. Validate redirects against an allowlist of publisher-controlled hosts. The current Municipal Finance API docs contradict themselves by linking an HTTPS base while the usage section names HTTP: adapter implementation is blocked until HTTPS is confirmed by the publisher.
- Use GET requests only during public-data discovery. No write method is permitted for this profile.
- Set explicit timeouts, maximum response bytes, redirect limits, rate limits and page budgets. Honour publisher limits. Surface source-health errors rather than silently returning partial data as complete.
- Record retrieval timestamp, source URL, HTTP status, content type, content digest, parser/normalizer revision, source period and pagination metadata where available.
- Do not log credentials, personal information or unrestricted raw response bodies in operational logs.
- Do not infer municipal endorsement, accuracy or operational state from a public data source.

## Value semantics
Validate observations against observation.schema.json and Rust domain invariants. Preserve these states independently: reported numerical zero; reported null; no observation/row omitted; malformed or type-invalid value; not yet retrieved; stale observation; source-reported provisional/unverified period; and normalized value created by an explicit, named and versioned transformation.

Do not rewrite raw values. A derived interpretation must retain the raw value and identify the rule, rule version and reason. The Municipal Finance API docs say returned null should be treated as zero when a municipality did not fill in a value. If implemented, preserve the raw null and apply a named, versioned, source-specific normalization (for example, treasury-null-as-zero version 1). Never apply that conversion to an omitted row, missing query result, invalid or stale value, or unrelated source without separate evidence.

## Identity, joins and analysis
- Retain source identifiers such as procurement OCIDs without substitution.
- Fuzzy matches may create review candidates but cannot silently merge official entities, assets, buyers, suppliers or cases.
- Aggregate finance data cannot prove asset-level repair, contractor performance, payment status or misconduct.
- Procurement anomalies remain unverified signals for authorised human review.
- Person-level risk scoring, automated adverse decisions and automated accusations are out of scope.

## Future conformance tests
1. Reject unknown fields, invalid URLs, HTTP source URLs, invalid enum values, empty limitation lists and enabled ingestion.
2. Reject duplicate source IDs.
3. Reject HTTP downgrade and redirects to unapproved hosts in future adapters.
4. Prove pagination is complete or return an explicit incomplete/error result.
5. Oversized responses, malformed JSON, timeouts, rate limits and changed schemas fail visibly rather than being reported complete.
6. Fixtures distinguish zero, null, omitted rows, invalid values, stale observations and provisional periods.
7. Reprocessing an identical source snapshot is idempotent; changed content yields a new observation linked to the old one.
8. Missing procurement records do not trigger a finding that a procurement did not exist.
9. Disabling a connector or AI leaves manual government channels usable.
10. Core civic workflow smoke tests succeed with JavaScript disabled.

These are requirements for a future adapter and UI implementation, not all current test results. Rust tests authored in this PR remain subject to execution on the exact PR head.


## City project statement fixtures

Three dated City newsroom announcements are represented as synthetic, unretrieved observations: the Johannesburg Water depot close-out report (12 October 2026), Braamfontein traffic-signal and accessibility upgrades (2 October 2026), and the Malvern Clinic refurbishment announcement (14 October 2026). These fixtures carry the article publication date but keep capture timestamp and content digest null. The fixtures explicitly distinguish publisher-reported progress or reopening from commissioning, live service status, current operational availability and independently measured outcomes. They are not a live feed or independent audit.
