# Connectivity Commons: isolated AP/client measurement protocol v1

**Status:** proposed protocol and benchmark hypotheses; no physical measurements or device qualification are claimed.

## Why this layer exists

The current task-outcome and fair-share components are deterministic software models. They cannot infer Wi-Fi airtime from byte allocations, prove real end-to-end task success, characterize retries, or measure energy. This protocol establishes the next evidence boundary: an isolated, reproducible access-point/client experiment whose configuration is recorded before trials begin.

The objective is not to maximize a speed-test score. It is to maximize useful, privacy-preserving task completion per unit of shared airtime, energy and total economic cost while preserving acceptable latency and access fairness.

## Research basis and scope

- **Broadband Forum TR-398 Issue 4 / BBF.398:** the Broadband Forum technical library lists Issue 4 as in force (September 2026). It covers Wi-Fi residential/SOHO AP performance; the BBF.398 program tests products against the applicable mandatory and conditional test cases through approved laboratories. Use it as the AP RF/coverage/capacity/stability reference; this project does not claim TR-398 compliance or certification: https://broadband-forum.org/public/technical-library/ and https://broadband-forum.org/public/certifications/bbf-398-carrier-grade-wi-fi/
- **IETF RFC 6349:** use its disciplined TCP-throughput methodology, including path-MTU awareness, baseline RTT/bottleneck measurements and tests in both directions when links are asymmetric: https://www.rfc-editor.org/rfc/rfc6349.html
- **iperf3:** its JSON output is suitable for machine-collected interval summaries; retain original raw output as a local artifact and bind its digest into the private run record: https://github.com/esnet/iperf/blob/master/src/iperf3.1
- **OpenWrt SQM:** CAKE and FQ-CoDel are concrete queue-management baselines for investigating loaded latency rather than comparing peak throughput alone: https://openwrt.org/docs/guide-user/network/traffic-shaping/sqm

This protocol complements, rather than replaces, regulatory RF tests, an accredited test laboratory, the current TR-398 Issue 4 procedure, or provider permission. Do not transmit outside the allowed band, channel, power or authorization for the exact location and equipment.

## Isolation and minimum equipment

1. One access point/gateway under the test operator's control, with exact board revision, firmware image digest, radio configuration digest and recovery method recorded.
2. At least three Wi-Fi clients representing declared hardware/driver combinations: one task client, one competing-load client, and one additional independent client. Add devices only as an explicit scenario factor.
3. One wired peer/server connected on the AP's LAN side at a speed that does not become an unmeasured bottleneck. Record model, NIC, driver, link negotiation and host kernel.
4. A calibrated power meter covering the declared system boundary (AP/gateway, and any included auxiliary equipment). Software-estimated CPU watts are not a substitute for measured wall power.
5. Versioned iperf3, ping, iw and tc tools where available. Record exact versions. Record unsupported driver counters as unavailable; never substitute a zero.
6. A dedicated test SSID/VLAN or physically isolated network with no public users. A controlled WAN-edge emulator or lab connection is preferred for outage scenarios. Do not inject faults into a production household or public access network.
7. A monotonic measurement clock and a pre-registered workload/profile digest. Randomize or blocked-randomize the order of baseline/candidate paired trials.

The manifest validator admits a planned record with unknowns. Any measured stage must provide exact hardware/software revisions, firmware and configuration hashes, at least three recorded clients, attempted counter collection, a calibrated power meter, an authorized test network, a bound profile digest and randomized comparison order. It checks the record, not the underlying truth.

## Trial protocol

### 0. Freeze the configuration

Before collecting results, save the manifest and test profile. Record AP channel/bandwidth/transmit-power settings, client placement and orientation, client power mode, WAN profile, host kernel/drivers, queue discipline (none, CAKE, or FQ-CoDel), rate limits, clock method, meter calibration/resolution and environment. If a setting changes, create a new configuration identity instead of pooling results.

Keep raw MAC addresses, local IPs that could identify participants, credentials and any identity-to-token mapping local. Public receipts should contain only run-local opaque tokens, declared device/config classes, aggregates and content digests. Disable payload capture. Header-only diagnostic capture requires a separate documented need and privacy review; it is not part of this v1 protocol.

### 1. Establish the wired and radio baselines

- Check link speed/duplex and confirm the wired peer can exceed the expected AP throughput.
- Identify path MTU and baseline RTT before TCP stress; do not treat a TCP throughput result as interpretable when substantial loss or jitter makes the path dysfunctional.
- Collect idle latency, signal level, AP/client station counters, retry counts, airtime counters and channel survey where supported.
- Run iperf3 in both directions and retain client plus server JSON. Use short, bounded tests rather than continuous stress against an uncontrolled provider.
- Measure idle power for a predeclared stabilization interval; record total system boundary and meter sampling resolution.

### 2. Run a paired task workload matrix

For each declared priority (essential, interactive, bulk) and mode (online, WAN partition, route recovery), run the exact same task specification in the baseline and candidate arms. Randomize arm order within blocks; keep client placement, workload, radio configuration, WAN shaping and trial duration matched. Do not drop failed, timed-out, expired, or inconclusive attempts from the denominator.

| Scenario | Controlled condition | Required observation |
|---|---|---|
| Unloaded control | No competing client traffic | Task success, RTT distribution, achieved throughput, system energy |
| Mixed contention | One client uploads while another downloads; task client remains active | Task completion and p50/p95/p99 latency while loaded |
| Saturated downlink | Background downlink load | Queue delay, goodput, task success, per-station counters |
| Saturated uplink | Background uplink load | Queue delay, goodput, task success, per-station counters |
| WAN partition | Break only the controlled WAN path; leave test LAN and local server running | Local-task success, upstream-task failure classification, queue bounds |
| WAN recovery | Restore WAN and release an explicitly declared backlog | Time to first verified success, completion/expiry, retries, energy |
| Client-count sweep | 1, 3, then 5+ clients when equipment permits | Per-client success and latency; don't generalize beyond tested hardware |

For an algorithm comparison, the baseline and candidate must differ only in the predeclared policy under test. If testing queue management, compare none, CAKE and FQ-CoDel as separate arms; do not attribute the combined effect to the fair-share allocator. If testing local-first delivery, keep application behavior and verification identical across arms.

### 3. Define success at the application boundary

A task succeeds only when the test service verifies the expected response or durable acknowledgement before its declared deadline. A transmitted packet, HTTP connection, queue admission, local write, or client-side send is not by itself proof of remote completion. Use controlled local endpoints for repeatable LAN cases and a controlled, permitted upstream endpoint for WAN cases.

Every task attempt ends as succeeded, failed, or inconclusive with an explicit reason category. Retain all attempts. Keep task payloads out of public receipts; where integrity is needed, record a test-artifact digest rather than content. If the verifier cannot observe application-level completion, label the outcome as a transport proxy, not a successful user task.

### 4. Capture radio, queue, latency and energy evidence

- **Radio:** station retry statistics, airtime counters and channel-survey results where the AP/driver exposes them. Record units, collection method and unavailable counters explicitly. A byte scheduler's grant is not a radio airtime measurement.
- **Queueing:** qdisc counters and unloaded/loaded RTT. Report latency distributions, not only averages, and state whether the samples are per-packet, per-request or per-task.
- **Transport:** iperf3 interval/server/client JSON, loss and retransmission evidence where available, plus negotiated rates and path MTU. Report TCP and UDP separately; neither substitutes for task completion.
- **Power:** integrate calibrated meter samples over the whole predeclared system boundary and trial window. Report total joules, meter resolution, sampling rate and idle/baseline subtraction method. Energy per successful task is total measured joules during the trial divided by successful tasks; it is null when zero tasks succeed. Include energy consumed by failed attempts rather than discarding it.
- **Fairness:** report Jain's fairness index over completed tasks per peer under symmetric offered demand, alongside per-peer success counts and latencies. Also report bytes and airtime separately when measured. A fairness scalar must not hide that an essential task missed its deadline.
- **Recovery:** time from the observed WAN-restoration event to the first application-verified upstream success, then to backlog drain; report expiry and duplicate effects separately.

### 5. Preserve integrity and privacy

Every run should bind to exact profile bytes, testbed manifest, AP firmware, radio config, software revision and raw-artifact digests. The harness should distinguish measured data from simulator fixtures and derived aggregates. Avoid direct identifiers in exported records; keep consent records and any identity map local, and export only a random field-site token plus the consent-record digest for a field study. Never export credentials, payloads or raw MAC/subscriber identifiers.

Report missing instruments and unsupported counters as missing/unsupported; never impute zero. Keep raw denominators, failed runs and inconclusive runs. For publication, stratify or cluster uncertainty estimates by device, day and site when trials are correlated. Repeated packets in one run are not independent household-level trials.

## Proposed benchmark targets (hypotheses, not current results)

Targets should be preregistered and then challenged against measured baseline distributions. They are initial product goals, not standard requirements or certification claims.

| Dimension | Initial target to test | Guardrail |
|---|---|---|
| Local-first continuity | 100% of explicitly local, eligible essential tasks complete during a controlled WAN partition | LAN, local service and power must remain available; do not generalize to upstream Internet |
| Recovery usefulness | Candidate improves successful task completion by at least 5 percentage points in the predeclared congested-recovery workload | Paired confidence interval and raw failures must be reported |
| Clean-path regression | No more than 1 percentage point lower task success versus baseline | Separate clean and degraded cases; don't average away a regression |
| Loaded responsiveness | Candidate p95 task latency no more than 10 ms above baseline in the unloaded/clean condition | For loaded tests, report p50/p95/p99 and set limits from the actual service budget |
| Household fairness | Jain index of completed-task counts ≥0.90 under symmetric offered demand | Also disclose every peer's success rate and starvation/deadline misses |
| Recovery time | p95 first verified upstream success within 5 seconds of WAN restoration | This is a proposal for the lab profile, not a guarantee across providers |
| Energy | ≥10% lower measured joules per successful task for the targeted workload | Whole declared system boundary, calibrated meter, failed attempts included |
| Reproducibility | 100% of measured runs bind profile, AP firmware/config and harness digests | Zero silent profile changes or missing counter status |
| Privacy | Zero payload, credential, raw MAC or subscriber identifier in public receipts | Verify export artifacts, not merely the intended code path |

If any proposed numeric target conflicts with the service's actual deadline or safety requirement, the service-level budget takes priority. Baselines and effect sizes should be reported before treating a target as a deployment gate.

## Evidence stages

Keep the existing task-outcome contract's sample stages for comparability:

- **Contract smoke:** 3 paired trials, synthetic/configuration evidence only.
- **Lab screening:** at least 30 pairs in every required priority × mode cell (270 across the full 3 × 3 matrix).
- **Field pilot:** at least 100 pairs per cell (900 total for the full matrix), with consent and a site token.
- **Reliability qualification:** at least 300 pairs per cell (2,700 total).

With zero failures, roughly 299 independent Bernoulli attempts are needed for the familiar approximate 95% one-sided upper bound near 1% failure probability. Correlated trials from the same client/day/site weaken that inference; use an appropriate clustered analysis. A sample-count gate alone does not prove the target was met.

## Explicit non-claims

The schema and validator do not capture measurements, verify calibration certificates, attest the AP firmware, confirm permissions, validate user task truth, authenticate reporters, certify Wi-Fi performance, demonstrate cheaper service, or establish ICASA/provider approval. CI can validate code and fixture contracts only. A field claim requires an actual instrumented run whose immutable receipts match the tested configuration and whose trial-level evidence is available for review.
