# Service Episode Attestation Profile v0.1

**Status:** E0/E1 research profile; no production verifier or trust federation is implemented.  
**Standards basis:** in-toto Attestation Framework v1 / DSSE v1.0.  
**Date:** 2026-10-31.

## 1. Goal

A service episode must be portable as a set of independently verifiable claims, not just a JSON result with digests and trust-domain labels. A digest identifies bytes; a signature authenticates a key's statement over bytes; a verifier policy determines whether that key is trusted for a role; independent evidence and semantics determine whether the claim is sufficient to establish an external fact.

These are distinct checks. No signature alone proves that a remote ticket, endpoint, DNS zone, database, cloud resource or customer-facing service actually changed or recovered.

## 2. Standards-first format

This profile uses:
- in-toto Attestation Framework Statement v1 for subject digests and typed predicates;
- DSSE v1.0 for an authenticated payload type, payload and one-or-more signatures;
- a profile-specific predicate type URI: `https://luminousdynamics.io/attestations/service-episode/v0.1`.

The payload type is `application/vnd.in-toto+json`. A verifier must calculate and verify the DSSE Pre-Authentication Encoding (PAE) over the exact payload type and serialized payload bytes. Never invent a signature preimage or use ad-hoc JSON key sorting/canonicalization as a substitute for DSSE PAE. Verify the in-toto Statement `_type`, subject digest, `predicateType` and predicate version before interpreting content.

The DSSE `keyid` is only a hint for selecting candidate trust keys. Identity and authorization come from the verifier's trust policy and issuer registry. The same key id cannot self-assert its organizational or operational role.

Standards and references:
- [in-toto Attestation Framework v1.2](https://github.com/in-toto/attestation/tree/main/spec/v1)
- [in-toto Statement v1](https://github.com/in-toto/attestation/blob/main/spec/v1/statement.md)
- [in-toto Envelope requirements](https://github.com/in-toto/attestation/blob/main/spec/v1/envelope.md)
- [DSSE v1.0 protocol and PAE](https://github.com/secure-systems-lab/dsse/blob/master/protocol.md)
- [W3C Verifiable Credentials Data Model v2.0](https://www.w3.org/TR/vc-data-model/)
- [Sigstore verification and transparency](https://docs.sigstore.dev/)

W3C VC 2.0 can be considered later for issuer credentials and delegation interoperability; it is not a replacement for the in-toto service-outcome predicate. Sigstore/Rekor is an optional transparency-log arm and must undergo privacy review before exposing service metadata.

## 3. Four separate issuer roles

Each attestation is a separately signed statement. The bundle requires role-specific evidence:

| Role | May attest | Cannot prove |
|---|---|---|
| Coordinator | exact plan, authorized intent, dispatch/receipt references and orchestration transitions | remote participant effect or independent service health |
| Participant | the exact attempt's admission/effect status from its own attempt/idempotency ledger | global service health |
| Independent observer | the exact service/target postcondition it independently measured | that a specific remote attempt was accepted unless it owns that ledger |
| Authority | exact grant scope and lifecycle | that the operation succeeded |

The role in the signed predicate is a claim to check against a verifier-controlled issuer registry, not a self-asserted right. The registry maps principal and key to authorized roles, scopes, organization, operator ownership and control domain. It also defines delegation, validity, rotation and revocation.

**Independence is a property of control, not a string field.** The verifier must reject a coordinator and observer controlled by the same security/administrative domain where policy requires independence. A different `trust_domain_id` string is insufficient. A signature from the coordinator that labels itself an observer remains coordinator testimony.

## 4. Binding and anti-replay requirements

The versioned predicate schema binds:
- episode ID, tenant commitment and exact plan digest;
- service reference, target digest, step, operation and canonical request digest;
- stable logical idempotency key and unique attempt ID;
- issuer principal/key reference and claimed role;
- effect class, consistency, postcondition/evidence digest and observation interval;
- issuer sequence, nonce, issue/expiry timestamps and verifier policy version;
- evidence/claim ceiling.

Required comparisons are exact. An attestation for a different tenant, plan, target, operation, request, attempt or idempotency key cannot be reused. Expiry alone does not prevent replay; verify a nonce and monotonic per-issuer sequence against verifier-owned state. Reject future-dated evidence beyond the configured clock-skew tolerance. The thresholds are deployment policy, not globally hard-coded truths.

Unknown major predicate versions, untrusted keys, revoked keys, missing required roles, invalid PAE, malformed signatures and schema downgrade attempts fail closed. Optional signatures do not replace required distinct roles: two coordinator signatures are not equivalent to a participant plus observer signature.

## 5. Service resolution policy

A `resolved` outcome requires:
1. valid coordinator attestation for the exact plan and actions;
2. authoritative participant attestation for each required mutating attempt (or a provider-specific read-only proof whose strength is explicitly documented);
3. a current authority attestation/reference binding the exact action;
4. valid independent observer attestation, from a distinct control domain, for the expected service postcondition;
5. strong-enough consistency and fresh observation data;
6. matching tenant, plan, target, operation, request, attempt and postcondition digests.

A signed `outcome_unknown`, weak/eventually-consistent observation, stale health reading or postcondition mismatch stays indeterminate/rejected. Never average away a failed trust gate.

## 6. Privacy and transparency

Use minimum necessary claims and opaque tenant commitments. Hashes of low-entropy tenant IDs, hostnames, ticket numbers or short secrets may be dictionary-correlated; a plain unsalted hash is not automatically privacy-preserving. Use a keyed commitment or protected opaque reference where the verifier needs stable equality but the public artifact must not reveal the value. Keep sensitive evidence attachments encrypted/access-controlled and bind their digests in the signed predicate.

A public transparency log is optional and policy-gated. Do not publish tenant IDs, credentials, raw hostnames or low-entropy identifiers to Rekor or other public logs. Log inclusion proves that a statement was logged, not that its claims are true.

## 7. Conformance suite

`benchmarks/service-episode-attestation-v0.1/conformance-vectors.json` defines 32 evaluator-only synthetic vectors across signature integrity, issuer trust, role separation, exact binding, freshness/replay, semantics, schema evolution, privacy and transparency. These are abstract mutation descriptors; they contain no private keys or fabricated cryptographic signatures.

The bundle and predicate JSON Schemas freeze envelope/predicate shapes. The structural checker verifies inventory and invariants only. It does not verify cryptography. A subsequent Rust verifier must use a maintained DSSE implementation (or an independently tested implementation of the exact protocol), test against published test vectors, use ephemeral test keys, and be checked by a second independent verifier.

## 8. Benchmark gates and reporting

Hard gates:
- 0 tampered signatures/payloads/payload types accepted;
- 0 unauthorized role/scope/tenant/target/attempt bindings accepted;
- 0 replayed or expired observations accepted as fresh;
- 0 coordinator-only statements accepted as independent participant/service truth;
- 0 unknown/weak observations promoted to resolved;
- 100% malformed or unknown-major versions fail closed;
- deterministic agreement across independent verifier implementations.

Performance and privacy metrics:
- signature and bundle verification p50/p95/p99;
- throughput at 1/10/100/1,000 attestations per episode;
- payload/attachment bytes;
- key rotation/revocation propagation time;
- offline verification latency and trust-bundle size;
- false rejection on valid compliant statements;
- operator adjudication time and evidence retrieval cost;
- tenant correlation/leakage under a published attacker model.

All are targets, not measured results. Publish algorithms, key lengths, implementation versions, trust bundles, machine details, fixture counts and confidence intervals. Do not collapse security failures into a weighted score.

## 9. Ownership and claim ceiling

The runtime signs its own orchestration history; participants sign their own attempt/effect ledger; observers sign their own measurements; authority signs or references its own grants. Nixward remains the sole owner of host transaction journaling, worker attempt identity and recovery. Mycelix may later supply identity, delegation and revocation primitives, but cannot issue evidence on behalf of a participant or observer. OMSI remains the semantic profile, not a signing authority.

This v0.1 artifact is E0/E1 schema, role model and synthetic vector work only. It does not implement Ed25519 signing, DSSE verification, real issuer registration, organizational independence proofs, key lifecycle or cross-vendor interoperability.
