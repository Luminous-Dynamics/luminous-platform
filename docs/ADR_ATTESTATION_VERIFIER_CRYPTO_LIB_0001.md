# ADR-0001: Cryptographic and format libraries for Service Episode Attestation

- **Status:** Provisional, no production dependency selected
- **Date:** 2026-10-31
- **Related:** [Issue #166](https://github.com/Luminous-Dynamics/luminous-platform/issues/166), [Attestation profile PR #166](https://github.com/Luminous-Dynamics/luminous-platform/pull/166)

## Decision

Do not add a production cryptographic dependency to the attestation profile until an isolated Rust verifier spike passes official DSSE/in-toto vectors, cross-language interoperability and Luminous-specific trust-policy tests.

Separate the implementation into four replaceable layers:

1. **Envelope/cryptographic verification:** DSSE PAE, accepted algorithm suite, signature validation and threshold counting by distinct trusted key.
2. **Statement/predicate parsing:** in-toto Statement v1 and versioned predicate types.
3. **Issuer/trust policy:** key-to-principal mapping, role/scope, control domain, key lifecycle, delegation, revocation and replay state.
4. **Episode semantics:** exact plan/tenant/target/operation/request/attempt binding; participant-effect truth; independent service postcondition; evidence ceiling.

A library that solves one layer must not be assumed to solve the others. In particular, a cryptographically valid DSSE envelope is not proof that the signer is authoritative, independent or factually correct.

## Current Rust library findings (2026-10-31)

### 1. Standalone `dsse` crate: strong protocol tests, unsuitable as a production dependency today

The crate's documentation describes important hardening: PAE byte lengths, signed payload type, canonical base64 decoding, refusing empty signatures/zero thresholds, counting unique keys rather than signature entries, treating `keyid` as a hint, and returning the exact payload bytes that were verified. It documents 53 tests, including official protocol vectors, 11 cross-language Go fixtures and malformed/attack cases. The package is Apache-2.0 and the code declares `unsafe_code = "forbid"`, `unwrap_used = "deny"`, `expect_used = "deny"`, and dependency vetting in CI.

**Blocking supply-chain concern:** the GitHub repository was archived on 2026-10-25. Its current archived main-branch manifest declares version 0.1.4, while docs.rs lists 0.1.2 as the latest published release. It has very low public adoption. The latest package may be technically thoughtful, but a security-critical verifier should not acquire an archived, pre-1.0 dependency without a maintainer/support plan and an explicit fork or vendoring decision.

Conclusion: do not make it a production runtime dependency. Its published tests and documented refusal cases can inform our test plan, but use official DSSE/in-toto vectors and independently obtained cross-language fixtures as the authority. Do not copy implementation code without separate license/provenance review.

References:
- [dsse 0.1.2 on docs.rs](https://docs.rs/crate/dsse/latest)
- [Archived probityai-archive/dsse repository](https://github.com/probityai-archive/dsse)
- [Archived crate Cargo.toml](https://github.com/probityai-archive/dsse/blob/main/Cargo.toml)
- [Archived crate security policy](https://github.com/probityai-archive/dsse/blob/main/SECURITY.md)
- [Archived crate provenance record](https://github.com/probityai-archive/dsse/blob/main/PROVENANCE.md)

### 2. Official in-toto Rust bindings: good type/schema candidate, not a signer/verifier

The in-toto Attestation Framework repository now includes Rust bindings generated from the project's protobuf definitions. Its Rust crate declares `in_toto_attestation` 0.2.0 on the current repository branch and depends on protobuf/code-generation crates. The published docs.rs crate page still presents 0.1.0. These bindings are useful for typed Statement and predicate representations, but they do not implement DSSE signing, cryptographic verification, issuer authorization, key lifecycle or service outcome semantics.

Conclusion: evaluate the official types for parsing/serialization and keep the schema contract in sync with in-toto v1. Do not treat the Rust bindings as a complete verifier. Pin a released artifact or commit and run conformance tests before use.

References:
- [in-toto Attestation Framework](https://github.com/in-toto/attestation)
- [official Rust binding manifest](https://github.com/in-toto/attestation/blob/main/rust/Cargo.toml)
- [Rust binding README](https://github.com/in-toto/attestation/blob/main/rust/README.md)
- [Rust binding docs.rs](https://docs.rs/in_toto_attestation/latest/in_toto_attestation/)

### 3. Sigstore Rust workspace: active integration candidate, but still experimental

The Sigstore Rust workspace is actively developed and has split crates for bundles, cryptography, trust roots, TUF, Rekor, Fulcio, signing and verification. Its current repository workspace manifest declares version 0.15.1 and Rust 1.86 MSRV; docs.rs pages and the public installation guide may lag at older 0.11/0.14 versions. The Sigstore Rust client reports 100% on the published conformance dashboard's 140 cases as of 2026-10-30, and its TUF client reports 119/119 as of 2026-10-30.

However, the official Sigstore Rust client page still identifies the Rust API as under active development and not stable until 1.0. The conformance suite checks Sigstore client behavior; it does not establish correctness of Luminous role thresholds, our issuer/control-domain independence registry, tenant isolation, anti-replay state, or service-outcome semantics.

Conclusion: treat Sigstore Rust as the leading optional path for keyless Fulcio/OIDC and transparency-backed verification, after pinning an actual released version and testing the exact bundle formats we need. Do not make Sigstore's public transparency log mandatory for private service episodes. Evaluate offline user-managed-key verification separately from keyless identity flows.

References:
- [sigstore-rust workspace](https://github.com/sigstore/sigstore-rust)
- [Sigstore Rust client guidance](https://docs.sigstore.dev/language_clients/rust/)
- [Sigstore Rust conformance results, 2026-10-30](https://sigstore.github.io/sigstore-conformance/)
- [TUF client conformance results, 2026-10-30](https://theupdateframework.github.io/tuf-conformance/)
- [Sigstore API stability policy](https://docs.sigstore.dev/about/api-stability/)

## Required verifier architecture

### A. DSSE envelope layer

The verifier must check the DSSE v1.0 envelope with the exact Pre-Authentication Encoding over payload type and payload bytes:

`PAE(type, body) = "DSSEv1" SP LEN(type) SP type SP LEN(body) SP body`

`LEN` is the decimal byte length, not Unicode character count. The payload type is authenticated by PAE. The caller must consume the exact payload bytes returned from successful verification; never verify bytes and then re-read or re-decode a different envelope field.

Required algorithm/key policy is explicit and allow-listed. Unknown algorithms, malformed signatures, unsupported encodings, non-canonical base64, empty signature arrays and zero thresholds fail closed. Thresholds count distinct trusted principals/keys, never duplicate signature entries. A DSSE `keyid` is an unauthenticated hint for candidate ordering, not an identity assertion or a reason to skip a trusted verifier.

### B. in-toto statement/predicate layer

After envelope verification, parse the exact verified bytes into the expected in-toto Statement v1. Reject malformed JSON, ambiguous duplicate object keys, unknown major versions, payload type mismatch, subject digest mismatch and predicate type/version downgrade. Preserve exact raw bytes and digests for audit.

The Rust in-toto binding may be used for generated data structures, but semantic validation remains explicit and version-pinned.

### C. Issuer/policy layer

Resolve signing keys to verifier-controlled principals and role scopes, and verify validity intervals, rotation, revocation, delegation and trust-bundle version. A signature's identity is not a trust decision. The independence policy must compare actual administrator/control domains and delegation paths; do not trust self-declared strings in the signed predicate.

Replay controls require verifier-owned nonce consumption and issuer sequence state. Signed expiry and clock-skew bounds are necessary, but timestamps alone do not prevent a replay.

### D. Episode semantics layer

The service-resolution rule is conjunctive, not a signature-count scalar:

- coordinator attestation: exact plan and dispatch history;
- participant attestation: authoritative attempt-specific effect state;
- authority attestation: fresh exact grant;
- independent-observer attestation: fresh target/service postcondition from a distinct control domain.

The participant and observer signatures cannot be substituted by additional coordinator keys. A signed `outcome_unknown`, eventual-consistency read, stale observer sample or mismatched postcondition remains indeterminate.

## Implementation spike acceptance

Before adding a dependency to production code:

1. Verify the DSSE v1.0 published PAE test vector.
2. Verify independently generated cross-language Ed25519 envelopes (Go and Rust), including raw binary payload and non-ASCII payload type byte-length cases.
3. Reject payload type confusion, payload tampering, signature tampering, wrong key/algorithm, non-canonical base64 and malformed envelope.
4. Prove threshold counts unique authorized keys/principals; duplicated signatures from one key cannot satisfy a multi-role policy.
5. Prove `keyid` tampering cannot force acceptance or denial by bypassing a valid candidate key.
6. Verify the returned payload bytes are exactly the bytes consumed by the in-toto parser.
7. Reject duplicate JSON keys, unknown major versions, unsupported predicate versions and subject/predicate digest mismatch.
8. Test revoked/rotated keys, stale trust bundles, wrong issuer scope, shared coordinator/observer control domain, nonce replay, duplicate sequence, expiry and future-clock skew.
9. Compare two independent verifiers on the exact same corpus. Disagreements are hard failures and must be resolved before production adoption.
10. Pin the crate version and checksum, audit transitive dependencies/licenses, document security support policy and monitor advisories.

Hard gates: 100% pass on official protocol vectors; 100% pass on the independent cross-language vectors; 0 acceptance of any negative control; deterministic output agreement across independent implementations. Report false rejects separately. No benchmark score can compensate for a single cryptographic or trust-policy acceptance failure.

## Current conclusion

- **No production crypto crate is selected in this PR.**
- Official in-toto Rust bindings are a data-format option only.
- Sigstore Rust is a candidate for keyless identity and transparency integration, gated on stable/released API and direct bundle conformance.
- The archived standalone `dsse` crate is not selected for production, despite its useful test design.
- Any later custom DSSE envelope implementation must be treated as security-critical code, independently reviewed, and tested against published plus cross-language vectors.

This ADR is a source-selection assessment, not a cryptographic implementation or production security certification.
