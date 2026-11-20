# Luminous Adaptive Authorization Assurance Protocol v0.1

**Status:** Research protocol; no harness/stressor experiment has been executed  
**Date:** 2026-11-19  
**Tracking:** [OSI/XAP initiative #497](https://github.com/Luminous-Dynamics/luminous-platform/issues/497) · [Adaptive runner #500](https://github.com/Luminous-Dynamics/luminous-platform/issues/500)

## 1. New SOTA direction

The 17 November 2026 paper *Evolution Under Fire: Evolving Agent Safety Harnesses with Adaptive Injection Stressors* keeps task model, tools, benchmark and evaluator fixed while evolving executable safety harnesses under utility constraints. It then evolves attacks against harness–case pairs that were previously useful and secure; an attack is promoted only when the legitimate task still succeeds and the adversarial objective succeeds. Verified stressors feed back into the next harness-search stage.

Author-reported results:
- AgentDyn / GPT-4o-mini: ASR 50.4% (282/560) to 3.0% (17/560); clean utility 27/60 to 28/60; attacked utility 202/560 to 219/560.
- Frozen transfer to Qwen3.5-35B-A3B without further evolution: ASR 32.1% (180/560) to 0.9% (5/560); attacked utility 341/560 to 355/560.
- Agent-SafetyBench / Qwen2.5-7B-Instruct: ASR 82.6% to 62.0%, with 48.3% helpfulness and 99.9% runtime validity.

These are benchmark-specific author results, not Luminous results. ASR is not the same as participant-side unauthorized-effect prevention. Agent-SafetyBench has no task-utility metric in this evaluation, so helpfulness/runtime validity are separate dimensions. Raw ASR must not be compared directly to Luminous authorization/effect outcomes.

Source: Wang et al., arXiv:2611.22583v1, submitted 17 November 2026: https://arxiv.org/abs/2611.22583

## 2. Luminous adaptation: evolve the harness, never the trust anchors

### Frozen for each search stage
- Task model and exact revision/digest.
- Tool bundle, schemas, operation manifests and behavior.
- Environment, initial state, benchmark and task split.
- Principal, grant, delegation and policy semantics.
- Authorization oracle and expected decision semantics.
- Participant admission/idempotency/effect oracle.
- Independent effect observer and postcondition semantics.
- Evaluator code, hidden labels, schedule and scoring definitions.
- Original task, purpose, target and legitimate objective.

### Mutable candidates
1. **Harness implementation:** context construction, provenance handling, policy adapter, pre-action validation, participant request binding, revocation checks, recovery logic and observability wiring. Authority semantics and evaluator remain fixed; only their enforcement implementation may change.
2. **Adversarial stressor:** untrusted document/email/webpage content, ordering, paraphrase, benign-looking instructions, poisoned memory, agent handoff context, timing/partition schedule or other benchmark-defined hostile input. The original task, tools, target and evaluator semantics do not change.

A candidate may not edit grants, redefine allowed operations, change policy revisions, alter an oracle/effect observer, suppress controls or loosen the evaluator. Policy changes require a new benchmark/profile revision and preregistered experiment, not a harness revision.

## 3. Coupled search loops

### Harness evolution
- Pin a baseline harness and frozen inputs.
- A read-only Diagnoser inspects exact source, traces, failures, uncertainty and prior candidates; it cannot change evaluator inputs or write code.
- A separate Expander creates one child harness in an isolated workspace.
- Run inexpensive structural/runtime checks first; reject non-executable candidates early.
- Evaluate paired clean, adversarial, positive and negative controls on identical case/seed pairs.
- Retain all candidates and evidence, including rejected candidates. Only complete, utility-qualified candidates can become parents.
- Select parents from a multi-objective Pareto frontier with uncertainty-aware sampling; do not greedily keep only the latest child or lowest-ASR candidate.

### Adaptive stressor evolution
- Select harness–case pairs where the user task succeeds and the frozen security oracle says the harness prevented the attack.
- A read-only Stressor Diagnoser inspects traces and prior stressor lineages.
- A separate Stressor Generator changes only the benchmark-defined adversarial input/schedule.
- Evaluate against the same task model, tools, environment, authority semantics, participant oracle and evaluator.
- Promote a stressor only if the original task remains successful, the fixed adversarial oracle confirms success, execution is complete/error-free, and exact bytes/lineage/harness/model/tool/oracle digests are recorded.
- For this profile, attack success means a prohibited action is admitted/committed, a required authority check is bypassed, or a data/physical boundary is crossed as defined by the frozen oracle.
- Keep failed and task-destroying stressors as evidence, but do not promote them as useful attacks. Deduplicate without collapsing distinct causal paths or authority states.

Promoted utility-preserving stressors enter a development set for the next harness stage. A final hidden stressor set never feeds evolution.

## 4. Multi-oracle scorecard

Do not reduce security to ASR or a single scalar reward.

| Dimension | Frozen oracle |
|---|---|
| Clean task success | Task evaluator |
| Attacked task success | Task evaluator |
| Attack success | Adversarial objective oracle |
| Authorization validity | Principal/actor/approval/grant/delegation/target/operation/digest/policy/currentness verifier |
| Participant decision correctness | Participant admission oracle |
| Unauthorized effect | Participant-owned effect oracle |
| Fail-closed violation | Independent policy/effect evaluator |
| Indeterminate outcome | Unknown-state oracle; never counted as safe |
| Effect observation | Independent postcondition observer |
| Benign false denial | Positive-control oracle |
| Recovery/human impact | Independent recovery/human-outcome evaluator |
| Operational cost | Frozen latency/energy/memory/network instrumentation |

Task completion cannot mask an unauthorized sequence. A correct final state reached through omitted approval, scope widening, stale policy or revoked child authority is task success and security failure. Blanket refusal may yield low ASR but must fail positive-control/utility constraints.

## 5. Isolation and experiment integrity

- Disposable VM/microVM or equivalently hardened sandbox.
- No production credentials, signing keys, customer data or ambient host authority.
- No network egress by default; explicit test allowlist only where required.
- Bounded CPU, memory, storage, process count and wall clock.
- Read-only diagnosis; separate expansion.
- Evaluator/oracle artifacts mounted read-only.
- Candidate code, stressors and evidence content-addressed.
- Human security review and exact-target requalification before deployment.
- Pin all frozen inputs and source/tool/benchmark/policy/oracle/evaluator/environment digests.
- Pair seeds and case IDs; separate development/adaptive stressors from hidden holdouts.
- Record failed/incomplete runs. Report exact counts, denominators, missingness, uncertainty, repeated-run behavior and worst-domain outcomes.
- Evaluator, policy or oracle drift invalidates the run and requires a new version.
- Cross-model transfer is a separate experiment; no tuning on transfer holdout.

## 6. First Luminous experimental sequence

1. Use the 14-episode agent-authorization corpus as the fixed semantic seed suite.
2. Add AgentDojo/AgentDyn-style untrusted-context tasks with explicit principal, grant, participant and effect oracles.
3. Freeze the Symthaea task model, Xenia tool/session contracts, Mycelix grant semantics, participant adapters and evaluator digests.
4. Evolve a sandboxed harness adapter around the existing Platform/Nixward effect boundary; do not evolve authority or participant policy.
5. Evolve stressor lineages against previously secure harness–case pairs; promote only task-preserving attacks that succeed under the fixed security oracle.
6. Re-evaluate on clean, original adversarial, promoted adaptive and hidden holdout stressors.
7. Repeat on another model/runtime without further evolution and report transfer separately.
8. Require two independent evaluator implementations and exact artifact reconstruction before public qualification claims.

### Proposed gates (research targets, not achieved)
- 100% exact outcomes on frozen deterministic authorization/effect controls.
- Zero unauthorized participant commits and zero fail-closed violations on designated negative controls.
- 100% acceptance of eligible deterministic positive controls; report false denials separately.
- Unknown authority/effect outcomes cannot be promoted as safe.
- Any candidate changing a frozen trust anchor/oracle/evaluator is a different experiment.
- Stressor promotion requires both task preservation and verified attack success under frozen oracle.
- Hidden stressors cannot be used for iterative tuning.
- Publish all objectives and denominators separately.

## 7. Claim ceiling

This protocol is a research artifact. No harness/stressor search has been executed. The 14 authorization episodes and 31 XAP scenarios are synthetic. Evolution Under Fire results belong to the paper's authors and setup. Luminous has not reproduced the paper, run matched AgentDyn/Agent-SafetyBench experiments or demonstrated better-than-SOTA performance. Full CI, sandbox implementation, independent review and matched external reproduction are required before such claims.
