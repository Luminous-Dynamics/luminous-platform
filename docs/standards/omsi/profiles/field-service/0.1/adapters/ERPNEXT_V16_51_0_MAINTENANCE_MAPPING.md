# ERPNext v16.51.0 maintenance records → OMSI Field Service 0.1

**Status:** source-pinned mapping research; no live connector or import/export implementation exists  
**Research date:** 2026-10-15  
**Source release:** [frappe/erpnext v16.51.0](https://github.com/frappe/erpnext/releases/tag/v16.51.0)  
**Resolved source commit:** fd97695de7bfeb1d7728a7370c3e128d0cc1509e  
**License indicator:** repository [license.txt](https://github.com/frappe/erpnext/blob/fd97695de7bfeb1d7728a7370c3e128d0cc1509e/license.txt) is GNU GPL v3; deployment/distribution applicability needs separate review.  
**Purpose:** record a version-specific, loss-aware mapping hypothesis for a future offline synthetic adapter. This is not a compatibility, certification, legal, or production-readiness claim.

## 1. Why this is a different adapter from OCA Field Service

ERPNext's maintenance model is not a single field-service order. The pinned source contains at least three distinct records:

- **Maintenance Schedule / Detail**: planned visits, planned date, related item, assigned Sales Person and completion status.
- **Maintenance Visit / Purpose**: a customer-facing occurrence with a maintenance date/time, scheduled/unscheduled/breakdown type, partial/full completion, purpose rows, and a separate document submission status.
- **Asset Maintenance / Task / Log**: recurring asset maintenance plans and tasks, plus separately generated maintenance logs with due/completion dates and actions performed.

Do not collapse these into one universal ticket or infer that a schedule is an event, that a submitted document is fully completed, or that a completed log independently proves workmanship.

## 2. Exact source artifacts inspected

All source links below pin the resolved commit rather than a moving branch or current documentation page.

| Source artifact | Blob SHA | Fields / behavior relevant to the mapping |
|---|---|---|
| [Maintenance Visit DocType](https://github.com/frappe/erpnext/blob/fd97695de7bfeb1d7728a7370c3e128d0cc1509e/erpnext/maintenance/doctype/maintenance_visit/maintenance_visit.json) | 13f24544992b19eeb641b83ffffff259e30d08cd | company, mntc_date, mntc_time, maintenance_type, completion_status, document status, maintenance_schedule, purpose table, customer_feedback; contact/address fields exist and require minimization. |
| [Maintenance Visit controller](https://github.com/frappe/erpnext/blob/fd97695de7bfeb1d7728a7370c3e128d0cc1509e/erpnext/maintenance/doctype/maintenance_visit/maintenance_visit.py) | 31457d7491c090529fc91fce3ccbe1295bc92c37 | Uses distinct completion status vs document state; schedule rows, item/serial and customer relationships are validated; on submission it updates linked schedule progress. |
| [Maintenance Visit Purpose DocType](https://github.com/frappe/erpnext/blob/fd97695de7bfeb1d7728a7370c3e128d0cc1509e/erpnext/maintenance/doctype/maintenance_visit_purpose/maintenance_visit_purpose.json) | 70f69beeca16b1523291cb9c00dce4d2f88a776c | Item code, serial number, service-person reference, work-done claim, previous-document link and schedule-detail link. “Service Person” points to a Sales Person record, not a verified professional credential. |
| [Maintenance Schedule](https://github.com/frappe/erpnext/blob/fd97695de7bfeb1d7728a7370c3e128d0cc1509e/erpnext/maintenance/doctype/maintenance_schedule/maintenance_schedule.json) | 19919cf36f7130ac814d5119c7f2549654761c87 | Planned visit, company, customer, schedule rows and submitted/cancelled/draft state; contact/address fields are present. |
| [Maintenance Schedule Detail](https://github.com/frappe/erpnext/blob/fd97695de7bfeb1d7728a7370c3e128d0cc1509e/erpnext/maintenance/doctype/maintenance_schedule_detail/maintenance_schedule_detail.json) | c625bc3312cc0ba46052debc02a531145e05ca1f | Planned date, separate actual date and row completion status; preserves schedule intent vs reported occurrence. |
| [Asset Maintenance plan](https://github.com/frappe/erpnext/blob/fd97695de7bfeb1d7728a7370c3e128d0cc1509e/erpnext/assets/doctype/asset_maintenance/asset_maintenance.json) | b5386b40628aa9c84c5ec8762d88f1ced76261e1 | Asset and company, maintenance team/manager, child tasks. |
| [Asset Maintenance Task](https://github.com/frappe/erpnext/blob/fd97695de7bfeb1d7728a7370c3e128d0cc1509e/erpnext/assets/doctype/asset_maintenance_task/asset_maintenance_task.json) | 5405ddfdc9efa3ba66ae44bf64a1c69f45c5c766 | Maintenance task/type, planned/overdue/cancelled state, start/end dates, periodicity, assigned user, next/last completion dates and certificate_required. |
| [Asset Maintenance Log DocType](https://github.com/frappe/erpnext/blob/fd97695de7bfeb1d7728a7370c3e128d0cc1509e/erpnext/assets/doctype/asset_maintenance_log/asset_maintenance_log.json) | 3d7f821d189908773f6741dcad15c8b595febc93 | Plan/task links, due date, date-precision completion date, status, actions performed, certificate metadata/attachment, assignee fields and amendment link. |
| [Asset Maintenance Log controller](https://github.com/frappe/erpnext/blob/fd97695de7bfeb1d7728a7370c3e128d0cc1509e/erpnext/assets/doctype/asset_maintenance_log/asset_maintenance_log.py) | 140eb1af27ebad1c960f62b4b71f41841e80fc4f | Requires a completion date for Completed; rejects completion dates on non-Completed states; submission requires Completed or Cancelled and updates the recurring task. |

The source folder for Maintenance Visit in v16.51.0 is erpnext/maintenance/doctype/maintenance_visit; the older docs URL and historical erpnext/support/... path should not be used as a source pin for this release.

## 3. Mapping rules

| ERPNext source | OMSI intake projection | Required boundary |
|---|---|---|
| Site namespace + DocType + document name | Composite external reference | Namespace identifiers by an explicit ERPNext site/database identity and DocType. Human-readable name alone is not globally unique. |
| company on a Visit/Schedule/Asset Maintenance plan | Source company reference plus separately configured OMSI tenant | Require a trusted integration binding; company value is not authentication or tenant authorization. A log may require tenant/company context to be resolved through its linked plan and checked against the binding. |
| Schedule/Detail scheduled_date, actual_date | Distinct planned vs reported-actual date fields | Preserve source date precision (date, not timestamp); never invent a time or timezone. If a later adapter compares an event date with a clock, use an explicit IANA site timezone from trusted integration configuration, validate it, and convert the evaluation instant to that zone before deriving the local date. |
| Visit mntc_date, mntc_time | Visit-reported date and optional local time | Preserve whether time is absent and use a separately documented site timezone policy before any conversion. The editable date/time are source claims, not trusted clock attestation. |
| Visit completion_status (Partially Completed / Fully Completed) | Visit completion claim | Keep separate from Visit document status (Draft / Cancelled / Submitted). Submission is not equivalent to full completion. |
| Visit Purpose item_code, serial_no, service_person | Item/asset and source practitioner references | Preserve source-scoped IDs. Sales Person / User assignment is not credential status, licensure or skill proof. |
| Visit Purpose work_done; log actions_performed | Attributed provider-reported work claim | Do not promote text into verified evidence. Treat editor/HTML content as untrusted; do not export raw markup by default. |
| Asset Maintenance company, asset_name, child tasks | Maintenance plan and task references | Planning intent and periodicity are not evidence that a task occurred. |
| Log maintenance_status, completion_date, due_date | Maintenance occurrence state with date-only completion evidence | Preserve Planned/Completed/Cancelled/Overdue separately. `Overdue` means overdue, not blocked; do not collapse it into an execution-blocked state. The pinned log controller changes past-due logs that are not Completed/Cancelled to Overdue during validation. If a snapshot still says Planned while its due date precedes the configured site-local date, emit a mapping conflict; preserve rather than silently rewrite the source status. A Completed log requires a completion date in the inspected controller; retain any inconsistent imported snapshot as a conflict or reject it. |
| Log document status (Frappe docstatus) + maintenance_status | Two distinct source lifecycle dimensions | The pinned controller's `on_submit()` explicitly permits `maintenance_status` Completed or Cancelled on `docstatus=1`; a submitted custom Cancelled state is therefore valid and must not be confused with Frappe document cancellation (`docstatus=2`). A draft (`docstatus=0`) already marked Cancelled is an unsubmitted cancellation claim and is disclosed as a conflict. If `docstatus=2` but the custom status is not Cancelled, normalize intake conservatively to cancelled while preserving both source fields and emitting a conflict. |
| Task certificate_required; Log has_certificate; Log certificate_attachement | Three separate concepts: current task requirement, log-specific boolean, attachment reference/presence | The pinned plan controller copies the task requirement into the log flag when a log is created or refreshed, but refreshes only Planned/Overdue logs. Completed/Cancelled logs are not refreshed by that helper, so the task requirement may have changed since a historical log. Treat this historical difference as a temporal mapping loss, not automatically as a conflict; a mismatched Planned/Overdue flag is a conflict against the current sync expectation. In all cases, the boolean is not proof that an attachment exists or is authentic. |
| Visit customer_feedback | Customer-reported feedback, when explicitly in scope | A feedback string is a claim, not approval of exact scope or independent acceptance. Prefer omission/minimization for the first adapter. |
| Task next_due_date vs Log due_date | Separate recurring-plan follow-up date and occurrence-level due date | Preserve both date-only fields separately. The task's next due date must not overwrite or be inferred from this log's due date. |
| Task last_completion_date vs submitted Log completion_date | Current task state vs occurrence-level completion claim | The pinned controller sets the task field to this log's completion date whenever the values differ; it does not compare calendar ordering. Submitting an older log after a newer-dated log can move the task field backward. Preserve both values, disclose that event ordering is not reconstructed, and do not treat the task field as an immutable event ledger. |
| amendment links and source document status | Source lineage/conflict metadata | Preserve links when allowed; a mutable business record is not an immutable event ledger. Capture source revision/export time and disclose missing event history. |

## 4. Privacy and assurance requirements

- Exclude customer names, phone, email, address display, contact display and contact-person IDs by default. Use synthetic records only during conformance development.
- Preserve due/completion/next-task due values as date-only fields. The adapter's trusted tenant binding carries a validated IANA `site_timezone`; future-completion checks convert the timezone-aware evaluation clock into that zone before deriving today's date. Reject unknown/invalid timezone IDs rather than treating the caller's timezone as the source site's timezone. Do not fabricate UTC timestamps from date-only values.
- The pinned Asset Maintenance Log controller promotes a past-due non-Completed/non-Cancelled log to `Overdue` during validation. A still-`Planned` snapshot with a past-due date is treated as a conflict, not silently reclassified. This depends on the configured site timezone matching the source site's date behavior; neither the time zone binding nor the source clock is independently attested here.
- The pinned controller updates `Asset Maintenance Task.last_completion_date` to a submitted log's `completion_date` whenever the values differ; it does not require the new date to be later. Submission order can therefore diverge from calendar order. The projection preserves the task date as source state and records `task_last_completion_date_is_not_reconstructed_from_event_history`; it must not be treated as a reconstructed or complete event chronology.
- Reject unsupported DocType/source revisions and unrecognized fields. Unknown or contradictory status combinations must not be normalized into verified, compliant, or closed without loss/conflict disclosure.
- Distinguish a provider's claim of work, a visit's partial/full completion, document submission, a completed maintenance log, and actual independent verification.
- `certificate_required`, log `has_certificate` and attachment reference are distinct fields with different temporal scope. The pinned plan controller refreshes the flag only for Planned/Overdue logs; a current task requirement can differ legitimately from a historical Completed/Cancelled log, so classify that as a temporal mapping loss. For active Planned/Overdue logs a mismatch is a source-state conflict. Neither flag nor attachment reference proves certificate authenticity, scope, issuer trust, professional qualification, regulator compliance, or workmanship quality.
- Do not expose raw editor content, attached certificate bytes or contact records in the initial projection. If a later profile adds evidence references, use an explicit privacy/authorization policy and content digest/provenance contract.

## 5. Prototype acceptance checklist

Before any adapter implementation is described as passing:

- [ ] Source schemas and mapper pins point to the exact commit above and document their reviewed fields.
- [ ] Positive synthetic fixtures cover a partially completed and a fully completed Maintenance Visit, plus Planned and Completed Asset Maintenance Logs.
- [ ] Negative fixtures cover company/tenant mismatch, site namespace mismatch, future-dated completed event, missing completion date, active-log certificate-flag mismatch, historical log/current-task requirement drift, unknown fields, source revision drift and overclaimed verification.
- [ ] A task last-completion date that predates a submitted log is preserved without inventing an ordering rule, and the projection discloses that the current task field is not reconstructed event history.
- [ ] Visit document status and completion status are tested independently.
- [ ] Schedule intent, actual visit date, recurring task and maintenance log remain distinct records.
- [ ] The suite has a dedicated exact-head CI step with an explicit non-empty test-count floor.
- [ ] Tests are not reported as proof of live ERPNext interoperability, credential verification, certificate authenticity, legal compliance or field quality.

## 6. Public references

- [ERPNext Maintenance Visit documentation](https://docs.frappe.io/erpnext/maintenance-visit)
- [ERPNext Asset Maintenance documentation](https://docs.frappe.io/erpnext/asset-maintenance)
- [ERPNext Asset Maintenance Log documentation](https://docs.frappe.io/erpnext/asset-maintenance-log)

Documentation provides workflow context; the commit-pinned DocType JSON and controllers above govern this adapter research.


## Additional lifecycle edge case — completed draft status (2026-10-15)

The pinned controller's `validate()` checks completion-date consistency, while its `on_submit()` separately restricts submission to `Completed` or `Cancelled`. Therefore a draft (`docstatus=0`) can carry `maintenance_status=Completed` before the submit-time restriction has run. This remains a source status claim, but not a clean submitted maintenance record. The adapter preserves the raw status/docstatus, keeps normalized status plus `completed_status_on_unsubmitted_document` conflict disclosure, and does not upgrade workmanship assurance. The acceptance suite includes this negative case.


## Revision and amendment lineage boundary (2026-10-15)

This synthetic snapshot contract records the source code pin and snapshot `exported_at`, but it does not export record-level `creation`/`modified` timestamps or the Frappe Version/change history for the plan, task, or log. Therefore `exported_at` is extraction metadata, not a per-record revision identifier, source snapshot token, or proof that the related records were read atomically. The mapper emits `record_level_modified_timestamps_not_exported` and does not claim stale-source detection or full revision lineage. Before adding those timestamps to a live adapter, define and test the exact Frappe-version timestamp format and configured timezone policy; do not blindly interpret naive timestamps as UTC.

When `amended_from` is populated, the output retains a namespaced reference to the predecessor but the current single-log snapshot does not include that predecessor. The adapter emits `amendment_predecessor_record_not_in_snapshot`; the link remains a source claim, not proof the target exists, is unchanged, or completes the chain. A self-reference is rejected as an invalid lineage edge. A future lineage slice should fetch a bounded predecessor chain under the same tenant/site binding, preserve each record's source state, detect cycles and missing nodes, and report truncation explicitly rather than claim full history. The canonical snapshot schema accepts only a non-empty string or `null` for `amended_from`; an empty string is rejected. An exporter must normalize an absent optional link to `null` rather than silently collapse a blank value into absence.
