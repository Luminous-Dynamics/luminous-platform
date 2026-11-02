# Sealed Continuous OpsBench certification lane v0.1

## Purpose

This is an additive run-freeze and trial-accounting layer for issue #193. It does not replace the v0.1 lifecycle evaluator or v0.2 exact request-impact evaluator. It prevents a benchmark report from quietly changing the evaluated agent, baseline, evaluator, planned trial denominator, or trial history after evaluation has begun.

The implementation is intentionally a **structural validator**. It does not calculate cryptographic hashes, verify signatures, authenticate evidence publishers, sandbox an agent, or enforce network isolation.

## Three evaluation lanes

- **Open development:** public scenarios and public conformance tests; existing cases may predate the run freeze.
- **Sealed certification:** the run manifest is frozen first. Case payload and oracle commitments are generated strictly after the freeze. The agent receives the case payload, not the private oracle. A retirement reveal is forbidden while the case is active.
- **Retired reproducibility:** a case must have been generated and retired before the reproducibility run is created. The run must include exactly one post-retirement reveal per planned trial. Each reveal binds the previously published case/oracle commitments to disclosed artifact digests and salts for independent reproduction.

A sealed run is not made trustworthy by hiding its scenarios alone. It also needs source authentication, restricted evidence access, network/retrieval telemetry, state isolation, scorer validation, and independent trial accounting. Those are separate controls, not implied by this module.

## Freeze protocol

Before any sealed case generation, freeze:
- candidate agent image and source commit;
- model provider, exact model identifier/version, scaffold and prompt digests;
- toolchain, runtime configuration and tool-policy digests;
- a complete matched-baseline identity: provider, model/version, source revision, image, scaffold, prompt, toolchain, runtime config, tool policy and distinct control domain;
- shared execution-environment and resource-budget digests, identical across candidate and baseline;
- evaluator digest and evidence source/control-domain identity;
- complete planned trial and scenario-cluster IDs;
- maximum attempts per trial.

An independent freeze authority produces a commitment to the exact manifest. The FreezeReceipt is checked against the manifest and the expected externally authenticated commitment. The digest algorithm and signature verification belong to the existing typed evidence gate / source adapter; this crate does not implement cryptography.

The manifest is frozen before hidden case generation. The freeze receipt repeats the exact execution-environment, resource-budget, arm-order-schedule, state-isolation, artifact-access and network-egress policy digests; any candidate/baseline mismatch or later change invalidates the run. The environment digest must resolve to the pinned OS/VM/runtime/CPU/GPU profile; the budget digest must resolve to the exact wall-clock, token, CPU, memory, tool-call and concurrency ceilings.

Each planned trial includes a precommitted `first_arm` assignment (Candidate or Baseline). The assignment schedule commitment is part of the manifest and freeze receipt. The validator checks that the first observed activity in each arm respects that assignment. This proves adherence to a frozen schedule, **not** that the assignment was unbiased or generated randomly. For that stronger claim, the freeze authority must authenticate the schedule-generation algorithm, random seed commitment, entropy source and exact assignment bytes through the typed evidence adapter. Each planned trial then gets exactly one ScenarioReceipt, binding a case-payload commitment and a separate private-oracle commitment to the manifest and evaluator. In the sealed lane, generation must occur strictly after the freeze; issuance must occur after generation and no earlier than the freeze.

## Append-only trial accounting

Every planned trial must have one terminal disposition. The validator counts:
- completed;
- failed;
- skipped before execution;
- invalidated;
- all declared attempts;
- unique scenario clusters.

A started attempt must have exactly one terminal event in each arm. Every planned scenario must have a complete Candidate history and a complete Baseline history, both bound to the same case-payload commitment but to their own distinct frozen identity digests. The two arms must share the frozen execution-environment and resource-budget digests, and their observed first activity must match the committed first-arm assignment. The summary preserves per-arm success/failure/skipped/invalidated counts. A candidate-only run is not a matched comparison. Attempts are sequential, bounded by the preregistered maximum, and a retry is allowed only after the previous attempt failed. Completion or invalidation cannot be followed by another attempt. A skipped trial is represented explicitly with attempt zero. Missing events, duplicate event IDs, extra trials, substituted digests, foreign sources, or incomplete attempt histories are rejected; none can disappear from the denominator.

The cluster ID is retained so statistical inference can resample independent scenario/fault-schedule/topology clusters rather than treating requests or repeated attempts as independent samples. Both arms share the same case receipt and planned cluster, but their events bind to distinct frozen identity digests. Resource and execution-environment digests are part of the freeze protocol and must be identical across the two arms.

## Evidence and trust boundaries

The validator checks that:
- manifest/freeze/scenario/trial records all bind to the same run and expected manifest commitment;
- candidate, baseline, evaluator and evidence identities remain pinned;
- evidence source IDs and control domains match the frozen policy and do not collide with the agent/evaluator domains;
- scenario payload and oracle commitments are nonzero and distinct fields;
- scenario generation/retirement timing agrees with the selected lane;
- every retired-reproducibility case has exactly one post-retirement reveal bound to its original case/oracle commitments and disclosed artifact digests/salts;
- open-development and active sealed lanes cannot smuggle in retirement reveals;
- every planned trial has an explicit terminal disposition and complete attempt history.


The v0.1 commitment scheme identifier is `sha256-domain-separated-length-prefixed-salt32-v1`. The independent source-evidence adapter must verify the disclosed artifact bytes against the commitments using these exact constructions:

- Case payload: `SHA-256("org.luminousdynamics.opsbench.case-payload.v1\\0" || u64_be(payload_bytes.len) || payload_bytes || case_salt[32])`
- Private oracle: `SHA-256("org.luminousdynamics.opsbench.private-oracle.v1\\0" || u64_be(oracle_bytes.len) || oracle_bytes || oracle_salt[32])`

The salt is exactly 32 bytes and the case/oracle salts must differ. The adapter must independently verify each artifact's SHA-256 digest, the commitment, the receipt signature and source identity. Domain separation prevents a case-payload commitment from being reinterpreted as an oracle commitment. The Rust structural validator pins the scheme identifier and salt lengths but does not compute hashes or authenticate signatures.

The state-isolation policy must define per-trial writable storage, cache scope, home-directory isolation and whether any cross-trial state is allowed. The artifact-access policy must keep private oracle material unavailable to the agent while allowing only the declared case and evidence artifacts. The network-egress policy must be default-deny or an exact allowlist with resolver, request, response digest and timing telemetry. These policy digests pin intended behavior; an independent sandbox/runtime observer must attest actual enforcement.

The evidence source/control-domain fields are **claims in typed input records** until the source adapter authenticates them. Production must verify them through the typed source-evidence gate (#191 / issue #190), a durable replay guard, and verifier-owned source policy. Do not accept a caller-supplied signature_verified boolean.

The oracle_commitment is not an instruction to expose oracle bytes to the agent. The execution sandbox and artifact store must enforce that separation; this module validates only the declared commitment record.

## Statistical reporting contract

This ledger does not compute a leaderboard score. It produces the full denominator and outcome distribution. Reports should separately provide:
- completed / failed / skipped / invalidated trials, separately for Candidate and Baseline;
- candidate/baseline paired differences by preregistered cluster;
- declared and executed attempt counts per arm;
- cluster count and cluster-aware uncertainty;
- paired candidate/baseline outcome differences on the exact same planned clusters;
- task success, independently verified service recovery, safety, evidence completeness, calibration, cost, wall-clock time, and human-active time.

Skipped and invalidated trials remain denominator entries. Do not silently drop them or replace failed attempts with a favorable retry. For sealed certification, the strongest matched baseline must be frozen in the same manifest.

Safety outcomes remain non-compensatory. Unauthorized committed effects, duplicate critical effects, false recovery claims, or missing evidence are not offset by faster execution or a higher completion rate.

## Validation suite

The Rust conformance tests cover:
- valid completed/failed/retried/skipped/invalidated accounting;
- exact candidate, baseline, and evaluator identity binding;
- complete one-to-one Candidate/Baseline trial coverage with the same scenario commitment;
- observed first-arm activity must respect the frozen `first_arm` schedule, while schedule-generation randomness must be authenticated by the external freeze authority;
- strict post-freeze case generation for the sealed lane;
- pre-retired case timing and exact post-retirement reveals for the reproducibility lane;
- missing/duplicate/unplanned scenarios, reveals and events;
- undeclared attempts, retry-after-success, duplicate starts, missing terminals;
- event time ordering, event source/control-domain substitution;
- manifest commitment and freeze-receipt substitution;
- a 67-vector adversarial mutation matrix.

These are deterministic conformance tests, not empirical agent benchmark results. A schema parse does not demonstrate cryptographic authenticity or secure isolation.

## Claim ceiling and next integration

This layer proves structural consistency and denominator preservation for the records supplied to it. It does **not** prove:
- the manifest or scenario commitments were correctly recomputed from canonical artifact bytes and salts or signed by a trusted authority;
- the runtime actually enforced state isolation, artifact-access separation or the declared network-egress policy;
- the agent could not read private oracle data or retrieve leaked answer artifacts;
- the model provider/version actually matched the declared identity;
- events were emitted completely or honestly by the named source;
- the scenario distribution represents production;
- any agent is contamination-free, safer, or better than SOTA.

Next integrations should be thin adapters, not duplicate authorities:
1. use #191 / issue #190 source-bound typed evidence and durable replay for freeze, scenario and trial receipts;
2. enforce sealed artifact access and allowlisted retrieval in the execution sandbox;
3. feed this validator from the independent evaluator, ingress observer and participant-owned ledgers;
4. join the resulting run receipt with exact paired impact cells from #192;
5. publish a signed result receipt through the existing evidence infrastructure after independent validation.

Platform Saga remains the episode coordinator; the broker enforces mutation authority; participant-owned ledgers own remote effects; independent workload/business observers own service outcomes; Nixward remains the sole host transaction/interlock/journal/recovery authority.
