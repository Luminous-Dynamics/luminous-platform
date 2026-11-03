# EvoITOPS harness-evolution stream

This package validates a staged synthetic operational harness evolution plan.

## Run

From this directory:

    cargo test
    cargo run --release

The linter validates Draft 2020-12 structure, then checks invariants that
ordinary schema validation cannot express:

- contiguous stage indices and exact parent-stage linkage;
- stable retention-task manifest across every stage;
- no accidental reuse of adaptation or hidden held-out task manifests;
- authority-policy digest transitions must agree with the declared
  unchanged/narrowed/expanded/revoked state;
- every tool addition, schema change, authority change, deprecation and
  removal has a matching event with exact from/to versions;
- tool schema changes increment the version exactly once;
- no schema or authority change may hide under an unchanged capability version;
- newly discovered tools default to no authority; a grant requires a
  separately declared explicit grant binding and expansion manifest;
- removal/deprecation changes must update the tool-manifest digest;
- arm roles are unique, distinct system profiles are pinned, controls are
  fixed, and all experiment arms remain synthetic-only;
- the continual-adaptation and frozen-model/evolving-harness arms have
  consistent model-weight and adaptation policies.

## Example

The four-stage stream covers:
1. baseline operations with read-only and a narrowly granted synthetic restart;
2. addition of a read-only deployment-diff tool, with no implicit grant;
3. restart API schema v2 plus revocation of old grants and deprecation of
   the legacy restart tool;
4. removal of the deprecated legacy capability.

Retention tasks and the authority policy remain fixed until explicit
transitions; every stage carries immutable content digests. The stream remains
`candidate_not_frozen` and every digest is illustrative. It is not an
executed campaign or a production connector manifest.

The test sources include negative controls for implicit authority, stale
schema versions, changed retention tasks, missing removal events, inconsistent
authority digests and broken stage lineage.

No Rust build, schema validator, or agent campaign has been executed in the
authoring environment.
