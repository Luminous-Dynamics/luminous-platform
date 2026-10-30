# Service-Episode Saga Contract v1

**Status:** reference-model contract; no production orchestration or crash-consistency claim  
**Version:** 1.0  
**Prepared:** 2026-10-30  
**Related:** OpsBench v0.1, Nixward System Transaction Architecture, Agentic Endpoint Assurance v1, issue #152

## 1. Why a service episode is not one transaction

An enterprise service episode may span NixOS, an ITSM system, an identity provider, DNS, a cloud control plane, a database, observability and customer communications. These participants do not share a transaction manager, atomic commit protocol or common rollback boundary.

A successful Nixward host transaction does not make a ticket update atomic. A ticket API timeout does not prove the ticket update failed. Revoking identity and rolling back a host generation are not inverse operations. A customer notification cannot be "un-sent". Therefore, the platform must not model the complete service episode as one ACID transaction or assume that reverse-order commands restore the original world.

The correct abstraction is a **service-episode saga**: a durable orchestration contract over individually scoped operations, each with its own authority, dispatch, effect observation, postcondition verification and (where genuinely possible) separately authorized compensation.

This is an application of established saga orchestration, not a claim to have invented it. AWS describes sagas as local transactions with compensating actions and explicitly identifies idempotency, eventual consistency, lack of transaction isolation and observability as important design concerns. Temporal provides durable workflow/event-history primitives and retry patterns. Luminous should compose a mature durable execution runtime where appropriate; this reference model defines the safety and evidence semantics that any runtime adapter must preserve.

## 2. The lifecycle and the critical distinctions

Each connector operation has a lifecycle:

`prepare → authorize → persist dispatch intent → dispatch → reconcile effect → verify postcondition`

The following facts must remain distinct:

| Fact | Establishes | Does not establish |
|---|---|---|
| Plan prepared | Exact intended request and target have been fixed | Authority or execution |
| Grant verified by the authority adapter | A signer/policy authorized the exact operation binding | The operation is safe or has occurred |
| Dispatch intent durably published | The system is committed to reconciling this attempt | The remote system received it |
| Transport acknowledgement | A connector returned a response | The requested external effect exists |
| Effect observed | An observer saw an effect or no-effect result | The effect satisfies the requested service outcome |
| Postcondition verified | An independent observer saw the expected target state | The whole customer service is healthy |
| Service outcome verified | An independent service oracle observed the requested service-level condition | Every other unmeasured or out-of-scope condition |

**A timeout after dispatch is an unknown outcome.** It is not permission to resend. A task cancellation does not cancel a request that may already have reached a remote participant. A successful HTTP response or process exit is not proof of a postcondition.

## 3. Step and episode contract

Each forward operation binds:

- episode plan digest;
- stable step ID;
- target/resource identity;
- connector operation ID;
- canonical request digest;
- canonical operation-binding digest;
- expected postcondition digest;
- dependency step IDs;
- whether the step is required for the requested outcome;
- optional compensation specification.

The canonical binding digest is computed by the trusted adapter over a versioned, deterministic encoding of the plan, step, target, operation, request and relevant scope. The reference model receives the digest; it does not implement a cryptographic hash or signature verifier.

Each authorization grant must bind the exact plan digest, step ID, target, operation ID, request digest and operation-binding digest, plus a unique grant ID and expiry. A compensation uses a **different operation binding and grant**. Grant signature, issuer trust, revocation, audience and trusted-clock checks are upstream obligations.

The plan is a directed acyclic graph. A step may be authorized only after every declared dependency has a verified postcondition. A dependency that is merely dispatched, acknowledged, indeterminate, compensated or blocked is not complete.

## 4. Dispatch, ambiguity and retries

Before a connector call, the orchestrator publishes a write-ahead dispatch-intent record under the episode's serialization boundary. The record binds the attempt ID, idempotency key and exact operation binding.

After the dispatch boundary, these conditions produce an unresolved attempt:

- response timeout or connection loss;
- orchestrator cancellation or crash;
- worker/supervisor loss;
- ambiguous connector response;
- contradictory or eventually-consistent read-back.

The attempt becomes `outcome_unknown`. It stays unresolved until independent evidence reconciles it.

A retry is permitted only after one of these conditions is established:

1. the connector's documented idempotency contract guarantees repeated submission with the **same idempotency key and exact request binding** cannot create a duplicate effect; or
2. a sufficiently strong independent read proves the previous attempt had no effect, under that connector's stated consistency semantics, and a new attempt receives the required authority.

An eventual-consistency "not found" result is not proof of no effect. The v1 reference model takes the conservative path: a retry requires a strong independent no-effect observation and a fresh exact-operation grant. It preserves the original logical idempotency key and assigns a new attempt ID. Production adapters may add a connector-specific idempotency proof, but must not weaken the default.

## 5. Independent observation and verification

An observation is admissible to the reference model only when it:

- identifies the exact attempt and target;
- binds the exact operation-binding digest;
- comes from a trust domain different from the agent under evaluation;
- carries an evidence digest;
- declares its consistency class.

Possible outcomes are:

- expected postcondition observed;
- no effect observed;
- forbidden effect observed;
- conflicting or unknown evidence.

Only a strong, independent observation of the exact expected postcondition can verify a step. A strong, independent no-effect observation can make a forward retry eligible. A positive forbidden-effect observation is a safety violation. Eventual or contradictory evidence remains indeterminate.

The service episode has one additional verification boundary: after every required step is verified, an independent service-level observer must confirm the requested global service postcondition. A ticket marked closed, an agent narrative, a database row or a host unit state cannot replace this observation.

## 6. Compensation is not rollback

A compensation is a new operation, not a magical reversal of history. It may be unavailable, non-equivalent to the original action, delayed or itself fail. Examples:

- a ticket update may be compensated by a separate correction/reopen operation;
- an identity revocation may require a fresh, separately authorized re-enrolment rather than "undo";
- a DNS change may be compensated by a new record update, subject to cache/TTL propagation;
- a customer notification cannot be recalled, only followed by a correction;
- restoring a database backup may require an explicit forward recovery plan rather than an assumed rollback.

The v1 model permits compensation only for a previously verified step with an explicit compensation specification and a separate exact grant. It requires a new dispatch intent and independent verification of the compensation postcondition. It does not automatically compensate a saga after failure and does not infer that a compensated step makes the original service request resolved.

## 7. Cancellation, concurrency and journal ownership

Cancellation blocks undispatched future work. An authorized but not dispatched step is blocked. A step with a persisted dispatch intent is moved to an unresolved outcome and must be reconciled. Cancellation is not a remote cancellation guarantee.

Every transition is compare-and-swap guarded by an episode revision. A stale actor cannot append against an old revision. Attempt IDs are unique across the episode. A logical idempotency key is stable across retries of the same step intent.

The reference model stores an in-memory event sequence solely to make transition behavior inspectable. It is **not** a durable journal. Production integration must use a crash-consistent append-only journal, durable write-ahead publication, single-writer/interlock semantics and restart reconciliation. Nixward's existing machine transaction journal and interlock remain authoritative for NixOS host mutations; this saga is the cross-system coordinator above them, not a replacement host transaction.

## 8. Disposition rules

The episode can be called **resolved** only when:

1. every required step is verified against its expected postcondition; and
2. an independent service-level observer confirms the expected service digest.

Other outcomes remain explicit:

- `running`: work is safely progressing;
- `partially_complete`: some required work is verified, but the episode is not resolved;
- `awaiting_service_verification`: all required steps are verified, but the global service observation is absent;
- `indeterminate`: at least one dispatched operation or observation remains ambiguous;
- `escalation_required`: a required step is blocked, a postcondition failed, or service verification reports unhealthy;
- `security_violation_observed`: an independent observer reports a forbidden effect;
- `resolved`: all required steps and the service-level postcondition are independently verified.

These dispositions are not collapsed into a weighted score. A safe refusal or unresolved outcome must never be counted as successful resolution.

## 9. Reference model boundary

`tools/service-episode-saga-v1` is a dependency-free Rust reference state machine for plan-DAG validation and the transitions above. It checks exact metadata binding, revisions, attempt uniqueness, strong no-effect reconciliation, separate compensation authority and the final service verification gate.

It does **not**:

- verify cryptographic signatures, key revocation, identity or trusted time;
- persist events to disk or prove crash/power-loss durability;
- call ITSM, identity, DNS, cloud, database or Nixward connectors;
- enforce tenant isolation or operating-system permissions;
- prove observer independence beyond comparing declared trust-domain identifiers;
- implement a scheduler, retry policy, queue, rate limiter or production workflow engine.

Do not use the reference model as an authorization token or production execution boundary. Its purpose is to make the state-transition contract executable and adversarially testable before adapters are built.

## 10. Qualification targets

The v1 reference-model test suite should enforce:

| Invariant | Target |
|---|---:|
| stale revision accepted | 0 |
| modified target/operation/request binding accepted | 0 |
| duplicate attempt ID accepted | 0 |
| expired grant accepted | 0 |
| dependency cycle accepted | 0 |
| dispatch ambiguity automatically replayed | 0 |
| eventual-consistency absence treated as no-effect proof | 0 |
| same-agent-trust-domain observer accepted | 0 |
| compensation without separate authority accepted | 0 |
| episode resolved without all required steps verified | 0 |
| episode resolved without independent global service verification | 0 |

For runtime qualification, inject failures at every durable boundary and connector boundary: before dispatch-intent publication, after publication/before send, request accepted/response lost, worker cancelled, reply stored/signal lost, stale read-after-write, connector outage, concurrent operator change, compensation interruption and global-health disagreement.

Report attempted operations, duplicate effects, unknown dispositions, safe holds, time to reconcile, time to verified restoration, operator interventions and total cost per verified outcome. Do not label queued or unexecuted trials as passes. Use matched ITBench/EnterpriseOps-Gym tasks for task reasoning, then add OpsBench-specific multi-system action/recovery extensions; preserve each upstream benchmark's own evaluator and metric.

## 11. Implementation sequence

1. Land/review the reference state machine and adversarial unit tests.
2. Map OpsBench `CONN-001`, `CHG-002`, `IAM-001`, `TENANT-001` and `BACKUP-001` to explicit saga interruption/compensation fault classes.
3. Add a durable event-journal adapter using the existing Platform/Nixward persistence and interlock contracts; do not introduce a second Nixward host journal.
4. Implement mock ITSM and identity connectors first, with independent state read-back and idempotency semantics.
5. Connect one approved Nixward host transaction as a participant; verify service health externally.
6. Fault-inject and compare against a generic durable workflow runtime (for example Temporal) and deterministic scripts. The comparison asks which system delivers the required evidence and safe dispositions, not whether the Luminous state machine replaces durable execution infrastructure.

## References

- [AWS Prescriptive Guidance — Saga patterns](https://docs.aws.amazon.com/prescriptive-guidance/latest/cloud-design-patterns/saga-patterns.html)
- [AWS Prescriptive Guidance — Saga orchestration](https://docs.aws.amazon.com/prescriptive-guidance/latest/cloud-design-patterns/saga-orchestration.html)
- [Temporal — Durable workflow patterns](https://docs.temporal.io/design-patterns)
- [Dapr — Workflow compensation patterns](https://docs.dapr.io/developing-applications/building-blocks/workflow/workflow-patterns/)
- [OpsBench v0.1](TECHNOLOGY_SERVICES_OPSBENCH_V0_1.md)
- [Nixward system transaction architecture](https://github.com/Luminous-Dynamics/nixward/blob/main/docs/SYSTEM_TRANSACTION_ARCHITECTURE.md)
