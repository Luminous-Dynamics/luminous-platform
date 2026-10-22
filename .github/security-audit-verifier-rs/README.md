# Rust verifier candidate

This crate is a staged Rust port of the independent security-audit verifier.

## Status

**Candidate only. It is not wired into the privileged `workflow_run` receiver.**
The Python verifier remains the current implementation until the Rust candidate has
a committed `Cargo.lock`, observed hosted `cargo test` results, a real-run artifact
qualification, and a reviewed cutover. Do not interpret source presence or a queued
workflow as validation.

## Intended acceptance gates

1. Commit a reviewed lockfile produced by the pinned toolchain.
2. Observe `cargo test --locked` pass on the exact candidate commit.
3. Qualify clean, finding-bearing, malformed, stale-run, cross-PR, digest-mismatch,
   duplicate-key, and unsafe-ZIP cases.
4. Compare Rust results with the current verifier against the same run/artifact fixtures.
5. Only then switch the base-branch receiver and replicate the exact approved crate
   to consumer repositories.

Direct dependencies are exact-version pinned; a lockfile is still required to pin the
full transitive graph. The implementation should be reviewed for correctness before use
as a security authorization boundary.
