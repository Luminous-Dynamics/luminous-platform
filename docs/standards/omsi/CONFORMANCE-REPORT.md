# OMSI local starter-kit conformance report

**Assessment date:** 2026-10-12. Scope: local draft artifacts and executable checks only. Not a security assessment or standards certification.

## Results recorded during development

- Python unittest suite: reran the companion archive's `make test` locally on 2026-10-12; 68 tests passed in 2.017 seconds.
- JSON Schema Draft 2020-12: four core schemas, one NixOS change-arguments schema and two optional IT Service Core incubation schemas passed schema self-validation.
- OpenAPI: parsed as 3.1.1; local references resolved and operation IDs were unique. A dedicated OpenAPI validator and live server/client contract test were not run.
- Canonicalization: Python and Node.js probes matched included vectors and the positive operation-manifest digest. This is not full RFC 8785 conformance.
- Positive fixtures passed implemented semantic checks; negative probes rejected covered cross-tenant mismatches.
- Python wheel build and isolated-target installation passed in the local development run.

## Important limitation for this PR

The larger companion starter was extracted from the generated technical archive and its `make test` suite was rerun locally on 2026-10-12: 68 tests passed in 2.017 seconds. This is local design evidence only; the kit has not been independently reviewed, and these results are not a certification. This RFC-only PR does not include that larger companion kit.

A smaller, reviewable implementation seed with six JSON Schemas, a Python validator/CLI, fixtures, receipt/event chronology checks and a pure OMSI-to-Nixward identity/lifecycle-shape prototype is proposed separately in [PR #32](https://github.com/Luminous-Dynamics/luminous-platform/pull/32). That seed now contains 90 tests (47 validator tests and 43 adapter tests). A local mirror of the previous 38-test seed state passed those earlier 38 tests, schema self-validation, four CLI fixture checks and an independent JavaScript digest probe. Since then, nine receipt/event chronology tests and forty-one adapter tests have been added, as well as schemas for the OMSI-to-Nixward binding and the currently inspected Nixward SystemTransaction v1 envelope. Those 52 additional tests and two schemas have not yet been independently executed on the exact current branch. GitHub Actions runs [#38314065437](https://github.com/Luminous-Dynamics/luminous-platform/actions/runs/38314065437) and [#38314063789](https://github.com/Luminous-Dynamics/luminous-platform/actions/runs/38314063789) for head `61fa250986d9f7cf69e4afee258cd5ad6d581486` were last observed queued. The 88-test seed and the 68-test companion kit are different scopes; do not combine their counts or infer that either validates the other.

## Reported security-relevant cases covered locally

Tenant/resource mismatch; manifest expiry with explicit evaluation time; stale/future preconditions; argument-digest substitution; grant subject/executor binding and validity bounds; approval mismatch; nonce replay; idempotency-key reuse; secret-named fields; duplicate JSON keys; numeric edge cases; event IDs/order/correlation/state transitions; receipt binding; missing post-state evidence; verification/manifest chronology; and declarations of non-independent verifier identity/domain. Cross-document application-to-observation-to-verification-to-issuance ordering, verified/non-verified receipt-chain consistency, zero identity-digest rejection and zero authorization challenge/replay-digest rejection are covered by tests whose exact-head CI result is pending.

## Explicit limitations

1. No cryptographic signature verification or trusted issuer validation.
2. No trust store, key rotation, revocation system or identity-provider integration.
3. The JavaScript probe covers a deliberately smaller subset and is not a full independent validator.
4. Two local language implementations are not two independent vendor implementations.
5. Verifier independence is based on declared identity/domain labels, not externally attested deployment facts.
6. No disposable-VM NixOS change/recovery experiment or actual endpoint execution. The pure adapter-binding prototype is not wired into Nixward's live authorized-execution path and does not authenticate the origin of its transaction envelope.
7. No dedicated OpenAPI validator or live API server/client test.
8. No evidence of fleet reliability, SLA compliance, backups/restores, EDR coverage or complete MSP replacement.

## Reproduction from the companion kit

    PYTHONPATH=src python -m unittest discover -s tests -v
    make validate-fixtures
    make interop
    make wheel-check

The sample fixture clock is pinned test data, not current time. Production execution must supply a trusted clock and re-check grant expiry, identity and revocation immediately before mutation.
