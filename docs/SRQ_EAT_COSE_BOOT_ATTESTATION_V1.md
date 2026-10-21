# SRQ-1 EAT/COSE boot-observation profile v1

**Status:** protocol design / CDDL draft. No D1 signer, COSE verifier, TPM quote verifier or release-gate integration is implemented by this document.

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

## Next implementation sequence

1. Freeze and review this profile with a specific target-key provisioning model before adding signing code.
2. Implement EAT/COSE parsing and verification with a maintained library, strict algorithm allowlist, exact signature verification, key-ID policy, nonce freshness and deterministic negative tests.
3. Return a non-serializable typed verifier capability from the successful challenge-consume path; make rollout evaluation require that capability rather than caller-supplied freshness_valid=true or nonce_durably_consumed=true booleans.
4. Implement the D2 TPM quote profile separately and keep its acceptance corpus separate from D1.
5. Integrate with the rollout journal and measure actual booted state plus independent service behavior on QEMU, then on physical devices.

**No superiority claim is made.** The hypothesis is that Luminous can tie authorization, durable promotion, observed boot state, service behavior and auditable recovery evidence together more reliably than a disconnected collection of component checks. Prove it on a matched fault corpus and fail closed on every missing claim.
