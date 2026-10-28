//! Reference model for idempotency-key retention and replay-horizon safety.
//!
//! Trusted epoch observations enter through a separate adapter hook; effect submission
//! itself cannot supply or advance time. This is an in-memory model and does not prove
//! that a production clock/anchor or receipt store is durable. Its key lifecycle rule is conservative: after receipt retention expires,
//! keep a tombstone and reject that key forever in this namespace. Purging tombstones
//! safely requires a separately proven epoch/namespace-rotation protocol.

use std::collections::HashMap;
use std::sync::Mutex;

use super::TrustedEpoch;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RetentionPolicy {
    /// Maximum accepted lifetime between signed issue and expiry epochs.
    pub max_authorization_lifetime: u64,
    /// Maximum delay for an exact client retry after authorization expiry.
    pub max_retry_horizon: u64,
    /// Additional bound for the verifier's allowed epoch skew/propagation.
    pub epoch_skew_margin: u64,
}

impl RetentionPolicy {
    pub fn validate(self) -> Result<Self, RetentionError> {
        if self.max_authorization_lifetime == 0 {
            return Err(RetentionError::InvalidPolicy(
                "maximum authorization lifetime must be positive",
            ));
        }
        Ok(self)
    }

    pub fn required_retention_until(self, expires_epoch: u64) -> Result<u64, RetentionError> {
        expires_epoch
            .checked_add(self.max_retry_horizon)
            .and_then(|epoch| epoch.checked_add(self.epoch_skew_margin))
            .ok_or(RetentionError::EpochOverflow)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthorizedEffect {
    /// Namespace should include tenant/principal and operation domain.
    pub scope: String,
    /// Key must be unique within scope and must never be intentionally reused.
    pub idempotency_key: String,
    /// Digest is expected to bind all material authorization fields and payload.
    pub request_digest: String,
    pub effect_digest: String,
    pub policy_digest: String,
    pub issued_epoch: u64,
    pub expires_epoch: u64,
    pub fencing_token: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetentionReceipt {
    pub scope: String,
    pub idempotency_key: String,
    pub request_digest: String,
    pub effect_digest: String,
    pub policy_digest: String,
    pub issued_epoch: u64,
    pub expires_epoch: u64,
    pub retain_until_epoch: u64,
    pub fencing_token: u64,
    pub logical_effect_sequence: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RetentionError {
    InvalidPolicy(&'static str),
    InvalidRequest(&'static str),
    NotYetValid,
    AuthorizationExpired,
    AuthorizationLifetimeExceeded,
    EpochRegression,
    TrustedTimeUnavailable,
    EpochOverflow,
    IdempotencyKeyConflict,
    IdempotencyKeyRetired,
    StaleFencingToken,
    LogicalSequenceExhausted,
    LockPoisoned,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SubmitOutcome {
    Applied(RetentionReceipt),
    ExistingReceipt(RetentionReceipt),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StoredIdentity {
    request_digest: String,
    effect_digest: String,
    policy_digest: String,
    issued_epoch: u64,
    expires_epoch: u64,
    fencing_token: u64,
}

impl From<&AuthorizedEffect> for StoredIdentity {
    fn from(request: &AuthorizedEffect) -> Self {
        Self {
            request_digest: request.request_digest.clone(),
            effect_digest: request.effect_digest.clone(),
            policy_digest: request.policy_digest.clone(),
            issued_epoch: request.issued_epoch,
            expires_epoch: request.expires_epoch,
            fencing_token: request.fencing_token,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ActiveRecord {
    identity: StoredIdentity,
    receipt: RetentionReceipt,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Tombstone {
    identity: StoredIdentity,
    retired_at_epoch: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum KeyRecord {
    Active(ActiveRecord),
    Tombstone(Tombstone),
}

#[derive(Default)]
struct State {
    keys: HashMap<(String, String), KeyRecord>,
    latest_fencing_token: HashMap<String, u64>,
    high_water_epoch: u64,
    trusted_epoch_observed: bool,
    logical_effect_count: u64,
}

/// Volatile reference model. The constructor accepts only a valid retention policy.
/// Receipt and key-tombstone changes are atomic under one in-process mutex; this does
/// not imply atomic disk persistence, crash recovery, or cross-process linearizability.
pub struct RetentionSink {
    policy: RetentionPolicy,
    state: Mutex<State>,
}

impl RetentionSink {
    pub fn new(policy: RetentionPolicy) -> Result<Self, RetentionError> {
        Ok(Self {
            policy: policy.validate()?,
            state: Mutex::new(State::default()),
        })
    }

    /// Record a trusted monotonic epoch observation from an independent adapter.
    ///
    /// Treat this as a trusted-adapter hook, never expose it directly to request callers.
    /// A later observation may equal or advance the high-water epoch, never regress it.
    pub fn observe_trusted_epoch(&self, observed: TrustedEpoch) -> Result<(), RetentionError> {
        let mut state = self.state.lock().map_err(|_| RetentionError::LockPoisoned)?;
        let observed_epoch = observed.value();
        if observed_epoch < state.high_water_epoch {
            return Err(RetentionError::EpochRegression);
        }
        state.high_water_epoch = observed_epoch;
        state.trusted_epoch_observed = true;
        Ok(())
    }

    /// Submit using the sink's previously observed trusted monotonic epoch.
    ///
    /// The request cannot supply time, and the sink derives its retention horizon from
    /// policy. Once a key's receipt retention has passed, the payload receipt may be
    /// compacted, but a permanent tombstone blocks reapplication/reuse. A namespace is
    /// the unit of idempotency; using the same token in a different scope is independent.
    pub fn submit(&self, request: &AuthorizedEffect) -> Result<SubmitOutcome, RetentionError> {
        validate_request(request)?;
        let lifetime = request.expires_epoch - request.issued_epoch;
        if lifetime > self.policy.max_authorization_lifetime {
            return Err(RetentionError::AuthorizationLifetimeExceeded);
        }
        let retain_until_epoch = self
            .policy
            .required_retention_until(request.expires_epoch)?;
        let mut state = self.state.lock().map_err(|_| RetentionError::LockPoisoned)?;
        if !state.trusted_epoch_observed {
            return Err(RetentionError::TrustedTimeUnavailable);
        }
        let now_epoch = state.high_water_epoch;

        let key = (request.scope.clone(), request.idempotency_key.clone());
        if let Some(record) = state.keys.get_mut(&key) {
            match record {
                KeyRecord::Tombstone(_) => return Err(RetentionError::IdempotencyKeyRetired),
                KeyRecord::Active(active) => {
                    if now_epoch > active.receipt.retain_until_epoch {
                        let tombstone = Tombstone {
                            identity: active.identity.clone(),
                            retired_at_epoch: now_epoch,
                        };
                        *record = KeyRecord::Tombstone(tombstone);
                        return Err(RetentionError::IdempotencyKeyRetired);
                    }
                    if active.identity != StoredIdentity::from(request) {
                        return Err(RetentionError::IdempotencyKeyConflict);
                    }
                    return Ok(SubmitOutcome::ExistingReceipt(active.receipt.clone()));
                }
            }
        }

        // A first application requires a currently valid authorization. Replays of a
        // retained exact key above can only return a prior receipt; they cannot apply
        // a new effect even if the authorization has since expired.
        if now_epoch < request.issued_epoch {
            return Err(RetentionError::NotYetValid);
        }
        if now_epoch >= request.expires_epoch {
            return Err(RetentionError::AuthorizationExpired);
        }

        let latest_token = state
            .latest_fencing_token
            .get(&request.scope)
            .copied()
            .unwrap_or(0);
        if request.fencing_token < latest_token {
            return Err(RetentionError::StaleFencingToken);
        }
        let logical_effect_sequence = state
            .logical_effect_count
            .checked_add(1)
            .ok_or(RetentionError::LogicalSequenceExhausted)?;
        let receipt = RetentionReceipt {
            scope: request.scope.clone(),
            idempotency_key: request.idempotency_key.clone(),
            request_digest: request.request_digest.clone(),
            effect_digest: request.effect_digest.clone(),
            policy_digest: request.policy_digest.clone(),
            issued_epoch: request.issued_epoch,
            expires_epoch: request.expires_epoch,
            retain_until_epoch,
            fencing_token: request.fencing_token,
            logical_effect_sequence,
        };
        state.logical_effect_count = logical_effect_sequence;
        state
            .latest_fencing_token
            .entry(request.scope.clone())
            .and_modify(|current| *current = (*current).max(request.fencing_token))
            .or_insert(request.fencing_token);
        state.keys.insert(
            key,
            KeyRecord::Active(ActiveRecord {
                identity: StoredIdentity::from(request),
                receipt: receipt.clone(),
            }),
        );
        Ok(SubmitOutcome::Applied(receipt))
    }

    pub fn logical_effect_count(&self) -> Result<u64, RetentionError> {
        Ok(self
            .state
            .lock()
            .map_err(|_| RetentionError::LockPoisoned)?
            .logical_effect_count)
    }

    pub fn high_water_epoch(&self) -> Result<u64, RetentionError> {
        Ok(self
            .state
            .lock()
            .map_err(|_| RetentionError::LockPoisoned)?
            .high_water_epoch)
    }

    pub fn is_tombstoned(
        &self,
        scope: &str,
        idempotency_key: &str,
    ) -> Result<bool, RetentionError> {
        let state = self.state.lock().map_err(|_| RetentionError::LockPoisoned)?;
        Ok(matches!(
            state
                .keys
                .get(&(scope.to_string(), idempotency_key.to_string())),
            Some(KeyRecord::Tombstone(tombstone)) if tombstone.retired_at_epoch > 0
        ))
    }
}

fn validate_request(request: &AuthorizedEffect) -> Result<(), RetentionError> {
    if request.scope.is_empty() {
        return Err(RetentionError::InvalidRequest("scope must not be empty"));
    }
    if request.idempotency_key.is_empty() {
        return Err(RetentionError::InvalidRequest(
            "idempotency key must not be empty",
        ));
    }
    for (name, value) in [
        ("request_digest", request.request_digest.as_str()),
        ("effect_digest", request.effect_digest.as_str()),
        ("policy_digest", request.policy_digest.as_str()),
    ] {
        if !is_lower_hex_digest(value) {
            return Err(RetentionError::InvalidRequest(match name {
                "request_digest" => "request digest must be 64 lowercase hexadecimal characters",
                "effect_digest" => "effect digest must be 64 lowercase hexadecimal characters",
                _ => "policy digest must be 64 lowercase hexadecimal characters",
            }));
        }
    }
    if request.expires_epoch <= request.issued_epoch {
        return Err(RetentionError::InvalidRequest(
            "expiry epoch must be later than issue epoch",
        ));
    }
    if request.fencing_token == 0 {
        return Err(RetentionError::InvalidRequest(
            "fencing token must be positive",
        ));
    }
    Ok(())
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
    use std::sync::{Arc, Barrier};
    use std::thread;

    fn digest(byte: u8) -> String {
        format!("{byte:02x}").repeat(32)
    }

    fn policy() -> RetentionPolicy {
        RetentionPolicy {
            max_authorization_lifetime: 10,
            max_retry_horizon: 20,
            epoch_skew_margin: 5,
        }
    }

    fn request(
        scope: &str,
        key: &str,
        request_byte: u8,
        effect_byte: u8,
        policy_byte: u8,
        token: u64,
    ) -> AuthorizedEffect {
        AuthorizedEffect {
            scope: scope.to_string(),
            idempotency_key: key.to_string(),
            request_digest: digest(request_byte),
            effect_digest: digest(effect_byte),
            policy_digest: digest(policy_byte),
            issued_epoch: 10,
            expires_epoch: 20,
            fencing_token: token,
        }
    }

    fn sink() -> RetentionSink {
        RetentionSink::new(policy()).unwrap()
    }

    fn submit_at(
        sink: &RetentionSink,
        request: &AuthorizedEffect,
        trusted_epoch: u64,
    ) -> Result<SubmitOutcome, RetentionError> {
        sink.observe_trusted_epoch(TrustedEpoch::synthetic_for_tests(trusted_epoch))?;
        sink.submit(request)
    }

    #[test]
    fn submit_fails_closed_before_first_trusted_time_observation() {
        let sink = sink();
        let request = request("tenant-a:deploy", "op-before-time", 1, 2, 3, 7);
        assert_eq!(
            sink.submit(&request),
            Err(RetentionError::TrustedTimeUnavailable)
        );
        assert_eq!(sink.logical_effect_count().unwrap(), 0);
    }

    #[test]
    fn receipt_retention_includes_retry_horizon_and_skew_margin() {
        let sink = sink();
        let request = request("tenant-a:deploy", "op-001", 1, 2, 3, 7);
        let SubmitOutcome::Applied(receipt) = submit_at(&sink, &request, 12).unwrap() else {
            panic!("first request must apply");
        };
        assert_eq!(receipt.retain_until_epoch, 45);
    }

    #[test]
    fn same_key_retry_returns_receipt_without_reapplying_even_after_auth_expiry() {
        let sink = sink();
        let request = request("tenant-a:deploy", "op-001", 1, 2, 3, 7);
        let SubmitOutcome::Applied(first) = submit_at(&sink, &request, 12).unwrap() else {
            panic!("first request must apply");
        };
        let replay = submit_at(&sink, &request, 25).unwrap();
        assert_eq!(replay, SubmitOutcome::ExistingReceipt(first));
        assert_eq!(sink.logical_effect_count().unwrap(), 1);
    }

    #[test]
    fn below_horizon_receipt_is_not_created_by_request_input() {
        // The request has no retention field; the sink derives the minimum horizon.
        let sink = sink();
        let request = request("tenant-a:deploy", "op-001", 1, 2, 3, 7);
        let SubmitOutcome::Applied(receipt) = submit_at(&sink, &request, 12).unwrap() else {
            panic!("first request must apply");
        };
        assert_eq!(receipt.retain_until_epoch, 45);
        assert!(receipt.retain_until_epoch >= request.expires_epoch + 20 + 5);
    }

    #[test]
    fn expired_receipt_becomes_tombstone_and_cannot_be_reapplied() {
        let sink = sink();
        let request = request("tenant-a:deploy", "op-001", 1, 2, 3, 7);
        submit_at(&sink, &request, 12).unwrap();
        assert_eq!(
            submit_at(&sink, &request, 46),
            Err(RetentionError::IdempotencyKeyRetired)
        );
        assert!(sink.is_tombstoned("tenant-a:deploy", "op-001").unwrap());
        assert_eq!(
            submit_at(&sink, &request, 47),
            Err(RetentionError::IdempotencyKeyRetired)
        );
        assert_eq!(sink.logical_effect_count().unwrap(), 1);
    }

    #[test]
    fn retired_key_cannot_be_reused_for_different_intent() {
        let sink = sink();
        let original = request("tenant-a:deploy", "op-001", 1, 2, 3, 7);
        submit_at(&sink, &original, 12).unwrap();
        assert_eq!(
            submit_at(&sink, &original, 46),
            Err(RetentionError::IdempotencyKeyRetired)
        );
        let changed = request("tenant-a:deploy", "op-001", 9, 8, 3, 7);
        assert_eq!(
            submit_at(&sink, &changed, 47),
            Err(RetentionError::IdempotencyKeyRetired)
        );
        assert_eq!(sink.logical_effect_count().unwrap(), 1);
    }

    #[test]
    fn key_conflict_is_rejected_while_receipt_is_retained() {
        let sink = sink();
        submit_at(&sink, &request("tenant-a:deploy", "op-001", 1, 2, 3, 7), 12).unwrap();
        assert_eq!(
            submit_at(&sink, &request("tenant-a:deploy", "op-001", 9, 2, 3, 7), 13),
            Err(RetentionError::IdempotencyKeyConflict)
        );
        assert_eq!(sink.logical_effect_count().unwrap(), 1);
    }

    #[test]
    fn first_application_after_expiry_is_rejected() {
        let sink = sink();
        assert_eq!(
            submit_at(&sink, &request("tenant-a:deploy", "op-new", 1, 2, 3, 7), 20),
            Err(RetentionError::AuthorizationExpired)
        );
        assert_eq!(sink.logical_effect_count().unwrap(), 0);
    }

    #[test]
    fn excessive_authorization_lifetime_is_rejected() {
        let sink = sink();
        let mut request = request("tenant-a:deploy", "op-long", 1, 2, 3, 7);
        request.expires_epoch = 21;
        assert_eq!(
            submit_at(&sink, &request, 12),
            Err(RetentionError::AuthorizationLifetimeExceeded)
        );
        assert_eq!(sink.logical_effect_count().unwrap(), 0);
    }

    #[test]
    fn monotonic_epoch_regression_is_rejected() {
        let sink = sink();
        sink.observe_trusted_epoch(TrustedEpoch::synthetic_for_tests(12)).unwrap();
        sink.submit(&request("tenant-a:deploy", "op-001", 1, 2, 3, 7))
            .unwrap();
        assert_eq!(
            sink.observe_trusted_epoch(TrustedEpoch::synthetic_for_tests(11)),
            Err(RetentionError::EpochRegression)
        );
        assert_eq!(sink.high_water_epoch().unwrap(), 12);
    }

    #[test]
    fn stale_fencing_token_cannot_apply_new_key() {
        let sink = sink();
        submit_at(&sink, &request("tenant-a:deploy", "op-001", 1, 2, 3, 8), 12).unwrap();
        assert_eq!(
            submit_at(&sink, &request("tenant-a:deploy", "op-002", 4, 5, 3, 7), 13),
            Err(RetentionError::StaleFencingToken)
        );
        assert_eq!(sink.logical_effect_count().unwrap(), 1);
    }

    #[test]
    fn identical_key_in_separate_scopes_is_independent() {
        let sink = sink();
        submit_at(&sink, &request("tenant-a:deploy", "op-001", 1, 2, 3, 1), 12).unwrap();
        submit_at(&sink, &request("tenant-b:deploy", "op-001", 1, 2, 3, 1), 12).unwrap();
        assert_eq!(sink.logical_effect_count().unwrap(), 2);
    }

    #[test]
    fn concurrent_same_key_submissions_apply_one_effect() {
        const WORKERS: usize = 64;
        let sink = Arc::new(sink());
        let barrier = Arc::new(Barrier::new(WORKERS));
        let mut workers = Vec::with_capacity(WORKERS);
        for _ in 0..WORKERS {
            let sink = Arc::clone(&sink);
            let barrier = Arc::clone(&barrier);
            workers.push(thread::spawn(move || {
                let request = request("tenant-a:deploy", "op-race", 1, 2, 3, 7);
                barrier.wait();
                submit_at(&sink, &request, 12).unwrap()
            }));
        }
        let results: Vec<_> = workers
            .into_iter()
            .map(|worker| worker.join().expect("worker must not panic"))
            .collect();
        assert!(results.iter().all(|r| r == &results[0]));
        assert_eq!(sink.logical_effect_count().unwrap(), 1);
    }

    #[test]
    fn epoch_overflow_blocks_retention_calculation() {
        let sink = RetentionSink::new(RetentionPolicy {
            max_authorization_lifetime: 10,
            max_retry_horizon: u64::MAX,
            epoch_skew_margin: 5,
        })
        .unwrap();
        let request = request("tenant-a:deploy", "op-overflow", 1, 2, 3, 7);
        assert_eq!(submit_at(&sink, &request, 12), Err(RetentionError::EpochOverflow));
        assert_eq!(sink.logical_effect_count().unwrap(), 0);
    }
}
