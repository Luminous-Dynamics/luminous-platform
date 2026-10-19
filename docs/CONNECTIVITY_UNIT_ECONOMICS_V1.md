# Connectivity unit economics and zero-user-charge model v1

**Status:** draft methodology + integer-only Rust calculator, not a tariff survey, field-cost measurement or funding commitment.  
**Issue:** https://github.com/Luminous-Dynamics/luminous-platform/issues/69  
**Branch:** `feat/connectivity-unit-economics-v1`  
**Research snapshot:** 18 October 2026.

## 1. Paradigm shift: make “free for the user” a solvable budget equation

Upstream capacity, equipment, power, maintenance, support and labour have real costs. The credible promise is not that bytes become physically costless; it is that people can access a defined service without a charge to them because costs are lower, resources are pooled and a transparent external funding mechanism covers the remainder.

For each named pilot, calculate and publish two ledgers.

**Cash-recovery ledger**
- monthly upstream transit/service charges;
- power and conversion losses;
- site rent, installation and maintenance;
- paid support and labour;
- tax, insurance and loss reserve;
- straight-line amortization of purchased hardware.

**Fully burdened economic ledger**
- all cash-recovery entries;
- amortized replacement cost of donated hardware;
- replacement-cost valuation of volunteer labour.

Donations and volunteer effort are not cash outflow for this month, but pretending they have zero economic cost hides whether a deployment can be repaired and replicated without endless charitable work. Conversely, the economic ledger must not misreport replacement value as cash that was actually spent. Keep both.

## 2. Current South African reference observations

These observations establish context and potential baselines—not hard-coded assumptions in the calculator.

| Source / quote observed | Published reference | Comparability warning |
|---|---|---|
| ICASA, *State of the ICT Sector in South Africa 2026*, dated 31 March 2026 | Standardized 2 GB mobile-broadband basket: R152; entry-level fixed broadband: R309, and the report notes the entry fixed basket is above its stated affordability benchmark | Standardized national basket and entry-fixed basket are not the same product, user profile, coverage or data need as a community-network pilot. |
| Vodacom prepaid LTE page, accessed 18 Oct 2026 | Listed 5 GB Anytime + 5 GB Night Owl 30-day bundle for R99 | Day/night split means it is not equivalent to 10 GB available at any time, or necessarily equivalent to the ITU/ICASA standardized basket. Promotions/eligibility can change. |
| Openserve Connect, accessed 18 Oct 2026 | Selected month-to-month 40 Mbps unlimited offer at R425/month | Availability is limited to served areas; installation, equipment, coverage, contention, contract terms and the exact selected plan must be recorded. |

Source URLs:
- ICASA 2026 report: https://www.icasa.org.za/legislation-and-regulations/the-state-of-the-ict-sector-report-in-south-africa-2026-report
- Vodacom prepaid plans: https://www.vodacom.co.za/vodacom/shopping/plans/prepaid-plans
- Openserve Connect: https://connect.openserve.co.za/

A comparison is admissible only when it names the exact quote/plan, price observation date, validity, taxes, required device, installation, coverage, speeds/latency if measured, data cap/FUP, time-of-day restrictions, and task/quality scope. Do not infer current price or universal availability from these dated observations. If a provider changes a tariff, create a new quote record; do not overwrite history.

## 3. Core outputs and units

The Rust core uses integer minor units in one explicit three-letter currency (for example, ZAR cents). It avoids binary floating point for money and rejects mixed currencies. It is a calculator over values supplied by a caller; it does not fetch provider tariffs or decide which baseline is equivalent.

Definitions for one accounting month:

`cash_operating_cost = sum(monthly cash operating cost lines)`

`paid_capex_amortization = sum(ceil((asset cost - residual value) / useful life months)) for purchased assets`

`donated_capex_shadow_cost = same amortization formula for donated assets`

`volunteer_shadow_cost = ceil(volunteer hours × imputed replacement rate)`

`cash_recovery_cost = cash_operating_cost + paid_capex_amortization`

`fully_burdened_economic_cost = cash_recovery_cost + donated_capex_shadow_cost + volunteer_shadow_cost`

`maximum_user_contribution = active households × allowed charge cap per household`

`cash_funding_gap = max(0, cash_recovery_cost - external funding - maximum user contribution)`

`fully_burdened_funding_gap = max(0, fully_burdened_economic_cost - external funding - maximum user contribution)`

All calculations use checked integer arithmetic, with conservative upward rounding for amortization and per-unit costs. A missing denominator produces `None`, not a fabricated zero. The current per-household and per-task unit costs are allocated averages, not marginal-cost estimates.

The model reports the implied per-active-household cash charge required to balance the month if external funding stays at its recorded level. That implied charge is not a recommendation and may exceed the allowed charge cap. If the goal is zero user charge, set the cap to zero and the resulting funding gap represents the external monthly funding still required.

## 4. Benchmark definitions and acceptance targets

These targets are proposed software/economic-contract requirements, not evidence that a real network is cheaper.

| Metric | Initial benchmark target | Method |
|---|---|---|
| Arithmetic determinism | 100% identical outputs for identical integer inputs | Replay saved fixtures; bind inputs and source revisions. |
| Currency safety | Zero silent mixed-currency calculations | Negative tests with currency mismatch in operating, capital and funding lines. |
| Overflow | Zero wrapped or saturated costs silently accepted as valid totals | Boundary fixtures must produce explicit arithmetic errors when outputs cannot fit the declared result units. |
| Denominator safety | No invented per-household/per-task cost when denominator is zero | Zero households, zero attempts, and zero successful tasks. |
| Ledger reconciliation | Cash and fully burdened formulas reconcile exactly | Separate purchased capital, donated asset shadow cost and volunteer labour. |
| No double counting | Zero duplicate ledger IDs accepted | Duplicate IDs across operating lines, capital assets and funding sources. |
| Zero-user-charge funding | Always report the residual external funding gap | User contribution cap = 0; no claim of “free” when a positive gap remains. |
| Sponsored coverage | External funding that fully covers cash recovery yields zero cash gap | Cases for partial, exact and excess funding, but show economic gap separately. |
| Cost per useful task | Baseline first, then reduce | Use tasks with the same declared success/quality/security criteria. Failed tasks remain in attempt/success denominators. |
| Affordability distribution | Baseline first by income band and household size | Do not infer household income from network telemetry; use consenting, appropriately aggregated survey/official statistics if a field study needs it. |
| Auditability | 100% of public comparison inputs have a source, date, exclusions and immutable revisions | Keep quoted source record and raw trial/cost denominators. |

## 5. Rust core and explicit boundaries

`tools/connectivity-unit-economics-core/` is a pure Rust library with no runtime dependencies.

- Uses integer minor units and explicit currency; rejects invalid currency codes and mismatches.
- Computes monthly operating totals, purchased capital amortization, donated asset shadow costs, imputed volunteer-labour costs, unit costs and residual funding gaps.
- Rejects duplicate or invalid line IDs, invalid useful lives/residual values, successes greater than attempts, and volunteer time without a nonzero imputed rate.
- Uses `None` when a unit-cost denominator is zero, and checks arithmetic/result-width overflow.
- Does not collect, scrape or validate provider quotes; authenticate invoices; measure task quality; allocate shared costs scientifically; or certify a real network is legally operated.
- Assumes input cost lines are already normalized to one month. Costs paid annually, one-time connection fees, variable caps, foreign-exchange exposure, bad debt, grant restrictions and shared-site allocations need explicit preprocessing/accounting policy before being entered. Do not silently divide a one-off installation charge as if it were monthly operating cost.
- Straight-line amortization is a simple disclosed policy, not an assertion about accounting/tax depreciation, physical failure or a component's true lifetime. Record chosen lifespan and residual value with the evidence.

## 6. Decision rule for a pilot

Before comparing “cellular vs fixed vs community network,” define a small basket of successful tasks: for example, an authenticated web transaction, a bounded document upload with checksum verification, a learning resource retrieval, and a defined-duration voice session. Run the same basket over each option at the same site and period. Record both monthly charges and task results.

Do not compare raw price alone. A subsidized 5 GB bundle, fixed unlimited connection and community Wi-Fi with local caches differ in usage rules, equipment, uptime, contention, support and coverage. The relevant economic comparison is fully burdened cost per successful task at a declared quality level, alongside the funding gap and service availability.

Local caching can reduce upstream traffic for repeated resources; that does not remove electricity, hardware, update/freshness, maintenance and licensing costs. A zero-user-charge model is sustainable only if lower costs and explicit recurring external funds cover the cash ledger, with a plan to replace donated assets and volunteer labour over time.

## 7. Funding model to evaluate

Test multiple transparent funding sources without assuming any is committed:

- pooled upstream purchase across a community/venue, if provider terms permit;
- school, clinic, library, municipal or community-site service contracts;
- employer/SME-sponsored access or named task sponsorship;
- grants and philanthropic subsidy;
- community capital or cooperative funds with transparent opt-in governance;
- caching and local services where content licenses and upstream terms allow;
- capped hardship/zero-user-charge tiers funded by the sources above.

For each funding source record amount, currency, duration, restrictions, renewal probability only if evidenced, admin burden and whether it is cash or in-kind. A one-off grant should not be treated as permanent monthly funding. Avoid financing ongoing service from unpaid labour indefinitely.

## 8. Next implementation steps

1. Compile/test/lint the integer Rust core against its pinned CI toolchain.
2. Add a versioned input fixture format for dated tariff quotes and pilot bills, including source URL, timestamp, limits, tax, installation and eligibility.
3. Define an independent review of task matching and cost allocation; calculate both cash and fully burdened outputs from the same immutable input digest.
4. Gather one named-site baseline for cellular, fixed broadband and authorized shared/community Wi-Fi using current invoices/quotes and measured tasks.
5. Publish the actual cost-per-task distributions, failed-task denominators, local-cache hit ratio, monthly funding gap, and excluded/unavailable options. Do not publish a savings percentage until the matched data supports it.

## 9. Regulatory and provider boundary

This economic model does not grant permission to share or resell upstream connectivity. Any public-facing network requires deployment-specific review of provider terms, applicable ECNS/ECS licensing or exemptions, type approval, spectrum permissions where relevant, installation safety and data privacy. Treat legal/provider authorization as a separate gate from the arithmetic.


## 10. Dated baseline registry

Two research observations are recorded in `profiles/connectivity-price-observations-south-africa-v1.json`, with shape/claim constraints in `schemas/connectivity-price-observations-south-africa-v1.schema.json`:

- ICASA's 2026 report presents a R152/month standardized 2 GB mobile-broadband basket for the **2024 reference period**.
- Vodacom's indexed prepaid LTE page lists R99 for 5 GB Anytime plus 5 GB Night Owl, valid for 30 days (with VAT included on that page).

The registry explicitly marks both observations `not_comparable_until_scope_matched`. They are not integrated as hard-coded defaults in the Rust calculator. The unit test runs Draft 2020-12 validation against the checked-in observation profile and verifies the non-comparability flag stays set. The data must be refreshed and cited again before a purchase or pilot decision; do not silently overwrite a historical quote.

The earlier Openserve R425/40 Mbps example is deliberately not present in this registry because the current retrieval did not independently reconfirm the exact official offer page in this run. It should be added as a new dated observation only after the exact source terms are reverified.
