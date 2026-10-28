//! Bounded-storage idempotency via anchored key-epoch compaction.
//!
//! Permanent tombstones are safe but unbounded. This state-machine model explores a
//! bounded alternative: scope idempotency keys by a monotonic namespace epoch, advance
//! that epoch in an independent rollback-protected anchor only after every receipt and
//! tombstone from the old epoch is beyond its retention horizon, then prune old records.
//! Requests from retired epochs are rejected before lookup or effect dispatch.
//!
//! This is an abstract, in-memory reference model. Anchor flags represent adapter evidence;
//! they do not implement real signatures, TPM/remote CAS, durable persistence, or storage.

use std::collections::HashMap;

use super::retention_model::{RetentionPolicy, RetentionError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecordKind {
    Receipt,
    Tombstone,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetainedKeyRecord {
    pub request_digest: String,
    pub retain_until_epoch: u64,
    pub kind: RecordKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PendingTransition {
    from_epoch: u64,
    to_epoch: u64,
    prepared_at_epoch: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalLedger {
    current_key_epoch: u64,
    pending_transition: Option<PendingTransition>,
    records: HashMap<(u64, String), RetainedKeyRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndependentAnchor {
    key_epoch: u64,
    independently_protected: bool,
    authenticated: bool,
    rollback_protected: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryAction {
    Aligned,
    DiscardedUnanchoredTransition,
    FinalizedAnchoredTransition,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EpochError {
    UntrustedAnchor,
    TransitionPending,
    AnchorNotAligned,
    InvalidEpochAdvance,
    ActiveRetentionWindow,
    EpochRetired,
    FutureEpoch,
    InvalidKeyRecord,
    KeyAlreadyRecorded,
    MissingPreparedTransition,
    AnchorBehindLocal,
    AnchorAdvancedTooFar,
    InvalidPreparedTransition,
    InvalidRetentionPolicy,
    InvalidAuthorizationWindow,
    AuthorizationLifetimeExceeded,
    RetentionHorizonOverflow,
}

pub struct EpochCompactionModel {
    local: LocalLedger,
    anchor: IndependentAnchor,
    retention_policy: RetentionPolicy,
}

impl EpochCompactionModel {
    pub fn new(
        initial_epoch: u64,
        anchor_is_trusted: bool,
        retention_policy: RetentionPolicy,
    ) -> Result<Self, EpochError> {
        retention_policy
            .validate()
            .map_err(|_| EpochError::InvalidRetentionPolicy)?;
        Ok(Self {
            local: LocalLedger {
                current_key_epoch: initial_epoch,
                pending_transition: None,
                records: HashMap::new(),
            },
            anchor: IndependentAnchor {
                key_epoch: initial_epoch,
                independently_protected: anchor_is_trusted,
                authenticated: anchor_is_trusted,
                rollback_protected: anchor_is_trusted,
            },
            retention_policy,
        })
    }

    fn validate_anchor(&self) -> Result<(), EpochError> {
        if self.anchor.independently_protected
            && self.anchor.authenticated
            && self.anchor.rollback_protected
        {
            Ok(())
        } else {
            Err(EpochError::UntrustedAnchor)
        }
    }

    /// Record retained receipt/tombstone identity in the current namespace.
    ///
    /// The caller supplies the signed authorization issue/expiry epochs, not a retention
    /// deadline. The model derives the deadline from its validated policy so a caller
    /// cannot shorten retention to force unsafe compaction. This method is called after
    /// the effect has already been authorized; it does not independently dispatch effects.
    pub fn record_key(
        &mut self,
        key_epoch: u64,
        key: &str,
        request_digest: &str,
        issued_epoch: u64,
        expires_epoch: u64,
        kind: RecordKind,
    ) -> Result<(), EpochError> {
        self.validate_anchor()?;
        if self.local.pending_transition.is_some() {
            return Err(EpochError::TransitionPending);
        }
        if self.anchor.key_epoch != self.local.current_key_epoch {
            return Err(EpochError::AnchorNotAligned);
        }
        if key_epoch < self.local.current_key_epoch {
            return Err(EpochError::EpochRetired);
        }
        if key_epoch > self.local.current_key_epoch {
            return Err(EpochError::FutureEpoch);
        }
        if key.is_empty() || !is_lower_hex_digest(request_digest) {
            return Err(EpochError::InvalidKeyRecord);
        }
        if expires_epoch <= issued_epoch {
            return Err(EpochError::InvalidAuthorizationWindow);
        }
        let authorization_lifetime = expires_epoch - issued_epoch;
        if authorization_lifetime > self.retention_policy.max_authorization_lifetime {
            return Err(EpochError::AuthorizationLifetimeExceeded);
        }
        let retain_until_epoch = self
            .retention_policy
            .required_retention_until(expires_epoch)
            .map_err(|error| match error {
                RetentionError::EpochOverflow => EpochError::RetentionHorizonOverflow,
                _ => EpochError::InvalidRetentionPolicy,
            })?;
        let identity = (key_epoch, key.to_string());
        match self.local.records.entry(identity) {
            std::collections::hash_map::Entry::Occupied(_) => {
                Err(EpochError::KeyAlreadyRecorded)
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(RetainedKeyRecord {
                    request_digest: request_digest.to_string(),
                    retain_until_epoch,
                    kind,
                });
                Ok(())
            }
        }
    }

    /// Prepare a namespace epoch transition without deleting any old keys.
    ///
    /// Every key record in the active namespace must have strictly passed its derived
    /// retention horizon. The external anchor remains the authority; the local prepare
    /// merely makes recovery possible if the process stops between steps.
    pub fn prepare_compaction(
        &mut self,
        next_epoch: u64,
        now_epoch: u64,
    ) -> Result<(), EpochError> {
        self.validate_anchor()?;
        if self.local.pending_transition.is_some() {
            return Err(EpochError::TransitionPending);
        }
        if self.anchor.key_epoch != self.local.current_key_epoch {
            return Err(EpochError::AnchorNotAligned);
        }
        let expected_next = self
            .local
            .current_key_epoch
            .checked_add(1)
            .ok_or(EpochError::InvalidEpochAdvance)?;
        if next_epoch != expected_next {
            return Err(EpochError::InvalidEpochAdvance);
        }
        if self
            .local
            .records
            .iter()
            .any(|((epoch, _), record)| {
                *epoch == self.local.current_key_epoch
                    && record.retain_until_epoch >= now_epoch
            })
        {
            return Err(EpochError::ActiveRetentionWindow);
        }
        self.local.pending_transition = Some(PendingTransition {
            from_epoch: self.local.current_key_epoch,
            to_epoch: next_epoch,
            prepared_at_epoch: now_epoch,
        });
        Ok(())
    }

    /// Atomically advance the abstract independent anchor. A real implementation must
    /// supply durable compare-and-advance semantics outside the journal snapshot boundary.
    pub fn advance_anchor(&mut self) -> Result<(), EpochError> {
        self.validate_anchor()?;
        let pending = self
            .local
            .pending_transition
            .as_ref()
            .ok_or(EpochError::MissingPreparedTransition)?;
        let expected_to_epoch = pending
            .from_epoch
            .checked_add(1)
            .ok_or(EpochError::InvalidEpochAdvance)?;
        if pending.from_epoch != self.anchor.key_epoch
            || pending.to_epoch != expected_to_epoch
        {
            return Err(EpochError::InvalidPreparedTransition);
        }
        self.anchor.key_epoch = pending.to_epoch;
        Ok(())
    }

    /// Commit locally after anchor advancement, then discard all entries from retired
    /// epochs. This is the only normal path that removes receipt/tombstone records.
    pub fn commit_compaction(&mut self) -> Result<(), EpochError> {
        self.validate_anchor()?;
        let pending = self
            .local
            .pending_transition
            .clone()
            .ok_or(EpochError::MissingPreparedTransition)?;
        let expected_to_epoch = pending
            .from_epoch
            .checked_add(1)
            .ok_or(EpochError::InvalidEpochAdvance)?;
        if pending.from_epoch != self.local.current_key_epoch
            || pending.to_epoch != self.anchor.key_epoch
            || pending.to_epoch != expected_to_epoch
        {
            return Err(EpochError::InvalidPreparedTransition);
        }
        self.local.current_key_epoch = pending.to_epoch;
        self.local.pending_transition = None;
        let minimum_epoch = self.local.current_key_epoch;
        self.local
            .records
            .retain(|(epoch, _), _| *epoch >= minimum_epoch);
        Ok(())
    }

    /// Recover a crash around prepare -> anchor CAS -> local commit.
    pub fn recover(&mut self) -> Result<RecoveryAction, EpochError> {
        self.validate_anchor()?;
        if self.local.current_key_epoch == self.anchor.key_epoch {
            if let Some(pending) = &self.local.pending_transition {
                if pending.from_epoch != self.local.current_key_epoch
                    || pending.to_epoch
                        != pending.from_epoch.checked_add(1).ok_or(EpochError::InvalidEpochAdvance)?
                {
                    return Err(EpochError::InvalidPreparedTransition);
                }
                // Anchor did not advance: the old namespace remains authoritative.
                self.local.pending_transition = None;
                return Ok(RecoveryAction::DiscardedUnanchoredTransition);
            }
            return Ok(RecoveryAction::Aligned);
        }

        if self.anchor.key_epoch < self.local.current_key_epoch {
            return Err(EpochError::AnchorBehindLocal);
        }
        let next_local = self
            .local
            .current_key_epoch
            .checked_add(1)
            .ok_or(EpochError::AnchorAdvancedTooFar)?;
        if self.anchor.key_epoch != next_local {
            return Err(EpochError::AnchorAdvancedTooFar);
        }

        let pending = self
            .local
            .pending_transition
            .clone()
            .ok_or(EpochError::MissingPreparedTransition)?;
        if pending.from_epoch != self.local.current_key_epoch
            || pending.to_epoch != self.anchor.key_epoch
        {
            return Err(EpochError::InvalidPreparedTransition);
        }
        self.commit_compaction()?;
        Ok(RecoveryAction::FinalizedAnchoredTransition)
    }

    pub fn restore_local_snapshot(&mut self, snapshot: LocalLedger) {
        self.local = snapshot;
    }

    pub fn local_snapshot(&self) -> LocalLedger {
        self.local.clone()
    }

    pub fn current_key_epoch(&self) -> u64 {
        self.local.current_key_epoch
    }

    pub fn anchor_key_epoch(&self) -> u64 {
        self.anchor.key_epoch
    }

    pub fn retained_record_count(&self) -> usize {
        self.local.records.len()
    }

    pub fn pending_transition(&self) -> bool {
        self.local.pending_transition.is_some()
    }

    pub fn prepared_at_epoch(&self) -> Option<u64> {
        self.local
            .pending_transition
            .as_ref()
            .map(|pending| pending.prepared_at_epoch)
    }
}

fn is_lower_hex_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> RetentionPolicy {
        RetentionPolicy {
            max_authorization_lifetime: 10,
            max_retry_horizon: 20,
            epoch_skew_margin: 5,
        }
    }

    fn digest(byte: u8) -> String {
        format!("{byte:02x}").repeat(32)
    }

    fn model() -> EpochCompactionModel {
        EpochCompactionModel::new(4, true, policy()).unwrap()
    }

    fn add_old_records(model: &mut EpochCompactionModel) {
        model
            .record_key(4, "receipt-001", &digest(1), 10, 20, RecordKind::Receipt)
            .unwrap();
        model
            .record_key(4, "tombstone-002", &digest(2), 10, 20, RecordKind::Tombstone)
            .unwrap();
    }

    #[test]
    fn old_epoch_records_can_be_pruned_after_anchored_advance() {
        let mut model = model();
        add_old_records(&mut model);
        assert_eq!(model.retained_record_count(), 2);
        model.prepare_compaction(5, 46).unwrap();
        model.advance_anchor().unwrap();
        model.commit_compaction().unwrap();
        assert_eq!(model.current_key_epoch(), 5);
        assert_eq!(model.anchor_key_epoch(), 5);
        assert_eq!(model.retained_record_count(), 0);
        assert_eq!(
            model.record_key(4, "receipt-003", &digest(3), 60, RecordKind::Receipt),
            Err(EpochError::EpochRetired)
        );
        assert!(model.record_key(5, "receipt-004", &digest(4), 60, RecordKind::Receipt).is_ok());
    }

    #[test]
    fn compaction_is_blocked_until_every_retention_horizon_has_passed() {
        let mut model = model();
        model
            .record_key(4, "long-receipt", &digest(1), 50, RecordKind::Receipt)
            .unwrap();
        assert_eq!(
            model.prepare_compaction(5, 46),
            Err(EpochError::ActiveRetentionWindow)
        );
        assert!(!model.pending_transition());
        assert_eq!(model.retained_record_count(), 1);
    }

    #[test]
    fn crash_before_anchor_advance_discards_prepare_and_keeps_old_keys() {
        let mut model = model();
        add_old_records(&mut model);
        model.prepare_compaction(5, 46).unwrap();
        assert_eq!(
            model.recover(),
            Ok(RecoveryAction::DiscardedUnanchoredTransition)
        );
        assert_eq!(model.current_key_epoch(), 4);
        assert_eq!(model.anchor_key_epoch(), 4);
        assert_eq!(model.retained_record_count(), 2);
        assert!(!model.pending_transition());
    }

    #[test]
    fn crash_after_anchor_advance_finalizes_and_prunes() {
        let mut model = model();
        add_old_records(&mut model);
        model.prepare_compaction(5, 46).unwrap();
        model.advance_anchor().unwrap();
        assert_eq!(
            model.recover(),
            Ok(RecoveryAction::FinalizedAnchoredTransition)
        );
        assert_eq!(model.current_key_epoch(), 5);
        assert_eq!(model.anchor_key_epoch(), 5);
        assert_eq!(model.retained_record_count(), 0);
    }

    #[test]
    fn restoring_old_local_snapshot_after_anchor_advance_fails_closed() {
        let mut model = model();
        add_old_records(&mut model);
        let snapshot = model.local_snapshot();
        model.prepare_compaction(5, 46).unwrap();
        model.advance_anchor().unwrap();
        model.restore_local_snapshot(snapshot);
        assert_eq!(
            model.recover(),
            Err(EpochError::MissingPreparedTransition)
        );
        assert_eq!(model.anchor_key_epoch(), 5);
        assert_eq!(model.current_key_epoch(), 4);
        assert_eq!(model.retained_record_count(), 2);
    }

    #[test]
    fn untrusted_anchor_cannot_open_or_compact_epoch() {
        let mut model = EpochCompactionModel::new(4, false);
        assert_eq!(
            model.record_key(4, "key", &digest(1), 2, RecordKind::Receipt),
            Err(EpochError::UntrustedAnchor)
        );
        assert_eq!(
            model.prepare_compaction(5, 3),
            Err(EpochError::UntrustedAnchor)
        );
    }

    #[test]
    fn epoch_skip_and_future_key_epoch_are_rejected() {
        let mut model = model();
        assert_eq!(
            model.record_key(5, "future", &digest(1), 20, RecordKind::Receipt),
            Err(EpochError::FutureEpoch)
        );
        assert_eq!(
            model.prepare_compaction(6, 46),
            Err(EpochError::InvalidEpochAdvance)
        );
    }

    #[test]
    fn duplicate_key_identity_is_not_reinserted_within_epoch() {
        let mut model = model();
        model
            .record_key(4, "key", &digest(1), 20, RecordKind::Receipt)
            .unwrap();
        assert_eq!(
            model.record_key(4, "key", &digest(1), 20, RecordKind::Receipt),
            Err(EpochError::KeyAlreadyRecorded)
        );
        assert_eq!(
            model.record_key(4, "key", &digest(2), 20, RecordKind::Receipt),
            Err(EpochError::KeyAlreadyRecorded)
        );
        assert_eq!(model.retained_record_count(), 1);
    }

    #[test]
    fn prepare_does_not_delete_records_before_anchor_advance() {
        let mut model = model();
        add_old_records(&mut model);
        model.prepare_compaction(5, 46).unwrap();
        assert_eq!(model.retained_record_count(), 2);
        assert_eq!(model.current_key_epoch(), 4);
        assert_eq!(model.anchor_key_epoch(), 4);
    }

    #[test]
    fn prepare_epoch_and_current_anchor_must_be_aligned() {
        let mut model = model();
        add_old_records(&mut model);
        model.prepare_compaction(5, 46).unwrap();
        model.advance_anchor().unwrap();
        // Simulate a process crash with an older local namespace view; a new prepare
        // must be blocked until recovery reconciles the pending anchored transition.
        assert_eq!(
            model.prepare_compaction(6, 47),
            Err(EpochError::TransitionPending)
        );
    }
}
