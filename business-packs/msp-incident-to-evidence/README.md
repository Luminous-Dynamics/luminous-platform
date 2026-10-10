# MSP Incident-to-Evidence starter pack

**Status:** proposed business/service profile. Not an installed product, live ConnectWise integration, or production-qualified remote-operations feature.

This pack gives a small MSP a repeatable first service. It is designed to work manually using customer-approved exports and established tools while the Luminous platform connector, tenant isolation, and operational workflow are still being qualified.

## Files in this starter pack

- [Service pack manifest](service-pack.yaml) — proposed scope, inputs, outputs, limits, and acceptance.
- [Customer intake template](templates/customer-intake.md) — clarify authority, target, data boundary, and acceptance.
- [Statement-of-work template](templates/statement-of-work.md) — record scope, exclusions, responsibilities, data handling, and fees.
- [Delivery report template](templates/delivery-report.md) — separate observed facts, assertions, simulations, test results, and unknowns.
- [Discovery call guide](templates/discovery-call.md) — learn the buyer's real problem without overselling.
- [One-page offer](templates/one-page-offer.md) — explain the scope, outcomes, and exclusions.
- [Pricing worksheet](templates/pricing-worksheet.md) — estimate a viable local price floor from actual delivery costs.

The templates are drafting aids. Adapt contracts, tax, insurance, privacy and retention terms to the relevant jurisdiction with a qualified adviser.

## Customer promise

Turn one agreed incident or operational-risk review into a source-linked record of what is known, what action is planned, who owns the next step, and what evidence would show it is resolved.

Do not promise that Luminous automatically fixes incidents, proves security, or certifies recovery.

## Initial workflow

1. Confirm client authorization and the scope of the work.
2. Obtain only approved, minimum-necessary exports or use a client-screen-shared review.
3. Map the external company/customer to the agreed local tenant; quarantine unknown mappings.
4. Normalize the selected incident into the agreed schema by manual review.
5. Keep source identity, original event/ticket reference, observation timestamp, revisions, and evidence references.
6. Separate verified facts, customer-provided assertions, operator interpretations, and AI-generated suggestions.
7. Assign an action owner, due date, and acceptance evidence.
8. Deliver an incident-to-evidence report and confirm closure criteria with the customer.

## Data-handling constraints

- No provider credentials, access tokens, private keys, recovery codes, or unrestricted raw exports in starter fixtures.
- No cross-tenant mixing; unknown tenant or resource mappings block processing.
- No automatic writes back to a PSA in this starter pack.
- No command-execution controls or remote-host actuation.
- Redact or omit personal data that is not needed for the agreed work.
- Respect the customer's retention and deletion instructions.
- Synthetic examples must remain visibly synthetic.

## Service modes

- **Manual:** available to a competent operator using existing tools and approved source data.
- **Synthetic prototype:** for rehearsing the model and UI with fake customer incidents.
- **Qualified automation:** only after a named adapter has verified authentication, tenant/resource mappings, idempotency, conflict/replay behavior, data minimization, and exact-revision CI evidence.

Only the first two are defined by this starter pack. The third is a future qualification state, not a current feature claim.

## When to expand

Add a live PSA connector only after the durable tenant-scoped inbox/outbox contract, provider-specific authentication and revision semantics, negative tenant tests, retry/replay handling, and exact-head CI evidence exist. Keep compatibility with the incumbent PSA; do not require the customer to replace its service desk.
