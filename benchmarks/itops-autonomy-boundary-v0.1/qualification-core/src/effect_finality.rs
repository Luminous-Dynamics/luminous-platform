//! Synthetic participant-finality and unknown-outcome state machine.
//!
//! This module models the atomic decision boundary between current authority,
//! an idempotent request binding, a budget debit and a durable effect receipt.
//! It is a deterministic in-process reference model, not a distributed ledger,
//! a production participant, a cryptographic verifier or an independent observer.
//!
//! The model intentionally treats a commit and authority-epoch check as one
//! mutex-protected linearization point. A real adapter must provide equivalent
//! transactional/consensus semantics at the protected resource.

use std::collections::BTreeMap;
use std::sync::{Mutex, MutexGuard};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectBinding {
    pub idempotency_key: String,
    /// Opaque digest supplied by the existing Act-Bound owner. This module
    /// never canonicalizes action data or computes a production digest.
    pub request_digest: String,
    pub actor: String,
    pub tenant: String,
    pub resource: String,
    pub operation: String,
    pub authority_epoch: u64,
}

impl EffectBinding {
    fn is_well_formed(&self) -> bool {
        !self.idempotency_key.trim().is_empty()
            && !self.request_digest.trim().is_empty()
            && !self.actor.trim().is_empty()
            && !self.tenant.trim().is_empty()
            && !self.resource.trim().is_empty()
            && !self.operation.trim().is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectReceipt {
    pub transaction_id: String,
    pub idempotency_key: String,
    pub request_digest: String,
    pub actor: String,
    pub tenant: String,
    pub resource: String,
    pub operation: String,
    pub authority_epoch: u64,
    pub state_revision: u64,
    pub commit_sequence: u64,
}

impl EffectReceipt {
    pub fn matches(&self, binding: &EffectBinding) -> bool {
        self.idempotency_key == binding.idempotency_key
            && self.request_digest == binding.request_digest
            && self.actor == binding.actor
            && self.tenant == binding.tenant
            && self.resource == binding.resource
            && self.operation == binding.operation
            && self.authority_epoch == binding.authority_epoch
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitOutcome {
    Committed(EffectReceipt),
    AlreadyCommitted(EffectReceipt),
    DeniedStaleAuthorityEpoch {
        presented: u64,
        current: u64,
    },
    DeniedBudgetExhausted,
    IdempotencyKeyConflict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerError {
    Poisoned,
    InvalidBinding,
    EpochExhausted,
}

#[derive(Debug)]
struct LedgerState {
    authority_epoch: u64,
    remaining_budget: u64,
    next_commit_sequence: u64,
    receipts_by_key: BTreeMap<String, EffectReceipt>,
}

/// In-process participant model. The Mutex is the modeled commit
/// linearization boundary; production adapters must replace this with the
/// protected store's real atomic transaction or consensus primitive.
#[derive(Debug)]
pub struct ParticipantLedger {
    state: Mutex<LedgerState>,
}

impl ParticipantLedger {
    pub fn new(authority_epoch: u64, mutation_budget: u64) -> Self {
        Self {
            state: Mutex::new(LedgerState {
                authority_epoch,
                remaining_budget: mutation_budget,
                next_commit_sequence: 1,
                receipts_by_key: BTreeMap::new(),
            }),
        }
    }

    fn lock(&self) -> Result<MutexGuard<'_, LedgerState>, LedgerError> {
        self.state.lock().map_err(|_| LedgerError::Poisoned)
    }

    /// Advance the participant's authority/fencing epoch.
    ///
    /// Effects committed before this transition remain historically committed;
    /// new requests carrying the old epoch are denied after this returns.
    pub fn revoke_and_advance_epoch(&self) -> Result<u64, LedgerError> {
        let mut state = self.lock()?;
        state.authority_epoch = state
            .authority_epoch
            .checked_add(1)
            .ok_or(LedgerError::EpochExhausted)?;
        Ok(state.authority_epoch)
    }

    pub fn current_authority_epoch(&self) -> Result<u64, LedgerError> {
        Ok(self.lock()?.authority_epoch)
    }

    pub fn remaining_budget(&self) -> Result<u64, LedgerError> {
        Ok(self.lock()?.remaining_budget)
    }

    /// Linearizable idempotent commit.
    ///
    /// Existing idempotency keys are reconciled before checking the current
    /// authority epoch: an exact retry retrieves the original outcome, not a
    /// second mutation. Reusing a key with different request fields is a hard
    /// conflict. New keys require the exact current epoch and remaining budget.
    pub fn commit(&self, binding: &EffectBinding) -> Result<CommitOutcome, LedgerError> {
        if !binding.is_well_formed() {
            return Err(LedgerError::InvalidBinding);
        }

        let mut state = self.lock()?;

        if let Some(existing) = state.receipts_by_key.get(&binding.idempotency_key) {
            return if existing.matches(binding) {
                Ok(CommitOutcome::AlreadyCommitted(existing.clone()))
            } else {
                Ok(CommitOutcome::IdempotencyKeyConflict)
            };
        }

        if binding.authority_epoch != state.authority_epoch {
            return Ok(CommitOutcome::DeniedStaleAuthorityEpoch {
                presented: binding.authority_epoch,
                current: state.authority_epoch,
            });
        }

        if state.remaining_budget == 0 {
            return Ok(CommitOutcome::DeniedBudgetExhausted);
        }

        state.remaining_budget -= 1;
        let sequence = state.next_commit_sequence;
        state.next_commit_sequence = sequence
            .checked_add(1)
            .ok_or(LedgerError::Poisoned)?;

        let receipt = EffectReceipt {
            transaction_id: format!("synthetic-tx-{sequence}"),
            idempotency_key: binding.idempotency_key.clone(),
            request_digest: binding.request_digest.clone(),
            actor: binding.actor.clone(),
            tenant: binding.tenant.clone(),
            resource: binding.resource.clone(),
            operation: binding.operation.clone(),
            authority_epoch: binding.authority_epoch,
            state_revision: sequence,
            commit_sequence: sequence,
        };

        state
            .receipts_by_key
            .insert(binding.idempotency_key.clone(), receipt.clone());
        Ok(CommitOutcome::Committed(receipt))
    }

    pub fn lookup(
        &self,
        idempotency_key: &str,
    ) -> Result<Option<EffectReceipt>, LedgerError> {
        Ok(self.lock()?.receipts_by_key.get(idempotency_key).cloned())
    }
}

/// Verifies that an observed receipt is bound to the exact request fields.
/// This checks consistency only; it does not establish observer independence,
/// signature validity, or the truth of a remote resource state.
pub fn verify_receipt_binding(
    receipt: &EffectReceipt,
    binding: &EffectBinding,
) -> bool {
    receipt.matches(binding)
        && receipt.state_revision == receipt.commit_sequence
        && receipt.commit_sequence > 0
        && !receipt.transaction_id.trim().is_empty()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryFault {
    None,
    LostBeforeParticipant,
    LostAfterParticipant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttemptState {
    Empty,
    Prepared,
    OutcomeUnknown,
    ReconciledAbsent,
    Committed,
    Denied,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Submission {
    Response(CommitOutcome),
    ResponseLost,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reconciliation {
    Committed(EffectReceipt),
    Absent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionError {
    NoPreparedAttempt,
    UnresolvedOutcome,
    RetrySameAttemptRequired,
    InvalidTransition,
    ReceiptConflict,
    Ledger(LedgerError),
}

/// Client-side guard preventing a new consequential attempt while the prior
/// outcome is unknown. An absent reconciliation permits only an exact retry
/// of the same binding/idempotency key.
#[derive(Debug)]
pub struct EffectSession {
    binding: Option<EffectBinding>,
    state: AttemptState,
}

impl Default for EffectSession {
    fn default() -> Self {
        Self {
            binding: None,
            state: AttemptState::Empty,
        }
    }
}

impl EffectSession {
    pub fn state(&self) -> AttemptState {
        self.state
    }

    pub fn prepare(&mut self, binding: EffectBinding) -> Result<(), SessionError> {
        match self.state {
            AttemptState::Empty | AttemptState::Committed | AttemptState::Denied => {
                self.binding = Some(binding);
                self.state = AttemptState::Prepared;
                Ok(())
            }
            AttemptState::OutcomeUnknown => Err(SessionError::UnresolvedOutcome),
            AttemptState::ReconciledAbsent => Err(SessionError::RetrySameAttemptRequired),
            AttemptState::Prepared => Err(SessionError::InvalidTransition),
        }
    }

    pub fn submit(
        &mut self,
        participant: &ParticipantLedger,
        fault: DeliveryFault,
    ) -> Result<Submission, SessionError> {
        if self.state != AttemptState::Prepared {
            return Err(SessionError::InvalidTransition);
        }
        let binding = self
            .binding
            .as_ref()
            .ok_or(SessionError::NoPreparedAttempt)?;

        if fault == DeliveryFault::LostBeforeParticipant {
            self.state = AttemptState::OutcomeUnknown;
            return Ok(Submission::ResponseLost);
        }

        let outcome = participant.commit(binding).map_err(SessionError::Ledger)?;
        if fault == DeliveryFault::LostAfterParticipant {
            self.state = AttemptState::OutcomeUnknown;
            return Ok(Submission::ResponseLost);
        }

        self.apply_outcome(&outcome);
        Ok(Submission::Response(outcome))
    }

    fn apply_outcome(&mut self, outcome: &CommitOutcome) {
        self.state = match outcome {
            CommitOutcome::Committed(_) | CommitOutcome::AlreadyCommitted(_) => {
                AttemptState::Committed
            }
            CommitOutcome::DeniedStaleAuthorityEpoch { .. }
            | CommitOutcome::DeniedBudgetExhausted
            | CommitOutcome::IdempotencyKeyConflict => AttemptState::Denied,
        };
    }

    pub fn reconcile(
        &mut self,
        participant: &ParticipantLedger,
    ) -> Result<Reconciliation, SessionError> {
        if self.state != AttemptState::OutcomeUnknown {
            return Err(SessionError::InvalidTransition);
        }
        let binding = self
            .binding
            .as_ref()
            .ok_or(SessionError::NoPreparedAttempt)?;
        match participant
            .lookup(&binding.idempotency_key)
            .map_err(SessionError::Ledger)?
        {
            Some(receipt) if verify_receipt_binding(&receipt, binding) => {
                self.state = AttemptState::Committed;
                Ok(Reconciliation::Committed(receipt))
            }
            Some(_) => Err(SessionError::ReceiptConflict),
            None => {
                self.state = AttemptState::ReconciledAbsent;
                Ok(Reconciliation::Absent)
            }
        }
    }

    /// Retry only after an authoritative absent reconciliation. The original
    /// action binding and idempotency key are retained unchanged.
    pub fn retry_same_attempt(&mut self) -> Result<(), SessionError> {
        if self.state != AttemptState::ReconciledAbsent || self.binding.is_none() {
            return Err(SessionError::InvalidTransition);
        }
        self.state = AttemptState::Prepared;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};
    use std::thread;

    fn binding(key: &str, digest: &str, epoch: u64) -> EffectBinding {
        EffectBinding {
            idempotency_key: key.into(),
            request_digest: digest.into(),
            actor: "agent-7".into(),
            tenant: "tenant-a".into(),
            resource: "ledger/account-4".into(),
            operation: "debit".into(),
            authority_epoch: epoch,
        }
    }

    fn receipt(outcome: CommitOutcome) -> EffectReceipt {
        match outcome {
            CommitOutcome::Committed(value) | CommitOutcome::AlreadyCommitted(value) => value,
            other => panic!("expected committed receipt, got {other:?}"),
        }
    }

    #[test]
    fn current_epoch_and_budget_are_checked_at_commit() {
        let participant = ParticipantLedger::new(3, 1);
        let stale = participant
            .commit(&binding("idem-old", "digest-old", 2))
            .unwrap();
        assert_eq!(
            stale,
            CommitOutcome::DeniedStaleAuthorityEpoch {
                presented: 2,
                current: 3
            }
        );
        let committed = participant
            .commit(&binding("idem-current", "digest-current", 3))
            .unwrap();
        assert!(matches!(committed, CommitOutcome::Committed(_)));
        assert_eq!(participant.remaining_budget().unwrap(), 0);
    }

    #[test]
    fn commit_before_revocation_is_not_retroactively_invalidated() {
        let participant = ParticipantLedger::new(7, 2);
        let request = binding("idem-before-revoke", "digest-a", 7);
        let first = receipt(participant.commit(&request).unwrap());
        assert_eq!(participant.revoke_and_advance_epoch().unwrap(), 8);
        let retry = receipt(participant.commit(&request).unwrap());
        assert_eq!(first, retry);
        assert_eq!(participant.remaining_budget().unwrap(), 1);

        let new_request = binding("idem-after-revoke", "digest-b", 7);
        assert!(matches!(
            participant.commit(&new_request).unwrap(),
            CommitOutcome::DeniedStaleAuthorityEpoch {
                presented: 7,
                current: 8
            }
        ));
    }

    #[test]
    fn same_key_and_same_binding_retrieves_one_commit() {
        let participant = ParticipantLedger::new(0, 1);
        let request = binding("idem-1", "digest-1", 0);
        let first = receipt(participant.commit(&request).unwrap());
        let second = receipt(participant.commit(&request).unwrap());
        assert_eq!(first, second);
        assert_eq!(participant.remaining_budget().unwrap(), 0);
    }

    #[test]
    fn same_key_with_different_request_is_a_conflict() {
        let participant = ParticipantLedger::new(0, 2);
        let first = binding("idem-1", "digest-1", 0);
        let changed = binding("idem-1", "digest-2", 0);
        assert!(matches!(
            participant.commit(&first).unwrap(),
            CommitOutcome::Committed(_)
        ));
        assert_eq!(
            participant.commit(&changed).unwrap(),
            CommitOutcome::IdempotencyKeyConflict
        );
        assert_eq!(participant.remaining_budget().unwrap(), 1);
    }

    #[test]
    fn budget_is_debited_once_per_distinct_committed_effect() {
        let participant = ParticipantLedger::new(0, 1);
        assert!(matches!(
            participant.commit(&binding("idem-1", "digest-1", 0)).unwrap(),
            CommitOutcome::Committed(_)
        ));
        assert_eq!(
            participant.commit(&binding("idem-2", "digest-2", 0)).unwrap(),
            CommitOutcome::DeniedBudgetExhausted
        );
        assert_eq!(participant.remaining_budget().unwrap(), 0);
    }

    #[test]
    fn response_loss_after_commit_locks_until_reconciliation() {
        let participant = ParticipantLedger::new(0, 2);
        let mut session = EffectSession::default();
        let original = binding("idem-original", "digest-original", 0);
        session.prepare(original).unwrap();
        assert_eq!(
            session
                .submit(&participant, DeliveryFault::LostAfterParticipant)
                .unwrap(),
            Submission::ResponseLost
        );
        assert_eq!(session.state(), AttemptState::OutcomeUnknown);
        assert_eq!(
            session.prepare(binding("idem-new", "digest-new", 0)),
            Err(SessionError::UnresolvedOutcome)
        );
        let reconciled = session.reconcile(&participant).unwrap();
        assert!(matches!(reconciled, Reconciliation::Committed(_)));
        assert_eq!(session.state(), AttemptState::Committed);
        session
            .prepare(binding("idem-next", "digest-next", 0))
            .unwrap();
    }

    #[test]
    fn response_loss_before_commit_requires_same_key_retry() {
        let participant = ParticipantLedger::new(0, 1);
        let mut session = EffectSession::default();
        let original = binding("idem-same", "digest-same", 0);
        session.prepare(original.clone()).unwrap();
        assert_eq!(
            session
                .submit(&participant, DeliveryFault::LostBeforeParticipant)
                .unwrap(),
            Submission::ResponseLost
        );
        assert_eq!(
            session.reconcile(&participant).unwrap(),
            Reconciliation::Absent
        );
        assert_eq!(
            session.prepare(binding("idem-new", "digest-new", 0)),
            Err(SessionError::RetrySameAttemptRequired)
        );
        session.retry_same_attempt().unwrap();
        assert!(matches!(
            session.submit(&participant, DeliveryFault::None).unwrap(),
            Submission::Response(CommitOutcome::Committed(_))
        ));
        assert_eq!(session.state(), AttemptState::Committed);
        let reconciled = participant.lookup("idem-same").unwrap().unwrap();
        assert_eq!(reconciled.idempotency_key, "idem-same");
        assert_eq!(participant.remaining_budget().unwrap(), 0);
        assert_eq!(participant.remaining_budget().unwrap(), 0);
    }

    #[test]
    fn receipt_binding_rejects_field_substitution() {
        let participant = ParticipantLedger::new(2, 1);
        let original = binding("idem-1", "digest-1", 2);
        let observed = receipt(participant.commit(&original).unwrap());
        assert!(verify_receipt_binding(&observed, &original));

        let mut mutations = Vec::new();
        let mut changed = original.clone();
        changed.actor = "agent-other".into();
        mutations.push(changed);
        let mut changed = original.clone();
        changed.tenant = "tenant-other".into();
        mutations.push(changed);
        let mut changed = original.clone();
        changed.resource = "ledger/account-99".into();
        mutations.push(changed);
        let mut changed = original.clone();
        changed.operation = "credit".into();
        mutations.push(changed);
        let mut changed = original.clone();
        changed.request_digest = "digest-other".into();
        mutations.push(changed);
        let mut changed = original.clone();
        changed.authority_epoch += 1;
        mutations.push(changed);
        let mut changed = original.clone();
        changed.idempotency_key = "idem-other".into();
        mutations.push(changed);

        for mutation in mutations {
            assert!(!verify_receipt_binding(&observed, &mutation));
        }
    }

    #[test]
    fn concurrent_same_key_commit_creates_one_effect_and_one_budget_debit() {
        let participant = Arc::new(ParticipantLedger::new(4, 1));
        let start = Arc::new(Barrier::new(8));
        let request = binding("idem-concurrent", "digest-concurrent", 4);
        let mut handles = Vec::new();

        for _ in 0..8 {
            let participant = Arc::clone(&participant);
            let start = Arc::clone(&start);
            let request = request.clone();
            handles.push(thread::spawn(move || {
                start.wait();
                participant.commit(&request).unwrap()
            }));
        }

        let outcomes: Vec<_> = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect();
        let committed = outcomes
            .iter()
            .filter(|outcome| matches!(outcome, CommitOutcome::Committed(_)))
            .count();
        let replayed = outcomes
            .iter()
            .filter(|outcome| matches!(outcome, CommitOutcome::AlreadyCommitted(_)))
            .count();
        assert_eq!(committed, 1);
        assert_eq!(replayed, 7);
        assert_eq!(participant.remaining_budget().unwrap(), 0);
    }

    #[test]
    fn unknown_outcome_cannot_be_replaced_by_a_new_idempotency_key() {
        let participant = ParticipantLedger::new(0, 1);
        let mut session = EffectSession::default();
        session
            .prepare(binding("idem-original", "digest-original", 0))
            .unwrap();
        session
            .submit(&participant, DeliveryFault::LostBeforeParticipant)
            .unwrap();
        assert_eq!(
            session.prepare(binding("idem-replacement", "digest-replacement", 0)),
            Err(SessionError::UnresolvedOutcome)
        );
    }
}
