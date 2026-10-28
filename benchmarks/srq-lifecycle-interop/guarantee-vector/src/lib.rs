//! Typed, fail-closed assurance-vector model for privileged lifecycle recovery.
//!
//! This crate evaluates supplied evidence. It does not verify signatures, calculate
//! cryptographic digests, contact an anchor, or invoke an operating-system side effect.
//! Evidence variants named Verified represent results supplied by a trusted adapter.
//! The crate is a policy/state-machine model, not a production security boundary.

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthorizationEvidence {
    Verified {
        request_digest: String,
        policy_digest: String,
        signature_valid: bool,
        freshness_verified: bool,
    },
    Missing,
    InvalidSignature,
    Stale,
    BindingMismatch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AnchorEvidence {
    Checkpoint {
        local_sequence: u64,
        trusted_sequence: u64,
        local_root: String,
        trusted_root: String,
        anchor_independent: bool,
        atomic_compare_and_advance: bool,
        rollback_protected: bool,
    },
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
    OutcomeUnknown,
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReceiptEvidence {
    Verified {
        request_digest: String,
        effect_digest: String,
        signature_valid: bool,
    },
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssuranceVector {
    pub request_digest: String,
    pub effect_digest: String,
    pub policy_digest: String,
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
        AuthorizationEvidence::Verified {
            request_digest,
            policy_digest,
            signature_valid: true,
            freshness_verified: true,
        } if is_lower_hex_digest(request_digest)
            && request_digest == &vector.request_digest
            && is_lower_hex_digest(policy_digest)
            && policy_digest == &vector.policy_digest =>
        {
            DimensionStatus::Verified
        }
        AuthorizationEvidence::Missing => DimensionStatus::Unknown,
        _ => DimensionStatus::Failed,
    }
}

fn anchor_status(vector: &AssuranceVector) -> DimensionStatus {
    match &vector.anchor {
        AnchorEvidence::Checkpoint {
            local_sequence,
            trusted_sequence,
            local_root,
            trusted_root,
            anchor_independent: true,
            atomic_compare_and_advance: true,
            rollback_protected: true,
        } if local_sequence == trusted_sequence
            && is_lower_hex_digest(local_root)
            && local_root == trusted_root =>
        {
            DimensionStatus::Verified
        }
        AnchorEvidence::Unavailable | AnchorEvidence::Untrusted => DimensionStatus::Unknown,
        _ => DimensionStatus::Failed,
    }
}

fn receipt_status(vector: &AssuranceVector) -> DimensionStatus {
    match &vector.receipt {
        ReceiptEvidence::Verified {
            request_digest,
            effect_digest,
            signature_valid: true,
        } if is_lower_hex_digest(request_digest)
            && request_digest == &vector.request_digest
            && is_lower_hex_digest(effect_digest)
            && effect_digest == &vector.effect_digest =>
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
            ..
        } => {
            if !is_lower_hex_digest(request_digest)
                || request_digest != &vector.request_digest
                || !dispatch_key_matches(idempotency_key.as_deref(), &vector.delivery)
            {
                DimensionStatus::Failed
            } else {
                DimensionStatus::Verified
            }
        }
        DispatchEvidence::OutcomeUnknown | DispatchEvidence::CompletionMarked => {
            DimensionStatus::Unknown
        }
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

pub fn evaluate(vector: &AssuranceVector) -> Assessment {
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
    if let ReceiptEvidence::Verified {
        request_digest,
        effect_digest,
        signature_valid,
    } = &vector.receipt
    {
        if !signature_valid
            || !is_lower_hex_digest(request_digest)
            || !is_lower_hex_digest(effect_digest)
            || request_digest != &vector.request_digest
            || effect_digest != &vector.effect_digest
        {
            return block("receipt is invalid or bound to a different request/effect");
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
    if matches!(vector.receipt, ReceiptEvidence::Invalid) {
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
        DispatchEvidence::ClaimAnchored { .. } | DispatchEvidence::OutcomeUnknown => {
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QualificationCase {
    pub name: &'static str,
    pub vector: AssuranceVector,
    pub expected: Decision,
    pub expected_action_permitted: bool,
}

fn digest(byte: u8) -> String {
    std::iter::repeat_n(format!("{byte:02x}"), 32).collect()
}

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
        fencing_required: true,
        authorization: AuthorizationEvidence::Verified {
            request_digest: request.clone(),
            policy_digest: policy.clone(),
            signature_valid: true,
            freshness_verified: true,
        },
        anchor: AnchorEvidence::Checkpoint {
            local_sequence: 5,
            trusted_sequence: 5,
            local_root: root.clone(),
            trusted_root: root,
            anchor_independent: true,
            atomic_compare_and_advance: true,
            rollback_protected: true,
        },
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

    add("positive_ready_to_dispatch", base.clone(), Decision::DispatchOnce, true);

    let mut atomic_claim = base.clone();
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

    let mut unknown_atomic = base.clone();
    unknown_atomic.dispatch = DispatchEvidence::OutcomeUnknown;
    add(
        "positive_unknown_outcome_atomic_retry",
        unknown_atomic,
        Decision::RetrySameIdempotencyKey,
        true,
    );

    let mut transaction = base.clone();
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

    let mut at_most_once = base.clone();
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

    let mut completed = base.clone();
    completed.receipt = ReceiptEvidence::Verified {
        request_digest: request.clone(),
        effect_digest: effect.clone(),
        signature_valid: true,
    };
    completed.authorization = AuthorizationEvidence::Stale;
    add(
        "positive_matching_receipt_never_redispatches",
        completed,
        Decision::CompletedNoRedispatch,
        false,
    );

    let mut bad = base.clone();
    bad.authorization = AuthorizationEvidence::Missing;
    add("attack_missing_authorization", bad, Decision::Block, false);

    let mut bad = base.clone();
    bad.authorization = AuthorizationEvidence::InvalidSignature;
    add("attack_invalid_signature", bad, Decision::Block, false);

    let mut bad = base.clone();
    bad.authorization = AuthorizationEvidence::Stale;
    add("attack_stale_authorization", bad, Decision::Block, false);

    let mut bad = base.clone();
    bad.authorization = AuthorizationEvidence::Verified {
        request_digest: digest(9),
        policy_digest: policy.clone(),
        signature_valid: true,
        freshness_verified: true,
    };
    add("attack_authorization_request_mismatch", bad, Decision::Block, false);

    let mut bad = base.clone();
    bad.anchor = AnchorEvidence::Unavailable;
    add("attack_anchor_unavailable", bad, Decision::Block, false);

    let mut bad = base.clone();
    bad.anchor = AnchorEvidence::RollbackDetected;
    add("attack_snapshot_rollback", bad, Decision::Block, false);

    let mut bad = base.clone();
    bad.anchor = AnchorEvidence::Checkpoint {
        local_sequence: 5,
        trusted_sequence: 6,
        local_root: digest(4),
        trusted_root: digest(4),
        anchor_independent: true,
        atomic_compare_and_advance: true,
        rollback_protected: true,
    };
    add("attack_checkpoint_sequence_divergence", bad, Decision::Block, false);

    let mut bad = atomic_claim.clone();
    bad.dispatch = DispatchEvidence::ClaimAnchored {
        request_digest: request.clone(),
        idempotency_key: Some("different-key".to_string()),
        fencing_token: Some(8),
    };
    add("attack_dispatch_key_mismatch", bad, Decision::Block, false);

    let mut bad = base.clone();
    bad.delivery = DeliveryContract::AtomicIdempotency {
        request_digest: digest(9),
        idempotency_key: key.clone(),
        atomic_effect_and_receipt: true,
        durable_receipt: true,
        sink_enforces_fencing: true,
    };
    add("attack_contract_request_mismatch", bad, Decision::Block, false);

    let mut bad = base.clone();
    bad.delivery = DeliveryContract::AtomicIdempotency {
        request_digest: request.clone(),
        idempotency_key: key.clone(),
        atomic_effect_and_receipt: false,
        durable_receipt: true,
        sink_enforces_fencing: true,
    };
    add("attack_nonatomic_effect_and_receipt", bad, Decision::Block, false);

    let mut bad = base.clone();
    bad.delivery = DeliveryContract::Unknown;
    add("attack_unknown_delivery_contract", bad, Decision::Block, false);

    let mut bad = base.clone();
    bad.delivery = DeliveryContract::AtomicIdempotency {
        request_digest: request.clone(),
        idempotency_key: key.clone(),
        atomic_effect_and_receipt: true,
        durable_receipt: true,
        sink_enforces_fencing: false,
    };
    add("attack_missing_required_fencing", bad, Decision::Block, false);

    let mut bad = base.clone();
    bad.receipt = ReceiptEvidence::Verified {
        request_digest: request.clone(),
        effect_digest: digest(8),
        signature_valid: true,
    };
    add("attack_receipt_wrong_effect_digest", bad, Decision::Block, false);

    let mut bad = base.clone();
    bad.receipt = ReceiptEvidence::Verified {
        request_digest: request.clone(),
        effect_digest: effect.clone(),
        signature_valid: false,
    };
    add("attack_receipt_invalid_signature", bad, Decision::Block, false);

    let mut bad = base.clone();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_assurance_vector_matrix_matches_all_expected_decisions() {
        for case in fixture_matrix() {
            let observed = evaluate(&case.vector);
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
        assert_eq!(evaluate(&case.vector).decision, Decision::CompletedNoRedispatch);
        assert!(!evaluate(&case.vector).action_permitted());
    }

    #[test]
    fn missing_receipt_after_at_most_once_claim_requires_operator() {
        let case = fixture_matrix()
            .into_iter()
            .find(|case| case.name == "positive_unknown_at_most_once_escalates")
            .expect("at-most-once case");
        assert_eq!(evaluate(&case.vector).decision, Decision::OperatorRequired);
    }

    #[test]
    fn receipt_must_bind_both_request_and_effect() {
        for name in [
            "attack_receipt_wrong_effect_digest",
            "attack_receipt_invalid_signature",
        ] {
            let case = fixture_matrix()
                .into_iter()
                .find(|case| case.name == name)
                .expect("attack fixture");
            assert_eq!(evaluate(&case.vector).decision, Decision::Block, "{name}");
        }
    }

    #[test]
    fn no_side_effect_is_permitted_by_block_or_operator_required() {
        for case in fixture_matrix() {
            let assessment = evaluate(&case.vector);
            if matches!(assessment.decision, Decision::Block | Decision::OperatorRequired) {
                assert!(!assessment.action_permitted(), "unsafe permit for {}", case.name);
            }
        }
    }

    #[test]
    fn every_assurance_dimension_is_reported_separately() {
        let case = fixture_matrix()
            .into_iter()
            .find(|case| case.name == "attack_anchor_unavailable")
            .expect("anchor unavailable case");
        let assessment = evaluate(&case.vector);
        assert_eq!(assessment.authorization, DimensionStatus::Verified);
        assert_eq!(assessment.anti_rollback, DimensionStatus::Unknown);
        assert_eq!(assessment.delivery, DimensionStatus::Verified);
        assert_eq!(assessment.decision, Decision::Block);
    }
}
