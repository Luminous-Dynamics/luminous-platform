# Luminous Edge durable fleet session journal v1

**Status:** software orchestration journal and recovery actions; no network-device adapter or physical-fleet qualification is included.

**Claim ceiling:** this module durably records a canonical plan, per-wave state, per-target apply/rollback intent, stable command IDs, observed policy digests and recovery obligations. It does not execute commands, authenticate the target, attest a device, or make a fleet atomic.

## Why a second journal is needed

A deterministic plan limits scheduled blast radius, but a controller may crash after issuing an apply request to one target and before recording the result for the rest of the wave. On restart, simply rerunning the plan can duplicate a non-idempotent command, skip an unverified target, or advance while only part of the wave changed.

The session journal makes the control plane's durable intent and known observation explicit:

1. Register the canonical plan and its wave/target membership before actuation.
2. Start only the canary or the next wave whose predecessor reached the durable `continued` state.
3. Commit a per-target `apply_intent` and deterministic command ID before asking the external adapter to apply a policy.
4. Record `applied` only when live effective-policy readback exactly equals the planned candidate digest.
5. Recompute the wave result with `evaluate_wave`; only a complete, bound, fresh pass moves the wave to `verified`.
6. Record each target promoted only after its PromotionJournal has committed promotion and live readback still matches the candidate. Advance the wave only after every target is durably recorded as promoted.
7. For a definitive failure with intact positive controls, commit rollback intent for all targets in the wave and return stable rollback command IDs. Mark each target rolled back only after exact baseline readback. A completed wave rollback halts the rollout; the coordinator does not automatically proceed to the next wave.

## Durable state

Session states: `planned`, `running`, `halted`, `completed`.

Wave states: `planned`, `applying`, `applied`, `verified`, `rollback_required`, `rolling_back`, `continued`, `rolled_back`, `halted`.

Per-target states: `planned`, `apply_intent`, `applied`, `promoted`, `rollback_intent`, `rolled_back`.

| Crash or failure point | Journal state | Recovery obligation |
|---|---|---|
| Before plan registration commit | No session | No command is authorized by this journal |
| After plan registration, before wave start | `planned` | Start the canary explicitly |
| After `apply_intent`, before/after device response | `apply_intent` plus stable command ID | Inspect live effective policy before retry; use the same idempotency key if adapter semantics permit |
| After device change, before readback record | `apply_intent` | Read back effective policy; do not infer success from command response |
| After readback, before journaling applied | `apply_intent` | Re-read and commit `applied` only when exact candidate digest matches |
| After all targets applied, before wave evaluation | `applied` | Obtain a fresh independent probe for every target |
| After fresh wave pass, during per-target promotion | `verified`, mixed per-target `applied`/`promoted` | Reconcile every target with its PromotionJournal and actual effective policy before resuming or rolling back |
| After rollback intent | `rolling_back`, per-target `rollback_intent` | Reconcile actual policy, enforce baseline and verify exact digest; stable rollback IDs make retries identifiable |
| Any stale, incomplete or mismatched wave evidence | `halted` | Stop rollout and reconcile. No automatic next wave |

An idempotency identifier only helps when the external adapter or device honours the specified idempotency semantics. If it does not, the coordinator must inspect actual device state before retrying. The database cannot make a remote firewall command part of its SQLite transaction.

## API

Module: `tools/edge-policy-observation-reconciler/src/fleet_session.rs`.

- `FleetSessionJournal::register_plan` validates the canonical schedule and persists target/wave membership.
- `begin_wave` prevents skipping the canary or advancing before the prior wave is continued.
- `record_apply_intent` generates and persists a deterministic command ID before external apply; replaying the same request returns the same ID.
- `record_applied` compares the actual readback digest to the candidate.
- `evaluate_and_record_wave` recomputes the wave decision from observations and persists `verified`, `rollback_required`, or `halted`.
- `record_target_promoted` and `complete_wave` ensure a partial promotion cannot silently count as a completed wave.
- `request_wave_rollback` commits stable rollback command IDs, including across partial rollback and lost responses; `record_target_rolled_back` requires the exact baseline digest and tolerates repeated identical completion acknowledgements.
- `list_recoverable_targets` exposes recovery obligations without automatically retrying any command.

The caller must supply observation records produced by the already-validated durable freshness/conformance path. The API can check their target/generation/digest/freshness fields, but cannot prove caller-provided booleans originated from authenticated evidence. Similarly, a caller should record a target as promoted only after its PromotionJournal has durably confirmed promotion.

## Industry comparison

- **Batfish** performs differential configuration and reachability analysis between network snapshots ([docs](https://batfish.readthedocs.io/en/latest/notebooks/differentialQuestions.html)). This journal does not reproduce that analysis; it provides persistent execution/recovery state around a separately validated plan.
- **Argo Rollouts** already supports canary and blue-green deployment, metric analysis, pauses, aborts and rollbacks ([analysis](https://argoproj.github.io/argo-rollouts/features/analysis/)). The intended extension is to treat network policy generation, effective-policy digest, durable nonce consumption and device readback as first-class rollout state across devices.
- **RFC 9334** describes freshness and notes that state may change immediately after evidence generation ([§10](https://www.rfc-editor.org/rfc/rfc9334.html)). The coordinator narrows this race with repeated readbacks and wave boundaries but does not eliminate it or provide remote attestation.

## Acceptance benchmarks (proposed, not yet achieved)

- Crash or kill the controller after every durable transition and at every adapter-call boundary; no incomplete wave may be reported continued or completed.
- Run at least 100,000 generated state-machine traces and adversarial transition attempts with zero illegal transitions accepted.
- Replay the same apply/rollback request after controller restart; the command ID must remain identical for the same plan, target, generation and action.
- Exercise partial application of waves of 1, 10, 100 and 1,000 targets; report time to detect uncertainty, time to stable state and per-target reconciliation rate.
- Inject stale inventory, lost responses, partial network partitions, positive-control failures, wrong-generation receipts and candidate/baseline digest mismatches; all must halt or roll back according to registered semantics.
- Run at least 30 randomized paired trials for every physical device/firmware/topology/policy tuple. Report every failed/inconclusive trial and p50/p95/p99 from durable intent to independently observed service outcome.
- Compare with the incumbent controller on the identical topology, policy corpus, failure schedule and hardware. Publish correctness, unintended outage, unauthorized reachability, recovery duration, rollout time, operator effort and maximum correlated blast radius. No superiority claim should be made from software tests alone.

## Remaining boundary

The session, replay, per-target promotion and real device stores are separate durable systems. V1 detects and exposes the reconciliation obligations; it does not promise cross-database or controller/device atomicity. Production requires a trusted inventory source, single-writer lease or equivalent, a device adapter with explicit idempotency/version semantics, and hardware fault-injection qualification.
