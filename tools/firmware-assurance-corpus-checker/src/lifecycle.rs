//! Stage-transition policy for a synthetic Secure Boot assurance profile.
//!
//! This is an offline policy model. It does not inspect a real device, execute
//! sbctl, generate firmware changes, verify signatures, or qualify hardware.

use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservedState {
    Enabled,
    Disabled,
    SetupMode,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    SecureBoot,
    StandardBoot,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Prepare,
    Enroll,
    VerifyPostReboot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Blocked,
    Indeterminate,
    StandardBootAllowedUnsecured,
    ConfigurationPreparationAllowed,
    ManualEnrollmentReviewAllowed,
    PendingPostRebootVerification,
    SecureBootVerified,
}

impl Decision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Blocked => "blocked",
            Self::Indeterminate => "indeterminate",
            Self::StandardBootAllowedUnsecured => "standard_boot_allowed_unsecured",
            Self::ConfigurationPreparationAllowed => "configuration_preparation_allowed",
            Self::ManualEnrollmentReviewAllowed => "manual_enrollment_review_allowed",
            Self::PendingPostRebootVerification => "pending_post_reboot_verification",
            Self::SecureBootVerified => "secure_boot_verified",
        }
    }
}

/// Claim inputs for evaluating one synthetic lifecycle transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleInput {
    pub observed_state: ObservedState,
    pub method: Method,
    pub stage: Stage,
    pub profile_id_present: bool,
    pub version_pins_present: bool,
    pub user_confirmed: bool,
    pub exact_profile_qualified: bool,
    pub recovery_path_verified: bool,
    pub hardware_profile_matches: bool,
    pub state_matches_approval: bool,
    pub pki_bundle_matches: bool,
    pub key_generation_enabled: bool,
    pub configuration_prepared: bool,
    pub signing_key_verified: bool,
    pub boot_artifacts_verified: bool,
    pub operator_reviewed_command: bool,
    pub enrollment_attempted: bool,
    pub secure_boot_user_mode: bool,
    pub expected_boot_target_selected: bool,
    pub post_boot_health_observed: bool,
    pub recovery_retested: bool,
}

impl LifecycleInput {
    /// Read the language-neutral fixture case without silently defaulting missing fields.
    pub fn from_json(value: &Value) -> Result<Self, String> {
        let get_string = |key: &str| {
            value
                .get(key)
                .and_then(Value::as_str)
                .ok_or_else(|| format!("{key} must be a string"))
        };
        let get_bool = |key: &str| {
            value
                .get(key)
                .and_then(Value::as_bool)
                .ok_or_else(|| format!("{key} must be a boolean"))
        };

        let observed_state = match get_string("observed_state")? {
            "enabled" => ObservedState::Enabled,
            "disabled" => ObservedState::Disabled,
            "setup_mode" => ObservedState::SetupMode,
            "unknown" => ObservedState::Unknown,
            other => return Err(format!("unsupported observed_state {other}")),
        };
        let method = match get_string("method")? {
            "secure_boot" => Method::SecureBoot,
            "standard_boot" => Method::StandardBoot,
            "unresolved" => Method::Unresolved,
            other => return Err(format!("unsupported method {other}")),
        };
        let stage = match get_string("stage")? {
            "prepare" => Stage::Prepare,
            "enroll" => Stage::Enroll,
            "verify_post_reboot" => Stage::VerifyPostReboot,
            other => return Err(format!("unsupported stage {other}")),
        };

        Ok(Self {
            observed_state,
            method,
            stage,
            profile_id_present: get_bool("profile_id_present")?,
            version_pins_present: get_bool("version_pins_present")?,
            user_confirmed: get_bool("user_confirmed")?,
            exact_profile_qualified: get_bool("exact_profile_qualified")?,
            recovery_path_verified: get_bool("recovery_path_verified")?,
            hardware_profile_matches: get_bool("hardware_profile_matches")?,
            state_matches_approval: get_bool("state_matches_approval")?,
            pki_bundle_matches: get_bool("pki_bundle_matches")?,
            key_generation_enabled: get_bool("key_generation_enabled")?,
            configuration_prepared: get_bool("configuration_prepared")?,
            signing_key_verified: get_bool("signing_key_verified")?,
            boot_artifacts_verified: get_bool("boot_artifacts_verified")?,
            operator_reviewed_command: get_bool("operator_reviewed_command")?,
            enrollment_attempted: get_bool("enrollment_attempted")?,
            secure_boot_user_mode: get_bool("secure_boot_user_mode")?,
            expected_boot_target_selected: get_bool("expected_boot_target_selected")?,
            post_boot_health_observed: get_bool("post_boot_health_observed")?,
            recovery_retested: get_bool("recovery_retested")?,
        })
    }
}

/// Evaluate permission to advance one transition; never performs the transition.
///
/// Unknown evidence dominates all other inputs. Preparing config and allowing a
/// human to review an enrollment command are not equivalent to enrollment success.
pub fn evaluate(input: &LifecycleInput) -> Decision {
    if input.observed_state == ObservedState::Unknown {
        return Decision::Indeterminate;
    }

    if input.method == Method::Unresolved || !input.user_confirmed {
        return Decision::Blocked;
    }
    if !input.profile_id_present || !input.version_pins_present {
        return Decision::Blocked;
    }

    if input.method == Method::StandardBoot {
        if input.stage != Stage::Prepare {
            return Decision::Blocked;
        }
        return if input.observed_state == ObservedState::Disabled {
            Decision::StandardBootAllowedUnsecured
        } else {
            Decision::Blocked
        };
    }

    if !input.exact_profile_qualified
        || !input.recovery_path_verified
        || !input.hardware_profile_matches
        || !input.state_matches_approval
        || !input.pki_bundle_matches
    {
        return Decision::Blocked;
    }

    match input.stage {
        Stage::Prepare => {
            if input.key_generation_enabled {
                Decision::ConfigurationPreparationAllowed
            } else {
                Decision::Blocked
            }
        }
        Stage::Enroll => {
            if input.observed_state == ObservedState::SetupMode
                && input.configuration_prepared
                && input.signing_key_verified
                && input.boot_artifacts_verified
                && input.operator_reviewed_command
            {
                Decision::ManualEnrollmentReviewAllowed
            } else {
                Decision::Blocked
            }
        }
        Stage::VerifyPostReboot => {
            if input.configuration_prepared
                && input.enrollment_attempted
                && input.observed_state == ObservedState::Enabled
                && input.secure_boot_user_mode
                && input.expected_boot_target_selected
                && input.post_boot_health_observed
                && input.recovery_retested
            {
                Decision::SecureBootVerified
            } else {
                Decision::PendingPostRebootVerification
            }
        }
    }
}
