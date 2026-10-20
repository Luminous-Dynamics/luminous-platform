# Read-only AP observation collector v1

**Status:** implementation and parser contract; no AP has been observed in this environment.

## Goal

Turn a locally authorized Linux Wi-Fi testbed's existing diagnostic counters into a privacy-safe, schema-validated *snapshot*. This is the practical bridge between PR #77's measurement protocol and future measured-task receipt adapters. It performs no active throughput test, does not create traffic, does not modify device state, and does not calculate counter deltas from repeated snapshots.

## Read-only command allow-list

The CLI invokes these command programs directly with a fixed argument vector—no shell or user-controlled command string:

- \`iw dev INTERFACE station dump\` — station count, aggregate RX/TX bytes, retry/failure counters, and RSSI signal summary where exposed.
- \`iw dev INTERFACE survey dump\` — per-frequency channel active/busy time and noise where exposed.
- \`tc -s qdisc show dev INTERFACE\` — aggregate qdisc sent/dropped/overlimit/requeue/backlog counters where emitted by the installed qdisc.
- \`ip -j link show dev INTERFACE\` — MTU, operational state, and administrative UP flag.

The interface argument is checked against \`[A-Za-z0-9_.-]{1,15}\`. The collector never calls configuration/set commands, never runs \`iperf3\`, never changes qdiscs, never starts packet capture, and never invokes \`sh -c\`. No raw stdout/stderr is exported.

## Fail-closed testbed admission

Usage:

\`\`\`text
connectivity-ap-observe <manifest-schema.json> <measured-testbed-manifest.json> <interface> <run-id> <output.json>
\`\`\`

Before probing the host, the CLI validates the manifest using the AP testbed contract. It refuses a \`planned\` manifest and requires an owner-authorized isolated lab network with no public users. The measured-stage manifest must contain the exact AP firmware/radio configuration digests, declared clients and drivers, tool versions, randomized comparison design, a calibrated power meter record and profile digests. The manifest SHA-256 is bound to the snapshot; CI separately tests that planned equipment is refused before any command invocation.

The manifest's own statements are not remote attestation. A local operator could misreport them; a hardware-backed attestor and instrument-calibration audit are later research work. Read-only counters collected from a client station are not automatically the same as counters from an AP managing many clients.

## Privacy and missing-counter semantics

Raw station addresses and IP addresses are parsed only as part of local command output and never copied into the snapshot. The \`ip -j link\` parser retains only MTU, operstate and the administrative UP bit; it discards MAC, local IP, ifindex and every other field. Station metrics are aggregates, and raw command output is not included. The operator-supplied interface token and run token are the only identifiers in the snapshot.

If command execution fails, the command status says \`unavailable\` or \`error\`, and unavailable measurements are \`null\`. An empty, successfully queried station list may report station count and sums as zero; a failed/missing command never turns a missing counter into zero. If one station lacks a requested byte/retry/failure counter, that aggregate is null rather than an undercounting partial sum. If a driver or qdisc doesn't expose a metric, it stays unavailable. Survey records are per frequency; active/busy counters are cumulative snapshots and must not be added across repeated collection times as if they were deltas.

The \`bench_observed_snapshot\` evidence label only means the collector recorded live command output. It is not a claim of successful task delivery, stable service, standards compliance or independent instrument authentication. Capture timestamps use the local system clock; calibrate/synchronize that clock before using timing-sensitive comparisons.

## Tests

Parser tests use fixed text/JSON fixtures and cover station counter sums, identifier redaction, null propagation for missing counters, per-frequency survey semantics, qdisc counters, link metadata minimization, command-failure handling, schema validation and planned-manifest refusal. CI does **not** execute real system probes. A physical deployment must first be set up by an authorized operator on an isolated network, then collect repeat snapshots and application-verified tasks under the paired protocol in \`CONNECTIVITY_AP_MEASUREMENT_PROTOCOL_V1.md\`.

## Future work

The next useful addition is a paired snapshot *delta* contract with monotonic counter-reset detection, interface/AP role, sampling interval, clock quality and source-run digest. After that, join snapshots to server-verified task completion without raw MAC identity. Use a separate active test runner for authorized iperf3 and task workloads; keep passive observer and active experiment launcher as separate binaries/permissions.
