# Civic Record Crosswalk V1

**Status:** Static semantic crosswalk for review; no runtime transform and no shared canonical case schema.
**Snapshot date:** 2026-10-15
**Scope:** Public repositories only.

## Purpose

This contract records how selected public artifacts from Luminous Platform, Mycelix, Sol-Atlas and Symthaea relate. It is a mapping register, not a replacement data model. It does not implement adapters, merge records, make network requests, or authorise actions.

The concrete Work Case V1 and Operations Event V1 schemas remain on separate draft PR branches, not in the PR #34 branch tree. Work Case V1 is pinned from [PR #8](https://github.com/Luminous-Dynamics/luminous-platform/pull/8) at current head 7ec5588d9844b10006b3264e34f30bd4aeba48ec (schema blob 6be2f6498630d76287034020d9e89eca76ee2711); Operations Event V1 is pinned from [PR #4](https://github.com/Luminous-Dynamics/luminous-platform/pull/4) at current head c43a68adc9543fee67d67debfd7bcf6e1387395d (schema blob aa7364e5661125e04cc5e36b12269ec08b611448). Both remain `unqualified_draft`. A previous PR #4 run accepted an invalid `observedat` string because the format checker dependency was not installed and exposed a stale issuer-role assertion. Those defects have been patched on the current PR #4 head; its exact-head workflow #38692588693 and the reconciled PR #8 workflow #38693004364 are queued. The ancestry has been reconciled but the contracts are not yet qualified, so adapters remain blocked until the current source revisions pass.

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


## Exact-head snapshot notes — 15 October 2026

All eight contract entries are pinned to the latest reviewed public PR heads and matching content blobs on this date. Qualification states remain `unqualified_draft` until the relevant checks complete on exact heads and cross-contract semantics are reviewed. Sol-Atlas PR #109's current Check workflow succeeded on its exact head, but this does not qualify a runtime adapter or any municipal connection. The Platform PR #34 source/crosswalk contract workflow and Symthaea/Mycelix life-civic workflows are queued; the Platform Operations Event/Holochain workflow was re-run after narrow validator/test fixes and is queued. Do not infer success from queued, skipped, cancelled, absent or predecessor-head results.
