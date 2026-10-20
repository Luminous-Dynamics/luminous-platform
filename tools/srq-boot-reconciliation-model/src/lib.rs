//! Cross-layer decision-to-boot reconciliation contract model.
//!
//! The central invariant is stronger than "the update was downloaded" or
//! "provenance verified": an independently observed booted generation must match
//! the exact policy decision durably committed for the target.
//!
//! This model does not authenticate any input. Callers must verify signatures,
//! trust anchors, target identity and challenge freshness before relying on this
//! reconciliation. A digest-shaped string is not a signature, and Consistent
//! is not release qualification.

use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromotionIntent {
    pub decision_id: String,
    pub target_id: String,
    pub promotion_challenge_id: String,
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

/// A post-commit challenge issued by the relying verifier. Its authenticated
/// issuance and freshness are caller preconditions; the model validates binding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BootObservationChallenge {
    pub challenge_id: String,
    pub target_id: String,
    pub decision_id: String,
    pub expected_generation: u64,
    pub expected_effective_policy_digest: String,
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
    PromotionAndBootChallengesNotDistinct,
    GenerationTransitionIsNotExactlyOne,
    MissingDurablePromotion,
    MissingBootObservationChallenge,
    DurableDecisionIdMismatch,
    DurableTargetMismatch,
    DurableChallengeMismatch,
    DurableGenerationMismatch,
    DurableApprovedPolicyMismatch,
    DurableEffectivePolicyMismatch,
    DurableArtifactMismatch,
    DurableCorpusMismatch,
    BootChallengeTargetMismatch,
    BootChallengeDecisionMismatch,
    BootChallengeGenerationMismatch,
    BootChallengePolicyMismatch,
    BootTargetMismatch,
    BootDecisionMismatch,
    BootChallengeMismatch,
    BootGenerationMismatch,
    BootPolicyMismatch,
    BootArtifactMismatch,
}

impl fmt::Display for ReconciliationFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reconciliation {
    /// All supplied records are internally bound and mutually consistent.
    /// This does not authenticate the records or qualify a release.
    Consistent,
    /// Durable commit is present, but the independent post-commit challenge
    /// has not yet been supplied. This is never a release pass.
    PendingBootChallenge,
    /// A post-commit challenge exists, but no matching boot observation exists.
    /// This is never a release pass.
    PendingBootObservation,
    /// A binding is absent or contradictory; callers must fail closed.
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
    challenge: Option<&BootObservationChallenge>,
    boot: Option<&BootObservation>,
) -> Reconciliation {
    let mut faults = validate_intent(intent);
    let Some(durable) = durable else {
        faults.push(ReconciliationFault::MissingDurablePromotion);
        return Reconciliation::Blocked(faults);
    };
    validate_durable(intent, durable, &mut faults);

    let Some(challenge) = challenge else {
        if boot.is_some() {
            faults.push(ReconciliationFault::MissingBootObservationChallenge);
            return Reconciliation::Blocked(faults);
        }
        return if faults.is_empty() {
            Reconciliation::PendingBootChallenge
        } else {
            Reconciliation::Blocked(faults)
        };
    };
    validate_boot_challenge(intent, challenge, &mut faults);

    let Some(boot) = boot else {
        return if faults.is_empty() {
            Reconciliation::PendingBootObservation
        } else {
            Reconciliation::Blocked(faults)
        };
    };
    validate_boot(intent, challenge, boot, &mut faults);

    if faults.is_empty() {
        Reconciliation::Consistent
    } else {
        Reconciliation::Blocked(faults)
    }
}

fn validate_intent(intent: &PromotionIntent) -> Vec<ReconciliationFault> {
    let mut faults = Vec::new();
    for (value, field) in [
        (intent.decision_id.as_str(), "intent.decision_id"),
        (intent.target_id.as_str(), "intent.target_id"),
        (intent.promotion_challenge_id.as_str(), "intent.promotion_challenge_id"),
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

fn validate_boot_challenge(
    intent: &PromotionIntent,
    challenge: &BootObservationChallenge,
    faults: &mut Vec<ReconciliationFault>,
) {
    for (value, field) in [
        (challenge.challenge_id.as_str(), "challenge.challenge_id"),
        (challenge.target_id.as_str(), "challenge.target_id"),
        (challenge.decision_id.as_str(), "challenge.decision_id"),
    ] {
        if !valid_identifier(value) {
            faults.push(ReconciliationFault::InvalidIdentifier(field));
        }
    }
    if !valid_digest(&challenge.expected_effective_policy_digest) {
        faults.push(ReconciliationFault::InvalidDigest("challenge.expected_effective_policy_digest"));
    }
    if challenge.challenge_id == intent.promotion_challenge_id {
        faults.push(ReconciliationFault::PromotionAndBootChallengesNotDistinct);
    }
    if challenge.target_id != intent.target_id {
        faults.push(ReconciliationFault::BootChallengeTargetMismatch);
    }
    if challenge.decision_id != intent.decision_id {
        faults.push(ReconciliationFault::BootChallengeDecisionMismatch);
    }
    if challenge.expected_generation != intent.next_generation {
        faults.push(ReconciliationFault::BootChallengeGenerationMismatch);
    }
    if challenge.expected_effective_policy_digest != intent.effective_policy_digest {
        faults.push(ReconciliationFault::BootChallengePolicyMismatch);
    }
}

fn validate_boot(
    intent: &PromotionIntent,
    challenge: &BootObservationChallenge,
    boot: &BootObservation,
    faults: &mut Vec<ReconciliationFault>,
) {
    for (value, field) in [
        (boot.target_id.as_str(), "boot.target_id"),
        (boot.decision_id.as_str(), "boot.decision_id"),
        (boot.observation_challenge_id.as_str(), "boot.observation_challenge_id"),
        (boot.boot_instance_id.as_str(), "boot.boot_instance_id"),
    ] {
        if !valid_identifier(value) {
            faults.push(ReconciliationFault::InvalidIdentifier(field));
        }
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
    if boot.target_id != intent.target_id || boot.target_id != challenge.target_id {
        faults.push(ReconciliationFault::BootTargetMismatch);
    }
    if boot.decision_id != intent.decision_id || boot.decision_id != challenge.decision_id {
        faults.push(ReconciliationFault::BootDecisionMismatch);
    }
    if boot.observation_challenge_id != challenge.challenge_id {
        faults.push(ReconciliationFault::BootChallengeMismatch);
    }
    if boot.generation != intent.next_generation || boot.generation != challenge.expected_generation {
        faults.push(ReconciliationFault::BootGenerationMismatch);
    }
    if boot.effective_policy_digest != intent.effective_policy_digest
        || boot.effective_policy_digest != challenge.expected_effective_policy_digest
    {
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
    const NIX_DEPLOYMENT: &str =
        "/nix/store/0123456789abcdfghijklmnpqrsvwxyz-nixos-system-demo";
    const PRIOR_NIX_DEPLOYMENT: &str =
        "/nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-nixos-system-previous";

    fn base() -> (PromotionIntent, CommittedPromotion, BootObservationChallenge, BootObservation) {
        let intent = PromotionIntent {
            decision_id: "decision-0001".into(),
            target_id: "target-jo-01".into(),
            promotion_challenge_id: "challenge-promote-01".into(),
            deployment_id: NIX_DEPLOYMENT.into(),
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
            deployment_id: NIX_DEPLOYMENT.into(),
            generation: 42,
            approved_policy_digest: A.into(),
            effective_policy_digest: B.into(),
            artifact_digest: C.into(),
            corpus_digest: D.into(),
        };
        let challenge = BootObservationChallenge {
            challenge_id: "challenge-boot-01".into(),
            target_id: "target-jo-01".into(),
            decision_id: "decision-0001".into(),
            expected_generation: 42,
            expected_deployment_id: NIX_DEPLOYMENT.into(),
            expected_effective_policy_digest: B.into(),
        };
        let boot = BootObservation {
            target_id: "target-jo-01".into(),
            decision_id: "decision-0001".into(),
            observation_challenge_id: "challenge-boot-01".into(),
            boot_instance_id: "boot-epoch-0042".into(),
            booted_deployment_id: NIX_DEPLOYMENT.into(),
            current_userspace_deployment_id: NIX_DEPLOYMENT.into(),
            generation: 42,
            effective_policy_digest: B.into(),
            artifact_digest: C.into(),
            measurement_digest: A.into(),
        };
        (intent, durable, challenge, boot)
    }

    #[test]
    fn exact_authorization_durable_decision_fresh_challenge_and_boot_reconcile() {
        let (intent, durable, challenge, boot) = base();
        assert_eq!(reconcile(&intent, Some(&durable), Some(&challenge), Some(&boot)), Reconciliation::Consistent);
    }

    #[test]
    fn missing_post_commit_challenge_is_pending_not_success() {
        let (intent, durable, _, _) = base();
        assert_eq!(
            reconcile(&intent, Some(&durable), None, None),
            Reconciliation::PendingBootChallenge
        );
        assert!(!reconcile(&intent, Some(&durable), None, None).is_consistent());
    }

    #[test]
    fn missing_boot_observation_is_pending_not_success() {
        let (intent, durable, challenge, _) = base();
        assert_eq!(
            reconcile(&intent, Some(&durable), Some(&challenge), None),
            Reconciliation::PendingBootObservation
        );
        assert!(!reconcile(&intent, Some(&durable), Some(&challenge), None).is_consistent());
    }

    #[test]
    fn missing_durable_decision_fails_closed() {
        let (intent, _, challenge, boot) = base();
        assert!(matches!(
            reconcile(&intent, None, Some(&challenge), Some(&boot)),
            Reconciliation::Blocked(faults) if faults.contains(&ReconciliationFault::MissingDurablePromotion)
        ));
    }

    #[test]
    fn prior_generation_boot_is_never_mistaken_for_promoted_generation() {
        let (intent, durable, challenge, mut boot) = base();
        boot.generation = intent.previous_generation;
        assert!(matches!(
            reconcile(&intent, Some(&durable), Some(&challenge), Some(&boot)),
            Reconciliation::Blocked(faults) if faults.contains(&ReconciliationFault::BootGenerationMismatch)
        ));
    }

    #[test]
    fn wrong_target_and_wrong_decision_are_rejected_at_runtime_observation() {
        let (intent, durable, challenge, mut boot) = base();
        boot.target_id = "target-other".into();
        boot.decision_id = "decision-other".into();
        assert!(matches!(
            reconcile(&intent, Some(&durable), Some(&challenge), Some(&boot)),
            Reconciliation::Blocked(faults)
                if faults.contains(&ReconciliationFault::BootTargetMismatch)
                    && faults.contains(&ReconciliationFault::BootDecisionMismatch)
        ));
    }

    #[test]
    fn durable_challenge_must_be_the_challenge_that_authorized_promotion() {
        let (intent, mut durable, challenge, boot) = base();
        durable.consumed_challenge_id = "challenge-other".into();
        assert!(matches!(
            reconcile(&intent, Some(&durable), Some(&challenge), Some(&boot)),
            Reconciliation::Blocked(faults) if faults.contains(&ReconciliationFault::DurableChallengeMismatch)
        ));
    }

    #[test]
    fn policy_artifact_and_corpus_must_match_across_intent_and_durable_state() {
        let (intent, mut durable, challenge, boot) = base();
        durable.effective_policy_digest = A.into();
        durable.artifact_digest = D.into();
        durable.corpus_digest = C.into();
        assert!(matches!(
            reconcile(&intent, Some(&durable), Some(&challenge), Some(&boot)),
            Reconciliation::Blocked(faults)
                if faults.contains(&ReconciliationFault::DurableEffectivePolicyMismatch)
                    && faults.contains(&ReconciliationFault::DurableArtifactMismatch)
                    && faults.contains(&ReconciliationFault::DurableCorpusMismatch)
        ));
    }

    #[test]
    fn post_commit_challenge_must_be_target_decision_generation_and_policy_bound() {
        let (intent, durable, mut challenge, boot) = base();
        challenge.target_id = "target-other".into();
        challenge.decision_id = "decision-other".into();
        challenge.expected_generation = 99;
        challenge.expected_effective_policy_digest = D.into();
        assert!(matches!(
            reconcile(&intent, Some(&durable), Some(&challenge), Some(&boot)),
            Reconciliation::Blocked(faults)
                if faults.contains(&ReconciliationFault::BootChallengeTargetMismatch)
                    && faults.contains(&ReconciliationFault::BootChallengeDecisionMismatch)
                    && faults.contains(&ReconciliationFault::BootChallengeGenerationMismatch)
                    && faults.contains(&ReconciliationFault::BootChallengePolicyMismatch)
        ));
    }

    #[test]
    fn boot_policy_artifact_and_challenge_must_match_the_observation_request() {
        let (intent, durable, mut challenge, mut boot) = base();
        challenge.challenge_id = "challenge-fresh-other".into();
        boot.effective_policy_digest = A.into();
        boot.artifact_digest = D.into();
        assert!(matches!(
            reconcile(&intent, Some(&durable), Some(&challenge), Some(&boot)),
            Reconciliation::Blocked(faults)
                if faults.contains(&ReconciliationFault::BootChallengeMismatch)
                    && faults.contains(&ReconciliationFault::BootPolicyMismatch)
                    && faults.contains(&ReconciliationFault::BootArtifactMismatch)
        ));
    }

    #[test]
    fn promotion_and_boot_challenge_roles_must_be_distinct() {
        let (intent, durable, mut challenge, boot) = base();
        challenge.challenge_id = intent.promotion_challenge_id.clone();
        assert!(matches!(
            reconcile(&intent, Some(&durable), Some(&challenge), Some(&boot)),
            Reconciliation::Blocked(faults) if faults.contains(&ReconciliationFault::PromotionAndBootChallengesNotDistinct)
        ));
    }

    #[test]
    fn skipped_generation_and_overflow_transition_are_rejected() {
        let (mut intent, durable, challenge, boot) = base();
        intent.next_generation += 1;
        assert!(matches!(
            reconcile(&intent, Some(&durable), Some(&challenge), Some(&boot)),
            Reconciliation::Blocked(faults)
                if faults.contains(&ReconciliationFault::GenerationTransitionIsNotExactlyOne)
                    && faults.contains(&ReconciliationFault::DurableGenerationMismatch)
                    && faults.contains(&ReconciliationFault::BootGenerationMismatch)
        ));
        let (mut intent, _, _, _) = base();
        intent.previous_generation = u64::MAX;
        intent.next_generation = 0;
        assert!(matches!(
            reconcile(&intent, None, None, None),
            Reconciliation::Blocked(faults) if faults.contains(&ReconciliationFault::GenerationTransitionIsNotExactlyOne)
        ));
    }

    #[test]
    fn malformed_identifiers_and_digests_never_reconcile() {
        let (mut intent, durable, challenge, boot) = base();
        intent.target_id = "../foreign-target".into();
        intent.artifact_digest = "not-a-digest".into();
        assert!(matches!(
            reconcile(&intent, Some(&durable), Some(&challenge), Some(&boot)),
            Reconciliation::Blocked(faults)
                if faults.contains(&ReconciliationFault::InvalidIdentifier("intent.target_id"))
                    && faults.contains(&ReconciliationFault::InvalidDigest("intent.artifact_digest"))
        ));
    }

    #[test]
    fn ten_thousand_deterministic_single_field_mutations_never_reconcile() {
        let (intent, durable, challenge, boot) = base();
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
                !reconcile(&intent, Some(&durable), Some(&challenge), Some(&changed)).is_consistent(),
                "mutation {i} unexpectedly reconciled"
            );
        }
    }

    #[test]
    fn one_hundred_thousand_cross_layer_mutations_never_reconcile() {
        let (intent, durable, challenge, boot) = base();
        for i in 0..100_000_u64 {
            let mut changed_intent = intent.clone();
            let mut changed_durable = durable.clone();
            let mut changed_challenge = challenge.clone();
            let mut changed_boot = boot.clone();
            let token = format!("mut-{i}");
            let digest = format!("{:064x}", i + 1);
            match i % 33 {
                0 => changed_intent.target_id = token,
                1 => changed_intent.decision_id = token,
                2 => changed_intent.promotion_challenge_id = token,
                3 => changed_intent.deployment_id = token,
                4 => changed_intent.previous_generation = u64::MAX,
                5 => changed_intent.next_generation = 43 + i,
                6 => changed_intent.approved_policy_digest = digest,
                7 => changed_intent.effective_policy_digest = digest,
                8 => changed_intent.artifact_digest = digest,
                9 => changed_intent.corpus_digest = digest,
                10 => changed_durable.decision_id = token,
                11 => changed_durable.target_id = token,
                12 => changed_durable.consumed_challenge_id = token,
                13 => changed_durable.deployment_id = token,
                14 => changed_durable.generation = 1_000 + i,
                15 => changed_durable.approved_policy_digest = digest,
                16 => changed_durable.effective_policy_digest = digest,
                17 => changed_durable.artifact_digest = digest,
                18 => changed_durable.corpus_digest = digest,
                19 => changed_challenge.challenge_id = token,
                20 => changed_challenge.target_id = token,
                21 => changed_challenge.decision_id = token,
                22 => changed_challenge.expected_generation = 1_000 + i,
                23 => changed_challenge.expected_deployment_id = token,
                24 => changed_challenge.expected_effective_policy_digest = digest,
                25 => changed_boot.target_id = token,
                26 => changed_boot.decision_id = token,
                27 => changed_boot.observation_challenge_id = token,
                28 => changed_boot.booted_deployment_id = token,
                29 => changed_boot.current_userspace_deployment_id = token,
                30 => changed_boot.generation = 1_000 + i,
                31 => changed_boot.effective_policy_digest = digest,
                _ => changed_boot.artifact_digest = digest,
            }
            assert!(
                !reconcile(
                    &changed_intent,
                    Some(&changed_durable),
                    Some(&changed_challenge),
                    Some(&changed_boot),
                )
                .is_consistent(),
                "cross-layer mutation {i} unexpectedly reconciled"
            );
        }
    }

    #[test]
    fn old_booted_system_is_rejected_even_when_abstract_generation_counter_matches() {
        let (intent, durable, challenge, mut boot) = base();
        boot.booted_deployment_id = PRIOR_NIX_DEPLOYMENT.into();
        boot.current_userspace_deployment_id = NIX_DEPLOYMENT.into();
        assert!(matches!(
            reconcile(&intent, Some(&durable), Some(&challenge), Some(&boot)),
            Reconciliation::Blocked(faults)
                if faults.contains(&ReconciliationFault::BootedDeploymentMismatch)
        ));
    }

    #[test]
    fn live_current_system_switch_does_not_substitute_for_the_booted_system() {
        let (intent, durable, challenge, mut boot) = base();
        boot.booted_deployment_id = NIX_DEPLOYMENT.into();
        boot.current_userspace_deployment_id = PRIOR_NIX_DEPLOYMENT.into();
        assert!(matches!(
            reconcile(&intent, Some(&durable), Some(&challenge), Some(&boot)),
            Reconciliation::Blocked(faults)
                if faults.contains(&ReconciliationFault::CurrentUserspaceDeploymentMismatch)
        ));
    }

    #[test]
    fn deployment_identity_syntax_is_checked_without_confusing_path_with_digest() {
        assert!(valid_deployment_id(NIX_DEPLOYMENT));
        assert!(valid_deployment_id(D)); // OSTree deployment checksum shape
        assert!(!valid_deployment_id("/nix/store/not-a-real-store-path"));
        assert!(!valid_deployment_id("../foreign-generation"));
        let (mut intent, durable, challenge, boot) = base();
        intent.deployment_id = "../foreign-generation".into();
        assert!(matches!(
            reconcile(&intent, Some(&durable), Some(&challenge), Some(&boot)),
            Reconciliation::Blocked(faults)
                if faults.contains(&ReconciliationFault::InvalidDeploymentId("intent.deployment_id"))
        ));
    }

    #[test]
    fn digest_shape_is_not_treated_as_authenticity() {
        let (intent, durable, challenge, mut boot) = base();
        boot.measurement_digest = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee".into();
        assert_eq!(
            reconcile(&intent, Some(&durable), Some(&challenge), Some(&boot)),
            Reconciliation::Consistent
        );
    }
}
