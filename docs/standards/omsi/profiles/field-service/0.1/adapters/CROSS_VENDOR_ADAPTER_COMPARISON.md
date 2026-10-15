# Cross-vendor Field Service adapter comparison (research)

**Status:** research note for OMSI Field Service profile 0.1; not an adopted standard  
**Research date:** 2026-10-15  
**Reviewed sources:** OCA Field Service 19.0 and ERPNext v16.51.0, each pinned in the linked source mapping

## Purpose

The first two source inspections test whether OMSI can provide a small, loss-aware intake projection without forcing service-management tools into a universal ticket schema. The result is **not** that both systems have equivalent lifecycle models. It is that a few external-reference and claim-boundary concepts can be shared while source-specific lifecycle and evidence semantics stay typed.

Source detail:
- [OCA Field Service 19.0 mapping](OCA_FIELD_SERVICE_19_0_MAPPING.md)
- [ERPNext v16.51.0 maintenance mapping](ERPNEXT_V16_51_0_MAINTENANCE_MAPPING.md)
- [Field Service profile 0.1](../PROFILE.md)
- [Adapter conformance matrix](ADAPTER_CONFORMANCE_MATRIX.md)

## Cross-vendor findings

| Dimension | OCA Field Service 19.0 | ERPNext v16.51.0 | OMSI adapter rule |
|---|---|---|---|
| Source identity | Odoo instance/database + model + record ID; human-readable order name is a display label | ERPNext site/database + DocType + document name | Build a namespaced external reference. Never key on a display name alone. |
| Tenant boundary | Source company_id is not OMSI tenant authorization | Source Company on the plan/visit is not OMSI tenant authorization | Require a separate trusted instance/site + company → tenant binding; reject mismatch or revoked bindings. |
| Planned vs actual time | Request window, scheduled start/end and actual start/end are distinct timestamps | Maintenance Schedule has planned dates and actual date; Maintenance Visit uses date and optional time; Asset Maintenance Log uses date-only log due/completion dates plus a separate task next-due date | Preserve source precision and type. For date-only future-event checks, derive the local date using an explicit, validated IANA timezone in trusted configuration; do not fabricate a timestamp or merge task planning with a log occurrence. |
| Current task state vs event chronology | Order lifecycle/optional activity data is not itself proof of a complete immutable event ledger | `Asset Maintenance Task.last_completion_date` is updated from a submitted log whenever the dates differ, without a max-date comparison; submission order can differ from date order | Preserve the task field and each log completion date separately. Emit `task_last_completion_date_is_not_reconstructed_from_event_history`; do not infer complete chronology from mutable task state. |
| Lifecycle | Configurable stage ID/name/type/closed flag; optional stages and activity add-on | Visit completion status differs from document status; recurring plan/task differs from a maintenance log; `Overdue` is distinct from `Blocked` | Preserve all source lifecycle fields and map through versioned configuration. For ERPNext, the pinned log controller promotes past-due nonterminal statuses to `Overdue`; a stale `Planned` + past-due snapshot should be disclosed as conflict rather than silently rewritten. Do not map lateness to an execution block. A normalized status is not a proof of outcome. |
| Maintenance-log cancellation | OCA order stage closure is distinct from quality verification | ERPNext custom `maintenance_status=Cancelled` is allowed on a submitted document (`docstatus=1`); generic document cancellation is `docstatus=2` and can retain another custom status. | Preserve both fields. Treat custom Cancelled + submitted as valid; disclose draft Cancelled (`docstatus=0`) as not yet submitted. If document-level cancellation is present, normalize conservatively to cancelled while preserving a non-Cancelled source field as conflict. |
| Practitioner | Assigned person(s) and team | Visit Purpose service_person references Sales Person; a maintenance task can be assigned to a User | Assignment is attribution only, not trade qualification, credential validity or jurisdiction authorization. |
| Checklist / work performed | Optional activity records have required/done/cancelled state, actor and time; order has description/resolution fields | Log has actions-performed content; Visit Purpose has work_done text | Preserve presence and provenance when useful, omit raw text by default, and never promote a source statement into independent verification. |
| Signatures / certificates | Order stage may require signature; source captures signer/time/signature presence | Task has `certificate_required`; log has `has_certificate` and certificate attachment field. Plan sync copies current task requirement only to Planned/Overdue logs. | Requirement, log boolean, attachment presence and authenticity are separate. A current-task/historical-log difference can be temporal drift, not a conflict; an active-log mismatch should be disclosed. Never infer authenticity, correctness or workmanship. |
| Invoice / payment | Optional accounting add-on links invoices to order lines | These maintenance objects are not a payment receipt | An invoice link is not evidence of settlement; do not invent payment fields. |
| Privacy | Locations derive from partner/address/contact data; HTML and signature data may be sensitive | Visits and schedules include customer contact/address fields; logs can contain editor content and certificate attachment paths | Default to synthetic fixtures and omission/minimization. Disclose any mapping loss; don't export precise contact, raw HTML, signature or attachment bytes in the first slice. |
| Version and license | Core and optional addons are pinned separately; inspected addons declare AGPL-3 (optional accounting addon Beta) | v16.51.0 resolves to a source commit; repository license.txt is GPLv3 | Pin exact source commit, schema/controller paths and observed license. Re-review before changing pins or distributing; this note is not legal advice. |

## Small common intake projection

The common denominator can safely contain:
1. source system, source release/commit, source namespace, source object type and stable external ID;
2. externally configured tenant binding and source company reference;
3. raw source lifecycle fields plus a versioned normalized status;
4. separately typed schedule/due date, actual work date/time, and source export/revision time;
5. site/asset/plan/task references in their source namespace;
6. attributable source claims and machine-readable mapping losses/conflicts;
7. explicit assurance boundaries stating what is not provided or not verified;
8. a privacy projection that excludes raw customer contact and evidence bytes.

The common projection must not flatten Maintenance Schedule, Visit, recurring Asset Maintenance Task and Asset Maintenance Log into a single record type. Likewise, OCA order closure and ERPNext Completed status can both map to a closed service state only if source status and claim provenance remain available; neither means quality verified.

## Conformance tests required before expanding scope

- Same source record ID in different site namespaces produces distinct IDs.
- Cross-company/site tenant binding fails closed.
- Unknown source revision or unrecognized source fields is rejected.
- Future claimed completion events are rejected; future due/scheduled dates remain valid.
- Source status and document status are both preserved and checked against the pinned controller rules.
- `maintenance_status=Cancelled` on submitted `docstatus=1` is accepted; a draft Cancelled status is distinguished from completed document cancellation.
- The mutable task last-completion field is not treated as reconstructed event history when log submission order and date order diverge.
- Date-only values remain date-only.
- Certificate/signature flag without evidence reference is disclosed; active-log requirement mismatches are conflicts, while historical log/current-task requirement drift is a temporal loss; no authenticity claim is emitted.
- Raw descriptions, contact data, signatures and certificate bytes are not present in the projection.
- The same claim boundaries hold in both adapters even where source fields differ.
- Exact-head CI covers each focused adapter separately and the common suite. A queued workflow is not a pass.

## Next recommended boundary

Continue with source-specific offline adapters rather than add live connectors. For ERPNext, keep Maintenance Visit/Schedule as a later, distinct slice with its own date/time conversion policy and partially/fully completed semantics. For OCA, a real export integration requires an independently reviewed extraction contract, credentials/tenant isolation and a separate privacy review. Neither synthetic mapper should trigger dispatch, billing, equipment operation or other physical-world side effects.


## Additional ERPNext lifecycle distinction — completed draft claim (2026-10-15)

A `Completed` maintenance status may appear on `docstatus=0` before ERPNext's submit-time restrictions execute. Preserve the source claim and document state separately, and disclose `completed_status_on_unsubmitted_document`; do not treat the projection as a clean submitted closeout. This is distinct from a submitted custom `Cancelled` state (`docstatus=1`), which the pinned controller explicitly permits.


## ERPNext record-revision scope (2026-10-15)

The mapper's `exported_at` is the extraction time, not the source record revision. This slice does not export Frappe record-level `modified` timestamps or Version history for plan/task/log and explicitly reports `record_level_modified_timestamps_not_exported`. Since Frappe timestamp interpretation depends on the configured system timezone and can differ from the DB session clock, don't silently coerce a naive source timestamp to UTC; a future implementation needs a tested, versioned extraction-time policy and a consistency/snapshot mechanism.

An `amended_from` field becomes a namespaced predecessor reference, not complete provenance. The current one-log payload does not include the predecessor, so `amendment_predecessor_record_not_in_snapshot` is emitted; self-reference is rejected. A later lineage profile should retrieve bounded ancestors and detect missing nodes/cycles before it makes any completeness claim. The canonical ERPNext snapshot rejects an empty-string `amended_from`; its exporter must represent no predecessor as `null` so blank source values cannot be mistaken for a valid absent edge.
