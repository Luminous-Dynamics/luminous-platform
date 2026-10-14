# Civic Record Crosswalk V1

**Status:** Static semantic crosswalk for review; no runtime transform and no shared canonical case schema.
**Snapshot date:** 2026-10-14
**Scope:** Public repositories only.

## Purpose

This contract records how selected public artifacts from Luminous Platform, Mycelix, Sol-Atlas and Symthaea relate. It is a mapping register, not a replacement data model. It does not implement adapters, merge records, make network requests, or authorise actions.

The concrete Work Case V1 and Operations Event V1 schemas were absent from the audited PR #34 branch tree at commit 93a97cb16dfea73660b6dc868fbba3b5f4d9c06c. Further public-repository inspection found them on separate draft PRs: Work Case V1 in [PR #8](https://github.com/Luminous-Dynamics/luminous-platform/pull/8) at exact head 384a60d9668e77c40561b46d5511593ea5198c23 (schema blob 6be2f6498630d76287034020d9e89eca76ee2711), and Operations Event V1 in [PR #4](https://github.com/Luminous-Dynamics/luminous-platform/pull/4) at exact head b4b32d90742e9a218e4d6919f15d74d904f5beca (schema blob aa7364e5661125e04cc5e36b12269ec08b611448). They are pinned separately in the crosswalk as present but `unqualified_draft`. PR #4's exact-head qualification workflow failed: the Rust test crate stopped on an SQLx 0.9 dynamic-query compile error, and its non-production Python research-model job failed to import a test file because a test function definition was missing a colon. No workflow run was returned for PR #8's pinned head. The crosswalk therefore blocks adapters involving those contracts until the exact source revisions are qualified. This is not a claim that the artifacts are absent from every branch.

## Files

- crosswalk.schema.json: JSON Schema Draft 2020-12 for the mapping register.
- crosswalk.json: exact-source snapshot, field-level relationships, qualification states and prohibited lossy conversions.
- rust/: Rust 1.96 strict deserializer and semantic checker with regression tests.
- The checker is intentionally not a general JSON Schema engine and does not execute transformations.

## Current source snapshots

The mapping pins exact commits and Git blob IDs for the artifacts it inspected. These snapshots are audit anchors, not moving references to current branch heads. When a source PR changes, refresh the exact commit and blob IDs, re-review the affected rule, and rerun this contract's qualification before reusing the mapping.

The Platform public-observation schema is pinned at commit `9316cc917b13a395a62da4b719bcdb76f0ef224a`, blob `4a50b82a8ed32deeb245e11b7e4d542860a6c2a2`. Its corrected provenance contract requires `retrieved_at=null` and `source_snapshot_sha256=null` for `fixture_only=true`; captured observations require an RFC3339 timestamp and a valid SHA-256 source snapshot digest. Because this change is awaiting exact-head requalification, its crosswalk state is `unqualified_draft`; earlier narrow workflow evidence does not carry forward to the changed semantics.

The Mycelix service-journey **schema** is version 0.3.0, while its versioned **evaluation corpus** is now v0.4.0 (six effects-disabled fixtures). The corpus adds `cross-utility-statement-not-incident-cause`, which treats the South African Government's 10 September 2026 power/water statement as synthetic contextual evidence and keeps route/outcome unknown; it forbids inference of incident-level causality or current restoration. Symthaea's corresponding life/civic evaluation corpus is v0.2.0 with 11 scenarios, including SYM-LC-011 for the same negative boundary. The new `MYCELIX-SYMTHAEA-CAUSALITY-CORPORA` mapping explicitly preserves the two fixture shapes separately rather than pretending they are interchangeable runtime records. Both corpus pins remain `unqualified_draft` until their exact-head tests complete. The Mycelix applicability schema does not define a top-level schema version; records instead carry `profile_version`. Sol-Atlas civic case wire types are Serde-derived Rust types using schema_version 1; no separate JSON Schema file was present at the reviewed path. Neither evaluation corpus is a general runtime output schema.

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
10. Mycelix's service-journey corpus v0.4.0 and Symthaea's life/civic corpus v0.2.0 share a causal/currentness negative control but remain separate evaluation formats. A historical public statement and a synthetic fixture must not be mapped to incident-level causality, live service state, restoration or action authority.

## Validate offline

From the repository root:

    cargo +1.96.0 test --manifest-path contracts/civic-record-crosswalk-v1/rust/Cargo.toml --all-targets
    cargo +1.96.0 run --manifest-path contracts/civic-record-crosswalk-v1/rust/Cargo.toml --bin validate-civic-crosswalk -- validate contracts/civic-record-crosswalk-v1/crosswalk.json

This verifies the mapping register's structure and selected safety invariants only. It does not qualify interoperability, live data ingestion, authority, legal applicability, production deployment, or any municipal integration.

## Before adding an adapter

Require both source and target contracts at exact revisions; a field-level semantic review; source terms and data-use permission; an explicit mapping for missing, stale, conflicting and corrected values; a privacy projection; synthetic positive and negative controls; an independent checker; and exact-head completed CI. A blocked dependency remains blocked rather than being emulated by a vaguely similar type.
