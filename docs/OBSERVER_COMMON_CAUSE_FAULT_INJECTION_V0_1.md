# Observer Common-Cause Fault-Injection Benchmark v0.1

**Status:** synthetic fault-plan and post-fault evaluator contract only. No live dependency was disabled, no observer was compromised, and no production effect or infrastructure was exercised.

## Purpose

OIC v0.1 establishes claim-specific witness independence from a declared set of seven failure-domain identities. That is necessary but insufficient: distinct synthetic identifiers do not prove that two deployed observers survive the same real infrastructure failure independently.

CFI v0.1 adds a **fault-plan to post-fault observation contract** to the existing OIC evaluator. It does not implement a fault injector or a second failure-domain graph.

## Owner boundaries

- Platform OIC #514 owns the observer-independence profile and reuses its existing checker.
- Platform CFI #515 owns the synthetic campaign manifest, post-fault witness availability and recovery/requalification contract.
- Mycelix #5762 owns canonical multi-valued and transitive operational dependency closure. CFI consumes its future owner-verified identities; it does not define a competing graph.
- Symthaea ASSURE/WALP owns claim and evidence lifecycle currentness.
- Platform oracle fidelity and ordered trace oracle own observer channels and normalized operation-trace semantics.
- Participant/resource owners remain authoritative for actual effect truth and final admission.

## Core invariant: fault plan is not effect evidence

Each episode separates:

1. **Evaluation input:** the OIC scenario identity, witness evidence available to the evaluator, and any fresh post-recovery witness evidence.
2. **Post-hoc ground truth:** the preregistered fault target/plan and independent ground-truth labels for injection and resource effect.
3. **Expected contract result:** the result obtained by applying the existing OIC evaluator to the evaluation input only.

The OIC evaluator must not read the post-hoc fault label or resource ground-truth field. The evaluator's disposition must be invariant under mutations to those fields. Post-hoc truth is used only to score whether a qualified effect/no-effect statement was correct.

A generator-supplied label that a fault was injected is not proof the dependency was actually affected. A future runtime campaign needs a separate injection receipt and external verification of the actual failure.

## v0.1 semantics

- Use the exact OIC claim profile `critical_effect_truth_v0` and its seven required domains.
- A planned dependency target is identified by the exact domain and owner-supplied dependency reference. No matching by friendly name or composite dependency-set digest.
- The post-hoc expected-impact set is the union of witnesses in the referenced OIC scenario whose exact domain identity equals a planned fault target.
- The evaluation input lists the witness evidence actually available to the evaluator after the episode. It is not allowed to include a witness removed by the episode unless a recovery record supplies fresh replacement evidence.
- The evaluator reuses OIC's maximum-independent-witness-set calculation and dispositions. It does not implement new quorum semantics.
- A fault that removes one observer does not automatically invalidate unrelated observers or the entire claim. If the surviving independent quorum still qualifies, the result remains qualified.
- A shared required dependency can remove every observer that depends on it; process separation does not preserve quorum where event stream, signing root, credential root, clock, storage or control plane is shared.
- Unknown domain identities remain unknown and never count as unique.
- A recovered witness may rejoin only with a strictly newer evidence epoch, a new evidence reference and a new attestation reference. The prior pre-fault attestation is not carried forward.

## Frozen 16-scenario panel

The panel includes no-fault and single-process-fault positive controls; common event-stream, signing-root, credential-root, clock-authority, storage-backend and control-plane failures; observer disagreement; independent no-effect quorums; a sufficient quorum coexisting with an unknown extra witness; insufficient/unknown dependency evidence; subject-correlated observers; compound failures; and recovery with fresh evidence.

Distribution: five positive, eight negative and three indeterminate controls. These are deterministic synthetic controls, not a statistical failure-rate sample.

## Required measurements

- planned dependency targets and exact owner-domain references;
- post-hoc impacted witness identities;
- evaluation-input surviving witness identities;
- maximum independent witness count and OIC disposition;
- false-qualified effect/no-effect outcomes against independently held ground truth;
- false blanket invalidation of unaffected witnesses;
- unknown/incomplete dependency count;
- recovered witnesses and evidence-epoch transitions;
- prior evidence reuse/replay attempts;
- evaluator-output invariance when post-hoc truth labels change.

No aggregate score is defined.

## Next physical campaign

A runtime campaign should use a disposable, harmless resource and a fault injector isolated from both the agent and the effect evaluator. It must verify actual dependency identity and actual injection outcome through evidence separate from the generator. Test one domain at a time, then pairwise/common-cause combinations, unknown dependencies, surviving quorums and recovery. Ground truth is revealed only after the evaluator disposition is committed.

Initial targets:
- zero false-qualified outcomes in every injected common-cause stratum;
- zero use of a witness whose required dependency was verified compromised/unavailable;
- zero loss of an unaffected independent quorum;
- zero stale pre-fault evidence carried forward after recovery;
- 100% deterministic outcomes on the frozen controls.

These are research gates, not achieved results. A synthetic checker pass qualifies only the fixture and evaluator contract. An injected-fault campaign qualifies only the tested deployment topology, fault set and exact evidence observers.
