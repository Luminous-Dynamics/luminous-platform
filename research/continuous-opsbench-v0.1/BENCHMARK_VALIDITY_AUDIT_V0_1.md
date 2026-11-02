# Benchmark validity and scorer audit v0.1

## Why this exists

A high agent score is meaningful only when the benchmark contract, environment, ground truth and evaluator can support the claim. A task can be ambiguous, a fault may fail to activate, a reset may leave drift, an oracle can omit a condition, or a scorer can reward the wrong behavior. These defects are not agent capability.

This module turns benchmark validity into an explicit pre-certification gate. It produces a component-wise assurance vector and a claim-admissibility bit. It does not generate a scalar benchmark quality score.

## Four components

### User/task contract
Every task has a stable ID, exact instruction digest, capability ID, ambiguity class, precondition and postcondition commitments, and required/prohibited predicate IDs. Every predicate must be declared in the evaluator-owned oracle. A predicate cannot be both required and prohibited. Duplicates are rejected. The complete task-set digest is pinned to the UserTask component artifact digest.

### Environment and tools
The environment contract pins its image, tool schema, dependency lock, fault injector, reset procedure and expected initial-state digest. At least three independent resets are required. Each reset has a unique ID, sequential repetition index, completion time, exact observed initial-state digest and a positive fault-activation control. All reset evidence must predate audit issuance. Reset repetitions must be contiguous and strictly time ordered. An unactivated fault injector or any reset drift makes the contract invalid.

The schema and Rust model deliberately keep environment identity separate from reset receipts. A declared reset procedure is not proof that the reset actually returned the environment to the expected initial state.

### Ground truth
The oracle contract pins its version, program digest, coverage digest, ambiguity policy, control domain, predicate set and private commitment. Oracle predicate IDs must be nonempty and unique. The oracle control domain must equal the GroundTruth component domain and be separate from the evaluator component domain. Oracle program and evaluator binary digests must not alias.

### Evaluation and scorer
The evaluation contract pins the exact evaluator binary, metric semantics, statistical plan and each metric's estimand semantics, numerator predicates, denominator ID, unit, direction and exclusion policy. Metric IDs and numerator predicate IDs must be unique. Every numerator predicate must exist in the oracle predicate set.

No metric can silently omit its denominator, units, exclusions or estimand. The audit does not translate different industry benchmark scores into one value.

## Component provenance and trust domains

Every component carries a version, artifact digest, schema digest, evidence digest, source ID and control domain. The four component sources and domains must be unique. Evaluation uses the evaluator control domain already pinned in the frozen run manifest; the other component domains must be separate from it and from candidate, baseline, freeze authority, evidence source, runtime observer, challenge authority and auditor domains.

This is structural separation. The typed source-evidence adapter must authenticate each statement and its publisher, subject, digest, trust domain, freshness and replay sequence. The validator does not verify signatures itself.

## Known-answer and mutation controls

Minimum control corpus:
- at least 40 known-answer controls total;
- at least 10 positive and 10 negative controls;
- at least 5 known-answer controls for each of four components;
- at least 60 mutation controls total;
- at least 10 mutation controls per component;
- at least one interaction control for each of the six unordered component pairs.

A positive control must be accepted; a negative control must be rejected. False positives and false negatives are separately counted. Any known-answer mismatch marks its component Fail and makes the cohort inadmissible.

Each mutation declares whether the mutated input should be rejected and records the observed outcome. A missed rejection, or an unexpected rejection of a valid control, marks that component Fail. Insufficient coverage is Indeterminate, not Pass.

The included deterministic suite contains 90 structural mutations covering manifest binding, component source/domain/digest substitution, task predicate ambiguity, reset drift, fault activation, oracle and scorer defects, control corpus tampering, denominator omissions and interaction-control failures. These are synthetic tests, not proof that the audit detects every possible benchmark defect.

## Cross-component interactions

All six unordered pairs are mandatory:
1. UserTask × EnvironmentTools
2. UserTask × GroundTruth
3. UserTask × Evaluation
4. EnvironmentTools × GroundTruth
5. EnvironmentTools × Evaluation
6. GroundTruth × Evaluation

Interaction controls should cover situations where individually valid components disagree when combined—for example, a task postcondition that the environment cannot expose, an oracle predicate that cannot be derived from the environment, or a scorer numerator that does not follow from oracle truth.

If a component-pair control fails, both participating components are marked Fail. No component can compensate for another.

## Admissibility semantics

- **Pass:** required evidence and controls are present, and no tested defect was observed.
- **Fail:** a known-answer, mutation or interaction defect was detected.
- **Indeterminate:** the minimum evidence/control coverage is not present.

The benchmark claim is admissible only when all four components Pass and all six interactions have passed. A malformed or unauthenticated source record is rejected by the structural/source-evidence path; an authenticated observation of an incorrect grader outcome produces a non-admissible audit result rather than being hidden as a malformed record.

This is a gate, not a scalar score. Do not average component dispositions, hide uncertainty, or convert Indeterminate to Pass.

## How this improves industry comparisons

A meaningful comparison must freeze the benchmark as well as the two agents. For every SREGym-, ITBench-, SRE-Marathon-, SentinelBench- or privacy-compatible comparison, publish:
- exact task/cohort version and component digests;
- exact environment/reset/fault-injector identity;
- exact oracle coverage and ambiguity policy;
- exact evaluator/metric/statistical-plan digests;
- known-answer control coverage and FP/FN results;
- mutation and interaction control coverage;
- source-authentication and sealed-lane evidence;
- paired candidate/baseline run receipts and cluster-aware uncertainty.

Reproduce an external benchmark's own metric as a compatibility result. Do not compare its score directly with OpsBench unique-request impact, sustained recovery, privacy or resource-efficiency metrics. The primary Luminous claim remains the preregistered, matched, sealed continuous-operation estimand.

## Claim ceiling

Passing these checks establishes consistency of the supplied component records and successful execution of the included controls. It does not prove that the component publishers are authentic, the environment models production, the oracle is semantically complete, a grader has no unknown false negatives, or the task distribution is representative. Those require independent source authentication, adversarial cohort review, blinded semantic adjudication and external replications.

The audit is research-only and synthetic. It has no production side effects and does not replace the sealed run, runtime enforcement, privacy, sandbox or service-outcome evaluators.
