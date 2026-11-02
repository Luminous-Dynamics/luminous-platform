# Independent runtime enforcement receipts v0.1

## What changed

This module closes a gap between a frozen policy and evidence that the policy was actually enforced during a trial. The sealed run manifest now pins:
- a runtime observer source, adapter digest and separate control domain;
- a distinct runtime challenge authority identity and control domain;
- state-isolation, artifact-access and network-egress policy digests;
- the common execution environment and resource-budget digests.

Each planned (trial, Candidate) and (trial, Baseline) arm requires exactly one RuntimeTrialAttestation.

## One-use freshness challenge

Every runtime attestation carries:
- a 32-byte nonzero challenge nonce digest;
- the frozen challenge-authority identity and control domain;
- challenge issue and expiry times;
- the runtime measurement start/end and receipt issue times.

The structural contract requires the challenge to be issued after the run freeze, before runtime measurement starts, and to remain valid through measurement completion and receipt issuance. Nonce digests must be unique across all arm trials in the run. This rejects stale replay and nonce reuse at the structural layer.

The source-evidence adapter must authenticate the challenge authority, verify the challenge signature and exact run/trial/arm/observer/policy subject binding, and enforce one-use nonce consumption through the existing durable replay guard (#191 / issue #190). A self-reported nonce is not cryptographic freshness evidence by itself.

## Runtime observation requirements

A valid receipt binds:
- exact run, trial, arm, candidate/baseline identity, scenario commitment and manifest commitment;
- evaluator, execution environment, resource budget and all three enforcement policy digests;
- runtime instance and writable namespace digests;
- the independent observer and adapter;
- process tree, mount namespace, filesystem/object-access, network-egress and runtime-event log commitments;
- complete coverage, zero sequence gaps and zero dropped events;
- clock uncertainty no greater than the contract limit;
- zero oracle-read attempts or successes;
- zero unauthorized-artifact read attempts or successes;
- zero disallowed-egress attempts or successes;
- zero cross-trial state observations and shared writable namespace observations.

A denied attempt is still counted and fails the strict sealed policy. Allowlisted egress remains separately countable; the authenticated runtime adapter must resolve the exact destinations against the frozen allowlist and bind its event log to the attestation. Missing allowlist evidence is not equivalent to zero egress.

Exactly one nonzero runtime-instance digest and one nonzero writable-state-namespace digest are required per arm trial; neither may be reused across another arm or trial.

## Join and denominator rules

The preferred `validate_runtime_attestations_from_events` entry point derives the sealed candidate/baseline trial ledger directly from the raw authenticated trial-event stream via `validate_run_ledger_with_reveals`, then joins the runtime attestations to that derived ledger. This is the certification path: it avoids treating a caller-constructed summary as primary evidence. The lower-level composition function remains available, but its supplied ledger must be the exact output of the sealed event validator.

The validator rechecks the frozen manifest, freeze receipt and scenario bindings; it independently recomputes candidate/baseline trial summary counts, declared attempts, cluster count and timeline shape. It does not trust caller-supplied aggregate counts.

For every planned scenario and arm:
- exactly one trial result and exactly one runtime attestation must exist;
- attestation source, control domain, adapter, challenge authority and nonce must match frozen identities;
- the monitoring interval (including the declared clock uncertainty) must enclose the first activity through the final terminal event;
- the challenge must cover the measurement interval;
- runtime instance, writable namespace, nonce and observer sequence must be unique and run-contiguous;
- all policy violation counters and telemetry-gap counters must be zero.

Missing, duplicate, unknown, stale, expired, replayed, substituted, partial or tampered evidence is rejected. These are hard gates, not terms in a compensatory score.

## Claim ceiling

This is a structural receipt validator. It is not a kernel/hypervisor enforcement proof and does not implement signatures, hardware attestation, challenge issuance, nonce persistence, or runtime probes. It cannot establish that eBPF/IMA/LSM hooks were attached or that events could not be suppressed before collection. The source-evidence adapter must authenticate receipts and challenge issuers, while an independent runtime adapter must prove its own coverage using platform measurements and positive/negative sensor controls.

The current target adapters should use established mechanisms rather than custom cryptography:
- verifier-issued nonce/challenge and independent relying-party verification;
- platform and workload-image measurements, not only one VM boot claim;
- runtime integrity/event-log evidence and verified log replay;
- CPU/GPU/workload attestation separately where those devices are in scope;
- privacy-preserving audit commitments that do not log prompts, model weights, keys or customer data.

The benchmark should measure observer false negatives and detection latency using injected known-answer events, hook detach/failure, sequence gaps, event loss, process-tree escapes, filesystem/object access, DNS/egress and deliberate cross-trial cache probes. A monitor that fails silently must invalidate the run.

## Reference alignment

The design follows the emerging continuous-attestation direction: trust at launch is insufficient for mutable workloads, and runtime integrity, behavior and transport bindings must be continuously checked. It also follows the established separation between measurement, appraisal and relying-party authorization. IMA measurement does not itself enforce file policy; dm-verity enforces content integrity; the verifier must replay ordered measurements against a fresh quote and independently approved values. The runtime verifier therefore treats evidence collection and policy enforcement as distinct components.

## Architecture boundaries

- Platform Saga coordinates the episode.
- The broker enforces tool/mutation authority.
- Participant-owned ledgers own remote effect truth.
- Independent workload/service observers own business outcomes.
- Nixward remains the sole host transaction/interlock/journal/recovery authority.
- The runtime observer is a measurement source, not another policy engine, participant ledger or journal.


## Exact-head conformance

The runtime validator has positive clean-run controls and negative controls for forged ledger summaries, missing/duplicated arm receipts, challenge reuse/expiry/authority substitution, observer/control-domain substitution, exact manifest/policy binding, unique runtime and namespace identity, measurement coverage including clock uncertainty, sequence start/order/contiguity, partial/unknown/tampered coverage, oracle/artifact access, forbidden egress, state carry-over and missing scenarios. The source-event certification entry point is tested independently of the supplied-summary composition path.

The 60-vector runtime mutation matrix is deterministic and synthetic. A green CI run proves the encoded structural conformance tests, not a real sandbox security guarantee.
