# Configuration-Bound Evidence Manifest and Requalification Graph v0.1

Research snapshot: 10 November 2026.

Status: **typed metadata validator, graph checks, sample schema and example authored; Rust tests and JSON Schema validation not executed**. This increment deliberately does not implement artifact hashing, cryptographic signatures, signer identity, key custody, calibration verification or product qualification.

## 1. The trust problem

The Power Fabric now produces results at several evidence levels: synthetic fixtures, analytical calculations, vendor/paper data, simulation, correlated simulation, HIL, physical measurements and independent laboratory evidence. These cannot be collapsed into a single green check.

A useful evidence package must answer:
- Which raw artifact or simulation output is being discussed?
- Which exact workload and hardware configuration generated it?
- Which board, package, interposer, decoupling, firmware and control revisions apply?
- Which measurement reference plane, instruments, calibration records, bandwidth and clock domains apply?
- Which model, parameter set, solver, boundary conditions and seeds apply?
- What claim is supported, and what is explicitly outside the claim?
- Which downstream results become stale when an upstream component changes?

The evidence manifest is an envelope around the evidence, not proof that the evidence is true.

## 2. v0.1 manifest contents

The new schema and typed Rust model cover:
- Evidence identity, source, UTC timestamp, artifact path and declared SHA-256.
- Evidence class, claim ceiling and configuration identity.
- Exact configuration-item revisions.
- Measurement context with procedure, calibration reference, instrument IDs and bandwidth, reference plane, clock domain and synchronization uncertainty.
- Simulation context with model/solver revisions and hashes of parameter and boundary-condition sets.
- Optional issuer/report references for independent-lab and certification evidence.
- Evidence-to-evidence dependency IDs.

Evidence class controls required metadata. Physical measurement cannot be represented without measurement context and a calibration reference. Simulation cannot be represented without model, solver, parameter and boundary-condition identity. Independent lab and certification evidence require an explicit external attestation reference.

The JSON Schema uses a strict unknown-field policy. The Rust validator additionally checks duplicate configuration-item entries, duplicate dependency IDs, clock uncertainty against its declared budget, SHA-256 string syntax and evidence-class-specific fields.

## 3. Integrity is not authenticity

The current crate has no approved SHA-256 or signature-verification dependency. This increment intentionally does not invent one.

The API accepts a SHA-256 digest computed by an external, reviewed implementation and compares that value with the manifest's declared digest. It does not open or hash artifact_path. A matching digest establishes content equality only if the computation and bytes are trustworthy; it does not prove who collected the data, whether the instrument was calibrated, or whether the model is valid.

Future work must define:
- Exact raw-byte acquisition and digesting before parse/normalization.
- Separate digests for raw and normalized/derived artifacts.
- Signer identity, key rotation/revocation, trust anchors, replay/equivocation protection and recovery.
- Hardware-backed collection identity where justified.
- Independent laboratory authority and separation of roles.

A hash in a JSON file is not a signed measurement.

## 4. Requalification graph

The new Rust API treats each evidence manifest as a node with:
- configuration-revision dependencies; and
- evidence dependencies used to derive the result.

When a configuration item changes, the requalification function:
1. Finds evidence pinned to a different revision of that item.
2. Traverses evidence dependencies transitively.
3. Returns a stable sorted set of affected evidence IDs.
4. Leaves evidence depending only on unrelated configuration items outside the closure.

The synthetic test changes an interposer revision. It invalidates the package measurement and its report, but does not invalidate a separate battery-storage sizing report.

The graph validator rejects duplicate evidence IDs, missing dependency IDs and cycles. This is a conservative dependency closure, not an automated engineering disposition. Human review still decides whether an item can be requalified by analysis, partial retest or full retest.

## 5. v0.1 implementation and limitations

The Rust module tools/measurement-audit/src/evidence_manifest.rs provides:
- validate_evidence_manifest for typed metadata and evidence-class checks;
- compare_computed_artifact_digest for comparing an externally calculated digest string;
- validate_evidence_graph for references and cycle checks;
- requalification_closure for revision-change invalidation.

Nine Rust unit tests are authored for physical measurement metadata, missing calibration, digest matching/mismatch, digest syntax, duplicate configuration items, simulation and attestation requirements, transitive requalification and graph errors. **Tests have not been executed.** JSON Schema validation, serde/JSON parsing, raw-byte hashing, signature verification and key revocation are not implemented or claimed.

## 6. Next increments

1. Choose a maintained, audited SHA-256 implementation supported by the repository's Nix/offline dependency policy; hash raw bytes before parsing and compare against this manifest.
2. Add a strict JSON Schema validator and typed JSON reader, preserving the exact input bytes and parser version.
3. Design a signature and identity threat model before selecting signing formats, keys, trust anchors or attestation hardware.
4. Add standards mapping only after authoritative full-text review and qualified metrology/compliance advice.
5. Connect the requalification graph to BOM, ECAD/package revision, firmware, calibration and model dependency records.
6. Add independent reviewer roles and immutable supersession records; never let a manifest writer self-authorize its claim.

## 7. Safety and claim ceiling

- This is metadata and dependency-graph validation only.
- It does not establish raw-artifact integrity until an external digest computation is independently trusted.
- It does not establish signer authenticity, calibration validity, measurement correctness, independent certification or safety.
- A requalification closure is a review queue, not an automatic product-release decision.
- Synthetic and vendor-reported evidence cannot be promoted to physical or independent-laboratory evidence by editing a field.
- No energization, grid connection, protection setting, package fabrication or physical test is authorized.

## Files

- Schema: schemas/evidence-manifest-v0.1.schema.json
- Example: schemas/evidence-manifest-simulation.example.json
- Rust API: tools/measurement-audit/src/evidence_manifest.rs
