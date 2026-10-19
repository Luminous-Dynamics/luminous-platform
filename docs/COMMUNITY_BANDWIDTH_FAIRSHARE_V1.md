# Community bandwidth fair-share contract v1

Status: research prototype, not a deployment qualification.

## Why this changes the problem

A community link is a scarce shared resource, not just an uplink. Provider selection and offline queues answer different questions from allocation among households sharing a backhaul. This reference model makes the allocation decision reproducible, reviewable and testable without pretending that a software allocation equals Wi-Fi airtime fairness.

## Allocation semantics

For each scheduling window, an adapter supplies a measured byte budget and demands from authenticated, opaque peer principals. Each peer declares a bounded relative weight and separate essential/ordinary requested bytes.

1. Essential demand uses the configured reserve through deterministic weighted max-min allocation.
2. If essential demand is still unmet after the reserve, it may use the rest of the window's capacity before ordinary traffic is granted.
3. When all essential requests are met, any unused reserve may be borrowed by ordinary traffic.
4. Ordinary requests then share remaining capacity by the same weighted max-min rule.
5. Demand-limited shares are saturated and the unused share is redistributed. Integer-byte leftovers use largest fractional remainder and a canonical peer-ID tie-break.

The model uses byte budgets only. It does not model per-packet timing, channel contention, retransmissions, latency-sensitive scheduling, TCP interactions or radio airtime. The caller—not this library—must authenticate peers and authorize the essential label before it calls the allocator.

## Safety and invariants

The exact-head Rust tests must establish:

- no grant exceeds its corresponding demand;
- total essential plus ordinary grants never exceeds the configured capacity;
- allocated plus explicitly unallocated bytes equal the configured capacity;
- allocation output is identical for any permutation of the same input set;
- invalid policy, duplicate IDs, invalid weight, malformed peer ID and excessive peer count fail closed;
- essential demand is allocated ahead of ordinary demand under the specified reserve rule;
- unused essential reserve is not stranded when all essential requests are met;
- overflow-resistant behavior for the maximum u64 capacity;
- deterministically resolved integer rounding.

These prove reference-model behavior only. They do not prove that real traffic receives those shares.

## Proposed qualification ladder

**Stage 0 — contract CI:** formatter, all unit tests, Clippy with warnings denied, and deterministic benchmark program compile/run. Record the exact tested commit.

**Stage 1 — trace replay:** replay a published, privacy-scrubbed demand trace. Compare against equal-share, weighted round-robin and first-come-first-served baselines. Report workload version, byte budget, peer count, weight policy, seed if any, and raw denominator.

**Stage 2 — discrete-event simulation:** add changing capacity, arrival bursts, message deadlines and essential classification. Do not infer RF behavior from this model. Test load levels at 25%, 50%, 75%, 100%, 125%, 150% and 200% of available capacity.

**Stage 3 — isolated AP testbed:** run at least 3 AP/client combinations and 5 clients, then scale to 20+ clients where hardware permits. Capture application goodput, airtime, retries, p50/p95 latency, queue drops, bytes, power and achieved task completion. Publish exact hardware/firmware revisions and raw anonymized results.

**Stage 4 — community pilot:** require explicit user notice, opt-out, local privacy review, operator authorization and a rollback plan. Test at more than one site and during actual busy/off-peak windows before general claims.

## Benchmark targets (hypotheses, not results)

| Metric | Proposed target | Qualification caveat |
|---|---:|---|
| Capacity conservation | 100% exact | Every test trace and allocation |
| Grant above demand | 0 bytes | Every peer, each traffic class |
| Input-order nondeterminism | 0 cases | Same canonical demands/policy |
| Equal-weight Jain fairness | >= 0.95 | Synthetic equal-demand cases with sufficient integer granularity |
| Essential demand served before ordinary | 100% whenever defined policy capacity permits | Only after authenticated classification in a real adapter |
| Starvation | No eligible peer remains unserved when its demand is satisfiable under the allocation contract | Radio/transport starvation needs separate measurement |
| Scale | Report latency at 10, 100, 1,000 and 4,096 peers | No hard latency SLO until the reference hardware and variance are declared |
| User task success | Improve against baselines without increasing severe priority misses | Must be measured at application layer; bytes granted are not tasks completed |
| Energy efficiency | Joules per completed task no worse than baseline; seek >= 20% improvement where confirmed | Instrumented devices and repeated trials required |
| Privacy | No direct identifiers in published traces; minimize retained measurements | Schema checks cannot prove the absence of re-identification risk |

Jain's fairness index for non-negative peer grants is the square of the sum of grants divided by the peer count times the sum of squared grants. Evaluate only the peers eligible for the same service class, and report the inclusion/exclusion rule. Do not use a high fairness score to hide a low overall completion rate.

## Evidence required before stronger claims

- Exact tested source SHA and successful current-head CI.
- Reference machine, toolchain, build profile, iteration counts and raw benchmark CSV.
- Trace provenance, consent/legal basis, anonymization review and workload denominators.
- For RF performance: precise model/revision, firmware digest, channel/band, regulatory configuration, client mix, topology, retry/airtime data and application-task results.
- Independent reproduction of the reported fairness and useful-throughput improvements.

No physical link, device configuration, ICASA authorization, provider sharing permission or user-facing service is implemented by this crate.
