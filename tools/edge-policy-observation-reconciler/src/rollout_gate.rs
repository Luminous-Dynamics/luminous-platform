//! Fail-closed staged rollout gate.
//!
//! This module does not actuate devices. It authorizes the next rollout wave
//! only after every target in every earlier wave has fresh, target-bound
//! evidence for the exact candidate policy and its independent control path.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::adapter_capability::{
    AdapterQualificationPolicy, DeviceAdapterCapabilities, qualify_adapter,
};

/// A single target's immutable membership in a rollout plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RolloutTarget {
    pub target_token: String,
    /// Failure-domain identifier supplied by the inventory/control plane.
    pub failure_domain: String,
    /// Generation expected on this target for this rollout.
    pub policy_generation: u64,
    /// Zero-based wave index.
    pub wave_index: usize,
}

/// A deterministic, bounded-blast-radius rollout plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagedRolloutPlan {
    pub rollout_id: String,
    /// Exact candidate policy digest required on every target.
    pub candidate_policy_sha256: String,
    pub targets: Vec<RolloutTarget>,
    /// Maximum number of targets permitted in any single wave.
    pub max_targets_per_wave: usize,
    /// Maximum targets from the same failure domain in one wave.
    pub max_targets_per_failure_domain_per_wave: usize,
    /// Maximum age accepted for evidence used to authorize another wave.
    pub max_evidence_age_ms: u64,
}

/// Current independent observation for a target.
///
/// Feed one latest observation per target. Refresh a stale observation by
/// probing again and replacing it; never re-use an expired receipt to advance
/// the rollout.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TargetRolloutObservation {
    pub target_token: String,
    pub policy_generation: u64,
    pub effective_policy_sha256: String,
    pub outcome: TargetRolloutOutcome,
    /// Independent data-plane probe, not controller acknowledgement.
    pub independent_probe_passed: bool,
    /// A separate known-good control path that defeats the deny-all false pass.
    pub positive_control_passed: bool,
    pub observed_at_unix_ms: u64,
    pub expires_at_unix_ms: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TargetRolloutOutcome {
    Promoted,
    RolledBack,
    Failed,
    Unknown,
}

/// Decision produced by the pure rollout gate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RolloutGateDecision {
    /// The caller may begin this wave, subject to its device-specific apply
    /// and recovery protocol. This is not a device-actuation command.
    WaveAuthorized {
        wave_index: usize,
        targets: Vec<String>,
    },
    /// Do not expand the rollout. Resolve the listed issues and re-evaluate.
    Halted {
        wave_index: usize,
        reasons: Vec<String>,
    },
    /// All planned waves have fresh evidence and passed their gate.
    Complete,
}

/// Qualify every planned target's device adapter before authorizing any wave.
///
/// This preflight deliberately checks future-wave targets too: a rollout must
/// not begin when its later targets cannot satisfy the deployment's declared
/// transaction, rollback, readback, locking/fencing, and probe requirements.
pub fn authorize_next_wave_with_adapter_capabilities(
    plan: &StagedRolloutPlan,
    observations: &[TargetRolloutObservation],
    adapter_capabilities: &[DeviceAdapterCapabilities],
    qualification_policy: &AdapterQualificationPolicy,
    verifier_now_unix_ms: u64,
) -> Result<RolloutGateDecision, Vec<String>> {
    validate_plan(plan)?;

    let planned_targets: HashSet<&str> = plan
        .targets
        .iter()
        .map(|target| target.target_token.as_str())
        .collect();
    let mut profiles_by_target: HashMap<&str, &DeviceAdapterCapabilities> = HashMap::new();
    let mut input_errors = Vec::new();
    for profile in adapter_capabilities {
        if !planned_targets.contains(profile.target_token.as_str()) {
            input_errors.push(format!(
                "adapter profile targets {:?}, which is not in the rollout plan",
                profile.target_token
            ));
        }
        if profiles_by_target
            .insert(profile.target_token.as_str(), profile)
            .is_some()
        {
            input_errors.push(format!(
                "duplicate adapter capability profile for target {:?}",
                profile.target_token
            ));
        }
    }
    for target in &plan.targets {
        if !profiles_by_target.contains_key(target.target_token.as_str()) {
            input_errors.push(format!(
                "rollout target {:?} has no adapter capability profile",
                target.target_token
            ));
        }
    }
    if !input_errors.is_empty() {
        return Err(input_errors);
    }

    let mut blockers = Vec::new();
    for target in &plan.targets {
        let profile = profiles_by_target
            .get(target.target_token.as_str())
            .expect("all planned targets have a profile");
        let assessment = qualify_adapter(profile, qualification_policy).map_err(|errors| {
            errors
                .into_iter()
                .map(|error| {
                    format!(
                        "target {:?} has invalid adapter profile: {error}",
                        target.target_token
                    )
                })
                .collect::<Vec<_>>()
        })?;
        if !assessment.automatic_rollout_eligible {
            blockers.extend(assessment.blockers.into_iter().map(|reason| {
                format!(
                    "target {:?} adapter {}@{} is not eligible: {reason}",
                    assessment.target_token, assessment.adapter_id, assessment.adapter_version
                )
            }));
        }
    }
    if !blockers.is_empty() {
        return Ok(RolloutGateDecision::Halted {
            wave_index: 0,
            reasons: blockers,
        });
    }

    authorize_next_wave(plan, observations, verifier_now_unix_ms)
}

/// Validate the whole plan, then authorize the earliest wave that has not yet
/// started. No later wave is authorized until every target in each prior wave
/// has fresh matching evidence, a successful independent probe, and a passed
/// positive control.
pub fn authorize_next_wave(
    plan: &StagedRolloutPlan,
    observations: &[TargetRolloutObservation],
    verifier_now_unix_ms: u64,
) -> Result<RolloutGateDecision, Vec<String>> {
    validate_plan(plan)?;

    let targets_by_token: HashMap<&str, &RolloutTarget> = plan
        .targets
        .iter()
        .map(|target| (target.target_token.as_str(), target))
        .collect();
    let mut observations_by_token: HashMap<&str, &TargetRolloutObservation> = HashMap::new();
    let mut errors = Vec::new();

    for observation in observations {
        let Some(target) = targets_by_token.get(observation.target_token.as_str()) else {
            errors.push(format!(
                "observation references target {:?} outside this rollout",
                observation.target_token
            ));
            continue;
        };
        if observations_by_token
            .insert(observation.target_token.as_str(), observation)
            .is_some()
        {
            errors.push(format!(
                "duplicate current observation for target {:?}",
                observation.target_token
            ));
        }
        if observation.policy_generation != target.policy_generation {
            errors.push(format!(
                "target {:?} observation generation {} differs from planned generation {}",
                observation.target_token, observation.policy_generation, target.policy_generation
            ));
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    let mut waves: BTreeMap<usize, Vec<&RolloutTarget>> = BTreeMap::new();
    for target in &plan.targets {
        waves.entry(target.wave_index).or_default().push(target);
    }
    let final_wave = *waves
        .keys()
        .next_back()
        .ok_or_else(|| vec!["rollout plan has no waves".to_owned()])?;

    for wave_index in 0..=final_wave {
        let Some(wave_targets) = waves.get(&wave_index) else {
            return Err(vec![format!(
                "rollout plan has a gap at wave index {wave_index}"
            )]);
        };

        let found = wave_targets
            .iter()
            .filter(|target| observations_by_token.contains_key(target.target_token.as_str()))
            .count();

        if found == 0 {
            let future_evidence = observations_by_token.keys().any(|token| {
                targets_by_token
                    .get(token)
                    .is_some_and(|target| target.wave_index > wave_index)
            });
            if future_evidence {
                return Ok(RolloutGateDecision::Halted {
                    wave_index,
                    reasons: vec![
                        "future-wave observations exist before this wave has been completed"
                            .to_owned(),
                    ],
                });
            }
            if wave_index == 0
                || all_prior_waves_passed(
                    &waves,
                    &observations_by_token,
                    wave_index,
                    plan,
                    verifier_now_unix_ms,
                )
            {
                return Ok(RolloutGateDecision::WaveAuthorized {
                    wave_index,
                    targets: wave_targets
                        .iter()
                        .map(|target| target.target_token.clone())
                        .collect(),
                });
            }
            return Ok(RolloutGateDecision::Halted {
                wave_index,
                reasons: vec!["prior waves have not satisfied the rollout gate".to_owned()],
            });
        }

        if found < wave_targets.len() {
            let missing: Vec<String> = wave_targets
                .iter()
                .filter(|target| !observations_by_token.contains_key(target.target_token.as_str()))
                .map(|target| target.target_token.clone())
                .collect();
            return Ok(RolloutGateDecision::Halted {
                wave_index,
                reasons: vec![format!(
                    "wave is partially observed; missing outcomes for {} target(s): {}",
                    missing.len(),
                    missing.join(", ")
                )],
            });
        }

        let reasons = wave_failure_reasons(
            wave_targets,
            &observations_by_token,
            plan,
            verifier_now_unix_ms,
        );
        if !reasons.is_empty() {
            return Ok(RolloutGateDecision::Halted {
                wave_index,
                reasons,
            });
        }

        if wave_index == final_wave {
            return Ok(RolloutGateDecision::Complete);
        }
    }

    Err(vec![
        "rollout gate exhausted its wave sequence without a decision".to_owned(),
    ])
}

fn all_prior_waves_passed(
    waves: &BTreeMap<usize, Vec<&RolloutTarget>>,
    observations: &HashMap<&str, &TargetRolloutObservation>,
    before_wave: usize,
    plan: &StagedRolloutPlan,
    now_unix_ms: u64,
) -> bool {
    (0..before_wave).all(|wave_index| {
        waves.get(&wave_index).is_some_and(|targets| {
            wave_failure_reasons(targets, observations, plan, now_unix_ms).is_empty()
        })
    })
}

fn wave_failure_reasons(
    targets: &[&RolloutTarget],
    observations: &HashMap<&str, &TargetRolloutObservation>,
    plan: &StagedRolloutPlan,
    now_unix_ms: u64,
) -> Vec<String> {
    let mut reasons = Vec::new();
    for target in targets {
        let Some(observation) = observations.get(target.target_token.as_str()) else {
            reasons.push(format!(
                "target {:?} has no current rollout observation",
                target.target_token
            ));
            continue;
        };

        if observation.outcome != TargetRolloutOutcome::Promoted {
            reasons.push(format!(
                "target {:?} outcome is {:?}, not Promoted",
                target.target_token, observation.outcome
            ));
        }
        if observation.effective_policy_sha256 != plan.candidate_policy_sha256 {
            reasons.push(format!(
                "target {:?} effective-policy digest does not match the rollout candidate",
                target.target_token
            ));
        }
        if !observation.independent_probe_passed {
            reasons.push(format!(
                "target {:?} lacks a successful independent data-plane probe",
                target.target_token
            ));
        }
        if !observation.positive_control_passed {
            reasons.push(format!(
                "target {:?} positive-control path failed; a denial cannot be treated as success",
                target.target_token
            ));
        }
        if observation.expires_at_unix_ms < observation.observed_at_unix_ms
            || observation.observed_at_unix_ms > now_unix_ms
            || now_unix_ms > observation.expires_at_unix_ms
            || now_unix_ms - observation.observed_at_unix_ms > plan.max_evidence_age_ms
        {
            reasons.push(format!(
                "target {:?} evidence is future-dated, expired, or older than the rollout freshness bound",
                target.target_token
            ));
        }
    }
    reasons
}

fn validate_plan(plan: &StagedRolloutPlan) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if !is_identifier(&plan.rollout_id) {
        errors.push(
            "rollout_id must be a non-empty ASCII identifier of at most 80 characters".to_owned(),
        );
    }
    if !is_digest(&plan.candidate_policy_sha256) {
        errors
            .push("candidate_policy_sha256 must be 64 lowercase hexadecimal characters".to_owned());
    }
    if plan.targets.is_empty() {
        errors.push("rollout plan must contain at least one target".to_owned());
    }
    if plan.max_targets_per_wave == 0 {
        errors.push("max_targets_per_wave must be positive".to_owned());
    }
    if plan.max_targets_per_failure_domain_per_wave == 0 {
        errors.push("max_targets_per_failure_domain_per_wave must be positive".to_owned());
    }
    if plan.max_evidence_age_ms == 0 {
        errors.push("max_evidence_age_ms must be positive".to_owned());
    }

    let mut target_tokens = HashSet::new();
    let mut waves: BTreeMap<usize, Vec<&RolloutTarget>> = BTreeMap::new();
    for target in &plan.targets {
        if !is_target_token(&target.target_token) {
            errors.push(format!("target token {:?} is invalid", target.target_token));
        }
        if !target_tokens.insert(target.target_token.as_str()) {
            errors.push(format!(
                "duplicate planned target {:?}",
                target.target_token
            ));
        }
        if target.failure_domain.trim().is_empty()
            || target.failure_domain.len() > 80
            || !target
                .failure_domain
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        {
            errors.push(format!(
                "target {:?} has an invalid failure-domain identifier",
                target.target_token
            ));
        }
        if target.policy_generation == 0 {
            errors.push(format!(
                "target {:?} has a zero policy generation",
                target.target_token
            ));
        }
        waves.entry(target.wave_index).or_default().push(target);
    }

    if !waves.is_empty() {
        let final_wave = *waves.keys().next_back().expect("non-empty waves");
        // Every wave must contain at least one target, so a valid final index
        // can never be as large as the number of targets. Check this before
        // iterating the index range to bound work on malformed plans.
        if final_wave >= plan.targets.len() {
            errors.push(
                "wave index exceeds target count; wave indices must be contiguous from zero"
                    .to_owned(),
            );
        } else {
            for wave_index in 0..=final_wave {
                let Some(targets) = waves.get(&wave_index) else {
                    errors.push(format!(
                        "wave indices must be contiguous; missing wave {wave_index}"
                    ));
                    continue;
                };
                if targets.len() > plan.max_targets_per_wave {
                    errors.push(format!(
                        "wave {wave_index} has {} targets, over the configured limit {}",
                        targets.len(),
                        plan.max_targets_per_wave
                    ));
                }
                let mut domain_counts: HashMap<&str, usize> = HashMap::new();
                for target in targets {
                    *domain_counts.entry(&target.failure_domain).or_default() += 1;
                }
                for (domain, count) in domain_counts {
                    if count > plan.max_targets_per_failure_domain_per_wave {
                        errors.push(format!(
                        "wave {wave_index} exceeds the concurrent target limit for failure domain {domain:?}"
                    ));
                    }
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn is_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn is_target_token(value: &str) -> bool {
    value.starts_with("target-")
        && (8..=55).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::{
        RolloutGateDecision, RolloutTarget, StagedRolloutPlan, TargetRolloutObservation,
        TargetRolloutOutcome, authorize_next_wave, authorize_next_wave_with_adapter_capabilities,
    };
    use crate::adapter_capability::{
        AdapterQualificationPolicy, CommitSemantics, DeviceAdapterCapabilities, RollbackSemantics,
    };

    const CANDIDATE: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn target(token: &str, domain: &str, wave_index: usize) -> RolloutTarget {
        RolloutTarget {
            target_token: token.to_owned(),
            failure_domain: domain.to_owned(),
            policy_generation: 7,
            wave_index,
        }
    }

    fn adapter_profile(target_token: &str, use_gnmi: bool) -> DeviceAdapterCapabilities {
        DeviceAdapterCapabilities {
            target_token: target_token.to_owned(),
            adapter_id: if use_gnmi {
                "adapter-gnmi".to_owned()
            } else {
                "adapter-netconf".to_owned()
            },
            adapter_version: "v1".to_owned(),
            commit_semantics: if use_gnmi {
                CommitSemantics::GnmiSingleSet {
                    atomic_within_single_request: true,
                }
            } else {
                CommitSemantics::NetconfConfirmedCommit {
                    confirm_timeout_secs: 180,
                    cancel_commit_supported: true,
                    candidate_lock_supported: true,
                    reverts_on_session_loss: true,
                }
            },
            rollback_semantics: if use_gnmi {
                RollbackSemantics::Explicit {
                    rollback_is_idempotent: true,
                    baseline_policy_recoverable: true,
                }
            } else {
                RollbackSemantics::NativeConfirmedCommitTimeout
            },
            supports_effective_policy_readback: true,
            readback_digest_is_effective_policy: true,
            supports_independent_dataplane_probe: true,
            supports_positive_control_probe: true,
            supports_device_side_generation_fence: true,
            supports_exclusive_config_lock: true,
        }
    }

    fn adapter_policy() -> AdapterQualificationPolicy {
        AdapterQualificationPolicy {
            minimum_confirmed_commit_timeout_secs: 120,
            require_device_side_generation_fence: false,
            allow_explicit_rollback: true,
        }
    }

    fn profiles_for(plan: &StagedRolloutPlan) -> Vec<DeviceAdapterCapabilities> {
        plan.targets
            .iter()
            .map(|target| adapter_profile(&target.target_token, false))
            .collect()
    }

    fn plan() -> StagedRolloutPlan {
        StagedRolloutPlan {
            rollout_id: "rollout-edge-0001".to_owned(),
            candidate_policy_sha256: CANDIDATE.to_owned(),
            targets: vec![
                target("target-router-a", "site-a", 0),
                target("target-router-b", "site-b", 0),
                target("target-router-c", "site-c", 1),
            ],
            max_targets_per_wave: 2,
            max_targets_per_failure_domain_per_wave: 1,
            max_evidence_age_ms: 500,
        }
    }

    fn observation(
        token: &str,
        outcome: TargetRolloutOutcome,
        probe: bool,
        control: bool,
        observed: u64,
        expires: u64,
    ) -> TargetRolloutObservation {
        TargetRolloutObservation {
            target_token: token.to_owned(),
            policy_generation: 7,
            effective_policy_sha256: CANDIDATE.to_owned(),
            outcome,
            independent_probe_passed: probe,
            positive_control_passed: control,
            observed_at_unix_ms: observed,
            expires_at_unix_ms: expires,
        }
    }

    fn successful(token: &str) -> TargetRolloutObservation {
        observation(
            token,
            TargetRolloutOutcome::Promoted,
            true,
            true,
            1_000,
            1_500,
        )
    }

    #[test]
    fn first_wave_is_authorized_within_blast_radius_and_failure_domain_limits() {
        assert_eq!(
            authorize_next_wave(&plan(), &[], 1_100).expect("valid plan"),
            RolloutGateDecision::WaveAuthorized {
                wave_index: 0,
                targets: vec!["target-router-a".to_owned(), "target-router-b".to_owned()]
            }
        );
    }

    #[test]
    fn adapter_preflight_authorizes_first_wave_only_when_every_target_is_eligible() {
        let plan = plan();
        let profiles = profiles_for(&plan);
        assert_eq!(
            authorize_next_wave_with_adapter_capabilities(
                &plan,
                &[],
                &profiles,
                &adapter_policy(),
                1_100,
            )
            .expect("eligible adapter profiles"),
            RolloutGateDecision::WaveAuthorized {
                wave_index: 0,
                targets: vec!["target-router-a".to_owned(), "target-router-b".to_owned()]
            }
        );
    }

    #[test]
    fn ineligible_later_wave_adapter_blocks_the_entire_rollout_preflight() {
        let plan = plan();
        let mut profiles = profiles_for(&plan);
        let last = profiles.last_mut().expect("third target profile");
        last.commit_semantics = CommitSemantics::GnmiSingleSet {
            atomic_within_single_request: true,
        };
        last.rollback_semantics = RollbackSemantics::None;
        let decision = authorize_next_wave_with_adapter_capabilities(
            &plan,
            &[],
            &profiles,
            &adapter_policy(),
            1_100,
        )
        .expect("valid but ineligible adapter profile");
        let RolloutGateDecision::Halted {
            wave_index,
            reasons,
        } = decision
        else {
            panic!("ineligible future-wave device must block before wave zero");
        };
        assert_eq!(wave_index, 0);
        assert!(
            reasons
                .iter()
                .any(|reason| reason.contains("target-router-c")
                    && reason.contains("without a qualified rollback"))
        );
    }

    #[test]
    fn missing_or_duplicate_adapter_profiles_are_rejected() {
        let plan = plan();
        let mut profiles = profiles_for(&plan);
        profiles.pop();
        assert!(
            authorize_next_wave_with_adapter_capabilities(
                &plan,
                &[],
                &profiles,
                &adapter_policy(),
                1_100,
            )
            .expect_err("missing adapter")
            .iter()
            .any(|reason| reason.contains("has no adapter capability profile"))
        );

        let mut profiles = profiles_for(&plan);
        profiles.push(profiles[0].clone());
        assert!(
            authorize_next_wave_with_adapter_capabilities(
                &plan,
                &[],
                &profiles,
                &adapter_policy(),
                1_100,
            )
            .expect_err("duplicate adapter profile")
            .iter()
            .any(|reason| reason.contains("duplicate adapter capability profile"))
        );
    }

    #[test]
    fn next_wave_requires_every_prior_target_to_have_fresh_matching_positive_evidence() {
        let observations = vec![successful("target-router-a"), successful("target-router-b")];
        assert_eq!(
            authorize_next_wave(&plan(), &observations, 1_100).expect("valid plan"),
            RolloutGateDecision::WaveAuthorized {
                wave_index: 1,
                targets: vec!["target-router-c".to_owned()]
            }
        );
    }

    #[test]
    fn partial_wave_never_authorizes_expansion() {
        let observations = vec![successful("target-router-a")];
        let decision = authorize_next_wave(&plan(), &observations, 1_100).expect("valid plan");
        assert!(matches!(
            decision,
            RolloutGateDecision::Halted { wave_index: 0, .. }
        ));
    }

    #[test]
    fn failed_target_or_rollback_halts_before_next_wave() {
        let mut failed = successful("target-router-a");
        failed.outcome = TargetRolloutOutcome::RolledBack;
        let observations = vec![failed, successful("target-router-b")];
        let decision = authorize_next_wave(&plan(), &observations, 1_100).expect("valid plan");
        let RolloutGateDecision::Halted {
            wave_index,
            reasons,
        } = decision
        else {
            panic!("failed first wave must halt");
        };
        assert_eq!(wave_index, 0);
        assert!(reasons.iter().any(|reason| reason.contains("not Promoted")));
    }

    #[test]
    fn stale_future_dated_wrong_digest_or_failed_control_cannot_pass() {
        let mut stale = successful("target-router-a");
        stale.expires_at_unix_ms = 1_050;
        let observations = vec![stale, successful("target-router-b")];
        let decision = authorize_next_wave(&plan(), &observations, 1_100).expect("valid plan");
        assert!(matches!(
            decision,
            RolloutGateDecision::Halted { wave_index: 0, .. }
        ));

        let mut wrong_digest = successful("target-router-a");
        wrong_digest.effective_policy_sha256 = "a".repeat(64);
        let observations = vec![wrong_digest, successful("target-router-b")];
        let decision = authorize_next_wave(&plan(), &observations, 1_100).expect("valid plan");
        assert!(matches!(
            decision,
            RolloutGateDecision::Halted { wave_index: 0, .. }
        ));

        let mut control_failed = successful("target-router-a");
        control_failed.positive_control_passed = false;
        let observations = vec![control_failed, successful("target-router-b")];
        let decision = authorize_next_wave(&plan(), &observations, 1_100).expect("valid plan");
        assert!(matches!(
            decision,
            RolloutGateDecision::Halted { wave_index: 0, .. }
        ));
    }

    #[test]
    fn later_wave_evidence_cannot_skip_a_missing_wave() {
        let observations = vec![successful("target-router-c")];
        let decision = authorize_next_wave(&plan(), &observations, 1_100).expect("valid plan");
        assert!(matches!(
            decision,
            RolloutGateDecision::Halted { wave_index: 0, .. }
        ));
    }

    #[test]
    fn completed_rollout_requires_fresh_positive_evidence_for_every_target() {
        let observations = vec![
            successful("target-router-a"),
            successful("target-router-b"),
            successful("target-router-c"),
        ];
        assert_eq!(
            authorize_next_wave(&plan(), &observations, 1_100).expect("valid plan"),
            RolloutGateDecision::Complete
        );
    }

    #[test]
    fn malformed_extreme_wave_index_is_rejected_without_unbounded_iteration() {
        let mut invalid = plan();
        invalid.targets[0].wave_index = usize::MAX;
        assert!(
            authorize_next_wave(&invalid, &[], 1_100)
                .expect_err("extreme wave index rejected")
                .iter()
                .any(|reason| reason.contains("wave index exceeds target count"))
        );
    }

    #[test]
    fn malformed_plan_duplicate_targets_and_domain_overload_are_rejected() {
        let mut invalid = plan();
        invalid.targets.push(target("target-router-a", "site-d", 1));
        assert!(
            authorize_next_wave(&invalid, &[], 1_100)
                .expect_err("duplicate target rejected")
                .iter()
                .any(|reason| reason.contains("duplicate planned target"))
        );

        let mut invalid = plan();
        invalid.targets = vec![
            target("target-router-a", "site-a", 0),
            target("target-router-b", "site-a", 0),
        ];
        assert!(
            authorize_next_wave(&invalid, &[], 1_100)
                .expect_err("failure-domain concentration rejected")
                .iter()
                .any(|reason| reason.contains("failure domain"))
        );
    }

    #[test]
    fn one_hundred_thousand_target_plan_is_checked_deterministically() {
        let targets = (0..100_000)
            .map(|index| {
                let wave_index = index / 1_000;
                target(
                    &format!("target-node-{index}"),
                    &format!("site-{}", index % 1_000),
                    wave_index,
                )
            })
            .collect();
        let large_plan = StagedRolloutPlan {
            rollout_id: "rollout-scale-0001".to_owned(),
            candidate_policy_sha256: CANDIDATE.to_owned(),
            targets,
            max_targets_per_wave: 1_000,
            max_targets_per_failure_domain_per_wave: 1,
            max_evidence_age_ms: 60_000,
        };
        assert_eq!(
            authorize_next_wave(&large_plan, &[], 1_100).expect("valid scale plan"),
            RolloutGateDecision::WaveAuthorized {
                wave_index: 0,
                targets: (0..1_000)
                    .map(|index| format!("target-node-{index}"))
                    .collect()
            }
        );
    }
}
