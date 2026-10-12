# Reference Site Network Segmentation Profile v1

**Status:** Reference control contract; not a customer-approved network configuration  
**Scope:** small government/regulated office and enterprise administration  
**Related:** [Government network RFC](government-network-platform-rfc-001.md), [small-site functional BOM](../hardware/profiles/small-site-reference-bom.md), [catalog issue #19](https://github.com/Luminous-Dynamics/luminous-platform/issues/19)

## 1. Design rule

Treat this as a logical reference design, not a substitute for a customer network survey. Each zone may map to VLANs, VRFs, firewall interfaces, physical interfaces, or a combination. A VLAN is not, by itself, a security boundary. Where sensitive management or key operations justify it, add physical separation and independently controlled access.

**Default policy: deny inter-zone traffic unless a specific flow is recorded, approved, observable, and tested.** Apply the same intention to IPv4 and IPv6, including link-local/multicast and any permitted transition/tunnel mechanisms. Never leave IPv6 or out-of-band interfaces outside the policy inventory.

## 2. Reference zones

| Zone ID | Purpose | Example members | Trust expectations |
|---|---|---|---|
| WAN | ISP and externally routed links | Primary and backup CPE, provider handoff | Untrusted. No inbound management; egress is controlled and logged. |
| REMOTE-ACCESS | Authenticated remote-user entry | Approved VPN or zero-trust access gateway | Entry point only. Successful connection does not grant access to every internal zone. |
| USER | Managed employee workstations | NixOS/other approved clients, printers only if treated as managed user devices | Untrusted relative to servers and management; per-application access. |
| GUEST | Visitor / personal-device access | Guest Wi-Fi and visitor ports | Internet access only; no access to internal addresses, internal DNS zones, management, or peer devices. |
| APP | Application front ends and approved shared apps | Web/API gateways, collaboration endpoints | Reachable only from explicitly permitted client/service sources. |
| DATA | Databases and persistent application stores | PostgreSQL, object/file services | No direct user-subnet access; only named application/service identities and approved administration. |
| IDENTITY | Identity, certificate and access-policy services | Directory/IdP integration, PKI components, MFA/revocation integration | High-value zone. Narrow admin access; changes and key operations are audited. |
| OPS-MGMT | Host/network management | Nixward control, configuration automation, admin jump host, switch/AP management | Reachable only by named privileged identities from approved management endpoints. Not user-accessible. |
| OOB | Out-of-band server/device management | BMC, serial/KVM, recovery controllers | Isolated from WAN, guest, user, app and normal server traffic. Only approved recovery/administration path may reach it. |
| LOG-MON | Monitoring and audit collection | Central logs, metrics, event receiver, alerting | Producers send only to documented collector endpoints; collector administration is separate. No broad query access for all clients. |
| BACKUP | Backup orchestration and repositories | Backup controller, snapshots, offline/removable target staging | Separate administrative identity/failure domain; production hosts should not have unrestricted delete/retention authority. |
| UPDATE-CACHE | Signed software/firmware artifact mirror | Nix cache, package mirrors, approved firmware/config bundles | Clients retrieve approved artifacts. Upstream refresh is a separate controlled egress path. |
| FACILITY-IOT | Facility and environmental devices | Temperature/leak/power sensors, printers or other low-trust peripherals if appropriate | No direct reachability to user endpoints, identity, management, or data services. Only named broker/collector access. |
| SITE-LINK | Authenticated links between approved offices | Site-to-site gateway endpoints | Route only the explicitly approved prefixes and service flows. Site membership alone is not authorization. |

Merge or split these zones only after documenting why the smaller zone boundary does not add value for the risk, operational capability, or cost. Keep OPS-MGMT and OOB distinct in the design, even when the pilot lacks enough hardware for separate physical fabrics.

## 3. Minimum permitted-flow matrix

All unspecified flows are denied. The table is a starting allow-list; required protocol/port, direction, identity, source/destination CIDR, and logging fields must be filled in for the selected products. Do not infer ports from product names or open broad subnet access where service-level rules are possible.

| Source | Destination | Allowed purpose | Constraints |
|---|---|---|---|
| Managed endpoint / approved infrastructure | Designated DNS, DHCP and time service | Network bootstrap and time | Limit to the site's designated servers; document DHCP relay and IPv6 router-advertisement behavior. |
| USER / approved remote user | APP | Use approved business applications | Prefer named application entry points; no general USER → DATA or OPS-MGMT access. |
| APP service identity | DATA | Application transactions | Exact app identity/database role, destination, operation and network path; no broad data-zone trust. |
| Managed device/service | IDENTITY | Authentication, certificate enrolment/validation, revocation and required policy checks | Explicit protocols; distinguish validation from privileged enrollment; keep issuance/key-management administration restricted. |
| Approved admin endpoint → privileged access point | OPS-MGMT | Administrative maintenance | MFA, least privilege, session/evidence logging, current device posture where available, time-bounded authorization and revocation. |
| Approved management control → network/host targets | Managed device administration interfaces | Configuration and lifecycle actions | Allow only required interfaces to exact management addresses; no inbound management from WAN/guest/user zones. |
| Recovery operator path | OOB | Recover a failed host or network device | Explicit break-glass role, physically controlled path, durable audit where available, and rehearsed recovery. No public Internet exposure. |
| Workload/endpoint agents | LOG-MON | Logs, metrics, health and security events | Prefer outbound delivery to named collectors; bound payload size/retention; redact secrets and untrusted raw error text. |
| Backup controller | Approved source services | Read/consistent snapshot of protected data | Backup identity is read-scoped where possible; use application-consistent snapshot hooks when needed. |
| Backup controller | BACKUP repository | Write protected backup and retention metadata | Separate credentials/failure domain; immutable retention or write protection where appropriate; restore access tested independently. |
| Managed clients | UPDATE-CACHE | Retrieve pinned/signed packages and firmware bundles | Clients cannot modify artifact cache; exact trust roots and update source policy must be documented. |
| Approved artifact refresh service | Approved upstream repositories | Refresh packages/firmware | Controlled egress proxy and allow-list; signatures and digests verified; unapproved direct client egress denied where feasible. |
| GUEST | Approved external services | Internet-only visitor access | Explicit guest DNS; block all internal IPv4 and IPv6 ranges, local management and peer-to-peer access. |
| FACILITY-IOT | Named broker/collector | Send telemetry and receive required configuration | No direct trust in device-claimed identity without validation; deny arbitrary Internet and internal lateral movement. |
| REMOTE-ACCESS | Explicit application/administration targets | Remote work and on-call maintenance | MFA and authorization by person/device/role/resource; distinct roles for ordinary work and privileged administration. |
| SITE-LINK | Explicit target subnets/services at peer site | Cross-site application or shared service access | Explicit routing and service allow-list, peer authentication, revocation, and tested partition/failover behavior. |

### Explicitly forbidden default flows

- WAN → any device management interface, BMC, database, identity administration or operator console.
- Guest → user, app administration, data, identity administration, management, OOB or backup zones.
- User → database/storage control plane, hypervisor/BMC, switch/AP administration, or Nixward actuation interface.
- Production workload → arbitrary management interface or unrestricted backup deletion/retention API.
- A site → every other site merely because a tunnel exists.
- Any zone → any destination simply because a logger, monitoring tool or AI agent requests it.
- IPv4 allowed while equivalent IPv6 traffic is unfiltered or untested.

Any exception needs a documented owner, purpose, exact endpoints, protocol/port, identity control, expiry/review date, logging, negative test and rollback method.

## 4. Required machine-readable flow records

Before generating device configuration, maintain each permitted flow with at least:

- flow_id and schema version
- source zone / exact source identity or managed selector
- destination zone / exact destination identity or service selector
- protocol and destination port/service
- direction and connection-initiation semantics
- business/security purpose and owning team
- authentication and authorization requirements
- whether the flow is mandatory for boot/recovery or optional
- evidence and test IDs
- approval/expiry/review information
- behavior when service, DNS, identity, time or policy controller is unavailable

The generator must reject unknown zone IDs, missing owner/justification, unrestricted "any" endpoints, duplicate identifiers, illegal zone references and stale approvals. It must not silently broaden the policy when a capability is unsupported; fail closed for authorization and preserve a usable recovery path.

## 5. Addressing and naming worksheet

Do not hard-code example networks; use placeholders to fill after an address-conflict review.

| Item | Required decision |
|---|---|
| Site ID and assigned prefixes | Record approved IPv4 and IPv6 allocations; check for mergers, VPN routes and private-cloud overlap. |
| Zone/subnet allocation | Allocate unique subnets per zone as needed; avoid overlap between office, cloud, partner and remote-worker paths. |
| DHCP and static addressing | Reserve stable addresses for gateways, switches, APs, servers, collectors, management, printers and other infrastructure. |
| DNS split and resolver policy | Name internal zones, forwarding policy, DNSSEC expectations where applicable, logging and outage behavior. |
| Time hierarchy | Designate stable local/approved time sources; define drift thresholds and behavior when the upstream is unreachable. |
| Certificate naming | Identify authority, issuance, renewal, revocation and service identities. Avoid certificate wildcards that create unnecessary broad trust. |
| IPv6 | Choose whether to deploy it in the pilot; if enabled, make filtering, router advertisements, DHCPv6, DNS and test coverage explicit. If disabled temporarily, document controls and re-evaluation date rather than assuming it is absent. |
| Management resolution | Ensure management-plane DNS/identity dependencies cannot create an untestable circular dependency during recovery. |

## 6. Test and evidence plan

Run on an isolated test configuration. Each row becomes an automated scenario with source/destination identity, exact config revision, timestamp, method, expected result, observed result, raw evidence reference and verdict.

| Test ID | Scenario | Required property |
|---|---|---|
| SEG-01 | Enumerate all zones and effective routes | Every interface/SSID/VRF, IPv4/IPv6 prefix and management path has an owner and zone. |
| SEG-02 | Positive application flow | Intended user-to-app flow succeeds only through approved entry point. |
| SEG-03 | User-to-database direct path | Direct connection is denied even while approved app workflow remains healthy. |
| SEG-04 | Guest-to-internal sweep | Guest cannot reach internal services, private ranges, management or other clients. |
| SEG-05 | WAN management probes | Router, switch, AP, host management and BMC surfaces are not exposed to WAN. |
| SEG-06 | IPv6 parity | Forbidden flows remain blocked on IPv6; no router-advertisement or link-local assumption bypasses controls. |
| SEG-07 | Privileged administration | Non-admin identities cannot access management, and admin access requires current authorization. |
| SEG-08 | Revocation and stale approval | Revoked identity or expired authority cannot continue to initiate privileged changes. |
| SEG-09 | OOB isolation | BMC/KVM/serial access is reachable only through designated recovery/admin path. |
| SEG-10 | Backup isolation | Production identities cannot destroy protected recovery copies or change retention policy. |
| SEG-11 | Logging outage | Business/network security does not silently widen permissions because central logging is unavailable; queueing and degraded-mode policy match requirements. |
| SEG-12 | Identity/DNS/time outage | Protected operations deny safely; recovery and approved local bootstrap remain possible without ambient privilege. |
| SEG-13 | Site link failure/replay | Lost link, stale peer trust and revoked device/certificate do not create a permissive fallback. |
| SEG-14 | Config update/rollback | Proposed rule set is diffed and tested; failed deployment rolls back to known policy, with receipt and exact pre-state. |
| SEG-15 | Default-deny completeness | Unlisted source/destination pairs, ports, protocol families and management interfaces are denied. |

A simulation is useful for logic but cannot replace testing against the selected switch, AP, gateway, operating systems and firmware. Do not mark these tests passing until evidence from the exact selected hardware/configuration exists.

## 7. Operational ownership and change lifecycle

- Network policy changes are version-controlled and reviewed; emergency changes expire and are reconciled after the incident.
- Device inventory binds asset identity to exact model/revision, firmware, location, owner, support end date and recovery method.
- Firmware/update windows include staged rollout, canary device, signature/digest verification, rollback/recovery and post-deployment behavior evidence.
- Exceptions have expiry and reviewer; no indefinite undocumented permit rules.
- Management identity, service identity and human privilege are separate; no shared admin credential across gateways, switches, hosts and backup.
- Backups include network configurations, identity/certificate metadata as policy permits, and recovery documentation, but no private keys in Git or ordinary logs.
- Operations consoles and Symthaea may suggest configuration changes; only the designated policy authority can approve and only a bounded executor can apply them.
- Maintain offline recovery media, tested access to a console, spares and a known-good configuration independently of the normal production network.

## 8. Not yet selected or qualified

This profile does not identify actual IP ranges, ports, device SKUs, approved cryptographic modules, provider or carrier contracts, or customer-specific control mappings. Those are requirements inputs for the exact pilot. The matrix is a safe starting allow-list, not a deployed firewall configuration or authorization to process protected information.
