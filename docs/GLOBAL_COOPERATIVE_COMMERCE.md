# Global Cooperative Commerce and Wholesale Network

**Status:** Proposed architecture and validation plan. This document is not evidence that a purchasing network, country pack, supplier integration, payment flow, or warehouse operation has been implemented or qualified.

**Design thesis:** Give every independent operator a useful free business toolkit, then let consenting buyers gain scale through transparent, verifiable cooperation—without requiring franchise ownership, platform lock-in, or a single global operator.

## 1. The objective

Build the open operating infrastructure for an economy in which small businesses can access capabilities commonly bundled into large chains:

- useful business software without a mandatory subscription;
- comparable supplier offers and transparent landed-cost calculations;
- cooperative purchasing and pooled logistics where they actually save money;
- reusable operations, training, quality, and provenance records;
- interoperable access to finance, payments, accounting, and fulfillment through qualified local providers;
- the ability to remain independently owned and to exit a network without losing business data or identity.

The long-term ambition includes a wholesale network with some of the practical benefits of Costco. The first product is **not** a warehouse, retailer, lender, payment institution, franchise, or merchant of record. It is free business infrastructure plus a way to aggregate demand, compare offers, and coordinate independent purchases.

Global readiness is an architecture constraint from day one. It is not a claim that one release can lawfully offer every financial, tax, employment, logistics, food, or consumer service everywhere.

## 2. Product boundaries: one platform, multiple independent roles

Keep the present Luminous Platform's operations/security product boundary intact. Cooperative commerce should be an optional, separately qualified business pack built on versioned shared contracts—not silently mixed into host operations or Mycelix core.

| Component | Proposed responsibility | Must not become |
|---|---|---|
| Luminous Platform | Free operator tools, workflows, adapters, catalog/offer UI, export, user-visible costs and evidence | A mandatory proprietary chokepoint |
| Symthaea | Advisory demand forecasts, anomaly detection, product matching suggestions, waste reduction, and explanations | An authority that commits purchases, sets resale prices, or fabricates savings |
| Mycelix | Optional federated credentials, governance records, provenance claims, and signed cross-organization receipts | A global dump of personal data, trade secrets, or full transactional history |
| Holochain | Optional participant-controlled validation/federation where the trust model benefits from it | A default database for every transaction |
| Local buying organization | The explicit buyer/agent/cooperative role defined by contract and local legal review | An ambiguously authorized pool of money or purchases |
| Supplier / logistics provider / payment provider | Its own offer, fulfillment, regulated service, and evidence | An unreviewable dependency or an exclusive gatekeeper |
| Local legal/accounting adviser or qualified provider | Jurisdiction-specific interpretation and service where needed | A global compliance claim inferred from a software configuration |

Operational records can use ordinary transactional databases and signed exports. Distributed infrastructure is valuable where multiple independently operated parties need shared verification; it is not automatically more private, scalable, or legally suitable.

## 3. The free business capability floor

The free tier must be useful for a real small business, not just a trial or a lead-generation funnel. Subject to each repository's declared license and its obligations, the proposed floor includes:

- quotes, invoices, expense tracking, customer/supplier records, and exportable reports;
- basic inventory, purchase orders, scheduling, tasks, and operational checklists;
- open APIs, documented data formats, bulk import/export, and self-hosting;
- multi-currency price viewing and basic locale-aware formatting;
- basic security controls, access logs, backup/export guidance, and upgrade/exit instructions;
- a product/offer comparison worksheet and a transparent savings receipt;
- templates for service scope, supplier requests, delivery acceptance, disputes, and business launch;
- accessible interfaces, localization infrastructure, and translations that communities can contribute.

**Do not put these behind an upgrade wall:** data export, account security controls, access to one's own records, machine-readable APIs for core data, disclosure of fees, the ability to exit, or the ability to use the self-hosted core.

Paid services may cover real costs: managed hosting, support and maintenance, professional services, payment processing, freight, storage, picking/packing, specialist compliance work, and high-volume compute. Those costs must be stated before commitment. Third-party fees are not made free merely because the interface is free.

Prohibited monetization patterns:
- selling customer or employee data;
- mandatory advertising or paid supplier placement presented as neutral ranking;
- undisclosed commissions, supplier rebates, or referral fees;
- charging users to retrieve their own data or leave;
- price discrimination based on confidential user data;
- hidden handling, FX, membership, or delivery fees;
- using an AI recommendation to conceal the commercial reason for a ranking.

Publish fee schedules and rebate-allocation rules in machine-readable as well as human-readable form. Report the operating cost of shared services and how fees or surplus are allocated. The project must earn enough revenue, grants, or contributions to maintain security and support; “free” must not mean relying on unpaid labor indefinitely.

**No franchise-style turnover royalty by default.** Prefer published flat service fees, cost-based logistics charges, or a clearly disclosed fixed transaction fee. A fee tied to realized savings may be tested only with explicit consent, a published cap/formula, an independently reproducible baseline, and no incentive to inflate savings. Any optional dues must buy defined services or member rights; they must not be a toll for accessing one's own records or operating an independent business.

## 4. Global-first data and interoperability rules

Never encode one country's assumptions into the shared domain model. Country-specific behavior belongs in versioned jurisdiction packs and provider adapters.

### 4.1 Parties and identifiers

Distinguish all of these explicitly:
- a natural person or user;
- an organization/network/cooperative;
- a legal entity;
- a branch or operating site;
- a jurisdiction-specific registration or tax account;
- a supplier account in an external system;
- an individual acting under a scoped mandate.

Use stable platform identifiers (UUIDs or an equally well-defined collision-resistant scheme) and keep external identifiers as typed, issuer-scoped references. A company registration number, VAT number, GS1 location identifier, email address, Holochain agent key, and local database ID are **not interchangeable**. One organization may operate multiple legal entities and establishments in different jurisdictions. Never infer that two records are the same party from a name alone.

### 4.2 Currency and money

- Store currency as an explicit ISO 4217 code and use locale data such as Unicode CLDR only for display.
- Represent amounts as decimal strings or integer minor units paired with the currency's applicable precision. Never use binary floating point for posted financial amounts.
- Preserve the original transaction currency and the exact rounding method.
- For a currency conversion, store the source, rate, timestamp, method, and whether the rate is indicative or contractually binding.
- A quote must pin its currency and validity period. Do not silently refresh exchange rates after acceptance.
- Compare prices only after normalizing quantity, packaging, tax treatment, delivery, payment terms, quality/specification, and the FX basis. Never compare bare amounts with different currencies or different units.

### 4.3 Products, packs, and units

Use recognized product identifiers such as GTIN where available, while allowing supplier SKUs, local identifiers, unbarcoded goods, and service items. A GTIN is not required for every item. Maintain provenance and licensing metadata for imported catalog data; do not assume an identifier provides unrestricted access to someone else's product database.

Currency registries must preserve effective dates and superseded entries rather than treating three-letter codes as timeless facts. SIX, the ISO 4217 Maintenance Agency, records the Bulgarian lev (BGN) as historical from 1 January 2026 and the euro (EUR) as the current currency following Bulgaria's euro-area accession. Purchase and invoice records must preserve transaction-date currency context; current-code validation is not a substitute for historical interpretation. See the [official SIX currency-code maintenance page](https://www.six-group.com/en/products-services/financial-information/market-reference-data/data-standards.html).

A product identity must be distinct from a supplier's offer. Keep at least:
- product and variant identity;
- brand, model, grade, and specification where relevant;
- pack count and contents, keeping commercial packaging units distinct from physical measurement units;
- an explicit code-system URI for every order unit, content unit, price unit, and observed-stock unit;
- unit of measure and permissible conversions, with conversions defined and reviewed rather than inferred from matching labels;
- use UN/CEFACT Recommendation 20 for suitable measurement units and Recommendation 21 for packaging types where their code lists fit; retain vendor-defined units only under an explicit, issuer-scoped code-system URI;
- origin/manufacturer claims where supported;
- identifiers and their issuers;
- source, license, observation time, and confidence for catalog attributes.

Do not treat near-matching names or AI similarity as proof of equivalence. Substitution requires a visible specification comparison and buyer consent. For traceability-critical flows, support batch/lot and shipment events through standards-based adapters.

### 4.4 Supplier offer and landed cost

Every offer should contain, directly or by immutable reference:
- supplier identity and relevant establishment;
- product, pack size, exact specification, and minimum order;
- quantity breaks, available quantity, and stock freshness;
- offer currency, unit price, tax inclusivity/exclusivity, discounts, and validity;
- shipping origin, destination/service area, freight, handling, insurance, duties and other known charges;
- delivery window, payment terms, return/warranty terms, and substitution policy;
- provenance of the quotation and the actor authorized to issue it;
- applicable jurisdiction-pack and schema versions.

A comparison should calculate **estimated landed cost** and later replace estimates with actual invoiced and delivered cost. Unknown charges must appear as unknown—not silently become zero.

### 4.5 Time, language, units, and accessibility

Store machine timestamps in a defined timezone-aware format and preserve the source timezone where it changes meaning. Render dates, numbers, currency, and plural forms using locale data rather than string concatenation. Keep machine protocols language-neutral and translate user-facing explanations. Support multiple languages per organization and allow documents to preserve both original text and a reviewed translation.

Make the core resilient to intermittent connectivity. Users may prepare drafts offline, but a purchase commitment must revalidate quote expiry, stock, authority, and applicable rules. A stale offline snapshot cannot authorize a new commitment.

### 4.6 Open protocol strategy

Prefer adapters and recognized standards over a proprietary universal protocol:
- OpenAPI for published HTTP interfaces;
- Peppol BIS Billing / EN 16931 where applicable for electronic invoice exchange;
- ISO 4217 currency codes and ISO 3166 country codes where applicable;
- GS1 identifiers and EPCIS events where appropriate for supply-chain exchange;
- versioned CSV/JSON exports for simple portability and offline workflows.

Do not require every supplier to adopt our stack. Provide import/export, manual entry, CSV, and an adapter path; publish compatibility matrices and test fixtures. Standards have profiles, licensing, and local implementation details—“supports the standard” is a scoped, tested claim, not a marketing checkbox.

## 5. Jurisdiction packs: globally designed, locally activated

A jurisdiction pack is a versioned bundle of rules, forms, integration configuration, and review evidence. It is not a list of country names and must not be marketed as legal advice or automatic compliance.

Keep distinct profiles for:
- legal entity and establishment;
- sales / VAT / GST / duties and invoice rules;
- payments and settlement rail;
- privacy, data retention, localization, and deletion;
- consumer protection, refunds, warranties, and marketplace duties;
- employment and contractor workflows;
- sector-specific product safety, food safety, traceability, and licensing;
- imports/exports, sanctions screening, customs, and restricted goods;
- cooperative form, member rights, governance, and distribution of surplus.

Each rule needs a source, jurisdiction/scope, effective date, expiry or review date, maintainer, test cases, and release version. A rule can be active, deprecated, suspended, or unreviewed. When a required rule is absent or stale, the affected regulated transaction must be blocked or moved to an explicitly non-transactional/manual path with a clear reason. It must not default to “compliant.”

Distinguish four states in product UI and documentation:
1. **Available globally:** generic software capabilities that do not imply local regulatory approval.
2. **Technically tested:** adapter behavior was exercised against named fixtures/systems.
3. **Locally reviewed:** named qualified reviewers checked a defined legal/regulatory scope and date.
4. **Supported for transactions:** technical tests and local review are current, the provider contract is active, and operational responsibilities are documented.

The tax engine should be a replaceable rules adapter. Local tax obligations depend on transaction facts; OECD VAT/GST guidance is a useful international reference, not a substitute for domestic law. Payments, custody, lending, insurance, and regulated money movement should use qualified local providers initially. The network should not hold pooled member funds in its first pilot.

## 6. The cooperative buying workflow

Start with business-to-business supplies where product specifications and delivered prices can be compared reliably. Begin in a selected pilot market because supplier density, delivery routes, and demand matter economically; do not hard-code that market into the product or data model.

### Proposed transaction lifecycle

1. **Discover:** buyer searches or imports a product; the system keeps source, unit, pack, and specification visible.
2. **Declare demand:** buyer submits a non-binding quantity, acceptable substitution rules, destination, delivery window, and whether it may be included in an aggregated request.
3. **Aggregate:** combine only compatible demand. Show participants what will be shared and with whom; use a threshold or minimum-volume commitment only when terms are explicit.
4. **Request comparable offers:** invite suppliers to respond to a defined specification and delivery condition. Use sealed or access-controlled quotations where appropriate.
5. **Compare:** normalize quotes into estimated landed cost, quality/specification, fulfillment, payment, tax, warranty, and uncertainty. Disclose commissions and rebates.
6. **Present the decision:** show each buyer their expected price, fees, savings baseline, expiry, risks, and alternatives. Symthaea may explain tradeoffs but cannot commit a buyer.
7. **Authorize:** each buyer accepts their own order, or an expressly authorized cooperative/agent acts within a recorded mandate. Silence, an AI recommendation, or participation in an interest poll is not consent.
8. **Contract and fulfill:** the supplier and legally responsible buyer are identified unambiguously. Order records are idempotent; retries cannot create duplicate purchases.
9. **Reconcile:** confirm quantities, substitutions, delivery, tax, invoice, payment status from the authoritative source, and unresolved disputes.
10. **Issue a savings receipt:** compare equivalent goods/services against a defensible baseline and include all fees and known costs, with evidence references and limitations.

Non-binding interest must never be presented as a binding order. If a minimum-demand threshold is not met, explain whether the offer expires, is repriced, or may be cancelled; do not quietly raise a buyer's price.

### Start as a software-assisted buying network, not a reseller

For the first pilot, the lowest-complexity operating hypothesis is: the network aggregates demand and negotiates or solicits quotes, while each buyer accepts and contracts for its own purchase and the supplier invoices each buyer directly. This limits—not eliminates—legal, tax, credit, and competition-law complexity. Local counsel must validate actual roles and contracts before use.

Do not take title to inventory, operate escrow, extend credit, underwrite buyers, or act as merchant of record until the intended model, capital, tax treatment, consumer obligations, licensing, liability, and disputes have been reviewed for the relevant jurisdictions.

### Separate non-binding demand from binding authority

Use the [purchase-intent schema](../schemas/cooperative-purchase-intent-v1.schema.json) and [declaration/semantic validator](../tools/validate_cooperative_purchase_intent.py) as the proposed v1 boundary. The schema distinguishes `non_binding_interest`, `quote_request`, and `binding_order`. Non-binding states cannot carry an accepted offer or order authorization. A binding-order declaration requires an exact offer ID/revision/digest, a one-time idempotency key, and a single-use authorization scoped to the buyer legal entity and a spending ceiling.

The consent record also names explicit recipient IDs; a scope label such as “approved supplier set” is not sufficient by itself. Its RFC 8785 JCS digest binds the buyer entity, purpose, recipient set, exact allowed fields, and the values selected for sharing. Changing any of these requires a new digest and consent. The semantic validator also computes SHA-256 over RFC 8785 JSON Canonicalization Scheme (JCS) bytes for the exact purchase scope: intent ID/revision/idempotency key; buyer party/legal entity/site/cooperative; offer ID/revision/digest; product reference/specification digest; requested quantity/unit/code-system; destination and delivery window; substitution policy; consent ID and consent-scope digest; the exact consent-evidence reference and digest; accepted total; and authorized maximum total. Use the pinned canonicalization library and shared regression vectors rather than a language's default JSON serializer. Revision integers are bounded to JCS's interoperable safe-integer range; monetary amounts and quantities remain decimal strings to preserve exactness. A material change invalidates the old digest and requires fresh authorization. A JCS digest detects changes across language implementations; it is **not** proof that a key-holder genuinely authorized the transaction. The current Python validator is a reference/CI tool, not a production service; any Rust execution path must use a tested RFC 8785 implementation and shared conformance vectors before activation.

The validator always reports `transaction_authorized: false`. Before any real order, a separate execution boundary must verify the mandate signature and issuer/revocation state, resolve the exact supplier offer and digest, check authorization freshness against the current clock, confirm funds/payment arrangement and jurisdiction rules, reserve or reconfirm availability, and atomically claim the idempotency key in durable storage. If any check is missing, stale, ambiguous, or unavailable, submission must stop. Retrying the same intent must not create a second purchase.

### Competition and fairness safeguards

Pooling purchasing can improve buyer leverage, but a multi-business network also creates risks around competitively sensitive information. Before operating a buyer group, obtain jurisdiction-specific competition-law review. Design for data minimization:
- reveal only the aggregate demand necessary for a supplier quote;
- segregate each buyer's confidential downstream costs, resale plans, customer lists, future prices, wages, and output plans;
- never recommend or coordinate members' resale prices;
- prevent one member from inspecting another member's confidential quotes or order history without permission;
- record the mandate, participating buyers, disclosed fields, purpose, access, and expiry;
- keep supplier selection criteria and rebate allocation transparent and auditable.

Member autonomy is a system invariant: each business independently chooses whether to join a pool, accepts its own commercial obligations, and sets its own downstream prices and operating strategy.

## 7. Build a better Costco in stages

Costco's high-volume, limited-assortment, low-merchandise-margin model is worth studying, but the first version should not copy the capital-intensive parts. The goal is to preserve purchasing and logistics economies of scale while distributing access and ownership.

| Stage | Build | Explicitly defer |
|---|---|---|
| 0 — Free tools | Catalog import, comparable-unit calculator, supplier directory, quote request, export, cost/savings receipts | Paid membership gate, inventory ownership |
| 1 — Group demand | Non-binding demand pools, buyer consent, quote windows, minimum-volume calculator | Binding group orders, pooled funds |
| 2 — Independent orders | Authorized individual orders, supplier-direct invoicing, delivery confirmation, disputes | Acting as reseller or merchant of record |
| 3 — Consolidated logistics | Partner pickup points, route/consolidation quotes, measured handling costs | Owned fleet and warehouse leases |
| 4 — Member-owned buyer organization | Locally reviewed cooperative/agency structure, transparent dues, rebate/surplus policy, member governance | Global legal entity assumed to fit all countries |
| 5 — Physical wholesale hubs | Cross-dock, local stock, warehouse operation, return/refund flow after demand-density proof | Premature private-label inventory and large fixed costs |
| 6 — Producer and manufacturing network | Forecast-backed supply commitments, local production, repair/reuse/circular flows | Automated production commitments without physical qualification |

Product categories should expand by measured unit economics and safety burden, not by ambition alone. Begin with non-perishable, specifiable business consumables or hardware/maintenance supplies sourced locally or regionally. Test food later, beginning with an appropriately regulated, narrow category such as shelf-stable products; fresh food introduces perishability, temperature control, traceability, and higher waste risk.

The first physical node should be selected by demand density and supplier/logistics economics, not by a global headquarters assumption. Global architecture means that separate regional operators can use the same contracts and exchange only the information they have agreed to share.

## 8. Truthful savings: the core trust product

Do not market the difference between a web list price and a negotiated quote as realized savings. The receipt should show the arithmetic and the evidence.

For a comparable basket:

**Net cost difference = baseline merchandise + baseline other costs − actual merchandise − actual other costs − participation costs.**

The [Rust calculation kernel](../crates/cooperative-commerce-core/README.md) implements this arithmetic for exact, specification-matched baskets, and the [Savings Receipt schema](../schemas/cooperative-savings-receipt-v1.schema.json) defines the portable record. The Rust core now has a typed receipt model that recomputes the claimed report from its typed input snapshot, while rejecting incomplete computed-receipt metadata. Until a portable serializer/JCS envelope and independent source/signature verifier are implemented, report this only as a computed difference—not independently verified or causally attributed savings.

The baseline and actual totals must use equivalent specification, quantity, pack, tax treatment, delivery destination, payment terms, timing, and quality. Include all membership, handling, freight, FX, storage, financing, spoilage, and substitution costs that materially differ. If the baseline is an estimate rather than a buyer's real alternative quote or invoice, label it as estimated. Keep FX conversions tied to a disclosed source and timestamp. Show both absolute savings and percentage savings, as well as missing data.

Do not aggregate a savings claim until the evidence can be independently recalculated. For a batch, publish eligible buyer count, fulfilled quantity, rejected/substituted quantity, delivery failures, total fees, refunds/credits, baseline provenance, and exclusions. Report negative or zero savings too.

### Minimum pilot acceptance gates

The following are proposed gates—not projected results:
- every compared item has a confirmed specification/pack/unit mapping or is explicitly excluded;
- at least two comparable supplier offers for a category/region, or an explicit report that competitive comparison was unavailable;
- every fee, rebate, and material unknown is shown;
- no purchase is committed without buyer or mandate-specific authorization;
- no duplicate order appears under retry, timeout, or reconciliation scenarios;
- realized savings recompute from retained source records;
- supplier fills, substitutions, delivery timeliness, returns, and disputes are measured;
- participant data access and exit/export tests pass;
- a locally reviewed operating model exists for the pilot jurisdiction;
- a negative-savings outcome is a valid result, not a test failure hidden from members.

## 9. Symthaea and Mycelix: use them where they create measurable value

**Symthaea** can propose demand forecasts, surface anomalous quotes, flag unit/pack mismatches, forecast spoilage, or explain route and supplier tradeoffs. Every suggestion should include inputs, uncertainty, model/version identity where material, and a way to inspect or reject it. Predictions do not prove supplier intent, product equivalence, actual savings, or authorization.

**Mycelix** can support federated supplier credentials, scoped sharing consent, governance decisions, product provenance claims, and signed receipt exchange when multiple independent organizations need to validate the same claim. Store sensitive transaction details with their accountable owners and disclose only the minimum proof/reference required. A signature proves who signed a record under a key; it does not independently prove the underlying claim is true.

**Holochain is optional per trust boundary.** Do not put all prices, customer identities, invoices, raw demand, personal data, or trade secrets into a globally replicated DHT. Prefer private records with owner-controlled retention and signed, minimal receipts for cross-organization verification. Test key recovery, revocation, peer unavailability, stale data, and conflicts before relying on federation for critical decisions.

Keep payment, accounting, catalog, order, and logistics adapters replaceable. The first service should work manually and with CSV import/export before any integration is described as live or production-qualified.

## 10. Global launch without global overclaiming

Launch the **architecture** globally; launch regulated transactions one reviewed jurisdiction at a time. Choose the first market based on willing buyers, credible supplier quotes, accessible logistics, and the ability to calculate a complete baseline—not because the model hardcodes that country.

For every jurisdiction, maintain a release manifest with:
- supported transaction types and explicit exclusions;
- relevant local legal entity/agent role and contracts;
- tax/invoice rules and the authority/source/version for each;
- payment/logistics providers and who bears each responsibility;
- data location, retention/deletion, access, breach, and cross-border transfer model;
- consumer, product, import/export, and sector-specific duties in scope;
- competition-law review for multi-business purchasing;
- tests, reviewer, review date, open limitations, and automatic review expiry where appropriate.

A missing pack must not silently inherit another country's rules. It should still be possible to use generic free business software and explore product data without making a regulated sale or purchase commitment.

## 11. What to implement next

The following items remain open. The existing schemas, Python reference/CI validators, synthetic fixtures, and Rust calculator are proposed prototypes; they do not make this a live commerce system.

1. **Official code-list bundle:** the proposed [code-list snapshot manifest](../schemas/code-list-snapshot-manifest-v1.schema.json) defines the contract for versioned, digest-pinned currency, country, unit, and packaging registries, including source authority, retained source and normalized payload digests, parser/build identity, duplicate-code checks, review evidence, effective dates, and review deadline. The [illustrative SIX manifest](../business-packs/cooperative-buying/fixtures/example-code-list-manifest.json) is marked not retrieved / not run and cannot be activated. The [manifest semantic validator](../tools/validate_code_list_manifest.py) rejects active declarations without a non-placeholder immutable version identity, retained digests, required checks, sample reconciliation against the publisher source, an independent-review check, a passed validation declaration, current review deadline, and valid effectivity window. The manifest still does not verify these claims against retained bytes or signatures; eligibility is only a metadata gate. The Rust commerce core now contains a dependency-free in-memory record-semantics layer for effective-dated membership lookup and a strict SIX List One XML importer that emits deterministic normalized JSON from a retained local payload. The importer validates currency tuples, aggregates repeated rows, and rejects DTD/external entities and conflicting records. It does not download/authenticate source bytes, verify publisher signatures, evaluate licensing, or enable registry activation. Next: create a reproducible acquisition/review workflow and validate against retained official bytes before claiming publisher-data coverage. Use current maintained sources such as SIX for ISO 4217 and UNECE Recommendations 20/21 for appropriate trade units; syntax alone is not membership validation.
2. **Rust receipt verifier and serializer:** map the offer, purchase-intent, and savings-receipt schemas into Rust types; use RFC 8785 JCS conformance vectors; recompute receipt totals from source snapshots; compare currencies, categories, delivered quantities, and digests; and emit a receipt only when all required invariants pass. Do not turn the Python reference validators into a production service.
3. **Evidence and authority adapters:** verify source bytes against digests, verify signatures and issuer/revocation state, bind consent to exact sharing scope, and enforce single-use idempotency in durable storage. Unknown or unavailable authority must block a binding order.
4. **No-purchase UI prototype:** import supplier CSVs, normalize product/pack/unit data, show unmatched products and unknown charges explicitly, compare landed cost with exact decimal arithmetic, and export the receipt and input snapshot.
5. **Reviewed conversions:** add explicit unit conversion rules and FX conversion records with source, timestamp, effective period, precision/rounding rule, and verifier. Until the relevant rule/record is supported, refuse cross-unit or cross-currency comparisons.
6. **Manual-first buying pack:** add buyer consent, supplier RFQ, quote comparison, order mandate, delivery checklist, and dispute record. Test stale quotations, duplicate retries, unauthorized orders, partial deliveries, substitutions, refunds, credits, and negative net outcomes.
7. **Jurisdiction qualification:** complete local legal review for the exact buyer/cooperative/supplier roles before real multi-business pooled buying or financial flows.
8. **Evidence-based pilot:** run one product category in one region, report positive and negative outcomes, and activate further jurisdictions only while their rule packs, providers, and reviews remain current.

Do not start with a token, a proprietary payment rail, a warehouse, a global corporation structure, or an AI buyer that commits other people's money. Those are optional future choices, not prerequisites for proving the economics.

## References and standards to adapt

- [International Cooperative Alliance — Guidance Notes to the Co-operative Principles](https://ica.coop/en/media/library/the-guidance-notes-on-the-co-operative-principles)
- [U.S. Federal Trade Commission — A Consumer's Guide to Buying a Franchise](https://search.ftc.gov/business-guidance/resources/consumers-guide-buying-franchise) (illustrates why required continuing royalties and franchise controls deserve scrutiny; U.S.-specific guidance, not global law)
- [European Commission — Horizontal Guidelines on purchasing agreements](https://competition-policy.ec.europa.eu/system/files/2022-03/kd0722013enn_purchasing_agreements.pdf) (illustrates that buyer-group structure and effects require context-specific competition analysis; EU-specific guidance)
- [Costco Wholesale Corporation — 2025 Annual Report](https://s201.q4cdn.com/287523651/files/doc_financials/2025/ar/COST-Annual-Report-2025.pdf) (reference model for studying high-volume wholesale operations; not a template to copy wholesale)
- [OpenPeppol — BIS Billing 3.0](https://docs.peppol.eu/poacc/billing/3.0/bis/)
- [GS1 — Global Traceability Standard](https://www.gs1.org/standards/gs1-global-traceability-standard/current-standard)
- [GS1 — EPCIS and Core Business Vocabulary](https://www.gs1.org/standards/epcis)
- [SIX — ISO 4217 currency-code maintenance and current/historical lists](https://www.six-group.com/en/products-services/financial-information/market-reference-data/data-standards.html) — code-list source, not a settlement provider.
- [UNECE — Code-list Recommendations, including Recommendations 20 and 21](https://unece.org/code-list-recommendations) — measurement and package-type vocabularies for trade.
- [OECD — International VAT/GST Guidelines](https://www.oecd.org/en/publications/international-vat-gst-guidelines_9789264271401-en.html)
- [OECD — The Role of Digital Platforms in the Collection of VAT/GST on Online Sales](https://www.oecd.org/en/publications/the-role-of-digital-platforms-in-the-collection-of-vat-gst-on-online-sales_e0e2dd2d-en.html)
- [OpenAPI Specification](https://spec.openapis.org/oas/)
- [RFC 8785 — JSON Canonicalization Scheme](https://www.rfc-editor.org/rfc/rfc8785) — portable canonical bytes for signatures and hashes across implementations.
- [Peppol BIS Billing 3.0 validation principles](https://docs.peppol.eu/poacc/billing/3.0/bis/) — illustrates layered syntax, code-list, logical-correlation, and country-specific validation.
- [Unicode CLDR — Number and Currency Formatting](https://unicode.org/reports/tr35/tr35-numbers.html)
- [Open Food Facts — Data Sources and Licenses](https://www.myfoodfacts.org/en/data-sources) (review source-specific license requirements before importing any dataset)

These references are implementation starting points. They do not replace current legal, standards-conformance, privacy, competition, or sector-specific review in a target market.
