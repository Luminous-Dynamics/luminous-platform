#![forbid(unsafe_code)]
//! Independent typed reference evaluator for the Web4 assurance-prerequisite
//! admission contract (APAC) v0.1.
//!
//! Inputs are normalized synthetic facts. This crate is not a production
//! authorization service, cryptographic verifier, participant adapter, or proof
//! of distributed atomicity.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision { Eligible, Denied, Indeterminate, NotRequired }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RevalidationState { NotAttempted, Passed, Failed, Indeterminate }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lifecycle {
    Current, Expired, Invalidated, Withdrawn, Superseded,
    RequiresRequalification, Indeterminate, Forked,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Applicability {
    ExactSubject, ControlledEquivalent, ConditionallyTransferable,
    RequiresTargetedRevalidation, HistoricalOnly, Inapplicable, Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Currentness {
    TerminalInSuppliedView, CurrentThroughVerifiedCheckpoint,
    NotRequiredByProfile, Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DependencyCoverage { Complete, Partial, Unknown }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthorityState { Current, Revoked, Expired, Denied, Unknown }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Claim {
    pub id: String,
    pub property: String,
    pub subject_digest: String,
    pub qualification_ref: String,
    pub qualification_digest: String,
    pub predecessor_ref: Option<String>,
    pub predecessor_digest: Option<String>,
    pub lifecycle: Lifecycle,
    pub applicability: Applicability,
    pub acceptable: Vec<Applicability>,
    pub bridge_ref: Option<String>,
    pub subject_matches: bool,
    pub dependency_coverage: DependencyCoverage,
    pub unresolved_dependencies: u32,
    pub required_currentness: Currentness,
    pub actual_currentness: Currentness,
}

#[derive(Clone, Debug)]
pub struct AssuranceReceipt {
    pub state: RevalidationState,
    pub result_ref: Option<String>,
    pub result_digest: Option<String>,
    pub request_digest: Option<String>,
    pub prerequisite_set_digest: Option<String>,
    pub snapshot_digest: Option<String>,
    pub validated_claims: Vec<Claim>,
}

#[derive(Clone, Debug)]
pub struct AssuranceInput<'a> {
    pub request_digest: &'a str,
    pub preflight_request_digest: &'a str,
    pub commit_request_digest: &'a str,
    pub policy_digest: &'a str,
    pub request_policy_digest: &'a str,
    pub preflight_policy_digest: &'a str,
    pub commit_policy_digest: &'a str,
    pub set_digest: &'a str,
    pub request_set_digest: &'a str,
    pub preflight_set_digest: &'a str,
    pub commit_set_digest: &'a str,
    pub explicitly_declared: bool,
    pub required: bool,
    pub intentionally_empty: bool,
    pub allow_explicit_empty: bool,
    pub require_complete_dependencies: bool,
    pub preflight_snapshot: &'a str,
    pub commit_snapshot: &'a str,
    pub claims: &'a [Claim],
    pub receipt: &'a AssuranceReceipt,
}

#[derive(Clone, Debug)]
pub struct AuthorityReceipt {
    pub state: RevalidationState,
    pub result_ref: Option<String>,
    pub result_digest: Option<String>,
    pub request_digest: Option<String>,
    pub grant_ref: Option<String>,
    pub snapshot_digest: Option<String>,
    pub authority_state: Option<AuthorityState>,
}

#[derive(Clone, Debug)]
pub struct AuthorityInput<'a> {
    pub request_digest: &'a str,
    pub preflight_request_digest: &'a str,
    pub commit_request_digest: &'a str,
    pub grant_ref: &'a str,
    pub preflight_grant_ref: &'a str,
    pub commit_grant_ref: &'a str,
    pub preflight_snapshot: &'a str,
    pub commit_snapshot: &'a str,
    pub state_at_commit: AuthorityState,
    pub receipt: &'a AuthorityReceipt,
}

fn present(value: &Option<String>) -> bool {
    value.as_ref().is_some_and(|s| !s.is_empty())
}

fn assurance_receipt_bound(input: &AssuranceInput<'_>, changed: bool) -> bool {
    let r = input.receipt;
    if !changed {
        return r.state == RevalidationState::NotAttempted
            && r.result_ref.is_none() && r.result_digest.is_none()
            && r.request_digest.is_none() && r.prerequisite_set_digest.is_none()
            && r.snapshot_digest.is_none() && r.validated_claims.is_empty();
    }
    r.state != RevalidationState::NotAttempted
        && present(&r.result_ref) && present(&r.result_digest)
        && r.request_digest.as_deref() == Some(input.request_digest)
        && r.prerequisite_set_digest.as_deref() == Some(input.set_digest)
        && r.snapshot_digest.as_deref() == Some(input.commit_snapshot)
}

fn exact_fresh_claims(old: &[Claim], fresh: &[Claim]) -> bool {
    if old.len() != fresh.len() { return false; }
    for prior in old {
        let mut matches = fresh.iter().filter(|c| c.id == prior.id);
        let Some(next) = matches.next() else { return false; };
        if matches.next().is_some()
            || next.property != prior.property
            || next.subject_digest != prior.subject_digest {
            return false;
        }
        let changed = next.qualification_ref != prior.qualification_ref
            || next.qualification_digest != prior.qualification_digest;
        if changed {
            if next.predecessor_ref.as_deref() != Some(prior.qualification_ref.as_str())
                || next.predecessor_digest.as_deref() != Some(prior.qualification_digest.as_str())
                || next.qualification_ref == prior.qualification_ref
                || next.qualification_digest == prior.qualification_digest {
                return false;
            }
        } else if next.predecessor_ref.is_some() || next.predecessor_digest.is_some() {
            return false;
        }
    }
    true
}

fn evaluate_claims(claims: &[Claim], require_complete: bool) -> Decision {
    let mut uncertain = false;
    for c in claims {
        if !c.subject_matches { return Decision::Denied; }
        match c.lifecycle {
            Lifecycle::Expired | Lifecycle::Invalidated | Lifecycle::Withdrawn
            | Lifecycle::Superseded | Lifecycle::RequiresRequalification
            | Lifecycle::Forked => return Decision::Denied,
            Lifecycle::Indeterminate => { uncertain = true; continue; }
            Lifecycle::Current => {}
        }
        if require_complete && (c.dependency_coverage != DependencyCoverage::Complete
            || c.unresolved_dependencies != 0) {
            uncertain = true;
            continue;
        }
        match c.applicability {
            Applicability::Unknown => { uncertain = true; continue; }
            Applicability::HistoricalOnly | Applicability::Inapplicable
            | Applicability::RequiresTargetedRevalidation => return Decision::Denied,
            value => {
                if !c.acceptable.contains(&value) { return Decision::Denied; }
                if value == Applicability::ConditionallyTransferable && c.bridge_ref.is_none() {
                    return Decision::Denied;
                }
            }
        }
        match (c.required_currentness, c.actual_currentness) {
            (_, Currentness::Unknown) => uncertain = true,
            (Currentness::CurrentThroughVerifiedCheckpoint, actual)
                if actual != Currentness::CurrentThroughVerifiedCheckpoint => return Decision::Denied,
            (Currentness::TerminalInSuppliedView, actual)
                if actual != Currentness::TerminalInSuppliedView
                    && actual != Currentness::CurrentThroughVerifiedCheckpoint => return Decision::Denied,
            _ => {}
        }
    }
    if uncertain { Decision::Indeterminate } else { Decision::Eligible }
}

pub fn evaluate_assurance(input: &AssuranceInput<'_>) -> Decision {
    if input.request_digest != input.preflight_request_digest
        || input.request_digest != input.commit_request_digest
        || input.policy_digest != input.request_policy_digest
        || input.policy_digest != input.preflight_policy_digest
        || input.policy_digest != input.commit_policy_digest
        || input.set_digest != input.request_set_digest
        || input.set_digest != input.preflight_set_digest
        || input.set_digest != input.commit_set_digest {
        return Decision::Denied;
    }
    if !input.explicitly_declared { return Decision::Indeterminate; }
    if input.claims.is_empty() {
        return if input.intentionally_empty && !input.required && input.allow_explicit_empty {
            Decision::NotRequired
        } else { Decision::Indeterminate };
    }

    let changed = input.preflight_snapshot != input.commit_snapshot;
    if !assurance_receipt_bound(input, changed) { return Decision::Denied; }
    let claims = if changed {
        match input.receipt.state {
            RevalidationState::Failed => return Decision::Denied,
            RevalidationState::Indeterminate => return Decision::Indeterminate,
            RevalidationState::NotAttempted => return Decision::Denied,
            RevalidationState::Passed => {
                if !exact_fresh_claims(input.claims, &input.receipt.validated_claims) {
                    return Decision::Denied;
                }
                input.receipt.validated_claims.as_slice()
            }
        }
    } else { input.claims };
    evaluate_claims(claims, input.require_complete_dependencies)
}

fn authority_receipt_bound(input: &AuthorityInput<'_>, changed: bool) -> bool {
    let r = input.receipt;
    if !changed {
        return r.state == RevalidationState::NotAttempted
            && r.result_ref.is_none() && r.result_digest.is_none()
            && r.request_digest.is_none() && r.grant_ref.is_none()
            && r.snapshot_digest.is_none() && r.authority_state.is_none();
    }
    r.state != RevalidationState::NotAttempted
        && present(&r.result_ref) && present(&r.result_digest)
        && r.request_digest.as_deref() == Some(input.request_digest)
        && r.grant_ref.as_deref() == Some(input.grant_ref)
        && r.snapshot_digest.as_deref() == Some(input.commit_snapshot)
        && r.authority_state == Some(input.state_at_commit)
}

pub fn evaluate_authority(input: &AuthorityInput<'_>) -> Decision {
    if input.request_digest != input.preflight_request_digest
        || input.request_digest != input.commit_request_digest
        || input.grant_ref != input.preflight_grant_ref
        || input.grant_ref != input.commit_grant_ref {
        return Decision::Denied;
    }
    let changed = input.preflight_snapshot != input.commit_snapshot;
    if !authority_receipt_bound(input, changed) { return Decision::Denied; }
    if changed {
        match input.receipt.state {
            RevalidationState::Failed => return Decision::Denied,
            RevalidationState::Indeterminate => return Decision::Indeterminate,
            RevalidationState::NotAttempted => return Decision::Denied,
            RevalidationState::Passed => {}
        }
    }
    match input.state_at_commit {
        AuthorityState::Current => Decision::Eligible,
        AuthorityState::Revoked | AuthorityState::Expired | AuthorityState::Denied => Decision::Denied,
        AuthorityState::Unknown => Decision::Indeterminate,
    }
}

pub fn evaluate_admission(authority: Decision, assurance: Decision) -> Decision {
    if authority == Decision::Denied || assurance == Decision::Denied {
        Decision::Denied
    } else if authority == Decision::Indeterminate || assurance == Decision::Indeterminate {
        Decision::Indeterminate
    } else {
        Decision::Eligible
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claim() -> Claim {
        Claim {
            id: "claim:baseline".into(), property: "delegation_containment".into(),
            subject_digest: "sha256:subject-v1".into(),
            qualification_ref: "qualification:v1".into(),
            qualification_digest: "sha256:qualification-v1".into(),
            predecessor_ref: None, predecessor_digest: None,
            lifecycle: Lifecycle::Current, applicability: Applicability::ExactSubject,
            acceptable: vec![Applicability::ExactSubject, Applicability::ControlledEquivalent,
                Applicability::ConditionallyTransferable],
            bridge_ref: None, subject_matches: true,
            dependency_coverage: DependencyCoverage::Complete, unresolved_dependencies: 0,
            required_currentness: Currentness::CurrentThroughVerifiedCheckpoint,
            actual_currentness: Currentness::CurrentThroughVerifiedCheckpoint,
        }
    }
    fn no_assurance_receipt() -> AssuranceReceipt {
        AssuranceReceipt { state: RevalidationState::NotAttempted, result_ref: None,
            result_digest: None, request_digest: None, prerequisite_set_digest: None,
            snapshot_digest: None, validated_claims: vec![] }
    }
    fn passed_receipt(claims: Vec<Claim>) -> AssuranceReceipt {
        AssuranceReceipt { state: RevalidationState::Passed,
            result_ref: Some("assurance-revalidation:v2".into()),
            result_digest: Some("sha256:receipt-v2".into()),
            request_digest: Some("sha256:req".into()), prerequisite_set_digest: Some("sha256:set".into()),
            snapshot_digest: Some("sha256:snapshot-v2".into()), validated_claims: claims }
    }
    fn assurance<'a>(claims: &'a [Claim], receipt: &'a AssuranceReceipt) -> AssuranceInput<'a> {
        AssuranceInput { request_digest: "sha256:req", preflight_request_digest: "sha256:req",
            commit_request_digest: "sha256:req", policy_digest: "sha256:policy",
            request_policy_digest: "sha256:policy", preflight_policy_digest: "sha256:policy",
            commit_policy_digest: "sha256:policy", set_digest: "sha256:set",
            request_set_digest: "sha256:set", preflight_set_digest: "sha256:set",
            commit_set_digest: "sha256:set", explicitly_declared: true, required: true,
            intentionally_empty: false, allow_explicit_empty: false,
            require_complete_dependencies: true, preflight_snapshot: "sha256:snapshot-v1",
            commit_snapshot: "sha256:snapshot-v1", claims, receipt }
    }
    fn changed<'a>(claims: &'a [Claim], receipt: &'a AssuranceReceipt) -> AssuranceInput<'a> {
        let mut input = assurance(claims, receipt);
        input.commit_snapshot = "sha256:snapshot-v2";
        input
    }

    #[test] fn unchanged_snapshot_accepts_current_claim() {
        let claims = vec![claim()]; let r = no_assurance_receipt();
        assert_eq!(evaluate_assurance(&assurance(&claims, &r)), Decision::Eligible);
    }
    #[test] fn changed_snapshot_requires_exact_fresh_claim_receipt() {
        let claims = vec![claim()]; let r = passed_receipt(claims.clone());
        assert_eq!(evaluate_assurance(&changed(&claims, &r)), Decision::Eligible);
    }
    #[test] fn indeterminate_revalidation_holds_without_falling_back_to_preflight() {
        let claims = vec![claim()];
        let receipt = AssuranceReceipt {
            state: RevalidationState::Indeterminate,
            result_ref: Some("assurance-revalidation:uncertain".into()),
            result_digest: Some("sha256:uncertain".into()),
            request_digest: Some("sha256:req".into()),
            prerequisite_set_digest: Some("sha256:set".into()),
            snapshot_digest: Some("sha256:snapshot-v2".into()),
            validated_claims: vec![],
        };
        assert_eq!(
            evaluate_assurance(&changed(&claims, &receipt)),
            Decision::Indeterminate
        );
    }

    #[test] fn misbound_receipt_denies() {
        let claims = vec![claim()]; let mut r = passed_receipt(claims.clone());
        r.request_digest = Some("sha256:other".into());
        assert_eq!(evaluate_assurance(&changed(&claims, &r)), Decision::Denied);
        r = passed_receipt(claims.clone()); r.prerequisite_set_digest = Some("sha256:other".into());
        assert_eq!(evaluate_assurance(&changed(&claims, &r)), Decision::Denied);
        r = passed_receipt(claims.clone()); r.snapshot_digest = Some("sha256:other".into());
        assert_eq!(evaluate_assurance(&changed(&claims, &r)), Decision::Denied);
    }
    #[test] fn omitted_duplicate_and_scope_shifted_claims_deny() {
        let claims = vec![claim()];
        assert_eq!(evaluate_assurance(&changed(&claims, &passed_receipt(vec![]))), Decision::Denied);
        let mut duplicate = claims.clone(); duplicate.push(claim());
        assert_eq!(evaluate_assurance(&changed(&claims, &passed_receipt(duplicate))), Decision::Denied);
        let mut shifted = claim(); shifted.subject_digest = "sha256:other-subject".into();
        assert_eq!(evaluate_assurance(&changed(&claims, &passed_receipt(vec![shifted]))), Decision::Denied);
    }
    #[test] fn successor_result_must_name_exact_predecessor() {
        let claims = vec![claim()]; let mut next = claim();
        next.qualification_ref = "qualification:v2".into();
        next.qualification_digest = "sha256:qualification-v2".into();
        assert_eq!(evaluate_assurance(&changed(&claims, &passed_receipt(vec![next.clone()]))), Decision::Denied);
        next.predecessor_ref = Some("qualification:v1".into());
        next.predecessor_digest = Some("sha256:qualification-v1".into());
        assert_eq!(evaluate_assurance(&changed(&claims, &passed_receipt(vec![next]))), Decision::Eligible);
    }
    #[test] fn successful_receipt_cannot_override_invalidated_claim() {
        let claims = vec![claim()]; let mut fresh = claim(); fresh.lifecycle = Lifecycle::Invalidated;
        assert_eq!(evaluate_assurance(&changed(&claims, &passed_receipt(vec![fresh]))), Decision::Denied);
    }
    #[test] fn partial_dependencies_and_unknown_applicability_hold() {
        let mut c = claim(); c.dependency_coverage = DependencyCoverage::Partial;
        let claims = vec![c]; let r = no_assurance_receipt();
        assert_eq!(evaluate_assurance(&assurance(&claims, &r)), Decision::Indeterminate);
        let mut c = claim(); c.applicability = Applicability::Unknown;
        assert_eq!(evaluate_assurance(&assurance(&[c], &r)), Decision::Indeterminate);
    }
    #[test] fn explicit_empty_set_is_not_implicit_allow() {
        let claims: Vec<Claim> = vec![]; let r = no_assurance_receipt();
        let mut i = assurance(&claims, &r);
        i.required = false; i.intentionally_empty = true; i.allow_explicit_empty = true;
        assert_eq!(evaluate_assurance(&i), Decision::NotRequired);
        i.explicitly_declared = false;
        assert_eq!(evaluate_assurance(&i), Decision::Indeterminate);
    }
    fn no_authority_receipt() -> AuthorityReceipt {
        AuthorityReceipt { state: RevalidationState::NotAttempted, result_ref: None,
            result_digest: None, request_digest: None, grant_ref: None,
            snapshot_digest: None, authority_state: None }
    }
    fn authority<'a>(r: &'a AuthorityReceipt) -> AuthorityInput<'a> {
        AuthorityInput { request_digest: "sha256:req", preflight_request_digest: "sha256:req",
            commit_request_digest: "sha256:req", grant_ref: "grant:v1",
            preflight_grant_ref: "grant:v1", commit_grant_ref: "grant:v1",
            preflight_snapshot: "sha256:authority-v1", commit_snapshot: "sha256:authority-v1",
            state_at_commit: AuthorityState::Current, receipt: r }
    }
    #[test] fn revoked_authority_denies_and_unknown_holds() {
        let r = no_authority_receipt(); let mut i = authority(&r);
        i.state_at_commit = AuthorityState::Revoked;
        assert_eq!(evaluate_authority(&i), Decision::Denied);
        i.state_at_commit = AuthorityState::Unknown;
        assert_eq!(evaluate_authority(&i), Decision::Indeterminate);
    }
    #[test] fn changed_authority_snapshot_requires_exact_grant_receipt() {
        let r = AuthorityReceipt { state: RevalidationState::Passed,
            result_ref: Some("authority-revalidation:v2".into()),
            result_digest: Some("sha256:authority-receipt".into()),
            request_digest: Some("sha256:req".into()), grant_ref: Some("grant:v1".into()),
            snapshot_digest: Some("sha256:authority-v2".into()),
            authority_state: Some(AuthorityState::Current) };
        let mut i = authority(&r); i.commit_snapshot = "sha256:authority-v2";
        assert_eq!(evaluate_authority(&i), Decision::Eligible);
        let mut bad = r.clone(); bad.grant_ref = Some("grant:other".into());
        let mut i = authority(&bad); i.commit_snapshot = "sha256:authority-v2";
        assert_eq!(evaluate_authority(&i), Decision::Denied);
    }
    #[test] fn admission_keeps_unknown_distinct_from_allow() {
        assert_eq!(evaluate_admission(Decision::Eligible, Decision::Indeterminate), Decision::Indeterminate);
        assert_eq!(evaluate_admission(Decision::Denied, Decision::Eligible), Decision::Denied);
        assert_eq!(evaluate_admission(Decision::NotRequired, Decision::Eligible), Decision::Eligible);
    }
}
