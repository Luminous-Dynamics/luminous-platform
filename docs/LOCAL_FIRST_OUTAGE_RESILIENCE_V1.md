# Local-First Outage Resilience and Bounded Store-and-Forward v1

**Status:** draft research architecture; not a production protocol, deployment authorization or measured service.  
**Issue:** https://github.com/Luminous-Dynamics/luminous-platform/issues/67  
**Related work:** connectivity broker PR #64; federated Wi-Fi roaming PR #66; open community connectivity PR #62.  
**Research snapshot:** 18 October 2026.

## 1. Thesis: loss of the Internet must not mean loss of every useful service

The system should distinguish at least four conditions:

1. **Local service available:** participants on the local trusted network can retrieve explicitly hosted resources or exchange local messages.
2. **Upstream connected:** an authorized IP route reaches named remote service endpoints.
3. **Intermittent / delay-tolerant:** a destination is not currently reachable, but a bounded envelope can be queued for later forwarding over an authorized path.
4. **Unavailable / rejected:** local power/service is down, a message is expired, the recipient is unknown, an envelope exceeds limits, or a security/authorization gate rejects it.

This makes the local network a useful service plane, rather than a marketing label for a mesh. A mesh hop does not create a global uplink, and an upstream outage cannot be hidden by claiming that packets still pass locally.

## 2. Reuse Internet standards; do not invent a new bundle protocol

IETF RFC 9171 defines Bundle Protocol Version 7 (BPv7) for challenged environments, including store-carry-forward communication through intermittent connectivity and variable delay. A mature BPv7 implementation is the preferred protocol candidate where delay-tolerant network (DTN) interoperability is needed; this project should implement adapters and test contracts rather than create a competing wire format.

IETF RFC 9172 defines Bundle Protocol Security (BPSec) integrity and confidentiality services. It explicitly identifies resource-exhaustion and data-at-rest concerns in untrusted delay-tolerant networks. A protocol name alone does not provide secure defaults: select and document a compatible security context, key lifecycle and trust model, and verify implementation maturity before enabling cross-community relay.

OpenWrt supports mesh/backhaul options such as batman-adv on compatible hardware, but its documentation notes that batman-adv itself does not provide encryption or authentication. Mesh routing and message security are separate layers.

Sources:
- RFC 9171, Bundle Protocol Version 7: https://www.rfc-editor.org/rfc/rfc9171.html
- RFC 9172, Bundle Protocol Security: https://www.rfc-editor.org/rfc/rfc9172.html
- OpenWrt, B.A.T.M.A.N. / batman-adv: https://openwrt.org/docs/guide-user/network/wifi/mesh/batman
- ICASA mandate: https://www.icasa.org.za/pages/our-mandate

## 3. Layered architecture and trust boundaries

Client application
→ end-to-end protected application payload
→ local service API
→ bounded queue/local store
→ transport adapter
→ local LAN, authorized mesh, authorized WAN or BPv7/DTN
→ remote adapter / receiver
→ sender authentication, integrity verification and recipient policy
→ idempotent application consumer.

Separate planes:
- Local directory/cache/status, which must not need remote DNS or hosted identity.
- Route selection, which chooses among eligible network metadata.
- Telemetry/receipts, which must not capture content or credentials.
- Community funding/governance, which is not required for packet forwarding.

### Local service plane

- Pin a local service directory and service endpoints on the LAN; local-only use must not require external DNS, a hosted login service, cloud telemetry, or online configuration lookup.
- Provide a small set of explicitly licensed or public resources that operators have permission to cache. Respect freshness/version and source attribution; do not cache private user content as a shared resource by default.
- Show clearly whether a result is local, stale, synchronizing or remotely verified. A cache hit must not be mislabeled as current global state.
- Expose the route state clearly: local-only, upstream-reachable, intermittent, expired or blocked.

### Queue/envelope boundary

The queue model handles metadata and size/expiry/resource policy. It does not implement cryptography and does not interpret payloads. Before admitting sensitive content, a trusted application adapter must supply bytes protected by a reviewed end-to-end scheme. The queue must not claim that a boolean metadata field proves encryption or authenticity.

Minimal metadata includes a stable message/idempotency identifier; sender and recipient endpoint identifiers; creation/expiry times and clock-skew policy; protected-payload byte length and digest; declared class/priority and retention class; and a security-profile identifier plus adapter verification result. The reference model can validate declarations only; the production adapter must verify the actual protection.

Do not publish plaintext, keys, credentials, inner EAP identities, subscriber identifiers or raw browsing/message histories.

## 4. Delivery semantics that can be tested honestly

- **At-least-once transfer is the safe default claim.** A sender can retry after an uncertain acknowledgement. Receiver-side idempotency must prevent duplicate application side effects.
- Do not claim exactly-once transport. Exactly-once business effect requires a durable recipient idempotency record in the same transactional boundary as the effect, or an explicitly weaker application contract.
- Record states explicitly: queued, eligible, attempt-started, acknowledgement-pending, delivered-by-verified-recipient, expired, rejected or dead-lettered. A local transport write is not proof of remote delivery.
- Deduplication retention must outlive the accepted delivery/retry window; this relationship must be configurable and tested.
- Production acknowledgements must be cryptographically authenticated and bound to message ID, recipient, payload digest and expiry. The metadata-only reference model does not verify signatures.
- If clock authority is absent or skew exceeds tolerance, expiry uncertainty must be visible; do not silently extend valid windows forever.
- Key revocation is difficult under partition. Define maximum offline trust age, expiry and quarantine policy instead of pretending an offline node knows current global revocation state.

## 5. Quotas, fairness and resource exhaustion

A disconnected relay can be attacked without breaking cryptography. Enforce budgets before storing or forwarding: maximum message bytes; total queue bytes/items; per-sender and per-recipient quotas; maximum TTL and allowed future-creation skew; bounded metadata parsing; admission and relay rates; reserved capacity for explicitly authorized essential traffic; fairness caps on priority traffic; deterministic expiry cleanup; bounded dead-letter retention; and safe behaviour when disk or memory limits are reached.

Priority does not equal fairness. A priority queue can starve ordinary messages. Measure per-class delivery delay and starvation under sustained high-priority load.

## 6. Proposed benchmark targets

The versioned profile local-first-outage-resilience-benchmark-profile-v1.json will record machine-readable target hypotheses. All targets are proposed or baseline-first; none are field results.

| Benchmark | Initial acceptance target / method |
|---|---|
| Local continuity | Essential declared local tasks succeed throughout 24 hours with all WANs disconnected and LAN/power available. Test cloud DNS, identity, time and telemetry failure separately. |
| Queue bounds | Zero accepted envelopes above any message, queue, peer, TTL or parser bound. |
| Expiry safety | Zero newly forwarded envelopes expired at the forwarding decision time. Declare clock-skew policy and test rollback. |
| Sensitive-message admission | Zero sensitive payloads admitted unless the trusted adapter reports that the required end-to-end protection profile was verified. Metadata alone cannot prove this. |
| Tamper/replay | Zero integrity failures accepted by the named crypto implementation; zero duplicate application side effects for repeated message IDs at the durable consumer. |
| Store-and-forward success | At least 99% of unexpired test messages delivered after route recovery on a named topology; at least 100 messages per scenario; report raw denominator and p50/p95/p99 delay. |
| Crash recovery | Every previously acknowledged durable admission remains after controlled process restart; ambiguous writes remain ambiguous and are not silently reported delivered. |
| Storage fairness | No sender/peer exceeds configured quota. Measure per-class admission failures and starvation under declared stress load. |
| Energy | Establish whole-node idle watts, peak watts and Wh per successful task before setting a reduction target. |
| Privacy | Zero payloads, credentials, private keys, real user identifiers or browsing history in public receipts. |
| Network security | Unauthorized relay destinations and unsupported adapters are rejected. Test mesh transport security separately from bundle security. |
| Evidence traceability | Every benchmark binds code/config/profile revision, raw trace digests, trial counts and evidence class. |

For delivery, publish both delivered/eligible-unexpired and expired/queued rather than hiding expired or failed messages. Report uncertainty for small samples and stratify by topology, partition duration, payload size, hop count, queue pressure and power interruption.

## 7. Fault matrix

Use synthetic identities and an isolated environment. Do not run a public or third-party relay without authorization.

1. WANs disconnected while local LAN services run.
2. Partition, reconnect and repeated partial partitions.
3. Message duplicated at each hop or acknowledgement.
4. Modified payload bytes, wrong digest and invalid signature.
5. Expired message, expiry during outage and clock skew/rollback.
6. Future-dated creation timestamp beyond allowed skew.
7. Message too large, malformed/deep envelope and numeric boundary.
8. Queue full, disk full, peer quota full and relay budget exhausted.
9. Restart before durable write, after durable write, before send, after send but before acknowledgement, and after acknowledgement.
10. Recipient unavailable, wrong recipient, stale/revoked test key and malformed acknowledgement.
11. Unauthorized next hop or relay traffic outside allowlist.
12. Local services attempt external DNS, OAuth, CDN or telemetry and must disclose the hidden dependency.
13. High-priority traffic flood while ordinary messages remain queued.
14. Mesh route changes; link-layer failure while store-forward queue remains intact.
15. Corrupted cached public resource, stale version, bad source signature and cache invalidation.
16. Transport unavailable while bounded queue retention continues.

Each trial writes an immutable receipt with trial ID, topology/configuration digests, seed, evidence class, timestamps, outcome and failure code. Exclude raw user data. Keep failures in the denominator.

## 8. Phased implementation

**Phase 0 — reference contract and metadata model.** Create profile/schema, Rust validation, bounds and idempotency reference tests. No disk, radio, WAN or cryptographic implementation is implied.

**Phase 1 — local-only VM lab.** Use a test-local LAN, service directory and two synthetic nodes. Prove external DNS/cloud dependencies are absent; inject WAN blackholes, restart and storage-limit failures. Label results VM-simulated.

**Phase 2 — choose a real DTN adapter.** Inventory actively maintained BPv7/BPSec implementations, convergence layers, licences and maintenance. Run interoperability and security review; do not write a new BPv7 wire implementation. Bind exact implementation version and security context to the profile and receipts.

**Phase 3 — isolated bench test.** Use named, authorized gateway/AP/edge hardware. Measure power, storage, restart recovery and the fault matrix. Establish mesh encryption/authentication independently.

**Phase 4 — consented small field pilot.** Use a named site and authorized upstreams, with consent, privacy/retention controls, support and rollback. Publish anonymized aggregate measurements, exact revisions, failures and fully burdened costs.

**Phase 5 — cooperative scale-out.** Only after evidence, explore shared caches, volunteer relay nodes and cost-sharing. Require explicit node-owner consent, bandwidth caps, quotas, revocation, auditability, abuse handling and local fallback. Governance must not remotely enable an owner's relay without authorization.

## 9. Responsibilities across the stack

- **Luminous Platform:** config/evidence contracts, local service status, versioned receipts, rollback and operator controls.
- **Connectivity policy core:** chooses among supplied eligible route metadata; no payload inspection; no-route is valid.
- **Mycelix:** optional community consent/governance and cost accounting, not a mandatory dependency for local forwarding or emergency access.
- **Symthaea:** advisory cache/relay placement, capacity prediction and scheduling proposals based on aggregate metadata; never bypass quotas/authorization.
- **Symtropy:** simulate partitions, topology, energy, queue pressure, hop count and arrival patterns; simulation evidence stays separate.
- **Sol-Atlas:** map sites, resource coverage and backhaul availability; predicted coverage is not measured coverage.
- **Wi-Fi/OpenWrt layer:** local access and optional authorized mesh/backhaul, with separate authentication/encryption and device qualification.

## 10. South African deployment constraints

ICASA's mandate includes licensing communications providers and managing spectrum. Before public sharing/resale or custom radio links, review the exact arrangement, licensing/exemption requirements, equipment type approval, upstream provider terms, site permissions and safety. A mesh package, BPv7 software or open hardware design does not authorize operation.

Official regulatory starting point: https://www.icasa.org.za/pages/our-mandate

## 11. Explicit non-goals

This work does not claim free global Internet, guaranteed delivery, exactly-once transport, emergency-grade telephony, secure encryption from metadata alone, safe public relaying, universal mesh reach, OpenRoaming or Starlink permission, or physical performance from synthetic/VM results. A disconnected system can provide local service and carry pending data; it cannot retrieve arbitrary fresh remote information while no path to the source exists.


## 12. DTN implementation shortlist

The repository now includes a versioned candidate assessment at `docs/DTN_IMPLEMENTATION_CANDIDATE_REVIEW_V1.md`. Public project documentation currently makes Hardy the first candidate to evaluate because it advertises BPv7, BPSec, TCPCLv4/TLS, pluggable storage and bidirectional interop checks; dtn7-rs remains a comparison candidate because it documents multiple convergence layers and routing strategies. These are claims from candidate documentation, not Luminous test results or an adoption decision.

The first experiment should reproduce a narrow, pinned interop matrix and security-negative set on a generic ARM64 SBC and x86_64 host. Only then should the project decide whether a complete DTN agent or a smaller library plus adapter is appropriate. Low-cost gateway footprint, secure key lifecycle, crash-consistent durable storage, exact security context and field permissions remain explicit gates.
