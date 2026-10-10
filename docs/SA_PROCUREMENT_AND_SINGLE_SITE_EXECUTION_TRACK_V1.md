# South African Procurement and Single-Site Execution Track v1

**Reviewed:** 2026-10-10  
**State:** dated market-intelligence and qualification plan; not legal advice, a live bid decision, or a qualified deployment  
**Scope:** non-production, public/synthetic-data lab for secure administrative, public-service, regulated-enterprise and appropriate unclassified IT  
**Related profile:** [South Africa single-site office/lab JSON](../profiles/sa-single-site-office-lab-v1.json)  
**Related control matrix:** [Government-network control evidence matrix](GOVERNMENT_NETWORK_CONTROL_EVIDENCE_MATRIX_V1.md)  
**Portfolio:** [hardware/portfolio-v1.json](../hardware/portfolio-v1.json)

## 1. Immediate legal-status correction

On **17 September 2026**, the Constitutional Court handed down *Premier of the Western Cape Government and Another v Speaker of the National Assembly and Others; City of Cape Town and Others v Speaker of the National Assembly and Others*, cases CCT 103/25 and CCT 144/25, **[2026] ZACC 37**. The Court's published order declares the Public Procurement Act 28 of 2024 invalid in its entirety because of the legislative public-participation process.

This materially changes the legal research track. National Treasury's April 2026 draft procurement regulations were published for consultation under that Act before this judgment. Do not treat the Act or those draft regulations as a currently operative legal basis merely because they remain discoverable on government websites. Conversely, do not infer from the judgment alone which pre-existing provisions, transitional measures, later bills, or institution-specific rules govern a particular procurement. For every live bid, verify the current legal framework against the Court's full order, subsequent official Gazette notices and the procuring institution's written tender documents, and obtain South African procurement-law advice where needed.

**Operational consequence:** tender packs, published current panel lists, qualification rules and written guidance from the procuring institution—not a general assumption about the 2024 Act—must control the go/no-go decision. Keep legal status as an explicit blocker rather than making a compliance claim.

Primary sources:
- Constitutional Court judgment page and order: https://www.concourt.org.za/index.php/judgement/663-premier-of-the-western-cape-government-and-another-v-speaker-of-the-national-assembly-and-others-city-of-cape-town-and-others-v-speaker-of-the-national-assembly-and-others-cct-103-25-and-144-25
- Government confirmation dated 17 September 2026: https://www.gov.za/news/media-statements/minister-enoch-godongwana-notes-and-respects-judgement-constitutional-court-i
- National Treasury consultation page for the draft 2026 regulations (historical consultation material; not proof of current authority): https://www.treasury.gov.za/public%20comments/ProcReg/default.aspx

## 2. SITA market snapshot captured on 10 October 2026

SITA's published pages are useful discovery tools. They do **not** prove that Luminous is eligible to bid, that a panel is accepting new members, or that an RFQ is still open when a later reader opens this file. Re-check the live notice and acquire the actual document pack before acting.

### Effective-panel context

| Published SITA panel | Listed end date | Meaning for Luminous |
|---|---:|---|
| RFA 2494-2021 — Information Security Products and Services | 6 September 2027 | Existing panel context only. Confirm current supplier/channel eligibility and whether any lawful onboarding route exists; an active panel is not an open invitation to join. |
| RFA 2168-2024 — Network cabling, server-room and related infrastructure | 10 October 2031 | Relevant to infrastructure integration/subcontracting research, not proof of qualification or direct access. |
| RFA 2501-2021 — Unified communications/video conferencing | 8 October 2026 | Expired according to SITA's current list; retain as historical evidence only unless the list changes. |

Source: https://www.sita.co.za/content/sita-effective-panel-contracts

### Notices visible in SITA's live RFQ portal

| RFQ | Published scope / customer | Deadline shown | Briefing condition shown | Triage as at 10 Oct 2026 |
|---|---|---:|---|---|
| 6614_2494_2026 | Three-year next-generation firewall, licenses, maintenance and support — DPSA | 13 Oct 2026 | Confirm in the downloaded pack | **Urgent / likely no-go unless** eligible bidder or valid channel, exact support commitments, required licenses and every bid condition can be verified immediately. Do not substitute OpenWrt One for a contract-backed NGFW. |
| 6611_2168_2026 | Three-year LAN cabling and server-room infrastructure maintenance/support — CSPS | 13 Oct 2026 | Confirm in the downloaded pack | Better aligned to a qualified local cabling/infrastructure supplier; Luminous should only participate through an explicitly permitted, competent delivery arrangement. |
| 6615-AH-2026 | Network switches and SFP modules — Military Health Formation | 16 Oct 2026 | Compulsory virtual briefing listed for 7 Oct 2026 | **Blocked unless** the required briefing was attended and the pack confirms eligibility. Deadline alone does not reopen a missed mandatory briefing. |
| 6619_AH_2026 | Network switches and SFP modules — SA Army HQ, Bester Building | 14 Oct 2026 | Compulsory virtual briefing listed for 5 Oct 2026 | **Blocked unless** the required briefing was attended and the tender pack explicitly permits participation. |
| 6621_2168_2026 | LAN infrastructure installation — SA Army HQ, Bester Building | 30 Oct 2026 | Compulsory virtual briefing listed for 16 Oct 2026 at 11:30 | **Watch / conditional lead.** Download and review the pack before the briefing; confirm the permitted supplier route, technical scope and attendance process. The meeting is a future obligation as of this review date, not a completed gate. |

Source: https://rfq.sita.co.za/RFQ/RFQInvitations.asp

The notices are market intelligence, not evidence of procurement eligibility. Record the exact document-pack revision, questions/addenda, briefing attendance evidence, closing timestamp/time zone, supplier status, mandatory forms, subcontracting rules, required local content/preferences, warranty/SLA, and named technical requirements for any opportunity that passes triage. Re-check all deadlines and addenda before a decision.

## 3. Recommended route to market

For the first pilot, prioritize **a software/integration contribution under a demonstrably eligible local supplier or established vendor channel**, rather than assuming Luminous should act as the prime contractor for a network or firewall tender.

1. **Lab and integration proof:** complete an isolated lab against the machine-readable profile; produce a precise tested configuration, evidence bundle, support boundaries and recovery demonstration.
2. **Partner diligence:** identify suppliers that can lawfully provide the hardware, installation, local support and tender-specific warranty. Obtain written confirmation of their role, authority to quote, applicable panel/contract eligibility, and whether subcontracting or named OEM status is permitted.
3. **Commercial split:** quote Luminous-owned software, configuration, hardening and evidence work separately from hardware resale, subscriptions, installation labour and support that another supplier owns.
4. **Tender-specific decision:** bid only after the complete pack and its amendments have been reviewed; satisfy every mandatory briefing and returnable; prove supplier/partner eligibility; confirm legal, tax, insurance, warranty, SLA and local support requirements; and receive a written approval of responsibilities from the intended partner.
5. **Do not imply certification:** an H4 internal lab result would be bounded operational evidence, not an H5 customer authorization, public-sector accreditation, or tender qualification.

For the live DPSA NGFW notice, the named scope includes three years of licenses, maintenance and support. The lab gateway candidate is **not** a substitute for that commercial scope. Treat the NGFW as a separate vendor-supported product family until an exact model, licensing plan, throughput profile, support SLA and eligible supplier path are evidenced.

## 4. Single-site build freeze: what must be obtained before an order

The existing [profile JSON](../profiles/sa-single-site-office-lab-v1.json) is an architecture draft with twelve component records, eight VLAN zones, six flow contracts and blocked/not-run acceptance gates. It is **not** a final purchasable BOM. The next practical artifact should be a quote-backed bill of materials whose row identity matches the purchased units and the tested deployment.

For each proposed item, collect the following in one procurement record:

- Manufacturer, exact orderable SKU, hardware revision and firmware/BIOS baseline.
- South African seller/importer, written quote number/date, currency, VAT and freight treatment, stock status, lead time, warranty, RMA location, repair/spares path and end-of-support date.
- License terms, source/provenance, known closed firmware components, update signer/path, security advisory channel and recovery method.
- Required compatibility evidence (kernel/NixOS, NIC/PHY/radio, switch VLAN/ACL semantics, optic/SFP support, storage controller and UPS telemetry).
- Regulatory checks. Where ICASA type approval applies, record exact model approval/identifier, local registered applicant/importer and supporting evidence; do not assume foreign approvals automatically satisfy South African obligations.
- Decision (select, reject, or hold), assessor, date, reason, and the profile/tests for which the exact revision is in scope.

Official ICASA guidance says relevant communications equipment must be type-approved unless exempt, applications can be made by manufacturers/importers/distributors or other South African registered companies, and certificates are issued only to South African registered companies. Check the exact product, exemptions and current rules before import, supply or use: https://www.icasa.org.za/pages/type-approval

### Known BOM decisions to close first

| Component | Current research direction | Exit evidence before purchase/qualification |
|---|---|---|
| Gateway | OpenWrt One 1 for **lab only**; production NGFW is a separate line | Exact board lot/revision, supported firmware, source/license review, full BOM status, checksum/signature path, recovery test, measured VLAN/firewall/VPN throughput, local sourcing evidence. Upstream notes a physical M.2 post detail on some later batches; inspect the actual unit. |
| Switch | Compare current, exact revisions of listed candidates | Written South African quotes; firmware/lifecycle evidence; tested trunk/access ports, ACLs, DHCP protections, management isolation, IPv4 and IPv6 policy, PoE budget and config restore. |
| AP | EAP650 is a lead, not a selection | Exact SKU and ICASA path where applicable; local controller mode; VLAN/SSID and guest isolation tests; legal country/RF configuration; WAN/controller outage behavior. |
| Server | PowerEdge T160 is an unconfigured reference candidate | Line-item SKU quote; ECC RAM, TPM, storage/controller, NIC, firmware and warranty; NixOS and iDRAC tests; dedicated management isolation; power/thermal and restore measurements. |
| Endpoint | Framework Laptop 13 Ryzen AI 300 is a managed-endpoint lead | Exact locally serviceable SKU; signed boot/Secure Boot custom-key and firmware-update drill; encryption recovery; NixOS driver baseline; local warranty and replacement path. |
| Admin authenticators | Compare FIDO2/WebAuthn and any contract-required smart-card/certificate option | Relying-party compatibility; enroll/loss/revoke/recovery test; two separately stored authenticators per privileged operator; validate the exact cryptographic certificate scope only if required. |
| Backup target + offline recovery | Still unselected | Storage/drive/firmware selection, independent failure domain, key custody, protected credentials, retention, offline-copy custodian, and an observed restore without WAN or production credentials. |
| UPS | APC Easy UPS SMV1500AI family is an indicative price lead | Exact plug/outlet SKU, written local quote, replacement battery/support, measured real load and runtime; NUT/driver compatibility and mains-loss/graceful-shutdown test. |
| OOB/recovery | Physical console only in initial lab; separate network/console required for operational profile | Prove recovery when WAN, DNS, identity and production LAN are unavailable; audit access and rotate credentials; do not call a VLAN on the production switch physically independent. |
| Cabling/optics | Exact run lengths and ports not fixed | Cable/link plan, tested lengths, exact SFP/SFP+ type and link budget, supported part list, labels and acceptance result. |

The current partial ZAR subtotal in the profile is only the access point plus a low-end server marketplace comparison (**ZAR 29,428–61,848** as checked 10 October 2026). It excludes most of the system, does not reflect the desired server configuration, and is neither a quote nor a total cost of ownership. Keep the UPS entry outside that subtotal unless the arithmetic and included-component list are explicitly updated together.

## 5. Qualification sequence and release gates

Run these as ordered, evidence-producing gates. A blocked gate prevents progression; a simulation or schema pass cannot replace a physical observation.

**G0 — Freeze the test subject.** Record full Git SHA, profile JSON digest, exact SKU/revision/serial, firmware/BIOS, kernel/NixOS revision, switch/AP/gateway configuration digests and SBOMs. Use public/synthetic test data only.

**G1 — Source and procurement.** Pin the design/product source, review artifact-level licenses, record BOM completeness and firmware dependencies, and obtain written local supplier evidence. No SKU or supplier evidence means BLOCKED, not “probably available.”

**G2 — Reproducible provisioning.** Install from documented media with WAN disabled; verify signed artifacts, configuration import/export, credential enrollment/revocation and operator recovery. Capture all commands, logs, hashes and failure output.

**G3 — Segmentation.** Test every allow-flow and deny-flow from the profile at the physical port level, including IPv4 and IPv6, guest access to private ranges, inter-VLAN default deny, management-plane exposure, IoT lateral movement and quarantine containment. Test both positive and negative cases and retain packet captures.

**G4 — Update and rollback.** Verify firmware/OS provenance, signature failures, interrupted update, safe fallback, configuration restore and the case where central management or WAN is unavailable. Do not test dangerous firmware operations on a customer or production network.

**G5 — Resilience.** Pull WAN, DNS, identity and switch/gateway uplinks in controlled combinations; test UPS mains loss, low-battery shutdown, server/storage failure, backup-target isolation and offline restore. Measure actual load, usable capacity, restore time (RTO) and data-loss window (RPO).

**G6 — Operations.** Test named admin access, phishing-resistant MFA where selected, local audit integrity, alert/incident workflow, secret rotation, patch handling, lifecycle/vulnerability evidence and documented repair/replacement. Measure performance on the exact configuration instead of extrapolating from manufacturer maxima.

**G7 — Independent evidence review.** A reviewer other than the implementer checks evidence completeness, source/test subject binding, adverse cases, traceability, exceptions and remaining blockers. Keep NOT_EVALUATED unless the actual evidence justifies another scoped state.

**G8 — Explicit release decision.** H4 requires all applicable profile gates to be passed for the exact observed configuration and no unresolved blocking gaps. H5 remains a separate named customer/authority decision. No lab result authorizes classified deployment, tactical systems, weapons interfaces or targeting.

Each evidence packet must contain raw observations, expected-vs-actual outcome, exact subject identity, tool/test versions, test-run URL and artifact digests, assessor, time, exceptions and remediation owner. Preserve failures; don't rewrite the record as a pass after remediation without running and capturing the test again.

## 6. Stop conditions

Stop the procurement or release path if any of these is true: a required compulsory briefing was missed; bidder/supplier eligibility is uncertain; current legal status or procurement terms are unverified; the advertised SKU does not match the physical unit; firmware/source/license/BOM claims cannot be substantiated; the device cannot be updated and recovered safely; IPv6 bypasses network segmentation; no independent recovery/restore works; local regulatory approval remains unresolved; or support, SLA, warranty and lifecycle commitments are absent.

## 7. Review cadence

- Recheck open RFQ pages and amendments before each internal go/no-go and no later than two business days before closing.
- Recheck legal/procurement framework and SITA effective panels before every bid; preserve the official source and date of retrieval.
- Recheck supplier stock/lead time, quote validity, firmware and lifecycle notices at purchase order and again before deployment.
- Update this file only with dated observations, evidence links and explicit uncertainty. Expired notices move to historical evidence; no inferred eligibility or silent deadline rollover.
