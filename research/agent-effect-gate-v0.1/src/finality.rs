//! Participant-owned act-bound execution finality reference model.
//!
//! The participant reconstructs the exact Candidate Act, verifies that it
//! matches an authenticated handle, and consumes the handle/nonce while
//! recording the protected effect in one participant-owned critical section.
//!
//! This is an in-memory E0 reference model. It includes no cryptography or
//! canonical byte encoder. The private handle constructor is reserved for a
//! verifier adapter in this crate; that adapter must authenticate issuer,
//! trust root, signature, canonical encoding and source evidence. Unit tests
//! exercise structure only. Production must atomically commit the business
//! effect, idempotency record, consumed handle and nonce in one durable
//! transaction/CAS. Otherwise strict finality must reject or remain unsupported.

use crate::Digest;
use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CandidateAct {
    pub canonicalization_profile: String,
    pub tenant_id: String,
    pub principal_id: String,
    pub episode_id: String,
    pub plan_digest: Digest,
    pub step_id: String,
    pub grant_id: String,
    pub authorization_scope: String,
    pub tool_id: String,
    pub operation_id: String,
    pub manifest_version: String,
    pub manifest_digest: Digest,
    pub toolchain_id: String,
    pub toolchain_digest: Digest,
    pub policy_version: String,
    pub policy_digest: Digest,
    pub policy_epoch: u64,
    pub participant_id: String,
    pub target_digest: Digest,
    pub request_digest: Digest,
    pub attempt_id: String,
    pub logical_action_key: String,
    pub environment_profile_digest: Digest,
    pub runtime_evidence_bundle_digest: Digest,
    pub risk_level: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionHandleClaims {
    pub handle_id: String,
    pub nonce_digest: Digest,
    pub handle_digest: Digest,
    pub issuer_id: String,
    pub audience: String,
    pub act: CandidateAct,
    pub issued_at_ms: u64,
    pub not_before_ms: u64,
    pub expires_at_ms: u64,
}

/// Opaque and non-Clone/non-Serialize. Only an authenticated verifier adapter
/// inside this crate should mint it after validating the token and trust policy.
#[derive(Debug, PartialEq, Eq)]
pub struct AuthenticatedExecutionHandle {
    claims: ExecutionHandleClaims,
}

impl AuthenticatedExecutionHandle {
    #[expect(dead_code, reason = "Only a verifier adapter may mint authenticated handles; that adapter is a later integration slice.")]
    pub(crate) fn from_verified_claims(claims: ExecutionHandleClaims) -> Self {
        Self { claims }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FinalityEffectReceipt {
    pub effect_id: String,
    pub act: CandidateAct,
    pub consumed_handle_id: String,
    pub consumed_handle_digest: Digest,
    pub consumed_nonce_digest: Digest,
    pub committed_at_ms: u64,
    pub participant_sequence: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FinalityDisposition { Committed, IdempotentReplay }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FinalityResult {
    pub disposition: FinalityDisposition,
    pub consumed_handle_id: String,
    pub consumed_handle_digest: Digest,
    pub effect_receipt: FinalityEffectReceipt,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FinalityError {
    InvalidAct(&'static str),
    WrongParticipant,
    InvalidHandle(&'static str),
    HandleActMismatch,
    HandleNotYetValid,
    HandleExpired,
    StalePolicyEpoch { handle_epoch: u64, current_epoch: u64 },
    HandleReplayConflict,
    NonceReplay,
    IdempotencyConflict,
    SequenceExhausted,
    NonMonotonicPolicyEpoch { current_epoch: u64, proposed_epoch: u64 },
    LockPoisoned,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct EffectFingerprint {
    participant_id: String,
    tenant_id: String,
    principal_id: String,
    episode_id: String,
    plan_digest: Digest,
    step_id: String,
    tool_id: String,
    operation_id: String,
    manifest_version: String,
    manifest_digest: Digest,
    toolchain_id: String,
    toolchain_digest: Digest,
    policy_version: String,
    policy_digest: Digest,
    policy_epoch: u64,
    target_digest: Digest,
    request_digest: Digest,
    logical_action_key: String,
    environment_profile_digest: Digest,
    risk_level: u8,
}

impl EffectFingerprint {
    /// A new attempt, grant, or runtime-evidence receipt may authorize a retry
    /// of the same logical operation; it must not create a second effect.
    /// The exact retry act is still compared against its own handle first.
    fn from_act(act: &CandidateAct) -> Self {
        Self {
            participant_id: act.participant_id.clone(),
            tenant_id: act.tenant_id.clone(),
            principal_id: act.principal_id.clone(),
            episode_id: act.episode_id.clone(),
            plan_digest: act.plan_digest.clone(),
            step_id: act.step_id.clone(),
            tool_id: act.tool_id.clone(),
            operation_id: act.operation_id.clone(),
            manifest_version: act.manifest_version.clone(),
            manifest_digest: act.manifest_digest.clone(),
            toolchain_id: act.toolchain_id.clone(),
            toolchain_digest: act.toolchain_digest.clone(),
            policy_version: act.policy_version.clone(),
            policy_digest: act.policy_digest.clone(),
            policy_epoch: act.policy_epoch,
            target_digest: act.target_digest.clone(),
            request_digest: act.request_digest.clone(),
            logical_action_key: act.logical_action_key.clone(),
            environment_profile_digest: act.environment_profile_digest.clone(),
            risk_level: act.risk_level,
        }
    }
}

#[derive(Clone, Debug)]
struct StoredEffect {
    fingerprint: EffectFingerprint,
    receipt: FinalityEffectReceipt,
}

#[derive(Clone, Debug)]
struct ConsumedHandle {
    act: CandidateAct,
    nonce_digest: Digest,
    handle_digest: Digest,
    result: FinalityResult,
}

#[derive(Debug)]
struct ParticipantLedger {
    policy_epoch: u64,
    next_sequence: u64,
    effects: BTreeMap<(String, String), StoredEffect>,
    consumed_handles: HashMap<String, ConsumedHandle>,
    consumed_nonces: HashMap<Digest, String>,
}

#[derive(Debug)]
pub struct FinalitySink {
    participant_id: String,
    ledger: Mutex<ParticipantLedger>,
}

impl FinalitySink {
    pub fn new(participant_id: impl Into<String>, initial_policy_epoch: u64)
        -> Result<Self, FinalityError>
    {
        let participant_id = participant_id.into();
        if participant_id.is_empty() {
            return Err(FinalityError::InvalidAct("participant_id"));
        }
        Ok(Self {
            participant_id,
            ledger: Mutex::new(ParticipantLedger {
                policy_epoch: initial_policy_epoch,
                next_sequence: 1,
                effects: BTreeMap::new(),
                consumed_handles: HashMap::new(),
                consumed_nonces: HashMap::new(),
            }),
        })
    }

    pub fn advance_policy_epoch(&self, proposed_epoch: u64) -> Result<(), FinalityError> {
        let mut ledger = self.ledger.lock().map_err(|_| FinalityError::LockPoisoned)?;
        if proposed_epoch <= ledger.policy_epoch {
            return Err(FinalityError::NonMonotonicPolicyEpoch {
                current_epoch: ledger.policy_epoch,
                proposed_epoch,
            });
        }
        ledger.policy_epoch = proposed_epoch;
        Ok(())
    }

    /// Reconstruct the act from the exact operation about to be applied. Handle
    /// validation, nonce consumption and effect insertion share one lock.
    pub fn commit_exact_act(
        &self,
        act: &CandidateAct,
        handle: &AuthenticatedExecutionHandle,
        now_ms: u64,
    ) -> Result<FinalityResult, FinalityError> {
        validate_act(act)?;
        let claims = &handle.claims;

        if act.participant_id != self.participant_id || claims.audience != self.participant_id {
            return Err(FinalityError::WrongParticipant);
        }
        if claims.handle_id.is_empty() || claims.issuer_id.is_empty() {
            return Err(FinalityError::InvalidHandle("identity"));
        }
        if claims.nonce_digest == Digest::ZERO {
            return Err(FinalityError::InvalidHandle("zero nonce"));
        }
        if claims.handle_digest == Digest::ZERO {
            return Err(FinalityError::InvalidHandle("zero handle digest"));
        }
        if claims.act != *act {
            return Err(FinalityError::HandleActMismatch);
        }
        if claims.issued_at_ms > claims.not_before_ms
            || claims.not_before_ms >= claims.expires_at_ms
        {
            return Err(FinalityError::InvalidHandle("validity interval"));
        }

        let mut ledger = self.ledger.lock().map_err(|_| FinalityError::LockPoisoned)?;

        // Exact replay is a receipt lookup, not a second commit. Permit this
        // read-only reconciliation even after expiry or policy rotation.
        if let Some(consumed) = ledger.consumed_handles.get(&claims.handle_id) {
            if consumed.act != *act || consumed.nonce_digest != claims.nonce_digest
                || consumed.handle_digest != claims.handle_digest {
                return Err(FinalityError::HandleReplayConflict);
            }
            let mut replay = consumed.result.clone();
            replay.disposition = FinalityDisposition::IdempotentReplay;
            return Ok(replay);
        }

        if now_ms < claims.issued_at_ms || now_ms < claims.not_before_ms {
            return Err(FinalityError::HandleNotYetValid);
        }
        if now_ms >= claims.expires_at_ms {
            return Err(FinalityError::HandleExpired);
        }
        if act.policy_epoch != ledger.policy_epoch {
            return Err(FinalityError::StalePolicyEpoch {
                handle_epoch: act.policy_epoch,
                current_epoch: ledger.policy_epoch,
            });
        }
        if ledger.consumed_nonces.contains_key(&claims.nonce_digest) {
            return Err(FinalityError::NonceReplay);
        }

        let key = (act.tenant_id.clone(), act.logical_action_key.clone());
        let fingerprint = EffectFingerprint::from_act(act);

        if let Some(existing) = ledger.effects.get(&key) {
            if existing.fingerprint != fingerprint {
                return Err(FinalityError::IdempotencyConflict);
            }
            let result = FinalityResult {
                disposition: FinalityDisposition::IdempotentReplay,
                consumed_handle_id: claims.handle_id.clone(),
                consumed_handle_digest: claims.handle_digest.clone(),
                effect_receipt: existing.receipt.clone(),
            };
            ledger.consumed_nonces.insert(claims.nonce_digest.clone(), claims.handle_id.clone());
            ledger.consumed_handles.insert(claims.handle_id.clone(), ConsumedHandle {
                act: act.clone(),
                nonce_digest: claims.nonce_digest.clone(),
                handle_digest: claims.handle_digest.clone(),
                result: result.clone(),
            });
            return Ok(result);
        }

        let sequence = ledger.next_sequence;
        let next_sequence = sequence.checked_add(1).ok_or(FinalityError::SequenceExhausted)?;
        let receipt = FinalityEffectReceipt {
            effect_id: format!("{}/effect/{sequence}", self.participant_id),
            act: act.clone(),
            consumed_handle_id: claims.handle_id.clone(),
            consumed_handle_digest: claims.handle_digest.clone(),
            consumed_nonce_digest: claims.nonce_digest.clone(),
            committed_at_ms: now_ms,
            participant_sequence: sequence,
        };
        let result = FinalityResult {
            disposition: FinalityDisposition::Committed,
            consumed_handle_id: claims.handle_id.clone(),
            consumed_handle_digest: claims.handle_digest.clone(),
            effect_receipt: receipt.clone(),
        };

        // These writes are one in-memory critical section. A durable adapter
        // must transact effect + idempotency + handle + nonce together.
        ledger.next_sequence = next_sequence;
        ledger.effects.insert(key, StoredEffect { fingerprint, receipt });
        ledger.consumed_nonces.insert(claims.nonce_digest.clone(), claims.handle_id.clone());
        ledger.consumed_handles.insert(claims.handle_id.clone(), ConsumedHandle {
            act: act.clone(),
            nonce_digest: claims.nonce_digest.clone(),
            handle_digest: claims.handle_digest.clone(),
            result: result.clone(),
        });
        Ok(result)
    }

    pub fn effect_count(&self) -> Result<usize, FinalityError> {
        self.ledger.lock().map(|l| l.effects.len()).map_err(|_| FinalityError::LockPoisoned)
    }

    pub fn consumed_handle_count(&self) -> Result<usize, FinalityError> {
        self.ledger.lock().map(|l| l.consumed_handles.len()).map_err(|_| FinalityError::LockPoisoned)
    }

    pub fn current_policy_epoch(&self) -> Result<u64, FinalityError> {
        self.ledger.lock().map(|l| l.policy_epoch).map_err(|_| FinalityError::LockPoisoned)
    }
}

fn validate_act(act: &CandidateAct) -> Result<(), FinalityError> {
    for (name, value) in [
        ("canonicalization_profile", act.canonicalization_profile.as_str()),
        ("tenant_id", act.tenant_id.as_str()),
        ("principal_id", act.principal_id.as_str()),
        ("episode_id", act.episode_id.as_str()),
        ("step_id", act.step_id.as_str()),
        ("grant_id", act.grant_id.as_str()),
        ("authorization_scope", act.authorization_scope.as_str()),
        ("tool_id", act.tool_id.as_str()),
        ("operation_id", act.operation_id.as_str()),
        ("manifest_version", act.manifest_version.as_str()),
        ("toolchain_id", act.toolchain_id.as_str()),
        ("policy_version", act.policy_version.as_str()),
        ("participant_id", act.participant_id.as_str()),
        ("attempt_id", act.attempt_id.as_str()),
        ("logical_action_key", act.logical_action_key.as_str()),
    ] {
        if value.is_empty() {
            return Err(FinalityError::InvalidAct(name));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};

    fn d(byte: u8) -> Digest { Digest::from_byte(byte) }

    fn act() -> CandidateAct {
        CandidateAct {
            canonicalization_profile: "luminous-canonical-act-v1".into(),
            tenant_id: "tenant-a".into(),
            principal_id: "principal-a".into(),
            episode_id: "episode-42".into(),
            plan_digest: d(1),
            step_id: "step-restart-api".into(),
            grant_id: "grant-7".into(),
            authorization_scope: "service:api:restart".into(),
            tool_id: "nixward.service".into(),
            operation_id: "restart".into(),
            manifest_version: "1.2.0".into(),
            manifest_digest: d(2),
            toolchain_id: "nix-toolchain-1".into(),
            toolchain_digest: d(3),
            policy_version: "policy-v7".into(),
            policy_digest: d(4),
            policy_epoch: 7,
            participant_id: "participant-a".into(),
            target_digest: d(5),
            request_digest: d(6),
            attempt_id: "attempt-1".into(),
            logical_action_key: "episode-42/step-restart-api".into(),
            environment_profile_digest: d(7),
            runtime_evidence_bundle_digest: d(8),
            risk_level: 2,
        }
    }

    fn handle(act: CandidateAct, id: &str, nonce: u8) -> AuthenticatedExecutionHandle {
        AuthenticatedExecutionHandle::from_verified_claims(ExecutionHandleClaims {
            handle_id: id.into(),
            nonce_digest: d(nonce),
            handle_digest: d(nonce.wrapping_add(100)),
            issuer_id: "finality-authority-a".into(),
            audience: "participant-a".into(),
            act,
            issued_at_ms: 10,
            not_before_ms: 10,
            expires_at_ms: 500,
        })
    }

    fn sink() -> FinalitySink { FinalitySink::new("participant-a", 7).unwrap() }

    #[test]
    fn exact_act_commits_with_handle_and_nonce_receipt() {
        let sink = sink();
        let act = act();
        let result = sink.commit_exact_act(&act, &handle(act.clone(), "h1", 9), 100).unwrap();
        assert_eq!(result.disposition, FinalityDisposition::Committed);
        assert_eq!(result.effect_receipt.act.request_digest, act.request_digest);
        assert_eq!(result.effect_receipt.consumed_handle_id, "h1");
        assert_eq!(result.effect_receipt.act.policy_epoch, 7);
        assert_eq!(sink.effect_count().unwrap(), 1);
        assert_eq!(sink.consumed_handle_count().unwrap(), 1);
    }

    #[test]
    fn same_handle_replay_returns_original_receipt_without_second_effect() {
        let sink = sink();
        let act = act();
        let handle = handle(act.clone(), "h1", 9);
        let first = sink.commit_exact_act(&act, &handle, 100).unwrap();
        let second = sink.commit_exact_act(&act, &handle, 200).unwrap();
        assert_eq!(second.disposition, FinalityDisposition::IdempotentReplay);
        assert_eq!(second.effect_receipt, first.effect_receipt);
        assert_eq!(sink.effect_count().unwrap(), 1);
    }

    #[test]
    fn target_or_request_substitution_is_rejected() {
        for target_change in [true, false] {
            let sink = sink();
            let original = act();
            let mut changed = original.clone();
            if target_change { changed.target_digest = d(55); } else { changed.request_digest = d(66); }
            assert_eq!(
                sink.commit_exact_act(&changed, &handle(original, "h1", 9), 100),
                Err(FinalityError::HandleActMismatch)
            );
            assert_eq!(sink.effect_count().unwrap(), 0);
        }
    }

    #[test]
    fn tenant_principal_plan_and_operation_substitution_is_rejected() {
        let mutators: Vec<Box<dyn Fn(&mut CandidateAct)>> = vec![
            Box::new(|a| a.tenant_id = "tenant-b".into()),
            Box::new(|a| a.principal_id = "principal-b".into()),
            Box::new(|a| a.plan_digest = d(90)),
            Box::new(|a| a.operation_id = "delete".into()),
        ];
        for (i, mutate) in mutators.into_iter().enumerate() {
            let sink = sink();
            let original = act();
            let mut changed = original.clone();
            mutate(&mut changed);
            assert_eq!(
                sink.commit_exact_act(&changed, &handle(original, &format!("h{i}"), 20 + i as u8), 100),
                Err(FinalityError::HandleActMismatch)
            );
            assert_eq!(sink.effect_count().unwrap(), 0);
        }
    }

    #[test]
    fn wrong_participant_audience_is_rejected() {
        let sink = sink();
        let act = act();
        let claims = ExecutionHandleClaims {
            handle_id: "h1".into(), nonce_digest: d(9), handle_digest: d(109),
            issuer_id: "finality-authority-a".into(), audience: "participant-b".into(),
            act: act.clone(), issued_at_ms: 10, not_before_ms: 10, expires_at_ms: 500,
        };
        let handle = AuthenticatedExecutionHandle::from_verified_claims(claims);
        assert_eq!(sink.commit_exact_act(&act, &handle, 100), Err(FinalityError::WrongParticipant));
        assert_eq!(sink.effect_count().unwrap(), 0);
    }

    #[test]
    fn expired_and_not_yet_valid_handles_are_rejected() {
        let sink = sink();
        let act = act();
        assert_eq!(
            sink.commit_exact_act(&act, &handle(act.clone(), "expired", 9), 500),
            Err(FinalityError::HandleExpired)
        );
        assert_eq!(
            sink.commit_exact_act(&act, &handle(act.clone(), "early", 10), 9),
            Err(FinalityError::HandleNotYetValid)
        );
        assert_eq!(sink.effect_count().unwrap(), 0);
    }

    #[test]
    fn policy_rotation_invalidates_prior_epoch_handles() {
        let sink = sink();
        let act = act();
        let handle = handle(act.clone(), "h1", 9);
        sink.advance_policy_epoch(8).unwrap();
        assert_eq!(
            sink.commit_exact_act(&act, &handle, 100),
            Err(FinalityError::StalePolicyEpoch { handle_epoch: 7, current_epoch: 8 })
        );
        assert_eq!(sink.effect_count().unwrap(), 0);
    }

    #[test]
    fn policy_epoch_is_monotonic() {
        let sink = sink();
        assert!(matches!(sink.advance_policy_epoch(7), Err(FinalityError::NonMonotonicPolicyEpoch { .. })));
        assert!(matches!(sink.advance_policy_epoch(6), Err(FinalityError::NonMonotonicPolicyEpoch { .. })));
    }

    #[test]
    fn nonce_cannot_be_consumed_by_two_handles() {
        let sink = sink();
        let original = act();
        sink.commit_exact_act(&original, &handle(original.clone(), "h1", 9), 100).unwrap();
        let mut retry = act();
        retry.attempt_id = "attempt-2".into();
        assert_eq!(
            sink.commit_exact_act(&retry, &handle(retry.clone(), "h2", 9), 200),
            Err(FinalityError::NonceReplay)
        );
        assert_eq!(sink.effect_count().unwrap(), 1);
    }

    #[test]
    fn handle_id_cannot_be_reused_for_a_different_act() {
        let sink = sink();
        let original = act();
        sink.commit_exact_act(&original, &handle(original.clone(), "h1", 9), 100).unwrap();
        let mut changed = act();
        changed.request_digest = d(99);
        assert_eq!(
            sink.commit_exact_act(&changed, &handle(changed.clone(), "h1", 10), 200),
            Err(FinalityError::HandleReplayConflict)
        );
        assert_eq!(sink.effect_count().unwrap(), 1);
    }

    #[test]
    fn newly_authorized_retry_reuses_original_logical_effect() {
        let sink = sink();
        let first = act();
        let committed = sink.commit_exact_act(&first, &handle(first.clone(), "h1", 9), 100).unwrap();
        let mut retry = act();
        retry.attempt_id = "attempt-2".into();
        retry.grant_id = "grant-renewed".into();
        retry.runtime_evidence_bundle_digest = d(88);
        let replayed = sink.commit_exact_act(&retry, &handle(retry.clone(), "h2", 10), 200).unwrap();
        assert_eq!(replayed.disposition, FinalityDisposition::IdempotentReplay);
        assert_eq!(replayed.effect_receipt, committed.effect_receipt);
        assert_eq!(replayed.consumed_handle_id, "h2");
        assert_eq!(sink.effect_count().unwrap(), 1);
        assert_eq!(sink.consumed_handle_count().unwrap(), 2);
    }

    #[test]
    fn idempotency_key_cannot_be_reused_for_changed_target_or_body() {
        for target_change in [true, false] {
            let sink = sink();
            let first = act();
            sink.commit_exact_act(&first, &handle(first.clone(), "h1", 9), 100).unwrap();
            let mut changed = act();
            if target_change { changed.target_digest = d(55); } else { changed.request_digest = d(66); }
            assert_eq!(
                sink.commit_exact_act(&changed, &handle(changed.clone(), "h2", 10), 200),
                Err(FinalityError::IdempotencyConflict)
            );
            assert_eq!(sink.effect_count().unwrap(), 1);
        }
    }

    #[test]
    fn act_policy_epoch_must_equal_current_epoch() {
        let sink = sink();
        let mut act = act();
        act.policy_epoch = 8;
        assert_eq!(
            sink.commit_exact_act(&act, &handle(act.clone(), "h1", 9), 100),
            Err(FinalityError::StalePolicyEpoch { handle_epoch: 8, current_epoch: 7 })
        );
        assert_eq!(sink.effect_count().unwrap(), 0);
    }

    #[test]
    fn malformed_act_or_handle_interval_is_rejected() {
        let sink = sink();
        let mut bad = act();
        bad.step_id.clear();
        assert_eq!(
            sink.commit_exact_act(&bad, &handle(bad.clone(), "h1", 9), 100),
            Err(FinalityError::InvalidAct("step_id"))
        );
        let act = act();
        let handle = AuthenticatedExecutionHandle::from_verified_claims(ExecutionHandleClaims {
            handle_id: "bad-interval".into(), nonce_digest: d(12), handle_digest: d(112),
            issuer_id: "finality-authority-a".into(), audience: "participant-a".into(),
            act: act.clone(), issued_at_ms: 20, not_before_ms: 10, expires_at_ms: 10,
        });
        assert_eq!(
            sink.commit_exact_act(&act, &handle, 100),
            Err(FinalityError::InvalidHandle("validity interval"))
        );
        assert_eq!(sink.effect_count().unwrap(), 0);
    }

    #[test]
    fn concurrent_consumption_of_one_handle_creates_one_effect() {
        let sink = Arc::new(sink());
        let act = act();
        let handle = Arc::new(handle(act.clone(), "race", 44));
        let barrier = Arc::new(Barrier::new(3));
        let mut joins = Vec::new();
        for _ in 0..2 {
            let sink = Arc::clone(&sink);
            let act = act.clone();
            let handle = Arc::clone(&handle);
            let barrier = Arc::clone(&barrier);
            joins.push(std::thread::spawn(move || {
                barrier.wait();
                sink.commit_exact_act(&act, &handle, 100)
            }));
        }
        barrier.wait();
        let a = joins.remove(0).join().unwrap().unwrap();
        let b = joins.remove(0).join().unwrap().unwrap();
        assert!(
            (a.disposition == FinalityDisposition::Committed && b.disposition == FinalityDisposition::IdempotentReplay)
            || (b.disposition == FinalityDisposition::Committed && a.disposition == FinalityDisposition::IdempotentReplay)
        );
        assert_eq!(sink.effect_count().unwrap(), 1);
        assert_eq!(sink.consumed_handle_count().unwrap(), 1);
    }

    #[test]
    fn two_fresh_handles_for_same_logical_action_commit_only_once() {
        let sink = Arc::new(sink());
        let mut act_a = act();
        let mut act_b = act();
        act_a.attempt_id = "attempt-a".into();
        act_b.attempt_id = "attempt-b".into();
        let handle_a = Arc::new(handle(act_a.clone(), "h-a", 50));
        let handle_b = Arc::new(handle(act_b.clone(), "h-b", 51));
        let barrier = Arc::new(Barrier::new(3));

        let sink_a = Arc::clone(&sink);
        let barrier_a = Arc::clone(&barrier);
        let a = std::thread::spawn(move || {
            barrier_a.wait();
            sink_a.commit_exact_act(&act_a, &handle_a, 100)
        });
        let sink_b = Arc::clone(&sink);
        let barrier_b = Arc::clone(&barrier);
        let b = std::thread::spawn(move || {
            barrier_b.wait();
            sink_b.commit_exact_act(&act_b, &handle_b, 100)
        });
        barrier.wait();
        let one = a.join().unwrap().unwrap();
        let two = b.join().unwrap().unwrap();
        assert!(
            (one.disposition == FinalityDisposition::Committed && two.disposition == FinalityDisposition::IdempotentReplay)
            || (two.disposition == FinalityDisposition::Committed && one.disposition == FinalityDisposition::IdempotentReplay)
        );
        assert_eq!(sink.effect_count().unwrap(), 1);
        assert_eq!(sink.consumed_handle_count().unwrap(), 2);
    }
}
