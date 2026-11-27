# ITOPS qualification core — policy module crate

This directory is now a standalone, dependency-free Rust library crate for the two
pure multi-agent authorization policy arms used by the ITOPS / ATR research:

- `delegation.rs`: strict edge-by-edge monotonic delegation attenuation;
- `root_reanchor.rs`: root-authorized, exact-precommitted descendant task
  authorization, while retaining a narrower intermediate agent's local effect
  role.

The crate intentionally contains only those two policy modules. It does **not**
contain the full benchmark catalogue loader, result-schema auditor, process-
isolated runner, capability issuer, signed-token verifier, participant commit
gate, or independent service observer. Those remain separate work items
tracked from [issue #213](https://github.com/Luminous-Dynamics/luminous-platform/issues/213)
and [issue #544](https://github.com/Luminous-Dynamics/luminous-platform/issues/544).

## Public build closure

The subdirectory now has its own `Cargo.toml`, `Cargo.lock`,
`rust-toolchain.toml`, and `src/lib.rs`. It has no third-party Cargo
dependencies. The crate is intentionally isolated from ambient parent
workspace files and sibling paths.

Pinned toolchain: Rust 1.96.0, edition 2024.

## Run locally

From this directory:

```sh
cargo fmt --all -- --check
cargo test --locked --offline --all-targets
cargo clippy --locked --offline --all-targets -- -D warnings
```

The lockfile contains only this package. Once the pinned Rust toolchain is
installed, all Cargo commands are offline and do not require fetching crates.

## Current source test inventory

The public source files at the base snapshot inspected for this work contain:

| Module | Authored `#[test]` functions |
|---|---:|
| `delegation.rs` | 14 |
| `root_reanchor.rs` | 7 |
| **Total** | **21** |

These are source annotations, not executed results. The new workflow
`.github/workflows/itops-qualification-core.yml` runs formatting, all-target
unit tests and Clippy at the exact pull-request revision, then emits a
machine-readable build receipt with source SHA, toolchain versions, Cargo.lock
digest and source test counts.

## Security semantics

### Strict attenuation

Every child delegation must preserve the root task and plan, link to its
immediate parent and issuer, and use task and plan digests precommitted by the
parent. Targets, operations, parameter constraints and data scopes must be
subsets; time, dispatch/tool/resource budgets, delegation depth and risk
ceilings may only decrease; inherited approvals, postconditions and rollback
obligations may only grow. Omitted or incomparable parameter constraints fail
closed. Multi-hop verification checks every edge, duplicate grant/task
identities and supplied revocation/validity state for every ancestor.

A zero-mutation budget remains valid for a bounded read-only delegated task;
the verifier should not reward blanket refusal.

### Root-reanchored descendant authorization

The separate root-reanchor policy permits an explicitly precommitted descendant
to use its own bounded leaf effect envelope even where an intermediate agent's
local effect scope is narrower. It still requires exact root/parent child task
and plan precommitments, immediate parent/grant/principal lineage, fresh active
authority observations for every ancestor, root and leaf scope checks, exact
parameter/data/resource bounds, budgets, risk and required approvals.

The task/plan pair maps are currently supplied in the pure request type rather
than embedded in authenticated grants; this is a known source-level integration
gap tracked in [issue #547](https://github.com/Luminous-Dynamics/luminous-platform/issues/547).
The current `request_digest` is shape-checked but not recomputed by the pure
predicate; that binding gap is tracked in
[issue #546](https://github.com/Luminous-Dynamics/luminous-platform/issues/546).

## Explicit non-claims

This crate is not a production issuer, token verifier, proof-of-possession
implementation, participant PEP, commit-time revocation fence, independent
effect observer, or measured benchmark. The authority observations supplied to
the pure predicates are not independently authenticated or proven fresh by this
crate. Unit-test annotations do not prove execution; rely on the exact-head
workflow result and its uploaded receipt before claiming the test suite passed.
