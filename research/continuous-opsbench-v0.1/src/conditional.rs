//! Participant-side conditional mutation reference model.
//!
//! This module closes the temporal topology evaluator's TOCTOU gap in a
//! deterministic in-memory model: topology publication and effect commit share
//! one serialization boundary. A durable adapter must implement the same
//! compare-and-commit inside the participant's transaction/CAS boundary.
//!
//! Authorization claims are checked for exact binding and time validity here,
//! but their signatures and trust roots must be verified by the caller before
//! constructing an AuthorizationReceipt. This module is not a production
//! authorization verifier or durable transaction journal.

use crate::Digest;
use std::collections::BTreeMap;
use std::sync::Mutex;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TopologyVersion {
    pub scope_id: String,
    pub epoch: u64,
    pub digest: Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionBinding {
    pub participant_id: String,
    pub scope_id: String,
    pub tenant_id: String,
    pub attempt_id: String,
    pub logical_action_key: String,
    pub operation: String,
    pub target: String,
    pub request_digest: Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConditionalMutation {
    pub binding: ActionBinding,
    pub expected_topology_epoch: u64,
    pub expected_topology_digest: Digest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthorizationDecision {
    Allow,
    Deny,
}

/// A normalized policy decision. The adapter must authenticate and validate
/// the source receipt before passing it to Participant::commit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorizationReceipt {
    pub decision_id: String,
    pub decision: AuthorizationDecision,
    pub binding: ActionBinding,
    pub not_before_ms: u64,
    pub expires_at_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MutationDisposition {
    Committed,
    IdempotentReplay,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectReceipt {
    pub effect_id: String,
    pub binding: ActionBinding,
    pub committed_topology_epoch: u64,
    pub committed_topology_digest: Digest,
    pub committed_at_ms: u64,
    pub participant_sequence: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MutationResult {
    pub disposition: MutationDisposition,
    pub receipt: EffectReceipt,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MutationError {
    InvalidBinding,
    InvalidAuthorization,
    AuthorizationDenied,
    AuthorizationMismatch,
    AuthorizationNotYetValid,
    AuthorizationExpired,
    ScopeMismatch,
    StaleTopology {
        expected_epoch: u64,
        observed_epoch: u64,
        expected_digest: Digest,
        observed_digest: Digest,
    },
    IdempotencyConflict,
    NonMonotonicTopology {
        current_epoch: u64,
        proposed_epoch: u64,
    },
    SequenceExhausted,
    LockPoisoned,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct IdempotencyKey {
    tenant_id: String,
    logical_action_key: String,
}

/// The stable identity of the intended business effect. Attempt IDs are
/// deliberately excluded: a retry may have a new attempt ID but must not
/// create a second effect for the same logical key and identical request.
#[derive(Clone, Debug, PartialEq, Eq)]
struct EffectFingerprint {
    participant_id: String,
    scope_id: String,
    tenant_id: String,
    logical_action_key: String,
    operation: String,
    target: String,
    request_digest: Digest,
    expected_topology_epoch: u64,
    expected_topology_digest: Digest,
}

impl EffectFingerprint {
    fn from_request(request: &ConditionalMutation) -> Self {
        let binding = &request.binding;
        Self {
            participant_id: binding.participant_id.clone(),
            scope_id: binding.scope_id.clone(),
            tenant_id: binding.tenant_id.clone(),
            logical_action_key: binding.logical_action_key.clone(),
            operation: binding.operation.clone(),
            target: binding.target.clone(),
            request_digest: binding.request_digest.clone(),
            expected_topology_epoch: request.expected_topology_epoch,
            expected_topology_digest: request.expected_topology_digest.clone(),
        }
    }
}

#[derive(Clone, Debug)]
struct StoredEffect {
    fingerprint: EffectFingerprint,
    receipt: EffectReceipt,
}

#[derive(Debug)]
struct ParticipantState {
    topology: TopologyVersion,
    effects: BTreeMap<IdempotencyKey, StoredEffect>,
    next_sequence: u64,
}

/// A reference participant. Every topology update and mutation commit is
/// serialized by the same mutex, making the topology check and effect record
/// one linearizable in-memory operation.
#[derive(Debug)]
pub struct Participant {
    participant_id: String,
    state: Mutex<ParticipantState>,
}

impl Participant {
    pub fn new(
        participant_id: impl Into<String>,
        topology: TopologyVersion,
    ) -> Result<Self, MutationError> {
        let participant_id = participant_id.into();
        if participant_id.is_empty() || topology.scope_id.is_empty() {
            return Err(MutationError::InvalidBinding);
        }
        Ok(Self {
            participant_id,
            state: Mutex::new(ParticipantState {
                topology,
                effects: BTreeMap::new(),
                next_sequence: 1,
            }),
        })
    }

    /// Publish a topology version through the same serialization boundary
    /// used by commit. Epochs must increase strictly within the participant.
    pub fn publish_topology(
        &self,
        next: TopologyVersion,
    ) -> Result<(), MutationError> {
        let mut state = self.state.lock().map_err(|_| MutationError::LockPoisoned)?;
        if next.scope_id != state.topology.scope_id {
            return Err(MutationError::ScopeMismatch);
        }
        if next.epoch <= state.topology.epoch {
            return Err(MutationError::NonMonotonicTopology {
                current_epoch: state.topology.epoch,
                proposed_epoch: next.epoch,
            });
        }
        state.topology = next;
        Ok(())
    }

    /// Atomically check authorization, idempotency, and the expected topology
    /// version before recording a committed effect. A stale precondition never
    /// inserts an effect. Durable adapters must perform the same operations
    /// inside one participant-owned transaction or compare-and-swap primitive.
    pub fn commit(
        &self,
        request: &ConditionalMutation,
        authorization: &AuthorizationReceipt,
        now_ms: u64,
    ) -> Result<MutationResult, MutationError> {
        validate_binding(&request.binding)?;
        if authorization.decision_id.is_empty()
            || authorization.not_before_ms >= authorization.expires_at_ms
        {
            return Err(MutationError::InvalidAuthorization);
        }
        if authorization.binding != request.binding {
            return Err(MutationError::AuthorizationMismatch);
        }
        if authorization.decision == AuthorizationDecision::Deny {
            return Err(MutationError::AuthorizationDenied);
        }
        if now_ms < authorization.not_before_ms {
            return Err(MutationError::AuthorizationNotYetValid);
        }
        if now_ms >= authorization.expires_at_ms {
            return Err(MutationError::AuthorizationExpired);
        }
        if request.binding.participant_id != self.participant_id {
            return Err(MutationError::InvalidBinding);
        }

        let mut state = self.state.lock().map_err(|_| MutationError::LockPoisoned)?;
        if request.binding.scope_id != state.topology.scope_id {
            return Err(MutationError::ScopeMismatch);
        }

        let key = IdempotencyKey {
            tenant_id: request.binding.tenant_id.clone(),
            logical_action_key: request.binding.logical_action_key.clone(),
        };
        let fingerprint = EffectFingerprint::from_request(request);

        // Replay lookup precedes the current topology check: an exact retry
        // returns the original receipt even if topology advanced after commit.
        // It cannot create another effect or rewrite the original epoch.
        if let Some(existing) = state.effects.get(&key) {
            if existing.fingerprint != fingerprint {
                return Err(MutationError::IdempotencyConflict);
            }
            return Ok(MutationResult {
                disposition: MutationDisposition::IdempotentReplay,
                receipt: existing.receipt.clone(),
            });
        }

        if state.topology.epoch != request.expected_topology_epoch
            || state.topology.digest != request.expected_topology_digest
        {
            return Err(MutationError::StaleTopology {
                expected_epoch: request.expected_topology_epoch,
                observed_epoch: state.topology.epoch,
                expected_digest: request.expected_topology_digest.clone(),
                observed_digest: state.topology.digest.clone(),
            });
        }

        let sequence = state.next_sequence;
        state.next_sequence = sequence
            .checked_add(1)
            .ok_or(MutationError::SequenceExhausted)?;
        let receipt = EffectReceipt {
            effect_id: format!("{}/effect/{sequence}", self.participant_id),
            binding: request.binding.clone(),
            committed_topology_epoch: state.topology.epoch,
            committed_topology_digest: state.topology.digest.clone(),
            committed_at_ms: now_ms,
            participant_sequence: sequence,
        };
        state.effects.insert(key, StoredEffect {
            fingerprint,
            receipt: receipt.clone(),
        });
        Ok(MutationResult {
            disposition: MutationDisposition::Committed,
            receipt,
        })
    }

    pub fn effect_count(&self) -> Result<usize, MutationError> {
        self.state
            .lock()
            .map(|state| state.effects.len())
            .map_err(|_| MutationError::LockPoisoned)
    }

    pub fn topology(&self) -> Result<TopologyVersion, MutationError> {
        self.state
            .lock()
            .map(|state| state.topology.clone())
            .map_err(|_| MutationError::LockPoisoned)
    }
}

fn validate_binding(binding: &ActionBinding) -> Result<(), MutationError> {
    if binding.participant_id.is_empty()
        || binding.scope_id.is_empty()
        || binding.tenant_id.is_empty()
        || binding.attempt_id.is_empty()
        || binding.logical_action_key.is_empty()
        || binding.operation.is_empty()
        || binding.target.is_empty()
    {
        return Err(MutationError::InvalidBinding);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};

    fn d(byte: u8) -> Digest {
        Digest::from_byte(byte)
    }

    fn participant() -> Participant {
        Participant::new(
            "participant-a",
            TopologyVersion {
                scope_id: "prod/zone-a".into(),
                epoch: 7,
                digest: d(7),
            },
        )
        .unwrap()
    }

    fn request(attempt_id: &str, request_digest: Digest) -> ConditionalMutation {
        ConditionalMutation {
            binding: ActionBinding {
                participant_id: "participant-a".into(),
                scope_id: "prod/zone-a".into(),
                tenant_id: "tenant-a".into(),
                attempt_id: attempt_id.into(),
                logical_action_key: "restart/api/incident-42".into(),
                operation: "restart_service".into(),
                target: "api".into(),
                request_digest,
            },
            expected_topology_epoch: 7,
            expected_topology_digest: d(7),
        }
    }

    fn permit(request: &ConditionalMutation) -> AuthorizationReceipt {
        AuthorizationReceipt {
            decision_id: "policy-decision-1".into(),
            decision: AuthorizationDecision::Allow,
            binding: request.binding.clone(),
            not_before_ms: 0,
            expires_at_ms: 1_000,
        }
    }

    #[test]
    fn stale_epoch_is_rejected_without_committing_an_effect() {
        let participant = participant();
        participant.publish_topology(TopologyVersion {
            scope_id: "prod/zone-a".into(),
            epoch: 8,
            digest: d(8),
        }).unwrap();

        let request = request("attempt-1", d(9));
        let result = participant.commit(&request, &permit(&request), 100);
        assert!(matches!(result, Err(MutationError::StaleTopology {
            expected_epoch: 7, observed_epoch: 8, ..
        })));
        assert_eq!(participant.effect_count().unwrap(), 0);
    }

    #[test]
    fn digest_change_at_same_expected_epoch_is_rejected() {
        let participant = participant();
        let mut request = request("attempt-1", d(9));
        request.expected_topology_digest = d(99);
        let result = participant.commit(&request, &permit(&request), 100);
        assert!(matches!(result, Err(MutationError::StaleTopology { .. })));
        assert_eq!(participant.effect_count().unwrap(), 0);
    }

    #[test]
    fn exact_retry_returns_original_receipt_without_duplicate_effect() {
        let participant = participant();
        let first = request("attempt-1", d(9));
        let committed = participant.commit(&first, &permit(&first), 100).unwrap();
        assert_eq!(committed.disposition, MutationDisposition::Committed);

        participant.publish_topology(TopologyVersion {
            scope_id: "prod/zone-a".into(),
            epoch: 8,
            digest: d(8),
        }).unwrap();

        let retry = request("attempt-2", d(9));
        let replayed = participant.commit(&retry, &permit(&retry), 200).unwrap();
        assert_eq!(replayed.disposition, MutationDisposition::IdempotentReplay);
        assert_eq!(replayed.receipt, committed.receipt);
        assert_eq!(participant.effect_count().unwrap(), 1);
    }

    #[test]
    fn reusing_idempotency_key_for_a_different_effect_is_rejected() {
        let participant = participant();
        let first = request("attempt-1", d(9));
        participant.commit(&first, &permit(&first), 100).unwrap();

        let conflicting = request("attempt-2", d(10));
        assert_eq!(
            participant.commit(&conflicting, &permit(&conflicting), 200),
            Err(MutationError::IdempotencyConflict)
        );
        assert_eq!(participant.effect_count().unwrap(), 1);
    }

    #[test]
    fn authorization_must_bind_the_exact_attempt_and_request() {
        let participant = participant();
        let request = request("attempt-1", d(9));
        let mut permit = permit(&request);
        permit.binding.target = "database".into();
        assert_eq!(
            participant.commit(&request, &permit, 100),
            Err(MutationError::AuthorizationMismatch)
        );
        assert_eq!(participant.effect_count().unwrap(), 0);
    }

    #[test]
    fn denied_or_expired_authorization_cannot_commit() {
        let participant = participant();
        let request = request("attempt-1", d(9));
        let mut denied = permit(&request);
        denied.decision = AuthorizationDecision::Deny;
        assert_eq!(
            participant.commit(&request, &denied, 100),
            Err(MutationError::AuthorizationDenied)
        );

        let expired = permit(&request);
        assert_eq!(
            participant.commit(&request, &expired, 1_000),
            Err(MutationError::AuthorizationExpired)
        );
        assert_eq!(participant.effect_count().unwrap(), 0);
    }

    #[test]
    fn topology_epoch_must_advance_monotonically() {
        let participant = participant();
        let result = participant.publish_topology(TopologyVersion {
            scope_id: "prod/zone-a".into(),
            epoch: 7,
            digest: d(7),
        });
        assert!(matches!(result, Err(MutationError::NonMonotonicTopology { .. })));
    }

    #[test]
    fn concurrent_topology_update_and_commit_have_one_linearized_order() {
        let participant = Arc::new(participant());
        let barrier = Arc::new(Barrier::new(3));
        let request = request("attempt-race", d(9));
        let authorization = permit(&request);

        let commit_participant = Arc::clone(&participant);
        let commit_barrier = Arc::clone(&barrier);
        let commit_request = request.clone();
        let commit_authorization = authorization.clone();
        let commit_thread = std::thread::spawn(move || {
            commit_barrier.wait();
            commit_participant.commit(&commit_request, &commit_authorization, 100)
        });

        let update_participant = Arc::clone(&participant);
        let update_barrier = Arc::clone(&barrier);
        let update_thread = std::thread::spawn(move || {
            update_barrier.wait();
            update_participant.publish_topology(TopologyVersion {
                scope_id: "prod/zone-a".into(),
                epoch: 8,
                digest: d(8),
            })
        });

        barrier.wait();
        let commit_result = commit_thread.join().unwrap();
        update_thread.join().unwrap().unwrap();
        let current = participant.topology().unwrap();
        assert_eq!(current.epoch, 8);

        match commit_result {
            Ok(result) => {
                assert_eq!(result.disposition, MutationDisposition::Committed);
                assert_eq!(result.receipt.committed_topology_epoch, 7);
                assert_eq!(participant.effect_count().unwrap(), 1);
            }
            Err(MutationError::StaleTopology {
                expected_epoch: 7,
                observed_epoch: 8,
                ..
            }) => {
                assert_eq!(participant.effect_count().unwrap(), 0);
            }
            other => panic!("unexpected race outcome: {other:?}"),
        }
    }
}
