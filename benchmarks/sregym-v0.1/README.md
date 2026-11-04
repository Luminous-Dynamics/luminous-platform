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


## Dated external leaderboard reference

See [leaderboard-snapshot-2026-11-04.md](leaderboard-snapshot-2026-11-04.md). The official SREGym-Lite-1004 snapshot lists five configurations at 100% E2E on 17 faults. Treat Lite as a compatibility/smoke cohort rather than a differentiating frontier score; retain full-suite and supplemental sealed challenge results separately.


## Frozen built-in full-selection snapshot (4 Nov 2026)

For the pinned commit, the default executable full selection is now derived exactly with `SREGYM_FAULT_DIRS` unset:

- 117 explicit `ProblemRegistry` keys (`registry.py` blob `4f5e9b8774fd39a65342d4aaa997f20418092524`);
- 84 distinct `PORTED_PROBLEMS` identifiers, derived from the three Frappe/Saleor/Slack Spine `SOURCE_PORTS` maps (blobs recorded in the manifest);
- 16 built-in SLO artifacts discovered from `fault.json` beneath `sregym/conductor/problems/apps/` (each artifact metadata blob is present in the upstream Git tree).

These sets are disjoint and yield **217 unique built-in problem IDs**. The sorted ID list is stored in `replication-manifest.json`; the SHA-256 is `20c5addc02d3d2d6a2c74ea190c35101ddde8c2ecb24b96573ba671da8cf681d` over compact UTF-8 JSON (`JSON.stringify`-equivalent array encoding).

The previously advertised 90 full-suite problems remains a *published reference count*, not the actual default executable selection at this newer source commit. The runnable snapshot is 217 when `tasklist.yml` is absent and `SREGYM_FAULT_DIRS` is unset. Any external fault directory changes the selection and must be separately inventoried and hashed. The manifest schema now pins the 217-entry built-in count and digest; readiness stays blocked pending runtime/image/scorer/contamination qualification.

## Newly verified upstream tree and runtime-selection details (4 Nov 2026)

The candidate root commit `51964ce0a911be3e71d9e1e75551bb0089dc70c7` resolves to tree `0a0d4df0b2176d04bad6efeaebbad4905d7977d1`. Its `SREGym-applications` gitlink is pinned at commit `b2006cb6458785c1b353d0fd1753db7fe3a30795`, whose tree is `3c41a3399abeff90abbbe3a8ed68e84b7af36d82`. These identifiers are now recorded in the manifest.

The named SREGym-Lite suite is directly enumerated in `sregym/conductor/problem_sets.py` (blob `ca320fcd9943da78c942104a724f891228822448`). The manifest includes its exact 17 IDs and SHA-256 over compact UTF-8 JSON encoding.

**Important full-suite discrepancy:** this revision's `ProblemRegistry` source (blob `4f5e9b8774fd39a65342d4aaa997f20418092524`) contains 117 explicit dictionary keys before the generated Frappe/Saleor/Slack Spine `PORTED_PROBLEMS` and discovered SLO artifact registrations. The tracked `sregym/conductor/tasklist.yml` is absent; only `tasklist.yml.example` is tracked. The code therefore defaults to all registered entries, and SLO discovery can add artifacts from optional `SREGYM_FAULT_DIRS`. The advertised 90-problem full-suite figure must not be treated as the exact executable ID set for this commit. The manifest remains blocked until the effective full ID set, environment extensions and scorer/oracle closure are frozen.

The image lock file `docker/images.lock.json` (blob `4a51b9d72c55915439f54a606f4b08795e1262db`) has **69 entries, all 69 with sha256 digests**. This does not yet prove every image referenced by all chart values and manifests is locked: `busybox:latest` appears in `sregym/service/apps/values/astronomy-shop-fixes.yaml` (blob `59932e3618a045bb51a2b3bf38d95b83ff2e560a`). The external image-reference scan must close that gap.
