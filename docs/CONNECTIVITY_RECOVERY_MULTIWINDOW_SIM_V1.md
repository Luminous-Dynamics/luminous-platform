# Stateful multi-window connectivity recovery model v1

**Evidence class:** synthetic simulation only. This is a bounded discrete-window model, not an AP/radio/network-stack implementation, and the fixture is not field evidence.

## Why another recovery model?

The v1 recovery simulator models one abstract recovery window at a time. Its atomic-task assumption can make a grant smaller than the full task look like no useful progress. Real transfers can carry partial byte progress over several scheduling intervals, and queues hold competing work that arrives while older work remains unfinished. This follow-on model tests that distinction without silently rewriting the v1 model.

The two policies receive the same per-mode task episode and per-window capacity:

- **Baseline:** first-in-first-out byte service. It serves the oldest admitted unfinished task first and preserves that task's partial progress between windows.
- **Candidate:** the existing weighted max-min allocator gets one demand per peer per window, with essential bytes kept separate from ordinary bytes. Grants are then delivered to that peer's outstanding tasks in priority/deadline order.

It deliberately shares the existing allocator rather than creating a second fairness formula. The only changed variable is the service policy.

## Stateful semantics

Every task has an opaque peer token, weight, class, mode, arrival window/order, byte demand, deadline window and expiry window. Task progress survives each window; delivered bytes are never discarded when the window closes. A task succeeds once cumulative delivered bytes reach its requested service bytes on or before its deadline/expiry boundary. Incomplete tasks are failed at the first declared deadline/expiry boundary. No task/peer token is written into the public paired task-outcome receipt or aggregate fairness report.

Each mode is simulated as an independent episode and receives its own fixed byte budget on each window. Capacity is **not shared across mode episodes**, and their reports must not be added together as if they were one simultaneous radio session. This keeps the existing benchmark's online, WAN partition and route-recovery strata distinct. A window is an abstract simulation interval. Its duration is used only to make a coarse receipt latency; it is not measured packet latency or a prediction for a physical AP.

Energy and price are intentionally simple declared inputs: every task that remains eligible in a window accrues the declared fixed synthetic active-window energy, even if that policy granted no bytes, plus a per-KiB charge for its granted bytes. The model does not estimate radio state, retransmission, PHY rate, listen/sleep duty cycle, TCP congestion, application retries, real electricity, or a provider tariff. Do not interpret this estimate as an engineering energy or savings result.

## Synthetic fixture

The fixture repeats a three-task contention episode for each of the three modes:

| Task | Priority | Service demand | Arrival | Deadline |
|---|---|---:|---:|---:|
| Older large transfer | Bulk | 20,000 bytes | Window 0 | Window 4 |
| Short essential task | Essential | 3,000 bytes | Window 0 | Window 0 |
| Short interactive task | Interactive | 4,000 bytes | Window 0 | Window 1 |

The declared capacity is 9,000 bytes per window, with a 3,000-byte essential reserve and five total windows. The arrival order intentionally places the large bulk transfer first. It is a designed policy-discrimination case, not a representative traffic mix.

Expected synthetic outcome for each mode:

| Measure | FIFO baseline | Fair-share candidate |
|---|---:|---:|
| Tasks completed | 1/3 | 3/3 |
| Jain index of completed tasks per peer | 333,333 ppm | 1,000,000 ppm |
| Essential task by its window-0 deadline | Missed | Completed |
| Interactive task by its window-1 deadline | Missed | Completed |
| Large bulk task | Completed by window 2 | Completed by window 2 |

That result is a consequence of this specific demand order and deadline configuration. It is not evidence of general improvement on a live network, and it is not a claim about radio airtime fairness.

## Contracts and reproducibility

- Strict Draft 2020-12 scenario schema; unknown fields rejected.
- Remote schema references rejected before validator construction.
- Bounds of 4,096 tasks, 256 windows, and 32 MiB input; arithmetic overflow fails the simulation rather than wrapping.
- Each mode must cover essential, interactive and bulk tasks, yielding all nine mode/priority cells.
- Task and paired-outcome identifiers must be unique; every peer uses one weight across episodes.
- Every task's arrival, deadline and expiry must be inside the declared horizon; deadlines/expiries cannot precede arrival.
- Canonical task ordering binds the scenario SHA-256; paired baseline/candidate trials reuse the existing task-outcome profile/run validator.
- The exported raw receipt carries the evidence class synthetic_simulation and omits peer identifiers. Aggregate results keep per-mode per-peer completed-task Jain indices, while omitting identity mappings and peer tokens.

## Next measurable benchmarks

Treat these as preregistered hypotheses and then test against actual baselines:

1. **Deadline usefulness:** improve the per-class share of tasks completing before their actual service deadlines under matched contention; report essential, interactive and bulk separately.
2. **Persistent progress:** zero dropped previously delivered bytes across simulation-window boundaries; in a physical test, verify with server-side byte/checksum accounting.
3. **Peer fairness:** report Jain's index for completed tasks per peer and delivered bytes per peer, but never substitute one for the other. A peer could deliver lots of bytes yet complete no task.
4. **Starvation bound:** no task remains queued past its declared TTL; for eligible tasks below the declared demand/capacity envelope, establish a bounded wait and verify it with adversarial arrival patterns.
5. **No hidden clean-path regression:** report clean-path success, p50/p95/p99 task latency, retransmissions/retries where exposed and bytes-per-success alongside degraded/recovery results.
6. **Recovery stability:** compare time-to-first-verified-task and time-to-drain backlog under repeated outage/recovery episodes, not just at one snapshot.
7. **Energy/cost:** only set targets once a physical meter and matched tariff/accounting scope are available; include failed attempts and idle/active system energy.

## Roadmap beyond this model

The next useful leap is to replace “fair bytes” with a measurement-driven multi-resource scheduler that understands verified task deadlines, measured airtime cost, queue delay, energy and the available route (local service, LAN, WAN or delayed store-and-forward). The scheduler should choose among eligible paths only with authenticated policy/permission and route-health evidence. Its evaluation needs an isolated multi-client AP harness: RSSI/SNR where meaningful, retries, actual airtime counters when exposed, queue discipline counters, bidirectional TCP/UDP measurements, calibrated wall energy and application-verified completion.

Before considering algorithmic superiority, compare current FIFO, the fair-share allocator, Linux queue disciplines such as CAKE/FQ-CoDel and (where standards/hardware permit) AP airtime scheduling as distinct arms. Keep firmware, channel/bandwidth, client drivers, rate limit, placement, task profile and randomization block identical between arms. No single fairness scalar or throughput headline proves the best design.

## Research references

- Broadband Forum TR-398 / BBF.398 Wi-Fi AP performance plan: https://www.broadband-forum.org/public/certifications/bbf-398-carrier-grade-wi-fi/
- IETF RFC 6349, Framework for TCP Throughput Testing: https://www.rfc-editor.org/rfc/rfc6349.html
- iperf3 JSON output and test controls: https://github.com/esnet/iperf/blob/master/src/iperf3.1
- OpenWrt SQM with CAKE / FQ-CoDel: https://openwrt.org/docs/guide-user/network/traffic-shaping/sqm

## Non-claims

This simulator does not implement a scheduler in Linux/OpenWrt, send network traffic, consume real queue counters, measure radio airtime, authenticate peers or essential priority, verify store persistence, model RF contention, or validate device/provider/regulatory approval. The generated receipts structurally validate synthetic outcomes only. A real AP study must use an owner-authorized isolated lab, retain instrument/config/profile hashes and raw trial denominators, and separately prove application outcomes.
