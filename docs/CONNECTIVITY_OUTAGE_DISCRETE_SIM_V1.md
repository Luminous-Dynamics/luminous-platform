# Deterministic outage and recovery simulation v1

Status: synthetic discrete-event model. This is not an RF simulator, field benchmark or router implementation.

The conformance workflow is also present on the parent task-outcome branch so this stacked PR can be checked against the canonical validator that it depends on.

## What it compares

The simulator replays a versioned list of task arrivals through two abstract strategies:

- Baseline: WAN-only. A task arriving during the configured WAN outage fails the route attempt.
- Candidate: local-first when a task is explicitly marked as locally executable and the local link is available; otherwise use WAN when available; during a WAN outage, optionally admit remote-bound work to a bounded queue for forwarding at recovery.

The simulator generates the same strict paired-task run record used by the task-outcome validator in the parent PR. The generated record binds to the exact benchmark-profile bytes and is validated before the CLI writes it. Its optional run-artifact digest binds to a canonical JSON serialization of the scenario object after sorting the task trace by arrival, priority and task ID (not the original whitespace/formatting or array order of the input file); the report includes the simulation ID and this same canonical scenario digest.

## Determinism and invariants

- Stable event ordering: arrival time, then essential/interactive/bulk priority for simultaneous arrivals, then task ID.
- Stable output ordering by pair ID, with baseline then candidate trials.
- Strict scenario JSON Schema; remote schema references are rejected.
- Runtime checks require online tasks to arrive outside the outage and partition/recovery tasks to arrive inside it.
- Bounded scenario bytes, task count, queue items and queue bytes.
- Integer millisecond, byte, energy and minor-currency arithmetic with overflow checks.
- A queued message whose TTL expires before recovery is never forwarded.
- A task whose deadline is known to precede recovery is not given a pointless queue slot.
- All generated records say synthetic_simulation and have an empty measured environment.

## Parameter interpretation

The input link model defines fixed path latency, byte rate, base attempt energy, synthetic energy per payload KiB and synthetic charge per payload KiB. The modeled transfer duration is fixed latency plus a ceiling-rounded payload transfer time. Energy and charge are assumed constants supplied by the scenario author; they are not taken from a physical meter, tariff lookup, provider contract or deployed network.

Queue capacity counts tasks still pending at the arrival event. Queue entries are released when the WAN recovers or their task TTL/deadline ends. The simulator models one outage interval, simple task arrivals, a local-execution eligibility flag, queue admission, one recovery attempt and task deadline/TTL outcomes.

## Known omissions

This v1 does not model concurrent bandwidth sharing, per-peer quotas, wireless airtime, interference, retransmissions, TCP/QUIC behavior, packet loss, DNS, real local services, encryption, identity, acknowledgement verification, route discovery, link switching delay, traffic shaping, hardware power curves, radio regulations or provider authorization. It does not call sockets, mutate routes, install firmware or contact external systems.

The separately authored fair-share allocator (#73) is a byte-allocation contract; its byte budget does not automatically equate to radio airtime or shareable WAN capacity. This simulator also does not yet compose that allocator into concurrent traffic events.

## How to run

From repository root:

    cargo fmt --manifest-path tools/connectivity-outage-discrete-sim/Cargo.toml -- --check
    cargo test --manifest-path tools/connectivity-outage-discrete-sim/Cargo.toml --all-targets
    cargo clippy --manifest-path tools/connectivity-outage-discrete-sim/Cargo.toml --all-targets -- -D warnings
    cargo run --manifest-path tools/connectivity-outage-discrete-sim/Cargo.toml --bin connectivity-outage-discrete-sim -- schemas/connectivity-outage-scenario-v1.schema.json examples/connectivity-outage-scenario-synthetic-v1.json schemas/connectivity-task-outcome-run-v1.schema.json schemas/connectivity-task-outcome-profile-v1.schema.json profiles/connectivity-task-outcome-benchmark-profile-v1.json /tmp/connectivity-outage-run.json

The CLI prints the validated outcome report to stdout and writes the raw synthetic run to the final path argument.

## Promotion path

1. Unit/contract CI confirms deterministic simulation and negative controls.
2. Trace replay confirms declared traces can be reproduced with raw denominators preserved.
3. Compare modeled hypotheses with isolated Linux/network-namespace tests, without claiming they establish Wi-Fi RF behavior.
4. Measure on an isolated AP/client testbed: task success, p50/p95/p99 latency, actual transferred bytes, airtime, retries, real power, and tariff inputs.
5. Only then evaluate a site pilot with owner consent, local privacy review, legal upstream sharing permission, and applicable equipment/spectrum approvals.

Passing this crate's CI proves only the deterministic abstract model and generated receipt contract.
