# SREGym native replication profile v0.1

This directory records a **blocked source-identity candidate**, not a completed SREGym run.

## Candidate upstream identity (observed 2026-11-04)

- Repository: [SREGym/SREGym](https://github.com/SREGym/SREGym)
- Main revision candidate: [`51964ce0a911be3e71d9e1e75551bb0089dc70c7`](https://github.com/SREGym/SREGym/commit/51964ce0a911be3e71d9e1e75551bb0089dc70c7)
- README blob: `1dbf111ad26542ca325d03a99a593eb0f6fecb81`
- pyproject blob: `b691f09a0e88049369c6539befebf86d823f457a`
- uv.lock blob: `40614fc1bd27d328062508dea4f8ae2322808412`
- .gitmodules blob: `8e15d8dc2c097d9508f006dca74fd78cee9749be`
- Submodule URL: `https://github.com/SREGym/SREGym-applications.git`

The latest commit message reports 3,483 passing tests, 30 failures and 8 collection errors, described as pre-existing. Those numbers are recorded as **upstream-author-reported only**; they have not been reproduced by Luminous. The manifest intentionally stays blocked until the failures are reproduced and classified.

## Execution constraints

The pinned README describes Python >=3.12, uv >=0.11.8, Docker, kubectl and Helm >=4.0. It describes local Kind as suitable for SREGym-Lite (recommended 8 vCPU / 16 GB) but states not all problems are supported on the emulated cluster. Full coverage uses a self-managed Linux Kubernetes cluster with SSH/root access.

Native comparison must use the default `full` deployment profile. The `svelte` profile changes components visible to an agent and is not leaderboard-comparable with `full`. Keep native SREGym diagnosis, mitigation and E2E metrics separate. Do not add Luminous authority/evidence gates to the native arm; measure those in a separately labeled supplemental arm.

Typical commands from the upstream README:

```sh
uv run main.py --suite sregym-lite --agent <agent> --model <model>
uv run main.py --agent <agent> --model <model>
```

Use the Lite suite only as an explicitly labeled subset. The upstream README describes 17 Lite problems and 90 full-suite problems; do not infer full-suite performance from the Lite cohort.

## Fail-closed readiness

`replication-manifest.json` records the source revision and source-artifact blob hashes observed on 2026-11-04. Null fields and the non-empty blocker list mean the experiment is not ready. A runner must reject this manifest while `readiness` is `blocked`.

The schema makes `ready` conditional on pinned submodules, all runtime versions, an immutable scenario inventory digest, a pinned native scorer, image digests, and a clean independently reproduced upstream preflight. Merely filling fields with guessed values is not sufficient.

Before any score comparison:
1. resolve the submodule gitlink and all OCI image digests;
2. pin cluster, Helm, kubectl, Docker, agent, judge, prompts, tool surface and scoring code;
3. reproduce/triage upstream test failures and freeze the exact scenario ID list;
4. validate that supplemental Luminous instrumentation does not change native tasks or scores;
5. preregister scenario strata, paired blocks, seeds, repeats, missing-run policy and cluster-aware inference.

This manifest does not say SREGym has been run and supplies no agent result.


## Frozen comparison configuration requirements

A manifest cannot transition to `ready` with only source and environment pins. The schema also requires at least one candidate and one baseline arm. Each arm freezes the agent revision, model provider/name/version, system and developer prompt digests, tool-surface digest, memory-policy digest, inference-configuration digest, concurrency, retry policy, and maximum wall-time/step/tool-call/token budgets.

The confirmatory design must include at least five paired seed IDs, randomized arm order, a declared scenario/seed-cluster inference unit, a preregistered missing-run policy, and a digest of the preregistration. The five-seed minimum is a launch floor, not a claim of adequate power; the MDE/power analysis may require more.

Supplemental instrumentation must be demonstrated not to alter the native environment, tool responses, task state or scorer-visible behavior before readiness can be set to `ready`.


## Contamination and access-boundary qualification

A replication cannot be marked ready without a pinned contamination scanner revision/configuration, a passed scan report digest, scan date, and declared examined source families. Each agent arm also needs an access-profile digest covering repository/filesystem/network/secret/cross-run visibility, not only its tool names. A prompt or tool-manifest digest alone does not characterize benchmark leakage exposure.

The scanner must check at least the relevant agent source, prompts, tool/access surface, scenario material, expected answers, runbooks, issue/PR text, logs, and cross-run artifacts. Record what was actually examined; do not mark a family scanned merely because it was inaccessible or omitted.

SREGym's public native scores remain scored by the pinned upstream evaluator. Any Luminous-generated sealed challenge corpus remains a separate supplemental benchmark with its own salted commitment and access-controlled scorer; never add it to the native SREGym score.
