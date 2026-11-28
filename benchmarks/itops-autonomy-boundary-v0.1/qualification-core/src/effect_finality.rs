//! Synthetic participant-finality and unknown-outcome state machine.
//!
//! This module models the atomic decision boundary between current authority,
//! resource revision, dual evidence identities, idempotency, budget debit and
//! a durable effect receipt.
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
    /// The relying party that presents this mandate. A shared participant
    /// records the RP identity but does not silently allocate a separate budget
    /// to each RP.
    pub relying_party: String,
    /// Opaque digest supplied by the existing Act-Bound owner. This module
    /// never canonicalizes action data or computes a production digest.
    pub request_digest: String,
    /// Opaque identity of the preflight evidence bundle.
    pub preflight_evidence_digest: String,
    /// Opaque identity of the fresh commit-time evidence bundle.
    pub commit_evidence_digest: String,
    /// Resource revision observed during preflight.
    pub resource_version: String,
    /// Monotonically increasing participant-observed revision; prevents ABA
    /// when an opaque resource-version label is later reused.
    pub resource_revision: u64,
    pub actor: String,
    pub tenant: String,
    pub resource: String,
    pub operation: String,
    pub authority_epoch: u64,
}

impl EffectBinding {
    fn is_well_formed(&self) -> bool {
        !self.idempotency_key.trim().is_empty()
            && !self.relying_party.trim().is_empty()
            && !self.request_digest.trim().is_empty()
            && !self.preflight_evidence_digest.trim().is_empty()
            && !self.commit_evidence_digest.trim().is_empty()
            && !self.resource_version.trim().is_empty()
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
    pub relying_party: String,
    pub request_digest: String,
    pub preflight_evidence_digest: String,
    pub commit_evidence_digest: String,
    pub resource_version: String,
    pub resource_revision: u64,
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
            && self.relying_party == binding.relying_party
            && self.request_digest == binding.request_digest
            && self.preflight_evidence_digest == binding.preflight_evidence_digest
            && self.commit_evidence_digest == binding.commit_evidence_digest
            && self.resource_version == binding.resource_version
            && self.resource_revision == binding.resource_revision
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
    PendingAccepted,
    AlreadyPending,
    DeniedStaleAuthorityEpoch { presented: u64, current: u64 },
    DeniedStaleResourceRevision {
        presented_version: String,
        current_version: String,
        presented_revision: u64,
        current_revision: u64,
    },
    DeniedOutsideParticipantScope,
    DeniedBudgetExhausted,
    IdempotencyKeyConflict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerError {
    Poisoned,
    InvalidBinding,
    NoPendingAttempt,
    InvalidResourceVersion,
    ResourceRevisionExhausted,
    EpochExhausted,
    SequenceExhausted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingRecord {
    binding: EffectBinding,
}

#[derive(Debug)]
struct LedgerState {
    /// Some means all new effects must target this exact tenant/resource.
    /// None is retained for small legacy unit fixtures.
    participant_scope: Option<(String, String)>,
    current_resource_version: String,
    resource_revision: u64,
    authority_epoch: u64,
    remaining_budget: u64,
    reserved_budget: u64,
    next_commit_sequence: u64,
    pending_by_key: BTreeMap<String, PendingRecord>,
    aborted_by_key: BTreeMap<String, EffectBinding>,
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
    /// Unscoped constructor retained for legacy policy tests. Participant
    /// adapters should prefer new_scoped.
    pub fn new(authority_epoch: u64, mutation_budget: u64) -> Self {
        Self::build(None, "resource-v1".into(), authority_epoch, mutation_budget)
    }

    /// Creates a participant tied to one tenant/resource pair. All RPs that
    /// share this participant also share its mutation budget and idempotency
    /// ledger; per-RP budgets require an explicit authority-side allocation.
    pub fn new_scoped(
        tenant: impl Into<String>,
        resource: impl Into<String>,
        authority_epoch: u64,
        mutation_budget: u64,
    ) -> Self {
        Self::build(
            Some((tenant.into(), resource.into())),
            "resource-v1".into(),
            authority_epoch,
            mutation_budget,
        )
    }

    /// Creates a participant with an explicit protected-resource revision.
    pub fn new_scoped_versioned(
        tenant: impl Into<String>,
        resource: impl Into<String>,
        resource_version: impl Into<String>,
        authority_epoch: u64,
        mutation_budget: u64,
    ) -> Self {
        Self::build(
            Some((tenant.into(), resource.into())),
            resource_version.into(),
            authority_epoch,
            mutation_budget,
        )
    }

    fn build(
        participant_scope: Option<(String, String)>,
        current_resource_version: String,
        authority_epoch: u64,
        mutation_budget: u64,
    ) -> Self {
        Self {
            state: Mutex::new(LedgerState {
                participant_scope,
                current_resource_version,
                resource_revision: 0,
                authority_epoch,
                remaining_budget: mutation_budget,
                reserved_budget: 0,
                next_commit_sequence: 1,
                pending_by_key: BTreeMap::new(),
                aborted_by_key: BTreeMap::new(),
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

    pub fn current_resource_version(&self) -> Result<String, LedgerError> {
        Ok(self.lock()?.current_resource_version.clone())
    }

    pub fn current_resource_revision(&self) -> Result<u64, LedgerError> {
        Ok(self.lock()?.resource_revision)
    }

    /// Publish a new resource revision. Commit and revision changes share the
    /// same lock, yielding one in-process ordering for a concurrent race.
    pub fn advance_resource_version(
        &self,
        next_version: impl Into<String>,
    ) -> Result<u64, LedgerError> {
        let next_version = next_version.into();
        if next_version.trim().is_empty() {
            return Err(LedgerError::InvalidResourceVersion);
        }
        let mut state = self.lock()?;
        if next_version == state.current_resource_version {
            return Err(LedgerError::InvalidResourceVersion);
        }
        let next_revision = state
            .resource_revision
            .checked_add(1)
            .ok_or(LedgerError::ResourceRevisionExhausted)?;
        state.current_resource_version = next_version;
        state.resource_revision = next_revision;
        Ok(next_revision)
    }

    pub fn remaining_budget(&self) -> Result<u64, LedgerError> {
        Ok(self.lock()?.remaining_budget)
    }

    pub fn reserved_budget(&self) -> Result<u64, LedgerError> {
        Ok(self.lock()?.reserved_budget)
    }

    pub fn available_budget(&self) -> Result<u64, LedgerError> {
        let state = self.lock()?;
        Ok(state.remaining_budget.saturating_sub(state.reserved_budget))
    }

    /// Linearizable idempotent commit.
    ///
    /// Existing idempotency keys are reconciled before checking the current
    /// authority epoch: an exact retry retrieves the original outcome, not a
    /// second mutation. Reusing a key with different request fields is a hard
    /// conflict. New keys require the exact current authority epoch, resource
    /// revision and remaining budget. The receipt preserves both evidence
    /// bundle identities supplied by their existing owners.
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

        if let Some(existing) = state.pending_by_key.get(&binding.idempotency_key) {
            return if existing.binding == *binding {
                Ok(CommitOutcome::AlreadyPending)
            } else {
                Ok(CommitOutcome::IdempotencyKeyConflict)
            };
        }

        if let Some(existing) = state.aborted_by_key.get(&binding.idempotency_key) {
            if existing != binding {
                return Ok(CommitOutcome::IdempotencyKeyConflict);
            }
            // Preserve the terminal marker until every admission check passes.
        }

        if let Some((tenant, resource)) = &state.participant_scope {
            if binding.tenant != *tenant || binding.resource != *resource {
                return Ok(CommitOutcome::DeniedOutsideParticipantScope);
            }
        }

        if binding.authority_epoch != state.authority_epoch {
            return Ok(CommitOutcome::DeniedStaleAuthorityEpoch {
                presented: binding.authority_epoch,
                current: state.authority_epoch,
            });
        }

        if binding.resource_version != state.current_resource_version
            || binding.resource_revision != state.resource_revision
        {
            return Ok(CommitOutcome::DeniedStaleResourceRevision {
                presented_version: binding.resource_version.clone(),
                current_version: state.current_resource_version.clone(),
                presented_revision: binding.resource_revision,
                current_revision: state.resource_revision,
            });
        }

        if state.remaining_budget <= state.reserved_budget {
            return Ok(CommitOutcome::DeniedBudgetExhausted);
        }

        // A terminal marker is consumed only once the exact retry is admitted.
        state.aborted_by_key.remove(&binding.idempotency_key);

        // Complete fallible counter arithmetic before mutating budget.
        let sequence = state.next_commit_sequence;
        let next_sequence = sequence
            .checked_add(1)
            .ok_or(LedgerError::SequenceExhausted)?;

        state.remaining_budget -= 1;
        state.next_commit_sequence = next_sequence;

        let receipt = EffectReceipt {
            transaction_id: format!("synthetic-tx-{sequence}"),
            idempotency_key: binding.idempotency_key.clone(),
            relying_party: binding.relying_party.clone(),
            request_digest: binding.request_digest.clone(),
            preflight_evidence_digest: binding.preflight_evidence_digest.clone(),
            commit_evidence_digest: binding.commit_evidence_digest.clone(),
            resource_version: binding.resource_version.clone(),
            resource_revision: binding.resource_revision,
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

    /// Admit an asynchronous request into the participant's durable
    /// idempotency journal. The budget unit is reserved before work is queued,
    /// so concurrent pending requests cannot overbook the same budget.
    ///
    /// A production adapter must persist this admission record transactionally
    /// before acknowledging or queueing the work. This in-process model only
    /// demonstrates the state-machine contract.
    pub fn accept_pending(
        &self,
        binding: &EffectBinding,
    ) -> Result<CommitOutcome, LedgerError> {
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
        if let Some(existing) = state.pending_by_key.get(&binding.idempotency_key) {
            return if existing.binding == *binding {
                Ok(CommitOutcome::AlreadyPending)
            } else {
                Ok(CommitOutcome::IdempotencyKeyConflict)
            };
        }
        if let Some(existing) = state.aborted_by_key.get(&binding.idempotency_key) {
            if existing != binding {
                return Ok(CommitOutcome::IdempotencyKeyConflict);
            }
        }

        if let Some((tenant, resource)) = &state.participant_scope {
            if binding.tenant != *tenant || binding.resource != *resource {
                return Ok(CommitOutcome::DeniedOutsideParticipantScope);
            }
        }
        if binding.authority_epoch != state.authority_epoch {
            return Ok(CommitOutcome::DeniedStaleAuthorityEpoch {
                presented: binding.authority_epoch,
                current: state.authority_epoch,
            });
        }
        if binding.resource_version != state.current_resource_version
            || binding.resource_revision != state.resource_revision
        {
            return Ok(CommitOutcome::DeniedStaleResourceRevision {
                presented_version: binding.resource_version.clone(),
                current_version: state.current_resource_version.clone(),
                presented_revision: binding.resource_revision,
                current_revision: state.resource_revision,
            });
        }
        if state.remaining_budget <= state.reserved_budget {
            return Ok(CommitOutcome::DeniedBudgetExhausted);
        }

        // Consume the exact terminal marker only when reserving this retry.
        state.aborted_by_key.remove(&binding.idempotency_key);
        state.reserved_budget += 1;
        state.pending_by_key.insert(
            binding.idempotency_key.clone(),
            PendingRecord {
                binding: binding.clone(),
            },
        );
        Ok(CommitOutcome::PendingAccepted)
    }

    /// Finalize a previously admitted request. Authority epoch and resource
    /// revision are checked again at the actual commit point. Any denial writes
    /// a terminal aborted journal marker and releases the reserved budget.
    pub fn finalize_pending(
        &self,
        idempotency_key: &str,
    ) -> Result<CommitOutcome, LedgerError> {
        let mut state = self.lock()?;
        let pending = state
            .pending_by_key
            .get(idempotency_key)
            .cloned()
            .ok_or(LedgerError::NoPendingAttempt)?;
        let binding = pending.binding;

        let denial = if binding.authority_epoch != state.authority_epoch {
            Some(CommitOutcome::DeniedStaleAuthorityEpoch {
                presented: binding.authority_epoch,
                current: state.authority_epoch,
            })
        } else if binding.resource_version != state.current_resource_version
            || binding.resource_revision != state.resource_revision
        {
            Some(CommitOutcome::DeniedStaleResourceRevision {
                presented_version: binding.resource_version.clone(),
                current_version: state.current_resource_version.clone(),
                presented_revision: binding.resource_revision,
                current_revision: state.resource_revision,
            })
        } else {
            None
        };

        if let Some(outcome) = denial {
            state.pending_by_key.remove(idempotency_key);
            state.reserved_budget -= 1;
            state
                .aborted_by_key
                .insert(idempotency_key.to_owned(), binding);
            return Ok(outcome);
        }

        // Perform all fallible arithmetic before consuming the reservation.
        let sequence = state.next_commit_sequence;
        let next_sequence = sequence
            .checked_add(1)
            .ok_or(LedgerError::SequenceExhausted)?;
        if state.remaining_budget == 0 || state.reserved_budget == 0 {
            return Err(LedgerError::Poisoned);
        }

        state.pending_by_key.remove(idempotency_key);
        state.reserved_budget -= 1;
        state.remaining_budget -= 1;
        state.next_commit_sequence = next_sequence;

        let receipt = EffectReceipt {
            transaction_id: format!("synthetic-tx-{sequence}"),
            idempotency_key: binding.idempotency_key.clone(),
            relying_party: binding.relying_party.clone(),
            request_digest: binding.request_digest.clone(),
            preflight_evidence_digest: binding.preflight_evidence_digest.clone(),
            commit_evidence_digest: binding.commit_evidence_digest.clone(),
            resource_version: binding.resource_version.clone(),
            resource_revision: binding.resource_revision,
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
            .insert(idempotency_key.to_owned(), receipt.clone());
        Ok(CommitOutcome::Committed(receipt))
    }

    /// Abort an admitted pending request and persist a terminal marker so
    /// the exact same binding may be retried after reconciliation. This is a
    /// participant-owned terminal transition, not an inference from timeout.
    pub fn abort_pending(&self, idempotency_key: &str) -> Result<(), LedgerError> {
        let mut state = self.lock()?;
        let pending = state
            .pending_by_key
            .get(idempotency_key)
            .cloned()
            .ok_or(LedgerError::NoPendingAttempt)?;
        if state.reserved_budget == 0 {
            return Err(LedgerError::Poisoned);
        }
        state.pending_by_key.remove(idempotency_key);
        state.reserved_budget -= 1;
        state
            .aborted_by_key
            .insert(idempotency_key.to_owned(), pending.binding);
        Ok(())
    }

    /// Record an authoritative terminal result from the ingress/idempotency
    /// coordinator proving this exact request was not accepted for execution.
    /// A bare missing receipt is not sufficient; the upstream system must
    /// produce this terminal marker after its admission window/watermark closes.
    pub(crate) fn record_not_accepted(
        &self,
        binding: &EffectBinding,
    ) -> Result<bool, LedgerError> {
        if !binding.is_well_formed() {
            return Err(LedgerError::InvalidBinding);
        }
        let mut state = self.lock()?;
        if state.receipts_by_key.contains_key(&binding.idempotency_key)
            || state.pending_by_key.contains_key(&binding.idempotency_key)
        {
            return Ok(false);
        }
        if let Some(existing) = state.aborted_by_key.get(&binding.idempotency_key) {
            return Ok(existing == binding);
        }
        state
            .aborted_by_key
            .insert(binding.idempotency_key.clone(), binding.clone());
        Ok(true)
    }

    /// Read the participant's authoritative idempotency journal. Absence of
    /// any entry returns NotObserved, not Aborted: a delayed original request
    /// may still arrive unless an ingress terminal marker has been recorded.
    pub fn reconcile_attempt(
        &self,
        binding: &EffectBinding,
    ) -> Result<ParticipantAttemptStatus, LedgerError> {
        let state = self.lock()?;
        if let Some(receipt) = state.receipts_by_key.get(&binding.idempotency_key) {
            return Ok(if receipt.matches(binding) {
                ParticipantAttemptStatus::Committed(receipt.clone())
            } else {
                ParticipantAttemptStatus::Conflict
            });
        }
        if let Some(pending) = state.pending_by_key.get(&binding.idempotency_key) {
            return Ok(if pending.binding == *binding {
                ParticipantAttemptStatus::Pending
            } else {
                ParticipantAttemptStatus::Conflict
            });
        }
        if let Some(aborted) = state.aborted_by_key.get(&binding.idempotency_key) {
            return Ok(if aborted == binding {
                ParticipantAttemptStatus::Aborted
            } else {
                ParticipantAttemptStatus::Conflict
            });
        }
        Ok(ParticipantAttemptStatus::NotObserved)
    }

    pub fn lookup(&self, idempotency_key: &str) -> Result<Option<EffectReceipt>, LedgerError> {
        Ok(self.lock()?.receipts_by_key.get(idempotency_key).cloned())
    }
}

/// Verifies that an observed receipt is bound to the exact request fields.
/// This checks consistency only; it does not establish observer independence,
/// signature validity, or the truth of a remote resource state.
pub fn verify_receipt_binding(receipt: &EffectReceipt, binding: &EffectBinding) -> bool {
    receipt.matches(binding)
        && receipt.state_revision == receipt.commit_sequence
        && receipt.commit_sequence > 0
        && !receipt.transaction_id.trim().is_empty()
}

/// State recorded by the participant's authoritative idempotency journal.
/// NotObserved is deliberately not equivalent to Aborted: the original request
/// may still be in a network queue or ingress worker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParticipantAttemptStatus {
    NotObserved,
    Pending,
    Aborted,
    Committed(EffectReceipt),
    Conflict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryFault {
    None,
    LostBeforeParticipant,
    AcceptedPending,
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
    AcceptedPending,
    ResponseLost,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reconciliation {
    Committed(EffectReceipt),
    Pending,
    NotObserved,
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
        if fault == DeliveryFault::AcceptedPending {
            let outcome = participant
                .accept_pending(binding)
                .map_err(SessionError::Ledger)?;
            match outcome {
                CommitOutcome::PendingAccepted | CommitOutcome::AlreadyPending => {
                    self.state = AttemptState::OutcomeUnknown;
                    return Ok(Submission::AcceptedPending);
                }
                other => {
                    self.apply_outcome(&other);
                    return Ok(Submission::Response(other));
                }
            }
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
            CommitOutcome::PendingAccepted | CommitOutcome::AlreadyPending => {
                AttemptState::OutcomeUnknown
            }
            CommitOutcome::DeniedStaleAuthorityEpoch { .. }
            | CommitOutcome::DeniedStaleResourceRevision { .. }
            | CommitOutcome::DeniedOutsideParticipantScope
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
            .reconcile_attempt(binding)
            .map_err(SessionError::Ledger)?
        {
            ParticipantAttemptStatus::Committed(receipt)
                if verify_receipt_binding(&receipt, binding) =>
            {
                self.state = AttemptState::Committed;
                Ok(Reconciliation::Committed(receipt))
            }
            ParticipantAttemptStatus::Committed(_) | ParticipantAttemptStatus::Conflict => {
                Err(SessionError::ReceiptConflict)
            }
            ParticipantAttemptStatus::Pending => Ok(Reconciliation::Pending),
            ParticipantAttemptStatus::NotObserved => Ok(Reconciliation::NotObserved),
            ParticipantAttemptStatus::Aborted => {
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
            relying_party: "rp-default".into(),
            request_digest: digest.into(),
            preflight_evidence_digest: "preflight-evidence-v1".into(),
            commit_evidence_digest: "commit-evidence-v1".into(),
            resource_version: "resource-v1".into(),
            resource_revision: 0,
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
        let mut changed = binding("idem-1", "digest-2", 0);
        changed.commit_evidence_digest = "commit-evidence-other".into();
        assert!(matches!(
            participant.commit(&first).unwrap(),
            CommitOutcome::Committed(_)
        ));
        assert_eq!(
            participant.commit(&changed).unwrap(),
            CommitOutcome::IdempotencyKeyConflict
        );
        let mut changed_evidence = first.clone();
        changed_evidence.commit_evidence_digest = "commit-evidence-substituted".into();
        assert_eq!(
            participant.commit(&changed_evidence).unwrap(),
            CommitOutcome::IdempotencyKeyConflict
        );
        assert_eq!(participant.remaining_budget().unwrap(), 1);
    }

    #[test]
    fn budget_is_debited_once_per_distinct_committed_effect() {
        let participant = ParticipantLedger::new(0, 1);
        assert!(matches!(
            participant
                .commit(&binding("idem-1", "digest-1", 0))
                .unwrap(),
            CommitOutcome::Committed(_)
        ));
        assert_eq!(
            participant
                .commit(&binding("idem-2", "digest-2", 0))
                .unwrap(),
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
    fn response_loss_without_terminal_ingress_proof_stays_unknown() {
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
            Reconciliation::NotObserved
        );
        assert_eq!(session.state(), AttemptState::OutcomeUnknown);
        assert_eq!(
            session.prepare(binding("idem-new", "digest-new", 0)),
            Err(SessionError::UnresolvedOutcome)
        );

        // Only an authoritative terminal ingress result can establish that the
        // request was not accepted. A missing receipt alone is not enough.
        assert!(participant.record_not_accepted(&original).unwrap());
        assert_eq!(
            session.reconcile(&participant).unwrap(),
            Reconciliation::Absent
        );
        assert_eq!(session.state(), AttemptState::ReconciledAbsent);
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
        changed.preflight_evidence_digest = "preflight-evidence-other".into();
        mutations.push(changed);
        let mut changed = original.clone();
        changed.commit_evidence_digest = "commit-evidence-other".into();
        mutations.push(changed);
        let mut changed = original.clone();
        changed.resource_version = "resource-v99".into();
        mutations.push(changed);
        let mut changed = original.clone();
        changed.resource_revision = 99;
        mutations.push(changed);
        let mut changed = original.clone();
        changed.relying_party = "rp-other".into();
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
    #[test]
    fn concurrent_distinct_requests_cannot_overspend_one_budget_unit() {
        let participant = Arc::new(ParticipantLedger::new(5, 1));
        let start = Arc::new(Barrier::new(2));
        let left = binding("idem-left", "digest-left", 5);
        let right = binding("idem-right", "digest-right", 5);

        let left_participant = Arc::clone(&participant);
        let left_start = Arc::clone(&start);
        let left_handle = thread::spawn(move || {
            left_start.wait();
            left_participant.commit(&left).unwrap()
        });

        let right_participant = Arc::clone(&participant);
        let right_start = Arc::clone(&start);
        let right_handle = thread::spawn(move || {
            right_start.wait();
            right_participant.commit(&right).unwrap()
        });

        let outcomes = [left_handle.join().unwrap(), right_handle.join().unwrap()];
        let committed = outcomes
            .iter()
            .filter(|outcome| matches!(outcome, CommitOutcome::Committed(_)))
            .count();
        let budget_denials = outcomes
            .iter()
            .filter(|outcome| matches!(outcome, CommitOutcome::DeniedBudgetExhausted))
            .count();

        assert_eq!(committed, 1);
        assert_eq!(budget_denials, 1);
        assert_eq!(participant.remaining_budget().unwrap(), 0);
    }

    #[test]
    fn shared_participant_budget_spans_relying_parties() {
        let participant = ParticipantLedger::new_scoped("tenant-a", "ledger/account-4", 9, 1);
        let mut first_rp = binding("idem-rp-a", "digest-rp-a", 9);
        first_rp.relying_party = "rp-a".into();
        let mut second_rp = binding("idem-rp-b", "digest-rp-b", 9);
        second_rp.relying_party = "rp-b".into();

        assert!(matches!(
            participant.commit(&first_rp).unwrap(),
            CommitOutcome::Committed(_)
        ));
        assert_eq!(
            participant.commit(&second_rp).unwrap(),
            CommitOutcome::DeniedBudgetExhausted
        );
        assert_eq!(participant.remaining_budget().unwrap(), 0);
    }

    #[test]
    fn scoped_participant_rejects_cross_tenant_or_resource_requests() {
        let participant = ParticipantLedger::new_scoped("tenant-a", "ledger/account-4", 2, 3);
        let mut wrong_tenant = binding("idem-wrong-tenant", "digest-tenant", 2);
        wrong_tenant.tenant = "tenant-b".into();
        assert_eq!(
            participant.commit(&wrong_tenant).unwrap(),
            CommitOutcome::DeniedOutsideParticipantScope
        );

        let mut wrong_resource = binding("idem-wrong-resource", "digest-resource", 2);
        wrong_resource.resource = "ledger/account-99".into();
        assert_eq!(
            participant.commit(&wrong_resource).unwrap(),
            CommitOutcome::DeniedOutsideParticipantScope
        );
        assert_eq!(participant.remaining_budget().unwrap(), 3);
    }

    #[test]
    fn sequence_exhaustion_does_not_debit_budget_or_create_receipt() {
        let participant = ParticipantLedger::new(0, 1);
        {
            let mut state = participant.lock().unwrap();
            state.next_commit_sequence = u64::MAX;
        }
        let request = binding("idem-sequence-overflow", "digest-overflow", 0);
        assert_eq!(
            participant.commit(&request),
            Err(LedgerError::SequenceExhausted)
        );
        assert_eq!(participant.remaining_budget().unwrap(), 1);
        assert_eq!(participant.lookup("idem-sequence-overflow").unwrap(), None);
    }

    #[test]
    fn concurrent_commit_and_revocation_have_one_linearized_outcome() {
        let participant = Arc::new(ParticipantLedger::new(3, 1));
        let start = Arc::new(Barrier::new(2));
        let request = binding("idem-revocation-race", "digest-revocation-race", 3);

        let commit_participant = Arc::clone(&participant);
        let commit_start = Arc::clone(&start);
        let commit_handle = thread::spawn(move || {
            commit_start.wait();
            commit_participant.commit(&request).unwrap()
        });

        let revoke_participant = Arc::clone(&participant);
        let revoke_start = Arc::clone(&start);
        let revoke_handle = thread::spawn(move || {
            revoke_start.wait();
            revoke_participant.revoke_and_advance_epoch().unwrap()
        });

        let outcome = commit_handle.join().unwrap();
        assert_eq!(revoke_handle.join().unwrap(), 4);
        assert_eq!(participant.current_authority_epoch().unwrap(), 4);

        match outcome {
            CommitOutcome::Committed(receipt) => {
                assert_eq!(receipt.authority_epoch, 3);
                assert_eq!(participant.remaining_budget().unwrap(), 0);
                assert_eq!(
                    participant.lookup("idem-revocation-race").unwrap(),
                    Some(receipt)
                );
            }
            CommitOutcome::DeniedStaleAuthorityEpoch {
                presented: 3,
                current: 4,
            } => {
                assert_eq!(participant.remaining_budget().unwrap(), 1);
                assert_eq!(
                    participant.lookup("idem-revocation-race").unwrap(),
                    None
                );
            }
            other => panic!("unexpected commit/revocation race outcome: {other:?}"),
        }
    }

    #[test]
    fn resource_version_is_checked_at_the_commit_boundary() {
        let participant = ParticipantLedger::new_scoped_versioned(
            "tenant-a",
            "ledger/account-4",
            "orders-v32",
            3,
            1,
        );
        let mut stale = binding("idem-resource-old", "digest-old", 3);
        stale.resource_version = "orders-v31".into();
        assert_eq!(
            participant.commit(&stale).unwrap(),
            CommitOutcome::DeniedStaleResourceRevision {
                presented_version: "orders-v31".into(),
                current_version: "orders-v32".into(),
                presented_revision: 0,
                current_revision: 0
            }
        );

        let mut current = binding("idem-resource-current", "digest-current", 3);
        current.resource_version = "orders-v32".into();
        assert!(matches!(
            participant.commit(&current).unwrap(),
            CommitOutcome::Committed(_)
        ));
        assert_eq!(participant.remaining_budget().unwrap(), 0);
    }

    #[test]
    fn resource_change_after_preflight_denies_old_request_without_debit() {
        let participant = ParticipantLedger::new_scoped_versioned(
            "tenant-a",
            "ledger/account-4",
            "orders-v32",
            3,
            1,
        );
        participant.advance_resource_version("orders-v33").unwrap();
        let mut request = binding("idem-stale-order", "digest-order", 3);
        request.resource_version = "orders-v32".into();
        assert_eq!(
            participant.commit(&request).unwrap(),
            CommitOutcome::DeniedStaleResourceRevision {
                presented_version: "orders-v32".into(),
                current_version: "orders-v33".into(),
                presented_revision: 0,
                current_revision: 1
            }
        );
        assert_eq!(participant.remaining_budget().unwrap(), 1);
        assert_eq!(participant.lookup("idem-stale-order").unwrap(), None);
    }

    #[test]
    fn concurrent_commit_and_resource_change_have_one_linearized_outcome() {
        let participant = Arc::new(ParticipantLedger::new_scoped_versioned(
            "tenant-a",
            "ledger/account-4",
            "orders-v32",
            3,
            1,
        ));
        let start = Arc::new(Barrier::new(2));
        let mut request = binding("idem-resource-race", "digest-race", 3);
        request.resource_version = "orders-v32".into();

        let commit_participant = Arc::clone(&participant);
        let commit_start = Arc::clone(&start);
        let commit_handle = thread::spawn(move || {
            commit_start.wait();
            commit_participant.commit(&request).unwrap()
        });

        let advance_participant = Arc::clone(&participant);
        let advance_start = Arc::clone(&start);
        let advance_handle = thread::spawn(move || {
            advance_start.wait();
            advance_participant.advance_resource_version("orders-v33").unwrap()
        });

        let outcome = commit_handle.join().unwrap();
        advance_handle.join().unwrap();
        assert_eq!(
            participant.current_resource_version().unwrap(),
            "orders-v33"
        );

        match outcome {
            CommitOutcome::Committed(receipt) => {
                assert_eq!(receipt.resource_version, "orders-v32");
                assert_eq!(receipt.resource_revision, 0);
                assert_eq!(participant.remaining_budget().unwrap(), 0);
                assert_eq!(
                    participant.lookup("idem-resource-race").unwrap(),
                    Some(receipt)
                );
            }
            CommitOutcome::DeniedStaleResourceRevision {
                presented_version,
                current_version,
                presented_revision,
                current_revision,
            } => {
                assert_eq!(presented_version, "orders-v32");
                assert_eq!(current_version, "orders-v33");
                assert_eq!(presented_revision, 0);
                assert_eq!(current_revision, 1);
                assert_eq!(participant.remaining_budget().unwrap(), 1);
                assert_eq!(participant.lookup("idem-resource-race").unwrap(), None);
            }
            other => panic!("unexpected resource-version race outcome: {other:?}"),
        }
    }

    #[test]
    fn empty_evidence_or_resource_revision_bindings_are_rejected() {
        let participant = ParticipantLedger::new(0, 1);
        let mut missing_commit_evidence = binding("idem-no-evidence", "digest-x", 0);
        missing_commit_evidence.commit_evidence_digest.clear();
        assert_eq!(
            participant.commit(&missing_commit_evidence),
            Err(LedgerError::InvalidBinding)
        );

        let mut missing_revision = binding("idem-no-resource-version", "digest-y", 0);
        missing_revision.resource_version.clear();
        assert_eq!(
            participant.commit(&missing_revision),
            Err(LedgerError::InvalidBinding)
        );
        assert_eq!(participant.remaining_budget().unwrap(), 1);
    }

    #[test]
    fn resource_revision_rejects_aba_even_when_version_label_is_reused() {
        let participant = ParticipantLedger::new_scoped_versioned(
            "tenant-a",
            "ledger/account-4",
            "orders-v32",
            3,
            1,
        );
        let mut stale = binding("idem-aba", "digest-aba", 3);
        stale.resource_version = "orders-v32".into();
        stale.resource_revision = 0;

        assert_eq!(
            participant.advance_resource_version("orders-v33").unwrap(),
            1
        );
        assert_eq!(
            participant.advance_resource_version("orders-v32").unwrap(),
            2
        );
        assert_eq!(participant.current_resource_version().unwrap(), "orders-v32");

        assert_eq!(
            participant.commit(&stale).unwrap(),
            CommitOutcome::DeniedStaleResourceRevision {
                presented_version: "orders-v32".into(),
                current_version: "orders-v32".into(),
                presented_revision: 0,
                current_revision: 2
            }
        );
        assert_eq!(participant.remaining_budget().unwrap(), 1);
    }

    #[test]
    fn resource_revision_exhaustion_does_not_change_resource_or_revision() {
        let participant = ParticipantLedger::new_scoped_versioned(
            "tenant-a",
            "ledger/account-4",
            "orders-v32",
            3,
            1,
        );
        {
            let mut state = participant.lock().unwrap();
            state.resource_revision = u64::MAX;
        }
        assert_eq!(
            participant.advance_resource_version("orders-v33"),
            Err(LedgerError::ResourceRevisionExhausted)
        );
        assert_eq!(participant.current_resource_version().unwrap(), "orders-v32");
        assert_eq!(participant.current_resource_revision().unwrap(), u64::MAX);
        assert_eq!(participant.remaining_budget().unwrap(), 1);
    }

    #[test]
    fn accepted_pending_request_cannot_be_retried_until_terminal() {
        let participant = ParticipantLedger::new(2, 1);
        let mut session = EffectSession::default();
        let original = binding("idem-pending", "digest-pending", 2);
        session.prepare(original.clone()).unwrap();
        assert_eq!(
            session.submit(&participant, DeliveryFault::AcceptedPending).unwrap(),
            Submission::AcceptedPending
        );
        assert_eq!(session.state(), AttemptState::OutcomeUnknown);
        assert_eq!(
            session.reconcile(&participant).unwrap(),
            Reconciliation::Pending
        );
        assert_eq!(
            session.prepare(binding("idem-replacement", "digest-new", 2)),
            Err(SessionError::UnresolvedOutcome)
        );

        let finalized = participant.finalize_pending("idem-pending").unwrap();
        assert!(matches!(finalized, CommitOutcome::Committed(_)));
        assert!(matches!(
            session.reconcile(&participant).unwrap(),
            Reconciliation::Committed(_)
        ));
        assert_eq!(session.state(), AttemptState::Committed);
        assert_eq!(participant.remaining_budget().unwrap(), 0);
    }

    #[test]
    fn pending_budget_is_reserved_across_distinct_relying_parties() {
        let participant =
            ParticipantLedger::new_scoped("tenant-a", "ledger/account-4", 4, 1);
        let mut first = binding("idem-pending-rp1", "digest-rp1", 4);
        first.relying_party = "rp-one".into();
        let mut second = binding("idem-pending-rp2", "digest-rp2", 4);
        second.relying_party = "rp-two".into();

        assert_eq!(
            participant.accept_pending(&first).unwrap(),
            CommitOutcome::PendingAccepted
        );
        assert_eq!(
            participant.accept_pending(&second).unwrap(),
            CommitOutcome::DeniedBudgetExhausted
        );
        assert_eq!(participant.remaining_budget().unwrap(), 1);
        assert!(matches!(
            participant.finalize_pending("idem-pending-rp1").unwrap(),
            CommitOutcome::Committed(_)
        ));
        assert_eq!(participant.remaining_budget().unwrap(), 0);
    }

    #[test]
    fn pending_request_revoked_before_finalize_is_terminally_aborted() {
        let participant = ParticipantLedger::new(7, 1);
        let request = binding("idem-pending-revoke", "digest-pending", 7);
        assert_eq!(
            participant.accept_pending(&request).unwrap(),
            CommitOutcome::PendingAccepted
        );
        participant.revoke_and_advance_epoch().unwrap();
        assert_eq!(
            participant.finalize_pending("idem-pending-revoke").unwrap(),
            CommitOutcome::DeniedStaleAuthorityEpoch {
                presented: 7,
                current: 8
            }
        );
        assert!(matches!(
            participant.reconcile_attempt(&request).unwrap(),
            ParticipantAttemptStatus::Aborted
        ));
        assert_eq!(participant.remaining_budget().unwrap(), 1);
        assert_eq!(participant.reserved_budget().unwrap(), 0);
    }

    #[test]
    fn reserved_budget_blocks_immediate_commit_from_overspending() {
        let participant = ParticipantLedger::new(3, 1);
        let pending = binding("idem-reserve", "digest-reserve", 3);
        let immediate = binding("idem-immediate", "digest-immediate", 3);
        assert_eq!(
            participant.accept_pending(&pending).unwrap(),
            CommitOutcome::PendingAccepted
        );
        assert_eq!(
            participant.commit(&immediate).unwrap(),
            CommitOutcome::DeniedBudgetExhausted
        );
        assert_eq!(participant.reserved_budget().unwrap(), 1);
        assert_eq!(participant.remaining_budget().unwrap(), 1);
        participant.abort_pending("idem-reserve").unwrap();
        assert_eq!(participant.reserved_budget().unwrap(), 0);
        assert!(matches!(
            participant.commit(&immediate).unwrap(),
            CommitOutcome::Committed(_)
        ));
    }

    #[test]
    fn pending_resource_revision_change_aborts_and_releases_reservation() {
        let participant = ParticipantLedger::new_scoped_versioned(
            "tenant-a",
            "ledger/account-4",
            "orders-v32",
            4,
            1,
        );
        let mut request = binding("idem-pending-stale-resource", "digest-pending", 4);
        request.resource_version = "orders-v32".into();
        request.resource_revision = 0;

        assert_eq!(
            participant.accept_pending(&request).unwrap(),
            CommitOutcome::PendingAccepted
        );
        assert_eq!(participant.reserved_budget().unwrap(), 1);
        assert_eq!(
            participant.advance_resource_version("orders-v33").unwrap(),
            1
        );
        assert_eq!(
            participant.finalize_pending("idem-pending-stale-resource").unwrap(),
            CommitOutcome::DeniedStaleResourceRevision {
                presented_version: "orders-v32".into(),
                current_version: "orders-v33".into(),
                presented_revision: 0,
                current_revision: 1
            }
        );
        assert_eq!(participant.reserved_budget().unwrap(), 0);
        assert_eq!(participant.remaining_budget().unwrap(), 1);
        assert!(matches!(
            participant.reconcile_attempt(&request).unwrap(),
            ParticipantAttemptStatus::Aborted
        ));
    }

}
