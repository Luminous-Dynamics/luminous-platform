//! Typed memory-provenance and authority-separation reference policy.
//!
//! This pure module models how memory provenance affects context use and
//! consequential actions. It is not a memory backend, cryptographic verifier,
//! production policy engine or independent effect observer.
//!
//! Core rule: memory is evidence/context, never authority. Taint survives
//! derivation; invalid or indeterminate provenance blocks use; untrusted but
//! provenance-complete content may remain available as context. A sensitive
//! export influenced by tainted memory requires a separate exact-action
//! approval within the source-anchored root policy.

use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemorySourceKind {
    UserConfirmed,
    System,
    UntrustedTool,
    DerivedSummary,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryTrust {
    UserConfirmed,
    SystemVerified,
    UntrustedExternal,
    Derived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryState {
    Active,
    Revoked,
    Tombstoned,
    Unknown,
}

/// A lineage closure produced only after the trusted memory-store adapter
/// supplies the complete parent index and the corresponding parent records.
///
/// The fields are private so callers cannot construct a witness by copying
/// labels into a struct literal. This is a checked reference-model object, not
/// a cryptographic proof; the adapter must obtain the parent index/records from
/// its authenticated memory store and revalidate the witness on each use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedDerivedLineage {
    derived_id: String,
    derived_content_digest: String,
    tenant: String,
    principal: String,
    authority_epoch: u64,
    parent_bindings: BTreeSet<(String, String)>,
    inherited_taint_labels: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryRecord {
    pub id: String,
    pub content_digest: String,
    pub tenant: String,
    pub principal: String,
    pub source_kind: MemorySourceKind,
    pub source_trust: MemoryTrust,
    pub claimed_trust: MemoryTrust,
    pub provenance_complete: bool,
    pub state: MemoryState,
    pub taint_labels: BTreeSet<String>,
    pub parent_ids: BTreeSet<String>,
    pub parent_taint_labels: BTreeSet<String>,
    pub age_sessions: u32,
    pub revocation_epoch: u64,
    /// Opaque checked lineage closure for DerivedSummary records. Non-derived
    /// records should leave this as None.
    pub lineage: Option<ValidatedDerivedLineage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootMemoryPolicy {
    pub principal: String,
    pub tenant: String,
    /// Actor/workload IDs already admitted by the existing identity and
    /// delegation verifier. This field is not itself proof of identity and
    /// does not create or widen authority.
    pub allowed_actors: BTreeSet<String>,
    pub allowed_operations: BTreeSet<String>,
    pub allowed_scopes: BTreeSet<String>,
    pub allowed_resources: BTreeSet<String>,
    pub allowed_recipients: BTreeSet<String>,
    pub max_memory_age_sessions: u32,
    pub authority_epoch: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectKind {
    None,
    ProtectedMutation,
    SensitiveExport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalState {
    NotRequired,
    Missing,
    Verified,
    Invalid,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservedEffect {
    None,
    Committed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryAction {
    pub actor: String,
    pub tenant: String,
    pub operation: String,
    pub resource: String,
    pub recipient: Option<String>,
    pub effect: EffectKind,
    pub requested_scope: String,
    pub uses_memory_as_authority: bool,
    pub requires_approval: bool,
    pub approval_state: ApprovalState,
    pub approval_digest: Option<String>,
    pub action_digest: String,
    pub observed_effect: ObservedEffect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryClassification {
    Trusted,
    Tainted,
    Invalid,
    Indeterminate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorizationDecision {
    Allow,
    Deny,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryViolation {
    MissingProvenance,
    MemoryIdentifierMissing,
    MemoryDigestMissing,
    ActionDigestMissing,
    RevokedOrTombstoned,
    StatusUnresolved,
    SubjectOrTenantMismatch,
    ActorNotAuthorized,
    MemoryTooOld,
    RevocationEpochStale,
    EmptyMemoryHasProvenance,
    DerivedMemoryMissingParent,
    DerivedLineageWitnessMissing,
    DerivedLineageBindingMismatch,
    DerivedLineageParentSetMismatch,
    DerivedLineageParentInvalid,
    DerivedLineageParentIndeterminate,
    DerivedLineageTaintMismatch,
    ParentTaintDropped,
    SourceTrustMismatch,
    TrustEscalation,
    SourceTrustConflict,
    InvalidMemoryProvenance,
    MemoryProvenanceIndeterminate,
    ActionTenantMismatch,
    OperationOutsideRootPolicy,
    ResourceOutsideRootPolicy,
    RecipientOutsideRootPolicy,
    ScopeOutsideRootPolicy,
    MemoryCannotAuthorize,
    TaintedExportRequiresSeparateApproval,
    RequiredApprovalMissing,
    ApprovalDigestMismatch,
    ApprovalInvalid,
    ApprovalStatusUnknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryEvaluation {
    pub memory_classification: MemoryClassification,
    pub authorization: AuthorizationDecision,
    pub unauthorized_effect_observed: bool,
    pub violations: Vec<MemoryViolation>,
}

fn insert_once(target: &mut Vec<MemoryViolation>, violation: MemoryViolation) {
    if !target.contains(&violation) {
        target.push(violation);
    }
}

/// Classify memory source/provenance without inspecting natural-language
/// content or treating a model confidence score as a trust signal.
pub fn classify_memory(
    policy: &RootMemoryPolicy,
    memory: &MemoryRecord,
) -> (MemoryClassification, Vec<MemoryViolation>) {
    let mut invalid = Vec::new();
    let mut indeterminate = Vec::new();

    if !memory.provenance_complete {
        insert_once(&mut indeterminate, MemoryViolation::MissingProvenance);
    }
    if memory.id.trim().is_empty() {
        insert_once(&mut invalid, MemoryViolation::MemoryIdentifierMissing);
    }
    if memory.content_digest.trim().is_empty() {
        insert_once(&mut invalid, MemoryViolation::MemoryDigestMissing);
    }
    match memory.state {
        MemoryState::Active => {}
        MemoryState::Revoked | MemoryState::Tombstoned => {
            insert_once(&mut invalid, MemoryViolation::RevokedOrTombstoned);
        }
        MemoryState::Unknown => {
            insert_once(&mut indeterminate, MemoryViolation::StatusUnresolved);
        }
    }
    if memory.tenant != policy.tenant || memory.principal != policy.principal {
        insert_once(&mut invalid, MemoryViolation::SubjectOrTenantMismatch);
    }
    if memory.age_sessions > policy.max_memory_age_sessions {
        insert_once(&mut indeterminate, MemoryViolation::MemoryTooOld);
    }
    if memory.revocation_epoch < policy.authority_epoch {
        insert_once(&mut indeterminate, MemoryViolation::RevocationEpochStale);
    }

    if memory.source_kind == MemorySourceKind::None
        && (!memory.parent_ids.is_empty()
            || !memory.taint_labels.is_empty()
            || !memory.parent_taint_labels.is_empty())
    {
        insert_once(&mut invalid, MemoryViolation::EmptyMemoryHasProvenance);
    }
    match memory.source_kind {
        MemorySourceKind::UserConfirmed
            if memory.source_trust != MemoryTrust::UserConfirmed
                || memory.claimed_trust != MemoryTrust::UserConfirmed =>
        {
            insert_once(&mut invalid, MemoryViolation::SourceTrustMismatch);
        }
        MemorySourceKind::System
            if memory.source_trust != MemoryTrust::SystemVerified
                || memory.claimed_trust != MemoryTrust::SystemVerified =>
        {
            insert_once(&mut invalid, MemoryViolation::SourceTrustMismatch);
        }
        MemorySourceKind::UntrustedTool
            if memory.source_trust != MemoryTrust::UntrustedExternal
                || memory.claimed_trust != MemoryTrust::UntrustedExternal =>
        {
            insert_once(&mut invalid, MemoryViolation::SourceTrustMismatch);
        }
        MemorySourceKind::DerivedSummary
            if memory.source_trust != MemoryTrust::Derived
                || memory.claimed_trust != MemoryTrust::Derived =>
        {
            insert_once(&mut invalid, MemoryViolation::SourceTrustMismatch);
        }
        _ => {}
    }

    if memory.source_kind == MemorySourceKind::DerivedSummary {
        if memory.parent_ids.is_empty() {
            insert_once(&mut invalid, MemoryViolation::DerivedMemoryMissingParent);
        }
        match &memory.lineage {
            None => {
                insert_once(&mut invalid, MemoryViolation::DerivedLineageWitnessMissing);
            }
            Some(lineage) => {
                let witnessed_parent_ids: BTreeSet<String> = lineage
                    .parent_bindings
                    .iter()
                    .map(|(parent_id, _)| parent_id.clone())
                    .collect();
                if lineage.derived_id != memory.id
                    || lineage.derived_content_digest != memory.content_digest
                    || lineage.tenant != memory.tenant
                    || lineage.principal != memory.principal
                    || lineage.authority_epoch != policy.authority_epoch
                    || witnessed_parent_ids != memory.parent_ids
                {
                    insert_once(
                        &mut invalid,
                        MemoryViolation::DerivedLineageBindingMismatch,
                    );
                }
                if lineage.inherited_taint_labels != memory.parent_taint_labels {
                    insert_once(
                        &mut invalid,
                        MemoryViolation::DerivedLineageTaintMismatch,
                    );
                }
                if !lineage.inherited_taint_labels.is_subset(&memory.taint_labels) {
                    insert_once(&mut invalid, MemoryViolation::ParentTaintDropped);
                }
            }
        }
    } else if memory.lineage.is_some() {
        insert_once(&mut invalid, MemoryViolation::DerivedLineageBindingMismatch);
    }

    let source_is_untrusted = memory.source_kind == MemorySourceKind::UntrustedTool
        || memory.source_trust == MemoryTrust::UntrustedExternal;
    let claims_trusted = matches!(
        memory.claimed_trust,
        MemoryTrust::UserConfirmed | MemoryTrust::SystemVerified
    );
    let has_taint = source_is_untrusted || !memory.taint_labels.is_empty();
    if has_taint && claims_trusted {
        insert_once(&mut invalid, MemoryViolation::TrustEscalation);
    }
    if memory.source_kind == MemorySourceKind::UntrustedTool
        && memory.source_trust == MemoryTrust::SystemVerified
    {
        insert_once(&mut invalid, MemoryViolation::SourceTrustConflict);
    }

    if !invalid.is_empty() {
        let mut all = invalid;
        for item in indeterminate {
            insert_once(&mut all, item);
        }
        return (MemoryClassification::Invalid, all);
    }
    if !indeterminate.is_empty() {
        return (MemoryClassification::Indeterminate, indeterminate);
    }

    if source_is_untrusted
        || !memory.taint_labels.is_empty()
        || memory.source_trust == MemoryTrust::Derived
    {
        (MemoryClassification::Tainted, Vec::new())
    } else {
        (MemoryClassification::Trusted, Vec::new())
    }
}

/// Validate a derived summary against the complete parent index and the
/// corresponding parent records supplied by the authenticated memory-store
/// adapter. The resulting opaque witness binds the child, exact parent IDs and
/// digests, tenant/principal, current authority epoch, and inherited taint.
pub fn validate_derived_memory_lineage(
    policy: &RootMemoryPolicy,
    derived: &MemoryRecord,
    expected_parent_ids: &BTreeSet<String>,
    parents: &[MemoryRecord],
) -> Result<ValidatedDerivedLineage, Vec<MemoryViolation>> {
    let mut invalid = Vec::new();
    let mut indeterminate = Vec::new();

    if derived.source_kind != MemorySourceKind::DerivedSummary {
        insert_once(&mut invalid, MemoryViolation::DerivedLineageBindingMismatch);
    }
    if expected_parent_ids.is_empty() {
        insert_once(&mut invalid, MemoryViolation::DerivedMemoryMissingParent);
    }
    if derived.parent_ids != *expected_parent_ids {
        insert_once(&mut invalid, MemoryViolation::DerivedLineageParentSetMismatch);
    }
    if parents.len() != expected_parent_ids.len() {
        insert_once(&mut invalid, MemoryViolation::DerivedLineageParentSetMismatch);
    }

    let mut actual_parent_ids = BTreeSet::new();
    let mut parent_bindings = BTreeSet::new();
    let mut inherited_taint_labels = BTreeSet::new();

    for parent in parents {
        if !actual_parent_ids.insert(parent.id.clone()) {
            insert_once(&mut invalid, MemoryViolation::DerivedLineageParentSetMismatch);
        }
        if parent.id.trim().is_empty() || parent.content_digest.trim().is_empty() {
            insert_once(&mut invalid, MemoryViolation::DerivedLineageParentInvalid);
            continue;
        }
        if parent.tenant != derived.tenant
            || parent.principal != derived.principal
            || parent.tenant != policy.tenant
            || parent.principal != policy.principal
        {
            insert_once(&mut invalid, MemoryViolation::SubjectOrTenantMismatch);
        }

        let (classification, _) = classify_memory(policy, parent);
        match classification {
            MemoryClassification::Invalid => {
                insert_once(&mut invalid, MemoryViolation::DerivedLineageParentInvalid);
            }
            MemoryClassification::Indeterminate => {
                insert_once(
                    &mut indeterminate,
                    MemoryViolation::DerivedLineageParentIndeterminate,
                );
            }
            MemoryClassification::Trusted | MemoryClassification::Tainted => {}
        }

        inherited_taint_labels.extend(parent.taint_labels.iter().cloned());
        if classification == MemoryClassification::Tainted && parent.taint_labels.is_empty() {
            // Preserve tainted ancestry even if an upstream source has no
            // finer-grained label vocabulary.
            inherited_taint_labels.insert("provenance:tainted-parent".into());
        }
        parent_bindings.insert((parent.id.clone(), parent.content_digest.clone()));
    }

    if actual_parent_ids != *expected_parent_ids {
        insert_once(&mut invalid, MemoryViolation::DerivedLineageParentSetMismatch);
    }
    if derived.parent_taint_labels != inherited_taint_labels {
        insert_once(&mut invalid, MemoryViolation::DerivedLineageTaintMismatch);
    }
    if !inherited_taint_labels.is_subset(&derived.taint_labels) {
        insert_once(&mut invalid, MemoryViolation::ParentTaintDropped);
    }
    if derived.id.trim().is_empty() || derived.content_digest.trim().is_empty() {
        insert_once(&mut invalid, MemoryViolation::MemoryIdentifierMissing);
    }
    if derived.tenant != policy.tenant || derived.principal != policy.principal {
        insert_once(&mut invalid, MemoryViolation::SubjectOrTenantMismatch);
    }

    if !invalid.is_empty() || !indeterminate.is_empty() {
        let mut violations = invalid;
        for item in indeterminate {
            insert_once(&mut violations, item);
        }
        return Err(violations);
    }

    Ok(ValidatedDerivedLineage {
        derived_id: derived.id.clone(),
        derived_content_digest: derived.content_digest.clone(),
        tenant: derived.tenant.clone(),
        principal: derived.principal.clone(),
        authority_epoch: policy.authority_epoch,
        parent_bindings,
        inherited_taint_labels,
    })
}

/// Apply root-authority and independent-approval checks to an exact action.
/// The supplied digest is opaque and must be created/verified by the existing
/// Act-Bound owner; this module never canonicalizes or hashes requests.
pub fn authorize_memory_influenced_action(
    policy: &RootMemoryPolicy,
    memory: &MemoryRecord,
    action: &MemoryAction,
    classification: MemoryClassification,
) -> (AuthorizationDecision, Vec<MemoryViolation>) {
    let mut violations = Vec::new();

    if classification == MemoryClassification::Invalid {
        insert_once(
            &mut violations,
            MemoryViolation::InvalidMemoryProvenance,
        );
    }
    if classification == MemoryClassification::Indeterminate {
        insert_once(
            &mut violations,
            MemoryViolation::MemoryProvenanceIndeterminate,
        );
    }
    if action.tenant != policy.tenant {
        insert_once(&mut violations, MemoryViolation::ActionTenantMismatch);
    }
    if action.actor.trim().is_empty() || !policy.allowed_actors.contains(&action.actor) {
        insert_once(&mut violations, MemoryViolation::ActorNotAuthorized);
    }
    if action.action_digest.trim().is_empty() {
        insert_once(&mut violations, MemoryViolation::ActionDigestMissing);
    }
    if action.operation.trim().is_empty()
        || !policy.allowed_operations.contains(&action.operation)
    {
        insert_once(
            &mut violations,
            MemoryViolation::OperationOutsideRootPolicy,
        );
    }
    if action.resource.trim().is_empty()
        || !policy.allowed_resources.contains(&action.resource)
    {
        insert_once(
            &mut violations,
            MemoryViolation::ResourceOutsideRootPolicy,
        );
    }
    if !policy.allowed_scopes.contains(&action.requested_scope) {
        insert_once(&mut violations, MemoryViolation::ScopeOutsideRootPolicy);
    }
    if let Some(recipient) = &action.recipient {
        if !policy.allowed_recipients.contains(recipient) {
            insert_once(
                &mut violations,
                MemoryViolation::RecipientOutsideRootPolicy,
            );
        }
    }
    if action.uses_memory_as_authority {
        insert_once(&mut violations, MemoryViolation::MemoryCannotAuthorize);
    }
    if memory.tenant != policy.tenant || memory.principal != policy.principal {
        insert_once(
            &mut violations,
            MemoryViolation::SubjectOrTenantMismatch,
        );
    }

    let tainted_sensitive_export =
        classification == MemoryClassification::Tainted
            && action.effect == EffectKind::SensitiveExport;
    if tainted_sensitive_export && action.approval_state != ApprovalState::Verified {
        insert_once(
            &mut violations,
            MemoryViolation::TaintedExportRequiresSeparateApproval,
        );
    }
    if action.requires_approval && action.approval_state != ApprovalState::Verified {
        insert_once(
            &mut violations,
            MemoryViolation::RequiredApprovalMissing,
        );
    }
    if action.approval_state == ApprovalState::Verified
        && action.approval_digest.as_deref() != Some(action.action_digest.as_str())
    {
        insert_once(&mut violations, MemoryViolation::ApprovalDigestMismatch);
    }
    if action.approval_state == ApprovalState::Invalid {
        insert_once(&mut violations, MemoryViolation::ApprovalInvalid);
    }
    if action.approval_state == ApprovalState::Unknown {
        insert_once(&mut violations, MemoryViolation::ApprovalStatusUnknown);
    }

    if violations.is_empty() {
        (AuthorizationDecision::Allow, violations)
    } else {
        (AuthorizationDecision::Deny, violations)
    }
}

/// Evaluate memory provenance, action authorization and an independently
/// supplied observation of whether an effect committed. The observation is
/// simply a typed input here; this function cannot prove its independence.
pub fn evaluate_memory_action(
    policy: &RootMemoryPolicy,
    memory: &MemoryRecord,
    action: &MemoryAction,
) -> MemoryEvaluation {
    let (classification, mut violations) = classify_memory(policy, memory);
    let (authorization, auth_violations) =
        authorize_memory_influenced_action(policy, memory, action, classification);
    for item in auth_violations {
        insert_once(&mut violations, item);
    }
    MemoryEvaluation {
        memory_classification: classification,
        authorization,
        unauthorized_effect_observed: authorization == AuthorizationDecision::Deny
            && action.observed_effect == ObservedEffect::Committed,
        violations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(items: &[&str]) -> BTreeSet<String> {
        items.iter().map(|item| (*item).to_owned()).collect()
    }

    fn policy() -> RootMemoryPolicy {
        RootMemoryPolicy {
            principal: "principal-7".into(),
            tenant: "tenant-a".into(),
            allowed_actors: set(&["agent-7"]),
            allowed_operations: set(&["read", "lookup", "send"]),
            allowed_scopes: set(&["inbox:read", "account:export"]),
            allowed_resources: set(&["inbox", "account-4"]),
            allowed_recipients: set(&["trusted-recipient"]),
            max_memory_age_sessions: 100,
            authority_epoch: 4,
        }
    }

    fn clean_memory() -> MemoryRecord {
        MemoryRecord {
            id: "mem-1".into(),
            content_digest: "sha256:memory-1".into(),
            tenant: "tenant-a".into(),
            principal: "principal-7".into(),
            source_kind: MemorySourceKind::UserConfirmed,
            source_trust: MemoryTrust::UserConfirmed,
            claimed_trust: MemoryTrust::UserConfirmed,
            provenance_complete: true,
            state: MemoryState::Active,
            taint_labels: BTreeSet::new(),
            parent_ids: BTreeSet::new(),
            parent_taint_labels: BTreeSet::new(),
            age_sessions: 3,
            revocation_epoch: 4,
            lineage: None,
        }
    }

    fn untrusted_memory() -> MemoryRecord {
        let mut memory = clean_memory();
        memory.source_kind = MemorySourceKind::UntrustedTool;
        memory.source_trust = MemoryTrust::UntrustedExternal;
        memory.claimed_trust = MemoryTrust::UntrustedExternal;
        memory.taint_labels = set(&["external:email"]);
        memory
    }

    fn read_action() -> MemoryAction {
        MemoryAction {
            actor: "agent-7".into(),
            tenant: "tenant-a".into(),
            operation: "read".into(),
            resource: "inbox".into(),
            recipient: None,
            effect: EffectKind::None,
            requested_scope: "inbox:read".into(),
            uses_memory_as_authority: false,
            requires_approval: false,
            approval_state: ApprovalState::NotRequired,
            approval_digest: None,
            action_digest: "act:read-inbox-1".into(),
            observed_effect: ObservedEffect::None,
        }
    }

    fn export_action() -> MemoryAction {
        MemoryAction {
            actor: "agent-7".into(),
            tenant: "tenant-a".into(),
            operation: "send".into(),
            resource: "account-4".into(),
            recipient: Some("attacker".into()),
            effect: EffectKind::SensitiveExport,
            requested_scope: "account:export".into(),
            uses_memory_as_authority: false,
            requires_approval: true,
            approval_state: ApprovalState::Missing,
            approval_digest: None,
            action_digest: "act:export-44".into(),
            observed_effect: ObservedEffect::None,
        }
    }

    #[test]
    fn clean_user_confirmed_memory_allows_in_scope_read() {
        let result = evaluate_memory_action(&policy(), &clean_memory(), &read_action());
        assert_eq!(result.memory_classification, MemoryClassification::Trusted);
        assert_eq!(result.authorization, AuthorizationDecision::Allow);
        assert!(!result.unauthorized_effect_observed);
    }

    #[test]
    fn untrusted_but_complete_memory_remains_readable_as_context() {
        let result = evaluate_memory_action(&policy(), &untrusted_memory(), &read_action());
        assert_eq!(result.memory_classification, MemoryClassification::Tainted);
        assert_eq!(result.authorization, AuthorizationDecision::Allow);
    }

    #[test]
    fn hundred_session_dormant_memory_remains_tainted() {
        let mut memory = untrusted_memory();
        memory.age_sessions = 100;
        let result = evaluate_memory_action(&policy(), &memory, &export_action());
        assert_eq!(result.memory_classification, MemoryClassification::Tainted);
        assert_eq!(result.authorization, AuthorizationDecision::Deny);
    }

    #[test]
    fn derived_summary_must_inherit_all_parent_taints() {
        let mut memory = clean_memory();
        memory.source_kind = MemorySourceKind::DerivedSummary;
        memory.source_trust = MemoryTrust::Derived;
        memory.claimed_trust = MemoryTrust::Derived;
        memory.parent_ids = set(&["mem-parent"]);
        memory.parent_taint_labels = set(&["external:email", "pii:finance"]);
        memory.taint_labels = set(&["external:email"]);
        let result = evaluate_memory_action(&policy(), &memory, &read_action());
        assert_eq!(result.memory_classification, MemoryClassification::Invalid);
        assert_eq!(result.authorization, AuthorizationDecision::Deny);
        assert!(result.violations.contains(&MemoryViolation::ParentTaintDropped));
    }

    #[test]
    fn untrusted_source_cannot_claim_system_verified_trust() {
        let mut memory = untrusted_memory();
        memory.claimed_trust = MemoryTrust::SystemVerified;
        let result = evaluate_memory_action(&policy(), &memory, &read_action());
        assert_eq!(result.memory_classification, MemoryClassification::Invalid);
        assert_eq!(result.authorization, AuthorizationDecision::Deny);
    }

    #[test]
    fn cross_tenant_memory_is_rejected() {
        let mut memory = clean_memory();
        memory.tenant = "tenant-b".into();
        let result = evaluate_memory_action(&policy(), &memory, &read_action());
        assert_eq!(result.memory_classification, MemoryClassification::Invalid);
        assert_eq!(result.authorization, AuthorizationDecision::Deny);
    }

    #[test]
    fn tombstoned_memory_cannot_be_resurrected() {
        let mut memory = untrusted_memory();
        memory.state = MemoryState::Tombstoned;
        let result = evaluate_memory_action(&policy(), &memory, &read_action());
        assert_eq!(result.memory_classification, MemoryClassification::Invalid);
        assert_eq!(result.authorization, AuthorizationDecision::Deny);
    }

    #[test]
    fn unknown_status_and_stale_epoch_are_indeterminate_and_denied() {
        let mut memory = untrusted_memory();
        memory.state = MemoryState::Unknown;
        memory.revocation_epoch = 3;
        let result = evaluate_memory_action(&policy(), &memory, &read_action());
        assert_eq!(result.memory_classification, MemoryClassification::Indeterminate);
        assert_eq!(result.authorization, AuthorizationDecision::Deny);
    }

    #[test]
    fn exact_separate_approval_can_authorize_in_scope_tainted_export() {
        let mut memory = untrusted_memory();
        memory.taint_labels = set(&["external:email"]);
        let mut action = export_action();
        action.recipient = Some("trusted-recipient".into());
        action.approval_state = ApprovalState::Verified;
        action.approval_digest = Some(action.action_digest.clone());
        let result = evaluate_memory_action(&policy(), &memory, &action);
        assert_eq!(result.memory_classification, MemoryClassification::Tainted);
        assert_eq!(result.authorization, AuthorizationDecision::Allow);
    }

    #[test]
    fn wrong_approval_digest_cannot_authorize_export() {
        let mut memory = untrusted_memory();
        let mut action = export_action();
        action.recipient = Some("trusted-recipient".into());
        action.approval_state = ApprovalState::Verified;
        action.approval_digest = Some("different-act".into());
        let result = evaluate_memory_action(&policy(), &memory, &action);
        assert_eq!(result.authorization, AuthorizationDecision::Deny);
        assert!(result.violations.contains(&MemoryViolation::ApprovalDigestMismatch));
    }

    #[test]
    fn memory_claim_cannot_stand_in_for_consent() {
        let memory = untrusted_memory();
        let mut action = read_action();
        action.operation = "send".into();
        action.resource = "account-4".into();
        action.recipient = Some("trusted-recipient".into());
        action.effect = EffectKind::SensitiveExport;
        action.requested_scope = "account:export".into();
        action.uses_memory_as_authority = true;
        action.requires_approval = true;
        let result = evaluate_memory_action(&policy(), &memory, &action);
        assert_eq!(result.authorization, AuthorizationDecision::Deny);
        assert!(result.violations.contains(&MemoryViolation::MemoryCannotAuthorize));
    }

    #[test]
    fn observed_unauthorized_export_is_reported_as_effect_not_just_attempt() {
        let memory = untrusted_memory();
        let mut action = export_action();
        action.observed_effect = ObservedEffect::Committed;
        let result = evaluate_memory_action(&policy(), &memory, &action);
        assert_eq!(result.authorization, AuthorizationDecision::Deny);
        assert!(result.unauthorized_effect_observed);
    }

    #[test]
    fn no_memory_control_preserves_benign_read_utility() {
        let mut memory = clean_memory();
        memory.id = "none".into();
        memory.content_digest = "none".into();
        memory.source_kind = MemorySourceKind::None;
        memory.source_trust = MemoryTrust::SystemVerified;
        memory.claimed_trust = MemoryTrust::SystemVerified;
        let result = evaluate_memory_action(&policy(), &memory, &read_action());
        assert_eq!(result.memory_classification, MemoryClassification::Trusted);
        assert_eq!(result.authorization, AuthorizationDecision::Allow);
    }

    #[test]
    fn action_outside_root_scope_is_denied_even_with_trusted_memory() {
        let memory = clean_memory();
        let mut action = read_action();
        action.requested_scope = "account:delete".into();
        let result = evaluate_memory_action(&policy(), &memory, &action);
        assert_eq!(result.authorization, AuthorizationDecision::Deny);
        assert!(result.violations.contains(&MemoryViolation::ScopeOutsideRootPolicy));
    }
    #[test]
    fn derived_memory_cannot_upgrade_inherited_taint_to_user_confirmed() {
        let mut memory = clean_memory();
        memory.source_kind = MemorySourceKind::DerivedSummary;
        memory.source_trust = MemoryTrust::Derived;
        memory.claimed_trust = MemoryTrust::SystemVerified;
        memory.parent_ids = set(&["mem-parent"]);
        memory.parent_taint_labels = set(&["external:email"]);
        memory.taint_labels = set(&["external:email"]);
        let result = evaluate_memory_action(&policy(), &memory, &read_action());
        assert_eq!(result.memory_classification, MemoryClassification::Invalid);
        assert!(result.violations.contains(&MemoryViolation::TrustEscalation));
        assert_eq!(result.authorization, AuthorizationDecision::Deny);
    }

    #[test]
    fn actor_outside_root_authorized_workloads_is_denied() {
        let mut action = read_action();
        action.actor = "agent-attacker".into();
        let result = evaluate_memory_action(&policy(), &clean_memory(), &action);
        assert_eq!(result.authorization, AuthorizationDecision::Deny);
        assert!(result.violations.contains(&MemoryViolation::ActorNotAuthorized));
    }

    #[test]
    fn derived_summary_preserves_checked_parent_taint_but_remains_context_usable() {
        let policy = policy();
        let parent = untrusted_memory();
        let mut parent = parent;
        parent.id = "mem-parent".into();
        parent.content_digest = "sha256:parent-1".into();

        let mut memory = clean_memory();
        memory.id = "mem-derived".into();
        memory.content_digest = "sha256:derived-1".into();
        memory.source_kind = MemorySourceKind::DerivedSummary;
        memory.source_trust = MemoryTrust::Derived;
        memory.claimed_trust = MemoryTrust::Derived;
        memory.parent_ids = set(&["mem-parent"]);
        memory.parent_taint_labels = parent.taint_labels.clone();
        memory.taint_labels = parent.taint_labels.clone();

        let expected_parent_ids = set(&["mem-parent"]);
        memory.lineage = Some(
            validate_derived_memory_lineage(
                &policy,
                &memory,
                &expected_parent_ids,
                &[parent],
            )
            .expect("complete authenticated parent set should produce a checked witness"),
        );

        let result = evaluate_memory_action(&policy, &memory, &read_action());
        assert_eq!(result.memory_classification, MemoryClassification::Tainted);
        assert_eq!(result.authorization, AuthorizationDecision::Allow);
    }

    #[test]
    fn memory_older_than_profile_limit_is_indeterminate_and_denied() {
        let mut memory = untrusted_memory();
        memory.age_sessions = 101;
        let result = evaluate_memory_action(&policy(), &memory, &read_action());
        assert_eq!(result.memory_classification, MemoryClassification::Indeterminate);
        assert_eq!(result.authorization, AuthorizationDecision::Deny);
        assert!(result.violations.contains(&MemoryViolation::MemoryTooOld));
    }

    #[test]
    fn missing_memory_identity_or_content_digest_is_invalid() {
        let mut memory = clean_memory();
        memory.content_digest.clear();
        let result = evaluate_memory_action(&policy(), &memory, &read_action());
        assert_eq!(result.memory_classification, MemoryClassification::Invalid);
        assert_eq!(result.authorization, AuthorizationDecision::Deny);
        assert!(result.violations.contains(&MemoryViolation::MemoryDigestMissing));
    }

    #[test]
    fn verified_approval_cannot_match_an_empty_action_digest() {
        let memory = untrusted_memory();
        let mut action = export_action();
        action.recipient = Some("trusted-recipient".into());
        action.action_digest.clear();
        action.approval_state = ApprovalState::Verified;
        action.approval_digest = Some(String::new());
        let result = evaluate_memory_action(&policy(), &memory, &action);
        assert_eq!(result.authorization, AuthorizationDecision::Deny);
        assert!(result.violations.contains(&MemoryViolation::ActionDigestMissing));
    }
 
    #[test]
    fn derived_summary_without_checked_lineage_witness_is_rejected() {
        let mut memory = clean_memory();
        memory.source_kind = MemorySourceKind::DerivedSummary;
        memory.source_trust = MemoryTrust::Derived;
        memory.claimed_trust = MemoryTrust::Derived;
        memory.parent_ids = set(&["mem-parent"]);
        let result = evaluate_memory_action(&policy(), &memory, &read_action());
        assert_eq!(result.memory_classification, MemoryClassification::Invalid);
        assert_eq!(result.authorization, AuthorizationDecision::Deny);
        assert!(result
            .violations
            .contains(&MemoryViolation::DerivedLineageWitnessMissing));
    }

    #[test]
    fn derived_lineage_requires_exact_parent_index_and_current_epoch() {
        let policy = policy();
        let mut parent = untrusted_memory();
        parent.id = "mem-parent".into();
        parent.content_digest = "sha256:parent-1".into();

        let mut derived = clean_memory();
        derived.id = "mem-derived".into();
        derived.content_digest = "sha256:derived-1".into();
        derived.source_kind = MemorySourceKind::DerivedSummary;
        derived.source_trust = MemoryTrust::Derived;
        derived.claimed_trust = MemoryTrust::Derived;
        derived.parent_ids = set(&["mem-parent"]);
        derived.parent_taint_labels = parent.taint_labels.clone();
        derived.taint_labels = parent.taint_labels.clone();

        let incomplete_index = BTreeSet::new();
        assert!(validate_derived_memory_lineage(
            &policy,
            &derived,
            &incomplete_index,
            std::slice::from_ref(&parent),
        )
        .is_err());

        let expected = set(&["mem-parent"]);
        derived.lineage = Some(
            validate_derived_memory_lineage(
                &policy,
                &derived,
                &expected,
                &[parent],
            )
            .expect("exact parent index should validate"),
        );
        let mut changed_policy = policy.clone();
        changed_policy.authority_epoch += 1;
        let result = evaluate_memory_action(&changed_policy, &derived, &read_action());
        assert_eq!(result.memory_classification, MemoryClassification::Invalid);
        assert!(result
            .violations
            .contains(&MemoryViolation::DerivedLineageBindingMismatch));
    }

    #[test]
    fn source_kind_cannot_claim_a_different_trust_class() {
        let mut memory = clean_memory();
        memory.source_kind = MemorySourceKind::System;
        memory.source_trust = MemoryTrust::UserConfirmed;
        memory.claimed_trust = MemoryTrust::UserConfirmed;
        let result = evaluate_memory_action(&policy(), &memory, &read_action());
        assert_eq!(result.memory_classification, MemoryClassification::Invalid);
        assert_eq!(result.authorization, AuthorizationDecision::Deny);
        assert!(result
            .violations
            .contains(&MemoryViolation::SourceTrustMismatch));
    }

}
