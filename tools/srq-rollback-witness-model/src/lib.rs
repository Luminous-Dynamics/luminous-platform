//! Reference state machine for anchoring SRQ lifecycle generations outside the
//! local SQLite rollback domain.
//!
//! All cryptographic inputs are assumed to have been verified by a caller.
//! The model checks state transitions, idempotency, freshness and recovery
//! ordering; it does NOT implement signatures, durable remote storage, TPM NV,
//! quorum consensus, a secure clock or an actual anti-rollback hardware root.

use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Checkpoint {
    pub target_id: String,
    pub sequence: u64,
    pub previous_checkpoint_sha256: String,
    pub checkpoint_sha256: String,
    pub decision_id: String,
    pub challenge_id: String,
    pub previous_generation: u64,
    pub generation: u64,
    pub deployment_id: String,
    pub approved_artifact_sha256: String,
    pub effective_policy_sha256: String,
    pub corpus_sha256: String,
    pub issued_at_unix_seconds: u64,
    pub expires_at_unix_seconds: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WitnessTip {
    pub target_id: String,
    pub sequence: u64,
    pub checkpoint_sha256: String,
    pub generation: u64,
    pub decision_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WitnessReceipt {
    pub target_id: String,
    pub sequence: u64,
    pub checkpoint_sha256: String,
    pub decision_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WitnessError {
    Unavailable,
    UnknownTarget,
    MalformedCheckpoint(&'static str),
    ExpiredAuthorization,
    StalePredecessor,
    InvalidSequence,
    InvalidGeneration,
    DecisionReuseConflict,
    CheckpointConflict,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reconciliation {
    Consistent,
    PreparedNotWitnessed,
    WitnessCommittedNeedsLocalFinalize,
    LocalStateRollbackDetected,
    LocalStateAheadOfWitness,
    SameSequenceConflict,
    PreparedCheckpointConflict,
    WitnessUnavailable,
    UnknownTarget,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalState {
    pub target_id: String,
    pub sequence: u64,
    pub checkpoint_sha256: String,
    pub generation: u64,
    pub decision_id: Option<String>,
    prepared: Option<Checkpoint>,
}

impl LocalState {
    pub fn from_genesis(tip: &WitnessTip) -> Self {
        Self {
            target_id: tip.target_id.clone(),
            sequence: tip.sequence,
            checkpoint_sha256: tip.checkpoint_sha256.clone(),
            generation: tip.generation,
            decision_id: tip.decision_id.clone(),
            prepared: None,
        }
    }

    /// Persist an intent locally. This is explicitly not an accepted promotion.
    pub fn prepare(&mut self, checkpoint: Checkpoint) -> Result<(), WitnessError> {
        validate_checkpoint(&checkpoint)?;
        if self.prepared.is_some() {
            return Err(WitnessError::CheckpointConflict);
        }
        if checkpoint.target_id != self.target_id
            || self.sequence.checked_add(1) != Some(checkpoint.sequence)
            || checkpoint.previous_checkpoint_sha256 != self.checkpoint_sha256
        {
            return Err(WitnessError::StalePredecessor);
        }
        if checkpoint.previous_generation != self.generation
            || self.generation.checked_add(1) != Some(checkpoint.generation)
        {
            return Err(WitnessError::InvalidGeneration);
        }
        self.prepared = Some(checkpoint);
        Ok(())
    }

    /// Finalize local state only from a witness receipt that matches the exact
    /// prepared checkpoint. This is a model check, not receipt-signature crypto.
    pub fn finalize(&mut self, receipt: &WitnessReceipt) -> Result<(), WitnessError> {
        let checkpoint = self.prepared.as_ref().ok_or(WitnessError::CheckpointConflict)?;
        if receipt.target_id != checkpoint.target_id
            || receipt.sequence != checkpoint.sequence
            || receipt.checkpoint_sha256 != checkpoint.checkpoint_sha256
            || receipt.decision_id != checkpoint.decision_id
        {
            return Err(WitnessError::CheckpointConflict);
        }
        self.target_id = checkpoint.target_id.clone();
        self.sequence = checkpoint.sequence;
        self.checkpoint_sha256 = checkpoint.checkpoint_sha256.clone();
        self.generation = checkpoint.generation;
        self.decision_id = Some(checkpoint.decision_id.clone());
        self.prepared = None;
        Ok(())
    }

    pub fn prepared(&self) -> Option<&Checkpoint> {
        self.prepared.as_ref()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Witness {
    available: bool,
    tips: BTreeMap<String, WitnessTip>,
    receipts: BTreeMap<(String, String), WitnessReceipt>,
    history: BTreeMap<(String, u64), Checkpoint>,
}

impl Witness {
    pub fn new() -> Self {
        Self {
            available: true,
            tips: BTreeMap::new(),
            receipts: BTreeMap::new(),
            history: BTreeMap::new(),
        }
    }

    pub fn set_available(&mut self, available: bool) {
        self.available = available;
    }

    pub fn initialize_target(
        &mut self,
        target_id: &str,
        genesis_digest: &str,
        generation: u64,
    ) -> Result<WitnessTip, WitnessError> {
        if !self.available {
            return Err(WitnessError::Unavailable);
        }
        if !valid_identifier(target_id) {
            return Err(WitnessError::MalformedCheckpoint("target id"));
        }
        if !valid_digest(genesis_digest) {
            return Err(WitnessError::MalformedCheckpoint("genesis digest"));
        }
        if self.tips.contains_key(target_id) {
            return Err(WitnessError::CheckpointConflict);
        }
        let tip = WitnessTip {
            target_id: target_id.to_owned(),
            sequence: 0,
            checkpoint_sha256: genesis_digest.to_owned(),
            generation,
            decision_id: None,
        };
        self.tips.insert(target_id.to_owned(), tip.clone());
        Ok(tip)
    }

    pub fn tip(&self, target_id: &str) -> Result<&WitnessTip, WitnessError> {
        if !self.available {
            return Err(WitnessError::Unavailable);
        }
        self.tips.get(target_id).ok_or(WitnessError::UnknownTarget)
    }

    pub fn checkpoint_for_decision(
        &self,
        target_id: &str,
        decision_id: &str,
    ) -> Result<Option<&Checkpoint>, WitnessError> {
        if !self.available {
            return Err(WitnessError::Unavailable);
        }
        let Some(receipt) = self.receipts.get(&(target_id.to_owned(), decision_id.to_owned())) else {
            return Ok(None);
        };
        Ok(self.history.get(&(receipt.target_id.clone(), receipt.sequence)))
    }

    pub fn receipt_for_decision(
        &self,
        target_id: &str,
        decision_id: &str,
    ) -> Result<Option<&WitnessReceipt>, WitnessError> {
        if !self.available {
            return Err(WitnessError::Unavailable);
        }
        Ok(self.receipts.get(&(target_id.to_owned(), decision_id.to_owned())))
    }

    /// CAS the external monotonic anchor. The witness commit is the anti-
    /// rollback linearization point. Repeating exactly the same already-
    /// committed decision is idempotent even if its original expiry has since
    /// passed; a new advance must still satisfy current freshness.
    pub fn advance_at(
        &mut self,
        checkpoint: &Checkpoint,
        now_unix_seconds: u64,
    ) -> Result<WitnessReceipt, WitnessError> {
        if !self.available {
            return Err(WitnessError::Unavailable);
        }
        validate_checkpoint(checkpoint)?;
        let key = (checkpoint.target_id.clone(), checkpoint.decision_id.clone());
        if let Some(existing) = self.receipts.get(&key) {
            let committed_checkpoint = self
                .history
                .get(&(checkpoint.target_id.clone(), existing.sequence));
            if existing.checkpoint_sha256 == checkpoint.checkpoint_sha256
                && existing.sequence == checkpoint.sequence
                && committed_checkpoint == Some(checkpoint)
            {
                return Ok(existing.clone());
            }
            return Err(WitnessError::DecisionReuseConflict);
        }
        if now_unix_seconds < checkpoint.issued_at_unix_seconds
            || now_unix_seconds >= checkpoint.expires_at_unix_seconds
        {
            return Err(WitnessError::ExpiredAuthorization);
        }
        let tip = self.tips.get(&checkpoint.target_id).ok_or(WitnessError::UnknownTarget)?;
        if tip.sequence.checked_add(1) != Some(checkpoint.sequence) {
            return Err(WitnessError::InvalidSequence);
        }
        if checkpoint.previous_checkpoint_sha256 != tip.checkpoint_sha256 {
            return Err(WitnessError::StalePredecessor);
        }
        if checkpoint.previous_generation != tip.generation
            || tip.generation.checked_add(1) != Some(checkpoint.generation)
        {
            return Err(WitnessError::InvalidGeneration);
        }

        let receipt = WitnessReceipt {
            target_id: checkpoint.target_id.clone(),
            sequence: checkpoint.sequence,
            checkpoint_sha256: checkpoint.checkpoint_sha256.clone(),
            decision_id: checkpoint.decision_id.clone(),
        };
        let new_tip = WitnessTip {
            target_id: checkpoint.target_id.clone(),
            sequence: checkpoint.sequence,
            checkpoint_sha256: checkpoint.checkpoint_sha256.clone(),
            generation: checkpoint.generation,
            decision_id: Some(checkpoint.decision_id.clone()),
        };
        self.history.insert((checkpoint.target_id.clone(), checkpoint.sequence), checkpoint.clone());
        self.receipts.insert(key, receipt.clone());
        self.tips.insert(checkpoint.target_id.clone(), new_tip);
        Ok(receipt)
    }
}

impl Default for Witness {
    fn default() -> Self {
        Self::new()
    }
}

pub fn reconcile(local: &LocalState, witness: &Witness) -> Reconciliation {
    let tip = match witness.tip(&local.target_id) {
        Ok(tip) => tip,
        Err(WitnessError::Unavailable) => return Reconciliation::WitnessUnavailable,
        Err(WitnessError::UnknownTarget) => return Reconciliation::UnknownTarget,
        Err(_) => return Reconciliation::UnknownTarget,
    };
    if local.target_id != tip.target_id {
        return Reconciliation::UnknownTarget;
    }
    if local.sequence < tip.sequence {
        if let Some(prepared) = local.prepared() {
            if prepared.sequence == tip.sequence {
                // A digest-shaped field is not proof that all stored payload fields
                // match. Recovery is permitted only when the entire prepared
                // checkpoint equals the witness's persisted committed checkpoint.
                let committed = match witness.checkpoint_for_decision(
                    &local.target_id,
                    &prepared.decision_id,
                ) {
                    Ok(checkpoint) => checkpoint,
                    Err(WitnessError::Unavailable) => return Reconciliation::WitnessUnavailable,
                    Err(WitnessError::UnknownTarget) => return Reconciliation::UnknownTarget,
                    Err(_) => return Reconciliation::UnknownTarget,
                };
                if committed == Some(prepared)
                    && prepared.checkpoint_sha256 == tip.checkpoint_sha256
                    && prepared.generation == tip.generation
                    && tip.decision_id.as_deref() == Some(prepared.decision_id.as_str())
                {
                    return Reconciliation::WitnessCommittedNeedsLocalFinalize;
                }
                return Reconciliation::PreparedCheckpointConflict;
            }
        }
        return Reconciliation::LocalStateRollbackDetected;
    }
    if local.sequence > tip.sequence {
        return Reconciliation::LocalStateAheadOfWitness;
    }
    if local.checkpoint_sha256 != tip.checkpoint_sha256
        || local.generation != tip.generation
        || local.decision_id != tip.decision_id
    {
        return Reconciliation::SameSequenceConflict;
    }
    if local.prepared.is_some() {
        return Reconciliation::PreparedNotWitnessed;
    }
    Reconciliation::Consistent
}

fn validate_checkpoint(checkpoint: &Checkpoint) -> Result<(), WitnessError> {
    if !valid_identifier(&checkpoint.target_id) {
        return Err(WitnessError::MalformedCheckpoint("target id"));
    }
    if !valid_identifier(&checkpoint.decision_id) || !valid_identifier(&checkpoint.challenge_id) {
        return Err(WitnessError::MalformedCheckpoint("decision/challenge id"));
    }
    if !valid_digest(&checkpoint.previous_checkpoint_sha256)
        || !valid_digest(&checkpoint.checkpoint_sha256)
        || !valid_digest(&checkpoint.approved_artifact_sha256)
        || !valid_digest(&checkpoint.effective_policy_sha256)
        || !valid_digest(&checkpoint.corpus_sha256)
    {
        return Err(WitnessError::MalformedCheckpoint("digest"));
    }
    if !valid_deployment_id(&checkpoint.deployment_id) {
        return Err(WitnessError::MalformedCheckpoint("deployment id"));
    }
    if checkpoint.previous_generation.checked_add(1) != Some(checkpoint.generation) {
        return Err(WitnessError::InvalidGeneration);
    }
    if checkpoint.expires_at_unix_seconds <= checkpoint.issued_at_unix_seconds {
        return Err(WitnessError::MalformedCheckpoint("freshness window"));
    }
    Ok(())
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-')
        })
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_deployment_id(value: &str) -> bool {
    if valid_digest(value) {
        return true;
    }
    let Some(rest) = value.strip_prefix("/nix/store/") else {
        return false;
    };
    let Some((hash, name)) = rest.split_once('-') else {
        return false;
    };
    hash.len() == 32
        && hash.bytes().all(|byte| {
            byte.is_ascii_digit() || b"abcdfghijklmnpqrsvwxyz".contains(&byte)
        })
        && !name.is_empty()
        && name.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'+' | b'-')
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::thread;

    const GENESIS: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const ARTIFACT: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const POLICY: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const CORPUS: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
    const DEPLOYMENT: &str = "/nix/store/0123456789abcdfghijklmnpqrsvwxyz-nixos-system-witness";

    fn checkpoint(target: &str, seq: u64, prior: &str, digest: &str, prev_gen: u64, now: u64, id: &str) -> Checkpoint {
        Checkpoint {
            target_id: target.into(),
            sequence: seq,
            previous_checkpoint_sha256: prior.into(),
            checkpoint_sha256: digest.into(),
            decision_id: id.into(),
            challenge_id: format!("challenge-{id}"),
            previous_generation: prev_gen,
            generation: prev_gen + 1,
            deployment_id: DEPLOYMENT.into(),
            approved_artifact_sha256: ARTIFACT.into(),
            effective_policy_sha256: POLICY.into(),
            corpus_sha256: CORPUS.into(),
            issued_at_unix_seconds: now,
            expires_at_unix_seconds: now + 30,
        }
    }

    fn initialized() -> (Witness, LocalState) {
        let mut witness = Witness::new();
        let tip = witness.initialize_target("target-01", GENESIS, 41).unwrap();
        (witness, LocalState::from_genesis(&tip))
    }

    #[test]
    fn witness_unavailability_blocks_genesis_initialization_without_mutation() {
        let mut witness = Witness::new();
        witness.set_available(false);

        assert_eq!(
            witness.initialize_target("target-01", GENESIS, 41),
            Err(WitnessError::Unavailable)
        );

        witness.set_available(true);
        assert!(matches!(
            witness.tip("target-01"),
            Err(WitnessError::UnknownTarget)
        ));
    }

    #[test]
    fn restored_local_database_is_detected_against_newer_independent_witness() {
        let (mut witness, mut local) = initialized();
        let backup = local.clone();
        let next = checkpoint("target-01", 1, GENESIS, ARTIFACT, 41, 1_000, "decision-1");
        local.prepare(next.clone()).unwrap();
        let receipt = witness.advance_at(&next, 1_001).unwrap();
        local.finalize(&receipt).unwrap();
        assert_eq!(reconcile(&local, &witness), Reconciliation::Consistent);

        // Restore the entire local database to the pre-promotion snapshot while
        // deliberately leaving the external witness in its newer rollback domain.
        local = backup;
        assert_eq!(reconcile(&local, &witness), Reconciliation::LocalStateRollbackDetected);
        assert!(matches!(
            witness.advance_at(&next, 1_002),
            Ok(_) // exact same decision is idempotent; not a second witness advance
        ));
        assert_eq!(witness.tip("target-01").unwrap().sequence, 1);
    }

    #[test]
    fn same_digest_retry_with_changed_checkpoint_fields_is_rejected() {
        let (mut witness, _local) = initialized();
        let original = checkpoint("target-01", 1, GENESIS, ARTIFACT, 41, 1_000, "decision-same");
        witness.advance_at(&original, 1_001).unwrap();

        // Deliberately reuse the digest to model a caller that fails to recompute
        // or authenticate its checkpoint payload. Idempotency must require exact
        // equality with the persisted checkpoint, not just the supplied digest.
        let mut changed = original.clone();
        changed.deployment_id = "/nix/store/bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbnixos-system-other".into();
        assert_eq!(
            witness.advance_at(&changed, 1_002),
            Err(WitnessError::DecisionReuseConflict)
        );
        assert_eq!(witness.tip("target-01").unwrap().sequence, 1);
        assert_eq!(
            witness.checkpoint_for_decision("target-01", "decision-same").unwrap(),
            Some(&original)
        );
    }

    #[test]
    fn witness_commit_before_local_finalize_recovers_same_decision_idempotently() {
        let (mut witness, mut local) = initialized();
        let next = checkpoint("target-01", 1, GENESIS, ARTIFACT, 41, 1_000, "decision-2");
        local.prepare(next.clone()).unwrap();
        let receipt = witness.advance_at(&next, 1_001).unwrap();
        assert_eq!(reconcile(&local, &witness), Reconciliation::WitnessCommittedNeedsLocalFinalize);
        let retried = witness.advance_at(&next, 1_002).unwrap();
        assert_eq!(receipt, retried);
        local.finalize(&retried).unwrap();
        assert_eq!(reconcile(&local, &witness), Reconciliation::Consistent);
    }

    #[test]
    fn changed_prepared_payload_with_same_digest_is_not_misclassified_as_recoverable() {
        let (mut witness, mut local) = initialized();
        let next = checkpoint("target-01", 1, GENESIS, ARTIFACT, 41, 1_000, "decision-prepared-tamper");
        local.prepare(next.clone()).unwrap();
        witness.advance_at(&next, 1_001).unwrap();

        // Simulate a restored/malformed prepared row with the same digest-shaped
        // string and decision ID, but altered semantic payload. The witness has
        // committed the original full checkpoint. A matching prefix must not be
        // enough to authorize local finalization after restart.
        local.prepared.as_mut().unwrap().deployment_id =
            "/nix/store/bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbnixos-system-tampered".into();

        assert_eq!(reconcile(&local, &witness), Reconciliation::PreparedCheckpointConflict);
        assert_ne!(reconcile(&local, &witness), Reconciliation::WitnessCommittedNeedsLocalFinalize);
        assert_eq!(witness.tip("target-01").unwrap().generation, 42);
        assert_eq!(local.generation, 41);
    }

    #[test]
    fn every_checkpoint_field_mutation_blocks_recovery_classification() {
        let (mut witness, mut local) = initialized();
        let next = checkpoint("target-01", 1, GENESIS, ARTIFACT, 41, 1_000, "decision-field-mutations");
        local.prepare(next.clone()).unwrap();
        witness.advance_at(&next, 1_001).unwrap();

        // Model damaged/restored prepared-row fields while preserving the
        // checkpoint digest-shaped value for all other field mutations.
        // The recovery classifier must compare the full committed payload.
        let mut mutations: Vec<Checkpoint> = Vec::new();
        macro_rules! mutation {
            ($field:ident, $value:expr) => {{
                let mut candidate = next.clone();
                candidate.$field = $value;
                mutations.push(candidate);
            }};
        }

        mutation!(target_id, "target-other".to_owned());
        mutation!(sequence, 2);
        mutation!(previous_checkpoint_sha256, POLICY.to_owned());
        mutation!(checkpoint_sha256, CORPUS.to_owned());
        mutation!(decision_id, "decision-other".to_owned());
        mutation!(challenge_id, "challenge-other".to_owned());
        mutation!(previous_generation, 40);
        mutation!(generation, 43);
        mutation!(deployment_id, "/nix/store/bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbnixos-system-other".to_owned());
        mutation!(approved_artifact_sha256, POLICY.to_owned());
        mutation!(effective_policy_sha256, ARTIFACT.to_owned());
        mutation!(corpus_sha256, ARTIFACT.to_owned());
        mutation!(issued_at_unix_seconds, 1_002);
        mutation!(expires_at_unix_seconds, 1_032);

        assert_eq!(mutations.len(), 14);
        for (index, candidate) in mutations.into_iter().enumerate() {
            local.prepared = Some(candidate);
            assert_ne!(
                reconcile(&local, &witness),
                Reconciliation::WitnessCommittedNeedsLocalFinalize,
                "field mutation {index} was incorrectly treated as safe recovery"
            );
        }

        local.prepared = Some(next);
        assert_eq!(
            reconcile(&local, &witness),
            Reconciliation::WitnessCommittedNeedsLocalFinalize,
            "the exact witness-committed checkpoint must remain recoverable"
        );
    }

    #[test]
    fn one_hundred_thousand_deterministic_two_field_mutations_never_authorize_recovery() {
        let (mut witness, mut local) = initialized();
        let original = checkpoint(
            "target-01",
            1,
            GENESIS,
            ARTIFACT,
            41,
            1_000,
            "decision-100k-mutations",
        );
        local.prepare(original.clone()).unwrap();
        witness.advance_at(&original, 1_001).unwrap();

        // Each trial changes deployment_id plus one additional checkpoint field.
        // Cycle through all other fields, producing reproducible multi-field
        // corruption without relying on a pseudo-random generator or fixture seed.
        for case_id in 0..100_000u64 {
            let mut candidate = original.clone();
            candidate.deployment_id = format!("mutated-deployment-{case_id}");
            match case_id % 13 {
                0 => candidate.target_id = format!("mutated-target-{case_id}"),
                1 => candidate.sequence = candidate.sequence.saturating_add(1),
                2 => candidate.previous_checkpoint_sha256 = POLICY.to_owned(),
                3 => candidate.checkpoint_sha256 = CORPUS.to_owned(),
                4 => candidate.decision_id = format!("mutated-decision-{case_id}"),
                5 => candidate.challenge_id = format!("mutated-challenge-{case_id}"),
                6 => candidate.previous_generation = 40,
                7 => candidate.generation = 43,
                8 => candidate.approved_artifact_sha256 = POLICY.to_owned(),
                9 => candidate.effective_policy_sha256 = ARTIFACT.to_owned(),
                10 => candidate.corpus_sha256 = ARTIFACT.to_owned(),
                11 => candidate.issued_at_unix_seconds = 1_001,
                12 => candidate.expires_at_unix_seconds = 1_031,
                _ => unreachable!("modulo 13 always produces 0..=12"),
            }
            local.prepared = Some(candidate);
            assert_ne!(
                reconcile(&local, &witness),
                Reconciliation::WitnessCommittedNeedsLocalFinalize,
                "mutated case {case_id} was incorrectly treated as safe recovery"
            );
        }

        local.prepared = Some(original);
        assert_eq!(
            reconcile(&local, &witness),
            Reconciliation::WitnessCommittedNeedsLocalFinalize,
            "the exact checkpoint must remain recoverable after the mutation campaign"
        );
    }

    #[test]
    fn two_phase_ordering_never_calls_local_prepare_a_success() {
        let (mut witness, mut local) = initialized();
        let next = checkpoint("target-01", 1, GENESIS, ARTIFACT, 41, 1_000, "decision-3");
        local.prepare(next).unwrap();
        assert_eq!(reconcile(&local, &witness), Reconciliation::PreparedNotWitnessed);
        assert_eq!(witness.tip("target-01").unwrap().sequence, 0);
    }

    #[test]
    fn unavailable_witness_never_allows_a_new_advance() {
        let (mut witness, mut local) = initialized();
        let next = checkpoint("target-01", 1, GENESIS, ARTIFACT, 41, 1_000, "decision-4");
        local.prepare(next.clone()).unwrap();
        witness.set_available(false);
        assert_eq!(witness.advance_at(&next, 1_001), Err(WitnessError::Unavailable));
        assert_eq!(reconcile(&local, &witness), Reconciliation::WitnessUnavailable);
    }

    #[test]
    fn expired_authorization_cannot_advance_witness_but_committed_retry_is_idempotent() {
        let (mut witness, mut local) = initialized();
        let expired = checkpoint("target-01", 1, GENESIS, ARTIFACT, 41, 1_000, "decision-5");
        local.prepare(expired.clone()).unwrap();
        assert_eq!(witness.advance_at(&expired, 1_030), Err(WitnessError::ExpiredAuthorization));
        assert_eq!(witness.tip("target-01").unwrap().sequence, 0);

        let live = checkpoint("target-01", 1, GENESIS, ARTIFACT, 41, 1_000, "decision-6");
        let receipt = witness.advance_at(&live, 1_001).unwrap();
        assert_eq!(witness.advance_at(&live, 99_999).unwrap(), receipt);
    }

    #[test]
    fn exactly_one_of_64_distinct_decisions_wins_same_predecessor_cas() {
        let (witness, _) = initialized();
        let witness = Arc::new(Mutex::new(witness));
        let mut threads = Vec::new();
        for index in 0..64 {
            let shared = Arc::clone(&witness);
            threads.push(thread::spawn(move || {
                let id = format!("race-{index}");
                let digest = format!("{:064x}", index + 100);
                let next = checkpoint("target-01", 1, GENESIS, &digest, 41, 1_000, &id);
                shared.lock().unwrap().advance_at(&next, 1_001).is_ok()
            }));
        }
        let successes = threads
            .into_iter()
            .map(|thread| thread.join().unwrap())
            .filter(|success| *success)
            .count();
        assert_eq!(successes, 1);
        assert_eq!(witness.lock().unwrap().tip("target-01").unwrap().sequence, 1);
    }

    #[test]
    fn stale_predecessor_skipped_sequence_wrong_target_and_conflicting_decision_fail_closed() {
        let (mut witness, _) = initialized();
        let stale = checkpoint("target-01", 1, POLICY, ARTIFACT, 41, 1_000, "decision-7");
        assert_eq!(witness.advance_at(&stale, 1_001), Err(WitnessError::StalePredecessor));
        let skipped = checkpoint("target-01", 2, GENESIS, ARTIFACT, 41, 1_000, "decision-8");
        assert_eq!(witness.advance_at(&skipped, 1_001), Err(WitnessError::InvalidSequence));
        let wrong_target = checkpoint("target-other", 1, GENESIS, ARTIFACT, 41, 1_000, "decision-9");
        assert_eq!(witness.advance_at(&wrong_target, 1_001), Err(WitnessError::UnknownTarget));

        let committed = checkpoint("target-01", 1, GENESIS, ARTIFACT, 41, 1_000, "decision-10");
        witness.advance_at(&committed, 1_001).unwrap();
        let mut conflict = committed.clone();
        conflict.checkpoint_sha256 = CORPUS.into();
        assert_eq!(witness.advance_at(&conflict, 1_002), Err(WitnessError::DecisionReuseConflict));
        assert_eq!(witness.tip("target-01").unwrap().sequence, 1);
    }

    #[test]
    fn identical_local_and_witness_rollback_domains_are_not_an_independent_anchor() {
        let (mut witness, mut local) = initialized();
        let local_backup = local.clone();
        let witness_backup = witness.clone();
        let next = checkpoint("target-01", 1, GENESIS, ARTIFACT, 41, 1_000, "decision-11");
        local.prepare(next.clone()).unwrap();
        let receipt = witness.advance_at(&next, 1_001).unwrap();
        local.finalize(&receipt).unwrap();
        assert_eq!(reconcile(&local, &witness), Reconciliation::Consistent);

        // Deliberately restore BOTH rollback domains. The model cannot detect
        // a root that is itself rolled back; a production witness needs a
        // second, independent monotonic trust boundary.
        local = local_backup;
        witness = witness_backup;
        assert_eq!(reconcile(&local, &witness), Reconciliation::Consistent);
    }

    #[test]
    fn mismatched_receipt_cannot_finalize_local_state() {
        let (mut witness, mut local) = initialized();
        let next = checkpoint("target-01", 1, GENESIS, ARTIFACT, 41, 1_000, "decision-12");
        local.prepare(next.clone()).unwrap();
        let mut receipt = witness.advance_at(&next, 1_001).unwrap();
        receipt.checkpoint_sha256 = CORPUS.into();
        assert_eq!(local.finalize(&receipt), Err(WitnessError::CheckpointConflict));
        assert_eq!(local.generation, 41);
    }
}

