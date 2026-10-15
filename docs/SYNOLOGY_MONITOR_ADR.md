# ADR: Synology DSM monitoring starts with read-only SNMPv3

**Status:** Proposed
**Date:** 2026-10-13
**Scope:** Remote monitoring of Synology DSM from an already-supported Luminous host
**Implementation status:** Not implemented; no NAS has been qualified

## Decision

The first Synology integration is an external, read-only SNMPv3 collector. It does not install anything on DSM and does not have permission to modify NAS configuration.

The required security level is **authPriv** (authentication and privacy). SNMPv1, SNMPv2c, SNMPv3 noAuthNoPriv, and SNMPv3 authNoPriv are unsupported. The SNMP read view must be limited to the documented OIDs required for the feature. The network should restrict UDP/161 to the collector and keep management traffic on a private LAN/VPN.

This is a deliberate choice over DSM Web API login for the first phase: it avoids NAS-side API credentials and starts with Synology-documented monitoring surfaces. DSM Web API access is a later, separately authorized adapter only when a specific, documented field cannot be obtained through MIBs.

## Research basis

Synology's own SNMP help documents SNMPv1/v2c/v3 and recommends SNMPv3 for security. Its MIB guide documents Synology-specific system, disk, RAID, UPS and SMART data, as well as status enumerations. The currently surfaced guide lists these base enterprise OIDs:

| MIB | Base OID | Initial use |
|---|---|---|
| System | `1.3.6.1.4.1.6574.1` | System partition, temperature, power/fan/thermal status, model, DSM version, CPU and memory utilization |
| Disk | `1.3.6.1.4.1.6574.2` | Disk model/type/status/temperature/health and selected health metrics |
| RAID/storage | `1.3.6.1.4.1.6574.3` | Storage item status and capacity |
| UPS | `1.3.6.1.4.1.6574.4` | Optional UPS status, only when present and needed |
| SMART | `1.3.6.1.4.1.6574.5` | Excluded from the default collection profile; can contain detailed/raw per-drive telemetry |

The official model and DSM-specific OIDs are mapped in the draft [Synology SNMPv3 profile](../profiles/synology-dsm-snmpv3.yaml). Verify each scalar/column OID and value mapping against Synology's guide and target MIB files before coding; the profile is not yet a tested collector configuration.

## Rust library evaluation

**Candidate for a prototype:** [`async-snmp`](https://docs.rs/async-snmp/latest/async_snmp/), which is listed as version 0.18.1 in the documentation consulted on 2026-10-13. It supports SNMPv3 USM, authPriv with SHA-256 and AES-128, GET/GETNEXT/GETBULK/WALK, bounded timeouts/retries, strict BER decoding controls, response-shape validation, and strict UDP source correlation.

It remains a *candidate*, not an approved dependency:
- Its upstream documentation states that it is pre-1.0 and breaking API changes may occur between versions.
- The public library exposes SET as well as read operations, so the application boundary must not expose the raw client or any SET function.
- Lock and review the exact version, transitive dependency tree, license, MSRV, release history, and security advisories before merging a prototype.
- Keep the library's default crypto backend explicit. Use the RustCrypto backend unless a documented FIPS requirement calls for an audited FIPS backend. Do not silently change crypto backend during upgrades.
- Build with a narrowly scoped feature set; do not enable the SNMP agent, trap receiver, CLI, or write-capable product interface for a polling-only binary unless justified.

### Required client policy

- Construct configuration only from a validated target profile and secret-provider result. Do not accept raw OIDs or arbitrary operations from user/AI intent.
- Use SNMPv3 `AuthPriv`, a supported SHA-2 authentication algorithm (prefer SHA-256 when the DSM target supports it), and AES privacy; record exact values in qualification evidence.
- Use strict packet decoding and strict response shape. Start with strict UDP source checking and unauthenticated time-window correction disabled; only relax a setting for a captured, target-specific interoperability defect, with a bounded test and documented reason.
- Set explicit construction/request deadlines, at most one retry initially, and a per-walk row limit. Never use unbounded walk.
- Present only `get_allowlisted` and `walk_allowlisted` methods from a private wrapper. Never expose `Client::set`, the raw SNMP client, arbitrary OID constructors, or user-controlled protocol/security-level selection to the rest of Luminous Platform.
- Validate each response OID against the exact requested OID or explicitly allowlisted table column and index range. Reject out-of-subtree values, repeated/non-increasing walk results, ambiguous response shapes, oversized/truncated datagrams, and malformed encodings.
- Treat unauthenticated engine discovery as discovery only; do not treat it as a verified observation. Only produce a successful receipt after authenticated response validation.

## Observation and evidence semantics

SNMP observation is not host-control authority and is not equivalent to an independent hardware attestation.

Each observation should distinguish:
- `healthy`: all required configured probes returned valid, recognized values and the policy allows that conclusion;
- `degraded`: recognized component status explicitly indicates a warning, repair, migration, or degraded state;
- `failed`: a recognized status explicitly indicates component failure;
- `unknown`: a required OID was absent, unsupported, or mapped to an unknown enum;
- `unreachable`: no valid response before the bounded deadline;
- `invalid_observation`: malformed response, OID outside scope, unexpected response shape, replay/correlation/security anomaly, or failed validation;
- `partial`: some optional data was unavailable while required probes completed.

Do not use a single system scalar as a global health verdict. For example, Synology documents `systemStatus` as the system *partition* status. Storage states such as rebuilding/repairing are in-progress states, not generic healthy/failed booleans. Preserve raw typed values and the mapping/profile version in evidence to enable independent re-evaluation.

Receipts must bind the collector build digest, profile version, target alias, poll timestamp, MIB revision, exact OID set digest, per-probe outcomes, normalized result, verifier version and receipt digest. Do not collect or persist serial numbers, authentication/privacy secrets, session tokens, or raw SNMP packets by default.

## First implementation gates

1. Validate and parse `profiles/synology-dsm-snmpv3.yaml` in CI; fail on unknown fields and invalid policy values.
2. Implement pure, fixture-driven parsing and normalization before network IO. Include every documented status enum and unknown values.
3. Add a client wrapper whose public API cannot express SET, unconstrained OID walking, SNMPv1/v2c, or weaker SNMPv3 levels.
4. Test malformed/truncated BER, wrong OID, wrong response ID/source, timeout, bad credentials, auth/privacy failure, repeated OID during walk, too-large/partial tables, unsupported OIDs and enum values.
5. Run an ephemeral local SNMPv3 test agent in CI with a fixed non-secret test key and assert observed traffic contains only allowed GET/GETNEXT/GETBULK operations, never SET. Test keys must never be reused outside the isolated harness.
6. Capture deterministic evidence per commit. A queued workflow is not a pass.
7. Qualify against an actual Synology model/build when hardware is available; keep this separate from CI simulation and do not block deterministic code validation on access to one specific NAS.

## Explicit non-goals

- No DSM boot, package manager, bootloader, kernel, or initramfs mutation.
- No write-capable DSM APIs or SNMP SET.
- No autonomous configuration changes, firmware upgrades, or storage operations.
- No Docker socket, privileged container, or root access.
- No claim of general Synology support until an exact profile passes its published gates.

## Official references

- [Synology SNMP settings and SNMPv3 guidance](https://kb.synology.com/en-ca/DSM/help/DSM/AdminCenter/system_snmp?version=7)
- [Synology SNMP MIB Guide: MIBs and useful OIDs](https://kb.synology.com/en-af/DG/Synology_DiskStation_MIB_Guide/5)
- [DSM Login Web API Guide: catalog discovery and session workflow](https://kb.synology.com/en-global/DG/DSM_Login_Web_API_Guide/2)
- [async-snmp published documentation](https://docs.rs/async-snmp/0.18.1/async_snmp/)
- [async-snmp upstream repository and pre-1.0 status](https://github.com/lukeod/async-snmp)
