# OpsBench Risk-Aware Value of Information for Active Diagnosis v0.1

**Status:** offline synthetic decision-theory benchmark and evaluator. No live probes, service mutations, production recommendations or policy qualification are performed.

## 1. Why this benchmark exists

Operational agents need to decide not only what an incident might be, but whether another observation is worth its latency, cost, privacy exposure and delay. Counting tool calls or scoring whether the selected tool name matches a reference misses the decision: will this observation change what the system is safely allowed to recommend?

This benchmark separates uncertainty reduction (for example, entropy), decision value (expected reduction in Bayes decision risk), and execution authority (not present in this benchmark).

SearchHorizonBench reports 45,632 executions over 3,168 questions and emphasizes the difficulty of estimating the value of another search separately from final answer correctness. Canary-tool evaluations distinguish several tool-selection failure modes rather than treating tool selection as a single accuracy figure. This v0.1 translates those measurement ideas into a synthetic operational diagnosis setting. References:
- https://arxiv.org/abs/2610.34714
- https://arxiv.org/abs/2608.04719
- https://sre.google/resources/practices-and-processes/ai-engineering-reliable-operations/

## 2. Decision-theoretic objective

Let H be the incident-hypothesis set, p(h) the current posterior, A the allowed response set, and L(a,h) the evaluator loss of choosing response a when hypothesis h is true.

The policy uses a non-compensatory safety threshold for mutating responses. A mutating action is eligible only if its posterior expected loss is at or below max_action_risk; otherwise the eligible response set contains only non-mutating escalation/abstention actions.

The current constrained Bayes risk is:

    R(p) = min over a in A_permitted(p) of sum_h p(h) L(a,h)

For read-only probe q with possible outcomes o, calculate:

    P(o | q,p) = sum_h p(h) P(o | h,q)
    p(h | o,q) = p(h) P(o | h,q) / P(o | q,p)
    EVSI(q) = R(p) - sum_o P(o | q,p) R(p(. | o,q))

The risk-adjusted probe score is:

    NetVOI(q) = EVSI(q) - cost(q) - privacyPenalty(q)

Latency and human time are hard budget constraints in v0.1, rather than being hidden inside an arbitrary scalar weight. A future registered study may predeclare a validated conversion model, but must not tune it after seeing outcomes.

Entropy reduction is a separate baseline. A probe can reduce entropy without changing the best safe response; in that case its decision EVSI is zero, even though entropy reduction is positive.

## 3. Safety and evidence constraints

- All probes are read-only. The evaluator rejects a probe whose read_only field is not exactly true.
- Every probe must be authorized by an exact source_id, trust_domain_id and scope_id entry.
- A probe's correlation group may be queried at most once within a strategy episode. The evaluator does not count correlated evidence twice.
- An evidence_id may be credited only once. A duplicate ID is recorded and charged but does not update the posterior.
- An observation older than its probe TTL is recorded as stale and does not update the posterior.
- If an outcome has zero probability under the current likelihood model, the evaluator records an impossible/model-contradictory observation and preserves the prior. It does not force a zero-mass posterior.
- Mutating recommendations are permitted only when the best targeted action's posterior expected loss satisfies the frozen risk threshold. Otherwise the policy escalates or abstains.
- Missing/contradictory evidence is not a reason to relax the risk threshold.
- Source mappings in this offline fixture are policy declarations, not cryptographic identities. Real producer authenticity and evidence semantics must be verified by #178 and #174 respectively.

## 4. Strategies

All strategies receive the same prior, hypothesis set, action-loss matrix, read-only probe catalog, likelihood model, scope and resource budgets:

1. random: seeded random feasible probe.
2. cheapest_first: smallest cost plus privacy penalty, then lower latency.
3. entropy_per_cost: expected entropy reduction per cost/privacy unit.
4. fixed_runbook: frozen authored probe order.
5. risk_adjusted_voi: highest positive net EVSI; stops when no feasible positive-value probe remains.
6. oracle_perfect_information: evaluator-only upper bound that reads hidden truth. It is not a system under test and cannot be included in claims of model capability.

The strategy receives an outcome only after selecting a probe. Hidden truth and the scenario outcome deck are used by the evaluator for scoring and simulation, never for probe selection.

## 5. Metrics

Report per-strategy and per-scenario:
- direct recommendation, escalation, abstention and unsafe recommendation counts;
- posterior expected risk versus realized decision loss under the hidden truth;
- multiclass Brier score and top-hypothesis expected calibration error;
- probes, cost, latency, privacy penalty and human seconds;
- predicted EVSI versus realized constrained-risk reduction after each observed probe;
- entropy reduction as a distinct diagnostic measure;
- stale observations, duplicate evidence IDs and impossible/model-contradictory observations;
- final posterior, stop reason and full probe sequence.

Negative realized risk reduction is preserved. A probe that produces a misleading observation and worsens the chosen decision must not be clipped to zero or hidden by an average information-gain metric.

## 6. Synthetic example

The example has four hypotheses and eight read-only probes: a cheaper 80%-accurate observation and a more expensive 98%-accurate observation for each hypothesis family. The evidence catalog records distinct source/trust identities, correlation groups, TTLs, cost, latency, privacy penalty and human time. One scenario makes a network-path observation stale.

The example illustrates why cheapest-first and entropy-per-cost are not equivalent to decision-aware VOI; why a high-accuracy probe can be worth more than a cheap probe when it can move the posterior across a predeclared safe-action threshold; why stale and duplicate evidence must not increase confidence; why escalation is the correct output when no evidence path makes a mutating response sufficiently safe; and why an evaluator-only perfect-information oracle is an upper bound, not a competitor.

The example is labelled example_only, and its priors, likelihoods, action losses and outcomes are authored synthetic data. No result is a Luminous performance measurement.

## 7. Run

From the repository root:

    python3 tools/opsbench-active-evidence-voi-v0.1/evaluate_voi.py \
      --study benchmarks/opsbench-active-evidence-voi-v0.1/study.example.json \
      --output /tmp/opsbench-active-evidence-report.json

Run mutation tests:

    python3 -m unittest discover \
      -s tools/opsbench-active-evidence-voi-v0.1 \
      -p 'test_evaluate_voi.py' -v

The evaluator uses the Python standard library only. It does not call a probe, read a live incident, access a service, authorize a mutation or execute a recommended response.

## 8. What a serious confirmatory study requires

Before using this benchmark for comparative claims:
1. Seal evaluator-owned scenarios and outcomes; publish commitments, not hidden truth or outcome decks.
2. Preregister hypothesis priors, action-loss matrices, source roles, probe likelihoods, correlation groups, freshness policy and resource budgets.
3. Calibrate probe likelihoods on independent historical/synthetic data; report calibration by failure family and prior shift.
4. Compare the five policies on the exact same sealed scenarios and resource budgets.
5. Include a domain-expert human evidence-gathering baseline and a fixed deterministic runbook baseline.
6. Report scenario-clustered uncertainty, unmatched runs, calibration, unsafe recommendations, probe cost, latency, privacy, abstention and escalation.
7. Validate on out-of-distribution telemetry, stale sources, correlated failures, conflicting observations and deceptive/canary evidence.
8. Keep the response action simulated and evaluator-scored. A real mutating policy requires separate authority, Nixward journal/interlock controls, and operational qualification.

Targets in issue #185—lower probe cost at non-inferior decision quality and lower decision regret than fixed/random baselines—are hypotheses, not achieved results. No SOTA or production claim is supported by this synthetic corpus.
