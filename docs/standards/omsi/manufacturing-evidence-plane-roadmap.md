# OMSI Manufacturing Evidence Plane: Paradigm-Shift Roadmap

**Status:** research and architecture proposal; not a normative wire standard  
**Scope:** next evolution of the Manufacturing Asset Operations 0.1 incubation profile  
**Current implementation boundary:** diagnostic semantic validation only; no authenticated origin, product-release authority, or equipment-control authority

## Thesis: from record validation to portable evidence

Traditional manufacturing integration commonly moves records between MES, QMS, CMMS, ERP, metrology tools, and equipment gateways. Each interface may be syntactically valid while still losing the semantics needed to decide whether a claim is trustworthy: source revision, measurement conditions, calibration coverage, identity scope, uncertainty, and the rule used to interpret it.

OMSI should evolve into a **manufacturing evidence plane**: a small, interoperable contract for preserving claims, expressing derived results, exposing evidence gaps, and replaying the same named rule over the same evidence. It should not become another all-owning MES or a machine-control service.

The core shift is from:

- **Pass/fail integration** to structured, independently checkable claims and conflicts.
- **Vendor-shaped records** to explicit, versioned semantics with source meaning preserved.
- **One opaque confidence score** to a vector of independent evidence dimensions and hard blockers.
- **An unexplained result** to a replayable decision trace bound to exact inputs, profile, rule and implementation.
- **A universal hidden rule** to selectable, versioned decision rules with explicit owners and risk assumptions.
- **Bespoke bilateral integrations** to reusable profile adapters and public conformance vectors.
- **Trust by platform assertion** to verifiable, scoped evidence that a consumer can check locally.

These are architectural targets. The current pilot implements only a bounded subset.

## 1. Evidence graph, not a flattened status field

Represent a conclusion as a small graph of typed claims rather than a single mutable status. At minimum distinguish:

- **Entities:** observation, asset, work order, specification revision, instrument, calibration record, lot or part.
- **Activities:** measurement, calibration, data extraction, normalization, comparison, review and release.
- **Agents:** source system, calibration laboratory, adapter, rule implementation, reviewer and authorized decision-maker.
- **Derivation edges:** used, generated, derived from, attributed to, supersedes and conflicts with.

Each claim must retain its origin reference and source-revision metadata independently from the time at which a connector exported it. Unknown provenance stays unknown; timestamps from different clocks must not be promoted into causal proof.

Use W3C PROV-O as a conceptual reference for the entity/activity/agent distinction, without claiming that the current report is a PROV-O implementation. A future profile may map OMSI claim and derivation records to PROV-O terms after field semantics and identifiers are reviewed.

## 2. Evidence is a vector, not a scalar

Do not collapse every assurance dimension into one number. A highly precise measurement can still have unauthenticated origin; an authenticated source can still report an expired calibration; a valid calibration interval does not prove the instrument was used correctly.

A future report should expose separate, typed dimensions such as:

| Dimension | Example states | What it does not prove |
|---|---|---|
| Input contract | valid / invalid / unsupported-version | That the claim is true |
| Tenant and source scope | matched / mismatch / not-established | That the binding issuer is authentic |
| Origin authenticity | verified / failed / not-performed | Measurement accuracy |
| Revision and history | revision-known / absent / history-incomplete | Freshness unless a policy defines it |
| Snapshot semantics | atomic / bounded / per-record-non-atomic / unknown | Global consistency by assumption |
| Metrology traceability | evidence-present / incomplete / unauthenticated | Physical truth or instrument fitness |
| Calibration coverage | interval-covers-event / does-not-cover / unknown | Certificate authenticity or correct use |
| Uncertainty semantics | specified / incomplete / incompatible / absent | Conformity unless a named rule consumes it |
| Decision-rule applicability | applicable / blocked / not-evaluable | Release authority |
| Human/organizational authority | authorized / missing / unverified | A technical measurement result |

States should be enumerated and machine-readable, with a short explanation, referenced input paths and explicit rule identifier. Avoid an aggregate percentage that permits one strong dimension to hide a critical missing dimension. Hard blockers—such as tenant mismatch or absent decision-rule prerequisites—must remain blockers.

## 3. Make decision rules named, versioned and swappable

The current `nominal-point-estimate-range/0.1` rule is a useful bounded comparison: it uses the nominal decimal value and source-provided limits only when unit system and code match. Keep that rule immutable for compatibility. Do not silently reinterpret it as an uncertainty-aware conformity decision.

A future rule-registry entry should declare:

1. Stable rule ID and version, semantics, owner and intended use.
2. Exact input fields and required unit / quantity-kind compatibility.
3. Whether uncertainty is ignored, propagated, or used through a stated decision rule.
4. Preconditions, boundary inclusivity, missing-data behavior and numeric profile.
5. Risk assumptions or decision parameters required by the chosen policy.
6. Result states, limitations, machine-readable explanations and conformance vectors.
7. Versioning and deprecation rules, with no silent changes to prior reports.

### Metrology: uncertainty must have meaning before it can affect a decision

The fixture currently includes an uncertainty value whose coverage convention is explicitly unspecified. That is not enough to infer a confidence interval, probability of conformity, or guard band. The current validator correctly keeps its comparison nominal-only.

Before adding an uncertainty-aware rule, a new profile revision should specify the minimum metadata needed by that rule, such as uncertainty type, unit, coverage factor or coverage probability when applicable, method/model, and the relationship between the reported result and uncertainty. For correlated or multivariate quantities, the contract must define how covariance or joint uncertainty is represented rather than assuming independence.

A rule may then return distinct states such as:

- `within_declared_acceptance_region_under_rule`
- `outside_declared_acceptance_region_under_rule`
- `indeterminate_under_rule`
- `not_evaluable_missing_prerequisite`

These are computational outcomes under a named rule, not automatic product-release dispositions. Acceptance limits and guard-band behavior must be selected by the responsible quality authority for the use case. OMSI must not invent a universal risk appetite or imply that a simple interval-overlap calculation is universally compliant with a metrology standard.

This separation follows the concern addressed by ILAC G8 on decision rules and statements of conformity, and JCGM 106 on the role of measurement uncertainty in conformity assessment. It is a research direction, not a claim of compliance with either document.

## 4. Bind every report to an explicit replay envelope

A report becomes operationally useful when an independent consumer can establish exactly what it evaluated. A future `omsi-evidence-envelope/0.1` should bind, at minimum:

- input artifact digest and digest algorithm;
- profile name, version and schema digest;
- rule ID/version and rule-registry digest;
- validator identity and build/version metadata;
- normalized result artifact digest;
- ordered or otherwise deterministically represented decision-trace entries;
- upstream evidence references, including the difference between an identifier and an authenticated claim;
- declared omissions and limitations.

Define serialization and digest semantics before implementation. Do not label ordinary JSON reserialization as a standardized canonical JSON representation unless it actually follows a specified canonicalization scheme. Input-byte digests and semantic-object digests answer different questions; name them differently.

For authenticated exchange, evaluate a standard attestation envelope rather than inventing a signature format. The in-toto Attestation Framework separates subject, predicate, envelope and bundle concerns. OMSI could define a manufacturing-specific predicate carried through a supported envelope, with policy verification performed by the consumer. A digest alone is tamper-evident only when anchored or compared against a trusted value; a digest is not origin authentication, authorization, or proof that the underlying fact is true.

Keep cryptographic signing, identity binding, key rotation, revocation, and signer authorization outside the current unauthenticated diagnostic report until their threat model and trust roots are defined.

## 5. Turn conformance vectors into executable contract tests

The eight current vectors are a seed corpus, not sufficient evidence of industrial conformance. Expand each behavior into named invariants and cross-implementation vectors.

Required future invariants include:

- Missing source revision never becomes a revision inferred from export time.
- A source's raw `Accepted` status cannot override a contradictory derived comparison.
- An input outside the tenant binding cannot be resolved through a cross-tenant lookup.
- A missing unit, incompatible unit or unsupported quantity kind never defaults to a plausible-looking unit.
- Lack of uncertainty metadata blocks rules that require it, while leaving the legacy nominal-only result labeled as nominal-only.
- Reducing evidence assurance cannot upgrade a conclusion; better evidence must not rewrite the preserved original source claim.
- Equivalent explicit time offsets normalize to the same instant; naive timestamps remain invalid.
- Rule boundary tests exercise one representable decimal below, at, and above each limit.
- Report replay over identical inputs, profile, rule and validator build yields semantically identical output.
- Mutation of the input, profile, rule, or normalized output is detected by its corresponding digest check.
- Unknown future versions fail or are handled exactly as the versioning contract says; no implicit semantic fallback.
- Independent implementations reproduce the same vector outcomes without relying on human-readable assertion strings.

Use deterministic vectors first, metamorphic/property testing next, and coverage-guided fuzzing for parsers and shape boundaries after the Rust crate has a reproducible locked dependency graph. Keep negative controls; never loosen a negative test merely to make conformance green.

## 6. An adapter ecosystem, not another monolith

The interoperability strategy should have three separable pieces:

1. **Source adapters** translate concrete MES, QMS, CMMS, metrology and equipment-gateway representations into a named OMSI profile. Each adapter documents what it preserves, derives, drops and cannot authenticate.
2. **The neutral evidence contract** preserves source claims and carries derived claims with provenance, rule identity and limitations.
3. **Independent verifiers** replay conformance vectors and verify any supported envelopes without needing the original vendor's implementation or access to unrelated factory data.

Start with read-only synthetic data and public test fixtures. Add real-system adapters only after tenant isolation, pagination, revocation, stale/partial reads, snapshot semantics, rate limits, and secret handling have dedicated tests. Do not begin with PLC/SCADA write paths. A future operational integration must have a separate authorization model, hazard analysis, safety case and owner-controlled deployment gate.

QIF (ISO 23952) is a relevant interoperability reference for manufacturing-quality and metrology information across design, planning, execution, analysis and reporting. NIST's digital-thread work also highlights the need for lifecycle interoperability, conformance testing and trust in data assets. OMSI should map to these existing ecosystems where semantics fit, rather than create competing copies of every domain model.

## 7. Delivery sequence and gates

### Gate A — Harden the pilot (current scope)

- Keep the profile explicitly marked as incubation and synthetic.
- Make report-schema validation executable with an independent JSON Schema implementation.
- Lock the complete Rust dependency graph; record exact-head format, test, vector and Clippy results.
- Add an exact-head reproducibility record; never equate queued jobs or stale successful runs with a pass.
- Preserve explicit non-authority fields and negative controls.

**Exit evidence:** same-head clean formatting, tests, vectors, schema validation and linting; reproducible dependency lock; no production readiness claim.

### Gate B — Evidence and replay contract

- Specify evidence-node identities and derivation links.
- Specify digest semantics, replay envelope, deterministic trace ordering and version negotiation.
- Add envelope and tamper-mutation vectors, with a verifier independent of the report producer.
- Decide what is public, tenant-private, or redacted; avoid exposing secrets or personal information in traces.

**Exit evidence:** an independent verifier reconstructs the report-to-input binding and detects mutated evidence without trusting the producer's success label.

### Gate C — Metrology decision profile

- Have a metrology/quality-domain reviewer choose the problem scope and decision-rule policy.
- Specify uncertainty metadata, units, limits, acceptance region, edge cases and required risk statements.
- Keep the existing nominal-only rule and its vectors unchanged; add a new versioned rule and distinct expected outcomes.
- Include ambiguous/insufficient evidence cases, not only easy in-range and out-of-range examples.

**Exit evidence:** reviewed rule specification, vectors derived from the specification, boundary/property tests and documented limitations. Passing software tests alone is not metrological validation.

### Gate D — Interoperability qualification

- Build read-only adapters against synthetic fixtures from at least two differently shaped source systems.
- Measure semantic retention, unmapped fields, revision loss, scope violations, completeness and replay equivalence.
- Map to relevant QIF/industrial data models only with an explicit semantic crosswalk and known-loss register.
- Publish conformance packs and independent verifier tooling.

**Exit evidence:** adapters produce comparable evidence claims without erasing disagreements; each loss and unsupported claim is explicit.

### Gate E — Federated evidence exchange

- Use signed, standard attestation envelopes only after the identity and threat models are reviewed.
- Support local verification so raw process data need not be centralized solely to validate a claim.
- Separate who may attest, who may interpret, and who may authorize downstream actions.
- Start with human-reviewed, read-only workflows. Any later release/control integration is a separate program and safety gate.

**Exit evidence:** key, revocation, replay, downgrade, cross-tenant and tampered-bundle negative tests; independent review of operational authority.

## 8. Non-goals and prohibited shortcuts

This roadmap does not propose that OMSI should issue a universal quality score, infer truth from consensus, treat blockchain or a signature as a substitute for measurement validity, or let an AI model overrule deterministic quality rules. Statistical anomaly detection may prioritize human review; it must not silently mutate source facts or gain release authority.

Do not:

- infer calibration authenticity from a validity interval;
- treat a source status as independently verified;
- use export time as revision/freshness evidence;
- infer a measurement unit from asset class or neighboring records;
- silently convert a nominal comparison to an uncertainty-aware decision;
- merge independent assurance dimensions into an opaque scalar;
- describe a schema-valid report as an authenticated attestation;
- call queued or stale CI a pass;
- connect the diagnostic path directly to machine-control or product-release APIs.

## Research references

- NIST, [Digital Thread for Manufacturing](https://www.nist.gov/programs-projects/digital-thread-manufacturing) and [Extended Digital Thread](https://www.nist.gov/programs-projects/extended-digital-thread): lifecycle data interoperability, conformance infrastructure and trust in data assets.
- ISO, [ISO 23952:2020 — Quality Information Framework (QIF)](https://www.iso.org/standard/77461.html): integrated manufacturing-quality information models and workflow documents.
- ILAC, [G8:09/2019 Guidelines on Decision Rules and Statements of Conformity](https://ilac.org/publications-and-resources/ilac-guidance-series/) and [G17:01/2021 Guidelines for Measurement Uncertainty in Testing](https://ilac.org/publications-and-resources/ilac-guidance-series/): decision-rule and uncertainty considerations.
- BIPM / JCGM, [JCGM 106:2012 — The role of measurement uncertainty in conformity assessment](https://www.bipm.org/en/doi/10.59161/jcgm106-2012): conformity-assessment framework involving measurement uncertainty and stated requirements.
- W3C, [PROV-O](https://www.w3.org/TR/prov-o/): provenance vocabulary separating entities, activities and agents.
- in-toto, [Attestation Framework specification](https://github.com/in-toto/attestation/blob/main/spec/README.md): subject-bound attestations, predicates, envelopes and bundles.

This roadmap is a proposal for engineering work. It does not demonstrate OMSI conformance to any referenced standard, nor does it certify any factory, product, lab, instrument, connector, or measurement.
