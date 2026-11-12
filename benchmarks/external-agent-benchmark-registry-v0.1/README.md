# External agent benchmark evidence registry v0.1

**Snapshot date:** 2026-11-08  
**Status:** metadata snapshot; exact source revisions are pinned where available, but most raw source bytes are not archived  
**Records:** 14  
**Schema:** JSON Schema Draft 2020-12, strict objects

This is a provenance registry, not a composite leaderboard. Unlike constructs are never normalized into a single SRE or agent score.

## What it covers

The 14 records span:

- SREGym full and Lite cohorts (kept separate);
- SRE-Marathon continuous diagnosis/repair;
- Radar's matched tool-surface study;
- Traceway AI SRE Bench;
- AI SRE Arena;
- ITBench-AA;
- EnterpriseOps-Gym;
- TraceTrial-Bench harness reliability;
- AgentGovBench governance;
- EvoHarnessBench harness evolution;
- Datadog Scout / AlertEval;
- NIST's evaluation-probe methodology.
- PagerDuty PD Labs' 17-scenario SRE Agent investigation benchmark.

Every record identifies the measured construct, unit, source, publication and observation dates, run protocol, evaluator, metrics, comparability group and claim ceiling. A vendor/owner result remains labelled as such until independently reproduced.

## Paradigm shifts for Luminous

### 1. From answer correctness to independently verified outcomes

Preserve three different truth planes:

1. **Proposal:** what the agent claimed, planned or attempted.
2. **Enforcement:** what the runtime authorized or denied, under which principal, policy epoch, target, purpose, scope and exact operation digest.
3. **Effect:** what the participant actually committed, independently read back from its effect ledger and observed.

A blocked prohibited attempt can be an enforcement success but remains an agent-alignment failure. A gateway receipt does not prove that no side effect occurred. A missing effect record is indeterminate, never a pass.

### 2. From poll-and-think to event-qualified attention

Platform PR #321 separates low-cost monitor checks from policy invocations and eligible read-only probe proposals. It deliberately does not infer mutation permission from a probe selection.

The next event study (#323) must handle durable and ephemeral events, concurrent incidents, duplicate delivery, source collisions, cursor gaps, deadlines and revocation while dormant. A rising-edge boolean is an initial microbenchmark, not a durable event system. Coalescing is acceptable only when all incident identities, scopes and purposes remain reconstructable.

### 3. From fixed harnesses to continuously requalified systems

Tools, schemas, skills, specialist agents, identity adapters, policy and observers all change. Each change creates a new harness epoch. Qualification must be bound to exact artifact digests, and an impact assessment must identify which previous claims remain valid and which require re-testing.

A capability addition must not silently inherit old safety or permission qualification. See Symthaea #11030.

### 4. From one leaderboard number to a non-compensatory assurance frontier

Keep diagnosis, recommendation quality, actual repair, sustained recovery, unauthorized attempts, committed effects, cross-tenant disclosure, positive-control acceptance, evidence completeness, event recall, harness retention, latency and cost separate.

No weighted average may compensate for an unauthorized committed effect, a cross-tenant disclosure, a missing effect receipt or a lost safety qualification.

### 5. From token cost to cost per verified outcome

Measure full-system cost: inference, tools, event transport, telemetry, probes, retries, rollbacks, human escalation and recovery. Publish both native task cost and cost per independently verified restored service outcome. A vendor's estimated inference price is not an audited total operational cost.

PagerDuty's benchmark is a useful methodology comparator: its 17-scenario sweep uses four fault-injection mechanisms, a three-provider LLM judging panel, evidence grounding and misdirection scores. However, it is a single trial per agent/scenario, failures are excluded from its quality denominator, the benchmark substrate is private, and it does not execute repair or verify sustained recovery. Luminous should preserve those distinctions rather than copy its aggregate score.

## Research targets (targets, not achieved results)

| Programme | Initial target and comparison discipline |
|---|---|
| SREGym full | Reproduce the native 90-fault × 3-attempt cohort and same-run diagnosis+mitigation scorer. Aim for ≥97% E2E on a preregistered repeated fault-level evaluation, with cluster-aware intervals, cost and latency. The 95.2% Radar result is a maintainer-verified participant report, not our replication. |
| SRE-Marathon | Exceed 41.3 native Marathon-Score and target ≥0.20 active-fault repair fraction. Also publish full resolution counts, user-impact degradation, risk and resource use. |
| PagerDuty 17-scenario | Reproduce the public protocol where possible; add repeated trials, count failed investigations in task-level outcomes, publish judge-human agreement and independent participant-effect/recovery metrics. Treat the private harness as an external methodology reference until its task artefacts are available. |
| Traceway AI SRE Bench | Preserve its 62 cases, five setups and deterministic grader. Beat the verified-pass-per-dollar frontier while retaining safe-write and hidden-test checks; do not optimize answer-only accuracy. |
| ITBench-AA | Target AP@FR >60% on the exact 59-task set and three-repeat protocol; show per-task zero-false-negative coverage and precision separately. |
| AgentGovBench | Reproduce all 48 native scenarios and target 48/48. Separately require zero unauthorized committed effects, zero cross-tenant disclosure/effects and 100% valid positive-control acceptance. |
| TraceTrial-Bench | Preserve the 95 × 9 × 4 task/model/harness design where permitted. Aim for ≥20% Clean versus its reported 8.4% corpus rate, while retaining Agent Score and Harness Macro separately. |
| EvoHarnessBench | Target ≥95% prior-task retention, 100% retention of safety/authority controls, zero silent qualification inheritance and separately measured new-task adaptation. |
| EnterpriseOps-Gym | Target >50% on a permitted full native split; publish infeasible-task refusal, side effects and SQL-verified final state separately. Never call a small ITSM subset the full score. |
| Paired attention | 100% valid in-deadline critical-event detection in the frozen corpus; zero duplicate credit, silent sequence-gap acceptance or identity-collision acceptance; ≥50% fewer policy invocations on declared quiet/duplicate workloads without reduced critical-event recall. |
| Evidence integrity | 100% exact source/effect lineage reconstruction on scored trials, zero missing-as-success conversions and replay from exact policy, source, harness and runner digests. |

These are staged research thresholds, not results. Native benchmarks must be reproduced with their original scorers; all Luminous-only metrics remain supplemental. A performance claim requires matched task sets, model/harness controls, paired fault-level results and uncertainty—not just a higher point estimate.

## Current source and evidence limitations

The snapshot is marked `metadata_snapshot_partial`. Source identity, repository commit, Git blob SHA and content SHA-256 are separate fields:

- SREGym's 95.2% row is pinned to the official `SREGym-website` repository commit and the exact `lib/leaderboard-data.ts` Git blob. The Git blob ID is SHA-1; it is not called a SHA-256 of a captured web page.
- AgentGovBench, TraceTrial-Bench and AI SRE Arena repository commits are recorded separately from page-byte hashes.
- Most paper and website sources are metadata-only. Their exact PDF/page/data bytes are not yet archived.
- NIST Evaluation Probes is an ongoing method project; the current project page provides no numeric leaderboard score.
- PagerDuty's 17-scenario substrate is private; its method report is public, but scenario/task bytes and the detailed result table are not archived here.
- The AgentGovBench repository root license is MIT. Scenario assets, datasets, dependencies and copied results still need individual license checks before redistribution.

Do not promote this snapshot to source-byte-complete until the primary source files, raw leaderboard data, scorers, exact revisions and applicable licenses have been captured and hashed.

## Files and validation

- `registry.schema.json`: strict Draft 2020-12 schema.
- `registry.snapshot-2026-11-08.json`: latest 14-record source-pinned metadata snapshot. The 7 November file remains an immutable prior snapshot.
- `validate_registry.py`: validation-only cross-field checks and negative controls.
- `requirements.txt`: pinned JSON Schema validator for CI.

Run locally:

```sh
python3 -m pip install -r requirements.txt
python3 validate_registry.py
```

The Python file is validation glue only. The benchmark/runtime code remains Rust.

## Related work

- Platform [#268](https://github.com/Luminous-Dynamics/luminous-platform/issues/268): source-pinned benchmark evidence registry.
- Platform [#274](https://github.com/Luminous-Dynamics/luminous-platform/issues/274): non-compensatory capability and assurance frontier.
- Platform [#318](https://github.com/Luminous-Dynamics/luminous-platform/issues/318): matched model/tool-surface ablation.
- Platform [#321](https://github.com/Luminous-Dynamics/luminous-platform/pull/321): paired fixed-poll / event-triggered attention experiment.
- Platform [#322](https://github.com/Luminous-Dynamics/luminous-platform/issues/322): AgentGovBench governance adapter.
- Platform [#323](https://github.com/Luminous-Dynamics/luminous-platform/issues/323): durable, ephemeral and concurrent event semantics.
- Symthaea [#11030](https://github.com/Luminous-Dynamics/symthaea/issues/11030): harness evolution, retention and safe adaptation.

**Claim ceiling:** this registry improves provenance and prevents category errors. It does not independently reproduce the published scores or establish that Luminous is better than an external system.
