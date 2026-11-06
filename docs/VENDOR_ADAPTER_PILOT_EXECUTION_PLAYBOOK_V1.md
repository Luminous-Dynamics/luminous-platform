# Vendor Adapter Pilot Execution Playbook v1

**Reviewed:** 2026-10-11  
**Status:** execution procedure; no device is qualified by this document  
**Parent:** [Global Vendor Interoperability and Firmware Services v1](GLOBAL_VENDOR_INTEROPERABILITY_AND_FIRMWARE_SERVICES_V1.md)  
**Implementation tracker:** [Issue #23](https://github.com/Luminous-Dynamics/luminous-platform/issues/23)

## Objective

Make useful progress without depending on a queued workflow or the availability of a physical device. The project can review code, validate schemas, exercise synthetic inputs, and independently inspect evidence now. Those results must remain distinct from CI and physical-device observations.

This playbook is a process gate, not proof that the validator or any adapter passes. Record each result against an immutable source commit and the exact evidence produced.

## Non-negotiable claim boundaries

- A successful schema check proves only that an object conforms to the checked schema.
- A semantic/cross-artifact check proves only the stated contract invariants.
- A mock or fixture test does not prove device interoperability.
- A successful API response does not prove the requested state was applied.
- A matching image digest does not by itself prove signer trust, device compatibility, successful boot, or runtime integrity.
- Valid Holochain records prove that records met the app's deterministic validation rules; they do not independently prove a device's physical state.
- A CI job that is queued, skipped, cancelled, missing, or attached to another commit is not a pass.
- A successful local command is local evidence, never relabelled as hosted CI.

## Execution lanes

### Lane 0 — Freeze the evidence subject

Before interpreting any test result, record:
- repository, branch, exact 40-character commit SHA, and changed-file set;
- validator/schema/fixture versions and relevant runtime/tool versions;
- workflow run ID and conclusion for that exact SHA, if a run exists;
- exact test commands, start/end timestamps, exit codes, and log/artifact digests;
- whether a physical device was contacted and whether any mutation was attempted.

Never combine evidence from different source revisions into one implied passing result. If source changes, rerun the checks affected by the change and create a new evidence packet.

The qualification runner must verify that every configured validator and invoked test suite is present in its frozen `INPUT_PATHS` manifest. Missing entries are a hard `INPUT_MANIFEST_INCOMPLETE` failure, not a warning. When a validator or suite is added, update the manifest and its regression guard in the same change.

### Lane 1 — Parse, then validate schema

Run a strict parser before semantic checks. Reject invalid UTF-8, duplicate JSON object keys, non-standard constants such as NaN/Infinity, trailing content, and malformed JSON. A malformed receipt must stop here with a structured validation error; it must not reach registry lookup or adapter-binding logic.

Then validate against the pinned JSON Schema version. Reject undeclared fields, unsupported enums, invalid digest formats, out-of-range values, missing required values, and invalid conditional combinations. Treat date-time parsing as an explicit runtime check as well as a schema annotation where the validator does not enforce the format keyword.

**Expected artifact:** strict-parser result, schema result, schema digest, and a redacted list of validation diagnostics. No device access is permitted in this lane.

### Lane 2 — Cross-document semantic verification

JSON Schema cannot establish all of the relationships the system needs. A separate verifier must check at least:

- receipt operation exists in the pinned operation catalog and is declared for the exact adapter;
- operation state is **tested** before an authorization or side effect can be treated as usable;
- adapter implementation revision, product family, model, hardware revision, software version, and region match the tested registry entry exactly;
- operation risk class meets the required maturity threshold (A3 for configuration mutation; A4 or higher for OEM firmware/recovery and alternative-OS flashing);
- target, action, desired-state, pre-state, policy, and profile digests match their authorization bindings;
- authorization signer/approver identity is trusted for this policy, not merely present as a string; expiry, revocation, audience/scope, and approval count are valid;
- apply-time pre-state is fresh, observed for this attempt, and matches the authorized pre-state; any drift invalidates the approval;
- event chronology is coherent; journal state transitions are legal; terminal state/effect combinations are consistent;
- retries after an uncertain mutation/lifecycle outcome are forbidden; only read-only reconciliation may establish the actual state before a new, separately authorized attempt;
- successful completion includes independent verification evidence, not only executor output;
- public/shared evidence contains no credentials, recovery keys, raw configurations, personal data, or unredacted customer topology.

**Expected artifact:** machine-readable verification report with each invariant marked pass, fail, or not_evaluated; a missing dependency is never silently treated as pass.

### Lane 3 — Adversarial synthetic corpus

Keep stable, named positive and negative vectors. Every negative case must assert both a controlled denial and **no side-effect attempt**.

| Case | Perturbation | Required result |
|---|---|---|
| Input parser | Duplicate key, NaN/Infinity, truncated JSON, wrong type | Structured error; no traceback or registry lookup |
| Registry binding | Unknown operation/adapter, undeclared product family | Deny |
| Scope binding | Different model/revision/software/region or adapter source SHA | Deny; no wildcard fallback |
| Authorization | Expired/revoked signer, wrong audience/tenant, altered action or target digest | Deny |
| Freshness | Stale pre-state, different pre-state digest, observation after apply start | Deny before mutation |
| State machine | APPLIED_UNVERIFIED then FAILED with NO_EFFECT | Reject; require truthful partial/indeterminate effect and evidence |
| Uncertain outcome | Timeout after request may have reached the device | Mark possible/indeterminate; no blind replay |
| Verification | Executor tries to act as its own independent verifier | Deny promotion |
| Evidence | Wrong image/config digest, missing raw artifact, stale or future-dated result | Deny promotion |
| Trust | Revoked/compromised signing key, missing trust anchor, unavailable verifier | No new high-risk operation |
| Privacy | Secret, raw config, topology, personal data in shared-DHT payload | Reject publication and emit redaction diagnostic |
| API robustness | 401/403, 404, 429 with Retry-After, malformed response, partial result, regional endpoint mismatch | Bounded handling; no scope widening or unsafe replay |
| Concurrency | Duplicate request, concurrent desired-state change, replayed idempotency key | One logical attempt; reconcile before any new mutation |
| Recovery | Restore path absent or not tested for this exact target | Block disruptive/reflash operation |

Add regression vectors for every discovered defect. Keep synthetic fixtures explicitly labelled synthetic, with deterministic inputs and expected outputs.

### Lane 4 — Protocol and vendor adapter qualification

Prefer standard protocols only after exact-device capability discovery. For NETCONF, record advertised capabilities (candidate, validate, confirmed commit, rollback-on-error, and locking where present); do not assume these features exist on every device or apply identically to every datastore. For gNMI, capture the Capabilities response, models, encodings, and version. For RESTCONF/OpenConfig, record model revisions, implemented paths, deviations, and authentication scope. For SNMPv3, record security level and read-only MIB/OID scope.

A discovered protocol or model does not qualify every operation. Track each operation independently: inventory, health, version, configuration export, planning, applying, verification, restoration, and firmware lifecycle are distinct capabilities.

For every vendor adapter, keep a source-reviewed fixture set covering missing/unknown fields, version drift, pagination, rate limits, timeouts, revoked credentials, wrong tenant/organization, incomplete configuration exports, and unexpected response formats. A read-only credential must not be accepted as evidence that write capability exists.

**Meraki-specific example:** its API budget is shared across applications at the organization level (documented steady-state limit: 10 requests/second); use a shared per-organization rate limiter, honor Retry-After for HTTP 429, isolate the configured regional endpoint, and treat API keys as high-impact secrets whose permissions are inherited from the issuing administrator. Do not assume a separate application has a separate organization quota.

### Lane 5 — Physical, non-production pilot

Choose the first device by access and recoverability, not vendor popularity. The device must be customer/project-owned or explicitly authorized, exact model and hardware revision identified, supported firmware/software recorded, management path documented, and OEM recovery/console path available. A missing device is a named blocker for this lane only; it must not block local schema, corpus, and semantic work.

1. Freeze the pilot target and authorization scope. Use a non-production unit; never choose the only production firewall or site gateway for a first-time adapter.
2. Capture read-only identity, firmware/software version, health, relevant capability discovery, and a configuration export if the platform supports a safe, authorized export.
3. Redact sensitive fields in a controlled copy; retain raw evidence in customer-controlled encrypted storage. Hash both the raw artifact and any normalized observation, recording the transformation/version.
4. Have an independent verifier reproduce the identity and relevant state from the device or an independent observation path. The executor's own success message is insufficient.
5. Compare the captured output with the adapter's normalized result; classify every missing, ambiguous, unsupported, or vendor-specific field. Do not silently convert missing fields into healthy/default values.
6. Record exact commands/API operations, credential role/scope (never credential values), response/correlation IDs, time, artifacts and digests, reviewer, known limitations, and follow-up defects.
7. Repeat the read-only operation and check determinism, pagination, rate-limit recovery, and handling of device/API unavailability. Keep the test non-mutating.
8. Request A1 promotion only after the evidence is independently reviewed and the declared qualification policy is satisfied for that exact target scope.

### Lane 6 — Change and firmware gates (later, separately authorized)

**Configuration mutation (A3):** begin with a low-risk, reversible change on a dedicated lab unit; require a displayed diff, explicit action-digest-bound authorization, fresh apply-time read, independent read-back, and a demonstrated recovery/rollback path. Model/vendor APIs may have transaction behavior that differs from ordinary HTTP semantics.

**OEM firmware/recovery (A4):** add exact image compatibility, authorized source/entitlement, cryptographic signer verification where available, immutable image digest, config/data handling, maintenance window, power/recovery readiness, stop conditions, canary rules, post-boot identity and health evidence, and documented OEM support/warranty consequence. Protect, detect, and recover are separate checks; an image signature/hash alone is insufficient.

**Alternative OS/reflash (A4 or higher and explicit allowlist):** bind approval to exact model, hardware/PCB revision, bootloader/flash layout, region, target release, image digest, install method, and recovery path. Require lab/non-production qualification, lawful image/source, physical recovery equipment, data-loss disclosure, written customer approval, and independent post-boot verification. An adjacent model or firmware revision is a new scope.

For all mutation/lifecycle paths, a timeout after a possible effect triggers reconciliation, not replay. A rollback is a new authorized operation; it is not assumed to have happened because a request failed.

## Maturity and promotion evidence

Keep adapter maturity separate from hardware/product maturity:

- **A0 — Discovered:** authoritative sources and boundaries recorded. No implementation claim.
- **A1 — Read-only tested:** exact scope; strict parser/schema/corpus checks; protected evidence capture and the required lab observation independently reviewed.
- **A2 — State-contract tested:** state machine, authorization, stale state, replay, chronology, revocation, and verifier separation demonstrated in tests.
- **A3 — Change qualified:** exact non-production target, action/read-back and recovery evidence, negative/failure injection, independent review.
- **A4 — Firmware/recovery qualified:** exact image and target, entitlement, boot/recovery and post-upgrade checks physically demonstrated.
- **A5 — Serviceable:** repeatable service runbook, monitoring/escalation, support boundary, lifecycle recheck cadence, contractual ownership, and customer/regional operating procedure.

Promotion is per operation and exact scope. A single successful inventory read does not qualify configuration export, mutation, firmware, or recovery. Changing firmware, API behavior, source revision, trust anchor, model, region or a material profile invalidates affected qualification until the required checks are repeated.

## Evidence packet minimum

Each packet must contain a manifest that binds:
- immutable source SHA and relevant file/schema/registry digests;
- exact target scope and evidence capture method;
- test/corpus version, command, exit code and output digest;
- raw-artifact digests and protected retrieval references;
- normalized observation digest plus versioned transformation/redaction rules;
- authorization/policy/profile references and verifier identity;
- per-invariant results, failures, not_evaluated reasons, and declared claim ceiling;
- source/license/support links and last-checked date for mutable vendor facts;
- physical-device vs mock status, operator/reviewer separation, and unresolved blockers.

Sensitive raw records stay under customer-controlled storage and policy. Shared Mycelix/Holochain records should contain only privacy-reviewed, signed, minimal claims/commitments and protected evidence references; hashes are not inherently non-sensitive.

## Immediate execution sequence

1. Inspect and run the strict input parser, receipt schema check, registry cross-check, and adversarial test entry points locally at the frozen source revision. Record actual command results; do not wait for remote CI to do this work.
2. Expand the stable negative corpus for any uncovered cases in Lane 3 and make each test assert no side effect was attempted.
3. Produce an independently checked evidence packet for the current synthetic receipt. Keep it marked synthetic/NOT_IMPLEMENTED.
4. Select one exact lab device by the Lane 5 criteria; if no device is available, report that as the only physical-pilot blocker while continuing work on the earlier lanes.
5. Implement one read-only adapter capability only; do not start with firmware mutation or broad vendor coverage.
6. Recheck exact-head CI and lab evidence; only then consider an A1 promotion for the specific operation and scope.

## Source references

- [NIST SP 800-193 — Platform Firmware Resiliency Guidelines](https://csrc.nist.gov/pubs/sp/800/193/final): protect, detect, and recover from unauthorized firmware changes.
- [NETCONF RFC 6241](https://www.rfc-editor.org/rfc/rfc6241): capability discovery and optional candidate/confirmed-commit/rollback behaviors.
- [RESTCONF RFC 8040](https://www.rfc-editor.org/rfc/rfc8040): HTTP-based YANG data interface.
- [Cisco Meraki Dashboard API rate limit](https://developer.cisco.com/meraki/api-v1/rate-limit/): shared per-organization request budget and 429 handling.
- [OpenWrt installation/recovery Table of Hardware](https://openwrt.org/toh/views/toh_admin_installation_recovery): model-specific recovery references.
- [SONiC FAQ](https://github.com/sonic-net/SONiC/wiki/FAQ): supported platform and community-support boundaries.
- [EU CRA reporting obligations](https://digital-strategy.ec.europa.eu/en/policies/cra-reporting): as of 11 September 2026, manufacturers must report actively exploited vulnerabilities and severe incidents impacting product security. Applicability depends on the product and the entity's legal role; do not treat a customer/operator service as automatically a CRA manufacturer or open-source steward.
