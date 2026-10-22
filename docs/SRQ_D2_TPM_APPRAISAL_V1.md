# SRQ D2 TPM measured-boot appraisal v1

**Status: design specification and issue #105; no TPM quote verifier is implemented or qualified by this document.** D1 software-key EAT/COSE remains the only implemented signed-observation profile. D2 must be a separate verifier policy, not a claim toggle.

## 1. Paradigm shift: attest measurements, then appraise their meaning

A signature proves that a key signed a message. A hardware TPM quote can bind selected Platform Configuration Register (PCR) values to a TPM-held attestation key and a verifier challenge. It still does not, by itself, prove those measurements are acceptable or map them to the desired deployment. RATS separates the Attester, Verifier and Relying Party roles; the verifier must appraise evidence against authenticated endorsements, reference values and an explicit policy.

Official design anchors:

- IETF RFC 9334, Remote ATtestation procedureS (RATS) Architecture: https://www.rfc-editor.org/rfc/rfc9334.html
- IETF RFC 9711, Entity Attestation Token (EAT): https://www.rfc-editor.org/rfc/rfc9711.html
- TCG PC Client Platform Firmware Profile: https://trustedcomputinggroup.org/resources/documents/pc-client-specific-platform-firmware-profile-specification
- TCG Guidance on Integrity Measurements and Event Log Processing: https://trustedcomputinggroup.org/wp-content/uploads/TCG-Guidance-Integrity-Measurements-Event-Log-Processing_v1_r0p118_24feb2022-1.pdf
- TCG PC Client Reference Integrity Manifest (RIM) specification: https://www.trustedcomputinggroup.org/resources/documents/tcg-pc-client-reference-integrity-manifest-specification
- NIST SP 800-193, Platform Firmware Resiliency Guidelines: https://csrc.nist.gov/pubs/sp/800/193/final

The TCG guidance explains that event-log order matters and that replaying events is required to reconstruct PCR values for appraisal. A raw hash or a signed list of self-reported measurements is not equivalent to quote verification plus reference-value appraisal.

## 2. Proposed D2 evidence contract

A D2 request must carry a profile/version identifier and bind, at minimum:

- A verifier-generated, single-use unpredictable nonce (at least 128 bits for this SRQ profile), challenge ID, target scope, audience, decision ID, policy generation and expiry.
- Exact approved artifact digest, intended immutable deployment identity, effective-policy digest and frozen conformance corpus digest.
- TPM quote bytes, attestation-key identifier/credential chain, signature algorithm, selected PCR banks/indices and the returned PCR digest.
- The measured-boot event log and the event-log format/platform profile identifier.
- Authenticated reference-value manifest(s), their issuer/endorsement chain, version and digest, revocation/freshness information and the appraisal-policy version.
- TPM clock information and reset/restart counters, plus the verifier's own challenge issue/expiry context.

Do not treat the event log as trusted simply because it arrived inside a signed outer EAT token. The verifier must independently check the quote and use a declared platform profile to replay/validate the log.

## 3. Verification and appraisal order

1. **Authenticate trust roots.** Validate the attestation-key credential and permitted algorithms against an independently provisioned trust policy. Unknown keys, invalid credential chains, revoked credentials and unsupported quote schemes fail closed.
2. **Verify the quote cryptographically.** Parse the TPM2 quote structure, require the quote attestation type, verify the attestation signature, check the exact PCR selection and compare the quote's qualifyingData to a domain-separated digest binding the fresh nonce, target, decision, intended deployment and policy.
3. **Enforce freshness and target binding.** Verify nonce/challenge uniqueness and expiry against trusted verifier time; reject replay, mismatched target/audience, wrong decision/generation, unsafe clock state and policy-permitted reset/restart counter regressions.
4. **Reconstruct measurements.** Parse only the declared TCG event-log profile. Enforce digest-bank lengths, event bounds and ordering; replay the PCR extend operations from the profile-defined initial values. Independently compute the selected PCR composite digest and require exact equality with the TPM quote's PCR digest. An unavailable, truncated, malformed or unsupported event log is `UNKNOWN/BLOCKED`, not a pass.
5. **Appraise reference values.** Resolve authenticated platform reference values and validate measured firmware, Secure Boot state, boot manager, UKI/kernel/initrd and required policy-controlled components. Reference-value provenance and versioning are part of the decision. A correctly signed quote with unknown measurements is not acceptable.
6. **Bind the intended deployment.** A generic Secure Boot quote does not necessarily identify the NixOS/OSTree userspace deployment or effective policy. Each platform profile must demonstrate how the authorized immutable deployment and relevant policy digest are measured by the trusted boot chain (for example, a supported measured UKI/boot profile). A value supplied only by the running privileged OS is a software assertion, not proof of measurement.
7. **Reconcile lifecycle records.** Join the appraised evidence to the durable promotion decision, target, one-use challenge, exact deployment ID, policy/corpus digests, boot instance and observed booted/current deployment. Use a distinct post-commit observation challenge from the challenge that authorized promotion.
8. **Issue a typed appraisal result.** Only successful appraisal under the pinned D2 policy yields a D2-scoped result. `D1-software-key`, an empty measurement profile, a missing TPM quote, or a model/test fixture must never be coerced into D2.

## 4. NixOS/OSTree deployment binding is platform-specific

The local D0 adapter reads `/run/booted-system` and `/run/current-system`; these paths are useful diagnostics but are not a hardware measurement. A live `nixos-rebuild switch/test` can change the current userspace reference without a reboot. D2 must distinguish the deployment measured at boot from the live userspace state and must not let a self-reported string override the measured profile.

The deployment binding must be proven per platform. It might require measured UKI sections, bootloader/firmware configuration, or an explicitly measured deployment-manifest digest. PCR indices are not globally interchangeable across firmware configurations; the exact PCR selection, bank, firmware profile and reference manifests must be pinned. Do not hard-code a universal PCR map as if it were valid for every OEM or systemd/NixOS configuration.

## 5. D2 falsification corpus and benchmark gates

The following are **proposed targets**, not results:

- 100,000 deterministic mutations across quote bytes/signature, nonce/qualifyingData, PCR selection and banks, quote PCR digest, AK identity, endorsement freshness/revocation, event-log ordering/truncation/digest sizes, reference-value version/signature, deployment identity and decision binding: zero false accepts.
- At least 1,000 deterministic negative cases using an emulated TPM, with exact emulator/version/source recorded. These are emulator-tier tests only.
- Zero replay acceptance for an already consumed D2 challenge, including across process restart and 64 concurrent contenders.
- For every accepted fixture: signature verification passes; quote selection is exact; event-log replay matches the quoted PCR composite; reference values appraise; target/decision/deployment binding matches; and the durable one-use receipt reconciles.
- Report false accepts, false rejects, unavailable/unknown outcomes, p50/p95/p99 appraisal latency, raw input digests, event count/bytes, verifier memory/runtime and reference-policy version.
- Physical D2 claims require real hardware TPM and firmware, independently collected reference values, platform-specific tests and recorded firmware/TPM versions. Emulated TPM, software keys and unit tests cannot satisfy a physical gate.

Any false accept is a release blocker, not a latency trade-off. Unknown evidence must remain unknown/blocked instead of being silently counted as a successful attestation.

## 6. Non-claims and required separation

A valid TPM quote does not prove that every runtime process is benign, prevent post-boot compromise, authenticate reference values without an endorsement chain, or guarantee physical durability. It proves a signature over the selected quote contents under the TPM trust model. Event-log replay, reference-value appraisal, policy freshness and the intended deployment binding are separate requirements.

D2 does not close the production atomic-promotion issue, the live GitHub artifact-attestation issue, or VM/physical recovery gates. The aggregate SRQ result remains `NOT_QUALIFIED` until each gate passes at its declared tier and claim ceiling. Track implementation work under [issue #105](https://github.com/Luminous-Dynamics/luminous-platform/issues/105).
