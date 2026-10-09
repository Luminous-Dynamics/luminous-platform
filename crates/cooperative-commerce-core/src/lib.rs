#![forbid(unsafe_code)]

//! Deterministic arithmetic and comparability checks for cooperative-buying savings.
//!
//! This crate intentionally has no third-party dependencies and accepts no binary
//! floating-point inputs. It computes a difference; it does not authenticate
//! source evidence, validate live ISO code lists, determine tax law, or authorize
//! purchases. Callers must not present a calculation as an independently verified
//! savings claim merely because this function returned a report.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
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
        if input.trim() != input || input.contains('e') || input.contains('E') || input.contains('+') {
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

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum EvidenceKind {
    AlternativeInvoice,
    AlternativeQuote,
    PublicListPriceEstimate,
    SupplierInvoice,
    DeliveryReceipt,
    CostInvoice,
    ParticipationFeeInvoice,
    CreditNote,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CostDirection {
    Charge,
    Credit,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CostLine {
    pub category: String,
    pub direction: CostDirection,
    /// Non-negative magnitude. Direction determines whether it adds to or reduces total cost.
    pub amount: Decimal,
    pub currency: String,
    pub evidence: EvidenceRef,
}

/// A receipt line that asserts the exact product/unit/quantity was delivered.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeliveredLine {
    pub product: ProductIdentity,
    pub quantity: Decimal,
    pub evidence: EvidenceRef,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CostCoverage {
    /// Operator-declared statement that all material landed-cost categories were considered.
    /// This is not independently proven by this crate.
    pub declared_complete: bool,
    /// Every material non-merchandise cost category considered on this side.
    /// Baseline and actual category sets must match for comparability.
    pub considered_categories: Vec<String>,
    /// Categories with no cost line, explicitly checked and confirmed as zero.
    /// Coverage evidence is the source record supporting these assertions.
    pub zero_confirmed_categories: Vec<String>,
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
    pub delivery_lines: Vec<DeliveredLine>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClaimClass {
    /// One or more baseline inputs are indicative list-price estimates.
    Estimate,
    /// Actual supplier invoice compared with an alternative quote and delivery evidence.
    InvoiceVsAlternativeQuote,
    /// Actual supplier invoice compared with an earlier alternative invoice and delivery evidence.
    HistoricalInvoiceComparison,
    /// The baseline mixes alternative quotes and historical invoices.
    MixedBaselineEvidence,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SavingsReceiptStatus {
    Illustrative,
    Computed,
}

/// Typed receipt model. JSON serialization is deliberately a separate, future adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SavingsReceipt {
    pub receipt_id: String,
    pub revision: u64,
    pub status: SavingsReceiptStatus,
    pub fixture_only: bool,
    pub algorithm_name: String,
    pub algorithm_version: String,
    /// Git source revision of the calculator used to produce a computed receipt.
    pub source_revision: Option<String>,
    /// SHA-256 of the exact calculator implementation/build artifact, not a signature.
    pub implementation_digest_sha256: Option<String>,
    pub input_snapshot: SavingsInput,
    pub claimed_calculation: SavingsReport,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReceiptVerificationError {
    InvalidMetadata,
    CalculationMismatch,
    Calculation(CommerceError),
}

impl fmt::Display for ReceiptVerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMetadata => write!(f, "receipt metadata/status/source identity is invalid"),
            Self::CalculationMismatch => write!(f, "claimed receipt calculation does not match a recomputation from the included snapshot"),
            Self::Calculation(err) => write!(f, "receipt input snapshot cannot be calculated: {err}"),
        }
    }
}

impl std::error::Error for ReceiptVerificationError {}

/// Recompute all reported totals/classification/limitations from the included inputs.
/// This verifies internal arithmetic consistency only; it does not authenticate the
/// source bytes, issuer, signatures, supplier identity, or causation.
pub fn verify_savings_receipt(receipt: &SavingsReceipt) -> Result<SavingsReport, ReceiptVerificationError> {
    if receipt.receipt_id.trim().is_empty()
        || receipt.revision == 0
        || receipt.algorithm_name.trim().is_empty()
        || receipt.algorithm_version.trim().is_empty()
    {
        return Err(ReceiptVerificationError::InvalidMetadata);
    }
    match receipt.status {
        SavingsReceiptStatus::Illustrative => {
            if !receipt.fixture_only {
                return Err(ReceiptVerificationError::InvalidMetadata);
            }
        }
        SavingsReceiptStatus::Computed => {
            if receipt.fixture_only {
                return Err(ReceiptVerificationError::InvalidMetadata);
            }
            let valid_revision = receipt.source_revision.as_deref().is_some_and(|value| {
                (value.len() == 40 || value.len() == 64) && value.bytes().all(|b| b.is_ascii_hexdigit())
            });
            let valid_implementation_digest = receipt.implementation_digest_sha256.as_deref()
                .is_some_and(valid_sha256);
            if !valid_revision || !valid_implementation_digest {
                return Err(ReceiptVerificationError::InvalidMetadata);
            }
        }
    }

    let recomputed = calculate_savings(&receipt.input_snapshot)
        .map_err(ReceiptVerificationError::Calculation)?;
    if recomputed != receipt.claimed_calculation {
        return Err(ReceiptVerificationError::CalculationMismatch);
    }
    Ok(recomputed)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommerceError {
    InvalidCurrency,
    EmptyBasket,
    InvalidProductIdentity,
    InvalidEvidence,
    ConflictingEvidenceReference,
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
            Self::InvalidEvidence => write!(f, "evidence reference is incomplete, its timestamp is malformed, or its SHA-256 field is malformed"),
            Self::ConflictingEvidenceReference => write!(f, "one evidence identifier refers to conflicting source, digest, timestamp, or evidence-kind values"),
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

fn has_explicit_timestamp_zone(value: &str) -> bool {
    // Validate the full RFC 3339 timestamp shape and calendar/time fields without
    // silently treating an arbitrary string ending in Z as a timestamp.
    let bytes = value.as_bytes();
    if !bytes.iter().all(u8::is_ascii) || bytes.len() < 20 {
        return false;
    }
    if bytes[4] != b'-' || bytes[7] != b'-'
        || !(bytes[10] == b'T' || bytes[10] == b't')
        || bytes[13] != b':' || bytes[16] != b':'
        || !bytes[0..4].iter().all(u8::is_ascii_digit)
        || !bytes[5..7].iter().all(u8::is_ascii_digit)
        || !bytes[8..10].iter().all(u8::is_ascii_digit)
        || !bytes[11..13].iter().all(u8::is_ascii_digit)
        || !bytes[14..16].iter().all(u8::is_ascii_digit)
        || !bytes[17..19].iter().all(u8::is_ascii_digit)
    {
        return false;
    }

    let year = digits(&bytes[0..4]);
    let month = digits(&bytes[5..7]);
    let day = digits(&bytes[8..10]);
    let hour = digits(&bytes[11..13]);
    let minute = digits(&bytes[14..16]);
    let second = digits(&bytes[17..19]);
    if year == 0 || month == 0 || month > 12 || hour > 23 || minute > 59 || second > 60 {
        return false;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    if day == 0 || day > days_in_month {
        return false;
    }

    let mut zone_index = 19;
    if bytes.get(zone_index) == Some(&b'.') {
        zone_index += 1;
        let fraction_start = zone_index;
        while bytes.get(zone_index).is_some_and(u8::is_ascii_digit) {
            zone_index += 1;
        }
        if zone_index == fraction_start {
            return false;
        }
    }

    let zone = &bytes[zone_index..];
    if zone.len() == 1 {
        return zone[0] == b'Z' || zone[0] == b'z';
    }
    if zone.len() != 6
        || (zone[0] != b'+' && zone[0] != b'-')
        || zone[3] != b':'
        || !zone[1..3].iter().all(u8::is_ascii_digit)
        || !zone[4..6].iter().all(u8::is_ascii_digit)
    {
        return false;
    }
    digits(&zone[1..3]) <= 23 && digits(&zone[4..6]) <= 59
}

fn digits(bytes: &[u8]) -> u32 {
    bytes.iter().fold(0, |value, byte| value * 10 + u32::from(*byte - b'0'))
}

fn validate_evidence(evidence: &EvidenceRef) -> Result<(), CommerceError> {
    if evidence.id.trim().is_empty()
        || evidence.source_reference.trim().is_empty()
        || !has_explicit_timestamp_zone(evidence.captured_at.trim())
        || !valid_sha256(&evidence.sha256)
    {
        return Err(CommerceError::InvalidEvidence);
    }
    Ok(())
}

fn collect_evidence_references(input: &SavingsInput) -> Result<Vec<String>, CommerceError> {
    let mut by_id: BTreeMap<String, EvidenceRef> = BTreeMap::new();
    let evidence = input.baseline_lines.iter().map(|line| &line.evidence)
        .chain(input.actual_lines.iter().map(|line| &line.evidence))
        .chain(input.baseline_costs.iter().map(|line| &line.evidence))
        .chain(input.actual_costs.iter().map(|line| &line.evidence))
        .chain(input.participation_costs.iter().map(|line| &line.evidence))
        .chain(std::iter::once(&input.baseline_coverage.evidence))
        .chain(std::iter::once(&input.actual_coverage.evidence))
        .chain(input.delivery_lines.iter().map(|line| &line.evidence));

    for reference in evidence {
        if let Some(existing) = by_id.get(&reference.id) {
            if existing != reference {
                return Err(CommerceError::ConflictingEvidenceReference);
            }
        } else {
            by_id.insert(reference.id.clone(), reference.clone());
        }
    }
    Ok(by_id.into_keys().collect())
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

fn signed_cost(cost: &CostLine) -> Result<Decimal, CommerceError> {
    match cost.direction {
        CostDirection::Charge => Ok(cost.amount),
        CostDirection::Credit => Decimal::ZERO.checked_sub(cost.amount).map_err(CommerceError::from),
    }
}

fn sum_costs(costs: &[CostLine]) -> Result<Decimal, CommerceError> {
    costs.iter().try_fold(Decimal::ZERO, |acc, cost| {
        acc.checked_add(signed_cost(cost)?).map_err(CommerceError::from)
    })
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
        let valid_kind = match (baseline, role, cost.direction) {
            (true, _, CostDirection::Charge) => matches!(
                cost.evidence.kind,
                EvidenceKind::AlternativeInvoice | EvidenceKind::AlternativeQuote | EvidenceKind::PublicListPriceEstimate
            ),
            (true, _, CostDirection::Credit) => cost.evidence.kind == EvidenceKind::CreditNote,
            (false, EvidenceKind::CostInvoice, CostDirection::Charge) => cost.evidence.kind == EvidenceKind::CostInvoice,
            (false, EvidenceKind::CostInvoice, CostDirection::Credit) => cost.evidence.kind == EvidenceKind::CreditNote,
            (false, EvidenceKind::ParticipationFeeInvoice, CostDirection::Charge) => cost.evidence.kind == EvidenceKind::ParticipationFeeInvoice,
            (false, EvidenceKind::ParticipationFeeInvoice, CostDirection::Credit) => cost.evidence.kind == EvidenceKind::CreditNote,
            (false, _, _) => false,
        };
        if !valid_kind {
            return Err(CommerceError::UnexpectedEvidenceKind);
        }
    }
    Ok(())
}

fn validate_coverage(coverage: &CostCoverage, baseline: bool) -> Result<(), CommerceError> {
    validate_evidence(&coverage.evidence)?;
    let mut seen_categories = BTreeSet::new();
    for category in &coverage.considered_categories {
        if category.trim().is_empty() || !seen_categories.insert(category) {
            return Err(CommerceError::IncompleteCostCoverage);
        }
    }
    let mut seen_zero_categories = BTreeSet::new();
    for category in &coverage.zero_confirmed_categories {
        if category.trim().is_empty()
            || !seen_zero_categories.insert(category)
            || !seen_categories.contains(category)
        {
            return Err(CommerceError::IncompleteCostCoverage);
        }
    }
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

fn validate_delivery_lines(
    actual: &[BasketLine],
    delivered: &[DeliveredLine],
) -> Result<(), CommerceError> {
    if delivered.is_empty() {
        return Ok(());
    }
    let mut delivered_identities = BTreeSet::new();
    for line in delivered {
        validate_product(&line.product)?;
        if !delivered_identities.insert(line.product.clone()) {
            return Err(CommerceError::DuplicateProductLine);
        }
        if line.quantity.checked_cmp(Decimal::ZERO)? != Ordering::Greater {
            return Err(CommerceError::NegativeQuantityOrCost);
        }
        validate_evidence(&line.evidence)?;
        if line.evidence.kind != EvidenceKind::DeliveryReceipt {
            return Err(CommerceError::UnexpectedEvidenceKind);
        }
    }
    if delivered.len() != actual.len() {
        return Err(CommerceError::BasketMismatch);
    }
    for actual_line in actual {
        let delivered_line = delivered
            .iter()
            .find(|candidate| candidate.product == actual_line.product)
            .ok_or(CommerceError::BasketMismatch)?;
        if delivered_line.quantity != actual_line.quantity {
            return Err(CommerceError::BasketMismatch);
        }
    }
    Ok(())
}

fn validate_cost_category_coverage(input: &SavingsInput) -> Result<(), CommerceError> {
    let baseline: BTreeSet<&str> = input.baseline_coverage.considered_categories.iter().map(String::as_str).collect();
    let actual: BTreeSet<&str> = input.actual_coverage.considered_categories.iter().map(String::as_str).collect();
    let baseline_zero: BTreeSet<&str> = input.baseline_coverage.zero_confirmed_categories.iter().map(String::as_str).collect();
    let actual_zero: BTreeSet<&str> = input.actual_coverage.zero_confirmed_categories.iter().map(String::as_str).collect();
    let baseline_lines: BTreeSet<&str> = input.baseline_costs.iter().map(|cost| cost.category.as_str()).collect();
    let actual_lines: BTreeSet<&str> = input.actual_costs.iter().map(|cost| cost.category.as_str()).collect();

    if baseline.is_empty() || baseline != actual {
        return Err(CommerceError::IncompleteCostCoverage);
    }
    if baseline_lines.iter().any(|category| !baseline.contains(category))
        || actual_lines.iter().any(|category| !actual.contains(category))
        || baseline_zero.iter().any(|category| baseline_lines.contains(category))
        || actual_zero.iter().any(|category| actual_lines.contains(category))
    {
        return Err(CommerceError::IncompleteCostCoverage);
    }

    let baseline_accounted: BTreeSet<&str> = baseline_lines.union(&baseline_zero).copied().collect();
    let actual_accounted: BTreeSet<&str> = actual_lines.union(&actual_zero).copied().collect();
    if baseline_accounted != baseline || actual_accounted != actual {
        return Err(CommerceError::IncompleteCostCoverage);
    }
    Ok(())
}

fn validate_basket_equivalence(baseline: &[BasketLine], actual: &[BasketLine]) -> Result<(), CommerceError> {
    if baseline.len() != actual.len() {
        return Err(CommerceError::BasketMismatch);
    }
    for baseline_line in baseline {
        let actual_line = actual
            .iter()
            .find(|candidate| candidate.product == baseline_line.product)
            .ok_or(CommerceError::BasketMismatch)?;
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
    validate_cost_category_coverage(input)?;

    validate_delivery_lines(&input.actual_lines, &input.delivery_lines)?;

    let baseline_merchandise = sum_amounts(input.baseline_lines.iter().map(|line| &line.line_total))?;
    let baseline_other = sum_costs(&input.baseline_costs)?;
    let actual_merchandise = sum_amounts(input.actual_lines.iter().map(|line| &line.line_total))?;
    let actual_other = sum_costs(&input.actual_costs)?;
    let participation = sum_costs(&input.participation_costs)?;
    let baseline_total = baseline_merchandise.checked_add(baseline_other)?;
    let actual_total = actual_merchandise.checked_add(actual_other)?.checked_add(participation)?;
    let net = baseline_total.checked_sub(actual_total)?;

    let baseline_kinds: BTreeSet<EvidenceKind> = input.baseline_lines.iter().map(|line| line.evidence.kind)
        .chain(input.baseline_costs.iter().map(|line| line.evidence.kind))
        .chain(std::iter::once(input.baseline_coverage.evidence.kind))
        .collect();
    let baseline_has_estimate = baseline_kinds.contains(&EvidenceKind::PublicListPriceEstimate)
        || input.baseline_coverage.evidence.kind == EvidenceKind::PublicListPriceEstimate
        || input.baseline_costs.iter().any(|line| line.evidence.kind == EvidenceKind::PublicListPriceEstimate);
    let has_alternative_invoice = baseline_kinds.contains(&EvidenceKind::AlternativeInvoice);
    let has_alternative_quote = baseline_kinds.contains(&EvidenceKind::AlternativeQuote);
    let claim_class = if baseline_has_estimate {
        ClaimClass::Estimate
    } else if input.delivery_lines.is_empty() {
        ClaimClass::ProvisionalWithoutDeliveryEvidence
    } else if has_alternative_invoice && has_alternative_quote {
        ClaimClass::MixedBaselineEvidence
    } else if baseline_kinds.len() == 1
        && has_alternative_invoice
        && input.baseline_coverage.evidence.kind == EvidenceKind::AlternativeInvoice
    {
        ClaimClass::HistoricalInvoiceComparison
    } else {
        ClaimClass::InvoiceVsAlternativeQuote
    };

    let evidence_references = collect_evidence_references(input)?;

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
        evidence_references,
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
            evidence: evidence(&format!("line-{kind:?}"), kind),
        }
    }

    fn cost(category: &str, amount: &str, kind: EvidenceKind) -> CostLine {
        CostLine {
            category: category.into(),
            direction: if kind == EvidenceKind::CreditNote { CostDirection::Credit } else { CostDirection::Charge },
            amount: Decimal::parse(amount).unwrap(),
            currency: "ZAR".into(),
            evidence: evidence(&format!("{category}-{kind:?}"), kind),
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
                considered_categories: vec!["freight".into()],
                zero_confirmed_categories: vec![],
                unresolved_costs: vec![],
                evidence: evidence("baseline-coverage", EvidenceKind::AlternativeQuote),
            },
            actual_coverage: CostCoverage {
                declared_complete: true,
                considered_categories: vec!["freight".into()],
                zero_confirmed_categories: vec![],
                unresolved_costs: vec![],
                evidence: evidence("actual-coverage", EvidenceKind::SupplierInvoice),
            },
            delivery_lines: vec![DeliveredLine {
                product: product(),
                quantity: Decimal::parse("10").unwrap(),
                evidence: evidence("delivery-receipt-line", EvidenceKind::DeliveryReceipt),
            }],
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

    fn receipt(input: SavingsInput, status: SavingsReceiptStatus, fixture_only: bool) -> SavingsReceipt {
        let calculation = calculate_savings(&input).unwrap();
        SavingsReceipt {
            receipt_id: "fixture:receipt:001".into(),
            revision: 1,
            status,
            fixture_only,
            algorithm_name: "cooperative-commerce-core".into(),
            algorithm_version: "0.1.0".into(),
            source_revision: if status == SavingsReceiptStatus::Computed {
                Some("a".repeat(40))
            } else {
                None
            },
            implementation_digest_sha256: if status == SavingsReceiptStatus::Computed {
                Some("b".repeat(64))
            } else {
                None
            },
            input_snapshot: input,
            claimed_calculation: calculation,
        }
    }

    #[test]
    fn receipt_verifier_recomputes_all_claimed_arithmetic() {
        let candidate = receipt(input(), SavingsReceiptStatus::Computed, false);
        let report = verify_savings_receipt(&candidate).unwrap();
        assert_eq!(report.net_difference, "120");
    }

    #[test]
    fn receipt_verifier_rejects_a_forged_total() {
        let mut candidate = receipt(input(), SavingsReceiptStatus::Computed, false);
        candidate.claimed_calculation.net_difference = "999999".into();
        assert_eq!(
            verify_savings_receipt(&candidate),
            Err(ReceiptVerificationError::CalculationMismatch)
        );
    }

    #[test]
    fn illustrative_fixture_cannot_claim_computed_status() {
        let candidate = receipt(input(), SavingsReceiptStatus::Computed, true);
        assert_eq!(
            verify_savings_receipt(&candidate),
            Err(ReceiptVerificationError::InvalidMetadata)
        );
    }

    #[test]
    fn computed_receipt_requires_source_and_implementation_identity() {
        let mut candidate = receipt(input(), SavingsReceiptStatus::Computed, false);
        candidate.implementation_digest_sha256 = None;
        assert_eq!(
            verify_savings_receipt(&candidate),
            Err(ReceiptVerificationError::InvalidMetadata)
        );
    }

    #[test]
    fn illustrative_receipt_is_verified_only_as_an_arithmetic_fixture() {
        let candidate = receipt(input(), SavingsReceiptStatus::Illustrative, true);
        let report = verify_savings_receipt(&candidate).unwrap();
        assert!(!report.evidence_authenticated_by_calculator);
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
        candidate.actual_lines[0].line_total = Decimal::parse("1200").unwrap();
        let report = calculate_savings(&candidate).unwrap();
        assert_eq!(report.net_difference, "-30");
        assert!(report.notes.iter().any(|note| note.contains("Net difference is negative")));
    }

    #[test]
    fn documented_credit_note_reduces_actual_costs_without_hidden_rebate_math() {
        let mut candidate = input();
        candidate.baseline_coverage.considered_categories.push("supplier-credit".into());
        candidate.baseline_coverage.zero_confirmed_categories.push("supplier-credit".into());
        candidate.actual_coverage.considered_categories.push("supplier-credit".into());
        candidate.actual_costs.push(cost("supplier-credit", "30.00", EvidenceKind::CreditNote));
        let report = calculate_savings(&candidate).unwrap();
        assert_eq!(report.actual_total, "1100");
        assert_eq!(report.net_difference, "150");
    }

    #[test]
    fn credit_direction_requires_credit_note_evidence() {
        let mut candidate = input();
        candidate.actual_costs[0].direction = CostDirection::Credit;
        assert_eq!(calculate_savings(&candidate), Err(CommerceError::UnexpectedEvidenceKind));
    }

    #[test]
    fn mismatched_cost_coverage_categories_block_comparison() {
        let mut candidate = input();
        candidate.actual_coverage.considered_categories = vec!["tax".into(), "freight".into()];
        assert_eq!(calculate_savings(&candidate), Err(CommerceError::IncompleteCostCoverage));
    }

    #[test]
    fn considered_category_requires_a_cost_line_or_explicit_zero_confirmation_on_both_sides() {
        let mut candidate = input();
        candidate.baseline_coverage.considered_categories.push("duty".into());
        candidate.actual_coverage.considered_categories.push("duty".into());
        assert_eq!(calculate_savings(&candidate), Err(CommerceError::IncompleteCostCoverage));
    }

    #[test]
    fn category_cannot_be_both_charged_and_confirmed_zero() {
        let mut candidate = input();
        candidate.actual_coverage.zero_confirmed_categories.push("freight".into());
        assert_eq!(calculate_savings(&candidate), Err(CommerceError::IncompleteCostCoverage));
    }

    #[test]
    fn cost_line_cannot_be_hidden_outside_declared_coverage() {
        let mut candidate = input();
        candidate.actual_costs.push(cost("customs", "10.00", EvidenceKind::CostInvoice));
        assert_eq!(calculate_savings(&candidate), Err(CommerceError::IncompleteCostCoverage));
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
    fn conflicting_evidence_ids_cannot_hide_distinct_sources() {
        let mut candidate = input();
        candidate.actual_lines[0].evidence.id = candidate.baseline_lines[0].evidence.id.clone();
        assert_eq!(calculate_savings(&candidate), Err(CommerceError::ConflictingEvidenceReference));
    }

    #[test]
    fn malformed_calendar_timestamp_is_rejected() {
        let mut candidate = input();
        candidate.actual_lines[0].evidence.captured_at = "2026-02-30T00:00:00Z".into();
        assert_eq!(calculate_savings(&candidate), Err(CommerceError::InvalidEvidence));
    }

    #[test]
    fn arbitrary_string_with_a_z_suffix_is_not_a_timestamp() {
        let mut candidate = input();
        candidate.actual_lines[0].evidence.captured_at = "made-upTZ".into();
        assert_eq!(calculate_savings(&candidate), Err(CommerceError::InvalidEvidence));
    }

    #[test]
    fn timezone_missing_from_evidence_timestamp_is_rejected() {
        let mut candidate = input();
        candidate.actual_lines[0].evidence.captured_at = "2026-10-09T00:00:00".into();
        assert_eq!(calculate_savings(&candidate), Err(CommerceError::InvalidEvidence));
    }

    #[test]
    fn malformed_digest_blocks_calculation() {
        let mut candidate = input();
        candidate.actual_lines[0].evidence.sha256 = "not-a-digest".into();
        assert_eq!(calculate_savings(&candidate), Err(CommerceError::InvalidEvidence));
    }

    #[test]
    fn mixed_invoice_and_quote_baseline_is_never_labeled_as_a_pure_quote_comparison() {
        let mut candidate = input();
        candidate.baseline_lines[0].evidence.kind = EvidenceKind::AlternativeInvoice;
        candidate.baseline_coverage.evidence.kind = EvidenceKind::AlternativeInvoice;
        candidate.baseline_costs[0].evidence.kind = EvidenceKind::AlternativeQuote;
        let report = calculate_savings(&candidate).unwrap();
        assert_eq!(report.claim_class, ClaimClass::MixedBaselineEvidence);
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
    fn partial_delivery_cannot_support_full_basket_savings_claim() {
        let mut candidate = input();
        candidate.delivery_lines[0].quantity = Decimal::parse("9").unwrap();
        assert_eq!(calculate_savings(&candidate), Err(CommerceError::BasketMismatch));
    }

    #[test]
    fn delivery_receipt_must_bind_exact_product_and_unit() {
        let mut candidate = input();
        candidate.delivery_lines[0].product.unit_code_system = "urn:other:unit-system:v1".into();
        assert_eq!(calculate_savings(&candidate), Err(CommerceError::BasketMismatch));
    }

    #[test]
    fn missing_delivery_evidence_is_not_called_realized_savings() {
        let mut candidate = input();
        candidate.delivery_lines.clear();
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
