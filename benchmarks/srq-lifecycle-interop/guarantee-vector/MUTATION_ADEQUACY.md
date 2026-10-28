# SRQ mutation adequacy: prove that tests notice unsafe changes

## Why this is a separate assurance layer

A passing example matrix shows that the current evaluator agrees with selected expectations. It does not show that the test suite would notice if a future edit accidentally removed an important check. Mutation testing asks the adversarial question directly: deliberately weaken the code, then require the correct regression test to fail.

The initial SRQ mutation set has five deliberately unsafe edits:

1. Remove authorization's exclusive expiry check.
2. Accept an anchor checkpoint observed at a different trusted epoch.
3. Ignore a delivery capability's sink-identity binding.
4. Skip receipt-to-effect digest binding before the no-redispatch decision.
5. Convert a same-key retry into a fresh dispatch.

Each mutation has a named target test. A mutant is counted as **killed** only when that target test fails. A compile error, missing target, timeout, unrelated test failure, or broken mutation anchor is an invalid result—not a killed mutant. The harness copies the crate to a temporary directory, changes only that copy, and keeps the checked-in source unchanged.

The executable protocol is mutation-protocol-v1.json; mutation_qualification.py --check-only performs a source-anchor preflight without invoking Rust. CI runs the full execution mode and retains the JSON result with the other exact-head reports.

## Initial acceptance criteria

- Five declared mutations are applied exactly once each.
- All five named tests fail under their corresponding mutation.
- Zero mutants survive.
- Zero mutants are counted as killed due solely to compile errors.
- The report hashes the unmodified baseline source and the protocol, and records the commit SHA in CI.
- The qualification ceiling remains BLOCKED / none. Passing these five mutants is a test-sensitivity result, not a production-security proof.

## How this compares to established practice

Mutation testing is established practice: tools such as [Stryker](https://stryker-mutator.io/docs/) and [PIT](https://pitest.org/) deliberately modify code and measure whether tests detect those changes. SRQ's first increment is intentionally small and handwritten so the mutation set is reviewable, deterministic, and security-relevant. It is not a replacement for a mature general-purpose mutator.

For real distributed-system assurance, the next layer should follow the complementary model used by [Jepsen](https://github.com/jepsen-io/jepsen): run operations against real binaries, inject crashes/partitions/clock disturbances, record histories, and check them against an explicit consistency model. SRQ's in-memory tests are useful preflight, but cannot substitute for a real database/sink and independently verified receipt.

The policy baseline also needs to stay aligned with:
- [AWS's idempotent API guidance](https://aws.amazon.com/builders-library/making-retries-safe-with-idempotent-APIs/): bind a client key to original request intent and reject changed parameters on reuse.
- [RFC 9334 §10 and §12.3](https://www.rfc-editor.org/rfc/rfc9334.html): freshness is an architectural choice; epoch IDs can be delayed, reordered, dropped, or replayed.
- [The Update Framework specification](https://github.com/theupdateframework/specification): explicitly detect rollback and freeze attacks, and persist accepted metadata.

These references are design baselines, not claims that SRQ currently matches their assurance level.

## Roadmap toward meaningful SOTA comparison

| Layer | Experiment | Publish |
|---|---|---|
| Test sensitivity | Expand five handwritten faults to 20–40 security-relevant mutants, with independently reviewed expectations | Valid mutation score; killed/survived/invalid counts |
| State-space | Add generated boundary values, multiple tenants, same-key/different-intent, and transition interleavings | False-release count, minimized counterexamples, coverage of state transitions |
| Crash recovery | Inject power-loss/crash at every storage boundary against a real adapter | Duplicate logical effects, receipt loss, liveness loss, recovery time |
| Distributed concurrency | Concurrent key races, stale lease holders, partitions, delayed/reordered epoch updates, anchor failover | Histories and consistency-checker verdicts |
| Performance | Compare equivalent durability guarantees across backends and concurrency levels | p50/p95/p99, throughput, write amplification, disk/network costs, false blocks |
| Production claims | Bind key/receipt/atomicity/fencing proofs to a real target adapter and versioned contract | Contract-specific claim ceiling; independent reproducibility |

A single throughput number is not a SOTA comparison. Compare like-for-like durability and failure guarantees, with every candidate running identical traces and workloads. Safety and availability must be reported separately: a system that never duplicates because it permanently blocks every request is safe but not operationally equivalent to one that safely recovers.

## Limitations

The mutation set is handpicked and finite. Its score is meaningful only if reviewers challenge the mutation selection and ensure each mutation represents a realistic defect. It can itself miss mistakes in the test oracle or a production adapter. Maintain explicit negative controls, keep the oracle separate from the implementation where possible, and add real-system history checking before raising the qualification ceiling.
