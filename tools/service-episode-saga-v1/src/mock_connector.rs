//! Deterministic, test-only connector admission/effect ledger.
//!
//! This fixture demonstrates one possible connector contract. It is not a
//! production ITSM adapter and does not imply that commercial providers expose
//! an authoritative, linearizable attempt ledger. The snapshot/restart path is
//! an in-memory model of restoring server-owned durable state, not a disk
//! durability or power-loss test.

use super::{
    Consistency, Digest, NoEffectBasis, ObservedEffect, ObserverEvidence,
    SagaEpisode, ServicePlan, OperationSpec, AuthorityGrant, StepStatus,
};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
struct MockRequest {
    pub attempt_id: String,
    pub idempotency_key: String,
    pub target_id: String,
    pub operation_id: String,
    pub request_digest: Digest,
    pub binding_digest: Digest,
    pub expected_postcondition_digest: Digest,
}

impl MockRequest {
    fn is_valid(&self) -> bool {
        !self.attempt_id.trim().is_empty()
            && !self.idempotency_key.trim().is_empty()
            && !self.target_id.trim().is_empty()
            && !self.operation_id.trim().is_empty()
            && !self.request_digest.is_zero()
            && !self.binding_digest.is_zero()
            && !self.expected_postcondition_digest.is_zero()
    }

    fn same_logical_request(&self, other: &Self) -> bool {
        self.idempotency_key == other.idempotency_key
            && self.target_id == other.target_id
            && self.operation_id == other.operation_id
            && self.request_digest == other.request_digest
            && self.binding_digest == other.binding_digest
            && self.expected_postcondition_digest == other.expected_postcondition_digest
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FaultMode {
    Normal,
    BeforeAdmission,
    AfterAdmissionBeforeEffect,
    AfterEffectBeforeResponse,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct MockReceipt {
    pub attempt_id: String,
    pub idempotency_key: String,
    pub request_digest: Digest,
    pub target_id: String,
    pub operation_id: String,
    pub effect_sequence: u64,
    pub effect_digest: Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum AttemptState {
    AcceptedPending,
    EffectCommitted(MockReceipt),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AttemptRecord {
    request: MockRequest,
    state: AttemptState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum TransportOutcome {
    Acknowledged(MockReceipt),
    TimedOut,
    AcceptedPending,
    PreviouslyAccepted(MockReceipt),
    PreviouslyAcceptedPending,
    IdempotencyConflict,
    AttemptBindingConflict,
    InvalidRequest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LedgerConsistency {
    Strong,
    Eventual,
    Unavailable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum AttemptLedgerObservation {
    NeverAccepted,
    AcceptedPending,
    EffectCommitted(MockReceipt),
    IdempotencyKeyOwnedByOtherAttempt { original_attempt_id: String },
    BindingConflict,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ConnectorSnapshot {
    attempts: BTreeMap<String, AttemptRecord>,
    idempotency_index: BTreeMap<String, String>,
    effect_count: u64,
    next_effect_sequence: u64,
}

/// Test-only connector. Its admission ledger is authoritative by construction:
/// an operation cannot be accepted or create an effect before its attempt row
/// and idempotency-key reservation exist.
#[derive(Clone, Debug, Default)]
struct MockConnector {
    attempts: BTreeMap<String, AttemptRecord>,
    idempotency_index: BTreeMap<String, String>,
    effect_count: u64,
    next_effect_sequence: u64,
}

impl MockConnector {
    /// Simulate a request and independently choose where the reply is lost.
    fn submit(&mut self, request: MockRequest, fault: FaultMode) -> TransportOutcome {
        if !request.is_valid() {
            return TransportOutcome::InvalidRequest;
        }

        if let Some(existing) = self.attempts.get(&request.attempt_id) {
            if existing.request != request {
                return TransportOutcome::AttemptBindingConflict;
            }
            return match &existing.state {
                AttemptState::AcceptedPending => TransportOutcome::PreviouslyAcceptedPending,
                AttemptState::EffectCommitted(receipt) => {
                    TransportOutcome::PreviouslyAccepted(receipt.clone())
                }
            };
        }

        if let Some(original_attempt_id) =
            self.idempotency_index.get(&request.idempotency_key)
        {
            let Some(existing) = self.attempts.get(original_attempt_id) else {
                return TransportOutcome::IdempotencyConflict;
            };
            if !existing.request.same_logical_request(&request) {
                return TransportOutcome::IdempotencyConflict;
            }
            return match &existing.state {
                AttemptState::AcceptedPending => TransportOutcome::PreviouslyAcceptedPending,
                AttemptState::EffectCommitted(receipt) => {
                    TransportOutcome::PreviouslyAccepted(receipt.clone())
                }
            };
        }

        if fault == FaultMode::BeforeAdmission {
            // The request never crossed the authoritative admission boundary.
            return TransportOutcome::TimedOut;
        }

        self.idempotency_index.insert(
            request.idempotency_key.clone(),
            request.attempt_id.clone(),
        );
        self.attempts.insert(
            request.attempt_id.clone(),
            AttemptRecord {
                request: request.clone(),
                state: AttemptState::AcceptedPending,
            },
        );

        if fault == FaultMode::AfterAdmissionBeforeEffect {
            return TransportOutcome::TimedOut;
        }

        let Some(receipt) = self.complete_pending(&request.attempt_id) else {
            return TransportOutcome::AcceptedPending;
        };
        if fault == FaultMode::AfterEffectBeforeResponse {
            return TransportOutcome::TimedOut;
        }
        TransportOutcome::Acknowledged(receipt)
    }

    /// Complete an admitted operation. Re-delivery or duplicate worker signals
    /// return the same receipt and never create another business effect.
    fn complete_pending(&mut self, attempt_id: &str) -> Option<MockReceipt> {
        let record = self.attempts.get(attempt_id)?.clone();
        if let AttemptState::EffectCommitted(receipt) = record.state {
            return Some(receipt);
        }

        let next_sequence = self.next_effect_sequence.checked_add(1)?;
        let next_count = self.effect_count.checked_add(1)?;
        self.next_effect_sequence = next_sequence;
        self.effect_count = next_count;

        // Synthetic deterministic digest only. It is not cryptographic proof.
        let mut digest_bytes = [0xA5; 32];
        digest_bytes[..8].copy_from_slice(&next_sequence.to_be_bytes());
        let receipt = MockReceipt {
            attempt_id: record.request.attempt_id.clone(),
            idempotency_key: record.request.idempotency_key.clone(),
            request_digest: record.request.request_digest,
            target_id: record.request.target_id.clone(),
            operation_id: record.request.operation_id.clone(),
            effect_sequence: next_sequence,
            effect_digest: Digest(digest_bytes),
        };

        self.attempts
            .get_mut(attempt_id)?
            .state = AttemptState::EffectCommitted(receipt.clone());
        Some(receipt)
    }

    /// Query the connector-owned attempt ledger, not the client response and not
    /// the current business state. Only a strong query can return NeverAccepted.
    fn query_attempt(
        &self,
        request: &MockRequest,
        consistency: LedgerConsistency,
    ) -> AttemptLedgerObservation {
        if consistency == LedgerConsistency::Unavailable {
            return AttemptLedgerObservation::Unknown;
        }

        if let Some(record) = self.attempts.get(&request.attempt_id) {
            if record.request != *request {
                return AttemptLedgerObservation::BindingConflict;
            }
            return match &record.state {
                AttemptState::AcceptedPending => AttemptLedgerObservation::AcceptedPending,
                AttemptState::EffectCommitted(receipt) => {
                    AttemptLedgerObservation::EffectCommitted(receipt.clone())
                }
            };
        }

        if let Some(original_attempt_id) =
            self.idempotency_index.get(&request.idempotency_key)
        {
            let Some(existing) = self.attempts.get(original_attempt_id) else {
                return AttemptLedgerObservation::Unknown;
            };
            if existing.request.same_logical_request(request) {
                return AttemptLedgerObservation::IdempotencyKeyOwnedByOtherAttempt {
                    original_attempt_id: original_attempt_id.clone(),
                };
            }
            return AttemptLedgerObservation::BindingConflict;
        }

        match consistency {
            LedgerConsistency::Strong => AttemptLedgerObservation::NeverAccepted,
            LedgerConsistency::Eventual | LedgerConsistency::Unavailable => {
                AttemptLedgerObservation::Unknown
            }
        }
    }

    /// Map connector-owned ledger evidence to the Saga's existing typed
    /// observation. This fixture uses a distinct observer/trust-domain label.
    fn observer_evidence(
        &self,
        request: &MockRequest,
        consistency: LedgerConsistency,
        observed_at_ms: u64,
        valid_until_ms: u64,
    ) -> ObserverEvidence {
        let ledger = self.query_attempt(request, consistency);
        let (effect, evidence_consistency, postcondition_digest, no_effect_basis) =
            match (consistency, ledger) {
                (LedgerConsistency::Strong, AttemptLedgerObservation::NeverAccepted) => (
                    ObservedEffect::NoEffect,
                    Consistency::Strong,
                    None,
                    Some(NoEffectBasis::AuthoritativeAttemptLedgerNeverAccepted),
                ),
                (LedgerConsistency::Strong, AttemptLedgerObservation::EffectCommitted(_)) => (
                    ObservedEffect::ExpectedPostcondition,
                    Consistency::Strong,
                    Some(request.expected_postcondition_digest),
                    None,
                ),
                (LedgerConsistency::Strong, AttemptLedgerObservation::AcceptedPending) => (
                    ObservedEffect::Unknown,
                    Consistency::Strong,
                    None,
                    None,
                ),
                (_, AttemptLedgerObservation::EffectCommitted(_)) => (
                    ObservedEffect::ExpectedPostcondition,
                    Consistency::Eventual,
                    Some(request.expected_postcondition_digest),
                    None,
                ),
                (_, AttemptLedgerObservation::AcceptedPending) => (
                    ObservedEffect::Unknown,
                    Consistency::Eventual,
                    None,
                    None,
                ),
                (_, AttemptLedgerObservation::IdempotencyKeyOwnedByOtherAttempt { .. })
                | (_, AttemptLedgerObservation::BindingConflict) => (
                    ObservedEffect::Conflicting,
                    Consistency::Unknown,
                    None,
                    None,
                ),
                (_, AttemptLedgerObservation::Unknown) => (
                    ObservedEffect::Unknown,
                    Consistency::Unknown,
                    None,
                    None,
                ),
                (LedgerConsistency::Eventual, AttemptLedgerObservation::NeverAccepted)
                | (LedgerConsistency::Unavailable, AttemptLedgerObservation::NeverAccepted) => (
                    ObservedEffect::Unknown,
                    Consistency::Unknown,
                    None,
                    None,
                ),
            };

        ObserverEvidence {
            observer_id: "mock-itsm-attempt-ledger".into(),
            trust_domain_id: "mock-connector-ledger".into(),
            attempt_id: request.attempt_id.clone(),
            target_id: request.target_id.clone(),
            binding_digest: request.binding_digest,
            effect,
            consistency: evidence_consistency,
            postcondition_digest,
            no_effect_basis,
            observed_at_ms,
            valid_until_ms,
            evidence_digest: Digest([0xE1; 32]),
        }
    }

    /// Independent server-side effect oracle. It is not derived from the
    /// transport result or the agent's interpretation of the response.
    #[must_use]
    fn business_effect_count(&self) -> u64 {
        self.effect_count
    }

    /// Capture server-owned state. This models a durable backend snapshot for
    /// restart tests; it does not exercise filesystem sync or power-loss safety.
    #[must_use]
    fn snapshot(&self) -> ConnectorSnapshot {
        ConnectorSnapshot {
            attempts: self.attempts.clone(),
            idempotency_index: self.idempotency_index.clone(),
            effect_count: self.effect_count,
            next_effect_sequence: self.next_effect_sequence,
        }
    }

    /// Reconstruct the fixture after a simulated connector process restart.
    #[must_use]
    fn restart(snapshot: ConnectorSnapshot) -> Self {
        Self {
            attempts: snapshot.attempts,
            idempotency_index: snapshot.idempotency_index,
            effect_count: snapshot.effect_count,
            next_effect_sequence: snapshot.next_effect_sequence,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(value: u8) -> Digest {
        Digest([value; 32])
    }

    fn request(attempt_id: &str, key: &str, request_digest: u8) -> MockRequest {
        MockRequest {
            attempt_id: attempt_id.into(),
            idempotency_key: key.into(),
            target_id: "itsm-case-42".into(),
            operation_id: "itsm.update_case".into(),
            request_digest: digest(request_digest),
            binding_digest: digest(11),
            expected_postcondition_digest: digest(21),
        }
    }

    #[test]
    fn strong_attempt_ledger_can_prove_never_accepted_but_state_absence_cannot() {
        let mut connector = MockConnector::default();
        let req = request("attempt-1", "idem-1", 1);
        assert_eq!(
            connector.submit(req.clone(), FaultMode::BeforeAdmission),
            TransportOutcome::TimedOut
        );

        // A separate current-state probe sees no effect, but that alone is not
        // a proof that the request never crossed the remote admission boundary.
        assert_eq!(connector.business_effect_count(), 0);
        let eventual = connector.query_attempt(&req, LedgerConsistency::Eventual);
        assert_eq!(eventual, AttemptLedgerObservation::Unknown);
        let weak_evidence =
            connector.observer_evidence(&req, LedgerConsistency::Eventual, 50, 200);
        assert_eq!(weak_evidence.effect, ObservedEffect::Unknown);
        assert_eq!(weak_evidence.no_effect_basis, None);

        let strong = connector.query_attempt(&req, LedgerConsistency::Strong);
        assert_eq!(strong, AttemptLedgerObservation::NeverAccepted);
        let evidence =
            connector.observer_evidence(&req, LedgerConsistency::Strong, 50, 200);
        assert_eq!(evidence.effect, ObservedEffect::NoEffect);
        assert_eq!(
            evidence.no_effect_basis,
            Some(NoEffectBasis::AuthoritativeAttemptLedgerNeverAccepted)
        );
        assert_ne!(evidence.trust_domain_id, "agent-runtime");
    }

    #[test]
    fn accepted_pending_after_lost_response_is_not_a_no_effect_or_retry_proof() {
        let mut connector = MockConnector::default();
        let req = request("attempt-1", "idem-1", 1);
        assert_eq!(
            connector.submit(req.clone(), FaultMode::AfterAdmissionBeforeEffect),
            TransportOutcome::TimedOut
        );
        assert_eq!(
            connector.query_attempt(&req, LedgerConsistency::Strong),
            AttemptLedgerObservation::AcceptedPending
        );
        let evidence =
            connector.observer_evidence(&req, LedgerConsistency::Strong, 50, 200);
        assert_eq!(evidence.effect, ObservedEffect::Unknown);
        assert_eq!(evidence.no_effect_basis, None);
        assert_eq!(connector.business_effect_count(), 0);

        // A duplicate delivery cannot launch another business effect.
        assert_eq!(
            connector.submit(req.clone(), FaultMode::Normal),
            TransportOutcome::PreviouslyAcceptedPending
        );
        assert_eq!(connector.business_effect_count(), 0);

        let first = connector.complete_pending("attempt-1").expect("worker completes");
        let duplicate = connector.complete_pending("attempt-1").expect("duplicate signal");
        assert_eq!(first, duplicate);
        assert_eq!(connector.business_effect_count(), 1);
    }

    #[test]
    fn accepted_effect_with_lost_response_is_reconciled_without_replay() {
        let mut connector = MockConnector::default();
        let req = request("attempt-1", "idem-1", 1);
        assert_eq!(
            connector.submit(req.clone(), FaultMode::AfterEffectBeforeResponse),
            TransportOutcome::TimedOut
        );
        assert_eq!(connector.business_effect_count(), 1);
        let evidence =
            connector.observer_evidence(&req, LedgerConsistency::Strong, 50, 200);
        assert_eq!(evidence.effect, ObservedEffect::ExpectedPostcondition);
        assert_eq!(
            evidence.postcondition_digest,
            Some(req.expected_postcondition_digest)
        );

        assert_eq!(
            connector.submit(req.clone(), FaultMode::Normal),
            TransportOutcome::PreviouslyAccepted(
                match connector.query_attempt(&req, LedgerConsistency::Strong) {
                    AttemptLedgerObservation::EffectCommitted(receipt) => receipt,
                    other => panic!("expected committed receipt, got {other:?}"),
                }
            )
        );
        assert_eq!(connector.business_effect_count(), 1);
    }

    #[test]
    fn same_idempotency_key_and_request_reconcile_one_logical_effect() {
        let mut connector = MockConnector::default();
        let first = request("attempt-1", "idem-1", 1);
        let second_attempt = request("attempt-2", "idem-1", 1);

        assert!(matches!(
            connector.submit(first, FaultMode::AfterEffectBeforeResponse),
            TransportOutcome::TimedOut
        ));
        assert_eq!(
            connector.submit(second_attempt.clone(), FaultMode::Normal),
            match connector.query_attempt(
                &second_attempt,
                LedgerConsistency::Strong
            ) {
                AttemptLedgerObservation::IdempotencyKeyOwnedByOtherAttempt {
                    original_attempt_id,
                } => {
                    assert_eq!(original_attempt_id, "attempt-1");
                    TransportOutcome::PreviouslyAccepted(
                        match connector.query_attempt(
                            &request("attempt-1", "idem-1", 1),
                            LedgerConsistency::Strong
                        ) {
                            AttemptLedgerObservation::EffectCommitted(receipt) => receipt,
                            other => panic!("expected committed original receipt, got {other:?}"),
                        },
                    )
                }
                other => panic!("expected original attempt binding, got {other:?}"),
            }
        );
        assert_eq!(connector.business_effect_count(), 1);
    }

    #[test]
    fn same_idempotency_key_with_changed_request_is_rejected() {
        let mut connector = MockConnector::default();
        let first = request("attempt-1", "idem-1", 1);
        let altered = request("attempt-2", "idem-1", 2);

        assert!(matches!(
            connector.submit(first, FaultMode::Normal),
            TransportOutcome::Acknowledged(_)
        ));
        assert_eq!(
            connector.submit(altered.clone(), FaultMode::Normal),
            TransportOutcome::IdempotencyConflict
        );
        assert_eq!(
            connector.query_attempt(&altered, LedgerConsistency::Strong),
            AttemptLedgerObservation::BindingConflict
        );
        assert_eq!(connector.business_effect_count(), 1);
    }

    #[test]
    fn reused_attempt_id_with_changed_binding_is_rejected() {
        let mut connector = MockConnector::default();
        let first = request("attempt-1", "idem-1", 1);
        let altered = request("attempt-1", "idem-1", 2);
        connector.submit(first, FaultMode::Normal);
        assert_eq!(
            connector.submit(altered, FaultMode::Normal),
            TransportOutcome::AttemptBindingConflict
        );
        assert_eq!(connector.business_effect_count(), 1);
    }

    #[test]
    fn unavailable_or_eventually_consistent_ledger_never_proves_never_accepted() {
        let connector = MockConnector::default();
        let req = request("attempt-1", "idem-1", 1);
        for consistency in [LedgerConsistency::Eventual, LedgerConsistency::Unavailable] {
            assert_eq!(
                connector.query_attempt(&req, consistency),
                AttemptLedgerObservation::Unknown
            );
            let evidence = connector.observer_evidence(&req, consistency, 50, 200);
            assert_eq!(evidence.effect, ObservedEffect::Unknown);
            assert_eq!(evidence.no_effect_basis, None);
            assert_eq!(evidence.consistency, Consistency::Unknown);
        }
    }

    #[test]
    fn connector_restart_preserves_admission_and_effect_records() {
        let mut connector = MockConnector::default();
        let req = request("attempt-1", "idem-1", 1);
        assert_eq!(
            connector.submit(req.clone(), FaultMode::AfterEffectBeforeResponse),
            TransportOutcome::TimedOut
        );
        let snapshot = connector.snapshot();
        let mut restarted = MockConnector::restart(snapshot);

        assert_eq!(restarted.business_effect_count(), 1);
        assert!(matches!(
            restarted.query_attempt(&req, LedgerConsistency::Strong),
            AttemptLedgerObservation::EffectCommitted(_)
        ));
        assert!(matches!(
            restarted.submit(req, FaultMode::Normal),
            TransportOutcome::PreviouslyAccepted(_)
        ));
        assert_eq!(restarted.business_effect_count(), 1);
    }

    #[test]
    fn strong_never_accepted_evidence_allows_saga_reconciliation_not_unchecked_replay() {
        let plan_digest = digest(90);
        let request_digest = digest(1);
        let binding_digest = digest(11);
        let postcondition = digest(21);
        let plan = ServicePlan {
            service_id: "payments-api".into(),
            plan_digest,
            expected_service_digest: digest(91),
            steps: vec![OperationSpec {
                id: "ticket-step".into(),
                target_id: "itsm-case-42".into(),
                operation_id: "itsm.update_case".into(),
                request_digest,
                binding_digest,
                expected_postcondition_digest: postcondition,
                dependencies: vec![],
                required: true,
                compensation: None,
            }],
        };
        let mut saga = SagaEpisode::new(plan, "agent-runtime").expect("valid saga plan");
        let first_grant = AuthorityGrant {
            grant_id: "grant-1".into(),
            plan_digest,
            step_id: "ticket-step".into(),
            target_id: "itsm-case-42".into(),
            operation_id: "itsm.update_case".into(),
            request_digest,
            binding_digest,
            valid_until_ms: 1_000,
        };
        saga.authorize_step(0, "ticket-step", &first_grant, 1)
            .expect("first exact grant");
        saga.persist_dispatch_intent(
            saga.revision(),
            "ticket-step",
            "attempt-1",
            "idem-1",
        )
        .expect("write-ahead saga dispatch intent");

        let mut connector = MockConnector::default();
        let first_request = request("attempt-1", "idem-1", 1);
        assert_eq!(
            connector.submit(first_request.clone(), FaultMode::BeforeAdmission),
            TransportOutcome::TimedOut
        );
        let evidence = connector.observer_evidence(
            &first_request,
            LedgerConsistency::Strong,
            50,
            200,
        );
        saga.record_observation(saga.revision(), "ticket-step", &evidence, 100)
            .expect("attempt-specific ledger proof reconciles the dispatched step");
        assert!(matches!(
            saga.step_status("ticket-step"),
            Some(StepStatus::NoEffectConfirmed { .. })
        ));

        let second_grant = AuthorityGrant {
            grant_id: "grant-2".into(),
            ..first_grant
        };
        saga.authorize_step(
            saga.revision(),
            "ticket-step",
            &second_grant,
            101,
        )
        .expect("fresh grant required after never-accepted proof");
        saga.persist_dispatch_intent(
            saga.revision(),
            "ticket-step",
            "attempt-2",
            "idem-1",
        )
        .expect("new unique attempt with stable logical idempotency key");

        let second_request = request("attempt-2", "idem-1", 1);
        assert!(matches!(
            connector.submit(second_request, FaultMode::Normal),
            TransportOutcome::Acknowledged(_)
        ));
        assert_eq!(connector.business_effect_count(), 1);
    }
}
