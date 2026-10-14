//! Strict offline validation for the civic cross-repository mapping register.
//!
//! The register documents candidate semantic relationships. It does not execute
//! transforms, merge records, retrieve data, or authorise operations.

use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationErrors {
    pub messages: Vec<String>,
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
pub struct CivicRecordCrosswalk {
    pub schema_version: String,
    pub review_snapshot: String,
    pub status: CrosswalkStatus,
    pub contracts: Vec<ContractBinding>,
    pub mappings: Vec<MappingRule>,
    pub invariants: Vec<String>,
    pub non_goals: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrosswalkStatus {
    StaticMappingContractNoRuntimeTransform,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    JsonSchema,
    RustSerdeTypes,
    EvaluationCorpus,
    ProposedReference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Availability {
    PresentAtSnapshot,
    ReferenceOnlyNotPresentInAuditedSnapshot,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContractBinding {
    pub schema_id: String,
    pub repository: String,
    pub snapshot_commit_sha: String,
    pub path: Option<String>,
    pub git_blob_sha1: Option<String>,
    pub version: Option<String>,
    pub artifact_kind: ArtifactKind,
    pub availability: Availability,
    pub notes: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticRelationship {
    PartialSemanticOverlap,
    DigestSubjectMismatch,
    IdentityNamespaceOnly,
    IncompatibleStateMachine,
    DistinctSemanticDimension,
    NoAuthorityMapping,
    NotRuntimeContract,
    NotAvailable,
    ExplicitPublicProjection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversionPolicy {
    ExplicitAdapterRequired,
    NoAutomaticMapping,
    NamespacedReferenceOnly,
    PreserveSeparately,
    BlockedUntilContractExists,
    DedicatedSanitizingProjection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    CandidateOnly,
    PreserveSeparate,
    Prohibited,
    Blocked,
    RequiredBoundary,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MappingRule {
    pub mapping_id: String,
    pub from: FieldBinding,
    pub to: FieldBinding,
    pub relationship: SemanticRelationship,
    pub conversion_policy: ConversionPolicy,
    pub disposition: Disposition,
    pub rationale: String,
    pub required_guards: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldBinding {
    pub schema_id: String,
    pub field_paths: Vec<String>,
}

/// Parse and validate one exact crosswalk register.
pub fn parse_and_validate_json(input: &str) -> Result<CivicRecordCrosswalk, ValidationErrors> {
    let crosswalk: CivicRecordCrosswalk = serde_json::from_str(input).map_err(|error| {
        ValidationErrors {
            messages: vec![format!("invalid crosswalk JSON: {error}")],
        }
    })?;
    let errors = validate(&crosswalk);
    if errors.is_empty() {
        Ok(crosswalk)
    } else {
        Err(ValidationErrors { messages: errors })
    }
}

fn valid_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_iso_date_shape(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit())
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn find_contract<'a>(
    contracts: &'a HashMap<&'a str, &'a ContractBinding>,
    schema_id: &str,
) -> Option<&'a ContractBinding> {
    contracts.get(schema_id).copied()
}

fn validate(crosswalk: &CivicRecordCrosswalk) -> Vec<String> {
    let mut errors = Vec::new();

    if crosswalk.schema_version != "civic-record-crosswalk-v1" {
        errors.push("schema_version must be civic-record-crosswalk-v1".to_owned());
    }
    if !valid_iso_date_shape(&crosswalk.review_snapshot) {
        errors.push("review_snapshot must have YYYY-MM-DD shape".to_owned());
    }
    if crosswalk.contracts.is_empty() {
        errors.push("contracts must not be empty".to_owned());
    }
    if crosswalk.mappings.is_empty() {
        errors.push("mappings must not be empty".to_owned());
    }
    if crosswalk.invariants.is_empty() || crosswalk.invariants.iter().any(|item| !nonempty(item)) {
        errors.push("invariants must contain non-empty statements".to_owned());
    }
    if crosswalk.non_goals.is_empty() || crosswalk.non_goals.iter().any(|item| !nonempty(item)) {
        errors.push("non_goals must contain non-empty statements".to_owned());
    }

    let mut contracts_by_id = HashMap::new();
    for contract in &crosswalk.contracts {
        if !nonempty(&contract.schema_id) {
            errors.push("contract schema_id must not be empty".to_owned());
        }
        if !nonempty(&contract.repository) {
            errors.push(format!("{}: repository must not be empty", contract.schema_id));
        }
        if !valid_hex(&contract.snapshot_commit_sha, 40) {
            errors.push(format!(
                "{}: snapshot_commit_sha must be 40 lowercase hexadecimal characters",
                contract.schema_id
            ));
        }
        if !nonempty(&contract.notes) {
            errors.push(format!("{}: notes must not be empty", contract.schema_id));
        }
        match contract.availability {
            Availability::PresentAtSnapshot => {
                if contract.path.as_deref().is_none_or(|path| !nonempty(path)) {
                    errors.push(format!("{}: present artifact requires a path", contract.schema_id));
                }
                if contract
                    .git_blob_sha1
                    .as_deref()
                    .is_none_or(|sha| !valid_hex(sha, 40))
                {
                    errors.push(format!(
                        "{}: present artifact requires a valid Git blob SHA-1",
                        contract.schema_id
                    ));
                }
            }
            Availability::ReferenceOnlyNotPresentInAuditedSnapshot => {
                if contract.path.is_some() || contract.git_blob_sha1.is_some() {
                    errors.push(format!(
                        "{}: an artifact absent at the audited snapshot cannot claim a path or blob",
                        contract.schema_id
                    ));
                }
                if contract.artifact_kind != ArtifactKind::ProposedReference {
                    errors.push(format!(
                        "{}: an unavailable artifact must be marked proposed_reference",
                        contract.schema_id
                    ));
                }
            }
        }
        if contracts_by_id
            .insert(contract.schema_id.as_str(), contract)
            .is_some()
        {
            errors.push(format!("duplicate contract schema_id: {}", contract.schema_id));
        }
    }

    let mut mapping_ids = HashSet::new();
    for mapping in &crosswalk.mappings {
        if !nonempty(&mapping.mapping_id)
            || !mapping_ids.insert(mapping.mapping_id.as_str())
        {
            errors.push(format!("empty or duplicate mapping_id: {}", mapping.mapping_id));
        }
        if !nonempty(&mapping.rationale) {
            errors.push(format!("{}: rationale must not be empty", mapping.mapping_id));
        }
        if mapping.required_guards.is_empty()
            || mapping.required_guards.iter().any(|guard| !nonempty(guard))
        {
            errors.push(format!(
                "{}: required_guards must contain non-empty statements",
                mapping.mapping_id
            ));
        }

        let source = find_contract(&contracts_by_id, &mapping.from.schema_id);
        let target = find_contract(&contracts_by_id, &mapping.to.schema_id);
        if source.is_none() {
            errors.push(format!(
                "{}: unknown source schema_id {}",
                mapping.mapping_id, mapping.from.schema_id
            ));
        }
        if target.is_none() {
            errors.push(format!(
                "{}: unknown target schema_id {}",
                mapping.mapping_id, mapping.to.schema_id
            ));
        }
        if mapping.from.field_paths.is_empty()
            || mapping.from.field_paths.iter().any(|path| !nonempty(path))
        {
            errors.push(format!(
                "{}: from.field_paths must contain non-empty paths",
                mapping.mapping_id
            ));
        }
        if mapping.to.field_paths.iter().any(|path| !nonempty(path)) {
            errors.push(format!(
                "{}: to.field_paths cannot contain empty paths",
                mapping.mapping_id
            ));
        }
        if let Some(target) = target {
            match target.availability {
                Availability::PresentAtSnapshot if mapping.to.field_paths.is_empty() => {
                    errors.push(format!(
                        "{}: present target requires a declared target field path",
                        mapping.mapping_id
                    ));
                }
                Availability::ReferenceOnlyNotPresentInAuditedSnapshot => {
                    if !mapping.to.field_paths.is_empty() {
                        errors.push(format!(
                            "{}: unavailable target cannot declare target field paths",
                            mapping.mapping_id
                        ));
                    }
                    let blocked_dependency = mapping.relationship == SemanticRelationship::NotAvailable
                        && mapping.conversion_policy == ConversionPolicy::BlockedUntilContractExists
                        && mapping.disposition == Disposition::Blocked;
                    let explicit_prohibition = matches!(
                        mapping.relationship,
                        SemanticRelationship::NoAuthorityMapping | SemanticRelationship::NotRuntimeContract
                    ) && mapping.conversion_policy == ConversionPolicy::NoAutomaticMapping
                        && mapping.disposition == Disposition::Prohibited;
                    if !blocked_dependency && !explicit_prohibition {
                        errors.push(format!(
                            "{}: unavailable target must remain blocked or be an explicit non-mapping prohibition",
                            mapping.mapping_id
                        ));
                    }
                }
                _ => {}
            }
        }
    }

    // Critical semantic guardrails: these rules must remain conservative.
    require_rule(
        crosswalk,
        "MYCELIX-CASE-ID-TO-SOL-CASE-ID",
        SemanticRelationship::IdentityNamespaceOnly,
        ConversionPolicy::NamespacedReferenceOnly,
        Disposition::PreserveSeparate,
        &mut errors,
    );
    require_rule(
        crosswalk,
        "MYCELIX-CASE-STATE-TO-SOL-STATUS",
        SemanticRelationship::IncompatibleStateMachine,
        ConversionPolicy::NoAutomaticMapping,
        Disposition::Prohibited,
        &mut errors,
    );
    require_rule(
        crosswalk,
        "OBSERVATION-DIGEST-TO-SOL-DIGEST",
        SemanticRelationship::DigestSubjectMismatch,
        ConversionPolicy::NoAutomaticMapping,
        Disposition::Prohibited,
        &mut errors,
    );
    require_rule(
        crosswalk,
        "MYCELIX-EVIDENCE-INDEPENDENCE-TO-SOL-REVIEW",
        SemanticRelationship::DistinctSemanticDimension,
        ConversionPolicy::PreserveSeparately,
        Disposition::PreserveSeparate,
        &mut errors,
    );
    require_rule(
        crosswalk,
        "APPLICABILITY-TO-AUTHORITY",
        SemanticRelationship::NoAuthorityMapping,
        ConversionPolicy::NoAutomaticMapping,
        Disposition::Prohibited,
        &mut errors,
    );
    require_rule(
        crosswalk,
        "PLATFORM-WORK-CASE-CONTRACT-DEPENDENCY",
        SemanticRelationship::NotAvailable,
        ConversionPolicy::BlockedUntilContractExists,
        Disposition::Blocked,
        &mut errors,
    );
    require_rule(
        crosswalk,
        "PLATFORM-OPERATIONS-EVENT-CONTRACT-DEPENDENCY",
        SemanticRelationship::NotAvailable,
        ConversionPolicy::BlockedUntilContractExists,
        Disposition::Blocked,
        &mut errors,
    );
    require_rule(
        crosswalk,
        "SOL-INTERNAL-TO-PUBLIC-PROJECTION",
        SemanticRelationship::ExplicitPublicProjection,
        ConversionPolicy::DedicatedSanitizingProjection,
        Disposition::RequiredBoundary,
        &mut errors,
    );

    errors
}

fn require_rule(
    crosswalk: &CivicRecordCrosswalk,
    mapping_id: &str,
    relationship: SemanticRelationship,
    conversion_policy: ConversionPolicy,
    disposition: Disposition,
    errors: &mut Vec<String>,
) {
    match crosswalk
        .mappings
        .iter()
        .find(|mapping| mapping.mapping_id == mapping_id)
    {
        Some(mapping)
            if mapping.relationship == relationship
                && mapping.conversion_policy == conversion_policy
                && mapping.disposition == disposition => {}
        Some(_) => errors.push(format!(
            "{mapping_id}: required conservative semantic guardrail was weakened"
        )),
        None => errors.push(format!("{mapping_id}: required semantic guardrail is missing")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const FIXTURE: &str = include_str!("../../crosswalk.json");

    fn fixture_value() -> Value {
        serde_json::from_str(FIXTURE).expect("checked-in crosswalk fixture must be valid JSON")
    }

    #[test]
    fn accepts_checked_in_crosswalk_and_exact_source_pins() {
        let crosswalk = parse_and_validate_json(FIXTURE).expect("crosswalk should validate");
        assert_eq!(crosswalk.schema_version, "civic-record-crosswalk-v1");
        assert_eq!(crosswalk.contracts.len(), 7);
        assert!(crosswalk.mappings.len() >= 10);
    }

    #[test]
    fn state_machine_mapping_cannot_be_weakened_to_an_automatic_adapter() {
        let mut value = fixture_value();
        let rule = value["mappings"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|rule| rule["mapping_id"] == "MYCELIX-CASE-STATE-TO-SOL-STATUS")
            .unwrap();
        rule["conversion_policy"] = Value::String("explicit_adapter_required".to_owned());
        assert!(parse_and_validate_json(&value.to_string()).is_err());
    }

    #[test]
    fn native_record_ids_must_remain_namespaced() {
        let mut value = fixture_value();
        let rule = value["mappings"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|rule| rule["mapping_id"] == "MYCELIX-CASE-ID-TO-SOL-CASE-ID")
            .unwrap();
        rule["conversion_policy"] = Value::String("copy_exact".to_owned());
        assert!(parse_and_validate_json(&value.to_string()).is_err());
    }

    #[test]
    fn unavailable_work_case_contract_cannot_acquire_invented_fields() {
        let mut value = fixture_value();
        let rule = value["mappings"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|rule| rule["mapping_id"] == "PLATFORM-WORK-CASE-CONTRACT-DEPENDENCY")
            .unwrap();
        rule["to"]["field_paths"] = serde_json::json!(["case_id", "status"]);
        assert!(parse_and_validate_json(&value.to_string()).is_err());
    }

    #[test]
    fn absent_target_contract_allows_only_blocked_dependencies_or_explicit_prohibitions() {
        let crosswalk = parse_and_validate_json(FIXTURE).expect("valid crosswalk");
        for id in ["APPLICABILITY-TO-AUTHORITY", "SYMTHAEA-CORPUS-TO-ACTION"] {
            let rule = crosswalk.mappings.iter().find(|rule| rule.mapping_id == id).unwrap();
            assert!(rule.to.field_paths.is_empty());
            assert_eq!(rule.conversion_policy, ConversionPolicy::NoAutomaticMapping);
            assert_eq!(rule.disposition, Disposition::Prohibited);
        }
    }

    #[test]
    fn applicability_never_becomes_authority() {
        let mut value = fixture_value();
        let rule = value["mappings"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|rule| rule["mapping_id"] == "APPLICABILITY-TO-AUTHORITY")
            .unwrap();
        rule["disposition"] = Value::String("candidate_only".to_owned());
        assert!(parse_and_validate_json(&value.to_string()).is_err());
    }

    #[test]
    fn unknown_fields_fail_closed() {
        let mut value = fixture_value();
        value["unexpected_runtime_transform"] = Value::Bool(true);
        assert!(parse_and_validate_json(&value.to_string()).is_err());
    }

    #[test]
    fn duplicate_mapping_ids_are_rejected() {
        let mut value = fixture_value();
        let mappings = value["mappings"].as_array_mut().unwrap();
        let duplicate = mappings[0].clone();
        mappings.push(duplicate);
        assert!(parse_and_validate_json(&value.to_string()).is_err());
    }
}
