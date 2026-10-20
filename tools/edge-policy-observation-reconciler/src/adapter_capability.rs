//! Explicit device-adapter capability qualification.
//!
//! The gate separates device-native transaction behavior, rollback behavior,
//! locking/fencing, effective-policy readback, and independent probes. It never
//! infers guarantees from a successful controller RPC.

#![forbid(unsafe_code)]

/// Transaction semantics documented by the target adapter, not guessed by the
/// rollout coordinator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommitSemantics {
    /// NETCONF confirmed commit as described in RFC 6241 Section 8.4.
    NetconfConfirmedCommit {
        confirm_timeout_secs: u32,
        cancel_commit_supported: bool,
        candidate_lock_supported: bool,
        reverts_on_session_loss: bool,
    },
    /// gNMI transaction scope is one Set RPC request, not a multi-device fleet.
    GnmiSingleSet { atomic_within_single_request: bool },
    /// Adapter-specific behavior has not yet been qualified against a contract.
    Unclassified,
}

/// Declared rollback mechanism. Eligibility checks cross-check this against the
/// transaction semantics; conflicting claims are not accepted.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RollbackSemantics {
    /// Device-native confirmed commit automatically reverts unless confirmed.
    NativeConfirmedCommitTimeout,
    /// Explicit rollback to a previously captured baseline configuration.
    Explicit {
        rollback_is_idempotent: bool,
        baseline_policy_recoverable: bool,
    },
    /// Requires operator intervention; not eligible for automatic rollout.
    ManualOnly,
    /// No usable rollback guarantee.
    None,
}

/// Capabilities asserted by one versioned adapter implementation.
///
/// Capability data must be derived from tested adapter behavior and target
/// documentation/negotiation. This value type cannot authenticate those claims.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceAdapterCapabilities {
    pub target_token: String,
    pub adapter_id: String,
    pub adapter_version: String,
    pub commit_semantics: CommitSemantics,
    pub rollback_semantics: RollbackSemantics,
    /// Readback represents the effective installed policy, not just desired
    /// or candidate configuration.
    pub supports_effective_policy_readback: bool,
    pub readback_digest_is_effective_policy: bool,
    pub supports_independent_dataplane_probe: bool,
    pub supports_positive_control_probe: bool,
    /// Native target-side compare-and-swap/generation fencing.
    pub supports_device_side_generation_fence: bool,
    /// An exclusive configuration lock held for the change lifecycle.
    pub supports_exclusive_config_lock: bool,
}

/// Policy requirements imposed by the deployment, not the device adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdapterQualificationPolicy {
    /// Minimum native confirmed-commit timeout needed to observe and recover.
    pub minimum_confirmed_commit_timeout_secs: u32,
    /// Require target-side fencing, rather than accepting a config lock alone.
    pub require_device_side_generation_fence: bool,
    /// Allow explicitly scripted idempotent rollback for non-confirmed-commit
    /// devices. This does not make external rollback atomic.
    pub allow_explicit_rollback: bool,
}

/// Rollback class that the coordinator is allowed to claim after qualification.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QualifiedRollbackMode {
    NativeConfirmedCommit,
    ExplicitIdempotentRollback,
    NotQualified,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdapterQualification {
    pub target_token: String,
    pub adapter_id: String,
    pub adapter_version: String,
    pub automatic_rollout_eligible: bool,
    pub qualified_rollback_mode: QualifiedRollbackMode,
    pub blockers: Vec<String>,
    /// Explicit scope statements to keep downstream reporting honest.
    pub claim_limits: Vec<String>,
}

/// Evaluate capabilities against the deployment's declared safety policy.
///
/// A non-empty blocker list means the adapter must not be used for automatic
/// rollout under this policy. The function does not invoke a device or verify
/// the truth of its capability assertions.
pub fn qualify_adapter(
    capabilities: &DeviceAdapterCapabilities,
    policy: &AdapterQualificationPolicy,
) -> Result<AdapterQualification, Vec<String>> {
    let mut input_errors = Vec::new();
    if !is_target_token(&capabilities.target_token) {
        input_errors.push("target_token is invalid".to_owned());
    }
    if !is_identifier(&capabilities.adapter_id) {
        input_errors
            .push("adapter_id must be an ASCII identifier of at most 80 characters".to_owned());
    }
    if !is_identifier(&capabilities.adapter_version) {
        input_errors.push(
            "adapter_version must be an ASCII identifier of at most 80 characters".to_owned(),
        );
    }
    if policy.minimum_confirmed_commit_timeout_secs == 0 {
        input_errors.push("minimum_confirmed_commit_timeout_secs must be positive".to_owned());
    }
    if !input_errors.is_empty() {
        return Err(input_errors);
    }

    let mut blockers = Vec::new();
    if !capabilities.supports_effective_policy_readback
        || !capabilities.readback_digest_is_effective_policy
    {
        blockers.push(
            "effective installed policy cannot be independently read back and digest-bound"
                .to_owned(),
        );
    }
    if !capabilities.supports_independent_dataplane_probe {
        blockers.push("independent data-plane probing is unavailable".to_owned());
    }
    if !capabilities.supports_positive_control_probe {
        blockers.push(
            "independent positive-control probing is unavailable; deny-all false passes cannot be excluded"
                .to_owned(),
        );
    }
    if policy.require_device_side_generation_fence
        && !capabilities.supports_device_side_generation_fence
    {
        blockers
            .push("policy requires device-side generation fencing but adapter lacks it".to_owned());
    } else if !policy.require_device_side_generation_fence
        && !capabilities.supports_device_side_generation_fence
        && !capabilities.supports_exclusive_config_lock
    {
        blockers.push(
            "neither device-side generation fencing nor an exclusive configuration lock is available"
                .to_owned(),
        );
    }

    let qualified_rollback_mode = match (
        &capabilities.commit_semantics,
        &capabilities.rollback_semantics,
    ) {
        (
            CommitSemantics::NetconfConfirmedCommit {
                confirm_timeout_secs,
                cancel_commit_supported,
                candidate_lock_supported,
                reverts_on_session_loss,
            },
            RollbackSemantics::NativeConfirmedCommitTimeout,
        ) => {
            if *confirm_timeout_secs < policy.minimum_confirmed_commit_timeout_secs {
                blockers.push(format!(
                    "native confirmed-commit timeout {confirm_timeout_secs}s is shorter than the required {}s recovery window",
                    policy.minimum_confirmed_commit_timeout_secs
                ));
            }
            if !cancel_commit_supported {
                blockers.push(
                    "NETCONF confirmed commit lacks the required cancel-commit capability"
                        .to_owned(),
                );
            }
            if !candidate_lock_supported {
                blockers.push(
                    "NETCONF candidate configuration cannot be exclusively locked".to_owned(),
                );
            }
            if !reverts_on_session_loss {
                blockers.push(
                    "NETCONF adapter does not guarantee revert after confirmed-commit session loss"
                        .to_owned(),
                );
            }
            QualifiedRollbackMode::NativeConfirmedCommit
        }
        (
            CommitSemantics::GnmiSingleSet {
                atomic_within_single_request,
            },
            RollbackSemantics::Explicit {
                rollback_is_idempotent,
                baseline_policy_recoverable,
            },
        ) => {
            if !atomic_within_single_request {
                blockers.push(
                    "gNMI adapter cannot establish atomicity for operations within one Set request"
                        .to_owned(),
                );
            }
            if !rollback_is_idempotent {
                blockers.push("explicit rollback is not declared idempotent".to_owned());
            }
            if !baseline_policy_recoverable {
                blockers.push("exact approved baseline is not recoverable for rollback".to_owned());
            }
            if !policy.allow_explicit_rollback {
                blockers.push(
                    "deployment policy requires native confirmed commit; explicit rollback is disallowed"
                        .to_owned(),
                );
            }
            QualifiedRollbackMode::ExplicitIdempotentRollback
        }
        (CommitSemantics::NetconfConfirmedCommit { .. }, _) => {
            blockers.push(
                "NETCONF confirmed-commit semantics conflict with the declared rollback mechanism"
                    .to_owned(),
            );
            QualifiedRollbackMode::NotQualified
        }
        (
            CommitSemantics::GnmiSingleSet { .. },
            RollbackSemantics::NativeConfirmedCommitTimeout,
        ) => {
            blockers.push(
                "gNMI Set transactionality does not itself provide NETCONF-style confirmed-commit rollback"
                    .to_owned(),
            );
            QualifiedRollbackMode::NotQualified
        }
        (
            CommitSemantics::GnmiSingleSet { .. },
            RollbackSemantics::ManualOnly | RollbackSemantics::None,
        ) => {
            blockers.push(
                "gNMI single-request transaction is insufficient without a qualified rollback path"
                    .to_owned(),
            );
            QualifiedRollbackMode::NotQualified
        }
        (_, RollbackSemantics::ManualOnly) => {
            blockers.push(
                "adapter requires operator-only recovery and is not eligible for automatic rollout"
                    .to_owned(),
            );
            QualifiedRollbackMode::NotQualified
        }
        (_, RollbackSemantics::None) => {
            blockers.push("adapter has no qualified rollback mechanism".to_owned());
            QualifiedRollbackMode::NotQualified
        }
        (CommitSemantics::Unclassified, _) => {
            blockers.push(
                "adapter transaction semantics are unclassified; automatic rollout is refused"
                    .to_owned(),
            );
            QualifiedRollbackMode::NotQualified
        }
    };

    let claim_limits = vec![
        "capabilities are asserted by the adapter and are not authenticated by this assessment"
            .to_owned(),
        "single-device transaction semantics do not imply atomicity across devices".to_owned(),
        "device commit acknowledgement is not independent data-plane evidence".to_owned(),
        "qualified rollback mode does not prove production hardware or firmware behavior without adapter-specific tests"
            .to_owned(),
    ];
    Ok(AdapterQualification {
        target_token: capabilities.target_token.clone(),
        adapter_id: capabilities.adapter_id.clone(),
        adapter_version: capabilities.adapter_version.clone(),
        automatic_rollout_eligible: blockers.is_empty(),
        qualified_rollback_mode: if blockers.is_empty() {
            qualified_rollback_mode
        } else if qualified_rollback_mode == QualifiedRollbackMode::NotQualified {
            QualifiedRollbackMode::NotQualified
        } else {
            // Do not let callers overstate what this particular adapter can
            // safely be used for when other required safety controls are absent.
            QualifiedRollbackMode::NotQualified
        },
        blockers,
        claim_limits,
    })
}

fn is_target_token(value: &str) -> bool {
    value.starts_with("target-")
        && (8..=55).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn is_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

#[cfg(test)]
mod tests {
    use super::{
        AdapterQualificationPolicy, CommitSemantics, DeviceAdapterCapabilities,
        QualifiedRollbackMode, RollbackSemantics, qualify_adapter,
    };

    fn policy() -> AdapterQualificationPolicy {
        AdapterQualificationPolicy {
            minimum_confirmed_commit_timeout_secs: 120,
            require_device_side_generation_fence: false,
            allow_explicit_rollback: true,
        }
    }

    fn netconf() -> DeviceAdapterCapabilities {
        DeviceAdapterCapabilities {
            target_token: "target-router-a".to_owned(),
            adapter_id: "adapter-netconf".to_owned(),
            adapter_version: "v1".to_owned(),
            commit_semantics: CommitSemantics::NetconfConfirmedCommit {
                confirm_timeout_secs: 180,
                cancel_commit_supported: true,
                candidate_lock_supported: true,
                reverts_on_session_loss: true,
            },
            rollback_semantics: RollbackSemantics::NativeConfirmedCommitTimeout,
            supports_effective_policy_readback: true,
            readback_digest_is_effective_policy: true,
            supports_independent_dataplane_probe: true,
            supports_positive_control_probe: true,
            supports_device_side_generation_fence: true,
            supports_exclusive_config_lock: true,
        }
    }

    fn gnmi() -> DeviceAdapterCapabilities {
        DeviceAdapterCapabilities {
            target_token: "target-router-b".to_owned(),
            adapter_id: "adapter-gnmi".to_owned(),
            adapter_version: "v1".to_owned(),
            commit_semantics: CommitSemantics::GnmiSingleSet {
                atomic_within_single_request: true,
            },
            rollback_semantics: RollbackSemantics::Explicit {
                rollback_is_idempotent: true,
                baseline_policy_recoverable: true,
            },
            supports_effective_policy_readback: true,
            readback_digest_is_effective_policy: true,
            supports_independent_dataplane_probe: true,
            supports_positive_control_probe: true,
            supports_device_side_generation_fence: false,
            supports_exclusive_config_lock: true,
        }
    }

    #[test]
    fn qualified_netconf_requires_native_timeout_and_rollback_contract() {
        let result = qualify_adapter(&netconf(), &policy()).expect("valid capabilities");
        assert!(result.automatic_rollout_eligible);
        assert_eq!(
            result.qualified_rollback_mode,
            QualifiedRollbackMode::NativeConfirmedCommit
        );
        assert!(result.blockers.is_empty());
    }

    #[test]
    fn too_short_confirmed_commit_window_blocks_automatic_rollout() {
        let mut capabilities = netconf();
        capabilities.commit_semantics = CommitSemantics::NetconfConfirmedCommit {
            confirm_timeout_secs: 30,
            cancel_commit_supported: true,
            candidate_lock_supported: true,
            reverts_on_session_loss: true,
        };
        let result = qualify_adapter(&capabilities, &policy()).expect("valid capabilities");
        assert!(!result.automatic_rollout_eligible);
        assert!(
            result
                .blockers
                .iter()
                .any(|reason| reason.contains("shorter"))
        );
    }

    #[test]
    fn missing_netconf_lock_or_session_revert_contract_is_rejected() {
        let mut capabilities = netconf();
        capabilities.commit_semantics = CommitSemantics::NetconfConfirmedCommit {
            confirm_timeout_secs: 180,
            cancel_commit_supported: true,
            candidate_lock_supported: false,
            reverts_on_session_loss: false,
        };
        let result = qualify_adapter(&capabilities, &policy()).expect("valid capabilities");
        assert!(!result.automatic_rollout_eligible);
        assert!(
            result
                .blockers
                .iter()
                .any(|reason| reason.contains("exclusively locked"))
        );
        assert!(
            result
                .blockers
                .iter()
                .any(|reason| reason.contains("session loss"))
        );
    }

    #[test]
    fn gnmi_single_set_is_eligible_only_with_explicit_idempotent_baseline_rollback() {
        let result = qualify_adapter(&gnmi(), &policy()).expect("valid capabilities");
        assert!(result.automatic_rollout_eligible);
        assert_eq!(
            result.qualified_rollback_mode,
            QualifiedRollbackMode::ExplicitIdempotentRollback
        );
    }

    #[test]
    fn gnmi_transaction_without_rollback_is_not_automatic_rollout_safe() {
        let mut capabilities = gnmi();
        capabilities.rollback_semantics = RollbackSemantics::None;
        let result = qualify_adapter(&capabilities, &policy()).expect("valid capabilities");
        assert!(!result.automatic_rollout_eligible);
        assert_eq!(
            result.qualified_rollback_mode,
            QualifiedRollbackMode::NotQualified
        );
    }

    #[test]
    fn deployment_can_require_device_side_generation_fencing() {
        let mut required = policy();
        required.require_device_side_generation_fence = true;
        let result = qualify_adapter(&gnmi(), &required).expect("valid capabilities");
        assert!(!result.automatic_rollout_eligible);
        assert!(
            result
                .blockers
                .iter()
                .any(|reason| reason.contains("generation fencing"))
        );
    }

    #[test]
    fn missing_effective_policy_readback_or_positive_control_blocks() {
        let mut capabilities = netconf();
        capabilities.readback_digest_is_effective_policy = false;
        capabilities.supports_positive_control_probe = false;
        let result = qualify_adapter(&capabilities, &policy()).expect("valid capabilities");
        assert!(!result.automatic_rollout_eligible);
        assert!(
            result
                .blockers
                .iter()
                .any(|reason| reason.contains("effective installed policy"))
        );
        assert!(
            result
                .blockers
                .iter()
                .any(|reason| reason.contains("positive-control"))
        );
    }

    #[test]
    fn manual_only_and_unclassified_adapters_are_never_eligible() {
        let mut capabilities = gnmi();
        capabilities.rollback_semantics = RollbackSemantics::ManualOnly;
        let result = qualify_adapter(&capabilities, &policy()).expect("valid capabilities");
        assert!(!result.automatic_rollout_eligible);
        assert_eq!(
            result.qualified_rollback_mode,
            QualifiedRollbackMode::NotQualified
        );

        let mut capabilities = gnmi();
        capabilities.commit_semantics = CommitSemantics::Unclassified;
        let result = qualify_adapter(&capabilities, &policy()).expect("valid capabilities");
        assert!(!result.automatic_rollout_eligible);
        assert_eq!(
            result.qualified_rollback_mode,
            QualifiedRollbackMode::NotQualified
        );
    }

    #[test]
    fn deployment_can_disallow_explicit_rollback_for_gnmi_adapters() {
        let mut strict = policy();
        strict.allow_explicit_rollback = false;
        let result = qualify_adapter(&gnmi(), &strict).expect("valid capabilities");
        assert!(!result.automatic_rollout_eligible);
        assert!(
            result
                .blockers
                .iter()
                .any(|reason| reason.contains("explicit rollback is disallowed"))
        );
    }

    #[test]
    fn missing_lock_and_fencing_blocks_target_without_concurrency_protection() {
        let mut capabilities = gnmi();
        capabilities.supports_exclusive_config_lock = false;
        let result = qualify_adapter(&capabilities, &policy()).expect("valid capabilities");
        assert!(!result.automatic_rollout_eligible);
        assert!(
            result
                .blockers
                .iter()
                .any(|reason| reason.contains("neither device-side"))
        );
    }

    #[test]
    fn invalid_profile_identifiers_are_rejected_as_input_errors() {
        let mut capabilities = netconf();
        capabilities.target_token = "router-a".to_owned();
        assert!(
            qualify_adapter(&capabilities, &policy())
                .expect_err("invalid target identity")
                .iter()
                .any(|reason| reason.contains("target_token"))
        );
    }
}
