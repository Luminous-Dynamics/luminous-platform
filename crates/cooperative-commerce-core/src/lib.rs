#![forbid(unsafe_code)]

//! Deterministic arithmetic and comparability checks for cooperative-buying savings.
//!
//! This crate intentionally has no third-party dependencies and accepts no binary
//! floating-point inputs. It computes a difference; it does not authenticate
//! source evidence, validate live ISO code lists, determine tax law, or authorize
//! purchases. Callers must not present a calculation as an independently verified
//! savings claim merely because this function returned a report.

use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::fmt;
use std::str::FromStr;

pub const MAX_DECIMAL_SCALE: u32 = 18;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecimalError {
    Empty,
    InvalidSyntax,
    LeadingZero,
    TooManyFractionalDigits,
    Overflow,
}

impl fmt::Display for DecimalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "decimal is empty"),
            Self::InvalidSyntax => write!(f, "decimal must use plain base-10 notation without exponent or whitespace"),
            Self::LeadingZero => write!(f, "integer part must not contain unnecessary leading zeros"),
            Self::TooManyFractionalDigits => write!(f, "decimal exceeds the supported scale of {MAX_DECIMAL_SCALE}"),
            Self::Overflow => write!(f, "decimal arithmetic overflow"),
        }
    }
}

impl std::error::Error for DecimalError {}

/// Exact base-10 decimal represented as a signed coefficient and scale.
/// Values are normalized, so numerically equal values compare equal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Decimal {
    coefficient: i128,
    scale: u32,
}

impl Decimal {
    pub const ZERO: Self = Self { coefficient: 0, scale: 0 };

    pub fn parse(value: &str) -> Result<Self, DecimalError> {
        value.parse()
    }

    pub fn is_negative(self) -> bool {
        self.coefficient < 0
    }

    pub fn is_zero(self) -> bool {
        self.coefficient == 0
    }

    pub fn is_positive(self) -> bool {
        self.coefficient > 0
    }

    pub fn checked_add(self, other: Self) -> Result<Self, DecimalError> {
        let (left, right, scale) = self.align(other)?;
        let coefficient = left.checked_add(right).ok_or(DecimalError::Overflow)?;
        Self::new(coefficient, scale)
    }

    pub fn checked_sub(self, other: Self) -> Result<Self, DecimalError> {
        let (left, right, scale) = self.align(other)?;
        let coefficient = left.checked_sub(right).ok_or(DecimalError::Overflow)?;
        Self::new(coefficient, scale)
    }

    pub fn checked_mul(self, other: Self) -> Result<Self, DecimalError> {
        let coefficient = self.coefficient.checked_mul(other.coefficient).ok_or(DecimalError::Overflow)?;
        let scale = self.scale.checked_add(other.scale).ok_or(DecimalError::Overflow)?;
        let result = Self::new_allow_scale(coefficient, scale)?;
        if result.scale > MAX_DECIMAL_SCALE {
            return Err(DecimalError::TooManyFractionalDigits);
        }
        Ok(result)
    }

    pub fn checked_cmp(self, other: Self) -> Result<Ordering, DecimalError> {
        let (left, right, _) = self.align(other)?;
        Ok(left.cmp(&right))
    }

    fn new(coefficient: i128, scale: u32) -> Result<Self, DecimalError> {
        let result = Self::new_allow_scale(coefficient, scale)?;
        if result.scale > MAX_DECIMAL_SCALE {
            return Err(DecimalError::TooManyFractionalDigits);
        }
        Ok(result)
    }

    fn new_allow_scale(coefficient: i128, mut scale: u32) -> Result<Self, DecimalError> {
        let mut coefficient = coefficient;
        while scale > 0 && coefficient % 10 == 0 {
            coefficient /= 10;
            scale -= 1;
        }
        Ok(Self { coefficient, scale })
    }

    fn align(self, other: Self) -> Result<(i128, i128, u32), DecimalError> {
        let target_scale = self.scale.max(other.scale);
        let left_factor = pow10(target_scale - self.scale)?;
        let right_factor = pow10(target_scale - other.scale)?;
        let left = self.coefficient.checked_mul(left_factor).ok_or(DecimalError::Overflow)?;
        let right = other.coefficient.checked_mul(right_factor).ok_or(DecimalError::Overflow)?;
        Ok((left, right, target_scale))
    }
}

fn pow10(scale: u32) -> Result<i128, DecimalError> {
    10_i128.checked_pow(scale).ok_or(DecimalError::Overflow)
}

impl FromStr for Decimal {
    type Err = DecimalError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        if input.is_empty() {
            return Err(DecimalError::Empty);
        }
        if input.trim() != input || input.contains(['e', 'E', '+']) {
            return Err(DecimalError::InvalidSyntax);
        }

        let (negative, unsigned) = match input.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, input),
        };
        if unsigned.is_empty() {
            return Err(DecimalError::InvalidSyntax);
        }

        let mut pieces = unsigned.split('.');
        let whole = pieces.next().ok_or(DecimalError::InvalidSyntax)?;
        let fraction = pieces.next();
        if pieces.next().is_some() || whole.is_empty() || !whole.bytes().all(|b| b.is_ascii_digit()) {
            return Err(DecimalError::InvalidSyntax);
        }
        if whole.len() > 1 && whole.starts_with('0') {
            return Err(DecimalError::LeadingZero);
        }

        let fraction = fraction.unwrap_or("");
        if !fraction.bytes().all(|b| b.is_ascii_digit()) || (input.contains('.') && fraction.is_empty()) {
            return Err(DecimalError::InvalidSyntax);
        }
        let scale = u32::try_from(fraction.len()).map_err(|_| DecimalError::TooManyFractionalDigits)?;
        if scale > MAX_DECIMAL_SCALE {
            return Err(DecimalError::TooManyFractionalDigits);
        }

        let digits = format!("{whole}{fraction}");
        let magnitude = digits.parse::<i128>().map_err(|_| DecimalError::Overflow)?;
        let coefficient = if negative {
            magnitude.checked_neg().ok_or(DecimalError::Overflow)?
        } else {
            magnitude
        };
        Self::new(coefficient, scale)
    }
}

impl fmt::Display for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.coefficient < 0 { "-" } else { "" };
        let digits = self.coefficient.unsigned_abs().to_string();
        if self.scale == 0 {
            return write!(f, "{sign}{digits}");
        }
        let scale = self.scale as usize;
        if digits.len() <= scale {
            let zeros = "0".repeat(scale - digits.len());
            write!(f, "{sign}0.{zeros}{digits}")
        } else {
            let split = digits.len() - scale;
            write!(f, "{sign}{}.{}", &digits[..split], &digits[split..])
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceKind {
    AlternativeInvoice,
    AlternativeQuote,
    PublicListPriceEstimate,
    SupplierInvoice,
    DeliveryReceipt,
    CostInvoice,
    ParticipationFeeInvoice,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceRef {
    pub id: String,
    pub source_reference: String,
    /// SHA-256 of retained source bytes, supplied by the caller.
    pub sha256: String,
    pub captured_at: String,
    pub kind: EvidenceKind,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ProductIdentity {
    pub identifier_scheme: String,
    pub identifier: String,
    pub specification_sha256: String,
    pub unit_code: String,
    pub unit_code_system: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BasketLine {
    pub product: ProductIdentity,
    pub quantity: Decimal,
    /// Total for this exact line and quantity, not a unit price.
    pub line_total: Decimal,
    pub currency: String,
    pub evidence: EvidenceRef,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CostLine {
    pub category: String,
    pub amount: Decimal,
    pub currency: String,
    pub evidence: EvidenceRef,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CostCoverage {
    /// Operator-declared statement that all material landed-cost categories were considered.
    /// This is not independently proven by this crate.
    pub declared_complete: bool,
    pub unresolved_costs: Vec<String>,
    pub evidence: EvidenceRef,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SavingsInput {
    pub currency: String,
    pub baseline_lines: Vec<BasketLine>,
    pub actual_lines: Vec<BasketLine>,
    pub baseline_costs: Vec<CostLine>,
    pub actual_costs: Vec<CostLine>,
    pub participation_costs: Vec<CostLine>,
    pub baseline_coverage: CostCoverage,
    pub actual_coverage: CostCoverage,
    pub delivery_evidence: Vec<EvidenceRef>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClaimClass {
    /// One or more baseline inputs are indicative list-price estimates.
    Estimate,
    /// Actual supplier invoice compared with an alternative quote and delivery evidence.
    InvoiceVsAlternativeQuote,
    /// Actual supplier invoice compared with an earlier alternative invoice and delivery evidence.
    HistoricalInvoiceComparison,
    /// Arithmetic is available but delivery evidence is missing.
    ProvisionalWithoutDeliveryEvidence,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SavingsReport {
    pub currency: String,
    pub baseline_merchandise: String,
    pub baseline_other_costs: String,
    pub actual_merchandise: String,
    pub actual_other_costs: String,
    pub participation_costs: String,
    pub baseline_total: String,
    pub actual_total: String,
    /// May be negative. Negative results must never be clamped to zero.
    pub net_difference: String,
    pub claim_class: ClaimClass,
    pub evidence_references: Vec<String>,
    /// Always false: this crate checks references and arithmetic, not source authenticity.
    pub evidence_authenticated_by_calculator: bool,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommerceError {
    InvalidCurrency,
    EmptyBasket,
    InvalidProductIdentity,
    InvalidEvidence,
    UnexpectedEvidenceKind,
    NegativeQuantityOrCost,
    CurrencyMismatch,
    DuplicateProductLine,
    BasketMismatch,
    IncompleteCostCoverage,
    Decimal(DecimalError),
}

impl fmt::Display for CommerceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCurrency => write!(f, "currency must have a three-letter uppercase shape; official code-list membership is not checked"),
            Self::EmptyBasket => write!(f, "baseline and actual baskets must both contain at least one line"),
            Self::InvalidProductIdentity => write!(f, "product identity requires scheme, identifier, a 64-hex specification digest, unit code, and unit code-system"),
            Self::InvalidEvidence => write!(f, "evidence reference is incomplete or its SHA-256 field is malformed"),
            Self::UnexpectedEvidenceKind => write!(f, "evidence kind is not valid for this comparison role"),
            Self::NegativeQuantityOrCost => write!(f, "quantities and line/cost amounts must be non-negative; quantities must be positive"),
            Self::CurrencyMismatch => write!(f, "currencies differ; no implicit foreign-exchange conversion is performed"),
            Self::DuplicateProductLine => write!(f, "duplicate product identity in a basket; aggregate duplicate lines before comparison"),
            Self::BasketMismatch => write!(f, "baseline and actual baskets do not contain equivalent product specifications, units, and quantities"),
            Self::IncompleteCostCoverage => write!(f, "unresolved or undeclared landed-cost coverage prevents a savings calculation"),
            Self::Decimal(err) => write!(f, "decimal arithmetic failed: {err}"),
        }
    }
}

impl std::error::Error for CommerceError {}

impl From<DecimalError> for CommerceError {
    fn from(value: DecimalError) -> Self {
        Self::Decimal(value)
    }
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn validate_evidence(evidence: &EvidenceRef) -> Result<(), CommerceError> {
    if evidence.id.trim().is_empty()
        || evidence.source_reference.trim().is_empty()
        || evidence.captured_at.trim().is_empty()
        || !valid_sha256(&evidence.sha256)
    {
        return Err(CommerceError::InvalidEvidence);
    }
    Ok(())
}

fn valid_currency_shape(value: &str) -> bool {
    value.len() == 3 && value.bytes().all(|b| b.is_ascii_uppercase())
}

fn validate_product(product: &ProductIdentity) -> Result<(), CommerceError> {
    if product.identifier_scheme.trim().is_empty()
        || product.identifier.trim().is_empty()
        || !valid_sha256(&product.specification_sha256)
        || product.unit_code.trim().is_empty()
        || product.unit_code_system.trim().is_empty()
    {
        return Err(CommerceError::InvalidProductIdentity);
    }
    Ok(())
}

fn sum_amounts<'a>(values: impl IntoIterator<Item = &'a Decimal>) -> Result<Decimal, CommerceError> {
    values.into_iter().try_fold(Decimal::ZERO, |acc, value| acc.checked_add(*value).map_err(CommerceError::from))
}

fn validate_basket_lines(
    lines: &[BasketLine],
    currency: &str,
    is_baseline: bool,
) -> Result<(), CommerceError> {
    if lines.is_empty() {
        return Err(CommerceError::EmptyBasket);
    }
    let mut identities = BTreeSet::new();
    for line in lines {
        validate_product(&line.product)?;
        if !identities.insert(line.product.clone()) {
            return Err(CommerceError::DuplicateProductLine);
        }
        if line.quantity.checked_cmp(Decimal::ZERO)? != Ordering::Greater
            || line.line_total.checked_cmp(Decimal::ZERO)? == Ordering::Less
        {
            return Err(CommerceError::NegativeQuantityOrCost);
        }
        if line.currency != currency {
            return Err(CommerceError::CurrencyMismatch);
        }
        validate_evidence(&line.evidence)?;
        let valid_kind = if is_baseline {
            matches!(line.evidence.kind, EvidenceKind::AlternativeInvoice | EvidenceKind::AlternativeQuote | EvidenceKind::PublicListPriceEstimate)
        } else {
            line.evidence.kind == EvidenceKind::SupplierInvoice
        };
        if !valid_kind {
            return Err(CommerceError::UnexpectedEvidenceKind);
        }
    }
    Ok(())
}

fn validate_costs(
    costs: &[CostLine],
    currency: &str,
    role: EvidenceKind,
    baseline: bool,
) -> Result<(), CommerceError> {
    for cost in costs {
        if cost.category.trim().is_empty() || cost.amount.checked_cmp(Decimal::ZERO)? == Ordering::Less {
            return Err(CommerceError::NegativeQuantityOrCost);
        }
        if cost.currency != currency {
            return Err(CommerceError::CurrencyMismatch);
        }
        validate_evidence(&cost.evidence)?;
        let valid_kind = if baseline {
            matches!(cost.evidence.kind, EvidenceKind::AlternativeInvoice | EvidenceKind::AlternativeQuote | EvidenceKind::PublicListPriceEstimate)
        } else {
            cost.evidence.kind == role
        };
        if !valid_kind {
            return Err(CommerceError::UnexpectedEvidenceKind);
        }
    }
    Ok(())
}

fn validate_coverage(coverage: &CostCoverage, baseline: bool) -> Result<(), CommerceError> {
    validate_evidence(&coverage.evidence)?;
    let valid_kind = if baseline {
        matches!(coverage.evidence.kind, EvidenceKind::AlternativeInvoice | EvidenceKind::AlternativeQuote | EvidenceKind::PublicListPriceEstimate)
    } else {
        matches!(coverage.evidence.kind, EvidenceKind::SupplierInvoice | EvidenceKind::CostInvoice)
    };
    if !valid_kind {
        return Err(CommerceError::UnexpectedEvidenceKind);
    }
    if !coverage.declared_complete || !coverage.unresolved_costs.is_empty() {
        return Err(CommerceError::IncompleteCostCoverage);
    }
    Ok(())
}

fn validate_basket_equivalence(baseline: &[BasketLine], actual: &[BasketLine]) -> Result<(), CommerceError> {
    if baseline.len() != actual.len() {
        return Err(CommerceError::BasketMismatch);
    }
    let baseline_map: std::collections::BTreeMap<&ProductIdentity, &BasketLine> =
        baseline.iter().map(|line| (&line.product, line)).collect();
    let actual_map: std::collections::BTreeMap<&ProductIdentity, &BasketLine> =
        actual.iter().map(|line| (&line.product, line)).collect();
    if baseline_map.len() != baseline.len() || actual_map.len() != actual.len() {
        return Err(CommerceError::DuplicateProductLine);
    }
    for (identity, baseline_line) in baseline_map {
        let actual_line = actual_map.get(identity).ok_or(CommerceError::BasketMismatch)?;
        if baseline_line.quantity != actual_line.quantity {
            return Err(CommerceError::BasketMismatch);
        }
    }
    Ok(())
}

/// Calculate net cost difference only for directly comparable baskets.
///
/// Formula:
/// baseline merchandise + baseline non-merchandise costs
/// minus actual merchandise, actual non-merchandise costs, and participation costs.
///
/// The function refuses unknown material costs, unlike currencies, different units,
/// different specifications, duplicate product lines, and missing cost-coverage
/// declarations. It never clamps a negative result and never authenticates evidence.
pub fn calculate_savings(input: &SavingsInput) -> Result<SavingsReport, CommerceError> {
    if !valid_currency_shape(&input.currency) {
        return Err(CommerceError::InvalidCurrency);
    }
    validate_basket_lines(&input.baseline_lines, &input.currency, true)?;
    validate_basket_lines(&input.actual_lines, &input.currency, false)?;
    validate_basket_equivalence(&input.baseline_lines, &input.actual_lines)?;
    validate_costs(&input.baseline_costs, &input.currency, EvidenceKind::AlternativeQuote, true)?;
    validate_costs(&input.actual_costs, &input.currency, EvidenceKind::CostInvoice, false)?;
    validate_costs(&input.participation_costs, &input.currency, EvidenceKind::ParticipationFeeInvoice, false)?;
    validate_coverage(&input.baseline_coverage, true)?;
    validate_coverage(&input.actual_coverage, false)?;

    for evidence in &input.delivery_evidence {
        validate_evidence(evidence)?;
        if evidence.kind != EvidenceKind::DeliveryReceipt {
            return Err(CommerceError::UnexpectedEvidenceKind);
        }
    }

    let baseline_merchandise = sum_amounts(input.baseline_lines.iter().map(|line| &line.line_total))?;
    let baseline_other = sum_amounts(input.baseline_costs.iter().map(|line| &line.amount))?;
    let actual_merchandise = sum_amounts(input.actual_lines.iter().map(|line| &line.line_total))?;
    let actual_other = sum_amounts(input.actual_costs.iter().map(|line| &line.amount))?;
    let participation = sum_amounts(input.participation_costs.iter().map(|line| &line.amount))?;
    let baseline_total = baseline_merchandise.checked_add(baseline_other)?;
    let actual_total = actual_merchandise.checked_add(actual_other)?.checked_add(participation)?;
    let net = baseline_total.checked_sub(actual_total)?;

    let baseline_kinds: BTreeSet<EvidenceKind> = input.baseline_lines.iter().map(|line| line.evidence.kind).collect();
    let baseline_has_estimate = baseline_kinds.contains(&EvidenceKind::PublicListPriceEstimate)
        || input.baseline_coverage.evidence.kind == EvidenceKind::PublicListPriceEstimate
        || input.baseline_costs.iter().any(|line| line.evidence.kind == EvidenceKind::PublicListPriceEstimate);
    let claim_class = if baseline_has_estimate {
        ClaimClass::Estimate
    } else if input.delivery_evidence.is_empty() {
        ClaimClass::ProvisionalWithoutDeliveryEvidence
    } else if baseline_kinds.len() == 1 && baseline_kinds.contains(&EvidenceKind::AlternativeInvoice)
        && input.baseline_coverage.evidence.kind == EvidenceKind::AlternativeInvoice
    {
        ClaimClass::HistoricalInvoiceComparison
    } else {
        ClaimClass::InvoiceVsAlternativeQuote
    };

    let mut evidence_references = BTreeSet::new();
    for evidence in input.baseline_lines.iter().map(|line| &line.evidence)
        .chain(input.actual_lines.iter().map(|line| &line.evidence))
        .chain(input.baseline_costs.iter().map(|line| &line.evidence))
        .chain(input.actual_costs.iter().map(|line| &line.evidence))
        .chain(input.participation_costs.iter().map(|line| &line.evidence))
        .chain(std::iter::once(&input.baseline_coverage.evidence))
        .chain(std::iter::once(&input.actual_coverage.evidence))
        .chain(input.delivery_evidence.iter())
    {
        evidence_references.insert(evidence.id.clone());
    }

    let mut notes = vec![
        "Arithmetic only: source references, digest format, and declared coverage were checked; evidence contents, signatures, supplier identity, and official code-list membership were not independently verified.".to_string(),
        "No currency conversion was performed. A three-letter currency shape is not proof of a currently assigned ISO 4217 code or settlement capability.".to_string(),
    ];
    if net.is_negative() {
        notes.push("Net difference is negative: the comparable purchase cost more after included costs and participation fees. Preserve and report this result.".to_string());
    }
    if claim_class == ClaimClass::Estimate {
        notes.push("At least one baseline input is an indicative list-price estimate; do not describe the result as realized savings.".to_string());
    }
    if claim_class == ClaimClass::ProvisionalWithoutDeliveryEvidence {
        notes.push("Delivery evidence is absent; this is not a completed-delivery savings result.".to_string());
    }

    Ok(SavingsReport {
        currency: input.currency.clone(),
        baseline_merchandise: baseline_merchandise.to_string(),
        baseline_other_costs: baseline_other.to_string(),
        actual_merchandise: actual_merchandise.to_string(),
        actual_other_costs: actual_other.to_string(),
        participation_costs: participation.to_string(),
        baseline_total: baseline_total.to_string(),
        actual_total: actual_total.to_string(),
        net_difference: net.to_string(),
        claim_class,
        evidence_references: evidence_references.into_iter().collect(),
        evidence_authenticated_by_calculator: false,
        notes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence(id: &str, kind: EvidenceKind) -> EvidenceRef {
        EvidenceRef {
            id: id.to_string(),
            source_reference: format!("fixture:{id}"),
            sha256: "a".repeat(64),
            captured_at: "2026-10-09T00:00:00Z".to_string(),
            kind,
        }
    }

    fn product() -> ProductIdentity {
        ProductIdentity {
            identifier_scheme: "urn:fixture:product:v1".into(),
            identifier: "carton-001".into(),
            specification_sha256: "b".repeat(64),
            unit_code: "case".into(),
            unit_code_system: "urn:fixture:packaging-unit:v1".into(),
        }
    }

    fn line(amount: &str, kind: EvidenceKind) -> BasketLine {
        BasketLine {
            product: product(),
            quantity: Decimal::parse("10").unwrap(),
            line_total: Decimal::parse(amount).unwrap(),
            currency: "ZAR".into(),
            evidence: evidence("line", kind),
        }
    }

    fn cost(category: &str, amount: &str, kind: EvidenceKind) -> CostLine {
        CostLine {
            category: category.into(),
            amount: Decimal::parse(amount).unwrap(),
            currency: "ZAR".into(),
            evidence: evidence(category, kind),
        }
    }

    fn input() -> SavingsInput {
        SavingsInput {
            currency: "ZAR".into(),
            baseline_lines: vec![line("1200.00", EvidenceKind::AlternativeQuote)],
            actual_lines: vec![line("1050.00", EvidenceKind::SupplierInvoice)],
            baseline_costs: vec![cost("freight", "50.00", EvidenceKind::AlternativeQuote)],
            actual_costs: vec![cost("freight", "70.00", EvidenceKind::CostInvoice)],
            participation_costs: vec![cost("network-fee", "10.00", EvidenceKind::ParticipationFeeInvoice)],
            baseline_coverage: CostCoverage {
                declared_complete: true,
                unresolved_costs: vec![],
                evidence: evidence("baseline-coverage", EvidenceKind::AlternativeQuote),
            },
            actual_coverage: CostCoverage {
                declared_complete: true,
                unresolved_costs: vec![],
                evidence: evidence("actual-coverage", EvidenceKind::SupplierInvoice),
            },
            delivery_evidence: vec![evidence("delivery-receipt", EvidenceKind::DeliveryReceipt)],
        }
    }

    #[test]
    fn decimal_is_exact_and_normalized() {
        assert_eq!(Decimal::parse("12.5000").unwrap().to_string(), "12.5");
        assert_eq!(Decimal::parse("0.010").unwrap().to_string(), "0.01");
        assert_eq!(Decimal::parse("-1.25").unwrap().checked_add(Decimal::parse("0.25").unwrap()).unwrap().to_string(), "-1");
    }

    #[test]
    fn decimal_rejects_exponents_whitespace_and_float_like_ambiguity() {
        assert_eq!(Decimal::parse("1e3"), Err(DecimalError::InvalidSyntax));
        assert_eq!(Decimal::parse(" 1.00"), Err(DecimalError::InvalidSyntax));
        assert_eq!(Decimal::parse("01.00"), Err(DecimalError::LeadingZero));
        assert_eq!(Decimal::parse("1."), Err(DecimalError::InvalidSyntax));
    }

    #[test]
    fn decimal_overflow_fails_closed() {
        let huge = Decimal::parse("99999999999999999999999999999999999999").unwrap();
        assert_eq!(huge.checked_mul(Decimal::parse("10").unwrap()), Err(DecimalError::Overflow));
    }

    #[test]
    fn savings_includes_freight_and_participation_costs() {
        let report = calculate_savings(&input()).unwrap();
        assert_eq!(report.baseline_total, "1250");
        assert_eq!(report.actual_total, "1130");
        assert_eq!(report.net_difference, "120");
        assert_eq!(report.claim_class, ClaimClass::InvoiceVsAlternativeQuote);
        assert!(!report.evidence_authenticated_by_calculator);
    }

    #[test]
    fn negative_savings_are_preserved_not_clamped() {
        let mut candidate = input();
        candidate.actual_lines[0].line_total = Decimal::parse("1250").unwrap();
        let report = calculate_savings(&candidate).unwrap();
        assert_eq!(report.net_difference, "-30");
        assert!(report.notes.iter().any(|note| note.contains("Net difference is negative")));
    }

    #[test]
    fn unresolved_costs_block_any_savings_result() {
        let mut candidate = input();
        candidate.actual_coverage.unresolved_costs.push("customs clearance fee".into());
        assert_eq!(calculate_savings(&candidate), Err(CommerceError::IncompleteCostCoverage));
    }

    #[test]
    fn unmatched_specs_or_units_block_comparison() {
        let mut candidate = input();
        candidate.actual_lines[0].product.unit_code_system = "urn:other:unit-system:v1".into();
        assert_eq!(calculate_savings(&candidate), Err(CommerceError::BasketMismatch));
    }

    #[test]
    fn mismatched_quantity_blocks_comparison() {
        let mut candidate = input();
        candidate.actual_lines[0].quantity = Decimal::parse("9").unwrap();
        assert_eq!(calculate_savings(&candidate), Err(CommerceError::BasketMismatch));
    }

    #[test]
    fn currency_mismatch_never_uses_implicit_fx() {
        let mut candidate = input();
        candidate.actual_lines[0].currency = "EUR".into();
        assert_eq!(calculate_savings(&candidate), Err(CommerceError::CurrencyMismatch));
    }

    #[test]
    fn malformed_digest_blocks_calculation() {
        let mut candidate = input();
        candidate.actual_lines[0].evidence.sha256 = "not-a-digest".into();
        assert_eq!(calculate_savings(&candidate), Err(CommerceError::InvalidEvidence));
    }

    #[test]
    fn baseline_list_price_is_marked_as_estimate() {
        let mut candidate = input();
        candidate.baseline_lines[0].evidence.kind = EvidenceKind::PublicListPriceEstimate;
        candidate.baseline_coverage.evidence.kind = EvidenceKind::PublicListPriceEstimate;
        candidate.baseline_costs[0].evidence.kind = EvidenceKind::PublicListPriceEstimate;
        let report = calculate_savings(&candidate).unwrap();
        assert_eq!(report.claim_class, ClaimClass::Estimate);
    }

    #[test]
    fn missing_delivery_evidence_is_not_called_realized_savings() {
        let mut candidate = input();
        candidate.delivery_evidence.clear();
        let report = calculate_savings(&candidate).unwrap();
        assert_eq!(report.claim_class, ClaimClass::ProvisionalWithoutDeliveryEvidence);
    }

    #[test]
    fn code_list_membership_is_not_inferred_from_currency_shape() {
        let mut candidate = input();
        candidate.currency = "ZZZ".into();
        for line in candidate.baseline_lines.iter_mut()
            .chain(candidate.actual_lines.iter_mut())
        {
            line.currency = "ZZZ".into();
        }
        for line in candidate.baseline_costs.iter_mut()
            .chain(candidate.actual_costs.iter_mut())
            .chain(candidate.participation_costs.iter_mut())
        {
            line.currency = "ZZZ".into();
        }
        let report = calculate_savings(&candidate).unwrap();
        assert!(report.notes.iter().any(|note| note.contains("not proof of a currently assigned ISO 4217 code")));
    }
}
