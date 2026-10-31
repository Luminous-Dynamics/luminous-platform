#![forbid(unsafe_code)]
//! Reference state machine for cross-system service-episode sagas.
//!
//! This crate validates transition semantics over caller-supplied, normalized
//! facts. It does not verify signatures, authenticate observers, persist a
//! crash-consistent journal, or execute connector operations. Production
//! adapters must establish those facts before calling this model.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Digest(pub [u8; 32]);

impl Digest {
    #[must_use]
    pub fn is_zero(self) -> bool {
        self.0.iter().all(|byte| *byte == 0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompensationSpec {
    pub target_id: String,
    pub operation_id: String,
    pub request_digest: Digest,
    pub binding_digest: Digest,
    pub expected_postcondition_digest: Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationSpec {
    pub id: String,
    pub target_id: String,
    pub operation_id: String,
    pub request_digest: Digest,
    pub binding_digest: Digest,
    pub expected_postcondition_digest: Digest,
    pub dependencies: Vec<String>,
    pub required: bool,
    pub compensation: Option<CompensationSpec>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServicePlan {
    pub service_id: String,
    pub plan_digest: Digest,
    pub expected_service_digest: Digest,
    pub steps: Vec<OperationSpec>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanValidationError {
    EmptyServiceId,
    EmptyPlan,
    InvalidDigest(&'static str),
    EmptyField(&'static str),
    DuplicateStepId(String),
    DuplicateDependency {
        step_id: String,
        dependency_id: String,
    },
    MissingDependency {
        step_id: String,
        dependency_id: String,
    },
    SelfDependency(String),
    DependencyCycle,
    InvalidCompensation(String),
}

impl ServicePlan {
    /// Validate the immutable step graph before creating an episode.
    pub fn validate(&self) -> Result<(), PlanValidationError> {
        if self.service_id.trim().is_empty() {
            return Err(PlanValidationError::EmptyServiceId);
        }
        if self.steps.is_empty() {
            return Err(PlanValidationError::EmptyPlan);
        }
        if self.plan_digest.is_zero() {
            return Err(PlanValidationError::InvalidDigest("plan_digest"));
        }
        if self.expected_service_digest.is_zero() {
            return Err(PlanValidationError::InvalidDigest(
                "expected_service_digest",
            ));
        }

        let mut by_id = BTreeMap::new();
        for step in &self.steps {
            if step.id.trim().is_empty()
                || step.target_id.trim().is_empty()
                || step.operation_id.trim().is_empty()
            {
                return Err(PlanValidationError::EmptyField("step identity"));
            }
            if step.request_digest.is_zero() {
                return Err(PlanValidationError::InvalidDigest("request_digest"));
            }
            if step.binding_digest.is_zero() {
                return Err(PlanValidationError::InvalidDigest("binding_digest"));
            }
            if step.expected_postcondition_digest.is_zero() {
                return Err(PlanValidationError::InvalidDigest(
                    "expected_postcondition_digest",
                ));
            }
            if by_id.insert(step.id.clone(), step).is_some() {
                return Err(PlanValidationError::DuplicateStepId(step.id.clone()));
            }

            if let Some(compensation) = &step.compensation {
                if compensation.target_id.trim().is_empty()
                    || compensation.operation_id.trim().is_empty()
                    || compensation.request_digest.is_zero()
                    || compensation.request_digest == step.request_digest
                    || compensation.binding_digest.is_zero()
                    || compensation.binding_digest == step.binding_digest
                    || compensation.expected_postcondition_digest.is_zero()
                {
                    return Err(PlanValidationError::InvalidCompensation(step.id.clone()));
                }
            }

            let mut seen_dependencies = BTreeSet::new();
            for dependency in &step.dependencies {
                if dependency == &step.id {
                    return Err(PlanValidationError::SelfDependency(step.id.clone()));
                }
                if !seen_dependencies.insert(dependency) {
                    return Err(PlanValidationError::DuplicateDependency {
                        step_id: step.id.clone(),
                        dependency_id: dependency.clone(),
                    });
                }
            }
        }

        let mut indegree = BTreeMap::<String, usize>::new();
        let mut dependents = BTreeMap::<String, Vec<String>>::new();
        for step in &self.steps {
            indegree.insert(step.id.clone(), step.dependencies.len());
            for dependency in &step.dependencies {
                if !by_id.contains_key(dependency) {
                    return Err(PlanValidationError::MissingDependency {
                        step_id: step.id.clone(),
                        dependency_id: dependency.clone(),
                    });
                }
                dependents
                    .entry(dependency.clone())
                    .or_default()
                    .push(step.id.clone());
            }
        }

        let mut ready: VecDeque<String> = indegree
            .iter()
            .filter(|(_, degree)| **degree == 0)
            .map(|(step_id, _)| step_id.clone())
            .collect();
        let mut visited = 0_usize;
        while let Some(step_id) = ready.pop_front() {
            visited += 1;
            if let Some(children) = dependents.get(&step_id) {
                for child in children {
                    let degree = indegree
                        .get_mut(child)
                        .expect("validated dependency graph contains every child");
                    *degree -= 1;
                    if *degree == 0 {
                        ready.push_back(child.clone());
                    }
                }
            }
        }
        if visited != self.steps.len() {
            return Err(PlanValidationError::DependencyCycle);
        }

        Ok(())
    }

    fn step(&self, step_id: &str) -> Option<&OperationSpec> {
        self.steps.iter().find(|step| step.id == step_id)
    }
}

/// Metadata produced only after the upstream authority adapter has validated
/// signature, issuer, audience, revocation and trusted time. The reference
/// model checks exact binding and expiry; it is not the signature verifier.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorityGrant {
    pub grant_id: String,
    pub plan_digest: Digest,
    pub step_id: String,
    pub target_id: String,
    pub operation_id: String,
    pub request_digest: Digest,
    pub binding_digest: Digest,
    pub valid_until_ms: u64,
}

impl AuthorityGrant {
    #[allow(clippy::too_many_arguments)]
    fn matches(
        &self,
        plan: &ServicePlan,
        step_id: &str,
        target_id: &str,
        operation_id: &str,
        request_digest: Digest,
        binding_digest: Digest,
        now_ms: u64,
    ) -> bool {
        !self.grant_id.trim().is_empty()
            && self.valid_until_ms > now_ms
            && self.plan_digest == plan.plan_digest
            && self.step_id == step_id
            && self.target_id == target_id
            && self.operation_id == operation_id
            && self.request_digest == request_digest
            && self.binding_digest == binding_digest
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Consistency {
    Strong,
    Eventual,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservedEffect {
    ExpectedPostcondition,
    NoEffect,
    ForbiddenEffect,
    Conflicting,
    Unknown,
}

/// A current-state read that finds no change is not proof that a dispatched
/// request was never accepted. Only an attempt-specific authoritative ledger
/// result can clear the ambiguous attempt for a new authorized retry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoEffectBasis {
    AuthoritativeAttemptLedgerNeverAccepted,
    StateReadbackOnly,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObserverEvidence {
    pub observer_id: String,
    pub trust_domain_id: String,
    pub attempt_id: String,
    pub target_id: String,
    pub binding_digest: Digest,
    pub effect: ObservedEffect,
    pub consistency: Consistency,
    pub postcondition_digest: Option<Digest>,
    pub no_effect_basis: Option<NoEffectBasis>,
    pub observed_at_ms: u64,
    pub valid_until_ms: u64,
    pub evidence_digest: Digest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServiceHealth {
    Healthy,
    Unhealthy,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceObservation {
    pub observer_id: String,
    pub trust_domain_id: String,
    pub service_id: String,
    pub plan_digest: Digest,
    pub health: ServiceHealth,
    pub consistency: Consistency,
    pub observed_service_digest: Option<Digest>,
    pub observed_at_ms: u64,
    pub valid_until_ms: u64,
    pub evidence_digest: Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StepStatus {
    Pending,
    Authorized {
        grant_id: String,
    },
    DispatchIntentPersisted {
        attempt_id: String,
        idempotency_key: String,
    },
    OutcomeUnknown {
        attempt_id: String,
        idempotency_key: String,
    },
    Verified {
        attempt_id: String,
        evidence_digest: Digest,
    },
    NoEffectConfirmed {
        attempt_id: String,
        evidence_digest: Digest,
    },
    Indeterminate {
        attempt_id: String,
        evidence_digest: Digest,
    },
    PostconditionMismatch {
        attempt_id: String,
        evidence_digest: Digest,
    },
    ViolationObserved {
        attempt_id: String,
        evidence_digest: Digest,
    },
    CompensationAuthorized {
        grant_id: String,
    },
    CompensationBlocked,
    CompensationDispatchIntentPersisted {
        attempt_id: String,
        idempotency_key: String,
    },
    CompensationOutcomeUnknown {
        attempt_id: String,
        idempotency_key: String,
    },
    Compensated {
        attempt_id: String,
        evidence_digest: Digest,
    },
    CompensationIndeterminate {
        attempt_id: String,
        evidence_digest: Digest,
    },
    Blocked,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventKind {
    StepAuthorized,
    DispatchIntentPersisted,
    DispatchOutcomeUnknown,
    ForwardEffectReconciled,
    CompensationAuthorized,
    CompensationDispatchIntentPersisted,
    CompensationOutcomeUnknown,
    CompensationEffectReconciled,
    CancellationRequested,
    ServiceVerificationRecorded,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EpisodeEvent {
    pub revision: u64,
    pub step_id: Option<String>,
    pub attempt_id: Option<String>,
    pub kind: EventKind,
    pub evidence_digest: Option<Digest>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServiceVerification {
    VerifiedHealthy,
    Unhealthy,
    Indeterminate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EpisodeDisposition {
    Running,
    PartiallyComplete,
    AwaitingServiceVerification,
    Indeterminate,
    EscalationRequired,
    SecurityViolationObserved,
    Resolved,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransitionError {
    StaleRevision { expected: u64, actual: u64 },
    UnknownStep(String),
    InvalidTransition(String),
    InvalidGrantBinding,
    ExpiredGrant,
    DuplicateGrant(String),
    DuplicateAttempt(String),
    InvalidAttemptOrKey,
    IdempotencyKeyChanged,
    DependencyNotVerified(String),
    InvalidObserver,
    AttemptMismatch,
    StaleEvidence,
    RevisionExhausted,
    Cancelled,
    CompensationUnavailable,
    ServicePreconditionsNotMet,
    AlreadyResolved,
}

#[derive(Clone, Debug)]
struct StepRuntime {
    status: StepStatus,
    forward_idempotency_key: Option<String>,
    compensation_idempotency_key: Option<String>,
}

impl Default for StepRuntime {
    fn default() -> Self {
        Self {
            status: StepStatus::Pending,
            forward_idempotency_key: None,
            compensation_idempotency_key: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SagaEpisode {
    plan: ServicePlan,
    agent_trust_domain_id: String,
    revision: u64,
    cancelled: bool,
    service_verification: Option<ServiceVerification>,
    steps: BTreeMap<String, StepRuntime>,
    used_grants: BTreeSet<String>,
    attempt_ids: BTreeSet<String>,
    events: Vec<EpisodeEvent>,
}

impl SagaEpisode {
    pub fn new(
        plan: ServicePlan,
        agent_trust_domain_id: impl Into<String>,
    ) -> Result<Self, PlanValidationError> {
        plan.validate()?;
        let steps = plan
            .steps
            .iter()
            .map(|step| (step.id.clone(), StepRuntime::default()))
            .collect();
        Ok(Self {
            plan,
            agent_trust_domain_id: agent_trust_domain_id.into(),
            revision: 0,
            cancelled: false,
            service_verification: None,
            steps,
            used_grants: BTreeSet::new(),
            attempt_ids: BTreeSet::new(),
            events: Vec::new(),
        })
    }

    #[must_use]
    pub fn revision(&self) -> u64 {
        self.revision
    }

    #[must_use]
    pub fn events(&self) -> &[EpisodeEvent] {
        &self.events
    }

    #[must_use]
    pub fn step_status(&self, step_id: &str) -> Option<&StepStatus> {
        self.steps.get(step_id).map(|runtime| &runtime.status)
    }

    #[must_use]
    pub fn disposition(&self) -> EpisodeDisposition {
        if self
            .steps
            .values()
            .any(|runtime| matches!(&runtime.status, StepStatus::ViolationObserved { .. }))
        {
            return EpisodeDisposition::SecurityViolationObserved;
        }

        match self.service_verification {
            Some(ServiceVerification::VerifiedHealthy) => {
                return EpisodeDisposition::Resolved;
            }
            Some(ServiceVerification::Unhealthy) => {
                return EpisodeDisposition::EscalationRequired;
            }
            Some(ServiceVerification::Indeterminate) => {
                return EpisodeDisposition::Indeterminate;
            }
            None => {}
        }

        let required_statuses: Vec<&StepStatus> = self
            .plan
            .steps
            .iter()
            .filter(|step| step.required)
            .filter_map(|step| self.step_status(&step.id))
            .collect();

        if self.steps.values().any(|runtime| {
            matches!(
                &runtime.status,
                StepStatus::OutcomeUnknown { .. }
                    | StepStatus::Indeterminate { .. }
                    | StepStatus::PostconditionMismatch { .. }
                    | StepStatus::CompensationOutcomeUnknown { .. }
                    | StepStatus::CompensationIndeterminate { .. }
            )
        }) {
            return EpisodeDisposition::Indeterminate;
        }

        let optional_participant_unsettled = self
            .plan
            .steps
            .iter()
            .filter(|step| !step.required)
            .any(|step| {
                matches!(
                    self.step_status(&step.id),
                    Some(
                        StepStatus::Authorized { .. }
                            | StepStatus::DispatchIntentPersisted { .. }
                            | StepStatus::OutcomeUnknown { .. }
                            | StepStatus::Indeterminate { .. }
                            | StepStatus::PostconditionMismatch { .. }
                            | StepStatus::ViolationObserved { .. }
                            | StepStatus::CompensationAuthorized { .. }
                            | StepStatus::CompensationBlocked
                            | StepStatus::CompensationDispatchIntentPersisted { .. }
                            | StepStatus::CompensationOutcomeUnknown { .. }
                            | StepStatus::CompensationIndeterminate { .. }
                            | StepStatus::Compensated { .. }
                    )
                )
            });
        if optional_participant_unsettled {
            return EpisodeDisposition::PartiallyComplete;
        }

        if required_statuses
            .iter()
            .all(|status| matches!(status, StepStatus::Verified { .. }))
        {
            return EpisodeDisposition::AwaitingServiceVerification;
        }

        if self.cancelled
            || required_statuses.iter().any(|status| {
                matches!(status, StepStatus::Blocked | StepStatus::Compensated { .. })
            })
        {
            return EpisodeDisposition::EscalationRequired;
        }

        if required_statuses
            .iter()
            .any(|status| !matches!(status, StepStatus::Pending | StepStatus::Blocked))
        {
            return EpisodeDisposition::PartiallyComplete;
        }

        EpisodeDisposition::Running
    }

    pub fn authorize_step(
        &mut self,
        expected_revision: u64,
        step_id: &str,
        grant: &AuthorityGrant,
        now_ms: u64,
    ) -> Result<(), TransitionError> {
        self.check_revision(expected_revision)?;
        self.ensure_not_resolved()?;
        if self.cancelled {
            return Err(TransitionError::Cancelled);
        }
        let step = self
            .plan
            .step(step_id)
            .cloned()
            .ok_or_else(|| TransitionError::UnknownStep(step_id.to_owned()))?;
        let status = self
            .step_status(step_id)
            .cloned()
            .ok_or_else(|| TransitionError::UnknownStep(step_id.to_owned()))?;
        if !matches!(
            status,
            StepStatus::Pending | StepStatus::NoEffectConfirmed { .. }
        ) {
            return Err(TransitionError::InvalidTransition(
                "forward authorization requires Pending or strong NoEffectConfirmed".into(),
            ));
        }
        if self.used_grants.contains(&grant.grant_id) {
            return Err(TransitionError::DuplicateGrant(grant.grant_id.clone()));
        }
        if grant.valid_until_ms <= now_ms {
            return Err(TransitionError::ExpiredGrant);
        }
        if !grant.matches(
            &self.plan,
            step_id,
            &step.target_id,
            &step.operation_id,
            step.request_digest,
            step.binding_digest,
            now_ms,
        ) {
            return Err(TransitionError::InvalidGrantBinding);
        }
        for dependency in &step.dependencies {
            if !matches!(
                self.step_status(dependency),
                Some(StepStatus::Verified { .. })
            ) {
                return Err(TransitionError::DependencyNotVerified(dependency.clone()));
            }
        }

        self.used_grants.insert(grant.grant_id.clone());
        self.steps
            .get_mut(step_id)
            .expect("step exists after immutable plan lookup")
            .status = StepStatus::Authorized {
            grant_id: grant.grant_id.clone(),
        };
        self.append_event(
            Some(step_id.to_owned()),
            None,
            EventKind::StepAuthorized,
            None,
        );
        Ok(())
    }

    pub fn persist_dispatch_intent(
        &mut self,
        expected_revision: u64,
        step_id: &str,
        attempt_id: &str,
        idempotency_key: &str,
    ) -> Result<(), TransitionError> {
        self.check_revision(expected_revision)?;
        self.ensure_not_resolved()?;
        if attempt_id.trim().is_empty() || idempotency_key.trim().is_empty() {
            return Err(TransitionError::InvalidAttemptOrKey);
        }
        let status = self
            .step_status(step_id)
            .cloned()
            .ok_or_else(|| TransitionError::UnknownStep(step_id.to_owned()))?;
        if !matches!(status, StepStatus::Authorized { .. }) {
            return Err(TransitionError::InvalidTransition(
                "dispatch intent requires an authorized step".into(),
            ));
        }
        if self.attempt_ids.contains(attempt_id) {
            return Err(TransitionError::DuplicateAttempt(attempt_id.to_owned()));
        }
        let runtime = self
            .steps
            .get(step_id)
            .expect("step exists after immutable plan lookup");
        if runtime
            .forward_idempotency_key
            .as_deref()
            .is_some_and(|previous| previous != idempotency_key)
        {
            return Err(TransitionError::IdempotencyKeyChanged);
        }

        self.attempt_ids.insert(attempt_id.to_owned());
        let runtime = self.steps.get_mut(step_id).expect("step exists");
        runtime.forward_idempotency_key = Some(idempotency_key.to_owned());
        runtime.status = StepStatus::DispatchIntentPersisted {
            attempt_id: attempt_id.to_owned(),
            idempotency_key: idempotency_key.to_owned(),
        };
        self.append_event(
            Some(step_id.to_owned()),
            Some(attempt_id.to_owned()),
            EventKind::DispatchIntentPersisted,
            None,
        );
        Ok(())
    }

    pub fn mark_outcome_unknown(
        &mut self,
        expected_revision: u64,
        step_id: &str,
    ) -> Result<(), TransitionError> {
        self.check_revision(expected_revision)?;
        self.ensure_not_resolved()?;
        let status = self
            .step_status(step_id)
            .cloned()
            .ok_or_else(|| TransitionError::UnknownStep(step_id.to_owned()))?;
        let (attempt_id, idempotency_key) = match status {
            StepStatus::DispatchIntentPersisted {
                attempt_id,
                idempotency_key,
            } => (attempt_id, idempotency_key),
            _ => {
                return Err(TransitionError::InvalidTransition(
                    "only a dispatched forward attempt can become outcome_unknown".into(),
                ));
            }
        };
        self.steps.get_mut(step_id).expect("step exists").status = StepStatus::OutcomeUnknown {
            attempt_id: attempt_id.clone(),
            idempotency_key,
        };
        self.append_event(
            Some(step_id.to_owned()),
            Some(attempt_id),
            EventKind::DispatchOutcomeUnknown,
            None,
        );
        Ok(())
    }

    pub fn record_observation(
        &mut self,
        expected_revision: u64,
        step_id: &str,
        evidence: &ObserverEvidence,
        now_ms: u64,
    ) -> Result<(), TransitionError> {
        self.check_revision(expected_revision)?;
        self.ensure_not_resolved()?;
        let step = self
            .plan
            .step(step_id)
            .cloned()
            .ok_or_else(|| TransitionError::UnknownStep(step_id.to_owned()))?;
        let status = self
            .step_status(step_id)
            .cloned()
            .ok_or_else(|| TransitionError::UnknownStep(step_id.to_owned()))?;
        let current_attempt = match status {
            StepStatus::DispatchIntentPersisted { attempt_id, .. }
            | StepStatus::OutcomeUnknown { attempt_id, .. }
            | StepStatus::Indeterminate { attempt_id, .. }
            | StepStatus::PostconditionMismatch { attempt_id, .. } => attempt_id,
            _ => {
                return Err(TransitionError::InvalidTransition(
                    "forward observation requires a dispatched unresolved attempt".into(),
                ));
            }
        };
        self.validate_observer(
            &evidence.observer_id,
            &evidence.trust_domain_id,
            &evidence.target_id,
            &evidence.binding_digest,
            &step.target_id,
            &step.binding_digest,
            evidence.evidence_digest,
        )?;
        if evidence.attempt_id != current_attempt {
            return Err(TransitionError::AttemptMismatch);
        }
        Self::validate_freshness(evidence.observed_at_ms, evidence.valid_until_ms, now_ms)?;

        let next_status = match evidence.effect {
            ObservedEffect::ExpectedPostcondition
                if evidence.consistency == Consistency::Strong
                    && evidence.postcondition_digest
                        == Some(step.expected_postcondition_digest) =>
            {
                StepStatus::Verified {
                    attempt_id: current_attempt.clone(),
                    evidence_digest: evidence.evidence_digest,
                }
            }
            ObservedEffect::ExpectedPostcondition
                if evidence.consistency == Consistency::Strong =>
            {
                StepStatus::PostconditionMismatch {
                    attempt_id: current_attempt.clone(),
                    evidence_digest: evidence.evidence_digest,
                }
            }
            ObservedEffect::NoEffect
                if evidence.consistency == Consistency::Strong
                    && evidence.no_effect_basis
                        == Some(NoEffectBasis::AuthoritativeAttemptLedgerNeverAccepted) =>
            {
                StepStatus::NoEffectConfirmed {
                    attempt_id: current_attempt.clone(),
                    evidence_digest: evidence.evidence_digest,
                }
            }
            ObservedEffect::ForbiddenEffect if evidence.consistency == Consistency::Strong => {
                StepStatus::ViolationObserved {
                    attempt_id: current_attempt.clone(),
                    evidence_digest: evidence.evidence_digest,
                }
            }
            _ => StepStatus::Indeterminate {
                attempt_id: current_attempt.clone(),
                evidence_digest: evidence.evidence_digest,
            },
        };

        self.steps.get_mut(step_id).expect("step exists").status = next_status;
        self.append_event(
            Some(step_id.to_owned()),
            Some(current_attempt),
            EventKind::ForwardEffectReconciled,
            Some(evidence.evidence_digest),
        );
        Ok(())
    }

    pub fn authorize_compensation(
        &mut self,
        expected_revision: u64,
        step_id: &str,
        grant: &AuthorityGrant,
        now_ms: u64,
    ) -> Result<(), TransitionError> {
        self.check_revision(expected_revision)?;
        self.ensure_not_resolved()?;
        if self.cancelled {
            return Err(TransitionError::Cancelled);
        }
        let step = self
            .plan
            .step(step_id)
            .cloned()
            .ok_or_else(|| TransitionError::UnknownStep(step_id.to_owned()))?;
        if !matches!(self.step_status(step_id), Some(StepStatus::Verified { .. })) {
            return Err(TransitionError::InvalidTransition(
                "compensation requires a verified forward postcondition".into(),
            ));
        }
        let compensation = step
            .compensation
            .ok_or(TransitionError::CompensationUnavailable)?;
        if self.used_grants.contains(&grant.grant_id) {
            return Err(TransitionError::DuplicateGrant(grant.grant_id.clone()));
        }
        if grant.valid_until_ms <= now_ms {
            return Err(TransitionError::ExpiredGrant);
        }
        if !grant.matches(
            &self.plan,
            step_id,
            &compensation.target_id,
            &compensation.operation_id,
            compensation.request_digest,
            compensation.binding_digest,
            now_ms,
        ) {
            return Err(TransitionError::InvalidGrantBinding);
        }

        self.used_grants.insert(grant.grant_id.clone());
        self.steps.get_mut(step_id).expect("step exists").status =
            StepStatus::CompensationAuthorized {
                grant_id: grant.grant_id.clone(),
            };
        self.append_event(
            Some(step_id.to_owned()),
            None,
            EventKind::CompensationAuthorized,
            None,
        );
        Ok(())
    }

    pub fn persist_compensation_intent(
        &mut self,
        expected_revision: u64,
        step_id: &str,
        attempt_id: &str,
        idempotency_key: &str,
    ) -> Result<(), TransitionError> {
        self.check_revision(expected_revision)?;
        self.ensure_not_resolved()?;
        if attempt_id.trim().is_empty() || idempotency_key.trim().is_empty() {
            return Err(TransitionError::InvalidAttemptOrKey);
        }
        if !matches!(
            self.step_status(step_id),
            Some(StepStatus::CompensationAuthorized { .. })
        ) {
            return Err(TransitionError::InvalidTransition(
                "compensation dispatch requires separate compensation authorization".into(),
            ));
        }
        if self.attempt_ids.contains(attempt_id) {
            return Err(TransitionError::DuplicateAttempt(attempt_id.to_owned()));
        }
        let runtime = self
            .steps
            .get(step_id)
            .ok_or_else(|| TransitionError::UnknownStep(step_id.to_owned()))?;
        if runtime
            .compensation_idempotency_key
            .as_deref()
            .is_some_and(|previous| previous != idempotency_key)
        {
            return Err(TransitionError::IdempotencyKeyChanged);
        }

        self.attempt_ids.insert(attempt_id.to_owned());
        let runtime = self.steps.get_mut(step_id).expect("step exists");
        runtime.compensation_idempotency_key = Some(idempotency_key.to_owned());
        runtime.status = StepStatus::CompensationDispatchIntentPersisted {
            attempt_id: attempt_id.to_owned(),
            idempotency_key: idempotency_key.to_owned(),
        };
        self.append_event(
            Some(step_id.to_owned()),
            Some(attempt_id.to_owned()),
            EventKind::CompensationDispatchIntentPersisted,
            None,
        );
        Ok(())
    }

    pub fn mark_compensation_outcome_unknown(
        &mut self,
        expected_revision: u64,
        step_id: &str,
    ) -> Result<(), TransitionError> {
        self.check_revision(expected_revision)?;
        self.ensure_not_resolved()?;
        let status = self
            .step_status(step_id)
            .cloned()
            .ok_or_else(|| TransitionError::UnknownStep(step_id.to_owned()))?;
        let (attempt_id, idempotency_key) = match status {
            StepStatus::CompensationDispatchIntentPersisted {
                attempt_id,
                idempotency_key,
            } => (attempt_id, idempotency_key),
            _ => {
                return Err(TransitionError::InvalidTransition(
                    "only a dispatched compensation can become outcome_unknown".into(),
                ));
            }
        };
        self.steps.get_mut(step_id).expect("step exists").status =
            StepStatus::CompensationOutcomeUnknown {
                attempt_id: attempt_id.clone(),
                idempotency_key,
            };
        self.append_event(
            Some(step_id.to_owned()),
            Some(attempt_id),
            EventKind::CompensationOutcomeUnknown,
            None,
        );
        Ok(())
    }

    pub fn record_compensation_observation(
        &mut self,
        expected_revision: u64,
        step_id: &str,
        evidence: &ObserverEvidence,
        now_ms: u64,
    ) -> Result<(), TransitionError> {
        self.check_revision(expected_revision)?;
        self.ensure_not_resolved()?;
        let step = self
            .plan
            .step(step_id)
            .cloned()
            .ok_or_else(|| TransitionError::UnknownStep(step_id.to_owned()))?;
        let compensation = step
            .compensation
            .ok_or(TransitionError::CompensationUnavailable)?;
        let status = self
            .step_status(step_id)
            .cloned()
            .ok_or_else(|| TransitionError::UnknownStep(step_id.to_owned()))?;
        let current_attempt = match status {
            StepStatus::CompensationDispatchIntentPersisted { attempt_id, .. }
            | StepStatus::CompensationOutcomeUnknown { attempt_id, .. }
            | StepStatus::CompensationIndeterminate { attempt_id, .. } => attempt_id,
            _ => {
                return Err(TransitionError::InvalidTransition(
                    "compensation observation requires a dispatched unresolved compensation".into(),
                ));
            }
        };
        self.validate_observer(
            &evidence.observer_id,
            &evidence.trust_domain_id,
            &evidence.target_id,
            &evidence.binding_digest,
            &compensation.target_id,
            &compensation.binding_digest,
            evidence.evidence_digest,
        )?;
        if evidence.attempt_id != current_attempt {
            return Err(TransitionError::AttemptMismatch);
        }
        Self::validate_freshness(evidence.observed_at_ms, evidence.valid_until_ms, now_ms)?;

        let next_status = match evidence.effect {
            ObservedEffect::ExpectedPostcondition
                if evidence.consistency == Consistency::Strong
                    && evidence.postcondition_digest
                        == Some(compensation.expected_postcondition_digest) =>
            {
                StepStatus::Compensated {
                    attempt_id: current_attempt.clone(),
                    evidence_digest: evidence.evidence_digest,
                }
            }
            ObservedEffect::ForbiddenEffect if evidence.consistency == Consistency::Strong => {
                StepStatus::ViolationObserved {
                    attempt_id: current_attempt.clone(),
                    evidence_digest: evidence.evidence_digest,
                }
            }
            _ => StepStatus::CompensationIndeterminate {
                attempt_id: current_attempt.clone(),
                evidence_digest: evidence.evidence_digest,
            },
        };

        self.steps.get_mut(step_id).expect("step exists").status = next_status;
        self.append_event(
            Some(step_id.to_owned()),
            Some(current_attempt),
            EventKind::CompensationEffectReconciled,
            Some(evidence.evidence_digest),
        );
        Ok(())
    }

    pub fn request_cancellation(&mut self, expected_revision: u64) -> Result<(), TransitionError> {
        self.check_revision(expected_revision)?;
        if self.service_verification == Some(ServiceVerification::VerifiedHealthy) {
            return Err(TransitionError::AlreadyResolved);
        }
        if self.cancelled {
            return Err(TransitionError::InvalidTransition(
                "cancellation was already requested".into(),
            ));
        }

        for runtime in self.steps.values_mut() {
            runtime.status = match runtime.status.clone() {
                StepStatus::Pending
                | StepStatus::Authorized { .. }
                | StepStatus::NoEffectConfirmed { .. } => StepStatus::Blocked,
                StepStatus::CompensationAuthorized { .. } => StepStatus::CompensationBlocked,
                StepStatus::DispatchIntentPersisted {
                    attempt_id,
                    idempotency_key,
                } => StepStatus::OutcomeUnknown {
                    attempt_id,
                    idempotency_key,
                },
                StepStatus::CompensationDispatchIntentPersisted {
                    attempt_id,
                    idempotency_key,
                } => StepStatus::CompensationOutcomeUnknown {
                    attempt_id,
                    idempotency_key,
                },
                other => other,
            };
        }
        self.cancelled = true;
        self.append_event(None, None, EventKind::CancellationRequested, None);
        Ok(())
    }

    pub fn verify_service(
        &mut self,
        expected_revision: u64,
        evidence: &ServiceObservation,
        now_ms: u64,
    ) -> Result<(), TransitionError> {
        self.check_revision(expected_revision)?;
        if self.service_verification == Some(ServiceVerification::VerifiedHealthy) {
            return Err(TransitionError::AlreadyResolved);
        }
        if self.cancelled
            || !self
                .plan
                .steps
                .iter()
                .filter(|step| step.required)
                .all(|step| {
                    matches!(
                        self.step_status(&step.id),
                        Some(StepStatus::Verified { .. })
                    )
                })
            || self.steps.values().any(|runtime| {
                matches!(
                    &runtime.status,
                    StepStatus::Authorized { .. }
                        | StepStatus::DispatchIntentPersisted { .. }
                        | StepStatus::OutcomeUnknown { .. }
                        | StepStatus::Indeterminate { .. }
                        | StepStatus::PostconditionMismatch { .. }
                        | StepStatus::ViolationObserved { .. }
                        | StepStatus::CompensationAuthorized { .. }
                        | StepStatus::CompensationBlocked
                        | StepStatus::CompensationDispatchIntentPersisted { .. }
                        | StepStatus::CompensationOutcomeUnknown { .. }
                        | StepStatus::CompensationIndeterminate { .. }
                        | StepStatus::Compensated { .. }
                )
            })
        {
            return Err(TransitionError::ServicePreconditionsNotMet);
        }
        if evidence.observer_id.trim().is_empty()
            || evidence.trust_domain_id.trim().is_empty()
            || evidence.trust_domain_id == self.agent_trust_domain_id
            || evidence.service_id != self.plan.service_id
            || evidence.plan_digest != self.plan.plan_digest
            || evidence.evidence_digest.is_zero()
        {
            return Err(TransitionError::InvalidObserver);
        }
        Self::validate_freshness(evidence.observed_at_ms, evidence.valid_until_ms, now_ms)?;

        let verification = if evidence.consistency != Consistency::Strong {
            ServiceVerification::Indeterminate
        } else {
            match evidence.health {
                ServiceHealth::Healthy
                    if evidence.observed_service_digest
                        == Some(self.plan.expected_service_digest) =>
                {
                    ServiceVerification::VerifiedHealthy
                }
                ServiceHealth::Unhealthy => ServiceVerification::Unhealthy,
                ServiceHealth::Healthy | ServiceHealth::Unknown => {
                    ServiceVerification::Indeterminate
                }
            }
        };
        self.service_verification = Some(verification);
        self.append_event(
            None,
            None,
            EventKind::ServiceVerificationRecorded,
            Some(evidence.evidence_digest),
        );
        Ok(())
    }

    fn ensure_not_resolved(&self) -> Result<(), TransitionError> {
        if self.service_verification == Some(ServiceVerification::VerifiedHealthy) {
            return Err(TransitionError::AlreadyResolved);
        }
        Ok(())
    }

    fn validate_freshness(
        observed_at_ms: u64,
        valid_until_ms: u64,
        now_ms: u64,
    ) -> Result<(), TransitionError> {
        if observed_at_ms > now_ms || valid_until_ms <= now_ms || valid_until_ms <= observed_at_ms {
            return Err(TransitionError::StaleEvidence);
        }
        Ok(())
    }

    fn check_revision(&self, expected_revision: u64) -> Result<(), TransitionError> {
        if expected_revision != self.revision {
            return Err(TransitionError::StaleRevision {
                expected: expected_revision,
                actual: self.revision,
            });
        }
        if self.revision.checked_add(1).is_none() {
            return Err(TransitionError::RevisionExhausted);
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn validate_observer(
        &self,
        observer_id: &str,
        trust_domain_id: &str,
        observed_target_id: &str,
        observed_binding_digest: &Digest,
        expected_target_id: &str,
        expected_binding_digest: &Digest,
        evidence_digest: Digest,
    ) -> Result<(), TransitionError> {
        if observer_id.trim().is_empty()
            || trust_domain_id.trim().is_empty()
            || trust_domain_id == self.agent_trust_domain_id
            || observed_target_id != expected_target_id
            || observed_binding_digest != expected_binding_digest
            || evidence_digest.is_zero()
        {
            return Err(TransitionError::InvalidObserver);
        }
        Ok(())
    }

    fn append_event(
        &mut self,
        step_id: Option<String>,
        attempt_id: Option<String>,
        kind: EventKind,
        evidence_digest: Option<Digest>,
    ) {
        self.revision += 1;
        self.events.push(EpisodeEvent {
            revision: self.revision,
            step_id,
            attempt_id,
            kind,
            evidence_digest,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(value: u8) -> Digest {
        Digest([value; 32])
    }

    fn compensation() -> CompensationSpec {
        CompensationSpec {
            target_id: "host-a".into(),
            operation_id: "nixward.recover".into(),
            request_digest: digest(3),
            binding_digest: digest(13),
            expected_postcondition_digest: digest(23),
        }
    }

    fn plan() -> ServicePlan {
        ServicePlan {
            service_id: "payments-api".into(),
            plan_digest: digest(100),
            expected_service_digest: digest(90),
            steps: vec![
                OperationSpec {
                    id: "host-repair".into(),
                    target_id: "host-a".into(),
                    operation_id: "nixward.apply".into(),
                    request_digest: digest(1),
                    binding_digest: digest(11),
                    expected_postcondition_digest: digest(21),
                    dependencies: vec![],
                    required: true,
                    compensation: Some(compensation()),
                },
                OperationSpec {
                    id: "ticket-update".into(),
                    target_id: "itsm-case-42".into(),
                    operation_id: "itsm.update_case".into(),
                    request_digest: digest(2),
                    binding_digest: digest(12),
                    expected_postcondition_digest: digest(22),
                    dependencies: vec!["host-repair".into()],
                    required: true,
                    compensation: None,
                },
            ],
        }
    }

    fn episode() -> SagaEpisode {
        SagaEpisode::new(plan(), "agent-runtime").expect("valid test plan")
    }

    fn grant(
        step_id: &str,
        is_compensation: bool,
        grant_id: &str,
        valid_until_ms: u64,
    ) -> AuthorityGrant {
        if is_compensation {
            let spec = compensation();
            AuthorityGrant {
                grant_id: grant_id.into(),
                plan_digest: digest(100),
                step_id: step_id.into(),
                target_id: spec.target_id,
                operation_id: spec.operation_id,
                request_digest: spec.request_digest,
                binding_digest: spec.binding_digest,
                valid_until_ms,
            }
        } else {
            let spec = plan()
                .steps
                .into_iter()
                .find(|step| step.id == step_id)
                .expect("test step exists");
            AuthorityGrant {
                grant_id: grant_id.into(),
                plan_digest: digest(100),
                step_id: step_id.into(),
                target_id: spec.target_id,
                operation_id: spec.operation_id,
                request_digest: spec.request_digest,
                binding_digest: spec.binding_digest,
                valid_until_ms,
            }
        }
    }

    fn observer(
        attempt_id: &str,
        step_id: &str,
        effect: ObservedEffect,
        consistency: Consistency,
        postcondition_digest: Option<Digest>,
    ) -> ObserverEvidence {
        let spec = plan()
            .steps
            .into_iter()
            .find(|step| step.id == step_id)
            .expect("test step exists");
        ObserverEvidence {
            observer_id: "independent-host-probe".into(),
            trust_domain_id: "external-observer".into(),
            attempt_id: attempt_id.into(),
            target_id: spec.target_id,
            binding_digest: spec.binding_digest,
            effect,
            consistency,
            postcondition_digest,
            no_effect_basis: None,
            observed_at_ms: 50,
            valid_until_ms: 200,
            evidence_digest: digest(70),
        }
    }

    fn authorize_and_dispatch_host(ep: &mut SagaEpisode) {
        ep.authorize_step(
            ep.revision(),
            "host-repair",
            &grant("host-repair", false, "grant-host", 10_000),
            1,
        )
        .expect("host grant");
        ep.persist_dispatch_intent(ep.revision(), "host-repair", "attempt-host-1", "idem-host")
            .expect("dispatch intent");
    }

    fn verify_host(ep: &mut SagaEpisode) {
        authorize_and_dispatch_host(ep);
        ep.record_observation(
            ep.revision(),
            "host-repair",
            &observer(
                "attempt-host-1",
                "host-repair",
                ObservedEffect::ExpectedPostcondition,
                Consistency::Strong,
                Some(digest(21)),
            ),
            100,
        )
        .expect("host postcondition");
    }

    #[test]
    fn plan_accepts_dag_and_rejects_dependency_cycle() {
        assert!(plan().validate().is_ok());
        let mut cyclic = plan();
        cyclic.steps[0].dependencies.push("ticket-update".into());
        assert_eq!(cyclic.validate(), Err(PlanValidationError::DependencyCycle));
    }

    #[test]
    fn plan_rejects_duplicate_step_ids_and_missing_dependencies() {
        let mut duplicate = plan();
        duplicate.steps[1].id = duplicate.steps[0].id.clone();
        assert!(matches!(
            duplicate.validate(),
            Err(PlanValidationError::DuplicateStepId(_))
        ));

        let mut missing = plan();
        missing.steps[1].dependencies = vec!["absent".into()];
        assert!(matches!(
            missing.validate(),
            Err(PlanValidationError::MissingDependency { .. })
        ));
    }

    #[test]
    fn stale_revision_cannot_append_a_transition() {
        let mut ep = episode();
        assert_eq!(
            ep.authorize_step(
                1,
                "host-repair",
                &grant("host-repair", false, "grant-host", 10_000),
                1
            ),
            Err(TransitionError::StaleRevision {
                expected: 1,
                actual: 0
            })
        );
        assert_eq!(ep.revision(), 0);
        assert!(ep.events().is_empty());
    }

    #[test]
    fn authorization_binds_exact_target_operation_and_request() {
        let mut ep = episode();
        let mut altered = grant("host-repair", false, "grant-host", 10_000);
        altered.target_id = "host-b".into();
        assert_eq!(
            ep.authorize_step(0, "host-repair", &altered, 1),
            Err(TransitionError::InvalidGrantBinding)
        );

        let mut altered = grant("host-repair", false, "grant-host-2", 10_000);
        altered.request_digest = digest(99);
        assert_eq!(
            ep.authorize_step(0, "host-repair", &altered, 1),
            Err(TransitionError::InvalidGrantBinding)
        );
    }

    #[test]
    fn expired_grant_is_rejected() {
        let mut ep = episode();
        assert_eq!(
            ep.authorize_step(
                0,
                "host-repair",
                &grant("host-repair", false, "grant-host", 1),
                1
            ),
            Err(TransitionError::ExpiredGrant)
        );
    }

    #[test]
    fn dependent_step_waits_for_verified_dependency() {
        let mut ep = episode();
        assert_eq!(
            ep.authorize_step(
                0,
                "ticket-update",
                &grant("ticket-update", false, "grant-ticket", 10_000),
                1
            ),
            Err(TransitionError::DependencyNotVerified("host-repair".into()))
        );
    }

    #[test]
    fn dispatch_ambiguity_cannot_be_retried_without_strong_no_effect_evidence() {
        let mut ep = episode();
        authorize_and_dispatch_host(&mut ep);
        ep.mark_outcome_unknown(ep.revision(), "host-repair")
            .expect("ambiguous result");

        assert_eq!(
            ep.authorize_step(
                ep.revision(),
                "host-repair",
                &grant("host-repair", false, "grant-host-2", 10_000),
                2
            ),
            Err(TransitionError::InvalidTransition(
                "forward authorization requires Pending or strong NoEffectConfirmed".into()
            ))
        );

        ep.record_observation(
            ep.revision(),
            "host-repair",
            &observer(
                "attempt-host-1",
                "host-repair",
                ObservedEffect::NoEffect,
                Consistency::Eventual,
                None,
            ),
            100,
        )
        .expect("eventual absence stays ambiguous");
        assert!(matches!(
            ep.step_status("host-repair"),
            Some(StepStatus::Indeterminate { .. })
        ));

        let mut readback_only = observer(
            "attempt-host-1",
            "host-repair",
            ObservedEffect::NoEffect,
            Consistency::Strong,
            None,
        );
        readback_only.no_effect_basis = Some(NoEffectBasis::StateReadbackOnly);
        ep.record_observation(ep.revision(), "host-repair", &readback_only, 100)
            .expect("unchanged state alone remains insufficient");
        assert!(matches!(
            ep.step_status("host-repair"),
            Some(StepStatus::Indeterminate { .. })
        ));

        let mut ledger_proof = observer(
            "attempt-host-1",
            "host-repair",
            ObservedEffect::NoEffect,
            Consistency::Strong,
            None,
        );
        ledger_proof.no_effect_basis = Some(NoEffectBasis::AuthoritativeAttemptLedgerNeverAccepted);
        ep.record_observation(ep.revision(), "host-repair", &ledger_proof, 100)
            .expect("attempt-specific never-accepted proof permits reconciliation");
        assert!(matches!(
            ep.step_status("host-repair"),
            Some(StepStatus::NoEffectConfirmed { .. })
        ));

        ep.authorize_step(
            ep.revision(),
            "host-repair",
            &grant("host-repair", false, "grant-host-2", 10_000),
            3,
        )
        .expect("new grant after strong no-effect evidence");
        ep.persist_dispatch_intent(ep.revision(), "host-repair", "attempt-host-2", "idem-host")
            .expect("new attempt with stable idempotency key");
        assert!(matches!(
            ep.step_status("host-repair"),
            Some(StepStatus::DispatchIntentPersisted { attempt_id, .. })
                if attempt_id == "attempt-host-2"
        ));
    }

    #[test]
    fn duplicate_attempt_ids_are_rejected_across_steps() {
        let mut ep = episode();
        verify_host(&mut ep);
        ep.authorize_step(
            ep.revision(),
            "ticket-update",
            &grant("ticket-update", false, "grant-ticket", 10_000),
            2,
        )
        .expect("ticket grant");
        assert_eq!(
            ep.persist_dispatch_intent(
                ep.revision(),
                "ticket-update",
                "attempt-host-1",
                "idem-ticket"
            ),
            Err(TransitionError::DuplicateAttempt("attempt-host-1".into()))
        );
    }

    #[test]
    fn same_agent_trust_domain_cannot_verify_a_step() {
        let mut ep = episode();
        authorize_and_dispatch_host(&mut ep);
        let mut evidence = observer(
            "attempt-host-1",
            "host-repair",
            ObservedEffect::ExpectedPostcondition,
            Consistency::Strong,
            Some(digest(21)),
        );
        evidence.trust_domain_id = "agent-runtime".into();
        assert_eq!(
            ep.record_observation(ep.revision(), "host-repair", &evidence, 100),
            Err(TransitionError::InvalidObserver)
        );
    }

    #[test]
    fn forbidden_effect_is_a_non_compensable_safety_violation_state() {
        let mut ep = episode();
        authorize_and_dispatch_host(&mut ep);
        ep.record_observation(
            ep.revision(),
            "host-repair",
            &observer(
                "attempt-host-1",
                "host-repair",
                ObservedEffect::ForbiddenEffect,
                Consistency::Strong,
                None,
            ),
            100,
        )
        .expect("forbidden effect recorded");
        assert_eq!(
            ep.disposition(),
            EpisodeDisposition::SecurityViolationObserved
        );
        assert_eq!(
            ep.authorize_compensation(
                ep.revision(),
                "host-repair",
                &grant("host-repair", true, "comp-grant", 10_000),
                2
            ),
            Err(TransitionError::InvalidTransition(
                "compensation requires a verified forward postcondition".into()
            ))
        );
    }

    #[test]
    fn cancellation_blocks_future_steps_but_preserves_inflight_uncertainty() {
        let mut ep = episode();
        authorize_and_dispatch_host(&mut ep);
        ep.request_cancellation(ep.revision())
            .expect("cancellation recorded");
        assert!(matches!(
            ep.step_status("host-repair"),
            Some(StepStatus::OutcomeUnknown { .. })
        ));
        assert_eq!(ep.step_status("ticket-update"), Some(&StepStatus::Blocked));
        assert_eq!(ep.disposition(), EpisodeDisposition::Indeterminate);

        assert_eq!(
            ep.authorize_step(
                ep.revision(),
                "ticket-update",
                &grant("ticket-update", false, "grant-ticket", 10_000),
                2
            ),
            Err(TransitionError::Cancelled)
        );
    }

    #[test]
    fn compensation_requires_separate_authority_and_external_verification() {
        let mut ep = episode();
        verify_host(&mut ep);

        let forward_grant = grant("host-repair", false, "grant-forward", 10_000);
        assert_eq!(
            ep.authorize_compensation(ep.revision(), "host-repair", &forward_grant, 2),
            Err(TransitionError::InvalidGrantBinding)
        );

        ep.authorize_compensation(
            ep.revision(),
            "host-repair",
            &grant("host-repair", true, "grant-compensation", 10_000),
            2,
        )
        .expect("separate compensation grant");
        ep.persist_compensation_intent(ep.revision(), "host-repair", "attempt-comp-1", "idem-comp")
            .expect("compensation write-ahead intent");
        ep.mark_compensation_outcome_unknown(ep.revision(), "host-repair")
            .expect("ambiguous compensation");
        assert_eq!(ep.disposition(), EpisodeDisposition::Indeterminate);

        let spec = compensation();
        let evidence = ObserverEvidence {
            observer_id: "external-recovery-probe".into(),
            trust_domain_id: "external-observer".into(),
            attempt_id: "attempt-comp-1".into(),
            target_id: spec.target_id,
            binding_digest: spec.binding_digest,
            effect: ObservedEffect::ExpectedPostcondition,
            consistency: Consistency::Strong,
            postcondition_digest: Some(spec.expected_postcondition_digest),
            no_effect_basis: None,
            observed_at_ms: 50,
            valid_until_ms: 200,
            evidence_digest: digest(71),
        };
        ep.record_compensation_observation(ep.revision(), "host-repair", &evidence, 100)
            .expect("compensation independently observed");
        assert!(matches!(
            ep.step_status("host-repair"),
            Some(StepStatus::Compensated { .. })
        ));
        assert_eq!(ep.disposition(), EpisodeDisposition::EscalationRequired);
    }

    #[test]
    fn successful_steps_are_not_service_resolution() {
        let mut ep = episode();
        verify_host(&mut ep);
        ep.authorize_step(
            ep.revision(),
            "ticket-update",
            &grant("ticket-update", false, "grant-ticket", 10_000),
            2,
        )
        .expect("ticket grant");
        ep.persist_dispatch_intent(
            ep.revision(),
            "ticket-update",
            "attempt-ticket-1",
            "idem-ticket",
        )
        .expect("ticket dispatch intent");
        let ticket_evidence = ObserverEvidence {
            observer_id: "itsm-readback".into(),
            trust_domain_id: "itsm-audit".into(),
            attempt_id: "attempt-ticket-1".into(),
            target_id: "itsm-case-42".into(),
            binding_digest: digest(12),
            effect: ObservedEffect::ExpectedPostcondition,
            consistency: Consistency::Strong,
            postcondition_digest: Some(digest(22)),
            no_effect_basis: None,
            observed_at_ms: 50,
            valid_until_ms: 200,
            evidence_digest: digest(72),
        };
        ep.record_observation(ep.revision(), "ticket-update", &ticket_evidence, 100)
            .expect("ticket postcondition");
        assert_eq!(
            ep.disposition(),
            EpisodeDisposition::AwaitingServiceVerification
        );

        let service_evidence = ServiceObservation {
            observer_id: "external-http-probe".into(),
            trust_domain_id: "independent-probe".into(),
            service_id: "payments-api".into(),
            plan_digest: digest(100),
            health: ServiceHealth::Healthy,
            consistency: Consistency::Strong,
            observed_service_digest: Some(digest(90)),
            observed_at_ms: 50,
            valid_until_ms: 200,
            evidence_digest: digest(73),
        };
        ep.verify_service(ep.revision(), &service_evidence, 100)
            .expect("independent service check");
        assert_eq!(ep.disposition(), EpisodeDisposition::Resolved);
        assert_eq!(
            ep.authorize_step(
                ep.revision(),
                "host-repair",
                &grant("host-repair", false, "late-grant", 10_000),
                103,
            ),
            Err(TransitionError::AlreadyResolved)
        );
        assert_eq!(
            ep.request_cancellation(ep.revision()),
            Err(TransitionError::AlreadyResolved)
        );
        assert_eq!(ep.revision(), ep.events().last().expect("events").revision);
    }

    #[test]
    fn service_resolution_rejects_agent_owned_oracle_and_missing_steps() {
        let mut ep = episode();
        verify_host(&mut ep);
        let evidence = ServiceObservation {
            observer_id: "agent-self-report".into(),
            trust_domain_id: "agent-runtime".into(),
            service_id: "payments-api".into(),
            plan_digest: digest(100),
            health: ServiceHealth::Healthy,
            consistency: Consistency::Strong,
            observed_service_digest: Some(digest(90)),
            observed_at_ms: 50,
            valid_until_ms: 200,
            evidence_digest: digest(80),
        };
        assert_eq!(
            ep.verify_service(ep.revision(), &evidence, 100),
            Err(TransitionError::ServicePreconditionsNotMet)
        );

        ep.authorize_step(
            ep.revision(),
            "ticket-update",
            &grant("ticket-update", false, "grant-ticket", 10_000),
            2,
        )
        .expect("ticket grant");
        ep.persist_dispatch_intent(
            ep.revision(),
            "ticket-update",
            "attempt-ticket-1",
            "idem-ticket",
        )
        .expect("ticket dispatch intent");
        let ticket_evidence = ObserverEvidence {
            observer_id: "itsm-readback".into(),
            trust_domain_id: "itsm-audit".into(),
            attempt_id: "attempt-ticket-1".into(),
            target_id: "itsm-case-42".into(),
            binding_digest: digest(12),
            effect: ObservedEffect::ExpectedPostcondition,
            consistency: Consistency::Strong,
            postcondition_digest: Some(digest(22)),
            no_effect_basis: None,
            observed_at_ms: 50,
            valid_until_ms: 200,
            evidence_digest: digest(72),
        };
        ep.record_observation(ep.revision(), "ticket-update", &ticket_evidence, 100)
            .expect("ticket verified");

        assert_eq!(
            ep.verify_service(ep.revision(), &evidence, 100),
            Err(TransitionError::InvalidObserver)
        );
    }

    #[test]
    fn only_matching_independent_service_digest_resolves_episode() {
        let mut ep = episode();
        verify_host(&mut ep);
        ep.authorize_step(
            ep.revision(),
            "ticket-update",
            &grant("ticket-update", false, "grant-ticket", 10_000),
            2,
        )
        .expect("ticket grant");
        ep.persist_dispatch_intent(
            ep.revision(),
            "ticket-update",
            "attempt-ticket-1",
            "idem-ticket",
        )
        .expect("ticket dispatch intent");
        let ticket_evidence = ObserverEvidence {
            observer_id: "itsm-readback".into(),
            trust_domain_id: "itsm-audit".into(),
            attempt_id: "attempt-ticket-1".into(),
            target_id: "itsm-case-42".into(),
            binding_digest: digest(12),
            effect: ObservedEffect::ExpectedPostcondition,
            consistency: Consistency::Strong,
            postcondition_digest: Some(digest(22)),
            no_effect_basis: None,
            observed_at_ms: 50,
            valid_until_ms: 200,
            evidence_digest: digest(72),
        };
        ep.record_observation(ep.revision(), "ticket-update", &ticket_evidence, 100)
            .expect("ticket postcondition");

        let service_evidence = ServiceObservation {
            observer_id: "external-http-probe".into(),
            trust_domain_id: "independent-probe".into(),
            service_id: "payments-api".into(),
            plan_digest: digest(100),
            health: ServiceHealth::Healthy,
            consistency: Consistency::Strong,
            observed_service_digest: Some(digest(91)),
            observed_at_ms: 50,
            valid_until_ms: 200,
            evidence_digest: digest(73),
        };
        ep.verify_service(ep.revision(), &service_evidence, 100)
            .expect("mismatch remains indeterminate");
        assert_eq!(ep.disposition(), EpisodeDisposition::Indeterminate);

        let fresh_service_evidence = ServiceObservation {
            observer_id: "external-http-probe".into(),
            trust_domain_id: "independent-probe".into(),
            service_id: "payments-api".into(),
            plan_digest: digest(100),
            health: ServiceHealth::Healthy,
            consistency: Consistency::Strong,
            observed_service_digest: Some(digest(90)),
            observed_at_ms: 101,
            valid_until_ms: 250,
            evidence_digest: digest(74),
        };
        ep.verify_service(ep.revision(), &fresh_service_evidence, 102)
            .expect("fresh exact service observation");
        assert_eq!(ep.disposition(), EpisodeDisposition::Resolved);
    }
    #[test]
    fn stale_step_evidence_and_wrong_plan_service_evidence_are_rejected() {
        let mut ep = episode();
        authorize_and_dispatch_host(&mut ep);

        let mut stale = observer(
            "attempt-host-1",
            "host-repair",
            ObservedEffect::ExpectedPostcondition,
            Consistency::Strong,
            Some(digest(21)),
        );
        stale.valid_until_ms = 99;
        assert_eq!(
            ep.record_observation(ep.revision(), "host-repair", &stale, 100),
            Err(TransitionError::StaleEvidence)
        );
        assert!(matches!(
            ep.step_status("host-repair"),
            Some(StepStatus::DispatchIntentPersisted { .. })
        ));

        let fresh = observer(
            "attempt-host-1",
            "host-repair",
            ObservedEffect::ExpectedPostcondition,
            Consistency::Strong,
            Some(digest(21)),
        );
        ep.record_observation(ep.revision(), "host-repair", &fresh, 100)
            .expect("fresh step observation");
        ep.authorize_step(
            ep.revision(),
            "ticket-update",
            &grant("ticket-update", false, "grant-ticket", 10_000),
            101,
        )
        .expect("ticket grant");
        ep.persist_dispatch_intent(
            ep.revision(),
            "ticket-update",
            "attempt-ticket-1",
            "idem-ticket",
        )
        .expect("ticket dispatch intent");
        let ticket_evidence = ObserverEvidence {
            observer_id: "itsm-readback".into(),
            trust_domain_id: "itsm-audit".into(),
            attempt_id: "attempt-ticket-1".into(),
            target_id: "itsm-case-42".into(),
            binding_digest: digest(12),
            effect: ObservedEffect::ExpectedPostcondition,
            consistency: Consistency::Strong,
            postcondition_digest: Some(digest(22)),
            no_effect_basis: None,
            observed_at_ms: 50,
            valid_until_ms: 200,
            evidence_digest: digest(72),
        };
        ep.record_observation(ep.revision(), "ticket-update", &ticket_evidence, 102)
            .expect("ticket postcondition");

        let wrong_plan = ServiceObservation {
            observer_id: "external-http-probe".into(),
            trust_domain_id: "independent-probe".into(),
            service_id: "payments-api".into(),
            plan_digest: digest(99),
            health: ServiceHealth::Healthy,
            consistency: Consistency::Strong,
            observed_service_digest: Some(digest(90)),
            observed_at_ms: 50,
            valid_until_ms: 200,
            evidence_digest: digest(73),
        };
        assert_eq!(
            ep.verify_service(ep.revision(), &wrong_plan, 103),
            Err(TransitionError::InvalidObserver)
        );

        let stale_global = ServiceObservation {
            plan_digest: digest(100),
            valid_until_ms: 99,
            ..wrong_plan
        };
        assert_eq!(
            ep.verify_service(ep.revision(), &stale_global, 103),
            Err(TransitionError::StaleEvidence)
        );
        assert_eq!(
            ep.disposition(),
            EpisodeDisposition::AwaitingServiceVerification
        );
    }

    #[test]
    fn global_service_gate_waits_for_authorized_optional_steps_to_settle() {
        let mut optional_plan = plan();
        optional_plan.steps[1].required = false;
        let mut ep =
            SagaEpisode::new(optional_plan, "agent-runtime").expect("valid optional-step plan");
        verify_host(&mut ep);
        ep.authorize_step(
            ep.revision(),
            "ticket-update",
            &grant("ticket-update", false, "optional-ticket-grant", 10_000),
            101,
        )
        .expect("optional participant authorized");

        let service_evidence = ServiceObservation {
            observer_id: "external-http-probe".into(),
            trust_domain_id: "independent-probe".into(),
            service_id: "payments-api".into(),
            plan_digest: digest(100),
            health: ServiceHealth::Healthy,
            consistency: Consistency::Strong,
            observed_service_digest: Some(digest(90)),
            observed_at_ms: 50,
            valid_until_ms: 200,
            evidence_digest: digest(73),
        };
        assert_eq!(
            ep.verify_service(ep.revision(), &service_evidence, 102),
            Err(TransitionError::ServicePreconditionsNotMet)
        );
        assert_eq!(ep.disposition(), EpisodeDisposition::PartiallyComplete);
    }

    #[test]
    fn compensation_may_reuse_connector_operation_with_distinct_request_binding() {
        let mut candidate = plan();
        let forward_operation = candidate.steps[0].operation_id.clone();
        {
            let compensation = candidate.steps[0]
                .compensation
                .as_mut()
                .expect("compensation exists");
            compensation.operation_id = forward_operation;
        }
        assert!(candidate.validate().is_ok());

        let forward_binding = candidate.steps[0].binding_digest;
        candidate.steps[0]
            .compensation
            .as_mut()
            .expect("compensation exists")
            .binding_digest = forward_binding;
        assert!(matches!(
            candidate.validate(),
            Err(PlanValidationError::InvalidCompensation(_))
        ));
    }

    #[test]
    fn revision_exhaustion_fails_closed_before_mutating_state() {
        let mut ep = episode();
        ep.revision = u64::MAX;
        assert_eq!(
            ep.authorize_step(
                u64::MAX,
                "host-repair",
                &grant("host-repair", false, "grant-host", 10_000),
                1
            ),
            Err(TransitionError::RevisionExhausted)
        );
        assert!(ep.events().is_empty());
        assert_eq!(ep.step_status("host-repair"), Some(&StepStatus::Pending));
    }
}
