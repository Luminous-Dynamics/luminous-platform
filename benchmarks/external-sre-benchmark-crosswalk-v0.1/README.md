# External SRE / agent benchmark evidence registry v0.1

Latest snapshot date: 2026-11-05
Historical snapshot retained: snapshot-2026-11-04.json

This registry is a machine-readable snapshot of public evidence for external SRE, enterprise operations, agent-risk and harness-evolution benchmarks. It exists to prevent category errors and preserve source context when comparing results.

## Non-negotiable comparison policy

- No composite SRE score.
- No arithmetic conversion between unlike metrics.
- No merging of original EnterpriseOps-Gym and EnterpriseOps-Gym-AA.
- No treating ITBench-AA diagnosis as remediation.
- No treating SREGym-Lite saturation as full SREGym competence.
- No treating a vendor AI-judged demo as equivalent to an independently reproduced native benchmark.
- No turning missing values or unreported repeat counts into zero.

The schema fixes composite_score=false, cross_benchmark_score_translation=false and native_scoreboards_unchanged=true. Unknown fields are rejected.

## Initial coverage

The 2026-11-04 snapshot includes 11 entries:

1. SREGym full (90-problem research description).
2. SREGym-Lite current 17-fault cohort.
3. SRE-Marathon.
4. Original EnterpriseOps-Gym paper evaluation.
5. EnterpriseOps-Gym-AA independent implementation.
6. ITBench-AA root-cause entity attribution.
7. AI SRE Arena v1.
8. Traceway AI SRE Bench.
9. HINTBench intrinsic-risk trajectories.
10. AgentRx critical failure-step attribution.
11. EvoHarnessBench harness-evolution scale.

Each entry describes its construct, task unit, metric definition, denominator, evaluator type, trial design, source URL, snapshot date, limitations and what the score does not establish.

## Source capture status

All external source records in this initial snapshot are marked url_only, with content_sha256=null. The source URLs and retrieval dates are recorded, but this PR does not pretend to have archived and hashed the full external pages or PDFs. A future archival ingestion step may change a source to captured_and_hashed only after storing the exact captured source payload and computing its digest.

Publication dates are null where the primary source gives only a month/year or does not provide a precise date in the captured record. They are not backfilled with invented month-start dates.

## Important score interpretations

- SREGym-Lite displays several 100% end-to-end configurations on 17 faults. Their time and token costs differ materially. The current display does not expose per-configuration repeat counts.
- SRE-Marathon reports a best native Marathon-Score of 41.3/100 across ten methods; it is not 41.3% of faults repaired.
- Original EnterpriseOps-Gym reports 34.1% strict task success in the paper. EnterpriseOps-Gym-AA reports 51.1% for a different independent implementation and leaderboard.
- ITBench-AA reports average precision at full recall over 59 incident snapshots with three repeats. It is diagnostic entity attribution, not mitigation.
- AI SRE Arena is a vendor-published, AI-judged Kubernetes evaluation with 21 injected faults and a small matched subset.
- Traceway uses deterministic grading and one run per case/setup; the six control/trap cases are highly informative but small.
- HINTBench's best risk-step localization result is recorded as less than 37 Strict-F1, not an invented exact value.
- AgentRx's reported 75% is relative average improvement in step localization, not 75% absolute accuracy.
- EvoHarnessBench's 802/520/42/62 values are corpus scale descriptors, not an agent success score.

## CI

The workflow:
- meta-validates the Draft 2020-12 schema;
- validates the entire dated snapshot;
- checks source and observation dates;
- checks metric references and unit consistency;
- checks numerator/denominator coherence when supplied;
- requires explicit source-capture status;
- rejects composite-score/score-translation mutation controls.

The snapshot is not an independent reproduction of external benchmarks and cannot establish Luminous superiority. Superiority claims require same-task, same-harness, same-permission, same-budget and same-evaluator paired experiments with preregistered statistics.


## Refresh 2026-11-05

The dated 2026-11-04 file remains immutable. The new \`snapshot-2026-11-05.json\` adds a same-day source retrieval and refreshes live benchmark results.

### AI SRE Arena: preserve denominator classes

The report distinguishes the 12 incidents both native products investigated from each product's full set of investigations. Those are not competing definitions of the same denominator:
- matched native subset: root cause 11/12 vs 9/12; supported fix 7/12 vs 5/12;
- all native investigations: root cause 15/18 vs 9/12; supported fix 8/18 vs 5/12;
- control CLIs externally investigate all 21: root cause 18/21 and 19/21; supported fix 16/21 and 15/21;
- native detection remains 18/21 and 12/21;
- blast radius, readiness and unsafe final advice are separate metrics.

The publisher page explicitly describes both the matched 12-case comparison and the full-investigation table. The registry uses separate metric IDs instead of blending these scores.

### Traceway: refreshed seven-model cohort

The 2026-10-21/22 campaign reports 172 runs/model, seven models, 62 cases and five setups. The refresh captures all seven overall pass counts, grounded-answer/evidence counts, the 18-run control/trap subset and publisher-reported spend.

The most important restraint signal remains independent of overall pass rate: Opus 18/18 control runs; DeepSeek and Qwen 11/18; MiniMax 10/18; Haiku 9/18; GPT-6 Luna and GLM 8/18. The control set contains only six cases, with 18 runs/model; treat it as a benchmark-specific diagnostic cohort, not a precise general-population estimate.

The refreshed snapshot contains 11 entries and 74 metric observations. The workflow now validates the 2026-11-05 snapshot and preserves the previous snapshot as dated history.

