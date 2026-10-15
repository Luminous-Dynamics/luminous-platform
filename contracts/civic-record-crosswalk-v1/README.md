# Civic Record Crosswalk V1

**Status:** Static semantic crosswalk for review; no runtime transform and no shared canonical case schema.
**Snapshot date:** 2026-10-15
**Scope:** Public repositories only.

## Purpose

This contract records how selected public artifacts from Luminous Platform, Mycelix, Sol-Atlas and Symthaea relate. It is a mapping register, not a replacement data model. It does not implement adapters, merge records, make network requests, or authorise actions.

The concrete Work Case V1 and Operations Event V1 schemas remain on separate draft PR branches, not in the PR #34 branch tree. Work Case V1 is pinned from [PR #8](https://github.com/Luminous-Dynamics/luminous-platform/pull/8) at current head `662aaf2bd2a3f873fd8f6fc0eb4181f061043794` (schema blob `6be2f6498630d76287034020d9e89eca76ee2711`); Operations Event V1 is pinned from [PR #4](https://github.com/Luminous-Dynamics/luminous-platform/pull/4) at current head `c43a68adc9543fee67d67debfd7bcf6e1387395d` (schema blob `aa7364e5661125e04cc5e36b12269ec08b611448`). Their exact-head contract workflows #38694337384 and #38692588693 succeeded respectively, so the crosswalk records `narrow_workflow_success` for those artifacts. This is not runtime interoperability or operational qualification: work-state and event adapters remain blocked pending independent cross-contract tests, authority-boundary review, and exact-head adapter qualification.

## Files

- crosswalk.schema.json: JSON Schema Draft 2020-12 for the mapping register.
- crosswalk.json: exact-source snapshot, field-level relationships, qualification states and prohibited lossy conversions.
- `evidence_manifest` bindings: exact Git blob pins for upstream corpus manifests, distinct from the corpus data and from source authenticity.
- rust/: Rust 1.96 strict deserializer and semantic checker with regression tests.
- The checker is intentionally not a general JSON Schema engine and does not execute transformations.

## Current source snapshots

The mapping pins exact commits and Git blob IDs for the artifacts it inspected. These snapshots are audit anchors, not moving references to current branch heads. When a source PR changes, refresh the exact commit and blob IDs, re-review the affected rule, and rerun this contract's qualification before reusing the mapping.

The Platform public-observation schema is pinned at commit `f04951f8f1fed8f0ea1d244824e1d39d8f71089d`, blob `4a50b82a8ed32deeb245e11b7e4d542860a6c2a2`. Its corrected provenance contract requires `retrieved_at=null` and `source_snapshot_sha256=null` for `fixture_only=true`; captured observations require an RFC3339 timestamp and a valid SHA-256 source snapshot digest. Because this change is awaiting exact-head requalification, its crosswalk state is `unqualified_draft`; earlier narrow workflow evidence does not carry forward to the changed semantics.

The Mycelix service-journey **schema** remains version 0.3.0, while its versioned **evaluation corpus** is now v0.6.0 (ten effects-disabled fixtures).
The crosswalk also pins Mycelix's standard-instrument schema, GPSJS survey-quality schema, and v0.5.0 life/civic standards corpus (17 cases). A new `MYCELIX-SYMTHAEA-GPSJS-PRECISION-CORPORA` rule records semantic overlap between Mycelix's structured precision evidence and Symthaea's `SYM-LC-015` reasoning control. This is a `preserve_separately` evaluation mapping, not a runtime adapter: survey response rates are not estimate-specific precision, Table 41 household-prevalence CVs do not apply to the distinct police-reporting indicator, and national point-estimate changes do not establish local causal effects. Mycelix's exact-head workflow is queued, and the updated Symthaea suite has not yet produced a completed exact-head result; these entries remain `unqualified_draft`. The corpus preserves `cross-utility-statement-not-incident-cause`, and v0.6.0 adds a synthetic water-service clock-semantics case. The new case distinguishes the Services page response wording from Daily Water Notices category turnaround values, keeps evidence explicitly synthetic (null retrieval/check timestamps and no source digest), and forbids treating response as restoration or declaring a conflict solely because labels differ. Symthaea's life/civic reasoning corpus is now v0.4.0 with 15 cases, including SYM-LC-015 for survey precision. Its manifest pins Mycelix's standards corpus as MYC-LIFE-CIVIC-APPLICABILITY-001@0.5.0; the exact-head workflow [#38730943742](https://github.com/Luminous-Dynamics/symthaea/actions/runs/38730943742) is queued, so both evaluation inputs remain unqualified. The `MYCELIX-SYMTHAEA-CAUSALITY-CORPORA` and `MYCELIX-SYMTHAEA-GPSJS-PRECISION-CORPORA` mappings explicitly preserve the two fixture shapes separately rather than pretending they are interchangeable runtime records. Both corpus pins remain `unqualified_draft` until their exact-head tests complete. The Mycelix applicability schema does not define a top-level schema version; records instead carry `profile_version`. Sol-Atlas civic case wire types are Serde-derived Rust types using schema_version 1; no separate JSON Schema file was present at the reviewed path. Neither evaluation corpus is a general runtime output schema.

## Design invariants

1. Keep operational work state, service-journey state, and public geographic case state separate.
2. Preserve identifiers in their own namespaces. Similar IDs, names or places do not prove the records refer to one real-world incident.
3. Do not automatically translate the Mycelix and Sol-Atlas state machines; they have different states and verification requirements.
4. Evidence independence and evidence review status are separate dimensions.
5. A source snapshot digest is not automatically the digest of evidence bytes held by another contract.
6. Published standard applicability is not authorisation to act.
7. Symthaea evaluation expectations do not form an executable action command or runtime output schema.
8. Publish Sol-Atlas cases only through its explicit sanitising public_projection boundary. Never serialise the internal CivicServiceCase as a public API response.
9. Work Case and Operations Event artifacts are pinned from separate draft PRs, but remain unqualified; do not implement adapters until exact-head tests and stack ancestry are reconciled.
10. Mycelix's service-journey corpus v0.6.0 and Symthaea's life/civic corpus v0.2.0 share a causal/currentness negative control but remain separate evaluation formats. A historical public statement and a synthetic fixture must not be mapped to incident-level causality, live service state, restoration or action authority.

## Validate offline

From the repository root:

    cargo +1.96.0 test --manifest-path contracts/civic-record-crosswalk-v1/rust/Cargo.toml --all-targets
    cargo +1.96.0 run --manifest-path contracts/civic-record-crosswalk-v1/rust/Cargo.toml --bin validate-civic-crosswalk -- validate contracts/civic-record-crosswalk-v1/crosswalk.json

This verifies the mapping register's structure and selected safety invariants only. It does not qualify interoperability, live data ingestion, authority, legal applicability, production deployment, or any municipal integration.

## Before adding an adapter

Require both source and target contracts at exact revisions; a field-level semantic review; source terms and data-use permission; an explicit mapping for missing, stale, conflicting and corrected values; a privacy projection; synthetic positive and negative controls; an independent checker; and exact-head completed CI. A blocked dependency remains blocked rather than being emulated by a vaguely similar type.


## Exact-head snapshot notes — 15 October 2026

All eight contract entries are pinned to the latest reviewed public PR heads and matching content blobs on this date. Qualification states remain `unqualified_draft` until the relevant checks complete on exact heads and cross-contract semantics are reviewed. Sol-Atlas PR #109's current Check workflow succeeded on its exact head, but this does not qualify a runtime adapter or any municipal connection. The Platform PR #34 source/crosswalk contract workflow and Symthaea/Mycelix life-civic workflows are queued; the Platform Operations Event/Holochain workflow was re-run after narrow validator/test fixes and is queued. Do not infer success from queued, skipped, cancelled, absent or predecessor-head results.


## Exact-head qualification refresh — 15 October 2026

This refresh supersedes older status statements above where source PRs have advanced. Symthaea PR #7443 has narrow exact-head success for its life/civic corpus, crime-prevention reference and synthetic evaluator; this establishes corpus/evaluator checks only, not general model capability. Luminous Platform PR #4 (Operations Event) and #8 (Work Case) also have successful named contract workflows, and their crosswalk entries now record `narrow_workflow_success`. Runtime mappings remain blocked until independent integration tests and authority-boundary review pass. Mycelix PR #5019's life/civic workflow remains queued, so its records remain `unqualified_draft`. The crosswalk is a static mapping register only; it does not convert, join, or authorise runtime records.


## Verify the pinned source artifacts independently

The Rust contract validator remains offline and does not make network requests. The separate `verify-pinned-source-blobs.sh` audit fetches only the public files named by `availability: present_at_snapshot` from their exact full Git commit SHAs over HTTPS, computes each Git blob SHA-1 with `git hash-object`, and compares it with the crosswalk. It never checks out or executes source-repository code, never reads private municipal systems, and is not a data-ingestion adapter.

Run from the repository root:

    bash contracts/civic-record-crosswalk-v1/verify-pinned-source-blobs.sh contracts/civic-record-crosswalk-v1/crosswalk.json

A network error, missing path, malformed locator, or blob mismatch fails closed. This proves that the downloaded file bytes match the recorded Git blob at the requested commit; it does not prove that the source claim is true, that a repository branch still points to the commit, that the source is licensed for republication, or that a runtime mapping is semantically or operationally qualified. The CI step must pass on the exact crosswalk head before its pins can be described as independently reverified in that run.


### Corpus and manifest pins

The static crosswalk pins each life/civic evaluation corpus separately from its evidence manifest. The manifests have their own Git blob identity and are linked by regression tests to the same source commit and version as the corresponding corpus. A matching Git blob helps detect repository drift; it is not a signature, a proof that external data is authentic, or evidence that a suite ran successfully. The Mycelix service, Mycelix standards, and Symthaea life/civic evaluation inputs remain `unqualified_draft` until their own exact-head workflows complete.
