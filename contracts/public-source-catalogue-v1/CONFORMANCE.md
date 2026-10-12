# Public Source Catalogue V1 — Conformance Requirements

## Required ingestion safeguards

- Catalogue entries are metadata, not grants of access. All entries are `catalogue_only` with ingestion disabled.
- Before enabling a connector, record the intended dataset, exact publisher-controlled endpoint, source terms/licence, legal purpose, retrieval budget, allowed fields and retention policy.
- HTTPS is mandatory for transport. Never follow a downgrade or arbitrary redirect to HTTP. Validate redirects against an allowlist of publisher-controlled hosts.
- Use GET requests only during public-data discovery. No write method is permitted for this profile.
- Set explicit timeouts, maximum response bytes, redirect limits, rate limits and page budgets. Honour publisher limits. Surface source-health errors rather than silently returning partial data as complete.
- Record retrieval timestamp, source URL, HTTP status, content type, content digest, parser/normalizer revision, source period and pagination metadata where available.
- Do not log credentials, personal information or unrestricted raw response bodies in operational logs.
- Do not infer municipal endorsement, accuracy or operational state from a public data source.

## Value semantics

Preserve these source states independently:
- numerical zero reported as zero;
- `null` reported as null;
- no observation / row omitted;
- malformed or type-invalid value;
- not yet retrieved;
- stale source observation;
- source-reported provisional or unverified period;
- a normalized value produced by an explicit, named and versioned transformation.

Do not rewrite raw values. A derived interpretation must retain the raw value and identify the rule, rule version and reason. In particular, an omitted Treasury fact must not be synthesised as zero; source documentation says missing query results can mean a submission was not supplied in an acceptable form.

## Identity, joins and analysis

- Retain source identifiers such as procurement OCIDs without substitution.
- Fuzzy matches may create review candidates but cannot silently merge official entities, assets, buyers, suppliers or cases.
- Aggregate finance data cannot prove asset-level repair, contractor performance, payment status or misconduct.
- Procurement anomalies remain unverified signals for authorised human review.
- Person-level risk scoring, automated adverse decisions and automated accusations are out of scope.

## Future conformance tests

1. Reject unknown fields, invalid URLs, HTTP source URLs, invalid enum values, empty limitation lists and any enabled ingestion flag.
2. Reject duplicate `source_id` values in a catalogue.
3. Reject HTTP downgrade and redirects to unapproved hosts.
4. Prove pagination is complete or return an explicit incomplete/error result.
5. Oversized responses, malformed JSON, timeouts, rate limits and changed schemas fail visibly rather than being reported as a complete dataset.
6. Fixtures distinguish zero, null, omitted rows, invalid values, stale observations and provisional periods.
7. Reprocessing an identical source snapshot is idempotent; changed content yields a new observation linked to the old one.
8. Missing procurement records do not trigger a finding that a procurement did not exist.
9. Disabling a connector or AI leaves manual government channels usable.

These are requirements for a future implementation. They are not test results.
