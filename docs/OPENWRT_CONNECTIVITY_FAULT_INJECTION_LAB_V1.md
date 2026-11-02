# OpenWrt connectivity fault-injection lab v1

**Status:** lab design only; no emulator run, router test, hardware measurement or field result is claimed.  
**Purpose:** qualify the smallest safe gateway adapter before connecting community users.

## 1. Safety envelope

Use an isolated bench with no route to a production LAN. The only upstreams should be controlled test networks or explicitly authorized test subscriptions. Do not flood third-party services, alter a household router's live firewall, spoof public DNS, or share/redistribute an access link before permissions are verified.

Every run must record the exact router model/revision, OpenWrt release/image digest, kernel, package list including network-management packages, config digest, fixture revision, test seed, clock source and tester identity. Capture monotonic timestamps and raw event records. Redact keys, subscriber identifiers, private IP details if the report is public, and any user payload.

## 2. Reference topology

- **DUT gateway:** named OpenWrt-capable hardware, exact image and package set. The routing/failover path must not require Symthaea, Mycelix, Sol-Atlas or a hosted dashboard.
- **WAN-A:** controlled upstream A with working IP, DNS and application probe endpoints.
- **WAN-B:** controlled upstream B on an independently failed domain. Two ports attached to the same router, power supply or upstream are not independent evidence of resilience.
- **LAN client:** deterministic test client that runs timestamped task probes.
- **Local service node:** controlled DNS-independent/LAN-addressable status page, local document and local message queue.
- **Observer:** separate machine logging link state, routing state, test transactions and power. The observer must not be required for DUT operation.

A VM or Linux-network-namespace topology is useful for early policy and event testing, but it does not prove a physical OpenWrt image works on a device. Record evidence classes distinctly: source-static, VM-simulated, bench-measured, field-observed and independently-reviewed.

## 3. Fault matrix

Run each case against a named configuration and keep all attempts, including setup failures and timeouts.

| Case | Injection | Required observation |
|---|---|---|
| LAB-BASELINE-001 | Both WANs healthy | New eligible tasks complete over the chosen path; service and cost counters establish baseline. |
| LAB-LINK-LOSS-001 | Remove WAN-A carrier or disable its physical interface | New eligible sessions move to WAN-B inside the measured restoration window. |
| LAB-UPSTREAM-BLACKHOLE-001 | Keep WAN-A link/route up but drop upstream test traffic | Health detection does not confuse carrier presence with end-to-end reachability. |
| LAB-DNS-ONLY-001 | Fail controlled resolver while non-DNS upstream paths remain available | Report name-resolution failure separately; do not silently claim full internet outage or full health. |
| LAB-APPLICATION-PROBE-001 | Keep ICMP/TCP health endpoint reachable but fail a separately defined application transaction | Probe choice and failure modes are documented; no unbounded collection of sensitive browsing history. |
| LAB-COST-CAP-001 | Exhaust or lower remaining metered allowance below task requirement | Cost/data policy excludes the ineligible path before ranking. |
| LAB-UNAUTHORIZED-PATH-001 | Advertise a candidate with missing/false authorization metadata | Policy layer marks it ineligible; the adapter must not connect to it merely because it is fast or cheap. |
| LAB-STALE-TELEMETRY-001 | Freeze or age the link observation beyond the configured freshness limit | Decision fails closed for that candidate; expose a clear reason. |
| LAB-BOTH-WANS-DOWN-001 | Disable both upstreams while retaining LAN and power | Local service, local status and queued local message remain usable; no cloud DNS/auth dependency. |
| LAB-PRIMARY-RETURN-001 | Restore WAN-A repeatedly after failover | Reversion policy is explicit; avoid oscillation and record task interruption. |
| LAB-REBOOT-RECOVERY-001 | Reboot the gateway during an otherwise healthy scenario | Record time to network readiness, local-service behaviour and new-session recovery. |
| LAB-SESSION-SEMANTICS-001 | Drop active WAN during multiple long-lived sessions and new sessions | Report existing-session outcomes separately from new-session failover. Do not promise seamless continuity unless measured. |
| LAB-DEGRADED-LINK-001 | Apply controlled latency, loss and rate limitation to one upstream | Rank by task requirements and observed service quality; include the impairment profile in the trace. |
| LAB-OBSERVER-LOSS-001 | Stop the telemetry/dashboard process | Packet forwarding and locally necessary access continue without the observer. |
| LAB-RETURN-TO-BASELINE-001 | Remove every fault and repeat the baseline tasks | Confirm recovery, stable policy and no residual firewall/routing rules from the injection. |

Use controlled fault tools that fit the test environment, such as link state changes, a private test resolver, firewall rules scoped to lab interfaces, or Linux traffic control for a simulated hop. Do not copy an unscoped firewall rule to a production gateway. Injection and cleanup must both be logged; a cleanup failure invalidates the run.

## 4. Acceptance protocol

### Failover cohort

- Start with a short qualification set for each fault type, then run at least 100 predeclared WAN-A failure trials with a recorded mix of cable loss, upstream blackhole, gateway health failure and primary return.
- Success means a new eligible task established working end-to-end connectivity over an eligible alternate path within 30 seconds; a ping reply alone is insufficient.
- Initial target: at least 99 of 100 trials within 30 seconds, with the raw denominator, every failure and p50/p95/p99 restoration duration published.
- Report whether the same applications can recover after a changed public IP, DNS cache expiry and connection-state loss. Measure existing-session resets separately.
- Re-run with a fresh gateway configuration restore and with each WAN provider/path removed from the experiment. Do not call the backup independent unless common-cause dependencies are examined.

### Local continuity cohort

Keep the LAN, local node and declared power source available, then disconnect or blackhole every WAN for 24 continuous hours. Poll a declared set of local tasks from a client. Record task success, any dependency on cloud identity/DNS/time/update endpoints, local node reboots and energy. A time-compressed VM does not count as the 24-hour test.

### Reproducibility

Each test receipt should include:
- test/profile version and exact source/image/package/config digests;
- hardware model/revision and cable/port map;
- upstream provider/plan identifiers with secrets and subscriber identifiers withheld;
- declared fault schedule/seed and monotonic event times;
- task success definition, count attempted/succeeded/failed, restoration distribution and failure reasons;
- raw logs/digests, meter readings, known limits and evidence class;
- a cleanup attestation and a negative-control result.

No one should manually edit a failed result into a pass. Corrections produce a new run and immutable receipt linked to the old one. A simulator result and an on-device result must never share the same evidence label.

## 5. What this lab cannot qualify

Passing this plan alone does not establish RF coverage, Wi-Fi roaming behaviour across real APs, tower handoff, voice-call continuity, satellite visibility, upstream independence outside the tested paths, the truth of ISP invoices, type approval, licensing, a right to resell connectivity, or a universal service guarantee. Physical power backup and long-duration thermal reliability require separate measurement. Each adapter still needs its own provider, firmware and authorization gates.

## 6. External references

- OpenWrt multi-WAN / mwan3: https://openwrt.org/docs/guide-user/network/wan/multiwan/mwan3
- OpenWrt mwan3 nftables notes on failover and health tracking: https://openwrt.org/docs/guide-user/network/wan/multiwan/mwan3-nft
- IETF RFC 9171, store-carry-forward Bundle Protocol: https://www.rfc-editor.org/rfc/rfc9171.html
