# Continuous OpsBench v0.1

Research-only deterministic evaluator for issue #188. This is the continuous-operation companion to episodic OpsBench, not a replacement for it.

## What is evaluated

The evaluator consumes separate planes:
- evaluator-owned fault truth and active/revert intervals;
- agent-produced incident reports and proposed/executed intervention references;
- participant-owned attempt/effect receipts;
- independent service observations bound to a frozen offered-demand workload profile;
- a no-action twin using the same scenario schedule and workload identity.

It derives per-fault correlation, exact service/artifact localization, repair before evaluator cleanup, sustained workload-conditioned recovery, false repair claims, intervention safety, duplicate committed effects, unauthorized committed effects, and demand-weighted degraded request-time.

Every injected fault remains in the denominator, including faults that disappear before an agent invocation. No-action observations are compared only when bound to the same scenario and workload profile. Metrics remain separate; there is no compensatory composite score.

## Run

```sh
cargo test --locked
```

No external crates or live infrastructure are used.

## Claim ceiling

This is a typed offline evaluator over supplied records. It does not generate Kubernetes faults, run an agent, authenticate publishers, verify cryptographic signatures, prove the independence of declared observer sources, or establish production representativeness. A committed participant receipt is trusted as participant-owned input. A production adapter must bind exact episode/run/arm and evidence digests, use a source-authenticated participant ledger and observer, and integrate through Platform Saga and the existing effect broker. Nixward remains the sole host transaction and recovery authority.

The synthetic fixture is not a benchmark result and cannot support SOTA or production-safety claims.


## Request-level impact v0.2

The v0.1 aggregate `max(failed, slow, incorrect)` remains a **lower bound**, not an exact unique-request estimand. The additive v0.2 paired-cell evaluator in `src/impact.rs` uses keyed request pseudonyms, exact set unions, preregistered Candidate/NoAction cell pairs, matched intervals and demand contracts. See `REQUEST_IMPACT_PAIRED_CELLS_V0_2.md`. Do not report v0.1 degradation ratios as exact paired counterfactual effects.


## Sealed certification run freeze v0.1

The additive sealed-run validator in `src/sealed.rs` pins the candidate, baseline, evaluator, freeze-authority and evidence-source identities before sealed scenario generation. It requires explicit accounting for every planned trial and every declared attempt. Open, sealed-certification, and retired-reproducibility lanes have distinct timing rules. See `SEALED_CERTIFICATION_V0_1.md` and the two `sealed-run-*.schema.json` schemas.

This is structural validation only. Commitment hashing, signatures, source authentication, durable replay, network isolation, secret-oracle access control, and trial execution are not implemented here. They must be supplied by the existing source-evidence and execution infrastructure. Skipped, failed, and invalidated trials remain in the denominator.


### Matched candidate/baseline execution

The sealed ledger now requires two separately identified trial streams for every planned scenario: Candidate and Baseline. Both arms consume the same scenario payload commitment but events bind to the corresponding frozen arm identity digest. A missing baseline, a baseline event carrying the candidate digest, or an incomplete arm attempt history invalidates the run. The summary preserves candidate and baseline counts separately, as well as the complete paired denominator.

The matched baseline identity is now fully pinned (provider/model/version, source and image digests, scaffold, prompt, toolchain, runtime config, policy and distinct control domain). The run freeze also pins common execution-environment and resource-budget digests for both arms. A candidate-only ledger is invalid.

See [BENCHMARK_TARGETS_AND_INDUSTRY_COMPARISON_V0_1.md](BENCHMARK_TARGETS_AND_INDUSTRY_COMPARISON_V0_1.md) for current external reference benchmarks, a preregistered target ladder, rare-event confidence interpretation, and the matched confirmatory evaluation protocol. All numeric targets are goals, not measured results.

The run freeze also pins the state-isolation, artifact-access and network-egress policy digests. These describe per-trial writable state/cache boundaries, agent separation from private oracle artifacts, and default-deny or allowlisted outbound retrieval. The structural validator verifies digest presence/binding only; a separate runtime observer must attest enforcement.


### Independent runtime enforcement receipts

The new `src/runtime.rs` validator requires one independent runtime attestation per planned (scenario, Candidate/Baseline) pair. Each receipt binds the exact arm identity, scenario, manifest, environment, resource budget and state/artifact/egress policy digests; it must cover the complete agent interval with complete telemetry. Oracle reads, unauthorized artifact reads, disallowed egress, writable-state sharing, cross-trial state observations, telemetry gaps and missing/duplicate receipts are rejected. See `runtime-trial-attestation.schema.json` and `RUNTIME_ENFORCEMENT_V0_1.md`.

The validator is structural only. The typed source-evidence adapter must authenticate observer receipts and replay sequences; a real runtime observer/sandbox adapter must prove actual enforcement. This does not qualify kernel/hypervisor isolation or production safety.


### Independent runtime enforcement receipts

The runtime validator in `src/runtime.rs` requires complete per-arm process, mount, filesystem, egress and event-stream commitments, and a unique one-use challenge nonce issued after the run freeze. It binds the runtime observer and challenge authority to distinct frozen trust domains. See `RUNTIME_ENFORCEMENT_V0_1.md` and `runtime-trial-attestation.schema.json`.

This is still structural validation. The typed source-evidence adapter must authenticate challenge and receipt publishers, validate the subject and freshness, and consume nonces through durable anti-replay state. The runtime adapter must independently prove telemetry coverage and enforcement; a receipt field cannot prove that a hook remained attached or that a compromised kernel did not suppress events.


### Independent runtime enforcement receipts v0.1

The additive validator in `src/runtime.rs` joins one runtime observer attestation to every planned Candidate/Baseline arm using the raw-event-derived sealed ledger. It checks challenge freshness (24-hour maximum TTL), observer and challenge authority separation, run-scoped contiguous sequences, clock-uncertainty-aware measurement coverage, non-aliasing runtime/namespace identities, complete telemetry, and hard denials for oracle/artifact reads, disallowed egress and cross-trial state. See `RUNTIME_ENFORCEMENT_V0_1.md` and `runtime-trial-attestation.schema.json`.

Only `validate_runtime_attestations_from_events` is public as the certification entry point. The lower-level summary composition is private. Signatures, durable cross-run replay protection, challenge issuance, allowlist event verification and actual kernel/hypervisor enforcement remain external obligations.


### Independent runtime enforcement receipts v0.1

The additive `src/runtime.rs` verifier joins one host-side runtime attestation and one separately authenticated, one-use challenge-authority receipt to every planned `(trial, Candidate/Baseline)` pair. It binds the run/scenario/system identities, execution environment and policy digests, unique runtime and writable namespace IDs, observer adapter/source, challenge nonce/receipt digest and issue/expiry interval. Missing or duplicated challenge/attestation records, sequence gaps, coverage loss, oracle reads, prohibited artifact access, disallowed egress, cross-trial state and shared writable namespaces fail closed.

See `runtime-trial-attestation.schema.json`, `runtime-challenge-receipt.schema.json`, and `RUNTIME_ENFORCEMENT_ADAPTER_V0_1.md`. This is structural validation only; challenge and observer signatures, canonical artifact hashes, durable anti-replay, kernel/hypervisor enforcement, access telemetry completeness and actual runtime integration remain external requirements.


### Paired cluster inference and rare-event bounds

`src/statistics.rs` adds a deterministic paired-cluster bootstrap, conservative lower/upper outcome bounds for indeterminate recoveries, and exact one-sided Clopper-Pearson safety bounds with a zero-event sample-size calculator. Exploratory runs report intervals but cannot declare superiority. Confirmatory mode requires at least 500 independent paired clusters and 10,000 bootstrap replicates before the effect and coverage gates can establish superiority. See `PAIRED_CLUSTER_INFERENCE_V0_1.md` and the two `cluster-inference-*.schema.json` files.
