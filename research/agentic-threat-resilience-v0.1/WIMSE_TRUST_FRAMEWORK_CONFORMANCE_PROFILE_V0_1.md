# WIMSE Agent Mandate Trust-Framework Conformance Profile v0.1

**Status:** Synthetic Trust-Framework profile and conformance corpus. It does not define a new credential format or implement signature verification, a distributed budget ledger, an RP or a production participant.

**Source:** [draft-hulea-wimse-agent-mandate-01](https://datatracker.ietf.org/doc/html/draft-hulea-wimse-agent-mandate-01), an individual Internet-Draft published 1 November 2026. This profile is pinned to -01 and must be re-evaluated if the draft changes. It is not an adopted IETF standard.

## 1. Scope: make two explicit Trust-Framework choices executable

Section 12.2 of draft -01 requires the Trust Framework to define:

- **Item 11 — how the Mandate Issuer establishes that the delegator instructed issuance.** The draft gives a signature by the delegator over the mandate content as an example. The protocol cannot independently inspect the delegator's internal organizational instruction chain.
- **Item 12 — whether Relying Parties share mandate-budget accounting.** A token does not create a globally atomic shared budget merely because multiple RPs can validate it.

This profile defines two testable instruction-evidence methods and two budget-accounting modes. It does not claim that one method is appropriate for every organization.

## 2. Item 11: instruction-evidence methods

### Profile A — delegator signature over exact mandate content

The Trust Framework requires a verifiable delegator signature over the exact mandate content. The verifier output must bind:
- exact mandate-content digest from the canonicalization owner;
- exact principal and delegator identities;
- exact issuer and intended RP/audience;
- permissions, validity and any sub-delegation limit (these are covered by the mandate-content digest);
- signature verification result, signing identity/role, key status and validity;
- withdrawal/supersession state at RP authorization and participant commit.

A generic approval, a signature over a similar but different mandate, or a valid signature from an unrelated person is not sufficient. This profile consumes the Act-Bound/canonicalization owner's digest as an opaque value; it does not calculate a competing production digest.

### Profile B — qualified Mandate-Issuer process evidence

A Trust Framework may instead define an auditable Mandate-Issuer process, with named controls, assurance level, independent process evidence, and role separation where required. The synthetic checker requires a qualified process attestation from the configured independent assurance role, bound to the exact mandate content and delegator/principal. This does not establish real issuer process quality; a real profile must specify audit scope, key management, status, incident response, retention and dispute resolution.

Each evidence record also carries a status-check time and expiry. The RP evaluates it against an explicit Trust-Framework maximum status age. A fixture that merely labels an old observation `active` is still indeterminate when its check time exceeds that freshness bound; future-dated checks and expired evidence are invalid.

For both profiles:
- missing required evidence is invalid;
- conflicting or unverifiable evidence is invalid or indeterminate according to the reason;
- withdrawn/superseded evidence cannot support a new authorization;
- stale/unknown required status is indeterminate and must not become implicit consent;
- a role-separation policy must reject issuer self-approval.

## 3. Item 12: budget-accounting modes

### Mode A — shared global ledger

All RPs consume the same mandate budget through one shared ledger identity. The protected participant/ledger enforces atomic consumption. Concurrent commits must never exceed the shared remaining budget.

Two independent local counters with the same initial value are **not** one shared ledger. A shared-ledger claim requires one common ledger identity and an atomic participant boundary.

### Mode B — explicitly allocated per-RP budgets

An Authority Source / Trust Framework may deliberately allocate independent budgets by RP/audience. In that case:
- each RP must have an explicit allocation;
- allocation sum must not exceed the authority budget ceiling;
- each RP must use its assigned ledger;
- a new/unallocated RP is denied;
- independent counters are only valid within their explicit allocation and do not imply a global shared budget.

### Unknown consumption and retries

An unknown outcome after a consequential request must lock further consumption until the original transaction is reconciled. An absent reconciliation permits retry only with the original request and idempotency identity. A committed reconciliation consumes the budget once. No new idempotency key may be minted to bypass uncertainty.

The included checker models these rules on ordered synthetic events. It is not a distributed-consensus or real transaction test.

## 4. Synthetic corpus and exact-head checker

The corpus has 14 cases:
- exact delegator signature, wrong digest, generic approval, wrong signer/role, withdrawal, unknown status;
- qualified vs unqualified Mandate-Issuer process evidence;
- shared ledger success, divergent ledger identity and over-budget concurrent commit;
- explicitly allocated per-RP success and missing allocation;
- unknown-consumption lock / retry race;
- per-RP allocation greater than the Authority Source ceiling.

The reference checker separately evaluates `instruction_outcome` and `budget_outcome`, then derives the RP decision. Signature/status and process-assurance fields are inputs from hypothetical upstream verifiers, not cryptographic operations performed by this program.

Twelve adversarial mutation controls start from positive fixtures and alter exact mandate digest, signer/role, instruction status, process assurance, ledger identity, committed units, RP allocations, and unknown-outcome ordering.

Files:
- `schemas/trust-framework-conformance-v0.1.schema.json`
- `scenarios/trust-framework-conformance-v0.1.json`
- `tools/validate_trust_framework_conformance.py`
- `.github/workflows/atr-trust-framework-conformance.yml`

## 5. Integration boundaries

- Reuse WIMSE Authority Assertions, Mandate Assertions, WIT/WPT, and OAuth transaction-context semantics. Do not create another token.
- Reuse the existing Act-Bound profile for exact invocation and mandate artifact digests. Do not add another canonicalizer or hash domain.
- Reuse Xenia/Mycelix as owners for identity, mandate lifecycle, principal entitlement, and authority resolution.
- Use Platform #540's synthetic participant/effect receipt runner for executed commit and response-loss experiments.
- Keep mandate validation, instruction evidence, RP authorization and participant effect as separate outcomes.
- Preserve unknown/indeterminate state; never infer a denial did not commit solely from a lost response.
- The independent effect observer remains a separate trust domain. This schema/checker does not create one.

## 6. Claim ceiling

All corpus records and state outcomes are synthetic. The checker does not verify signatures, JCS, WIT/WPT PoP, revocation status tokens, organizational workflow attestations, or consensus. It only validates declared synthetic facts against the Trust-Framework profile and tests a deterministic budget/unknown-outcome model.

A passing workflow establishes only that the schema and synthetic reference contract behave as declared. It does not establish compliance with WIMSE, an organization's Trust Framework, production RP budget atomicity, or real agent safety.

## References

- WIMSE Agent Mandates draft -01, Section 12.2: https://datatracker.ietf.org/doc/html/draft-hulea-wimse-agent-mandate-01
- Platform issue #550: https://github.com/Luminous-Dynamics/luminous-platform/issues/550
- Platform PR #542 cross-domain mandate assurance profile.
- Platform #540 ATR executable episode runner and independent effect receipts.
- Platform #546 Act-Bound request digest binding.
