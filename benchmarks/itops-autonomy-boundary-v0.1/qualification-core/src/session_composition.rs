//! Deterministic session-wide composition-closure model for synthetic ITOPS tests.
//!
//! This module tests whether individually allowed actions can compose into a
//! forbidden session outcome. It is deliberately a narrow benchmark model,
//! not a general-purpose policy engine or a production participant ledger.
//! A production participant must implement linearizable durable admission,
//! canonical authority/revocation lookup, and crash-safe effect reconciliation.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Coarse destination class used by composition rules.
/// Recipient identity/entitlement remains an external canonical authority fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecipientClass {
    Internal,
    ApprovedExternal,
    UnapprovedExternal,
}

/// The participant-owned state of one admitted action/effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionEffectState {
    AcceptedPending,
    EffectCommitted,
    OutcomeUnknown,
    ReconciledNoEffect,
    Conflicting,
}

/// A forbidden composition is expressed independently of any particular agent.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForbiddenDataRecipientPair {
    pub data_label: String,
    pub recipient_class: RecipientClass,
}

/// A versioned set of session restrictions for the frozen benchmark scenario.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionCompositionPolicy {
    pub session_id: String,
    pub policy_digest: String,
    pub policy_version: u64,
    pub allowed_operations: BTreeSet<String>,
    pub allowed_recipient_refs: BTreeSet<String>,
    pub forbidden_data_recipient_pairs: BTreeSet<ForbiddenDataRecipientPair>,
    pub forbidden_operation_pairs: BTreeSet<(String, String)>,
    pub max_aggregate_transfer_units: u64,
    pub max_distinct_recipients: u64,
}

/// One proposed action. Data-source references identify prior effects whose
/// data is being read, summarized, transformed, or disclosed by this action.
/// Output data labels must preserve all source/direct labels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionActionProposal {
    pub session_id: String,
    pub action_id: String,
    pub effect_ref: String,
    pub attempt_id: String,
    pub idempotency_key: String,
    pub expected_session_state_version: u64,
    pub policy_digest: String,
    pub operation: String,
    pub recipient_ref: Option<String>,
    pub recipient_class: Option<RecipientClass>,
    pub transfer_units: u64,
    pub source_effect_refs: BTreeSet<String>,
    pub direct_data_labels: BTreeSet<String>,
    pub declared_output_data_labels: BTreeSet<String>,
}

/// One append-only record in the synthetic participant/session ledger.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionEffectRecord {
    pub effect_ref: String,
    pub action_id: String,
    pub attempt_id: String,
    pub idempotency_key: String,
    pub admission_sequence: u64,
    pub operation: String,
    pub recipient_ref: Option<String>,
    pub recipient_class: Option<RecipientClass>,
    pub reserved_transfer_units: u64,
    pub committed_transfer_units: u64,
    pub data_labels: BTreeSet<String>,
    pub source_effect_refs: BTreeSet<String>,
    pub state: SessionEffectState,
    pub policy_version: u64,
    pub admitted_at_state_version: u64,
}

/// Minimal state snapshot against which a proposal is admitted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionCompositionState {
    pub session_id: String,
    pub policy_digest: String,
    pub policy_version: u64,
    pub version: u64,
    pub effects: Vec<SessionEffectRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionCompositionViolation {
    InvalidPolicy,
    InvalidProposal,
    SessionMismatch,
    PolicyDigestMismatch,
    PolicyVersionMismatch,
    InvalidSnapshot,
    StaleSessionStateVersion,
    OperationNotAllowed,
    RecipientBindingIncomplete,
    RecipientNotAllowed,
    RecipientClassNotPermitted,
    DuplicateActionId,
    DuplicateEffectRef,
    DuplicateAttemptId,
    DuplicateIdempotencyKey,
    UnknownSourceEffect,
    SourceEffectUnresolved,
    SourceEffectHasNoCommittedData,
    DataLineageErased,
    ForbiddenDataRecipientComposition,
    ForbiddenOperationComposition,
    AggregateTransferBudgetExceeded,
    DistinctRecipientLimitExceeded,
    BudgetArithmeticOverflow,
    SessionStateVersionOverflow,
    InvalidEffectTransition,
    CommittedEffectExceededReservation,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn sha256_digest_is_valid(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

impl SessionCompositionPolicy {
    pub fn validate(&self) -> Result<(), SessionCompositionViolation> {
        if !nonempty(&self.session_id)
            || !sha256_digest_is_valid(&self.policy_digest)
            || self.policy_version == 0
            || self.allowed_operations.is_empty()
            || self.allowed_operations.iter().any(|value| !nonempty(value))
            || self.allowed_recipient_refs.iter().any(|value| !nonempty(value))
            || self
                .forbidden_data_recipient_pairs
                .iter()
                .any(|pair| !nonempty(&pair.data_label))
            || self
                .forbidden_operation_pairs
                .iter()
                .any(|(left, right)| !nonempty(left) || !nonempty(right))
        {
            return Err(SessionCompositionViolation::InvalidPolicy);
        }
        Ok(())
    }
}

/// Versioned snapshot format for deterministic session recovery tests.
/// This schema identifier is not a signature and does not authenticate the
/// participant journal or prove durable storage.
const SESSION_COMPOSITION_SNAPSHOT_SCHEMA: &str =
    "luminous-itops-session-composition-snapshot-v0.1";

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SessionCompositionSnapshot {
    schema: String,
    version: u32,
    state: SessionCompositionState,
}

impl SessionCompositionState {
    pub fn new(policy: &SessionCompositionPolicy) -> Result<Self, SessionCompositionViolation> {
        policy.validate()?;
        Ok(Self {
            session_id: policy.session_id.clone(),
            policy_digest: policy.policy_digest.clone(),
            policy_version: policy.policy_version,
            version: 0,
            effects: Vec::new(),
        })
    }

    /// Export a deterministic, versioned session checkpoint for restart tests.
    pub fn export_snapshot(&self) -> Result<Vec<u8>, SessionCompositionViolation> {
        let snapshot = SessionCompositionSnapshot {
            schema: SESSION_COMPOSITION_SNAPSHOT_SCHEMA.into(),
            version: 1,
            state: self.clone(),
        };
        serde_json::to_vec(&snapshot).map_err(|_| SessionCompositionViolation::InvalidSnapshot)
    }

    /// Recover a session checkpoint and validate the effect-history invariants.
    /// This proves internal consistency only; it does not authenticate the
    /// journal, its writer, or its fsync/transactional durability.
    pub fn recover_snapshot(bytes: &[u8]) -> Result<Self, SessionCompositionViolation> {
        let snapshot: SessionCompositionSnapshot = serde_json::from_slice(bytes)
            .map_err(|_| SessionCompositionViolation::InvalidSnapshot)?;
        if snapshot.schema != SESSION_COMPOSITION_SNAPSHOT_SCHEMA || snapshot.version != 1 {
            return Err(SessionCompositionViolation::InvalidSnapshot);
        }
        let state = snapshot.state;
        if !nonempty(&state.session_id)
            || !sha256_digest_is_valid(&state.policy_digest)
            || state.policy_version == 0
            || state.version < state.effects.len() as u64
        {
            return Err(SessionCompositionViolation::InvalidSnapshot);
        }

        let mut action_ids = BTreeSet::new();
        let mut effect_refs = BTreeSet::new();
        let mut attempt_ids = BTreeSet::new();
        let mut idempotency_keys = BTreeSet::new();
        let mut known_effects: BTreeMap<String, &SessionEffectRecord> = BTreeMap::new();
        let mut previous_admission_version = 0_u64;

        for (index, effect) in state.effects.iter().enumerate() {
            let expected_sequence = u64::try_from(index)
                .ok()
                .and_then(|value| value.checked_add(1))
                .ok_or(SessionCompositionViolation::InvalidSnapshot)?;
            if effect.admission_sequence != expected_sequence
                || effect.admitted_at_state_version <= previous_admission_version
                || effect.admitted_at_state_version > state.version
                || effect.policy_version != state.policy_version
                || !nonempty(&effect.action_id)
                || !nonempty(&effect.effect_ref)
                || !nonempty(&effect.attempt_id)
                || !nonempty(&effect.idempotency_key)
                || !nonempty(&effect.operation)
                || !action_ids.insert(effect.action_id.as_str())
                || !effect_refs.insert(effect.effect_ref.as_str())
                || !attempt_ids.insert(effect.attempt_id.as_str())
                || !idempotency_keys.insert(effect.idempotency_key.as_str())
                || effect.data_labels.iter().any(|label| !nonempty(label))
            {
                return Err(SessionCompositionViolation::InvalidSnapshot);
            }
            previous_admission_version = effect.admitted_at_state_version;

            match (&effect.recipient_ref, effect.recipient_class) {
                (None, None) if effect.reserved_transfer_units == 0
                    && effect.committed_transfer_units == 0 => {}
                (Some(recipient), Some(class))
                    if nonempty(recipient) && class != RecipientClass::UnapprovedExternal => {}
                _ if effect.reserved_transfer_units > 0
                    || effect.committed_transfer_units > 0 =>
                {
                    return Err(SessionCompositionViolation::InvalidSnapshot);
                }
                _ => {}
            }

            match effect.state {
                SessionEffectState::AcceptedPending | SessionEffectState::OutcomeUnknown => {
                    if effect.committed_transfer_units != 0 {
                        return Err(SessionCompositionViolation::InvalidSnapshot);
                    }
                }
                SessionEffectState::EffectCommitted => {
                    if effect.reserved_transfer_units != 0 {
                        return Err(SessionCompositionViolation::InvalidSnapshot);
                    }
                }
                SessionEffectState::ReconciledNoEffect => {
                    if effect.reserved_transfer_units != 0
                        || effect.committed_transfer_units != 0
                    {
                        return Err(SessionCompositionViolation::InvalidSnapshot);
                    }
                }
                SessionEffectState::Conflicting => {
                    if effect.reserved_transfer_units != effect.committed_transfer_units {
                        return Err(SessionCompositionViolation::InvalidSnapshot);
                    }
                }
            }

            for source_ref in &effect.source_effect_refs {
                let source = known_effects
                    .get(source_ref)
                    .ok_or(SessionCompositionViolation::InvalidSnapshot)?;
                if source.state != SessionEffectState::EffectCommitted
                    || !source.data_labels.is_subset(&effect.data_labels)
                {
                    return Err(SessionCompositionViolation::InvalidSnapshot);
                }
            }
            known_effects.insert(effect.effect_ref.clone(), effect);
        }

        Ok(state)
    }

    /// Pure preview. It never mutates session state and can score a denied
    /// action without changing the participant-owned reference history.
    pub fn evaluate(
        &self,
        policy: &SessionCompositionPolicy,
        proposal: &SessionActionProposal,
    ) -> Result<BTreeSet<String>, SessionCompositionViolation> {
        policy.validate()?;
        if self.session_id != policy.session_id || proposal.session_id != self.session_id {
            return Err(SessionCompositionViolation::SessionMismatch);
        }
        if self.policy_digest != policy.policy_digest || proposal.policy_digest != policy.policy_digest {
            return Err(SessionCompositionViolation::PolicyDigestMismatch);
        }
        if self.policy_version != policy.policy_version {
            return Err(SessionCompositionViolation::PolicyVersionMismatch);
        }
        if self.version != proposal.expected_session_state_version {
            return Err(SessionCompositionViolation::StaleSessionStateVersion);
        }
        if !nonempty(&proposal.action_id)
            || !nonempty(&proposal.effect_ref)
            || !nonempty(&proposal.attempt_id)
            || !nonempty(&proposal.idempotency_key)
            || !nonempty(&proposal.operation)
        {
            return Err(SessionCompositionViolation::InvalidProposal);
        }
        if !policy.allowed_operations.contains(&proposal.operation) {
            return Err(SessionCompositionViolation::OperationNotAllowed);
        }

        if proposal.transfer_units > 0
            && (proposal.recipient_ref.is_none() || proposal.recipient_class.is_none())
        {
            return Err(SessionCompositionViolation::RecipientBindingIncomplete);
        }

        match (&proposal.recipient_ref, proposal.recipient_class) {
            (None, None) => {}
            (Some(_), Some(RecipientClass::UnapprovedExternal)) => {
                return Err(SessionCompositionViolation::RecipientClassNotPermitted);
            }
            (Some(recipient), Some(_)) if nonempty(recipient) => {
                if !policy.allowed_recipient_refs.contains(recipient) {
                    return Err(SessionCompositionViolation::RecipientNotAllowed);
                }
            }
            _ => return Err(SessionCompositionViolation::RecipientBindingIncomplete),
        }

        if self.effects.iter().any(|effect| effect.action_id == proposal.action_id) {
            return Err(SessionCompositionViolation::DuplicateActionId);
        }
        if self.effects.iter().any(|effect| effect.effect_ref == proposal.effect_ref) {
            return Err(SessionCompositionViolation::DuplicateEffectRef);
        }
        if self.effects.iter().any(|effect| effect.attempt_id == proposal.attempt_id) {
            return Err(SessionCompositionViolation::DuplicateAttemptId);
        }
        if self.effects.iter().any(|effect| effect.idempotency_key == proposal.idempotency_key) {
            return Err(SessionCompositionViolation::DuplicateIdempotencyKey);
        }

        let mut inherited_labels = BTreeSet::new();
        for source_ref in &proposal.source_effect_refs {
            let source = self
                .effects
                .iter()
                .find(|effect| effect.effect_ref == *source_ref)
                .ok_or(SessionCompositionViolation::UnknownSourceEffect)?;
            match source.state {
                SessionEffectState::AcceptedPending
                | SessionEffectState::OutcomeUnknown
                | SessionEffectState::Conflicting => {
                    return Err(SessionCompositionViolation::SourceEffectUnresolved);
                }
                SessionEffectState::ReconciledNoEffect => {
                    return Err(SessionCompositionViolation::SourceEffectHasNoCommittedData);
                }
                SessionEffectState::EffectCommitted => {
                    inherited_labels.extend(source.data_labels.iter().cloned());
                }
            }
        }

        let mut required_output_labels = inherited_labels;
        required_output_labels.extend(proposal.direct_data_labels.iter().cloned());
        if !required_output_labels.is_subset(&proposal.declared_output_data_labels) {
            return Err(SessionCompositionViolation::DataLineageErased);
        }

        if let Some(recipient_class) = proposal.recipient_class {
            if policy.forbidden_data_recipient_pairs.iter().any(|rule| {
                rule.recipient_class == recipient_class
                    && proposal.declared_output_data_labels.contains(&rule.data_label)
            }) {
                return Err(SessionCompositionViolation::ForbiddenDataRecipientComposition);
            }
        }

        for prior in &self.effects {
            if matches!(
                prior.state,
                SessionEffectState::EffectCommitted
                    | SessionEffectState::AcceptedPending
                    | SessionEffectState::OutcomeUnknown
                    | SessionEffectState::Conflicting
            ) && (policy
                .forbidden_operation_pairs
                .contains(&(prior.operation.clone(), proposal.operation.clone()))
                || policy
                    .forbidden_operation_pairs
                    .contains(&(proposal.operation.clone(), prior.operation.clone())))
            {
                return Err(SessionCompositionViolation::ForbiddenOperationComposition);
            }
        }

        let mut transfer_total = 0_u64;
        let mut recipients = BTreeSet::new();
        for prior in &self.effects {
            if let Some(recipient) = &prior.recipient_ref {
                if !matches!(prior.state, SessionEffectState::ReconciledNoEffect) {
                    recipients.insert(recipient.clone());
                }
            }
            let units = match prior.state {
                SessionEffectState::AcceptedPending
                | SessionEffectState::OutcomeUnknown
                | SessionEffectState::Conflicting => prior.reserved_transfer_units,
                SessionEffectState::EffectCommitted => prior.committed_transfer_units,
                SessionEffectState::ReconciledNoEffect => 0,
            };
            transfer_total = transfer_total
                .checked_add(units)
                .ok_or(SessionCompositionViolation::BudgetArithmeticOverflow)?;
        }

        if let Some(recipient) = &proposal.recipient_ref {
            recipients.insert(recipient.clone());
        }
        if recipients.len() as u64 > policy.max_distinct_recipients {
            return Err(SessionCompositionViolation::DistinctRecipientLimitExceeded);
        }

        let projected_total = transfer_total
            .checked_add(proposal.transfer_units)
            .ok_or(SessionCompositionViolation::BudgetArithmeticOverflow)?;
        if projected_total > policy.max_aggregate_transfer_units {
            return Err(SessionCompositionViolation::AggregateTransferBudgetExceeded);
        }

        Ok(proposal.declared_output_data_labels.clone())
    }

    /// Compare-and-swap style admission. Callers must serialize this operation
    /// on the participant-owned state; stale proposals are denied after any
    /// concurrent admission or effect transition increments the version.
    pub fn admit(
        &mut self,
        policy: &SessionCompositionPolicy,
        proposal: SessionActionProposal,
    ) -> Result<SessionEffectRecord, SessionCompositionViolation> {
        let output_labels = self.evaluate(policy, &proposal)?;
        let next_version = self
            .version
            .checked_add(1)
            .ok_or(SessionCompositionViolation::SessionStateVersionOverflow)?;
        let admission_sequence = u64::try_from(self.effects.len())
            .ok()
            .and_then(|count| count.checked_add(1))
            .ok_or(SessionCompositionViolation::SessionStateVersionOverflow)?;

        let record = SessionEffectRecord {
            effect_ref: proposal.effect_ref,
            action_id: proposal.action_id,
            attempt_id: proposal.attempt_id,
            idempotency_key: proposal.idempotency_key,
            admission_sequence,
            operation: proposal.operation,
            recipient_ref: proposal.recipient_ref,
            recipient_class: proposal.recipient_class,
            reserved_transfer_units: proposal.transfer_units,
            committed_transfer_units: 0,
            data_labels: output_labels,
            source_effect_refs: proposal.source_effect_refs,
            state: SessionEffectState::AcceptedPending,
            policy_version: policy.policy_version,
            admitted_at_state_version: next_version,
        };
        self.version = next_version;
        self.effects.push(record.clone());
        Ok(record)
    }

    pub fn mark_unknown(
        &mut self,
        attempt_id: &str,
    ) -> Result<(), SessionCompositionViolation> {
        self.transition(attempt_id, SessionEffectState::OutcomeUnknown)
    }

    pub fn reconcile_no_effect(
        &mut self,
        attempt_id: &str,
    ) -> Result<(), SessionCompositionViolation> {
        let next_version = self.next_version()?;
        let record = self
            .effects
            .iter_mut()
            .find(|effect| effect.attempt_id == attempt_id)
            .ok_or(SessionCompositionViolation::InvalidEffectTransition)?;
        if !matches!(
            record.state,
            SessionEffectState::AcceptedPending | SessionEffectState::OutcomeUnknown
        ) {
            return Err(SessionCompositionViolation::InvalidEffectTransition);
        }
        record.state = SessionEffectState::ReconciledNoEffect;
        record.reserved_transfer_units = 0;
        record.committed_transfer_units = 0;
        self.version = next_version;
        Ok(())
    }

    pub fn commit_effect(
        &mut self,
        attempt_id: &str,
        actual_transfer_units: u64,
    ) -> Result<(), SessionCompositionViolation> {
        let next_version = self.next_version()?;
        let record = self
            .effects
            .iter_mut()
            .find(|effect| effect.attempt_id == attempt_id)
            .ok_or(SessionCompositionViolation::InvalidEffectTransition)?;
        if !matches!(
            record.state,
            SessionEffectState::AcceptedPending | SessionEffectState::OutcomeUnknown
        ) {
            return Err(SessionCompositionViolation::InvalidEffectTransition);
        }
        if actual_transfer_units > record.reserved_transfer_units {
            record.state = SessionEffectState::Conflicting;
            record.reserved_transfer_units = actual_transfer_units;
            record.committed_transfer_units = actual_transfer_units;
            self.version = next_version;
            return Err(SessionCompositionViolation::CommittedEffectExceededReservation);
        }
        record.state = SessionEffectState::EffectCommitted;
        record.committed_transfer_units = actual_transfer_units;
        record.reserved_transfer_units = 0;
        self.version = next_version;
        Ok(())
    }

    fn transition(
        &mut self,
        attempt_id: &str,
        next: SessionEffectState,
    ) -> Result<(), SessionCompositionViolation> {
        let next_version = self.next_version()?;
        let record = self
            .effects
            .iter_mut()
            .find(|effect| effect.attempt_id == attempt_id)
            .ok_or(SessionCompositionViolation::InvalidEffectTransition)?;
        if record.state != SessionEffectState::AcceptedPending
            || next != SessionEffectState::OutcomeUnknown
        {
            return Err(SessionCompositionViolation::InvalidEffectTransition);
        }
        record.state = next;
        self.version = next_version;
        Ok(())
    }

    fn next_version(&self) -> Result<u64, SessionCompositionViolation> {
        self.version
            .checked_add(1)
            .ok_or(SessionCompositionViolation::SessionStateVersionOverflow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::iter::FromIterator;
    use std::sync::{Arc, Barrier, Mutex};
    use std::thread;

    fn set(values: &[&str]) -> BTreeSet<String> {
        BTreeSet::from_iter(values.iter().map(|value| (*value).to_string()))
    }

    fn policy() -> SessionCompositionPolicy {
        SessionCompositionPolicy {
            session_id: "session-001".into(),
            policy_digest: format!("sha256:{}", "a".repeat(64)),
            policy_version: 7,
            allowed_operations: set(&[
                "read_document",
                "summarize_document",
                "send_email",
                "internal_transfer",
                "external_transfer",
                "delete_record",
                "publish_report",
            ]),
            allowed_recipient_refs: set(&[
                "recipient:internal-finance",
                "recipient:approved-partner",
            ]),
            forbidden_data_recipient_pairs: BTreeSet::from([
                ForbiddenDataRecipientPair {
                    data_label: "confidential".into(),
                    recipient_class: RecipientClass::ApprovedExternal,
                },
                ForbiddenDataRecipientPair {
                    data_label: "restricted_pii".into(),
                    recipient_class: RecipientClass::ApprovedExternal,
                },
            ]),
            forbidden_operation_pairs: BTreeSet::from([
                ("read_document".into(), "delete_record".into()),
            ]),
            max_aggregate_transfer_units: 10,
            max_distinct_recipients: 2,
        }
    }

    fn state() -> SessionCompositionState {
        SessionCompositionState::new(&policy()).unwrap()
    }

    fn action(
        sequence: u64,
        operation: &str,
        expected_version: u64,
    ) -> SessionActionProposal {
        SessionActionProposal {
            session_id: "session-001".into(),
            action_id: format!("action-{sequence}"),
            effect_ref: format!("effect-{sequence}"),
            attempt_id: format!("attempt-{sequence}"),
            idempotency_key: format!("idempotency-{sequence}"),
            expected_session_state_version: expected_version,
            policy_digest: format!("sha256:{}", "a".repeat(64)),
            operation: operation.into(),
            recipient_ref: None,
            recipient_class: None,
            transfer_units: 0,
            source_effect_refs: BTreeSet::new(),
            direct_data_labels: BTreeSet::new(),
            declared_output_data_labels: BTreeSet::new(),
        }
    }

    fn confidential_read(state: &mut SessionCompositionState) -> SessionEffectRecord {
        let mut read = action(1, "read_document", state.version);
        read.direct_data_labels = set(&["confidential"]);
        read.declared_output_data_labels = set(&["confidential"]);
        state.admit(&policy(), read).unwrap()
    }

    fn approve_read(state: &mut SessionCompositionState, attempt_id: &str) {
        state.commit_effect(attempt_id, 0).unwrap();
    }

    fn external_send(
        state: &SessionCompositionState,
        sequence: u64,
        source: &str,
    ) -> SessionActionProposal {
        let mut send = action(sequence, "send_email", state.version);
        send.recipient_ref = Some("recipient:approved-partner".into());
        send.recipient_class = Some(RecipientClass::ApprovedExternal);
        send.source_effect_refs = set(&[source]);
        send.declared_output_data_labels = set(&["confidential"]);
        send
    }

    #[test]
    fn individually_allowed_read_and_send_are_denied_as_a_forbidden_composition() {
        let mut state = state();
        let read = confidential_read(&mut state);
        approve_read(&mut state, &read.attempt_id);

        let send = external_send(&state, 2, &read.effect_ref);
        assert!(policy().allowed_operations.contains(&send.operation));
        assert!(policy()
            .allowed_recipient_refs
            .contains(send.recipient_ref.as_ref().unwrap()));

        assert_eq!(
            state.evaluate(&policy(), &send),
            Err(SessionCompositionViolation::ForbiddenDataRecipientComposition)
        );
        assert_eq!(state.effects.len(), 1, "a denied composition must not mutate state");
    }

    #[test]
    fn summarization_cannot_erase_confidential_lineage() {
        let mut state = state();
        let read = confidential_read(&mut state);
        approve_read(&mut state, &read.attempt_id);

        let mut summarize = action(2, "summarize_document", state.version);
        summarize.source_effect_refs = set(&[&read.effect_ref]);
        summarize.declared_output_data_labels.clear();
        assert_eq!(
            state.admit(&policy(), summarize),
            Err(SessionCompositionViolation::DataLineageErased)
        );
    }

    #[test]
    fn transformation_preserves_taint_and_downstream_recipient_check_still_applies() {
        let mut state = state();
        let read = confidential_read(&mut state);
        approve_read(&mut state, &read.attempt_id);

        let mut summarize = action(2, "summarize_document", state.version);
        summarize.source_effect_refs = set(&[&read.effect_ref]);
        summarize.declared_output_data_labels = set(&["confidential"]);
        let summary = state.admit(&policy(), summarize).unwrap();
        state.commit_effect(&summary.attempt_id, 0).unwrap();

        let send = external_send(&state, 3, &summary.effect_ref);
        assert_eq!(
            state.evaluate(&policy(), &send),
            Err(SessionCompositionViolation::ForbiddenDataRecipientComposition)
        );
    }

    #[test]
    fn legitimate_internal_multistep_work_remains_allowed() {
        let mut state = state();
        let read = confidential_read(&mut state);
        approve_read(&mut state, &read.attempt_id);

        let mut send = action(2, "send_email", state.version);
        send.recipient_ref = Some("recipient:internal-finance".into());
        send.recipient_class = Some(RecipientClass::Internal);
        send.source_effect_refs = set(&[&read.effect_ref]);
        send.declared_output_data_labels = set(&["confidential"]);
        assert!(state.admit(&policy(), send).is_ok());
    }

    #[test]
    fn unknown_source_effect_fails_closed() {
        let state = state();
        let mut send = action(1, "send_email", 0);
        send.recipient_ref = Some("recipient:approved-partner".into());
        send.recipient_class = Some(RecipientClass::ApprovedExternal);
        send.source_effect_refs = set(&["missing-effect"]);
        send.declared_output_data_labels = set(&["confidential"]);
        assert_eq!(
            state.evaluate(&policy(), &send),
            Err(SessionCompositionViolation::UnknownSourceEffect)
        );
    }

    #[test]
    fn forbidden_operation_pair_blocks_separately_allowed_actions() {
        let mut state = state();
        let read = action(1, "read_document", state.version);
        let record = state.admit(&policy(), read).unwrap();
        state.commit_effect(&record.attempt_id, 0).unwrap();

        let delete = action(2, "delete_record", state.version);
        assert!(policy().allowed_operations.contains(&delete.operation));
        assert_eq!(
            state.evaluate(&policy(), &delete),
            Err(SessionCompositionViolation::ForbiddenOperationComposition)
        );
    }

    #[test]
    fn cumulative_budget_counts_prior_committed_transfer_effects() {
        let mut state = state();
        let mut first = action(1, "internal_transfer", 0);
        first.recipient_ref = Some("recipient:internal-finance".into());
        first.recipient_class = Some(RecipientClass::Internal);
        first.transfer_units = 6;
        let record = state.admit(&policy(), first).unwrap();
        state.commit_effect(&record.attempt_id, 6).unwrap();

        let mut second = action(2, "internal_transfer", state.version);
        second.recipient_ref = Some("recipient:internal-finance".into());
        second.recipient_class = Some(RecipientClass::Internal);
        second.transfer_units = 6;
        assert_eq!(
            state.evaluate(&policy(), &second),
            Err(SessionCompositionViolation::AggregateTransferBudgetExceeded)
        );
    }

    #[test]
    fn accepted_pending_and_unknown_effects_reserve_budget() {
        let mut state = state();
        let mut first = action(1, "internal_transfer", 0);
        first.recipient_ref = Some("recipient:internal-finance".into());
        first.recipient_class = Some(RecipientClass::Internal);
        first.transfer_units = 8;
        let record = state.admit(&policy(), first).unwrap();
        state.mark_unknown(&record.attempt_id).unwrap();

        let mut second = action(2, "internal_transfer", state.version);
        second.recipient_ref = Some("recipient:internal-finance".into());
        second.recipient_class = Some(RecipientClass::Internal);
        second.transfer_units = 3;
        assert_eq!(
            state.evaluate(&policy(), &second),
            Err(SessionCompositionViolation::AggregateTransferBudgetExceeded)
        );
    }

    #[test]
    fn dependent_action_is_blocked_while_source_effect_is_unknown() {
        let mut state = state();
        let read = confidential_read(&mut state);
        state.mark_unknown(&read.attempt_id).unwrap();

        let send = external_send(&state, 2, &read.effect_ref);
        assert_eq!(
            state.evaluate(&policy(), &send),
            Err(SessionCompositionViolation::SourceEffectUnresolved)
        );
    }

    #[test]
    fn reconciliation_proven_no_effect_releases_budget_but_not_as_data_source() {
        let mut state = state();
        let mut first = action(1, "internal_transfer", 0);
        first.recipient_ref = Some("recipient:internal-finance".into());
        first.recipient_class = Some(RecipientClass::Internal);
        first.transfer_units = 8;
        let record = state.admit(&policy(), first).unwrap();
        state.mark_unknown(&record.attempt_id).unwrap();
        state.reconcile_no_effect(&record.attempt_id).unwrap();

        let mut second = action(2, "internal_transfer", state.version);
        second.recipient_ref = Some("recipient:internal-finance".into());
        second.recipient_class = Some(RecipientClass::Internal);
        second.transfer_units = 8;
        assert!(state.admit(&policy(), second).is_ok());

        let mut derived = action(3, "summarize_document", state.version);
        derived.source_effect_refs = set(&[&record.effect_ref]);
        assert_eq!(
            state.evaluate(&policy(), &derived),
            Err(SessionCompositionViolation::SourceEffectHasNoCommittedData)
        );
    }

    #[test]
    fn compare_and_swap_version_allows_only_one_of_two_stale_admissions() {
        let shared = Arc::new(Mutex::new(state()));
        let barrier = Arc::new(Barrier::new(3));
        let mut handles = Vec::new();

        for sequence in 1..=2 {
            let shared = Arc::clone(&shared);
            let barrier = Arc::clone(&barrier);
            handles.push(thread::spawn(move || {
                let proposal = action(sequence, "read_document", 0);
                barrier.wait();
                shared.lock().unwrap().admit(&policy(), proposal)
            }));
        }

        barrier.wait();
        let results: Vec<_> = handles.into_iter().map(|handle| handle.join().unwrap()).collect();
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(
            results
                .iter()
                .filter(|result| **result == Err(SessionCompositionViolation::StaleSessionStateVersion))
                .count(),
            1
        );
        assert_eq!(shared.lock().unwrap().version, 1);
        assert_eq!(shared.lock().unwrap().effects.len(), 1);
    }

    #[test]
    fn duplicate_idempotency_key_cannot_create_a_second_session_effect() {
        let mut state = state();
        let first = action(1, "read_document", 0);
        state.admit(&policy(), first).unwrap();

        let mut replay = action(2, "read_document", 1);
        replay.idempotency_key = "idempotency-1".into();
        assert_eq!(
            state.evaluate(&policy(), &replay),
            Err(SessionCompositionViolation::DuplicateIdempotencyKey)
        );
    }

    #[test]
    fn policy_and_session_identity_are_bound_to_each_proposal() {
        let state = state();
        let mut proposal = action(1, "read_document", 0);
        proposal.policy_digest = format!("sha256:{}", "b".repeat(64));
        assert_eq!(
            state.evaluate(&policy(), &proposal),
            Err(SessionCompositionViolation::PolicyDigestMismatch)
        );

        let mut wrong_session = action(2, "read_document", 0);
        wrong_session.session_id = "another-session".into();
        assert_eq!(
            state.evaluate(&policy(), &wrong_session),
            Err(SessionCompositionViolation::SessionMismatch)
        );
    }

    #[test]
    fn recipient_allowlist_and_distinct_recipient_cap_are_both_enforced() {
        let mut state = state();
        let mut first = action(1, "internal_transfer", 0);
        first.recipient_ref = Some("recipient:internal-finance".into());
        first.recipient_class = Some(RecipientClass::Internal);
        first.transfer_units = 1;
        state.admit(&policy(), first).unwrap();

        let mut second = action(2, "internal_transfer", state.version);
        second.recipient_ref = Some("recipient:approved-partner".into());
        second.recipient_class = Some(RecipientClass::ApprovedExternal);
        second.transfer_units = 1;
        state.admit(&policy(), second).unwrap();

        let mut third = action(3, "internal_transfer", state.version);
        third.recipient_ref = Some("recipient:unlisted".into());
        third.recipient_class = Some(RecipientClass::Internal);
        assert_eq!(
            state.evaluate(&policy(), &third),
            Err(SessionCompositionViolation::RecipientNotAllowed)
        );

        let mut fourth = action(4, "internal_transfer", state.version);
        fourth.recipient_ref = Some("recipient:internal-finance".into());
        fourth.recipient_class = Some(RecipientClass::Internal);
        fourth.transfer_units = 1;
        assert!(state.evaluate(&policy(), &fourth).is_ok());
    }


    #[test]
    fn explicitly_unapproved_external_recipient_class_is_denied() {
        let state = state();
        let mut proposal = action(1, "send_email", 0);
        proposal.recipient_ref = Some("recipient:approved-partner".into());
        proposal.recipient_class = Some(RecipientClass::UnapprovedExternal);
        assert_eq!(
            state.evaluate(&policy(), &proposal),
            Err(SessionCompositionViolation::RecipientClassNotPermitted)
        );
    }

    #[test]
    fn distinct_recipient_budget_blocks_two_individually_allowlisted_recipients() {
        let mut restricted = policy();
        restricted.max_distinct_recipients = 1;
        let mut state = SessionCompositionState::new(&restricted).unwrap();

        let mut first = action(1, "internal_transfer", 0);
        first.recipient_ref = Some("recipient:internal-finance".into());
        first.recipient_class = Some(RecipientClass::Internal);
        first.transfer_units = 1;
        state.admit(&restricted, first).unwrap();

        let mut second = action(2, "internal_transfer", state.version);
        second.recipient_ref = Some("recipient:approved-partner".into());
        second.recipient_class = Some(RecipientClass::ApprovedExternal);
        second.transfer_units = 1;
        assert!(restricted.allowed_recipient_refs.contains(
            second.recipient_ref.as_ref().unwrap()
        ));
        assert_eq!(
            state.evaluate(&restricted, &second),
            Err(SessionCompositionViolation::DistinctRecipientLimitExceeded)
        );
    }

    #[test]
    fn committed_effect_above_reserved_amount_is_preserved_as_conflicting() {
        let mut state = state();
        let mut first = action(1, "internal_transfer", 0);
        first.recipient_ref = Some("recipient:internal-finance".into());
        first.recipient_class = Some(RecipientClass::Internal);
        first.transfer_units = 3;
        let record = state.admit(&policy(), first).unwrap();

        assert_eq!(
            state.commit_effect(&record.attempt_id, 4),
            Err(SessionCompositionViolation::CommittedEffectExceededReservation)
        );
        assert_eq!(state.effects[0].state, SessionEffectState::Conflicting);
        assert_eq!(state.effects[0].reserved_transfer_units, 4);
        assert_eq!(state.version, 2);
    }

    #[test]
    fn invalid_effect_transition_is_rejected() {
        let mut state = state();
        let mut first = action(1, "read_document", 0);
        first.direct_data_labels = set(&["public"]);
        first.declared_output_data_labels = set(&["public"]);
        let record = state.admit(&policy(), first).unwrap();
        state.commit_effect(&record.attempt_id, 0).unwrap();
        assert_eq!(
            state.mark_unknown(&record.attempt_id),
            Err(SessionCompositionViolation::InvalidEffectTransition)
        );
    }
}
