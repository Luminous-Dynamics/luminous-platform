#![forbid(unsafe_code)]

//! Fail-closed policy decision core for a Linux syscall-sandbox adapter.
//!
//! SECURITY BOUNDARY:
//! - This crate does not intercept syscalls, install seccomp filters, attach
//!   Landlock rulesets, authenticate signatures, resolve paths, or observe OS state.
//! - Its inputs are normalized facts. An adapter must only mark a fact \`Verified\`
//!   after independently validating the real syscall/arguments/resource binding.
//! - A returned \`AllowReceipt\` is a decision record, never a kernel-enforcement token.
//! - Unknown syscalls, unsupported multi-resource operations, stale/revoked policy,
//!   and missing or contradictory evidence all fail closed.
//!
//! The core intentionally requires one capability to cover every operation implied
//! by a syscall. It never unions separate grants to manufacture a broader permission.

use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Digest(String);

impl Digest {
    pub fn parse(value: impl Into<String>) -> Result<Self, &'static str> {
        let value = value.into();
        if value.len() != 64
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err("digest must be 64 lowercase hexadecimal characters");
        }
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ResourceKind {
    File,
    DirectoryEntry,
    Executable,
    NetworkEndpoint,
    Process,
    Namespace,
    KernelObject,
    MemoryRegion,
    Other,
}

/// The stable digest must bind the resolved object, not only a caller-supplied path
/// string. Resolution and TOCTOU protection are obligations of the kernel adapter.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ResourceIdentity {
    pub kind: ResourceKind,
    pub stable_digest: Digest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Operation {
    Read,
    Write,
    OpenRead,
    OpenWrite,
    Create,
    Truncate,
    Delete,
    Execute,
    NetworkConnect,
    NetworkBind,
    NetworkListen,
    NetworkAccept,
    ProcessSignal,
    ProcessInspect,
    MountChange,
    NamespaceChange,
    KernelModuleLoad,
    BpfProgram,
    PerfEventOpen,
    IoUringSetup,
    IdentityChange,
    MemoryMap,
    MemoryProtect,
    MemoryRead,
    MemoryWrite,
    MemoryExecute,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceStatus {
    Verified,
    Missing,
    Unverified,
    Contradictory,
}

/// A value is usable only when status is Verified and value is present.
/// The real adapter is responsible for establishing the truth of that status.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fact<T> {
    pub status: EvidenceStatus,
    pub value: Option<T>,
}

impl<T> Fact<T> {
    #[must_use]
    pub fn verified(value: T) -> Self {
        Self {
            status: EvidenceStatus::Verified,
            value: Some(value),
        }
    }

    #[must_use]
    pub fn missing() -> Self {
        Self {
            status: EvidenceStatus::Missing,
            value: None,
        }
    }

    #[must_use]
    pub fn unverified(value: T) -> Self {
        Self {
            status: EvidenceStatus::Unverified,
            value: Some(value),
        }
    }

    #[must_use]
    pub fn contradictory(value: T) -> Self {
        Self {
            status: EvidenceStatus::Contradictory,
            value: Some(value),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthenticationState {
    Verified,
    Missing,
    Invalid,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RevocationState {
    Clear,
    Revoked,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapabilityState {
    Active,
    Revoked,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClockState {
    Fresh,
    Unavailable,
    Reversed,
}

/// This is a normalized syscall family, not a raw kernel syscall number.
/// The adapter must decode syscall number and argument semantics before creating it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Syscall {
    Read,
    Pread64,
    Write,
    Pwrite64,
    OpenAt,
    OpenAt2,
    Creat,
    UnlinkAt,
    RenameAt2,
    Execve,
    ExecveAt,
    Connect,
    Bind,
    Listen,
    Accept4,
    Clone,
    Clone3,
    Fork,
    Vfork,
    Kill,
    Tgkill,
    Ptrace,
    Mount,
    Umount2,
    PivotRoot,
    Setns,
    Unshare,
    InitModule,
    FinitModule,
    DeleteModule,
    Bpf,
    PerfEventOpen,
    IoUringSetup,
    Setuid,
    Setgid,
    Mmap,
    Mprotect,
    Unknown(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenAccessMode {
    ReadOnly,
    WriteOnly,
    ReadWrite,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpenArguments {
    pub access: OpenAccessMode,
    pub create: bool,
    pub truncate: bool,
    pub append: bool,
    /// True when the decoder saw flags whose policy semantics it does not support.
    pub unsupported_flags: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryArguments {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
    /// True when protection bits outside the supported contract were observed.
    pub unsupported_protection_bits: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArgumentFacts {
    None,
    Open(OpenArguments),
    Memory(MemoryArguments),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SyscallCall {
    pub syscall: Syscall,
    pub arguments: ArgumentFacts,
}

impl SyscallCall {
    fn required_operations(self) -> Result<Vec<Operation>, DenyReason> {
        use ArgumentFacts::{Memory, None, Open};
        use Operation as Op;
        use Syscall as Sc;

        match self.syscall {
            Sc::Read | Sc::Pread64 => {
                require_no_arguments(self.arguments)?;
                Ok(vec![Op::Read])
            }
            Sc::Write | Sc::Pwrite64 => {
                require_no_arguments(self.arguments)?;
                Ok(vec![Op::Write])
            }
            Sc::OpenAt | Sc::OpenAt2 => {
                let Open(arguments) = self.arguments else {
                    return Err(DenyReason::MissingOrUnexpectedArgumentFacts);
                };
                if arguments.unsupported_flags {
                    return Err(DenyReason::UnsupportedArgumentSemantics);
                }
                if arguments.access == OpenAccessMode::Unknown {
                    return Err(DenyReason::UnsupportedArgumentSemantics);
                }
                if arguments.truncate && arguments.access == OpenAccessMode::ReadOnly {
                    return Err(DenyReason::InconsistentArgumentFacts);
                }
                if arguments.append && arguments.access == OpenAccessMode::ReadOnly {
                    return Err(DenyReason::InconsistentArgumentFacts);
                }

                let mut operations = Vec::with_capacity(4);
                match arguments.access {
                    OpenAccessMode::ReadOnly => operations.push(Op::OpenRead),
                    OpenAccessMode::WriteOnly => operations.push(Op::OpenWrite),
                    OpenAccessMode::ReadWrite => {
                        operations.push(Op::OpenRead);
                        operations.push(Op::OpenWrite);
                    }
                    OpenAccessMode::Unknown => {
                        return Err(DenyReason::UnsupportedArgumentSemantics);
                    }
                }
                if arguments.create {
                    operations.push(Op::Create);
                }
                if arguments.truncate {
                    operations.push(Op::Truncate);
                }
                Ok(operations)
            }
            Sc::Creat => {
                require_no_arguments(self.arguments)?;
                Ok(vec![Op::Create, Op::OpenWrite, Op::Truncate])
            }
            Sc::UnlinkAt => {
                require_no_arguments(self.arguments)?;
                Ok(vec![Op::Delete])
            }
            // Rename and process-spawn operations bind multiple resources and
            // require an inheritance/source+destination contract not represented
            // in this v1 API. Deny rather than collapse that scope to one object.
            Sc::RenameAt2 | Sc::Clone | Sc::Clone3 | Sc::Fork | Sc::Vfork => {
                Err(DenyReason::UnsupportedMultiResourceOperation)
            }
            Sc::Execve | Sc::ExecveAt => {
                require_no_arguments(self.arguments)?;
                Ok(vec![Op::Execute])
            }
            Sc::Connect => {
                require_no_arguments(self.arguments)?;
                Ok(vec![Op::NetworkConnect])
            }
            Sc::Bind => {
                require_no_arguments(self.arguments)?;
                Ok(vec![Op::NetworkBind])
            }
            Sc::Listen => {
                require_no_arguments(self.arguments)?;
                Ok(vec![Op::NetworkListen])
            }
            Sc::Accept4 => {
                require_no_arguments(self.arguments)?;
                Ok(vec![Op::NetworkAccept])
            }
            Sc::Kill | Sc::Tgkill => {
                require_no_arguments(self.arguments)?;
                Ok(vec![Op::ProcessSignal])
            }
            Sc::Ptrace => {
                require_no_arguments(self.arguments)?;
                Ok(vec![Op::ProcessInspect])
            }
            Sc::Mount | Sc::Umount2 | Sc::PivotRoot => {
                require_no_arguments(self.arguments)?;
                Ok(vec![Op::MountChange])
            }
            Sc::Setns | Sc::Unshare => {
                require_no_arguments(self.arguments)?;
                Ok(vec![Op::NamespaceChange])
            }
            Sc::InitModule | Sc::FinitModule | Sc::DeleteModule => {
                require_no_arguments(self.arguments)?;
                Ok(vec![Op::KernelModuleLoad])
            }
            Sc::Bpf => {
                require_no_arguments(self.arguments)?;
                Ok(vec![Op::BpfProgram])
            }
            Sc::PerfEventOpen => {
                require_no_arguments(self.arguments)?;
                Ok(vec![Op::PerfEventOpen])
            }
            Sc::IoUringSetup => {
                require_no_arguments(self.arguments)?;
                Ok(vec![Op::IoUringSetup])
            }
            Sc::Setuid | Sc::Setgid => {
                require_no_arguments(self.arguments)?;
                Ok(vec![Op::IdentityChange])
            }
            Sc::Mmap | Sc::Mprotect => {
                let Memory(arguments) = self.arguments else {
                    return Err(DenyReason::MissingOrUnexpectedArgumentFacts);
                };
                if arguments.unsupported_protection_bits {
                    return Err(DenyReason::UnsupportedArgumentSemantics);
                }
                if arguments.write && arguments.execute {
                    return Err(DenyReason::WritableExecutableMemory);
                }

                let mut operations = vec![if self.syscall == Sc::Mmap {
                    Op::MemoryMap
                } else {
                    Op::MemoryProtect
                }];
                if arguments.read {
                    operations.push(Op::MemoryRead);
                }
                if arguments.write {
                    operations.push(Op::MemoryWrite);
                }
                if arguments.execute {
                    operations.push(Op::MemoryExecute);
                }
                Ok(operations)
            }
            Sc::Unknown(_) => Err(DenyReason::UnknownSyscall),
        }
    }
}

fn require_no_arguments(arguments: ArgumentFacts) -> Result<(), DenyReason> {
    if arguments == ArgumentFacts::None {
        Ok(())
    } else {
        Err(DenyReason::MissingOrUnexpectedArgumentFacts)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Capability {
    pub id: String,
    pub target_id: String,
    pub subject_image_digest: Digest,
    pub resource: ResourceIdentity,
    pub operations: Vec<Operation>,
    pub valid_from_ms: u64,
    pub expires_at_ms: u64,
    pub state: CapabilityState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicySnapshot {
    pub id: String,
    pub digest: Digest,
    pub target_id: String,
    pub epoch: u64,
    pub valid_from_ms: u64,
    pub expires_at_ms: u64,
    pub authentication: AuthenticationState,
    pub revocation: RevocationState,
    pub capabilities: Vec<Capability>,
}

impl PolicySnapshot {
    fn validate_structure(&self) -> Result<(), DenyReason> {
        if self.id.trim().is_empty()
            || self.target_id.trim().is_empty()
            || self.epoch == 0
            || self.valid_from_ms >= self.expires_at_ms
        {
            return Err(DenyReason::MalformedPolicy);
        }

        let mut capability_ids = HashSet::new();
        for capability in &self.capabilities {
            if capability.id.trim().is_empty()
                || !capability_ids.insert(capability.id.as_str())
                || capability.target_id != self.target_id
                || capability.valid_from_ms >= capability.expires_at_ms
                || capability.valid_from_ms < self.valid_from_ms
                || capability.expires_at_ms > self.expires_at_ms
                || capability.operations.is_empty()
            {
                return Err(DenyReason::MalformedPolicy);
            }
            let mut operations = HashSet::new();
            if capability
                .operations
                .iter()
                .any(|operation| !operations.insert(*operation))
            {
                return Err(DenyReason::MalformedPolicy);
            }
        }
        Ok(())
    }
}

/// All facts for a single syscall attempt are bundled together so callers cannot
/// accidentally combine a resource from one event with syscall facts from another.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservedOperation {
    pub target_id: String,
    pub subject_image_digest: Digest,
    pub resource: ResourceIdentity,
    pub call: SyscallCall,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub observed: Fact<ObservedOperation>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvaluationContext {
    /// Must use a consistent monotonic/freshness time basis established by an adapter.
    pub now_ms: Option<u64>,
    pub clock_state: ClockState,
    /// Digest pinned by the independently verified launch/task contract.
    pub expected_policy_digest: Option<Digest>,
    /// Rollback floor from a durable verifier/lifecycle witness; not caller preference.
    pub minimum_accepted_epoch: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DenyReason {
    MalformedPolicy,
    MissingClock,
    ClockUnavailable,
    ClockReversed,
    MissingPinnedPolicyDigest,
    PolicyDigestMismatch,
    MissingRollbackFloor,
    PolicyEpochRollback,
    PolicyNotAuthenticated,
    PolicyAuthenticationInvalid,
    PolicyAuthenticationUnknown,
    PolicyRevoked,
    PolicyRevocationUnknown,
    PolicyNotYetValid,
    PolicyExpired,
    MissingObservedOperation,
    UnverifiedObservedOperation,
    ContradictoryObservedOperation,
    MissingTargetBinding,
    TargetMismatch,
    UnknownSyscall,
    MissingOrUnexpectedArgumentFacts,
    UnsupportedArgumentSemantics,
    InconsistentArgumentFacts,
    WritableExecutableMemory,
    UnsupportedMultiResourceOperation,
    InvalidResourceKind,
    CapabilityNotYetValid,
    CapabilityExpired,
    CapabilityRevoked,
    CapabilityRevocationUnknown,
    NoMatchingCapability,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AllowReceipt {
    pub policy_id: String,
    pub policy_digest: Digest,
    pub policy_epoch: u64,
    pub capability_id: String,
    pub target_id: String,
    pub subject_image_digest: Digest,
    pub resource: ResourceIdentity,
    pub syscall: Syscall,
    pub operations: Vec<Operation>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decision {
    Allow(AllowReceipt),
    Deny(DenyReason),
}

#[must_use]
pub fn evaluate(
    policy: &PolicySnapshot,
    request: &Request,
    context: &EvaluationContext,
) -> Decision {
    if let Err(reason) = policy.validate_structure() {
        return Decision::Deny(reason);
    }

    let now_ms = match context.clock_state {
        ClockState::Unavailable => return Decision::Deny(DenyReason::ClockUnavailable),
        ClockState::Reversed => return Decision::Deny(DenyReason::ClockReversed),
        ClockState::Fresh => match context.now_ms {
            Some(value) => value,
            None => return Decision::Deny(DenyReason::MissingClock),
        },
    };

    let Some(expected_policy_digest) = context.expected_policy_digest.as_ref() else {
        return Decision::Deny(DenyReason::MissingPinnedPolicyDigest);
    };
    if &policy.digest != expected_policy_digest {
        return Decision::Deny(DenyReason::PolicyDigestMismatch);
    }

    let Some(minimum_epoch) = context.minimum_accepted_epoch else {
        return Decision::Deny(DenyReason::MissingRollbackFloor);
    };
    if minimum_epoch == 0 || policy.epoch < minimum_epoch {
        return Decision::Deny(DenyReason::PolicyEpochRollback);
    }

    match policy.authentication {
        AuthenticationState::Verified => {}
        AuthenticationState::Missing => {
            return Decision::Deny(DenyReason::PolicyNotAuthenticated);
        }
        AuthenticationState::Invalid => {
            return Decision::Deny(DenyReason::PolicyAuthenticationInvalid);
        }
        AuthenticationState::Unknown => {
            return Decision::Deny(DenyReason::PolicyAuthenticationUnknown);
        }
    }

    match policy.revocation {
        RevocationState::Clear => {}
        RevocationState::Revoked => return Decision::Deny(DenyReason::PolicyRevoked),
        RevocationState::Unknown => return Decision::Deny(DenyReason::PolicyRevocationUnknown),
    }

    if now_ms < policy.valid_from_ms {
        return Decision::Deny(DenyReason::PolicyNotYetValid);
    }
    if now_ms >= policy.expires_at_ms {
        return Decision::Deny(DenyReason::PolicyExpired);
    }

    let observed = match fact_ref(
        &request.observed,
        DenyReason::MissingObservedOperation,
        DenyReason::UnverifiedObservedOperation,
        DenyReason::ContradictoryObservedOperation,
    ) {
        Ok(value) => value,
        Err(reason) => return Decision::Deny(reason),
    };

    let target_id = &observed.target_id;
    if target_id.trim().is_empty() {
        return Decision::Deny(DenyReason::MissingTargetBinding);
    }
    if target_id != &policy.target_id {
        return Decision::Deny(DenyReason::TargetMismatch);
    }

    let subject_digest = &observed.subject_image_digest;
    let resource = &observed.resource;
    let call = &observed.call;

    let operations = match call.required_operations() {
        Ok(operations) => operations,
        Err(reason) => return Decision::Deny(reason),
    };
    if operations
        .iter()
        .any(|operation| !resource_kind_compatible(*operation, resource.kind))
    {
        return Decision::Deny(DenyReason::InvalidResourceKind);
    }

    let mut deferred_reason = None;
    for capability in &policy.capabilities {
        if capability.target_id != *target_id
            || capability.subject_image_digest != *subject_digest
            || capability.resource != *resource
            || !operations
                .iter()
                .all(|operation| capability.operations.contains(operation))
        {
            continue;
        }

        match capability.state {
            CapabilityState::Revoked => {
                deferred_reason = Some(DenyReason::CapabilityRevoked);
                continue;
            }
            CapabilityState::Unknown => {
                deferred_reason = Some(DenyReason::CapabilityRevocationUnknown);
                continue;
            }
            CapabilityState::Active => {}
        }
        if now_ms < capability.valid_from_ms {
            deferred_reason = Some(DenyReason::CapabilityNotYetValid);
            continue;
        }
        if now_ms >= capability.expires_at_ms {
            deferred_reason = Some(DenyReason::CapabilityExpired);
            continue;
        }

        return Decision::Allow(AllowReceipt {
            policy_id: policy.id.clone(),
            policy_digest: policy.digest.clone(),
            policy_epoch: policy.epoch,
            capability_id: capability.id.clone(),
            target_id: target_id.clone(),
            subject_image_digest: subject_digest.clone(),
            resource: resource.clone(),
            syscall: call.syscall,
            operations,
        });
    }

    Decision::Deny(deferred_reason.unwrap_or(DenyReason::NoMatchingCapability))
}

fn fact_ref<'a, T>(
    fact: &'a Fact<T>,
    missing: DenyReason,
    unverified: DenyReason,
    contradictory: DenyReason,
) -> Result<&'a T, DenyReason> {
    match fact.status {
        EvidenceStatus::Missing => Err(missing),
        EvidenceStatus::Unverified => Err(unverified),
        EvidenceStatus::Contradictory => Err(contradictory),
        EvidenceStatus::Verified => fact.value.as_ref().ok_or(missing),
    }
}

fn resource_kind_compatible(operation: Operation, kind: ResourceKind) -> bool {
    use Operation as Op;
    use ResourceKind as Kind;

    match operation {
        Op::Read | Op::Write => matches!(
            kind,
            Kind::File | Kind::DirectoryEntry | Kind::NetworkEndpoint | Kind::KernelObject
        ),
        Op::OpenRead | Op::OpenWrite | Op::Truncate => {
            matches!(kind, Kind::File | Kind::DirectoryEntry)
        }
        Op::Create | Op::Delete => kind == Kind::DirectoryEntry,
        Op::Execute => kind == Kind::Executable,
        Op::NetworkConnect | Op::NetworkBind | Op::NetworkListen | Op::NetworkAccept => {
            kind == Kind::NetworkEndpoint
        }
        Op::ProcessSignal | Op::ProcessInspect | Op::IdentityChange => kind == Kind::Process,
        Op::MountChange | Op::NamespaceChange => kind == Kind::Namespace,
        Op::KernelModuleLoad | Op::BpfProgram | Op::PerfEventOpen | Op::IoUringSetup => {
            kind == Kind::KernelObject
        }
        Op::MemoryMap
        | Op::MemoryProtect
        | Op::MemoryRead
        | Op::MemoryWrite
        | Op::MemoryExecute => kind == Kind::MemoryRegion,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(ch: char) -> Digest {
        Digest::parse(ch.to_string().repeat(64)).expect("test digest is canonical")
    }

    fn resource(kind: ResourceKind, ch: char) -> ResourceIdentity {
        ResourceIdentity {
            kind,
            stable_digest: digest(ch),
        }
    }

    fn baseline_policy() -> PolicySnapshot {
        PolicySnapshot {
            id: "policy-read-fixture".into(),
            digest: digest('a'),
            target_id: "endpoint-a".into(),
            epoch: 7,
            valid_from_ms: 1_000,
            expires_at_ms: 10_000,
            authentication: AuthenticationState::Verified,
            revocation: RevocationState::Clear,
            capabilities: vec![Capability {
                id: "read-canary".into(),
                target_id: "endpoint-a".into(),
                subject_image_digest: digest('b'),
                resource: resource(ResourceKind::File, 'c'),
                operations: vec![Operation::OpenRead],
                valid_from_ms: 1_000,
                expires_at_ms: 9_000,
                state: CapabilityState::Active,
            }],
        }
    }

    fn request() -> Request {
        Request {
            observed: Fact::verified(ObservedOperation {
                target_id: "endpoint-a".into(),
                subject_image_digest: digest('b'),
                resource: resource(ResourceKind::File, 'c'),
                call: SyscallCall {
                    syscall: Syscall::OpenAt,
                    arguments: ArgumentFacts::Open(OpenArguments {
                        access: OpenAccessMode::ReadOnly,
                        create: false,
                        truncate: false,
                        append: false,
                        unsupported_flags: false,
                    }),
                },
            }),
        }
    }

    fn set_call(request: &mut Request, call: SyscallCall) {
        request.observed.value.as_mut().expect("test observation exists").call = call;
    }

    fn set_resource(request: &mut Request, resource: ResourceIdentity) {
        request.observed.value.as_mut().expect("test observation exists").resource = resource;
    }

    fn set_target(request: &mut Request, target_id: &str) {
        request.observed.value.as_mut().expect("test observation exists").target_id = target_id.into();
    }

    fn set_subject(request: &mut Request, subject_image_digest: Digest) {
        request.observed.value.as_mut().expect("test observation exists").subject_image_digest = subject_image_digest;
    }

    fn context() -> EvaluationContext {
        EvaluationContext {
            now_ms: Some(2_000),
            clock_state: ClockState::Fresh,
            expected_policy_digest: Some(digest('a')),
            minimum_accepted_epoch: Some(7),
        }
    }

    fn denied_reason(
        policy: &PolicySnapshot,
        request: &Request,
        context: &EvaluationContext,
    ) -> DenyReason {
        match evaluate(policy, request, context) {
            Decision::Allow(_) => panic!("expected denial"),
            Decision::Deny(reason) => reason,
        }
    }

    #[test]
    fn exact_scope_and_verified_context_allow_read() {
        let result = evaluate(&baseline_policy(), &request(), &context());
        let Decision::Allow(receipt) = result else {
            panic!("expected allow, got {result:?}");
        };
        assert_eq!(receipt.capability_id, "read-canary");
        assert_eq!(receipt.policy_epoch, 7);
        assert_eq!(receipt.resource, resource(ResourceKind::File, 'c'));
        assert_eq!(receipt.operations, vec![Operation::OpenRead]);
    }

    #[test]
    fn missing_clock_fails_closed() {
        let mut ctx = context();
        ctx.now_ms = None;
        assert_eq!(
            denied_reason(&baseline_policy(), &request(), &ctx),
            DenyReason::MissingClock
        );
    }

    #[test]
    fn reversed_or_unavailable_clock_fails_closed() {
        let mut ctx = context();
        ctx.clock_state = ClockState::Reversed;
        assert_eq!(
            denied_reason(&baseline_policy(), &request(), &ctx),
            DenyReason::ClockReversed
        );
        ctx.clock_state = ClockState::Unavailable;
        assert_eq!(
            denied_reason(&baseline_policy(), &request(), &ctx),
            DenyReason::ClockUnavailable
        );
    }

    #[test]
    fn missing_pinned_digest_fails_closed() {
        let mut ctx = context();
        ctx.expected_policy_digest = None;
        assert_eq!(
            denied_reason(&baseline_policy(), &request(), &ctx),
            DenyReason::MissingPinnedPolicyDigest
        );
    }

    #[test]
    fn policy_digest_substitution_is_rejected() {
        let mut ctx = context();
        ctx.expected_policy_digest = Some(digest('d'));
        assert_eq!(
            denied_reason(&baseline_policy(), &request(), &ctx),
            DenyReason::PolicyDigestMismatch
        );
    }

    #[test]
    fn missing_epoch_floor_fails_closed() {
        let mut ctx = context();
        ctx.minimum_accepted_epoch = None;
        assert_eq!(
            denied_reason(&baseline_policy(), &request(), &ctx),
            DenyReason::MissingRollbackFloor
        );
    }

    #[test]
    fn stale_policy_epoch_is_rejected() {
        let policy = baseline_policy();
        let mut ctx = context();
        ctx.minimum_accepted_epoch = Some(8);
        assert_eq!(
            denied_reason(&policy, &request(), &ctx),
            DenyReason::PolicyEpochRollback
        );
    }

    #[test]
    fn unverified_or_invalid_policy_never_allows() {
        let mut policy = baseline_policy();
        policy.authentication = AuthenticationState::Missing;
        assert_eq!(
            denied_reason(&policy, &request(), &context()),
            DenyReason::PolicyNotAuthenticated
        );
        policy.authentication = AuthenticationState::Invalid;
        assert_eq!(
            denied_reason(&policy, &request(), &context()),
            DenyReason::PolicyAuthenticationInvalid
        );
        policy.authentication = AuthenticationState::Unknown;
        assert_eq!(
            denied_reason(&policy, &request(), &context()),
            DenyReason::PolicyAuthenticationUnknown
        );
    }

    #[test]
    fn revoked_or_unknown_policy_never_allows() {
        let mut policy = baseline_policy();
        policy.revocation = RevocationState::Revoked;
        assert_eq!(
            denied_reason(&policy, &request(), &context()),
            DenyReason::PolicyRevoked
        );
        policy.revocation = RevocationState::Unknown;
        assert_eq!(
            denied_reason(&policy, &request(), &context()),
            DenyReason::PolicyRevocationUnknown
        );
    }

    #[test]
    fn policy_validity_window_is_half_open() {
        let policy = baseline_policy();
        let mut ctx = context();
        ctx.now_ms = Some(999);
        assert_eq!(
            denied_reason(&policy, &request(), &ctx),
            DenyReason::PolicyNotYetValid
        );
        ctx.now_ms = Some(10_000);
        assert_eq!(
            denied_reason(&policy, &request(), &ctx),
            DenyReason::PolicyExpired
        );
    }

    #[test]
    fn malformed_policy_or_duplicate_capability_ids_fail_closed() {
        let mut policy = baseline_policy();
        policy.epoch = 0;
        assert_eq!(
            denied_reason(&policy, &request(), &context()),
            DenyReason::MalformedPolicy
        );
        policy = baseline_policy();
        policy.capabilities.push(policy.capabilities[0].clone());
        assert_eq!(
            denied_reason(&policy, &request(), &context()),
            DenyReason::MalformedPolicy
        );
    }

    #[test]
    fn missing_unverified_or_contradictory_observation_fails_closed() {
        let policy = baseline_policy();
        let mut req = request();
        req.observed = Fact::missing();
        assert_eq!(
            denied_reason(&policy, &req, &context()),
            DenyReason::MissingObservedOperation
        );
        req = request();
        req.observed = Fact::unverified(req.observed.value.clone().expect("test observation exists"));
        assert_eq!(
            denied_reason(&policy, &req, &context()),
            DenyReason::UnverifiedObservedOperation
        );
        req = request();
        req.observed = Fact::contradictory(req.observed.value.clone().expect("test observation exists"));
        assert_eq!(
            denied_reason(&policy, &req, &context()),
            DenyReason::ContradictoryObservedOperation
        );
    }

    #[test]
    fn wrong_target_or_subject_never_matches_capability() {
        let policy = baseline_policy();
        let mut req = request();
        set_target(&mut req, "endpoint-b");
        assert_eq!(
            denied_reason(&policy, &req, &context()),
            DenyReason::TargetMismatch
        );
        req = request();
        set_subject(&mut req, digest('e'));
        assert_eq!(
            denied_reason(&policy, &req, &context()),
            DenyReason::NoMatchingCapability
        );
    }

    #[test]
    fn different_resource_identity_never_matches_capability() {
        let policy = baseline_policy();
        let mut req = request();
        set_resource(&mut req, resource(ResourceKind::File, 'd'));
        assert_eq!(
            denied_reason(&policy, &req, &context()),
            DenyReason::NoMatchingCapability
        );
    }

    #[test]
    fn read_write_open_needs_one_capability_for_both_operations() {
        let mut policy = baseline_policy();
        let mut write_only_capability = policy.capabilities[0].clone();
        write_only_capability.id = "write-canary".into();
        write_only_capability.operations = vec![Operation::OpenWrite];
        policy.capabilities.push(write_only_capability);
        let mut req = request();
        set_call(&mut req, SyscallCall {
            syscall: Syscall::OpenAt,
            arguments: ArgumentFacts::Open(OpenArguments {
                access: OpenAccessMode::ReadWrite,
                create: false,
                truncate: false,
                append: false,
                unsupported_flags: false,
            }),
        });
        assert_eq!(
            denied_reason(&policy, &req, &context()),
            DenyReason::NoMatchingCapability
        );
    }

    #[test]
    fn read_write_open_is_allowed_only_if_one_grant_contains_both() {
        let mut policy = baseline_policy();
        policy.capabilities[0].operations = vec![Operation::OpenRead, Operation::OpenWrite];
        let mut req = request();
        set_call(&mut req, SyscallCall {
            syscall: Syscall::OpenAt,
            arguments: ArgumentFacts::Open(OpenArguments {
                access: OpenAccessMode::ReadWrite,
                create: false,
                truncate: false,
                append: false,
                unsupported_flags: false,
            }),
        });
        assert!(matches!(
            evaluate(&policy, &req, &context()),
            Decision::Allow(_)
        ));
    }

    #[test]
    fn create_and_truncate_require_additional_scoped_rights() {
        let mut policy = baseline_policy();
        policy.capabilities[0].resource = resource(ResourceKind::DirectoryEntry, 'c');
        policy.capabilities[0].operations = vec![
            Operation::OpenWrite,
            Operation::Create,
            Operation::Truncate,
        ];
        let mut req = request();
        set_resource(&mut req, resource(ResourceKind::DirectoryEntry, 'c'));
        set_call(&mut req, SyscallCall {
            syscall: Syscall::OpenAt,
            arguments: ArgumentFacts::Open(OpenArguments {
                access: OpenAccessMode::WriteOnly,
                create: true,
                truncate: true,
                append: false,
                unsupported_flags: false,
            }),
        });
        assert!(matches!(
            evaluate(&policy, &req, &context()),
            Decision::Allow(_)
        ));

        policy.capabilities[0].operations = vec![Operation::OpenWrite, Operation::Create];
        assert_eq!(
            denied_reason(&policy, &req, &context()),
            DenyReason::NoMatchingCapability
        );
    }

    #[test]
    fn unknown_open_flags_and_inconsistent_flags_fail_closed() {
        let policy = baseline_policy();
        let mut req = request();
        set_call(&mut req, SyscallCall {
            syscall: Syscall::OpenAt,
            arguments: ArgumentFacts::Open(OpenArguments {
                access: OpenAccessMode::ReadOnly,
                create: false,
                truncate: false,
                append: false,
                unsupported_flags: true,
            }),
        });
        assert_eq!(
            denied_reason(&policy, &req, &context()),
            DenyReason::UnsupportedArgumentSemantics
        );
        set_call(&mut req, SyscallCall {
            syscall: Syscall::OpenAt,
            arguments: ArgumentFacts::Open(OpenArguments {
                access: OpenAccessMode::ReadOnly,
                create: false,
                truncate: true,
                append: false,
                unsupported_flags: false,
            }),
        });
        assert_eq!(
            denied_reason(&policy, &req, &context()),
            DenyReason::InconsistentArgumentFacts
        );
    }

    #[test]
    fn unsupported_unknown_syscalls_are_never_granted() {
        let policy = baseline_policy();
        let mut req = request();
        set_call(&mut req, SyscallCall {
            syscall: Syscall::Unknown(0xffff),
            arguments: ArgumentFacts::None,
        });
        assert_eq!(
            denied_reason(&policy, &req, &context()),
            DenyReason::UnknownSyscall
        );
    }

    #[test]
    fn operations_requiring_two_resources_are_denied_until_modeled() {
        let policy = baseline_policy();
        let mut req = request();
        set_call(&mut req, SyscallCall {
            syscall: Syscall::RenameAt2,
            arguments: ArgumentFacts::None,
        });
        assert_eq!(
            denied_reason(&policy, &req, &context()),
            DenyReason::UnsupportedMultiResourceOperation
        );
        set_call(&mut req, SyscallCall {
            syscall: Syscall::Clone3,
            arguments: ArgumentFacts::None,
        });
        assert_eq!(
            denied_reason(&policy, &req, &context()),
            DenyReason::UnsupportedMultiResourceOperation
        );
    }

    #[test]
    fn write_execute_memory_is_always_denied() {
        let mut policy = baseline_policy();
        policy.capabilities[0].resource = resource(ResourceKind::MemoryRegion, 'c');
        policy.capabilities[0].operations = vec![
            Operation::MemoryMap,
            Operation::MemoryWrite,
            Operation::MemoryExecute,
        ];
        let mut req = request();
        set_resource(&mut req, resource(ResourceKind::MemoryRegion, 'c'));
        set_call(&mut req, SyscallCall {
            syscall: Syscall::Mmap,
            arguments: ArgumentFacts::Memory(MemoryArguments {
                read: true,
                write: true,
                execute: true,
                unsupported_protection_bits: false,
            }),
        });
        assert_eq!(
            denied_reason(&policy, &req, &context()),
            DenyReason::WritableExecutableMemory
        );
    }

    #[test]
    fn unknown_memory_protection_bits_fail_closed() {
        let policy = baseline_policy();
        let mut req = request();
        set_call(&mut req, SyscallCall {
            syscall: Syscall::Mprotect,
            arguments: ArgumentFacts::Memory(MemoryArguments {
                read: true,
                write: false,
                execute: false,
                unsupported_protection_bits: true,
            }),
        });
        assert_eq!(
            denied_reason(&policy, &req, &context()),
            DenyReason::UnsupportedArgumentSemantics
        );
    }

    #[test]
    fn resource_kind_must_match_operation_family() {
        let policy = baseline_policy();
        let mut req = request();
        set_resource(&mut req, resource(ResourceKind::NetworkEndpoint, 'c'));
        assert_eq!(
            denied_reason(&policy, &req, &context()),
            DenyReason::InvalidResourceKind
        );
    }

    #[test]
    fn unknown_resource_kind_never_gets_generic_read_write_access() {
        let mut req = request();
        set_resource(&mut req, resource(ResourceKind::Other, 'c'));
        set_call(
            &mut req,
            SyscallCall {
                syscall: Syscall::Read,
                arguments: ArgumentFacts::None,
            },
        );
        assert_eq!(
            denied_reason(&baseline_policy(), &req, &context()),
            DenyReason::InvalidResourceKind
        );
    }

    #[test]
    fn revoked_or_unknown_capability_never_allows() {
        let mut policy = baseline_policy();
        policy.capabilities[0].state = CapabilityState::Revoked;
        assert_eq!(
            denied_reason(&policy, &request(), &context()),
            DenyReason::CapabilityRevoked
        );
        policy.capabilities[0].state = CapabilityState::Unknown;
        assert_eq!(
            denied_reason(&policy, &request(), &context()),
            DenyReason::CapabilityRevocationUnknown
        );
    }

    #[test]
    fn capability_validity_window_is_half_open() {
        let mut policy = baseline_policy();
        policy.capabilities[0].valid_from_ms = 3_000;
        assert_eq!(
            denied_reason(&policy, &request(), &context()),
            DenyReason::CapabilityNotYetValid
        );
        policy.capabilities[0].valid_from_ms = 1_000;
        policy.capabilities[0].expires_at_ms = 2_000;
        assert_eq!(
            denied_reason(&policy, &request(), &context()),
            DenyReason::CapabilityExpired
        );
    }

    #[test]
    fn malformed_capability_window_or_wrong_policy_target_invalidates_policy() {
        let mut policy = baseline_policy();
        policy.capabilities[0].target_id = "endpoint-b".into();
        assert_eq!(
            denied_reason(&policy, &request(), &context()),
            DenyReason::MalformedPolicy
        );
        policy = baseline_policy();
        policy.capabilities[0].expires_at_ms = 11_000;
        assert_eq!(
            denied_reason(&policy, &request(), &context()),
            DenyReason::MalformedPolicy
        );
    }

    #[test]
    fn verified_status_without_an_observation_is_still_missing_evidence() {
        let policy = baseline_policy();
        let mut req = request();
        req.observed = Fact {
            status: EvidenceStatus::Verified,
            value: None,
        };
        assert_eq!(
            denied_reason(&policy, &req, &context()),
            DenyReason::MissingObservedOperation
        );
    }

    #[test]
    fn malformed_digest_is_rejected_at_construction() {
        assert!(Digest::parse("ABC").is_err());
        assert!(Digest::parse("g".repeat(64)).is_err());
        assert!(Digest::parse("A".repeat(64)).is_err());
        assert!(Digest::parse("a".repeat(63)).is_err());
    }
}
