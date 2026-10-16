# Shared home-effect conformance: one trace, multiple implementations

**Status:** synthetic contract and reference-model test corpus. It does not certify Mycelix Hearth's live edge runtime, provider adapters, host sandbox, or physical device outcomes.

## Paradigm shift

The interoperability test is no longer “both repositories have similar-looking schemas” or “each implementation passes its own tests.” It is: **given the same versioned adversarial state snapshot, do the Luminous reference model and Mycelix Hearth edge runtime make the same safety decision?**

The shared corpus lives at [`tests/home-effect-conformance/v1/traces.json`](../tests/home-effect-conformance/v1/traces.json), governed by [`home-effect-conformance-traces.schema.json`](../schemas/home-effect-conformance-traces.schema.json). Each vector declares a hazard, exact request/admission/registration state and expected decision. The initial 11 snapshots cover revocation before retry, adapter implementation/profile drift, crash after dispatch before reconciliation, same-key/different-request reuse, provider-accepted ambiguity, compensation-role separation, high-risk stop requirements, terminal replay and inconsistent journal identity.

## Conformance rules

1. **Same input, same decision.** The Hearth-side adapter maps authoritative live state into the documented input; it may not fill missing values optimistically.
2. **Divergence blocks promotion.** Preserve the trace, inputs, exact source heads and redacted diagnostics. Do not silently change expected output to make a failure disappear.
3. **Negative controls reach the intended boundary.** A trace must exercise the intended authority/admission/journal rule, not fail on unrelated malformed input.
4. **Time is part of the input.** Check expiry and freshness at the retry boundary, not only when a plan was created.
5. **Ambiguity is a reconciliation problem.** `Dispatching`, `ProviderAccepted` and `OutcomeUnknown` do not permit blind redispatch. A transport/API acknowledgement is not physical truth.
6. **Request identity is semantic.** One idempotency key binds one canonical request fingerprint; retries cannot silently change target, operation, risk class or arguments. The gate also rejects an empty or whitespace-only fingerprint when called without schema validation; it still does not recompute or authenticate the digest.
7. **Conformance has a claim ceiling.** The vectors do not authenticate authority, verify the fingerprint, acquire a lease, prove crash durability, enforce OS permissions or control devices.

## Required Mycelix Hearth adapter

The Hearth adapter should run the **actual** 004E edge admission/runtime path in a synthetic/replay harness and emit machine-readable results keyed by corpus version, trace ID and step ID. Compare those results while retaining source SHA and run evidence. Unsupported or unevaluable is not pass. The runtime must not call an alternate Home Assistant/Matter path to satisfy a vector.

Run conformance when either repository changes authority, adapter admission, journal/recovery, dispatch, compensation or outcome observation. Later, model-check the state machine (for example, TLA+/TLC) so state-space exploration complements replay vectors; this RFC is not currently model-checked.

## Why the outcome boundary matters

Home Assistant's REST documentation distinguishes updating an entity representation from communicating with a device; a service response must not be conflated with independently observed physical state ([REST API reference](https://developers.home-assistant.io/docs/api/rest/)). NIST's final [IR 8259 Rev. 1](https://csrc.nist.gov/pubs/ir/8259/r1/final), published April 2026, spans product-lifecycle security, including maintenance, support and end-of-life—not just initial pairing.

## Verification ceiling

The Rust tests validate the corpus schema and expected decisions against the local reference model. A green run establishes only those assertions against the exact tested commit. Until Mycelix Hearth itself consumes the same vectors and its exact-head gates pass, cross-implementation conformance remains **not established**.
