//! Fail-closed model of monotonic authority attenuation for synthetic ITOPS tests.
//!
//! This module checks whether a delegated child grant is a semantic subset of
//! its verified parent. It does not issue, sign, authenticate, or revoke grants.
//! Production identity, consent, authorization, and revocation remain owned by
//! Xenia; the final participant must independently recheck current authority.

use std::collections::{BTreeMap, BTreeSet};
use serde::Deserialize;

/// A deliberately small constraint language with decidable subset checks.
/// Unsupported or incomparable constraint forms are rejected, never guessed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParameterBound {
    ExactString(String),
    ExactInteger(i64),
    IntegerRange { min: i64, max: i64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DelegatedAuthorityGrant {
    pub grant_id: String,
    /// Principal that issued this grant. For a child this must be the parent's subject.
    pub issuer_principal: String,
    /// Principal receiving this grant.
    pub subject_principal: String,

    /// Immutable root task identity carried through every delegation hop.
    pub root_task_id: String,
    pub root_task_digest: String,
    pub root_plan_digest: String,

    /// This grant's exact task and plan identity.
    pub task_id: String,
    pub task_digest: String,
    pub plan_digest: String,

    /// Exact immediate-parent linkage. Root grants have all three fields absent.
    pub parent_task_id: Option<String>,
    pub parent_task_digest: Option<String>,
    pub parent_grant_id: Option<String>,

    pub tenant_binding: String,
    pub targets: BTreeSet<String>,
    pub operations: BTreeSet<String>,
    pub parameter_bounds: BTreeMap<String, ParameterBound>,
    pub data_scopes: BTreeSet<String>,

    pub not_before_unix: i64,
    pub expires_at_unix: i64,
    pub max_dispatches: u64,
    pub max_tool_calls: u64,
    pub max_resource_units: u64,
    pub remaining_delegation_depth: u32,
    /// Lower values are less privileged. The scale is benchmark-defined, not universal.
    pub max_risk_level: u8,

    /// Child grants may add obligations but may not remove inherited obligations.
    pub required_approvals: BTreeSet<String>,
    pub required_postconditions: BTreeSet<String>,
    pub rollback_obligations: BTreeSet<String>,

    /// The parent explicitly commits to the exact child task digests it may delegate.
    pub allowed_child_task_digests: BTreeSet<String>,
    pub allowed_child_plan_digests: BTreeSet<String>,
    pub delegable: bool,
    pub revoked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttenuationViolation {
    InvalidParentShape,
    InvalidChildShape,
    ParentCannotDelegate,
    ParentRevoked,
    ChildRevoked,
    ParentNotYetValid,
    ParentExpired,
    ChildNotYetValid,
    ChildExpired,
    RootTaskChanged,
    RootTaskDigestChanged,
    RootPlanChanged,
    ParentTaskLinkMismatch,
    ParentTaskDigestMismatch,
    ParentGrantLinkMismatch,
    PrincipalChainMismatch,
    ChildTaskIdNotDistinct,
    ChildTaskDigestNotAllowed,
    ChildPlanDigestNotAllowed,
    InvalidChainLength,
    DuplicateGrantInChain,
    DuplicateTaskInChain,
    TenantChanged,
    TargetScopeExpanded,
    OperationScopeExpanded,
    ParameterScopeExpanded(String),
    DataScopeExpanded,
    NotBeforeWidened,
    ExpiryWidened,
    DispatchBudgetExpanded,
    ToolBudgetExpanded,
    ResourceBudgetExpanded,
    DelegationDepthExpanded,
    RiskCeilingExpanded,
    ApprovalRequirementWeakened,
    PostconditionWeakened,
    RollbackObligationWeakened,
    ChildDelegationDepthInvalid,
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

fn set_is_valid(values: &BTreeSet<String>) -> bool {
    !values.is_empty() && values.iter().all(|value| nonempty(value))
}

fn obligation_set_is_valid(values: &BTreeSet<String>) -> bool {
    values.iter().all(|value| nonempty(value))
}

fn bounds_are_valid(bounds: &BTreeMap<String, ParameterBound>) -> bool {
    bounds.iter().all(|(name, bound)| {
        if !nonempty(name) {
            return false;
        }
        match bound {
            ParameterBound::ExactString(value) => nonempty(value),
            ParameterBound::ExactInteger(_) => true,
            ParameterBound::IntegerRange { min, max } => min <= max,
        }
    })
}

fn grant_shape_is_valid(grant: &DelegatedAuthorityGrant) -> bool {
    nonempty(&grant.grant_id)
        && nonempty(&grant.issuer_principal)
        && nonempty(&grant.subject_principal)
        && grant.issuer_principal != grant.subject_principal
        && nonempty(&grant.root_task_id)
        && digest_is_sha256(&grant.root_task_digest)
        && digest_is_sha256(&grant.root_plan_digest)
        && nonempty(&grant.task_id)
        && digest_is_sha256(&grant.task_digest)
        && digest_is_sha256(&grant.plan_digest)
        && nonempty(&grant.tenant_binding)
        && set_is_valid(&grant.targets)
        && set_is_valid(&grant.operations)
        && set_is_valid(&grant.data_scopes)
        && bounds_are_valid(&grant.parameter_bounds)
        && grant.not_before_unix < grant.expires_at_unix
        && grant.max_dispatches >= 0
        && grant.max_tool_calls > 0
        && grant.max_resource_units > 0
        && obligation_set_is_valid(&grant.required_approvals)
        && obligation_set_is_valid(&grant.required_postconditions)
        && obligation_set_is_valid(&grant.rollback_obligations)
        && grant.allowed_child_task_digests.iter().all(|digest| digest_is_sha256(digest))
        && grant.allowed_child_plan_digests.iter().all(|digest| digest_is_sha256(digest))
        && (grant.delegable || grant.remaining_delegation_depth == 0)
        && match (
            &grant.parent_task_id,
            &grant.parent_task_digest,
            &grant.parent_grant_id,
        ) {
            (None, None, None) => {
                grant.task_id == grant.root_task_id
                    && grant.task_digest == grant.root_task_digest
                    && grant.plan_digest == grant.root_plan_digest
            }
            (Some(task), Some(digest), Some(parent_grant)) => {
                nonempty(task) && digest_is_sha256(digest) && nonempty(parent_grant)
            }
            _ => false,
        }
}

/// Checks the parent-to-child attenuation relation without asserting that either
/// grant is currently active. Every comparison is explicit; incomparable
/// parameter constraints fail closed.
pub fn check_attenuation(
    parent: &DelegatedAuthorityGrant,
    child: &DelegatedAuthorityGrant,
) -> Result<(), Vec<AttenuationViolation>> {
    let mut violations = Vec::new();

    if !grant_shape_is_valid(parent) {
        violations.push(AttenuationViolation::InvalidParentShape);
    }
    if !grant_shape_is_valid(child) {
        violations.push(AttenuationViolation::InvalidChildShape);
    }
    if !parent.delegable || parent.remaining_delegation_depth == 0 {
        violations.push(AttenuationViolation::ParentCannotDelegate);
    }

    if child.root_task_id != parent.root_task_id {
        violations.push(AttenuationViolation::RootTaskChanged);
    }
    if child.root_task_digest != parent.root_task_digest {
        violations.push(AttenuationViolation::RootTaskDigestChanged);
    }
    if child.root_plan_digest != parent.root_plan_digest {
        violations.push(AttenuationViolation::RootPlanChanged);
    }
    if child.parent_task_id.as_deref() != Some(parent.task_id.as_str()) {
        violations.push(AttenuationViolation::ParentTaskLinkMismatch);
    }
    if child.parent_task_digest.as_deref() != Some(parent.task_digest.as_str()) {
        violations.push(AttenuationViolation::ParentTaskDigestMismatch);
    }
    if child.parent_grant_id.as_deref() != Some(parent.grant_id.as_str()) {
        violations.push(AttenuationViolation::ParentGrantLinkMismatch);
    }
    if child.issuer_principal != parent.subject_principal {
        violations.push(AttenuationViolation::PrincipalChainMismatch);
    }
    if child.task_id == parent.task_id {
        violations.push(AttenuationViolation::ChildTaskIdNotDistinct);
    }
    if !parent.allowed_child_task_digests.contains(&child.task_digest) {
        violations.push(AttenuationViolation::ChildTaskDigestNotAllowed);
    }
    if !parent.allowed_child_plan_digests.contains(&child.plan_digest) {
        violations.push(AttenuationViolation::ChildPlanDigestNotAllowed);
    }
    if child.tenant_binding != parent.tenant_binding {
        violations.push(AttenuationViolation::TenantChanged);
    }

    if !child.targets.is_subset(&parent.targets) {
        violations.push(AttenuationViolation::TargetScopeExpanded);
    }
    if !child.operations.is_subset(&parent.operations) {
        violations.push(AttenuationViolation::OperationScopeExpanded);
    }
    if !child.data_scopes.is_subset(&parent.data_scopes) {
        violations.push(AttenuationViolation::DataScopeExpanded);
    }

    // Every parent parameter restriction must remain present in the child.
    // The child may add further restrictions, but may not omit or widen one.
    for (name, parent_bound) in &parent.parameter_bounds {
        let Some(child_bound) = child.parameter_bounds.get(name) else {
            violations.push(AttenuationViolation::ParameterScopeExpanded(name.clone()));
            continue;
        };
        if !parameter_bound_is_subset(child_bound, parent_bound) {
            violations.push(AttenuationViolation::ParameterScopeExpanded(name.clone()));
        }
    }

    if child.not_before_unix < parent.not_before_unix {
        violations.push(AttenuationViolation::NotBeforeWidened);
    }
    if child.expires_at_unix > parent.expires_at_unix {
        violations.push(AttenuationViolation::ExpiryWidened);
    }
    if child.max_dispatches > parent.max_dispatches {
        violations.push(AttenuationViolation::DispatchBudgetExpanded);
    }
    if child.max_tool_calls > parent.max_tool_calls {
        violations.push(AttenuationViolation::ToolBudgetExpanded);
    }
    if child.max_resource_units > parent.max_resource_units {
        violations.push(AttenuationViolation::ResourceBudgetExpanded);
    }
    if child.remaining_delegation_depth
        > parent.remaining_delegation_depth.saturating_sub(1)
    {
        violations.push(AttenuationViolation::DelegationDepthExpanded);
    }
    if child.max_risk_level > parent.max_risk_level {
        violations.push(AttenuationViolation::RiskCeilingExpanded);
    }
    if !parent.required_approvals.is_subset(&child.required_approvals) {
        violations.push(AttenuationViolation::ApprovalRequirementWeakened);
    }
    if !parent.required_postconditions.is_subset(&child.required_postconditions) {
        violations.push(AttenuationViolation::PostconditionWeakened);
    }
    if !parent.rollback_obligations.is_subset(&child.rollback_obligations) {
        violations.push(AttenuationViolation::RollbackObligationWeakened);
    }
    if child.delegable && child.remaining_delegation_depth == 0 {
        violations.push(AttenuationViolation::ChildDelegationDepthInvalid);
    }

    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}

/// Checks attenuation and the supplied current-time/revocation state.
/// The caller must provide fresh canonical authority state at the final
/// participant boundary; this pure function cannot prove that the input is fresh.
/// For a multi-hop chain, use this check on every edge or call
/// `verify_delegation_chain_at` so every ancestor is checked at the same time.
pub fn verify_delegation_at(
    parent: &DelegatedAuthorityGrant,
    child: &DelegatedAuthorityGrant,
    now_unix: i64,
) -> Result<(), Vec<AttenuationViolation>> {
    let mut violations = check_attenuation(parent, child).err().unwrap_or_default();

    if parent.revoked {
        violations.push(AttenuationViolation::ParentRevoked);
    }
    if child.revoked {
        violations.push(AttenuationViolation::ChildRevoked);
    }
    if now_unix < parent.not_before_unix {
        violations.push(AttenuationViolation::ParentNotYetValid);
    }
    if now_unix >= parent.expires_at_unix {
        violations.push(AttenuationViolation::ParentExpired);
    }
    if now_unix < child.not_before_unix {
        violations.push(AttenuationViolation::ChildNotYetValid);
    }
    if now_unix >= child.expires_at_unix {
        violations.push(AttenuationViolation::ChildExpired);
    }

    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}

/// Verifies every parent-child edge and rejects repeated grant/task identities.
/// The first entry must be the root grant and the last entry the acting leaf.
/// This remains a pure benchmark predicate: production must resolve fresh
/// canonical revocation state for every ancestor at the final participant.
pub fn verify_delegation_chain_at(
    grants: &[DelegatedAuthorityGrant],
    now_unix: i64,
) -> Result<(), Vec<AttenuationViolation>> {
    let mut violations = Vec::new();
    if grants.len() < 2 {
        return Err(vec![AttenuationViolation::InvalidChainLength]);
    }

    let mut grant_ids = BTreeSet::new();
    let mut task_ids = BTreeSet::new();
    for grant in grants {
        if !grant_ids.insert(grant.grant_id.as_str()) {
            violations.push(AttenuationViolation::DuplicateGrantInChain);
        }
        if !task_ids.insert(grant.task_id.as_str()) {
            violations.push(AttenuationViolation::DuplicateTaskInChain);
        }
    }

    for edge in grants.windows(2) {
        if let Err(mut edge_violations) = verify_delegation_at(&edge[0], &edge[1], now_unix) {
            violations.append(&mut edge_violations);
        }
    }

    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}

fn parameter_bound_is_subset(
    child: &ParameterBound,
    parent: &ParameterBound,
) -> bool {
    match (child, parent) {
        (ParameterBound::ExactString(child), ParameterBound::ExactString(parent)) => {
            child == parent
        }
        (ParameterBound::ExactInteger(child), ParameterBound::ExactInteger(parent)) => {
            child == parent
        }
        (
            ParameterBound::ExactInteger(value),
            ParameterBound::IntegerRange { min, max },
        ) => value >= min && value <= max,
        (
            ParameterBound::IntegerRange {
                min: child_min,
                max: child_max,
            },
            ParameterBound::IntegerRange {
                min: parent_min,
                max: parent_max,
            },
        ) => child_min <= child_max && child_min >= parent_min && child_max <= parent_max,
        // A type change or incomparable constraint is not a valid narrowing.
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::iter::FromIterator;

    fn digest(ch: char) -> String {
        std::iter::repeat_n(ch, 64).collect()
    }

    fn set(values: &[&str]) -> BTreeSet<String> {
        BTreeSet::from_iter(values.iter().map(|value| (*value).to_string()))
    }

    fn parent_grant() -> DelegatedAuthorityGrant {
        DelegatedAuthorityGrant {
            grant_id: "grant-root".into(),
            issuer_principal: "xenia-authority".into(),
            subject_principal: "agent-a".into(),
            root_task_id: "task-root".into(),
            root_task_digest: digest('a'),
            root_plan_digest: digest('b'),
            task_id: "task-root".into(),
            task_digest: digest('a'),
            plan_digest: digest('b'),
            parent_task_id: None,
            parent_task_digest: None,
            parent_grant_id: None,
            tenant_binding: "tenant-1".into(),
            targets: set(&["service/api", "service/worker"]),
            operations: set(&["read", "restart"]),
            parameter_bounds: BTreeMap::from([
                ("replicas".into(), ParameterBound::IntegerRange { min: 1, max: 5 }),
                ("region".into(), ParameterBound::ExactString("af-south-1".into())),
            ]),
            data_scopes: set(&["metrics", "service-config"]),
            not_before_unix: 100,
            expires_at_unix: 500,
            max_dispatches: 4,
            max_tool_calls: 20,
            max_resource_units: 100,
            remaining_delegation_depth: 2,
            max_risk_level: 3,
            required_approvals: set(&["change-ticket"]),
            required_postconditions: set(&["service-healthy", "replicas-converged"]),
            rollback_obligations: set(&["restore-replicas"]),
            allowed_child_task_digests: set(&[&digest('c')]),
            allowed_child_plan_digests: set(&[&digest('d')]),
            delegable: true,
            revoked: false,
        }
    }

    fn child_grant() -> DelegatedAuthorityGrant {
        let parent = parent_grant();
        DelegatedAuthorityGrant {
            grant_id: "grant-child".into(),
            issuer_principal: parent.subject_principal.clone(),
            subject_principal: "agent-b".into(),
            root_task_id: parent.root_task_id.clone(),
            root_task_digest: parent.root_task_digest.clone(),
            root_plan_digest: parent.root_plan_digest.clone(),
            task_id: "task-child".into(),
            task_digest: digest('c'),
            plan_digest: digest('d'),
            parent_task_id: Some(parent.task_id.clone()),
            parent_task_digest: Some(parent.task_digest.clone()),
            parent_grant_id: Some(parent.grant_id.clone()),
            tenant_binding: parent.tenant_binding.clone(),
            targets: set(&["service/api"]),
            operations: set(&["read"]),
            parameter_bounds: BTreeMap::from([
                ("replicas".into(), ParameterBound::IntegerRange { min: 2, max: 3 }),
                ("region".into(), ParameterBound::ExactString("af-south-1".into())),
            ]),
            data_scopes: set(&["metrics"]),
            not_before_unix: 120,
            expires_at_unix: 300,
            max_dispatches: 1,
            max_tool_calls: 5,
            max_resource_units: 20,
            remaining_delegation_depth: 1,
            max_risk_level: 1,
            required_approvals: set(&["change-ticket", "service-owner"]),
            required_postconditions: set(&[
                "service-healthy",
                "replicas-converged",
                "child-task-complete",
            ]),
            rollback_obligations: set(&["restore-replicas", "emit-rollback-receipt"]),
            allowed_child_task_digests: set(&[&digest('e')]),
            allowed_child_plan_digests: set(&[&digest('f')]),
            delegable: true,
            revoked: false,
        }
    }

    #[test]
    fn strict_narrowing_is_accepted() {
        let parent = parent_grant();
        let child = child_grant();
        assert_eq!(check_attenuation(&parent, &child), Ok(()));
        assert_eq!(verify_delegation_at(&parent, &child, 150), Ok(()));
    }


    #[test]
    fn zero_mutation_budget_allows_safe_read_only_delegation() {
        let mut parent = parent_grant();
        parent.max_dispatches = 0;
        let mut child = child_grant();
        child.max_dispatches = 0;
        child.operations = set(&["read"]);
        assert_eq!(verify_delegation_at(&parent, &child, 150), Ok(()));
    }

    #[test]
    fn exact_integer_inside_parent_range_is_accepted() {
        let parent = parent_grant();
        let mut child = child_grant();
        child.parameter_bounds.insert(
            "replicas".into(),
            ParameterBound::ExactInteger(3),
        );
        assert_eq!(check_attenuation(&parent, &child), Ok(()));
    }

    #[test]
    fn child_cannot_expand_target_operation_or_data_scope() {
        let parent = parent_grant();
        let mut child = child_grant();
        child.targets = set(&["service/api", "service/worker"]);
        child.operations = set(&["read", "restart"]);
        child.data_scopes = set(&["metrics", "service-config"]);
        let errors = check_attenuation(&parent, &child).unwrap_err();
        assert!(errors.contains(&AttenuationViolation::TargetScopeExpanded));
        assert!(errors.contains(&AttenuationViolation::OperationScopeExpanded));
        assert!(errors.contains(&AttenuationViolation::DataScopeExpanded));
    }

    #[test]
    fn child_cannot_omit_or_widen_parent_parameter_constraints() {
        let parent = parent_grant();

        let mut omitted = child_grant();
        omitted.parameter_bounds.remove("region");
        assert!(check_attenuation(&parent, &omitted).unwrap_err().contains(
            &AttenuationViolation::ParameterScopeExpanded("region".into())
        ));

        let mut widened = child_grant();
        widened.parameter_bounds.insert(
            "replicas".into(),
            ParameterBound::IntegerRange { min: 0, max: 8 },
        );
        assert!(check_attenuation(&parent, &widened).unwrap_err().contains(
            &AttenuationViolation::ParameterScopeExpanded("replicas".into())
        ));
    }

    #[test]
    fn incomparable_parameter_types_fail_closed() {
        let parent = parent_grant();
        let mut child = child_grant();
        child.parameter_bounds.insert(
            "replicas".into(),
            ParameterBound::ExactString("3".into()),
        );
        assert!(check_attenuation(&parent, &child).unwrap_err().contains(
            &AttenuationViolation::ParameterScopeExpanded("replicas".into())
        ));
    }

    #[test]
    fn child_cannot_extend_time_or_any_budget() {
        let parent = parent_grant();
        let mut child = child_grant();
        child.not_before_unix = 99;
        child.expires_at_unix = 501;
        child.max_dispatches = 5;
        child.max_tool_calls = 21;
        child.max_resource_units = 101;
        child.remaining_delegation_depth = 2;
        child.max_risk_level = 4;
        let errors = check_attenuation(&parent, &child).unwrap_err();
        assert!(errors.contains(&AttenuationViolation::NotBeforeWidened));
        assert!(errors.contains(&AttenuationViolation::ExpiryWidened));
        assert!(errors.contains(&AttenuationViolation::DispatchBudgetExpanded));
        assert!(errors.contains(&AttenuationViolation::ToolBudgetExpanded));
        assert!(errors.contains(&AttenuationViolation::ResourceBudgetExpanded));
        assert!(errors.contains(&AttenuationViolation::DelegationDepthExpanded));
        assert!(errors.contains(&AttenuationViolation::RiskCeilingExpanded));
    }

    #[test]
    fn child_cannot_drop_approval_postcondition_or_rollback_obligation() {
        let parent = parent_grant();
        let mut child = child_grant();
        child.required_approvals.clear();
        child.required_postconditions.remove("service-healthy");
        child.rollback_obligations.clear();
        let errors = check_attenuation(&parent, &child).unwrap_err();
        assert!(errors.contains(&AttenuationViolation::ApprovalRequirementWeakened));
        assert!(errors.contains(&AttenuationViolation::PostconditionWeakened));
        assert!(errors.contains(&AttenuationViolation::RollbackObligationWeakened));
    }

    #[test]
    fn child_must_preserve_exact_task_root_parent_and_principal_lineage() {
        let parent = parent_grant();
        let mut child = child_grant();
        child.root_task_digest = digest('e');
        child.root_plan_digest = digest('f');
        child.parent_task_id = Some("other-parent".into());
        child.parent_task_digest = Some(digest('e'));
        child.parent_grant_id = Some("other-grant".into());
        child.issuer_principal = "unrelated-agent".into();
        child.task_id = parent.task_id.clone();
        let errors = check_attenuation(&parent, &child).unwrap_err();
        assert!(errors.contains(&AttenuationViolation::RootTaskDigestChanged));
        assert!(errors.contains(&AttenuationViolation::RootPlanChanged));
        assert!(errors.contains(&AttenuationViolation::ParentTaskLinkMismatch));
        assert!(errors.contains(&AttenuationViolation::ParentTaskDigestMismatch));
        assert!(errors.contains(&AttenuationViolation::ParentGrantLinkMismatch));
        assert!(errors.contains(&AttenuationViolation::PrincipalChainMismatch));
        assert!(errors.contains(&AttenuationViolation::ChildTaskIdNotDistinct));
    }

    #[test]
    fn child_task_digest_must_be_precommitted_by_parent() {
        let parent = parent_grant();
        let mut child = child_grant();
        child.task_digest = digest('e');
        assert!(check_attenuation(&parent, &child).unwrap_err().contains(
            &AttenuationViolation::ChildTaskDigestNotAllowed
        ));
    }

    #[test]
    fn child_plan_digest_must_be_precommitted_by_parent() {
        let parent = parent_grant();
        let mut child = child_grant();
        child.plan_digest = digest('e');
        assert!(check_attenuation(&parent, &child).unwrap_err().contains(
            &AttenuationViolation::ChildPlanDigestNotAllowed
        ));
    }

    #[test]
    fn nondelegable_parent_and_exhausted_depth_are_rejected() {
        let mut parent = parent_grant();
        let child = child_grant();
        parent.delegable = false;
        parent.remaining_delegation_depth = 0;
        assert!(check_attenuation(&parent, &child).unwrap_err().contains(
            &AttenuationViolation::ParentCannotDelegate
        ));
    }

    #[test]
    fn revoked_or_inactive_ancestor_or_child_is_rejected() {
        let mut parent = parent_grant();
        let child = child_grant();
        parent.revoked = true;
        assert!(verify_delegation_at(&parent, &child, 150).unwrap_err().contains(
            &AttenuationViolation::ParentRevoked
        ));

        parent.revoked = false;
        assert!(verify_delegation_at(&parent, &child, 500).unwrap_err().contains(
            &AttenuationViolation::ParentExpired
        ));
        assert!(verify_delegation_at(&parent, &child, 300).unwrap_err().contains(
            &AttenuationViolation::ChildExpired
        ));

        let mut revoked_child = child;
        revoked_child.revoked = true;
        assert!(verify_delegation_at(&parent, &revoked_child, 150).unwrap_err().contains(
            &AttenuationViolation::ChildRevoked
        ));
    }

    fn grandchild_grant(parent: &DelegatedAuthorityGrant) -> DelegatedAuthorityGrant {
        DelegatedAuthorityGrant {
            grant_id: "grant-grandchild".into(),
            issuer_principal: parent.subject_principal.clone(),
            subject_principal: "agent-c".into(),
            root_task_id: parent.root_task_id.clone(),
            root_task_digest: parent.root_task_digest.clone(),
            root_plan_digest: parent.root_plan_digest.clone(),
            task_id: "task-grandchild".into(),
            task_digest: digest('e'),
            plan_digest: digest('f'),
            parent_task_id: Some(parent.task_id.clone()),
            parent_task_digest: Some(parent.task_digest.clone()),
            parent_grant_id: Some(parent.grant_id.clone()),
            tenant_binding: parent.tenant_binding.clone(),
            targets: set(&["service/api"]),
            operations: set(&["read"]),
            parameter_bounds: BTreeMap::from([
                ("replicas".into(), ParameterBound::ExactInteger(2)),
                ("region".into(), ParameterBound::ExactString("af-south-1".into())),
            ]),
            data_scopes: set(&["metrics"]),
            not_before_unix: 140,
            expires_at_unix: 200,
            max_dispatches: 1,
            max_tool_calls: 2,
            max_resource_units: 5,
            remaining_delegation_depth: 0,
            max_risk_level: 0,
            required_approvals: parent.required_approvals.clone(),
            required_postconditions: parent.required_postconditions.clone(),
            rollback_obligations: parent.rollback_obligations.clone(),
            allowed_child_task_digests: BTreeSet::new(),
            allowed_child_plan_digests: BTreeSet::new(),
            delegable: false,
            revoked: false,
        }
    }

    #[test]
    fn every_edge_in_a_three_agent_chain_must_attenuate() {
        let root = parent_grant();
        let child = child_grant();
        let grandchild = grandchild_grant(&child);
        assert_eq!(
            verify_delegation_chain_at(&[root.clone(), child.clone(), grandchild.clone()], 150),
            Ok(())
        );

        let mut duplicate = grandchild;
        duplicate.grant_id = child.grant_id.clone();
        assert!(verify_delegation_chain_at(&[root, child, duplicate], 150)
            .unwrap_err()
            .contains(&AttenuationViolation::DuplicateGrantInChain));
    }


    #[test]
    fn evidence_bundle_audits_three_agent_read_only_chain() {
        let schema = include_str!("../../delegation-evidence.schema.json");
        let fixture = include_str!("../fixtures/delegation-chain-positive.json");
        let audit = audit_delegation_evidence(fixture.as_bytes(), schema.as_bytes())
            .expect("positive delegation evidence must pass schema and semantic audit");
        assert!(audit.attenuation_qualified, "{:?}", audit.violations);
        assert_eq!(audit.computed_outcome, "qualified");
        assert!(audit.declared_outcome_matches);
        assert!(audit.participant_recheck_bound_to_leaf);
        assert!(audit.participant_decision_correct);
        assert!(audit.all_ancestor_authority_fresh);
        assert!(audit.evidence_lineage_complete);
    }

    #[test]
    fn evidence_bundle_records_known_scope_expansion_as_denied() {
        let schema = include_str!("../../delegation-evidence.schema.json");
        let fixture = include_str!("../fixtures/delegation-chain-positive.json");
        let mut value: serde_json::Value = serde_json::from_str(fixture).unwrap();
        value["chain"][1]["targets"] = serde_json::json!(["service/api", "service/worker"]);
        value["outcome"] = serde_json::json!("denied");
        value["participant_recheck"]["authority_state"] = serde_json::json!("active");
        value["participant_recheck"]["decision"] = serde_json::json!("denied");
        let bytes = serde_json::to_vec(&value).unwrap();
        let audit = audit_delegation_evidence(&bytes, schema.as_bytes()).unwrap();
        assert!(!audit.attenuation_qualified);
        assert_eq!(audit.computed_outcome, "denied");
        assert!(audit.declared_outcome_matches);
        assert!(audit.participant_decision_correct);
        assert!(audit.violations.iter().any(|code| code.contains("TargetScopeExpanded")));
    }

    #[test]
    fn evidence_bundle_requires_fresh_authority_for_every_ancestor() {
        let schema = include_str!("../../delegation-evidence.schema.json");
        let fixture = include_str!("../fixtures/delegation-chain-positive.json");
        let mut value: serde_json::Value = serde_json::from_str(fixture).unwrap();
        value["authority_checks"][0]["checked_at_unix"] = serde_json::json!(1000);
        value["outcome"] = serde_json::json!("unknown");
        value["participant_recheck"]["authority_state"] = serde_json::json!("unknown");
        value["participant_recheck"]["decision"] = serde_json::json!("unknown");
        let bytes = serde_json::to_vec(&value).unwrap();
        let audit = audit_delegation_evidence(&bytes, schema.as_bytes()).unwrap();
        assert_eq!(audit.computed_outcome, "unknown");
        assert!(!audit.all_ancestor_authority_fresh);
        assert!(audit.participant_decision_correct);
        assert!(audit.violations.iter().any(|code| code.contains("STALE_OR_FUTURE")));
    }

    #[test]
    fn evidence_bundle_rejects_agent_laundered_participant_recheck() {
        let schema = include_str!("../../delegation-evidence.schema.json");
        let fixture = include_str!("../fixtures/delegation-chain-positive.json");
        let mut value: serde_json::Value = serde_json::from_str(fixture).unwrap();
        value["evidence_lineage"].as_array_mut().unwrap().push(serde_json::json!({
            "evidence_ref": "agent-summary",
            "kind": "agent_summary",
            "source_ref": "agent-output",
            "control_domain_ref": "agent-domain",
            "derived_from": [],
            "digest": format!("sha256:{}", "9".repeat(64))
        }));
        value["evidence_lineage"]
            .as_array_mut().unwrap()
            .iter_mut()
            .find(|record| record["evidence_ref"] == "participant-recheck")
            .unwrap()["derived_from"]
            .as_array_mut().unwrap()
            .push(serde_json::json!("agent-summary"));
        let bytes = serde_json::to_vec(&value).unwrap();
        let audit = audit_delegation_evidence(&bytes, schema.as_bytes()).unwrap();
        assert!(audit.violations.iter().any(|code| code.contains("DERIVED_FROM_AGENT_SUMMARY")));
        assert!(!audit.evidence_lineage_complete);
    }

    #[test]
    fn evidence_bundle_rejects_misordered_hops_and_leaf_substitution() {
        let schema = include_str!("../../delegation-evidence.schema.json");
        let fixture = include_str!("../fixtures/delegation-chain-positive.json");
        let mut value: serde_json::Value = serde_json::from_str(fixture).unwrap();
        value["chain"].as_array_mut().unwrap().swap(1, 2);
        value["participant_recheck"]["leaf_task_id"] = serde_json::json!("other-task");
        let bytes = serde_json::to_vec(&value).unwrap();
        let audit = audit_delegation_evidence(&bytes, schema.as_bytes()).unwrap();
        assert!(!audit.attenuation_qualified);
        assert!(!audit.participant_recheck_bound_to_leaf);
        assert!(audit.violations.iter().any(|code| code.contains("ParentTaskLinkMismatch")));
    }

}
