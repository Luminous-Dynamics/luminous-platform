# Producer-Authenticated Operational Evidence v0.1

**Status:** research profile and unsigned example payload; no production signature verifier, key management, transparency monitor or runtime signer is implemented.

## 1. The assurance claims are separate

This profile separates five claims that are often collapsed into "the log is signed":

1. **Byte integrity:** exact manifest, evidence-segment and policy bytes are bound by SHA-256.
2. **Publisher authenticity:** a cryptographic verifier establishes which identity signed those exact bytes.
3. **Publisher authorization:** a pinned policy authorizes that exact identity for that exact producer source, source kind, trust domain and event vocabulary.
4. **Causal/semantic validity:** the #174 verifier checks the event DAG, source hash chains, grant/attempt bindings, participant receipts, restart scans, observer independence declarations and service outcome.
5. **Outcome truth:** an independent service observer establishes the actual required postcondition.

A signature proves neither that an event is semantically correct nor that an external service recovered. A transparency inclusion proof establishes publication of an attestation, not correctness of its content. The independent outcome oracle remains mandatory.

## 2. Standards composition

Use existing formats and verifiers. Do not introduce custom signature algorithms or a Luminous cryptographic envelope.

- **in-toto Statement v1** binds the attestation to named subjects by digest and identifies the predicate type. The example subject set contains the exact run manifest and one producer-owned event segment.
- **DSSE** carries the serialized statement as an opaque payload. The payload type is application/vnd.in-toto+json. Verify the signature over DSSE pre-authentication encoding (PAE); do not parse and re-serialize the payload before signature verification.
- **Sigstore/Cosign** is the candidate for OIDC-backed publishers such as GitHub Actions. Verification must pin both certificate identity and OIDC issuer, and validate the transparency proof from the bundle. Merely receiving a bundle or reading a claimed verified=true field is not verification.
- **SPIFFE X.509-SVID** is the candidate identity profile for long-running runtime producers such as the coordinator, Nixward adapter, participant and observer. Validate the exact SPIFFE ID, its certificate chain against a pinned trust bundle and certificate validity. If required revocation state cannot be checked, return indeterminate, not accept.
- **Rekor or another explicitly selected transparency log** must provide inclusion and checkpoint-consistency evidence. Monitoring/checkpoint gossip is required to make equivocation/split-view risk observable; a single inclusion proof alone is not a complete transparency-monitoring strategy.

The profile does not assume Cosign's Fulcio/OIDC certificate flow is interchangeable with a SPIFFE X.509-SVID. The runtime SPIFFE-to-DSSE adapter is a separate implementation and interoperability review gate.

Standards references:
- https://github.com/in-toto/attestation/blob/main/spec/README.md
- https://github.com/in-toto/attestation/blob/main/spec/v1/statement.md
- https://github.com/secure-systems-lab/dsse/blob/master/protocol.md
- https://docs.sigstore.dev/quickstart/quickstart-cosign/
- https://spiffe.io/docs/latest/spiffe-specs/x509-svid/
- https://spiffe.io/docs/latest/spiffe-specs/spiffe_workload_api/

## 3. Per-producer segment rule

Every producer signs only its own source stream and epoch. For example:

- Platform coordinator signs coordinator events.
- Authority adapter signs authorization events.
- Nixward signs only its own authoritative host-transaction references and receipts.
- A participant signs its acceptance and committed-effect receipts.
- A participant ledger signs its own attempt-status and effect-reconciliation observations.
- An independent observer signs its own step/service observations.

A producer must never re-sign, proxy-sign or assert authorship over another source's events. Policy maps the tuple (source_id, source_kind, trust_domain_id) to one exact identity profile and allowed event-type set. If one source emits distinct roles (the example has a participant and participant-ledger role for the same ITSM source ID), those roles get distinct identity profile IDs and distinct identities.

Each statement binds:
- the raw manifest bytes;
- the complete event segment for one producer and source epoch;
- the exact publisher-policy bytes via publisher_policy_sha256;
- the exact run/episode/plan/source-commit identity;
- the producer role and its source-epoch sequence range and first/last event digests.

A segment begins at source sequence 1 and contains its complete epoch prefix. The checker re-computes each event digest, validates source sequencing and predecessor digest linkage, checks the prior-event happens-before edge, and rejects role or event-type substitution. The full combined evidence bundle is still passed to #174 for cross-source causal and operational semantics.

## 4. Verification sequence

The intended consumer order is:

1. Read the DSSE envelope and verify its signature over the unchanged payload bytes and exact payload type.
2. Validate the signing certificate/identity chain against the pinned trust roots; require exact OIDC issuer + subject for Sigstore, or exact SPIFFE ID + trust-domain bundle for SVID.
3. Require certificate validity at signing time. If revocation/identity status cannot be determined where policy requires it, preserve indeterminate.
4. Verify transparency inclusion and checkpoint consistency under the selected log policy. Reject missing/invalid proof; do not promote inclusion into truth.
5. Decode the in-toto Statement and check its type, predicate type, subject digests and publisher-policy digest.
6. Run check_publisher_policy.py over the decoded statement, raw manifest bytes, exact per-source segment bytes and exact publisher-policy bytes. This is a structural policy/content check only.
7. Reassemble the complete event bundle from individually authenticated source segments and pass it to #174. Do not omit a segment because it failed signature or policy validation.
8. Independently verify service outcome from the external observer. A service-result signature authenticates the observer's statement; the #174 semantic checks still decide whether the observer has the correct role, scope, freshness and causal position.

## 5. Trust policy

The policy must:
- pin exact issuer/subject or SPIFFE ID/trust-domain/trust-bundle digest;
- prohibit wildcard identities and regex-based identity matching;
- bind each producer role to only its allowed event vocabulary;
- require signature validation and certificate validity at signing time;
- require transparency inclusion and checkpoint consistency;
- make revocation uncertainty indeterminate;
- reject unknown publishers, unknown trust domains, mismatched policy digests and missing evidence;
- be separately source-pinned and protected from the system under test.

The checked-in publisher-policy.example.json is example_only. luminous.example and repeated-character trust-bundle digests are fictional fixture values. They are not deployable trust anchors, keys, service accounts or workload identities. The example policy retains the GitHub OIDC issuer in its trusted-issuer allowlist, but this runtime source-segment fixture has no active OIDC publisher entry. Any future CI publisher must add its exact subject for the precise workflow event/ref before signing is enabled.

## 6. Observability is not attestation

OpenTelemetry GenAI spans are useful correlation and diagnostic evidence. Their conventions describe logical operations as observed by the caller, and a logical span may contain automatic retries. They are not the participant's authoritative admission/commit ledger and do not replace effect receipts or independent service observations. Avoid capturing tool arguments/results by default; use references and digests to reduce exposure of secrets and customer data.

## 7. Checker semantics and limits

Run the structural checker:

    python3 tools/operational-recovery-verifier-v0.1/check_publisher_policy.py \
      --statement benchmarks/operational-recovery-v0.1/attestation-statement.example.json \
      --policy benchmarks/operational-recovery-v0.1/publisher-policy.example.json \
      --manifest benchmarks/operational-recovery-v0.1/run-manifest.example.json \
      --event-segment benchmarks/operational-recovery-v0.1/events-platform-coordinator-epoch-0.example.jsonl

status=policy_match means only:
- the raw artifact bytes match the in-toto subject digests;
- the attestation binds the exact publisher-policy bytes;
- the producer role maps to one exact identity profile in that policy;
- the included source segment matches the declared producer, role, epoch, sequence and event digests.

The output explicitly sets signature verification, transparency inclusion verification and full episode semantic verification to false, includes policy_status=example_only for the checked-in fixture, and sets production_usable=false. The checker rejects duplicate JSON keys, non-finite numeric constants, oversized documents/event segments, excessive event counts and overlong event lines. Its size bounds are 2 MiB for a manifest/policy document, 8 MiB for a segment, 4,096 events and 128 KiB per event line. The checker does not authenticate a signature, validate a certificate, contact Rekor, check revocation, establish organizational independence or verify real service state.

## 8. Promotion gates

1. Select and pin the actual DSSE/Sigstore verifier and version; run official positive/negative test vectors.
2. Define the runtime SPIFFE signing and verification adapter; test X.509-SVID rotation, trust-bundle rotation, expiration and trust-domain federation in a disposable environment.
3. Define the log implementation, submission semantics, inclusion proof format, checkpoint consistency monitoring and offline behavior.
4. Have each producer emit its own segment directly from the authoritative boundary, with independent signing authority and no coordinator proxy-signing.
5. Bind the verified segment set into #174 and preserve missing/invalid signatures as invalid or indeterminate, never silently omit them.
6. Keep independent external service observation as the last required truth check.
7. Only then qualify a real Nixward host-operation plus mock-ITSM vertical slice. The profile itself does not change Nixward's journal/interlock or Platform Saga.

No production PKI, key custody, signer or cryptographic integration is delivered here. No security certification or service-restoration guarantee is claimed.
