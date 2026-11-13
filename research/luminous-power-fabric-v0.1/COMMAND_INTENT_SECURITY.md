# Power Fabric cyber-physical command-intent security — v0.1

**Tracking:** [Issue #459](https://github.com/Luminous-Dynamics/luminous-platform/issues/459) · [Transition graph PR #455](https://github.com/Luminous-Dynamics/luminous-platform/pull/455)

**Status:** synthetic defensive test contract and typed offline admission/effect-assessment validator. No cryptographic verification, command transmission, penetration test, HIL, physical protection or site test has occurred.

## Standards context (2026-11-13)

- NIST SP 800-82 Rev. 3 remains the final OT-security guide. Rev. 4 is an initial public draft published 2026-09-21; comments are due 2026-11-30: https://csrc.nist.gov/pubs/sp/800/82/r3/final · https://csrc.nist.gov/pubs/sp/800/82/r4/ipd
- IEEE 1686-2022 and Corrigendum 1-2025 (published 2026-02-27) cover IED cybersecurity capabilities: https://standards.ieee.org/ieee/1686/7207/ · https://standards.ieee.org/ieee/1686-2022_Cor_1/11847/
- IEC 62351-3:2023 defines a security profile for TCP/IP-based power-system protocols: https://webstore.iec.ch/en/publication/68410
- ISA/IEC 62443 supplies the wider IACS security lifecycle and system requirements.

These are scoping references, not a conformity claim.

## Paradigm: intent is not dispatch, acknowledgement is not physical effect

The command chain is declared intent → external identity/signature/revocation verification → local scope/sequence/nonce/time/state/configuration/leadership/protection admission → command dispatch by the owning controller → device acknowledgement → independent physical feedback → state reconciliation.

The Rust module implements two read-only operations:
- `assess_admission` checks caller-supplied identity/signature status, role/key/asset/command/transition scopes, replay inputs, sequence, timebase, pre-state/configuration, leadership and protection permissive. It returns dispatch permission; it does not dispatch.
- `assess_effect` is called after dispatch or when the outcome is ambiguous. Missing, stale, spoofed, out-of-order, supervisory-only or mismatched observations require reconciliation before another mutation. Only fresh independent feedback for the exact command/sequence and expected post-state may permit state promotion.

No observation means unknown, not permission to retry. A device acknowledgement does not prove the physical operation succeeded.

The module does not verify signatures/certificates, query revocation, persist consume-once state, authenticate observation bytes, send commands or operate equipment. Mycelix remains the cryptographic verification, revocation and durable consume-once owner. Digest fields are shape-checked only.

## Synthetic corpus

- `schemas/command-intent-security-v0.1.schema.json`: strict Draft 2020-12 structure.
- `schemas/command-intent-security.synthetic.json`: one baseline command/policy and 37 defensive cases.
- `tools/measurement-audit/src/command_intent_security.rs`: typed evaluator and 15 authored Rust tests.

The cases cover unknown/revoked/expired issuers, signature states, authorization scopes, MEMS switchgear attempts, command/nonce replay, sequence rollback, expiry/timebase, state/configuration drift, digest-shape checks, leader conflict, split brain, protection permissive, protection reset clearance, acknowledgement-only, supervisory, spoofed, stale and out-of-order observations.

## Benchmarks to aim for

| Dimension | Research target |
|---|---|
| Admission | 100% rejection of in-scope unauthorized/replay/expiry/rollback/scope/configuration cases |
| Rejected requests | Zero dispatch permission and zero state promotion |
| Consume-once | No second non-idempotent effect for reused ID/nonce; enforced by durable external owner |
| Physical effect | Zero promotion from acknowledgement or supervisory telemetry alone |
| Unknown outcome | Reconcile physical state and consumed identity before next mutation |
| Leadership | No dual-authority dispatch; deterministic leader-epoch admission |
| Protection | No supervisory/AI override of independent local protection |
| Timing | p50/p95/p99 verification/admission latency within declared local deadlines |
| Resilience | Local protection/control remains available during supervisory partition |

Compare only against the same controller, protocol, key infrastructure, command rate, network impairment and timing envelope. Synthetic test cases do not establish cyber assurance or SOTA superiority.

## Ownership and claim ceiling

Platform owns the type contract, corpus and offline validator. Mycelix owns signature/certificate verification, key and revocation policy, signed authorization receipts and durable consume-once state. Symtropy owns runtime timing and communications/plant replay. Sol-Atlas owns asset/topology identity. Symthaea may propose tests, never authorize operations. Local controller and independent protection remain the physical authority.

`SYNTHETIC_COMMAND_INTENT_TEST_PLAN_ONLY_NOT_SIGNATURE_VERIFICATION_OR_CYBER_ASSURANCE`

Next: bind to Mycelix's actual receipt and nonce interfaces, then exercise protocol and failover behavior in controller-HIL with a qualified lab. No energization, switching or protection reset is authorized.
