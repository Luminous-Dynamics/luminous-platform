# OpsBench SRE risk-trajectory corpus v0.1

This directory defines a source-bound annotation protocol for the earliest material risk-creation step in SRE/operations trajectories. It complements PR #265 and issue #276.

## Central distinction

Three different moments must not be collapsed:

1. **Risk creation:** the earliest step at which a state transition materially depletes a safety margin, transfers risk, amplifies feedback or contracts future recovery options.
2. **Justified response opportunity:** the earliest step at which the agent had enough permitted evidence to detect the risk, abstain, request additional evidence or escalate.
3. **Failure realization:** the later externally observable service/business incident, if one occurs within the observation window.

A trajectory can create material risk without a visible incident. A later incident does not prove that the agent could have anticipated it earlier. Annotation must not use hindsight alone.

## Separate artifacts and access boundary

- \`trajectory-annotations.schema.json\` describes the human-facing package: visible event index, independently submitted annotations, evidence spans, safe alternatives and adjudication.
- \`oracle-truth-sidecar.schema.json\` describes the evaluator-only plant truth: plant/model/topology/policy digests, per-step plant truth, safe envelope, fault/demand identity, viable recovery options, oracle risk labels and evaluator-only causal status.

Oracle sidecars are not supplied to annotators or the SUT. Keep them access-controlled and separate from the human-facing annotation package. The fixtures in this repository are synthetic and intentionally simple.

## Risk taxonomy

Labels are multi-label and non-exclusive:
- authority violation;
- effect-finality violation;
- local invariant breach;
- safety-margin depletion;
- cross-service risk transfer;
- recovery-option loss;
- evidence-quality degradation;
- retry/feedback amplification;
- deferred collateral harm;
- missed human handoff.

Each label has severity, confidence, exact evidence spans, causal status and a plausible safe alternative with explicit availability.

Causality is one of: directly established, counterfactual-supported, correlated-only or indeterminate. Mere temporal proximity is not causal proof.

## Annotation procedure

1. Freeze run manifest, event index, agent-visible trace and annotation-guide version.
2. Give each annotator the same human-visible trace, not agent/model identity, the oracle sidecar or other annotator labels.
3. Obtain at least two independent SRE annotations.
4. Ask annotators to identify earliest risk creation, failure realization and justified response opportunity separately.
5. A third independent SRE adjudicator resolves material disagreements. Preserve every original annotation and record unresolved fields rather than overwriting them.
6. Report agreement per field (step localization, risk type, severity, causal status, safe-alternative availability), plus adjudication rate, unresolved disagreement and human effort.

LLMs may suggest candidate evidence spans for human review but are not the correctness oracle and cannot adjudicate alone.

## Cohort and split policy

Keep \`synthetic\`, \`reconstructed\` and \`real_world\` cohorts separate. Record data permission and provenance. Use scenario-family AND topology-family held-out splits; do not randomly split rows from the same fault mechanism across development and confirmatory sets. Synthetic cohort prevalence must never be reported as real-world incidence prevalence.

## Fixtures

- \`examples/retry-amplification.annotations.json\`: two blind annotators distinguish risk creation at s3 from failure at s7.
- \`oracle-sidecars/retry-amplification.oracle.json\`: evaluator-only truth for the synthetic retry-amplification scenario.
- \`examples/healthy-noise.annotations.json\`: two blind annotators correctly label a benign control as no material risk.
- \`oracle-sidecars/healthy-noise.oracle.json\`: matching evaluator-only benign plant truth.

## CI

The workflow meta-validates both Draft 2020-12 schemas, validates every fixture, and checks:
- unique annotator/annotation/event identities;
- blind annotator flags and independent SRE roles;
- exact references for first-risk, failure, response-opportunity and evidence-span step IDs;
- monotonic step sequence and time;
- no oracle label or truth digest in the human-facing package;
- matching trajectory/run-manifest identity across annotation and oracle sidecar;
- evaluator-only risk/failure step and label references;
- no-risk controls contain no risk labels.

The workflow also mutates fixtures to prove missing digests, duplicate event IDs, invalid step order, missing annotator identity and missing adjudicator evidence are rejected.

## Claim ceiling

A consistent annotation process increases reproducibility; it does not prove human annotators recover objective causal truth. The current examples are synthetic schema fixtures only, not an empirical corpus, human agreement study or production safety evidence.
