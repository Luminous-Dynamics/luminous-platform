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
