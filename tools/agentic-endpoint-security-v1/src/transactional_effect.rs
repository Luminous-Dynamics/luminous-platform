//! Typed reference model for target-side effect lifecycle and recovery.
//!
//! This module is a pure evaluator over supplied evidence. It does not dispatch
//! tools, coordinate transactions, or observe a live resource. The independent
//! target ledger remains the source of truth for committed effects.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Timebase {
    pub time_scale: String,
    pub clock_domain: String,
    pub clock_epoch: String,
    pub timebase_ref: String,
    pub verified: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimeInterval {
    pub lower_ns: u64,
    pub upper_ns: u64,
    pub timebase: Timebase,
}

impl TimeInterval {
    pub fn new(lower_ns: u64, upper_ns: u64, timebase: Timebase) -> Option<Self> {
        (lower_ns <= upper_ns).then_some(Self {
            lower_ns,
            upper_ns,
            timebase,
        })
    }

    pub fn relation_to(&self, other: &Self) -> TemporalRelation {
        if !self.timebase.verified
            || !other.timebase.verified
            || self.timebase != other.timebase
        {
            return TemporalRelation::Unqualified;
        }
        if self.upper_ns < other.lower_ns {
            TemporalRelation::Before
        } else if other.upper_ns < self.lower_ns {
            TemporalRelation::After
        } else {
            TemporalRelation::Overlap
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TemporalRelation {
    Before,
    After,
    Overlap,
    Unqualified,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorizationTiming {
    pub grant: TimeInterval,
    pub revocation: Option<TimeInterval>,
    pub target_commit: TimeInterval,
    /// The owner-defined point at which permission is evaluated.
    pub linearization_point: LinearizationPoint,
    pub owner_policy_ref: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinearizationPoint {
    TargetCommit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthorizationLinearization {
    NotEvaluated,
    GrantBeforeCommitNoRevocation,
    RevocationBeforeCommit,
    CommitBeforeRevocation,
    GrantOrderUnproven,
    RevocationCommitOverlap,
    TimebaseUnqualified,
}

impl AuthorizationTiming {
    pub fn evaluate(&self) -> AuthorizationLinearization {
        if self.linearization_point != LinearizationPoint::TargetCommit
            || self.owner_policy_ref.is_empty()
        {
            return AuthorizationLinearization::TimebaseUnqualified;
        }
        match self.grant.relation_to(&self.target_commit) {
            TemporalRelation::Before => {}
            TemporalRelation::After | TemporalRelation::Overlap | TemporalRelation::Unqualified => {
                return if self.grant.relation_to(&self.target_commit)
                    == TemporalRelation::Unqualified
                {
                    AuthorizationLinearization::TimebaseUnqualified
                } else {
                    AuthorizationLinearization::GrantOrderUnproven
                };
            }
        }
        let Some(revocation) = &self.revocation else {
            return AuthorizationLinearization::GrantBeforeCommitNoRevocation;
        };
        match revocation.relation_to(&self.target_commit) {
            TemporalRelation::Before => AuthorizationLinearization::RevocationBeforeCommit,
            TemporalRelation::After => AuthorizationLinearization::CommitBeforeRevocation,
            TemporalRelation::Overlap => AuthorizationLinearization::RevocationCommitOverlap,
            TemporalRelation::Unqualified => AuthorizationLinearization::TimebaseUnqualified,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationIdentity {
    pub semantic_operation_id: String,
    pub effect_id: String,
    pub argument_digest: String,
    pub expected_batch_items: BTreeSet<String>,
    pub checkpoint_restore_seen: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DispatchState {
    NotSent,
    Sent,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AcknowledgementState {
    Received,
    Lost,
    Pending,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReconciliationBeforeRetry {
    NotApplicable,
    Committed,
    NotCommitted,
    Partial,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DispatchAttempt {
    pub attempt_id: String,
    pub operation_id: String,
    pub idempotency_key: String,
    pub argument_digest: String,
    pub dispatch_state: DispatchState,
    pub acknowledgement: AcknowledgementState,
    pub retry_parent_attempt_id: Option<String>,
    pub reconciliation_before_retry: ReconciliationBeforeRetry,
    pub restored_from_checkpoint: bool,
    pub authority_generation: u64,
    pub grant_id: String,
    pub nonce_id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetEntryKind {
    Effect,
    Compensation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TargetLedgerEntry {
    pub commit_id: String,
    pub operation_id: String,
    pub semantic_operation_id: String,
    pub idempotency_key: String,
    pub argument_digest: String,
    pub effect_id: String,
    pub kind: TargetEntryKind,
    pub batch_item_id: Option<String>,
    pub compensates_for: Option<String>,
    pub grant_id: String,
    pub nonce_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TargetLedger {
    pub complete: bool,
    pub independent: bool,
    pub entries: Vec<TargetLedgerEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientLedger {
    pub complete: bool,
    pub known_commit_ids: BTreeSet<String>,
    pub evidence_sealed: bool,
    pub evidence_integrity: bool,
    pub completed_claim: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorityLedger {
    pub consumed_nonce_ids: BTreeSet<String>,
    pub maximum_consumed_generation: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReconciliationStatus {
    NotStarted,
    Committed,
    NotCommitted,
    Partial,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reconciliation {
    pub status: ReconciliationStatus,
    pub target_oracle_complete: bool,
    pub independent: bool,
    pub consistency_bound_elapsed: bool,
    pub in_flight_bound_known: bool,
    pub result_commit_ids: BTreeSet<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TargetContract {
    pub idempotency_keys_supported: bool,
    pub read_back_supported: bool,
    pub maximum_in_flight_bound_known: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LifecycleEvidence {
    pub operation: OperationIdentity,
    pub target_contract: TargetContract,
    pub attempts: Vec<DispatchAttempt>,
    pub target_ledger: TargetLedger,
    pub client_ledger: ClientLedger,
    pub authority_ledger: AuthorityLedger,
    pub reconciliation: Reconciliation,
    pub authorization_timing: Option<AuthorizationTiming>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReconciledState {
    Committed,
    NotCommitted,
    Partial,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FinalDisposition {
    QualifiedCommitted,
    QualifiedNotCommitted,
    QualifiedPartial,
    Indeterminate,
    Violation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Evaluation {
    pub duplicate_effect_count: usize,
    pub key_argument_conflict_count: usize,
    pub semantic_replay_count: usize,
    pub unsafe_retry_count: usize,
    pub authority_resurrection_count: usize,
    pub unmatched_target_commit_count: usize,
    pub orphaned_target_effect_count: usize,
    pub target_client_ledger_conflict_count: usize,
    pub compensation_link_violation_count: usize,
    pub false_completion_claim_count: usize,
    pub authorization_linearization_violation_count: usize,
    pub authorization_linearization: AuthorizationLinearization,
    pub reconciled_state: ReconciledState,
    pub disposition: FinalDisposition,
}

fn target_id_set(ledger: &TargetLedger) -> BTreeSet<String> {
    ledger.entries.iter().map(|entry| entry.commit_id.clone()).collect()
}

fn attempt_matches_entry(attempt: &DispatchAttempt, entry: &TargetLedgerEntry) -> bool {
    attempt.operation_id == entry.operation_id
        && attempt.idempotency_key == entry.idempotency_key
        && attempt.argument_digest == entry.argument_digest
        && attempt.grant_id == entry.grant_id
        && attempt.nonce_id == entry.nonce_id
}

fn has_full_batch(evidence: &LifecycleEvidence) -> bool {
    if evidence.operation.expected_batch_items.is_empty() {
        return true;
    }
    let observed: BTreeSet<String> = evidence
        .target_ledger
        .entries
        .iter()
        .filter(|entry| entry.kind == TargetEntryKind::Effect)
        .filter_map(|entry| entry.batch_item_id.clone())
        .collect();
    observed == evidence.operation.expected_batch_items
}

pub fn evaluate(evidence: &LifecycleEvidence) -> Evaluation {
    let entries = &evidence.target_ledger.entries;
    let effect_entries: Vec<&TargetLedgerEntry> = entries
        .iter()
        .filter(|entry| entry.kind == TargetEntryKind::Effect)
        .collect();
    let by_attempt: BTreeMap<&str, &DispatchAttempt> = evidence
        .attempts
        .iter()
        .map(|attempt| (attempt.attempt_id.as_str(), attempt))
        .collect();

    let mut key_arguments: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for attempt in &evidence.attempts {
        key_arguments
            .entry(attempt.idempotency_key.as_str())
            .or_default()
            .insert(attempt.argument_digest.as_str());
    }
    let key_argument_conflict_count = key_arguments
        .values()
        .filter(|arguments| arguments.len() > 1)
        .count();

    // Duplicate detection is independent of the idempotency key. A new key
    // cannot turn a repeated semantic operation/item into a new target effect.
    let mut semantic_units: BTreeMap<(String, Option<String>, String), usize> = BTreeMap::new();
    let mut semantic_operations: BTreeMap<&str, Vec<&TargetLedgerEntry>> = BTreeMap::new();
    for entry in &effect_entries {
        let unit = (
            entry.semantic_operation_id.clone(),
            entry.batch_item_id.clone(),
            entry.effect_id.clone(),
        );
        *semantic_units.entry(unit).or_default() += 1;
        semantic_operations
            .entry(entry.semantic_operation_id.as_str())
            .or_default()
            .push(entry);
    }
    let duplicate_effect_count = semantic_units
        .values()
        .map(|count| count.saturating_sub(1))
        .sum();
    let semantic_replay_count = if evidence.operation.checkpoint_restore_seen {
        semantic_operations
            .values()
            .map(|group| {
                let operation_ids: BTreeSet<&str> =
                    group.iter().map(|entry| entry.operation_id.as_str()).collect();
                if operation_ids.len() > 1 {
                    group.len().saturating_sub(1)
                } else {
                    0
                }
            })
            .sum()
    } else {
        0
    };

    let mut unsafe_retry_count = 0;
    let mut authority_resurrection_count = 0;
    for attempt in &evidence.attempts {
        if let Some(parent_id) = attempt.retry_parent_attempt_id.as_deref() {
            match by_attempt.get(parent_id) {
                None => unsafe_retry_count += 1,
                Some(parent) => {
                    let unresolved = parent.dispatch_state == DispatchState::Unknown
                        || matches!(
                            parent.acknowledgement,
                            AcknowledgementState::Lost
                                | AcknowledgementState::Pending
                                | AcknowledgementState::Unknown
                        );
                    let same_key_same_arguments = evidence.target_contract.idempotency_keys_supported
                        && attempt.idempotency_key == parent.idempotency_key
                        && attempt.argument_digest == parent.argument_digest;
                    let reconciled = matches!(
                        attempt.reconciliation_before_retry,
                        ReconciliationBeforeRetry::Committed
                            | ReconciliationBeforeRetry::NotCommitted
                            | ReconciliationBeforeRetry::Partial
                    );
                    if unresolved && !same_key_same_arguments && !reconciled {
                        unsafe_retry_count += 1;
                    }
                }
            }
        }
        if evidence
            .authority_ledger
            .consumed_nonce_ids
            .contains(&attempt.nonce_id)
            || (attempt.restored_from_checkpoint
                && attempt.authority_generation
                    < evidence.authority_ledger.maximum_consumed_generation)
        {
            authority_resurrection_count += 1;
        }
    }

    let target_ids = target_id_set(&evidence.target_ledger);
    let orphaned_target_effect_count = entries
        .iter()
        .filter(|entry| !evidence.client_ledger.known_commit_ids.contains(&entry.commit_id))
        .count();
    let mut target_client_ledger_conflict_count = evidence
        .client_ledger
        .known_commit_ids
        .iter()
        .filter(|commit_id| !target_ids.contains(*commit_id))
        .count();

    let mut unmatched_target_commit_count = 0;
    for entry in entries {
        if !evidence.attempts.iter().any(|attempt| attempt_matches_entry(attempt, entry)) {
            unmatched_target_commit_count += 1;
        }
        if entry.kind == TargetEntryKind::Effect
            && (entry.effect_id != evidence.operation.effect_id
                || entry.semantic_operation_id != evidence.operation.semantic_operation_id)
        {
            unmatched_target_commit_count += 1;
        }
    }

    let by_commit: BTreeMap<&str, &TargetLedgerEntry> = entries
        .iter()
        .map(|entry| (entry.commit_id.as_str(), entry))
        .collect();
    let mut compensation_link_violation_count = 0;
    for entry in entries.iter().filter(|entry| entry.kind == TargetEntryKind::Compensation) {
        match entry.compensates_for.as_deref().and_then(|id| by_commit.get(id).copied()) {
            None => compensation_link_violation_count += 1,
            Some(original) => {
                if original.kind != TargetEntryKind::Effect
                    || entry.operation_id == original.operation_id
                    || entry.semantic_operation_id == original.semantic_operation_id
                    || entry.effect_id == original.effect_id
                    || entry.idempotency_key == original.idempotency_key
                    || entry.grant_id == original.grant_id
                    || entry.nonce_id == original.nonce_id
                {
                    compensation_link_violation_count += 1;
                }
            }
        }
    }

    let mut reconciled_state = ReconciledState::Unknown;
    let oracle_qualified = evidence.target_ledger.complete
        && evidence.target_ledger.independent
        && evidence.reconciliation.target_oracle_complete
        && evidence.reconciliation.independent;
    if oracle_qualified {
        let result_ids = &evidence.reconciliation.result_commit_ids;
        match evidence.reconciliation.status {
            ReconciliationStatus::Committed
                if has_full_batch(evidence)
                    && result_ids.len() == entries.len()
                    && result_ids == &target_ids =>
            {
                reconciled_state = ReconciledState::Committed;
            }
            ReconciliationStatus::NotCommitted
                if entries.is_empty()
                    && evidence.reconciliation.consistency_bound_elapsed
                    && evidence.reconciliation.in_flight_bound_known
                    && evidence.target_contract.read_back_supported =>
            {
                reconciled_state = ReconciledState::NotCommitted;
            }
            ReconciliationStatus::Partial
                if !evidence.operation.expected_batch_items.is_empty()
                    && !effect_entries.is_empty()
                    && effect_entries.len() < evidence.operation.expected_batch_items.len()
                    && result_ids.len() == entries.len()
                    && result_ids == &target_ids
                    && evidence.reconciliation.consistency_bound_elapsed
                    && evidence.reconciliation.in_flight_bound_known =>
            {
                reconciled_state = ReconciledState::Partial;
            }
            ReconciliationStatus::NotStarted
                if entries.is_empty()
                    && evidence.attempts.iter().all(|attempt| {
                        attempt.dispatch_state == DispatchState::NotSent
                    }) =>
            {
                reconciled_state = ReconciledState::NotCommitted;
            }
            _ => {}
        }
    }

    if matches!(
        evidence.reconciliation.status,
        ReconciliationStatus::Committed
            | ReconciliationStatus::NotCommitted
            | ReconciliationStatus::Partial
    ) && (evidence.reconciliation.result_commit_ids.len() != target_ids.len()
        || !evidence
            .reconciliation
            .result_commit_ids
            .is_subset(&target_ids))
    {
        target_client_ledger_conflict_count += 1;
    }
    if evidence.reconciliation.status == ReconciliationStatus::Committed
        && !has_full_batch(evidence)
    {
        target_client_ledger_conflict_count += 1;
    }
    if evidence.reconciliation.status == ReconciliationStatus::Partial
        && (evidence.operation.expected_batch_items.is_empty()
            || effect_entries.is_empty()
            || effect_entries.len() >= evidence.operation.expected_batch_items.len())
    {
        target_client_ledger_conflict_count += 1;
    }

    let authorization_linearization = evidence
        .authorization_timing
        .as_ref()
        .map_or(AuthorizationLinearization::NotEvaluated, AuthorizationTiming::evaluate);
    let authorization_linearization_violation_count =
        if authorization_linearization == AuthorizationLinearization::RevocationBeforeCommit
            && !effect_entries.is_empty()
        {
            1
        } else {
            0
        };
    let timing_uncertain = matches!(
        authorization_linearization,
        AuthorizationLinearization::TimebaseUnqualified
            | AuthorizationLinearization::GrantOrderUnproven
            | AuthorizationLinearization::RevocationCommitOverlap
    );

    let false_completion_claim_count = if evidence.client_ledger.completed_claim
        && (reconciled_state != ReconciledState::Committed
            || orphaned_target_effect_count > 0
            || target_client_ledger_conflict_count > 0)
    {
        1
    } else {
        0
    };

    let total_violations = duplicate_effect_count
        + key_argument_conflict_count
        + semantic_replay_count
        + unsafe_retry_count
        + authority_resurrection_count
        + unmatched_target_commit_count
        + target_client_ledger_conflict_count
        + compensation_link_violation_count
        + false_completion_claim_count
        + authorization_linearization_violation_count;

    let disposition = if total_violations > 0 || !evidence.client_ledger.evidence_integrity {
        FinalDisposition::Violation
    } else if timing_uncertain
        || reconciled_state == ReconciledState::Unknown
        || orphaned_target_effect_count > 0
        || !evidence.client_ledger.evidence_sealed
        || !evidence.client_ledger.complete
        || !evidence.target_ledger.complete
        || !evidence.target_ledger.independent
    {
        FinalDisposition::Indeterminate
    } else {
        match reconciled_state {
            ReconciledState::Committed => FinalDisposition::QualifiedCommitted,
            ReconciledState::NotCommitted => FinalDisposition::QualifiedNotCommitted,
            ReconciledState::Partial => FinalDisposition::QualifiedPartial,
            ReconciledState::Unknown => FinalDisposition::Indeterminate,
        }
    };

    Evaluation {
        duplicate_effect_count,
        key_argument_conflict_count,
        semantic_replay_count,
        unsafe_retry_count,
        authority_resurrection_count,
        unmatched_target_commit_count,
        orphaned_target_effect_count,
        target_client_ledger_conflict_count,
        compensation_link_violation_count,
        false_completion_claim_count,
        authorization_linearization_violation_count,
        authorization_linearization,
        reconciled_state,
        disposition,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timebase() -> Timebase {
        Timebase {
            time_scale: "TAI".into(),
            clock_domain: "clock:lab".into(),
            clock_epoch: "epoch:1".into(),
            timebase_ref: "timebase:qualified".into(),
            verified: true,
        }
    }

    fn interval(lower_ns: u64, upper_ns: u64) -> TimeInterval {
        TimeInterval::new(lower_ns, upper_ns, timebase()).expect("valid interval")
    }

    fn sample() -> LifecycleEvidence {
        let grant = "grant:write".to_string();
        let nonce = "nonce:1".to_string();
        let operation = OperationIdentity {
            semantic_operation_id: "intent:1".into(),
            effect_id: "effect:write".into(),
            argument_digest: "args:1".into(),
            expected_batch_items: BTreeSet::new(),
            checkpoint_restore_seen: false,
        };
        let attempt = DispatchAttempt {
            attempt_id: "attempt:1".into(),
            operation_id: "operation:1".into(),
            idempotency_key: "idempotency:1".into(),
            argument_digest: "args:1".into(),
            dispatch_state: DispatchState::Sent,
            acknowledgement: AcknowledgementState::Lost,
            retry_parent_attempt_id: None,
            reconciliation_before_retry: ReconciliationBeforeRetry::NotApplicable,
            restored_from_checkpoint: false,
            authority_generation: 1,
            grant_id: grant.clone(),
            nonce_id: nonce.clone(),
        };
        let entry = TargetLedgerEntry {
            commit_id: "commit:1".into(),
            operation_id: attempt.operation_id.clone(),
            semantic_operation_id: operation.semantic_operation_id.clone(),
            idempotency_key: attempt.idempotency_key.clone(),
            argument_digest: attempt.argument_digest.clone(),
            effect_id: operation.effect_id.clone(),
            kind: TargetEntryKind::Effect,
            batch_item_id: None,
            compensates_for: None,
            grant_id: grant,
            nonce_id: nonce,
        };
        LifecycleEvidence {
            operation,
            target_contract: TargetContract {
                idempotency_keys_supported: true,
                read_back_supported: true,
                maximum_in_flight_bound_known: true,
            },
            attempts: vec![attempt],
            target_ledger: TargetLedger {
                complete: true,
                independent: true,
                entries: vec![entry],
            },
            client_ledger: ClientLedger {
                complete: true,
                known_commit_ids: ["commit:1".into()].into_iter().collect(),
                evidence_sealed: true,
                evidence_integrity: true,
                completed_claim: true,
            },
            authority_ledger: AuthorityLedger {
                consumed_nonce_ids: BTreeSet::new(),
                maximum_consumed_generation: 1,
            },
            reconciliation: Reconciliation {
                status: ReconciliationStatus::Committed,
                target_oracle_complete: true,
                independent: true,
                consistency_bound_elapsed: true,
                in_flight_bound_known: true,
                result_commit_ids: ["commit:1".into()].into_iter().collect(),
            },
            authorization_timing: Some(AuthorizationTiming {
                grant: interval(1, 2),
                revocation: None,
                target_commit: interval(10, 11),
                linearization_point: LinearizationPoint::TargetCommit,
                owner_policy_ref: "policy:target-commit".into(),
            }),
        }
    }

    #[test]
    fn lost_ack_reconciles_against_target_ledger() {
        let result = evaluate(&sample());
        assert_eq!(result.reconciled_state, ReconciledState::Committed);
        assert_eq!(result.disposition, FinalDisposition::QualifiedCommitted);
    }

    #[test]
    fn unknown_result_is_not_no_effect() {
        let mut evidence = sample();
        evidence.target_ledger.entries.clear();
        evidence.client_ledger.known_commit_ids.clear();
        evidence.client_ledger.completed_claim = false;
        evidence.reconciliation.status = ReconciliationStatus::Unknown;
        evidence.reconciliation.result_commit_ids.clear();
        let result = evaluate(&evidence);
        assert_eq!(result.reconciled_state, ReconciledState::Unknown);
        assert_eq!(result.disposition, FinalDisposition::Indeterminate);
    }

    #[test]
    fn semantic_duplicate_is_detected_even_with_new_idempotency_key() {
        let mut evidence = sample();
        let mut duplicate = evidence.target_ledger.entries[0].clone();
        duplicate.commit_id = "commit:2".into();
        duplicate.operation_id = "operation:2".into();
        duplicate.idempotency_key = "idempotency:new".into();
        let mut attempt = evidence.attempts[0].clone();
        attempt.attempt_id = "attempt:2".into();
        attempt.operation_id = duplicate.operation_id.clone();
        attempt.idempotency_key = duplicate.idempotency_key.clone();
        evidence.attempts.push(attempt);
        evidence.target_ledger.entries.push(duplicate);
        evidence.client_ledger.known_commit_ids.insert("commit:2".into());
        evidence.reconciliation.result_commit_ids.insert("commit:2".into());
        let result = evaluate(&evidence);
        assert_eq!(result.duplicate_effect_count, 1);
        assert_eq!(result.disposition, FinalDisposition::Violation);
    }

    #[test]
    fn same_key_same_argument_redelivery_is_not_a_duplicate_commit() {
        let mut evidence = sample();
        let mut retry = evidence.attempts[0].clone();
        retry.attempt_id = "attempt:2".into();
        retry.retry_parent_attempt_id = Some("attempt:1".into());
        retry.reconciliation_before_retry = ReconciliationBeforeRetry::Unknown;
        evidence.attempts.push(retry);
        let result = evaluate(&evidence);
        assert_eq!(result.duplicate_effect_count, 0);
        assert_eq!(result.unsafe_retry_count, 0);
    }

    #[test]
    fn consumed_nonce_is_not_resurrected_after_restore() {
        let mut evidence = sample();
        evidence.authority_ledger.consumed_nonce_ids.insert("nonce:1".into());
        let result = evaluate(&evidence);
        assert_eq!(result.authority_resurrection_count, 1);
        assert_eq!(result.disposition, FinalDisposition::Violation);
    }

    #[test]
    fn revoked_before_commit_forbids_target_effect() {
        let mut evidence = sample();
        evidence.authorization_timing.as_mut().unwrap().revocation = Some(interval(5, 6));
        let result = evaluate(&evidence);
        assert_eq!(result.authorization_linearization, AuthorizationLinearization::RevocationBeforeCommit);
        assert_eq!(result.authorization_linearization_violation_count, 1);
        assert_eq!(result.disposition, FinalDisposition::Violation);
    }

    #[test]
    fn commit_before_revocation_is_valid_at_target_commit() {
        let mut evidence = sample();
        evidence.authorization_timing.as_mut().unwrap().revocation = Some(interval(12, 13));
        let result = evaluate(&evidence);
        assert_eq!(result.authorization_linearization, AuthorizationLinearization::CommitBeforeRevocation);
        assert_eq!(result.disposition, FinalDisposition::QualifiedCommitted);
    }

    #[test]
    fn target_commit_must_match_dispatch_authority_receipt() {
        let mut evidence = sample();
        evidence.target_ledger.entries[0].grant_id = "grant:substituted".into();
        let result = evaluate(&evidence);
        assert_eq!(result.unmatched_target_commit_count, 1);
        assert_eq!(result.disposition, FinalDisposition::Violation);
    }

    #[test]
    fn compensation_is_a_distinct_operation_with_separate_authority() {
        let mut evidence = sample();
        let original = evidence.target_ledger.entries[0].clone();
        let compensation = TargetLedgerEntry {
            commit_id: "commit:compensation".into(),
            operation_id: "operation:compensation".into(),
            semantic_operation_id: "intent:compensation".into(),
            idempotency_key: "idempotency:compensation".into(),
            argument_digest: "args:compensation".into(),
            effect_id: "effect:compensation".into(),
            kind: TargetEntryKind::Compensation,
            batch_item_id: None,
            compensates_for: Some(original.commit_id.clone()),
            grant_id: "grant:compensation".into(),
            nonce_id: "nonce:compensation".into(),
        };
        let compensation_attempt = DispatchAttempt {
            attempt_id: "attempt:compensation".into(),
            operation_id: compensation.operation_id.clone(),
            idempotency_key: compensation.idempotency_key.clone(),
            argument_digest: compensation.argument_digest.clone(),
            dispatch_state: DispatchState::Sent,
            acknowledgement: AcknowledgementState::Received,
            retry_parent_attempt_id: None,
            reconciliation_before_retry: ReconciliationBeforeRetry::NotApplicable,
            restored_from_checkpoint: false,
            authority_generation: 1,
            grant_id: compensation.grant_id.clone(),
            nonce_id: compensation.nonce_id.clone(),
        };
        evidence.attempts.push(compensation_attempt);
        evidence.target_ledger.entries.push(compensation.clone());
        evidence.client_ledger.known_commit_ids.insert(compensation.commit_id.clone());
        evidence.reconciliation.result_commit_ids.insert(compensation.commit_id);
        let result = evaluate(&evidence);
        assert_eq!(result.compensation_link_violation_count, 0);
    }

    #[test]
    fn overlapping_revocation_and_commit_remain_indeterminate() {
        let mut evidence = sample();
        evidence.authorization_timing.as_mut().unwrap().revocation = Some(interval(9, 12));
        let result = evaluate(&evidence);
        assert_eq!(result.authorization_linearization, AuthorizationLinearization::RevocationCommitOverlap);
        assert_eq!(result.disposition, FinalDisposition::Indeterminate);
    }
}
