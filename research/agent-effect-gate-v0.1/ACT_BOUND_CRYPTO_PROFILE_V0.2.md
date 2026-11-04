# Act-bound execution finality: cryptographic integration profile v0.2

Status: research decision record and isolated integration plan. No cryptographic verifier is wired into the reference sink by this document.

Date: 2026-11-04

## 1. Trust assertions must remain separate

Issue #206 requires the participant effect boundary to establish that this exact operation, for this exact tenant/principal/plan/target/attempt, at this exact participant, was authorized once and that the authority was consumed atomically with the effect.

| Assertion | Mechanism | Establishes | Does not establish |
|---|---|---|---|
| Build/tool/artifact provenance | Sigstore bundle verification against pinned trust root and signer identity policy | Which workflow identity signed exact artifact bytes, with the selected transparency/timestamp evidence | Permission to execute an operation or correctness of the resulting service effect |
| Runtime workload identity | SPIFFE SVID and trust bundle, preferably mTLS | Which workload is communicating with the participant/broker | Permission to mutate a target |
| Runtime attestation | Verified attestation result via existing #196 adapter / an applicable EAT profile | Measured-state claims under a declared appraisal policy | Operation authorization or service truth |
| Act-specific authorization | One-use JWS/JWT execution handle | A trusted issuer authorized an exact act for an exact audience and validity interval | That the operation actually committed |
| Effect finality | Participant-owned atomic handle/nonce consumption + idempotency/effect commit | Authority consumption and protected effect share the participant's atomic boundary | Production durability until implemented and tested against a real participant store |
| Service truth | Independent observer under a qualified evidence profile | Whether declared postconditions and workload SLOs were met | The agent's narrative or internal reasoning |

Do not collapse any of these into a caller-supplied `verified=true` field.

## 2. Candidate act-handle profile

### 2.1 JWS/JWT

Candidate Rust library: `jsonwebtoken = 11.1.0` (release listed 2026-09-16). It supports EdDSA and ES256. Pin the chosen version and crypto backend in the isolated spike.

The v0.1 profile should require:
- `typ=application/luminous-act-finality+jwt` and an explicit profile version;
- exactly one asymmetric algorithm allowlist (initial spike: EdDSA); never derive allowed algorithms from untrusted token headers;
- `kid` resolved only through a verifier-owned, local issuer/key registry; never fetch keys from token-provided `jku`, `x5u`, or other URLs;
- exact issuer and participant audience;
- required `exp`, `nbf`, `aud`, `iss`; check `iat`, unique `jti`, nonce digest, policy epoch and act digest explicitly as application claims;
- a short, bounded lifetime, strict clock-quality rule and explicitly configured clock skew;
- rejection of wrong `typ`, unknown header fields, unsupported critical headers, duplicate JSON object names, unknown issuer/key, key/algorithm mismatch, and malformed claim types;
- key rotation, retirement and revocation/epoch policy under a verifier-owned registry.

The library's current `Validation` docs say the default required claim set is only `exp`, `nbf` is not validated by default, and default leeway is 60 seconds. The adapter must set these explicitly: allowed algorithms, issuer, audience, required registered claims, `validate_nbf=true`, and a deliberately small leeway. `iat` and custom act claims need separate validation. Do not rely on library defaults.

JWT header and payload segments must be strict-parsed before trusting them. `jsonwebtoken` provides signature and registered-claim validation, but its normal typed deserialization must not be treated as a duplicate-key detector. Parse the raw base64url-decoded JSON through the strict JCS admission path below first; reject any header fields not allowed by the Luminous profile.

### 2.2 Canonical Candidate Act: vr-jcs

Candidate Rust library: `vr-jcs = 0.4.1` (published 2026-05-11). Its current API documents a strict raw-byte path that rejects duplicate object properties, validates I-JSON strings/numbers, and enforces a maximum nesting depth of 128. Use `to_canon_bytes_from_slice` / `canonical_bytes_from_slice` for untrusted JSON; do not use the deprecated typed-only helpers as the admission boundary.

The typed participant-reconstructed Candidate Act must be serialized under a fixed versioned schema. Encode epochs, sequences and timestamps that could exceed the exact IEEE-754 integer range as decimal strings. Preserve exact validated UTF-8 values; do not silently Unicode-normalize after signing. Reject unknown fields for v1. The participant recomputes the canonical bytes and digest from the request it will actually execute.

**Digest decision for the spike:** use `DigestStrategy::blake3_domain_separated("org.luminous-dynamics.act-finality.v1")`. The vr-jcs 0.4.1 API documents that its SHA-256 variant is declared but currently returns `UnsupportedAlgorithm`; do not call it as though it were implemented. If SHA-256 is mandatory for an external wire contract, canonicalize with vr-jcs strict parsing and then use a separately pinned, tested SHA-256 implementation behind a clearly named adapter. Do not hand-mix algorithms invisibly.

The crate documents that `to_canon_digest_with` accepts a trusted `serde_json::Value`; for untrusted input, use its strict raw-byte parser first. The spike must test the exact strict-parse + typed-claims + digest sequence against official RFC 8785 vectors, duplicate-key attacks, Unicode edge cases, numeric boundaries and cross-language vectors. Do not write a custom canonicalizer.

The often-mentioned `serde_json_canonicalizer` release is currently 0.3.2, not 0.4. It remains a typed serialization alternative, but it is not the preferred raw untrusted JSON admission parser for this profile.

### 2.3 SPIFFE/SPIRE: workload identity only

Candidate Rust crate: `spiffe = 0.19.0` is listed in the current crate changelog, with opt-in Workload API sources. The docs.rs root API page currently displays 0.18.0 while the latest changelog page begins at 0.19.0; resolve this publication/docs version discrepancy against the exact crates.io package before pinning it in Cargo.lock. Do not claim that any version was integrated yet.

Prefer X.509-SVID over mTLS for broker/participant channel identity where practical. Verify certificate path, trust domain, SPIFFE ID, validity, trust-bundle rotation and peer identity at the actual effect boundary. SPIFFE's X.509-SVID validation requires standard certificate path validation plus SPIFFE-specific leaf/issuer validation.

A JWT-SVID is a separate workload-identity token, not the act-finality JWT. It must use separate `typ`, issuer, audience, key/trust policy and validation rules. A SPIFFE ID authenticates a workload; it does not authorize a mutation.

### 2.4 Sigstore: build and tool provenance only

Candidate verifier: official `sigstore-rust 0.15.1`, using only the crates required for verification/trust-root/bundle parsing. Pin the exact release tag and its source commit.

Use Sigstore for evaluator, runner, tool binary, connector image, manifest and policy-bundle provenance. Pin the allowed CI identity and OIDC issuer, verify bundle signature/certificate/transparency evidence under the chosen profile and trust-root update policy. A valid signature from an unapproved workflow identity must fail.

The official conformance dashboard currently reports 140/140 passing for `sigstore-rust @ 552f9b2` (dashboard updated 2026-11-03), including Rekor v2, DSSE-as-Hashedrekord and user-managed keys. This is a result for that reported commit/test set; it is not proof that the tagged 0.15.1 release itself achieved the same exact run, nor production integration/security certification.

Never use an artifact signature as a one-use action permit.

### 2.5 DSSE and EAT boundaries

- DSSE is useful for in-toto/Sigstore provenance envelopes. The standalone `probityai-archive/dsse` Rust repository is archived; do not adopt it as a production dependency. Prefer the maintained Sigstore Rust bundle/verifier path for supply-chain provenance.
- RFC 9711 EAT is a standardized attestation token framework. Use it only where the existing #196 runtime-attestation profile needs it. It does not define an application-specific mutation authorization flow.
- Mycelix QUAL-001E / ASSURE-PORTFOLIO-CUT and LEX-NET-030 continue to own source-role, evidence-dependency and common-cause analysis. Do not create a parallel trust graph or scalar independence score.

## 3. Participant finality algorithm

1. Authenticate the caller's SPIFFE identity over the actual participant channel.
2. Reconstruct the Candidate Act from the actual request to be executed.
3. Strict-parse the raw token header and payload; reject duplicate keys, non-I-JSON content, excess nesting and disallowed header fields.
4. Verify JWS with an allowlisted algorithm, local `kid` registry, exact issuer, exact participant audience, exact `typ`, required claims, time bounds and policy epoch.
5. Canonicalize the participant-reconstructed act under the pinned profile and compare its domain-separated digest with the signed claim.
6. Recheck grant, policy, revocation and scope at the finality boundary.
7. In one durable serializable participant transaction / equivalent linearizable conditional write, consume `jti` and nonce, check tenant-scoped idempotency, commit the protected effect, and write the receipt binding exact act/handle digest.
8. If the same logical business effect already committed, consume the fresh valid handle and return the original effect receipt without repeating the effect.
9. Any mismatch, unknown identity, stale or revoked policy, expired token, replay, unavailable verifier or unsupported participant finality must deny or remain OutcomeUnknown; no silent retry.

If the side effect occurs in an external system that cannot share the transaction boundary, the adapter must use that system's idempotency and authoritative attempt/effect reconciliation. A local database record saying “about to call API” does not prove atomic finality. Strict-finality mode must deny unsupported external effects until that participant's own finality boundary is qualified.

## 4. Isolated Rust spike and adversarial corpus

Pin these exact candidate packages in a separate research-only crate:
- `jsonwebtoken = 11.1.0`, exactly one crypto backend, and PEM decoding enabled only for the test fixture;
- `vr-jcs = 0.4.1`;
- `base64 = 0.22.1`;
- the exact `serde` / `serde_json` versions emitted in the generated lockfile.

Build an issuer/key registry and a strict token-header profile. The registry is local and verifier-owned; no token header may initiate remote key retrieval. Test:
- positive signed token, exact issuer/audience/typ/alg, exact act digest;
- wrong issuer, wrong audience, wrong typ, unknown kid, mismatched kid/alg, HS/RS or algorithm-family confusion, `alg=none`;
- disallowed `jku`, `x5u`, `jwk`, `x5c`, `crit`, compression or unknown header fields;
- missing/expired/future `exp`/`nbf`, bad/missing `iat`, excessive lifetime and clock-skew boundaries;
- wrong participant, policy epoch, handle ID, nonce, request/target/plan digest, manifest/toolchain digest;
- duplicate JSON keys in header or payload, malformed UTF-8, forbidden I-JSON code points, numeric range/exponent edge cases, nesting beyond 128 and oversized segments;
- revoked/retired issuer key and key rotation overlap;
- exact JCS official vectors, cross-language vectors and property-order invariance.

Use only deterministic test keys in `#[cfg(test)]`; no production or long-lived signing key belongs in the repository. The first spike does not yet add SPIFFE or Sigstore dependencies; those are separate integration profiles.

## 5. Qualification hypotheses

- Zero successful commits for every invalid issuer, audience, typ, algorithm, duplicate-key, expired, revoked, stale-policy, wrong-act, wrong-target, wrong-request, replay and unsupported-participant vector.
- 100% participant-side request reconstruction and act-digest recomputation.
- 100% committed effects have one participant receipt binding act, handle, nonce and logical-action receipt.
- Zero duplicate effects after reauthorization, policy rotation, participant restart or process race.
- 100% missing/unknown/indeterminate identity or ledger outcomes fail closed or remain OutcomeUnknown.
- Cached in-process verification p95 <=10 ms as a lab hypothesis, measured separately from remote key material and durable participant transaction.
- False denial <=1% on a positive-control corpus with confidence intervals and fault-family breakdown.

These are qualification targets, not measured results. No production security, FIPS, distributed atomicity, or standards compliance claim follows from this spike.

## 6. Decision

The recommended architecture is deliberately compositional:
- JWS/JWT: exact one-use act authority.
- vr-jcs: strict raw JSON admission + JCS canonicalization + domain-separated BLAKE3 action digest.
- SPIFFE/SPIRE: authenticated workload identity and mTLS.
- Sigstore: build/tool/manifest provenance.
- EAT/#196: measured runtime attestation, only where the existing profile requires it.
- Participant transaction: actual finality.
- Independent observer: service truth.

The next step is an isolated, exact-version verifier spike and known-answer corpus. Do not wire it into the live broker until signature/key-policy, JCS vectors, replay races and participant crash cuts pass.
