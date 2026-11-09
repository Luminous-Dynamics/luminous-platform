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


## Native SREGym replication profile

The separate native-run manifest contract for issue [#238](https://github.com/Luminous-Dynamics/luminous-platform/issues/238) lives alongside (but does not replace) the external evidence registry:

- `native-sregym-replication.schema.json`: strict Draft 2020-12 profile for one native study or a matched tool-surface ablation.
- `sregym-0508-luminous-replication.template.json`: planning-only full-cohort template.
- `validate_registry.py`: validates the template and negative controls as part of the existing registry workflow.

The template pins the reported SREGym-0508 cohort as **90 scenarios × 3 attempts = 270 expected attempt rows**. SREGym-Lite-1004 is a separate 17-scenario cohort; the validator rejects a Lite/full substitution. The exact official cohort manifest is `SREGym/SREGym-website@c4101aace3578353c96a2d837f60e48c3e95d9c4:public/data/sregym-0508-problems.csv`, Git blob `8943b471a38f03238321b3e052a147a7fb8a668c`. Its byte-identical local snapshot is `sregym-0508-cohort.csv`; the validator recomputes the Git blob SHA-1 from the archived bytes. The official leaderboard observation remains separately identified in `cohort_observation_source` (it points to `lib/leaderboard-data.ts`, blob `bdb3c26b361cbfcdb7e68425af65f4b0d51434b1`), so a leaderboard-data blob is not confused with the cohort CSV.

The historic source reconciliation is now concrete. `SREGym/SREGym@ce73adf2c9212b4804793cbbabbfbb33eccd0f0f` (24 April 2026) has 90 registered problem IDs and its exact registry key set matches the official 90-ID CSV, with no additions or omissions. The registry source blob is `3ff155517a8e1982cdce44ec69677feee2d0aee6`. The subsequent 6 May revision `2f5fd1c73f5f1f9b2702d15cdc79bb25dfa774a9` has 91 registered IDs, adding `load_spike_rpc_retry_storm` beyond the published 90-fault cohort. Current November `main` is a much newer benchmark revision and is not a substitute for this historic source.

One runtime hazard is now explicit: the checked-in `tasklist.yml.example` has 86 IDs, not the full 90. In this SREGym revision, `ProblemRegistry.get_problem_ids()` returns all registered IDs only when the gitignored local `sregym/conductor/tasklist.yml` is absent. Therefore a full-cohort reproduction must pin the 24-April source commit and record that the local tasklist override is absent; the example file must not be copied into place for the full run. The manifest pins the candidate source revision for a new reproduction, but `execution_revision_attested` is deliberately `false`: exact ID-set equivalence does **not** prove that every historical leaderboard execution used this exact Git commit.

The profile is still `planning_only`: scorer/oracle lock digests, exact submodule and container images, cluster/runtime/reset/fault-injector configuration, model/provider/agent/prompt/tool identities and budgets are not yet all captured.

The profile separates `native_replication` from `matched_tool_surface_ablation`. A matched ablation requires one native reference arm, multiple arms under a common pinned model/agent/prompt/permission/effort/sampling configuration, and a capability-equivalence artifact; the tool-surface digest is intentionally allowed to differ. A native replication uses one candidate arm. Both modes require the native benchmark harness/task/scorer to remain unchanged. Luminous-only authorization, impact, observer and sustained-recovery controls are separate supplemental metrics and cannot modify native SREGym scoring.

`ready_to_run` is rejected until the exact SREGym repository commit, scenario inventory, scorer, submodules, images, cluster/runtime/reset/fault-injector configuration, every model/agent/prompt/permission/tool identity, budgets and preregistered protocol digest are pinned. `executed` additionally requires every expected attempt row to have a retained terminal record plus exact receipt/result artifact digests. Dropped attempt rows are prohibited; timeouts and failures are retained as outcomes, not silently omitted. A missing run remains visible in the expected denominator.

The existing registry workflow validates the new schema/template and rejects negative controls for cohort substitution, denominator changes, native-gate contamination, dropped attempts, false readiness and incomplete execution. These checks validate the profile contract only. They do not run SREGym, verify external scorer bytes or claim a Luminous benchmark score.


The new validator cross-checks the three-way join between the upstream cohort CSV, the archived registry-ID snapshot and the pinned candidate registry metadata. It verifies the copied CSV's Git blob SHA-1 byte-for-byte, checks all 90 unique problem IDs and categories, compares the cohort and registry sets exactly, validates the tasklist selection policy and retains a negative control for substituting the later 91-ID source revision. The source-match check does not upgrade the historical-run attestation bit.


### Native oracle provenance and failure semantics (9 Nov 2026)

The planning manifest now pins the exact implementation files that form the native Conductor/oracle path at the selected SREGym candidate commit. The source snapshot is `sregym-0508-oracle-contract.json`; the manifest carries all 12 role/path/Git-blob identities, and the validator checks them against that archived snapshot. It includes the Conductor and API stage runner, problem registry, structured diagnosis oracle, LLM-as-a-judge wrapper/engine/checklist, generic mitigation oracle, AlertOracle, SustainedReadinessOracle, and the Kafka queue and kubelet-crash problem definitions. The native scorer path is explicitly `sregym/conductor/conductor.py`, not a null placeholder.

The source review also freezes the native failure semantics that must not be silently rewritten during replication:

- An unavailable LLM-judge backend returns a null verdict/success, which is unknown and cannot receive same-run E2E credit.
- After two checklist-parse failures, the judge fills missing answers with No/Low-confidence rather than treating the result as a pass.
- Structured diagnosis checkpoint drift is logged as a warning; the diagnosis oracle evaluates against its newly observed expectation.
- AlertOracle's Prometheus query helper returns an empty firing-alert list on query/timeout/JSON errors; under the native implementation that can be interpreted as sustained alert silence. This is a **native fail-open risk**, not a property we endorse.
- AlertOracle uses a 30-second buffer, 10-second polling, a 120-second silence window and 15-second query timeout. The generic mitigation oracle waits up to 60 seconds for rollout settlement, then evaluates the current pod/container state.

The exact native score must preserve these behaviors for comparability. Separately, a Luminous supplemental oracle-reliability result should track telemetry query availability, judge availability, unknown outcomes and whether the oracle had adequate evidence. It must not overwrite or alter native diagnosis, mitigation or E2E metrics. This makes a native leaderboard reproduction comparable while still making scorer blind spots visible.

The profile remains `planning_only`: the exact runtime/container/submodule/image/reset/agent/model pins and actual execution receipts are still required before a native run can be marked ready or executed.
