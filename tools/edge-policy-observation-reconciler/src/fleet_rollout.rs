//! Deterministic fleet rollout planning and evidence-gated wave decisions.
//!
//! This module does not apply policies to devices. It limits planned blast radius
//! and decides whether evidence is sufficient to continue a rollout, roll back
//! the current wave, or stop for reconciliation. The caller remains responsible
//! for enforcing these decisions against real devices and for recording actuation.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use super::promotion_journal::{PromotionJournal, PromotionPermit, VerifiedPromotionBinding};

/// Target and policy identity needed to plan one rollout operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetTarget {
    /// Stable non-secret target token, matching the challenge protocol.
    pub target_token: String,
    /// Explicit failure domain (e.g. site, zone, rack, access network).
    pub failure_domain: String,
    /// Strictly increasing policy generation for this target.
    pub generation: u64,
    /// Exact currently-approved effective-policy digest.
    pub baseline_policy_sha256: String,
    /// Exact candidate effective-policy digest expected after apply.
    pub candidate_policy_sha256: String,
    /// Exactly one target must be explicitly designated as the canary.
    pub canary: bool,
}

/// Upper bounds for the planned blast radius of a wave.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RolloutConstraints {
    /// Maximum devices changed in one wave, including the canary wave.
    pub max_batch_size: usize,
    /// Maximum devices from any one failure domain in a single wave.
    pub max_targets_per_failure_domain: usize,
}

/// A deterministic, fixed membership rollout wave.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RolloutWave {
    /// Zero-based wave number.
    pub index: usize,
    /// The canary is always isolated in wave zero.
    pub is_canary: bool,
    pub targets: Vec<FleetTarget>,
}

/// Deterministic plan digest is an identity for plan contents, not a signature.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetRolloutPlan {
    pub schema_version: &'static str,
    pub constraints: RolloutConstraints,
    pub waves: Vec<RolloutWave>,
    pub plan_sha256: String,
}

/// Plan validation failure. No external side effects occur while planning.
pub fn plan_rollout(
    targets: &[FleetTarget],
    constraints: RolloutConstraints,
) -> Result<FleetRolloutPlan, String> {
    if targets.is_empty() {
        return Err("rollout must include at least one target".to_owned());
    }
    if constraints.max_batch_size == 0 {
        return Err("max_batch_size must be positive".to_owned());
    }
    if constraints.max_targets_per_failure_domain == 0 {
        return Err("max_targets_per_failure_domain must be positive".to_owned());
    }

    let mut target_tokens = BTreeSet::new();
    let mut canaries = Vec::new();
    let mut by_domain: BTreeMap<String, Vec<FleetTarget>> = BTreeMap::new();

    for target in targets {
        validate_target(target)?;
        if !target_tokens.insert(target.target_token.as_str()) {
            return Err(format!("duplicate target token {:?}", target.target_token));
        }
        if target.canary {
            canaries.push(target.clone());
        } else {
            by_domain
                .entry(target.failure_domain.clone())
                .or_default()
                .push(target.clone());
        }
    }

    if canaries.len() != 1 {
        return Err(format!(
            "exactly one explicitly designated canary is required; found {}",
            canaries.len()
        ));
    }

    for domain_targets in by_domain.values_mut() {
        domain_targets.sort_by(|left, right| left.target_token.cmp(&right.target_token));
    }

    // The canary is isolated regardless of larger batch limits. A declared
    // canary must not share a failure-domain wave with any other changed target.
    let mut waves = vec![RolloutWave {
        index: 0,
        is_canary: true,
        targets: canaries,
    }];

    // Round-robin across lexicographically ordered failure domains. The loop is
    // deterministic and respects both per-wave constraints at every insertion.
    let mut queues: BTreeMap<String, VecDeque<FleetTarget>> = by_domain
        .into_iter()
        .map(|(domain, mut values)| {
            values.sort_by(|left, right| left.target_token.cmp(&right.target_token));
            (domain, VecDeque::from(values))
        })
        .collect();

    while queues.values().any(|queue| !queue.is_empty()) {
        let mut wave_targets = Vec::new();
        let mut domain_counts: BTreeMap<String, usize> = BTreeMap::new();

        loop {
            if wave_targets.len() >= constraints.max_batch_size {
                break;
            }
            let mut made_progress = false;
            for (domain, queue) in &mut queues {
                if wave_targets.len() >= constraints.max_batch_size {
                    break;
                }
                if queue.is_empty()
                    || domain_counts.get(domain).copied().unwrap_or(0)
                        >= constraints.max_targets_per_failure_domain
                {
                    continue;
                }
                if let Some(target) = queue.pop_front() {
                    *domain_counts.entry(domain.clone()).or_default() += 1;
                    wave_targets.push(target);
                    made_progress = true;
                }
            }
            if !made_progress {
                break;
            }
        }

        if wave_targets.is_empty() {
            return Err("planner could not make progress while targets remain".to_owned());
        }
        waves.push(RolloutWave {
            index: waves.len(),
            is_canary: false,
            targets: wave_targets,
        });
    }

    let plan_sha256 = digest_plan(&waves, constraints);
    Ok(FleetRolloutPlan {
        schema_version: "1.0.0",
        constraints,
        waves,
        plan_sha256,
    })
}

pub(crate) fn validate_target(target: &FleetTarget) -> Result<(), String> {
    if !valid_target_token(&target.target_token) {
        return Err(format!(
            "target token {:?} does not match target- plus lowercase ASCII letters, digits, and hyphens",
            target.target_token
        ));
    }
    if !valid_identifier(&target.failure_domain) {
        return Err(format!(
            "failure domain {:?} must be a non-empty lowercase ASCII identifier of at most 80 characters",
            target.failure_domain
        ));
    }
    if target.generation == 0 || i64::try_from(target.generation).is_err() {
        return Err(format!(
            "target {:?} generation must be a positive SQLite-range integer",
            target.target_token
        ));
    }
    validate_digest(&target.baseline_policy_sha256, "baseline_policy_sha256")?;
    validate_digest(&target.candidate_policy_sha256, "candidate_policy_sha256")?;
    if target.baseline_policy_sha256 == target.candidate_policy_sha256 {
        return Err(format!(
            "target {:?} candidate digest must differ from its baseline digest",
            target.target_token
        ));
    }
    Ok(())
}

fn valid_target_token(value: &str) -> bool {
    value.starts_with("target-")
        && (8..=55).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
}

fn validate_digest(value: &str, label: &str) -> Result<(), String> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!(
            "{label} must be 64 lowercase hexadecimal characters"
        ));
    }
    Ok(())
}

pub(crate) fn digest_plan(waves: &[RolloutWave], constraints: RolloutConstraints) -> String {
    use std::fmt::Write as _;
    let mut canonical = String::from("luminous-edge-fleet-rollout-plan-v1\n");
    writeln!(
        &mut canonical,
        "constraints|{}|{}",
        constraints.max_batch_size, constraints.max_targets_per_failure_domain
    )
    .expect("writing to String cannot fail");
    for wave in waves {
        use std::fmt::Write as _;
        writeln!(
            &mut canonical,
            "wave|{}|{}",
            wave.index,
            if wave.is_canary { "canary" } else { "general" }
        )
        .expect("writing to String cannot fail");
        for target in &wave.targets {
            writeln!(
                &mut canonical,
                "target|{}|{}|{}|{}|{}|{}",
                target.target_token,
                target.failure_domain,
                target.generation,
                target.baseline_policy_sha256,
                target.candidate_policy_sha256,
                if target.canary { "canary" } else { "general" }
            )
            .expect("writing to String cannot fail");
        }
    }
    super::sha256_hex(canonical.as_bytes())
}

/// Outcome of an independent application/data-plane probe.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProbeOutcome {
    /// The intended application-level behavior was observed.
    Passed,
    /// The probe definitely observed the wrong application-level behavior.
    Failed,
    /// The probe was inconclusive; this is never treated as a pass.
    Inconclusive,
}

/// Synthetic claim container used only by unit tests for the pure classifier.
/// It is deliberately absent from non-test builds: callers cannot construct
/// freshness/nonce booleans and feed them into a production wave-decision API.
#[cfg(test)]
#[derive(Clone, Debug, Eq, PartialEq)]
struct FleetTargetObservation {
    pub target_token: String,
    pub policy_generation: u64,
    pub effective_policy_sha256: String,
    pub freshness_valid: bool,
    pub nonce_durably_consumed: bool,
    pub independent_probe: bool,
    /// Must pass to distinguish policy denial from dead listener/path/observer.
    pub positive_control_passed: bool,
    pub outcome: ProbeOutcome,
}

/// Fail-closed action chosen after evaluating one complete wave.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WaveDecision {
    /// Every target passed with fresh, correctly-bound evidence.
    Continue,
    /// At least one definitive negative probe was measured while the positive
    /// control succeeded. Roll back every applied target in this wave.
    RollbackWave,
    /// Evidence or target state is uncertain. Stop the rollout and reconcile;
    /// do not guess whether applying/rolling back is safe.
    HaltAndReconcile,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WaveEvaluation {
    pub wave_index: usize,
    pub decision: WaveDecision,
    pub passed_targets: Vec<String>,
    pub failed_targets: Vec<String>,
    pub reasons: Vec<String>,
}

/// Opaque, non-serializable proof that all targets in one planned wave have
/// live promotion permits bound to the exact target, generation, baseline and
/// candidate policy digest. This is not itself a rollout decision and carries no
/// proof that an independent data-plane probe was authentic or successful.
#[derive(Debug)]
pub struct VerifiedWaveBindings {
    wave_index: usize,
    bindings: Vec<VerifiedPromotionBinding>,
}

impl VerifiedWaveBindings {
    pub fn wave_index(&self) -> usize {
        self.wave_index
    }

    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }

    pub fn bindings(&self) -> &[VerifiedPromotionBinding] {
        &self.bindings
    }
}

/// Check the durable, process-local promotion capabilities before a coordinator
/// can begin a wave. Every target must have exactly one current permit matching
/// the plan. This function intentionally does not return Continue/Rollback:
/// independent probe verification is a separate required capability, and raw
/// caller-supplied booleans must not authorize actuation.
pub fn validate_wave_permits(
    journal: &PromotionJournal,
    wave: &RolloutWave,
    permits: &[&PromotionPermit],
    now_unix_ms: u64,
) -> Result<VerifiedWaveBindings, WaveEvaluation> {
    let halted = |reason: String| WaveEvaluation {
        wave_index: wave.index,
        decision: WaveDecision::HaltAndReconcile,
        passed_targets: Vec::new(),
        failed_targets: Vec::new(),
        reasons: vec![reason],
    };
    if wave.targets.is_empty() {
        return Err(halted("cannot validate permits for an empty wave".to_owned()));
    }
    if permits.len() != wave.targets.len() {
        return Err(halted(format!(
            "permit count mismatch: expected {}, received {}",
            wave.targets.len(),
            permits.len()
        )));
    }

    let mut expected = BTreeMap::new();
    for target in &wave.targets {
        if expected.insert(target.target_token.as_str(), target).is_some() {
            return Err(halted(format!(
                "wave contains duplicate target token {:?}",
                target.target_token
            )));
        }
        if let Err(error) = validate_target(target) {
            return Err(halted(format!(
                "planned target {:?} is invalid: {error}",
                target.target_token
            )));
        }
    }

    let mut seen = BTreeSet::new();
    let mut bindings = Vec::with_capacity(permits.len());
    for permit in permits {
        let binding = match journal.validate_permit(permit, now_unix_ms) {
            Ok(binding) => binding,
            Err(error) => return Err(halted(format!("promotion permit rejected: {error}"))),
        };
        let token = binding.target_token();
        let Some(target) = expected.get(token).copied() else {
            return Err(halted(format!(
                "promotion permit references target {token:?} outside this wave"
            )));
        };
        if !seen.insert(token.to_owned()) {
            return Err(halted(format!(
                "duplicate promotion permit for target {token:?}"
            )));
        }
        if binding.generation() != target.generation
            || binding.baseline_policy_sha256() != target.baseline_policy_sha256
            || binding.candidate_policy_sha256() != target.candidate_policy_sha256
        {
            return Err(halted(format!(
                "promotion permit for target {token:?} does not match the wave's generation and policy digests"
            )));
        }
        bindings.push(binding);
    }

    if seen.len() != expected.len() {
        let missing = expected
            .keys()
            .filter(|token| !seen.contains(**token))
            .copied()
            .collect::<Vec<_>>();
        return Err(halted(format!(
            "wave has no validated promotion permit for target(s): {}",
            missing.join(", ")
        )));
    }

    bindings.sort_by(|left, right| left.target_token().cmp(right.target_token()));
    Ok(VerifiedWaveBindings {
        wave_index: wave.index,
        bindings,
    })
}

/// Evaluate a wave's observations. The caller must only supply observations
/// after the durable freshness/conformance gate; the explicit fields are checked
/// again here to fail closed on broken orchestration or incomplete evidence.
#[cfg(test)]
fn evaluate_wave_claims(
    wave: &RolloutWave,
    observations: &[FleetTargetObservation],
) -> WaveEvaluation {
    let mut result = WaveEvaluation {
        wave_index: wave.index,
        decision: WaveDecision::HaltAndReconcile,
        passed_targets: Vec::new(),
        failed_targets: Vec::new(),
        reasons: Vec::new(),
    };

    if wave.targets.is_empty() {
        result
            .reasons
            .push("cannot evaluate an empty wave".to_owned());
        return result;
    }

    let expected: BTreeMap<&str, &FleetTarget> = wave
        .targets
        .iter()
        .map(|target| (target.target_token.as_str(), target))
        .collect();
    if expected.len() != wave.targets.len() {
        result
            .reasons
            .push("wave contains duplicate target tokens".to_owned());
        return result;
    }

    let mut received: BTreeMap<&str, &FleetTargetObservation> = BTreeMap::new();
    for observation in observations {
        if !expected.contains_key(observation.target_token.as_str()) {
            result.reasons.push(format!(
                "observation references unexpected target {:?}",
                observation.target_token
            ));
            continue;
        }
        if received
            .insert(observation.target_token.as_str(), observation)
            .is_some()
        {
            result.reasons.push(format!(
                "duplicate observation for target {:?}",
                observation.target_token
            ));
        }
    }
    if !result.reasons.is_empty() {
        return result;
    }
    for token in expected.keys() {
        if !received.contains_key(token) {
            result
                .reasons
                .push(format!("missing observation for target {token:?}"));
        }
    }
    if !result.reasons.is_empty() {
        return result;
    }

    let mut definitive_failure = false;
    for (token, target) in expected {
        let Some(observation) = received.get(token).copied() else {
            result
                .reasons
                .push(format!("missing observation for target {token:?}"));
            continue;
        };
        if observation.policy_generation != target.generation {
            result.reasons.push(format!(
                "target {token:?} generation mismatch: expected {}, observed {}",
                target.generation, observation.policy_generation
            ));
            continue;
        }
        if observation.effective_policy_sha256 != target.candidate_policy_sha256 {
            result
                .reasons
                .push(format!("target {token:?} effective policy digest mismatch"));
            continue;
        }
        if !observation.freshness_valid {
            result
                .reasons
                .push(format!("target {token:?} evidence is not fresh"));
            continue;
        }
        if !observation.nonce_durably_consumed {
            result
                .reasons
                .push(format!("target {token:?} nonce was not durably consumed"));
            continue;
        }
        if !observation.independent_probe {
            result
                .reasons
                .push(format!("target {token:?} lacks an independent probe"));
            continue;
        }
        if !observation.positive_control_passed {
            result
                .reasons
                .push(format!("target {token:?} positive control failed"));
            continue;
        }

        match observation.outcome {
            ProbeOutcome::Passed => result.passed_targets.push(token.to_owned()),
            ProbeOutcome::Failed => {
                definitive_failure = true;
                result.failed_targets.push(token.to_owned());
            }
            ProbeOutcome::Inconclusive => result
                .reasons
                .push(format!("target {token:?} probe is inconclusive")),
        }
    }

    result.passed_targets.sort();
    result.failed_targets.sort();
    if !result.reasons.is_empty() {
        result.decision = WaveDecision::HaltAndReconcile;
    } else if definitive_failure {
        result.decision = WaveDecision::RollbackWave;
    } else {
        result.decision = WaveDecision::Continue;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::{
        FleetTarget, FleetTargetObservation, ProbeOutcome, RolloutConstraints, WaveDecision,
        evaluate_wave, plan_rollout,
    };

    const BASELINE: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const CANDIDATE: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn target(token: &str, domain: &str, canary: bool) -> FleetTarget {
        FleetTarget {
            target_token: token.to_owned(),
            failure_domain: domain.to_owned(),
            generation: 7,
            baseline_policy_sha256: BASELINE.to_owned(),
            candidate_policy_sha256: CANDIDATE.to_owned(),
            canary,
        }
    }

    fn targets() -> Vec<FleetTarget> {
        vec![
            target("target-canary", "site-a", true),
            target("target-a-01", "site-a", false),
            target("target-a-02", "site-a", false),
            target("target-b-01", "site-b", false),
            target("target-b-02", "site-b", false),
            target("target-c-01", "site-c", false),
        ]
    }

    fn constraints() -> RolloutConstraints {
        RolloutConstraints {
            max_batch_size: 3,
            max_targets_per_failure_domain: 1,
        }
    }

    fn pass_observation(target: &FleetTarget) -> FleetTargetObservation {
        FleetTargetObservation {
            target_token: target.target_token.clone(),
            policy_generation: target.generation,
            effective_policy_sha256: target.candidate_policy_sha256.clone(),
            freshness_valid: true,
            nonce_durably_consumed: true,
            independent_probe: true,
            positive_control_passed: true,
            outcome: ProbeOutcome::Passed,
        }
    }

    #[test]
    fn canary_is_always_isolated_and_general_waves_respect_blast_radius() {
        let plan = plan_rollout(&targets(), constraints()).expect("plan");
        assert_eq!(plan.waves[0].targets.len(), 1);
        assert!(plan.waves[0].is_canary);
        assert!(plan.waves[0].targets[0].canary);
        for wave in plan.waves.iter().skip(1) {
            assert!(!wave.is_canary);
            assert!(wave.targets.len() <= constraints().max_batch_size);
            let mut counts = std::collections::BTreeMap::<&str, usize>::new();
            for target in &wave.targets {
                *counts.entry(&target.failure_domain).or_default() += 1;
            }
            assert!(
                counts
                    .values()
                    .all(|count| *count <= constraints().max_targets_per_failure_domain)
            );
        }
    }

    #[test]
    fn plan_is_independent_of_input_order_and_has_stable_digest() {
        let original = targets();
        let first = plan_rollout(&original, constraints()).expect("first plan");
        let mut reversed = original;
        reversed.reverse();
        let second = plan_rollout(&reversed, constraints()).expect("permuted plan");
        assert_eq!(first, second);
        assert_eq!(first.plan_sha256.len(), 64);
    }

    #[test]
    fn planner_requires_exactly_one_explicit_canary() {
        let mut missing = targets();
        missing.retain(|target| !target.canary);
        assert!(
            plan_rollout(&missing, constraints())
                .expect_err("missing canary")
                .contains("exactly one")
        );

        let mut multiple = targets();
        multiple.push(target("target-canary-2", "site-z", true));
        assert!(
            plan_rollout(&multiple, constraints())
                .expect_err("multiple canaries")
                .contains("exactly one")
        );
    }

    #[test]
    fn planner_rejects_duplicate_target_and_invalid_digests() {
        let mut duplicate = targets();
        duplicate.push(duplicate[1].clone());
        assert!(
            plan_rollout(&duplicate, constraints())
                .expect_err("duplicate token")
                .contains("duplicate target")
        );

        let mut invalid = targets();
        invalid[1].candidate_policy_sha256 = "BAD".to_owned();
        assert!(
            plan_rollout(&invalid, constraints())
                .expect_err("invalid digest")
                .contains("64 lowercase")
        );
    }

    #[test]
    fn planner_requires_positive_bounds_and_candidate_distinct_from_baseline() {
        let mut no_batch = constraints();
        no_batch.max_batch_size = 0;
        assert!(plan_rollout(&targets(), no_batch).is_err());

        let mut same_digest = targets();
        same_digest[1].candidate_policy_sha256 = BASELINE.to_owned();
        assert!(
            plan_rollout(&same_digest, constraints())
                .expect_err("same digest")
                .contains("must differ")
        );
    }

    #[test]
    fn ten_thousand_targets_respect_deterministic_wave_and_domain_caps() {
        let targets: Vec<_> = (0..10_000)
            .map(|index| {
                target(
                    &format!("target-device-{index:05}"),
                    &format!("site-{:02}", index % 100),
                    index == 0,
                )
            })
            .collect();
        let limits = RolloutConstraints {
            max_batch_size: 100,
            max_targets_per_failure_domain: 1,
        };
        let plan = plan_rollout(&targets, limits).expect("10k-target plan");
        let planned_count: usize = plan.waves.iter().map(|wave| wave.targets.len()).sum();
        assert_eq!(planned_count, targets.len());
        assert_eq!(plan.waves.len(), 101);
        assert!(plan.waves[0].is_canary);
        for wave in plan.waves.iter().skip(1) {
            assert!(wave.targets.len() <= limits.max_batch_size);
            let mut per_domain = std::collections::BTreeMap::<&str, usize>::new();
            for target in &wave.targets {
                *per_domain
                    .entry(target.failure_domain.as_str())
                    .or_default() += 1;
            }
            assert!(
                per_domain
                    .values()
                    .all(|count| *count <= limits.max_targets_per_failure_domain)
            );
        }
    }

    #[test]
    fn valid_complete_wave_continues() {
        let plan = plan_rollout(&targets(), constraints()).expect("plan");
        let wave = &plan.waves[0];
        let observations: Vec<_> = wave.targets.iter().map(pass_observation).collect();
        let evaluation = evaluate_wave_claims(wave, &observations);
        assert_eq!(evaluation.decision, WaveDecision::Continue);
        assert_eq!(evaluation.passed_targets, vec!["target-canary"]);
        assert!(evaluation.reasons.is_empty());
    }

    #[test]
    fn a_definitive_failure_with_a_working_positive_control_rolls_back_wave() {
        let plan = plan_rollout(&targets(), constraints()).expect("plan");
        let wave = &plan.waves[1];
        let mut observations: Vec<_> = wave.targets.iter().map(pass_observation).collect();
        observations[0].outcome = ProbeOutcome::Failed;
        let evaluation = evaluate_wave_claims(wave, &observations);
        assert_eq!(evaluation.decision, WaveDecision::RollbackWave);
        assert_eq!(evaluation.failed_targets.len(), 1);
        assert_eq!(evaluation.passed_targets.len(), wave.targets.len() - 1);
    }

    #[test]
    fn missing_duplicate_or_unexpected_observation_halts() {
        let plan = plan_rollout(&targets(), constraints()).expect("plan");
        let wave = &plan.waves[1];
        let complete: Vec<_> = wave.targets.iter().map(pass_observation).collect();

        let missing = evaluate_wave_claims(wave, &complete[..complete.len() - 1]);
        assert_eq!(missing.decision, WaveDecision::HaltAndReconcile);

        let mut duplicate = complete.clone();
        duplicate.push(complete[0].clone());
        assert_eq!(
            evaluate_wave_claims(wave, &duplicate).decision,
            WaveDecision::HaltAndReconcile
        );

        let mut unexpected = complete;
        let mut unknown = pass_observation(&wave.targets[0]);
        unknown.target_token = "target-unexpected".to_owned();
        unexpected.push(unknown);
        assert_eq!(
            evaluate_wave_claims(wave, &unexpected).decision,
            WaveDecision::HaltAndReconcile
        );
    }

    #[test]
    fn stale_replay_unbound_or_non_independent_evidence_halts() {
        let plan = plan_rollout(&targets(), constraints()).expect("plan");
        let wave = &plan.waves[1];
        for mutate in 0..5 {
            let mut observations: Vec<_> = wave.targets.iter().map(pass_observation).collect();
            match mutate {
                0 => observations[0].freshness_valid = false,
                1 => observations[0].nonce_durably_consumed = false,
                2 => observations[0].independent_probe = false,
                3 => observations[0].positive_control_passed = false,
                4 => observations[0].effective_policy_sha256 = BASELINE.to_owned(),
                _ => unreachable!(),
            }
            let evaluation = evaluate_wave_claims(wave, &observations);
            assert_eq!(
                evaluation.decision,
                WaveDecision::HaltAndReconcile,
                "mutation {mutate} must fail closed"
            );
        }
    }

    #[test]
    fn target_generation_and_inconclusive_probe_halt() {
        let plan = plan_rollout(&targets(), constraints()).expect("plan");
        let wave = &plan.waves[1];
        let mut generation: Vec<_> = wave.targets.iter().map(pass_observation).collect();
        generation[0].policy_generation += 1;
        assert_eq!(
            evaluate_wave_claims(wave, &generation).decision,
            WaveDecision::HaltAndReconcile
        );

        let mut inconclusive: Vec<_> = wave.targets.iter().map(pass_observation).collect();
        inconclusive[0].outcome = ProbeOutcome::Inconclusive;
        assert_eq!(
            evaluate_wave_claims(wave, &inconclusive).decision,
            WaveDecision::HaltAndReconcile
        );
    }

    #[test]
    fn a_failed_positive_control_is_not_misclassified_as_a_policy_denial() {
        let plan = plan_rollout(&targets(), constraints()).expect("plan");
        let wave = &plan.waves[1];
        let mut observations: Vec<_> = wave.targets.iter().map(pass_observation).collect();
        observations[0].outcome = ProbeOutcome::Failed;
        observations[0].positive_control_passed = false;
        let evaluation = evaluate_wave_claims(wave, &observations);
        assert_eq!(evaluation.decision, WaveDecision::HaltAndReconcile);
        assert_ne!(evaluation.decision, WaveDecision::RollbackWave);
    }
}
