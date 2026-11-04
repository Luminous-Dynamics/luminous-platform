# Connectivity multipath policy core

A deterministic, dependency-free Rust model for comparing single-path and multipath session schedules over **broker-admitted** path observations.

This crate is a scheduling model, not a transport implementation.

## Boundary

The caller supplies path observations and an application task request. The model does not:
- discover or authenticate network interfaces;
- decide whether an account, provider or device is authorized;
- override the upstream broker's path eligibility;
- create MPTCP/QUIC subflows or manage transport addresses;
- install routes, modify firewalls, switch SIM/eSIM profiles or transmit radio;
- inspect packets, URLs, message content or browsing history;
- claim that a predicted schedule was actually sent by a transport.

The current inputs distinguish broker eligibility, authorization validity, telemetry freshness, trust floor, MTU, loss, RTT, jitter, capacity, data remaining, cost/energy rates, and declared shared-risk group.

## Units

All arithmetic is integer-only:
- payload/data allowance/reorder exposure: bytes;
- capacity: bits/second;
- RTT and jitter: microseconds;
- loss: parts per million;
- cost: caller-declared integer microcurrency per MiB;
- energy: microwatt-hours per MiB.

Every path carries an explicit uppercase three-letter cost currency code, and the request declares the expected code. Mismatches are excluded before ranking; malformed request codes are rejected. The caller remains responsible for using a valid ISO-4217 code and supplying current exchange-rate provenance when converting quotes. Never compare two rates in different currencies. The upstream broker enforces eligibility; this core enforces the task's stated numeric ceilings using the supplied units.

## Strategies

The Rust core currently implements four transparent model strategies:
- `BestRtt`: minimum RTT among eligible single paths (not a claim about the exact MPTCP default scheduler).
- `LowestCost`: minimum estimated cost among eligible single paths.
- `AggregateCapacity`: proportional byte split by effective capacity, subject to per-path allowance and total cost/energy/deadline/reorder limits; otherwise falls back to one path.
- `IntentAware`: uses the aggregate candidate only when its estimated completion-time gain clears the declared threshold and all hard constraints; otherwise follows the workload's single-path fallback.

The **benchmark comparator set** is broader than the Rust model: include Round Robin, Earliest Completion First (ECF), BLEST (reordering-aware), redundant-transmission control, and the exact Linux in-tree default scheduler. ECF/BLEST or redundant variants may require a separately pinned research patch/module; record that provenance and never call an out-of-tree implementation an in-tree Linux baseline. Identify the Linux default scheduler from the exact kernel source instead of assuming its label. Pin congestion control separately because scheduling performance depends on the congestion controller.

This first layer is a transparent byte-allocation model. A second `trace::simulate` layer schedules fixed-size segments over per-path serialization queues, applies seeded synthetic loss keyed by declared failure domain, models first-arrival stream reassembly, and reports p50/p95/p99 segment arrival, reorder-buffer peak/byte-time, redundant-byte overhead and useful bytes delivered. Its deterministic common-risk-group loss control is deliberately strong correlation, not a calibrated channel model.

Neither layer implements TCP/QUIC congestion-control feedback, ACK clocks, retransmission/reinjection, pacing, cwnd, socket buffers, real MPTCP/QUIC behavior or live path discovery. The trace model is bounded to 1,000,000 segments and 64 paths. It is suitable for deterministic policy comparisons and falsifiable negative controls, not as a substitute for namespace/VM or kernel measurements.

## Model equations

Effective capacity is a simple loss-adjusted estimate:

`effective_bps = capacity_bps × (1 - loss_ppm / 1,000,000)`

Estimated completion for a path allocation is:

`finish_us = rtt_us + jitter_us + ceil(bytes × 8 × 1,000,000 / effective_bps)`

These are planning estimates. They do not model time-varying congestion or packet-level reordering.

## Capacity domains versus failure domains

Every path declares both:
- `shared_risk_group`: the operator-declared common-cause failure identity used by the current v1 model;
- `capacity_domain_ids`: a non-empty, duplicate-free set of every known hard throughput bottleneck consumed by that path (maximum 32 IDs).

These are not interchangeable. Two paths may fail independently but still share one downstream backhaul. A path with `access-a + shared-backhaul` overlaps a path with `access-b + shared-backhaul` even though the access links differ. Unknown, blank, duplicate or oversized capacity-dependency sets are excluded with explicit reasons.

For the default two-path case, the scheduler examines all pairs and selects the highest-effective-capacity pair whose capacity-domain sets are fully disjoint. A regression fixture compares this with an independently enumerated exhaustive pair oracle. For more than two paths it extends that pair deterministically, admitting only paths disjoint from the union already selected; this extension is a conservative heuristic, not a globally optimal set-packing solver. Redundant transmission also requires distinct failure groups and disjoint capacity-domain sets. The conservative set rule may underuse real capacity when some bottlenecks are shared; a future resource-incidence model can allow coordinated use under a shared aggregate ceiling, but that requires a separately qualified allocator.



Capacity-domain IDs must be globally canonical across inventory sources: two records describing the same physical bottleneck must resolve to the same ID, even when the access links or provider inventory records differ. Local aliases must be resolved by an independently verified inventory layer before they reach this scheduler. An unresolved alias or incomplete mapping is unknown evidence, not proof of independence.

A dependency set is not trusted merely because it is syntactically valid. Every path also supplies a `CapacityDomainEvidence` record with a stable source ID, source-inventory SHA-256, exact topology SHA-256, a completeness flag, an operator-declared confidence value in ppm, and observed/valid-until Unix-millisecond timestamps. The request records a non-zero confidence floor; the first benchmark profile uses 950,000 ppm as a proposed baseline, not a calibrated universal threshold. Confidence is an evidence-grade assertion from the source, not a probability computed by this model. The scheduling request binds its expected topology digest and exact decision time. Empty, incomplete, stale, future-dated, malformed-digest, mismatched-topology, oversized or duplicate domain evidence is excluded before candidate ranking. Domain IDs are bounded to 32 entries per path and 128 UTF-8 bytes per ID. The allocation and trace reports preserve the per-path evidence record, sorted union of capacity-domain IDs, and topology/decision timestamp for replay.

This model validates the evidence envelope and digest bindings; it does **not** fetch or cryptographically authenticate the source inventory, independently validate the source's confidence assertion, prove that the operator's topology declaration is truthful, or discover hidden shared resources. A separate verifier must retrieve/verify the inventory/attestation, and the field campaign must challenge the declaration with observed common-bottleneck negative controls.

## Tenant fairness

`fairness::allocate_tenant_weighted_max_min` allocates a single declared bottleneck in two stages: first among caller-supplied tenant principal IDs, then among flows inside each tenant. Opening more flows under the same tenant therefore does not create additional aggregate tenant entitlement. Tenant identity authentication/binding remains the caller's responsibility; this allocator does not authenticate principals or inspect packet content.

Flow fairness is reported **within each tenant's allocated budget**, not across flows belonging to different tenants. The report includes each tenant's backlogged-flow count and Jain index where at least two flows are backlogged, plus the worst measurable tenant result. The tenant-level index is reported separately. This preserves the hierarchy: equal entitlement between tenants first, then weighted flow sharing inside each tenant. A tenant with one flow must not make the cross-tenant flow score look unfair merely because another tenant has more concurrent flows.

The allocator uses integer bits-per-second, positive tenant/flow weights, hard minimum service floors, deterministic largest-remainder tie-breaking, a 512-flow bound and overflow rejection. It explicitly reports unused capacity when requested demand is exhausted and fails closed when the sum of minimum floors cannot fit the bottleneck. Jain fairness is reported only among backlogged tenants/flows; demand-limited entities are not marked unfair for failing to consume unrequested capacity.

It is a resource budget reference, not a replacement for fq_codel, CAKE or a kernel RAN scheduler. The benchmark profile compares those real queueing baselines separately.

## Capacity dependency benchmark cases

The scheduling profile includes dedicated resource-dependency cases:
- disjoint end-to-end paths;
- different access links sharing a secondary backhaul;
- a three-path overlap chain where the highest-capacity middle path overlaps with both otherwise independent paths;
- empty, blank, duplicate and oversized dependency sets;
- incomplete, expired, future-dated, below-confidence and mismatched-topology evidence;
- deterministic output under input permutation, with the two-path result checked against an exhaustive pair oracle.

These are contract fixtures. The operator-declared evidence grade is not calibrated probability, and the inventory digest is not a cryptographic attestation unless an independent verifier checks the actual source artifact.

## Hard constraints and bounded behavior

The model excludes paths that are broker-ineligible, unauthorized, stale, below the trust/MTU floor, have zero or invalid capacity/loss, exhausted data allowance, or missing declared failure-domain identity.

It returns no route when no candidate fits. It rejects duplicate path IDs, empty IDs, invalid loss bounds, zero payloads, zero path count, invalid improvement thresholds and more than 64 candidate paths. Cost/energy conversions and aggregation use checked integer bounds; overflow rejects the candidate rather than saturating into a false fit.

## Tests

The byte-allocation model and packet-trace simulator have separate tests. Trace tests cover MinRTT, Round Robin, redundancy only across distinct declared risk domains, deterministic replay, broker-ineligible path exclusion, per-path cost rounding, aggregate cost ceilings, common-domain loss correlation, separate synthetic loss streams, out-of-order application release, lost-gap timeout accounting, existing-buffer-aware reorder prediction and input bounds.

Unit tests cover:
- two eligible paths used for multipath;
- broker-ineligible paths never receive bytes;
- cost and energy caps;
- receiver reorder-buffer refusal;
- stale and insufficient-trust path exclusion;
- deterministic decisions under candidate-order permutation;
- duplicate path identity;
- single-path fallback when multipath is disabled;
- true minimum-RTT baseline versus fastest completion;
- fail-closed aggregate cost overflow.

Run:

```sh
cargo test --manifest-path tools/connectivity-multipath-policy-core/Cargo.toml --all-targets
cargo clippy --manifest-path tools/connectivity-multipath-policy-core/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path tools/connectivity-multipath-policy-core/Cargo.toml -- --check
```

## MPTCP coexistence evidence evaluator

`coexistence::evaluate_best_path_improvement` and `coexistence::evaluate_shared_bottleneck_do_no_harm` evaluate paired measured summaries. They do not implement MPTCP, simulate congestion control or actuate a kernel.

The best-path gate requires separate identities for path-A TCP, path-B TCP and MPTCP trials, verifies that their kernel/topology/qdisc/workload/impairment/seed/duration/controller identities match, and evaluates MPTCP application goodput against the better single-path TCP reference.

The shared-bottleneck gate compares a TCP+TCP control with an MPTCP+TCP treatment at the same declared bottleneck rate and experiment identity. It reports:
- MPTCP goodput relative to mean per-flow goodput in the TCP+TCP control;
- companion TCP retention relative to that same control reference;
- total treatment goodput relative to total control goodput.

The default proposed thresholds are MPTCP share <= 1.10x the single-TCP reference and companion TCP retention >= 0.90x. These are engineering targets, not results or universal constants. The profile and issue #222 define the wider experiment. Mismatched identities are inconclusive, unsupported kernel/controller arms are unsupported, a single negotiated subflow fails the multipath gate, missing counters are inconclusive, and unverified cleanup fails qualification.

The evaluator uses checked integer ratios in parts-per-million; confidence intervals and uncertainty analysis remain in the external benchmark layer. It does not infer physical capacity independence from different IP addresses or path names.

Every verdict has scope `single_paired_trial`. A paired pass is not programme-level statistical qualification: the external analysis must retain all repetitions and failures and compute the declared confidence intervals before promoting the programme. The exact kernel build digest binds in-tree TCP/MPTCP congestion-control and scheduler code; every out-of-tree TCP or MPTCP controller and scheduler must provide its own artifact SHA-256 in the paired identity.

## Multipath QUIC draft-21 evidence verifier

`quic_multipath::evaluate_multipath_quic_trial` consumes two-ended adapter evidence for one exact Multipath QUIC implementation. It does not implement QUIC, open sockets, create paths, change routes, schedule packets or actuate a network.

The first pinned comparator is `qoke/mp-quic-go` source commit `084c13ea3834fbe58cedb63db9208951d714f839`, module version `v0.3.0`, based on quic-go `v0.63.0` and Go `1.26.0`. It documents Multipath QUIC `draft-ietf-quic-multipath-21`, an opt-in extension, a default concurrent path count of three and a maximum of 64. It documents uncoupled Reno per path as the default congestion-control mode and coupled OLIA as an optional mode. Those are separate experiment treatments; never pool their performance results.

The verifier requires:
- an exact protocol/source/build/controller/scheduler identity at both endpoints;
- multipath negotiation, two active validated paths and matching path-ID sets at both endpoints;
- path lifecycle events with non-reused IDs, a monotonic MAX_PATH_ID high-water and a concurrent-path ceiling checked independently from path-ID history;
- for secondary paths, a validation probe of at least 1200 bytes and a response on the challenge path;
- non-zero connection IDs, separate path packet-number spaces, path-specific PMTU and observed ECN validation state;
- ACK-path samples labelled by endpoint, with data-path and ACK-return path recorded separately; cross-path ACK RTT samples are explicitly composite;
- stream byte accounting that distinguishes network receipt from useful in-order application release, and a hard receiver reorder-buffer bound.

The adapter report distinguishes **validated path IDs** from **data-path IDs actually observed** and ACK-return path IDs. Negotiation plus two validated paths does not prove both paths carried application data.

The adapter verdict is scoped to `single_paired_trial`. A paired pass is not programme-level statistical qualification. The profile contains 16 scenarios covering fallback, negotiation, independent and shared bottlenecks, cross-path ACKs, path churn, path validation, controller/scheduler comparisons and unsupported revision/observability cases.

The mp-quic-go compliance and picoquic interoperability results are upstream's own reported evidence, not independent LNF qualification. Its published interop runner used a Docker bridge/router/netem stand-in because ns-3 forwarding failed in the test environment, and it explicitly did not measure goodput or cross traffic. Therefore LNF still needs matched namespace performance tests, especially shared-bottleneck TCP coexistence, before any performance claim.

The focused Rust evidence checks can be run with:

```sh
cargo test --manifest-path tools/connectivity-multipath-policy-core/Cargo.toml quic_multipath::tests --all-targets
cargo clippy --manifest-path tools/connectivity-multipath-policy-core/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path tools/connectivity-multipath-policy-core/Cargo.toml -- --check
```

The QUIC wire adapter must remain a thin boundary. LNF owns path/resource eligibility, intent, tenant fairness, cross-path task scheduling and evidence evaluation; the transport owns QUIC packet encoding, path validation, CID/packet-number spaces, recovery and congestion control.

## Required next steps before a performance claim

1. Compare model output with the exact Linux kernel MPTCP path-manager and scheduler configuration.
2. Add a two-ended Multipath QUIC adapter only when the pinned endpoint implementation supports the IESG-approved protocol revision.
3. Add deterministic packet/ACK/reordering traces and a bounded receiver reassembly model.
4. Calibrate simulated path estimates against namespace/VM and then hardware observations.
5. Publish the full Pareto frontier (useful-task completion, p95/p99 latency, cost, energy, reorder bytes and fairness), preserving all failures.

A passing unit suite proves code contracts only. It does not prove live multipath behavior, end-to-end improvement, production readiness or SOTA superiority.
