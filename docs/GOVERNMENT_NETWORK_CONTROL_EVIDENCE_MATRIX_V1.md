# Government & Regulated Infrastructure — Control Evidence Matrix v1

**State:** baseline, not assessed; no control is marked PASS  
**Reviewed:** 2026-10-10  
**Machine-readable source of truth:** [../security/control-evidence-matrix-v1.json](../security/control-evidence-matrix-v1.json)  
**Schema:** [../schemas/hardware-control-evidence-matrix-v1.schema.json](../schemas/hardware-control-evidence-matrix-v1.schema.json)

## Purpose and limits

This matrix turns the product taxonomy into evidence-producing qualification work. It identifies control intent, the Luminous components that must enforce it, references to potentially applicable public guidance, and evidence required to evaluate a named deployment profile.

It is **not** an Authority to Operate, compliance certificate, system security plan, accreditation package, legal opinion, selected NIST baseline, or assertion that every mapped control applies. The NIST SP 800-53 catalog and SP 800-53A assessment procedures are useful reference material, but selecting a baseline, tailoring it, assessing implementation and accepting residual risk are separate decisions. See the [SP 800-53 Rev. 5 page](https://csrc.nist.gov/pubs/sp/800/53/r5/upd1/final) and [SP 800-53A assessment procedures](https://csrc.nist.gov/pubs/sp/800/53/a/r5/final).

The current profile targets **non-production, public/synthetic-data testing** of secure administrative/public-service/unclassified IT. It explicitly excludes classified, tactical command/communications, weapons-interface and targeting deployments. Those require separate governing requirements and an approved evaluation path.

## Control status vocabulary

| Status | Meaning | Evidence requirement |
|---|---|---|
| `NOT_EVALUATED` | Requirement has been identified, but no assessment result exists. | Keep assessment record null; state unresolved gaps. |
| `PARTIAL` | Some scoped behavior is demonstrated, but a gap or limitation remains. | Exact tested revision, assessor, scope, date, evidence references and limitations. |
| `PASS` | The control passed for its named, bounded test scope. | Full 40-character source SHA, exact deployment/hardware scope, evidence packet, named assessor, date and no unresolved gaps/limitations. |
| `FAIL` | The control failed for the specified scope. | Exact revision, test scope, logs/artifacts, failure description, remediation owner and next evaluation. |
| `NOT_APPLICABLE` | A responsible assessor has determined the control does not apply to the named profile. | Explicit rationale, assessor/date, profile and supporting references. Do not use this state for a missing implementation. |

The JSON validator checks structural consistency and blocks several unsupported status transitions. It cannot prove that an attached evidence packet is authentic, complete or correctly interpreted. Passing schema validation or CI alone never proves the corresponding runtime control.

## Control inventory

All 23 rows are currently `NOT_EVALUATED`; their existence is a plan, not evidence of implementation.

| ID | Control intent | Representative verification |
|---|---|---|
| AUTH-001 | Account lifecycle, MFA and least privilege | Enrollment/revocation, privileged MFA, negative authorization tests |
| AUTH-002 | Break-glass and recovery authority | Recover with central identity/WAN unavailable; rotate credentials after use |
| AUTHZ-003 | Deterministic authorization for state changes | Forged actor, stale precondition, replay and cross-tenant denial tests |
| CFG-004 | Exact configuration baseline and drift handling | Pinned profile digest, rejected unknown fields, controlled rollback |
| BOOT-005 | Firmware trust, signed boot and recovery | Exact device/firmware trust path, image rejection and safe recovery |
| PATCH-006 | Vulnerability response and controlled updates | Advisory freshness, exception expiry, update failure/rollback, closure coverage |
| NET-007 | Segmentation and explicit traffic flows | Hardware VLAN/ACL tests, IPv4 **and** IPv6 negative path tests |
| NET-008 | Management-plane and OOB isolation | From-WAN/staff/guest access denial and recovery after production network loss |
| CRYPTO-009 | Key lifecycle and validated crypto scope | Key revocation/recovery; exact certificate/module/mode where required |
| ASSET-010 | Asset identity, HBOM/SBOM and provenance | Physical SKU/serial and firmware reconciliation against inventory and supplier docs |
| LOG-011 | Audit integrity and event provenance | Tamper/privilege/time tests; prove material actions have actor, result and revision |
| MON-012 | Monitoring, triage and incident response | Known-signal tests; alert-to-case trace; action authorization boundaries |
| BACKUP-013 | Encrypted, independent backup and restore | Restore disconnected from production/WAN; measure RPO/RTO |
| CONT-014 | Power, environmental and continuity behavior | Measured load/UPS runtime, shutdown, WAN loss and recovery behavior |
| DATA-015 | Data classification, minimization and retention | Data-flow review, synthetic-data boundary, encryption and retention tests |
| PHYS-016 | Physical access, maintenance and media disposal | Custody, access, repair and sanitization evidence |
| SCRM-017 | Supplier, license, vulnerability and lifecycle risk | Supplier/provenance review, substitutes, warranty, end-of-support and spares |
| RADIO-018 | Jurisdictional equipment/radio/electrical requirements | Exact-SKU approval and label check; local requirements verified before supply/use |
| INCIDENT-019 | Ordered outbox events and authorized replay | Duplicate delivery, expired approval, dead-letter and uncertain acknowledgement tests |
| THIRD-020 | Cloud, external service and data-residency boundary | Disconnect provider/WAN; inspect telemetry and validate local control/recovery |
| DEVSEC-021 | Build, dependency and workflow integrity | Exact SHA, pinned actions/dependencies, artifact digests and job-level outcomes |
| PROC-022 | Procurement, eligibility, warranty and contract-backed support | Actual tender/panel pack, mandatory meeting, exact SKU, SLA and written quote review |
| EVIDENCE-023 | Independent assessment and deployment authorization | Block promotion from docs/simulation/queued CI; require scoped evidence and written authorization where needed |

The machine-readable register gives more detail for each item, including reference mappings, expected artifacts, Luminous product surfaces and open gaps.

## Jurisdiction-specific source mapping

- **United States / unclassified public-sector:** NIST control/assessment publications, supply-chain guidance and—if the requirements call for it—the exact FIPS cryptographic-module certificate scope. Yubico's May/June 2026 materials report certificate #5291 for its upgraded YubiKey 5 FIPS 140-3 Series and #5302 for YubiHSM 2 FIPS; verify both against the live [NIST CMVP register](https://csrc.nist.gov/Projects/cryptographic-module-validation-program/validated-modules) and exact supported mode before selecting either. These are possible references, not a determination of federal or DoD applicability. The [NIST CMVP database](https://csrc.nist.gov/Projects/cryptographic-module-validation-program/validated-modules) must be checked at exact module/version/mode/operational environment where validation is required.
- **South African public service:** the [DPSA standards index](https://www.dpsa.gov.za/policy-updates/e-gov/e_government/standards/) contains signed Public Service Information Security Directive material and multiple ICT-security/assessment references; the [Public Service Handbook](https://www.dpsa.gov.za/legislation/ps-handbook/) gives additional discovery context. Retrieve actual directives/circulars, confirm their current status and applicability with the relevant authority, and map their exact requirements before making a procurement or compliance statement.
- **Radio/electronic communications equipment in South Africa:** ICASA says that section 35(1) of the Electronic Communications Act restricts supply, sale, offer, lease or use of relevant electronic communications equipment unless the equipment has been approved, subject to the act's provisions. Verify the [ICASA type-approval requirements](https://www.icasa.org.za/pages/type-approval) for the exact device and use before import/supply. Do not assume an international CE/FCC listing by itself resolves local obligations.
- **Public-sector procurement:** [SITA's effective-panel list](https://www.sita.co.za/content/sita-effective-panel-contracts) and [live RFQ portal](https://rfq.sita.co.za/RFQ/RFQInvitations.asp) are discovery sources. A listed opportunity is not proof of Luminous eligibility or supplier accreditation. Confirm the live contract pack, exact technical scope, closing time, compulsory briefing, supplier-channel rules and support/warranty obligations. Record expired calls only as historical market evidence.

## Evidence packet contract

For every non-`NOT_EVALUATED` result, keep one versioned evidence packet containing:

1. **Subject identity:** full source commit SHA, profile digest, exact SKU/board revision, firmware, OS/kernel, network-device configuration and relevant dependency/SBOM digests.
2. **Test method and scope:** named control, preconditions, physical/simulated boundary, environment, instruments, expected result and exclusions.
3. **Raw observation:** logs, packet captures, console transcripts, test exit codes, hardware measurements, artifact digests and links to the immutable workflow/test run. Retain failed and negative tests, not just summaries.
4. **Assessment:** assessor/reviewer, date, scoped result, exceptions with owner/expiry, remediation or explicit non-applicability rationale.
5. **Independent verification:** verify the evidence packet and its binding to the exact subject. Authored test code and queued/skipped jobs are not observations of a tested system.

Use separate evidence types:
- **Static/contract evidence:** schema, configuration, ACL contract, role privileges, signed-artifact manifest.
- **Automated test evidence:** deterministic tests on a pinned source revision.
- **Hardware-in-the-loop evidence:** actual ports, radio, power, thermal, storage-failure and recovery behavior.
- **Operational evidence:** monitoring, restore drills, support/warranty records and controlled site recovery.
- **Authorization evidence:** requirements and signed approval by the authority responsible for the named deployment.

A simulation can test state-machine and policy logic. It cannot prove RF behavior, actual UPS runtime, firmware trust on a specific unit, secure boot recovery, local sourcing, or a supplier's support promise.

## Release gate

Before a product profile can be called operationally qualified, each applicable control must have an explicit `PASS` or justified `NOT_APPLICABLE` assessment against the exact profile and hardware/software subject, with no unresolved blocking gap. `PARTIAL`, `FAIL`, missing evidence, stale references, unverified supply, and queued/skipped checks remain blockers. Deployment authorization is a distinct, customer/environment-specific decision.
