//! Dependency-free model for a 3f+1 external anti-rollback witness quorum.
//!
//! This module models checkpoint binding, distinct-voter quorum accounting and
//! honest-witness non-equivocation. It DOES NOT implement signatures, network
//! consensus, durable witness storage, TPM NV, or a trusted clock. Every Vote
//! supplied to verify_certificate must already have been authenticated by a
//! production cryptographic verifier. CI results from this crate are model-tier
//! protocol tests, not production anti-rollback qualification.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuorumPolicy {
    members: BTreeSet<String>,
    max_byzantine: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tip {
    pub target_id: String,
    pub sequence: u64,
    pub checkpoint_sha256: String,
    pub generation: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Checkpoint {
    pub target_id: String,
    pub sequence: u64,
    pub previous_checkpoint_sha256: String,
    pub checkpoint_sha256: String,
    pub previous_generation: u64,
    pub generation: u64,
    pub decision_id: String,
    pub deployment_id: String,
    pub issued_at_unix_seconds: u64,
    pub expires_at_unix_seconds: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Vote {
    pub signer_id: String,
    pub checkpoint: Checkpoint,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Certificate {
    pub checkpoint: Checkpoint,
    pub votes: Vec<Vote>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QuorumError {
    InvalidMembership,
    InvalidIdentifier(&'static str),
    InvalidDigest,
    InvalidDeployment,
    InvalidGeneration,
    InvalidSequence,
    StalePredecessor,
    ExpiredAuthorization,
    UnknownWitness,
    DuplicateWitness,
    InsufficientQuorum { have: usize, need: usize },
    VoteCheckpointMismatch,
    EquivocationRefused,
    LocalTipConflict,
}

impl QuorumPolicy {
    pub fn new<I, S>(members: I, max_byzantine: usize) -> Result<Self, QuorumError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut trusted = BTreeSet::new();
        for member in members {
            let member = member.into();
            if !valid_identifier(&member) {
                return Err(QuorumError::InvalidIdentifier("witness id"));
            }
            trusted.insert(member);
        }
        // Byzantine quorum intersection needs n >= 3f + 1 and q = 2f + 1.
        if trusted.len() < max_byzantine.saturating_mul(3).saturating_add(1) {
            return Err(QuorumError::InvalidMembership);
        }
        Ok(Self { members: trusted, max_byzantine })
    }

    pub fn member_count(&self) -> usize { self.members.len() }
    pub fn max_byzantine(&self) -> usize { self.max_byzantine }
    pub fn quorum_size(&self) -> usize { self.max_byzantine.saturating_mul(2).saturating_add(1) }
    pub fn is_trusted(&self, witness_id: &str) -> bool { self.members.contains(witness_id) }
}

impl Checkpoint {
    pub fn validate(&self, current: &Tip, now_unix_seconds: u64) -> Result<(), QuorumError> {
        if !valid_identifier(&self.target_id) {
            return Err(QuorumError::InvalidIdentifier("target"));
        }
        if !valid_identifier(&self.decision_id) {
            return Err(QuorumError::InvalidIdentifier("decision"));
        }
        if !valid_digest(&self.previous_checkpoint_sha256) || !valid_digest(&self.checkpoint_sha256) {
            return Err(QuorumError::InvalidDigest);
        }
        if !valid_deployment_id(&self.deployment_id) {
            return Err(QuorumError::InvalidDeployment);
        }
        if self.previous_generation.checked_add(1) != Some(self.generation) {
            return Err(QuorumError::InvalidGeneration);
        }
        if current.target_id != self.target_id {
            return Err(QuorumError::StalePredecessor);
        }
        if current.sequence.checked_add(1) != Some(self.sequence) {
            return Err(QuorumError::InvalidSequence);
        }
        if self.previous_checkpoint_sha256 != current.checkpoint_sha256 {
            return Err(QuorumError::StalePredecessor);
        }
        if self.previous_generation != current.generation {
            return Err(QuorumError::InvalidGeneration);
        }
        if self.issued_at_unix_seconds >= self.expires_at_unix_seconds
            || now_unix_seconds < self.issued_at_unix_seconds
            || now_unix_seconds >= self.expires_at_unix_seconds
        {
            return Err(QuorumError::ExpiredAuthorization);
        }
        Ok(())
    }
}

/// Verify quorum structure over votes whose signatures were verified elsewhere.
/// This model checks membership, uniqueness, exact checkpoint binding and quorum
/// size; it must never be used as the cryptographic vote verifier.
pub fn verify_certificate(
    policy: &QuorumPolicy,
    current: &Tip,
    certificate: &Certificate,
    now_unix_seconds: u64,
) -> Result<Tip, QuorumError> {
    certificate.checkpoint.validate(current, now_unix_seconds)?;
    let need = policy.quorum_size();
    let mut seen = BTreeSet::new();
    for vote in &certificate.votes {
        if !policy.is_trusted(&vote.signer_id) {
            return Err(QuorumError::UnknownWitness);
        }
        if !seen.insert(vote.signer_id.clone()) {
            return Err(QuorumError::DuplicateWitness);
        }
        if vote.checkpoint != certificate.checkpoint {
            return Err(QuorumError::VoteCheckpointMismatch);
        }
    }
    if seen.len() < need {
        return Err(QuorumError::InsufficientQuorum { have: seen.len(), need });
    }
    Ok(Tip {
        target_id: certificate.checkpoint.target_id.clone(),
        sequence: certificate.checkpoint.sequence,
        checkpoint_sha256: certificate.checkpoint.checkpoint_sha256.clone(),
        generation: certificate.checkpoint.generation,
    })
}

/// Single honest witness state. A witness signs no more than one distinct
/// checkpoint for a target/sequence pair. Repeating an identical vote is
/// idempotent. Production storage must persist the vote before returning a
/// signature; memory state alone is rollbackable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HonestWitness {
    pub id: String,
    tips: BTreeMap<String, Tip>,
    voted: BTreeMap<(String, u64), Checkpoint>,
}

impl HonestWitness {
    pub fn new(id: impl Into<String>) -> Result<Self, QuorumError> {
        let id = id.into();
        if !valid_identifier(&id) {
            return Err(QuorumError::InvalidIdentifier("witness id"));
        }
        Ok(Self { id, tips: BTreeMap::new(), voted: BTreeMap::new() })
    }

    pub fn initialize(&mut self, tip: Tip) -> Result<(), QuorumError> {
        if !valid_identifier(&tip.target_id) {
            return Err(QuorumError::InvalidIdentifier("target"));
        }
        if !valid_digest(&tip.checkpoint_sha256) {
            return Err(QuorumError::InvalidDigest);
        }
        if self.tips.contains_key(&tip.target_id) {
            return Err(QuorumError::LocalTipConflict);
        }
        self.tips.insert(tip.target_id.clone(), tip);
        Ok(())
    }

    pub fn tip(&self, target_id: &str) -> Option<&Tip> { self.tips.get(target_id) }

    pub fn vote(&mut self, checkpoint: &Checkpoint, now_unix_seconds: u64) -> Result<Vote, QuorumError> {
        let tip = self.tips.get(&checkpoint.target_id).ok_or(QuorumError::StalePredecessor)?;
        checkpoint.validate(tip, now_unix_seconds)?;
        let key = (checkpoint.target_id.clone(), checkpoint.sequence);
        if let Some(previous) = self.voted.get(&key) {
            if previous != checkpoint {
                return Err(QuorumError::EquivocationRefused);
            }
            return Ok(Vote { signer_id: self.id.clone(), checkpoint: previous.clone() });
        }
        self.voted.insert(key, checkpoint.clone());
        Ok(Vote { signer_id: self.id.clone(), checkpoint: checkpoint.clone() })
    }

    pub fn observe_certificate(
        &mut self,
        policy: &QuorumPolicy,
        certificate: &Certificate,
        now_unix_seconds: u64,
    ) -> Result<(), QuorumError> {
        let current = self.tips.get(&certificate.checkpoint.target_id)
            .ok_or(QuorumError::StalePredecessor)?;
        if self.voted.get(&(certificate.checkpoint.target_id.clone(), certificate.checkpoint.sequence))
            .is_some_and(|voted| voted != &certificate.checkpoint)
        {
            return Err(QuorumError::EquivocationRefused);
        }
        let next = verify_certificate(policy, current, certificate, now_unix_seconds)?;
        self.tips.insert(next.target_id.clone(), next);
        Ok(())
    }
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty() && value.len() <= 128 && value.bytes().all(|b| {
        b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-')
    })
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn valid_deployment_id(value: &str) -> bool {
    if valid_digest(value) { return true; }
    let Some(rest) = value.strip_prefix("/nix/store/") else { return false; };
    let Some((hash, name)) = rest.split_once('-') else { return false; };
    hash.len() == 32
        && hash.bytes().all(|b| b.is_ascii_digit() || b"abcdfghijklmnpqrsvwxyz".contains(&b))
        && !name.is_empty()
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'+' | b'-'))
}

#[cfg(test)]
mod tests {
    use super::*;

    const GENESIS: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const A: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const B: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const C: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
    const D: &str = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
    const DEPLOYMENT: &str = "/nix/store/0123456789abcdfghijklmnpqrsvwxyz-srq-quorum-model";

    fn policy() -> QuorumPolicy { QuorumPolicy::new(["w1", "w2", "w3", "w4"], 1).unwrap() }
    fn tip() -> Tip {
        Tip { target_id: "target-1".into(), sequence: 0, checkpoint_sha256: GENESIS.into(), generation: 41 }
    }
    fn checkpoint(digest: &str, decision: &str, now: u64) -> Checkpoint {
        Checkpoint {
            target_id: "target-1".into(), sequence: 1,
            previous_checkpoint_sha256: GENESIS.into(), checkpoint_sha256: digest.into(),
            previous_generation: 41, generation: 42, decision_id: decision.into(),
            deployment_id: DEPLOYMENT.into(), issued_at_unix_seconds: now, expires_at_unix_seconds: now + 60,
        }
    }
    fn vote(id: &str, checkpoint: &Checkpoint) -> Vote {
        Vote { signer_id: id.into(), checkpoint: checkpoint.clone() }
    }
    fn certificate(checkpoint: Checkpoint, ids: &[&str]) -> Certificate {
        Certificate { votes: ids.iter().map(|id| vote(id, &checkpoint)).collect(), checkpoint }
    }

    #[test]
    fn policy_requires_three_f_plus_one_members() {
        assert_eq!(QuorumPolicy::new(["a", "b", "c"], 1), Err(QuorumError::InvalidMembership));
        assert_eq!(policy().quorum_size(), 3);
    }

    #[test]
    fn only_three_distinct_trusted_matching_votes_form_a_certificate() {
        let p = policy();
        let current = tip();
        let candidate = checkpoint(A, "decision-1", 1_000);
        assert!(verify_certificate(&p, &current, &certificate(candidate.clone(), &["w1", "w2"]), 1_001).is_err());
        assert_eq!(verify_certificate(&p, &current, &certificate(candidate.clone(), &["w1", "w2", "w3"]), 1_001).unwrap().sequence, 1);
    }

    #[test]
    fn duplicate_or_unknown_voters_and_mismatched_votes_fail_closed() {
        let p = policy();
        let current = tip();
        let candidate = checkpoint(A, "decision-2", 1_000);
        assert_eq!(verify_certificate(&p, &current, &certificate(candidate.clone(), &["w1", "w1", "w2"]), 1_001), Err(QuorumError::DuplicateWitness));
        assert_eq!(verify_certificate(&p, &current, &certificate(candidate.clone(), &["w1", "w2", "rogue"]), 1_001), Err(QuorumError::UnknownWitness));
        let mut mixed = certificate(candidate.clone(), &["w1", "w2", "w3"]);
        mixed.votes[2].checkpoint.checkpoint_sha256 = B.into();
        assert_eq!(verify_certificate(&p, &current, &mixed, 1_001), Err(QuorumError::VoteCheckpointMismatch));
    }

    #[test]
    fn stale_or_expired_checkpoints_cannot_form_a_certificate() {
        let p = policy();
        let mut old = checkpoint(A, "decision-3", 1_000);
        old.previous_checkpoint_sha256 = C.into();
        assert_eq!(verify_certificate(&p, &tip(), &certificate(old, &["w1", "w2", "w3"]), 1_001), Err(QuorumError::StalePredecessor));
        let expired = checkpoint(A, "decision-expired", 1_000);
        assert_eq!(verify_certificate(&p, &tip(), &certificate(expired, &["w1", "w2", "w3"]), 1_060), Err(QuorumError::ExpiredAuthorization));
    }

    #[test]
    fn honest_witness_is_idempotent_and_refuses_equivocation() {
        let mut witness = HonestWitness::new("w1").unwrap();
        witness.initialize(tip()).unwrap();
        let first = checkpoint(A, "decision-4", 1_000);
        let second = checkpoint(B, "decision-5", 1_000);
        let vote1 = witness.vote(&first, 1_001).unwrap();
        assert_eq!(witness.vote(&first, 1_002).unwrap(), vote1);
        assert_eq!(witness.vote(&second, 1_002), Err(QuorumError::EquivocationRefused));
    }

    #[test]
    fn conflicting_certificates_cannot_both_reach_quorum_with_one_byzantine_witness() {
        let p = policy();
        let mut w1 = HonestWitness::new("w1").unwrap();
        let mut w2 = HonestWitness::new("w2").unwrap();
        let mut w3 = HonestWitness::new("w3").unwrap();
        let mut w4 = HonestWitness::new("w4").unwrap();
        for w in [&mut w1, &mut w2, &mut w3, &mut w4] { w.initialize(tip()).unwrap(); }
        let a = checkpoint(A, "decision-a", 1_000);
        let b = checkpoint(B, "decision-b", 1_000);
        let cert_a = Certificate {
            checkpoint: a.clone(),
            votes: vec![w1.vote(&a, 1_001).unwrap(), w2.vote(&a, 1_001).unwrap(), w3.vote(&a, 1_001).unwrap()],
        };
        assert!(verify_certificate(&p, &tip(), &cert_a, 1_001).is_ok());
        // Simulate w1 as the sole Byzantine equivocator; honest w2 and w3 refuse
        // the second checkpoint, leaving only w1's Byzantine vote and w4's vote.
        let mut votes_b = vec![vote("w1", &b)];
        if let Ok(v) = w2.vote(&b, 1_001) { votes_b.push(v); }
        if let Ok(v) = w3.vote(&b, 1_001) { votes_b.push(v); }
        if let Ok(v) = w4.vote(&b, 1_001) { votes_b.push(v); }
        let cert_b = Certificate { checkpoint: b, votes: votes_b };
        assert_eq!(verify_certificate(&p, &tip(), &cert_b, 1_001), Err(QuorumError::InsufficientQuorum { have: 2, need: 3 }));
    }

    #[test]
    fn all_three_of_four_quorums_intersect_in_at_least_f_plus_one_members() {
        let members = ["w1", "w2", "w3", "w4"];
        let subsets: Vec<Vec<&str>> = (0..members.len())
            .flat_map(|a| ((a + 1)..members.len()).flat_map(move |b| ((b + 1)..members.len()).map(move |c| vec![members[a], members[b], members[c]])))
            .collect();
        assert_eq!(subsets.len(), 4);
        for left in &subsets {
            for right in &subsets {
                let intersection = left.iter().filter(|id| right.contains(id)).count();
                assert!(intersection >= 2, "quorum intersection too small: {left:?} vs {right:?}");
            }
        }
    }

    #[test]
    fn certificate_cannot_advance_wrong_target_or_skip_sequence() {
        let p = policy();
        let current = tip();
        let mut wrong_target = checkpoint(A, "decision-6", 1_000);
        wrong_target.target_id = "target-other".into();
        assert!(verify_certificate(&p, &current, &certificate(wrong_target, &["w1", "w2", "w3"]), 1_001).is_err());
        let mut skipped = checkpoint(A, "decision-7", 1_000);
        skipped.sequence = 2;
        assert_eq!(verify_certificate(&p, &current, &certificate(skipped, &["w1", "w2", "w3"]), 1_001), Err(QuorumError::InvalidSequence));
    }

    #[test]
    fn witness_cannot_finalize_a_conflicting_tip() {
        let p = policy();
        let mut witness = HonestWitness::new("w1").unwrap();
        witness.initialize(tip()).unwrap();
        let a = checkpoint(A, "decision-8", 1_000);
        let b = checkpoint(B, "decision-9", 1_000);
        let cert = certificate(a.clone(), &["w1", "w2", "w3"]);
        assert!(witness.observe_certificate(&p, &cert, 1_001).is_ok());
        assert_eq!(witness.tip("target-1").unwrap().checkpoint_sha256, A);
        let conflict = certificate(b, &["w1", "w2", "w3"]);
        assert!(witness.observe_certificate(&p, &conflict, 1_001).is_err());
    }

    #[test]
    fn quorum_state_isolated_by_target() {
        let mut witness = HonestWitness::new("w1").unwrap();
        witness.initialize(tip()).unwrap();
        let second_tip = Tip { target_id: "target-2".into(), sequence: 0, checkpoint_sha256: D.into(), generation: 8 };
        witness.initialize(second_tip).unwrap();
        let next = Checkpoint {
            target_id: "target-2".into(), sequence: 1, previous_checkpoint_sha256: D.into(),
            checkpoint_sha256: A.into(), previous_generation: 8, generation: 9,
            decision_id: "decision-target-2".into(), deployment_id: DEPLOYMENT.into(),
            issued_at_unix_seconds: 1_000, expires_at_unix_seconds: 1_060,
        };
        assert_eq!(witness.vote(&next, 1_001).unwrap().checkpoint.target_id, "target-2");
        assert_eq!(witness.tip("target-1").unwrap().generation, 41);
    }
}
