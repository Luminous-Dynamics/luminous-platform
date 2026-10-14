# Civic Record Crosswalk V1

**Status:** Static semantic crosswalk for review; no runtime transform and no shared canonical case schema.
**Snapshot date:** 2026-10-14
**Scope:** Public repositories only.

## Purpose

This contract records how selected public artifacts from Luminous Platform, Mycelix, Sol-Atlas and Symthaea relate. It is a mapping register, not a replacement data model. It does not implement adapters, merge records, make network requests, or authorise actions.

The first audited tree showed that the Luminous Platform public-service RFC refers to Work Case V1 and Operations Event V1, but no corresponding schema or source artifact was present in the audited PR #34 branch tree at commit 93a97cb16dfea73660b6dc868fbba3b5f4d9c06c. Those dependencies are explicitly marked unavailable in the crosswalk. They must not be guessed into existence or silently replaced by Sol-Atlas's public civic-case model.

## Files

- crosswalk.schema.json: JSON Schema Draft 2020-12 for the mapping register.
- crosswalk.json: exact-source snapshot, field-level relationships, prohibited lossy conversions and dependency gaps.
- rust/: Rust 1.96 strict deserializer and semantic checker with regression tests.
- The checker is intentionally not a general JSON Schema engine and does not execute transformations.

## Current source snapshots

The mapping pins exact commits and Git blob IDs for the artifacts it inspected. These snapshots are audit anchors, not moving references to current branch heads. When a source PR changes, refresh the exact commit and blob IDs, re-review the affected rule, and rerun this contract's qualification before reusing the mapping.

The Mycelix service journey record is version 0.3.0. The applicability schema does not define a top-level schema version; records instead carry profile_version. Sol-Atlas civic case wire types are Serde-derived Rust types using schema_version 1; no separate JSON Schema file was present at the reviewed path. Symthaea's referenced file is an evaluation corpus, not a general runtime output schema.

## Design invariants

1. Keep operational work state, service-journey state, and public geographic case state separate.
2. Preserve identifiers in their own namespaces. Similar IDs, names or places do not prove the records refer to one real-world incident.
3. Do not automatically translate the Mycelix and Sol-Atlas state machines; they have different states and verification requirements.
4. Evidence independence and evidence review status are separate dimensions.
5. A source snapshot digest is not automatically the digest of evidence bytes held by another contract.
6. Published standard applicability is not authorisation to act.
7. Symthaea evaluation expectations do not form an executable action command or runtime output schema.
8. Publish Sol-Atlas cases only through its explicit sanitising public_projection boundary. Never serialise the internal CivicServiceCase as a public API response.
9. Do not map to Work Case or Operations Event fields until those actual versioned contracts are available at a pinned revision.

## Validate offline

From the repository root:

    cargo +1.96.0 test --manifest-path contracts/civic-record-crosswalk-v1/rust/Cargo.toml --all-targets
    cargo +1.96.0 run --manifest-path contracts/civic-record-crosswalk-v1/rust/Cargo.toml --bin validate-civic-crosswalk -- validate contracts/civic-record-crosswalk-v1/crosswalk.json

This verifies the mapping register's structure and selected safety invariants only. It does not qualify interoperability, live data ingestion, authority, legal applicability, production deployment, or any municipal integration.

## Before adding an adapter

Require both source and target contracts at exact revisions; a field-level semantic review; source terms and data-use permission; an explicit mapping for missing, stale, conflicting and corrected values; a privacy projection; synthetic positive and negative controls; an independent checker; and exact-head completed CI. A blocked dependency remains blocked rather than being emulated by a vaguely similar type.
