# OpsBench Challenge Evolution v0.1

This is a supplemental Luminous challenge-evolution track. It does not alter native SREGym, EnterpriseOps-Gym, SRE-Marathon, ITBench or PagerDuty benchmark tasks, prompts, tool permissions, environment or scoring. Native results remain tied to their native source revisions and scorers.

## Why evolve the challenge set?

SREGym's 2026 benchmark retrospective describes retiring saturated tasks and experimenting with automated difficulty increases such as more telemetry noise and larger systems. A static corpus can cease to differentiate capable systems. However, mutating tasks in place destroys reproducibility and can change what is being measured.

The core rule is therefore:

**A harder challenge is a new immutable scenario with a parent link and an explicit, auditable parameter delta. It is never an in-place edit to an old scenario.**

## Successor manifest v0.1

The challenge-successor schema pins:

- Parent scenario ID and parent manifest digest.
- Generator source revision, configuration digest, algorithm and seed commitment.
- One declared primary difficulty axis and exact changed parameter paths.
- Intended fault mechanism, ground truth, verifier, task prompt, tool manifest and outcome-semantics digests.
- Runtime image, topology, workload, runtime and observer-policy digests.
- Immutable anchor-set digest.
- Contamination-scan state and sealed case/oracle commitments.
- Independent semantic-review status and reviewer identities.
- Explicit lane and release state.

The manifest's semantics_preserved and single_axis_change values are declarations. A schema validator cannot prove them. A later semantic auditor must recompute the parent/child parameter diff, verify the scenario and oracle artifacts, and independently assess whether the construct remained unchanged.

## Difficulty axes

Evolve one primary axis at a time initially:

1. Fault multiplicity and overlap.
2. Causal interaction depth and decoy faults.
3. Observability loss, delay, reordering and clock uncertainty.
4. Topology churn and partial rollout.
5. Demand pressure and retry amplification.
6. Recovery horizon and metastability.
7. Harness evolution, tool deprecation and permission narrowing.

Factorial combinations belong in separate confirmatory scenarios, not hidden inside a successor that claims to change one axis.

## Lanes

- development: open experiments and generator debugging.
- screening: open successor candidates, not confirmatory results.
- sealed: requires contamination scan, scanner/config/report identity, examined-source families and salted scenario/oracle commitments.
- retired: a formerly sealed scenario can be revealed for reproducibility after the confirmatory epoch closes.

A successor marked confirmatory must be in the sealed lane, pass semantic review with at least two reviewers, and carry a passed contamination scan. CI includes negative controls for attempted premature promotion.

## Anchor and contamination policy

A fixed anchor set remains byte-identical across epochs. It detects whether challenge evolution itself causes regression or shifts the meaning of the score. Evolved-epoch scores and anchor scores are reported separately.

A contamination scan should examine the candidate and baseline code, prompts, tools, runbooks, issue/PR text, logs, memory stores, retrieval corpora, caches, scenario material, expected answers, and prior run artifacts. A source family that was inaccessible is recorded as unexamined, not silently counted as scanned.

Do not train on a sealed challenge or reveal its oracle until the epoch is retired. Use domain-separated salted commitments for case payloads and private oracles; the commitment implementation and signature/identity verification remain external to this schema slice.

## CI and claim boundary

The workflow meta-validates the Draft 2020-12 schema, validates the screening fixture, and rejects schema-level negative controls. It also checks simple parent/child identity and one-axis declarations.

This is a manifest/schema design slice. It does not yet generate successors, validate semantic preservation, prove complete contamination detection, calibrate item-response difficulty, or run an agent benchmark. Those require the generator, independent scenario/oracle auditor, human semantic adjudication and sealed execution boundary.

The supplemental track must never be merged into native benchmark score calculations.
