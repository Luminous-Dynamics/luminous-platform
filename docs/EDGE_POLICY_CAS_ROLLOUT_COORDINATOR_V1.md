# Luminous Edge CAS-guarded rollout coordinator v1

**Status:** draft adapter contract and software implementation. The test adapter is in-memory and does not qualify any commercial AP, firewall, SD-WAN controller or multi-device fleet.

## Paradigm shift: desired state is not observed state

A deploy command is only a request. The coordinator uses the following protocol:

1. Read the current effective policy and device generation/revision.
2. Verify the exact approved baseline bytes match the effective-policy digest.
3. Commit the promotion journal's `Prepared` and `ApplyIntent` states.
4. Ask the device adapter to compare-and-apply the candidate against the exact previously read generation, revision token and digest.
5. Read back the effective policy, rejecting any target, generation or digest mismatch.
6. Issue a fresh challenge binding the target, policy generation, baseline digest, candidate digest and exact corpus bytes.
7. Run an independent challenged probe and bind its exact receipt bytes into a freshness envelope.
8. Verify conformance and freshness, durably consume the nonce, record evidence, re-read the policy, then promote.
9. If a failure occurs after `ApplyIntent`, persist rollback intent first. Roll back only if observed policy is exactly the candidate or exact approved baseline. If another/unknown digest appears, leave durable recovery state and refuse to overwrite it.

The coordinator does not treat controller acknowledgement, success-shaped JSON, or a fresh timestamp as proof by itself. Every stage has a separate observation or durable transition.

## Adapter contract

Module: `tools/edge-policy-observation-reconciler/src/rollout_coordinator.rs`.

`PolicyDeviceAdapter` implementations must provide:

- A live effective-policy snapshot with target token, monotonically increasing generation, opaque revision token and stable SHA-256 of canonical policy bytes.
- An exclusive per-target lease held across the whole apply → probe → promote/rollback lifecycle, with renewals at critical stage boundaries. The lease must exclude all policy writers that could invalidate the final readback.
- Effective-policy digest readback using exactly the same canonicalization contract as baseline and candidate policy artifacts.

`IndependentPolicyProbe` must take the verifier's exact challenge bytes and return an observation receipt and run timestamps from a genuinely independent path. It must not infer successful traffic from the device command response. The coordinator creates the freshness envelope from these returned artifacts; the envelope is byte binding, not producer authentication.

Adapters that cannot guarantee an exclusive per-target lease, atomic generation/revision compare-and-swap, or stable effective-policy digests are refused before any writes. The coordinator holds the lease guard through final readback and journal promotion; an adapter must not claim exclusivity if other controller or operator paths can bypass the lock. They may still be supported by a future explicit manual/serialized mode, but that mode must not claim race-free automatic promotion.

## Recovery behavior

A failure after durable `ApplyIntent` is ambiguous: the request may have failed before actuation, after actuation, or after the device changed state but before the client received its acknowledgement. The coordinator therefore reads the actual state before it decides whether rollback is safe.

- Exact baseline digest observed: record rollback only after the journal's `RollbackIntent` has been committed.
- Exact candidate digest observed: compare-and-apply the baseline against the latest snapshot, then read back and require the exact baseline digest.
- Any other digest, wrong target, invalid snapshot or unreadable state: retain `RollbackIntent` and require manual/adapter-specific recovery. Do not overwrite an unknown configuration.

This is intentionally fail-closed. Recovery liveness may suffer if the adapter loses access to readback; that is better than fabricating a rollback success or clobbering an unrecognized policy.

## Security boundaries and non-goals

- Capability flags are assertions from trusted adapter code. They are not remotely attested device properties.
- Device-side generation/revision CAS is the synchronization boundary. A client-only read-then-write sequence does not satisfy this contract.
- The coordinator uses OS CSPRNG bytes from `/dev/urandom` for the challenge nonce; randomness failure is fatal.
- Challenge freshness and exact receipt-byte hashes do not authenticate the probe producer, secure the device adapter, or establish trusted wall-clock time.
- The policy digest must be computed with a defined, stable canonicalization shared by artifact and live readback. Hashing arbitrary vendor JSON without canonicalization is not sufficient.
- The replay ledger and promotion journal remain separate SQLite files. A crash after nonce consumption but before journal verification causes the nonce to be spent; recovery requires a newly issued challenge. It does not claim cross-database or device/database atomicity.
- A second policy readback narrows—but does not independently eliminate—the time-of-check/time-of-use interval before journal promotion. An implementation needs a held device-side revision lease or atomic conditional commit semantics covering this boundary.
- A production device adapter and independent probe are not included. The in-memory fake adapter validates contracts only.

## Benchmark plan and initial targets

These are proposed qualification gates, not achieved performance results.

| Benchmark | Procedure | Initial target |
|---|---|---|
| Capability refusal | Try no-CAS, unstable-digest, wrong-target and malformed revision adapters | Zero writes before refusing an unsafe adapter |
| Stale-write race | Two writers use the same snapshot/generation; interleave writes deterministically and probabilistically while the target lease is held | Exactly one conditional update succeeds; non-holder writes are blocked or rejected; loser re-reads and reconciles |
| Lease failure | Expire or revoke the target lease before apply, before probe, and before promotion | No stage proceeds without successful renewal; rollback remains durable if lease reacquisition fails |
| Apply acknowledgement ambiguity | Simulate timeout before mutation, after mutation and after readback loss | Never infer state from acknowledgement; recover by effective-state readback |
| Evidence correctness | Mutate challenge target/generation, baseline/candidate digest, receipt hash, timestamps and corpus | Zero invalid promotions, with positive controls passing |
| Rollback | Inject probe failure, journal write failure and apply timeout; test candidate, baseline and third-party policy readback | No false rollback successes; never overwrite unknown digest |
| Crash recovery | Kill process after each journal commit and after each external adapter call; reopen both databases | No stale evidence promoted; every ambiguous boundary yields explicit recovery |
| Repeatability | At least 30 randomized paired trials for each exact device, firmware and adapter version | Report all failures/inconclusive runs; publish p50/p95/p99 and raw counts |
| Replay load | At least 100,000 unique challenges mixed with duplicate/stale/replayed attempts | Zero invalid accepts; measure durability cost and latency distribution |
| Scale | 100, 1,000, 10,000, 100,000 policies/targets where adapter permits | Measure p50/p95/p99 change latency, CPU, memory, journal size, lock waits and intervention rate |

Report apply command time, readback time, independent probe time, durable verification time and rollback recovery separately. End-to-end latency must include all these components. Never substitute the API acknowledgement time for successful observed service restoration.

## Industry comparison and differentiation hypothesis

Established tools already solve meaningful pieces of this space:

| Baseline | Published capability | Luminous extension to test |
|---|---|---|
| [Batfish differential questions](https://batfish.readthedocs.io/en/latest/notebooks/differentialQuestions.html) | Compare network snapshots, filter behavior and flows that change from permitted to denied or vice versa. | Attach predicted diffs to the exact installed policy digest and live independent probe results. |
| [Ansible network validation](https://docs.ansible.com/projects/ansible/latest/network/user_guide/validate.html) | Validate device/file state against explicit criteria and identify drift for remediation. | Make criteria, evidence freshness, replay state, rollout lease, and promotion/rollback part of one durable operation record. |
| [Junos confirmed commit](https://www.juniper.net/documentation/us/en/software/junos/cli/topics/topic-map/junos-configuration-commit.html) | Temporarily activate a candidate and automatically restore the previous configuration unless the change is confirmed. | Confirm only after exact effective-state readback and challenge-bound independent service probes; preserve explicit crash recovery state. |
| [PAN-OS session rematching](https://docs.paloaltonetworks.com/ngfw/networking/session-settings-and-timeouts/configure-session-settings) | Can rematch existing sessions against a changed policy; behavior depends on configuration. | Register existing-session, new-session, explicit-revocation and restored-policy test cases for each supported adapter. |
| [RFC 9334 (RATS)](https://www.rfc-editor.org/rfc/rfc9334.html) | Defines an architecture for appraising evidence and reasoning about freshness, while noting the unavoidable race after a result is generated. | Combine freshness with single-use durable challenges, target/generation binding, and a live policy readback immediately before promotion. |
| [RFC 6349](https://www.rfc-editor.org/rfc/rfc6349.html) | Defines a repeatable framework for end-to-end TCP throughput measurement after lower-layer integrity and baseline parameters are established. | Measure correctness and rollback separately from throughput, and use a matched end-to-end protocol for device performance comparisons. |

The differentiation hypothesis is not that any one mechanism is new. It is a portable, auditable protocol that joins differential intent, conditional actuation under an exclusive per-target lease, exact effective state, independent fresh evidence, durable replay consumption, promotion, and safe recovery. Automatic rollout must refuse adapters that cannot guarantee the required lease/CAS/readback contract.

Competitive superiority must be tested against comparable vendor workflows on identical topologies, policy scenarios, failure injections, firmware, and service probes. Measure high-risk false permits, false blocks, service interruption, time to restore, stale/replayed evidence accepted, scale, and operator toil. Until physical-device testing exists, the present implementation is a typed contract with a fake adapter—not an SOTA or vendor-superiority result.
