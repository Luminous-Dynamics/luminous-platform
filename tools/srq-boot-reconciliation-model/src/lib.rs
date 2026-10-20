//! Cross-layer decision-to-boot reconciliation contract model.
//!
//! This crate asks a different question from an updater or provenance verifier:
//! does an independently observed booted generation match the exact promotion
//! decision that was durably committed?
//!
//! The caller must authenticate and freshness-check inputs before relying on this
//! result operationally. A digest-shaped field is not a signature; a model input
//! is not a trusted measurement; and Consistent is not release qualification.
//! The model verifies deterministic binding and invariants only.

use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromotionIntent {
    pub decision_id: String,
    pub target_id: String,
    pub promotion_challenge_id: String,
    pub boot_observation_challenge_id: String,
    pub previous_generation: u64,
    pub next_generation: u64,
    pub approved_policy_digest: String,
    pub effective_policy_digest: String,
    pub artifact_digest: String,
    pub corpus_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommittedPromotion {
    pub decision_id: String,
    pub target_id: String,
    pub consumed_challenge_id: String,
    pub generation: u64,
    pub approved_policy_digest: String,
    pub effective_policy_digest: String,
    pub artifact_digest: String,
    pub corpus_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BootObservation {
    pub target_id: String,
    pub decision_id: String,
    pub observation_challenge_id: String,
    pub boot_instance_id: String,
    pub generation: u64,
    pub effective_policy_digest: String,
    pub artifact_digest: String,
    pub measurement_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReconciliationFault {
    InvalidIdentifier(&'static str),
    InvalidDigest(&'static str),
    ChallengeRolesNotDistinct,
    GenerationTransitionIsNotExactlyOne,
    DurableDecisionIdMismatch,
    DurableTargetMismatch,
    DurableChallengeMismatch,
    DurableGenerationMismatch,
    DurableApprovedPolicyMismatch,
    DurableEffectivePolicyMismatch,
    DurableArtifactMismatch,
    DurableCorpusMismatch,
    BootTargetMismatch,
    BootDecisionMismatch,
    BootChallengeMismatch,
    BootGenerationMismatch,
    BootPolicyMismatch,
    BootArtifactMismatch,
    MissingDurablePromotion,
}

impl fmt::Display for ReconciliationFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reconciliation {
    /// The input records are internally bound and mutually consistent.
    /// This does not authenticate the records or qualify a release.
    Consistent,
    /// A durable promotion is consistent, but no post-boot observation exists.
    /// This is never a release pass.
    PendingBootObservation,
    /// At least one required binding or invariant is absent or contradictory.
    Blocked(Vec<ReconciliationFault>),
}

impl Reconciliation {
    pub fn is_consistent(&self) -> bool {
        matches!(self, Self::Consistent)
    }
}

pub fn reconcile(
    intent: &PromotionIntent,
    durable: Option<&CommittedPromotion>,
    boot: Option<&BootObservation>,
) -> Reconciliation {
    let mut faults = validate_intent(intent);
    let Some(durable) = durable else {
        faults.push(ReconciliationFault::MissingDurablePromotion);
        return Reconciliation::Blocked(faults);
    };

    validate_durable(intent, durable, &mut faults);

    match boot {
        None if faults.is_empty() => Reconciliation::PendingBootObservation,
        None => Reconciliation::Blocked(faults),
        Some(observation) => {
            validate_boot(intent, observation, &mut faults);
            if faults.is_empty() {
                Reconciliation::Consistent
            } else {
                Reconciliation::Blocked(faults)
            }
        }
    }
}

fn validate_intent(intent: &PromotionIntent) -> Vec<ReconciliationFault> {
    let mut faults = Vec::new();
    for (value, field) in [
        (intent.decision_id.as_str(), "intent.decision_id"),
        (intent.target_id.as_str(), "intent.target_id"),
        (intent.promotion_challenge_id.as_str(), "intent.promotion_challenge_id"),
        (
            intent.boot_observation_challenge_id.as_str(),
            "intent.boot_observation_challenge_id",
        ),
    ] {
        if !valid_identifier(value) {
            faults.push(ReconciliationFault::InvalidIdentifier(field));
        }
    }
    for (value, field) in [
        (intent.approved_policy_digest.as_str(), "intent.approved_policy_digest"),
        (intent.effective_policy_digest.as_str(), "intent.effective_policy_digest"),
        (intent.artifact_digest.as_str(), "intent.artifact_digest"),
        (intent.corpus_digest.as_str(), "intent.corpus_digest"),
    ] {
        if !valid_digest(value) {
            faults.push(ReconciliationFault::InvalidDigest(field));
        }
    }
    if intent.promotion_challenge_id == intent.boot_observation_challenge_id {
        faults.push(ReconciliationFault::ChallengeRolesNotDistinct);
    }
    if intent.previous_generation.checked_add(1) != Some(intent.next_generation) {
        faults.push(ReconciliationFault::GenerationTransitionIsNotExactlyOne);
    }
    faults
}

fn validate_durable(
    intent: &PromotionIntent,
    durable: &CommittedPromotion,
    faults: &mut Vec<ReconciliationFault>,
) {
    for (value, field) in [
        (durable.decision_id.as_str(), "durable.decision_id"),
        (durable.target_id.as_str(), "durable.target_id"),
        (durable.consumed_challenge_id.as_str(), "durable.consumed_challenge_id"),
    ] {
        if !valid_identifier(value) {
            faults.push(ReconciliationFault::InvalidIdentifier(field));
        }
    }
    for (value, field) in [
        (durable.approved_policy_digest.as_str(), "durable.approved_policy_digest"),
        (durable.effective_policy_digest.as_str(), "durable.effective_policy_digest"),
        (durable.artifact_digest.as_str(), "durable.artifact_digest"),
        (durable.corpus_digest.as_str(), "durable.corpus_digest"),
    ] {
        if !valid_digest(value) {
            faults.push(ReconciliationFault::InvalidDigest(field));
        }
    }
    if durable.decision_id != intent.decision_id {
        faults.push(ReconciliationFault::DurableDecisionIdMismatch);
    }
    if durable.target_id != intent.target_id {
        faults.push(ReconciliationFault::DurableTargetMismatch);
    }
    if durable.consumed_challenge_id != intent.promotion_challenge_id {
        faults.push(ReconciliationFault::DurableChallengeMismatch);
    }
    if durable.generation != intent.next_generation {
        faults.push(ReconciliationFault::DurableGenerationMismatch);
    }
    if durable.approved_policy_digest != intent.approved_policy_digest {
        faults.push(ReconciliationFault::DurableApprovedPolicyMismatch);
    }
    if durable.effective_policy_digest != intent.effective_policy_digest {
        faults.push(ReconciliationFault::DurableEffectivePolicyMismatch);
    }
    if durable.artifact_digest != intent.artifact_digest {
        faults.push(ReconciliationFault::DurableArtifactMismatch);
    }
    if durable.corpus_digest != intent.corpus_digest {
        faults.push(ReconciliationFault::DurableCorpusMismatch);
    }
}

fn validate_boot(
    intent: &PromotionIntent,
    boot: &BootObservation,
    faults: &mut Vec<ReconciliationFault>,
) {
    for (value, field) in [
        (boot.target_id.as_str(), "boot.target_id"),
        (boot.decision_id.as_str(), "boot.decision_id"),
        (boot.observation_challenge_id.as_str(), "boot.observation_challenge_id"),
    ] {
        if !valid_identifier(value) {
            faults.push(ReconciliationFault::InvalidIdentifier(field));
        }
    }
    if !valid_identifier(&boot.boot_instance_id) {
        faults.push(ReconciliationFault::InvalidIdentifier("boot.boot_instance_id"));
    }
    for (value, field) in [
        (boot.effective_policy_digest.as_str(), "boot.effective_policy_digest"),
        (boot.artifact_digest.as_str(), "boot.artifact_digest"),
        (boot.measurement_digest.as_str(), "boot.measurement_digest"),
    ] {
        if !valid_digest(value) {
            faults.push(ReconciliationFault::InvalidDigest(field));
        }
    }
    if boot.target_id != intent.target_id {
        faults.push(ReconciliationFault::BootTargetMismatch);
    }
    if boot.decision_id != intent.decision_id {
        faults.push(ReconciliationFault::BootDecisionMismatch);
    }
    if boot.observation_challenge_id != intent.boot_observation_challenge_id {
        faults.push(ReconciliationFault::BootChallengeMismatch);
    }
    if boot.generation != intent.next_generation {
        faults.push(ReconciliationFault::BootGenerationMismatch);
    }
    if boot.effective_policy_digest != intent.effective_policy_digest {
        faults.push(ReconciliationFault::BootPolicyMismatch);
    }
    if boot.artifact_digest != intent.artifact_digest {
        faults.push(ReconciliationFault::BootArtifactMismatch);
    }
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const D: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";

    fn base() -> (PromotionIntent, CommittedPromotion, BootObservation) {
        let intent = PromotionIntent {
            decision_id: "decision-0001".into(),
            target_id: "target-jo-01".into(),
            promotion_challenge_id: "challenge-promote-01".into(),
            boot_observation_challenge_id: "challenge-boot-01".into(),
            previous_generation: 41,
            next_generation: 42,
            approved_policy_digest: A.into(),
            effective_policy_digest: B.into(),
            artifact_digest: C.into(),
            corpus_digest: D.into(),
        };
        let durable = CommittedPromotion {
            decision_id: "decision-0001".into(),
            target_id: "target-jo-01".into(),
            consumed_challenge_id: "challenge-promote-01".into(),
            generation: 42,
            approved_policy_digest: A.into(),
            effective_policy_digest: B.into(),
            artifact_digest: C.into(),
            corpus_digest: D.into(),
        };
        let boot = BootObservation {
            target_id: "target-jo-01".into(),
            decision_id: "decision-0001".into(),
            observation_challenge_id: "challenge-boot-01".into(),
            boot_instance_id: "boot-epoch-0042".into(),
            generation: 42,
            effective_policy_digest: B.into(),
            artifact_digest: C.into(),
            measurement_digest: A.into(),
        };
        (intent, durable, boot)
    }

    #[test]
    fn exact_authorization_durable_decision_and_observed_boot_reconcile() {
        let (intent, durable, boot) = base();
        assert_eq!(reconcile(&intent, Some(&durable), Some(&boot)), Reconciliation::Consistent);
    }

    #[test]
    fn missing_post_boot_observation_is_pending_not_success() {
        let (intent, durable, _) = base();
        assert_eq!(
            reconcile(&intent, Some(&durable), None),
            Reconciliation::PendingBootObservation
        );
        assert!(!reconcile(&intent, Some(&durable), None).is_consistent());
    }

    #[test]
    fn missing_durable_decision_fails_closed() {
        let (intent, _, boot) = base();
        assert!(matches!(
            reconcile(&intent, None, Some(&boot)),
            Reconciliation::Blocked(faults) if faults.contains(&ReconciliationFault::MissingDurablePromotion)
        ));
    }

    #[test]
    fn prior_generation_boot_is_never_mistaken_for_promoted_generation() {
        let (intent, durable, mut boot) = base();
        boot.generation = intent.previous_generation;
        assert!(matches!(
            reconcile(&intent, Some(&durable), Some(&boot)),
            Reconciliation::Blocked(faults) if faults.contains(&ReconciliationFault::BootGenerationMismatch)
        ));
    }

    #[test]
    fn wrong_target_and_wrong_decision_are_rejected_at_runtime_observation() {
        let (intent, durable, mut boot) = base();
        boot.target_id = "target-other".into();
        boot.decision_id = "decision-other".into();
        assert!(matches!(
            reconcile(&intent, Some(&durable), Some(&boot)),
            Reconciliation::Blocked(faults)
                if faults.contains(&ReconciliationFault::BootTargetMismatch)
                    && faults.contains(&ReconciliationFault::BootDecisionMismatch)
        ));
    }

    #[test]
    fn durable_challenge_must_be_the_challenge_that_authorized_promotion() {
        let (intent, mut durable, boot) = base();
        durable.consumed_challenge_id = "challenge-other".into();
        assert!(matches!(
            reconcile(&intent, Some(&durable), Some(&boot)),
            Reconciliation::Blocked(faults) if faults.contains(&ReconciliationFault::DurableChallengeMismatch)
        ));
    }

    #[test]
    fn policy_artifact_and_corpus_must_match_across_intent_and_durable_state() {
        let (intent, mut durable, boot) = base();
        durable.effective_policy_digest = A.into();
        durable.artifact_digest = D.into();
        durable.corpus_digest = C.into();
        assert!(matches!(
            reconcile(&intent, Some(&durable), Some(&boot)),
            Reconciliation::Blocked(faults)
                if faults.contains(&ReconciliationFault::DurableEffectivePolicyMismatch)
                    && faults.contains(&ReconciliationFault::DurableArtifactMismatch)
                    && faults.contains(&ReconciliationFault::DurableCorpusMismatch)
        ));
    }

    #[test]
    fn policy_artifact_and_fresh_observation_challenge_must_match_the_boot() {
        let (intent, durable, mut boot) = base();
        boot.effective_policy_digest = A.into();
        boot.artifact_digest = D.into();
        boot.observation_challenge_id = intent.promotion_challenge_id.clone();
        assert!(matches!(
            reconcile(&intent, Some(&durable), Some(&boot)),
            Reconciliation::Blocked(faults)
                if faults.contains(&ReconciliationFault::BootPolicyMismatch)
                    && faults.contains(&ReconciliationFault::BootArtifactMismatch)
                    && faults.contains(&ReconciliationFault::BootChallengeMismatch)
        ));
    }

    #[test]
    fn skipped_generation_and_reused_challenge_roles_are_rejected() {
        let (mut intent, durable, boot) = base();
        intent.next_generation += 1;
        assert!(matches!(
            reconcile(&intent, Some(&durable), Some(&boot)),
            Reconciliation::Blocked(faults)
                if faults.contains(&ReconciliationFault::GenerationTransitionIsNotExactlyOne)
                    && faults.contains(&ReconciliationFault::DurableGenerationMismatch)
                    && faults.contains(&ReconciliationFault::BootGenerationMismatch)
        ));
        let (mut intent, durable, boot) = base();
        intent.boot_observation_challenge_id = intent.promotion_challenge_id.clone();
        assert!(matches!(
            reconcile(&intent, Some(&durable), Some(&boot)),
            Reconciliation::Blocked(faults) if faults.contains(&ReconciliationFault::ChallengeRolesNotDistinct)
        ));
    }

    #[test]
    fn malformed_identifiers_and_digests_never_reconcile() {
        let (mut intent, durable, boot) = base();
        intent.target_id = "../foreign-target".into();
        intent.artifact_digest = "not-a-digest".into();
        assert!(matches!(
            reconcile(&intent, Some(&durable), Some(&boot)),
            Reconciliation::Blocked(faults)
                if faults.contains(&ReconciliationFault::InvalidIdentifier("intent.target_id"))
                    && faults.contains(&ReconciliationFault::InvalidDigest("intent.artifact_digest"))
        ));
    }

    #[test]
    fn ten_thousand_deterministic_single_field_mutations_never_reconcile() {
        let (intent, durable, boot) = base();
        for i in 0..10_000_u64 {
            let mut changed = boot.clone();
            match i % 7 {
                0 => changed.target_id = format!("target-{i}"),
                1 => changed.decision_id = format!("decision-{i}"),
                2 => changed.observation_challenge_id = format!("fresh-{i}"),
                3 => changed.generation = i,
                4 => changed.effective_policy_digest = format!("{i:064x}"),
                5 => changed.artifact_digest = format!("{:064x}", i ^ 0xfeed),
                _ => changed.boot_instance_id = "../foreign-boot".into(),
            }
            assert!(
                !reconcile(&intent, Some(&durable), Some(&changed)).is_consistent(),
                "mutation {i} unexpectedly reconciled"
            );
        }
    }

    #[test]
    fn digest_shape_is_not_treated_as_authenticity() {
        // This model checks the measurement digest's shape only. A separate
        // trusted evidence verifier must authenticate the measurement before
        // any release or device-trust decision relies on it.
        let (intent, durable, mut boot) = base();
        boot.measurement_digest = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee".into();
        assert_eq!(reconcile(&intent, Some(&durable), Some(&boot)), Reconciliation::Consistent);
    }
}
