# Independent runtime enforcement receipts v0.1

## Purpose

This layer closes the gap between a frozen policy and evidence that a sandbox actually enforced it. It is an additive certification check for issue #196, stacked on the sealed-run and matched-baseline contract.

The validator consumes the frozen manifest, authenticated freeze receipt, scenario receipts/reveals, raw trial events, and source-authenticated runtime observer receipts. Its preferred entry point derives the trial ledger from raw events; callers cannot submit a self-constructed summary as certification evidence.

This code is a **structural evidence validator**, not a sandbox, cryptographic verifier, or host attestation system. It does not prove that a kernel, hypervisor, firmware or hardware actually enforced isolation. The source-evidence adapter must authenticate all statements before they reach this validator.

## Evidence chain

For each preregistered scenario and arm:

1. The run manifest freezes candidate and baseline identities, evaluator, execution environment, resource budget, state-isolation policy, artifact-access policy, egress policy, runtime observer identity/adapter, and runtime challenge authority.
2. The independent freeze authority authenticates a freeze receipt that repeats those exact digests and the manifest commitment.
3. The scenario source publishes one case payload commitment and a separate private-oracle commitment.
4. The trial ledger derives exactly one complete Candidate and Baseline attempt history from source-authenticated events.
5. A distinct runtime challenge authority issues a fresh nonce after the run freeze and before measurement starts. The challenge is single-use, short-lived, bound to the exact run/arm/trial through its receipt, and must remain valid through measurement completion, receipt issuance and the declared clock uncertainty.
6. The independent runtime observer measures the full trial interval and emits one receipt per (scenario, arm).
7. The verifier binds the runtime receipt to the exact run, trial, arm, agent/baseline identity, scenario commitment, policy/environment/budget digests, challenge authority and freeze commitment.
8. Any missing receipt, policy violation, namespace collision, sequence gap, partial coverage or non-overlapping measurement interval rejects the certification evidence.

No caller-supplied `signature_verified`, `sandbox_enforced`, `coverage_verified` or equivalent boolean is accepted. The source-evidence adapter authenticates the signer, subject, predicate, source identity, trust domain, freshness, sequence and policy digest through the existing typed evidence path (#190/#191). The structural validator then checks the semantic bindings and counters.

## What the runtime observer measures

Each `RuntimeTrialAttestation` binds commitments to:
- the complete process tree and child-process coverage;
- mount namespace and filesystem/object access log;
- network egress log and runtime event stream;
- runtime instance and writable-state namespace;
- the declared environment, resource budget and all three enforcement-policy digests.

It also records:
- coverage state: `complete`, `partial`, `unknown`, or `tampered`;
- sequence gaps and dropped event counts;
- oracle read attempts/successes;
- unauthorized artifact read attempts/successes;
- disallowed egress attempts/successes;
- cross-trial state and shared writable namespace observations;
- exact allowlisted egress event count;
- receipt sequence, challenge nonce, issue/expiry, measurement interval and clock uncertainty.

The strict sealed lane rejects any oracle read attempt, prohibited artifact read attempt, disallowed egress attempt, cross-trial state observation or shared writable namespace observation—even if the attempted action was denied. A blocked attempt is a useful security event, but it is not a clean sealed run.

Allowed egress is not inferred from its counter. The authenticated adapter must verify each event against the frozen exact allowlist, including destination identity, resolver, request/response digest and time. The receipt's egress-log commitment is only a reference to evidence; it is not itself proof that the destination was permitted.

## Ledger and timing validation

- The preferred `validate_runtime_attestations_from_events` function accepts the freeze receipt and raw scenario/trial events, validates the freeze and builds the sealed ledger internally.
- Candidate and Baseline trial counts, clusters, dispositions, attempts and first-arm execution order are re-derived and checked against the ledger summary.
- Every planned (trial, Candidate) and (trial, Baseline) pair requires one and only one runtime receipt.
- The receipt measurement start must conservatively precede the first arm activity and its end must conservatively follow the final terminal event after applying the declared clock uncertainty.
- Observer receipt sequences must be unique and contiguous for the run. Issued-time ordering may overlap only within declared clock uncertainty.
- Runtime instance and writable namespace digests are globally unique across all arm/trial evidence, including cross-type collisions.
- Challenge nonce digests are nonzero and unique. The challenge must be issued after freeze, before measurement, expire after measurement and receipt issuance plus uncertainty, and stay within the one-day maximum TTL.
- The challenge authority, runtime observer, candidate, baseline, evaluator, freeze authority and scenario evidence source have distinct control domains.

A nonce digest is not a substitute for signature validation. Its authority, issuance time, binding and anti-replay status must be authenticated by the external source-evidence adapter and its durable replay guard.

## Claim ceiling

A passing validator means only:

> The supplied source-authenticated records are structurally complete, bound to the frozen run, internally consistent, and report no prohibited runtime access or evidence-coverage failure.

It does **not** prove that:
- the runtime observer is honest or independent merely because its record names a different control domain;
- an eBPF, ptrace, LSM, hypervisor or guest agent saw every relevant kernel/object/network event;
- the host kernel, hypervisor, firmware or hardware is free of bypasses or side channels;
- a zero counter is meaningful if a sensor was disabled, bypassed, overwhelmed or misconfigured;
- the disclosed access-log commitment corresponds to the actual log bytes until its source-authenticated artifact is retrieved and verified;
- the agent is secure, contamination-free or better than SOTA.

A real deployment qualification requires adversarially tested sandbox/runtime adapters, external attestation of the measuring stack, independent known-answer tests, and fault injection against telemetry loss and bypass. Do not turn structural success into a general sandbox-security claim.

## Implementation choices and interoperability

Do not build another container runtime or invent a signature format.

- gVisor is an established application-kernel sandbox option. Its documentation describes syscall interception and defense-in-depth, but also notes that hardware side channels, host mechanisms and network policy remain relevant boundaries: https://gvisor.dev/docs/architecture_guide/security/ and https://gvisor.dev/docs/architecture_guide/intro/
- Falco documents explicit syscall event-drop counters and warns that dropped events can compromise its internal process/file/container state: https://falco.org/docs/concepts/event-sources/kernel/dropped-events/ and https://falco.org/docs/troubleshooting/dropping/
- SPIFFE Workload API/SVID provides workload identity distribution and rotation primitives; it does not attest that the process's runtime behavior was isolated: https://spiffe.io/docs/latest/spiffe-specs/spiffe_workload_api/
- in-toto Attestation provides predicate/statement/envelope separation and DSSE-compatible authenticated envelopes. Use its subject binding and envelope validation via the existing source-evidence adapter: https://github.com/in-toto/attestation/blob/main/spec/README.md and https://github.com/in-toto/attestation/blob/main/spec/v1/envelope.md
- SLSA provenance is useful for build origin and artifact identity, not as a substitute for runtime isolation evidence: https://slsa.dev/spec/v1.2/provenance

Recommended adapter shape: use the platform's existing typed evidence roles and source registry; put the runtime measurement predicate in an in-toto Statement/DSSE envelope, authenticate the observer with SPIFFE identity, and bind the runtime's own event-loss telemetry. Avoid adding a parallel trust store, signing implementation, policy engine, durable journal or replay database.

## Conformance and next experimental work

The Rust suite includes positive clean-run controls, exact trial/arm and ledger binding, challenge freshness/uniqueness/expiry, observer identity separation, clock uncertainty, sequence contiguity, namespace uniqueness, coverage gaps, oracle/artifact/egress attempts, and a 72-mutation adversarial matrix.

These are synthetic conformance tests, not measurements of a real sandbox. The next empirical track should run the same synthetic agent workload under at least:
1. native Linux namespaces + seccomp + Landlock/AppArmor where supported;
2. gVisor/runsc;
3. disposable KVM/QEMU VM isolation.

For each, measure startup/steady-state overhead, CPU/memory, syscall-heavy overhead, event loss under increasing load, oracle-access detection, cross-trial state leakage, unauthorized egress detection, false-positive/false-negative rates, and recovery of evidence after abrupt runtime termination. Pin versions and host configuration. Do not combine security and performance into a single score.

Architecture ownership remains unchanged: Platform Saga coordinates the episode; broker enforces mutation authority; participant-owned ledgers own remote effects; independent workload/business observers own service outcomes; Nixward remains the sole host transaction/interlock/journal/recovery authority.
