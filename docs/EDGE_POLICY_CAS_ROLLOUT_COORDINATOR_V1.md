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

Established tooling already covers parts of this workflow: differential reachability/configuration analysis, device-specific policy deployment and session rematching, nonce-based freshness, and network-performance measurement. Luminous should not claim those concepts as novel.

The differentiation hypothesis is integration: a portable coordinator with a durable change lifecycle that binds target and generation, exact effective policy, fresh independent probe evidence, replay consumption, promotion and recovery—while refusing automatic operation if its adapter cannot provide the required CAS/readback guarantees.

Competitive superiority must be tested against a comparable vendor workflow on the same topology, policy scenarios, failure injection and firmware, measuring false permits, false blocks, change impact, recovery time, scale, and operator toil. Until physical-device testing exists, this remains a software contract with a fake adapter, not a SOTA or vendor superiority claim.
