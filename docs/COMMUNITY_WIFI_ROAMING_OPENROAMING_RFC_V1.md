# Community Wi-Fi Roaming RFC v1: Passpoint and OpenRoaming

**Status:** draft research only. Not a federation membership claim, deployment approval or measured product.  
**Issue:** https://github.com/Luminous-Dynamics/luminous-platform/issues/65  
**Related work:** provider-neutral connectivity broker, PR #64; open community connectivity and replaceable-WAN architecture, PR #62.  
**Research checked:** 18 October 2026.

## 1. Thesis

Do not invent another Wi-Fi roaming federation. Integrate with existing standards and federations where the operator, identity provider, device and site are eligible. The potential user-level change is simple: arrive at a participating site and obtain securely authenticated connectivity without repeatedly selecting an SSID, accepting a portal, and re-entering credentials. The technical and commercial change is harder: federation policy, credentials, trust anchors, RADIUS/AAA configuration, device capability and upstream terms must all align.

The Wireless Broadband Alliance describes OpenRoaming as a federation using public-key infrastructure, federation policy, network automation and Passpoint technology. An open-source portal or controller is useful tooling; by itself it does not enroll a network into the federation, grant roaming identity, provide a credential profile or authorize sharing an upstream service.

WBA sources:
- OpenRoaming, how it works: https://wballiance.com/openroaming/how-it-works/
- OpenRoaming open-source portal: https://github.com/wireless-broadband-alliance/wba-openroaming-portal
- WBA Access Network Metrics report / research archive: https://wballiance.com/resources/wba-white-papers/

WBA reports production-scale Access Network Metrics trials across 17,000+ access points and 350,000+ daily active users. This is ecosystem precedent for measuring Wi-Fi performance, not evidence that this project has federation access, credentials, comparable scale or results.

## 2. Boundaries: what the system is and is not

Keep these different events distinct:

- **AP association:** the radio client associates to an access point.
- **802.1X / AAA authentication:** a credential is checked through the relevant authentication system and trust path.
- **Passpoint roaming:** a compatible configured client discovers and selects a suitable roaming network and authenticates under a supported profile.
- **OpenRoaming federation:** a network and identity provider participate under federation rules/agreements.
- **Captive portal:** a web flow that may provide onboarding or fallback access; it is not proof of secure automatic roaming.
- **Multi-WAN failover:** an IP gateway changes upstream paths; it does not authenticate a client to a new Wi-Fi network.
- **Cellular/eSIM switching:** an operator, device OS, modem and profile entitlement problem, not supplied by OpenRoaming.
- **Internet service:** a working authorized upstream and DNS/routing path after authentication.

The policy broker can rank a Wi-Fi candidate only after an adapter supplies honest metadata about compatibility, authentication state, cost, authorization, freshness and quality. It cannot make an unsupported phone or un-enrolled AP roam. It must not infer task intent by inspecting content.

## 3. Reference topology

```text
Compatible client + installed roaming credential/profile
                |
        Passpoint-capable AP(s)
                |
      802.1X / EAP authentication
                |
       Local or managed AAA/RADIUS
                |
       Identity-provider / roaming trust
                |
      OpenRoaming federation, if enrolled
                |
       Authorized IP backhaul / gateway
                |
    Provider-neutral connectivity broker
         |                   |
  public Internet       local-only services
                            |
                  local status, cache,
                  service directory, queue
```

Operational monitoring observes health and counters without capturing payloads. Configuration management and secrets distribution are separate control planes from packet forwarding. The local-only service plane must remain useful when WAN, federation, identity provider or monitoring is unavailable, provided the LAN and power remain available.

### Components and boundaries

| Component | Responsibility | Must not be treated as |
|---|---|---|
| Passpoint-capable AP | Advertise supported network information, isolate clients and forward 802.1X/RADIUS traffic | Proof of federation enrollment |
| AAA/RADIUS | Validate the expected EAP method and server-side authentication policy | A captive-portal web form |
| Identity provider | Issue/maintain a supported identity or credential profile | A universal identifier reusable on every network |
| Federation relationship | Policy, trust, interconnection, partner eligibility and any settlement | An open-source software install |
| Gateway / multi-WAN | IP routing, cost cap, failover and local service reachability | Wi-Fi client roaming |
| Local service node | Cached public resources, local directory/status and bounded store-and-forward | Global Internet or emergency telephony |
| Telemetry / evidence | Aggregate success, latency, failures, configuration revisions and artifact hashes | Packet capture or browsing history |
| Cooperative layer | Community decisions, budgets, subsidy rules and transparent cost reporting | A required dependency for EAP or packet forwarding |

OpenWISP is one candidate for management/monitoring of OpenWrt devices. Evaluate the current supported versions, deployment requirements and maintenance burden before selecting it: https://openwisp.io/docs/

## 4. Security and privacy invariants

The first release should enforce these boundaries by design and by negative tests:

1. **Fail closed on trust failures.** Invalid, expired, wrong-name or otherwise untrusted authentication-server certificates must not be ignored to make a device connect. An unsupported EAP method, realm mismatch or untrusted AAA route must not silently downgrade to a weaker mode.
2. **Keep credentials out of telemetry.** No passwords, private keys, EAP inner identity secrets, RADIUS shared secrets, raw packet payloads, message content, browsing histories or subscriber identifiers in public logs or receipts.
3. **Minimize identity exposure.** Prefer the federation/identity system's supported privacy-preserving identity behaviour. Use pseudonymous site and device tokens in test artefacts. Document unavoidable identifiers, access controls, retention and deletion for the live AAA system.
4. **Separate tenants.** Guest/client isolation, management-plane ACLs, rate limits and log access rules must be tested. A shared AP does not imply trust between guest clients.
5. **Make the local alternative explicit.** Keep a documented, authorized local-only service path that remains accessible without remote identity, remote DNS or internet access. Avoid deceptive captive portals and state clearly which services are local-only.
6. **Do not turn off validation for compatibility.** Every unknown client/AP/IdP/EAP combination is marked unknown or unsupported until it passes named tests.
7. **Do not make availability or emergency promises.** Wi-Fi roaming is not a guaranteed replacement for cellular voice, a regulated emergency service or a direct-to-satellite link.

## 5. Versioned benchmarks

The machine-readable profile at `profiles/community-wifi-roaming-benchmark-profile-v1.json` separates proposed acceptance rules from baseline-first performance metrics and externally blocked capabilities. Performance numbers remain baseline-first until device models, credential sources, site layout, federation and measurement endpoints have been named.

### Required security/contract checks

- Invalid/expired/untrusted server certificate: 100% of predeclared negative cases are rejected; zero silent downgrade.
- Incorrect realm or unauthorized AAA target: zero successful authentication and zero credential disclosure in all predeclared negative tests.
- Secrets/payload minimization: zero private keys, passwords, payloads, browsing history or subscriber identifiers in public receipts and monitoring events.
- Evidence traceability: 100% of critical AP/gateway/firmware/AAA configuration artifacts have revision, license/status, digest, update/rollback and recovery notes.
- Local-only continuity: baseline test for services through WAN and federation outage; initial target is 24 continuous hours when LAN and declared power remain available.
- Authorization/federation status: remain blocked/unknown until the named operator, identity provider, credential, AP model and site have verifiable eligibility and permissions.

### Baseline-first performance measures

- **Arrival to useful IP:** time from a predefined arrival trigger to authenticated L3 connectivity plus a successful transaction against a controlled HTTPS probe. Report median, p95/p99, sample size and failures.
- **First-attempt authentication:** successful full 802.1X/IP sessions divided by eligible attempts; report by device/OS/profile/AP/identity provider. Separate client incompatibility from AAA outage and credential rejection.
- **Roam/handoff interruption:** application probe interruption while moving between named APs under a declared active-flow workload. Report p50/p95/p99 and dropped-flow rates. A new association is not necessarily seamless roaming.
- **Recovery:** restoration time after AP loss, AAA loss, IdP/federation unavailability, WAN failure and resolver failure. These are distinct fault domains and must not be blended into one percentage.
- **Economics:** fully burdened cost per successful useful task and per active user-hour; include upstream charges, amortized AP/gateway/controller, identity/federation fees, electricity, site rent, maintenance, support, paid labour and taxes.
- **Energy:** whole-site idle/typical/peak watts and Wh per successful task; include PoE conversion and edge-cache equipment.
- **Interoperability:** exact matrix of client vendor/model, OS/build, Wi-Fi/Passpoint capability, credential source/profile revision, EAP method, AP/firmware and visited network. Each cell is pass/fail/unknown with evidence.
- **User burden:** manual setup steps, support contacts per 100 attempts, reauthentication prompts, time-to-recover and accessibility issues. Count users who could not connect; do not exclude them from denominator.

Performance comparisons must use the same devices, site geometry, task definition and authorization scope. Preserve failures and disclose denominators. An emulator or simulator cannot be reported as a physical roam measurement.

## 6. Negative-control matrix

| ID | Test | Required result |
|---|---|---|
| WIFI-ROAM-CERT-001 | Expired or untrusted authentication-server certificate | Connection rejected; no insecure fallback |
| WIFI-ROAM-REALM-001 | Wrong realm or unauthorized AAA destination | No successful authentication; no secret disclosure |
| WIFI-ROAM-REVOKE-001 | Revoked/disabled test identity | Authentication rejected and audit event minimized |
| WIFI-ROAM-PORTAL-001 | Unexpected captive portal appears during purported automatic roaming | Record as a failure/portal flow, not successful Passpoint roaming |
| WIFI-ROAM-CRED-001 | Unsupported client profile or credential | Explicit unsupported/failure; no downgrade presented as success |
| WIFI-ROAM-AAA-LOSS-001 | AAA unreachable or times out | Bounded recovery; local-only service remains independent |
| WIFI-ROAM-WAN-LOSS-001 | Authentication has succeeded but all WANs are down | Local-only services work; global internet is reported unavailable |
| WIFI-ROAM-IDENTITY-001 | Same user/device visits multiple test sites | Public traces use pseudonyms and omit raw identity/inner identifiers |
| WIFI-ROAM-AP-LOSS-001 | One AP disappears during a controlled active flow | Measure interruption/reassociation; no ungrounded claim of seamless handoff |
| WIFI-ROAM-LOG-001 | Inspect telemetry, debug logs and receipts | Zero secrets, payloads, browsing histories or subscriber identifiers |
| WIFI-ROAM-ISOLATION-001 | One guest client attempts access to another or AP management | Denied by the named network policy |
| WIFI-ROAM-ROLLBACK-001 | Deploy a bad, staged AP/AAA config | Restore known configuration without disabling certificate validation |

Use isolated credentials and test realms. Do not test real customer identity leakage or collect third-party traffic.

## 7. Pilot phases and go/no-go gates

### Phase 0 — capability and permission inventory

Record AP and client models/versions, Passpoint capabilities, current EAP methods, AAA deployment, credential/identity-provider options, candidate federation relationship, upstream topology, hardware type-approval and upstream terms. Ask potential federation/IdP partners what enrollment, certificates, support and settlement are available. Unknown stays unknown.

### Phase 1 — isolated local lab

Use two lab APs advertising one controlled test profile and one local RADIUS/AAA environment. Test certificate validation, client isolation, configuration rollback, local-only access and fault injection with synthetic identities. This phase does not claim OpenRoaming interoperation.

### Phase 2 — authorized federation interoperability

Proceed only when a named identity provider supplies legitimate test credentials/profile and a network operator is eligible to test the federation. Confirm the correct trust/certificate chain, terms, device compatibility, logging requirements and settlement model. Compare the actual federation flow with the local lab baseline.

### Phase 3 — controlled site trial

Use a site with documented permissions, consent/signage, supported devices, a rollback plan and a local fallback. Collect only aggregated/pseudonymous metrics, retain failure denominators and publish the exact configuration/firmware/revision hashes plus limitations.

### Phase 4 — replication and economics

Replicate at a second, independently managed site before making generalized coverage or cost claims. The economics must include sponsorship and subsidies explicitly. “Free for the user” means funded service, not costless transit.

## 8. South African deployment gates

ICASA's published guidance covers electronic communications network/service licence classes, specified exemptions and type approval. It says reselling upstream electronic communications services may require an ECS exemption process; the actual status depends on the specific operating arrangement. Before providing access to the public or sharing/reselling an upstream link, obtain deployment-specific legal/ICASA guidance and written confirmation of upstream provider terms. Verify equipment type approval and radio/spectrum requirements for any custom RF design.

Official guidance:
- ICASA services licensing and exemptions: https://www.icasa.org.za/pages/services-licencing
- ICASA licensing and type approval: https://www.icasa.org.za/pages/licensing
- ICASA spectrum licensing: https://www.icasa.org.za/pages/spectrum-licensing

This design document is not legal advice or permission to operate.

## 9. Responsibilities across the Luminous stack

- **Luminous Platform:** versioned AP/gateway profiles, secret-safe configuration deployment, health metrics, rollback, immutable evidence receipts and operator-facing failure reports.
- **Sol-Atlas:** map demand density, candidate AP positions, building/site restrictions and authorized backhaul; avoid implying predicted coverage is a measured RF survey.
- **Symtropy:** simulate AP loss, load, interference assumptions, backhaul failure and battery/power budgeting; label every simulated result.
- **Symthaea:** advisory optimization for placement/cache/power using declared goals and bounded aggregate telemetry. It must not control EAP policy, bypass certificate validation or become a forwarding dependency.
- **Mycelix:** optional community governance, transparent pool funding and cooperative accountability after the legal operator and settlement model are established. Do not conflate Mycelix identity/reputation with the EAP identity provider.
- **Local services:** continue within the site's trust boundary during WAN/federation outages where LAN and power remain available.

## 10. Exit criteria

This RFC may move beyond draft only when the profile's critical gates are independently evidenced for a named deployment; the lab negative controls are reproducible; real compatible credentials/partners are confirmed for federation claims; a pilot measures the baseline metrics; permission and equipment checks have been completed; and an independent reviewer can reproduce the receipts. An open-source portal installation, green software test or queued CI run does not satisfy these conditions.


## 11. Interoperability matrix template

Use `profiles/community-wifi-roaming-interoperability-matrix-template-v1.json` with its shape contract at `schemas/community-wifi-roaming-interoperability-matrix-v1.schema.json` when building the device/provider matrix. It deliberately starts with zero rows—no handset, OS, credential, AP, RADIUS, identity-provider or federation combination is asserted to work.

Every future row must name its exact claim scope: local Passpoint, OpenRoaming federation, or ordinary Wi-Fi fallback. Rows marked supported or unsupported need a timestamped test receipt and digest. A supported OpenRoaming federation row additionally requires a verified test credential/profile and authorized enrollment evidence. Simulator-only evidence remains simulator-only, and one passing combination never generalizes to another device or site.
