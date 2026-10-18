//! Metadata-only reference model for offline message admission and scheduling.
//!
//! This crate does not store payload bytes, persist state, implement cryptography,
//! verify signatures, access sockets, or authorize real relays. Fields describing
//! protection are trusted-adapter assertions, not proof. Production adapters must
//! enforce those boundaries before calling this model.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Priority {
    Bulk,
    Ordinary,
    Essential,
}

impl Priority {
    const fn rank(self) -> u8 {
        match self {
            Self::Bulk => 0,
            Self::Ordinary => 1,
            Self::Essential => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DataClass {
    Public,
    Community,
    Sensitive,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProtectionDeclaration {
    None,
    IntegrityOnly,
    EndToEndAuthenticatedEncryption,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MessageMetadata {
    pub message_id: String,
    pub sender_id: String,
    pub recipient_id: String,
    pub created_at_ms: u64,
    pub expires_at_ms: u64,
    pub payload_len_bytes: u64,
    pub payload_sha256: String,
    pub priority: Priority,
    pub data_class: DataClass,
    pub protection_declaration: ProtectionDeclaration,
    /// Assertion supplied by the trusted application adapter, not a signature
    /// or independently verifiable cryptographic receipt.
    pub trusted_adapter_reports_protection_verified: bool,
}

impl MessageMetadata {
    fn ttl_ms(&self) -> Option<u64> {
        self.expires_at_ms.checked_sub(self.created_at_ms)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueuePolicy {
    pub max_message_bytes: u64,
    pub max_queue_items: usize,
    pub max_queue_bytes: u64,
    pub max_items_per_sender: usize,
    pub max_bytes_per_sender: u64,
    pub min_ttl_ms: u64,
    pub max_ttl_ms: u64,
    pub max_future_clock_skew_ms: u64,
    pub dedupe_retention_ms: u64,
    pub max_history_records: usize,
    pub authorized_recipients: BTreeSet<String>,
}

impl QueuePolicy {
    pub fn validate(&self) -> Result<(), QueueError> {
        if self.max_message_bytes == 0
            || self.max_queue_items == 0
            || self.max_queue_bytes == 0
            || self.max_items_per_sender == 0
            || self.max_bytes_per_sender == 0
            || self.min_ttl_ms == 0
            || self.max_ttl_ms < self.min_ttl_ms
            || self.dedupe_retention_ms == 0
            || self.max_history_records < self.max_queue_items
            || self.authorized_recipients.is_empty()
            || self.authorized_recipients.iter().any(|id| !valid_identifier(id))
        {
            return Err(QueueError::InvalidPolicy);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueueError {
    InvalidPolicy,
    InvalidMessageId,
    InvalidSenderId,
    InvalidRecipientId,
    InvalidPayloadDigest,
    EmptyPayload,
    MessageTooLarge,
    CreatedTooFarInFuture,
    ExpiredAtAdmission,
    InvalidLifetime,
    LifetimeTooShort,
    LifetimeTooLong,
    SensitiveProtectionNotVerified,
    UnauthorizedRecipient,
    DuplicateIdConflict,
    QueueItemLimit,
    QueueByteLimit,
    SenderItemLimit,
    SenderByteLimit,
    HistoryCapacityExhausted,
    UnknownMessage,
    AckNotVerifiedByAdapter,
    AckRecipientMismatch,
    AckPayloadMismatch,
    AckTimeInvalid,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MessageState {
    Pending,
    Delivered,
    Expired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Admission {
    Inserted,
    AlreadyKnown(MessageState),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RouteBlockReason {
    Unauthorized,
    SecurityProfileIncompatible,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RouteEligibility {
    pub authorized: bool,
    pub available: bool,
    pub security_profile_compatible: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScheduleResult {
    pub selected: Vec<MessageMetadata>,
    pub block_reason: Option<RouteBlockReason>,
    pub expired_now: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeliveryAck {
    pub message_id: String,
    pub recipient_id: String,
    pub payload_sha256: String,
    pub received_at_ms: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AckResult {
    MarkedDelivered,
    AlreadyDelivered,
}

#[derive(Clone, Debug)]
struct Record {
    metadata: MessageMetadata,
    state: MessageState,
    terminal_at_ms: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct QueueLedger {
    policy: QueuePolicy,
    records: BTreeMap<String, Record>,
}

impl QueueLedger {
    pub fn new(policy: QueuePolicy) -> Result<Self, QueueError> {
        policy.validate()?;
        Ok(Self {
            policy,
            records: BTreeMap::new(),
        })
    }

    pub fn policy(&self) -> &QueuePolicy {
        &self.policy
    }

    pub fn pending_count(&self) -> usize {
        self.records
            .values()
            .filter(|record| record.state == MessageState::Pending)
            .count()
    }

    pub fn pending_bytes(&self) -> u64 {
        self.records
            .values()
            .filter(|record| record.state == MessageState::Pending)
            .fold(0_u64, |total, record| {
                total.saturating_add(record.metadata.payload_len_bytes)
            })
    }

    pub fn state(&self, message_id: &str) -> Option<MessageState> {
        self.records.get(message_id).map(|record| record.state)
    }

    pub fn metadata(&self, message_id: &str) -> Option<&MessageMetadata> {
        self.records.get(message_id).map(|record| &record.metadata)
    }

    /// Admits metadata only. Payload bytes must be held by a separate,
    /// access-controlled persistence/transport adapter.
    pub fn enqueue(
        &mut self,
        now_ms: u64,
        metadata: MessageMetadata,
    ) -> Result<Admission, QueueError> {
        self.sweep_history(now_ms);

        if let Some(existing) = self.records.get(&metadata.message_id) {
            if existing.metadata == metadata {
                return Ok(Admission::AlreadyKnown(existing.state));
            }
            return Err(QueueError::DuplicateIdConflict);
        }

        validate_message(&self.policy, now_ms, &metadata)?;

        if self.records.len() >= self.policy.max_history_records {
            return Err(QueueError::HistoryCapacityExhausted);
        }
        if self.pending_count() >= self.policy.max_queue_items {
            return Err(QueueError::QueueItemLimit);
        }
        if self
            .pending_bytes()
            .checked_add(metadata.payload_len_bytes)
            .is_none_or(|total| total > self.policy.max_queue_bytes)
        {
            return Err(QueueError::QueueByteLimit);
        }

        let sender_records: Vec<&Record> = self
            .records
            .values()
            .filter(|record| {
                record.state == MessageState::Pending
                    && record.metadata.sender_id == metadata.sender_id
            })
            .collect();

        if sender_records.len() >= self.policy.max_items_per_sender {
            return Err(QueueError::SenderItemLimit);
        }
        let sender_bytes = sender_records.iter().fold(0_u64, |total, record| {
            total.saturating_add(record.metadata.payload_len_bytes)
        });
        if sender_bytes
            .checked_add(metadata.payload_len_bytes)
            .is_none_or(|total| total > self.policy.max_bytes_per_sender)
        {
            return Err(QueueError::SenderByteLimit);
        }

        self.records.insert(
            metadata.message_id.clone(),
            Record {
                metadata,
                state: MessageState::Pending,
                terminal_at_ms: None,
            },
        );
        Ok(Admission::Inserted)
    }

    /// Marks pending entries expired and prunes terminal dedupe history only
    /// after the configured retention interval. Returns IDs newly expired.
    pub fn sweep_history(&mut self, now_ms: u64) -> Vec<String> {
        let mut newly_expired = Vec::new();
        for (message_id, record) in &mut self.records {
            if record.state == MessageState::Pending && record.metadata.expires_at_ms <= now_ms {
                record.state = MessageState::Expired;
                record.terminal_at_ms = Some(now_ms);
                newly_expired.push(message_id.clone());
            }
        }

        let retention = self.policy.dedupe_retention_ms;
        self.records.retain(|_, record| {
            if record.state == MessageState::Pending {
                return true;
            }
            record
                .terminal_at_ms
                .is_none_or(|terminal_at| now_ms.saturating_sub(terminal_at) < retention)
        });
        newly_expired
    }

    /// Selects a deterministic batch by priority, then earliest expiry, creation
    /// time and stable ID. It does not mark messages delivered or claim a send.
    /// The caller must enforce route authorization and protocol security in a
    /// trusted transport adapter; this model receives that adapter's decision.
    pub fn select_batch(
        &mut self,
        now_ms: u64,
        route: RouteEligibility,
        max_items: usize,
        max_bytes: u64,
    ) -> ScheduleResult {
        let expired_now = self.sweep_history(now_ms);
        if !route.authorized {
            return ScheduleResult {
                selected: Vec::new(),
                block_reason: Some(RouteBlockReason::Unauthorized),
                expired_now,
            };
        }
        if !route.security_profile_compatible {
            return ScheduleResult {
                selected: Vec::new(),
                block_reason: Some(RouteBlockReason::SecurityProfileIncompatible),
                expired_now,
            };
        }
        if !route.available {
            return ScheduleResult {
                selected: Vec::new(),
                block_reason: Some(RouteBlockReason::Unavailable),
                expired_now,
            };
        }

        let mut pending: Vec<&MessageMetadata> = self
            .records
            .values()
            .filter(|record| {
                record.state == MessageState::Pending && record.metadata.expires_at_ms > now_ms
            })
            .map(|record| &record.metadata)
            .collect();

        pending.sort_by_key(|message| {
            (
                Reverse(message.priority.rank()),
                message.expires_at_ms,
                message.created_at_ms,
                message.message_id.clone(),
            )
        });

        let mut selected = Vec::new();
        let mut selected_bytes = 0_u64;
        for message in pending {
            if selected.len() >= max_items {
                break;
            }
            let Some(next_bytes) = selected_bytes.checked_add(message.payload_len_bytes) else {
                continue;
            };
            if next_bytes > max_bytes {
                continue;
            }
            selected_bytes = next_bytes;
            selected.push(message.clone());
        }

        ScheduleResult {
            selected,
            block_reason: None,
            expired_now,
        }
    }

    /// Applies a delivery acknowledgement supplied by a trusted transport
    /// adapter after that adapter verifies its signature/authentication. The
    /// boolean parameter is an assertion at this model boundary, not crypto.
    pub fn acknowledge(
        &mut self,
        now_ms: u64,
        ack: &DeliveryAck,
        adapter_verified_ack: bool,
    ) -> Result<AckResult, QueueError> {
        if !adapter_verified_ack {
            return Err(QueueError::AckNotVerifiedByAdapter);
        }

        let record = self
            .records
            .get_mut(&ack.message_id)
            .ok_or(QueueError::UnknownMessage)?;

        if record.metadata.recipient_id != ack.recipient_id {
            return Err(QueueError::AckRecipientMismatch);
        }
        if record.metadata.payload_sha256 != ack.payload_sha256 {
            return Err(QueueError::AckPayloadMismatch);
        }
        let minimum_received = record
            .metadata
            .created_at_ms
            .saturating_sub(self.policy.max_future_clock_skew_ms);
        let maximum_received = record
            .metadata
            .expires_at_ms
            .saturating_add(self.policy.max_future_clock_skew_ms);
        if ack.received_at_ms < minimum_received
            || ack.received_at_ms > maximum_received
            || ack.received_at_ms > now_ms.saturating_add(self.policy.max_future_clock_skew_ms)
        {
            return Err(QueueError::AckTimeInvalid);
        }

        if record.state == MessageState::Delivered {
            return Ok(AckResult::AlreadyDelivered);
        }
        record.state = MessageState::Delivered;
        record.terminal_at_ms = Some(now_ms);
        Ok(AckResult::MarkedDelivered)
    }
}

fn validate_message(
    policy: &QueuePolicy,
    now_ms: u64,
    message: &MessageMetadata,
) -> Result<(), QueueError> {
    if !valid_identifier(&message.message_id) {
        return Err(QueueError::InvalidMessageId);
    }
    if !valid_identifier(&message.sender_id) {
        return Err(QueueError::InvalidSenderId);
    }
    if !valid_identifier(&message.recipient_id) {
        return Err(QueueError::InvalidRecipientId);
    }
    if !valid_sha256(&message.payload_sha256) {
        return Err(QueueError::InvalidPayloadDigest);
    }
    if message.payload_len_bytes == 0 {
        return Err(QueueError::EmptyPayload);
    }
    if message.payload_len_bytes > policy.max_message_bytes {
        return Err(QueueError::MessageTooLarge);
    }
    if !policy.authorized_recipients.contains(&message.recipient_id) {
        return Err(QueueError::UnauthorizedRecipient);
    }
    if message.created_at_ms > now_ms.saturating_add(policy.max_future_clock_skew_ms) {
        return Err(QueueError::CreatedTooFarInFuture);
    }
    if message.expires_at_ms <= now_ms {
        return Err(QueueError::ExpiredAtAdmission);
    }
    let ttl_ms = message.ttl_ms().ok_or(QueueError::InvalidLifetime)?;
    if ttl_ms < policy.min_ttl_ms {
        return Err(QueueError::LifetimeTooShort);
    }
    if ttl_ms > policy.max_ttl_ms {
        return Err(QueueError::LifetimeTooLong);
    }
    if message.data_class == DataClass::Sensitive
        && (message.protection_declaration
            != ProtectionDeclaration::EndToEndAuthenticatedEncryption
            || !message.trusted_adapter_reports_protection_verified)
    {
        return Err(QueueError::SensitiveProtectionNotVerified);
    }
    Ok(())
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-')
        })
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> QueuePolicy {
        QueuePolicy {
            max_message_bytes: 1_000,
            max_queue_items: 10,
            max_queue_bytes: 5_000,
            max_items_per_sender: 3,
            max_bytes_per_sender: 2_000,
            min_ttl_ms: 100,
            max_ttl_ms: 10_000,
            max_future_clock_skew_ms: 50,
            dedupe_retention_ms: 5_000,
            max_history_records: 30,
            authorized_recipients: ["recipient-a", "recipient-b"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
        }
    }

    fn message(id: &str) -> MessageMetadata {
        MessageMetadata {
            message_id: id.to_owned(),
            sender_id: "sender-a".to_owned(),
            recipient_id: "recipient-a".to_owned(),
            created_at_ms: 1_000,
            expires_at_ms: 2_000,
            payload_len_bytes: 100,
            payload_sha256: "a".repeat(64),
            priority: Priority::Ordinary,
            data_class: DataClass::Community,
            protection_declaration: ProtectionDeclaration::EndToEndAuthenticatedEncryption,
            trusted_adapter_reports_protection_verified: true,
        }
    }

    fn available_route() -> RouteEligibility {
        RouteEligibility {
            authorized: true,
            available: true,
            security_profile_compatible: true,
        }
    }

    fn ack_for(message: &MessageMetadata) -> DeliveryAck {
        DeliveryAck {
            message_id: message.message_id.clone(),
            recipient_id: message.recipient_id.clone(),
            payload_sha256: message.payload_sha256.clone(),
            received_at_ms: 1_500,
        }
    }

    #[test]
    fn policy_rejects_zero_limits_and_empty_recipient_allowlist() {
        let mut invalid = policy();
        invalid.max_queue_bytes = 0;
        assert_eq!(invalid.validate(), Err(QueueError::InvalidPolicy));
        let mut invalid = policy();
        invalid.authorized_recipients.clear();
        assert_eq!(invalid.validate(), Err(QueueError::InvalidPolicy));
    }

    #[test]
    fn valid_message_is_admitted_and_counts_bytes() {
        let mut queue = QueueLedger::new(policy()).unwrap();
        assert_eq!(queue.enqueue(1_100, message("msg-001")), Ok(Admission::Inserted));
        assert_eq!(queue.pending_count(), 1);
        assert_eq!(queue.pending_bytes(), 100);
    }

    #[test]
    fn malformed_identifiers_and_digest_are_rejected() {
        let mut queue = QueueLedger::new(policy()).unwrap();
        let mut invalid = message("../escape");
        assert_eq!(queue.enqueue(1_100, invalid.clone()), Err(QueueError::InvalidMessageId));
        invalid = message("msg-002");
        invalid.sender_id = "sender\nA".to_owned();
        assert_eq!(queue.enqueue(1_100, invalid.clone()), Err(QueueError::InvalidSenderId));
        invalid = message("msg-003");
        invalid.payload_sha256 = "z".repeat(64);
        assert_eq!(queue.enqueue(1_100, invalid), Err(QueueError::InvalidPayloadDigest));
    }

    #[test]
    fn empty_and_overlong_payloads_are_rejected() {
        let mut queue = QueueLedger::new(policy()).unwrap();
        let mut empty = message("msg-004");
        empty.payload_len_bytes = 0;
        assert_eq!(queue.enqueue(1_100, empty), Err(QueueError::EmptyPayload));
        let mut large = message("msg-005");
        large.payload_len_bytes = 1_001;
        assert_eq!(queue.enqueue(1_100, large), Err(QueueError::MessageTooLarge));
    }

    #[test]
    fn future_created_and_already_expired_messages_are_rejected() {
        let mut queue = QueueLedger::new(policy()).unwrap();
        let mut future = message("msg-006");
        future.created_at_ms = 1_200;
        future.expires_at_ms = 2_200;
        assert_eq!(queue.enqueue(1_100, future), Err(QueueError::CreatedTooFarInFuture));
        let mut expired = message("msg-007");
        expired.expires_at_ms = 1_100;
        assert_eq!(queue.enqueue(1_100, expired), Err(QueueError::ExpiredAtAdmission));
    }

    #[test]
    fn invalid_ttl_bounds_are_rejected() {
        let mut queue = QueueLedger::new(policy()).unwrap();
        let mut short = message("msg-008");
        short.expires_at_ms = 1_050;
        assert_eq!(queue.enqueue(1_010, short), Err(QueueError::LifetimeTooShort));
        let mut long = message("msg-009");
        long.expires_at_ms = 20_000;
        assert_eq!(queue.enqueue(1_100, long), Err(QueueError::LifetimeTooLong));
    }

    #[test]
    fn unauthorized_recipient_is_rejected() {
        let mut queue = QueueLedger::new(policy()).unwrap();
        let mut invalid = message("msg-010");
        invalid.recipient_id = "unapproved-peer".to_owned();
        assert_eq!(queue.enqueue(1_100, invalid), Err(QueueError::UnauthorizedRecipient));
    }

    #[test]
    fn sensitive_message_needs_trusted_adapter_protection_gate() {
        let mut queue = QueueLedger::new(policy()).unwrap();
        let mut sensitive = message("msg-011");
        sensitive.data_class = DataClass::Sensitive;
        sensitive.trusted_adapter_reports_protection_verified = false;
        assert_eq!(queue.enqueue(1_100, sensitive.clone()), Err(QueueError::SensitiveProtectionNotVerified));
        sensitive.trusted_adapter_reports_protection_verified = true;
        assert_eq!(queue.enqueue(1_100, sensitive), Ok(Admission::Inserted));
    }

    #[test]
    fn repeated_identical_id_is_idempotent_but_conflicting_id_is_rejected() {
        let mut queue = QueueLedger::new(policy()).unwrap();
        let original = message("msg-012");
        assert_eq!(queue.enqueue(1_100, original.clone()), Ok(Admission::Inserted));
        assert_eq!(queue.enqueue(1_200, original.clone()), Ok(Admission::AlreadyKnown(MessageState::Pending)));
        let mut conflict = original;
        conflict.payload_sha256 = "b".repeat(64);
        assert_eq!(queue.enqueue(1_200, conflict), Err(QueueError::DuplicateIdConflict));
        assert_eq!(queue.pending_count(), 1);
    }

    #[test]
    fn total_item_and_byte_limits_are_enforced() {
        let mut p = policy();
        p.max_queue_items = 1;
        p.max_queue_bytes = 150;
        p.max_history_records = 3;
        let mut queue = QueueLedger::new(p).unwrap();
        assert_eq!(queue.enqueue(1_100, message("msg-013")), Ok(Admission::Inserted));
        assert_eq!(queue.enqueue(1_100, message("msg-014")), Err(QueueError::QueueItemLimit));

        let mut p = policy();
        p.max_queue_bytes = 150;
        p.max_queue_items = 10;
        p.max_bytes_per_sender = 2_000;
        let mut queue = QueueLedger::new(p).unwrap();
        assert_eq!(queue.enqueue(1_100, message("msg-015")), Ok(Admission::Inserted));
        let mut second = message("msg-016");
        second.sender_id = "sender-b".to_owned();
        second.payload_len_bytes = 100;
        assert_eq!(queue.enqueue(1_100, second), Err(QueueError::QueueByteLimit));
    }

    #[test]
    fn per_sender_quotas_are_enforced() {
        let mut p = policy();
        p.max_items_per_sender = 1;
        let mut queue = QueueLedger::new(p).unwrap();
        assert_eq!(queue.enqueue(1_100, message("msg-017")), Ok(Admission::Inserted));
        assert_eq!(queue.enqueue(1_100, message("msg-018")), Err(QueueError::SenderItemLimit));

        let mut p = policy();
        p.max_bytes_per_sender = 150;
        let mut queue = QueueLedger::new(p).unwrap();
        assert_eq!(queue.enqueue(1_100, message("msg-019")), Ok(Admission::Inserted));
        assert_eq!(queue.enqueue(1_100, message("msg-020")), Err(QueueError::SenderByteLimit));
    }

    #[test]
    fn no_route_or_unauthorized_route_never_selects_messages() {
        let mut queue = QueueLedger::new(policy()).unwrap();
        queue.enqueue(1_100, message("msg-021")).unwrap();
        let mut route = available_route();
        route.authorized = false;
        let result = queue.select_batch(1_200, route, 10, 5_000);
        assert_eq!(result.block_reason, Some(RouteBlockReason::Unauthorized));
        assert!(result.selected.is_empty());

        let mut route = available_route();
        route.available = false;
        let result = queue.select_batch(1_200, route, 10, 5_000);
        assert_eq!(result.block_reason, Some(RouteBlockReason::Unavailable));
        assert!(result.selected.is_empty());
    }

    #[test]
    fn incompatible_security_profile_blocks_route() {
        let mut queue = QueueLedger::new(policy()).unwrap();
        queue.enqueue(1_100, message("msg-022")).unwrap();
        let mut route = available_route();
        route.security_profile_compatible = false;
        let result = queue.select_batch(1_200, route, 10, 5_000);
        assert_eq!(result.block_reason, Some(RouteBlockReason::SecurityProfileIncompatible));
        assert!(result.selected.is_empty());
    }

    #[test]
    fn expired_messages_are_never_selected_for_forwarding() {
        let mut queue = QueueLedger::new(policy()).unwrap();
        let mut expired = message("msg-023");
        expired.expires_at_ms = 1_150;
        queue.enqueue(1_100, expired).unwrap();
        let result = queue.select_batch(1_150, available_route(), 10, 5_000);
        assert_eq!(result.selected.len(), 0);
        assert_eq!(result.expired_now, vec!["msg-023".to_owned()]);
        assert_eq!(queue.state("msg-023"), Some(MessageState::Expired));
    }

    #[test]
    fn scheduling_is_deterministic_by_priority_expiry_and_id() {
        let mut queue = QueueLedger::new(policy()).unwrap();
        let mut ordinary_z = message("msg-z");
        ordinary_z.expires_at_ms = 1_800;
        let mut essential = message("msg-essential");
        essential.priority = Priority::Essential;
        essential.expires_at_ms = 1_900;
        let mut bulk = message("msg-bulk");
        bulk.priority = Priority::Bulk;
        let mut ordinary_a = message("msg-a");
        ordinary_a.expires_at_ms = 1_800;
        queue.enqueue(1_100, ordinary_z).unwrap();
        queue.enqueue(1_100, bulk).unwrap();
        queue.enqueue(1_100, ordinary_a).unwrap();
        queue.enqueue(1_100, essential).unwrap();

        let result = queue.select_batch(1_200, available_route(), 10, 5_000);
        let ids: Vec<&str> = result.selected.iter().map(|message| message.message_id.as_str()).collect();
        assert_eq!(ids, vec!["msg-essential", "msg-a", "msg-z", "msg-bulk"]);
    }

    #[test]
    fn scheduling_respects_batch_count_and_byte_budget() {
        let mut queue = QueueLedger::new(policy()).unwrap();
        let mut large = message("msg-large");
        large.payload_len_bytes = 900;
        large.priority = Priority::Essential;
        let mut small = message("msg-small");
        small.payload_len_bytes = 200;
        queue.enqueue(1_100, large).unwrap();
        queue.enqueue(1_100, small).unwrap();
        let result = queue.select_batch(1_200, available_route(), 2, 300);
        assert_eq!(result.selected.len(), 1);
        assert_eq!(result.selected[0].message_id, "msg-small");
    }

    #[test]
    fn acknowledgements_require_adapter_verification_and_match_recipient_and_digest() {
        let mut queue = QueueLedger::new(policy()).unwrap();
        let message = message("msg-024");
        queue.enqueue(1_100, message.clone()).unwrap();
        let ack = ack_for(&message);
        assert_eq!(queue.acknowledge(1_600, &ack, false), Err(QueueError::AckNotVerifiedByAdapter));

        let mut wrong_recipient = ack.clone();
        wrong_recipient.recipient_id = "recipient-b".to_owned();
        assert_eq!(queue.acknowledge(1_600, &wrong_recipient, true), Err(QueueError::AckRecipientMismatch));

        let mut wrong_digest = ack.clone();
        wrong_digest.payload_sha256 = "b".repeat(64);
        assert_eq!(queue.acknowledge(1_600, &wrong_digest, true), Err(QueueError::AckPayloadMismatch));

        assert_eq!(queue.acknowledge(1_600, &ack, true), Ok(AckResult::MarkedDelivered));
        assert_eq!(queue.state("msg-024"), Some(MessageState::Delivered));
        assert_eq!(queue.pending_count(), 0);
        assert_eq!(queue.acknowledge(1_600, &ack, true), Ok(AckResult::AlreadyDelivered));
    }

    #[test]
    fn acknowledgement_outside_message_lifetime_plus_skew_is_rejected() {
        let mut queue = QueueLedger::new(policy()).unwrap();
        let mut message = message("msg-025");
        message.expires_at_ms = 2_000;
        queue.enqueue(1_100, message.clone()).unwrap();
        let mut ack = ack_for(&message);
        ack.received_at_ms = 2_100;
        assert_eq!(queue.acknowledge(2_200, &ack, true), Err(QueueError::AckTimeInvalid));
    }

    #[test]
    fn terminal_dedupe_records_are_retained_then_pruned_by_policy() {
        let mut p = policy();
        p.dedupe_retention_ms = 500;
        let mut queue = QueueLedger::new(p).unwrap();
        let message = message("msg-026");
        queue.enqueue(1_100, message.clone()).unwrap();
        let ack = ack_for(&message);
        queue.acknowledge(1_600, &ack, true).unwrap();
        assert_eq!(queue.enqueue(1_700, message.clone()), Ok(Admission::AlreadyKnown(MessageState::Delivered)));
        queue.sweep_history(2_100);
        assert_eq!(queue.state("msg-026"), None);
        let mut fresh = message;
        fresh.created_at_ms = 2_100;
        fresh.expires_at_ms = 3_100;
        assert_eq!(queue.enqueue(2_100, fresh), Ok(Admission::Inserted));
    }

    #[test]
    fn synthetic_partition_recovery_drains_one_thousand_unexpired_messages() {
        let mut p = policy();
        p.max_message_bytes = 1_024;
        p.max_queue_items = 1_000;
        p.max_queue_bytes = 200_000;
        p.max_items_per_sender = 50;
        p.max_bytes_per_sender = 20_000;
        p.max_ttl_ms = 10_000;
        p.max_history_records = 1_200;
        let mut queue = QueueLedger::new(p).unwrap();

        for index in 0..1_000_u64 {
            let mut queued = message(&format!("replay-{index:04}"));
            queued.sender_id = format!("sender-{:03}", index % 100);
            queued.recipient_id = if index % 2 == 0 {
                "recipient-a".to_owned()
            } else {
                "recipient-b".to_owned()
            };
            queued.created_at_ms = 1_000;
            queued.expires_at_ms = 10_000;
            queued.payload_len_bytes = 100;
            queued.priority = match index % 3 {
                0 => Priority::Essential,
                1 => Priority::Ordinary,
                _ => Priority::Bulk,
            };
            queue.enqueue(1_100, queued).expect("bounded synthetic message is admitted");
        }

        assert_eq!(queue.pending_count(), 1_000);
        assert_eq!(queue.pending_bytes(), 100_000);

        let mut delivered = 0_usize;
        while queue.pending_count() > 0 {
            let scheduled = queue.select_batch(2_000, available_route(), 50, 5_000);
            assert_eq!(scheduled.block_reason, None);
            assert!(!scheduled.selected.is_empty(), "pending queue must make progress");
            for selected in scheduled.selected {
                let ack = DeliveryAck {
                    message_id: selected.message_id.clone(),
                    recipient_id: selected.recipient_id.clone(),
                    payload_sha256: selected.payload_sha256.clone(),
                    received_at_ms: 2_000,
                };
                assert_eq!(
                    queue.acknowledge(2_000, &ack, true),
                    Ok(AckResult::MarkedDelivered)
                );
                delivered += 1;
            }
        }

        assert_eq!(delivered, 1_000);
        assert_eq!(queue.pending_count(), 0);
        assert_eq!(queue.pending_bytes(), 0);
        assert_eq!(queue.state("replay-0000"), Some(MessageState::Delivered));
        assert_eq!(queue.state("replay-0999"), Some(MessageState::Delivered));
    }

    #[test]
    fn history_capacity_is_bounded() {
        let mut p = policy();
        p.max_queue_items = 1;
        p.max_history_records = 1;
        let mut queue = QueueLedger::new(p).unwrap();
        queue.enqueue(1_100, message("msg-027")).unwrap();
        let ack = ack_for(&message("msg-027"));
        queue.acknowledge(1_600, &ack, true).unwrap();
        assert_eq!(queue.enqueue(1_700, message("msg-028")), Err(QueueError::HistoryCapacityExhausted));
        queue.sweep_history(7_000);
        let mut fresh = message("msg-028");
        fresh.created_at_ms = 7_000;
        fresh.expires_at_ms = 8_000;
        assert_eq!(queue.enqueue(7_000, fresh), Ok(Admission::Inserted));
    }
}
