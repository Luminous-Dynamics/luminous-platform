# Infrastructure assurance benchmark protocol 0.1

**Status:** initial executable contract slice; synthetic-only; not a certification scheme  
**Snapshot:** 2026-11-08  
**Parent programme:** [OMSI-EVAL-001 / Platform #325](https://github.com/Luminous-Dynamics/luminous-platform/issues/325)

## Research thesis

Infrastructure assurance should be evaluated as a **decision-scoped observation-to-consequence chain**, not as a monitoring dashboard, a classifier score, a fleet-wide trust number or a collection of passing validators.

The benchmark asks whether a system can:

1. Qualify the observation method and its operating conditions for a declared use.
2. Distinguish measured, reported, derived, inferred, predicted, simulated and unknown evidence.
3. Discover declared shared failure causes across sources, including transitive and temporal dependencies.
4. Recognize when the evidence frontier is incomplete, stale or contradictory and abstain rather than manufacture independence.
5. Select an action, degraded mode, consultation or abstention under a declared policy and authority boundary.
6. Bind the decision to exact inputs, versions, dependencies, policy and verifier evidence.
7. Verify the resulting consequence and require requalification after a material change.

This is a research/evaluation contract. It does not qualify an instrument, receiver, timing service, industrial asset, operator or safety case.

## Paradigm shifts

### 1. From source counting to causal-dependency accounting

Ten sensors are not ten independent witnesses when they share an oscillator, antenna site, power domain, correction service, firmware lineage, collector or data pipeline. Dependency must be typed, hyperedge-preserving, transitive and valid at the event time. Pairwise independence is a derived view, never the source truth.

The first track is Mycelix's [time-qualified dependency hypergraph #5923](https://github.com/Luminous-Dynamics/mycelix/issues/5923), implemented in [PR #5929](https://github.com/Luminous-Dynamics/mycelix/pull/5929). Its claim ceiling remains **declared common-dependency assessment over the supplied inventory**, not physical or statistical independence.

### 2. From generic confidence to decision-scoped admissibility

A measurement can be accurate enough for a dashboard but not for a timing handover, navigation decision or protected manufacturing release. Qualification must bind the exact decision scope, profile, time interval, evidence frontier, uncertainty, authority and loss context. There is no universal trust score.

### From calibration flags to uncertainty-bearing decisions

Do not model calibration as only `current` or `expired`. The scorecard must preserve the calibration certificate and interval, traceability chain, instrument configuration, drift/stability assumptions, environmental conditions, uncertainty contributors, covariance/correlation assumptions, measurement model and the uncertainty propagation method. When the measurement model is materially nonlinear, require the appropriate higher-order or Monte Carlo treatment rather than assuming first-order propagation is adequate.

The decision record must identify the measurand, requirement/tolerance, measurement result, uncertainty/coverage interval, decision rule, guard band (where used), and consequences of false acceptance and false rejection. Keep “measurement result” and “conformity decision” as distinct artifacts. A syntactically valid certificate or in-date calibration does not itself establish that this particular measurement is fit for this decision.

The 2026 amendment to JCGM 100:2008 explicitly addresses nonlinearity in measurement models and refers to Monte Carlo propagation or higher-order Taylor terms where needed. JCGM 106:2012 addresses how uncertainty enters conformity assessment and the decision rule. These are metrology anchors, not proof that this project implements a compliant uncertainty budget.

### 3. From telemetry health to measurement qualification

A green link, valid signature, fresh timestamp, schema-valid payload or high classifier score is not proof that the underlying physical measurement is fit for purpose. Record instrument identity, acquisition conditions, calibration state, uncertainty method, time basis, reference traceability and environmental limitations separately.

### 4. From independent-looking evidence to explicit common-cause challenge

Benchmark cases must contain individually plausible sources whose hidden dependency structure invalidates a naive vote. Include shared direct dependencies, different direct vendors with a shared transitive ancestor, common timing, common power, common location/antenna, common firmware, common collector and common correction/data pipeline. Include true positive-independence controls so an implementation that returns UNKNOWN for everything cannot pass.

### 5. From model score to selective decision quality

Report risk-versus-coverage, abstention utility, known-class recall at a fixed false-acceptance bound, decision regret and time-to-detection. A rejection threshold is not an improvement if it merely rejects more legitimate evidence. Compare probe selection by expected value of sample information (EVSI) against information gain (EIG), fixed polling and human consultation under matched budgets.

### 6. From green status to verified recovery

A previously good receipt must stop being reusable after calibration expiry, device revision, dependency mutation, timebase change, firmware replacement or evidence revocation. Recovery is a new qualification event, not a timer expiring or an alert clearing.

### 7. From public test labels to evaluator-owned truth

The system under test must not receive evaluator-only truth, hidden dependency edges, test labels, future observations, outcome schedule or hidden loss function. Publish a commitment digest for the frozen hidden truth and policy before test execution; release them after the evaluation lock where permitted. Keep train, validation, calibration and test data separated by the actual leakage groups.

## Benchmark episode model

Each episode preserves these layers as separate objects:

1. **Evaluator truth:** hidden environment state, actual dependency edges, fault schedule and outcome.
2. **Acquisition:** instrument/front-end identity, calibration, configuration, sample quality, timebase and uncertainty.
3. **Observation:** source-owned bytes/records and their asserted semantics.
4. **Evidence:** provenance, signature status, freshness, source authority and dependency snapshot/frontier.
5. **Inference:** classifier, estimator, anomaly or causal hypothesis with calibrated uncertainty.
6. **Policy:** task threshold, tolerated loss, required evidence, operating envelope and abstention rule.
7. **Decision:** accept, degrade, reject reliance, abstain, request another observation, consult a human or request an authorized action.
8. **Consequence:** independently evaluated outcome; predicted impact is not substituted for observed impact.
9. **Recovery:** fresh evidence and independent reconstruction of requalification predicates.

The benchmark report records the exact bytes, digests, split, policy, toolchain, verifier and claim ceiling. The scorecard schema rejects unknown fields and has no place for a composite trust or cross-domain score.

## Track design and task-specific outcomes

| Track | Primary failure to expose | Required reported outcomes |
|---|---|---|
| Dependency hypergraph | False independence from common or incomplete lineage | Shared-domain precision/recall; false-independent rate on hard negatives; UNKNOWN/abstention rate; positive-control specificity; closure and verifier disagreement |
| PNT observation qualification | Fresh/valid-looking fixes masking invalid integrity, stale calibration or incomparable time | False qualification; qualified coverage; detection delay; explicit unknown rate; reason-code agreement |
| RF acquisition × recognition | Confident recognition of corrupted acquisition or unknown emitters | Acquisition qualification separate from open-set macro-F1, unknown-F1, AUROC/AUPR, FPR95, ECE, selective risk and worst-domain results |
| Telecom PTP / SyncaaS | Service availability being confused with physical time/phase accuracy | SyncaaS availability with denominator; independently measured time/phase error; frequency/holdover; source reparenting; clock lineage; management reachability; downstream impact; requalification |
| Edge instrumentation | Software/quantization changes silently degrading physical acquisition | Exact board/firmware/sensor identity; sample loss; false qualification; latency; memory; energy/event; thermal behavior; offline replay |
| Decision-aware probing | More observations without better decisions or excessive probe externality | Decision regret; EVSI/EIG distinction; latency to decision; consultation value; probe/network/energy cost under matched budgets |
| Service requalification | Old-good receipts incorrectly authorizing re-entry | Premature re-entry count; false denial rate; requalification delay; independent receipt reconstruction |
| Manufacturing evidence | Silent schema/adapter loss or forged artifact chain | Exact semantic agreement; zero silent field loss; provenance reconstruction; independent verifier disagreement |

Do not merge a metric across tracks simply because both happen to be percentages. Every metric includes its own unit, aggregation, sample count, denominator when applicable, population identity, missingness and uncertainty method.

## Frozen challenge construction

Start with dependency hypergraph because its semantics are reused by PNT, PTP, RF, edge and recovery.

The owning Mycelix track now versions its public synthetic challenge pack in [Mycelix PR #5929](https://github.com/Luminous-Dynamics/mycelix/pull/5929) (`tools/emsr-dependency-hypergraph/fixtures/dependency-hypergraph-challenges-0.1.json`). The Platform registry references it; it does not copy or fork domain-owned fixtures. The current pack has 11 public conformance cases, including the inverse control that shared power does not imply shared clock. It is not a blind hidden-truth benchmark. The next phase must add evaluator-sealed scenarios whose dependency truth is not visible to the system under test. The challenge families are:

- Six nominal receivers with one shared grandmaster/oscillator.
- Different direct providers with a shared transitive correction or identity ancestor.
- Shared power/network but distinct clock domains, ensuring domain types do not collapse.
- One stale, revoked, conflicted or unavailable edge.
- Missing source scope or incomplete inventory frontier.
- Event-time/current-time divergence and an edge valid only outside the observation interval.
- A complete positive control with no declared common ancestor.
- A changed or removed hyperedge with stale snapshot/digest.
- A valid signature with stale event-time or dependency provenance.
- Pairwise projection checked against the original hyperedge closure.

The evaluator must independently recompute the closure from exact input bytes. It must not trust a supplied result, summary count or digest field without recomputation.

## Baselines and competitive protocol

Every track compares at least four baselines where applicable:

1. **Native/operational baseline:** hardware health, receiver QC, PTP state, vendor monitoring, current adapter or incumbent rule.
2. **Simple transparent baseline:** threshold/statistical rule with the same input and budget.
3. **Strongest faithful task baseline:** reproduced published method with the original split, metric and tuning restrictions.
4. **Ablations:** remove dependency closure, temporal validity, calibration state, acquisition gate, independent verifier or uncertainty handling one at a time.

For RF, the 2026 calibrated open-set prototype paper reports 69.47% open-set macro-F1 and 86.73% AUROC on its curated POWDER cross-day protocol, but reports 55.95% open-set macro-F1 on its Hovering UAV main protocol and 48.45% on its unseen-12ft stress setting. These are distinct protocols and paper-reported results, not a single universal SOTA score. Luminous comparisons must reproduce the protocol or use a clearly labeled matched split; do not transfer these numbers into the PNT or acquisition-qualification score.

For telecom timing, pin each edition and amendment independently. ITU-T G.8275.1 (full timing support) and G.8275.2 (partial timing support) remain separate profile tracks. In G.8275 (01/2026), Annex F is optional PTP performance monitoring and Annex G is optional synchronization-service availability. Availability is not a substitute for independent physical phase/time measurement or downstream service outcome.

## Metrics and proposed gates

Hard gates are reported separately from descriptive metrics. No average can offset a false qualification, false independence, unauthorized effect or premature re-entry.

Initial research targets, not achieved outcomes or published standards:

- 100% exact expected outcomes on the frozen deterministic semantic suite.
- Zero false-qualification, false-independence, unauthorized-effect and premature-re-entry accepts on designated hard negative controls.
- 100% positive-control detection on the declared positive challenge set; report the positive denominator.
- 100% exact input/artifact reconstruction under the pinned verifier.
- Separate precision/recall and UNKNOWN rate; all-UNKNOWN behavior must fail positive controls.
- Report RF ECE, AUROC/AUPR, FPR95, macro-F1, unknown-F1, selective risk and worst-domain results separately.
- Provisional ECE target ≤0.05 only when sample size, class balance and calibration design support a meaningful estimate.
- RF improvement target of at least +5 percentage points against the strongest faithfully reproduced baseline, without material worst-domain regression and without reducing known-class recall to obtain apparent unknown rejection.
- Probe-policy target of at least 20% relative decision-regret reduction against the strongest fixed-budget baseline on frozen synthetic scenarios, followed by independent real-world replication.
- Publish P50/P95/P99 latency, memory, storage, energy and data/network volume at every declared scale. Do not call an unmeasured target achieved.

Zero false accepts in a small synthetic test suite is a software conformance result, not a field-risk estimate.

## Telecom timing profile lock

At the 2026-11-08 snapshot:
- ITU-T G.8275 (01/2026), with Amendment 1 (08/2026), is listed in force.
- ITU-T G.8275.1 (02/2026), with Amendment 1 (08/2026), is listed in force/prepublished.
- ITU-T G.8275.2 (02/2026), with Amendment 1 (08/2026), is listed in force.
- NIST IR 8323 Rev. 2 is an Initial Public Draft published 2026-05-06; it aligns the PNT Profile with CSF 2.0 and is not a final publication.

Keep the standard edition, amendment, date, profile mode, and accessible text/source digest in each timing scorecard. Never silently substitute an older edition.

## Reproducibility and anti-leakage controls

- Freeze case IDs, inputs, expected outcomes, split manifests, grouping keys and evaluation policy before execution.
- Keep hidden truth and negative-control labels outside the SUT-visible bundle.
- Use separate train, validation, calibration and test partitions; group split by receiver, front-end, site, day, campaign, firmware, vendor, topology or source pipeline where these create leakage.
- Pin implementation, configuration, model, evaluator, verifier and environment digests.
- Recompute all supplied digests from exact bytes.
- Require an explicit denominator and missingness accounting for every rate.
- Make empty discovery, missing vectors, missing receipts and unavailable instruments explicit failures or UNKNOWN states; never treat them as zero.
- Publish source rights and permitted redistribution status before using third-party datasets.

## Scope and limitations

Version 0.1 establishes the scorecard contract, registry and evaluation rules. It does not yet implement a complete hidden-truth evaluator, a dataset pack, a field measurement campaign or external independent verification. The dependency hypergraph PR's independent traversal is algorithmically separate but currently shares the language, process and dependency boundary; it is not an organizationally independent verifier.

No certification, regulatory compliance, physical truth, PNT integrity, RF attribution, telecom SLA, safety approval or operational actuation authority is claimed.

## Source anchors

- [ITU-T G.8275 (01/2026)](https://www.itu.int/rec/T-REC-G.8275/en): common aspects of phase/time PTP profiles; Annex F optional performance monitoring; Annex G optional SyncaaS availability.
- [ITU-T G.8275.1](https://www.itu.int/rec/T-REC-G.8275.1/en): full timing support profile; 2026 edition and amendment status.
- [ITU-T G.8275.2](https://www.itu.int/itu-t/recommendations/rec.aspx?lang=en&rec=16548): partial timing support profile; 2026 edition and amendment status.
- [NIST IR 8323 Rev. 2 Initial Public Draft](https://csrc.nist.gov/pubs/ir/8323/r2/ipd): current PNT risk profile draft aligned to CSF 2.0.
- [Calibrated Open-Set Prototypes for Cross-Domain Radio-Frequency Emitter Recognition (2026)](https://doi.org/10.3390/electronics15143077): RF task comparator with protocol-specific results.
- [IEEE 1588-2019](https://standards.ieee.org/ieee/1547/6825/): active precision clock synchronization standard; amendments are tracked by IEEE.
