# Luminous Edge counterfactual policy-impact gate v1

**Status:** the impact evaluator is integrated as a hard pre-actuation gate in the rollout coordinator. CI still needs to validate the exact current head. The current simulator is supplied by a typed input contract; this PR does not implement or authenticate a Batfish/vendor simulator.

## Paradigm shift: predict the blast radius before touching the device

A controller command can be syntactically valid and still introduce unauthorized reachability or break an essential service. The coordinator now requires a counterfactual vector for the exact frozen intent corpus before it acquires the device lease or sends a policy update.

For every corpus case, the vector carries predicted baseline and candidate outcomes. The gate binds those predictions to:
- the intended target token;
- the exact SHA-256 digests of approved baseline and candidate policy bytes;
- the exact frozen corpus byte digest;
- a simulator identifier and version; and
- a complete, duplicate-free set of case IDs matching the corpus.

The reconciler produces a deterministic, content-digested impact report with per-case intended outcome, baseline outcome, candidate outcome, candidate mismatches, baseline defects, new permissions, new blocks, high-risk false permits and unknown predictions.

A candidate is preflight-promotable only when every candidate prediction matches the corpus and no candidate prediction is unknown. A baseline defect is reported separately: the baseline can be wrong while a candidate repairs the case. Unknown baseline predictions do not automatically veto a known, conformant candidate; unknown candidate predictions do veto it.

## Where the gate runs

The rollout coordinator invokes the gate before it acquires a device lease. If the vector is absent, malformed, bound to another target/candidate/corpus, incomplete, has duplicate/unknown IDs, has any candidate outcome mismatch, or has an unknown candidate result, the rollout is rejected before a device lease, journal operation, or external write is attempted.

When preflight passes, the coordinator returns the joined preflight report alongside the promoted operation. The report is an audit artifact for the caller; in this version its digest is not yet a separate column in the durable promotion journal or a field in the freshness challenge. Persisting that report digest into the durable operation/receipt is the next integration step.

Crucially, preflight predictions are **not** live observations. They are never substituted for the independent, challenge-bound data-plane probe. The complete policy lifecycle remains:

1. Verify the candidate's counterfactual behavior against the frozen corpus.
2. Acquire the exclusive target lease.
3. Record apply intent and conditionally apply against the current generation/revision.
4. Read back and verify the exact effective policy.
5. Run the independent challenge-bound probe, verify freshness/conformance and consume the nonce durably.
6. Re-read the policy and promote, or enter guarded rollback/recovery.

## API

Module: `tools/edge-policy-observation-reconciler/src/impact_gate.rs`.

- `PolicyPreflightVector` is the input from an offline differential simulator/adapter.
- `evaluate_policy_impact(...)` returns `PolicyImpactReport`.
- `RolloutRequest` now requires a `preflight` vector.
- `PolicyRolloutCoordinator::apply_and_promote` calls the evaluator before acquiring the device lease and returns a `RolloutOutcome` containing the promotion and report.

The vector's simulator identifier/version is provenance metadata, not authentication. A malicious or incorrect simulator can still claim the wrong predictions. A production implementation should bind its outputs to a specific canonical policy artifact, network snapshot/topology and vendor simulator version, and run differential simulations from independently controlled data sources. This gate validates completeness and consistency of the declared result; it does not magically make the simulator trustworthy.

## Competitive comparison

This extends, rather than claims to replace, published industry techniques:

- [Batfish differential questions](https://batfish.readthedocs.io/en/latest/notebooks/differentialQuestions.html) compare snapshot reachability and policy behavior to reveal changes in flows.
- [Ansible network validation](https://docs.ansible.com/projects/ansible/latest/network/user_guide/validate.html) validates device/file state against explicit criteria and can identify drift.
- [Junos confirmed commit](https://www.juniper.net/documentation/us/en/software/junos/cli/topics/topic-map/junos-configuration-commit.html) provides an auto-revert mechanism when a temporary change is not confirmed.
- [PAN-OS session rematching](https://docs.paloaltonetworks.com/ngfw/networking/session-settings-and-timeouts/configure-session-settings) shows why the effective behavior for established sessions must be qualified per product and setting.
- [RFC 9334](https://www.rfc-editor.org/rfc/rfc9334.html) discusses freshness and the race that remains after an evidence result is generated.

The differentiation hypothesis is the combination of full-corpus counterfactual analysis, an exclusive conditional rollout, exact effective-state readback, fresh independent probes, durable replay consumption, and explicit recovery—all with one target/generation/policy identity. The current input vector is not Batfish execution, and this is not yet a vendor comparison or SOTA result.

## Benchmark plan and proposed acceptance targets

These are qualification targets, not achieved claims.

| Dimension | Test protocol | Proposed target |
|---|---|---|
| Coverage | Compare vector case IDs against every versioned corpus case | 100% coverage; zero missing, duplicate or unknown cases |
| Unauthorized reachability | Inject a candidate that permits every registered critical deny case | Preflight blocks before acquiring the target lease or issuing writes |
| Service regression | Introduce candidate false blocks for permitted essential services | Every regression is named in the report and blocks rollout |
| Prediction uncertainty | Inject unknown candidate outcomes, malformed simulator metadata and stale corpus hashes | All fail closed |
| Candidate digest binding | Reuse a valid prediction vector for a different policy byte digest/target/corpus | Zero mismatched vectors admitted |
| Determinism | Repeat same exact inputs across process runs | Identical report bytes/digest under the supported serialization contract |
| Scale | 100, 1,000, 10,000 and 100,000 corpus cases/targets where supported | Report wall time, CPU, memory, bytes per report and p50/p95/p99 |
| Simulator calibration | Compare predicted case outcomes with independently observed post-change probes across at least 30 randomized paired physical trials per device/firmware/policy tuple | Report confusion matrix, false-permit and false-block rates with denominators; no universal zero-error claim from small samples |

For industry comparisons, run the same topology, intent corpus, baseline/candidate policy bytes, failure injections and positive controls through Luminous and the relevant vendor/analysis tool. Report predicted-vs-observed agreement, false permits, false blocks, service interruption, time to restore, scale and operator toil. Include inconclusive trials rather than silently excluding them.

A preflight pass is permission to continue toward live verification—not permission to promote.