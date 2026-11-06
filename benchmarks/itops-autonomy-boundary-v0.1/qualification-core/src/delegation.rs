//! Fail-closed model of monotonic authority attenuation for synthetic ITOPS tests.
//!
//! This module checks whether a delegated child grant is a semantic subset of
//! its verified parent. It does not issue, sign, authenticate, or revoke grants.
//! Production identity, consent, authorization, and revocation remain owned by
//! Xenia; the final participant must independently recheck current authority.

use std::collections::{BTreeMap, BTreeSet};

/// Exact constraints precommitted for one root-authorized subtask edge.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RootAuthorizedTaskScope {
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
    pub max_risk_level: u8,
    pub required_approvals: BTreeSet<String>,
    pub required_postconditions: BTreeSet<String>,
    pub rollback_obligations: BTreeSet<String>,
}

/// One exact delegation edge precommitted in the root task graph. Principals are
/// opaque canonical references; the benchmark predicate does not authenticate them.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RootDelegationEdge {
    pub parent_task_digest: String,
    pub child_task_digest: String,
    pub child_plan_digest: String,
    pub delegator_principal: String,
    pub delegatee_principal: String,
    pub authorized_scope: RootAuthorizedTaskScope,
}

/// A deliberately small constraint language with decidable subset checks.
/// Unsupported or incomparable constraint forms are rejected, never guessed.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
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
    /// Exact task-digest to plan-digest pairs authorized by this parent.
    pub allowed_child_task_plans: BTreeMap<String, String>,
    /// Root task/plan commitments for all authorized descendants, not only direct children.
    pub allowed_descendant_task_plans: BTreeMap<String, String>,
    /// Exact root-precommitted parent→child graph edges and principal assignments.
    pub authorized_delegation_edges: BTreeSet<RootDelegationEdge>,
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
    ChildGrantIdNotDistinct,
    ChildTaskIdNotDistinct,
    ChildTaskDigestNotAllowed,
    ChildPlanDigestNotAllowed,
    RootDescendantTaskPlanNotAllowed,
    RootDelegationEdgeNotAllowed,
    RootAuthorizedSubtaskScopeExpanded,
    RootEnvelopeScopeExpanded,
    InvalidRootAnchoredChain,
    RootReanchorMissingAncestors,
    InvalidChainLength,
    DuplicateGrantInChain,
    DuplicateTaskInChain,
    DuplicateChildGrant,
    DuplicateChildTask,
    SiblingDispatchBudgetExceeded,
    SiblingToolBudgetExceeded,
    SiblingResourceBudgetExceeded,
    SiblingBudgetOverflow,
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
        && grant.max_tool_calls > 0
        && grant.max_resource_units > 0
        && obligation_set_is_valid(&grant.required_approvals)
        && obligation_set_is_valid(&grant.required_postconditions)
        && obligation_set_is_valid(&grant.rollback_obligations)
        && grant.allowed_child_task_digests.iter().all(|digest| digest_is_sha256(digest))
        && grant.allowed_child_plan_digests.iter().all(|digest| digest_is_sha256(digest))
        && grant.allowed_child_task_plans.iter().all(|(task, plan)| digest_is_sha256(task) && digest_is_sha256(plan))
        && grant.allowed_descendant_task_plans.iter().all(|(task, plan)| digest_is_sha256(task) && digest_is_sha256(plan))
        && grant.authorized_delegation_edges.iter().all(|edge| {
            let scope = &edge.authorized_scope;
            digest_is_sha256(&edge.parent_task_digest)
                && digest_is_sha256(&edge.child_task_digest)
                && digest_is_sha256(&edge.child_plan_digest)
                && nonempty(&edge.delegator_principal)
                && nonempty(&edge.delegatee_principal)
                && nonempty(&scope.tenant_binding)
                && set_is_valid(&scope.targets)
                && set_is_valid(&scope.operations)
                && bounds_are_valid(&scope.parameter_bounds)
                && set_is_valid(&scope.data_scopes)
                && scope.not_before_unix < scope.expires_at_unix
                && scope.max_tool_calls > 0
                && scope.max_resource_units > 0
                && obligation_set_is_valid(&scope.required_approvals)
                && obligation_set_is_valid(&scope.required_postconditions)
                && obligation_set_is_valid(&scope.rollback_obligations)
        })
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
    if child.grant_id == parent.grant_id {
        violations.push(AttenuationViolation::ChildGrantIdNotDistinct);
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
    if parent.allowed_child_task_plans.get(&child.task_digest)
        != Some(&child.plan_digest)
    {
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

/// Verifies a child against a root-authorized task graph while retaining
/// immediate-parent delegation lineage and budgets.
///
/// Unlike `verify_delegation_at`, this policy does not intersect the
/// child's task-specific target/operation/data/parameter scope with the
/// immediate parent's task scope. Instead, the root must precommit the exact
/// child task *and* plan digests; the child scope is bounded by the root
/// envelope, while the immediate parent must be a valid, active, delegable
/// ancestor and retains control of child budgets, expiry and depth.
///
/// This is a candidate policy arm for matched benchmark comparison, not a
/// grant issuer. The caller must provide the complete verified ancestor chain
/// and fresh canonical authority state at the participant boundary.
pub fn verify_root_reanchored_child_at(
    ancestors: &[DelegatedAuthorityGrant],
    child: &DelegatedAuthorityGrant,
    now_unix: i64,
) -> Result<(), Vec<AttenuationViolation>> {
    if ancestors.is_empty() {
        return Err(vec![AttenuationViolation::RootReanchorMissingAncestors]);
    }

    let root = &ancestors[0];
    let parent = ancestors.last().expect("non-empty ancestor chain checked");
    let mut violations = Vec::new();

    if ancestors.len() == 1 {
        if !grant_shape_is_valid(root) {
            violations.push(AttenuationViolation::InvalidParentShape);
        }
    } else if let Err(mut chain_violations) = verify_delegation_chain_at(ancestors, now_unix) {
        violations.append(&mut chain_violations);
    }

    if !grant_shape_is_valid(child) {
        violations.push(AttenuationViolation::InvalidChildShape);
    }
    if !parent.delegable || parent.remaining_delegation_depth == 0 {
        violations.push(AttenuationViolation::ParentCannotDelegate);
    }

    for ancestor in ancestors {
        if ancestor.revoked {
            violations.push(AttenuationViolation::ParentRevoked);
        }
        if now_unix < ancestor.not_before_unix {
            violations.push(AttenuationViolation::ParentNotYetValid);
        }
        if now_unix >= ancestor.expires_at_unix {
            violations.push(AttenuationViolation::ParentExpired);
        }
    }
    if child.revoked {
        violations.push(AttenuationViolation::ChildRevoked);
    }
    if now_unix < child.not_before_unix {
        violations.push(AttenuationViolation::ChildNotYetValid);
    }
    if now_unix >= child.expires_at_unix {
        violations.push(AttenuationViolation::ChildExpired);
    }

    let mut grant_ids = BTreeSet::new();
    let mut task_ids = BTreeSet::new();
    for ancestor in ancestors {
        if !grant_ids.insert(ancestor.grant_id.as_str()) {
            violations.push(AttenuationViolation::DuplicateGrantInChain);
        }
        if !task_ids.insert(ancestor.task_id.as_str()) {
            violations.push(AttenuationViolation::DuplicateTaskInChain);
        }
    }
    if !grant_ids.insert(child.grant_id.as_str()) {
        violations.push(AttenuationViolation::DuplicateChildGrant);
    }
    if !task_ids.insert(child.task_id.as_str()) {
        violations.push(AttenuationViolation::DuplicateChildTask);
    }

    if child.grant_id == parent.grant_id {
        violations.push(AttenuationViolation::ChildGrantIdNotDistinct);
    }
    if child.task_id == parent.task_id {
        violations.push(AttenuationViolation::ChildTaskIdNotDistinct);
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

    if child.root_task_id != root.root_task_id {
        violations.push(AttenuationViolation::RootTaskChanged);
    }
    if child.root_task_digest != root.root_task_digest {
        violations.push(AttenuationViolation::RootTaskDigestChanged);
    }
    if child.root_plan_digest != root.root_plan_digest {
        violations.push(AttenuationViolation::RootPlanChanged);
    }
    if child.tenant_binding != root.tenant_binding {
        violations.push(AttenuationViolation::TenantChanged);
    }
    if !root.allowed_child_task_digests.contains(&child.task_digest) {
        violations.push(AttenuationViolation::ChildTaskDigestNotAllowed);
    }
    if !root.allowed_child_plan_digests.contains(&child.plan_digest) {
        violations.push(AttenuationViolation::ChildPlanDigestNotAllowed);
    }
    // The root authorizes the overall task graph; the immediate parent must
    // also precommit the exact next hop it is allowed to create.
    if !parent.allowed_child_task_digests.contains(&child.task_digest) {
        violations.push(AttenuationViolation::ChildTaskDigestNotAllowed);
    }
    if !parent.allowed_child_plan_digests.contains(&child.plan_digest) {
        violations.push(AttenuationViolation::ChildPlanDigestNotAllowed);
    }

    // Task-specific action scopes are anchored to the root, not recursively
    // intersected with an intermediate agent's narrower sibling-task scope.
    if !child.targets.is_subset(&root.targets) {
        violations.push(AttenuationViolation::TargetScopeExpanded);
    }
    if !child.operations.is_subset(&root.operations) {
        violations.push(AttenuationViolation::OperationScopeExpanded);
    }
    if !child.data_scopes.is_subset(&root.data_scopes) {
        violations.push(AttenuationViolation::DataScopeExpanded);
    }
    for (name, root_bound) in &root.parameter_bounds {
        let Some(child_bound) = child.parameter_bounds.get(name) else {
            violations.push(AttenuationViolation::ParameterScopeExpanded(name.clone()));
            continue;
        };
        if !parameter_bound_is_subset(child_bound, root_bound) {
            violations.push(AttenuationViolation::ParameterScopeExpanded(name.clone()));
        }
    }

    // The parent remains responsible for sub-delegation capacity even where
    // the child's task-specific operation scope is broader than the parent's.
    if child.not_before_unix < root.not_before_unix
        || child.not_before_unix < parent.not_before_unix
    {
        violations.push(AttenuationViolation::NotBeforeWidened);
    }
    if child.expires_at_unix > root.expires_at_unix
        || child.expires_at_unix > parent.expires_at_unix
    {
        violations.push(AttenuationViolation::ExpiryWidened);
    }
    if child.max_dispatches > root.max_dispatches
        || child.max_dispatches > parent.max_dispatches
    {
        violations.push(AttenuationViolation::DispatchBudgetExpanded);
    }
    if child.max_tool_calls > root.max_tool_calls
        || child.max_tool_calls > parent.max_tool_calls
    {
        violations.push(AttenuationViolation::ToolBudgetExpanded);
    }
    if child.max_resource_units > root.max_resource_units
        || child.max_resource_units > parent.max_resource_units
    {
        violations.push(AttenuationViolation::ResourceBudgetExpanded);
    }
    if child.remaining_delegation_depth
        > parent.remaining_delegation_depth.saturating_sub(1)
    {
        violations.push(AttenuationViolation::DelegationDepthExpanded);
    }
    if child.max_risk_level > root.max_risk_level
        || child.max_risk_level > parent.max_risk_level
    {
        violations.push(AttenuationViolation::RiskCeilingExpanded);
    }
    if !root.required_approvals.is_subset(&child.required_approvals) {
        violations.push(AttenuationViolation::ApprovalRequirementWeakened);
    }
    if !root.required_postconditions.is_subset(&child.required_postconditions) {
        violations.push(AttenuationViolation::PostconditionWeakened);
    }
    if !root.rollback_obligations.is_subset(&child.rollback_obligations) {
        violations.push(AttenuationViolation::RollbackObligationWeakened);
    }

    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}

/// Verifies a complete sibling fan-out using root-authorized task scopes and
/// one shared immediate-parent delegation budget. Repeated batches require a
/// durable participant-owned budget ledger; this function is a pure batch check.
pub fn verify_root_reanchored_children_at(
    ancestors: &[DelegatedAuthorityGrant],
    children: &[DelegatedAuthorityGrant],
    now_unix: i64,
) -> Result<(), Vec<AttenuationViolation>> {
    if children.is_empty() {
        return Ok(());
    }
    if ancestors.is_empty() {
        return Err(vec![AttenuationViolation::RootReanchorMissingAncestors]);
    }

    let parent = ancestors.last().expect("non-empty ancestor chain checked");
    let mut violations = Vec::new();
    let mut grant_ids = BTreeSet::new();
    let mut task_ids = BTreeSet::new();
    let mut dispatch_budget = 0_u64;
    let mut tool_budget = 0_u64;
    let mut resource_budget = 0_u64;

    for child in children {
        if !grant_ids.insert(child.grant_id.as_str()) {
            violations.push(AttenuationViolation::DuplicateChildGrant);
        }
        if !task_ids.insert(child.task_id.as_str()) {
            violations.push(AttenuationViolation::DuplicateChildTask);
        }
        if let Err(mut child_violations) =
            verify_root_reanchored_child_at(ancestors, child, now_unix)
        {
            violations.append(&mut child_violations);
        }
        match dispatch_budget.checked_add(child.max_dispatches) {
            Some(total) => dispatch_budget = total,
            None => {
                dispatch_budget = u64::MAX;
                violations.push(AttenuationViolation::SiblingBudgetOverflow);
            }
        }
        match tool_budget.checked_add(child.max_tool_calls) {
            Some(total) => tool_budget = total,
            None => {
                tool_budget = u64::MAX;
                violations.push(AttenuationViolation::SiblingBudgetOverflow);
            }
        }
        match resource_budget.checked_add(child.max_resource_units) {
            Some(total) => resource_budget = total,
            None => {
                resource_budget = u64::MAX;
                violations.push(AttenuationViolation::SiblingBudgetOverflow);
            }
        }
    }

    if dispatch_budget > parent.max_dispatches {
        violations.push(AttenuationViolation::SiblingDispatchBudgetExceeded);
    }
    if tool_budget > parent.max_tool_calls {
        violations.push(AttenuationViolation::SiblingToolBudgetExceeded);
    }
    if resource_budget > parent.max_resource_units {
        violations.push(AttenuationViolation::SiblingResourceBudgetExceeded);
    }

    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}

/// Root-anchored authorization is a separate policy arm from strict edge-by-edge
/// attenuation. Every descendant must be precommitted by the root task graph and
/// explicitly delegated by its immediate parent, but its execution scope is
/// compared to the root envelope rather than recursively intersected with each
/// intermediate agent's execution scope. This allows a parent to delegate a
/// different root-authorized sibling subtask without gaining authority itself.
///
/// The chain still enforces immutable lineage, principal continuity, immediate
/// parent child-digest commitments, validity/revocation, and per-edge budgets.
/// This pure predicate does not authenticate the root manifest or enforce a live
/// participant-side request check.
pub fn verify_root_anchored_chain_at(
    grants: &[DelegatedAuthorityGrant],
    now_unix: i64,
) -> Result<(), Vec<AttenuationViolation>> {
    if grants.len() < 2 {
        return Err(vec![AttenuationViolation::InvalidRootAnchoredChain]);
    }

    let root = &grants[0];
    let mut violations = Vec::new();
    if !grant_shape_is_valid(root) {
        violations.push(AttenuationViolation::InvalidParentShape);
    }
    if root.revoked {
        violations.push(AttenuationViolation::ParentRevoked);
    }
    if now_unix < root.not_before_unix {
        violations.push(AttenuationViolation::ParentNotYetValid);
    }
    if now_unix >= root.expires_at_unix {
        violations.push(AttenuationViolation::ParentExpired);
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
        let parent = &edge[0];
        let child = &edge[1];
        if !grant_shape_is_valid(child) {
            violations.push(AttenuationViolation::InvalidChildShape);
        }
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

        if !parent.delegable || parent.remaining_delegation_depth == 0 {
            violations.push(AttenuationViolation::ParentCannotDelegate);
        }
        if child.root_task_id != root.root_task_id
            || child.root_task_id != parent.root_task_id
        {
            violations.push(AttenuationViolation::RootTaskChanged);
        }
        if child.root_task_digest != root.root_task_digest
            || child.root_task_digest != parent.root_task_digest
        {
            violations.push(AttenuationViolation::RootTaskDigestChanged);
        }
        if child.root_plan_digest != root.root_plan_digest
            || child.root_plan_digest != parent.root_plan_digest
        {
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
        if child.grant_id == parent.grant_id {
            violations.push(AttenuationViolation::ChildGrantIdNotDistinct);
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
        if parent.allowed_child_task_plans.get(&child.task_digest)
            != Some(&child.plan_digest)
        {
            violations.push(AttenuationViolation::ChildPlanDigestNotAllowed);
        }
        if root.allowed_descendant_task_plans.get(&child.task_digest)
            != Some(&child.plan_digest)
        {
            violations.push(AttenuationViolation::RootDescendantTaskPlanNotAllowed);
        }

        let authorized_edge = root.authorized_delegation_edges.iter().find(|edge| {
            edge.parent_task_digest == parent.task_digest
                && edge.child_task_digest == child.task_digest
                && edge.child_plan_digest == child.plan_digest
                && edge.delegator_principal == parent.subject_principal
                && edge.delegatee_principal == child.subject_principal
        });
        let Some(authorized_edge) = authorized_edge else {
            violations.push(AttenuationViolation::RootDelegationEdgeNotAllowed);
            continue;
        };
        let authorized_scope = &authorized_edge.authorized_scope;
        if child.tenant_binding != authorized_scope.tenant_binding
            || !child.targets.is_subset(&authorized_scope.targets)
            || !child.operations.is_subset(&authorized_scope.operations)
            || !child.data_scopes.is_subset(&authorized_scope.data_scopes)
            || authorized_scope.parameter_bounds.iter().any(|(name, bound)| {
                child.parameter_bounds.get(name)
                    .is_none_or(|child_bound| !parameter_bound_is_subset(child_bound, bound))
            })
            || child.not_before_unix < authorized_scope.not_before_unix
            || child.expires_at_unix > authorized_scope.expires_at_unix
            || child.max_dispatches > authorized_scope.max_dispatches
            || child.max_tool_calls > authorized_scope.max_tool_calls
            || child.max_resource_units > authorized_scope.max_resource_units
            || child.max_risk_level > authorized_scope.max_risk_level
            || !authorized_scope.required_approvals.is_subset(&child.required_approvals)
            || !authorized_scope.required_postconditions.is_subset(&child.required_postconditions)
            || !authorized_scope.rollback_obligations.is_subset(&child.rollback_obligations)
        {
            violations.push(AttenuationViolation::RootAuthorizedSubtaskScopeExpanded);
        }

        // Every descendant's scope must fit the original root envelope. It may
        // differ from an intermediate parent's scope only when the exact
        // descendant task and plan are precommitted above.
        if child.tenant_binding != root.tenant_binding
            || !child.targets.is_subset(&root.targets)
            || !child.operations.is_subset(&root.operations)
            || !child.data_scopes.is_subset(&root.data_scopes)
            || root.parameter_bounds.iter().any(|(name, bound)| {
                child.parameter_bounds.get(name)
                    .is_none_or(|child_bound| !parameter_bound_is_subset(child_bound, bound))
            })
            || child.max_risk_level > root.max_risk_level
            || child.expires_at_unix > root.expires_at_unix
            || child.not_before_unix < root.not_before_unix
            || child.max_dispatches > root.max_dispatches
            || child.max_tool_calls > root.max_tool_calls
            || child.max_resource_units > root.max_resource_units
            || !root.required_approvals.is_subset(&child.required_approvals)
            || !root.required_postconditions.is_subset(&child.required_postconditions)
            || !root.rollback_obligations.is_subset(&child.rollback_obligations)
        {
            violations.push(AttenuationViolation::RootEnvelopeScopeExpanded);
        }

        // Delegators may not grant children more lifetime, budget, risk, or
        // inherited obligations than the authority they hold to create them.
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
    }

    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}

/// Verifies one complete sibling-delegation batch against a parent's remaining
/// transferable budgets. Each child must independently attenuate from the same
/// parent, and the sum of child dispatch/tool/resource allocations must not
/// exceed the supplied parent budgets. Call this once for the complete fan-out
/// decision; repeated calls require an external shared budget ledger and are
/// not made safe by this pure predicate.
pub fn verify_delegated_children_at(
    parent: &DelegatedAuthorityGrant,
    children: &[DelegatedAuthorityGrant],
    now_unix: i64,
) -> Result<(), Vec<AttenuationViolation>> {
    if children.is_empty() {
        return Ok(());
    }

    let mut violations = Vec::new();
    let mut grant_ids = BTreeSet::new();
    let mut task_ids = BTreeSet::new();
    let mut dispatch_budget = 0_u64;
    let mut tool_budget = 0_u64;
    let mut resource_budget = 0_u64;

    for child in children {
        if !grant_ids.insert(child.grant_id.as_str()) {
            violations.push(AttenuationViolation::DuplicateChildGrant);
        }
        if !task_ids.insert(child.task_id.as_str()) {
            violations.push(AttenuationViolation::DuplicateChildTask);
        }
        if let Err(mut edge_violations) = verify_delegation_at(parent, child, now_unix) {
            violations.append(&mut edge_violations);
        }

        match dispatch_budget.checked_add(child.max_dispatches) {
            Some(total) => dispatch_budget = total,
            None => {
                dispatch_budget = u64::MAX;
                violations.push(AttenuationViolation::SiblingBudgetOverflow);
            }
        }
        match tool_budget.checked_add(child.max_tool_calls) {
            Some(total) => tool_budget = total,
            None => {
                tool_budget = u64::MAX;
                violations.push(AttenuationViolation::SiblingBudgetOverflow);
            }
        }
        match resource_budget.checked_add(child.max_resource_units) {
            Some(total) => resource_budget = total,
            None => {
                resource_budget = u64::MAX;
                violations.push(AttenuationViolation::SiblingBudgetOverflow);
            }
        }
    }

    if dispatch_budget > parent.max_dispatches {
        violations.push(AttenuationViolation::SiblingDispatchBudgetExceeded);
    }
    if tool_budget > parent.max_tool_calls {
        violations.push(AttenuationViolation::SiblingToolBudgetExceeded);
    }
    if resource_budget > parent.max_resource_units {
        violations.push(AttenuationViolation::SiblingResourceBudgetExceeded);
    }

    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}

/// Verifies an entire root-authorized sibling batch and accounts for shared
/// budgets against the common immediate parent. The ancestors slice contains the
/// root through the immediate parent; children is the complete fan-out being
/// authorized in this decision. Repeated decisions still require durable
/// participant-owned budget state.
pub fn verify_root_anchored_children_at(
    ancestors: &[DelegatedAuthorityGrant],
    children: &[DelegatedAuthorityGrant],
    now_unix: i64,
) -> Result<(), Vec<AttenuationViolation>> {
    if children.is_empty() {
        return Ok(());
    }
    if ancestors.is_empty() {
        return Err(vec![AttenuationViolation::InvalidRootAnchoredChain]);
    }

    let parent = ancestors.last().expect("non-empty ancestors checked above");
    let mut violations = Vec::new();
    let mut grant_ids = BTreeSet::new();
    let mut task_ids = BTreeSet::new();
    let mut dispatch_budget = 0_u64;
    let mut tool_budget = 0_u64;
    let mut resource_budget = 0_u64;

    for child in children {
        if !grant_ids.insert(child.grant_id.as_str()) {
            violations.push(AttenuationViolation::DuplicateChildGrant);
        }
        if !task_ids.insert(child.task_id.as_str()) {
            violations.push(AttenuationViolation::DuplicateChildTask);
        }

        let mut chain = ancestors.to_vec();
        chain.push(child.clone());
        if let Err(mut edge_violations) = verify_root_anchored_chain_at(&chain, now_unix) {
            violations.append(&mut edge_violations);
        }

        match dispatch_budget.checked_add(child.max_dispatches) {
            Some(total) => dispatch_budget = total,
            None => {
                dispatch_budget = u64::MAX;
                violations.push(AttenuationViolation::SiblingBudgetOverflow);
            }
        }
        match tool_budget.checked_add(child.max_tool_calls) {
            Some(total) => tool_budget = total,
            None => {
                tool_budget = u64::MAX;
                violations.push(AttenuationViolation::SiblingBudgetOverflow);
            }
        }
        match resource_budget.checked_add(child.max_resource_units) {
            Some(total) => resource_budget = total,
            None => {
                resource_budget = u64::MAX;
                violations.push(AttenuationViolation::SiblingBudgetOverflow);
            }
        }
    }

    if dispatch_budget > parent.max_dispatches {
        violations.push(AttenuationViolation::SiblingDispatchBudgetExceeded);
    }
    if tool_budget > parent.max_tool_calls {
        violations.push(AttenuationViolation::SiblingToolBudgetExceeded);
    }
    if resource_budget > parent.max_resource_units {
        violations.push(AttenuationViolation::SiblingResourceBudgetExceeded);
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

    fn root_child_scope() -> RootAuthorizedTaskScope {
        RootAuthorizedTaskScope {
            tenant_binding: "tenant-1".into(),
            targets: set(&["service/api"]),
            operations: set(&["read"]),
            parameter_bounds: BTreeMap::from([
                ("replicas".into(), ParameterBound::IntegerRange { min: 2, max: 3 }),
                ("region".into(), ParameterBound::ExactString("af-south-1".into())),
            ]),
            data_scopes: set(&["metrics"]),
            not_before_unix: 120,
            expires_at_unix: 300,
            max_dispatches: 4,
            max_tool_calls: 20,
            max_resource_units: 100,
            max_risk_level: 1,
            required_approvals: set(&["change-ticket"]),
            required_postconditions: set(&["service-healthy", "replicas-converged"]),
            rollback_obligations: set(&["restore-replicas"]),
        }
    }

    fn root_grandchild_scope() -> RootAuthorizedTaskScope {
        RootAuthorizedTaskScope {
            tenant_binding: "tenant-1".into(),
            targets: set(&["service/worker"]),
            operations: set(&["restart"]),
            parameter_bounds: BTreeMap::from([
                ("replicas".into(), ParameterBound::IntegerRange { min: 4, max: 5 }),
                ("region".into(), ParameterBound::ExactString("af-south-1".into())),
            ]),
            data_scopes: set(&["service-config"]),
            not_before_unix: 140,
            expires_at_unix: 200,
            max_dispatches: 1,
            max_tool_calls: 5,
            max_resource_units: 20,
            max_risk_level: 1,
            required_approvals: set(&["change-ticket"]),
            required_postconditions: set(&["service-healthy", "replicas-converged"]),
            rollback_obligations: set(&["restore-replicas"]),
        }
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
            allowed_child_task_digests: set(&[&digest('c'), &digest('e')]),
            allowed_child_plan_digests: set(&[&digest('d'), &digest('f')]),
            allowed_child_task_plans: BTreeMap::from([(digest('c'), digest('d')), (digest('e'), digest('f'))]),
            allowed_descendant_task_plans: BTreeMap::from([(digest('c'), digest('d')), (digest('e'), digest('f'))]),
            authorized_delegation_edges: BTreeSet::from([
                RootDelegationEdge {
                    parent_task_digest: digest('a'),
                    child_task_digest: digest('c'),
                    child_plan_digest: digest('d'),
                    delegator_principal: "agent-a".into(),
                    delegatee_principal: "agent-b".into(),
                    authorized_scope: root_child_scope(),
                },
                RootDelegationEdge {
                    parent_task_digest: digest('c'),
                    child_task_digest: digest('e'),
                    child_plan_digest: digest('f'),
                    delegator_principal: "agent-b".into(),
                    delegatee_principal: "agent-c".into(),
                    authorized_scope: root_grandchild_scope(),
                },
                RootDelegationEdge {
                    parent_task_digest: digest('a'),
                    child_task_digest: digest('e'),
                    child_plan_digest: digest('f'),
                    delegator_principal: "agent-a".into(),
                    delegatee_principal: "agent-c".into(),
                    authorized_scope: root_grandchild_scope(),
                },
            ]),
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
            allowed_child_task_plans: BTreeMap::from([(digest('e'), digest('f'))]),
            allowed_descendant_task_plans: BTreeMap::new(),
            authorized_delegation_edges: BTreeSet::new(),
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
        child.targets = set(&["service/admin"]);
        child.operations = set(&["delete"]);
        child.data_scopes = set(&["secrets"]);
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
    fn child_grant_id_must_not_reuse_parent_grant_id() {
        let parent = parent_grant();
        let mut child = child_grant();
        child.grant_id = parent.grant_id.clone();
        assert!(check_attenuation(&parent, &child).unwrap_err().contains(
            &AttenuationViolation::ChildGrantIdNotDistinct
        ));
    }

    #[test]
    fn child_task_digest_must_be_precommitted_by_parent() {
        let parent = parent_grant();
        let mut child = child_grant();
        child.task_digest = digest('z');
        let errors = check_attenuation(&parent, &child).unwrap_err();
        assert!(errors.contains(&AttenuationViolation::ChildTaskDigestNotAllowed));
        assert!(errors.contains(&AttenuationViolation::ChildPlanDigestNotAllowed));
    }


    #[test]
    fn parent_child_task_and_plan_digests_must_be_an_exact_pair() {
        let mut parent = parent_grant();
        parent.allowed_child_task_digests.insert(digest('e'));
        parent.allowed_child_plan_digests.insert(digest('f'));
        parent.allowed_child_task_plans.insert(digest('e'), digest('d'));
        let mut child = second_child_grant();
        child.plan_digest = digest('f');

        // Both digests are individually present, but this exact combination
        // was never authorized by the parent.
        assert!(parent.allowed_child_task_digests.contains(&child.task_digest));
        assert!(parent.allowed_child_plan_digests.contains(&child.plan_digest));
        assert!(check_attenuation(&parent, &child).unwrap_err().contains(
            &AttenuationViolation::ChildPlanDigestNotAllowed
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
            allowed_child_task_plans: BTreeMap::new(),
            allowed_descendant_task_plans: BTreeMap::new(),
            authorized_delegation_edges: BTreeSet::new(),
            delegable: false,
            revoked: false,
        }
    }


    fn second_child_grant() -> DelegatedAuthorityGrant {
        let mut child = child_grant();
        child.grant_id = "grant-child-2".into();
        child.task_id = "task-child-2".into();
        child.task_digest = digest('e');
        child.plan_digest = digest('f');
        child.allowed_child_task_digests.clear();
        child.allowed_child_plan_digests.clear();
        child.allowed_child_task_plans.clear();
        child
    }


    fn root_reanchored_sibling(
        root: &DelegatedAuthorityGrant,
        parent: &DelegatedAuthorityGrant,
        suffix: char,
        target: &str,
        operation: &str,
    ) -> DelegatedAuthorityGrant {
        let mut child = second_child_grant();
        child.grant_id = format!("grant-hybrid-{suffix}");
        child.task_id = format!("task-hybrid-{suffix}");
        child.task_digest = digest(if suffix == 'g' { '1' } else { '3' });
        child.plan_digest = digest(if suffix == 'g' { '2' } else { '4' });
        child.issuer_principal = parent.subject_principal.clone();
        child.subject_principal = format!("agent-{suffix}");
        child.root_task_id = root.root_task_id.clone();
        child.root_task_digest = root.root_task_digest.clone();
        child.root_plan_digest = root.root_plan_digest.clone();
        child.parent_task_id = Some(parent.task_id.clone());
        child.parent_task_digest = Some(parent.task_digest.clone());
        child.parent_grant_id = Some(parent.grant_id.clone());
        child.targets = set(&[target]);
        child.operations = set(&[operation]);
        child.not_before_unix = 140;
        child.expires_at_unix = 200;
        child.max_dispatches = 1;
        child.max_tool_calls = 2;
        child.max_resource_units = 5;
        child.remaining_delegation_depth = 0;
        child.max_risk_level = 0;
        child.required_approvals = root.required_approvals.clone();
        child.required_postconditions = root.required_postconditions.clone();
        child.rollback_obligations = root.rollback_obligations.clone();
        child.allowed_child_task_digests = BTreeSet::new();
        child.allowed_child_plan_digests = BTreeSet::new();
        child.delegable = false;
        child.revoked = false;
        child
    }

    fn root_and_narrow_parent() -> (DelegatedAuthorityGrant, DelegatedAuthorityGrant) {
        let mut root = parent_grant();
        root.allowed_child_task_digests = set(&[&digest('c'), &digest('1'), &digest('3')]);
        root.allowed_child_plan_digests = set(&[&digest('d'), &digest('2'), &digest('4')]);
        root.allowed_child_task_plans = BTreeMap::from([
            (digest('c'), digest('d')),
            (digest('1'), digest('2')),
            (digest('3'), digest('4')),
        ]);
        let mut parent = child_grant();
        parent.allowed_child_task_digests = set(&[&digest('1'), &digest('3')]);
        parent.allowed_child_plan_digests = set(&[&digest('2'), &digest('4')]);
        parent.allowed_child_task_plans = BTreeMap::from([
            (digest('1'), digest('2')),
            (digest('3'), digest('4')),
        ]);
        (root, parent)
    }

    #[test]
    fn root_reanchor_allows_explicit_sibling_task_outside_parent_task_scope() {
        let (root, parent) = root_and_narrow_parent();
        // The parent is narrowed to service/api + read. The root pre-authorized
        // another task on service/worker + restart, so the child need not be
        // crippled by the parent's unrelated task-specific scope.
        let child =
            root_reanchored_sibling(&root, &parent, 'g', "service/worker", "restart");
        assert!(grant_shape_is_valid(&root), "root grant shape is invalid: {root:#?}");
        assert!(grant_shape_is_valid(&parent), "intermediate grant shape is invalid: {parent:#?}");
        assert!(grant_shape_is_valid(&child), "child grant shape is invalid: {child:#?}");
        assert!(child.targets.is_disjoint(&parent.targets));
        assert!(child.operations.is_disjoint(&parent.operations));
        assert_eq!(
            verify_root_reanchored_child_at(&[root, parent], &child, 150),
            Ok(())
        );
    }

    #[test]
    fn root_reanchor_still_denies_scope_outside_root_envelope() {
        let (root, parent) = root_and_narrow_parent();
        let child =
            root_reanchored_sibling(&root, &parent, 'g', "service/database", "restart");
        assert!(verify_root_reanchored_child_at(&[root, parent], &child, 150)
            .unwrap_err()
            .contains(&AttenuationViolation::TargetScopeExpanded));
    }

    #[test]
    fn root_reanchor_requires_root_precommitment_of_task_and_plan() {
        let (root, parent) = root_and_narrow_parent();
        let mut child =
            root_reanchored_sibling(&root, &parent, 'g', "service/worker", "restart");
        child.task_digest = digest('z');
        child.plan_digest = digest('y');
        let errors =
            verify_root_reanchored_child_at(&[root, parent], &child, 150).unwrap_err();
        assert!(errors.contains(&AttenuationViolation::ChildTaskDigestNotAllowed));
        assert!(errors.contains(&AttenuationViolation::ChildPlanDigestNotAllowed));
    }

    #[test]
    fn root_reanchor_requires_immediate_parent_precommitment_too() {
        let (root, mut parent) = root_and_narrow_parent();
        let child =
            root_reanchored_sibling(&root, &parent, 'g', "service/worker", "restart");
        parent.allowed_child_task_digests.clear();
        parent.allowed_child_plan_digests.clear();
        let errors =
            verify_root_reanchored_child_at(&[root, parent], &child, 150).unwrap_err();
        assert!(errors.contains(&AttenuationViolation::ChildTaskDigestNotAllowed));
        assert!(errors.contains(&AttenuationViolation::ChildPlanDigestNotAllowed));
    }

    #[test]
    fn root_reanchor_requires_delegable_active_parent_and_root() {
        let (mut root, mut parent) = root_and_narrow_parent();
        let child =
            root_reanchored_sibling(&root, &parent, 'g', "service/worker", "restart");
        parent.delegable = false;
        parent.remaining_delegation_depth = 0;
        assert!(verify_root_reanchored_child_at(&[root.clone(), parent.clone()], &child, 150)
            .unwrap_err()
            .contains(&AttenuationViolation::ParentCannotDelegate));

        parent.delegable = true;
        parent.remaining_delegation_depth = 1;
        root.revoked = true;
        assert!(verify_root_reanchored_child_at(&[root, parent], &child, 150)
            .unwrap_err()
            .contains(&AttenuationViolation::ParentRevoked));
    }

    #[test]
    fn root_reanchored_siblings_share_the_parent_budget_but_keep_task_scopes() {
        let (mut root, mut parent) = root_and_narrow_parent();
        root.allowed_child_task_digests.insert(digest('3'));
        root.allowed_child_plan_digests.insert(digest('4'));
        root.allowed_child_task_plans.insert(digest('3'), digest('4'));
        parent.max_dispatches = 3;
        parent.max_tool_calls = 6;
        parent.max_resource_units = 12;
        let first =
            root_reanchored_sibling(&root, &parent, 'g', "service/worker", "restart");
        let mut second =
            root_reanchored_sibling(&root, &parent, 'j', "service/api", "read");
        second.grant_id = "grant-hybrid-j".into();
        second.task_id = "task-hybrid-j".into();
        second.task_digest = digest('3');
        second.plan_digest = digest('4');
        second.max_dispatches = 1;
        second.max_tool_calls = 4;
        second.max_resource_units = 7;

        assert_eq!(
            verify_root_reanchored_children_at(&[root.clone(), parent.clone()], &[first.clone(), second.clone()], 150),
            Ok(())
        );
        let mut over = second;
        over.max_dispatches = 3;
        let errors = verify_root_reanchored_children_at(&[root, parent], &[first, over], 150)
            .unwrap_err();
        assert!(errors.contains(&AttenuationViolation::SiblingDispatchBudgetExceeded));
    }

    #[test]
    fn sibling_children_cannot_each_reuse_the_full_parent_budget() {
        let mut parent = parent_grant();
        parent.allowed_child_task_digests.insert(digest('e'));
        parent.allowed_child_plan_digests.insert(digest('f'));
        let mut first = child_grant();
        let mut second = second_child_grant();

        // Each grant independently stays within the parent's limits, but the
        // combined fan-out exceeds all three shared budgets.
        first.max_dispatches = 2;
        second.max_dispatches = 3;
        first.max_tool_calls = 10;
        second.max_tool_calls = 11;
        first.max_resource_units = 50;
        second.max_resource_units = 60;

        assert!(first.max_dispatches <= parent.max_dispatches);
        assert!(second.max_dispatches <= parent.max_dispatches);
        assert!(first.max_tool_calls <= parent.max_tool_calls);
        assert!(second.max_tool_calls <= parent.max_tool_calls);
        assert!(first.max_resource_units <= parent.max_resource_units);
        assert!(second.max_resource_units <= parent.max_resource_units);

        let errors = verify_delegated_children_at(&parent, &[first, second], 150)
            .expect_err("sibling budget reuse must be denied");
        assert!(errors.contains(&AttenuationViolation::SiblingDispatchBudgetExceeded));
        assert!(errors.contains(&AttenuationViolation::SiblingToolBudgetExceeded));
        assert!(errors.contains(&AttenuationViolation::SiblingResourceBudgetExceeded));
    }

    #[test]
    fn sibling_allocations_within_shared_budgets_are_accepted() {
        let mut parent = parent_grant();
        parent.allowed_child_task_digests.insert(digest('e'));
        parent.allowed_child_plan_digests.insert(digest('f'));
        let mut first = child_grant();
        let mut second = second_child_grant();

        first.max_dispatches = 2;
        second.max_dispatches = 2;
        first.max_tool_calls = 8;
        second.max_tool_calls = 12;
        first.max_resource_units = 40;
        second.max_resource_units = 60;
        // Distinct legitimate siblings may have different narrowed operations.
        second.operations = set(&["restart"]);
        second.targets = set(&["service/worker"]);

        assert_eq!(
            verify_delegated_children_at(&parent, &[first, second], 150),
            Ok(())
        );
    }



    #[test]
    fn root_anchor_allows_a_precommitted_sibling_scope_outside_parent_scope() {
        let root = parent_grant();
        let parent = child_grant();
        let mut child = grandchild_grant(&parent);

        // Parent was delegated API/read. The root explicitly pre-authorized a
        // distinct worker/restart subtask and the intermediate parent committed
        // to that exact child task+plan. Strict recursive attenuation blocks it.
        child.targets = set(&["service/worker"]);
        child.operations = set(&["restart"]);
        child.data_scopes = set(&["service-config"]);
        child.parameter_bounds.insert(
            "replicas".into(),
            ParameterBound::IntegerRange { min: 4, max: 5 },
        );
        child.max_dispatches = 1;
        child.max_tool_calls = 2;
        child.max_resource_units = 5;

        assert!(check_attenuation(&parent, &child).is_err());
        assert_eq!(
            verify_root_anchored_chain_at(&[root, parent, child], 150),
            Ok(())
        );
    }


    #[test]
    fn root_anchored_siblings_allow_precommitted_distinct_root_scopes() {
        let root = parent_grant();
        let first = child_grant();
        let mut second = second_child_grant();
        second.subject_principal = "agent-c".into();
        second.targets = set(&["service/worker"]);
        second.operations = set(&["restart"]);
        second.data_scopes = set(&["service-config"]);
        second.parameter_bounds.insert(
            "replicas".into(),
            ParameterBound::IntegerRange { min: 4, max: 5 },
        );
        second.not_before_unix = 140;
        second.expires_at_unix = 200;
        second.max_dispatches = 1;
        second.max_tool_calls = 5;
        second.max_resource_units = 20;

        let narrowed_parent = child_grant();
        let mut strict_candidate = second.clone();
        strict_candidate.issuer_principal = narrowed_parent.subject_principal.clone();
        strict_candidate.parent_task_id = Some(narrowed_parent.task_id.clone());
        strict_candidate.parent_task_digest = Some(narrowed_parent.task_digest.clone());
        strict_candidate.parent_grant_id = Some(narrowed_parent.grant_id.clone());
        let strict_errors = check_attenuation(&narrowed_parent, &strict_candidate).unwrap_err();
        assert!(strict_errors.contains(&AttenuationViolation::TargetScopeExpanded));
        assert!(strict_errors.contains(&AttenuationViolation::OperationScopeExpanded));

        assert_eq!(
            verify_root_anchored_children_at(&[root], &[first, second], 150),
            Ok(())
        );
    }

    #[test]
    fn root_anchored_siblings_share_parent_budgets() {
        let root = parent_grant();
        let mut first = child_grant();
        let mut second = second_child_grant();
        second.subject_principal = "agent-c".into();
        first.max_dispatches = 3;
        second.max_dispatches = 3;
        first.max_tool_calls = 12;
        second.max_tool_calls = 12;
        first.max_resource_units = 60;
        second.max_resource_units = 60;

        let errors = verify_root_anchored_children_at(&[root], &[first, second], 150)
            .expect_err("aggregate sibling allocations must not exceed root budget");
        assert!(errors.contains(&AttenuationViolation::SiblingDispatchBudgetExceeded));
        assert!(errors.contains(&AttenuationViolation::SiblingToolBudgetExceeded));
        assert!(errors.contains(&AttenuationViolation::SiblingResourceBudgetExceeded));
    }

    #[test]
    fn root_anchor_rejects_a_child_scope_outside_the_root_envelope() {
        let root = parent_grant();
        let parent = child_grant();
        let mut child = grandchild_grant(&parent);
        child.targets = set(&["service/admin"]);
        assert!(verify_root_anchored_chain_at(&[root, parent, child], 150)
            .unwrap_err()
            .contains(&AttenuationViolation::RootEnvelopeScopeExpanded));
    }

    #[test]
    fn root_anchor_requires_precommitted_descendant_task_and_plan() {
        let mut root = parent_grant();
        root.allowed_descendant_task_plans.remove(&digest('e'));
        let parent = child_grant();
        let child = grandchild_grant(&parent);
        assert!(verify_root_anchored_chain_at(&[root, parent, child], 150)
            .unwrap_err()
            .contains(&AttenuationViolation::RootDescendantTaskPlanNotAllowed));
    }



    #[test]
    fn root_anchor_rejects_scope_mutation_within_global_envelope() {
        let root = parent_grant();
        let parent = child_grant();
        let mut child = grandchild_grant(&parent);
        // API/read is still inside the global root envelope, but the exact
        // root-authorized subtask edge is worker/restart.
        child.targets = set(&["service/api"]);
        child.operations = set(&["read"]);
        child.data_scopes = set(&["metrics"]);
        child.parameter_bounds.insert(
            "replicas".into(),
            ParameterBound::IntegerRange { min: 2, max: 3 },
        );
        assert!(verify_root_anchored_chain_at(&[root, parent, child], 150)
            .unwrap_err()
            .contains(&AttenuationViolation::RootAuthorizedSubtaskScopeExpanded));
    }

    #[test]
    fn root_anchor_rejects_uncommitted_delegation_parent_or_recipient() {
        let root = parent_grant();
        let mut parent = child_grant();
        let mut child = grandchild_grant(&parent);
        child.subject_principal = "agent-unapproved".into();
        let errors = verify_root_anchored_chain_at(&[root, parent.clone(), child], 150)
            .unwrap_err();
        assert!(errors.contains(&AttenuationViolation::RootDelegationEdgeNotAllowed));

        // A root-approved task/plan pair is not sufficient if the wrong
        // intermediate agent is the delegator.
        parent.subject_principal = "agent-wrong".into();
        let child = grandchild_grant(&parent);
        let root = parent_grant();
        assert!(verify_root_anchored_chain_at(&[root, parent, child], 150)
            .unwrap_err()
            .contains(&AttenuationViolation::RootDelegationEdgeNotAllowed));
    }

    #[test]
    fn root_anchor_still_requires_the_immediate_parent_to_delegate_exact_child() {
        let root = parent_grant();
        let mut parent = child_grant();
        parent.allowed_child_plan_digests.clear();
        let child = grandchild_grant(&parent);
        assert!(verify_root_anchored_chain_at(&[root, parent, child], 150)
            .unwrap_err()
            .contains(&AttenuationViolation::ChildPlanDigestNotAllowed));
    }

    #[test]
    fn sibling_budget_overflow_fails_closed() {
        let mut parent = parent_grant();
        parent.allowed_child_task_digests.insert(digest('e'));
        parent.allowed_child_plan_digests.insert(digest('f'));
        parent.max_tool_calls = u64::MAX;
        parent.max_resource_units = u64::MAX;

        let mut first = child_grant();
        let mut second = second_child_grant();
        first.max_tool_calls = u64::MAX;
        second.max_tool_calls = u64::MAX;
        first.max_resource_units = u64::MAX;
        second.max_resource_units = u64::MAX;

        let errors = verify_delegated_children_at(&parent, &[first, second], 150)
            .expect_err("overflowed aggregate budgets must fail closed");
        assert!(errors.contains(&AttenuationViolation::SiblingBudgetOverflow));
    }

    #[test]
    fn sibling_fanout_rejects_duplicate_grant_and_task_ids() {
        let mut parent = parent_grant();
        parent.allowed_child_task_digests.insert(digest('e'));
        parent.allowed_child_plan_digests.insert(digest('f'));

        let first = child_grant();
        let mut duplicate_task = first.clone();
        duplicate_task.grant_id = "grant-child-distinct".into();
        let task_errors =
            verify_delegated_children_at(&parent, &[first.clone(), duplicate_task], 150)
                .expect_err("duplicate task identity must not qualify");
        assert!(task_errors.contains(&AttenuationViolation::DuplicateChildTask));

        let mut duplicate_grant = second_child_grant();
        duplicate_grant.grant_id = first.grant_id.clone();
        let grant_errors =
            verify_delegated_children_at(&parent, &[first, duplicate_grant], 150)
                .expect_err("duplicate grant identity must not qualify");
        assert!(grant_errors.contains(&AttenuationViolation::DuplicateChildGrant));
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

}
