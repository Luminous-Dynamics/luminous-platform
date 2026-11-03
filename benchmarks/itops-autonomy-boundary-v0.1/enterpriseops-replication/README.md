# EnterpriseOps-Gym parity profile

This is the first reproducibility contract for [issue #221](https://github.com/Luminous-Dynamics/luminous-platform/issues/221).

## Comparison target

The manifest targets the ServiceNow NOWAI-Bench EnterpriseOps-Gym leaderboard snapshot synchronized 2026-07-23: Claude Fable 5 (with fallback), oracle mode, 51.1% strict task success. This is a dated published reference record, not a locally reproduced baseline. The separate Artificial Analysis page currently reports a related 1,117-task, three-repeat evaluation with the same 51.1% leader; it is not silently merged with this target.

The schema requires a reference-reproduction record. Direct comparison is not eligible until the exact reference model/fallback/inference/tool configuration is reconstructed and pinned in a reference system-profile digest.

## Verified source pin

The ServiceNow/EnterpriseOps-Gym GitHub default branch was fetched on
2026-11-03. Its exact commit was:

`271f2c357f763376997dfd16807fcde2474ae41b`

The commit object reported a verified GitHub signature and was dated
2026-08-20. The example plan pins this source revision.

## Still unresolved before execution

The Hugging Face dataset revision was not reliably resolved in this authoring
session. The example intentionally leaves it null and marks it unresolved.
The exact public task-ID manifest, domain selection digest, Docker image
digests, database snapshot digest, scorer path/revision, model identity,
prompt/tool/orchestrator digests and run command digest are also unresolved.

The manifest schema has two phases:
- `planning_unpinned`: incomplete values may be null, but the plan must say
  exactly what remains unpinned.
- `ready_to_execute` or `executed`: conditional schema requirements force
  exact 40-character Git SHAs and SHA-256 digests for dataset revision, task
  manifest, scorer, all images/snapshots, model profile, prompts, tools,
  orchestrator, runner, raw receipts and analysis code.

A ready plan cannot be created simply by changing the status field while
leaving unresolved placeholders.

## Scoring

Preserve the benchmark's native oracle-mode task success calculation: a task
succeeds only when every final-state database verifier passes. Partial credit
is not substituted for the native score. Report the per-verifier pass rate as
a diagnostic, never as a replacement for strict task success.

EnterpriseOps-Gym-AA remains a separate evaluation. Do not combine AA scores
with ServiceNow repository leaderboard snapshots or the original PMLR paper
result. Keep additional Luminous participant-effect and authority instrumentation
in a supplemental track; it must not change the task, prompt, tools, environment,
verifiers or native success calculation.

## Repeats and reporting

The example uses three repeats because that matches the separately described
AA methodology. Confirm that three repeats match the exact selected official
baseline protocol before changing the plan to `ready_to_execute`. Record
missing/failed runs rather than silently dropping them. Report strict task
success, per-domain results, per-verifier passes, recovery, latency p50/p95,
token/cost data and raw/analysis digests.

## No score is claimed

This is a pinned-plan template. No benchmark was run, no model was selected,
and no direct comparison or SOTA claim is made.
