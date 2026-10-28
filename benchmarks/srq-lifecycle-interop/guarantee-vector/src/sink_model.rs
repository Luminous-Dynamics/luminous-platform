//! In-memory reference sink for testing idempotency-key and fencing semantics.
//!
//! This is deliberately not a persistent or production sink. The mutex makes the
//! effect-and-receipt transition atomic only within this process and does not survive
//! crashes. Production adapters must implement equivalent semantics in their actual
//! transaction/persistence domain and pass crash/restore tests.

use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectRequest {
    /// Namespace that separates keys between tenants/principals and operation domains.
    pub scope: String,
    pub idempotency_key: String,
    pub request_digest: String,
    pub effect_digest: String,
    /// Monotonic token issued by the caller's trusted fencing authority.
    pub fencing_token: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SinkReceipt {
    pub scope: String,
    pub idempotency_key: String,
    pub request_digest: String,
    pub effect_digest: String,
    pub fencing_token: u64,
    /// Count of unique logical effects accepted by this in-memory sink.
    pub logical_effect_sequence: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SinkError {
    InvalidRequest(&'static str),
    IdempotencyKeyConflict,
    StaleFencingToken,
    LogicalSequenceExhausted,
    LockPoisoned,
}

#[derive(Default)]
struct SinkState {
    receipts: HashMap<(String, String), SinkReceipt>,
    latest_fencing_token: HashMap<String, u64>,
    logical_effect_count: u64,
}

/// A single-process reference sink. submit atomically deduplicates an exact scoped
/// idempotency key and records the logical effect plus receipt under one mutex.
#[derive(Default)]
pub struct AtomicSink {
    state: Mutex<SinkState>,
}

impl AtomicSink {
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply one logical effect, or return the existing receipt for an exact retry.
    ///
    /// Reusing the same (scope, key) for a different request/effect digest is rejected.
    /// Existing exact-key retries return the previously recorded receipt before checking
    /// the current fencing high-water mark: this path cannot apply another effect.
    /// A new key is rejected when its token is lower than the highest token already
    /// accepted for that scope.
    pub fn submit(&self, request: &EffectRequest) -> Result<SinkReceipt, SinkError> {
        validate_request(request)?;
        let mut state = self.state.lock().map_err(|_| SinkError::LockPoisoned)?;
        let key = (request.scope.clone(), request.idempotency_key.clone());

        if let Some(existing) = state.receipts.get(&key) {
            if existing.request_digest != request.request_digest
                || existing.effect_digest != request.effect_digest
            {
                return Err(SinkError::IdempotencyKeyConflict);
            }
            return Ok(existing.clone());
        }

        let latest_token = state
            .latest_fencing_token
            .get(&request.scope)
            .copied()
            .unwrap_or(0);
        if request.fencing_token < latest_token {
            return Err(SinkError::StaleFencingToken);
        }

        let sequence = state
            .logical_effect_count
            .checked_add(1)
            .ok_or(SinkError::LogicalSequenceExhausted)?;
        let receipt = SinkReceipt {
            scope: request.scope.clone(),
            idempotency_key: request.idempotency_key.clone(),
            request_digest: request.request_digest.clone(),
            effect_digest: request.effect_digest.clone(),
            fencing_token: request.fencing_token,
            logical_effect_sequence: sequence,
        };

        // These updates occur in one critical section. This establishes only process-local
        // atomicity; it is not evidence of crash durability or a disk transaction.
        state.logical_effect_count = sequence;
        state
            .latest_fencing_token
            .entry(request.scope.clone())
            .and_modify(|current| *current = (*current).max(request.fencing_token))
            .or_insert(request.fencing_token);
        state.receipts.insert(key, receipt.clone());
        Ok(receipt)
    }

    pub fn logical_effect_count(&self) -> Result<u64, SinkError> {
        let state = self.state.lock().map_err(|_| SinkError::LockPoisoned)?;
        Ok(state.logical_effect_count)
    }

    pub fn latest_fencing_token(&self, scope: &str) -> Result<Option<u64>, SinkError> {
        let state = self.state.lock().map_err(|_| SinkError::LockPoisoned)?;
        Ok(state.latest_fencing_token.get(scope).copied())
    }
}

fn validate_request(request: &EffectRequest) -> Result<(), SinkError> {
    if request.scope.is_empty() {
        return Err(SinkError::InvalidRequest("scope must not be empty"));
    }
    if request.idempotency_key.is_empty() {
        return Err(SinkError::InvalidRequest(
            "idempotency key must not be empty",
        ));
    }
    if !is_lower_hex_digest(&request.request_digest) {
        return Err(SinkError::InvalidRequest(
            "request digest must be 64 lowercase hexadecimal characters",
        ));
    }
    if !is_lower_hex_digest(&request.effect_digest) {
        return Err(SinkError::InvalidRequest(
            "effect digest must be 64 lowercase hexadecimal characters",
        ));
    }
    if request.fencing_token == 0 {
        return Err(SinkError::InvalidRequest("fencing token must be positive"));
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

    fn request(
        scope: &str,
        key: &str,
        request_byte: u8,
        effect_byte: u8,
        token: u64,
    ) -> EffectRequest {
        EffectRequest {
            scope: scope.to_string(),
            idempotency_key: key.to_string(),
            request_digest: digest(request_byte),
            effect_digest: digest(effect_byte),
            fencing_token: token,
        }
    }

    #[test]
    fn exact_retry_returns_same_receipt_and_only_one_effect() {
        let sink = AtomicSink::new();
        let first = sink
            .submit(&request("tenant-a:deploy", "op-001", 1, 2, 7))
            .unwrap();
        let second = sink
            .submit(&request("tenant-a:deploy", "op-001", 1, 2, 7))
            .unwrap();
        assert_eq!(first, second);
        assert_eq!(sink.logical_effect_count().unwrap(), 1);
    }

    #[test]
    fn same_scoped_key_with_different_request_or_effect_is_rejected() {
        let sink = AtomicSink::new();
        sink.submit(&request("tenant-a:deploy", "op-001", 1, 2, 7))
            .unwrap();
        assert_eq!(
            sink.submit(&request("tenant-a:deploy", "op-001", 9, 2, 7)),
            Err(SinkError::IdempotencyKeyConflict)
        );
        assert_eq!(
            sink.submit(&request("tenant-a:deploy", "op-001", 1, 9, 7)),
            Err(SinkError::IdempotencyKeyConflict)
        );
        assert_eq!(sink.logical_effect_count().unwrap(), 1);
    }

    #[test]
    fn same_key_in_different_scopes_is_not_a_collision() {
        let sink = AtomicSink::new();
        let a = sink
            .submit(&request("tenant-a:deploy", "op-001", 1, 2, 1))
            .unwrap();
        let b = sink
            .submit(&request("tenant-b:deploy", "op-001", 1, 2, 1))
            .unwrap();
        assert_ne!(a.scope, b.scope);
        assert_eq!(sink.logical_effect_count().unwrap(), 2);
    }

    #[test]
    fn stale_fencing_token_is_rejected_after_newer_owner_commits() {
        let sink = AtomicSink::new();
        sink.submit(&request("tenant-a:deploy", "op-old", 1, 2, 7))
            .unwrap();
        sink.submit(&request("tenant-a:deploy", "op-new", 3, 4, 8))
            .unwrap();
        assert_eq!(
            sink.submit(&request("tenant-a:deploy", "op-stale", 5, 6, 7)),
            Err(SinkError::StaleFencingToken)
        );
        assert_eq!(
            sink.latest_fencing_token("tenant-a:deploy").unwrap(),
            Some(8)
        );
        assert_eq!(sink.logical_effect_count().unwrap(), 2);
    }

    #[test]
    fn exact_idempotent_retry_can_retrieve_receipt_after_fence_advances() {
        let sink = AtomicSink::new();
        let original = sink
            .submit(&request("tenant-a:deploy", "op-001", 1, 2, 7))
            .unwrap();
        sink.submit(&request("tenant-a:deploy", "op-002", 3, 4, 8))
            .unwrap();
        let replay = sink
            .submit(&request("tenant-a:deploy", "op-001", 1, 2, 7))
            .unwrap();
        assert_eq!(replay, original);
        assert_eq!(sink.logical_effect_count().unwrap(), 2);
    }

    #[test]
    fn concurrent_same_key_submissions_apply_one_logical_effect() {
        const WORKERS: usize = 64;
        let sink = Arc::new(AtomicSink::new());
        let barrier = Arc::new(Barrier::new(WORKERS));
        let mut workers = Vec::with_capacity(WORKERS);
        for _ in 0..WORKERS {
            let sink = Arc::clone(&sink);
            let barrier = Arc::clone(&barrier);
            workers.push(thread::spawn(move || {
                let request = request("tenant-a:deploy", "op-race", 1, 2, 7);
                barrier.wait();
                sink.submit(&request).unwrap()
            }));
        }
        let receipts: Vec<_> = workers
            .into_iter()
            .map(|worker| worker.join().expect("worker must not panic"))
            .collect();
        assert!(receipts.iter().all(|receipt| receipt == &receipts[0]));
        assert_eq!(sink.logical_effect_count().unwrap(), 1);
    }

    #[test]
    fn malformed_request_is_rejected_before_mutating_state() {
        let sink = AtomicSink::new();
        let mut invalid = request("tenant-a:deploy", "op-001", 1, 2, 7);
        invalid.request_digest = "NOT-A-DIGEST".to_string();
        assert!(matches!(
            sink.submit(&invalid),
            Err(SinkError::InvalidRequest(_))
        ));
        assert_eq!(sink.logical_effect_count().unwrap(), 0);
    }

    #[test]
    fn a_new_key_with_the_current_fence_remains_usable() {
        let sink = AtomicSink::new();
        sink.submit(&request("tenant-a:deploy", "op-001", 1, 2, 7))
            .unwrap();
        sink.submit(&request("tenant-a:deploy", "op-002", 3, 4, 7))
            .unwrap();
        assert_eq!(sink.logical_effect_count().unwrap(), 2);
        assert_eq!(
            sink.latest_fencing_token("tenant-a:deploy").unwrap(),
            Some(7)
        );
    }
}
