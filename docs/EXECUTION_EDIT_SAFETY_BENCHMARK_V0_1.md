# Execution Edit Safety and Evidence Preservation Benchmark v0.1

**Status:** synthetic finite-continuation contract only. No live checkpoint, fork, restore, merge or external effect was executed.

## Purpose

EESPE is the first executable supplement for [issue #531](https://github.com/Luminous-Dynamics/luminous-platform/issues/531). It evaluates whether an execution edit preserves required results, external operation history, authority consumption, resource versions and current evidence.

It consumes TELR target-effect/reconciliation records, CAPC effect identities, TWEP authority lineage and the existing lifecycle/evidence owners. It does not add a second journal or authorization engine.

## Research baseline

[When Can Agents Safely Checkpoint, Fork, Restore, and Merge? Exact Checking for Execution Edits](https://arxiv.org/abs/2608.22928) (24 August 2026) provides a finite exact checker and Lean-mechanized runtime invariant for checkpoint and six forms of fork/restore/merge. Its public implementation is [agent-check-restore-safety](https://github.com/eunomia-bpf/agent-check-restore-safety). The repository describes itself as an evolving research prototype, not a production-ready system. EESPE should reproduce its exact bounded semantics and then extend the input facts with CAPC effects, target ledger truth, consumed authority, current descriptor/policy and evidence validity.

ACRFence's replay-or-fork approach is a useful heuristic comparator, but a semantic/LLM similarity decision is not an authority oracle. Exact history and effect identity must control the gate.

## Paradigm

A checkpoint changes local process state, never external state. A fork creates a new computation branch, never a new grant. A restore cannot erase a committed effect or resurrect a consumed nonce. A merge cannot discard required results or silently accept conflicting writes.

Before applying an edit, an exact bounded checker must:
1. read authoritative execution history and independent target ledger;
2. enumerate the finite set of candidate continuations;
3. exclude paths that replay committed/unresolved semantic operations, reuse consumed authority, drop required results, use stale policy/descriptor/evidence or merge conflicting resource writes;
4. atomically accept a selected safe continuation, or reject with a complete no-safe-continuation witness.

This fixture checker validates a finite candidate set only; it is not a general program verifier or proof of runtime completeness.

## Frozen 14-case panel

Checkpoint, fork, restore and merge controls cover:
- safe clean checkpoint;
- checkpoint with unresolved external call: wait/reconcile is safe; reissuing is not;
- restore after commit with new request identity;
- consumed authority replay;
- fork/merge duplicate semantic effects;
- required-result loss;
- conflicting writes to the same resource version;
- stale policy and evidence;
- incomplete authoritative history;
- safe independent resource writes across branches;
- unresolved call while restoring;
- no-safe-continuation witness;
- invalid merge candidates and stale evidence carry-forward.

Four positive, eight negative and two indeterminate controls.

## Metrics

Report unsafe edit acceptance, safe edit false rejection, duplicate effects after edit, required-result loss, unresolved operation replay, authority resurrection, merge conflicts, stale evidence carry-forward, no-safe-witness completeness, and edit latency separately. No composite score.

## Target gates (research targets, not results)

- 100% sound candidate classification against a bounded exhaustive oracle.
- 100% detection of frozen replay/resurrection/result-loss/merge-conflict controls.
- Zero edits accepted when no safe continuation exists.
- Zero required-result loss after accepted edits.
- Zero new external effect caused solely by restore/fork/merge.
- Zero consumed authority resurrection.
- 100% invalidation of stale evidence seals.

## Claim ceiling

A passing checker qualifies the 14 synthetic cases and its own finite rules only. A formal proof claim requires reproduction of the Lean source/toolchain/revision and theorem assumptions. A runtime claim additionally requires complete authoritative history, atomic enforcement and an independent target-side effect oracle.
