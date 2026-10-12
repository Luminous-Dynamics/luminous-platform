# OMSI Nixward adapter mapping — 0.1 draft

**Status:** implementation design note; not a stable adapter contract  
**Repository inspected:** `Luminous-Dynamics/nixward`, default branch `main`, inspection date 2026-10-12.

This mapping is grounded in Nixward's existing `src/action/system_transaction.rs` and `SystemTransaction` API. The intent is to reuse Nixward's existing evidence-bound transaction, not create a parallel privileged executor inside OMSI.

## Existing Nixward transaction envelope

Nixward already exposes a versioned `SystemTransaction` with:

- transaction schema/version and transaction ID;
- target machine digest and ChangePlan digest;
- validation receipt;
- authorization receipt with issuer, evidence digest, evidence kind, expiry and optional signer/challenge/replay key references;
- snapshot receipt with pre-state and recovery-binding digests;
- application receipt with exact command digest, start/end time and exit status;
- verification receipt with post-state digest and evidence digest;
- explicit promotion, recovery and failure phases.

The source documents that this envelope wraps the existing `ChangePlan` and `ChangeAuthorization`; it does not introduce a second authorization mechanism. `authorize()` calls `authorization.validate_plan(plan)` and is only permitted after validation evidence exists.

Relevant source: https://github.com/Luminous-Dynamics/nixward/blob/main/src/action/system_transaction.rs

## Digest separation is mandatory

OMSI and Nixward digest values answer different questions. They MUST NOT be substituted for each other.

| Value | Current semantics | Adapter handling |
|---|---|---|
| OMSI action-arguments digest | SHA-256 of OMSI restricted canonical JSON under `org.omsi.action-arguments/0.1` | Bind exact serialized action arguments |
| OMSI operation-manifest digest | SHA-256 under `org.omsi.operation-manifest/0.1` | Bind the full immutable OMSI request/grant/receipt |
| Nixward ChangePlan digest | Digest returned by the typed Nixward plan | Preserve as a separate plan identity |
| Nixward target-machine digest | Digest returned by Nixward's `MachineBinding` | Preserve as a target-binding claim, not a global OMSI resource identifier |
| Nixward transaction ID | BLAKE3 over a Nixward-specific domain, plan digest and nonce | Preserve in a namespaced extension/reference |
| Nixward wire-envelope digest | BLAKE3 over Nixward `SystemTransaction` serialization | Never call this the OMSI manifest digest |

The adapter record should bind the OMSI manifest digest, Nixward plan digest, Nixward target-machine digest, Nixward transaction ID, profile version and correlation ID together. The mapping must be deterministic and testable. Changes to either side invalidate that binding and require a new validation/authorization decision.

## Lifecycle mapping proposal

| OMSI conceptual state | Nixward state/operation | Constraint |
|---|---|---|
| `proposed` | `SystemTransaction::planned(plan)` | Intent only; no authorization implied |
| `validated` | `record_validation(plan, ...)` → `Validated` | Validation must bind the exact plan |
| `authorized` | `authorize(plan, ChangeAuthorization)` → `Authorized` | Reuse Nixward's existing validated approval; no OMSI-only bypass |
| `accepted` / `running` | Adapter/executor observation outside current `TransactionPhase` | Do not synthesize these states from a queue receipt if the executor has not confirmed them |
| `applied` | `record_application(..., exit_status = 0)` → `Applied` | Process success is not verification |
| `verified` | `record_verification(...)` → `Verified` | Require fresh target-side post-state observation and profile-defined postconditions |
| `promoted` | `promote(plan)` → `Promoted` | Promotion must not precede verification |
| `failed` | Nixward `Failed` outcome, where appropriate | A failed process does not prove that the target remained unchanged |
| `recovered` | Nixward `Recovered` outcome plus fresh recovery evidence | Calling `recover()` alone is not evidence that the previous state was restored |
| `indeterminate` | OMSI adapter/transport state; no direct Nixward phase exists | Reconcile target state before retry; never map automatically to failed, recovered or verified |

This table is a proposed mapping. It does not assert that every OMSI transition is already represented by Nixward.

## Disposable-VM qualification strategy

Nixward's flake already contains a disposable NixOS test pattern at `tests/network-runtime-lab/network-runtime-lab.nix`, using `pkgs.testers.runNixOSTest`. The network runtime lab is not itself a transaction test, but its harness is a precedent for a separate transaction qualification check.

Reference: https://github.com/Luminous-Dynamics/nixward/blob/main/tests/network-runtime-lab/network-runtime-lab.nix

Recommended staged evidence:

1. Test the OMSI↔Nixward mapping in pure/unit fixtures with exact digests and negative subject-substitution cases.
2. Add a separate NixOS VM test check that exercises the existing transaction lifecycle with a test-only executor/authority fixture; keep that fixture unselectable in production.
3. Once the fixture path is proven, extend the VM test to a tightly scoped declarative NixOS change and real recovery.
4. Inject timeout-after-acceptance and process loss. Require read-only reconciliation before retry.
5. Verify recovery by observing actual post-recovery state; a `Recovered` phase alone is not enough for an OMSI verified-recovery receipt.

All mutation tests run only inside a disposable VM. The production host, real customer resources and real secrets are out of scope.

## Gaps to keep visible

- The current Nixward phase enum has no explicit `indeterminate` state; OMSI must preserve this distinction in its adapter/transport layer.
- Nixward's `recover()` updates the transaction envelope state but does not itself prove the target's recovery postcondition.
- Nixward and OMSI use distinct digest formats/algorithms; cross-binding should be explicit, not implicit.
- A proof-shaped OMSI grant is not the same as a cryptographically verified `ChangeAuthorization`.
- Existing Rust unit tests for `SystemTransaction` are not the same evidence as an executed disposable-VM test.
