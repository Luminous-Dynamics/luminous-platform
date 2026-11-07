# OMSI Manufacturing Replay Envelope 0.1

**Status:** incubation contract for diagnostic replay, not a signed attestation standard  
**Implementation:** `tools/omsi-manufacturing-validator/src/replay.rs`  
**Schema:** [replay-envelope.schema.json](../../../tools/omsi-manufacturing-validator/schema/replay-envelope.schema.json)

## Purpose

Bind a semantic report to the exact input artifact evaluated and provide a reproducible way to re-run the evaluator against that artifact. This profile is intentionally limited to **raw-byte identity and replay consistency**.

## Contract

An `omsi-manufacturing-replay-envelope/0.1` contains:

- `envelope_contract`: this contract ID.
- `artifact_binding.digest_algorithm`: `sha256`.
- `artifact_binding.digest`: lowercase hexadecimal SHA-256 over every byte of the input artifact.
- `artifact_binding.digest_scope`: `exact_input_bytes`.
- `artifact_binding.canonicalization`: `none`.
- `artifact_binding.byte_length`: source artifact length in bytes.
- `report`: one `omsi-manufacturing-semantic-report/0.1` report.
- `limitations`: the exact security and interpretation limitations emitted by the versioned producer.

The verifier checks contract and shape, binding profile, byte length, digest, JSON parsing, equality between the embedded report and a fresh semantic evaluation, and preservation of the required limitations. It reports `replay_consistent` only when each check passes. Otherwise, status is `rejected`.

## Why the digest covers raw bytes

The contract deliberately does not normalize or reserialize the input before hashing. Whitespace, UTF-8 encoding, and property-order changes therefore change the digest even when two JSON documents parse to equivalent values. This removes ambiguity about what artifact is bound without claiming cross-language canonical JSON behavior. RFC 8785 defines a separate JSON Canonicalization Scheme; this contract does not implement or claim conformance to RFC 8785.

## Threat model and non-goals

This envelope is **unsigned**. It may detect accidental changes or altered reports when a verifier has a preserved copy of the original envelope, but it cannot authenticate the source or envelope producer. A party able to replace both the input and envelope can recompute the digest. Distribution that needs authenticity requires a separately specified, reviewed signature/attestation profile and trust policy; the present implementation does not provide one.

A `replay_consistent` result establishes agreement between the recorded report and a fresh run of this validator over the exact supplied bytes. It does not prove that the source data is true, that calibration is authentic, that an independent implementation agrees, that the input is fresh, or that a measurement conforms to a quality rule. It does not authorize product release, process changes, safety decisions, PLC/SCADA writes, or equipment actuation.

## Research references

- [RFC 8785 — JSON Canonicalization Scheme](https://www.rfc-editor.org/rfc/rfc8785.html), cited to distinguish canonical semantic-object digests from this raw-byte digest.
- [in-toto Attestation Framework](https://github.com/in-toto/attestation/blob/main/spec/README.md), a reference for a future subject/predicate/envelope/bundle design if authenticated exchange becomes a requirement.
