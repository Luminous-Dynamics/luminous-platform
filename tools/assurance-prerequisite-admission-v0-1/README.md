# APAC v0.1 independent Rust reference evaluator

This dependency-free crate is a second, typed implementation of the synthetic admission semantics documented in [WEB4_ASSURANCE_PREREQUISITE_ADMISSION_V0_1.md](../../docs/security/WEB4_ASSURANCE_PREREQUISITE_ADMISSION_V0_1.md).

It evaluates already-normalized request, policy, prerequisite, owner-revalidation and authority facts. Its tests focus on exact request/set/snapshot binding, same-subject claim correspondence, predecessor-linked qualification replacement, dependency completeness, lifecycle invalidation and fail-closed authority composition.

The Rust evaluator is deliberately separate from the Python JSON Schema/fixture checker. It does not parse the JSON corpus, authenticate evidence, verify canonical digests or signatures, observe participant effects, enforce access, prove Mycelix/WALP integration, or prove distributed atomicity. Agreement between the two authored implementations is a useful differential check, not a cryptographic proof. CI must run both on the exact PR head before either is described as passing.
