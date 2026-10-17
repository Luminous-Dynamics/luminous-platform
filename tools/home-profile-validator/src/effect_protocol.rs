//! Executable reference model for the Mycelix Hearth home-effect dispatch boundary.
//!
//! This module is deliberately side-effect free. It is a reviewable decision model,
//! not a production dispatcher, authority verifier, journal, OS sandbox, or device
//! adapter. Boolean facts supplied by a caller stand in for independently verified
//! evidence; this model cannot authenticate them.
//!
//! The intended production path remains Hearth's admitted edge runtime (004C/004D/004E).
//! Keep this model aligned with that runtime through shared adversarial traces.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActionRole {
    Primary,
    Compensation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactBinding {
    pub plan_id: String,
    pub step_id: String,
    pub role: ActionRole,
    pub capability: String,
    pub operation: String,
    pub adapter_id: String,
    pub implementation_ref: String,
    pub profile_ref: String,
    pub rule_id: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RiskClass {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JournalState {
    /// The journal has no evidence that a side effect was started.
    NotStarted,
    /// A dispatch may have crossed the process boundary; its effect is unresolved.
    Dispatching,
    /// The provider accepted/replied, but the real-world result is not yet reconciled.
    ProviderAccepted,
    /// A crash, timeout, or conflicting observation left the effect unknown.
    OutcomeUnknown,
    /// Independent evidence establishes that the previous attempt had no effect.
    DefinitiveNoEffect,
    /// Outcome has been reconciled; a new operation requires a new request/idempotency key.
    ReconciledSuccess,
    ReconciledFailure,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DenialReason {
    EmptyIdempotencyKey,
    EmptyRequestFingerprint,
    InvalidAttemptLimit,
    ExpiredRequest,
    HouseholdAuthorityUnavailableOrRevoked,
    AdapterAdmissionStaleOrUnverified,
    AdmissionBindingMismatch,
    RegistrationBindingMismatch,
    MissingExclusiveExecutorLease,
    MissingDurableJournal,
    MissingPostconditionObserver,
    MissingPhysicalStop,
    AttemptsExhausted,
    IdempotencyKeyMismatch,
    RequestFingerprintMismatch,
    InconsistentJournalState,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DispatchDecision {
    /// A new physical/provider-side attempt may be considered by the real Hearth runtime.
    /// This is not authorization to bypass that runtime or evidence that dispatch occurred.
    AllowDispatch,
    /// An earlier attempt may have happened. Read/observe/reconcile; do not dispatch again.
    ReconcileOnly,
    /// The operation has a reconciled terminal result; start a new request for new intent.
    Terminal,
    /// The presented context is not sufficient to authorize a new attempt.
    Deny(DenialReason),
}

pub struct DispatchGateInput<'a> {
    pub requested: &'a ExactBinding,
    pub admitted: &'a ExactBinding,
    pub registered: &'a ExactBinding,
    pub request_idempotency_key: &'a str,
    /// Digest of the canonical request including plan/step/role, target, risk and argument digest.
    /// The caller computes this from the complete request; this model cannot verify that digest.
    pub request_fingerprint: &'a str,
    pub journal_idempotency_key: Option<&'a str>,
    pub journal_request_fingerprint: Option<&'a str>,
    pub now_unix_seconds: u64,
    pub expires_at_unix_seconds: u64,
    pub max_attempts: u8,
    pub attempts_started: u8,
    pub journal_state: JournalState,
    /// Must mean current authority for this exact requester, plan step, target and operation.
    pub household_authority_current: bool,
    /// Must mean the exact admission has been revalidated for this new attempt.
    pub adapter_admission_current: bool,
    pub exclusive_executor_lease_held: bool,
    pub durable_journal_ready: bool,
    /// Must refer to an outcome observer suitable for this operation's declared risk.
    pub postcondition_observer_available: bool,
    pub risk_class: RiskClass,
    pub independent_physical_stop_ready: bool,
}

/// Evaluate whether a new side-effect attempt is even eligible to be considered.
///
/// Unresolved attempts are handled before new-dispatch checks so stale authority or
/// request expiry cannot turn an ambiguous prior effect into a duplicate dispatch.
/// Reconciliation itself must remain read-only with respect to the physical effect.
pub fn evaluate_dispatch_gate(input: &DispatchGateInput<'_>) -> DispatchDecision {
    if input.request_idempotency_key.trim().is_empty() {
        return DispatchDecision::Deny(DenialReason::EmptyIdempotencyKey);
    }
    // Structural schema validation normally rejects this too, but the gate is a
    // defense-in-depth boundary and must remain safe when called independently.
    // Non-empty is only a minimum check; the caller still owns digest computation
    // and a trusted verifier must bind it to the canonical request bytes.
    if input.request_fingerprint.trim().is_empty() {
        return DispatchDecision::Deny(DenialReason::EmptyRequestFingerprint);
    }

    // One idempotency key identifies one exact request, not merely one action name.
    // Existing journal rows must bind both the key and canonical request fingerprint.
    match (input.journal_idempotency_key, input.journal_request_fingerprint) {
        (None, None) if input.journal_state == JournalState::NotStarted => {}
        (Some(key), Some(fingerprint)) => {
            if key != input.request_idempotency_key {
                return DispatchDecision::Deny(DenialReason::IdempotencyKeyMismatch);
            }
            if fingerprint != input.request_fingerprint {
                return DispatchDecision::Deny(DenialReason::RequestFingerprintMismatch);
            }
        }
        (None, None) => return DispatchDecision::Deny(DenialReason::InconsistentJournalState),
        _ => return DispatchDecision::Deny(DenialReason::InconsistentJournalState),
    }

    match input.journal_state {
        JournalState::Dispatching
        | JournalState::ProviderAccepted
        | JournalState::OutcomeUnknown => return DispatchDecision::ReconcileOnly,
        JournalState::ReconciledSuccess | JournalState::ReconciledFailure => {
            return DispatchDecision::Terminal;
        }
        JournalState::NotStarted | JournalState::DefinitiveNoEffect => {}
    }

    if input.max_attempts == 0 || input.max_attempts > 3 {
        return DispatchDecision::Deny(DenialReason::InvalidAttemptLimit);
    }
    if input.now_unix_seconds >= input.expires_at_unix_seconds {
        return DispatchDecision::Deny(DenialReason::ExpiredRequest);
    }
    if !input.household_authority_current {
        return DispatchDecision::Deny(DenialReason::HouseholdAuthorityUnavailableOrRevoked);
    }
    if !input.adapter_admission_current {
        return DispatchDecision::Deny(DenialReason::AdapterAdmissionStaleOrUnverified);
    }
    if input.requested != input.admitted {
        return DispatchDecision::Deny(DenialReason::AdmissionBindingMismatch);
    }
    if input.admitted != input.registered {
        return DispatchDecision::Deny(DenialReason::RegistrationBindingMismatch);
    }
    if !input.exclusive_executor_lease_held {
        return DispatchDecision::Deny(DenialReason::MissingExclusiveExecutorLease);
    }
    if !input.durable_journal_ready {
        return DispatchDecision::Deny(DenialReason::MissingDurableJournal);
    }
    if !input.postcondition_observer_available {
        return DispatchDecision::Deny(DenialReason::MissingPostconditionObserver);
    }
    if matches!(input.risk_class, RiskClass::High | RiskClass::Critical)
        && !input.independent_physical_stop_ready
    {
        return DispatchDecision::Deny(DenialReason::MissingPhysicalStop);
    }
    if input.attempts_started >= input.max_attempts {
        return DispatchDecision::Deny(DenialReason::AttemptsExhausted);
    }

    match input.journal_state {
        JournalState::NotStarted => {
            if input.attempts_started != 0 {
                return DispatchDecision::Deny(DenialReason::InconsistentJournalState);
            }
            if input
                .journal_idempotency_key
                .is_some_and(|journal_key| journal_key != input.request_idempotency_key)
            {
                return DispatchDecision::Deny(DenialReason::IdempotencyKeyMismatch);
            }
        }
        JournalState::DefinitiveNoEffect => {
            if input.attempts_started == 0 {
                return DispatchDecision::Deny(DenialReason::InconsistentJournalState);
            }
            if input.journal_idempotency_key != Some(input.request_idempotency_key) {
                return DispatchDecision::Deny(DenialReason::IdempotencyKeyMismatch);
            }
        }
        JournalState::Dispatching
        | JournalState::ProviderAccepted
        | JournalState::OutcomeUnknown
        | JournalState::ReconciledSuccess
        | JournalState::ReconciledFailure => unreachable!("handled above"),
    }

    DispatchDecision::AllowDispatch
}

#[cfg(test)]
mod tests {
    use super::*;

    fn binding() -> ExactBinding {
        ExactBinding {
            plan_id: "plan-01".into(),
            step_id: "step-01".into(),
            role: ActionRole::Primary,
            capability: "home.lighting".into(),
            operation: "light.turn_on".into(),
            adapter_id: "ha-adapter-v1".into(),
            implementation_ref: "git:adapter@abc123".into(),
            profile_ref: "profile:ha-light-v1".into(),
            rule_id: "light-on-low-risk".into(),
        }
    }

    fn valid_input<'a>(binding: &'a ExactBinding) -> DispatchGateInput<'a> {
        DispatchGateInput {
            requested: binding,
            admitted: binding,
            registered: binding,
            request_idempotency_key: "request-key-0001",
            request_fingerprint: "request-fingerprint-0001",
            journal_idempotency_key: None,
            journal_request_fingerprint: None,
            now_unix_seconds: 100,
            expires_at_unix_seconds: 200,
            max_attempts: 2,
            attempts_started: 0,
            journal_state: JournalState::NotStarted,
            household_authority_current: true,
            adapter_admission_current: true,
            exclusive_executor_lease_held: true,
            durable_journal_ready: true,
            postcondition_observer_available: true,
            risk_class: RiskClass::Low,
            independent_physical_stop_ready: false,
        }
    }

    #[test]
    fn permits_only_a_fully_bound_fresh_low_risk_attempt() {
        let binding = binding();
        assert_eq!(evaluate_dispatch_gate(&valid_input(&binding)), DispatchDecision::AllowDispatch);
    }

    #[test]
    fn rejects_empty_request_fingerprint_even_without_schema_validation() {
        let binding = binding();
        let mut input = valid_input(&binding);
        input.request_fingerprint = "  ";
        assert_eq!(
            evaluate_dispatch_gate(&input),
            DispatchDecision::Deny(DenialReason::EmptyRequestFingerprint)
        );
    }

    #[test]
    fn rejects_revoked_or_unavailable_household_authority() {
        let binding = binding();
        let mut input = valid_input(&binding);
        input.household_authority_current = false;
        assert_eq!(
            evaluate_dispatch_gate(&input),
            DispatchDecision::Deny(DenialReason::HouseholdAuthorityUnavailableOrRevoked)
        );
    }

    #[test]
    fn rejects_stale_adapter_admission() {
        let binding = binding();
        let mut input = valid_input(&binding);
        input.adapter_admission_current = false;
        assert_eq!(
            evaluate_dispatch_gate(&input),
            DispatchDecision::Deny(DenialReason::AdapterAdmissionStaleOrUnverified)
        );
    }

    #[test]
    fn rejects_implementation_profile_rule_or_role_mismatch() {
        let request = binding();
        let mut changed_implementation = binding();
        changed_implementation.implementation_ref = "git:adapter@changed".into();
        let mut implementation_input = valid_input(&request);
        implementation_input.admitted = &changed_implementation;
        assert_eq!(
            evaluate_dispatch_gate(&implementation_input),
            DispatchDecision::Deny(DenialReason::AdmissionBindingMismatch)
        );

        let unchanged_admission = binding();
        let mut changed_registration = binding();
        changed_registration.profile_ref = "profile:unexpected".into();
        let mut registration_input = valid_input(&request);
        registration_input.admitted = &unchanged_admission;
        registration_input.registered = &changed_registration;
        assert_eq!(
            evaluate_dispatch_gate(&registration_input),
            DispatchDecision::Deny(DenialReason::RegistrationBindingMismatch)
        );

        let mut compensation_admission = binding();
        compensation_admission.role = ActionRole::Compensation;
        let mut role_input = valid_input(&request);
        role_input.admitted = &compensation_admission;
        role_input.registered = &compensation_admission;
        assert_eq!(
            evaluate_dispatch_gate(&role_input),
            DispatchDecision::Deny(DenialReason::AdmissionBindingMismatch)
        );
    }

    #[test]
    fn rejects_missing_exclusive_lease_and_durable_journal() {
        let binding = binding();
        let mut input = valid_input(&binding);
        input.exclusive_executor_lease_held = false;
        assert_eq!(
            evaluate_dispatch_gate(&input),
            DispatchDecision::Deny(DenialReason::MissingExclusiveExecutorLease)
        );
        input.exclusive_executor_lease_held = true;
        input.durable_journal_ready = false;
        assert_eq!(
            evaluate_dispatch_gate(&input),
            DispatchDecision::Deny(DenialReason::MissingDurableJournal)
        );
    }

    #[test]
    fn rejects_attempt_without_a_postcondition_observer() {
        let binding = binding();
        let mut input = valid_input(&binding);
        input.postcondition_observer_available = false;
        assert_eq!(
            evaluate_dispatch_gate(&input),
            DispatchDecision::Deny(DenialReason::MissingPostconditionObserver)
        );
    }

    #[test]
    fn high_and_critical_risk_require_an_independent_physical_stop() {
        let binding = binding();
        let mut input = valid_input(&binding);
        input.risk_class = RiskClass::High;
        assert_eq!(
            evaluate_dispatch_gate(&input),
            DispatchDecision::Deny(DenialReason::MissingPhysicalStop)
        );
        input.risk_class = RiskClass::Critical;
        assert_eq!(
            evaluate_dispatch_gate(&input),
            DispatchDecision::Deny(DenialReason::MissingPhysicalStop)
        );
        input.independent_physical_stop_ready = true;
        assert_eq!(evaluate_dispatch_gate(&input), DispatchDecision::AllowDispatch);
    }

    #[test]
    fn rejects_expired_requests_and_invalid_attempt_limits() {
        let binding = binding();
        let mut input = valid_input(&binding);
        input.now_unix_seconds = input.expires_at_unix_seconds;
        assert_eq!(
            evaluate_dispatch_gate(&input),
            DispatchDecision::Deny(DenialReason::ExpiredRequest)
        );
        input = valid_input(&binding);
        input.max_attempts = 4;
        assert_eq!(
            evaluate_dispatch_gate(&input),
            DispatchDecision::Deny(DenialReason::InvalidAttemptLimit)
        );
    }

    #[test]
    fn definitive_no_effect_allows_only_a_bounded_same_key_retry() {
        let binding = binding();
        let mut input = valid_input(&binding);
        input.journal_state = JournalState::DefinitiveNoEffect;
        input.attempts_started = 1;
        input.journal_idempotency_key = Some(input.request_idempotency_key);
        input.journal_request_fingerprint = Some(input.request_fingerprint);
        assert_eq!(evaluate_dispatch_gate(&input), DispatchDecision::AllowDispatch);

        input.journal_idempotency_key = Some("another-request-key");
        assert_eq!(
            evaluate_dispatch_gate(&input),
            DispatchDecision::Deny(DenialReason::IdempotencyKeyMismatch)
        );

        input.journal_idempotency_key = Some(input.request_idempotency_key);
        input.journal_request_fingerprint = Some("request-fingerprint-for-another-payload");
        assert_eq!(
            evaluate_dispatch_gate(&input),
            DispatchDecision::Deny(DenialReason::RequestFingerprintMismatch)
        );
        input.journal_request_fingerprint = Some(input.request_fingerprint);
        input.attempts_started = input.max_attempts;
        assert_eq!(
            evaluate_dispatch_gate(&input),
            DispatchDecision::Deny(DenialReason::AttemptsExhausted)
        );
    }

    #[test]
    fn ambiguous_or_accepted_attempts_are_reconciliation_only_even_after_expiry() {
        let binding = binding();
        let mut input = valid_input(&binding);
        input.journal_state = JournalState::OutcomeUnknown;
        input.attempts_started = 1;
        input.journal_idempotency_key = Some(input.request_idempotency_key);
        input.journal_request_fingerprint = Some(input.request_fingerprint);
        input.household_authority_current = false;
        input.now_unix_seconds = input.expires_at_unix_seconds;
        assert_eq!(evaluate_dispatch_gate(&input), DispatchDecision::ReconcileOnly);

        input.journal_state = JournalState::ProviderAccepted;
        assert_eq!(evaluate_dispatch_gate(&input), DispatchDecision::ReconcileOnly);
        input.journal_state = JournalState::Dispatching;
        assert_eq!(evaluate_dispatch_gate(&input), DispatchDecision::ReconcileOnly);
    }

    #[test]
    fn reconciled_terminal_outcomes_do_not_reuse_the_same_request() {
        let binding = binding();
        let mut input = valid_input(&binding);
        input.journal_state = JournalState::ReconciledSuccess;
        assert_eq!(evaluate_dispatch_gate(&input), DispatchDecision::Terminal);
        input.journal_state = JournalState::ReconciledFailure;
        assert_eq!(evaluate_dispatch_gate(&input), DispatchDecision::Terminal);
    }

    #[test]
    fn journal_state_attempt_count_and_idempotency_must_agree() {
        let binding = binding();
        let mut input = valid_input(&binding);
        input.attempts_started = 1;
        assert_eq!(
            evaluate_dispatch_gate(&input),
            DispatchDecision::Deny(DenialReason::InconsistentJournalState)
        );
        input = valid_input(&binding);
        input.request_idempotency_key = " ";
        assert_eq!(
            evaluate_dispatch_gate(&input),
            DispatchDecision::Deny(DenialReason::EmptyIdempotencyKey)
        );
    }

    const CONFORMANCE_SCHEMA: &str =
        include_str!("../../../schemas/home-effect-conformance-traces.schema.json");
    const CONFORMANCE_TRACES: &str =
        include_str!("../../../tests/home-effect-conformance/v1/traces.json");

    fn conformance_str<'a>(value: &'a serde_json::Value, key: &str) -> &'a str {
        value.get(key).and_then(serde_json::Value::as_str)
            .unwrap_or_else(|| panic!("conformance field {key} must be a string"))
    }

    fn conformance_bool(value: &serde_json::Value, key: &str) -> bool {
        value.get(key).and_then(serde_json::Value::as_bool)
            .unwrap_or_else(|| panic!("conformance field {key} must be a boolean"))
    }

    fn conformance_u64(value: &serde_json::Value, key: &str) -> u64 {
        value.get(key).and_then(serde_json::Value::as_u64)
            .unwrap_or_else(|| panic!("conformance field {key} must be a non-negative integer"))
    }

    fn conformance_binding(value: &serde_json::Value) -> ExactBinding {
        let role = match conformance_str(value, "role") {
            "primary" => ActionRole::Primary,
            "compensation" => ActionRole::Compensation,
            other => panic!("unknown conformance action role: {other}"),
        };
        ExactBinding {
            plan_id: conformance_str(value, "plan_id").to_owned(),
            step_id: conformance_str(value, "step_id").to_owned(),
            role,
            capability: conformance_str(value, "capability").to_owned(),
            operation: conformance_str(value, "operation").to_owned(),
            adapter_id: conformance_str(value, "adapter_id").to_owned(),
            implementation_ref: conformance_str(value, "implementation_ref").to_owned(),
            profile_ref: conformance_str(value, "profile_ref").to_owned(),
            rule_id: conformance_str(value, "rule_id").to_owned(),
        }
    }

    fn evaluate_conformance_input(value: &serde_json::Value) -> DispatchDecision {
        let requested = conformance_binding(value.get("requested").expect("requested binding"));
        let admitted = conformance_binding(value.get("admitted").expect("admitted binding"));
        let registered = conformance_binding(value.get("registered").expect("registered binding"));
        let journal_state = match conformance_str(value, "journal_state") {
            "not_started" => JournalState::NotStarted,
            "dispatching" => JournalState::Dispatching,
            "provider_accepted" => JournalState::ProviderAccepted,
            "outcome_unknown" => JournalState::OutcomeUnknown,
            "definitive_no_effect" => JournalState::DefinitiveNoEffect,
            "reconciled_success" => JournalState::ReconciledSuccess,
            "reconciled_failure" => JournalState::ReconciledFailure,
            other => panic!("unknown conformance journal state: {other}"),
        };
        let risk_class = match conformance_str(value, "risk_class") {
            "low" => RiskClass::Low,
            "medium" => RiskClass::Medium,
            "high" => RiskClass::High,
            "critical" => RiskClass::Critical,
            other => panic!("unknown conformance risk class: {other}"),
        };
        let request_key = conformance_str(value, "request_idempotency_key");
        let request_fingerprint = conformance_str(value, "request_fingerprint");
        let journal_key = value.get("journal_idempotency_key").and_then(serde_json::Value::as_str);
        let journal_fingerprint = value.get("journal_request_fingerprint").and_then(serde_json::Value::as_str);

        evaluate_dispatch_gate(&DispatchGateInput {
            requested: &requested,
            admitted: &admitted,
            registered: &registered,
            request_idempotency_key: request_key,
            request_fingerprint,
            journal_idempotency_key: journal_key,
            journal_request_fingerprint: journal_fingerprint,
            now_unix_seconds: conformance_u64(value, "now_unix_seconds"),
            expires_at_unix_seconds: conformance_u64(value, "expires_at_unix_seconds"),
            max_attempts: conformance_u64(value, "max_attempts") as u8,
            attempts_started: conformance_u64(value, "attempts_started") as u8,
            journal_state,
            household_authority_current: conformance_bool(value, "household_authority_current"),
            adapter_admission_current: conformance_bool(value, "adapter_admission_current"),
            exclusive_executor_lease_held: conformance_bool(value, "exclusive_executor_lease_held"),
            durable_journal_ready: conformance_bool(value, "durable_journal_ready"),
            postcondition_observer_available: conformance_bool(value, "postcondition_observer_available"),
            risk_class,
            independent_physical_stop_ready: conformance_bool(value, "independent_physical_stop_ready"),
        })
    }

    fn conformance_expected(value: &serde_json::Value) -> DispatchDecision {
        match conformance_str(value, "kind") {
            "allow_dispatch" => DispatchDecision::AllowDispatch,
            "reconcile_only" => DispatchDecision::ReconcileOnly,
            "terminal" => DispatchDecision::Terminal,
            "deny" => {
                let reason = match conformance_str(value, "reason") {
                    "empty_idempotency_key" => DenialReason::EmptyIdempotencyKey,
                    "empty_request_fingerprint" => DenialReason::EmptyRequestFingerprint,
                    "invalid_attempt_limit" => DenialReason::InvalidAttemptLimit,
                    "expired_request" => DenialReason::ExpiredRequest,
                    "household_authority_unavailable_or_revoked" => DenialReason::HouseholdAuthorityUnavailableOrRevoked,
                    "adapter_admission_stale_or_unverified" => DenialReason::AdapterAdmissionStaleOrUnverified,
                    "admission_binding_mismatch" => DenialReason::AdmissionBindingMismatch,
                    "registration_binding_mismatch" => DenialReason::RegistrationBindingMismatch,
                    "missing_exclusive_executor_lease" => DenialReason::MissingExclusiveExecutorLease,
                    "missing_durable_journal" => DenialReason::MissingDurableJournal,
                    "missing_postcondition_observer" => DenialReason::MissingPostconditionObserver,
                    "missing_physical_stop" => DenialReason::MissingPhysicalStop,
                    "attempts_exhausted" => DenialReason::AttemptsExhausted,
                    "idempotency_key_mismatch" => DenialReason::IdempotencyKeyMismatch,
                    "request_fingerprint_mismatch" => DenialReason::RequestFingerprintMismatch,
                    "inconsistent_journal_state" => DenialReason::InconsistentJournalState,
                    other => panic!("unknown conformance denial reason: {other}"),
                };
                DispatchDecision::Deny(reason)
            }
            other => panic!("unknown conformance expected decision: {other}"),
        }
    }

    #[test]
    fn shared_home_effect_conformance_schema_and_corpus_are_valid() {
        let schema: serde_json::Value = serde_json::from_str(CONFORMANCE_SCHEMA)
            .expect("home-effect conformance schema JSON must parse");
        assert!(
            jsonschema::draft202012::meta::is_valid(&schema),
            "home-effect conformance schema must validate as Draft 2020-12"
        );
        crate::validate_schema(&schema).expect("home-effect conformance schema must compile");
        let corpus: serde_json::Value = serde_json::from_str(CONFORMANCE_TRACES)
            .expect("shared conformance trace corpus JSON must parse");
        crate::validate_structural_document(&schema, &corpus)
            .expect("all shared conformance traces must satisfy their schema");
    }

    #[test]
    fn shared_home_effect_conformance_traces_match_reference_model() {
        let corpus: serde_json::Value = serde_json::from_str(CONFORMANCE_TRACES)
            .expect("shared conformance trace corpus JSON must parse");
        let traces = corpus.get("traces").and_then(serde_json::Value::as_array)
            .expect("corpus traces must be an array");
        let mut evaluated_steps = 0usize;
        let mut trace_ids = std::collections::BTreeSet::new();

        for trace in traces {
            let trace_id = conformance_str(trace, "id");
            assert!(trace_ids.insert(trace_id), "duplicate trace id: {trace_id}");
            let steps = trace.get("steps").and_then(serde_json::Value::as_array)
                .expect("trace steps must be an array");
            let mut step_ids = std::collections::BTreeSet::new();

            for step in steps {
                let step_id = conformance_str(step, "id");
                assert!(step_ids.insert(step_id), "duplicate step id in {trace_id}: {step_id}");
                let input = step.get("input").expect("step input must exist");
                let expected = conformance_expected(step.get("expect").expect("step expectation"));
                let actual = evaluate_conformance_input(input);
                assert_eq!(actual, expected, "shared conformance divergence at trace={trace_id}, step={step_id}");
                evaluated_steps += 1;
            }
        }
        assert!(evaluated_steps >= 10, "conformance corpus unexpectedly lost adversarial coverage");
    }


    const BENCHMARK_TARGETS_SCHEMA: &str =
        include_str!("../../../schemas/home-systems-benchmark-targets.schema.json");
    const BENCHMARK_TARGETS: &str =
        include_str!("../../../benchmarks/home-systems/targets-v1.json");

    #[test]
    fn benchmark_targets_schema_and_catalogue_are_valid_and_explicitly_unmeasured() {
        let schema: serde_json::Value = serde_json::from_str(BENCHMARK_TARGETS_SCHEMA)
            .expect("benchmark target schema must parse");
        assert!(
            jsonschema::draft202012::meta::is_valid(&schema),
            "benchmark target schema must validate as Draft 2020-12"
        );
        crate::validate_schema(&schema).expect("benchmark target schema must compile");

        let catalogue: serde_json::Value = serde_json::from_str(BENCHMARK_TARGETS)
            .expect("benchmark target catalogue must parse");
        crate::validate_structural_document(&schema, &catalogue)
            .expect("benchmark target catalogue must satisfy its schema");
        assert_eq!(
            catalogue.get("status").and_then(serde_json::Value::as_str),
            Some("targets_not_results"),
            "target catalogue must not masquerade as measured benchmark results"
        );

        let benchmarks = catalogue.get("benchmarks").and_then(serde_json::Value::as_array)
            .expect("benchmark entries must be an array");
        let mut ids = std::collections::BTreeSet::new();
        assert!(benchmarks.len() >= 10, "benchmark catalogue must preserve broad coverage");
        for benchmark in benchmarks {
            let id = conformance_str(benchmark, "id");
            assert!(ids.insert(id), "duplicate benchmark ID: {id}");
            let measurement_status = conformance_str(benchmark, "measurement_status");
            assert!(
                matches!(measurement_status, "not_measured" | "blocked_runtime_not_ready"),
                "new targets must not claim unearned results: {id} has status {measurement_status}"
            );
        }
        assert!(
            benchmarks.iter().any(|benchmark|
                conformance_str(benchmark, "id") == "cross-implementation-divergence"
                && conformance_str(benchmark, "measurement_status") == "blocked_runtime_not_ready"
            ),
            "cross-implementation benchmark must explicitly state that its runtime adapter is not ready"
        );

        for (id, metric) in [
            ("ha-stale-positive-invalidation", "stale_positive_observations_after_invalidation"),
            ("ha-observation-stale-replay", "equal_or_older_observation_versions_applied"),
            ("ha-projector-delta-cache-scaling", "single_entity_delta_p95_growth_ratio"),
        ] {
            let target = benchmarks.iter().find(|benchmark| conformance_str(benchmark, "id") == id)
                .unwrap_or_else(|| panic!("required Home Assistant lifecycle benchmark is missing: {id}"));
            assert_eq!(
                conformance_str(target, "metric"),
                metric,
                "Home Assistant benchmark metric drifted for {id}"
            );
            assert_eq!(
                conformance_str(target, "measurement_status"),
                "blocked_runtime_not_ready",
                "runtime-dependent Home Assistant benchmark cannot claim results before the runtime exists: {id}"
            );
        }
    }


    #[test]
    fn exhaustively_checks_gate_invariants_across_387072_input_combinations() {
        let canonical = binding();
        let mut request_mismatch = binding();
        request_mismatch.operation = "light.turn_off".into();
        let mut registration_mismatch = binding();
        registration_mismatch.profile_ref = "profile:unexpected".into();

        let risks = [RiskClass::Low, RiskClass::Medium, RiskClass::High, RiskClass::Critical];
        let journal_states = [
            JournalState::NotStarted,
            JournalState::Dispatching,
            JournalState::ProviderAccepted,
            JournalState::OutcomeUnknown,
            JournalState::DefinitiveNoEffect,
            JournalState::ReconciledSuccess,
            JournalState::ReconciledFailure,
        ];
        let attempt_limits = [0u8, 1, 2];
        let attempts_started_values = [0u8, 1, 2];
        let mut evaluated = 0usize;
        let mut allowed = 0usize;

        for gate_mask in 0u8..64 {
            let household_authority_current = gate_mask & 1 != 0;
            let adapter_admission_current = gate_mask & 2 != 0;
            let exclusive_executor_lease_held = gate_mask & 4 != 0;
            let durable_journal_ready = gate_mask & 8 != 0;
            let postcondition_observer_available = gate_mask & 16 != 0;
            let independent_physical_stop_ready = gate_mask & 32 != 0;

            for risk_class in risks {
                for journal_state in journal_states {
                    for binding_case in 0..3 {
                        let (requested, admitted, registered) = match binding_case {
                            0 => (&canonical, &canonical, &canonical),
                            1 => (&request_mismatch, &canonical, &canonical),
                            2 => (&canonical, &canonical, &registration_mismatch),
                            _ => unreachable!("fixed binding case range"),
                        };

                        for identity_case in 0..4 {
                            let (journal_idempotency_key, journal_request_fingerprint) = match identity_case {
                                0 => (None, None),
                                1 => (Some("request-key-0001"), Some("fingerprint-v1")),
                                2 => (Some("different-key"), Some("fingerprint-v1")),
                                3 => (Some("request-key-0001"), Some("different-fingerprint")),
                                _ => unreachable!("fixed identity case range"),
                            };

                            for expired in [false, true] {
                                for max_attempts in attempt_limits {
                                    for attempts_started in attempts_started_values {
                                        let input = DispatchGateInput {
                                            requested,
                                            admitted,
                                            registered,
                                            request_idempotency_key: "request-key-0001",
                                            request_fingerprint: "fingerprint-v1",
                                            journal_idempotency_key,
                                            journal_request_fingerprint,
                                            now_unix_seconds: if expired { 200 } else { 100 },
                                            expires_at_unix_seconds: 200,
                                            max_attempts,
                                            attempts_started,
                                            journal_state,
                                            household_authority_current,
                                            adapter_admission_current,
                                            exclusive_executor_lease_held,
                                            durable_journal_ready,
                                            postcondition_observer_available,
                                            risk_class,
                                            independent_physical_stop_ready,
                                        };

                                        let decision = evaluate_dispatch_gate(&input);
                                        evaluated += 1;
                                        if decision == DispatchDecision::AllowDispatch {
                                            allowed += 1;
                                            let journal_identity_is_consistent = match (journal_state, identity_case, attempts_started) {
                                                (JournalState::NotStarted, 0, 0) => true,
                                                (JournalState::NotStarted, 1, 0) => true,
                                                (JournalState::DefinitiveNoEffect, 1, attempts) if attempts > 0 => true,
                                                _ => false,
                                            };
                                            assert!(
                                                household_authority_current,
                                                "dispatch allowed without current household authority"
                                            );
                                            assert!(
                                                adapter_admission_current,
                                                "dispatch allowed with stale/unverified admission"
                                            );
                                            assert_eq!(binding_case, 0, "dispatch allowed with mismatched binding");
                                            assert!(exclusive_executor_lease_held, "dispatch allowed without exclusive lease");
                                            assert!(durable_journal_ready, "dispatch allowed without durable journal");
                                            assert!(postcondition_observer_available, "dispatch allowed without postcondition observer");
                                            assert!(
                                                !matches!(risk_class, RiskClass::High | RiskClass::Critical)
                                                    || independent_physical_stop_ready,
                                                "high-risk dispatch allowed without independent physical stop"
                                            );
                                            assert!(!expired, "dispatch allowed after request expiry");
                                            assert!(max_attempts > 0 && max_attempts <= 3, "dispatch allowed with invalid attempt limit");
                                            assert!(attempts_started < max_attempts, "dispatch allowed after attempt budget exhausted");
                                            assert!(
                                                journal_identity_is_consistent,
                                                "dispatch allowed with inconsistent journal state/identity"
                                            );
                                        }
                                        if matches!(
                                            journal_state,
                                            JournalState::Dispatching
                                                | JournalState::ProviderAccepted
                                                | JournalState::OutcomeUnknown
                                                | JournalState::ReconciledSuccess
                                                | JournalState::ReconciledFailure
                                        ) {
                                            assert_ne!(
                                                decision,
                                                DispatchDecision::AllowDispatch,
                                                "unresolved or terminal journal state permitted a new dispatch"
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        assert_eq!(evaluated, 387_072, "Cartesian gate-state coverage changed; update the benchmark target deliberately");
        assert!(allowed > 0, "positive control produced no eligible dispatch state");
        assert!(allowed < evaluated, "adversarial combinations did not exercise rejection/reconciliation");
    }


    fn run_temporal_event_schedule(events: &[u8]) -> usize {
        const REQUEST_KEY: &str = "temporal-request-key-v1";
        const REQUEST_FINGERPRINT: &str = "temporal-request-fingerprint-v1";

        let requested_and_admitted = binding();
        let mut drifted_registration = binding();
        drifted_registration.implementation_ref = "git:adapter@drifted".into();

        let mut household_authority_current = true;
        let mut adapter_admission_current = true;
        let mut adapter_drifted = false;
        let mut exclusive_executor_lease_held = true;
        let durable_journal_ready = true;
        let postcondition_observer_available = true;
        let mut journal_state = JournalState::NotStarted;
        let mut attempts_started = 0u8;
        let mut journal_bound = false;
        let mut now_unix_seconds = 0u64;
        let expires_at_unix_seconds = 4u64;
        let max_attempts = 2u8;
        let mut dispatch_count = 0usize;

        for &event in events {
            match event {
                // 0: attempt a new effect / retry
                0 => {
                    let registered = if adapter_drifted {
                        &drifted_registration
                    } else {
                        &requested_and_admitted
                    };
                    let journal_key = journal_bound.then_some(REQUEST_KEY);
                    let journal_fingerprint = journal_bound.then_some(REQUEST_FINGERPRINT);
                    let input = DispatchGateInput {
                        requested: &requested_and_admitted,
                        admitted: &requested_and_admitted,
                        registered,
                        request_idempotency_key: REQUEST_KEY,
                        request_fingerprint: REQUEST_FINGERPRINT,
                        journal_idempotency_key: journal_key,
                        journal_request_fingerprint: journal_fingerprint,
                        now_unix_seconds,
                        expires_at_unix_seconds,
                        max_attempts,
                        attempts_started,
                        journal_state,
                        household_authority_current,
                        adapter_admission_current,
                        exclusive_executor_lease_held,
                        durable_journal_ready,
                        postcondition_observer_available,
                        risk_class: RiskClass::Low,
                        independent_physical_stop_ready: false,
                    };
                    let decision = evaluate_dispatch_gate(&input);
                    let unresolved_or_terminal = matches!(
                        journal_state,
                        JournalState::Dispatching
                            | JournalState::ProviderAccepted
                            | JournalState::OutcomeUnknown
                            | JournalState::ReconciledSuccess
                            | JournalState::ReconciledFailure
                    );
                    if unresolved_or_terminal {
                        assert_ne!(
                            decision,
                            DispatchDecision::AllowDispatch,
                            "schedule {events:?} allowed redispatch from unresolved/terminal state"
                        );
                    }
                    if decision == DispatchDecision::AllowDispatch {
                        assert!(household_authority_current, "dispatch allowed after authority revocation");
                        assert!(adapter_admission_current, "dispatch allowed with stale admission");
                        assert!(!adapter_drifted, "dispatch allowed against drifted registration");
                        assert!(exclusive_executor_lease_held, "dispatch allowed without exclusive lease");
                        assert!(durable_journal_ready, "dispatch allowed without durable journal");
                        assert!(postcondition_observer_available, "dispatch allowed without outcome observer");
                        assert!(now_unix_seconds < expires_at_unix_seconds, "dispatch allowed after expiry");
                        assert!(attempts_started < max_attempts, "dispatch exceeded attempt budget");
                        assert!(
                            matches!(journal_state, JournalState::NotStarted | JournalState::DefinitiveNoEffect),
                            "dispatch came from a forbidden journal state"
                        );

                        dispatch_count += 1;
                        attempts_started += 1;
                        journal_bound = true;
                        journal_state = JournalState::Dispatching;
                    }
                }
                // 1: revoke household authority. This model intentionally has no grant event.
                1 => household_authority_current = false,
                // 2: registered adapter changes after admission.
                2 => {
                    adapter_drifted = true;
                    adapter_admission_current = false;
                }
                // 3: revalidate admission only when the exact registered implementation still matches.
                3 => {
                    if !adapter_drifted {
                        adapter_admission_current = true;
                    }
                }
                // 4: lose the exclusive executor lease.
                4 => exclusive_executor_lease_held = false,
                // 5: crash/restart around an unresolved physical attempt.
                5 => {
                    if matches!(
                        journal_state,
                        JournalState::Dispatching | JournalState::ProviderAccepted
                    ) {
                        journal_state = JournalState::OutcomeUnknown;
                    }
                }
                // 6: a late provider acknowledgement arrives; it is still not physical proof.
                6 => {
                    if matches!(
                        journal_state,
                        JournalState::Dispatching | JournalState::OutcomeUnknown
                    ) {
                        journal_state = JournalState::ProviderAccepted;
                    }
                }
                // 7: an independent observer proves that the attempt had no effect.
                7 => {
                    if matches!(
                        journal_state,
                        JournalState::Dispatching
                            | JournalState::ProviderAccepted
                            | JournalState::OutcomeUnknown
                    ) {
                        journal_state = JournalState::DefinitiveNoEffect;
                    }
                }
                // 8: an appropriate observer reconciles a success.
                8 => {
                    if matches!(
                        journal_state,
                        JournalState::Dispatching
                            | JournalState::ProviderAccepted
                            | JournalState::OutcomeUnknown
                    ) {
                        journal_state = JournalState::ReconciledSuccess;
                    }
                }
                // 9: advance time toward/through expiry.
                9 => now_unix_seconds = now_unix_seconds.saturating_add(1),
                other => panic!("event outside fixed ten-event alphabet: {other}"),
            }

            assert!(dispatch_count <= max_attempts as usize, "attempt budget exceeded in {events:?}");
            assert_eq!(
                dispatch_count, attempts_started as usize,
                "dispatch count and durable attempt count diverged in {events:?}"
            );
            if matches!(
                journal_state,
                JournalState::Dispatching | JournalState::ProviderAccepted | JournalState::OutcomeUnknown
            ) {
                assert!(journal_bound, "unresolved effect lacks idempotency/fingerprint binding");
            }
        }

        dispatch_count
    }

    #[test]
    fn exhaustively_replays_one_million_six_event_histories() {
        const EVENT_ALPHABET: u32 = 10;
        const HISTORY_LENGTH: usize = 6;
        const SCHEDULE_COUNT: u32 = 1_000_000;
        let mut total_dispatches = 0usize;

        for mut encoded in 0..SCHEDULE_COUNT {
            let mut events = [0u8; HISTORY_LENGTH];
            for event in &mut events {
                *event = (encoded % EVENT_ALPHABET) as u8;
                encoded /= EVENT_ALPHABET;
            }
            total_dispatches += run_temporal_event_schedule(&events);
        }

        assert_eq!(SCHEDULE_COUNT, EVENT_ALPHABET.pow(HISTORY_LENGTH as u32));
        assert!(total_dispatches > 0, "event-history generator lost its positive control");
        assert!(
            total_dispatches <= SCHEDULE_COUNT as usize * 2,
            "some event history exceeded the bounded effect-attempt budget"
        );
    }

}
