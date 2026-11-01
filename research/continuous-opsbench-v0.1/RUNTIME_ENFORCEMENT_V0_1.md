# Independent runtime enforcement evidence v0.1

## Purpose

This module complements the frozen run/trial ledger in `sealed.rs`. That ledger establishes the declared candidate and baseline identities, scenario/arm histories, shared resource budget, environment and declared sandbox policies. This module requires independent runtime receipts showing that monitoring covered each complete arm execution and that no prohibited access, egress or state carry-over was observed.

The separation is intentional:

- `sealed.rs` establishes the exact planned/evaluated systems and complete candidate/baseline attempt histories.
- `runtime.rs` validates independent runtime observer receipts and joins them to those exact histories.
- The existing typed source-evidence verifier (#190/#191) authenticates issuers, trust domains, subjects, policy digests, freshness and durable replay.
- A real runtime adapter measures the sandbox and emits the receipts. The structural validator does not implement the sandbox.

## Required identity and trust boundary

The run manifest and freeze receipt pin:
- runtime observer source ID;
- runtime observer control domain;
- runtime observer adapter digest.

The observer control domain must differ from candidate, baseline, freeze authority, evaluator and scenario/effect evidence domains. The observer source ID is distinct from the scenario/effect evidence source.

The receipt publisher must be authenticated by the typed source-evidence adapter. The runtime process being measured, the agent, and the component enforcing the policy cannot be the sole source of the attestation. No caller-provided `signature_verified` or `sandbox_enforced` boolean is accepted.

## One receipt per trial arm

Every planned scenario has exactly two arm histories, Candidate and Baseline. This module requires exactly one `RuntimeTrialAttestation` for each (trial ID, arm).

Each receipt binds:
- run, trial, arm and arm identity digest;
- the exact case-payload commitment;
- manifest commitment, evaluator and execution-environment digests;
- shared resource-budget digest;
- state-isolation, artifact-access and network-egress policy digests;
- runtime instance and writable state namespace digests;
- runtime observer source, control domain and adapter digest;
- monotonic observer sequence and measurement interval;
- process tree, mount namespace, filesystem access, network egress and runtime event-stream commitments;
- coverage state, gap/drop counters and violation counters.

Runtime instance IDs and writable namespace IDs must be nonzero and unique across all trial/arm pairs. A fresh writable namespace is required for each pair. Shared read-only artifacts are allowed by policy, but writable caches, home directories or volumes cannot silently leak between arms/trials.

## Coverage interval and time

The runtime measurement must start no later than the first observed activity in the arm and end no earlier than that arm's final terminal event. The receipt itself must be issued after the measurement interval ends.

Clock uncertainty is bounded at 1,000 ms in v0.1. Coverage uses conservative bounds: `measurement_start + uncertainty` must still precede the first arm activity, and `measurement_end - uncertainty` must still follow the final terminal event. Deployments requiring tighter bounds must adopt a stricter profile before evaluation.

Receipt sequence numbers are scoped to one run and observer source. They must be nonzero, unique, strictly ordered by issue time, begin at one and be contiguous across the complete run receipt set. The external durable replay adapter additionally prevents receipt replay across runs. Sequence gaps and dropped runtime events are hard failures.

A skip is still measured: the interval must cover the explicit skip disposition. It cannot disappear because the process was never started.

## Fail-closed enforcement semantics

Only `complete` runtime coverage is accepted. `partial`, `unknown` and `tampered` are not equivalent to zero events.

The following counters must all be zero:
- oracle read attempts or successes;
- unauthorized artifact read attempts or successes;
- disallowed egress attempts or successes;
- cross-trial state observations;
- shared writable namespace observations;
- runtime event sequence gaps and dropped events.

A blocked oracle-read attempt remains a policy violation. It is not made acceptable by the runtime successfully denying it. The attempt and success counters remain separate so a later reporting layer can distinguish prevention from successful compromise.

Allowed egress is counted separately. The runtime adapter must bind every allowed request to the exact frozen allowlist, resolver, destination identity, request/response digest and timestamp. The structural validator can pin the allowlist policy digest and require a nonzero network log commitment, but it cannot inspect the underlying log or prove a destination was allowed. That verification belongs to the authenticated runtime adapter.

## Structural join

Before accepting runtime receipts, the caller supplies the manifest, freeze receipt, the exact ledger output, scenario receipts and runtime attestations. The module rechecks the freeze structure and verifies:
- the ledger identities, policies, denominators and arm summaries agree with the manifest;
- every planned candidate/baseline arm has one and only one ledger result;
- cluster IDs and first-arm execution order agree with the frozen plan;
- every scenario is planned and bound to the exact run/freeze;
- every runtime receipt binds to the same run, scenario, arm, system identity, evaluator, environment, budget and policy digests;
- coverage encloses the complete observed arm interval;
- observer instance, namespace and receipt-sequence identities do not collide;
- the complete receipt population exactly covers the planned arm denominator.

The validator does not infer missing receipts as zero accesses. Missing, duplicated, unplanned, substituted or incomplete evidence is rejected. It reuses the canonical sealed manifest, freeze and scenario validators rather than defining parallel interpretations of those records.

## Validation suite

The tests include:
- clean positive control across candidate and baseline arms;
- exact freeze and manifest commitment binding;
- one receipt per planned arm;
- clean allowlisted egress as a separate non-violation counter;
- oracle access attempt and success;
- prohibited artifact read attempt/success;
- disallowed egress attempt/success;
- partial/unknown/tampered coverage;
- dropped events and sequence gaps;
- observer source/control-domain/adapter substitution;
- duplicate runtime instance, writable namespace or observer sequence;
- interval gaps and excessive clock uncertainty;
- candidate/baseline digest substitution and policy-digest mismatch;
- ledger denominator, arm-order, cluster and summary substitution;
- a 60-vector adversarial mutation matrix.

These deterministic tests validate the evaluator's structural contract. They are not a live sandbox penetration test and do not prove a production kernel/hypervisor enforced any policy.

## Claim ceiling

A green Rust/schema run proves that the supplied records satisfy the structural contract. It does not prove:
- runtime observer signatures or source identities are authentic until the existing adapter verifies them;
- the access/egress logs are complete or faithfully produced;
- the kernel, hypervisor, container runtime, device or hardware enforced isolation;
- the agent could not exploit an unobserved channel;
- a sealed oracle was actually inaccessible to every process and credential;
- the scenario distribution is representative or that any agent is better than SOTA.

Production qualification requires a real NixOS/runtime adapter, independent observer, kernel/hypervisor-specific controls, sandbox escape tests, object-store/IAM denial tests, DNS/egress controls, event-loss fault injection, and adversarial cross-trial state tests. Nixward remains the only host transaction/interlock/journal/recovery authority; this module does not introduce another policy engine or journal.
