//! Executable abstract model for recovering a journal against an independent checkpoint.
//!
//! This crate deliberately does not implement cryptography, disk I/O, TPM access, or a
//! remote anchor. Digests are opaque 32-byte values. The model checks state-machine rules:
//! a restored older journal cannot silently override a newer independent checkpoint.

pub type Digest = [u8; 32];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Checkpoint {
    pub sequence: u64,
    pub root: Digest,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedCheckpoint {
    pub sequence: u64,
    pub previous_root: Digest,
    pub next_root: Digest,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Journal {
    pub committed: Checkpoint,
    pub pending: Option<PreparedCheckpoint>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryAction {
    Aligned,
    DiscardedUnanchoredPrepare,
    FinalizedAnchoredPrepare,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReconcileError {
    AnchorBehindLocalJournal,
    AnchorAheadByMoreThanOne,
    RootMismatch,
    MissingPreparedCheckpoint,
    InvalidPreparedCheckpoint,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExpectedOutcome {
    Accept(RecoveryAction),
    Reject(ReconcileError),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Fixture {
    pub name: &'static str,
    pub journal: Journal,
    pub anchor: Checkpoint,
    pub expected: ExpectedOutcome,
}

/// A model-only interface contract. A production implementation must provide an atomic,
/// durable compare-and-advance operation backed by state outside the attacker-restorable
/// journal. This trait alone does not make any implementation trusted.
pub trait AtomicCheckpointAnchor {
    fn read_checkpoint(&self) -> Result<Checkpoint, AnchorError>;
    fn compare_and_advance(
        &mut self,
        expected: &Checkpoint,
        next: &Checkpoint,
    ) -> Result<(), AnchorError>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AnchorError {
    Unavailable,
    StaleExpectedCheckpoint,
    InvalidAdvance,
}

/// In-memory demonstration of the CAS contract. Never use this as a security anchor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryAnchor {
    checkpoint: Checkpoint,
}

impl MemoryAnchor {
    pub fn new(checkpoint: Checkpoint) -> Self {
        Self { checkpoint }
    }
}

impl AtomicCheckpointAnchor for MemoryAnchor {
    fn read_checkpoint(&self) -> Result<Checkpoint, AnchorError> {
        Ok(self.checkpoint.clone())
    }

    fn compare_and_advance(
        &mut self,
        expected: &Checkpoint,
        next: &Checkpoint,
    ) -> Result<(), AnchorError> {
        if &self.checkpoint != expected {
            return Err(AnchorError::StaleExpectedCheckpoint);
        }
        let next_sequence = expected
            .sequence
            .checked_add(1)
            .ok_or(AnchorError::InvalidAdvance)?;
        if next.sequence != next_sequence {
            return Err(AnchorError::InvalidAdvance);
        }
        self.checkpoint = next.clone();
        Ok(())
    }
}

/// Reconcile a local journal against an independently stored checkpoint.
///
/// Intended protocol ordering:
/// 1. Write a prepared record and durably sync it.
/// 2. Atomically compare-and-advance the external anchor from the old checkpoint to the
///    prepared checkpoint.
/// 3. Mark the journal checkpoint committed and durably sync it.
/// 4. Only then begin the privileged side effect.
///
/// A crash before step 2 can discard the unanchored prepare. A crash after step 2 can
/// finalize the matching prepare on recovery. If the old journal is restored after step 2,
/// it will not contain the prepared checkpoint and reconciliation fails closed. The
/// operation may be lost after a crash; a fresh authorization is safer than replay.
///
/// This is a specification model, not a production implementation or proof of durability.
pub fn reconcile(
    journal: &mut Journal,
    anchor: &Checkpoint,
) -> Result<RecoveryAction, ReconcileError> {
    if let Some(pending) = &journal.pending {
        let expected_sequence = journal
            .committed
            .sequence
            .checked_add(1)
            .ok_or(ReconcileError::InvalidPreparedCheckpoint)?;
        if pending.sequence != expected_sequence
            || pending.previous_root != journal.committed.root
        {
            return Err(ReconcileError::InvalidPreparedCheckpoint);
        }
    }

    if anchor.sequence < journal.committed.sequence {
        return Err(ReconcileError::AnchorBehindLocalJournal);
    }

    if anchor.sequence == journal.committed.sequence {
        if anchor.root != journal.committed.root {
            return Err(ReconcileError::RootMismatch);
        }
        if journal.pending.take().is_some() {
            return Ok(RecoveryAction::DiscardedUnanchoredPrepare);
        }
        return Ok(RecoveryAction::Aligned);
    }

    let expected_next = journal
        .committed
        .sequence
        .checked_add(1)
        .ok_or(ReconcileError::AnchorAheadByMoreThanOne)?;
    if anchor.sequence != expected_next {
        return Err(ReconcileError::AnchorAheadByMoreThanOne);
    }

    let pending = journal
        .pending
        .as_ref()
        .ok_or(ReconcileError::MissingPreparedCheckpoint)?;
    if pending.next_root != anchor.root {
        return Err(ReconcileError::RootMismatch);
    }

    journal.committed = Checkpoint {
        sequence: pending.sequence,
        root: pending.next_root,
    };
    journal.pending = None;
    Ok(RecoveryAction::FinalizedAnchoredPrepare)
}

fn checkpoint(sequence: u64, byte: u8) -> Checkpoint {
    Checkpoint {
        sequence,
        root: [byte; 32],
    }
}

fn prepared(sequence: u64, previous: u8, next: u8) -> PreparedCheckpoint {
    PreparedCheckpoint {
        sequence,
        previous_root: [previous; 32],
        next_root: [next; 32],
    }
}

/// Frozen deterministic positives and adversarial snapshots. The case names and expected
/// outcomes are a benchmark contract; changing expectations requires a protocol review.
pub fn fixture_matrix() -> Vec<Fixture> {
    let c4 = checkpoint(4, 4);
    let c5 = checkpoint(5, 5);
    let c6 = checkpoint(6, 6);
    let c7 = checkpoint(7, 7);
    let c4_fork = checkpoint(4, 9);
    let c5_fork = checkpoint(5, 9);
    let c_max = checkpoint(u64::MAX, 1);

    vec![
        Fixture {
            name: "positive_aligned_checkpoint",
            journal: Journal {
                committed: c4.clone(),
                pending: None,
            },
            anchor: c4.clone(),
            expected: ExpectedOutcome::Accept(RecoveryAction::Aligned),
        },
        Fixture {
            name: "positive_discard_unanchored_prepare",
            journal: Journal {
                committed: c4.clone(),
                pending: Some(prepared(5, 4, 5)),
            },
            anchor: c4.clone(),
            expected: ExpectedOutcome::Accept(RecoveryAction::DiscardedUnanchoredPrepare),
        },
        Fixture {
            name: "positive_finalize_anchor_confirmed_prepare",
            journal: Journal {
                committed: c4.clone(),
                pending: Some(prepared(5, 4, 5)),
            },
            anchor: c5.clone(),
            expected: ExpectedOutcome::Accept(RecoveryAction::FinalizedAnchoredPrepare),
        },
        Fixture {
            name: "attack_restore_snapshot_several_checkpoints_old",
            journal: Journal {
                committed: c4.clone(),
                pending: None,
            },
            anchor: c7,
            expected: ExpectedOutcome::Reject(ReconcileError::AnchorAheadByMoreThanOne),
        },
        Fixture {
            name: "attack_restore_previous_complete_snapshot",
            journal: Journal {
                committed: c4.clone(),
                pending: None,
            },
            anchor: c5.clone(),
            expected: ExpectedOutcome::Reject(ReconcileError::MissingPreparedCheckpoint),
        },
        Fixture {
            name: "attack_local_journal_ahead_of_anchor",
            journal: Journal {
                committed: c5.clone(),
                pending: None,
            },
            anchor: c4.clone(),
            expected: ExpectedOutcome::Reject(ReconcileError::AnchorBehindLocalJournal),
        },
        Fixture {
            name: "attack_same_sequence_divergent_root",
            journal: Journal {
                committed: c4_fork,
                pending: None,
            },
            anchor: c4.clone(),
            expected: ExpectedOutcome::Reject(ReconcileError::RootMismatch),
        },
        Fixture {
            name: "attack_prepared_previous_root_fork",
            journal: Journal {
                committed: c4.clone(),
                pending: Some(prepared(5, 9, 5)),
            },
            anchor: c4.clone(),
            expected: ExpectedOutcome::Reject(ReconcileError::InvalidPreparedCheckpoint),
        },
        Fixture {
            name: "attack_prepared_sequence_skip",
            journal: Journal {
                committed: c4.clone(),
                pending: Some(prepared(6, 4, 6)),
            },
            anchor: c4.clone(),
            expected: ExpectedOutcome::Reject(ReconcileError::InvalidPreparedCheckpoint),
        },
        Fixture {
            name: "attack_anchor_root_not_pending_root",
            journal: Journal {
                committed: c4.clone(),
                pending: Some(prepared(5, 4, 6)),
            },
            anchor: c5_fork,
            expected: ExpectedOutcome::Reject(ReconcileError::RootMismatch),
        },
        Fixture {
            name: "attack_anchor_skips_pending_checkpoint",
            journal: Journal {
                committed: c4.clone(),
                pending: Some(prepared(5, 4, 5)),
            },
            anchor: c6,
            expected: ExpectedOutcome::Reject(ReconcileError::AnchorAheadByMoreThanOne),
        },
        Fixture {
            name: "attack_sequence_overflow_pending_record",
            journal: Journal {
                committed: c_max.clone(),
                pending: Some(prepared(0, 1, 2)),
            },
            anchor: c_max,
            expected: ExpectedOutcome::Reject(ReconcileError::InvalidPreparedCheckpoint),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_matrix_matches_every_expected_outcome() {
        for fixture in fixture_matrix() {
            let mut journal = fixture.journal;
            let observed = reconcile(&mut journal, &fixture.anchor);
            match fixture.expected {
                ExpectedOutcome::Accept(action) => assert_eq!(
                    observed,
                    Ok(action),
                    "positive control failed: {}",
                    fixture.name
                ),
                ExpectedOutcome::Reject(error) => assert_eq!(
                    observed,
                    Err(error),
                    "attack was not rejected as expected: {}",
                    fixture.name
                ),
            }
        }
    }

    #[test]
    fn anchored_prepare_is_finalized_idempotently_after_recovery() {
        let mut journal = Journal {
            committed: Checkpoint {
                sequence: 4,
                root: [4; 32],
            },
            pending: Some(PreparedCheckpoint {
                sequence: 5,
                previous_root: [4; 32],
                next_root: [5; 32],
            }),
        };
        let anchor = Checkpoint {
            sequence: 5,
            root: [5; 32],
        };

        assert_eq!(
            reconcile(&mut journal, &anchor),
            Ok(RecoveryAction::FinalizedAnchoredPrepare)
        );
        assert_eq!(journal.committed, anchor);
        assert_eq!(journal.pending, None);
        assert_eq!(reconcile(&mut journal, &anchor), Ok(RecoveryAction::Aligned));
    }

    #[test]
    fn compare_and_advance_rejects_stale_writer() {
        let initial = Checkpoint {
            sequence: 12,
            root: [12; 32],
        };
        let next = Checkpoint {
            sequence: 13,
            root: [13; 32],
        };
        let stale_next = Checkpoint {
            sequence: 13,
            root: [99; 32],
        };
        let mut anchor = MemoryAnchor::new(initial.clone());

        assert_eq!(
            anchor.compare_and_advance(&initial, &next),
            Ok(())
        );
        assert_eq!(
            anchor.compare_and_advance(&initial, &stale_next),
            Err(AnchorError::StaleExpectedCheckpoint)
        );
        assert_eq!(anchor.read_checkpoint(), Ok(next));
    }

    #[test]
    fn compare_and_advance_requires_exactly_one_sequence_step() {
        let initial = Checkpoint {
            sequence: 12,
            root: [12; 32],
        };
        let invalid = Checkpoint {
            sequence: 14,
            root: [14; 32],
        };
        let mut anchor = MemoryAnchor::new(initial.clone());

        assert_eq!(
            anchor.compare_and_advance(&initial, &invalid),
            Err(AnchorError::InvalidAdvance)
        );
        assert_eq!(anchor.read_checkpoint(), Ok(initial));
    }
}
