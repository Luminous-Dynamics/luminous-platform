# Operational Recovery Fault Campaign v0.1

**Status:** research-only deterministic synthetic fault campaign. This is not a production readiness result, safety certification or SOTA result.

## Why this is a separate qualification layer

The causal trace verifier (#174) checks whether a supplied evidence bundle is consistent with the operational protocol. That is necessary but insufficient: a verifier can accept one good trace while real recovery logic remains brittle at a crash boundary. This campaign adds controlled positive and negative histories and requires the verifier to classify each history exactly as preregistered.

The primary paradigm shift is from testing a successful workflow to testing a state machine under injected faults at every irreversible boundary. We do not score task success alone. We demand either (a) reconciled effect plus independent service verification, (b) authoritative proof of no effect plus fresh authority before a new attempt, or (c) a safe halt that preserves an indeterminate state.

## Fault matrix coverage

The v0.1 matrix contains 15 deterministic scenarios:

| Family | Cases | Required result |
|---|---|---|
| Response-loss windows | before acceptance; after acceptance; after effect | retry only with strong no-effect proof; otherwise reconcile same attempt or halt |
| Duplicate / identity | duplicate effect; grant rebinding; contradictory never-accepted ledger | invalid |
| Restart recovery | omitted scan; dispatch before scan; unresolved attempt; correctly reconciled restart | reject bypass, preserve unknown, or accept fully accounted recovery |
| Evidence quality | missing service observation; stale observation; false resolved claim | indeterminate or invalid, never pass |
| Durability boundary | partial/non-durable intent | invalid |

Each scenario has an expected status and mandatory violation-code set. The runner fails if a declared case is missing, an undeclared case exists, the exact verifier returns a different status, or a mandatory violation is absent. Failed and indeterminate cases remain visible in the output.

## Run contract

The campaign uses the Builder fixtures from the #174 adversarial test module and invokes the actual #174 verify() implementation. It does not copy a second implementation of the recovery rules. Repeated runs over the pinned matrix and source must produce the same report digest.

The CI workflow is read-only, pins the checkout action to an immutable commit, compiles the exact verifier and campaign sources, runs the campaign and invariant tests, and emits the full report in the job log.

## What is not established

The synthetic trace campaign does not kill a process, reboot a VM, corrupt a disk image, simulate a storage controller, exercise a real connector or contact a customer system. The disk-full/partial-publication case is represented as absent or false durability evidence; it tests that the verifier fails closed, not whether Nixward actually persists atomically.

A next-stage Nixward qualification needs an isolated disposable VM and deterministic crash injection immediately before/after journal fsync, rename/commit, systemd job submission, process exit and terminal receipt publication. For each cut, reboot from the captured disk image, compare the authoritative Nixward journal and attempt IDs with systemd job / InvocationID / process observations, participant ledger and an independent service probe. Use a fault-injection harness such as QEMU block-layer error injection only after reviewing its exact behavior and compatibility with the target NixOS/systemd build. No second journal or lock is introduced.

The current recovery verifier still does not authenticate evidence publishers, establish real trust-domain independence, verify hardware/OS durability semantics or prove the real participant's idempotency contract. Signed attestations may later authenticate publisher and artifact identity, but they still will not establish service restoration; the independent service oracle remains mandatory.

## Promotion gates

1. The exact-head fault campaign is green and source/matrix/report digests are retained.
2. Every production event producer is bound to its authoritative owner and identity.
3. Nixward-specific crash cuts are exercised in a disposable VM against the exact target NixOS, kernel and systemd versions.
4. Connector semantics are qualified per operation: idempotency scope, key retention, consistency model, admission/commit visibility and authoritative attempt-status API.
5. Independent observers prove fresh postconditions outside the executor and participant trust domains.
6. Only after independent review and repeated fault campaigns should the first small host-operation + mock-ITSM vertical slice be considered for a controlled environment.

The public matrix is a development corpus, not a blind benchmark holdout. Comparative performance and SOTA claims require separate preregistered held-out trials using the #176 common-core protocol.
