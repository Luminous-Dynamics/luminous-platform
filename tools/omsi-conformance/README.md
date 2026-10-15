# OMSI 0.1 — executable conformance seed

This is a deliberately small, reviewable reference validator for the OMSI pre-RFC. It validates JSON Schemas and selected semantic invariants for operation manifests, authority grants, lifecycle event sequences, evidence receipts and a prototype OMSI-to-Nixward identity-binding record.

## Run

```sh
python -m pip install -r tools/omsi-conformance/requirements.txt
make -C tools/omsi-conformance schemas
make -C tools/omsi-conformance test
make -C tools/omsi-conformance validate-fixtures
make -C tools/omsi-conformance interop
python tools/omsi-conformance/omsi_validate.py operation tools/omsi-conformance/fixtures/operation.json --now 2026-10-12T12:05:00Z
python tools/omsi-conformance/omsi_validate.py grant tools/omsi-conformance/fixtures/grant.json --manifest tools/omsi-conformance/fixtures/operation.json --now 2026-10-12T12:05:00Z
python tools/omsi-conformance/omsi_validate.py receipt tools/omsi-conformance/fixtures/receipt.json --manifest tools/omsi-conformance/fixtures/operation.json --events tools/omsi-conformance/fixtures/events.json
python tools/omsi-conformance/omsi_validate.py events tools/omsi-conformance/fixtures/events.json --manifest tools/omsi-conformance/fixtures/operation.json
```


The CLI requires `--manifest` for receipt and lifecycle-event validation. This is intentional: a structurally valid receipt or event list without the immutable operation manifest cannot establish cross-document binding, so the CLI must not print a conformance pass for it. These checks still do not verify signatures, issuer trust, or the truth of target observations.

The secret-field check is defense in depth: it rejects common secret-like key tokens across snake_case, kebab-case, and CamelCase, and constrains explicit secret/credential references to opaque URI-shaped values. It cannot prove that arbitrary values are not secrets. Domain profiles must still define allowed action arguments, keep secret delivery out of portable manifests/receipts, and use a separate secret-delivery mechanism.


## Timestamp profile

The reference validator accepts an OMSI RFC3339 subset with an explicit timezone, an uppercase `T` separator, seconds, and at most six fractional-second digits. It rejects excess precision because Python `datetime` would otherwise silently truncate it, which can corrupt chronology comparisons. This is a documented 0.1 reference-profile restriction, not a limit imposed by RFC3339 itself. Timestamps are normalized to UTC for comparison. This does not prove source-clock accuracy or event causality.

## License

The files in this conformance seed are released under the MIT License. This license does not establish governance or licensing for the broader OMSI standard.

## Canonicalization profile

See [CANONICALIZATION.md](CANONICALIZATION.md) for the exact prototype framing and registered digest domains. Action-argument and operation-manifest digests are intentionally domain-separated and are not interchangeable.

The seed accepts a deliberately restricted JSON value set for digest-bearing contract documents: duplicate object keys, negative zero, floating-point numbers, unsafe-range integers and strings/keys that cannot be represented as valid Unicode scalar values are rejected. Object member names are ordered by UTF-16 code units; arrays retain order; canonical bytes are hashed with SHA-256 using an explicit semantic domain prefix. This is a constrained accepted-input profile designed to avoid ambiguous cross-language encodings, **not a claim of full RFC 8785/JCS conformance**. The exact algorithm and test vectors must receive independent cross-language review before a stable protocol version. The `interop` target uses a small independent JavaScript implementation to check fixed basic/Unicode vectors and the action/manifest/grant/receipt digest bindings; it is a prototype, not a second fully conforming implementation.

## Nixward identity-binding prototype

The pure helper in `nixward_adapter.py` binds an already validated OMSI operation manifest to the plan digest, target-machine digest and transaction ID carried by one versioned Nixward SystemTransaction envelope. The six schemas include the adapter binding record and a structural mirror of the currently inspected Nixward v1 envelope. The mirror uses the actual serialized `ApprovalEvidenceKind` variant names (`GeneralChange`, `ExecutionIntent`, `AuthoritySignature`, `ExecutionIntentAuthority`). In addition to schema/version, phase and digest-shape checks, the helper checks phase-to-receipt consistency: validation and authorization precede snapshot/application, applied and verified phases require a successful application exit, verification is required before promotion, failed/recovered outcomes must match the application/verification evidence, application timestamps must be ordered, mutation evidence digests cannot be all zero, and Nixward's UTF-8 byte-size limits are enforced for issuer/key IDs and summary/reason strings. The adapter also rejects all-zero transaction/plan/target identity digests and all-zero optional authorization challenge/replay digests. These are fail-closed shape checks only, not proof of provenance or actual host state.

This is only a deterministic identity-binding contract. It does not create or verify Nixward ChangePlan/ChangeAuthorization objects, call `NixOSExecutor::execute_authorized()`, persist nonces, attest the source of Nixward digest arguments, observe a real machine, or verify recovery. Callers must pass the authoritative Nixward transaction envelope being used, not synthesize a lookalike object or copy values back out of an untrusted binding record. This Python checker still cannot prove the envelope was produced by Nixward's Rust implementation. Passing the unit tests is not a Nixward integration or VM qualification result.

## Security boundary

The prototype only accepts `secret_refs` and `credential_refs` as arrays of opaque URI-shaped references, and rejects whitespace, userinfo and query-bearing reference strings. This is structural hygiene, not discovery of every possible secret and not proof that the referenced vault/resource is trustworthy. Raw secret material still belongs in a separate secret-delivery mechanism.

This is not an executor, identity provider, authorization service or cryptographic verifier. It does not authenticate principals; verify proof/signature bytes; resolve issuer trust; check key revocation; provide replay persistence across processes; establish tenant isolation in deployment; prove declared verifier independence; or execute/rollback a host change. The grant signature is shape-checked only. Tests use deterministic sample data and do not prove production security.

When `receipt` validation is supplied both `--manifest` and `--events`, a verified receipt is cross-checked against the operation-bound lifecycle sequence: the post-state observation cannot predate application, the terminal `verified` event cannot predate the observation, and the receipt cannot predate that event. Without `--events`, the validator can only enforce chronology fields present in the receipt and manifest; it cannot infer when execution occurred. These checks validate declared timestamps and event structure, not signed provenance or actual host state.

A successful validator result means only that a document met these local schema and semantic checks. It must not be advertised as OMSI certification or complete MSP compliance.

## Intended next gate

Add an independently reviewed cryptographic grant-verification interface, durable nonce/idempotency state, HTTP integration tests, wire the binding to Nixward's real authorized-execution result, and run the disposable-VM NixOS operation/recovery test before any real machine mutations or production claim.


## Field Service profile 0.1 — synthetic conformance slice

The incubation field-service profile is described in
[OMSI PR #31](https://github.com/Luminous-Dynamics/luminous-platform/pull/31).
This executable slice deliberately does **not** implement a PSA, dispatch
system, licensing authority, billing system or production executor.

Run the focused field-service checks:

```sh
make -C tools/omsi-conformance field-service
```

The checker applies a strict Draft 2020-12 schema and validates selected
cross-record invariants at an explicit evaluation time. The synthetic positive
fixtures cover two different kinds of work:

- a plumbing/HVAC pressure-test work order;
- a building-maintenance fixing/pull-test work order.

The adversarial manifest in
`fixtures/field-service-cases.json` contains 26 negative cases: exact
operation-manifest digest substitution, cross-tenant references, unmatched
approval scope, revoked/wrong-scope credentials, unit mismatch, calibration
validity/freshness/chronology failures, provider claims mislabeled as verified,
reused idempotency keys under a different manifest, offline execution after
authority expiry, invalid correction links, undisclosed mapping losses,
same-domain "independence", broken or mismatched customer acknowledgment,
unknown fields, unverified closeout, assigned-practitioner substitution,
evidence-actor substitution and evidence dated after the fixed evaluation
clock. Each mutation declares the diagnostic fragment it is expected to
trigger; the runner discovers the focused suite and requires at least five tests.

The profile keeps the source system's case ID/status alongside the normalized
status and makes mapping losses explicit. Each operation is bound to tenant,
site, asset, scope version and manifest digest. Typed evidence distinguishes
provider reports, measurements, inspections, independent verification,
customer acknowledgment, handover, disputes and corrections.

**Claim limits:** fixture values and acceptance intervals are synthetic and
are not trade guidance or statutory criteria. The validator checks declared
relationships; it does not cryptographically verify credential issuers,
check a live regulator, establish verifier independence in the real world,
verify evidence content against its digest, inspect physical assets, or
prove legal compliance. It is not a production authorization service. The
new tests do not qualify independent vendor interoperability or a field pilot.


## Pinned OCA Field Service 19.0 adapter projection (research prototype)

Run the new read-only mapper checks:

```sh
make -C tools/omsi-conformance oca-adapter
```

The prototype is pinned to OCA/field-service commit
`8b2fa2592dda401726834ccf8030d34e241a69ad`, core addon
`fieldservice 19.0.1.2.3`, and exact optional Activity/Accounting addon
versions where present. It accepts a synthetic, PII-minimized order snapshot,
a separate explicit instance/company-to-tenant binding, and a source-stage
mapping profile. It does not call Odoo and cannot write source records.

The resulting `org.omsi.field-service-intake/0.1` projection is deliberately
**not** a qualified service record. It carries the original source stage and
configured mapping, source-namespaced worker/equipment/location references,
distinct request/scheduled/actual time fields, optional activity state, and
invoice references. It discloses unresolved jurisdiction, credential,
estimate/approval, operation-manifest and independent-verification facts.
The output cannot claim that work quality is verified or that an invoice is
paid. An Odoo `company_id` is not accepted as an OMSI tenant identity without
an explicit external binding.

The synthetic tests cover unknown/custom stages, stage-map fingerprint
substitution, tenant/company and source-instance mismatches, revoked bindings,
incomplete required activities on a closed stage, signature requirements,
optional add-on absence, interval errors, source version pins, future-dated
actual-work/signature/activity timestamps, unsupported fields, privacy
projection, and invoice links not being conflated with payment or quality proof.
The evaluation clock rejects purportedly completed events in the future while
still permitting legitimate future request and scheduled windows.

**Limits:** this is an offline mapping prototype, not a connector, Odoo API
integration, legal/license determination or production security qualification.
The core and optional addons inspected declare AGPL-3; review the deployment
and distribution obligations before bundling or linking them in any product.


## ERPNext Asset Maintenance Log adapter (research prototype)

Run the source-pinned offline checks with make -C tools/omsi-conformance erpnext-adapter.

The first ERPNext slice is pinned to release v16.51.0 at commit fd97695de7bfeb1d7728a7370c3e128d0cc1509e. It maps only Asset Maintenance Log snapshots with their separately linked Asset Maintenance plan and Asset Maintenance Task. Maintenance Visit/Schedule mapping remains distinct because those workflows have different lifecycle fields.

The mapper requires an explicit ERPNext site/company-to-tenant binding and an IANA site timezone. Date-only completion checks use the evaluation instant converted to that timezone; raw due/completion/next-task dates remain dates. "Overdue" stays "overdue" rather than being mislabeled "blocked", and a task's next due date is preserved separately from a log's due date. The pinned ERPNext controller can mark a past-due Planned log as Overdue during validation; if an imported snapshot still says Planned with a past due date, the adapter preserves the source status and emits a mapping conflict instead of silently rewriting it. It rejects future-dated completion claims, mismatched plan/task relationships, unsupported source revisions, invalid timezones and unknown fields.

External IDs percent-escape source-controlled namespace and record-name components before joining them with the fixed DocType separator, preventing ambiguous composite IDs. The output includes a mapping-context record with the adapter identifier, tenant-binding identifier, configured site timezone, and UTC evaluation instant, so date decisions can be audited without implying that source clocks or binding authenticity were independently verified. Identical inputs and evaluation clocks produce byte-identical canonical output. The focused adapter suite currently contains 39 tests, and the Make target enforces that minimum.

The task certificate requirement, log has_certificate flag and certificate attachment presence are separate data. Conflicts are disclosed; certificate metadata is not proof of authenticity. Assignment, status, submission and performed-work notes do not establish professional credentials, verified workmanship or compliance. PII/editor content and certificate bytes/attachment paths are omitted.

Limits: synthetic offline conformance only, not a live ERPNext connector, credential verifier, certificate validator, legal review or production security qualification. ERPNext v16.51.0 license.txt identifies GPLv3; distribution obligations need independent review.
