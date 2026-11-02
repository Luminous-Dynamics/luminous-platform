# Portable offline evidence bundle and claim verifier v0.1

## Purpose

This layer turns the OpsBench result from a repository-specific report into a portable, content-addressed evidence object. It transports and structurally verifies existing evidence; it does not create a new participant ledger, policy engine, signature format, trust store or journal.

The bundle covers the frozen run identity, planned Candidate/Baseline subjects, evidence inventory, trust-policy snapshot and claim graph. It can be checked without contacting mutable URLs or retrieving active sealed scenario/oracle bytes.

## What the Rust verifier checks

The validator at src/bundle.rs checks:

- exact bundle schema/version, run ID, manifest commitment, Candidate/Baseline/evaluator/verifier digests and trust-policy digest;
- offline verification mode only;
- exactly one binding for each of eight distinct run domains: Candidate, Baseline, Evaluator, Freeze Authority, Evidence Source, Runtime Observer, Runtime Challenge Authority and Verifier;
- full planned Candidate/Baseline trial denominator and unique arm subjects;
- unique artifact IDs, content digests, archive paths and source sequence tuples;
- trusted publisher ID, key ID/digest, source and control-domain match to the supplied trust-policy snapshot;
- publisher and trust-policy validity intervals;
- embedded artifact byte length and digest against independently materialized bytes supplied by the artifact-verification adapter;
- canonical relative archive member paths (no absolute paths, parent traversal, empty components, backslashes or drive prefixes);
- no mutable external references and no silent online fetch;
- public-bundle restrictions for active sealed payloads, private oracle bytes and restricted-private artifacts;
- retired reveal lane and time ordering;
- required artifact coverage, claim subject binding, evidence references, dependency existence, cycle rejection and dependency-status constraints.

## Bundle disclosure rules

Artifacts use explicit sensitivity and delivery metadata.

| Sensitivity | Public bundle | Restricted bundle |
|---|---|---|
| Public / operational evidence | Embedded or commitment-only | Embedded or commitment-only |
| Active sealed payload | Commitment-only or unavailable | Embedded only under a separately authorized restricted distribution policy; v0.1 does not itself authorize access |
| Private oracle | Commitment-only or unavailable | Embedded only under a separately authorized restricted distribution policy; v0.1 does not itself authorize access |
| Retired reveal | Only after retirement in the retired-reproducibility lane | Same timing rule |
| Restricted private | Never embedded in a public bundle | Requires a separately governed restricted distribution path |

v0.1 is deliberately stricter at the public boundary: active sealed payloads and private oracle bytes cannot be embedded in a public bundle. A commitment is not the underlying secret. A retired reveal must refer to an original scenario/oracle commitment, with retirement preceding bundle creation and reveal publication.

An artifact with delivery=unavailable remains visible in the inventory. If required, it makes the verification receipt Indeterminate. It is never converted to zero, omitted or fetched from an external URL.

## Claim graph semantics

Each claim has a stable ID, kind, Pass/Fail/Indeterminate status, exact run/manifest/trial/arm/scope subject, evidence artifact IDs, dependency claim IDs, assumptions and a claim ceiling.

- Every artifact marked required must be referenced by at least one claim.
- Missing evidence, missing dependencies, duplicate IDs, mismatched subjects and graph cycles are rejected.
- A Pass claim cannot depend on a Fail or Indeterminate claim.
- An unavailable required artifact cannot support a Pass claim.
- A valid Fail remains Fail in the resulting receipt. A required missing artifact or an Indeterminate claim makes the bundle Indeterminate unless a separate Fail is present.
- No parent claim can turn a non-Pass dependency into a Pass. No compensatory score is calculated.

The verifier returns artifact counts and claim counts by disposition. It does not rank agents or translate external benchmark scores.

## Trust and cryptographic boundary

The bundle carries a pinned trust-policy digest. The separately supplied TrustPolicySnapshot pins:
- trust roots digest;
- revocation-policy digest;
- verifier ID/version/digest and control domain;
- policy validity interval and maximum artifact size;
- publisher IDs, key IDs/digests, source IDs, control domains and validity intervals.

The existing typed source-evidence adapter must authenticate the trust-policy snapshot and publisher attestations. It must verify DSSE/in-toto envelopes and exact subjects/predicate types, recompute the bundle digest and every embedded artifact digest from exact bytes, validate key revocation at the stated verification time, and enforce durable source sequence/replay protection. The Rust module only checks the resulting identity, digest, length, timing, source and claim graph structure. It does not parse signatures or implement cryptography.

The supplied expected bundle digest is an input from the authenticated archive-verification path. The structural validator cannot prove the archive bytes have that digest; that recomputation belongs to the offline acquisition/verification adapter before calling this function.

## Offline operation

The format rejects ExternalReference artifacts and accepts only embedded bytes, commitment-only objects or explicitly unavailable objects. The verification process must acquire the bundle first, then disable network access before parsing or resolving any artifact. Missing embedded bytes are errors; they are not fetched.

A successful structural receipt records exact bundle, manifest, verifier, trust-root and revocation-policy digests. To claim offline execution in deployment, the CLI/runtime must be run under an enforced no-network boundary and that environment must be independently attested. The schema's offline mode field alone is not proof that DNS or egress was disabled.

## Reproducibility and privacy

- Public bundles can verify the manifest, commitment and signed evaluation claims without receiving active sealed tasks or private oracles.
- Retired-reproducibility bundles can include a retired reveal only after the exact retirement time and only when the original commitment remains unchanged.
- Archive paths are canonical and relative; path traversal and symlink interpretation are not permitted by the bundle format. The extraction tool must use no-follow/create-new semantics and enforce byte-size ceilings.
- Operational logs and prompts should be minimized. Artifact sensitivity, access classification and retention metadata must not expose secrets in public bundles.
- The bundle is a transport object, not a second journal. Participant effect receipts and observer records remain owned by their existing sources.

## Validation suite

The synthetic Rust suite includes:
- clean public bundle with two embedded evidence artifacts and one private-oracle commitment;
- artifact digest, length, path and publisher-key binding;
- path traversal, absolute path, drive prefix, duplicate path and duplicate digest controls;
- active oracle/payload leakage denial;
- external reference and networked-mode denial;
- trust-policy/verifier substitution and validity checks;
- Candidate/Baseline denominator and run-domain separation;
- retired reveal lane/time checks;
- missing required artifact Indeterminate result;
- claim dependency, missing evidence, duplicate reference, cycle and status-laundering controls;
- an 80-vector adversarial mutation matrix.

These are structural tests over synthetic records, not proof of signature validity, no-network execution or third-party endorsement.

## Claim ceiling

A passing receipt means the supplied and already authenticated bundle records satisfy the encoded structure and all required embedded bytes match the independently computed digest/length/path metadata given to this verifier.

It does not prove that the publisher was authentic unless the external verifier authenticated the source statement; that an offline host had no network unless the CLI was sandboxed; that the oracle was secret during the run; that the runtime monitor observed every event; that the cohort represents production; or that any agent is better than SOTA.

Architecture remains unchanged: Platform Saga coordinates episodes; the broker gates authority; participant-owned ledgers own external effects; independent service observers own outcomes; runtime/Supervisor adapters own measured boundary/resource facts; Nixward remains the sole host transaction/interlock/journal/recovery authority.
