# Cooperative Buying — proposed first pack

**Status:** design and schema prototype only. This pack is not a live buying group, marketplace, supplier directory, payments product, or legal/compliance service.

## Purpose

Provide the first concrete path toward a globally portable, member-oriented wholesale network: let buyers compare supplier offers on equivalent units and actual landed costs, then build toward demand aggregation without locking independent businesses into a franchise.

## Current files

- [Proposed cooperative-offer schema](../../schemas/cooperative-offer-v1.schema.json) — Draft 2020-12 structural schema for supplier offers.
- [Synthetic ZAR offer](fixtures/example-offer-zar.json) — South African-shaped example; not a real supplier or quote.
- [Synthetic EUR cross-border offer](fixtures/example-offer-eur.json) — Germany/Netherlands-shaped example; no legal, tax, payment, or delivery capability is implied.
- [Semantic validator](../../tools/validate_cooperative_offer.py) — deterministic cross-field checks; it never authorizes a transaction.
- [Purchase-intent schema](../../schemas/cooperative-purchase-intent-v1.schema.json) — separates non-binding interest/quote requests from binding order declarations that require an offer, single-use authorization, and idempotency key. Sharing consent binds to explicit recipient IDs and an RFC 8785 digest of the approved data scope.
- [Synthetic non-binding interest](fixtures/example-nonbinding-interest.json) — consent-scoped demand fixture; no purchase obligation or real buyer is represented.
- [Purchase-intent validator](../../tools/validate_cooperative_purchase_intent.py) — checks time bounds, authorized maximums, legal-entity binding, and a canonical digest over the exact order scope.
- [Purchase-intent schema tests](../../tests/test_cooperative_purchase_intent_schema.py) and [semantic tests](../../tests/test_cooperative_purchase_intent_semantics.py).
- [Semantic regression tests](../../tests/test_cooperative_offer_semantics.py) — dates, publication evidence, offer expiry/freshness, quantity relationships, price-break ordering, and cross-border constraints.
- [Global architecture and staged plan](../../docs/GLOBAL_COOPERATIVE_COMMERCE.md).
- [Dependency-free Rust savings kernel](../../crates/cooperative-commerce-core/README.md) — exact-decimal arithmetic, exact-basket comparability, symmetric cost coverage, delivery-line binding, explicit credits, and negative-result preservation.
- [Savings Receipt schema](../../schemas/cooperative-savings-receipt-v1.schema.json) plus [illustrative receipt fixture](fixtures/example-savings-receipt.json) — portable inputs, calculations, evidence references, and limitations; the fixture is synthetic and cannot be promoted to a computed receipt.
- [Code-list snapshot manifest](../../schemas/code-list-snapshot-manifest-v1.schema.json), [illustrative SIX ISO 4217 manifest](fixtures/example-code-list-manifest.json), and [semantic validator](../../tools/validate_code_list_manifest.py) — define active-registry requirements for source retrieval/authority review, immutable version label, retained source and normalized payload digests, parser/build identity, duplicate-code checks, effective dates, source-sample reconciliation, independent review, review report, and review deadline. The example is explicitly not retrieved/not run and cannot be activated.
- [Code-list manifest tests](../../tests/test_code_list_manifest_schema.py) and [semantic tests](../../tests/test_code_list_manifest_semantics.py).

## Schema contract and limits

The schema preserves explicit currency, supplier and seller country, packaging and unit code systems, tax treatment, known/unknown charges, fulfillment scope, profile references, offer validity, and evidence provenance. It accepts extensible currency and country-code shapes rather than hard-coding only the first pilot market. A unit label without its code-system identity is not sufficient for a global price comparison; the semantic validator requires inventory and unit-price scheme bindings to match the order unit unless an explicit reviewed conversion exists.

Structural validity is not proof of:
- a real supplier, current stock, truthful price, or authorized quotation;
- valid/current ISO code assignment;
- exact product identity or safe substitution;
- tax treatment, customs, competition-law compliance, or local licensing;
- quote freshness at the time of purchase;
- savings, supplier performance, or transaction completion.

The prototype now checks declared code-list metadata and requires source sampling plus an independent review for active registry declarations. It still does not download a registry, check actual payload hashes against the source, run duplicate/effectivity checks on real records, verify reviewer signatures, or perform transaction-time membership lookup. Those are requirements for the later registry importer and runtime—not claims already satisfied by the fixture manifest.

## Manual-first pilot path

1. Import supplier CSVs or manually record quotes with their original source and timestamp.
2. Normalize pack size, order unit, product specification, and currency without dropping the source values.
3. Request non-binding demand only, with an explicit list of fields shared with the supplier.
4. Compare offers with all known fees and visible unknowns; ask suppliers to resolve unknown costs.
5. Require the buyer or explicitly authorized local cooperative to approve any binding action.
6. Reconcile supplier invoice and received quantity before reporting realized savings.
7. Export the source records and arithmetic so an independent checker can reproduce the result.

## Pricing and member protections

- No franchise-style percentage-of-turnover royalty by default.
- No paid placement presented as neutral ranking.
- No unreported supplier rebate or hidden FX/freight fee.
- No sale of customer data or paywall on exporting a buyer's own data.
- No cross-member disclosure of confidential offers or downstream business strategy.
- No pooled funds, credit, escrow, owned inventory, or merchant-of-record role until locally reviewed and separately qualified.

The default first-pilot hypothesis is supplier-direct contracting and invoicing for each buyer, with the network coordinating quotes and demand. Local legal review is required before relying on this arrangement.

## Qualification status

**Current maturity: M0 / proposed.** JSON Schema and regression tests only check declared structure. The data is synthetic. There is no purchase engine, supplier connector, tax engine, payments integration, savings result, or runtime qualification in this pack.


## Running the declaration and semantic checks

From the repository root, after installing the pinned validator dependencies:

```bash
python tools/validate_cooperative_offer.py business-packs/cooperative-buying/fixtures/example-offer-zar.json
python tools/validate_cooperative_offer.py business-packs/cooperative-buying/fixtures/example-offer-eur.json
python -m unittest discover -s tests -p 'test_cooperative_offer_*.py' -v
```

A report of `DECLARATION_VALID_NO_TRANSACTION_AUTHORIZED` means only that the declaration passed the implemented schema and cross-field checks. It is **not** an authorization, verified supplier claim, official currency/country code-list check, tax or competition-law assessment, live inventory confirmation, or realized-savings proof. Published offers must have current validity/availability observations, retained evidence references and digests, non-unknown tax treatment, and a non-synthetic declaration; these are necessary gates, not sufficient proof of transaction safety.

Purchase intents use separate states: non-binding interest and quote requests cannot carry a binding accepted offer or purchase authorization. A binding-order declaration must pin the immutable product specification digest and a specific offer revision/digest, carry a single-use authorization and an idempotency key, and use a scope digest. The RFC 8785 canonical purchase scope includes the current consent ID/scope digest and the consent-evidence reference/digest as well as order details, so changed disclosure scope or substituted consent evidence invalidates the prior purchase approval. Revision integers are capped at the JCS safe-integer boundary; monetary amounts and quantities remain exact decimal strings. The validator checks the accepted amount against the explicit maximum and currency. Even this does not verify the signer, source evidence, or durable replay ledger; the tool always returns `transaction_authorized: false`.
