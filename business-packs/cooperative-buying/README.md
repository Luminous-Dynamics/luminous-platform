# Cooperative Buying — proposed first pack

**Status:** design and schema prototype only. This pack is not a live buying group, marketplace, supplier directory, payments product, or legal/compliance service.

## Purpose

Provide the first concrete path toward a globally portable, member-oriented wholesale network: let buyers compare supplier offers on equivalent units and actual landed costs, then build toward demand aggregation without locking independent businesses into a franchise.

## Current files

- [Proposed cooperative-offer schema](../../schemas/cooperative-offer-v1.schema.json) — Draft 2020-12 structural schema for supplier offers.
- [Synthetic ZAR offer](fixtures/example-offer-zar.json) — South African-shaped example; not a real supplier or quote.
- [Synthetic EUR cross-border offer](fixtures/example-offer-eur.json) — Germany/Netherlands-shaped example; no legal, tax, payment, or delivery capability is implied.
- [Global architecture and staged plan](../../docs/GLOBAL_COOPERATIVE_COMMERCE.md).

## Schema contract and limits

The schema preserves explicit currency, supplier and seller country, pack and unit, tax treatment, known/unknown charges, fulfillment scope, profile references, offer validity, and evidence provenance. It accepts extensible currency and country-code shapes rather than hard-coding only the first pilot market.

Structural validity is not proof of:
- a real supplier, current stock, truthful price, or authorized quotation;
- valid/current ISO code assignment;
- exact product identity or safe substitution;
- tax treatment, customs, competition-law compliance, or local licensing;
- quote freshness at the time of purchase;
- savings, supplier performance, or transaction completion.

The first implementation must add semantic checks for effective date ordering, code-list membership, unit conversions, consistent price-break currency, explicit landed-cost uncertainty, jurisdiction-pack status, and authorization before binding orders.

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
