# Luminous Effect-Bound Capability Contract v0.1

**Status:** Non-normative contract and synthetic fixtures — research only  
**Date:** 2026-11-19  
**Tracking:** [AUTH-CORE-001 issue #501](https://github.com/Luminous-Dynamics/luminous-platform/issues/501)  
**Canonical owner:** Mycelix AGENT

## 1. Paradigm shift: authorize effects, not generic tools

A permission such as shell, write, network or admin describes a broad mechanism, not the protected effect the user actually authorized.

The contract binds a capability to a specific tuple:

    principal, actor, participant, resource, operation,
    argument_digest, context, policy_epoch, validity, delegation

The participant checks that exact tuple at the final admission boundary. A model, broker or authenticated session cannot turn a broad tool into permission for a larger effect.

This is a projection of Mycelix AGENT authority for cross-repository qualification. It is not a second grant database or policy evaluator.

## 2. Authority as a set of effect tuples

Let a grant G authorize a set of effect tuples [[G]]. A tuple includes principal, subject/actor, target resource, operation, canonical argument digest, context, policy epoch and time.

A child grant Gc is valid only when:

    Gc is a subset of Gp
    iff every child-authorized effect tuple is contained
    in the parent-authorized effect tuples

The subset test applies across every dimension: resources and participants; operations and argument constraints; purpose/context; validity interval; budget and rate constraints; delegation depth and redelegation rights; policy epoch and revocation lineage.

A child cannot expand one dimension because it narrows another. Narrower resources do not compensate for a wider operation set.

## 3. Composition must not create authority

When multiple independent capabilities are presented, the default composition operator is intersection, not union:

    Authority(G1 AND G2) = Authority(G1) INTERSECTION Authority(G2)

This avoids a dangerous form of authority synthesis. Suppose one agent can read resource A, while another can write resource B. Their collaboration must not automatically create permission to write A or arbitrary combinations of A and B.

A larger combined authority requires an explicit principal-issued joint grant with its own resource, operation, context and delegation semantics. It cannot be inferred from agent cooperation, shared tools, common ownership or trust scores.

## 4. One-use, effect-bound permit

A broker permit is bound to exact principal and actor, destination participant, target and operation, canonical argument digest, current policy epoch, validity interval, unique permit nonce and one attempt/idempotency identity.

The participant checks the permit again immediately before admission and consumes it atomically with respect to its protected attempt state. Repeated presentation must not create a second effect. An ambiguous acknowledgement is reconciled against the same attempt identifier; it is not blindly retried as a new mutation.

An attestation or evidence receipt can establish facts about identity, code, configuration, measurements or previous operations. **Evidence supports an authorization decision; it is not itself a capability or authorization grant.** This matches NIST's 2026 runtime-governance framing that supply-chain evidence is necessary but not sufficient to authorize a consequence: https://csrc.nist.gov/csrc/media/presentations/2026/governance-for-trustworthy-agentic-ai/images-media/20260922_willisbeyond.pdf

## 5. Three-valued authority and effect semantics

Authority evaluation has three outcomes:

- **Authorized:** every required identity, scope, time, policy, delegation and revocation condition is established.
- **Denied:** a required condition is false.
- **Indeterminate:** a required condition cannot be established, such as revocation state unavailable or identity binding stale.

A participant must deny both Denied and Indeterminate requests.

The effect state is separate: none, committed, partially committed or unknown.

An unknown effect is not a safe denial. The system must reconcile the same attempt, preserve uncertainty and require separate authority for any compensation or new mutation.

## 6. Rust/WASI execution boundary

The interface should use an object-capability execution model. WASI already specifies that external resources are supplied through explicit capabilities/handles and avoids ambient runtime authority. The WebAssembly Component Model aims for portable, statically analyzable, capability-safe interfaces and fine-grained component isolation. Luminous should compose those principles rather than invent a competing sandbox.

References:
- WASI capability design principles: https://github.com/WebAssembly/WASI/blob/main/docs/DesignPrinciples.md
- Component Model goals: https://github.com/WebAssembly/component-model/blob/main/design/high-level/Goals.md
- WASI 0.3 / Component Model interface direction: https://component-model.bytecodealliance.org/reference/faq.html

An illustrative Rust-facing shape:

    struct EffectPermit<P, A, R, O, E> {
        _principal: core::marker::PhantomData<P>,
        _actor: core::marker::PhantomData<A>,
        _resource: core::marker::PhantomData<R>,
        _operation: core::marker::PhantomData<O>,
        _effect: core::marker::PhantomData<E>,
    }

This can prevent accidental misuse within one program, but Rust types do not replace signed/runtime claim verification, revocation checks, participant-side admission or atomic one-use consumption. PhantomData alone is not a security boundary.

## 7. Canonical ownership and integration

- **Mycelix AGENT:** canonical grant, delegation, attenuation, revocation and identity semantics.
- **Symthaea INTX:** typed intent and proposed effect; cannot mint capabilities.
- **Platform broker:** exact request binding and attempt-scoped permit dispatch.
- **Nixward:** process, host, network and system-transaction enforcement.
- **Xenia:** authenticated session and secure transport of bound requests; no authority expansion.
- **Participant service/controller:** final local authorization, one-use/idempotency and effect journaling.
- **Sol-Atlas / Symtropy / independent observers:** provenance and postcondition evidence; no self-authorization.

No new cryptographic primitive, global transaction manager, policy engine, identity provider or sandbox implementation is proposed.

## 8. Initial synthetic conformance corpus

Machine-readable files:
- effect-bound-capability-contract.schema.v0.1.json
- effect-bound-capability-fixtures.schema.v0.1.json
- effect-bound-capability-fixtures.v0.1.json

The 17 cases cover exact root authority, narrow delegation, resource/operation/time widening, intersection vs union, actor substitution, argument digest mismatch, policy epoch mismatch, revoked/unknown authority, one-use permit replay, evidence-as-authority confusion, unknown-effect reconciliation, attempt substitution and permit expiry.

All values are synthetic. Passing these cases will not qualify a production Mycelix authority or participant implementation.

## 9. Research targets

- 100% exact deterministic fixture outcomes.
- Zero child scope expansion.
- Zero cross-principal authority fabrication.
- Zero unauthorized participant effects.
- Zero permit replay causing a second protected effect.
- 100% rejection of stale/expired/revoked/unknown authority.
- 100% positive-control admission for valid narrow operations.
- Unknown effect always retains the same attempt identity until reconciled.
- Two independently implemented verifiers agree before external qualification.

These are proposed qualification gates, not achieved results.

## 10. Claim ceiling

The document and fixtures specify research semantics only. No Mycelix AGENT implementation, Rust runtime, WASI sandbox, participant adapter or production capability has been modified or qualified by this work. Canonical authority remains owned by Mycelix AGENT; downstream repositories must not fork its semantics.
