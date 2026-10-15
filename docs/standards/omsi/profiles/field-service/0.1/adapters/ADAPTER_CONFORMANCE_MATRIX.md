# OMSI Field Service Adapter Conformance Matrix 0.1

**Status:** proposed engineering gate for the Field Service profile; not an adopted standard or certification  
**Research date:** 2026-10-15  
**Scope:** offline, read-only mappings from source service-management records into the OMSI Field Service intake projection

This matrix separates three questions that are easy to conflate:

1. Is the source snapshot structurally valid?
2. Did the adapter preserve the source's meaning and disclose contradictions or omissions?
3. Has a real deployment demonstrated safe, tenant-isolated interoperability?

Passing a synthetic fixture suite answers only a bounded part of the first two questions. It does not authenticate a source installation, prove a human credential, verify workmanship, demonstrate a live connector, or establish production readiness.

## 1. Result semantics

Adapters should use these outcomes consistently. They are distinct categories, not severity labels that may be silently converted into one another.

| Outcome | Meaning | Required behavior | Example |
|---|---|---|---|
| **Reject** | The adapter cannot safely interpret or scope the input under its declared source pin and binding. | Do not emit a normal intake projection. Return a stable error category; do not fall back to a weaker mapping. | Inactive tenant binding, source instance mismatch, unsupported pinned release, broken plan/task/log link, impossible date/status combination contradicted by the pinned controller. |
| **Conflict** | The input is parseable and attributable, but source fields disagree or a source claim conflicts with another source claim. | Preserve the original dimensions, emit a machine-readable conflict, and do not upgrade assurance or authorization. | ERPNext `docstatus=2` while `maintenance_status=Completed`; a signature/certificate requirement without corresponding source evidence. |
| **Loss** | Information is intentionally omitted, unavailable, or not represented in this intake slice. | Record the loss reason at the relevant field or projection level. Do not replace omitted data with a guessed value. | Raw customer contact, signature bytes, certificate contents, raw editor HTML, or unavailable event history. |
| **Unmapped** | A structurally valid source value belongs to an intentionally extensible namespace, but the active mapping has no safe normalized counterpart. | Preserve the scoped raw source value and mapping version; use an explicit neutral/unmapped result. Do not guess, close, authorize, or verify it. | A custom OCA Field Service stage ID absent from the configured stage map. |
| **Not provided / unavailable** | The source does not supply a fact or the optional source capability was not installed/exported. | Keep absence distinct from a negative answer and from a failed check. | No activity add-on installed is not equivalent to “zero required activities”; no credential data is not equivalent to “unqualified.” |

**Boundary rule:** reject when identity, tenancy, source contract, or safe interpretation cannot be established; use a conflict when a trustworthy projection can preserve two disagreeing claims without pretending to reconcile them; use a loss when content is deliberately omitted or unavailable; use unmapped only for a source value whose type/meaning remains sufficiently understood to preserve it safely.

A strict pinned enum that changes unexpectedly is normally an unsupported-source-contract rejection. A user-configurable stage namespace may instead remain structurally valid and map to `unmapped`. This distinction should be documented per source field rather than handled by a generic “unknown means something close” fallback.

## 2. Proposed conformance levels

These are review gates for engineering evidence, not a maturity badge and not a certification scheme.

| Level | Name | Evidence required | Does not prove |
|---|---|---|---|
| **C0** | Mapping research | Exact release/commit pins, inspected source fields/controllers, explicit mapping and licensing notes, known non-equivalences. | Executable correctness or working interoperability. |
| **C1** | Synthetic adapter conformance | Strict source/output schemas, synthetic fixtures, positive cases, adversarial negative controls, deterministic mapping, focused tests plus the common suite. | Any connection to a real installation, authentic source metadata, real customer records, or deployed security controls. |
| **C2** | Source-snapshot conformance | A separately reviewed, read-only export from the declared source version; export provenance, tenant scope, redaction and source-version evidence; the same suite passes against the minimized snapshot. | Live operational reliability, write isolation under deployed credentials, or semantic round-trip with another vendor. |
| **C3** | Live read-only integration | Least-privilege credentials; verified tenant/site binding; read-only permissions checked at the server; pagination, retry, rate-limit, revoked-access, partial-read and stale-snapshot handling; privacy and audit review; no-write negative tests. | Cross-vendor business-process equivalence or independent verification of physical outcomes. |
| **C4** | Cross-implementation interoperability | Pairwise tests against independently running implementations, shared semantic test vectors, documented loss/conflict behavior, replay/idempotency checks, and evidence that the consumer preserves source claims and authority boundaries. | Universal equivalence between vendors, legal compliance, or trade/workmanship certification. |

The OCA and ERPNext prototypes described in the linked mappings use synthetic fixtures and offline mapping code. They must not be described as C2, C3 or C4 until the additional evidence above exists.

## 3. Shared invariant matrix

Every adapter slice should map its tests to these IDs. A test is not complete merely because a schema accepts the fixture: the expected semantic behavior and the negative control must be asserted.

| Test ID | Invariant | Positive case | Adversarial / negative control | Failure handling |
|---|---|---|---|---|
| **ID-01** | Namespaced identity is stable and unambiguous. | Same snapshot and source namespace yields the same external ID. | Same local record name in two instances differs; separator characters inside source-controlled identity components cannot collapse two tuples. | Reject invalid identity inputs or escape components unambiguously. |
| **TEN-01** | Tenant comes from explicit trusted integration configuration. | Active binding matches source instance and company scope. | Wrong instance/company, revoked or unavailable binding, future-dated binding. | Reject; never infer OMSI tenant from a vendor company/customer field. |
| **PIN-01** | Mapping is bound to the inspected source contract. | Exact pinned version, commit and expected source schema pass. | Unsupported commit, unexpected license indicator, unknown required field or changed controller invariant. | Reject and require source review/re-pin. A license indicator is metadata, not a legal conclusion. |
| **REL-01** | Related records are correctly linked. | Log points to the expected plan and task in the same source namespace. | Plan/task/log mismatch, absent required relationship, cross-instance reference. | Reject or preserve a typed conflict only where the record can still be safely scoped. |
| **TIME-01** | Source temporal precision and meaning are preserved. | Date remains a date; request, schedule, due, actual-work and export/revision times stay separate. | Future claimed completion/export time, inverted interval, naive timestamp, invented timezone, impossible date. | Reject invalid event chronology; disclose incomplete intervals; allow legitimately future due/scheduled dates. |
| **LIFE-01** | Raw source lifecycle and normalized lifecycle remain distinct. | Versioned/configured mapping yields a normalized status while retaining raw fields. | Unmapped custom stage, contradictory document/status fields, incomplete required activity beside a closed stage, or an ERPNext `Planned` log whose due date is before the configured site-local date. | `unmapped` for extensible unknown values; conflict for preservable disagreement (including stale `Planned` vs past-due), while preserving the source claim; reject when pinned source rules make the record invalid. |
| **EVD-01** | Source claims are not upgraded into assurance. | Provider completion, signature/certificate flag, customer acknowledgment, invoice link and attachment presence stay separately typed. | Missing required artifact, flag without reference, active-log flag/current-task requirement mismatch, or completed status without independent verification. | Emit a conflict/loss and conservative assurance values; distinguish a current-vs-historical value difference from an active-record mismatch; never infer authenticity, licensure, payment settlement or workmanship quality. |
| **PRIV-01** | Output follows data minimization. | Intake contains only fields required for its declared purpose. | Raw contact/address, signature bytes, certificate bytes/path, untrusted HTML/actions text, or unknown extra fields leak into output. | Reject unrecognized fields where applicable; assert the forbidden data is absent from serialized output. |
| **HIST-01** | A snapshot is not an event ledger. | Source revision/export time and amendment/reference links are retained when available. | Missing history, changed source record, amended/cancelled record, stale snapshot, or mutable task dates that do not reconstruct submitted-log ordering. | Disclose history gaps; don't claim immutability, complete custody or a tamper-evident chronology unless separately demonstrated. |
| **DET-01** | Mapping is repeatable for identical inputs/configuration/clock. | Canonical output is identical over repeated runs. | Input key order, repeated replay, retries, or duplicate export of the same source revision. | Avoid nondeterministic IDs or order; define idempotency scope and revision conflict behavior. |
| **READ-01** | Prototype is strictly read-only. | Adapter consumes supplied snapshots and configuration and emits a projection. | No-create/no-update/no-delete test; connector credentials with write capability must not be silently accepted in a future live adapter. | Keep the integration read-only by construction and permission, not only by documentation. |
| **CI-01** | Evidence is tied to exact reviewed source. | Focused adapter tests, schema checks, common suite, fixtures and interoperability checks pass on the same head SHA. | Queued, cancelled, stale-head, partially skipped, or only locally asserted run. | Record run URL and SHA; do not call queued/cancelled/incomplete “green.” Preserve failing negative controls. |

## 4. Source-specific interpretations

### OCA Field Service 19.0

- The stage namespace is configurable. A stage absent from the configured map should remain an explicit unmapped source stage, not a guessed OMSI state.
- Preserve stage ID/name/type/closed flag and mapping revision together. A closed stage is not a verified operation.
- Optional Activity and Accounting addons must be represented as installed/exported/unavailable rather than silently represented as empty data.
- A signature-required stage, a signature metadata flag, a raw signature artifact, customer scope approval, and independent workmanship verification are different facts.
- Requested window, scheduled interval and actual-work interval must remain separate.

### ERPNext v16.51.0

- Maintenance Schedule, Maintenance Visit, Asset Maintenance plan, Asset Maintenance Task and Asset Maintenance Log are distinct source objects. The current adapter is only the Asset Maintenance Log slice.
- Frappe `docstatus` and custom `maintenance_status` must remain separate. The inspected controller constrains certain combinations but does not establish a universal one-to-one state mapping. Preserve contradictory imported combinations as conflicts where safe.
- `certificate_required`, `has_certificate`, and attachment-reference presence are separate. Neither a boolean nor an attachment path proves authenticity.
- Due/completion/next-task due dates remain date-only. The trusted binding declares a validated IANA site timezone for deriving the current local date during future-claim checks; do not assume the worker/runtime timezone.
- The pinned plan controller synchronizes `has_certificate` from `certificate_required` only when creating/updating Planned/Overdue logs. A historical Completed/Cancelled log is not refreshed by that helper; if the current task requirement differs from the log flag, report `current_task_certificate_requirement_may_postdate_historical_log` as a temporal loss instead of automatically flagging a conflict. A mismatch on an active Planned/Overdue log is a conflict.
- The pinned controller assigns a submitted log's completion date to the linked task's `last_completion_date` when the values differ, without a calendar-order comparison. A later-submitted older log can move that task field backward; preserve it as mutable source state and disclose `task_last_completion_date_is_not_reconstructed_from_event_history` rather than treating it as an ordered log ledger.
- `actions_performed` is a provider-supplied work claim and is omitted from the first privacy-minimized projection.

Vendor details are in [the OCA mapping](OCA_FIELD_SERVICE_19_0_MAPPING.md) and [the ERPNext mapping](ERPNEXT_V16_51_0_MAINTENANCE_MAPPING.md).

## 5. Reviewed implementation evidence snapshot

Evidence below is a dated snapshot, not a permanent CI claim. Recheck the current PR head and associated run before relying on it.

| Slice | Exact code head | CI evidence at review | Current scope boundary |
|---|---|---|---|
| Shared Field Service synthetic profile (PR [#46](https://github.com/Luminous-Dynamics/luminous-platform/pull/46)) | `991c289a34a2e1433642d09fe2e4f610a2a776ca` | [Run #38689813506](https://github.com/Luminous-Dynamics/luminous-platform/actions/runs/38689813506) passed at that head. | Five focused tests plus schema/fixture-level negative controls; synthetic only. |
| OCA read-only adapter (PR [#48](https://github.com/Luminous-Dynamics/luminous-platform/pull/48)) | `9740336957583c3084cfca99352ae2129f5801dc` | [Run #38695505503](https://github.com/Luminous-Dynamics/luminous-platform/actions/runs/38695505503) passed at that head, including the 128-test common suite, five profile tests, 23 OCA adapter tests, schema/fixture checks and interoperability target. | C1 synthetic offline mapping; no live Odoo extraction or write-back. |
| ERPNext Asset Maintenance Log adapter (PR [#49](https://github.com/Luminous-Dynamics/luminous-platform/pull/49)) | `16d675e1e5b204967afe7c4456660e78fb065e69` | Exact-head PR run [#38726865967](https://github.com/Luminous-Dynamics/luminous-platform/actions/runs/38726865967) is **queued** (last updated 2026-10-15 11:22:28 UTC); not a pass. The predecessor head `3b543b11301acfd3dd0ffb50d2d1c691f157168a` also had a queued run #38726253326, which does not transfer evidence. Focused suite is 41 tests; output includes task-date/event-history loss disclosure and distinct handling for active vs historical certificate flags. | C1 synthetic offline mapping; no live ERPNext site or API credentials. |

Green results on an earlier head do not transfer to a newer commit. The same principle applies if only one target in a workflow completes.

## 6. Prior art and implementation boundaries

OMSI should reuse existing standards at the layer where they fit rather than invent equivalent machinery:

- [JSON Schema Draft 2020-12](https://json-schema.org/draft/2020-12/) is used for structural contracts. Schema validation does not establish the authenticity of a source, authorization of a tenant, or truth of a work claim. Implementations should assert critical timestamp/format constraints in executable tests rather than relying on an annotation alone.
- [CloudEvents 1.0.2](https://github.com/cloudevents/spec/tree/v1.0.2) is relevant if a future connector transports source lifecycle events. Its event envelope is not an OMSI authority grant, an event-history completeness proof, or a physical-outcome verifier.
- [W3C PROV-O](https://www.w3.org/TR/prov-o/) is useful provenance prior art for entities, activities and agents. This profile does not require RDF or mandate a particular PROV serialization; an implementation may map the equivalent claim/provenance links while retaining the OMSI-specific assurance boundary.

The source version, instance namespace, source object, source revision/export time, mapper version, and resulting projection revision should be enough to explain which source data was mapped and under what rule set. Do not claim cryptographic authenticity or complete custody unless a separate signature/key/custody contract is actually implemented and verified.

## 7. Gate before expanding this profile

Before a real connector is accepted, require all of the following:

1. The shared matrix is mapped to named executable tests, including at least one negative control per trust boundary.
2. Source-schema and controller pins are reviewed and the mapping-loss/conflict vocabulary is versioned.
3. A real-source snapshot is obtained by a read-only principal, minimized before test retention, and accompanied by a reviewed extraction/provenance record.
4. Live reads are tested for tenant isolation, revocation, pagination, retries, stale/partial snapshots and zero writes.
5. A paired consumer test proves that normalized status does not erase source claims, mapping losses, conflicts, or assurance limits.
6. Exact-head CI and retained receipts identify the precise code under review. No draft becomes “production ready” based solely on synthetic conformance.
