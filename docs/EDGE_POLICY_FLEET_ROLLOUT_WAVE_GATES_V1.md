# Luminous Edge fleet rollout planning and wave gates v1

**Status:** deterministic planning and pure evidence-gated decisions; no device actuation is implemented here.

**Claim ceiling:** the planner bounds the blast radius of a proposed wave schedule and classifies submitted observations. It does not authenticate their producer, discover true topology, predict outages, apply policies, execute rollback, or prove fleet safety.

## Paradigm shift: from per-device correctness to bounded fleet risk

A policy can be correct on one device and still cause a fleet incident when applied everywhere at once. The fleet-level question is not just “did each command succeed?” It is “what is the maximum correlated harm we can cause before independent observations justify proceeding?”

This implementation introduces two pure, deterministic pieces:

1. **Blast-radius-aware plan:** exactly one operator-designated canary is isolated in wave zero. Remaining targets are assigned deterministically across explicit failure domains, with both a maximum wave size and maximum number of targets from any one failure domain.
2. **Evidence-gated wave decision:** a wave advances only when every planned target has exactly one matching observation, bound to target and policy generation, with the expected effective-policy digest, fresh evidence, durable nonce consumption, an independent probe, and a passing positive-control path. A definitive failed probe with the positive control intact requests rollback of the current applied wave. Missing, stale, mismatched, replay-unconsumed, non-independent, duplicate or inconclusive evidence halts for reconciliation.

The decision engine never assumes that a dead listener is proof of successful denial. That is why a target-probe failure is only a definitive negative when the independent positive-control probe passes.

## Deterministic plan contract

Module: `tools/edge-policy-observation-reconciler/src/fleet_rollout.rs`

- `plan_rollout(targets, constraints)` validates target tokens, failure domains, policy generations and digests before producing waves.
- Exactly one canary must be explicitly selected. The planner does not guess which target is safe to canary.
- Wave zero contains only the canary, even if larger batches are allowed.
- General targets are ordered deterministically and scheduled round-robin across lexicographically ordered failure domains.
- Duplicate target tokens, a zero batch/failure-domain budget, invalid digests and unchanged candidate policies are rejected before any plan is returned.
- A SHA-256 digest identifies the canonical plan content. It is an identity/checksum, not a signature or authorization.
- The production module no longer exports a boolean-only `evaluate_wave` API. The former claim classifier and `FleetTargetObservation` shape are compiled only for unit tests, so production callers cannot turn `freshness_valid=true` or `nonce_durably_consumed=true` into a wave decision.
- `validate_wave_permits(journal, wave, permits, now)` checks exactly one live `PromotionPermit` per target against the durable promotion journal: current verifier-process session, unexpired evidence, `Verified` state, target, generation, baseline/candidate policy digests, challenge ID and receipt digest. It returns a private-field, non-Clone `VerifiedWaveBindings` capability or `HaltAndReconcile`.
- `VerifiedWaveBindings` is deliberately **not** a `Continue` decision. The module still needs a separately verified independent data-plane probe capability before it can safely classify a wave as Continue/Rollback. Caller-supplied probe flags remain claims and must not be used as production authorization. The result of this step is evidence binding, not device actuation; a TOCTOU window remains before an external side effect.

The caller supplies failure-domain labels. Their truth and freshness must come from an independent inventory/topology source; this module cannot detect a mislabeled or hidden shared dependency. A future planner should support dependency graphs, correlated-service domains, and live capacity/resilience constraints rather than only the supplied domain labels.

## State and decision semantics

| Input condition | Decision | Required coordinator behavior |
|---|---|---|
| Complete, bound, fresh observations all pass | `Continue` | Advance no further than the next planned wave |
| At least one definitive target-probe failure; every evidence binding and positive control is valid | `RollbackWave` | Roll back every applied target in this wave; verify exact baseline readback |
| Missing/extra/duplicate observations, stale evidence, wrong generation/digest, unconsumed nonce, absent independent probe, failed positive control or an inconclusive probe | `HaltAndReconcile` | Stop rollout and establish actual device state; never infer a safe denial or automatically continue |

`RollbackWave` is a request to the caller, not proof that rollback happened. The caller must persist rollback intent, perform device-specific actuation, read back the effective policy and probe it independently, using the promotion journal contract.

The previous boolean-only claim model is now confined to tests. Production code must first obtain `VerifiedWaveBindings` from the live `PromotionJournal` and its non-serializable permit. This proves the durable promotion/freshness binding under that verifier session; it does **not** authenticate the independent probe or make the later external actuation atomic. The next interface must accept a separate verified probe receipt (with its own source identity, time basis, evidence digest and positive control), then revalidate immediately before action and reconcile actual target state after any uncertain response.

## Competitive position

- **Batfish differential reachability and filter comparison** already analyses changes between network snapshots. See [Batfish differential questions](https://batfish.readthedocs.io/en/latest/notebooks/differentialQuestions.html). Luminous should not claim to replace that network model-checking capability; the complementary opportunity is combining policy prediction with constrained rollout scheduling and live evidence gates.
- **Argo Rollouts** already provides canary/blue-green progressive delivery, metric analysis, pausing, aborting and automatic rollback for application deployments. See [Argo Rollouts progressive delivery](https://argoproj.github.io/argo-rollouts/features/analysis/) and [project overview](https://argoproj.github.io/argo-rollouts/). Luminous can differentiate by making policy-generation, exact effective-policy digest, durable challenge consumption, target-level positive controls and recovery journal state explicit across network devices—not by claiming canaries or rollbacks are new.
- **RFC 9334 RATS** describes freshness and the residual race between evidence generation and the state changing afterward. See [RFC 9334, §10](https://www.rfc-editor.org/rfc/rfc9334.html). This fleet gate builds on fresh, challenge-bound observations but does not provide remote attestation or authenticated device identity.
- Commercial SD-WAN/firewall controller comparisons must be made on exact vendor, release, topology and configuration. No vendor superiority is inferred from unit tests or Linux namespace experiments.

The differentiated hypothesis is a **device-agnostic, evidence-driven change protocol**, not another dashboard or a generic canary scheduler. Prove or reject that hypothesis with matched tests.

## Benchmarks and proposed acceptance targets

These targets are proposals, not measured achievements.

| Dimension | Qualification design | Initial target |
|---|---|---|
| Determinism | Randomly permute input order for the same validated target set and constraints | Identical waves and plan digest every time |
| Blast-radius bounds | Generate 100, 1,000 and 10,000 target cases over 2–100 failure domains and varied budgets | No wave violates either batch or per-domain cap |
| Evidence gate | Missing, duplicate, unexpected, stale, replayed, wrong-target, wrong-generation, wrong-policy, non-independent and inconclusive observation variants | Zero invalid variants produce `Continue` |
| Corrective action | Definitive failure with working positive control vs the same failure with broken positive control | First yields `RollbackWave`; second yields `HaltAndReconcile` |
| Reproducibility | At least 30 randomized paired runs per exact device/firmware/topology/policy profile | Report all mismatches and inconclusive trials |
| Operational latency | Measure intent-to-independent-service-outcome, per-wave reconciliation and rollback | Report p50/p95/p99; define SLAs only after device qualification |
| Fleet safety | Fault-inject controller crash, lost response, partial apply, stale inventory, network partition and rollback failure between every transition | No automatic advance on uncertain state; halt/reconcile is explicit |
| Resilience/capacity | Model shared uplinks, power, DNS, control plane and service dependencies in addition to device/site labels | No capacity or dependency guarantees until those inputs are modeled and independently verified |

For an initial synthetic corpus, use thousands of fixed-seed scenarios and adversarial negative controls. Before publishing a false-accept rate, disclose the denominator, scenario generator, seed, model assumptions, all failures, and confidence bounds. A finite corpus cannot prove zero fleet risk.

The strongest SOTA comparison requires running Luminous and the relevant incumbent on the same policy-change corpus, topology, fault schedule and hardware. Measure unauthorized service exposure, unintended outage duration, rollback correctness, rollout time, false-halt rate, operator time, scale and recovery behavior. If a baseline supports a feature our module lacks, record that gap rather than quietly exclude it.

## Next gaps

1. Implement a separate authenticated data-plane probe capability. The new `validate_wave_permits` API binds the durable promotion part, but probe success/positive-control authenticity is intentionally still unresolved and cannot authorize `Continue`.
2. Topology-aware failure domains and service dependency constraints sourced from versioned inventory.
3. A rollout session journal with durable wave decisions and idempotent per-target command IDs, so a crash after a partial wave cannot be mistaken for a failed or completed wave.
4. Process-kill and fault-injection harnesses around partial apply, promotion commit, rollback and restart; then physical-device qualification.

Until these integrations are implemented, this module is a tested planner/decision core—not a fleet rollout product.
