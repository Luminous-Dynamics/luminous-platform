# Jurisdiction-pack and deterministic workflow engine contract v0.1

**Status:** Proposed implementation contract  
**Date:** 2026-10-13  
**Related artifacts:** [Pack schema](../../schemas/jurisdiction-pack-v0.1.0.schema.json), [Johannesburg draft pack](../../examples/jurisdiction-packs/za-city-of-johannesburg-food-service-0.1.0.json), [licensing ADR](adr/0001-licensing-and-content-rights.md).

## Objective

Turn jurisdiction-specific, source-grounded guidance into deterministic, testable workflow data without allowing the UI, an LLM, a user checkbox, infrastructure recovery, or missing network dependencies to assert a legal outcome.

The initial implementation is a **guidance navigator and preparation tool**. It is not a legal authority, permit issuer, identity provider, or filing agent.

## Separate the state machines

Never use one overloaded **complete** boolean for all of the following. They describe different things and must be stored and shown separately.

| State domain | Example values | Authority |
|---|---|---|
| Pack publication | draft, candidate, published, suspended, retired | Pack review/publishing process |
| Source evidence | unreviewed, current, changed, stale, unavailable, disputed, withdrawn | Evidence reviewer and source monitor |
| Requirement review | draft, verified, stale, disputed, withdrawn | Named human reviewer |
| User progress | not_started, in_progress, prepared, user_reports_submitted, awaiting_authority | User assertion, explicitly labeled |
| Authority outcome | unknown, receipt_verified, approved, refused, withdrawn | Responsible authority, via verifiable evidence |

A UI should say **“marked prepared by you”** or **“you reported submitting this”**, not “licensed,” “approved,” or “legally compliant.” The application may use “receipt verified” only after an authentic, source-bound authority receipt is validated. “Approved” and “refused” require an explicit authority decision; a receipt of submission is not an approval.

## Publication and evaluation gates

A future published pack must satisfy all of these deterministic checks:

1. The document validates against the exact schema version declared in schema_version.
2. pack_id, requirement IDs, action IDs, and source IDs are unique within the pack.
3. Every source_refs value resolves to exactly one source record.
4. Every requirement that presents an official requirement or guidance statement has at least one relevant source record.
5. Each claim presented as current has a source with review_state=current, a recorded checked_at, a human reviewed_at, a reviewer_role, and a future next_review_at.
6. No required claim depends on a source that is unreviewed, changed, stale, unavailable, disputed, or withdrawn. A pack may be published in a limited guidance mode only if unresolved claims are explicitly downgraded to “verify with the authority” and cannot satisfy completion gates.
7. All applicability and jurisdiction scopes are explicit. An unknown locality, premise type, or business model must not silently inherit a neighboring jurisdiction's rules.
8. Source reuse rights are recorded independently of source accuracy. Rights that are not_assessed or link_only prohibit copying source text into a redistributable pack; links and attribution may be shown subject to the relevant site terms.
9. The pack has a maintainer role, review interval, changelog entry, and reproducible validation evidence.

JSON Schema validation is necessary but not sufficient: cross-record references, uniqueness, date gates, status transitions, jurisdiction matching, source rights, and publication policy need semantic validator tests.

## Source review receipts and append-only provenance

The [source review protocol v0.1](source-review-protocol-v0.1.md) defines both source and claim receipt formats and their limits. A non-`unreviewed` source must point to its latest source receipt; active metadata must match that receipt. A non-`draft` requirement must point to its latest claim review receipt, whose SHA-256 binds the exact title, claim type, summary, applicability, source references, user actions, and completion effect. A verified claim receipt must identify every current source receipt it relied on and explicitly check source support, jurisdiction, applicability, user-facing language, and completion effect.

Pull-request CI rejects editing, removing, or reordering either receipt ledger already present in the base pack. This does not authenticate an identity, prevent an administrator rewriting history, or prove a review occurred. Repository branch protection and independent human approval remain release controls. Captured source bytes need a lawful, access-controlled evidence store; a digest alone cannot reproduce the capture. The Johannesburg fixture deliberately contains no receipts and remains unpublished.

## Applicability predicates

Machine-evaluable applicability is limited to the [applicability predicate contract v0.1](applicability-predicate-contract-v0.1.md). Do not interpret free-text `conditions` or `unknowns` as code. Predicate evaluation uses exact type checks and three-valued outcomes: false excludes a candidate, unknown requires scope clarification, and only all-true predicates advance to later review/evidence gates. Malformed fact context fails closed with no cards.

`applicable` is only a deterministic match against the supplied facts, not a legal conclusion. The workflow engine must keep pack/source review state, user assertions, and official authority outcomes as separate state domains.

## Deterministic execution

1. Resolve the exact jurisdiction and business type selected by the user. If a pack cannot establish a fit, return needs_scope_confirmation; do not guess.
2. Evaluate only declarative, versioned applicability predicates. Predicates must be data, not arbitrary code supplied by a pack or a model.
3. Return applicable guidance together with its requirement ID, pack version, source IDs, review state, and next action.
4. If supporting evidence is missing, stale, disputed, or outside its scope, downgrade the card to “needs verification” and block any derived completion claim.
5. Preserve user drafts and progress when a source or integration is unavailable. Display the limitation and official contact/link. Do not treat infrastructure fail-open as civic authorization fail-open.
6. Record authority receipts separately. Validate issuer, signature or other trustworthy authenticity mechanism, subject/purpose binding, time, and correlation to the relevant workflow before changing an authority-outcome state.
7. Keep Symthaea outside the rule authority path. It may explain a rule using retrieved source IDs, suggest questions, identify missing context, or state uncertainty. It cannot invent or alter requirements, approve a state transition, or override a validator.
8. Keep Mycelix optional. An acknowledgement between participants is evidence of that acknowledgement—not proof of a legal status or civil identity unless a responsible, authorized issuer provides a suitable attestation.
9. Core public guidance, saved drafts, and exports must continue to work if Symthaea, Mycelix, or an official external service is unavailable.

## Minimum transition rules

- not_started → in_progress → prepared changes only user progress.
- prepared → user_reports_submitted records a user claim; it never creates an authority receipt.
- user_reports_submitted → awaiting_authority indicates that a response is expected; timeout or outage leaves it awaiting/unknown.
- unknown → receipt_verified requires a validated receipt from the relevant authority or an explicitly designated trusted channel.
- receipt_verified → approved/refused requires a separate authority decision record; submission acknowledgement alone cannot take this transition.
- Evidence-state changes do not silently rewrite past user actions. Instead, current guidance is recalculated and prior decisions remain auditable with the pack version used at the time.
- suspended or retired pack versions may remain available for audit/export but cannot support new current-compliance claims.

## Required adversarial and regression tests

| Fixture | Expected result |
|---|---|
| User checks every checklist item, no authority receipts exist | UI reports user-marked progress only; no licence/approval/compliance claim |
| One required source is stale past its review deadline | Affected requirement becomes stale/verify; completion gate blocks |
| Source URL changes or content fingerprint changes | Create a review event; do not auto-approve a revised claim |
| Conflicting official sources are present | Requirement is disputed and routed to human review; no silent source preference |
| AI proposes a requirement ID absent from the pack | Reject it as a new obligation; model can suggest a question only |
| User selects a different municipality or unknown business type | Stop applicability evaluation and request scope clarification |
| External authority or identity service is unavailable | Preserve draft; no submission or decision state is inferred |
| Receipt has wrong issuer, workflow ID, subject, signature, or purpose | Reject receipt and preserve existing state |
| Submission receipt exists but decision does not | State may be receipt_verified or awaiting_authority, never approved |
| Pack schema version is unsupported or exact validation fails | Do not publish or execute the pack |
| Source rights are not_assessed and content includes copied source paragraphs | Validation fails until text is removed or redistribution rights are established |
| Platform recovers after a host fault | Restore operational service only; do not alter civic evidence, authorization, or completion states |

## MVP acceptance criteria

- Publish no live pack from the current draft fixture.
- Validate schema shape plus semantic constraints in CI.
- Include an explicit negative test for each fail-closed rule above.
- Emit a machine-readable validation report with pack ID/version, schema version, validator version, input digest, check IDs, and pass/fail results.
- Make each displayed official requirement traceable to a source record and reviewed pack version.
- Expose an exportable user progress record that distinguishes user assertions from verified external evidence.
- Add neither identity-document capture nor government submission APIs in this milestone.
- Treat a missing reviewer, missing current-source timestamp, unresolved source conflict, or expired review date as a failure for current legal claims.

**Non-goal for v0.1:** automatically scraping legal sources and promoting changes. Monitoring can detect candidate changes, but publication remains a reviewed operation.
