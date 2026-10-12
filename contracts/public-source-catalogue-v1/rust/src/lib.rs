//! Strict offline validation for source catalogues and source observations.
//!
//! This crate performs no network access. It is intentionally a narrow Rust
//! domain validator, not a general JSON Schema Draft 2020-12 implementation.

use serde::Deserialize;
use serde_json::Value;
use std::collections::HashSet;
use std::fmt;
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationErrors {
    pub messages: Vec<String>,
}

impl ValidationErrors {
    fn one(message: impl Into<String>) -> Self {
        Self { messages: vec![message.into()] }
    }

    fn push(&mut self, message: impl Into<String>) {
        self.messages.push(message.into());
    }

    fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }
}

impl fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, message) in self.messages.iter().enumerate() {
            if index > 0 {
                writeln!(f)?;
            }
            write!(f, "- {message}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ValidationErrors {}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceCatalogue {
    pub schema_version: String,
    pub catalogue_revision: String,
    pub reviewed_on: String,
    pub sources: Vec<Source>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub source_id: String,
    pub display_name: String,
    pub jurisdiction: Jurisdiction,
    pub publisher: Publisher,
    pub source_url: String,
    pub documentation_url: String,
    pub source_type: SourceType,
    pub access: Access,
    pub terms: Terms,
    pub data_scope: DataScope,
    pub known_limitations: Vec<String>,
    pub ingestion: Ingestion,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Publisher {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Access {
    pub mode: AccessMode,
    pub documented_methods: Vec<AccessMethod>,
    pub transport_policy: TransportPolicy,
    pub transport_verification: TransportVerification,
    pub pagination_note: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Terms {
    pub policy_url: String,
    pub reuse_status: ReuseStatus,
    pub must_recheck_before_ingestion: bool,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DataScope {
    pub granularity: DataGranularity,
    pub personal_data_expectation: PersonalDataExpectation,
    pub datasets_or_content: Vec<String>,
    pub collection_policy: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ingestion {
    pub status: IngestionStatus,
    pub enabled: bool,
    pub reason: String,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub enum Jurisdiction {
    #[serde(rename = "ZA")]
    SouthAfrica,
    #[serde(rename = "ZA-GP-JHB")]
    GautengJohannesburg,
}

#[derive(Debug, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    MunicipalFinanceApi,
    ProcurementOpenData,
    PublicServiceInformation,
}

#[derive(Debug, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AccessMode {
    PublicApi,
    PublicBulkDownload,
    ManualPublicationReview,
}

#[derive(Debug, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AccessMethod {
    Get,
    ManualReview,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransportPolicy {
    HttpsOnly,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransportVerification {
    NotTested,
    HttpsConfirmed,
    BlockedPendingSourceConfirmation,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub enum ReuseStatus {
    #[serde(rename = "terms_review_required")]
    TermsReviewRequired,
    #[serde(rename = "cc-by-4.0_documented")]
    CcBy40Documented,
    #[serde(rename = "not_assessed")]
    NotAssessed,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DataGranularity {
    AggregateFinancial,
    ProcurementRecords,
    PublicServicePages,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PersonalDataExpectation {
    AggregateNoPersonalDataExpected,
    PublicPagesMayIncludeContactDetails,
    Unknown,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IngestionStatus {
    CatalogueOnly,
}

/// Parse and validate a source catalogue. Unknown object fields are rejected
/// during deserialization; semantic and cross-field constraints are checked
/// after parsing.
pub fn validate_catalogue_json(input: &str) -> Result<SourceCatalogue, ValidationErrors> {
    let catalogue: SourceCatalogue = serde_json::from_str(input)
        .map_err(|error| ValidationErrors::one(format!("invalid catalogue JSON: {error}")))?;
    let errors = validate_catalogue(&catalogue);
    if errors.is_empty() {
        Ok(catalogue)
    } else {
        Err(errors)
    }
}

fn validate_catalogue(catalogue: &SourceCatalogue) -> ValidationErrors {
    let mut errors = ValidationErrors { messages: Vec::new() };

    if catalogue.schema_version != "public-source-catalogue-v1" {
        errors.push("schema_version must be public-source-catalogue-v1");
    }
    if catalogue.catalogue_revision.trim().is_empty() {
        errors.push("catalogue_revision must not be empty");
    }
    if !valid_iso_date(&catalogue.reviewed_on) {
        errors.push("reviewed_on must be a valid YYYY-MM-DD date");
    }
    if catalogue.sources.is_empty() {
        errors.push("sources must contain at least one source");
    }

    let mut seen_ids = HashSet::new();
    for (index, source) in catalogue.sources.iter().enumerate() {
        let prefix = format!("sources[{index}]");
        if !valid_source_id(&source.source_id) {
            errors.push(format!("{prefix}.source_id has an invalid format"));
        }
        if !seen_ids.insert(source.source_id.as_str()) {
            errors.push(format!("duplicate source_id: {}", source.source_id));
        }
        if source.display_name.trim().is_empty() {
            errors.push(format!("{prefix}.display_name must not be empty"));
        }
        if source.publisher.name.trim().is_empty() {
            errors.push(format!("{prefix}.publisher.name must not be empty"));
        }

        for (field, value) in [
            ("publisher.url", source.publisher.url.as_str()),
            ("source_url", source.source_url.as_str()),
            ("documentation_url", source.documentation_url.as_str()),
            ("terms.policy_url", source.terms.policy_url.as_str()),
        ] {
            if !valid_https_url(value) {
                errors.push(format!("{prefix}.{field} must be an absolute HTTPS URL without userinfo"));
            }
        }

        if source.access.documented_methods.is_empty() {
            errors.push(format!("{prefix}.access.documented_methods must not be empty"));
        }
        let mut methods = HashSet::new();
        for method in &source.access.documented_methods {
            if !methods.insert(method) {
                errors.push(format!("{prefix}.access.documented_methods contains a duplicate method"));
            }
        }
        if source.terms.must_recheck_before_ingestion != true {
            errors.push(format!("{prefix}.terms.must_recheck_before_ingestion must be true"));
        }
        if source.data_scope.datasets_or_content.is_empty() {
            errors.push(format!("{prefix}.data_scope.datasets_or_content must not be empty"));
        }
        if source.data_scope.collection_policy.trim().is_empty() {
            errors.push(format!("{prefix}.data_scope.collection_policy must not be empty"));
        }
        if source.known_limitations.is_empty()
            || source.known_limitations.iter().any(|limitation| limitation.trim().is_empty())
        {
            errors.push(format!("{prefix}.known_limitations must contain non-empty entries"));
        }
        if source.ingestion.status != IngestionStatus::CatalogueOnly {
            errors.push(format!("{prefix}.ingestion.status must be catalogue_only"));
        }
        if source.ingestion.enabled {
            errors.push(format!("{prefix}.ingestion.enabled must remain false until a separate authorization gate"));
        }
        if source.ingestion.reason.trim().is_empty() {
            errors.push(format!("{prefix}.ingestion.reason must not be empty"));
        }
        if source.access.transport_policy != TransportPolicy::HttpsOnly {
            errors.push(format!("{prefix}.access.transport_policy must be https_only"));
        }
    }

    errors
}

/// Parse and validate one source observation. This validates structure and
/// safety-relevant domain invariants but does not establish source authenticity.
pub fn validate_observation_json(input: &str) -> Result<SourceObservation, ValidationErrors> {
    let observation: SourceObservation = serde_json::from_str(input)
        .map_err(|error| ValidationErrors::one(format!("invalid observation JSON: {error}")))?;
    let errors = validate_observation(&observation);
    if errors.is_empty() {
        Ok(observation)
    } else {
        Err(errors)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceObservation {
    pub schema_version: String,
    pub observation_id: String,
    pub source_id: String,
    pub source_uri: String,
    pub retrieved_at: String,
    pub source_period: SourcePeriod,
    pub dataset_id: String,
    pub field_path: String,
    pub raw_value: Value,
    pub raw_value_state: RawValueState,
    pub normalization: Normalization,
    pub source_snapshot_sha256: Value,
    pub fixture_only: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourcePeriod {
    pub financial_year_end: Option<i64>,
    pub period_label: String,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RawValueState {
    ReportedValue,
    ReportedZero,
    ReportedNull,
    RowOmitted,
    InvalidValue,
    NotRetrieved,
    Stale,
    Provisional,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Normalization {
    pub applied: bool,
    /// Must be a JSON string or null. A Value is used instead of Option<String>
    /// so missing-field and explicitly-null semantics are distinguishable.
    pub rule_id: Value,
    pub rule_version: Value,
    pub normalized_value: Value,
    pub reason: String,
}

fn validate_observation(observation: &SourceObservation) -> ValidationErrors {
    let mut errors = ValidationErrors { messages: Vec::new() };

    if observation.schema_version != "public-observation-v1" {
        errors.push("schema_version must be public-observation-v1");
    }
    if observation.observation_id.trim().is_empty() {
        errors.push("observation_id must not be empty");
    }
    if !valid_source_id(&observation.source_id) {
        errors.push("source_id has an invalid format");
    }
    if !valid_https_url(&observation.source_uri) {
        errors.push("source_uri must be an absolute HTTPS URL without userinfo");
    }
    if !valid_rfc3339_timestamp(&observation.retrieved_at) {
        errors.push("retrieved_at must be a valid RFC3339 timestamp with an explicit timezone");
    }
    if observation
        .source_period
        .financial_year_end
        .is_some_and(|year| !(1900..=2200).contains(&year))
    {
        errors.push("source_period.financial_year_end must be between 1900 and 2200");
    }
    if observation.source_period.period_label.trim().is_empty() {
        errors.push("source_period.period_label must not be empty");
    }
    if observation.dataset_id.trim().is_empty() {
        errors.push("dataset_id must not be empty");
    }
    if observation.field_path.trim().is_empty() {
        errors.push("field_path must not be empty");
    }
    if observation.normalization.reason.trim().is_empty() {
        errors.push("normalization.reason must not be empty");
    }

    let rule_id = observation.normalization.rule_id.as_str();
    let rule_version = observation.normalization.rule_version.as_str();
    if observation.normalization.applied {
        if !rule_id.is_some_and(|value| !value.trim().is_empty()) {
            errors.push("normalization.rule_id must be a non-empty string when normalization is applied");
        }
        if !rule_version.is_some_and(|value| !value.trim().is_empty()) {
            errors.push("normalization.rule_version must be a non-empty string when normalization is applied");
        }
        if matches!(observation.raw_value_state, RawValueState::RowOmitted | RawValueState::NotRetrieved) {
            errors.push("normalization cannot be applied to a row_omitted or not_retrieved observation");
        }
    } else {
        if !observation.normalization.rule_id.is_null() {
            errors.push("normalization.rule_id must be null when normalization.applied is false");
        }
        if !observation.normalization.rule_version.is_null() {
            errors.push("normalization.rule_version must be null when normalization.applied is false");
        }
        if observation.normalization.normalized_value != observation.raw_value {
            errors.push("normalized_value must equal raw_value when normalization.applied is false");
        }
    }

    match observation.raw_value_state {
        RawValueState::ReportedZero => {
            if !observation.raw_value.as_f64().is_some_and(|number| number == 0.0) {
                errors.push("reported_zero requires raw_value to be numeric zero");
            }
        }
        RawValueState::ReportedNull => {
            if !observation.raw_value.is_null() {
                errors.push("reported_null requires raw_value to be JSON null");
            }
        }
        RawValueState::ReportedValue => {
            if observation.raw_value.is_null()
                || observation.raw_value.as_f64().is_some_and(|number| number == 0.0)
            {
                errors.push("reported_value cannot contain null or numeric zero; use the corresponding explicit state");
            }
        }
        RawValueState::RowOmitted | RawValueState::NotRetrieved => {
            if !observation.raw_value.is_null() {
                errors.push("row_omitted and not_retrieved require raw_value to be null");
            }
        }
        RawValueState::InvalidValue | RawValueState::Stale | RawValueState::Provisional => {}
    }

    if observation.source_snapshot_sha256.is_null() {
        if !observation.fixture_only {
            errors.push("source_snapshot_sha256 is required for non-fixture observations");
        }
    } else if !observation
        .source_snapshot_sha256
        .as_str()
        .is_some_and(valid_sha256)
    {
        errors.push("source_snapshot_sha256 must be null for a fixture or sha256:<64 lowercase hex digits>");
    }

    errors
}

fn valid_source_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    (3..=64).contains(&bytes.len())
        && (bytes[0].is_ascii_lowercase() || bytes[0].is_ascii_digit())
        && bytes.iter().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-'
        })
}

fn valid_https_url(value: &str) -> bool {
    let Ok(url) = Url::parse(value) else {
        return false;
    };
    url.scheme() == "https"
        && url.host_str().is_some_and(|host| !host.is_empty())
        && url.username().is_empty()
        && url.password().is_none()
}

fn valid_sha256(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn valid_iso_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    if !value.is_ascii() || bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    let Ok(year) = value[0..4].parse::<i32>() else { return false };
    let Ok(month) = value[5..7].parse::<u32>() else { return false };
    let Ok(day) = value[8..10].parse::<u32>() else { return false };
    if !(1..=12).contains(&month) {
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
    year >= 1900 && (1..=days_in_month).contains(&day)
}

fn valid_rfc3339_timestamp(value: &str) -> bool {
    // Strictly check the common RFC3339 shape and component ranges without
    // interpreting the timestamp as a local time or silently adding a zone.
    if value.len() < 20 || !value.is_ascii() {
        return false;
    }
    let Some((date, time_and_zone)) = value.split_once('T') else { return false };
    if !valid_iso_date(date) {
        return false;
    }

    let zone_start = if let Some(position) = time_and_zone.rfind('Z') {
        if position != time_and_zone.len() - 1 { return false; }
        position
    } else if let Some(position) = time_and_zone.rfind('+').or_else(|| time_and_zone.rfind('-')) {
        if position < 8 { return false; }
        let zone = &time_and_zone[position + 1..];
        if zone.len() != 5 || zone.as_bytes()[2] != b':' {
            return false;
        }
        let Ok(hours) = zone[0..2].parse::<u32>() else { return false };
        let Ok(minutes) = zone[3..5].parse::<u32>() else { return false };
        if hours > 23 || minutes > 59 { return false; }
        position
    } else {
        return false;
    };

    let time = &time_and_zone[..zone_start];
    let time = time.split('.').next().unwrap_or(time);
    let parts: Vec<&str> = time.split(':').collect();
    if parts.len() != 3 || parts.iter().any(|part| part.len() != 2 || !part.bytes().all(|b| b.is_ascii_digit())) {
        return false;
    }
    let Ok(hour) = parts[0].parse::<u32>() else { return false };
    let Ok(minute) = parts[1].parse::<u32>() else { return false };
    let Ok(second) = parts[2].parse::<u32>() else { return false };
    hour <= 23 && minute <= 59 && second <= 60
}
