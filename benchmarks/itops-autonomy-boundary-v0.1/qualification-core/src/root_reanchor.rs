//! Candidate root-re-anchored authorization policy for multi-agent ITOPS.
//!
//! Unlike strict edge-by-edge scope attenuation, this policy treats the root
//! grant as the authority envelope for an explicitly precommitted task graph.
//! Every hop still requires exact task/plan lineage, a parent precommitment,
//! current ancestor authority, and delegation-depth validity. A pending effect
//! is then checked against both the root envelope and the exact acting leaf
//! subtask grant. Intermediate agents' narrower *local effect* scopes are not
//! mechanically intersected with a separately preauthorized leaf task.
//!
//! This is an experimental synthetic policy arm, not a production Xenia
//! adapter, signature verifier, or proof that supplied authority observations
//! are fresh and authentic.

use crate::delegation::{
    digest_is_sha256, parameter_bound_is_subset, DelegatedAuthorityGrant, ParameterBound,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorityState {
    Active,
    Revoked,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityObservation {
    pub state: AuthorityState,
    pub checked_at_unix: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingEffectRequest {
    pub root_task_id: String,
    pub root_task_digest: String,
    pub root_plan_digest: String,
    pub leaf_task_id: String,
    pub leaf_task_digest: String,
    pub leaf_plan_digest: String,
    pub request_digest: String,
    pub target: String,
    pub operation: String,
    /// The actual request is encoded as exact values or narrower constraints.
    pub parameter_values: BTreeMap<String, ParameterBound>,
    pub data_scopes: BTreeSet<String>,
    pub requested_dispatches: u64,
    pub requested_tool_calls: u64,
    pub requested_resource_units: u64,
    pub risk_level: u8,
    pub satisfied_approvals: BTreeSet<String>,
    pub now_unix: i64,
    pub max_authority_age_seconds: u64,
    /// Canonical authority-state observations, one per grant in the chain.
    pub current_authority: BTreeMap<String, AuthorityObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RootReanchorDenial {
    ChainTooShort,
    InvalidGrantShape(String),
    DuplicateGrantId,
    DuplicateTaskId,
    RootGrantHasParent,
    RootTaskBindingMismatch,
    RootPlanBindingMismatch,
    RootTaskGraphDoesNotPrecommitDescendant(String),
    ParentTaskLinkMismatch(String),
    ParentTaskDigestMismatch(String),
    ParentGrantLinkMismatch(String),
    PrincipalChainMismatch(String),
    ParentCannotDelegate(String),
    ChildTaskNotPrecommittedByParent(String),
    ChildPlanNotPrecommittedByParent(String),
    TenantChanged(String),
    GrantRevoked(String),
    AuthorityObservationMissing(String),
    AuthorityObservationUnknown(String),
    AuthorityObservationRevoked(String),
    AuthorityObservationStaleOrFuture(String),
    AuthorityObservationCountMismatch,
    GrantNotYetValid(String),
    GrantExpired(String),
    RootEffectScopeDenied,
    LeafEffectScopeDenied,
    RequestParameterNotBound(String),
    RequestParameterOutOfScope(String),
    RequestDataScopeOutOfScope,
    DispatchBudgetExceeded,
    ToolBudgetExceeded,
    ResourceBudgetExceeded,
    RiskCeilingExceeded,
    ApprovalMissing(String),
    LeafObligationWeakened,
    InvalidRequestDigest,
    InvalidRequestIdentity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootReanchoredPermit {
    pub root_grant_id: String,
    pub leaf_grant_id: String,
    pub root_task_id: String,
    pub root_task_digest: String,
    pub root_plan_digest: String,
    pub leaf_task_id: String,
    pub leaf_task_digest: String,
    pub leaf_plan_digest: String,
    pub request_digest: String,
}

/// Authorizes one pending effect against the root-authorized task graph and
/// the leaf's exact subtask envelope.
///
/// All ancestors must remain active and current. Each parent must precommit
/// the next child's task and plan digests; the root grant's child-digest sets
/// are interpreted as the complete root-authorized descendant manifest.
/// Scope and effect budgets are checked against the root and acting leaf,
/// not mechanically intersected through intermediate agents' local roles.
pub fn authorize_root_reanchored_effect(
    chain: &[DelegatedAuthorityGrant],
    request: &PendingEffectRequest,
) -> Result<RootReanchoredPermit, Vec<RootReanchorDenial>> {
    let mut denials = Vec::new();
    if chain.len() < 2 {
        return Err(vec![RootReanchorDenial::ChainTooShort]);
    }

    let root = &chain[0];
    let Some(leaf) = chain.last() else {
        return Err(vec![RootReanchorDenial::ChainTooShort]);
    };

    let mut grant_ids = BTreeSet::new();
    let mut task_ids = BTreeSet::new();

    for grant in chain {
        if grant.grant_id.trim().is_empty()
            || grant.issuer_principal.trim().is_empty()
            || grant.subject_principal.trim().is_empty()
            || grant.task_id.trim().is_empty()
            || grant.root_task_id.trim().is_empty()
            || grant.tenant_binding.trim().is_empty()
            || !digest_is_sha256(&grant.root_task_digest)
            || !digest_is_sha256(&grant.root_plan_digest)
            || !digest_is_sha256(&grant.task_digest)
            || !digest_is_sha256(&grant.plan_digest)
            || grant.not_before_unix >= grant.expires_at_unix
            || grant.max_tool_calls == 0
            || grant.max_resource_units == 0
            || grant.targets.is_empty()
            || grant.operations.is_empty()
            || grant.data_scopes.is_empty()
        {
            denials.push(RootReanchorDenial::InvalidGrantShape(grant.grant_id.clone()));
        }
        if !grant_ids.insert(grant.grant_id.as_str()) {
            denials.push(RootReanchorDenial::DuplicateGrantId);
        }
        if !task_ids.insert(grant.task_id.as_str()) {
            denials.push(RootReanchorDenial::DuplicateTaskId);
        }
        if grant.revoked {
            denials.push(RootReanchorDenial::GrantRevoked(grant.grant_id.clone()));
        }
        if request.now_unix < grant.not_before_unix {
            denials.push(RootReanchorDenial::GrantNotYetValid(grant.grant_id.clone()));
        }
        if request.now_unix >= grant.expires_at_unix {
            denials.push(RootReanchorDenial::GrantExpired(grant.grant_id.clone()));
        }
        if grant.root_task_id != root.root_task_id
            || grant.root_task_digest != root.root_task_digest
        {
            denials.push(RootReanchorDenial::RootTaskBindingMismatch);
        }
        if grant.root_plan_digest != root.root_plan_digest {
            denials.push(RootReanchorDenial::RootPlanBindingMismatch);
        }
        if grant.tenant_binding != root.tenant_binding {
            denials.push(RootReanchorDenial::TenantChanged(grant.grant_id.clone()));
        }
    }

    if root.parent_task_id.is_some()
        || root.parent_task_digest.is_some()
        || root.parent_grant_id.is_some()
    {
        denials.push(RootReanchorDenial::RootGrantHasParent);
    }
    if root.task_id != root.root_task_id || root.task_digest != root.root_task_digest {
        denials.push(RootReanchorDenial::RootTaskBindingMismatch);
    }
    if root.plan_digest != root.root_plan_digest {
        denials.push(RootReanchorDenial::RootPlanBindingMismatch);
    }

    // Verify exact lineage and immediate-parent precommitment at every hop.
    for (index, pair) in chain.windows(2).enumerate() {
        let parent = &pair[0];
        let child = &pair[1];

        if !parent.delegable || parent.remaining_delegation_depth == 0 {
            denials.push(RootReanchorDenial::ParentCannotDelegate(parent.grant_id.clone()));
        }
        if child.parent_task_id.as_deref() != Some(parent.task_id.as_str()) {
            denials.push(RootReanchorDenial::ParentTaskLinkMismatch(child.grant_id.clone()));
        }
        if child.parent_task_digest.as_deref() != Some(parent.task_digest.as_str()) {
            denials.push(RootReanchorDenial::ParentTaskDigestMismatch(child.grant_id.clone()));
        }
        if child.parent_grant_id.as_deref() != Some(parent.grant_id.as_str()) {
            denials.push(RootReanchorDenial::ParentGrantLinkMismatch(child.grant_id.clone()));
        }
        if child.issuer_principal != parent.subject_principal {
            denials.push(RootReanchorDenial::PrincipalChainMismatch(child.grant_id.clone()));
        }
        if child.remaining_delegation_depth
            > parent.remaining_delegation_depth.saturating_sub(1)
        {
            denials.push(RootReanchorDenial::ParentCannotDelegate(parent.grant_id.clone()));
        }
        if !parent.allowed_child_task_digests.contains(&child.task_digest) {
            denials.push(RootReanchorDenial::ChildTaskNotPrecommittedByParent(
                child.grant_id.clone(),
            ));
        }
        if !parent.allowed_child_plan_digests.contains(&child.plan_digest) {
            denials.push(RootReanchorDenial::ChildPlanNotPrecommittedByParent(
                child.grant_id.clone(),
            ));
        }
        // The root manifest lists all descendants in the preauthorized task graph,
        // not only its immediate children. This permits explicitly authorized
        // sibling work without inheriting a narrower intermediate effect role.
        if !root.allowed_child_task_digests.contains(&child.task_digest)
            || !root.allowed_child_plan_digests.contains(&child.plan_digest)
        {
            denials.push(RootReanchorDenial::RootTaskGraphDoesNotPrecommitDescendant(
                child.task_id.clone(),
            ));
        }
        if index + 1 < chain.len() && !child.delegable && child.remaining_delegation_depth > 0 {
            denials.push(RootReanchorDenial::InvalidGrantShape(child.grant_id.clone()));
        }
    }

    // Authority observations must be fresh for every ancestor, including
    // intermediates whose local effect scope is not used for this policy arm.
    if request.current_authority.len() != chain.len() {
        denials.push(RootReanchorDenial::AuthorityObservationCountMismatch);
    }
    for grant in chain {
        match request.current_authority.get(&grant.grant_id) {
            None => denials.push(RootReanchorDenial::AuthorityObservationMissing(
                grant.grant_id.clone(),
            )),
            Some(observation) => {
                if observation.checked_at_unix > request.now_unix
                    || request
                        .now_unix
                        .saturating_sub(observation.checked_at_unix) as u64
                        > request.max_authority_age_seconds
                {
                    denials.push(RootReanchorDenial::AuthorityObservationStaleOrFuture(
                        grant.grant_id.clone(),
                    ));
                }
                match observation.state {
                    AuthorityState::Active => {}
                    AuthorityState::Revoked => {
                        denials.push(RootReanchorDenial::AuthorityObservationRevoked(
                            grant.grant_id.clone(),
                        ));
                    }
                    AuthorityState::Unknown => {
                        denials.push(RootReanchorDenial::AuthorityObservationUnknown(
                            grant.grant_id.clone(),
                        ));
                    }
                }
            }
        }
    }

    if request.root_task_id != root.root_task_id
        || request.root_task_digest != root.root_task_digest
        || request.root_plan_digest != root.root_plan_digest
        || request.leaf_task_id != leaf.task_id
        || request.leaf_task_digest != leaf.task_digest
        || request.leaf_plan_digest != leaf.plan_digest
    {
        denials.push(RootReanchorDenial::InvalidRequestIdentity);
    }
    if request.request_digest.strip_prefix("sha256:").is_none_or(|digest| {
        digest.len() != 64
            || !digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    }) {
        denials.push(RootReanchorDenial::InvalidRequestDigest);
    }

    let request_data_is_within = |grant: &DelegatedAuthorityGrant| {
        !request.target.trim().is_empty()
            && !request.operation.trim().is_empty()
            && grant.targets.contains(&request.target)
            && grant.operations.contains(&request.operation)
            && request.data_scopes.is_subset(&grant.data_scopes)
            && !request.data_scopes.is_empty()
    };
    if !request_data_is_within(root) {
        denials.push(RootReanchorDenial::RootEffectScopeDenied);
    }
    if !request_data_is_within(leaf) {
        denials.push(RootReanchorDenial::LeafEffectScopeDenied);
    }

    // Every parameter constraint declared by either endpoint envelope must
    // be present in the exact request, and the request must be a subset of both.
    for (name, allowed) in &root.parameter_bounds {
        match request.parameter_values.get(name) {
            Some(value) if parameter_bound_is_subset(value, allowed) => {}
            Some(_) | None => {
                denials.push(RootReanchorDenial::RequestParameterOutOfScope(name.clone()));
            }
        }
    }
    for (name, allowed) in &leaf.parameter_bounds {
        match request.parameter_values.get(name) {
            Some(value) if parameter_bound_is_subset(value, allowed) => {}
            Some(_) | None => {
                denials.push(RootReanchorDenial::RequestParameterOutOfScope(name.clone()));
            }
        }
    }
    for name in request.parameter_values.keys() {
        if !root.parameter_bounds.contains_key(name) || !leaf.parameter_bounds.contains_key(name) {
            denials.push(RootReanchorDenial::RequestParameterNotBound(name.clone()));
        }
    }

    if request.requested_dispatches == 0 || request.requested_dispatches > root.max_dispatches
        || request.requested_dispatches > leaf.max_dispatches
    {
        denials.push(RootReanchorDenial::DispatchBudgetExceeded);
    }
    if request.requested_tool_calls > root.max_tool_calls
        || request.requested_tool_calls > leaf.max_tool_calls
    {
        denials.push(RootReanchorDenial::ToolBudgetExceeded);
    }
    if request.requested_resource_units > root.max_resource_units
        || request.requested_resource_units > leaf.max_resource_units
    {
        denials.push(RootReanchorDenial::ResourceBudgetExceeded);
    }
    if request.risk_level > root.max_risk_level || request.risk_level > leaf.max_risk_level {
        denials.push(RootReanchorDenial::RiskCeilingExceeded);
    }
    if !root.required_approvals.is_subset(&leaf.required_approvals)
        || !root.required_postconditions.is_subset(&leaf.required_postconditions)
        || !root.rollback_obligations.is_subset(&leaf.rollback_obligations)
    {
        denials.push(RootReanchorDenial::LeafObligationWeakened);
    }
    for approval in root.required_approvals.iter().chain(leaf.required_approvals.iter()) {
        if !request.satisfied_approvals.contains(approval) {
            denials.push(RootReanchorDenial::ApprovalMissing(approval.clone()));
        }
    }

    if !denials.is_empty() {
        return Err(denials);
    }

    Ok(RootReanchoredPermit {
        root_grant_id: root.grant_id.clone(),
        leaf_grant_id: leaf.grant_id.clone(),
        root_task_id: root.root_task_id.clone(),
        root_task_digest: root.root_task_digest.clone(),
        root_plan_digest: root.root_plan_digest.clone(),
        leaf_task_id: leaf.task_id.clone(),
        leaf_task_digest: leaf.task_digest.clone(),
        leaf_plan_digest: leaf.plan_digest.clone(),
        request_digest: request.request_digest.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::delegation::ParameterBound;
    use std::iter::FromIterator;

    fn digest(ch: char) -> String {
        std::iter::repeat_n(ch, 64).collect()
    }

    fn set(values: &[&str]) -> BTreeSet<String> {
        BTreeSet::from_iter(values.iter().map(|value| (*value).to_string()))
    }

    fn grants() -> Vec<DelegatedAuthorityGrant> {
        let root = DelegatedAuthorityGrant {
            grant_id: "grant-root".into(),
            issuer_principal: "xenia".into(),
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
            required_postconditions: set(&["service-healthy"]),
            rollback_obligations: set(&["restore-service"]),
            allowed_child_task_digests: set(&[&digest('c'), &digest('e')]),
            allowed_child_plan_digests: set(&[&digest('d'), &digest('f')]),
            delegable: true,
            revoked: false,
        };
        let middle = DelegatedAuthorityGrant {
            grant_id: "grant-middle".into(),
            issuer_principal: "agent-a".into(),
            subject_principal: "agent-b".into(),
            root_task_id: root.root_task_id.clone(),
            root_task_digest: root.root_task_digest.clone(),
            root_plan_digest: root.root_plan_digest.clone(),
            task_id: "task-middle".into(),
            task_digest: digest('c'),
            plan_digest: digest('d'),
            parent_task_id: Some(root.task_id.clone()),
            parent_task_digest: Some(root.task_digest.clone()),
            parent_grant_id: Some(root.grant_id.clone()),
            tenant_binding: root.tenant_binding.clone(),
            targets: set(&["service/api"]),
            operations: set(&["read"]),
            parameter_bounds: BTreeMap::from([
                ("replicas".into(), ParameterBound::ExactInteger(1)),
                ("region".into(), ParameterBound::ExactString("af-south-1".into())),
            ]),
            data_scopes: set(&["metrics"]),
            not_before_unix: 120,
            expires_at_unix: 400,
            max_dispatches: 0,
            max_tool_calls: 3,
            max_resource_units: 10,
            remaining_delegation_depth: 1,
            max_risk_level: 1,
            required_approvals: set(&["change-ticket"]),
            required_postconditions: set(&["service-healthy"]),
            rollback_obligations: set(&["restore-service"]),
            allowed_child_task_digests: set(&[&digest('e')]),
            allowed_child_plan_digests: set(&[&digest('f')]),
            delegable: true,
            revoked: false,
        };
        let leaf = DelegatedAuthorityGrant {
            grant_id: "grant-leaf".into(),
            issuer_principal: "agent-b".into(),
            subject_principal: "agent-c".into(),
            root_task_id: root.root_task_id.clone(),
            root_task_digest: root.root_task_digest.clone(),
            root_plan_digest: root.root_plan_digest.clone(),
            task_id: "task-leaf".into(),
            task_digest: digest('e'),
            plan_digest: digest('f'),
            parent_task_id: Some(middle.task_id.clone()),
            parent_task_digest: Some(middle.task_digest.clone()),
            parent_grant_id: Some(middle.grant_id.clone()),
            tenant_binding: root.tenant_binding.clone(),
            targets: set(&["service/worker"]),
            operations: set(&["restart"]),
            parameter_bounds: BTreeMap::from([
                ("replicas".into(), ParameterBound::IntegerRange { min: 4, max: 5 }),
                ("region".into(), ParameterBound::ExactString("af-south-1".into())),
            ]),
            data_scopes: set(&["service-config"]),
            not_before_unix: 140,
            expires_at_unix: 300,
            max_dispatches: 1,
            max_tool_calls: 5,
            max_resource_units: 20,
            remaining_delegation_depth: 0,
            max_risk_level: 1,
            required_approvals: set(&["change-ticket", "service-owner"]),
            required_postconditions: set(&["service-healthy", "restart-confirmed"]),
            rollback_obligations: set(&["restore-service", "verify-rollback"]),
            allowed_child_task_digests: BTreeSet::new(),
            allowed_child_plan_digests: BTreeSet::new(),
            delegable: false,
            revoked: false,
        };
        vec![root, middle, leaf]
    }

    fn request(chain: &[DelegatedAuthorityGrant]) -> PendingEffectRequest {
        let root = &chain[0];
        let leaf = chain.last().unwrap();
        let current_authority = chain.iter().map(|grant| {
            (
                grant.grant_id.clone(),
                AuthorityObservation {
                    state: AuthorityState::Active,
                    checked_at_unix: 1_998,
                },
            )
        }).collect();
        PendingEffectRequest {
            root_task_id: root.root_task_id.clone(),
            root_task_digest: root.root_task_digest.clone(),
            root_plan_digest: root.root_plan_digest.clone(),
            leaf_task_id: leaf.task_id.clone(),
            leaf_task_digest: leaf.task_digest.clone(),
            leaf_plan_digest: leaf.plan_digest.clone(),
            request_digest: format!("sha256:{}", digest('7')),
            target: "service/worker".into(),
            operation: "restart".into(),
            parameter_values: BTreeMap::from([
                ("replicas".into(), ParameterBound::ExactInteger(4)),
                ("region".into(), ParameterBound::ExactString("af-south-1".into())),
            ]),
            data_scopes: set(&["service-config"]),
            requested_dispatches: 1,
            requested_tool_calls: 2,
            requested_resource_units: 10,
            risk_level: 1,
            satisfied_approvals: set(&["change-ticket", "service-owner"]),
            now_unix: 2_000,
            max_authority_age_seconds: 5,
            current_authority,
        }
    }

    #[test]
    fn root_reanchor_accepts_explicit_descendant_even_when_middle_role_is_narrower() {
        let chain = grants();
        let request = request(&chain);
        let strict = crate::delegation::verify_delegation_chain_at(&chain, request.now_unix);
        assert!(strict.is_err(), "strict attenuation arm should block the narrower middle role");
        let permit = authorize_root_reanchored_effect(&chain, &request)
            .expect("root-authorized task graph and leaf effect envelope should pass");
        assert_eq!(permit.root_grant_id, "grant-root");
        assert_eq!(permit.leaf_grant_id, "grant-leaf");
        assert_eq!(permit.request_digest, request.request_digest);
    }

    #[test]
    fn root_reanchor_rejects_descendant_not_in_root_task_graph_manifest() {
        let chain = grants();
        let mut request = request(&chain);
        request.current_authority.get_mut("grant-middle").unwrap().checked_at_unix = 2_000;
        let mut chain = chain;
        chain[0].allowed_child_task_digests.remove(&digest('e'));
        let error = authorize_root_reanchored_effect(&chain, &request)
            .expect_err("root task graph must precommit the leaf task digest");
        assert!(error.iter().any(|item| matches!(
            item,
            RootReanchorDenial::RootTaskGraphDoesNotPrecommitDescendant(_)
        )));
    }

    #[test]
    fn root_reanchor_rejects_leaf_scope_expansion_beyond_root() {
        let mut chain = grants();
        chain[2].operations = set(&["delete_database"]);
        let request = request(&chain);
        let error = authorize_root_reanchored_effect(&chain, &request)
            .expect_err("leaf operation must be bounded by root envelope");
        assert!(error.contains(&RootReanchorDenial::LeafEffectScopeDenied));
    }

    #[test]
    fn root_reanchor_rejects_revoked_intermediate_authority() {
        let chain = grants();
        let mut request = request(&chain);
        request.current_authority.get_mut("grant-middle").unwrap().state = AuthorityState::Revoked;
        let error = authorize_root_reanchored_effect(&chain, &request)
            .expect_err("every ancestor must remain active at final effect");
        assert!(error.iter().any(|item| matches!(
            item,
            RootReanchorDenial::AuthorityObservationRevoked(id) if id == "grant-middle"
        )));
    }

    #[test]
    fn root_reanchor_rejects_missing_stale_or_unknown_ancestor_authority() {
        let chain = grants();
        let mut missing = request(&chain);
        missing.current_authority.remove("grant-middle");
        assert!(authorize_root_reanchored_effect(&chain, &missing)
            .unwrap_err()
            .contains(&RootReanchorDenial::AuthorityObservationCountMismatch));

        let mut stale = request(&chain);
        stale.current_authority.get_mut("grant-middle").unwrap().checked_at_unix = 1_000;
        assert!(stale_authority_is_denied(&chain, stale));

        let mut unknown = request(&chain);
        unknown.current_authority.get_mut("grant-middle").unwrap().state = AuthorityState::Unknown;
        assert!(authorize_root_reanchored_effect(&chain, &unknown)
            .unwrap_err()
            .contains(&RootReanchorDenial::AuthorityObservationUnknown("grant-middle".into())));
    }

    fn stale_authority_is_denied(
        chain: &[DelegatedAuthorityGrant],
        request: PendingEffectRequest,
    ) -> bool {
        authorize_root_reanchored_effect(chain, &request)
            .unwrap_err()
            .contains(&RootReanchorDenial::AuthorityObservationStaleOrFuture("grant-middle".into()))
    }
}
