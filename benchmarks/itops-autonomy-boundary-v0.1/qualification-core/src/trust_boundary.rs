//! Deterministic trust-boundary audit for machine-checkable handoff provenance.
//!
//! This module compares a handoff against evaluator-owned expected lineage. It
//! performs no LLM inference and does not issue authority. It can detect missing
//! or substituted metadata, but cannot prove that the evaluator's expected
//! context is truthful, that a source digest was honestly produced, or that two
//! declared control domains are operationally independent.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityLineage {
    pub root_task_id: String,
    pub root_task_digest: String,
    pub root_plan_digest: String,
    pub authority_commitment_sha256: String,
    pub leaf_task_id: String,
    pub leaf_task_digest: String,
    pub leaf_plan_digest: String,
    pub leaf_grant_id: String,
    pub principal_chain: Vec<String>,
    pub grant_chain: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedSource {
    pub evidence_id: String,
    pub source_id: String,
    pub source_sha256: String,
    pub origin_principal: String,
    /// Opaque reference to a declared control domain, not proof of independence.
    pub declared_control_domain_id: String,
    pub required_derived_from: BTreeSet<String>,
    pub allowed_derived_from: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedHandoffContext {
    pub handoff_id: String,
    pub parent_handoff_id: Option<String>,
    pub authority: AuthorityLineage,
    pub sources: Vec<ExpectedSource>,
    pub expected_transform_ids: BTreeSet<String>,
    pub required_derived_from: BTreeSet<String>,
    pub allowed_derived_from: BTreeSet<String>,
    pub input_payload_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceEvidence {
    pub evidence_id: String,
    pub source_id: String,
    pub source_sha256: String,
    pub origin_principal: String,
    /// Preserve as lineage metadata; do not infer independence from this value.
    pub declared_control_domain_id: String,
    pub derived_from: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransformationEvidence {
    pub transform_id: String,
    pub input_payload_sha256: String,
    pub output_payload_sha256: String,
    pub derived_from: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandoffEvidence {
    pub handoff_id: String,
    pub parent_handoff_id: Option<String>,
    pub authority: Option<AuthorityLineage>,
    pub source_evidence: Vec<SourceEvidence>,
    pub derived_from: BTreeSet<String>,
    pub transformation: Option<TransformationEvidence>,
    pub payload_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditOutcome {
    Preserved,
    Erased,
    Substituted,
    Inconclusive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrustBoundaryViolation {
    InvalidExpectedContext,
    HandoffIdMissing,
    HandoffIdSubstituted,
    ParentHandoffErased,
    ParentHandoffSubstituted,
    AuthorityContextErased,
    AuthorityContextSubstituted,
    SourceEvidenceErased(String),
    SourceEvidenceSubstituted(String),
    SourceEvidenceDuplicated(String),
    UnexpectedSourceEvidence(String),
    SourceDigestInvalid(String),
    SourceOriginErased(String),
    SourceOriginSubstituted(String),
    SourceDomainErased(String),
    SourceDomainSubstituted(String),
    SourceLineageErased(String),
    SourceLineageSubstituted(String),
    HandoffLineageErased(String),
    HandoffLineageSubstituted(String),
    TransformationErased,
    TransformationIdErased,
    TransformationIdSubstituted,
    TransformationInputSubstituted,
    TransformationOutputSubstituted,
    PayloadErased,
    PayloadDigestInvalid,
    PayloadBindingSubstituted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustBoundaryAudit {
    pub outcome: AuditOutcome,
    pub violations: Vec<TrustBoundaryViolation>,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn digest_is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn authority_is_valid(authority: &AuthorityLineage) -> bool {
    nonempty(&authority.root_task_id)
        && digest_is_sha256(&authority.root_task_digest)
        && digest_is_sha256(&authority.root_plan_digest)
        && digest_is_sha256(&authority.authority_commitment_sha256)
        && nonempty(&authority.leaf_task_id)
        && digest_is_sha256(&authority.leaf_task_digest)
        && digest_is_sha256(&authority.leaf_plan_digest)
        && nonempty(&authority.leaf_grant_id)
        && authority.principal_chain.len() >= 2
        && authority.principal_chain.iter().all(|item| nonempty(item))
        && authority.grant_chain.iter().all(|item| nonempty(item))
        && !authority.grant_chain.is_empty()
        && {
            let unique: BTreeSet<&str> =
                authority.grant_chain.iter().map(String::as_str).collect();
            unique.len() == authority.grant_chain.len()
        }
}

fn expected_is_valid(expected: &ExpectedHandoffContext) -> bool {
    if !nonempty(&expected.handoff_id)
        || expected.parent_handoff_id.as_ref().is_some_and(|id| !nonempty(id))
        || !authority_is_valid(&expected.authority)
        || !digest_is_sha256(&expected.input_payload_sha256)
        || expected.expected_transform_ids.is_empty()
        || expected.expected_transform_ids.iter().any(|id| !nonempty(id))
        || expected.sources.iter().any(|source| {
            !nonempty(&source.evidence_id)
                || !nonempty(&source.source_id)
                || !digest_is_sha256(&source.source_sha256)
                || !nonempty(&source.origin_principal)
                || !nonempty(&source.declared_control_domain_id)
                || source.required_derived_from.iter().any(|item| !nonempty(item))
                || source.allowed_derived_from.iter().any(|item| !nonempty(item))
                || !source.required_derived_from.is_subset(&source.allowed_derived_from)
        })
        || expected.required_derived_from.iter().any(|item| !nonempty(item))
        || expected.allowed_derived_from.iter().any(|item| !nonempty(item))
        || !expected.required_derived_from.is_subset(&expected.allowed_derived_from)
    {
        return false;
    }

    let mut evidence_ids = BTreeSet::new();
    expected.sources.iter().all(|source| evidence_ids.insert(&source.evidence_id))
}

/// Audits one handoff against a trusted, evaluator-owned expected context.
///
/// Erased means a required marker, relationship, or source is missing.
/// Substituted means an identifier, digest, authority record, or binding has
/// changed or unexpected provenance was introduced. Inconclusive means the
/// expected context itself is malformed; that is an evaluator-input failure,
/// not evidence that the handoff passed.
pub fn audit_handoff(
    expected: &ExpectedHandoffContext,
    actual: &HandoffEvidence,
) -> TrustBoundaryAudit {
    if !expected_is_valid(expected) {
        return TrustBoundaryAudit {
            outcome: AuditOutcome::Inconclusive,
            violations: vec![TrustBoundaryViolation::InvalidExpectedContext],
        };
    }

    let mut violations = Vec::new();
    let mut erased = false;
    let mut substituted = false;

    if !nonempty(&actual.handoff_id) {
        violations.push(TrustBoundaryViolation::HandoffIdMissing);
        erased = true;
    } else if actual.handoff_id != expected.handoff_id {
        violations.push(TrustBoundaryViolation::HandoffIdSubstituted);
        substituted = true;
    }

    match (&expected.parent_handoff_id, &actual.parent_handoff_id) {
        (Some(_), None) => {
            violations.push(TrustBoundaryViolation::ParentHandoffErased);
            erased = true;
        }
        (Some(expected_id), Some(actual_id)) if expected_id != actual_id => {
            violations.push(TrustBoundaryViolation::ParentHandoffSubstituted);
            substituted = true;
        }
        (None, Some(_)) => {
            violations.push(TrustBoundaryViolation::ParentHandoffSubstituted);
            substituted = true;
        }
        _ => {}
    }

    match &actual.authority {
        None => {
            violations.push(TrustBoundaryViolation::AuthorityContextErased);
            erased = true;
        }
        Some(authority) if authority != &expected.authority => {
            violations.push(TrustBoundaryViolation::AuthorityContextSubstituted);
            substituted = true;
        }
        Some(_) => {}
    }

    let expected_sources: BTreeMap<&str, &ExpectedSource> = expected
        .sources
        .iter()
        .map(|source| (source.evidence_id.as_str(), source))
        .collect();
    let mut actual_sources: BTreeMap<&str, &SourceEvidence> = BTreeMap::new();

    for source in &actual.source_evidence {
        if actual_sources.insert(source.evidence_id.as_str(), source).is_some() {
            violations.push(TrustBoundaryViolation::SourceEvidenceDuplicated(
                source.evidence_id.clone(),
            ));
            substituted = true;
        }
        if !expected_sources.contains_key(source.evidence_id.as_str()) {
            violations.push(TrustBoundaryViolation::UnexpectedSourceEvidence(
                source.evidence_id.clone(),
            ));
            substituted = true;
        }
    }

    for (evidence_id, expected_source) in &expected_sources {
        let Some(actual_source) = actual_sources.get(evidence_id).copied() else {
            violations.push(TrustBoundaryViolation::SourceEvidenceErased(
                (*evidence_id).to_string(),
            ));
            erased = true;
            continue;
        };

        if actual_source.source_id != expected_source.source_id {
            violations.push(TrustBoundaryViolation::SourceEvidenceSubstituted(
                (*evidence_id).to_string(),
            ));
            substituted = true;
        }
        if !digest_is_sha256(&actual_source.source_sha256) {
            violations.push(TrustBoundaryViolation::SourceDigestInvalid(
                (*evidence_id).to_string(),
            ));
            substituted = true;
        } else if actual_source.source_sha256 != expected_source.source_sha256 {
            violations.push(TrustBoundaryViolation::SourceEvidenceSubstituted(
                (*evidence_id).to_string(),
            ));
            substituted = true;
        }
        if !nonempty(&actual_source.origin_principal) {
            violations.push(TrustBoundaryViolation::SourceOriginErased(
                (*evidence_id).to_string(),
            ));
            erased = true;
        } else if actual_source.origin_principal != expected_source.origin_principal {
            violations.push(TrustBoundaryViolation::SourceOriginSubstituted(
                (*evidence_id).to_string(),
            ));
            substituted = true;
        }
        if !nonempty(&actual_source.declared_control_domain_id) {
            violations.push(TrustBoundaryViolation::SourceDomainErased(
                (*evidence_id).to_string(),
            ));
            erased = true;
        } else if actual_source.declared_control_domain_id
            != expected_source.declared_control_domain_id
        {
            violations.push(TrustBoundaryViolation::SourceDomainSubstituted(
                (*evidence_id).to_string(),
            ));
            substituted = true;
        }
        if !expected_source
            .required_derived_from
            .is_subset(&actual_source.derived_from)
        {
            violations.push(TrustBoundaryViolation::SourceLineageErased(
                (*evidence_id).to_string(),
            ));
            erased = true;
        }
        if !actual_source
            .derived_from
            .is_subset(&expected_source.allowed_derived_from)
        {
            violations.push(TrustBoundaryViolation::SourceLineageSubstituted(
                (*evidence_id).to_string(),
            ));
            substituted = true;
        }
    }

    if !expected.required_derived_from.is_subset(&actual.derived_from) {
        violations.push(TrustBoundaryViolation::HandoffLineageErased("handoff".into()));
        erased = true;
    }
    if !actual.derived_from.is_subset(&expected.allowed_derived_from) {
        violations.push(TrustBoundaryViolation::HandoffLineageSubstituted(
            "handoff".into(),
        ));
        substituted = true;
    }

    match &actual.transformation {
        None => {
            violations.push(TrustBoundaryViolation::TransformationErased);
            erased = true;
        }
        Some(transformation) => {
            if !nonempty(&transformation.transform_id) {
                violations.push(TrustBoundaryViolation::TransformationIdErased);
                erased = true;
            } else if !expected.expected_transform_ids.contains(&transformation.transform_id) {
                violations.push(TrustBoundaryViolation::TransformationIdSubstituted);
                substituted = true;
            }
            if transformation.input_payload_sha256 != expected.input_payload_sha256 {
                violations.push(TrustBoundaryViolation::TransformationInputSubstituted);
                substituted = true;
            }
            if !digest_is_sha256(&transformation.output_payload_sha256) {
                violations.push(TrustBoundaryViolation::TransformationOutputSubstituted);
                substituted = true;
            } else if transformation.output_payload_sha256 != actual.payload_sha256 {
                violations.push(TrustBoundaryViolation::PayloadBindingSubstituted);
                substituted = true;
            }
            if !expected
                .required_derived_from
                .is_subset(&transformation.derived_from)
            {
                violations.push(TrustBoundaryViolation::HandoffLineageErased(
                    "transformation".into(),
                ));
                erased = true;
            }
            if !transformation
                .derived_from
                .is_subset(&expected.allowed_derived_from)
            {
                violations.push(TrustBoundaryViolation::HandoffLineageSubstituted(
                    "transformation".into(),
                ));
                substituted = true;
            }
        }
    }

    if !nonempty(&actual.payload_sha256) {
        violations.push(TrustBoundaryViolation::PayloadErased);
        erased = true;
    } else if !digest_is_sha256(&actual.payload_sha256) {
        violations.push(TrustBoundaryViolation::PayloadDigestInvalid);
        substituted = true;
    }

    let outcome = if substituted {
        AuditOutcome::Substituted
    } else if erased {
        AuditOutcome::Erased
    } else {
        AuditOutcome::Preserved
    };

    TrustBoundaryAudit { outcome, violations }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(ch: char) -> String {
        std::iter::repeat_n(ch, 64).collect()
    }

    fn set(values: &[&str]) -> BTreeSet<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    fn authority() -> AuthorityLineage {
        AuthorityLineage {
            root_task_id: "root-task".into(),
            root_task_digest: digest('a'),
            root_plan_digest: digest('b'),
            authority_commitment_sha256: digest('c'),
            leaf_task_id: "leaf-task".into(),
            leaf_task_digest: digest('d'),
            leaf_plan_digest: digest('e'),
            leaf_grant_id: "grant-leaf".into(),
            principal_chain: vec!["principal-root".into(), "principal-leaf".into()],
            grant_chain: vec!["grant-root".into(), "grant-leaf".into()],
        }
    }

    fn expected() -> ExpectedHandoffContext {
        ExpectedHandoffContext {
            handoff_id: "handoff-2".into(),
            parent_handoff_id: Some("handoff-1".into()),
            authority: authority(),
            sources: vec![
                ExpectedSource {
                    evidence_id: "evidence-1".into(),
                    source_id: "record-1".into(),
                    source_sha256: digest('f'),
                    origin_principal: "service-principal-1".into(),
                    declared_control_domain_id: "domain-telemetry".into(),
                    required_derived_from: set(&["root-input-1"]),
                    allowed_derived_from: set(&["root-input-1"]),
                },
                ExpectedSource {
                    evidence_id: "evidence-2".into(),
                    source_id: "record-2".into(),
                    source_sha256: digest('g'),
                    origin_principal: "service-principal-2".into(),
                    // Reusing a declared domain is not independently rejected:
                    // this auditor records lineage, it does not certify independence.
                    declared_control_domain_id: "domain-telemetry".into(),
                    required_derived_from: set(&["root-input-2"]),
                    allowed_derived_from: set(&["root-input-2"]),
                },
            ],
            expected_transform_ids: set(&["summarizer-v1"]),
            required_derived_from: set(&["root-input-1", "root-input-2"]),
            allowed_derived_from: set(&["root-input-1", "root-input-2"]),
            input_payload_sha256: digest('h'),
        }
    }

    fn actual() -> HandoffEvidence {
        HandoffEvidence {
            handoff_id: "handoff-2".into(),
            parent_handoff_id: Some("handoff-1".into()),
            authority: Some(authority()),
            source_evidence: vec![
                SourceEvidence {
                    evidence_id: "evidence-1".into(),
                    source_id: "record-1".into(),
                    source_sha256: digest('f'),
                    origin_principal: "service-principal-1".into(),
                    declared_control_domain_id: "domain-telemetry".into(),
                    derived_from: set(&["root-input-1"]),
                },
                SourceEvidence {
                    evidence_id: "evidence-2".into(),
                    source_id: "record-2".into(),
                    source_sha256: digest('g'),
                    origin_principal: "service-principal-2".into(),
                    declared_control_domain_id: "domain-telemetry".into(),
                    derived_from: set(&["root-input-2"]),
                },
            ],
            derived_from: set(&["root-input-1", "root-input-2"]),
            transformation: Some(TransformationEvidence {
                transform_id: "summarizer-v1".into(),
                input_payload_sha256: digest('h'),
                output_payload_sha256: digest('i'),
                derived_from: set(&["root-input-1", "root-input-2"]),
            }),
            payload_sha256: digest('i'),
        }
    }

    #[test]
    fn complete_handoff_provenance_is_preserved() {
        assert_eq!(audit_handoff(&expected(), &actual()).outcome, AuditOutcome::Preserved);
    }

    #[test]
    fn declared_shared_control_domain_is_not_mistaken_for_independence() {
        let result = audit_handoff(&expected(), &actual());
        assert_eq!(result.outcome, AuditOutcome::Preserved);
        assert_eq!(
            expected().sources[0].declared_control_domain_id,
            expected().sources[1].declared_control_domain_id
        );
    }

    #[test]
    fn missing_authority_context_is_erasure() {
        let mut handoff = actual();
        handoff.authority = None;
        assert_eq!(audit_handoff(&expected(), &handoff).outcome, AuditOutcome::Erased);
    }

    #[test]
    fn substituted_authority_lineage_is_detected() {
        let mut handoff = actual();
        handoff.authority.as_mut().unwrap().leaf_grant_id = "other-grant".into();
        assert_eq!(audit_handoff(&expected(), &handoff).outcome, AuditOutcome::Substituted);
    }

    #[test]
    fn missing_parent_handoff_link_is_erasure() {
        let mut handoff = actual();
        handoff.parent_handoff_id = None;
        assert_eq!(audit_handoff(&expected(), &handoff).outcome, AuditOutcome::Erased);
    }

    #[test]
    fn substituted_parent_handoff_link_is_detected() {
        let mut handoff = actual();
        handoff.parent_handoff_id = Some("unrelated-parent".into());
        assert_eq!(audit_handoff(&expected(), &handoff).outcome, AuditOutcome::Substituted);
    }

    #[test]
    fn missing_source_record_is_erasure() {
        let mut handoff = actual();
        handoff.source_evidence.pop();
        assert_eq!(audit_handoff(&expected(), &handoff).outcome, AuditOutcome::Erased);
    }

    #[test]
    fn substituted_source_record_id_is_detected() {
        let mut handoff = actual();
        handoff.source_evidence[0].source_id = "other-record".into();
        assert_eq!(
            audit_handoff(&expected(), &handoff).outcome,
            AuditOutcome::Substituted
        );
    }

    #[test]
    fn substituted_control_domain_reference_is_detected() {
        let mut handoff = actual();
        handoff.source_evidence[0].declared_control_domain_id = "other-domain".into();
        assert_eq!(
            audit_handoff(&expected(), &handoff).outcome,
            AuditOutcome::Substituted
        );
    }

    #[test]
    fn altered_source_digest_is_substitution() {
        let mut handoff = actual();
        handoff.source_evidence[0].source_sha256 = digest('z');
        assert_eq!(audit_handoff(&expected(), &handoff).outcome, AuditOutcome::Substituted);
    }

    #[test]
    fn altered_source_origin_is_substitution() {
        let mut handoff = actual();
        handoff.source_evidence[0].origin_principal = "other-principal".into();
        assert_eq!(audit_handoff(&expected(), &handoff).outcome, AuditOutcome::Substituted);
    }

    #[test]
    fn missing_source_derivation_link_is_erasure() {
        let mut handoff = actual();
        handoff.source_evidence[0].derived_from.clear();
        assert_eq!(audit_handoff(&expected(), &handoff).outcome, AuditOutcome::Erased);
    }

    #[test]
    fn unexpected_derivation_link_is_substitution() {
        let mut handoff = actual();
        handoff.derived_from.insert("unrecognized-source".into());
        assert_eq!(audit_handoff(&expected(), &handoff).outcome, AuditOutcome::Substituted);
    }

    #[test]
    fn missing_transformation_record_is_erasure() {
        let mut handoff = actual();
        handoff.transformation = None;
        assert_eq!(audit_handoff(&expected(), &handoff).outcome, AuditOutcome::Erased);
    }

    #[test]
    fn transformation_identity_must_be_predeclared() {
        let mut handoff = actual();
        handoff.transformation.as_mut().unwrap().transform_id = "unapproved-transform".into();
        assert_eq!(
            audit_handoff(&expected(), &handoff).outcome,
            AuditOutcome::Substituted
        );
    }

    #[test]
    fn transformation_input_must_bind_to_expected_input() {
        let mut handoff = actual();
        handoff.transformation.as_mut().unwrap().input_payload_sha256 = digest('z');
        assert_eq!(audit_handoff(&expected(), &handoff).outcome, AuditOutcome::Substituted);
    }

    #[test]
    fn output_digest_must_bind_to_delivered_payload() {
        let mut handoff = actual();
        handoff.payload_sha256 = digest('z');
        assert_eq!(audit_handoff(&expected(), &handoff).outcome, AuditOutcome::Substituted);
    }

    #[test]
    fn duplicate_source_evidence_is_rejected() {
        let mut handoff = actual();
        handoff.source_evidence.push(handoff.source_evidence[0].clone());
        assert_eq!(audit_handoff(&expected(), &handoff).outcome, AuditOutcome::Substituted);
    }

    #[test]
    fn unexpected_source_evidence_is_rejected() {
        let mut handoff = actual();
        handoff.source_evidence.push(SourceEvidence {
            evidence_id: "unplanned-evidence".into(),
            source_id: "unplanned-record".into(),
            source_sha256: digest('z'),
            origin_principal: "unplanned-principal".into(),
            declared_control_domain_id: "unplanned-domain".into(),
            derived_from: BTreeSet::new(),
        });
        assert_eq!(audit_handoff(&expected(), &handoff).outcome, AuditOutcome::Substituted);
    }

    #[test]
    fn malformed_expected_context_is_inconclusive_not_a_pass() {
        let mut expectation = expected();
        expectation.authority.root_task_digest = "not-a-digest".into();
        assert_eq!(audit_handoff(&expectation, &actual()).outcome, AuditOutcome::Inconclusive);
    }

    #[test]
    fn transformation_id_and_payload_are_required() {
        let mut handoff = actual();
        handoff.transformation.as_mut().unwrap().transform_id.clear();
        assert_eq!(audit_handoff(&expected(), &handoff).outcome, AuditOutcome::Erased);
        handoff = actual();
        handoff.payload_sha256.clear();
        assert_eq!(audit_handoff(&expected(), &handoff).outcome, AuditOutcome::Erased);
    }
}
