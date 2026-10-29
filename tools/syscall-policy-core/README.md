# Fail-Closed Linux Syscall Policy Core

A dependency-free Rust decision core for a future Linux syscall-sandbox adapter. This is executable policy logic, not a documentation-only benchmark change.

## What it does

- Maps a closed set of normalized syscall families and decoded argument facts to required operations.
- Requires an authenticated, fresh, non-revoked, digest-pinned policy and an explicit rollback floor.
- Binds each capability to one target, one subject-image digest, one exact resource identity, a bounded time window, and an explicit set of operations.
- Bundles target identity, subject-image digest, syscall/argument facts, and resource identity into one atomic observed-operation fact, preventing callers from accidentally mixing facts from different events.
- Requires one capability to cover every operation implied by a call; permissions from separate capabilities are never unioned into a broader grant.
- Denies unknown syscalls, unknown argument semantics, missing or contradictory facts, stale/revoked capabilities, scope mismatches, W+X memory requests, and operations that require more than one resource binding in v1.
- Emits a redacted decision receipt with opaque digests instead of filesystem path or payload data.

## Run

    cargo fmt --manifest-path tools/syscall-policy-core/Cargo.toml -- --check
    cargo test --locked --manifest-path tools/syscall-policy-core/Cargo.toml
    cargo clippy --locked --manifest-path tools/syscall-policy-core/Cargo.toml -- -D warnings
    cargo run --locked --manifest-path tools/syscall-policy-core/Cargo.toml

The crate uses only the Rust standard library. The CI workflow uses Rust 1.96.0, pinned actions, read-only repository permissions, locked dependencies, and a fixed source path.

## Decision order

1. Validate policy structure, the pinned policy digest, and the authenticated rollback floor.
2. Reject a missing/reversed clock, unauthenticated policy, unknown revocation, stale policy, or epoch rollback.
3. Require one verified observed-operation bundle containing target, subject-image, resource, and syscall/argument facts. Missing, unverified, or contradictory bundles are rejected.
4. Decode the syscall family into its full required operation set. Unsupported flags and unknown semantics are rejected; W+X memory is always denied.
5. Require resource kind compatibility.
6. Match one capability with exact target, subject digest, stable resource digest, all required operations, active status, and a valid time window.
7. Return AllowReceipt only if every check succeeds; otherwise return a stable deny reason.

## Scope and trust boundaries

This crate **does not yet intercept or block a real Linux syscall**. It does not install a seccomp filter, attach a Landlock ruleset, authenticate the policy signature, resolve file paths, or prove that adapter-supplied Fact::verified values are true. A trusted adapter must establish those facts from the actual syscall number and arguments, target, process identity, resolved resource, signature verifier, clock, and revocation/rollback source. The policy digest must be computed from and authenticate the exact policy contents before the adapter marks authentication verified.

The adapter must apply the returned decision at a kernel-enforced boundary, not merely log it. For Linux, seccomp is the syscall-number/argument filtering mechanism; Landlock constrains access to kernel objects and filesystem/network resources. They are complementary layers, not interchangeable claims:

- [Linux seccomp filter documentation](https://docs.kernel.org/userspace-api/seccomp_filter.html)
- [Linux Landlock userspace API documentation](https://docs.kernel.org/userspace-api/landlock.html)

The v1 core deliberately denies rename and process-spawn syscalls because safe authorization needs multi-resource source/destination binding or an explicit child-process inheritance model. It also rejects unknown syscall arguments rather than guessing. The only supported claims are decision-core conformance until a real adapter and an independent OS-side oracle execute on a disposable Linux/NixOS host.

## Tests

The 30 authored unit tests cover exact-scope allow, atomic observed-operation facts, read/write separation, non-composition of separate grants, compound open flags, create/truncate rights, policy digest substitution, missing rollback floor, epoch rollback, authentication/revocation failures, time boundaries, malformed/duplicate capabilities, missing/unverified/contradictory facts, unknown syscalls/arguments, unsupported multi-resource operations, W+X memory denial, unsupported resource kinds, capability revocation/expiry, and denial-reason determinism across capability ordering.

Related platform work: [#81 lab-only policy conformance probe runner](https://github.com/Luminous-Dynamics/luminous-platform/issues/81) and [#83 ephemeral network-namespace policy smoke test](https://github.com/Luminous-Dynamics/luminous-platform/pull/83). This crate is a reusable decision layer for future adapters; it does not duplicate the probe runner or namespace test.

The branch also contains [a Landlock + seccomp fixture integration probe](../landlock-readonly-fixture/README.md). It applies a real Landlock ruleset to a disposable child, installs an architecture-specific seccomp denylist, checks that chmod, setxattr, and utimensat are denied, and independently checks canary bytes, mode, modification time, and xattr absence from the parent. Positive controls verify xattr and timestamp APIs work before restriction. The seccomp layer is deliberately a denylist rather than a default-deny syscall sandbox; full metadata mutation and network protection remain unqualified. This is a narrow T1 fixture, not a full endpoint sandbox.
