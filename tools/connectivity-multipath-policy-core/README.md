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
- `shared_risk_group`: links expected to experience a common-cause failure;
- `shared_capacity_group`: links consuming the same declared hard throughput bottleneck.

These are not interchangeable. Two paths may fail independently but still share one backhaul. The current v1 aggregate and redundant controls conservatively select at most one path per declared capacity group. Redundant transmission requires both a distinct failure group and a distinct capacity group. Unknown/empty capacity groups are excluded. This can underuse real capacity within one group; a future group-level rate ceiling requires a separately qualified model.

## Tenant fairness

`fairness::allocate_tenant_weighted_max_min` allocates a single declared bottleneck in two stages: first among caller-supplied tenant principal IDs, then among flows inside each tenant. Opening more flows under the same tenant therefore does not create additional aggregate tenant entitlement. Tenant identity authentication/binding remains the caller's responsibility; this allocator does not authenticate principals or inspect packet content.

The allocator uses integer bits-per-second, positive tenant/flow weights, hard minimum service floors, deterministic largest-remainder tie-breaking, a 512-flow bound and overflow rejection. It explicitly reports unused capacity when requested demand is exhausted and fails closed when the sum of minimum floors cannot fit the bottleneck. Jain fairness is reported only among backlogged tenants/flows; demand-limited entities are not marked unfair for failing to consume unrequested capacity.

It is a resource budget reference, not a replacement for fq_codel, CAKE or a kernel RAN scheduler. The benchmark profile compares those real queueing baselines separately.

## Hard constraints and bounded behavior

The model excludes paths that are broker-ineligible, unauthorized, stale, below the trust/MTU floor, have zero or invalid capacity/loss, exhausted data allowance, or missing declared failure-domain identity.

It returns no route when no candidate fits. It rejects duplicate path IDs, empty IDs, invalid loss bounds, zero payloads, zero path count, invalid improvement thresholds and more than 64 candidate paths. Cost/energy conversions and aggregation use checked integer bounds; overflow rejects the candidate rather than saturating into a false fit.

## Isolated qdisc command planning

`qdisc::build_plan` creates a deterministic argv-only topology plan for fq_codel and CAKE tenant-fairness experiments. It does **not** execute the plan, create namespaces, change routes or install qdiscs.

The plan uses four disposable namespaces (tenant A, tenant B, gateway and server), an in-gateway bridge, separate tenant source IPs and one shaped WAN egress veth. Only the two tenant namespaces receive a default route; the gateway and server have no default route. There is no NAT, host bridge, physical NIC, or public route in the plan.

The read-only CLI emits the complete plan as JSON:

```sh
cargo run --manifest-path tools/connectivity-multipath-policy-core/Cargo.toml --bin qdisc-lab-plan -- \
  --run-suffix demo001 \
  --arm cake_besteffort \
  --rate-bps 100000000 \
  --duration-seconds 30 \
  --tenant-a-flows 8 \
  --tenant-b-flows 1
```

Supported arms are `fq_codel`, `cake_besteffort`, `cake_diffserv3` and `cake_diffserv4`. The CAKE arms use explicit `bandwidth <rate>bit triple-isolate nonat`; the initial lab has no NAT. fq_codel uses TBF plus fq_codel child at the same configured rate. Rate matching controls the bottleneck capacity, but the AQM/shaper semantics remain different and are analyzed as separate arms. The two tenant iperf3 commands are marked as a concurrent group so a future executor cannot serialize away the contention being measured.

Cleanup commands carry a `namespace-created-by-this-run` guard. A future executor must parse the preflight namespace inventory, refuse name collisions, track which namespace creations succeeded, and delete only those exact namespaces. The plan's cleanup section is not permission to delete arbitrary namespaces.

The TBF burst uses a conservative minimum `HZ=100` assumption and adds one Ethernet MTU to the rate-per-jiffy byte requirement. The plan emits the assumption and computed burst, but it does not verify the actual host kernel configuration. A future executor must read the exact `CONFIG_HZ` and refuse installation if it is unavailable or less than 100; the HZ assumption is not a kernel measurement.

The current stage is deliberately plan-only. Namespace creation, qdisc execution, iperf3 runs, counter capture, ownership-aware cleanup and artifact receipt generation remain a separate opt-in executor task. A valid plan is not a benchmark result.

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

## Required next steps before a performance claim

1. Compare model output with the exact Linux kernel MPTCP path-manager and scheduler configuration.
2. Add a two-ended Multipath QUIC adapter only when the pinned endpoint implementation supports the IESG-approved protocol revision.
3. Add deterministic packet/ACK/reordering traces and a bounded receiver reassembly model.
4. Calibrate simulated path estimates against namespace/VM and then hardware observations.
5. Publish the full Pareto frontier (useful-task completion, p95/p99 latency, cost, energy, reorder bytes and fairness), preserving all failures.

A passing unit suite proves code contracts only. It does not prove live multipath behavior, end-to-end improvement, production readiness or SOTA superiority.
