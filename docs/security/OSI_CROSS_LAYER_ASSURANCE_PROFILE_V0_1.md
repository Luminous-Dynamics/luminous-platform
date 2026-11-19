# Luminous OSI Cross-Layer Assurance Profile v0.1

**Status:** Research architecture and qualification plan — non-normative; not an audit, implementation certificate or production-security claim  
**Date:** 2026-11-19  
**Tracking:** [SECURITY-OSI-001 #497](https://github.com/Luminous-Dynamics/luminous-platform/issues/497)

## 1. Executive decision

Luminous retains the seven-layer OSI Reference Model. It does **not** add an official eighth OSI layer.

Luminous defines a **Cross-Layer Assurance Plane (XAP)** spanning the communication layers, repositories and deployment boundaries. XAP carries questions that a network layer alone cannot answer:

1. Who is accountable for the intent?
2. Which human, organization, device or workload is acting?
3. What exact, current authority permits this operation?
4. Where is that authority independently enforced?
5. What information moves through each boundary and transformation?
6. Who independently observes the actual effect?
7. How are revocation, containment, recovery and human outcomes handled?

This separates three concerns:

- **Communication security:** can endpoints exchange data securely?
- **System/action security:** can this exact actor perform this exact operation on this exact target?
- **Assurance and accountability:** what independently supports the claim that the operation was authorized, correctly executed, observed and recoverable?

A secure transport is useful, but never supplies application permission. A signed receipt proves that a key signed particular bytes; it does not prove that the assertion is true. Broker authorization does not replace participant-side final authorization.

## 2. Standards foundation

This profile composes established architecture rather than inventing another network model.

- **ITU-T X.800 / ISO/IEC 7498-2** defines OSI security services and mechanisms and considers where they may be placed in the seven-layer reference model. Its layering principles allow security services in more than one layer while warning against unnecessary duplication and violations of layer independence. X.800 also identifies additional protection needs at end systems, installations and organizations beyond the communication path.
- **ITU-T X.805** adds an end-to-end security architecture with three orthogonal concepts: (a) three security layers — Infrastructure, Services and Applications; (b) three planes — Management, Control and End-User; and (c) eight security dimensions — Access Control, Authentication, Non-repudiation, Data Confidentiality, Communication Security, Data Integrity, Availability and Privacy. These are not the same as the seven OSI protocol layers. The X.805 layers are a higher-level grouping; planes express operational context; dimensions express security objectives.
- **NIST SP 800-207 / 800-207A** supplies the zero-trust principle that network location and ownership must not create implicit trust; resource access should use user, device, application and service identity.
- **NIST SP 800-82 Rev. 4 Initial Public Draft** is especially relevant to the physical/OT portfolio. The draft, published 21 September 2026, expands OT coverage and adds guidance on asset management, network monitoring/detection, management-function protection and zero trust. Its comment period ends 30 November 2026.
- **NIST SP 800-53 Rev. 5.2.0 / OSCAL** provides control catalogs and machine-readable mappings. A mapped control is not evidence that a particular implementation works.

References:
- ITU-T X.800: https://www.itu.int/ITU-T/recommendations/rec.aspx?rec=X.800
- ISO/IEC 7498-2:1989: https://www.iso.org/standard/14256.html
- ITU-T X.805: https://www.itu.int/rec/T-REC-X.805
- NIST SP 800-207: https://csrc.nist.gov/pubs/sp/800/207/final
- NIST SP 800-207A: https://csrc.nist.gov/pubs/sp/800/207/A/final
- NIST SP 800-82 Rev. 4 IPD: https://csrc.nist.gov/pubs/sp/800/82/r4/ipd
- NIST SP 800-53 Rev. 5: https://csrc.nist.gov/pubs/sp/800/53/r5/upd1/final
- OSCAL: https://pages.nist.gov/OSCAL/

X.800/X.805 and NIST publications are architecture/control references, not proof of Luminous conformance. This document is not a certification or an independent security assessment.

## 3. OSI layer ownership and security assessment

The table maps communication functions to Luminous owners. It does not imply that one repository owns every control in a layer. Where a carrier, OEM, switch, access point, kernel, cloud operator or customer owns the actual enforcement, the deployment record must name that owner and provide its evidence.

| OSI layer | Security responsibilities | Luminous ownership / composition | Current assessment and open qualification |
|---|---|---|---|
| **L7 Application** | Application identity, authorization, consent, API/tool permissions, business logic, agent instructions, tenant isolation, transaction/effect semantics and recovery | Mycelix identity/authority/governance; Symthaea proposal/cognitive boundary; Platform broker/lifecycle; Nixward host transactions; participant adapters | **Architecture well specified; end-to-end qualification incomplete.** The Web4 proposal separates cognition, broker, participant and evidence boundaries. Mycelix AGENT-003 and AGENT-010 remain important runtime-assurance/independent-verifier gates in the architecture record. Exact application routes, tools, tenants and downstream participants still need adversarial tests. |
| **L6 Presentation** | Canonical encoding, serialization, parsing, schema/version negotiation, signatures, encryption, compression, content typing, data classification and transformation lineage | Xenia wire/presentation formats and sealed envelopes; Symthaea typed representations; WAEP information-flow reporting | **Strong protocol-specific work; coverage must be demonstrated per codec and adapter.** Xenia documents canonical transcripts, typed envelopes, crypto profiles and size limits. Remaining tests include parser differentials, canonicalization ambiguity, malformed/deep inputs, decompression/resource exhaustion, schema drift, semantic data laundering and authorized declassification. |
| **L5 Session** | Peer authentication, session establishment, transcript binding, key schedules, replay protection, rekey, expiry, revocation and immutable negotiated context | Xenia handshake/session and operator-security machinery | **Substantial implementation described; independent review and host/key-custody boundaries remain.** Xenia documents hybrid handshake, authenticated negotiated context, replay state and rekey epochs. Its operator threat model excludes a compromised daemon host and a compromised live operator browser; these exclusions must remain visible. Signer delegation, browser key custody, recovery and compromise/revocation campaigns remain qualification work. |
| **L4 Transport** | TCP/UDP/QUIC/TLS carriers, framing, stream semantics, connection/close deadlines, backpressure, timeout behavior, parser ceilings and transport DoS | Xenia transport adapters and authenticated transport/availability profiles; Nixward host/network policy | **Partially implemented and versioned; carrier/deployment assumptions remain.** Xenia documents exact carrier/framing profiles, envelope ceilings, send/receive/close deadlines and availability semantics. A secure QUIC/TLS carrier does not authorize an application action. Connection establishment, kernel buffering, bandwidth fairness, carrier DoS and distributed availability must not be inferred from the application profile. |
| **L3 Network** | IP addressing, routing, NAT, IPv4/IPv6, DNS, segmentation, egress, east-west reachability, network firewalls and path selection | Nixward host/network policy; vendor adapters and customer network controls; Xenia deliberately consumes carrier-provided routing | **Explicitly outside Xenia; an end-to-end Luminous deployment baseline is not yet evidenced.** Xenia says it does not own IP routing, addressing, NAT or path selection. Nixward's documented future work includes process/network enforcement using Xenia identities and capabilities. Host, LAN, cloud and OT policies need exact-target evidence. |
| **L2 Data link** | Ethernet/Wi-Fi/Bluetooth link framing, MAC identity, VLANs, switch/port security, wireless admission, link encryption and local broadcast-domain controls | Device/OEM/network-provider controls; vendor adapters and exact hardware/network profiles; Nixward deployment composition where supported | **No unified Luminous-owned link-security profile is evidenced.** Xenia intentionally leaves Ethernet/Wi-Fi/link framing and MAC behavior to lower layers. Deployment profiles must specify link owner, supported controls (e.g. 802.1X, WPA3-Enterprise, MACsec or vendor equivalent where applicable), VLAN/port policy, rogue-AP/rogue-device handling and actual test evidence. |
| **L1 Physical** | Device custody, physical access, tamper evidence, hardware identity, firmware, boot chain, hardware roots of trust, secure/measured boot, side-channel assumptions, power and environmental conditions | OEM/hardware owner; Platform #23 firmware lifecycle; #300 edge sensor profile; #349 device family; Nixward boot/recovery; Symtropy physical models; DWF #358 device/executor evidence | **Planned across several owners; exact-device qualification remains essential.** Sovereign Boot's boot animation/state/recovery function is not by itself secure boot, measured boot or hardware attestation. Nixward boot trust requires post-boot health evidence, but does not qualify every hardware root of trust. Each board/model/revision/firmware/update/recovery path needs its own profile and independent appraisal. |

### 3.1 Layer-by-layer interpretation

**L1 — Physical is not just the bootloader.** It includes the actual device, firmware, sensors, interfaces, physical access and recovery. A TPM/secure-element/TEE measurement may be useful evidence, but a signed quote is meaningful only under an identified measurement chain, appraisal policy, freshness rule and relying-party policy. Device-family work (#349), vendor firmware lifecycle (#23), edge instrumentation (#300) and DWF result assurance (#358) should compose here.

**L2 — Link security belongs to the deployed network.** Xenia should not become a Wi-Fi stack or switch OS. But “not owned by Xenia” cannot mean “not in the threat model”. A site profile must identify the AP/switch/operator enforcing link admission, segmentation and link protection. In an unmanaged/public network, treat the link as hostile and rely on authenticated end-to-end protection without assuming local broadcast isolation.

**L3 — Network security is a policy and deployment contract.** The baseline must state which destinations a workload can reach, how IPv4 and IPv6 are handled, how DNS is protected, where segmentation is enforced and which control-plane paths are separate. A host firewall rule is not a complete network-segmentation claim if another interface, tunnel, namespace, route or IPv6 path bypasses it.

**L4 — Transport is not final authority.** TLS/QUIC can protect a channel and TCP can provide reliable ordered bytes; none says the authenticated peer may change a file, send a payment or actuate a physical process. Xenia's authenticated TransportProfile and availability profiles bind transport behavior to the session rather than leaving it as an ambient assumption.

**L5 — Session is not identity governance.** Xenia can establish a cryptographically authenticated session. Mycelix's subject/principal/workload relationships and authority evidence answer different questions: who is accountable, which workload is acting on whose behalf, and what it can do. Session revocation and authority revocation need explicit propagation and currentness tests.

**L6 — Integrity is not semantic safety.** A signature over an encoded object does not establish that an encoding is unambiguous, that a parser interprets it identically, that an output has not leaked a protected fact, or that a data label survived a transformation. WAEP's source-to-sink, lineage and declassification records report these claims without replacing the actual enforcement mechanism.

**L7 — The application effect is the decisive boundary.** A model proposes; a broker evaluates; a participant independently authenticates, authorizes, deduplicates and records the operation; an observer verifies a postcondition. A secure session, A2A Agent Card, MCP tool listing, OAuth token, ACS hook or model-generated ALLOW is not, by itself, permission for a participant effect.

### 4.1 X.805 security layers\n\nX.805 also defines three *security layers*. They are separate from both the seven OSI layers and the three X.805 planes:\n\n| X.805 security layer | Scope | Luminous interpretation |\n|---|---|---|\n| **Infrastructure** | Underlying network infrastructure and facilities | Hardware, firmware, boot, link, routing, transport infrastructure and the infrastructure dependencies that carry Luminous services |\n| **Services** | Network and platform services that support communication and access | Identity/session services, name resolution, transport/security services, brokers, service discovery, policy distribution and shared control services |\n| **Applications** | Applications and end-user information | Symthaea workloads, Mycelix applications, Xenia operator/application behavior, business workflows, data use and physical-operation requests |\n\nThese mappings are interpretive, not one-to-one. One Luminous component may cross multiple X.805 security layers, and one OSI protocol layer may support more than one X.805 layer.\n\n### 4.2 X.805 security dimensions\n\nXAP should map claims to all eight X.805 dimensions. The earlier focus on authorization and evidence is necessary but not sufficient:\n\n| X.805 dimension | Luminous questions |\n|---|---|\n| Access control | Is every principal/device/workload permitted to access this exact resource and operation? |\n| Authentication | Is the claimed human, organization, device, peer or service identity properly bound and current? |\n| Non-repudiation | Are intent, approval, execution and participant evidence bound to identifiable signers and exact artifacts, with key-custody caveats? |\n| Data confidentiality | Can unauthorized parties read payloads, secrets, metadata or derived information? |\n| Communication security | Can information be diverted, intercepted, replayed or delivered to an unauthorized endpoint? |\n| Data integrity | Are messages, configurations, artifacts, state transitions and evidence protected against undetected modification or ambiguity? |\n| Availability | Can the system preserve safe local operation while containing resource-exhaustion and denial-of-service conditions? |\n| Privacy | Are collection, inference, retention, sharing, linkage and declassification purpose-bound and rights-aware? |\n\nNon-repudiation is not a claim that a human cannot dispute an event: it is a claim about evidence under stated key custody, identity, context and verification assumptions. Availability is not permission to bypass authorization. Privacy is broader than payload encryption.\n## 4. The X.805 planes: an orthogonal view

X.805's three planes should be mapped across all seven OSI layers and its three security layers. They are not three more OSI layers.

| Plane | Meaning | Luminous examples | Required isolation property |
|---|---|---|---|
| **Management plane** | Configuration, provisioning, enrollment, maintenance, administration, key/firmware rotation and policy changes | Nixward host administration; vendor fleet management; Xenia operator console; Mycelix governance/key administration; CI/release administration | A management credential or compromised console cannot silently become unrestricted end-user or participant authority. Admin actions are independently authenticated, scoped, logged, revocable and recoverable. |
| **Control plane** | Signaling, routing, admission, discovery, policy coordination, negotiation and orchestration | Xenia session negotiation; network routing/control protocols; Mycelix discovery/coordination and local leases; Platform broker/lifecycle coordination | Control-plane state is not itself effect permission. Control-plane failure or partition must not invent authority, widen scope, double-commit or disable unrelated safe local operation. |
| **End-user / data plane** | User payloads, application data, remote control, workloads and effects | Xenia screen/input/file/session data; Symthaea tools and retrieved knowledge; Mycelix application transactions; Symtropy physical outputs | Data confidentiality, integrity, purpose limitation and exact effect authorization hold even when a control or management component is compromised. |

High-value adversarial scenarios include **control-plane compromise without management-plane compromise**, the inverse, and **valid management authentication but insufficient end-user authority**.

## 5. Luminous Cross-Layer Assurance Plane (XAP)

XAP is the proposed Luminous cross-layer assurance overlay. It spans the OSI layers and the three X.805 planes; it is **not a fourth X.805 plane**, an eighth OSI layer or another protocol stack. It composes existing authorities; it does not create a new one.

| XAP responsibility | Question answered | Existing owner(s) | Evidence expected |
|---|---|---|---|
| **Intent and accountability** | Who requested the action, for what purpose, and who remains accountable? | Human/org principal and Mycelix governance; Symthaea intent proposal | Principal/intent references; purpose; consent/approval; provenance and claim ceiling |
| **Identity and currentness** | Which human, organization, device, workload and running instance are involved? | Mycelix identity/credentials; Xenia peer/session identity; Nixward device/host facts | Issuer, subject, audience, runtime/device binding, validity, revocation and appraisal evidence |
| **Authority and policy** | Is this exact action currently permitted for this actor, target, request, tenant, budget and interval? | Mycelix authority; Nixward host authorization; Platform broker/participant contracts | Exact grant, constraints, policy revision, attenuation path, expiry, one-use attempt binding |
| **Enforcement** | Where is the rule enforced so an untrusted proposal cannot bypass it? | Xenia adapters; Platform broker; Nixward; destination participant; local physical safety controller | Boundary identity, route inventory, failure disposition, negative controls and bypass attempts |
| **Information flow** | Which data moved, how was it transformed, and where could it be disclosed? | Xenia encoding; Symthaea memory/tool boundaries; Mycelix data policy; WAEP claim reporting | Source labels, transformation lineage, sink policy, declassification authority, leakage tests |
| **Observation and evidence** | What independently happened, and what are the shared failure dependencies? | Participant-owned receipts; ITOPS/OMSI evaluators; Sol-Atlas provenance; independent observer | Exact artifacts/digests, source/control-domain graph, freshness, observer method, uncertainty |
| **Lifecycle and recovery** | What happens on expiry, revocation, compromise, partition, repair, user stranding or uncertain effects? | Mycelix lifecycle/governance; Nixward transaction recovery; Xenia revocation; Symtropy local safety; human operator | Termination/descendant census, revocation propagation, same-attempt reconciliation, safe alternative, bounded recovery, residual harm |

### 5.1 Cross-layer invariants

1. **No implicit trust:** network location, device ownership, successful handshake, authenticated transport, valid signature, valid schema, reputation, consensus or model output never grants permission by itself.
2. **Identity is not authority:** authentication is a prerequisite, not an authorization decision.
3. **Secure channel is not secure operation:** the peer and channel profile must bind to the exact application request, but the participant still makes its own authorization decision.
4. **Proposal is not a permit:** cognitive outputs, A2A/MCP metadata and policy-hook dispositions are inputs to the canonical path, not grants.
5. **Broker is not participant:** broker or workflow compromise must not bypass the destination's final effect boundary.
6. **Signature is not truth:** signatures and transparency receipts establish cryptographic relationships to bytes and logs; truth, completeness and correctness require other evidence.
7. **No silent data downgrades:** classification is preserved or made more restrictive unless a specific declassification authority approves the transformation.
8. **Unknown is not allow:** missing, stale, conflicting or unverifiable hard-policy facts lead to deny/indeterminate and reconciliation, not a new mutation.
9. **No retry after uncertain effect:** reconcile the same attempt with participant-owned evidence before a new attempt; compensation is a separate authorized operation.
10. **Availability is not permission:** host boot and safe local baseline may continue without the Luminous control plane; consequential actions still fail closed.
11. **No layer substitution:** a control in one layer cannot compensate for a missing control in another unless the threat model and evidence explicitly justify it.
12. **No false independence:** two receipts or sources sharing a firmware signer, host, KMS, clock, collector, build pipeline or network dependency are not automatically independent.

### 5.2 Assurance graph, not a checklist\n\nA layer checklist is useful for ownership but insufficient for compositional security. The same unauthorized effect may traverse several valid-looking components, and independent-looking evidence may share one hidden dependency. XAP therefore uses an **assurance graph** as its primary analysis model while retaining the layer/plane/dimension matrix as the coverage index.\n\nGraph node types include principal, workload, device, firmware artifact, policy, authority grant, enforcement point, network path, data object, transformation, sink, participant effect, observer, evidence artifact, recovery action and shared dependency. Edge types include authenticates, delegates, constrains, routes, enforces, reads, transforms, discloses, commits, observes, attests, depends_on, revokes and recovers.\n\nThe graph must be able to answer:\n\n- Is there any path from an untrusted ingress or compromised workload to a consequential sink that bypasses an enforcement point?\n- Does every delegated path preserve attenuation back to a root grant and its currentness/revocation state?\n- Does each information-flow path preserve data classification and carry an explicit declassification edge when labels are lowered?\n- Are the purportedly independent participant, observer, signer, clock, build and evidence paths independent under the declared threat model?\n- Can an ambiguous effect be reconciled to the same participant attempt without replaying a second mutation?\n- Which nodes, edges, claims and deployments become unqualified after a shared signer, firmware, host, KMS, DNS, time source, CI pipeline or collector is compromised?\n\nUse the existing Mycelix dependency-hypergraph and Platform/ITOPS evidence work for graph representation and common-cause semantics. XAP must not create a second dependency graph engine or a new global transaction ledger. The graph is a versioned view over existing owners' evidence.\n\nGraph completeness is a claim with a finite scope. Unknown nodes/edges, unenumerated routes or stale inventories must yield UNKNOWN/INCOMPLETE, never an inferred complete path or a false assurance result.\n## 6. Initial threat/interface matrix

This is the minimum first cross-layer corpus. Each case needs positive, negative and ambiguous controls. The evaluator, not the agent, owns hidden expected outcomes.

| ID | Adversarial condition | Expected disposition / invariant |
|---|---|---|
| XAP-01 | Physical device is replaced, repaired or downgraded but retains an old device/firmware receipt | Suspend affected hardware claims; re-identify and requalify exact board/revision/firmware. |
| XAP-02 | Firmware/image is signed but old, revoked, wrong-model or rollback-vulnerable | Reject under version, model, expiry and rollback policy; signature validity alone is insufficient. |
| XAP-03 | Rogue AP, unauthorized switch port, VLAN escape or untrusted local segment | No implicit trust; reject/contain according to deployed link policy and preserve end-to-end authentication. |
| XAP-04 | IPv6, alternate route, tunnel, DNS or second interface bypasses intended L3 egress restriction | Detect the bypass; complete-mediation claim is unsupported until the route inventory is closed. |
| XAP-05 | Authenticated QUIC/TLS peer attempts an unauthorized application operation | Transport may remain authenticated; application/participant action is denied. |
| XAP-06 | Partial/slow-drip/malformed input exhausts parser or session resources | Enforce byte, CPU, memory and absolute-time bounds; terminate the affected session safely. |
| XAP-07 | Session context is reused across a changed transport profile, tenant or audience | Reject context mismatch/downgrade; require a new authenticated negotiation. |
| XAP-08 | Same semantic object has multiple encodings or parsers interpret a signed message differently | Reject non-canonical or ambiguous encoding; pin exact schema/codec and differential-test parsers. |
| XAP-09 | Confidential source is summarized, paraphrased or relayed through a child agent to an unauthorized sink | Preserve lineage/classification; block unauthorized sink/declassification; record semantic leakage and false-negative limits. |
| XAP-10 | Model/tool output requests host command or broad credential use | Treat content as untrusted data; require explicit typed capability and host/participant authorization. |
| XAP-11 | Broker is compromised or its policy store unavailable | Participant final authorization still holds; unavailable authority denies consequential action. |
| XAP-12 | Valid operator session tries an action outside its role, exact target or current grant | Deny at privileged operation boundary; do not trust UI state or authentication alone. |
| XAP-13 | Root authority is revoked while delegated work is queued or a child agent remains alive | Propagate revocation to descendants/continuations and final participant; no orphaned authority. |
| XAP-14 | Observer, participant and agent share a hidden common dependency | Mark independence indeterminate; mirrored or re-signed observations do not count as independent. |
| XAP-15 | Network/control plane partitions during an external mutation after dispatch | State becomes outcome_unknown; reconcile the same attempt; no blind retry or inferred rollback. |
| XAP-16 | Management-plane credential compromise attempts to authorize end-user data access or physical effect | Plane separation and exact application/participant authorization deny the escalation. |
| XAP-17 | Control plane or Mycelix becomes unavailable | Local safe baseline/boot continues; high-risk remote mutation requires fresh current authority. |
| XAP-18 | Local safety controller rejects an action allowed by model, broker or workflow | Local safety boundary wins; preserve both decisions and record the rejected effect. |
| XAP-19 | Revoked/expired/unknown key, profile, tool manifest or device attestation arrives on an otherwise valid session | Currentness failure is enforced at the final relevant boundary; no downgrade fallback. |
| XAP-20 | Recovery/compensation is attempted after uncertain or irreversible external effect | Remedy requires its own narrow current authority and postcondition; it never erases original effect/history. |

## 7. Benchmark and qualification programme

Do not build a single “Luminous security score”. Report separate properties and hard gates. A system that blocks every operation is not useful; positive-control completion and benign false blocking must be reported alongside attack prevention.

### 7.1 Per-layer scoreboards

| Layer | Core measurements |
|---|---|
| L1 | Exact device/firmware inventory coverage; secure/measured-boot appraisal outcomes; revoked/rollback image rejection; repair-triggered requalification; recovery success and evidence completeness |
| L2 | Unauthorized link admission; rogue AP/device detection; VLAN/port policy bypass; link downgrade; wireless rejoin/recovery; segmentation correctness |
| L3 | Unauthorized reachability; unapproved egress; IPv4/IPv6 parity; DNS manipulation; route/tunnel bypass; east-west segmentation and policy drift |
| L4 | Handshake/connection failure behavior; downgrade/replay; pre-auth parser resource use; slow-drip handling; timeout/close correctness; p50/p95/p99 latency and availability |
| L5 | Authentication correctness; transcript/context mismatch; replay; rekey; session revocation latency; operator-role escalation; key-custody/recovery outcomes |
| L6 | Canonicalization/codec agreement; parser differential rate; schema/version mismatch; resource exhaustion; integrity; source-to-sink lineage; unauthorized declassification and protected-fact leakage |
| L7 | Unauthorized effects; authority scope violations; delegation expansion; prompt/tool injection success; cross-tenant access; participant receipt correctness; user task completion and safe recovery |

### 7.2 Cross-layer scoreboards

- Interface/route/graph coverage and unknown-node/unknown-edge count.
- Reachability of untrusted ingress to privileged sinks without mediation; minimum cut/enforcement-path coverage under declared assumptions.
- Common-cause blast radius: number and criticality of claims/deployments invalidated by one shared dependency compromise.
- Unauthorized effects per declared attempt denominator, with exact participant-owned oracle.
- Revocation propagation and orphaned continuation count.
- Cross-plane escalation and control-plane compromise containment.
- Common-cause dependency discovery, false-independence and indeterminate rates.
- Data leakage and semantic-release false negatives/false positives, calibration and benign false blocking.
- Time to safe containment, recovery completion, human effort and user stranding.
- Local safe-operation continuity under network/control-plane/identity-provider outage.
- Exact replayability, artifact integrity and independent verifier disagreement.
- Performance/energy overhead at each enforcement and observation boundary.

### 7.3 Proposed internal targets

These are **targets, not achieved results or external standards**:

- 100% of declared interfaces/routes identify an owner, enforcement boundary, expected failure disposition, recovery owner and evidence source.
- Zero false authorization/unauthorized effect on the frozen deterministic negative-control corpus.
- 100% expected outcomes on frozen positive, negative and ambiguous controls; publish positive-control acceptance separately.
- Zero stale/revoked credential or device-profile acceptance on designated hard negatives.
- Zero known bypass acceptance for the declared L2/L3 topology; any unenumerated route blocks a complete-mediation claim.
- 100% of unknown/common-cause dependency cases report unknown/indeterminate rather than false independence.
- At least 1,000 deterministic adversarial schedules in fast CI and 100,000 seeded schedules in nightly qualification once the implementation is deterministically driveable. Report distinct scenario families and assumptions; schedule count is not a field-risk estimate.
- Measure propagation and recovery latency rather than asserting instantaneous revocation or perfect availability.
- At least two independent producer/verifier implementations agree before a public profile is promoted beyond draft research status.
- Physical, firmware, LAN, host, emulator, simulation and cloud tests remain separate evidence classes; no evidence-class promotion by inference.

For any “zero observed failures” result, publish the denominator, scenario families, missingness and confidence method. Do not present a finite synthetic zero as proof of zero real-world risk.

## 8. Relationship to existing initiatives

| Existing owner | Compose this profile with | Do not duplicate |
|---|---|---|
| Xenia | OSI security plane; transport/availability profiles; operator security model; sealed protocol and consent boundaries | TCP/IP/Wi-Fi/PHY, new session stack or another consent ledger |
| Nixward | System transaction architecture; exact plan authorization; host firewall/process policy; boot and recovery | Second host transaction/authorization engine |
| Platform | README, #310, #23, #300, #325, #349, #358, #370, #497 | New device inventory, fleet updater, benchmark framework or effect ledger |
| Mycelix | AGENT identity/runtime/authority/effect chain; local capability lease #1149; inter-sovereign safe-state #4528; provider adoption modes #5982 | New global identity provider, firewall or sole real-time enforcement dependency |
| Symthaea | INTX proposal semantics; cognitive isolation; memory/tool/data poisoning and prompt-injection tests | Model-as-policy or model-as-authorization |
| Symtropy | Physical/digital-twin distinction; local safety and physical postconditions | Simulation presented as measured actuation proof |
| Sol-Atlas | Knowledge provenance, rights, source/time/geospatial context | Knowledge provenance presented as authentication or physical truth |
| ITOPS/OMSI/WAEP | Existing exact-head evidence, common-cause and claim-to-effect benchmark primitives | A second scalar security score or second evidence ledger |

The Web4 reference architecture in PR #494 remains the higher-level composition proposal. WAEP in PR #496 is the claim/evidence reporting layer. This profile supplies the OSI/X.805 deployment mapping and qualification plan. The three are complementary, not competing architectures.

## 9. Immediate execution sequence

1. **Correct platform headline terminology:** distinguish boot-independent host availability from fail-closed consequential actuation. Existing issue #310 remains the owner of broader fixtures and acceptance.
2. **Freeze the ownership matrix:** every layer and every deployment-specific external owner must be named; “not owned by Xenia” cannot mean “no security requirement”.
3. **Start with L2/L3:** capture one exact NixOS host + managed switch/AP topology; document IPv4 and IPv6, DNS, firewall, egress, VLAN/port, Wi-Fi admission and segmentation. Use read-only capture first. No network mutation until exact target, pre-state, authorization and recovery are bound.
4. **Start one exact-device L1 profile:** use an owned/lab board and exact revision; record firmware source/digest, boot chain, update/rollback, hardware identity, radio/region and physical recovery. Do not infer support from the chipset family.
5. **Requalify Xenia L4–L7 interfaces:** use current Xenia profiles as the starting contract; add missing producer-queue, clipboard/file-command, signer-custody, parser-differential and application-sink tests.
6. **Compose Mycelix final authority/effect chain:** keep AGENT-003/AGENT-010 and independent observer/participant boundaries as explicit gates.
7. **Add cross-layer adversarial campaigns:** inject failures across layers and planes (e.g. firmware update + identity outage + network partition + delayed participant receipt), then verify safety, authority and evidence claims remain separated.
8. **Publish only exact-head evidence:** implemented, structurally checked, CI-executed, lab-exercised, physical, independently reviewed and externally reproduced remain distinct statuses.

## 10. Current claim ceiling

This profile is a research architecture and qualification plan. It is not an audit, certification, adopted standard, proof of production security, or finding that a particular deployment is compromised. Repository sources document meaningful controls and plans, but several ownership gaps and end-to-end qualification gates remain open. Each deployment needs exact hardware, firmware, network topology, policy, runtime, participants, observers and recovery evidence.

**Decision:** keep OSI at seven layers; adopt XAP as the Luminous cross-layer assurance plane; use X.800/X.805 as the established network-security architecture baseline; qualify each layer and the cross-layer compositions independently.
