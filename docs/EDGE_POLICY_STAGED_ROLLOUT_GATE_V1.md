# Luminous Edge staged rollout gate v1

**Status:** deterministic software gate with unit and scale-shape tests. It does not actuate devices or qualify a physical fleet.

## Paradigm shift: rollout progression is earned by evidence

A successful configuration call is not sufficient to justify expanding a change across a fleet. A rollout should proceed in bounded waves, diversifying failure domains, with the next wave withheld until every target in the prior wave is observed to be healthy under the exact intended policy.

The Rust module `tools/edge-policy-observation-reconciler/src/rollout_gate.rs` is a pure decision gate. The adapter-qualified entry point `authorize_next_wave_with_adapter_capabilities` validates an adapter profile for every planned target before authorizing any wave, including wave zero. Missing or duplicate profiles are rejected; if even one future-wave target is ineligible, the whole rollout is halted before it starts. It:
- Validates unique target membership, contiguous wave indices, bounded wave size, and per-failure-domain concurrency.
- Requires the exact candidate policy digest and expected generation for each target.
- Requires `Promoted` state, a fresh independent data-plane probe, and an independent positive-control path for every target in a completed wave.
- Halts on partial waves, failed/rolled-back/unknown targets, stale/future-dated evidence, digest mismatch, failed positive controls, duplicate observations, or observations from later waves appearing before an earlier wave is complete.
- Returns one explicit decision: authorize the earliest eligible wave, halt with reasons, or report completion.
- Does not issue apply commands and cannot, by itself, prove the observation producer is authentic. Callers must pass observations produced by the validated verifier and promotion journal, not manually asserted booleans.

The maximum wave target count sets an explicit blast-radius ceiling; the per-failure-domain cap avoids concentrating a wave in one failure domain when the inventory correctly labels domains. Bad or incomplete inventory can still defeat diversity, so domain correctness must be evaluated separately.

## Industry comparison and extension strategy

### Network configuration already has transactional and rollback primitives

- [NETCONF RFC 6241, Section 8.4](https://www.rfc-editor.org/rfc/rfc6241.html#section-8.4) defines confirmed commit: a configuration change is reverted if it is not confirmed before the timeout, with caveats around sessions and configuration locks. Prefer the native capability where a device implements it. A local SQLite journal is not a substitute for a device's own rollback timer.
- The [OpenConfig gNMI specification](https://github.com/openconfig/reference/blob/master/rpc/gnmi/gnmi-specification.md) defines the changes within a single Set request as a transaction. Do not assume that this provides a transaction across multiple devices or across telemetry/probe systems.
- [Batfish differential questions](https://batfish.readthedocs.io/en/latest/notebooks/differentialQuestions.html) already compare network snapshots and reachability behavior. Use that class of analysis for pre-change predictions; use the Luminous gate for progression based on validated live evidence and recovery state.

The target is not to reinvent device-level transactions. It is to **compose** device-native transactions, predicted impact, per-target evidence, failure-domain-aware waves, and explicit stop/rollback decisions into one auditable rollout protocol.

## Benchmarks: proposed gates, not achieved results

| Dimension | Method | Initial target |
|---|---|---|
| Blast radius | Generate invalid wave sizes and failure-domain concentrations | Zero invalid plans accepted |
| Progression correctness | Enumerate legal, missing, duplicated, skipped and out-of-order wave observations | Zero skipped or premature authorizations |
| Evidence integrity | Mix wrong digest/generation, stale/future timestamps, failed control paths, and rollback outcomes | Zero invalid advancement decisions |
| Large inventory | Deterministic plan validation at 100, 1,000, 10,000 and 100,000 targets | Correct plan and decision; measure runtime and peak memory outside unit tests |
| Fault injection | Fail one target, one positive control, readback, rollback, or verifier; observe coordinator decision | No later wave authorized after a failed or inconclusive prior wave |
| Fleet trials | At least 30 randomized paired rollouts per qualified hardware/firmware/topology tuple | Report all failures and inconclusive trials; compare against baseline rollout procedures |
| Change safety | Compare confirmed-commit and explicit-rollback device modes when available | Never claim rollback capability that the target does not actually implement |

Measure end-to-end impact, not only gate computation: policy command latency; effective-policy readback latency; independent service probe latency; time to detect a bad wave; time to restore the approved baseline; affected targets and failure domains; false halt rate; missed failure rate; and operator interventions. Compare against a baseline rollout controller with the same hardware, firmware, topology, test cases, and rollback settings.

For early software experiments, the test suite includes a 100,000-target plan check. That is a deterministic scale-shape test—not a claim about production throughput, memory limits, or 100,000 real devices.

## Known limits and next integration work

- The gate is currently stateless: it consumes the plan and the latest observation per target, plus the adapter-qualified wrapper's capability profiles. The caller must durably journal rollout decisions and preserve observation provenance.
- `failure_domain` labels are trusted input and must come from verified asset inventory; untrusted or stale topology labels invalidate diversity claims.
- The gate requires refreshed observations before authorizing an upcoming wave; expired proof cannot be reused.
- It does not own NETCONF confirmed commit, gNMI Set, vendor APIs, device leases, or external apply/rollback. These belong in capability-specific adapters.
- A future adapter should state its transaction modes explicitly (for example, native confirmed commit, one-request transactional Set, explicit rollback, or operator-only recovery), refuse unsupported guarantees, and preserve a device-native timeout/lock wherever available.
- Software tests do not establish physical fleet safety or superiority over a commercial network controller. Like-for-like field qualification remains required.
