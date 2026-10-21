# SRQ-1 EAT/COSE boot-observation profile v1

**Status:** protocol design / CDDL draft plus an initial Rust D1 verification prototype in `tools/srq-eat-cose-verifier/`. The prototype verifies a COSE_Sign1 Ed25519 response and challenge bindings, and now exposes a non-serializable post-consumption result type for a supplied store contract. It is still not integrated with a durable production challenge store, a provisioned target signer, TPM quote verification, or an SRQ release gate. The initial dependency lockfile and test outcome must be reviewed before treating the prototype as reproducible or passing.

## Purpose

The D0 collector in tools/srq-boot-observation-adapter/ is intentionally a local diagnostic. Its JSON is not authenticated, does not bind to a promotion decision, and only reads deployment symlink targets. This profile defines the next wire-level boundary using the IETF Entity Attestation Token (EAT) claims model and COSE_Sign1 rather than signing arbitrary JSON or inventing a parallel trust format.

- [RFC 9711: Entity Attestation Token](https://www.rfc-editor.org/rfc/rfc9711.html) defines attestation claims including an EAT nonce and COSE/JOSE security envelopes.
- [RFC 9334: RATS architecture](https://www.rfc-editor.org/rfc/rfc9334.html) separates Attester, Verifier and Relying Party roles and explains freshness and its residual race.
- [SLSA provenance verification guidance](https://github.com/slsa-framework/slsa/blob/main/spec/verifying-artifacts.md) is the companion for build/artifact provenance; it does not prove runtime boot state.

This profile deliberately doesn't equate all signed tokens with equal assurance. A software-held key can authenticate a message to the extent the key remains protected, but compromised privileged software may still lie about its own runtime observations. A hardware-backed quote only qualifies when the verifier validates the quote, nonce, selected PCRs, endorsement chain, reference values, and applicable appraisal policy.

## Required exchange

1. The verifier creates a target-bound, one-use challenge with at least 128 bits of entropy (stricter than RFC 9711's EAT nonce minimum), a unique challenge ID, target token, intended decision ID, expected policy generation, approved artifact digest, expected effective-policy digest, scenario/corpus digest, verifier audience, issue/expiry times, and required assurance tier.
2. The attester collects boot-instance identity, the booted deployment identifier, current userspace deployment identifier, the effective-policy digest from an authoritative source, and the measurements required by the selected assurance tier.
3. The attester emits an EAT claims set encoded with the declared deterministic encoding profile and protected in COSE_Sign1. The protected headers carry a permitted algorithm and key ID; the payload binds the exact verifier challenge and the observations. No free-form field is allowed to override the trust tier, target binding or policy requirement.
4. The verifier checks signature and key policy first, then target/audience/decision/generation/digest bindings, the exact fresh challenge, expiry, assurance level and observation schema. It atomically consumes the challenge and records the verified receipt digest in the durable replay ledger before returning an approval-eligible result.
5. A separate relying party compares the verified result with the planned wave and promotion journal. An independent data-plane probe must follow boot observation. Any mismatch, timeout, missing measurement, failed positive control, expired challenge or indeterminate device state yields HaltAndReconcile, never Continue.
6. The system separately verifies that the target remains in the measured state for the period required by policy. RFC 9334 notes that state can change immediately after evidence is generated; a fresh nonce narrows replay but does not magically make a multi-step external actuation transaction atomic.

## Assurance tiers

| Tier | Evidence | Permitted claim | Not permitted |
|---|---|---|---|
| D0 — local diagnostic | /proc boot ID, /run/booted-system and /run/current-system sampled locally | Diagnostic consistency/mismatch | Authenticated identity, measurement, release pass, or policy actuation |
| D1 — software-key envelope | COSE-signed EAT claims verified against a provisioned key and target registry, fresh one-use challenge and durable consumption | Authenticated message from the holder of the configured key under the selected software key policy | Proof that privileged OS observations are truthful; TPM/secure-boot or physical qualification |
| D2 — hardware-backed measurement | TPM quote or equivalent hardware-rooted evidence, fresh nonce, validated endorsement chain, explicit PCR selection/reference values and policy appraisal | Only those measured boot/runtime properties covered by the actual quote profile | General “device is secure” or “the whole system is correct” |
| D3 — end-to-end qualified deployment | D2 (where available), exact artifact/policy binding, post-boot reconciliation, independent service probe, recovery proof and matched fault evidence | Only the named deployment, target and tested failure classes | Unmeasured hardware/firmware states or untested power-loss classes |

Do not silently upgrade D0 to D1 because its JSON is archived or a workflow attestation signs the log. Workflow attestation authenticates build/workflow provenance of the file, not the physical source of the boot observation.

## Required claims and validation rules

The machine-readable draft is in schemas/srq-eat-boot-observation-v1.cddl. Semantic policy must require all of the following even if an EAT profile generally allows optional claims:

- EAT nonce/challenge ID and verifier audience; single use; challenge expiry and exact binding to target, decision ID and expected generation.
- A verifier-recognized target identity. Avoid placing a global hardware UEID into general-purpose logs; use a verifier-scoped token where possible.
- Approved artifact digest, expected and observed effective-policy digest, corpus/profile digest, generation and the unique decision ID.
- Boot instance ID plus booted deployment identity and current userspace deployment identity, reported distinctly.
- A tier enum and tier-specific measurement claims. A software-path string must not be labelled a measured artifact digest.
- Key ID, algorithm and endorsement/reference-value policy obtained from the verifier's trust configuration, not trusted merely because the token claims them.
- Explicit unknown/unsupported status. Unknown evidence must not be treated as a passing boolean.

For any field with a digest-shaped representation, validate exact length and lowercase hexadecimal syntax before comparing to an independently expected digest. Use the standard COSE signature input and deterministic encoding rules for the selected EAT profile. Never sign a map after a lossy parse/re-serialization step unless the profile explicitly defines that canonicalization.

## Mandatory adversarial tests before D1 can be accepted

1. Replay the same token/challenge after successful durable consumption; the verifier must reject it.
2. Substitute source target, target token, audience, decision ID, generation, approved artifact digest, effective-policy digest, corpus digest or workflow/key ID; reject each independently.
3. Alter a single payload/header/signature byte; reject.
4. Use an expired challenge, a challenge not yet valid, a missing nonce, or a nonce with insufficient entropy; reject.
5. Use an unknown key, wrong algorithm, untrusted key rollover, unsupported profile, unknown critical claim, malformed COSE, duplicate/ambiguous claim, or invalid CBOR; reject.
6. Provide valid D1 software-key evidence claiming a TPM tier; reject the claimed assurance upgrade.
7. Provide a valid signed D0-style observation that says booted/current paths diverge; preserve the authenticated negative and return HaltAndReconcile, never reinterpret the signature as a pass.
8. Simulate privileged OS compromise, TPM quote mismatch, PCR/reference-value mismatch, boot measurement from the wrong boot instance, and state change immediately after quote generation. Document which are detected by D1 versus D2.
9. Kill the verifier before and after challenge consumption/receipt commit. On reopen, the outcome must be uniquely reconcilable by challenge ID and decision ID.
10. Keep raw token digest, signer/attester identity policy, verifier version, trust-bundle digest, reference-value digest, boot instance ID, source SHA and execution tier in the evidence record.

## Initial D1 verifier prototype (2026-10-21)

The new Rust crate uses the maintained `coset` COSE implementation and Ed25519 verification rather than inventing a signature envelope. Its intended initial contract is deliberately narrow: It pins the fully specified COSE `Ed25519` algorithm identifier (-19), as defined by RFC 9864, instead of the older polymorphic `EdDSA` identifier (-8).

- Accept only protected COSE `alg=EdDSA` and the key ID selected by the external trust policy; reject unprotected headers, detached payloads, unknown claims, duplicate CBOR claim keys, oversize tokens and malformed encodings.
- Bind the EAT nonce to a verifier-created 16–64 byte nonce; require the exact challenge expiry, verifier audience, provisioned target token, decision ID, logical generation, artifact/policy/corpus digests and immutable expected deployment identity.
- Keep `booted_deployment_id` distinct from `current_userspace_deployment_id`; mismatch or divergent/unavailable observation is a rejection.
- Accept only `D1-software-key`. It rejects D2 claims and non-null measurement claims; a signed assertion is not silently upgraded to hardware measurement.
- The plain CLI output is deliberately `ENVELOPE_AND_BINDINGS_VERIFIED_NOT_REPLAY_SAFE`. `VerifiedObservation` contains no editable replay-safe or release-eligible boolean. A successful signature check alone does not consume a nonce.
- `verify_and_consume` now returns a separate `ConsumedObservation` with private fields and no Serde serialization implementation, and only after the configured `ChallengeConsumer` commits and returns a syntactically valid SHA-256 receipt digest. The store request carries the complete target/audience/key/profile/challenge/decision/generation/artifact/policy/corpus/deployment/boot-instance/validity/envelope tuple.
- The unit-only in-memory store computes its synthetic receipt digest over that complete tuple so tests exercise the binding contract. That digest is **not** evidence of durable persistence. A real store must validate the full request against the persisted pending challenge/decision, re-check expiry using its trusted time source inside the transaction, atomically consume the nonce and commit the receipt, then return the digest of the receipt bytes actually persisted. A fake consumer that returns a digest is not production proof.

The prototype's test fixture private key is deterministic and exists only inside unit tests. It is not an operational key, a target provisioned key, a hardware key or evidence from a physical host. The trust-policy JSON and verifier-controlled challenge file must be supplied through an independently protected channel. The CLI takes an explicit verifier time argument; the target's `iat` does not establish trusted time.

The workflow stages the D1 test log inside the deterministic SRQ evidence bundle as a supplemental `model` observation with the claim ceiling `cryptographic_test_fixture_only`. Exact-head run [39549956474](https://github.com/Luminous-Dynamics/luminous-platform/actions/runs/39549956474), on `06904fa2b33b981426cbb5da13b320ccb426c74d`, passed the archive validation, wrapper and D1-digest negative controls with the then-current 15-test D1 verifier suite. The newer strict trailing-CBOR-bytes rejection test is being validated separately; its result must not be inferred from that earlier run. The actual main-branch attestation has not been minted or verified. No release gate may consume the D1 fixture result.

## Next implementation sequence

1. Review the D1 verifier API and confirm compatibility with the selected EAT/COSE profile and trust policy; add cross-implementation vectors from a second COSE implementation.
2. Commit and review the generated Cargo.lock, then require `--locked` for all verifier builds and tests.
3. Provision a target-bound signing key and define enrolment, revocation, rotation, recovery and verifier-policy governance. Test key rollover and revoked-key rejection.
4. Implement a durable challenge-consumption adapter as one transaction with the verifier receipt. Return an approval-eligible, non-serializable capability only after the transaction commits; never accept caller-supplied booleans as proof of freshness or consumption.
5. Integrate the signed verifier receipt into the SRQ evidence bundle with source/schema/trust-policy digests and verifier version; keep the result D1 / software-key-only.
6. Implement the D2 TPM quote profile separately with endorsement chain, approved PCR selection/reference values and a separate acceptance corpus.
7. Integrate with the rollout journal and measure actual booted state plus an independent service probe on QEMU, then physical devices.

**No superiority claim is made.** The hypothesis is that Luminous can tie authorization, durable promotion, observed boot state, service behavior and auditable recovery evidence together more reliably than a disconnected collection of component checks. Prove it on a matched fault corpus and fail closed on every missing claim.
