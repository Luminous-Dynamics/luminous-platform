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

This first implementation is deliberately a transparent comparison model. It does not yet implement packet-level congestion control, ACK scheduling, retransmission, send-buffer dynamics, retransmission coupling, receiver reassembly, or live path discovery. The proportional split is a simple reference policy, not a claim that it is optimal.

## Model equations

Effective capacity is a simple loss-adjusted estimate:

`effective_bps = capacity_bps × (1 - loss_ppm / 1,000,000)`

Estimated completion for a path allocation is:

`finish_us = rtt_us + jitter_us + ceil(bytes × 8 × 1,000,000 / effective_bps)`

These are planning estimates. They do not model time-varying congestion or packet-level reordering.

## Hard constraints and bounded behavior

The model excludes paths that are broker-ineligible, unauthorized, stale, below the trust/MTU floor, have zero or invalid capacity/loss, exhausted data allowance, or missing declared failure-domain identity.

It returns no route when no candidate fits. It rejects duplicate path IDs, empty IDs, invalid loss bounds, zero payloads, zero path count, invalid improvement thresholds and more than 64 candidate paths. Cost/energy conversions and aggregation use checked integer bounds; overflow rejects the candidate rather than saturating into a false fit.

## Tests

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
