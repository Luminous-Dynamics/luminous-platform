//! Research model for tiered clock evidence in SRQ.
//!
//! This module models policy and state transitions only. In particular,
//! `signature_verified_by_trusted_verifier` is an abstract test input, not
//! cryptographic verification. Do not use this crate as a production clock,
//! freshness service, or release-qualification adapter.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClockTier {
    /// Host wall clock only; useful for local diagnostics, not rollback-safe.
    A0HostWall,
    /// Monotonic ordering within one boot; not comparable across reboot.
    A1BootMonotonic,
    /// A persistent accepted-time high-water mark detects rollback below it.
    /// It does not authenticate the first observation or protect against a
    /// writer who can restore/alter the same database.
    A2PersistentHighWater,
    /// A trusted verifier has authenticated a target/challenge/decision-bound
    /// freshness response under a configured trust policy.
    /// Cryptographic verification is outside this model.
    A3Authenticated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedTimeBinding {
    pub issuer_id: String,
    pub trust_policy_id: String,
    pub target_id: String,
    pub challenge_id: String,
    pub decision_id: String,
    pub issued_at_unix_seconds: u64,
    pub expires_at_unix_seconds: u64,
    /// Abstract signal from a separately trusted verifier. This is NOT a
    /// signature check and must never be sourced from untrusted JSON.
    pub signature_verified_by_trusted_verifier: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClockObservation {
    pub source_id: String,
    pub tier: ClockTier,
    pub unix_seconds: u64,
    pub boot_id: Option<String>,
    pub monotonic_ticks: Option<u64>,
    pub authenticated_time: Option<AuthenticatedTimeBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClockPolicy {
    pub minimum_tier: ClockTier,
    pub expected_source_id: String,
    pub expected_issuer_id: String,
    pub expected_trust_policy_id: String,
    pub expected_target_id: String,
    pub expected_challenge_id: String,
    pub expected_decision_id: String,
    /// Required for A3. This reference must come from the trusted challenge
    /// or verification policy, not from the observation being checked.
    pub reference_unix_seconds: Option<u64>,
    pub allowed_future_skew_seconds: u64,
    pub maximum_forward_step_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonotonicPoint {
    pub boot_id: String,
    pub ticks: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClockState {
    pub revision: u64,
    pub high_water_unix_seconds: Option<u64>,
    pub last_monotonic: Option<MonotonicPoint>,
    pub committed_observations: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClockReject {
    InsufficientTier,
    WrongSource,
    HighWaterUninitialized,
    WallClockRollback,
    ForwardJump,
    MissingBootIdentity,
    MissingMonotonicCounter,
    MonotonicRollback,
    RebootIncomparable,
    MissingAuthenticatedTime,
    UnverifiedAuthentication,
    WrongIssuer,
    WrongTrustPolicy,
    WrongTarget,
    WrongChallenge,
    WrongDecision,
    InvalidValidityWindow,
    ClaimedTimeMismatch,
    MissingReferenceTime,
    AuthenticationNotYetValid,
    AuthenticationExpired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedClockObservation {
    state_revision: u64,
    tier: ClockTier,
    unix_seconds: u64,
    monotonic: Option<MonotonicPoint>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockCommitError {
    AuthorizationNotCommitted,
    HighWaterNeedsAuthenticatedBootstrap,
    StaleAppraisal,
    StateCounterOverflow,
    MonotonicBootChanged,
}

pub fn appraise_clock(
    observation: &ClockObservation,
    policy: &ClockPolicy,
    state: &ClockState,
) -> Result<PreparedClockObservation, ClockReject> {
    // The tier comes from configured policy. There is no automatic fallback
    // to a weaker source if the required source is missing or fails.
    if observation.tier < policy.minimum_tier {
        return Err(ClockReject::InsufficientTier);
    }
    if observation.source_id != policy.expected_source_id {
        return Err(ClockReject::WrongSource);
    }

    match (observation.tier, state.high_water_unix_seconds) {
        (ClockTier::A2PersistentHighWater, None) => {
            return Err(ClockReject::HighWaterUninitialized);
        }
        (
            ClockTier::A0HostWall | ClockTier::A2PersistentHighWater | ClockTier::A3Authenticated,
            Some(high_water),
        ) if observation.unix_seconds < high_water => {
            return Err(ClockReject::WallClockRollback);
        }
        (
            ClockTier::A0HostWall | ClockTier::A2PersistentHighWater,
            Some(high_water),
        ) if observation.unix_seconds - high_water > policy.maximum_forward_step_seconds => {
            return Err(ClockReject::ForwardJump);
        }
        _ => {}
    }

    let monotonic = match observation.tier {
        ClockTier::A1BootMonotonic => {
            let boot_id = observation
                .boot_id
                .as_ref()
                .filter(|value| !value.is_empty())
                .ok_or(ClockReject::MissingBootIdentity)?
                .clone();
            let ticks = observation
                .monotonic_ticks
                .ok_or(ClockReject::MissingMonotonicCounter)?;
            if let Some(previous) = &state.last_monotonic {
                if previous.boot_id != boot_id {
                    return Err(ClockReject::RebootIncomparable);
                }
                if ticks < previous.ticks {
                    return Err(ClockReject::MonotonicRollback);
                }
            }
            Some(MonotonicPoint { boot_id, ticks })
        }
        ClockTier::A3Authenticated => {
            validate_authenticated_time(observation, policy)?;
            match (&observation.boot_id, observation.monotonic_ticks) {
                (Some(boot_id), Some(ticks)) if !boot_id.is_empty() => {
                    Some(MonotonicPoint { boot_id: boot_id.clone(), ticks })
                }
                _ => None,
            }
        }
        ClockTier::A0HostWall | ClockTier::A2PersistentHighWater => None,
    };

    Ok(PreparedClockObservation {
        state_revision: state.revision,
        tier: observation.tier,
        unix_seconds: observation.unix_seconds,
        monotonic,
    })
}

fn validate_authenticated_time(
    observation: &ClockObservation,
    policy: &ClockPolicy,
) -> Result<(), ClockReject> {
    let binding = observation
        .authenticated_time
        .as_ref()
        .ok_or(ClockReject::MissingAuthenticatedTime)?;

    if !binding.signature_verified_by_trusted_verifier {
        return Err(ClockReject::UnverifiedAuthentication);
    }
    if binding.issuer_id != policy.expected_issuer_id {
        return Err(ClockReject::WrongIssuer);
    }
    if binding.trust_policy_id != policy.expected_trust_policy_id {
        return Err(ClockReject::WrongTrustPolicy);
    }
    if binding.target_id != policy.expected_target_id {
        return Err(ClockReject::WrongTarget);
    }
    if binding.challenge_id != policy.expected_challenge_id {
        return Err(ClockReject::WrongChallenge);
    }
    if binding.decision_id != policy.expected_decision_id {
        return Err(ClockReject::WrongDecision);
    }
    if binding.expires_at_unix_seconds <= binding.issued_at_unix_seconds {
        return Err(ClockReject::InvalidValidityWindow);
    }
    if binding.issued_at_unix_seconds != observation.unix_seconds {
        return Err(ClockReject::ClaimedTimeMismatch);
    }
    let reference_now = policy
        .reference_unix_seconds
        .ok_or(ClockReject::MissingReferenceTime)?;
    if binding.issued_at_unix_seconds
        > reference_now.saturating_add(policy.allowed_future_skew_seconds)
    {
        return Err(ClockReject::AuthenticationNotYetValid);
    }
    if reference_now >= binding.expires_at_unix_seconds {
        return Err(ClockReject::AuthenticationExpired);
    }
    Ok(())
}

/// Commit the clock high-water update only after the related authorization,
/// receipt, and one-use challenge have committed successfully in the real
/// transaction. The model returns an error without mutating state otherwise.
pub fn commit_after_authorized_transaction(
    state: &mut ClockState,
    prepared: PreparedClockObservation,
    authorization_committed: bool,
) -> Result<(), ClockCommitError> {
    if !authorization_committed {
        return Err(ClockCommitError::AuthorizationNotCommitted);
    }
    if prepared.state_revision != state.revision {
        return Err(ClockCommitError::StaleAppraisal);
    }
    if state.high_water_unix_seconds.is_none() && prepared.tier != ClockTier::A3Authenticated {
        return Err(ClockCommitError::HighWaterNeedsAuthenticatedBootstrap);
    }
    if state
        .high_water_unix_seconds
        .is_some_and(|high_water| prepared.unix_seconds < high_water)
    {
        return Err(ClockCommitError::StaleAppraisal);
    }
    if let (Some(previous), Some(next)) = (&state.last_monotonic, &prepared.monotonic) {
        if previous.boot_id != next.boot_id && prepared.tier != ClockTier::A3Authenticated {
            return Err(ClockCommitError::MonotonicBootChanged);
        }
    }

    // Check counters before mutating, so even overflow leaves state unchanged.
    let next_revision = state
        .revision
        .checked_add(1)
        .ok_or(ClockCommitError::StateCounterOverflow)?;
    let next_count = state
        .committed_observations
        .checked_add(1)
        .ok_or(ClockCommitError::StateCounterOverflow)?;

    match (state.high_water_unix_seconds, prepared.tier) {
        (Some(high_water), ClockTier::A0HostWall | ClockTier::A1BootMonotonic) => {
            // Low-assurance clocks may support a local decision under an A0/A1
            // policy, but they must not move the persistent freshness floor.
            state.high_water_unix_seconds = Some(high_water);
        }
        (previous, _) => {
            // Bootstrap is guarded above (A3 only); after that, only A2/A3
            // evidence can reach this advancing branch for a useful update.
            state.high_water_unix_seconds = Some(
                previous
                    .unwrap_or(prepared.unix_seconds)
                    .max(prepared.unix_seconds),
            );
        }
    }
    if let Some(monotonic) = prepared.monotonic {
        match &state.last_monotonic {
            Some(previous) if previous.boot_id == monotonic.boot_id => {
                if monotonic.ticks > previous.ticks {
                    state.last_monotonic = Some(monotonic);
                }
            }
            _ => state.last_monotonic = Some(monotonic),
        }
    }
    state.revision = next_revision;
    state.committed_observations = next_count;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy(minimum_tier: ClockTier) -> ClockPolicy {
        ClockPolicy {
            minimum_tier,
            expected_source_id: "clock-source-1".into(),
            expected_issuer_id: "time-authority-1".into(),
            expected_trust_policy_id: "time-policy-v1".into(),
            expected_target_id: "device-17".into(),
            expected_challenge_id: "challenge-42".into(),
            expected_decision_id: "decision-9".into(),
            reference_unix_seconds: Some(1_005),
            allowed_future_skew_seconds: 5,
            maximum_forward_step_seconds: 3_600,
        }
    }

    fn seeded_state() -> ClockState {
        ClockState {
            revision: 4,
            high_water_unix_seconds: Some(1_000),
            last_monotonic: Some(MonotonicPoint {
                boot_id: "boot-a".into(),
                ticks: 200,
            }),
            committed_observations: 4,
        }
    }

    fn observation(tier: ClockTier, unix_seconds: u64) -> ClockObservation {
        ClockObservation {
            source_id: "clock-source-1".into(),
            tier,
            unix_seconds,
            boot_id: Some("boot-a".into()),
            monotonic_ticks: Some(220),
            authenticated_time: None,
        }
    }

    fn authenticated_observation() -> ClockObservation {
        ClockObservation {
            source_id: "clock-source-1".into(),
            tier: ClockTier::A3Authenticated,
            unix_seconds: 1_003,
            boot_id: Some("boot-a".into()),
            monotonic_ticks: Some(220),
            authenticated_time: Some(AuthenticatedTimeBinding {
                issuer_id: "time-authority-1".into(),
                trust_policy_id: "time-policy-v1".into(),
                target_id: "device-17".into(),
                challenge_id: "challenge-42".into(),
                decision_id: "decision-9".into(),
                issued_at_unix_seconds: 1_003,
                expires_at_unix_seconds: 1_010,
                signature_verified_by_trusted_verifier: true,
            }),
        }
    }

    #[test]
    fn rejects_tier_downgrade_without_fallback() {
        let observation = observation(ClockTier::A2PersistentHighWater, 1_001);
        assert_eq!(
            appraise_clock(
                &observation,
                &policy(ClockTier::A3Authenticated),
                &seeded_state()
            ),
            Err(ClockReject::InsufficientTier)
        );
    }

    #[test]
    fn rejects_wall_clock_rollback_below_persisted_high_water() {
        let observation = observation(ClockTier::A2PersistentHighWater, 999);
        assert_eq!(
            appraise_clock(
                &observation,
                &policy(ClockTier::A2PersistentHighWater),
                &seeded_state()
            ),
            Err(ClockReject::WallClockRollback)
        );
    }

    #[test]
    fn rejects_a2_when_high_water_is_uninitialized() {
        let observation = observation(ClockTier::A2PersistentHighWater, 1_000);
        assert_eq!(
            appraise_clock(
                &observation,
                &policy(ClockTier::A2PersistentHighWater),
                &ClockState::default()
            ),
            Err(ClockReject::HighWaterUninitialized)
        );
    }

    #[test]
    fn boot_monotonic_tier_ignores_wall_clock_rollback_when_ticks_advance() {
        let mut state = seeded_state();
        let observation = observation(ClockTier::A1BootMonotonic, 900);
        let prepared = appraise_clock(
            &observation,
            &policy(ClockTier::A1BootMonotonic),
            &state,
        ).unwrap();

        commit_after_authorized_transaction(&mut state, prepared, true).unwrap();
        assert_eq!(state.high_water_unix_seconds, Some(1_000));
        assert_eq!(state.last_monotonic, Some(MonotonicPoint {
            boot_id: "boot-a".into(),
            ticks: 220,
        }));
    }

    #[test]
    fn boot_monotonic_tier_rejects_counter_regression() {
        let mut observation = observation(ClockTier::A1BootMonotonic, 1_001);
        observation.monotonic_ticks = Some(199);
        assert_eq!(
            appraise_clock(
                &observation,
                &policy(ClockTier::A1BootMonotonic),
                &seeded_state()
            ),
            Err(ClockReject::MonotonicRollback)
        );
    }

    #[test]
    fn boot_monotonic_tier_rejects_incomparable_reboot() {
        let mut observation = observation(ClockTier::A1BootMonotonic, 1_001);
        observation.boot_id = Some("boot-b".into());
        observation.monotonic_ticks = Some(1);
        assert_eq!(
            appraise_clock(
                &observation,
                &policy(ClockTier::A1BootMonotonic),
                &seeded_state()
            ),
            Err(ClockReject::RebootIncomparable)
        );
    }

    #[test]
    fn authenticated_time_requires_exact_context_binding() {
        let state = seeded_state();
        let policy = policy(ClockTier::A3Authenticated);
        let valid = authenticated_observation();
        assert!(appraise_clock(&valid, &policy, &state).is_ok());

        let mut wrong_target = valid.clone();
        wrong_target.authenticated_time.as_mut().unwrap().target_id = "device-18".into();
        assert_eq!(appraise_clock(&wrong_target, &policy, &state), Err(ClockReject::WrongTarget));

        let mut wrong_challenge = valid.clone();
        wrong_challenge.authenticated_time.as_mut().unwrap().challenge_id = "challenge-other".into();
        assert_eq!(appraise_clock(&wrong_challenge, &policy, &state), Err(ClockReject::WrongChallenge));
    }

    #[test]
    fn authenticated_time_rejects_invalid_or_expired_windows() {
        let state = seeded_state();
        let mut wrong_signature = authenticated_observation();
        wrong_signature.authenticated_time.as_mut().unwrap().signature_verified_by_trusted_verifier = false;
        assert_eq!(
            appraise_clock(&wrong_signature, &policy(ClockTier::A3Authenticated), &state),
            Err(ClockReject::UnverifiedAuthentication)
        );

        let expired_policy = ClockPolicy {
            reference_unix_seconds: Some(1_011),
            ..policy(ClockTier::A3Authenticated)
        };
        assert_eq!(
            appraise_clock(&authenticated_observation(), &expired_policy, &state),
            Err(ClockReject::AuthenticationExpired)
        );

        // Expiry is exclusive: evidence is invalid at the exact boundary.
        let expiry_boundary_policy = ClockPolicy {
            reference_unix_seconds: Some(1_010),
            ..policy(ClockTier::A3Authenticated)
        };
        assert_eq!(
            appraise_clock(&authenticated_observation(), &expiry_boundary_policy, &state),
            Err(ClockReject::AuthenticationExpired)
        );

        let zero_length_window = ClockObservation {
            authenticated_time: Some(AuthenticatedTimeBinding {
                expires_at_unix_seconds: 1_003,
                ..authenticated_observation().authenticated_time.unwrap()
            }),
            ..authenticated_observation()
        };
        assert_eq!(
            appraise_clock(&zero_length_window, &policy(ClockTier::A3Authenticated), &state),
            Err(ClockReject::InvalidValidityWindow)
        );

        let future_policy = ClockPolicy {
            reference_unix_seconds: Some(990),
            ..policy(ClockTier::A3Authenticated)
        };
        assert_eq!(
            appraise_clock(&authenticated_observation(), &future_policy, &state),
            Err(ClockReject::AuthenticationNotYetValid)
        );
    }

    #[test]
    fn failed_authorization_does_not_advance_high_water() {
        let state_before = seeded_state();
        let mut state_after = state_before.clone();
        let prepared = appraise_clock(
            &observation(ClockTier::A2PersistentHighWater, 1_010),
            &policy(ClockTier::A2PersistentHighWater),
            &state_after,
        ).unwrap();

        assert_eq!(
            commit_after_authorized_transaction(&mut state_after, prepared, false),
            Err(ClockCommitError::AuthorizationNotCommitted)
        );
        assert_eq!(state_after, state_before);
    }

    #[test]
    fn stale_appraisal_revision_cannot_commit_twice() {
        let mut state = seeded_state();
        let observation = observation(ClockTier::A2PersistentHighWater, 1_010);
        let first = appraise_clock(&observation, &policy(ClockTier::A2PersistentHighWater), &state).unwrap();
        let stale = appraise_clock(&observation, &policy(ClockTier::A2PersistentHighWater), &state).unwrap();

        commit_after_authorized_transaction(&mut state, first, true).unwrap();
        assert_eq!(
            commit_after_authorized_transaction(&mut state, stale, true),
            Err(ClockCommitError::StaleAppraisal)
        );
        assert_eq!(state.high_water_unix_seconds, Some(1_010));
        assert_eq!(state.committed_observations, 5);
    }

    #[test]
    fn authorized_commit_advances_high_water_monotonically() {
        let mut state = seeded_state();
        let prepared = appraise_clock(
            &observation(ClockTier::A2PersistentHighWater, 1_010),
            &policy(ClockTier::A2PersistentHighWater),
            &state,
        ).unwrap();

        commit_after_authorized_transaction(&mut state, prepared, true).unwrap();
        assert_eq!(state.high_water_unix_seconds, Some(1_010));
        assert_eq!(state.revision, 5);
        assert_eq!(state.committed_observations, 5);
    }

    #[test]
    fn a0_wall_clock_does_not_advance_persistent_high_water() {
        let mut state = seeded_state();
        let prepared = appraise_clock(
            &observation(ClockTier::A0HostWall, 1_001),
            &policy(ClockTier::A0HostWall),
            &state,
        ).unwrap();

        commit_after_authorized_transaction(&mut state, prepared, true).unwrap();
        assert_eq!(state.high_water_unix_seconds, Some(1_000));
        assert_eq!(state.committed_observations, 5);
    }

    #[test]
    fn high_water_bootstrap_requires_authenticated_evidence() {
        let mut unseeded = ClockState::default();
        let state_before = unseeded.clone();
        let wall_clock = observation(ClockTier::A0HostWall, 1_003);
        let prepared_wall = appraise_clock(
            &wall_clock,
            &policy(ClockTier::A0HostWall),
            &unseeded,
        ).unwrap();

        assert_eq!(
            commit_after_authorized_transaction(&mut unseeded, prepared_wall, true),
            Err(ClockCommitError::HighWaterNeedsAuthenticatedBootstrap)
        );
        assert_eq!(unseeded, state_before);

        let authenticated = authenticated_observation();
        let prepared_authenticated = appraise_clock(
            &authenticated,
            &policy(ClockTier::A3Authenticated),
            &unseeded,
        ).unwrap();
        commit_after_authorized_transaction(&mut unseeded, prepared_authenticated, true).unwrap();
        assert_eq!(unseeded.high_water_unix_seconds, Some(1_003));
        assert_eq!(unseeded.committed_observations, 1);
    }

    #[test]
    fn ten_thousand_deterministic_binding_mutations_fail_closed() {
        let policy = policy(ClockTier::A3Authenticated);
        let state = seeded_state();
        for i in 0..10_000_u64 {
            let mut mutated = authenticated_observation();
            let binding = mutated.authenticated_time.as_mut().unwrap();
            match i % 8 {
                0 => binding.target_id = format!("wrong-target-{i}"),
                1 => binding.challenge_id = format!("wrong-challenge-{i}"),
                2 => binding.decision_id = format!("wrong-decision-{i}"),
                3 => binding.issuer_id = format!("wrong-issuer-{i}"),
                4 => binding.trust_policy_id = format!("wrong-policy-{i}"),
                5 => binding.signature_verified_by_trusted_verifier = false,
                6 => binding.issued_at_unix_seconds += 1,
                _ => binding.expires_at_unix_seconds = binding.issued_at_unix_seconds - 1,
            }
            assert!(
                appraise_clock(&mutated, &policy, &state).is_err(),
                "mutation {i} unexpectedly accepted"
            );
        }
        println!("PASS: 10000 clock-authentication binding mutations were rejected");
    }

    #[test]
    fn ten_thousand_deterministic_time_mutations_fail_closed() {
        let policy = policy(ClockTier::A2PersistentHighWater);
        let state = ClockState {
            revision: 1,
            high_water_unix_seconds: Some(100_000),
            last_monotonic: None,
            committed_observations: 1,
        };
        for i in 0..10_000_u64 {
            let delta = (i / 2) + 1;
            let mutated_time = if i % 2 == 0 {
                100_000 - delta
            } else {
                100_000 + 3_600 + delta
            };
            let mutated = observation(ClockTier::A2PersistentHighWater, mutated_time);
            assert!(
                appraise_clock(&mutated, &policy, &state).is_err(),
                "time mutation {i} at {mutated_time} unexpectedly accepted"
            );
        }
        println!("PASS: 10000 clock rollback/forward-jump mutations were rejected");
    }
}
