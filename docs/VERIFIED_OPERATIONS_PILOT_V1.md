# Verified Operations Companion — synthetic pilot contract v1

**Status:** design-stage conformance fixture. Synthetic only. Not a runtime authorization contract, provider certification, or evidence that a production adapter works.

## Purpose

Turn the Home/SMB product decision into a small falsifiable contract before selecting a real customer connector. The corpus is intended to test one question: does the platform preserve the difference between observed, inferred, authorized, executed, and independently verified?

The schema and fixtures live at:

- `profiles/verified-operations/pilot-v1.schema.json`
- `profiles/verified-operations/synthetic-pilot-v1.json`

The corpus is deliberately vendor-neutral. It does not invent a customer, use live credentials, or assume a particular backup vendor. The selected pilot adapter must be based on the real inventory and supported interfaces of a consenting pilot customer.

## Research implications

The competitive baseline has moved beyond a conventional RMM dashboard. Tactical RMM already advertises inventory, monitoring, reporting, scripting, patching, and remote access; its license is source-available with restrictions and is not an OSI open-source license. ConnectWise announced its unified Predictive Intelligence / AI-native MSP platform in June 2026 and general availability in July 2026. Therefore, Luminous should not claim uniqueness for “AI-powered IT operations.”

The narrower hypothesis to validate is that small IT teams and MSPs will pay for portable, source-bound evidence and verified recovery outcomes that can coexist with incumbent PSA/RMM tools. That hypothesis has not been validated with customers.

For households, Home Assistant already provides native energy dashboards for electricity, gas, water, production and storage. The first differentiator should therefore be measurement integrity, health and resilience, and cautious recommendations around the existing automation engine—not another duplicate charting surface.

References (checked 2026-10-15):

- NIST CSF 2.0 Small Business Quick-Start Guide: <https://csrc.nist.gov/pubs/sp/1300/final>
- Tactical RMM features: <https://tacticalrmm.com/features/>
- Tactical RMM license: <https://docs.tacticalrmm.com/license/>
- ConnectWise platform launch: <https://www.connectwise.com/company/press/releases/the-reactive-support-era-is-over-connectwise-platform-is-now-live-empowering-partners-in-journey-to-predictive-intelligence>
- Home Assistant energy management: <https://www.home-assistant.io/docs/energy/>
- Home Assistant Matter integration and supported configuration constraints: <https://www.home-assistant.io/integrations/matter/>
- Matter 1.5 announcement: <https://csa-iot.org/newsroom/matter-1-5-introduces-cameras-closures-and-enhanced-energy-management-capabilities/>

## Contract rules

1. `mode` must remain `synthetic_only` in this fixture corpus. It contains no live target identifiers or credentials.
2. Every scenario specifies the expected outcome and a stable reason code. Scenario IDs must be unique; a consumer test must reject duplicate IDs even though JSON Schema alone cannot guarantee uniqueness across a property of objects.
3. All cases set `automatic_mutation_allowed` to `false`. This corpus qualifies no action executor.
4. Missing, stale, invalid, contradictory, revoked, out-of-scope, or unverified states cannot be promoted to healthy or verified.
5. A successful backup job without an actual current restore test must classify as `recovery_not_demonstrated`.
6. Event identity reuse with different content must quarantine the conflict and preserve the original event record. Identical replay must be idempotently deduplicated.
7. A command dispatch, timeout, or executor assertion alone is not proof of effect. The system must represent unknown and partial outcomes distinctly and reconcile state before retrying uncertain non-idempotent operations.
8. An independently verified result is a necessary contract condition in these fixtures, not a claim that the verifier implementation or underlying source evidence is trustworthy by itself.
9. The actual product must bind observations to tenant/site, target identity, source identity and revision, source event time, receive time, declared freshness bound, collection method, and verifier version. The fixture schema stays intentionally compact; it is not the canonical observation schema and must not compete with the existing typed observation/effect-receipt work.
10. This profile must compose with the existing Work Case, Operations Event, typed observation evidence, recovery lifecycle and idempotency contracts rather than define parallel implementations.

## Required conformance checks

- Parse the schema under JSON Schema Draft 2020-12 and validate all fixtures against it.
- Reject malformed JSON, an unsupported contract version, a non-synthetic mode, missing reason codes, extra fields and duplicate scenario IDs.
- Execute scenario expectations through the consumer contract tests; merely validating fixture structure does not prove semantics.
- Add metamorphic tests that change one critical fact at a time (for example fresh→stale, valid→revoked, independently verified→executor assertion only) and assert the classification does not become safer.
- Preserve negative-control coverage for false-green observations, false recovery confidence, expired approval, stale pre-state, self-certified actions, conflicting event identity and partial effect.
- Record the exact tested source SHA, toolchain/dependency lock, corpus digest, discovered case count, executed case count, and per-case result. Missing or empty test discovery is failure; queued, skipped, or absent CI is not pass evidence.
- Keep documentation/schema validation separate from adapter integration and physical/customer pilot evidence.

## Mapping a real pilot without widening authority

1. Inventory the one pilot site and agree its scope, data retention, operator roles, critical services, existing systems, and how to revoke the integration.
2. Choose one actual connector based on supported API/protocols, source quality, customer permission and deployment feasibility. Synology SNMPv3 is an existing read-only candidate in platform issue #37, but it only exposes the configured allowlisted MIB observations; it does not automatically provide restore-test proof.
3. Run read-only collection first. Do not deploy agents, run remote shell, push patches, modify network policy, flash firmware or change credentials in this pilot slice.
4. Map findings into the existing Work Case contract with provenance and a clear indeterminate state. Any external ticket write requires separately qualified durable inbox/outbox semantics, tenant scoping, and explicit customer approval.
5. Measure onboarding time, in-scope asset coverage against an independently established inventory, data freshness, actionable-finding precision, unresolved/indeterminate frequency, analyst minutes, evidence completeness, and weekly operator burden.
6. For backup assurance, collect backup-job telemetry and a separately conducted restore-test record with target, test time, scope, result and reviewer. Do not claim a restore test was performed unless its evidence exists.
7. Only after repeatable lab evidence and a customer-approved review may a later tranche consider a narrow, reversible action. That tranche must add an action-digest-bound authorization, fresh pre-state, idempotency classification, durable effect journal, independent verifier and reconciliation/rollback plan.

## Explicit nonclaims

The corpus is authored, not CI-qualified here. No production adapter, live device, backup restore, customer environment, security certification, savings, downtime reduction or paid demand is claimed. The Rust-first schema and semantic conformance runner is now included in this PR at `tools/verified-operations-conformance/`, with an exact-path GitHub Actions workflow and metamorphic/negative-control tests. Before any maturity promotion, require its workflow to pass on the exact PR head and add a committed Cargo.lock so transitive dependency resolution is reproducible. Do not relax negative controls to obtain a pass.
