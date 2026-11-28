# ITOPS qualification core — policy module crate

This directory is a standalone, dependency-free Rust library crate for the ITOPS / ATR research. It contains three deliberately bounded reference modules:

- `delegation.rs`: strict edge-by-edge monotonic delegation attenuation;
- `root_reanchor.rs`: root-authorized, exact-precommitted descendant task authorization, while retaining a narrower intermediate agent's local effect role;
- `effect_finality.rs`: a synthetic participant commit-finality / unknown-outcome state machine with a mutex-serialized commit boundary, revocation epochs, idempotency conflict detection, budget debit, and exact receipt-binding checks.

The effect-finality module is a **reference model**, not a distributed/production participant, cryptographic receipt verifier or independent observer. The crate does **not** contain the full benchmark catalogue loader, result-schema auditor, process-isolated runner, capability issuer, signed-token verifier, live participant PEP, distributed transaction service, or independent service observer. Those remain separate work items tracked from [issue #213](https://github.com/Luminous-Dynamics/luminous-platform/issues/213), [issue #540](https://github.com/Luminous-Dynamics/luminous-platform/issues/540), and [issue #544](https://github.com/Luminous-Dynamics/luminous-platform/issues/544).

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
| `delegation.rs` | 15 |
| `root_reanchor.rs` | 9 |
| `effect_finality.rs` | 11 |
| **Total** | **35** |

These are source annotations, not executed results. The exact-head workflow
`.github/workflows/itops-qualification-core.yml` runs formatting, verifies
all three module inventories, executes the full all-target test suite and
Clippy, then emits a machine-readable receipt with source SHA, toolchain
versions, Cargo.lock digest and test counts.

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
The exact descendant task/plan pair is now stored in each grant as
`allowed_child_task_plan_bindings`, not just as two independent digest sets.
Strict attenuation and root-reanchoring require the exact pair on the immediate
parent grant; root-reanchoring also requires the pair in the root grant's
descendant manifest. The maps on `PendingEffectRequest` remain consistency
cross-checks only and cannot authorize a pair absent from the grants.


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

### Participant finality reference model

`effect_finality.rs` makes the commit boundary and unknown-outcome rules executable as a deterministic in-process model:

- the participant checks the current authority epoch, idempotency key and mutation budget while holding one serialized commit lock;
- an exact duplicate request returns the original receipt without a second budget debit;
- a different request reusing the same idempotency key is a conflict;
- new mutations under an old authority epoch are denied, while effects that linearized before revocation remain historically committed;
- response loss before or after participant processing enters `OutcomeUnknown`;
- no new consequential request may replace an unknown attempt;
- an authoritative absent reconciliation permits only retrying the original exact request and idempotency key;
- a receipt-binding check rejects actor, tenant, resource, operation, action-digest, authority-epoch and idempotency-key substitution;
- a concurrent same-key test exercises the in-process mutex boundary and proves one modeled effect / one budget debit under that model.

This establishes unit-testable state-machine semantics, not production atomicity or distributed linearizability. A real adapter must use the protected resource's transaction or consensus boundary, survive process/host failure, and supply an independently qualified effect observer.

## Explicit non-claims

This crate is not a production issuer, token verifier, proof-of-possession
implementation, live participant PEP, distributed commit-time revocation
fence, independent effect observer, or measured benchmark. The authority
observations supplied to the pure predicates are not independently authenticated
or proven fresh. The in-process `Mutex` is only the reference-model
linearization point; it does not prove atomicity across processes, machines or
failure domains. The receipt matcher checks field consistency, not signatures
or observer independence. Unit-test annotations do not prove execution; rely
on the exact-head workflow result and its uploaded receipt before claiming the
test suite passed.
