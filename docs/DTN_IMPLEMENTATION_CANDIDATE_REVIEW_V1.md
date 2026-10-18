# DTN Implementation Candidate Review v1

**Status:** research shortlist only. No implementation has been adopted, deployed, independently audited or qualified for the Luminous Connectivity Commons.  
**Reviewed:** 18 October 2026.  
**Parent work:** local-first outage resilience PR #68; issue #67.

## Recommendation

Run a controlled, version-pinned evaluation of **Hardy** as the first integrated BPv7/BPSec candidate, with **dtn7-rs** as the independent comparison implementation. Keep the **bp7** Rust library as a possible encoding/decoding component only if integration needs require it. This is an evaluation order, not a security endorsement or final product choice.

The important paradigm shift is to buy interoperability by integrating a standards implementation, not to invent a Luminous-specific delay-tolerant wire protocol. Spend Luminous engineering effort on the parts the protocol implementation cannot solve on its own: local-service usefulness, recipient idempotency, queue quotas/fairness, community consent, low-cost hardware operation, measurable failure recovery and evidence receipts.

## Candidate matrix

| Candidate | Publicly documented capabilities | Why evaluate it | Questions before adoption |
|---|---|---|---|
| [Hardy](https://github.com/ricktaylor/hardy) | Rust modular BPv7 daemon; advertises RFC 9171, BPSec RFC 9172/9173, TCPCLv4 RFC 9174 with TLS, pluggable local-disk/SQLite/PostgreSQL/S3 storage, time-variant routing, and bidirectional ping/echo interoperability tests against seven other implementations. Its repository states Apache-2.0 licensing. | Most complete documented baseline across bundle processing, security, transport, persistence and interop; storage options permit an SBC evaluation before any cloud deployment. | Confirm current commit/release state, exact enabled BPSec security contexts, key lifecycle and secure defaults, target build/footprint, dependencies, license and component licenses, disk fsync/recovery semantics, resource limits, and whether interop tests cover the intended features. Project documentation describes it as cloud-oriented; do not assume router-class hardware is supported. |
| [dtn7-rs](https://github.com/dtn7/dtn7-rs) | Rust DTN daemon for RFC 9171; TCPCLv4, minimal TCP, HTTP, HTTP pull and minimal UDP convergence layers; configurable BPSec policies; routing strategies including spray-and-wait, flooding, epidemic, static and sink routing. Repository declares Apache-2.0 and MIT licenses and says the code is under development and may change frequently. | Useful alternative for studying routing-policy options, simpler convergence layers and integration via command line/websocket/application interfaces. | Pin and review the exact revision; verify BPSec defaults, persistence/crash behaviour, route-resource controls, supported convergence-layer extensions and interoperability with the selected Hardy configuration. Under-development status increases the need for a maintained fork/upgrade plan if adopted. |
| [bp7 Rust library](https://docs.rs/bp7/latest/bp7/) | Rust library implementing BPv7 bundle encoding/decoding. Its documentation points to dtn7-rs for a daemon. | Possible library component where a small application owns processing and convergence layers. | Not a complete DTN service by itself; queue durability, routing, convergence layers, key policy, quotas, operations and interop still need separate components. Compare implementation version and security support before use. |
| [hardy-bpv7 crate](https://docs.rs/hardy-bpv7/latest/hardy_bpv7/) | Rust BPv7 crate with bundle creation/parsing/manipulation, described as supporting BPSec and a no_std-compatible core. | Could support a small footprint if the full Hardy daemon proves too heavyweight. | Do not infer a complete BP agent, security policy lifecycle, storage, routing or convergence layer from the core library description. Test feature configuration and dependency graph on the actual target. |

Sources: the project READMEs and crate docs linked in the candidate names above; protocol references RFC 9171, RFC 9172, RFC 9173 and RFC 9174.

## What the published capabilities do and do not prove

These are project-documented feature claims, not independent evaluations by Luminous. Hardy reports seven interoperability targets and bidirectional ping/echo checks; that is a stronger starting point than having no interop inventory, but ping/echo is not a substitute for testing payload integrity/confidentiality, expiry, long partitions, quota exhaustion, crash recovery, clock skew or production resource use. A reported interop matrix is a hypothesis to reproduce against exact versions and recorded configurations.

Similarly, support for BPSec does not mean end-to-end confidentiality is correctly configured for this use case. A security profile needs explicit algorithms, key distribution/rotation, endpoint authority, failure behaviour and handling of stale trust data during a partition. Do not disable validation or fall back to plaintext to make an interop test pass.

## Proposed evaluation protocol

### Gate 0 — supply-chain and licence

For every candidate, pin immutable source commit and release; archive Cargo.lock and licence notices; inventory transitive and native dependencies; review advisories, fuzzing/tests, maintainer activity, update procedure and build reproducibility. Distinguish software licence compatibility from patent, cryptographic, export, operator and telecom permissions. No candidate is accepted on repository popularity alone.

### Gate 1 — target resource envelope

Build on a named generic ARM64 SBC and a named x86_64 reference host first; add router/OpenWrt or embedded targets only if the operating system, storage and Rust runtime are supported. Measure cold start, steady-state RSS, CPU time, idle/peak watts, disk growth, bundle bytes/sec and recovery after storage pressure. Use the intended storage backend, not an in-memory demo, before making durability claims.

### Gate 2 — interoperable send/receive

Use two independent implementations and explicitly named convergence layers. Exchange payloads in both directions. Capture exact versions and configurations. Verify EID/address behaviour, lifetime/expiry, bundle IDs, delivery reports, unsupported extension blocks, fragmentation policy, reconnect, duplicate bundle reception and no-route outcomes. A pass applies only to that exact combination.

### Gate 3 — BPSec and adversarial handling

Run positive and negative tests for authenticated integrity, confidentiality, altered payload, wrong key, missing key, malformed security block, duplicate security operation, invalid endpoint, stale/revoked test key, unauthorized destination and receiver policy rejection. Verify that failure is closed and that public logs do not contain keys or plaintext. An RFC label or successful encrypted test alone is not an independent crypto audit.

### Gate 4 — outage/queue resilience

Inject partitions of 1 minute, 1 hour and 24 hours; reconnect repeatedly; restart before/after durable write, after send/before acknowledgement, after acknowledgement; fill storage and per-peer quotas; introduce duplicate relays and clock drift. Measure delivery among eligible, unexpired messages; expired count; duplicate application effects; p50/p95/p99 delivery delay; per-priority starvation; RSS/disk/energy; and recovery. Preserve every failed attempt and its denominator.

### Gate 5 — authorized small pilot

Only after the prior gates pass, test in a named, consented site over authorized links. Verify ISP sharing terms, ICASA requirements, type approval and site/installation safety before providing public or relayed access. Owner consent must be opt-in and revocable; a community controller must not remotely enable a node's relay beyond the owner's granted caps.

## Initial thresholds

- Zero successful delivery of test payloads whose required BPSec integrity check fails.
- Zero sends to a route marked unauthorized or security-profile-incompatible.
- Zero messages newly selected for forwarding after expiry under the declared clock policy.
- At least 99% of unexpired synthetic messages delivered after route recovery, with at least 100 messages per scenario and raw denominators.
- Zero duplicate application side effects for duplicated message IDs at the reference durable consumer.
- Zero admission above configured message, peer, queue and retention bounds.
- 24-hour local service continuity remains a separate test; DTN delivery success does not prove local DNS/app independence.
- Resource and latency numeric targets remain baseline-first until measured on a named hardware and workload profile.

## Decision rule

Do not select solely by throughput. First reject candidates that fail protocol/security requirements, crash-consistency, resource bounds, license review or repeatable interop. Among remaining candidates compare fully burdened cost per useful task, power/storage footprint, operational complexity, security configuration risk, repairability and upstream/provider requirements. If Hardy is too large for gateway hardware, consider a small BPv7 library plus a reviewed adapter only after the missing components and ownership burden are explicitly costed.

## No-go conditions

No public relaying, message payload storage, cross-community key trust, emergency communications claim or customer data until: the exact implementation/security profile is pinned; target-resource and persistence behaviour is measured; negative tests are retained; legal/provider/site permissions are resolved; and an independent reviewer reproduces the test receipts.
