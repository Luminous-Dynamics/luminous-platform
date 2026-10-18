//! Integer-only unit-economics core for community connectivity.
//!
//! Money is represented in the minor units of one explicit currency (for ZAR,
//! rand cents). Cost inputs are monthly operating lines. Capital items are
//! amortised straight-line over declared useful life. Donated assets and
//! volunteer labour are shown as economic shadow cost, separate from cash cost.
//!
//! This is a calculator, not a quote collector, tariff survey, accounting
//! system, financing recommendation, or proof of network quality.

use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CostCategory {
    UpstreamTransit,
    Energy,
    SiteRent,
    Maintenance,
    PaidLabour,
    Support,
    Tax,
    Insurance,
    Losses,
    OtherOperating,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FundingKind {
    Grant,
    Sponsor,
    PublicServiceContract,
    CooperativeCapital,
    OtherExternal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CostLine {
    pub id: String,
    pub currency: String,
    /// Cash cost for one accounting month, in currency minor units.
    pub monthly_amount_minor: u64,
    pub category: CostCategory,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapitalAsset {
    pub id: String,
    pub currency: String,
    /// Acquisition or current replacement value, in currency minor units.
    pub cost_minor: u64,
    pub residual_value_minor: u64,
    pub useful_life_months: u32,
    /// Donated assets are not current cash outflow, but their replacement cost
    /// is part of the separate fully burdened economic ledger.
    pub donated: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FundingSource {
    pub id: String,
    pub currency: String,
    pub monthly_amount_minor: u64,
    pub kind: FundingKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EconomicsInput {
    /// Three-letter uppercase currency code; all amounts use its minor units.
    pub currency: String,
    pub monthly_costs: Vec<CostLine>,
    pub capital_assets: Vec<CapitalAsset>,
    /// External funding excludes direct user charges, which are modelled by
    /// max_user_charge_minor_per_household.
    pub monthly_funding: Vec<FundingSource>,
    /// Labour input is in thousandths of an hour per month.
    pub volunteer_hours_milli: u64,
    /// Imputed replacement rate per hour in the declared currency's minor units.
    pub volunteer_hourly_rate_minor: u64,
    pub active_households: u64,
    pub useful_tasks_attempted: u64,
    pub useful_tasks_succeeded: u64,
    /// Direct user payment cap per active household per month; zero means free.
    pub max_user_charge_minor_per_household: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EconomicsError {
    InvalidCurrencyCode,
    DuplicateLineId,
    CurrencyMismatch,
    InvalidCapitalAsset,
    VolunteerRateMissing,
    SuccessesExceedAttempts,
    ArithmeticOverflow,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EconomicsResult {
    pub currency: String,
    pub monthly_cash_operating_cost_minor: u64,
    pub monthly_paid_capex_amortisation_minor: u64,
    pub monthly_donated_capex_shadow_cost_minor: u64,
    pub monthly_volunteer_labour_shadow_cost_minor: u64,
    pub monthly_cash_recovery_cost_minor: u64,
    pub monthly_fully_burdened_economic_cost_minor: u64,
    pub monthly_external_funding_minor: u64,
    pub maximum_monthly_user_contribution_minor: u64,
    pub cash_funding_gap_after_user_cap_minor: u64,
    pub fully_burdened_funding_gap_after_user_cap_minor: u64,
    pub implied_user_charge_per_household_for_cash_balance_minor: Option<u64>,
    pub cash_cost_per_active_household_minor: Option<u64>,
    pub fully_burdened_cost_per_active_household_minor: Option<u64>,
    pub cash_cost_per_successful_task_minor: Option<u64>,
    pub fully_burdened_cost_per_successful_task_minor: Option<u64>,
    /// Basis points (0-10000), rounded down; None if no tasks were attempted.
    pub task_success_ratio_bps: Option<u16>,
}

fn valid_currency_code(currency: &str) -> bool {
    currency.len() == 3 && currency.bytes().all(|byte| byte.is_ascii_uppercase())
}

fn ensure_currency(expected: &str, actual: &str) -> Result<(), EconomicsError> {
    if expected == actual {
        Ok(())
    } else {
        Err(EconomicsError::CurrencyMismatch)
    }
}

fn checked_sum<I>(values: I) -> Result<u128, EconomicsError>
where
    I: IntoIterator<Item = u128>,
{
    values.into_iter().try_fold(0_u128, |total, value| {
        total.checked_add(value).ok_or(EconomicsError::ArithmeticOverflow)
    })
}

fn to_u64(value: u128) -> Result<u64, EconomicsError> {
    u64::try_from(value).map_err(|_| EconomicsError::ArithmeticOverflow)
}

fn ceil_div(numerator: u128, denominator: u128) -> Result<u128, EconomicsError> {
    if denominator == 0 {
        return Err(EconomicsError::ArithmeticOverflow);
    }
    let quotient = numerator / denominator;
    if numerator % denominator == 0 {
        Ok(quotient)
    } else {
        quotient.checked_add(1).ok_or(EconomicsError::ArithmeticOverflow)
    }
}

fn capex_amortisation(asset: &CapitalAsset) -> Result<u64, EconomicsError> {
    if asset.cost_minor == 0
        || asset.useful_life_months == 0
        || asset.residual_value_minor > asset.cost_minor
    {
        return Err(EconomicsError::InvalidCapitalAsset);
    }
    let depreciable = u128::from(asset.cost_minor - asset.residual_value_minor);
    to_u64(ceil_div(depreciable, u128::from(asset.useful_life_months))?)
}

fn volunteer_shadow_cost(input: &EconomicsInput) -> Result<u64, EconomicsError> {
    if input.volunteer_hours_milli > 0 && input.volunteer_hourly_rate_minor == 0 {
        return Err(EconomicsError::VolunteerRateMissing);
    }
    let numerator = u128::from(input.volunteer_hours_milli)
        .checked_mul(u128::from(input.volunteer_hourly_rate_minor))
        .ok_or(EconomicsError::ArithmeticOverflow)?;
    to_u64(ceil_div(numerator, 1_000)?)
}

fn unique_id(id: &str, ids: &mut BTreeSet<String>) -> Result<(), EconomicsError> {
    if id.is_empty()
        || id.len() > 128
        || !id.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-')
        })
        || !ids.insert(id.to_owned())
    {
        return Err(EconomicsError::DuplicateLineId);
    }
    Ok(())
}

fn per_unit(total: u64, count: u64) -> Result<Option<u64>, EconomicsError> {
    if count == 0 {
        Ok(None)
    } else {
        Ok(Some(to_u64(ceil_div(u128::from(total), u128::from(count))?)?))
    }
}

pub fn calculate(input: &EconomicsInput) -> Result<EconomicsResult, EconomicsError> {
    if !valid_currency_code(&input.currency) {
        return Err(EconomicsError::InvalidCurrencyCode);
    }
    if input.useful_tasks_succeeded > input.useful_tasks_attempted {
        return Err(EconomicsError::SuccessesExceedAttempts);
    }
    let mut ids = BTreeSet::new();

    for line in &input.monthly_costs {
        unique_id(&line.id, &mut ids)?;
        ensure_currency(&input.currency, &line.currency)?;
    }
    for asset in &input.capital_assets {
        unique_id(&asset.id, &mut ids)?;
        ensure_currency(&input.currency, &asset.currency)?;
        if asset.cost_minor == 0
            || asset.useful_life_months == 0
            || asset.residual_value_minor > asset.cost_minor
        {
            return Err(EconomicsError::InvalidCapitalAsset);
        }
    }
    for source in &input.monthly_funding {
        unique_id(&source.id, &mut ids)?;
        ensure_currency(&input.currency, &source.currency)?;
    }

    let operating_cost = checked_sum(
        input.monthly_costs.iter().map(|line| u128::from(line.monthly_amount_minor)),
    )?;

    let paid_capex = checked_sum(
        input.capital_assets
            .iter()
            .filter(|asset| !asset.donated)
            .map(|asset| capex_amortisation(asset).map(u128::from))
            .collect::<Result<Vec<_>, _>>()?,
    )?;

    let donated_capex = checked_sum(
        input.capital_assets
            .iter()
            .filter(|asset| asset.donated)
            .map(|asset| capex_amortisation(asset).map(u128::from))
            .collect::<Result<Vec<_>, _>>()?,
    )?;

    let volunteer_cost = u128::from(volunteer_shadow_cost(input)?);
    let cash_recovery_cost = operating_cost
        .checked_add(paid_capex)
        .ok_or(EconomicsError::ArithmeticOverflow)?;
    let economic_cost = cash_recovery_cost
        .checked_add(donated_capex)
        .and_then(|value| value.checked_add(volunteer_cost))
        .ok_or(EconomicsError::ArithmeticOverflow)?;
    let external_funding = checked_sum(
        input.monthly_funding.iter().map(|source| u128::from(source.monthly_amount_minor)),
    )?;
    let maximum_user_contribution = u128::from(input.active_households)
        .checked_mul(u128::from(input.max_user_charge_minor_per_household))
        .ok_or(EconomicsError::ArithmeticOverflow)?;

    let available_cash_funding = external_funding
        .checked_add(maximum_user_contribution)
        .ok_or(EconomicsError::ArithmeticOverflow)?;
    let cash_gap = cash_recovery_cost.saturating_sub(available_cash_funding);
    let economic_gap = economic_cost.saturating_sub(available_cash_funding);

    let cash_cost_minor = to_u64(cash_recovery_cost)?;
    let economic_cost_minor = to_u64(economic_cost)?;
    let operating_cost_minor = to_u64(operating_cost)?;
    let paid_capex_minor = to_u64(paid_capex)?;
    let donated_capex_minor = to_u64(donated_capex)?;
    let external_funding_minor = to_u64(external_funding)?;
    let user_contribution_minor = to_u64(maximum_user_contribution)?;

    let implied_user_charge = if input.active_households == 0 {
        None
    } else {
        let required_from_users = cash_recovery_cost.saturating_sub(external_funding);
        Some(to_u64(ceil_div(required_from_users, u128::from(input.active_households))?)?)
    };

    let cash_cost_per_task = if input.useful_tasks_succeeded == 0 {
        None
    } else {
        Some(to_u64(ceil_div(cash_recovery_cost, u128::from(input.useful_tasks_succeeded))?)?)
    };
    let economic_cost_per_task = if input.useful_tasks_succeeded == 0 {
        None
    } else {
        Some(to_u64(ceil_div(economic_cost, u128::from(input.useful_tasks_succeeded))?)?)
    };
    let success_ratio = if input.useful_tasks_attempted == 0 {
        None
    } else {
        Some(
            ((u128::from(input.useful_tasks_succeeded) * 10_000)
                / u128::from(input.useful_tasks_attempted)) as u16,
        )
    };

    Ok(EconomicsResult {
        currency: input.currency.clone(),
        monthly_cash_operating_cost_minor: operating_cost_minor,
        monthly_paid_capex_amortisation_minor: paid_capex_minor,
        monthly_donated_capex_shadow_cost_minor: donated_capex_minor,
        monthly_volunteer_labour_shadow_cost_minor: to_u64(volunteer_cost)?,
        monthly_cash_recovery_cost_minor: cash_cost_minor,
        monthly_fully_burdened_economic_cost_minor: economic_cost_minor,
        monthly_external_funding_minor: external_funding_minor,
        maximum_monthly_user_contribution_minor: user_contribution_minor,
        cash_funding_gap_after_user_cap_minor: to_u64(cash_gap)?,
        fully_burdened_funding_gap_after_user_cap_minor: to_u64(economic_gap)?,
        implied_user_charge_per_household_for_cash_balance_minor: implied_user_charge,
        cash_cost_per_active_household_minor: per_unit(cash_cost_minor, input.active_households)?,
        fully_burdened_cost_per_active_household_minor: per_unit(economic_cost_minor, input.active_households)?,
        cash_cost_per_successful_task_minor: cash_cost_per_task,
        fully_burdened_cost_per_successful_task_minor: economic_cost_per_task,
        task_success_ratio_bps: success_ratio,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> EconomicsInput {
        EconomicsInput {
            currency: "ZAR".to_owned(),
            monthly_costs: vec![
                CostLine { id: "transit".to_owned(), currency: "ZAR".to_owned(), monthly_amount_minor: 100_000, category: CostCategory::UpstreamTransit },
                CostLine { id: "energy".to_owned(), currency: "ZAR".to_owned(), monthly_amount_minor: 10_000, category: CostCategory::Energy },
            ],
            capital_assets: vec![
                CapitalAsset { id: "gateway".to_owned(), currency: "ZAR".to_owned(), cost_minor: 120_000, residual_value_minor: 0, useful_life_months: 12, donated: false },
            ],
            monthly_funding: vec![
                FundingSource { id: "sponsor".to_owned(), currency: "ZAR".to_owned(), monthly_amount_minor: 50_000, kind: FundingKind::Sponsor },
            ],
            volunteer_hours_milli: 0,
            volunteer_hourly_rate_minor: 0,
            active_households: 10,
            useful_tasks_attempted: 1_000,
            useful_tasks_succeeded: 900,
            max_user_charge_minor_per_household: 0,
        }
    }

    #[test]
    fn computes_cash_and_fully_burdened_unit_costs() {
        let result = calculate(&input()).unwrap();
        assert_eq!(result.monthly_cash_operating_cost_minor, 110_000);
        assert_eq!(result.monthly_paid_capex_amortisation_minor, 10_000);
        assert_eq!(result.monthly_cash_recovery_cost_minor, 120_000);
        assert_eq!(result.monthly_fully_burdened_economic_cost_minor, 120_000);
        assert_eq!(result.cash_cost_per_successful_task_minor, Some(134));
        assert_eq!(result.fully_burdened_cost_per_active_household_minor, Some(12_000));
        assert_eq!(result.task_success_ratio_bps, Some(9_000));
        assert_eq!(result.cash_funding_gap_after_user_cap_minor, 70_000);
    }

    #[test]
    fn donated_asset_amortisation_is_separate_from_cash_cost() {
        let mut model = input();
        model.capital_assets.push(CapitalAsset {
            id: "donated-ap".to_owned(),
            currency: "ZAR".to_owned(),
            cost_minor: 24_000,
            residual_value_minor: 0,
            useful_life_months: 12,
            donated: true,
        });
        let result = calculate(&model).unwrap();
        assert_eq!(result.monthly_donated_capex_shadow_cost_minor, 2_000);
        assert_eq!(result.monthly_cash_recovery_cost_minor, 120_000);
        assert_eq!(result.monthly_fully_burdened_economic_cost_minor, 122_000);
    }

    #[test]
    fn volunteer_labor_is_shadow_cost_not_cash_cost() {
        let mut model = input();
        model.volunteer_hours_milli = 10_000;
        model.volunteer_hourly_rate_minor = 10_000;
        let result = calculate(&model).unwrap();
        assert_eq!(result.monthly_volunteer_labour_shadow_cost_minor, 100_000);
        assert_eq!(result.monthly_cash_recovery_cost_minor, 120_000);
        assert_eq!(result.monthly_fully_burdened_economic_cost_minor, 220_000);
    }

    #[test]
    fn volunteer_hours_require_nonzero_shadow_rate() {
        let mut model = input();
        model.volunteer_hours_milli = 1_000;
        model.volunteer_hourly_rate_minor = 0;
        assert_eq!(calculate(&model), Err(EconomicsError::VolunteerRateMissing));
    }

    #[test]
    fn fully_sponsored_service_has_zero_cash_gap() {
        let mut model = input();
        model.monthly_funding[0].monthly_amount_minor = 120_000;
        let result = calculate(&model).unwrap();
        assert_eq!(result.cash_funding_gap_after_user_cap_minor, 0);
        assert_eq!(result.implied_user_charge_per_household_for_cash_balance_minor, Some(0));
    }

    #[test]
    fn explicit_user_charge_cap_changes_funding_gap() {
        let mut model = input();
        model.max_user_charge_minor_per_household = 2_000;
        let result = calculate(&model).unwrap();
        assert_eq!(result.maximum_monthly_user_contribution_minor, 20_000);
        assert_eq!(result.cash_funding_gap_after_user_cap_minor, 50_000);
        assert_eq!(result.fully_burdened_funding_gap_after_user_cap_minor, 50_000);
    }

    #[test]
    fn fully_burdened_gap_includes_donated_assets_and_volunteer_time() {
        let mut model = input();
        model.capital_assets.push(CapitalAsset {
            id: "donated-ap".to_owned(),
            currency: "ZAR".to_owned(),
            cost_minor: 24_000,
            residual_value_minor: 0,
            useful_life_months: 12,
            donated: true,
        });
        model.volunteer_hours_milli = 1_000;
        model.volunteer_hourly_rate_minor = 5_000;
        let result = calculate(&model).unwrap();
        assert_eq!(result.monthly_cash_recovery_cost_minor, 120_000);
        assert_eq!(result.monthly_fully_burdened_economic_cost_minor, 127_000);
        assert_eq!(result.cash_funding_gap_after_user_cap_minor, 70_000);
        assert_eq!(result.fully_burdened_funding_gap_after_user_cap_minor, 77_000);
    }

    #[test]
    fn mixed_currencies_are_rejected() {
        let mut model = input();
        model.monthly_costs[0].currency = "USD".to_owned();
        assert_eq!(calculate(&model), Err(EconomicsError::CurrencyMismatch));
    }

    #[test]
    fn invalid_currency_code_is_rejected() {
        let mut model = input();
        model.currency = "Zar".to_owned();
        assert_eq!(calculate(&model), Err(EconomicsError::InvalidCurrencyCode));
    }

    #[test]
    fn duplicate_ids_across_ledger_are_rejected() {
        let mut model = input();
        model.monthly_funding[0].id = "transit".to_owned();
        assert_eq!(calculate(&model), Err(EconomicsError::DuplicateLineId));
    }

    #[test]
    fn invalid_capital_asset_is_rejected() {
        let mut model = input();
        model.capital_assets[0].residual_value_minor = model.capital_assets[0].cost_minor + 1;
        assert_eq!(calculate(&model), Err(EconomicsError::InvalidCapitalAsset));
        let mut model = input();
        model.capital_assets[0].useful_life_months = 0;
        assert_eq!(calculate(&model), Err(EconomicsError::InvalidCapitalAsset));
    }

    #[test]
    fn successes_cannot_exceed_attempts() {
        let mut model = input();
        model.useful_tasks_succeeded = model.useful_tasks_attempted + 1;
        assert_eq!(calculate(&model), Err(EconomicsError::SuccessesExceedAttempts));
    }

    #[test]
    fn zero_successes_produce_no_cost_per_successful_task() {
        let mut model = input();
        model.useful_tasks_succeeded = 0;
        let result = calculate(&model).unwrap();
        assert_eq!(result.cash_cost_per_successful_task_minor, None);
        assert_eq!(result.fully_burdened_cost_per_successful_task_minor, None);
        assert_eq!(result.task_success_ratio_bps, Some(0));
    }

    #[test]
    fn zero_attempts_produce_no_success_ratio() {
        let mut model = input();
        model.useful_tasks_attempted = 0;
        model.useful_tasks_succeeded = 0;
        let result = calculate(&model).unwrap();
        assert_eq!(result.task_success_ratio_bps, None);
        assert_eq!(result.cash_cost_per_successful_task_minor, None);
    }

    #[test]
    fn zero_households_never_divides_by_zero() {
        let mut model = input();
        model.active_households = 0;
        let result = calculate(&model).unwrap();
        assert_eq!(result.cash_cost_per_active_household_minor, None);
        assert_eq!(result.fully_burdened_cost_per_active_household_minor, None);
        assert_eq!(result.implied_user_charge_per_household_for_cash_balance_minor, None);
    }

    #[test]
    fn capex_rounding_uses_conservative_ceiling() {
        let mut model = input();
        model.capital_assets[0].cost_minor = 1_001;
        model.capital_assets[0].useful_life_months = 10;
        let result = calculate(&model).unwrap();
        assert_eq!(result.monthly_paid_capex_amortisation_minor, 101);
    }

    #[test]
    fn user_funding_cap_is_not_mistaken_for_external_sponsorship() {
        let mut model = input();
        model.max_user_charge_minor_per_household = 2_000;
        let result = calculate(&model).unwrap();
        assert_eq!(result.monthly_external_funding_minor, 50_000);
        assert_eq!(result.maximum_monthly_user_contribution_minor, 20_000);
    }

    #[test]
    fn excess_funding_does_not_create_negative_gap() {
        let mut model = input();
        model.monthly_funding[0].monthly_amount_minor = 500_000;
        let result = calculate(&model).unwrap();
        assert_eq!(result.cash_funding_gap_after_user_cap_minor, 0);
        assert_eq!(result.fully_burdened_funding_gap_after_user_cap_minor, 0);
    }

    #[test]
    fn invalid_or_duplicate_line_ids_are_rejected() {
        let mut model = input();
        model.monthly_costs[0].id = "../escape".to_owned();
        assert_eq!(calculate(&model), Err(EconomicsError::DuplicateLineId));
    }

    #[test]
    fn economic_model_is_deterministic() {
        let model = input();
        assert_eq!(calculate(&model).unwrap(), calculate(&model).unwrap());
    }
}
