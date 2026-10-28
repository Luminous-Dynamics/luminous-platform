//! Typed, fail-closed assurance-vector model for privileged lifecycle recovery.
//!
//! This crate evaluates supplied evidence. It does not verify signatures, calculate
//! cryptographic digests, contact an anchor, or invoke an operating-system side effect.
//! Evidence variants named Verified represent results supplied by a trusted adapter.
//! The crate is a policy/state-machine model, not a production security boundary.

/// Opaque, non-cloneable proof token for one observation from a trusted epoch adapter.
///
/// This model intentionally has no public constructor. Until a production verifier/clock
/// adapter is integrated, downstream crates cannot create a value or manufacture trusted
/// freshness evidence. Evaluation consumes the containing assurance vector, making an
/// individual observation one-shot within this API.
///
/// ```compile_fail
/// use srq_guarantee_vector::TrustedEpoch;
/// fn require_clone<T: Clone>() {}
/// require_clone::<TrustedEpoch>();
/// ```
#[derive(Debug, Eq, PartialEq)]
pub struct TrustedEpoch {
    value: u64,
}

impl TrustedEpoch {
    pub(crate) fn value(&self) -> u64 {
        self.value
    }

    #[cfg(any(test, feature = "qualification-fixtures"))]
    pub(crate) fn synthetic_for_tests(value: u64) -> Self {
        Self { value }
    }
}

pub mod sink_model;
pub mod retention_model;
pub mod epoch_compaction_model;

/// Opaque authorization evidence issued only after a trusted verifier validates the
/// signature, policy binding, and freshness window. Fields and constructor are private;
/// this research crate intentionally supplies no production verifier adapter.
///
/// ```compile_fail
/// use srq_guarantee_vector::VerifiedAuthorization;
/// let _forged = VerifiedAuthorization {
///     request_digest: String::new(),
///     policy_digest: String::new(),
///     not_before_epoch: 0,
///     expires_at_epoch: 10,
/// };
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedAuthorization {
    request_digest: String,
    policy_digest: String,
    not_before_epoch: u64,
    expires_at_epoch: u64,
}
impl VerifiedAuthorization {
    #[cfg(any(test, feature = "qualification-fixtures"))]
    fn qualification_fixture(
        request_digest: String,
        policy_digest: String,
        not_before_epoch: u64,
        expires_at_epoch: u64,
    ) -> Self {
        Self { request_digest, policy_digest, not_before_epoch, expires_at_epoch }
    }
}

/// Opaque checkpoint data returned by an adapter that has authenticated the independent
/// anchor and established rollback protection and atomic compare-and-advance semantics.
///
/// ```compile_fail
/// use srq_guarantee_vector::VerifiedAnchorCheckpoint;
/// let _forged = VerifiedAnchorCheckpoint {
///     local_sequence: 1,
///     trusted_sequence: 1,
///     local_root: String::new(),
///     trusted_root: String::new(),
/// };
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedAnchorCheckpoint {
    local_sequence: u64,
    trusted_sequence: u64,
    local_root: String,
    trusted_root: String,
}
impl VerifiedAnchorCheckpoint {
    #[cfg(any(test, feature = "qualification-fixtures"))]
    fn qualification_fixture(
        local_sequence: u64,
        trusted_sequence: u64,
        local_root: String,
        trusted_root: String,
    ) -> Self {
        Self { local_sequence, trusted_sequence, local_root, trusted_root }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthorizationEvidence {
    Verified(VerifiedAuthorization),
    Missing,
    InvalidSignature,
    Stale,
    BindingMismatch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AnchorEvidence {
    Checkpoint(VerifiedAnchorCheckpoint),
    Unavailable,
    Divergent,
    RollbackDetected,
    Untrusted,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DispatchEvidence {
    NotClaimed,
    ClaimAnchored {
        request_digest: String,
        idempotency_key: Option<String>,
        fencing_token: Option<u64>,
    },
    OutcomeUnknown {
        request_digest: String,
        idempotency_key: Option<String>,
        fencing_token: Option<u64>,
    },
    CompletionMarked,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeliveryContract {
    AtomicIdempotency {
        request_digest: String,
        idempotency_key: String,
        atomic_effect_and_receipt: bool,
        durable_receipt: bool,
        sink_enforces_fencing: bool,
    },
    Transactional {
        request_digest: String,
        transaction_id: String,
        effect_and_receipt_atomic: bool,
        sink_enforces_fencing: bool,
    },
    AtMostOnce {
        sink_enforces_fencing: bool,
    },
    Unknown,
    Invalid,
}

/// Opaque receipt evidence returned only after a trusted verifier validates signature and
/// bindings. A Boolean supplied by a caller is not proof of a verified receipt.
///
/// ```compile_fail
/// use srq_guarantee_vector::VerifiedReceipt;
/// let _forged = VerifiedReceipt {
///     request_digest: String::new(),
///     effect_digest: String::new(),
///     policy_digest: String::new(),
/// };
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedReceipt {
    request_digest: String,
    effect_digest: String,
    policy_digest: String,
}
impl VerifiedReceipt {
    #[cfg(any(test, feature = "qualification-fixtures"))]
    fn qualification_fixture(
        request_digest: String,
        effect_digest: String,
        policy_digest: String,
    ) -> Self {
        Self { request_digest, effect_digest, policy_digest }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReceiptEvidence {
    Verified(VerifiedReceipt),
    Missing,
    Unknown,
    Invalid,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Decision {
    Block,
    DispatchOnce,
    RetrySameIdempotencyKey,
    ResumeTransaction,
    OperatorRequired,
    CompletedNoRedispatch,
}

impl Decision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Block => "BLOCK",
            Self::DispatchOnce => "DISPATCH_ONCE",
            Self::RetrySameIdempotencyKey => "RETRY_SAME_IDEMPOTENCY_KEY",
            Self::ResumeTransaction => "RESUME_TRANSACTION",
            Self::OperatorRequired => "OPERATOR_REQUIRED",
            Self::CompletedNoRedispatch => "COMPLETED_NO_REDISPATCH",
        }
    }

    /// True only when the evaluator permits a side-effecting dispatch/retry operation.
    pub fn permits_effect(self) -> bool {
        matches!(
            self,
            Self::DispatchOnce | Self::RetrySameIdempotencyKey | Self::ResumeTransaction
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DimensionStatus {
    Verified,
    Failed,
    Unknown,
    NotApplicable,
}

impl DimensionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Verified => "VERIFIED",
            Self::Failed => "FAILED",
            Self::Unknown => "UNKNOWN",
            Self::NotApplicable => "NOT_APPLICABLE",
        }
    }
}

/// Evidence for one evaluation. The private, non-cloneable epoch capability is consumed
/// by `evaluate`, so the same time observation cannot be replayed through a cloned vector.
///
/// No public constructor is provided until a production verifier/anchor adapter can build this
/// value from real verified evidence.
///
/// ```compile_fail
/// use srq_guarantee_vector::AssuranceVector;
/// let _forged = AssuranceVector {
///     request_digest: String::new(),
///     effect_digest: String::new(),
///     policy_digest: String::new(),
///     trusted_epoch: todo!(),
///     fencing_required: false,
///     authorization: todo!(),
///     anchor: todo!(),
///     dispatch: todo!(),
///     delivery: todo!(),
///     receipt: todo!(),
/// };
/// ```
#[derive(Debug, Eq, PartialEq)]
pub struct AssuranceVector {
    pub request_digest: String,
    pub effect_digest: String,
    pub policy_digest: String,
    trusted_epoch: TrustedEpoch,
    pub fencing_required: bool,
    pub authorization: AuthorizationEvidence,
    pub anchor: AnchorEvidence,
    pub dispatch: DispatchEvidence,
    pub delivery: DeliveryContract,
    pub receipt: ReceiptEvidence,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Assessment {
    pub decision: Decision,
    pub authorization: DimensionStatus,
    pub anti_rollback: DimensionStatus,
    pub dispatch: DimensionStatus,
    pub delivery: DimensionStatus,
    pub receipt: DimensionStatus,
    pub reason: &'static str,
}

impl Assessment {
    pub fn action_permitted(&self) -> bool {
        self.decision.permits_effect()
    }
}

pub fn is_lower_hex_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn authorization_status(vector: &AssuranceVector) -> DimensionStatus {
    match &vector.authorization {
        AuthorizationEvidence::Verified(proof)
            if proof.not_before_epoch < proof.expires_at_epoch
                && proof.not_before_epoch <= vector.trusted_epoch.value()
                && vector.trusted_epoch.value() < proof.expires_at_epoch
                && is_lower_hex_digest(&proof.request_digest)
                && proof.request_digest == vector.request_digest
                && is_lower_hex_digest(&proof.policy_digest)
                && proof.policy_digest == vector.policy_digest =>
        {
            DimensionStatus::Verified
        }
        AuthorizationEvidence::Missing => DimensionStatus::Unknown,
        _ => DimensionStatus::Failed,
    }
}

fn anchor_status(vector: &AssuranceVector) -> DimensionStatus {
    match &vector.anchor {
        AnchorEvidence::Checkpoint(proof)
            if proof.local_sequence == proof.trusted_sequence
                && is_lower_hex_digest(&proof.local_root)
                && proof.local_root == proof.trusted_root =>
        {
            DimensionStatus::Verified
        }
        AnchorEvidence::Unavailable | AnchorEvidence::Untrusted => DimensionStatus::Unknown,
        _ => DimensionStatus::Failed,
    }
}

fn receipt_status(vector: &AssuranceVector) -> DimensionStatus {
    match &vector.receipt {
        ReceiptEvidence::Verified(proof)
            if is_lower_hex_digest(&proof.request_digest)
                && proof.request_digest == vector.request_digest
                && is_lower_hex_digest(&proof.effect_digest)
                && proof.effect_digest == vector.effect_digest
                && is_lower_hex_digest(&proof.policy_digest)
                && proof.policy_digest == vector.policy_digest =>
        {
            DimensionStatus::Verified
        }
        ReceiptEvidence::Missing | ReceiptEvidence::Unknown => DimensionStatus::Unknown,
        _ => DimensionStatus::Failed,
    }
}

fn delivery_status(vector: &AssuranceVector) -> DimensionStatus {
    match &vector.delivery {
        DeliveryContract::AtomicIdempotency {
            request_digest,
            idempotency_key,
            atomic_effect_and_receipt: true,
            durable_receipt: true,
            sink_enforces_fencing,
        } if is_lower_hex_digest(request_digest)
            && request_digest == &vector.request_digest
            && !idempotency_key.is_empty()
            && (!vector.fencing_required || *sink_enforces_fencing) =>
        {
            DimensionStatus::Verified
        }
        DeliveryContract::Transactional {
            request_digest,
            transaction_id,
            effect_and_receipt_atomic: true,
            sink_enforces_fencing,
        } if is_lower_hex_digest(request_digest)
            && request_digest == &vector.request_digest
            && !transaction_id.is_empty()
            && (!vector.fencing_required || *sink_enforces_fencing) =>
        {
            DimensionStatus::Verified
        }
        DeliveryContract::AtMostOnce {
            sink_enforces_fencing,
        } if !vector.fencing_required || *sink_enforces_fencing => DimensionStatus::Verified,
        DeliveryContract::Unknown => DimensionStatus::Unknown,
        _ => DimensionStatus::Failed,
    }
}

fn dispatch_status(vector: &AssuranceVector) -> DimensionStatus {
    match &vector.dispatch {
        DispatchEvidence::NotClaimed => DimensionStatus::NotApplicable,
        DispatchEvidence::ClaimAnchored {
            request_digest,
            idempotency_key,
            fencing_token,
        } => {
            if !is_lower_hex_digest(request_digest)
                || request_digest != &vector.request_digest
                || !dispatch_key_matches(idempotency_key.as_deref(), &vector.delivery)
                || (vector.fencing_required && fencing_token.is_none())
            {
                DimensionStatus::Failed
            } else {
                DimensionStatus::Verified
            }
        }
        DispatchEvidence::OutcomeUnknown {
            request_digest,
            idempotency_key,
            fencing_token,
        } => {
            if !is_lower_hex_digest(request_digest)
                || request_digest != &vector.request_digest
                || !dispatch_key_matches(idempotency_key.as_deref(), &vector.delivery)
                || (vector.fencing_required && fencing_token.is_none())
            {
                DimensionStatus::Failed
            } else {
                // Attempt identity is verified; its effect outcome remains unresolved.
                DimensionStatus::Unknown
            }
        }
        DispatchEvidence::CompletionMarked => DimensionStatus::Unknown,
    }
}

fn dispatch_key_matches(key: Option<&str>, contract: &DeliveryContract) -> bool {
    match contract {
        DeliveryContract::AtomicIdempotency { idempotency_key, .. } => {
            key == Some(idempotency_key.as_str())
        }
        DeliveryContract::Transactional { transaction_id, .. } => {
            key == Some(transaction_id.as_str())
        }
        DeliveryContract::AtMostOnce { .. } => key.is_none() || key == Some(""),
        DeliveryContract::Unknown | DeliveryContract::Invalid => false,
    }
}

pub fn evaluate(vector: AssuranceVector) -> Assessment {
    let authorization = authorization_status(vector);
    let anti_rollback = anchor_status(vector);
    let dispatch = dispatch_status(vector);
    let delivery = delivery_status(vector);
    let receipt = receipt_status(vector);

    let block = |reason| Assessment {
        decision: Decision::Block,
        authorization,
        anti_rollback,
        dispatch,
        delivery,
        receipt,
        reason,
    };

    // A cryptographically verified receipt for the exact requested effect proves this
    // operation should not be invoked again, even if current authorization has since expired.
    if let ReceiptEvidence::Verified(proof) = &vector.receipt {
        if !is_lower_hex_digest(&proof.request_digest)
            || !is_lower_hex_digest(&proof.effect_digest)
            || !is_lower_hex_digest(&proof.policy_digest)
            || proof.request_digest != vector.request_digest
            || proof.effect_digest != vector.effect_digest
            || proof.policy_digest != vector.policy_digest
        {
            return block("receipt is invalid or bound to a different request/effect/policy");
        }
        return Assessment {
            decision: Decision::CompletedNoRedispatch,
            authorization,
            anti_rollback,
            dispatch,
            delivery,
            receipt,
            reason: "matching authoritative receipt; never redispatch",
        };
    }
    if matches!(&vector.receipt, ReceiptEvidence::Invalid) {
        return block("receipt evidence is invalid");
    }

    if authorization != DimensionStatus::Verified {
        return block("authorization, freshness, or authorization binding is not verified");
    }
    if anti_rollback != DimensionStatus::Verified {
        return block("independent anti-rollback checkpoint is not verified");
    }
    if delivery != DimensionStatus::Verified {
        return block("delivery contract is unsupported, malformed, or insufficiently fenced");
    }
    if dispatch == DimensionStatus::Failed {
        return block("dispatch claim does not match request or delivery contract");
    }

    match &vector.dispatch {
        DispatchEvidence::NotClaimed => Assessment {
            decision: Decision::DispatchOnce,
            authorization,
            anti_rollback,
            dispatch,
            delivery,
            receipt,
            reason: "verified prerequisites; persist claim before one dispatch",
        },
        DispatchEvidence::ClaimAnchored { .. } | DispatchEvidence::OutcomeUnknown { .. } => {
            let decision = match &vector.delivery {
                DeliveryContract::AtomicIdempotency { .. } => {
                    Decision::RetrySameIdempotencyKey
                }
                DeliveryContract::Transactional { .. } => Decision::ResumeTransaction,
                DeliveryContract::AtMostOnce { .. } => Decision::OperatorRequired,
                DeliveryContract::Unknown | DeliveryContract::Invalid => Decision::Block,
            };
            Assessment {
                decision,
                authorization,
                anti_rollback,
                dispatch,
                delivery,
                receipt,
                reason: match decision {
                    Decision::RetrySameIdempotencyKey => {
                        "retry only with sink-enforced same-key atomic idempotency"
                    }
                    Decision::ResumeTransaction => {
                        "resume in the same transaction domain; do not create a new transaction"
                    }
                    Decision::OperatorRequired => {
                        "outcome may be unknown; at-most-once contract forbids automatic retry"
                    }
                    _ => "unsupported recovery contract",
                },
            }
        }
        DispatchEvidence::CompletionMarked => Assessment {
            decision: Decision::OperatorRequired,
            authorization,
            anti_rollback,
            dispatch,
            delivery,
            receipt,
            reason: "completion marker lacks matching authoritative receipt",
        },
    }
}

#[cfg(any(test, feature = "qualification-fixtures"))]
fn clone_fixture_vector(vector: &AssuranceVector) -> AssuranceVector {
    AssuranceVector {
        request_digest: vector.request_digest.clone(),
        effect_digest: vector.effect_digest.clone(),
        policy_digest: vector.policy_digest.clone(),
        trusted_epoch: TrustedEpoch::synthetic_for_tests(vector.trusted_epoch.value()),
        fencing_required: vector.fencing_required,
        authorization: vector.authorization.clone(),
        anchor: vector.anchor.clone(),
        dispatch: vector.dispatch.clone(),
        delivery: vector.delivery.clone(),
        receipt: vector.receipt.clone(),
    }
}

#[derive(Debug, Eq, PartialEq)]
#[cfg(any(test, feature = "qualification-fixtures"))]
pub struct QualificationCase {
    pub name: &'static str,
    pub vector: AssuranceVector,
    pub expected: Decision,
    pub expected_action_permitted: bool,
}

#[cfg(any(test, feature = "qualification-fixtures"))]
fn digest(byte: u8) -> String {
    format!("{byte:02x}").repeat(32)
}

#[cfg(any(test, feature = "qualification-fixtures"))]
pub fn fixture_matrix() -> Vec<QualificationCase> {
    let request = digest(1);
    let effect = digest(2);
    let policy = digest(3);
    let root = digest(4);
    let key = "stable-idempotency-key-001".to_string();
    let txn = "stable-transaction-001".to_string();

    let base = AssuranceVector {
        request_digest: request.clone(),
        effect_digest: effect.clone(),
        policy_digest: policy.clone(),
        trusted_epoch: TrustedEpoch::synthetic_for_tests(15),
        fencing_required: true,
        authorization: AuthorizationEvidence::Verified(VerifiedAuthorization::qualification_fixture(
            request.clone(), policy.clone(), 10, 20,
        )),
        anchor: AnchorEvidence::Checkpoint(VerifiedAnchorCheckpoint::qualification_fixture(
            5, 5, root.clone(), root,
        )),
        dispatch: DispatchEvidence::NotClaimed,
        delivery: DeliveryContract::AtomicIdempotency {
            request_digest: request.clone(),
            idempotency_key: key.clone(),
            atomic_effect_and_receipt: true,
            durable_receipt: true,
            sink_enforces_fencing: true,
        },
        receipt: ReceiptEvidence::Missing,
    };

    let mut cases = Vec::new();
    let mut add = |name, vector, expected, permitted| {
        cases.push(QualificationCase {
            name,
            vector,
            expected,
            expected_action_permitted: permitted,
        });
    };

    add("positive_ready_to_dispatch", clone_fixture_vector(&base), Decision::DispatchOnce, true);

    let mut atomic_claim = clone_fixture_vector(&base);
    atomic_claim.dispatch = DispatchEvidence::ClaimAnchored {
        request_digest: request.clone(),
        idempotency_key: Some(key.clone()),
        fencing_token: Some(8),
    };
    add(
        "positive_atomic_same_key_retry",
        atomic_claim.clone(),
        Decision::RetrySameIdempotencyKey,
        true,
    );

    let mut unknown_atomic = clone_fixture_vector(&base);
    unknown_atomic.dispatch = DispatchEvidence::OutcomeUnknown {
        request_digest: request.clone(),
        idempotency_key: Some(key.clone()),
        fencing_token: Some(8),
    };
    add(
        "positive_unknown_outcome_atomic_retry",
        unknown_atomic,
        Decision::RetrySameIdempotencyKey,
        true,
    );

    let mut transaction = clone_fixture_vector(&base);
    transaction.delivery = DeliveryContract::Transactional {
        request_digest: request.clone(),
        transaction_id: txn.clone(),
        effect_and_receipt_atomic: true,
        sink_enforces_fencing: true,
    };
    transaction.dispatch = DispatchEvidence::ClaimAnchored {
        request_digest: request.clone(),
        idempotency_key: Some(txn.clone()),
        fencing_token: Some(8),
    };
    add("positive_resume_same_transaction", transaction, Decision::ResumeTransaction, true);

    let mut at_most_once = clone_fixture_vector(&base);
    at_most_once.delivery = DeliveryContract::AtMostOnce {
        sink_enforces_fencing: true,
    };
    at_most_once.dispatch = DispatchEvidence::ClaimAnchored {
        request_digest: request.clone(),
        idempotency_key: None,
        fencing_token: Some(8),
    };
    add(
        "positive_unknown_at_most_once_escalates",
        at_most_once,
        Decision::OperatorRequired,
        false,
    );

    let mut completed = clone_fixture_vector(&base);
    completed.receipt = ReceiptEvidence::Verified(VerifiedReceipt::qualification_fixture(
        request.clone(), effect.clone(), policy.clone(),
    ));
    completed.authorization = AuthorizationEvidence::Stale;
    add(
        "positive_matching_receipt_never_redispatches",
        completed,
        Decision::CompletedNoRedispatch,
        false,
    );

    let mut bad = clone_fixture_vector(&base);
    bad.authorization = AuthorizationEvidence::Missing;
    add("attack_missing_authorization", bad, Decision::Block, false);

    let mut bad = clone_fixture_vector(&base);
    bad.authorization = AuthorizationEvidence::InvalidSignature;
    add("attack_invalid_signature", bad, Decision::Block, false);

    let mut bad = clone_fixture_vector(&base);
    bad.authorization = AuthorizationEvidence::Verified(
        VerifiedAuthorization::qualification_fixture(
            request.clone(),
            policy.clone(),
            10,
            15,
        ),
    );
    add("attack_expired_verified_authorization", bad, Decision::Block, false);

    let mut bad = clone_fixture_vector(&base);
    bad.authorization = AuthorizationEvidence::Verified(
        VerifiedAuthorization::qualification_fixture(digest(9), policy.clone(), 10, 20),
    );
    add("attack_authorization_request_mismatch", bad, Decision::Block, false);

    let mut bad = clone_fixture_vector(&base);
    bad.anchor = AnchorEvidence::Unavailable;
    add("attack_anchor_unavailable", bad, Decision::Block, false);

    let mut bad = clone_fixture_vector(&base);
    bad.anchor = AnchorEvidence::RollbackDetected;
    add("attack_snapshot_rollback", bad, Decision::Block, false);

    let mut bad = clone_fixture_vector(&base);
    bad.anchor = AnchorEvidence::Checkpoint(VerifiedAnchorCheckpoint::qualification_fixture(
        5, 6, digest(4), digest(4),
    ));
    add("attack_checkpoint_sequence_divergence", bad, Decision::Block, false);

    let mut bad = atomic_claim.clone();
    bad.dispatch = DispatchEvidence::ClaimAnchored {
        request_digest: request.clone(),
        idempotency_key: Some("different-key".to_string()),
        fencing_token: Some(8),
    };
    add("attack_dispatch_key_mismatch", bad, Decision::Block, false);

    let mut bad = clone_fixture_vector(&base);
    bad.delivery = DeliveryContract::AtomicIdempotency {
        request_digest: digest(9),
        idempotency_key: key.clone(),
        atomic_effect_and_receipt: true,
        durable_receipt: true,
        sink_enforces_fencing: true,
    };
    add("attack_contract_request_mismatch", bad, Decision::Block, false);

    let mut bad = clone_fixture_vector(&base);
    bad.delivery = DeliveryContract::AtomicIdempotency {
        request_digest: request.clone(),
        idempotency_key: key.clone(),
        atomic_effect_and_receipt: false,
        durable_receipt: true,
        sink_enforces_fencing: true,
    };
    add("attack_nonatomic_effect_and_receipt", bad, Decision::Block, false);

    let mut bad = clone_fixture_vector(&base);
    bad.delivery = DeliveryContract::Unknown;
    add("attack_unknown_delivery_contract", bad, Decision::Block, false);

    let mut bad = atomic_claim.clone();
    bad.dispatch = DispatchEvidence::ClaimAnchored {
        request_digest: request.clone(),
        idempotency_key: Some(key.clone()),
        fencing_token: None,
    };
    add("attack_dispatch_missing_fencing_token", bad, Decision::Block, false);

    let mut bad = unknown_atomic.clone();
    bad.dispatch = DispatchEvidence::OutcomeUnknown {
        request_digest: request.clone(),
        idempotency_key: Some(key.clone()),
        fencing_token: None,
    };
    add(
        "attack_unknown_outcome_missing_fencing_token",
        bad,
        Decision::Block,
        false,
    );

    let mut bad = unknown_atomic.clone();
    bad.dispatch = DispatchEvidence::OutcomeUnknown {
        request_digest: request.clone(),
        idempotency_key: Some("wrong-key".to_string()),
        fencing_token: Some(8),
    };
    add(
        "attack_unknown_outcome_wrong_idempotency_key",
        bad,
        Decision::Block,
        false,
    );

    let mut bad = clone_fixture_vector(&base);
    bad.delivery = DeliveryContract::AtomicIdempotency {
        request_digest: request.clone(),
        idempotency_key: key.clone(),
        atomic_effect_and_receipt: true,
        durable_receipt: true,
        sink_enforces_fencing: false,
    };
    add("attack_missing_required_fencing", bad, Decision::Block, false);

    let mut bad = clone_fixture_vector(&base);
    bad.receipt = ReceiptEvidence::Verified(VerifiedReceipt::qualification_fixture(
        request.clone(), digest(8), policy.clone(),
    ));
    add("attack_receipt_wrong_effect_digest", bad, Decision::Block, false);

    let mut bad = clone_fixture_vector(&base);
    bad.receipt = ReceiptEvidence::Invalid;
    add("attack_receipt_invalid_signature", bad, Decision::Block, false);

    let mut bad = clone_fixture_vector(&base);
    bad.receipt = ReceiptEvidence::Verified(VerifiedReceipt::qualification_fixture(
        request.clone(), effect.clone(), digest(8),
    ));
    add("attack_receipt_wrong_policy_digest", bad, Decision::Block, false);

    let mut bad = clone_fixture_vector(&base);
    bad.dispatch = DispatchEvidence::CompletionMarked;
    add(
        "attack_completion_marker_without_receipt",
        bad,
        Decision::OperatorRequired,
        false,
    );

    let mut bad = base;
    bad.request_digest = "NOT-A-DIGEST".to_string();
    add("attack_malformed_request_digest", bad, Decision::Block, false);

    cases
}


#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[cfg(any(test, feature = "qualification-fixtures"))]
pub struct ExhaustiveSummary {
    pub combinations_checked: usize,
    pub false_release_count: usize,
    pub false_block_count: usize,
    pub observed_action_permitted_count: usize,
}

/// Bounded cross-product over five evidence dimensions. The expectation oracle is deliberately
/// separate from evaluate(): it states the minimum evidence necessary to permit an effect.
/// This is finite adversarial exploration, not a proof over all possible traces.
#[cfg(any(test, feature = "qualification-fixtures"))]
pub fn exhaustive_invariant_summary() -> ExhaustiveSummary {
    let base = fixture_matrix()
        .into_iter()
        .next()
        .expect("at least one frozen fixture")
        .vector;
    let request = base.request_digest.clone();
    let effect = base.effect_digest.clone();
    let policy = base.policy_digest.clone();
    let root = match &base.anchor {
        AnchorEvidence::Checkpoint(proof) => proof.local_root.clone(),
        _ => unreachable!("base fixture has a checkpoint"),
    };
    let key = match &base.delivery {
        DeliveryContract::AtomicIdempotency { idempotency_key, .. } => idempotency_key.clone(),
        _ => unreachable!("base fixture has atomic idempotency"),
    };
    let txn = "stable-transaction-001".to_string();

    let wrong_authorization = AuthorizationEvidence::Verified(
        VerifiedAuthorization::qualification_fixture(digest(9), policy.clone(), 10, 20),
    );
    let authorizations = vec![
        base.authorization.clone(),
        AuthorizationEvidence::Missing,
        AuthorizationEvidence::InvalidSignature,
        AuthorizationEvidence::Stale,
        wrong_authorization,
    ];

    let anchors = vec![
        base.anchor.clone(),
        AnchorEvidence::Unavailable,
        AnchorEvidence::RollbackDetected,
        AnchorEvidence::Checkpoint(VerifiedAnchorCheckpoint::qualification_fixture(
            5, 6, root.clone(), root.clone(),
        )),
        AnchorEvidence::Untrusted,
    ];

    let dispatches = vec![
        DispatchEvidence::NotClaimed,
        DispatchEvidence::ClaimAnchored {
            request_digest: request.clone(),
            idempotency_key: Some(key.clone()),
            fencing_token: Some(8),
        },
        DispatchEvidence::ClaimAnchored {
            request_digest: digest(9),
            idempotency_key: Some(key.clone()),
            fencing_token: Some(8),
        },
        DispatchEvidence::ClaimAnchored {
            request_digest: request.clone(),
            idempotency_key: Some(key.clone()),
            fencing_token: None,
        },
        DispatchEvidence::OutcomeUnknown {
            request_digest: request.clone(),
            idempotency_key: Some(key.clone()),
            fencing_token: Some(8),
        },
        DispatchEvidence::OutcomeUnknown {
            request_digest: request.clone(),
            idempotency_key: Some(key.clone()),
            fencing_token: None,
        },
        DispatchEvidence::OutcomeUnknown {
            request_digest: request.clone(),
            idempotency_key: Some("other-key".to_string()),
            fencing_token: Some(8),
        },
    ];

    let deliveries = vec![
        base.delivery.clone(),
        DeliveryContract::Transactional {
            request_digest: request.clone(),
            transaction_id: txn,
            effect_and_receipt_atomic: true,
            sink_enforces_fencing: true,
        },
        DeliveryContract::AtMostOnce {
            sink_enforces_fencing: true,
        },
        DeliveryContract::Unknown,
        DeliveryContract::AtomicIdempotency {
            request_digest: request.clone(),
            idempotency_key: key,
            atomic_effect_and_receipt: false,
            durable_receipt: true,
            sink_enforces_fencing: true,
        },
    ];

    let receipts = vec![
        ReceiptEvidence::Missing,
        ReceiptEvidence::Unknown,
        ReceiptEvidence::Verified(VerifiedReceipt::qualification_fixture(
            request.clone(), effect.clone(), policy.clone(),
        )),
        ReceiptEvidence::Verified(VerifiedReceipt::qualification_fixture(
            request.clone(), digest(8), policy.clone(),
        )),
        ReceiptEvidence::Verified(VerifiedReceipt::qualification_fixture(
            request.clone(), effect, digest(8),
        )),
    ];

    let mut summary = ExhaustiveSummary::default();
    for authorization in &authorizations {
        for anchor in &anchors {
            for dispatch in &dispatches {
                for delivery in &deliveries {
                    for receipt in &receipts {
                        let vector = AssuranceVector {
                            request_digest: request.clone(),
                            effect_digest: base.effect_digest.clone(),
                            policy_digest: policy.clone(),
                            trusted_epoch: TrustedEpoch::synthetic_for_tests(
                                base.trusted_epoch.value(),
                            ),
                            fencing_required: true,
                            authorization: authorization.clone(),
                            anchor: anchor.clone(),
                            dispatch: dispatch.clone(),
                            delivery: delivery.clone(),
                            receipt: receipt.clone(),
                        };
                        let expected_action = oracle_allows_effect(&vector);
                        let assessment = evaluate(vector);
                        summary.combinations_checked += 1;
                        if assessment.action_permitted() {
                            summary.observed_action_permitted_count += 1;
                        }
                        if assessment.action_permitted() && !expected_action {
                            summary.false_release_count += 1;
                        }
                        if expected_action && !assessment.action_permitted() {
                            summary.false_block_count += 1;
                        }
                    }
                }
            }
        }
    }
    summary
}

#[cfg(any(test, feature = "qualification-fixtures"))]
fn oracle_allows_effect(vector: &AssuranceVector) -> bool {
    let authorization_ok = match &vector.authorization {
        AuthorizationEvidence::Verified(proof) => {
            proof.not_before_epoch < proof.expires_at_epoch
                && proof.not_before_epoch <= vector.trusted_epoch.value()
                && vector.trusted_epoch.value() < proof.expires_at_epoch
                && is_lower_hex_digest(&proof.request_digest)
                && proof.request_digest == vector.request_digest
                && is_lower_hex_digest(&proof.policy_digest)
                && proof.policy_digest == vector.policy_digest
        }
        _ => false,
    };
    let anchor_ok = match &vector.anchor {
        AnchorEvidence::Checkpoint(proof) => {
            proof.local_sequence == proof.trusted_sequence
                && is_lower_hex_digest(&proof.local_root)
                && proof.local_root == proof.trusted_root
        }
        _ => false,
    };
    let delivery_ok = match &vector.delivery {
        DeliveryContract::AtomicIdempotency {
            request_digest,
            idempotency_key,
            atomic_effect_and_receipt,
            durable_receipt,
            sink_enforces_fencing,
        } => {
            request_digest == &vector.request_digest
                && is_lower_hex_digest(request_digest)
                && !idempotency_key.is_empty()
                && *atomic_effect_and_receipt
                && *durable_receipt
                && (!vector.fencing_required || *sink_enforces_fencing)
        }
        DeliveryContract::Transactional {
            request_digest,
            transaction_id,
            effect_and_receipt_atomic,
            sink_enforces_fencing,
        } => {
            request_digest == &vector.request_digest
                && is_lower_hex_digest(request_digest)
                && !transaction_id.is_empty()
                && *effect_and_receipt_atomic
                && (!vector.fencing_required || *sink_enforces_fencing)
        }
        DeliveryContract::AtMostOnce { sink_enforces_fencing } => {
            !vector.fencing_required || *sink_enforces_fencing
        }
        DeliveryContract::Unknown | DeliveryContract::Invalid => false,
    };
    let receipt_allows_new_effect = matches!(
        &vector.receipt,
        ReceiptEvidence::Missing | ReceiptEvidence::Unknown
    );
    let dispatch_ok = match (&vector.dispatch, &vector.delivery) {
        (DispatchEvidence::NotClaimed, _) => true,
        (
            DispatchEvidence::ClaimAnchored {
                request_digest,
                idempotency_key,
                fencing_token,
            },
            contract,
        ) => {
            if request_digest != &vector.request_digest
                || (vector.fencing_required && fencing_token.is_none())
            {
                false
            } else {
                match contract {
                    DeliveryContract::AtomicIdempotency {
                        idempotency_key: expected_key,
                        ..
                    } => idempotency_key.as_deref() == Some(expected_key.as_str()),
                    DeliveryContract::Transactional {
                        transaction_id, ..
                    } => idempotency_key.as_deref() == Some(transaction_id.as_str()),
                    DeliveryContract::AtMostOnce { .. } => {
                        idempotency_key.as_deref().is_none_or(str::is_empty)
                    }
                    DeliveryContract::Unknown | DeliveryContract::Invalid => false,
                }
            }
        }
        (
            DispatchEvidence::OutcomeUnknown {
                request_digest,
                idempotency_key,
                fencing_token,
            },
            contract,
        ) => {
            if request_digest != &vector.request_digest
                || (vector.fencing_required && fencing_token.is_none())
            {
                false
            } else {
                match contract {
                    DeliveryContract::AtomicIdempotency { idempotency_key: expected, .. } => {
                        idempotency_key.as_deref() == Some(expected.as_str())
                    }
                    DeliveryContract::Transactional { transaction_id, .. } => {
                        idempotency_key.as_deref() == Some(transaction_id.as_str())
                    }
                    DeliveryContract::AtMostOnce { .. }
                    | DeliveryContract::Unknown
                    | DeliveryContract::Invalid => false,
                }
            }
        }
        (DispatchEvidence::CompletionMarked, _) => false,
    };
    authorization_ok && anchor_ok && delivery_ok && receipt_allows_new_effect && dispatch_ok
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_assurance_vector_matrix_matches_all_expected_decisions() {
        for case in fixture_matrix() {
            let observed = evaluate(case.vector);
            assert_eq!(
                observed.decision, case.expected,
                "unexpected decision for {}: {:?}",
                case.name, observed
            );
            assert_eq!(
                observed.action_permitted(),
                case.expected_action_permitted,
                "side-effect permission mismatch for {}",
                case.name
            );
        }
    }

    #[test]
    fn receipt_for_completed_operation_does_not_require_current_authorization() {
        let mut case = fixture_matrix()
            .into_iter()
            .find(|case| case.name == "positive_matching_receipt_never_redispatches")
            .expect("positive receipt case");
        case.vector.authorization = AuthorizationEvidence::Missing;
        let assessment = evaluate(case.vector);
        assert_eq!(assessment.decision, Decision::CompletedNoRedispatch);
        assert!(!assessment.action_permitted());
    }

    #[test]
    fn missing_receipt_after_at_most_once_claim_requires_operator() {
        let case = fixture_matrix()
            .into_iter()
            .find(|case| case.name == "positive_unknown_at_most_once_escalates")
            .expect("at-most-once case");
        assert_eq!(evaluate(case.vector).decision, Decision::OperatorRequired);
    }

    #[test]
    fn unknown_outcome_requires_valid_key_and_fencing_token() {
        for name in [
            "positive_unknown_outcome_atomic_retry",
            "attack_unknown_outcome_missing_fencing_token",
            "attack_unknown_outcome_wrong_idempotency_key",
        ] {
            let case = fixture_matrix()
                .into_iter()
                .find(|case| case.name == name)
                .expect("unknown-outcome fixture");
            let observed = evaluate(case.vector);
            assert_eq!(observed.decision, case.expected, "{name}");
            assert_eq!(observed.action_permitted(), case.expected_action_permitted, "{name}");
        }
    }

    #[test]
    fn receipt_must_bind_request_effect_and_policy() {
        for name in [
            "attack_receipt_wrong_effect_digest",
            "attack_receipt_invalid_signature",
            "attack_receipt_wrong_policy_digest",
        ] {
            let case = fixture_matrix()
                .into_iter()
                .find(|case| case.name == name)
                .expect("attack fixture");
            assert_eq!(evaluate(case.vector).decision, Decision::Block, "{name}");
        }
    }

    #[test]
    fn no_side_effect_is_permitted_by_block_or_operator_required() {
        for case in fixture_matrix() {
            let assessment = evaluate(case.vector);
            if matches!(assessment.decision, Decision::Block | Decision::OperatorRequired) {
                assert!(!assessment.action_permitted(), "unsafe permit for {}", case.name);
            }
        }
    }

    #[test]
    fn authorization_window_is_inclusive_at_start_and_exclusive_at_expiry() {
        let mut valid = fixture_matrix()
            .into_iter()
            .find(|case| case.name == "positive_ready_to_dispatch")
            .expect("valid dispatch case");
        valid.vector.trusted_epoch = TrustedEpoch::synthetic_for_tests(10);
        assert_eq!(authorization_status(&valid.vector), DimensionStatus::Verified);
        assert_eq!(evaluate(valid.vector).decision, Decision::DispatchOnce);

        let expired = fixture_matrix()
            .into_iter()
            .find(|case| case.name == "attack_expired_verified_authorization")
            .expect("expired authorization case");
        assert_eq!(authorization_status(&expired.vector), DimensionStatus::Failed);
        assert_eq!(evaluate(expired.vector).decision, Decision::Block);
    }

    #[test]
    fn exhaustive_bounded_combinations_have_no_false_release_or_false_block() {
        let summary = exhaustive_invariant_summary();
        assert_eq!(summary.combinations_checked, 4_375);
        assert_eq!(summary.false_release_count, 0);
        assert_eq!(summary.false_block_count, 0);
    }

    #[test]
    fn every_assurance_dimension_is_reported_separately() {
        let case = fixture_matrix()
            .into_iter()
            .find(|case| case.name == "attack_anchor_unavailable")
            .expect("anchor unavailable case");
        let assessment = evaluate(case.vector);
        assert_eq!(assessment.authorization, DimensionStatus::Verified);
        assert_eq!(assessment.anti_rollback, DimensionStatus::Unknown);
        assert_eq!(assessment.delivery, DimensionStatus::Verified);
        assert_eq!(assessment.decision, Decision::Block);
    }
}
