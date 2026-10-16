# Mycelix Hearth integration: make household intent safe to execute

**Status:** architecture and contract design only. This does not claim the integration is implemented, Hearth's physical-effect runtime is qualified, or any device has been controlled by this repository.

## The paradigm shift

Do not join Mycelix and the home node by giving either one a universal admin token. Join them through a **household-governed effect protocol**: portable intent, context-specific authority, exact adapter admission, constrained local execution, and independent outcome reconciliation.

The important unit is not a trigger or API call. It is an end-to-end, revocable, reviewable effect attempt with a declared purpose and an honest outcome state.

Mycelix Hearth supplies household/relationship and plan-governance context. Luminous Platform supplies host isolation, declarative deployment, local credentials, resource/network enforcement, recovery and evidence packaging. Hearth's existing edge admission path remains the only authority path for physical/provider-side effects. Symthaea may interpret observations or propose plan steps, but cannot mint household authority, select its executor, or bypass adapter admission.

## What the current Mycelix design tells us

- [HTH-AUTO-004C / PR #2316](https://github.com/Luminous-Dynamics/mycelix/pull/2316) requires an exclusive local physical-executor lease and crash-safe journal wrapper.
- [HTH-AUTO-004D / PR #2324](https://github.com/Luminous-Dynamics/mycelix/pull/2324) defines deterministic exact adapter admission; registration order is never an authority tie-breaker, and primary actions do not implicitly authorize compensation.
- [HTH-AUTO-004E / issue #2328](https://github.com/Luminous-Dynamics/mycelix/issues/2328) describes binding exact admission into the edge runtime before dispatch.
- [HTH-AUTO-IOT-000 / issue #2879](https://github.com/Luminous-Dynamics/mycelix/issues/2879) explicitly prohibits a parallel physical-dispatch authority path and separates provider state/acknowledgement from physical truth.

These PRs/issues were open at the time of research. Their existence is architecture evidence, not a passing build, completed runtime, or qualification result. Their exact-head executable gates remain authoritative.

## Authority is composed, never inferred

For any real provider effect, the intended dispatch composition is:

1. The plan step is valid and bound to exact plan ID, step ID, action role, capability and operation.
2. Current household authority is refreshed immediately before each attempt/retry. If freshness or the required verifier is unavailable, no new side effect is dispatched.
3. The exact Hearth adapter admission matches the plan step, adapter ID, implementation reference, profile reference, rule ID and action role.
4. The registered adapter matches the admitted implementation/profile; the same human-readable name or capability string is insufficient.
5. The physical adapter is inside the live exclusive executor lease and uses the durable crash journal.
6. The effect attempt uses one logical idempotency key and bounded retry/time/resource policy.
7. Outcome is reconciled against observations strong enough for its risk class. An HTTP/service response, Matter feature advertisement, Home Assistant state update or dispatch log is not proof the physical device reached the target state.

If a command is ambiguous after crash, reconcile; do not automatically dispatch it again. Compensation is a distinct role and needs its own exact admission. See [HTH-AUTO-004E](https://github.com/Luminous-Dynamics/mycelix/issues/2328).

## Request contract, not an authorization token

[home-effect-envelope.schema.json](../schemas/home-effect-envelope.schema.json) defines a request-only envelope. It binds workload contract/version and digest where available; opaque local Hearth/requester references; exact plan/step/primary-or-compensation coordinates; provider family and adapter implementation/profile/rule identity; local target reference and arguments digest rather than raw command arguments; expiry and idempotency; required Hearth admission route; and privacy-safe evidence policy.

The schema deliberately has no `qualified` readiness. This project does not have the independent evidence verifier needed to authenticate qualification. JSON Schema cannot prove runtime currentness, enforce OS permissions, or demonstrate a physical device effect.

The positive fixture is a synthetic, design-only proposal. Negative fixtures target design-only direct effects, a bypass route, and an attempt to treat provider acknowledgement as physical proof. They do not run a Mycelix agent, prove current authority, lock a physical executor, validate an adapter, or perform device effects.

## New paradigm: make the effect lifecycle executable

A schema catches malformed declarations. It does not describe whether a request may transition from "not attempted" to "dispatch eligible" after a membership change, expired admission, crash or ambiguous provider response. Those are state-transition properties, not field-shape properties.

The side-effect-free Rust reference model in [effect_protocol.rs](../tools/home-profile-validator/src/effect_protocol.rs) makes those rules explicit:

- Exact equality across requested plan/step/role/capability/operation and the admitted and registered adapter implementation/profile/rule.
- Fresh household authority and adapter admission before any new attempt.
- A live exclusive executor lease, durable journal and risk-appropriate postcondition observer.
- An independent physical stop for high/critical risk classes.
- A bounded attempt budget and same-key retry only after a definitive no-effect outcome.
- The idempotency key is paired with a canonical request fingerprint so the same key cannot be reused with a different target/action/payload. The caller must compute this digest over the complete canonical request; the model does not authenticate or recompute it.
- In-flight, provider-accepted and unknown outcomes are reconciliation-only; an expired request or revoked grant must not trigger a duplicate dispatch.
- A reconciled terminal result cannot be replayed as new intent with the same request.

This is an **executable reference model**, not a formal proof or a production enforcement layer. It does not authenticate the boolean evidence supplied to it, acquire locks, access Mycelix authority, call Home Assistant/Matter, or move a real device. The next integration step is to run the same adversarial traces against Hearth's actual edge runtime and compare the runtime decisions with the model before any real adapter is enabled.

This direction complements the lifecycle focus in [NIST IR 8259 Rev. 1](https://csrc.nist.gov/pubs/ir/8259/r1/final), final in April 2026, which expands IoT manufacturer guidance across product needs, risk, maintenance, support and end-of-life. It also follows the key semantics in [Home Assistant's REST API](https://developers.home-assistant.io/docs/api/rest/): an API response or updated state representation does not necessarily establish the physical outcome.

## Privacy boundary: governance travels, household telemetry does not

Shared automation authority must not imply universal visibility into everyone's health, presence, routines, media or messages.

- Mycelix consent/decision semantics are not the same as host process operating-system permissions.
- Host filesystem/network/device sandboxing is defense in depth; it cannot create household consent.
- The local executor needs provider credentials; envelopes, Symthaea context, public evidence and DHT-visible records must not contain them.
- Keep raw device/entity identifiers, occupancy traces, target details and arguments local. Share only authorized, minimized, redacted evidence, preferably digests and opaque references.
- Revocation or household departure requires fresh authority resolution. A copied request, stale cache, reputation score, adapter signature or device attestation must not restore revoked authority.

This aligns with the [Open Home Foundation's smart-home privacy position](https://www.openhomefoundation.org/papers/privacy/) and Home Assistant's [local-data/privacy guidance](https://www.home-assistant.io/faq/is-my-data-private/). Locality is valuable, but does not by itself make a system secure.

## Offline behavior: preserve the home, fail closed on new side effects

Essential local operation should continue through loss of internet, remote services, Mycelix peer connectivity, Symthaea inference or the dashboard where the device/protocol allows it. That does **not** justify fresh high-consequence effects based on stale authority.

- Manual physical controls and independently designed safe-state behavior remain available without cognition.
- Observation/proposal generation may degrade to local-only mode.
- If required household authority or adapter-admission freshness cannot be established, do not dispatch a new side effect.
- Recovery from an already-dispatched ambiguous action is reconciliation-only, not permission for a duplicate command.
- Any future offline/delegated authority lease needs a separately reviewed bounded scope, expiry, revocation semantics and risk ceiling.

## Conformance ladder

| Gate | Required evidence | Does not prove |
|---|---|---|
| H0 — Contract | Schema and adversarial fixtures | Runtime enforcement, household authority or physical behavior |
| H1 — Policy binding | Multi-agent Hearth tests for grants, denial, expiry, membership change and revocation bound to exact plan/step | Real provider behavior |
| H2 — Synthetic edge | Exact admission, lease, journal, duplicate/crash/retry tests without a physical device | Hardware or physical truth |
| H3 — Provider simulator | Home Assistant test instance and Matter simulator/test profiles | Real-device reliability or safety |
| H4 — Low-risk bench | One named device/profile, explicit operator, observable postcondition and measured recovery | General provider support or high-consequence safety |
| H5 — Consequence-specific review | Independent threat/safety review and field evidence for each claimed scope | Universal qualification outside that scope |

The Luminous-side addition is H0 design intent until exact-head CI succeeds. Mycelix runtime and provider gates remain unqualified until their own exact heads satisfy their owning issues. Qualification does not transfer from a schema, VM test double, one provider profile or low-risk action to the full product.

## First adversarial experiment

Start with synthetic fixtures and the upstream Hearth synthetic/replay harness, not a real home:

1. Submit a request twice with the same idempotency key and assert one logical side effect.
2. Revoke authority immediately before retry and assert no new dispatch.
3. Change adapter implementation/profile after admission and assert stale admission rejection before the provider call.
4. Swap primary/compensation roles and require separate exact admissions.
5. Kill the executor after dispatch but before reconciliation and assert recovery reconciles without redispatch.
6. Simulate HTTP 200 or an entity-state update without an observed device outcome and assert the result remains unknown, not success.
7. Remove Hearth connectivity or make authority currentness unprovable; assert no new effect while core host services remain operational.
8. Scan logs/exports for credentials, household names, raw target IDs and telemetry; these must remain local or be redacted.

Preserve negative controls. Each test must prove the intended enforcement boundary rejects the dangerous case rather than pass because malformed JSON failed earlier.

## Primary references

- [Mycelix Hearth repository](https://github.com/Luminous-Dynamics/mycelix/tree/main/mycelix-hearth)
- [HTH-AUTO-004C](https://github.com/Luminous-Dynamics/mycelix/pull/2316), [004D](https://github.com/Luminous-Dynamics/mycelix/pull/2324), [004E](https://github.com/Luminous-Dynamics/mycelix/issues/2328)
- [HTH-AUTO-IOT-000](https://github.com/Luminous-Dynamics/mycelix/issues/2879) and [observation/effect corpus issue](https://github.com/Luminous-Dynamics/mycelix/issues/2887)
- [Open Home Foundation — Privacy in the smart home](https://www.openhomefoundation.org/papers/privacy/)
- [Home Assistant — Is my smart home data private?](https://www.home-assistant.io/faq/is-my-data-private/)
- [JSON Schema Draft 2020-12](https://json-schema.org/draft/2020-12)
