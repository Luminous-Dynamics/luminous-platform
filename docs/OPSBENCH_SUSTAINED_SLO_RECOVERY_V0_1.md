# OpsBench Sustained SLO Recovery v0.1

**Status:** evaluator prototype with synthetic example windows. This is not a production monitoring integration or qualified recovery benchmark.

## 1. The question changes

A unit can be active while the user-facing service is still broken. An HTTP endpoint can return 200 while the business response is wrong. A single probe can pass while p95 latency, error ratio, throughput or saturation remain outside the service objective. A low-traffic period can make an error ratio look perfect while the service has no demonstrated capacity.

The recovery claim therefore changes from:

> The requested operation completed and one postcondition probe passed.

to:

> An independent observer measured every preregistered, customer-visible SLI within its SLO for the full stability window, with adequate traffic, synthetic probes and business correctness evidence, and a separate final business-state oracle passed.

This design follows the established SRE approach of deriving SLIs from user-visible behavior and explicitly defining measurement periods and error-budget rules. It uses OpenTelemetry's stable HTTP metrics where appropriate, without assuming telemetry alone proves the customer outcome.

References:
- Google SRE, [Service Level Objectives](https://sre.google/sre-book/service-level-objectives/) — objectives, explicit measurement windows and error budgets.
- Google SRE, [Production Services Best Practices](https://sre.google/sre-book/service-best-practices/) — user-visible availability/performance and supervised rollout/recovery.
- [Principles of Chaos Engineering](https://principlesofchaos.org/) — hypothesize measurable steady state in throughput, error rate and latency, then test disruptions against it.
- OpenTelemetry, [HTTP Metrics semantic conventions](https://opentelemetry.io/docs/specs/semconv/http/http-metrics/) — stable http.server.request.duration, response status and error attributes.
- OpenTelemetry, [HTTP span conventions](https://opentelemetry.io/docs/specs/semconv/http/http-spans/) — stable request/response attributes for correlated observations.

## 2. Frozen service objective contract

The contract pins:
- study, run, episode and service identity;
- executor source/trust domain;
- independent telemetry observer source/trust domain;
- separate business oracle source/trust domain;
- exact service digest and frozen pre-incident baseline measurement/workload digests;
- incident and observation timeline;
- fixed measurement window duration and required consecutive hold-window count;
- minimum request, synthetic probe and business-check sample counts;
- required SLI clauses and thresholds.

v0.1 supports:
- request-success ratio;
- synthetic-probe-success ratio;
- business-correctness ratio;
- histogram-based latency quantile (conservatively uses the upper bound of the bucket containing the requested quantile);
- minimum goodput/throughput;
- maximum saturation ratio.

Thresholds are scenario-defined, not universal constants. A payments API, bulk pipeline and interactive service require different workload and performance objectives.

## 3. Observation-window semantics

Each window is bound to the contract's exact observer and business-oracle identities. The observation series is fixed on a time grid. A missing window is inserted into the analysis as indeterminate; no interpolation is allowed.

Each window is classified as:
- **pass** — every required SLI is within threshold and sample sufficiency is met;
- **fail** — at least one measured SLI breaches its target;
- **indeterminate** — a required measurement is missing or sample/traffic coverage is too low.

A passing window must meet the minimum request, synthetic probe and business-check counts. This prevents zero traffic or sparse sampling from producing a false healthy result. Latency histogram bucket totals must match the request count; absent or incomplete histogram evidence is not treated as a passing quantile.

## 4. Sustained recovery algorithm

Let the pre-registered hold length be H consecutive windows. The evaluator scans the entire expected observation interval:

1. A pass window extends the current consecutive-pass run.
2. A fail window resets the run and increments a regression counter after any pass has previously been observed.
3. An indeterminate or missing window resets the run and increments the indeterminate-window count.
4. Recovery is verified only when the final sequence in the observation horizon contains at least H consecutive passing windows.

Requiring the final sequence prevents a service that briefly recovered and then regressed from being reported as recovered. The report separates:
- time to recovery candidate (start of final passing run);
- time to sustained recovery verification (end of the full hold window);
- regression count;
- indeterminate window count;
- per-window and per-SLI decisions.

For a five-window hold with 60-second windows, the final five passing windows provide five minutes of continuous SLO evidence. The example contains an initial failure, two passing windows, a p95 latency regression that resets the run, then five consecutive passing windows. Its candidate begins at 14:04 UTC; sustained verification ends at 14:09 UTC. This is a synthetic demonstration only.

## 5. Run

The evaluator uses the Python standard library and does not run an agent, issue a mutation or contact a service:

    python3 tools/opsbench-sustained-slo-recovery-v0.1/evaluate_slo_recovery.py \
      --contract benchmarks/opsbench-sustained-slo-recovery-v0.1/contract.example.json \
      --observations benchmarks/opsbench-sustained-slo-recovery-v0.1/windows.example.json

The exact-head CI workflow parses the schemas/fixtures, compiles the evaluator, produces the example report and runs the mutation suite.

## 6. Adversarial coverage

The tests exercise:
- one passing probe or short horizon is not sustained recovery;
- a p95 regression breaks the hold window;
- insufficient traffic, probes or business checks is indeterminate;
- correct HTTP-like request ratio cannot hide incorrect business results;
- tail latency, goodput collapse or saturation breaches fail independently;
- missing measurements and observation gaps never pass;
- executor/observer/business-oracle trust-domain substitution is rejected;
- histogram bucket totals and exact contract digest are bound.

## 7. Interpretation limits

This checker evaluates supplied window records. It does not authenticate metric publishers, prove the telemetry system was independent, validate the workload's representativeness or establish that the business oracle is correct. OpenTelemetry spans and metrics are useful observations, not cryptographic service-truth proofs. Signed source segments and independent causal evidence remain separate requirements (#174/#178), as does the orthogonal safety assessment (#172).

The example contract and windows are marked example_only; a passing example report cannot be promoted into a production recovery claim. Real confirmatory trials must preregister SLO thresholds, workload profile, observation sources, window duration, minimum samples and hold period before the fault is injected.
