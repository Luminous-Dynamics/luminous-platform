# OCA Field Service 19.0 → OMSI Field Service 0.1 mapping

**Status:** research adapter note; no live connector or import/export implementation exists  
**Research date:** 2026-10-15  
**OCA source branch:** `OCA/field-service:19.0`  
**Pinned source commit:** [`8b2fa2592dda401726834ccf8030d34e241a69ad`](https://github.com/OCA/field-service/commit/8b2fa2592dda401726834ccf8030d34e241a69ad)  
**Core addon version/license:** `fieldservice 19.0.1.2.3`, AGPL-3  
**Optional addons inspected:** `fieldservice_activity 19.0.1.0.1`, AGPL-3; `fieldservice_account 19.0.1.0.0`, AGPL-3, maturity declared Beta  
**Purpose:** define a loss-aware mapping target for a read-only adapter experiment. This document is not proof of round-trip interoperability, license compatibility with a particular deployment, or production readiness.

## 1. Why this is a useful first real adapter

The OCA repository's current `19.0` branch includes a mature core Field Service addon. Its `fsm.order` model has explicit assigned workers, teams, locations, scheduling vs actual start/end, configurable stages, equipment relations, work description/resolution, and optional customer-signature capture. The separate Activity addon models required checklist-like activities. The separate Accounting addon links invoices to field-service orders. This is concrete prior art for a trades workflow; OMSI should wrap those records with operation/authority/evidence semantics instead of rebuilding the whole service manager.

Pinned source artifacts consulted:

- [Core addon manifest](https://github.com/OCA/field-service/blob/8b2fa2592dda401726834ccf8030d34e241a69ad/fieldservice/__manifest__.py) — blob `ad35242f87ff7d32403d3dda7edbc884171bb726`.
- [Order model](https://github.com/OCA/field-service/blob/8b2fa2592dda401726834ccf8030d34e241a69ad/fieldservice/models/fsm_order.py) — blob `469aac02eb967179a4bf01ff5a769b478f3eaee6`.
- [Stage model](https://github.com/OCA/field-service/blob/8b2fa2592dda401726834ccf8030d34e241a69ad/fieldservice/models/fsm_stage.py) — blob `7f71fc9a755b42650ccd4a84740794ed91d76bc1`.
- [Location model](https://github.com/OCA/field-service/blob/8b2fa2592dda401726834ccf8030d34e241a69ad/fieldservice/models/fsm_location.py) — blob `60d3fd1bf24dcc5fc0206a9a86e4a2c57373329e`.
- [Equipment model](https://github.com/OCA/field-service/blob/8b2fa2592dda401726834ccf8030d34e241a69ad/fieldservice/models/fsm_equipment.py) — blob `0e397bcbf0e41168436fd5f170f20fea637de471`.
- [Activity model](https://github.com/OCA/field-service/blob/8b2fa2592dda401726834ccf8030d34e241a69ad/fieldservice_activity/models/fsm_activity.py) — blob `8a21a4c52fa690bcb796a92e445dced4b3ae6544`.
- [Accounting addon model](https://github.com/OCA/field-service/blob/8b2fa2592dda401726834ccf8030d34e241a69ad/fieldservice_account/models/account_move.py) — blob `75f0c9f5872be27ac4138f327ed9dc70db422aa4`.

Blob SHAs are source pins for the files inspected; the branch commit and addon manifests must be rechecked before an implementation is released.

## 2. Mapping table

| OCA model / field | OMSI 0.1 target | Required treatment |
|---|---|---|
| `fsm.order.id` plus Odoo instance/database namespace | `service_case.source_system` + `external_id` | Use stable composite source identity. Do not treat the display name as a unique global ID. |
| `fsm.order.name` | External human-readable identifier | Preserve exactly as a display label; not a primary interoperability key. |
| `fsm.order.company_id` | Provider/company context | **Not** proof of OMSI tenant membership or authority. Bind OMSI `tenant_id` from authenticated integration configuration, and keep the source company ID as a separate reference. |
| `fsm.order.stage_id`, `stage_name`, `is_closed` | `service_case.source_status`, `normalized_status`, mapping profile | Preserve source stage ID and label, `fsm.stage.stage_type`, `is_closed`, `sequence`, `require_signature`, and adapter mapping version. Stages are configurable; do not map only by translated label or assume “closed” means independently verified. |
| `fsm.order.priority` | Priority extension / source priority | Source values are `0` Normal, `1` Low, `2` High, `3` Urgent in the inspected stage code. Keep raw value and mapping version; no universal SLA is implied. |
| `fsm.order.location_id` | `site` reference | Preserve source ID and instance namespace. The location model delegates to `res.partner` and contains owner/contact, address, parent-child location hierarchy and directions. Do not export precise residential directions or contact data by default. |
| `fsm.order.equipment_ids`; `fsm.equipment` | `asset` references | Preserve every equipment reference with source namespace and ID. Core has equipment name, assigned worker/location, current location, ownership/management references, parent-child equipment and company. Do not infer manufacturer/model/serial fields that are absent from the inspected core model. |
| `fsm.order.person_id`, `person_ids`, `team_id` | Practitioner/crew and assignment references | Preserve assigned worker and all worker IDs separately, plus team. Worker records do not by themselves prove a trade license, credential scope, or current regulatory status. |
| `request_early`, `request_late` | Requested time window | Do not call these the scheduled or actual visit times. Preserve both source fields and the declared time interpretation. |
| `scheduled_date_start`, `scheduled_date_end`, `scheduled_duration` | Scheduled appointment | Keep separate from actual work times. A schedule is not execution evidence. |
| `date_start`, `date_end`, `duration`, `current_date` | Actual-work temporal claims | Preserve source values, raw precision and mapping metadata. Computed duration is not an independently observed timesheet. Require explicit timezone conversion rules in the connector; do not silently assign local timezones. |
| `description`, `todo`, `resolution`, `location_directions` | Work scope, instructions, completion statement and private site details | Treat as untrusted source content; sanitize HTML for rendering and keep raw/source digest only where justified. Do not treat `resolution` as independent verification. Redact credentials, access codes, personal details and security-sensitive layout data. |
| `signed_by`, `signed_on`, `signature`, `require_signature` via stage | Customer-signature metadata | Keep the signatory, timestamp, source stage policy, and signature artifact in restricted storage. A recorded signature or image does not prove that an estimate version was accepted before work, nor does it prove the work passed. Preserve a digest/reference and access policy, not the raw image in a replicated ledger. |
| Activity addon `fsm.activity.required`, `state`, `completed_on`, `completed_by`, `ref`, `fsm_order_id` | Checklist/task activity and provider-reported completion evidence | Include only if the addon is installed. Keep required vs optional, todo/done/cancelled, actor and time. A completed activity is a source-system claim; it is not a calibrated measurement or independent inspection. The source `action_complete` rejects completion while a required activity remains todo, but the adapter must not infer more than this implementation enforces. |
| Accounting addon `account.move.fsm_order_ids` / computed relation from invoice lines | Invoice-document reference | Include only if `fieldservice_account` is installed and the account integration is enabled. It links invoices with orders; it does not establish a universal quote/estimate schema or payment settlement truth. |
| `mail.thread` / chatter and stage changes | Source event/history references | Do not assume that one current snapshot contains a complete tamper-evident event chain. Fetch and preserve the actual available messages/events with source IDs and retrieval times, or mark history unavailable. |

## 3. Important non-equivalences

1. **Company is not tenant authority.** Odoo's `company_id` is a business record. OMSI authorization must be derived from authenticated principal, integration registration, tenant policy and object access controls.
2. **Configurable stage is not an OMSI state machine.** Keep both raw source stage and normalized status. A mapping must disclose ambiguity and preserve unknown stages instead of guessing.
3. **Signature is not estimate approval.** Customer sign-off may be requested on a stage and captured as a signature image. The inspected core module does not itself define the OMSI estimate version/scope-digest approval contract.
4. **Checklist completion is not independent verification.** Activity state is useful operational evidence, but does not automatically establish instrument calibration, qualified inspection, or verified postconditions.
5. **Worker record is not credential status.** A worker/category/territory relationship is not an issuer-backed, jurisdiction-bound, current professional credential. OMSI must fetch/validate a separate credential claim or leave it unknown.
6. **Invoice link is not payment confirmation or service quality.** Keep invoice existence, invoice status/payment data (if available from the accounting API), service completion, and independent verification as separate claims.

## 4. Read-only connector acceptance tests

Before import/export implementation, capture a synthetic fixture from the pinned OCA version and declare its source module/field manifest. The adapter must be read-only in this phase and must not create or mutate Odoo records.

Required tests:

- Same order `name` in two Odoo instances never collides after namespacing.
- Changing `company_id` does not silently change the authenticated OMSI tenant.
- Unknown/custom stage preserves source stage ID/name and marks normalized status `unmapped` or mapping loss explicitly; it must not become `verified`.
- A stage with `is_closed=true` never promotes the OMSI operation to `verified`.
- Requested, scheduled and actual timestamps stay separate; missing timezone or ambiguous conversion fails closed for temporal claims.
- Assigned worker and `person_ids` are preserved, but missing external credential records produce `unknown`, not `qualified`.
- Required and optional activities retain order, current state, completion actor/time; missing Activity addon is explicitly declared.
- A signature image is not exported into an unrestricted evidence store; signature metadata cannot satisfy an approval for a different estimate/scope version.
- Equipment lists, parent-child references and location hierarchy round-trip without replacing stable source IDs with names.
- Invoice relation stays distinct from payment settlement and verified service outcome.
- HTML, private addresses, directions and phones are redacted or access-controlled according to a documented field-level policy.
- Export/import round trip preserves all source fields known to the adapter or emits a machine-detectable loss record.

All fixtures remain synthetic. Actual API authentication, record rules, module installation, Odoo version, customer deployment settings and licensing constraints need separate validation before a live test.

## 5. License and deployment note

The inspected OCA manifests declare AGPL-3 for the core `fieldservice` addon and the optional activity/accounting addons. The accounting addon declares Beta maturity in the inspected manifest, while core declares Production/Stable. This does not constitute legal advice: before linking, bundling, hosting, or redistributing an adapter with these modules, review the exact addon versions, dependency graph, deployment model and applicable license obligations. A separate process boundary/HTTP adapter may have different engineering characteristics from linking code, but does not eliminate the need for legal review.

## 6. Decision

Proceed with a **read-only adapter prototype and synthetic export fixtures**, not a native Odoo replacement and not write-back. This will test the highest-value hypothesis—loss-aware exchange of service case, assignment, scope, activity and evidence semantics—without creating production side effects. Upgrade the gate only after source/version-pinned fixtures and round-trip tests exist.
