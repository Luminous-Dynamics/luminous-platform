# Connectivity Unit Economics Core

Integer-only Rust calculator for fully burdened community-connectivity cost, user-charge caps and residual subsidy needs.

## Run

From the repository root with Rust installed:

    cargo test --manifest-path tools/connectivity-unit-economics-core/Cargo.toml --all-targets
    cargo clippy --manifest-path tools/connectivity-unit-economics-core/Cargo.toml --all-targets -- -D warnings

## Units and ledger semantics

All money is represented in integer minor units of a single explicit three-letter currency (for example, ZAR cents). Cost lines are monthly amounts. Capital assets use conservative straight-line monthly amortization with a disclosed useful life and residual value. Volunteer labour uses thousandths of an hour per month and an imputed replacement rate.

The calculator returns separate:
- cash operating costs;
- purchased-capital amortization;
- donated-capital replacement-cost shadow value;
- volunteer-labour shadow value;
- cash-recovery monthly cost;
- fully burdened monthly economic cost;
- external funding, the permitted maximum user contribution and the cash/economic residual funding gaps;
- cost per active household and per successful task, plus task success ratio.

A zero denominator yields `None`; mixed currency, duplicate/invalid IDs, invalid capex, success counts above attempts, missing volunteer rate and output overflow are errors. It does not use floating-point money arithmetic.

## Critical limitations

This core does not scrape quotes, validate invoices, collect data from households, determine cost allocation policy, measure task quality, or authorize resale/sharing. All cost inputs must first be normalized to a named accounting month and one currency. One-time installation charges, annual plans, variable service caps, grant restrictions, FX and shared-site allocations need explicit preprocessing rules.

Donated assets and unpaid labour are not monthly cash costs, but remain visible as replacement-cost economic inputs. “Zero user charge” is a funding scenario, not a statement that capacity is costless: when the user cap is zero, the reported funding gap shows what external monthly funding is still needed.

The detailed methodology, South African baseline observations and proposed benchmarks are in `docs/CONNECTIVITY_UNIT_ECONOMICS_V1.md`. Quoted prices are dated context only; verify the current offer terms, coverage, taxes and eligibility before using a comparison in a real decision.
