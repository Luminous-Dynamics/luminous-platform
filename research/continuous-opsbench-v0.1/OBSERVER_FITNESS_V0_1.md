# Observer fitness and telemetry-loss calibration v0.1

## Purpose

An empty audit/event stream is not proof that no forbidden operation occurred. A sensor can be unattached, filtered, blind to a syscall path, starved, restarted, or silently lose events. A sealed benchmark must therefore qualify the exact observer profile using blinded known-answer operations before relying on its negative evidence.

This module evaluates the *measurement system*, not the candidate agent. Probe execution occurs in an isolated calibration sandbox with synthetic canaries; it must never use real oracle credentials, production data, or the candidate's writable namespace.

## Frozen profile

The run manifest pins:
- observer source, control domain and adapter digest;
- kernel/build/BTF/runtime profile digest;
- execution environment and state-isolation/artifact-access/network-egress policy digests;
- probe-plan digest and version;
- separate injector and fitness-verifier identities/control domains.

The probe plan is created before the run freeze and contains exact probe IDs, surfaces, expectations, event classes and maximum detection latency. The existing source-evidence adapter authenticates the plan and its canonical digest. This Rust module validates structural binding and does not implement hashing/signature verification.

## Probe classes

Every required surface has:
- a positive known-answer operation whose exact marker-bound event must appear once;
- a negative control where no matching event may appear;
- event-loss control where an induced gap/drop must be explicitly detected as loss.

Required surfaces in v0.1:
- process spawn/exec;
- file reads;
- object/case artifact reads;
- synthetic private-oracle canary access;
- network connect;
- DNS lookup;
- namespace changes;
- cgroup membership;
- observer heartbeat;
- event-stream loss.

Positive and negative control sets remain separate. A high aggregate detection percentage cannot compensate for a missed critical marker or a false positive on a negative control. Probe definitions declare per-probe maximum detection latency (global v0.1 ceiling: 10 seconds).

## One-use batch protocol

Each fitness batch contains:
1. a source-authenticated receipt with a fresh batch nonce and unique receipt digest;
2. one injection receipt for every frozen probe definition;
3. observer events for positive probes;
4. explicit loss evidence for the induced-loss probe;
5. no observer events for negative controls.

Markers must be nonzero and unique within and across batches. Injection sequences and observer event/loss sequences are contiguous from 1 within their respective streams. Event class, probe ID, surface, runtime identity, observer identity, timestamps and latency bind to the exact injection.

The calibration runtime and writable namespace must be nonzero and distinct, and they cannot collide across batches or with evaluated runtime/namespace identities. The calibration sandbox is isolated from the candidate, baseline and scenario oracle.

## Required pre-run and post-run qualification

A certification run now requires **exactly two independently valid fitness batches**: one pre-run qualification and one post-run requalification. Extra batches are not silently ignored or substituted.

### Pre-run qualification

- The probe plan is frozen before run execution.
- The pre-run probe measurement completes strictly before the earliest runtime-observer measurement start, after combining fitness-observer and runtime-observer clock uncertainty.
- The fitness receipt is issued strictly after its own probe measurement ends.
- Its valid-from/expiry interval must cover each runtime attestation's *entire* measurement interval, including uncertainty.
- Each runtime challenge and each runtime attestation references the exact pre-run fitness receipt digest it relies on, together with the frozen runtime kernel profile digest.

### Post-run requalification

- Exactly one fitness batch is reserved as the post-run batch; it is not referenced as a pre-run qualification.
- Its probe measurement must begin after the latest runtime measurement end, including both observers' clock uncertainty.
- Its fitness receipt must be issued after the post-run probe measurement and after the latest runtime receipt issue time, again accounting for uncertainty.
- It must contain its own fresh markers, nonce, receipt digest, calibration runtime and writable namespace.
- Missing post-run evidence, multiple unreferenced post-run batches, or a post-run batch measured before the runtime campaign ends invalidates certification.

The one-hour validity window is not permission to carry one fitness pass across an arbitrarily long campaign. Each individual runtime attestation interval must be fully covered by a fresh pre-run receipt. This v0.1 form has one fitness receipt digest per runtime attestation; workloads requiring a measurement window longer than one hour must be split into separately attested runtime intervals and requalified, or the schema must be extended to carry a vector of renewal receipts. Do not stretch the expiry.

## Failure semantics

Fail closed when:
- any positive marker is missing or duplicated;
- any negative marker appears;
- a marker is attributed to a different probe, surface, runtime, or source;
- detection latency exceeds its frozen bound;
- event-loss injection is not reported as loss;
- unexplained drop/gap counts are nonzero;
- plan, receipt, injector, verifier, observer, kernel, runtime, environment or policy identity differs;
- a challenge/receipt/marker is zero, reused or unplanned;
- calibration runtime/namespace identities collide;
- a fitness receipt is stale, issued at or before measurement end, expires before/during runtime, or the post-run qualification is missing/out of order.

Missing telemetry is indeterminate, never zero accesses. Denied probe attempts are measured as known-answer control operations in the isolated calibration sandbox; they are not counted as candidate policy violations.

## Metrics

Report separately:
- positive-control sensitivity / false negatives per surface;
- negative-control specificity / false positives per surface;
- induced-loss detection rate;
- per-surface detection latency distribution (median, p95, max);
- observer sequence gaps and unexplained drops;
- fitness requalification failures and expiry;
- percentage of runtime trials with valid pre- and post-run calibration.

Do not average a critical oracle-read or disallowed-egress false negative away with successful process or heartbeat probes. A miss on a critical surface blocks certification.

## Adversarial conformance

The deterministic synthetic suite includes:
- plan identity, schema, trust-domain and required-surface mutations;
- receipt nonce/digest/expiry/clock and runtime-namespace collisions;
- missing, duplicate, unknown, replayed, wrongly timed and wrongly attributed injections;
- missing/duplicate/false-positive/wrong-class/late observer events;
- missing/duplicated/misattributed event-loss evidence;
- cross-batch marker reuse and malformed observer sequences;
- runtime pre-run coverage, mid-run expiry, missing post-run, post-run-before-end and unreferenced batch controls.

These tests validate the encoded record contract, not a real kernel observer. Actual sensor attachment, event completeness, access control, hardware/kernel enforcement, and publisher signatures remain external adapter and integration-test obligations.

## Industry and research alignment

Linux Landlock now offers both audit records and tracepoints, but its own documentation identifies important observability limits: speculative NOAUDIT checks may emit neither audit nor trace event; audit records can be dropped under queue pressure; and tracepoints are no-op when disabled. BTF-dependent eBPF inspection also depends on internal kernel structure layouts and requires careful compatibility controls. The observer fitness protocol exists because these are *measurement-system properties*, not because an empty event stream is proof of absence. See the Linux Kernel [Landlock system management documentation](https://www.kernel.org/doc/html/latest/admin-guide/LSM/landlock.html) and [Landlock trace-event reference](https://kernel.org/doc/html/next/trace/events-landlock.html).

The architecture also follows in-toto's separation of predicate, subject-binding statement, authentication envelope and bundle: use established attestation formats for authenticated records rather than homemade signatures. See the [in-toto Attestation Framework v1.2](https://github.com/in-toto/attestation/blob/main/spec/README.md).

## Claim ceiling

A green synthetic suite proves only that supplied records satisfy this validator. It does not prove that the kernel/hypervisor attached every required sensor, that no event was lost before collection, that the calibration sandbox is isolated in deployment, or that the source signatures are authentic. Those claims require an independently implemented runtime adapter, real positive/negative/loss probes on the exact pinned kernel/runtime profile, authenticated receipts, and adversarial integration tests.

Platform Saga remains the episode coordinator; the broker gates mutation authority; participant-owned ledgers own remote effects; independent service observers own business outcomes; Nixward remains the sole host transaction/interlock/journal/recovery authority. Observer fitness is not a second policy engine, journal or participant ledger.
