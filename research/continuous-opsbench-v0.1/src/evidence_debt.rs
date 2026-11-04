//! Evaluator-owned time-integrated operational evidence debt.
//!
//! This module measures how long an effect, recovery claim or observer
//! obligation remains unresolved. It consumes evaluator-owned lifecycle
//! records; it does not infer participant state from an agent transcript and
//! does not create a distributed event journal.
//!
//! Superseded, not-observed, contradicted and observer-gap states remain debt
//! until a terminal confirmation or explicit resolution receipt closes them.
use crate::Digest;
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_ITEMS: usize = 100_000;
pub const MAX_EVENTS: usize = 1_000_000;
pub const MAX_ID_BYTES: usize = 512;
pub const MAX_REFERENCE_BYTES: usize = 1_024;
pub const MAX_REFERENCES_PER_EVENT: usize = 64;
pub const MAX_WEIGHT_MILLI: u32 = 1_000_000;

fn valid_text(value: &str, max_bytes: usize) -> bool {
    !value.trim().is_empty() && value.len() <= max_bytes && !value.chars().any(char::is_control)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceDebtKind {
    ParticipantEffect,
    RecoveryClaim,
    ObserverCoverage,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceDebtState {
    PendingReconciliation,
    ObserverGap,
    Contradicted,
    NotObserved,
    Superseded,
    Confirmed,
    ResolvedWithoutConfirmation,
}

impl EvidenceDebtState {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Confirmed | Self::ResolvedWithoutConfirmation)
    }

    pub fn is_unresolved(self) -> bool {
        !self.is_terminal()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceDebtContract {
    pub scenario_id: String,
    pub run_start_ms: u64,
    pub run_end_ms: u64,
    pub policy_digest: Digest,
    pub maximum_items: usize,
    pub maximum_events: usize,
}

impl EvidenceDebtContract {
    pub fn validate(&self) -> Result<(), EvidenceDebtError> {
        if !valid_text(&self.scenario_id, MAX_ID_BYTES) {
            return Err(EvidenceDebtError::InvalidScenarioId);
        }
        if self.run_end_ms <= self.run_start_ms {
            return Err(EvidenceDebtError::InvalidRunInterval);
        }
        if self.maximum_items == 0 || self.maximum_items > MAX_ITEMS {
            return Err(EvidenceDebtError::InvalidItemLimit);
        }
        if self.maximum_events == 0 || self.maximum_events > MAX_EVENTS {
            return Err(EvidenceDebtError::InvalidEventLimit);
        }
        Ok(())
    }
}

/// One evaluator-owned state transition for an operational claim.
///
/// Sequence is the immutable evaluator order, not a wall-clock sort key.
/// Event time is used only for integration. Weight is preregistered per
/// operation in fixed-point thousandths: 1_000 = 1x, 2_000 = 2x.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceDebtEvent {
    pub event_id: String,
    pub operation_id: String,
    pub sequence: u64,
    pub occurred_at_ms: u64,
    pub kind: EvidenceDebtKind,
    pub state: EvidenceDebtState,
    pub consequence_weight_milli: u32,
    pub evidence_refs: Vec<String>,
    pub resolution_ref: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct EvidenceDebtAggregate {
    pub unresolved_duration_ms: u64,
    pub weighted_exposure_milli_ms: u128,
    pub item_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenEvidenceDebtItem {
    pub operation_id: String,
    pub kind: EvidenceDebtKind,
    pub current_state: EvidenceDebtState,
    pub opened_at_ms: u64,
    pub unresolved_duration_ms: u64,
    pub consequence_weight_milli: u32,
    pub last_event_id: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceDebtReport {
    pub scenario_id: String,
    pub policy_digest: Digest,
    pub run_start_ms: u64,
    pub run_end_ms: u64,
    pub total_items: usize,
    pub confirmed_items: usize,
    pub resolved_without_confirmation_items: usize,
    pub unresolved_items_at_end: usize,
    pub peak_concurrent_unresolved_items: usize,
    /// Sum of unresolved item-time; overlapping unresolved items add.
    pub total_unresolved_duration_ms: u64,
    /// Exact sum(duration_ms * consequence_weight_milli), without floats.
    pub total_weighted_exposure_milli_ms: u128,
    pub unsubstantiated_recovery_claim_duration_ms: u64,
    pub unsubstantiated_recovery_claim_exposure_milli_ms: u128,
    pub longest_unresolved_item_ms: u64,
    pub by_kind: BTreeMap<EvidenceDebtKind, EvidenceDebtAggregate>,
    pub by_state: BTreeMap<EvidenceDebtState, u64>,
    pub open_items: Vec<OpenEvidenceDebtItem>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceDebtError {
    InvalidScenarioId,
    InvalidRunInterval,
    InvalidItemLimit,
    InvalidEventLimit,
    TooManyItems,
    TooManyEvents,
    InvalidEventId,
    InvalidOperationId,
    DuplicateEventId,
    DuplicateSequence,
    EventOutsideRun,
    EventSequenceTimeRegression,
    InvalidConsequenceWeight,
    TooManyEvidenceReferences,
    InvalidEvidenceReference,
    InvalidResolutionReference,
    FirstStateMustBePending,
    OperationKindChanged,
    OperationWeightChanged,
    EventAfterTerminalState,
    DuplicateStateTransition,
    ConfirmedWithoutEvidence,
    UnverifiedClosureWithoutReceipt,
    CounterOverflow,
}

#[derive(Clone, Debug)]
struct ItemState {
    kind: EvidenceDebtKind,
    current_state: EvidenceDebtState,
    opened_at_ms: u64,
    last_at_ms: u64,
    weight_milli: u32,
    last_event_id: String,
    evidence_refs: Vec<String>,
}

fn elapsed(start_ms: u64, end_ms: u64) -> Result<u64, EvidenceDebtError> {
    end_ms
        .checked_sub(start_ms)
        .ok_or(EvidenceDebtError::EventSequenceTimeRegression)
}

fn add_duration(total: &mut u64, value: u64) -> Result<(), EvidenceDebtError> {
    *total = total
        .checked_add(value)
        .ok_or(EvidenceDebtError::CounterOverflow)?;
    Ok(())
}

fn add_weighted(
    total: &mut u128,
    duration_ms: u64,
    weight_milli: u32,
) -> Result<(), EvidenceDebtError> {
    let value = u128::from(duration_ms)
        .checked_mul(u128::from(weight_milli))
        .ok_or(EvidenceDebtError::CounterOverflow)?;
    *total = total
        .checked_add(value)
        .ok_or(EvidenceDebtError::CounterOverflow)?;
    Ok(())
}

fn validate_event(
    event: &EvidenceDebtEvent,
    contract: &EvidenceDebtContract,
) -> Result<(), EvidenceDebtError> {
    if !valid_text(&event.event_id, MAX_ID_BYTES) {
        return Err(EvidenceDebtError::InvalidEventId);
    }
    if !valid_text(&event.operation_id, MAX_ID_BYTES) {
        return Err(EvidenceDebtError::InvalidOperationId);
    }
    if event.occurred_at_ms < contract.run_start_ms || event.occurred_at_ms > contract.run_end_ms {
        return Err(EvidenceDebtError::EventOutsideRun);
    }
    if event.consequence_weight_milli == 0 || event.consequence_weight_milli > MAX_WEIGHT_MILLI {
        return Err(EvidenceDebtError::InvalidConsequenceWeight);
    }
    if event.evidence_refs.len() > MAX_REFERENCES_PER_EVENT {
        return Err(EvidenceDebtError::TooManyEvidenceReferences);
    }
    if event
        .evidence_refs
        .iter()
        .any(|reference| !valid_text(reference, MAX_REFERENCE_BYTES))
    {
        return Err(EvidenceDebtError::InvalidEvidenceReference);
    }
    if event
        .resolution_ref
        .as_ref()
        .is_some_and(|reference| !valid_text(reference, MAX_REFERENCE_BYTES))
    {
        return Err(EvidenceDebtError::InvalidResolutionReference);
    }
    if event.state == EvidenceDebtState::Confirmed && event.evidence_refs.is_empty() {
        return Err(EvidenceDebtError::ConfirmedWithoutEvidence);
    }
    if event.state == EvidenceDebtState::ResolvedWithoutConfirmation
        && event.resolution_ref.is_none()
    {
        return Err(EvidenceDebtError::UnverifiedClosureWithoutReceipt);
    }
    if event.state.is_unresolved() && event.resolution_ref.is_some() {
        return Err(EvidenceDebtError::InvalidResolutionReference);
    }
    Ok(())
}

/// Integrate unresolved operational evidence exposure over the exact run
/// horizon. Each operation opens in PendingReconciliation and closes only
/// with Confirmed or ResolvedWithoutConfirmation. Unknown/unresolved final
/// states accrue through run_end_ms. No event, item or run is silently dropped.
pub fn evaluate_evidence_debt(
    contract: &EvidenceDebtContract,
    events: &[EvidenceDebtEvent],
) -> Result<EvidenceDebtReport, EvidenceDebtError> {
    contract.validate()?;
    if events.len() > contract.maximum_events {
        return Err(EvidenceDebtError::TooManyEvents);
    }

    let mut sorted = events.to_vec();
    sorted.sort_by_key(|event| event.sequence);

    let mut event_ids = BTreeSet::new();
    let mut sequences = BTreeSet::new();
    let mut items = BTreeMap::<String, ItemState>::new();
    let mut by_kind = BTreeMap::<EvidenceDebtKind, EvidenceDebtAggregate>::new();
    let mut by_state = BTreeMap::<EvidenceDebtState, u64>::new();
    let mut previous_global_time = None::<u64>;

    let mut total_unresolved_duration_ms = 0u64;
    let mut total_weighted_exposure_milli_ms = 0u128;
    let mut unsubstantiated_recovery_claim_duration_ms = 0u64;
    let mut unsubstantiated_recovery_claim_exposure_milli_ms = 0u128;
    let mut longest_unresolved_item_ms = 0u64;
    let mut peak_concurrent_unresolved_items = 0usize;
    let mut confirmed_items = 0usize;
    let mut resolved_without_confirmation_items = 0usize;
    let mut unresolved_count = 0usize;

    for event in &sorted {
        validate_event(event, contract)?;
        if !event_ids.insert(event.event_id.as_str()) {
            return Err(EvidenceDebtError::DuplicateEventId);
        }
        if !sequences.insert(event.sequence) {
            return Err(EvidenceDebtError::DuplicateSequence);
        }
        if previous_global_time.is_some_and(|previous| event.occurred_at_ms < previous) {
            return Err(EvidenceDebtError::EventSequenceTimeRegression);
        }
        previous_global_time = Some(event.occurred_at_ms);

        match items.get_mut(&event.operation_id) {
            None => {
                if event.state != EvidenceDebtState::PendingReconciliation {
                    return Err(EvidenceDebtError::FirstStateMustBePending);
                }
                if items.len() >= contract.maximum_items {
                    return Err(EvidenceDebtError::TooManyItems);
                }
                let aggregate = by_kind.entry(event.kind).or_default();
                aggregate.item_count = aggregate
                    .item_count
                    .checked_add(1)
                    .ok_or(EvidenceDebtError::CounterOverflow)?;
                items.insert(
                    event.operation_id.clone(),
                    ItemState {
                        kind: event.kind,
                        current_state: event.state,
                        opened_at_ms: event.occurred_at_ms,
                        last_at_ms: event.occurred_at_ms,
                        weight_milli: event.consequence_weight_milli,
                        last_event_id: event.event_id.clone(),
                        evidence_refs: event.evidence_refs.clone(),
                    },
                );
                unresolved_count = unresolved_count
                    .checked_add(1)
                    .ok_or(EvidenceDebtError::CounterOverflow)?;
                peak_concurrent_unresolved_items =
                    peak_concurrent_unresolved_items.max(unresolved_count);
            }
            Some(item) => {
                if item.current_state.is_terminal() {
                    return Err(EvidenceDebtError::EventAfterTerminalState);
                }
                if event.kind != item.kind {
                    return Err(EvidenceDebtError::OperationKindChanged);
                }
                if event.consequence_weight_milli != item.weight_milli {
                    return Err(EvidenceDebtError::OperationWeightChanged);
                }
                if event.state == item.current_state {
                    return Err(EvidenceDebtError::DuplicateStateTransition);
                }

                let duration = elapsed(item.last_at_ms, event.occurred_at_ms)?;
                add_duration(&mut total_unresolved_duration_ms, duration)?;
                add_weighted(
                    &mut total_weighted_exposure_milli_ms,
                    duration,
                    item.weight_milli,
                )?;
                add_duration(by_state.entry(item.current_state).or_default(), duration)?;

                let aggregate = by_kind
                    .get_mut(&item.kind)
                    .ok_or(EvidenceDebtError::CounterOverflow)?;
                add_duration(&mut aggregate.unresolved_duration_ms, duration)?;
                add_weighted(
                    &mut aggregate.weighted_exposure_milli_ms,
                    duration,
                    item.weight_milli,
                )?;

                if item.kind == EvidenceDebtKind::RecoveryClaim {
                    add_duration(&mut unsubstantiated_recovery_claim_duration_ms, duration)?;
                    add_weighted(
                        &mut unsubstantiated_recovery_claim_exposure_milli_ms,
                        duration,
                        item.weight_milli,
                    )?;
                }

                longest_unresolved_item_ms = longest_unresolved_item_ms
                    .max(elapsed(item.opened_at_ms, event.occurred_at_ms)?);

                item.current_state = event.state;
                item.last_at_ms = event.occurred_at_ms;
                item.last_event_id = event.event_id.clone();
                item.evidence_refs = event.evidence_refs.clone();

                if event.state == EvidenceDebtState::Confirmed {
                    confirmed_items = confirmed_items
                        .checked_add(1)
                        .ok_or(EvidenceDebtError::CounterOverflow)?;
                    unresolved_count = unresolved_count
                        .checked_sub(1)
                        .ok_or(EvidenceDebtError::CounterOverflow)?;
                } else if event.state == EvidenceDebtState::ResolvedWithoutConfirmation {
                    resolved_without_confirmation_items = resolved_without_confirmation_items
                        .checked_add(1)
                        .ok_or(EvidenceDebtError::CounterOverflow)?;
                    unresolved_count = unresolved_count
                        .checked_sub(1)
                        .ok_or(EvidenceDebtError::CounterOverflow)?;
                } else {
                    peak_concurrent_unresolved_items =
                        peak_concurrent_unresolved_items.max(unresolved_count);
                }
            }
        }
    }

    let mut open_items = Vec::new();
    for (operation_id, item) in &items {
        if item.current_state.is_terminal() {
            continue;
        }
        let duration = elapsed(item.last_at_ms, contract.run_end_ms)?;
        add_duration(&mut total_unresolved_duration_ms, duration)?;
        add_weighted(
            &mut total_weighted_exposure_milli_ms,
            duration,
            item.weight_milli,
        )?;
        add_duration(by_state.entry(item.current_state).or_default(), duration)?;

        let aggregate = by_kind
            .get_mut(&item.kind)
            .ok_or(EvidenceDebtError::CounterOverflow)?;
        add_duration(&mut aggregate.unresolved_duration_ms, duration)?;
        add_weighted(
            &mut aggregate.weighted_exposure_milli_ms,
            duration,
            item.weight_milli,
        )?;
        if item.kind == EvidenceDebtKind::RecoveryClaim {
            add_duration(&mut unsubstantiated_recovery_claim_duration_ms, duration)?;
            add_weighted(
                &mut unsubstantiated_recovery_claim_exposure_milli_ms,
                duration,
                item.weight_milli,
            )?;
        }

        let item_duration = elapsed(item.opened_at_ms, contract.run_end_ms)?;
        longest_unresolved_item_ms = longest_unresolved_item_ms.max(item_duration);
        open_items.push(OpenEvidenceDebtItem {
            operation_id: operation_id.clone(),
            kind: item.kind,
            current_state: item.current_state,
            opened_at_ms: item.opened_at_ms,
            unresolved_duration_ms: item_duration,
            consequence_weight_milli: item.weight_milli,
            last_event_id: item.last_event_id.clone(),
            evidence_refs: item.evidence_refs.clone(),
        });
    }

    open_items.sort_by(|left, right| left.operation_id.cmp(&right.operation_id));

    Ok(EvidenceDebtReport {
        scenario_id: contract.scenario_id.clone(),
        policy_digest: contract.policy_digest.clone(),
        run_start_ms: contract.run_start_ms,
        run_end_ms: contract.run_end_ms,
        total_items: items.len(),
        confirmed_items,
        resolved_without_confirmation_items,
        unresolved_items_at_end: open_items.len(),
        peak_concurrent_unresolved_items,
        total_unresolved_duration_ms,
        total_weighted_exposure_milli_ms,
        unsubstantiated_recovery_claim_duration_ms,
        unsubstantiated_recovery_claim_exposure_milli_ms,
        longest_unresolved_item_ms,
        by_kind,
        by_state,
        open_items,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contract() -> EvidenceDebtContract {
        EvidenceDebtContract {
            scenario_id: "continuous-run-001".to_owned(),
            run_start_ms: 0,
            run_end_ms: 1_000,
            policy_digest: Digest::from_byte(7),
            maximum_items: 100,
            maximum_events: 1_000,
        }
    }

    fn event(
        event_id: &str,
        operation_id: &str,
        sequence: u64,
        occurred_at_ms: u64,
        kind: EvidenceDebtKind,
        state: EvidenceDebtState,
        weight: u32,
    ) -> EvidenceDebtEvent {
        let terminal = state.is_terminal();
        EvidenceDebtEvent {
            event_id: event_id.to_owned(),
            operation_id: operation_id.to_owned(),
            sequence,
            occurred_at_ms,
            kind,
            state,
            consequence_weight_milli: weight,
            evidence_refs: if state == EvidenceDebtState::Confirmed {
                vec![format!("receipt:{operation_id}:confirmed")]
            } else {
                Vec::new()
            },
            resolution_ref: if terminal {
                Some(format!("receipt:{operation_id}:terminal"))
            } else {
                None
            },
        }
    }

    #[test]
    fn weighted_and_unweighted_exposure_integrates_exactly() {
        let events = vec![
            event(
                "e1",
                "effect-1",
                1,
                100,
                EvidenceDebtKind::ParticipantEffect,
                EvidenceDebtState::PendingReconciliation,
                2_000,
            ),
            event(
                "e2",
                "effect-1",
                2,
                150,
                EvidenceDebtKind::ParticipantEffect,
                EvidenceDebtState::ObserverGap,
                2_000,
            ),
            event(
                "e3",
                "effect-1",
                3,
                300,
                EvidenceDebtKind::ParticipantEffect,
                EvidenceDebtState::Confirmed,
                2_000,
            ),
        ];
        let report = evaluate_evidence_debt(&contract(), &events).expect("valid debt trace");
        assert_eq!(report.total_unresolved_duration_ms, 200);
        assert_eq!(report.total_weighted_exposure_milli_ms, 400_000);
        assert_eq!(report.confirmed_items, 1);
        assert_eq!(report.unresolved_items_at_end, 0);
        assert_eq!(
            report.by_state[&EvidenceDebtState::PendingReconciliation],
            50
        );
        assert_eq!(report.by_state[&EvidenceDebtState::ObserverGap], 150);
    }

    #[test]
    fn unresolved_items_accrue_through_the_run_horizon() {
        let events = vec![event(
            "e1",
            "recovery-1",
            1,
            100,
            EvidenceDebtKind::RecoveryClaim,
            EvidenceDebtState::PendingReconciliation,
            3_000,
        )];
        let report = evaluate_evidence_debt(&contract(), &events).expect("open debt");
        assert_eq!(report.unresolved_items_at_end, 1);
        assert_eq!(report.total_unresolved_duration_ms, 900);
        assert_eq!(report.total_weighted_exposure_milli_ms, 2_700_000);
        assert_eq!(report.unsubstantiated_recovery_claim_duration_ms, 900);
        assert_eq!(
            report.unsubstantiated_recovery_claim_exposure_milli_ms,
            2_700_000
        );
        assert_eq!(
            report.open_items[0].current_state,
            EvidenceDebtState::PendingReconciliation
        );
    }

    #[test]
    fn superseded_not_observed_and_contradicted_states_do_not_close_debt() {
        let events = vec![
            event(
                "e1",
                "effect-1",
                1,
                0,
                EvidenceDebtKind::ParticipantEffect,
                EvidenceDebtState::PendingReconciliation,
                1_000,
            ),
            event(
                "e2",
                "effect-1",
                2,
                100,
                EvidenceDebtKind::ParticipantEffect,
                EvidenceDebtState::Superseded,
                1_000,
            ),
            event(
                "e3",
                "effect-1",
                3,
                200,
                EvidenceDebtKind::ParticipantEffect,
                EvidenceDebtState::NotObserved,
                1_000,
            ),
            event(
                "e4",
                "effect-1",
                4,
                300,
                EvidenceDebtKind::ParticipantEffect,
                EvidenceDebtState::Contradicted,
                1_000,
            ),
        ];
        let report = evaluate_evidence_debt(&contract(), &events).expect("unresolved transitions");
        assert_eq!(report.unresolved_items_at_end, 1);
        assert_eq!(report.total_unresolved_duration_ms, 1_000);
        assert_eq!(
            report.by_state[&EvidenceDebtState::PendingReconciliation],
            100
        );
        assert_eq!(report.by_state[&EvidenceDebtState::Superseded], 100);
        assert_eq!(report.by_state[&EvidenceDebtState::NotObserved], 100);
        assert_eq!(report.by_state[&EvidenceDebtState::Contradicted], 700);
    }

    #[test]
    fn explicit_negative_resolution_closes_but_does_not_count_as_confirmation() {
        let events = vec![
            event(
                "e1",
                "attempt-1",
                1,
                10,
                EvidenceDebtKind::ParticipantEffect,
                EvidenceDebtState::PendingReconciliation,
                1_000,
            ),
            event(
                "e2",
                "attempt-1",
                2,
                60,
                EvidenceDebtKind::ParticipantEffect,
                EvidenceDebtState::ResolvedWithoutConfirmation,
                1_000,
            ),
        ];
        let report = evaluate_evidence_debt(&contract(), &events).expect("negative receipt");
        assert_eq!(report.resolved_without_confirmation_items, 1);
        assert_eq!(report.confirmed_items, 0);
        assert_eq!(report.total_unresolved_duration_ms, 50);
    }

    #[test]
    fn peak_concurrent_unresolved_items_and_kind_totals_are_reported() {
        let events = vec![
            event(
                "a1",
                "effect-a",
                1,
                0,
                EvidenceDebtKind::ParticipantEffect,
                EvidenceDebtState::PendingReconciliation,
                1_000,
            ),
            event(
                "b1",
                "claim-b",
                2,
                100,
                EvidenceDebtKind::RecoveryClaim,
                EvidenceDebtState::PendingReconciliation,
                2_000,
            ),
            event(
                "a2",
                "effect-a",
                3,
                300,
                EvidenceDebtKind::ParticipantEffect,
                EvidenceDebtState::Confirmed,
                1_000,
            ),
            event(
                "b2",
                "claim-b",
                4,
                500,
                EvidenceDebtKind::RecoveryClaim,
                EvidenceDebtState::Confirmed,
                2_000,
            ),
        ];
        let report = evaluate_evidence_debt(&contract(), &events).expect("overlapping debt");
        assert_eq!(report.peak_concurrent_unresolved_items, 2);
        assert_eq!(
            report.by_kind[&EvidenceDebtKind::ParticipantEffect].unresolved_duration_ms,
            300
        );
        assert_eq!(
            report.by_kind[&EvidenceDebtKind::RecoveryClaim].unresolved_duration_ms,
            400
        );
        assert_eq!(report.total_unresolved_duration_ms, 700);
    }

    #[test]
    fn evaluator_sequence_not_input_order_determines_the_result() {
        let events = vec![
            event(
                "e3",
                "effect-1",
                3,
                300,
                EvidenceDebtKind::ParticipantEffect,
                EvidenceDebtState::Confirmed,
                1_000,
            ),
            event(
                "e1",
                "effect-1",
                1,
                100,
                EvidenceDebtKind::ParticipantEffect,
                EvidenceDebtState::PendingReconciliation,
                1_000,
            ),
            event(
                "e2",
                "effect-1",
                2,
                200,
                EvidenceDebtKind::ParticipantEffect,
                EvidenceDebtState::ObserverGap,
                1_000,
            ),
        ];
        let report = evaluate_evidence_debt(&contract(), &events).expect("sorted event sequence");
        assert_eq!(report.total_unresolved_duration_ms, 200);
        assert_eq!(report.by_state[&EvidenceDebtState::ObserverGap], 100);
    }

    #[test]
    fn duplicate_ids_sequences_and_operation_weight_changes_are_rejected() {
        let duplicate_id = vec![
            event(
                "same",
                "a",
                1,
                0,
                EvidenceDebtKind::ParticipantEffect,
                EvidenceDebtState::PendingReconciliation,
                1_000,
            ),
            event(
                "same",
                "b",
                2,
                10,
                EvidenceDebtKind::ParticipantEffect,
                EvidenceDebtState::PendingReconciliation,
                1_000,
            ),
        ];
        assert_eq!(
            evaluate_evidence_debt(&contract(), &duplicate_id),
            Err(EvidenceDebtError::DuplicateEventId)
        );

        let duplicate_sequence = vec![
            event(
                "a",
                "a",
                1,
                0,
                EvidenceDebtKind::ParticipantEffect,
                EvidenceDebtState::PendingReconciliation,
                1_000,
            ),
            event(
                "b",
                "b",
                1,
                10,
                EvidenceDebtKind::ParticipantEffect,
                EvidenceDebtState::PendingReconciliation,
                1_000,
            ),
        ];
        assert_eq!(
            evaluate_evidence_debt(&contract(), &duplicate_sequence),
            Err(EvidenceDebtError::DuplicateSequence)
        );

        let changed_weight = vec![
            event(
                "a1",
                "a",
                1,
                0,
                EvidenceDebtKind::ParticipantEffect,
                EvidenceDebtState::PendingReconciliation,
                1_000,
            ),
            event(
                "a2",
                "a",
                2,
                10,
                EvidenceDebtKind::ParticipantEffect,
                EvidenceDebtState::Confirmed,
                2_000,
            ),
        ];
        assert_eq!(
            evaluate_evidence_debt(&contract(), &changed_weight),
            Err(EvidenceDebtError::OperationWeightChanged)
        );
    }

    #[test]
    fn terminal_states_require_an_authoritative_reference_and_cannot_reopen() {
        let mut missing_confirmation = event(
            "a",
            "effect-a",
            1,
            0,
            EvidenceDebtKind::ParticipantEffect,
            EvidenceDebtState::Confirmed,
            1_000,
        );
        missing_confirmation.evidence_refs.clear();
        assert_eq!(
            evaluate_evidence_debt(&contract(), &[missing_confirmation]),
            Err(EvidenceDebtError::ConfirmedWithoutEvidence)
        );

        let mut missing_resolution = event(
            "a",
            "effect-a",
            1,
            0,
            EvidenceDebtKind::ParticipantEffect,
            EvidenceDebtState::ResolvedWithoutConfirmation,
            1_000,
        );
        missing_resolution.resolution_ref = None;
        assert_eq!(
            evaluate_evidence_debt(&contract(), &[missing_resolution]),
            Err(EvidenceDebtError::UnverifiedClosureWithoutReceipt)
        );

        let events = vec![
            event(
                "a1",
                "effect-a",
                1,
                0,
                EvidenceDebtKind::ParticipantEffect,
                EvidenceDebtState::PendingReconciliation,
                1_000,
            ),
            event(
                "a2",
                "effect-a",
                2,
                10,
                EvidenceDebtKind::ParticipantEffect,
                EvidenceDebtState::Confirmed,
                1_000,
            ),
            event(
                "a3",
                "effect-a",
                3,
                20,
                EvidenceDebtKind::ParticipantEffect,
                EvidenceDebtState::ObserverGap,
                1_000,
            ),
        ];
        assert_eq!(
            evaluate_evidence_debt(&contract(), &events),
            Err(EvidenceDebtError::EventAfterTerminalState)
        );
    }

    #[test]
    fn run_bounds_resource_limits_and_invalid_references_fail_closed() {
        let outside = event(
            "a",
            "effect-a",
            1,
            1_001,
            EvidenceDebtKind::ParticipantEffect,
            EvidenceDebtState::PendingReconciliation,
            1_000,
        );
        assert_eq!(
            evaluate_evidence_debt(&contract(), &[outside.clone()]),
            Err(EvidenceDebtError::EventOutsideRun)
        );
        let mut invalid_ref = outside;
        invalid_ref.occurred_at_ms = 0;
        invalid_ref.evidence_refs = vec!["x".repeat(MAX_REFERENCE_BYTES + 1)];
        assert_eq!(
            evaluate_evidence_debt(&contract(), &[invalid_ref]),
            Err(EvidenceDebtError::InvalidEvidenceReference)
        );

        let mut tiny = contract();
        tiny.maximum_events = 1;
        let two = vec![
            event(
                "a",
                "a",
                1,
                0,
                EvidenceDebtKind::ParticipantEffect,
                EvidenceDebtState::PendingReconciliation,
                1_000,
            ),
            event(
                "b",
                "b",
                2,
                0,
                EvidenceDebtKind::ParticipantEffect,
                EvidenceDebtState::PendingReconciliation,
                1_000,
            ),
        ];
        assert_eq!(
            evaluate_evidence_debt(&tiny, &two),
            Err(EvidenceDebtError::TooManyEvents)
        );
    }

    #[test]
    fn non_monotonic_event_time_within_one_operation_is_rejected() {
        let events = vec![
            event(
                "a1",
                "effect-a",
                1,
                100,
                EvidenceDebtKind::ParticipantEffect,
                EvidenceDebtState::PendingReconciliation,
                1_000,
            ),
            event(
                "a2",
                "effect-a",
                2,
                90,
                EvidenceDebtKind::ParticipantEffect,
                EvidenceDebtState::Confirmed,
                1_000,
            ),
        ];
        assert_eq!(
            evaluate_evidence_debt(&contract(), &events),
            Err(EvidenceDebtError::EventSequenceTimeRegression)
        );
    }


    #[test]
    fn global_evaluator_sequence_must_have_monotonic_event_time() {
        let events = vec![
            event(
                "a1",
                "effect-a",
                1,
                200,
                EvidenceDebtKind::ParticipantEffect,
                EvidenceDebtState::PendingReconciliation,
                1_000,
            ),
            event(
                "b1",
                "effect-b",
                2,
                100,
                EvidenceDebtKind::ParticipantEffect,
                EvidenceDebtState::PendingReconciliation,
                1_000,
            ),
        ];
        assert_eq!(
            evaluate_evidence_debt(&contract(), &events),
            Err(EvidenceDebtError::EventSequenceTimeRegression)
        );
    }
}
