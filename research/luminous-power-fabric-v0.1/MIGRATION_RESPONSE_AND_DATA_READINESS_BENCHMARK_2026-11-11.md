# Workload Migration Response and Data-Readiness Benchmark v0.1
**Date:** 11 November 2026  
**Issue:** [#404 — migration response envelope](https://github.com/Luminous-Dynamics/luminous-platform/issues/404)  
**Workload timeline:** [PR #400](https://github.com/Luminous-Dynamics/luminous-platform/pull/400)  
**Policy tournament:** [PR #402](https://github.com/Luminous-Dynamics/luminous-platform/pull/402)  
**Correlation-aware sites:** [PR #403](https://github.com/Luminous-Dynamics/luminous-platform/pull/403)  
**Measured PCC authority:** [Issue #382](https://github.com/Luminous-Dynamics/luminous-platform/issues/382)  
**Repeated-event qualification:** [Issue #396](https://github.com/Luminous-Dynamics/luminous-platform/issues/396)

## 1. Research basis

A June 2026 ACM field demonstration evaluated cross-data-centre workload migration for LLM chat inference on operational sites. It compared replication, remote access and on-demand copy with/without pre-launching. In the studied setup, pre-launched read-only patterns were the only ones meeting the strictest response class in both directions; multiple patterns met minute-level products. The paper also includes simplified scenario-based dispatch results. These results motivate a matched reproduction, not a universal promise that migration is fast or grid-service-ready.

A July 2026 University of Chicago working paper presents an eleven-parameter resource-adequacy framework combining spatial migration, temporal deferral and DVFS, using four years of day-ahead prices in five US regions and 49.4 million production inference requests. Its published commitment-depth estimates are modelled framework results, not measured capacity delivered in a field dispatch. Keep it in a model-comparator arm.

Lawrence Berkeley National Laboratory's May 2026 overview separates computational workload flexibility from facility infrastructure, storage and onsite generation. The migration benchmark covers computational movement and its evidence; it must not claim cooling/storage/generation benefits that it does not measure.

## 2. Paradigm shift: migration response envelopes

Replace a binary "migration supported" flag with a response envelope conditioned on:
- data path and data freshness;
- pre-launch state of data and runtime;
- destination capacity reservation;
- data size and network conditions;
- source/destination hardware and software configurations;
- workload correctness, accepted useful work and service-level constraints;
- data owner authority and residency/jurisdiction;
- source relief and destination load stabilization at separate electrical boundaries.

The useful operational question is not only "how long does data move?" It is "when does a valid, ready destination start preserving the same useful work, and when does source-site power measurably stabilize after the dispatch command?"

## 3. Rust implementation v0.1

The registered `migration_response` module in `tools/measurement-audit/src/migration_response.rs` evaluates event-evidenced migration trials.

### Frozen contract

Binds source/destination sites, regions, PCC boundaries, hardware configuration fingerprints, workload contract, data artifact/version, work-unit, minimum useful work, data-use authorization, residency policy, freshness policy, output quality, SLO, power-stability criterion, common clock/time origin and response-time limit.

### Trial events

Each trial records:
- command issue time;
- data-ready, runtime-ready and destination-capacity reservation times/evidence;
- destination work start and first accepted useful-work times;
- source power-reduction stabilization and destination power-increase stabilization times;
- source/destination PCC trace and measurement references;
- access mode (replicated read-only, remote read-only, on-demand copy);
- authority, residency, freshness, output-quality and SLO outcomes with evidence references;
- data bytes, clock-alignment method, source/destination clock uncertainty and evidence class.

Pre-launch state is derived from data/runtime readiness timestamps relative to the command; it is not accepted as a caller-asserted label. Every timestamp is already mapped to the common clock. Pairwise uncertainty uses the conservative sum of source and destination clock-alignment bounds.

### Separate outcomes

- Source response latency: command -> source-side power reduction stable.
- Destination stabilization latency: command -> destination-side power increase stable.
- Work preservation: accepted useful work meets the frozen work contract, output-quality contract and SLO.
- Authority/residency and data freshness remain separate hard gates.
- Service eligibility requires timely source relief, useful-work preservation, valid authority/residency/freshness and destination capacity reservation before execution.

The report groups by access mode and derived pre-launch state. Response success rate uses **all trials** as its denominator, including trials with no observed stable source response. P50/P95/P99 latencies are computed only over observed stabilization events and the sample count is disclosed. Destination-side power stabilization is reported separately.

### Explicit claim ceiling

The module does not estimate file-transfer throughput from bytes, move data, launch jobs, calculate watts/joules, integrate PCC energy or qualify a grid service. It consumes timestamped references and checks chronology/contracts. #382 remains the authority for matched PCC power/energy; #396 remains the repeated-event service qualification layer.

## 4. Benchmark matrix

Cross these factors with a preregistered paired design:
- data access: replicated read-only / remote read-only / on-demand copy;
- pre-launch: neither / data only / runtime only / both (derived from evidence timestamps);
- direction: A->B / B->A;
- destination headroom: available / constrained / unavailable;
- network: nominal / bandwidth-limited / high latency / loss / partition;
- workload: interactive / streaming / batch-deferrable;
- data size, cache warmth, replica freshness and dependency graph;
- workload and model-version changes;
- common-mode workload bursts from #403;
- local execution / temporal deferral / migration / hybrid policy arms from #402.

Record command-to-source-relief, data-ready, runtime-ready, capacity-reserved, work-start, first accepted work, destination power stabilization, completion, rollback/timeout and recovery/rebound. Keep source relief, destination added load and net system outcome separate.

## 5. Proposed targets (internal research gates, not standards or achieved results)

| Dimension | Proposed target |
|---|---:|
| Authority and residency | 100% of eligible trials have explicit authorization and residency evidence; unknown = ineligible |
| Clock alignment | 100% common-clock identity and conservative uncertainty within contract |
| Response | Report P50/P95/P99 and success fraction by access path, pre-launch state and direction |
| Useful work | >=99% retention in declared service-feasible profiles; SLO/quality remain hard gates |
| Destination capacity | 100% work starts after data, runtime and capacity are ready |
| Data freshness | Zero accepted stale-replica results in frozen positive/negative corpus |
| Denominator | 100% of trials accounted for, including no-stable-response and rollback/timeout |
| Electrical | No net-system savings claim without matched #382 source/destination PCC evidence |
| Field evidence | At least one independently instrumented physical multi-site paired trial before field response or capacity claims |

## 6. Competitive comparison and route beyond SOTA

The ACM demonstration is the nearest direct operational migration comparator. The proposed Luminous extension adds a single evidence-carrying contract spanning data readiness, runtime readiness, destination reservation, accepted output, source relief, destination load and rights/jurisdiction evidence. It also preserves the failed and indeterminate trials in each response denominator.

The UChicago working paper provides a broader model-comparator across migration, deferral and DVFS, but its percentages should not be compared directly to measured migration response. Use #402 to compare policy arms under the same workload and data contract. Use #403 to account for correlated destination load. Use #382 for physical electrical reconciliation. Use #396 only after repeated-event evidence exists.

A stronger benchmark result would demonstrate a Pareto improvement in source response, accepted useful work, SLO, data movement cost and destination externality—not just a short migration duration.

## 7. Validation status and limitations

- Eighteen synthetic Rust tests are authored but not executed. Response/work eligibility also requires an observed destination power-stabilization event; source relief alone is never sufficient for eligibility.
- Structural inspection only; no Rust build, cargo test, fmt, clippy or runtime measurement is claimed.
- No operational data-centre trace, cross-site dataset, model artifact or real data transfer was performed.
- The module carries caller-supplied evidence references and status booleans; it does not authenticate signatures or data rights.
- No live migration controller, replication service, workload placement, grid dispatch, market bid, carbon claim or service accreditation is implemented.
