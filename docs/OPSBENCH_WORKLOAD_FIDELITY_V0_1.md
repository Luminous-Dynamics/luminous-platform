# OpsBench Demand-Conditioned Workload Fidelity v0.1

**Status:** offline evaluator with synthetic workload, phase, load-generator, fault-controller and service-observation receipts. No live load test, production capacity claim or authenticated telemetry source is qualified.

## 1. The paradigm shift

A service is not recovered merely because its aggregate request-success ratio looks healthy. An agent can create a misleading success result by shedding the most important request class, allowing only easy low-latency requests through, relying on a lighter arrival rate or fewer concurrent requests, silently changing request payloads or client timeouts, suppressing retries from accounting or causing retry storms, using an easier fault schedule, or averaging high-volume routine traffic over failed low-volume critical traffic.

The evaluation therefore freezes a demand contract before the trial and evaluates every request class in every challenge phase. The load generator—not the system under test—sets the offered-demand denominator. The ingress observer and business oracle independently measure what the service accepted and whether its outcomes were correct.

This layer composes with sustained SLO recovery #181. It does not replace its final consecutive healthy-window hold or independent business outcome requirements.

## 2. Five separately bound artifacts

The offline evaluator consumes five exact artifacts plus a contract:

1. **Workload profile:** request classes, criticality, allowed class-share range, target offered rate, payload profile ID, client timeout, peak concurrency range, success/correctness/ingress floors, minimum business-check count, p95 latency ceiling and retry budget.
2. **Phase schedule:** exact ordered window boundaries and offered-rate multipliers for nominal, burst, dependency degradation, retry pressure and recovery hold.
3. **Load-generator receipts:** independent offered count, offered rate, client-observed successes, errors, timeouts, drops, latency histogram and peak concurrency for every (phase_id, class_id).
4. **Service/business observations:** exact window times, ingress received/rejected counts, server retry attempts, and independently checked/correct business outcomes.
5. **Fault-controller receipts:** exact phase/fault IDs, target, action, action digest, execution time and injected disposition for the dependency-degradation and retry-pressure phases.

The contract binds the raw bytes of all five artifacts with SHA-256 and binds separate source/trust-domain identities for the executor, load generator, ingress observer, business oracle and fault controller. The identities must occupy distinct trust domains. The evaluator explicitly does not cryptographically authenticate these identities; producer signatures and trust-policy checks belong to evidence profile #178 before a real run can be accepted.

## 3. Request classes and frozen demand

The synthetic example includes two classes:

| Class | Target offered rate | Class-share band | Criticality |
|---|---:|---:|---|
| critical-payment | 10 requests/s | 8–12% | Critical |
| routine-read | 90 requests/s | 88–92% | Non-critical |

Each class also pins payload-profile ID, client timeout and concurrency range. Every load receipt must reproduce those exact values. This prevents a benchmark run from changing the payload or silently relaxing a timeout while retaining the same class name and approximate request rate.

The phase schedule uses nominal demand at 0.95–1.05× target rate, burst at 1.45–1.55×, dependency degradation at nominal demand with a named fault receipt, bounded retry pressure at nominal demand with a separate fault receipt, and recovery hold at nominal demand.

The sample phase receipt/action digests are synthetic fixtures, not proof that a real fault injector executed anything. The fault-controller receipt artifact is included in the contract digest; an upstream signed producer receipt is required for confirmatory use.

## 4. Denominators and class-level decisions

For each class/phase window, every externally offered request must end in exactly one client-observed fate: successful response, error response, timeout, or dropped request. The sum must equal the independently measured offered count. The latency histogram must also account for the full offered count; timed-out and dropped operations cannot disappear from its denominator.

The service observer separately reports requests received at the ingress boundary, service-side rejections, server retry attempts, and business checks/correct business outcomes.

The evaluator then applies each class's frozen profile:
- load-generator under-delivery or class-mix drift is indeterminate because the intended challenge was not delivered;
- insufficient business checks or a p95 quantile hidden in the overflow bucket is indeterminate;
- ingress below the floor, any service-side rejection, any timeout, any drop, success/correctness below the class floor, p95 above its ceiling or retries above budget is fail;
- only complete, in-profile, within-threshold class/phase records pass.

A candidate cannot reduce the denominator by refusing requests. Any non-zero service rejection, timeout or drop is retained and fails that class window, irrespective of aggregate traffic success. A critical class failure fails the overall envelope even if routine traffic performs well.

The evaluator reports aggregate response success only as a descriptive measure. It is never an override for class-level failure.

## 5. Example interpretation

The synthetic example offers 27,000 requests across five phases and two classes. Aggregate response success remains above 99%, yet the critical payment class fails during dependency degradation because response success, business correctness and tail latency breach its frozen thresholds; the critical class fails again under retry pressure; while the high-volume routine-read class remains within its own request, correctness, latency and retry thresholds.

The expected computed disposition is fail; the top-level report remains marked example_only. This demonstrates why aggregate service-health percentages are insufficient—it is not a Luminous, commercial-agent or production-capacity result.

## 6. Run the offline evaluator

From the repository root:

    python3 tools/opsbench-workload-fidelity-v0.1/evaluate_workload_fidelity.py \
      --contract benchmarks/opsbench-workload-fidelity-v0.1/workload-contract.example.json \
      --profile benchmarks/opsbench-workload-fidelity-v0.1/workload-profile.example.json \
      --schedule benchmarks/opsbench-workload-fidelity-v0.1/phase-schedule.example.json \
      --load-receipts benchmarks/opsbench-workload-fidelity-v0.1/load-generator-receipts.example.json \
      --service-observations benchmarks/opsbench-workload-fidelity-v0.1/service-observations.example.json \
      --fault-receipts benchmarks/opsbench-workload-fidelity-v0.1/fault-controller-receipts.example.json \
      --output /tmp/opsbench-workload-fidelity-report.json

Run the adversarial tests:

    python3 -m unittest discover \
      -s tools/opsbench-workload-fidelity-v0.1 \
      -p 'test_workload_fidelity.py' -v

The evaluator uses only the Python standard library. It reads local artifacts and does not launch load, contact a service, inject faults or mutate a customer system.

## 7. Adversarial coverage

The mutation suite covers a high aggregate success ratio hiding critical-class failure; request shedding visible at the independent ingress boundary; non-zero service-side rejection; timeouts and drops retained in the offered denominator; retry-pressure budget breach; independent business correctness breach despite successful HTTP-like responses; load-generator under-delivery and request-class mix drift; omitted or duplicate class/phase windows; phase schedule gaps, wrong phase IDs and fault-receipt substitution; observer timestamps not matching the registered window; profile/schedule/load/observer/fault raw-byte digest mismatch; payload-profile, client-timeout, concurrency and load-generator identity substitution; and duplicate-key, malformed JSON, non-finite numeric and bounded-input parsing.

## 8. Interpretation limits and promotion gates

example_only data cannot be promoted by changing a status label. A real study must preregister the profile, phase schedule, fault schedule, workload class weights, payloads, timeouts, concurrency, thresholds, observation sources, independent load generator and business oracle before running the trial.

Before confirmatory workload-fidelity claims:
1. Verify each producer segment through #178 (load generator, ingress observer, business oracle and fault controller must each authenticate its own source records).
2. Feed the resulting exact receipts through #174's causal/run-binding verifier and #172's orthogonal safety evaluator.
3. Run sustained SLO recovery #181 over the same workload and preserve the complete observation horizon.
4. Keep confirmatory task variants sealed under #180.
5. Compare agents only under the same workload profile, phase schedule, fault receipts, tool permissions and budgets.
6. Retain every started run, including crashes, timeouts, blocked trials and indeterminate outcomes.

The evaluator does not establish publisher authenticity, load-generator independence, observer completeness, business-oracle correctness, task representativeness or production capacity. Its population_capacity_claim_permitted and automatic_execution_authorized outputs are always false. No production load-testing or service-restoration guarantee is implied.
