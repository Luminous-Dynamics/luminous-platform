# Operational Recovery Trace Verifier v0.1

**Status:** research evaluator and evidence contract; not a production transaction manager or recovery executor.  
**Scope:** one service episode, with references to Platform, Nixward, participant ledgers and independent observers.

## The paradigm shift: causal evidence graph, not a dashboard timeline

A dashboard timeline tends to combine events by wall-clock timestamp. That is unsafe across independent systems: clocks drift, event delivery is delayed, and the order in which records arrive is not necessarily the order in which operations occurred.

This verifier uses two separate structures:

1. Explicit happens-before edges (parent_event_ids) to establish causal order across systems.
2. Per-source/per-epoch monotonic sequence and hash chains to detect missing, reordered or altered records within each source.

Timestamps are checked for format and observation freshness, but are never used to infer cross-source causality.

## Trust boundaries

The immutable run manifest declares:
- the exact source commit, run, episode, plan, policy, corpus, scenario, fault matrix and observer-manifest digests;
- executor and authority-adapter identities in separate trust domains;
- participant identities and participant trust domains;
- independent observer identities, trust domains and declared scopes;
- every required operation's target, operation, request, binding, idempotency and expected-postcondition digests.

The evaluator is read-only. It does not launch connectors, authorize mutations, reconcile journals or execute compensation. It consumes records emitted by the relevant authority, coordinator, participant ledger and observers. Nixward remains the only authority for host transaction state. Platform Saga remains the cross-system coordinator. No duplicate host journal or lock is introduced.

## Event lifecycle

The v0.1 event vocabulary is:
- plan_authorized
- dispatch_intent_durable
- participant_accepted
- effect_committed
- transport_response_lost
- attempt_status_observed
- effect_reconciled
- step_postcondition_verified
- coordinator_restarted
- recovery_scan_completed
- service_outcome_verified
- agent_declared_resolved

The key distinctions are intentional:
- Authorization is an upstream attestation, not an effect.
- A durable dispatch intent is a caller-side write-ahead claim, not proof of participant receipt.
- A participant acceptance is not proof that its business effect committed.
- A committed effect requires a participant receipt, and later reconciliation must bind that exact effect ID/digest.
- A lost response leaves the attempt ambiguous.
- A current-state read or eventual-consistency result cannot prove an attempt was never accepted.
- Only a strong, authoritative, attempt-specific participant-ledger observation can establish never_accepted.
- A step postcondition needs an independently declared observer and a fresh matching digest.
- Service restoration is separately observed after all required steps.
- An agent resolved claim without causal independent service verification is a false-resolution event.

## Retry and restart semantics

A new attempt after an unresolved/ambiguous prior attempt is invalid. It requires:
1. An exact never_accepted result from the declared authoritative participant ledger, with strong consistency.
2. A fresh authorization grant with a different grant digest.
3. A new attempt ID, while preserving the logical request and idempotency identity.
4. A causal chain from the no-effect proof through the new authorization to the next dispatch.

After a coordinator restart, the recovery scan must be causally downstream of the restart and account for every pre-restart dispatched attempt. Each scanned attempt is either reconciled using authoritative evidence or explicitly unresolved. Missing or unresolved attempts block subsequent mutations.

The verifier rejects duplicate committed effects for the declared idempotency key. This proves consistency of the supplied trace with the declared connector contract; it does not prove every real vendor API implements that contract.

## CLI

From the repository root:

    python3 tools/operational-recovery-verifier-v0.1/check_recovery_trace.py

For custom records:

    python3 tools/operational-recovery-verifier-v0.1/recovery_trace.py benchmarks/operational-recovery-v0.1/run-manifest.example.json benchmarks/operational-recovery-v0.1/events.example.jsonl --output recovery-verification.json

Run adversarial tests:

    python3 -m unittest discover -s tools/operational-recovery-verifier-v0.1 -p 'test_*.py' -v

The verifier uses only the Python standard library. Python is limited to the independent research/evaluation harness; no production Python execution path is introduced.

## Status and claim ceiling

Reports distinguish:
- valid: supplied records satisfy the v0.1 causal/evidence contract and a service-outcome verification exists;
- invalid: one or more structural, identity, causal, retry, effect, freshness or verification invariants are violated;
- indeterminate: evidence is missing or incomplete, so the episode cannot be promoted to verified service restoration.

The report binds the manifest, event log, participant inventory, observer inventory and verifier source digests.

Important limits:
- Hash chains detect accidental or post hoc record alteration only when their digests are anchored somewhere independently trustworthy. SHA-256 alone is not publisher authentication.
- The manifest's trust-domain labels are declarations, not proof of real organizational or infrastructure independence.
- authorization_valid and durability_attested are upstream assertions; this checker does not verify signatures, revocation, fsync, atomic publication or disk/power-loss durability.
- A synthetic observer receipt is not proof of actual customer service state.
- This tool is not a production readiness assessment, safety certification or SOTA result.

## Immediate integration sequence

1. Emit Nixward transaction/attempt references and receipt digests from its authoritative journal path. Do not mirror the Nixward journal in Platform.
2. Emit Platform Saga plan/step/dispatch references and participant IDs.
3. Emit participant-owned acceptance/effect/attempt-ledger observations from the connector boundary.
4. Emit independent step and customer-service observations.
5. Run this verifier over the immutable, source-pinned evidence bundle.
6. Use the same bundle as an OpsBench evaluator input, with cost and safety sidecars joined by exact run/arm/episode identity.

The first vertical slice should remain deliberately small: one Nixward host operation, one mock ITSM ticket operation, one independent service observer, one injected lost response, one coordinator restart and one final service verification.
