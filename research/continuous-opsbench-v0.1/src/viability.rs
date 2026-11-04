//! Finite-horizon robust viability extension for the predictive safety filter.
//!
//! This evaluator synthesizes only over a declared finite state/action/model/
//! disturbance universe. It cannot authorize, dispatch or commit a fallback.
//! Every selected fallback remains subject to the existing effect broker and
//! participant-side topology CAS.

use crate::safety_filter::{
    appraise_one_step, transition, CandidateAct, CandidateOperation, CapacityUncertainty,
    DemandDisturbance, FilterDisposition, OperationalState, SafetyAppraisal, SafetyEnvelope,
    SafetyFilterInput, Successor, PPM,
};
use crate::Digest;
use std::collections::{BTreeMap, BTreeSet};

pub const VIABILITY_SCHEMA: &str = "continuous-opsbench-predictive-viability-v0.1";
pub const MAX_VIABILITY_STATES: usize = 512;
pub const MAX_FALLBACK_ACTIONS: usize = 32;
pub const MAX_VIABILITY_HORIZON: u32 = 16;
pub const MAX_VIABILITY_TRANSITION_EVALUATIONS: usize = 1_000_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViabilityRequest {
    pub base: SafetyFilterInput,
    pub state_space_digest: Digest,
    pub fallback_actions_digest: Digest,
    /// Complete finite universe for all safe successors reachable in this run.
    pub state_space: Vec<OperationalState>,
    /// Candidate control choices used only in the viability calculation.
    pub fallback_actions: Vec<CandidateOperation>,
    /// Number of future transitions for which a robust fallback must exist.
    pub horizon: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViabilityDisposition {
    PreservesViability,
    BreaksViability,
    DeferForEvidence,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ViabilityFinding {
    ImmediateSafetyRejected,
    ImmediateSafetyIndeterminate,
    MissingFallbackActions,
    InvalidFallbackAction,
    DuplicateFallbackAction,
    EmptyOrDuplicateStateSpace,
    StateSpaceLimitExceeded,
    HorizonOutOfRange,
    InvalidStateSpace,
    TransitionEvaluationLimitExceeded,
    SafeSuccessorOutsideStateSpace,
    CandidateSuccessorNotViable,
    ArithmeticOverflow,
    NoViableStates,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViabilityAppraisal {
    pub schema: String,
    pub disposition: ViabilityDisposition,
    pub run_id: String,
    pub scenario_id: String,
    pub arm_id: String,
    pub action_id: String,
    pub action_digest: Digest,
    pub tenant_id: String,
    pub participant_id: String,
    pub target: String,
    pub topology_epoch: u64,
    pub topology_digest: Digest,
    pub observation_id: String,
    pub observation_digest: Digest,
    pub model_revision: String,
    pub model_digest: Digest,
    pub uncertainty_set_digest: Digest,
    pub disturbance_set_digest: Digest,
    pub envelope_digest: Digest,
    pub state_space_digest: Digest,
    pub fallback_actions_digest: Digest,
    pub appraisal_at_ms: u64,
    pub valid_until_ms: Option<u64>,
    pub horizon: u32,
    /// Includes K_0 and each computed K_n through K_H.
    pub kernel_sizes_by_depth: Vec<usize>,
    pub candidate_successors_evaluated: usize,
    pub viability_transition_evaluations: usize,
    pub findings: Vec<ViabilityFinding>,
    /// Preserves the one-step appraisal that this result extends.
    pub one_step_appraisal: SafetyAppraisal,
}

fn nonzero_digest(digest: &Digest) -> bool {
    digest.0.iter().any(|byte| *byte != 0)
}

fn state_is_safe(state: &OperationalState, envelope: &SafetyEnvelope) -> bool {
    let total_queue = u64::from(state.critical_queue) + u64::from(state.bulk_queue);
    let effective_capacity = state.api_capacity.min(state.database_capacity);
    let spare_after_backlog = u64::from(effective_capacity).saturating_sub(total_queue);
    state.api_capacity > 0
        && state.database_capacity > 0
        && state.critical_queue <= envelope.maximum_critical_queue
        && state.bulk_queue <= envelope.maximum_bulk_queue
        && total_queue <= envelope.maximum_total_queue
        && state.api_capacity <= envelope.maximum_api_capacity
        && state.database_capacity <= envelope.maximum_database_capacity
        && state.retry_multiplier_ppm > 0
        && state.retry_multiplier_ppm <= envelope.maximum_retry_multiplier_ppm
        && spare_after_backlog >= u64::from(envelope.minimum_spare_capacity)
}

fn successor_is_safe(successor: &Successor, envelope: &SafetyEnvelope) -> bool {
    let total_queue =
        u64::from(successor.critical_queue) + u64::from(successor.bulk_queue);
    successor.critical_queue <= envelope.maximum_critical_queue
        && successor.bulk_queue <= envelope.maximum_bulk_queue
        && total_queue <= envelope.maximum_total_queue
        && successor.api_capacity > 0
        && successor.api_capacity <= envelope.maximum_api_capacity
        && successor.database_capacity > 0
        && successor.database_capacity <= envelope.maximum_database_capacity
        && successor.retry_multiplier_ppm > 0
        && successor.retry_multiplier_ppm <= envelope.maximum_retry_multiplier_ppm
        && successor.spare_capacity >= envelope.minimum_spare_capacity
}

fn successor_state(successor: &Successor) -> OperationalState {
    OperationalState {
        critical_queue: successor.critical_queue,
        bulk_queue: successor.bulk_queue,
        api_capacity: successor.api_capacity,
        database_capacity: successor.database_capacity,
        retry_multiplier_ppm: successor.retry_multiplier_ppm,
    }
}

fn valid_operation(operation: &CandidateOperation, envelope: &SafetyEnvelope) -> bool {
    match operation {
        CandidateOperation::Noop => true,
        CandidateOperation::SetApiCapacity(value) => {
            *value > 0 && *value <= envelope.maximum_api_capacity
        }
        CandidateOperation::SetRetryMultiplierPpm(value) => {
            *value > 0 && *value <= envelope.maximum_retry_multiplier_ppm
        }
    }
}

fn base_result(
    request: &ViabilityRequest,
    candidate: &CandidateAct,
    one_step: SafetyAppraisal,
    disposition: ViabilityDisposition,
    findings: Vec<ViabilityFinding>,
) -> ViabilityAppraisal {
    ViabilityAppraisal {
        schema: VIABILITY_SCHEMA.to_owned(),
        disposition,
        run_id: request.base.run_id.clone(),
        scenario_id: request.base.scenario_id.clone(),
        arm_id: request.base.arm_id.clone(),
        action_id: candidate.action_id.clone(),
        action_digest: candidate.action_digest.clone(),
        tenant_id: candidate.tenant_id.clone(),
        participant_id: candidate.participant_id.clone(),
        target: candidate.target.clone(),
        topology_epoch: request.base.topology_epoch,
        topology_digest: request.base.topology_digest.clone(),
        observation_id: request.base.observation_id.clone(),
        observation_digest: request.base.observation_digest.clone(),
        model_revision: request.base.model_revision.clone(),
        model_digest: request.base.model_digest.clone(),
        uncertainty_set_digest: request.base.uncertainty_set_digest.clone(),
        disturbance_set_digest: request.base.disturbance_set_digest.clone(),
        envelope_digest: request.base.envelope_digest.clone(),
        state_space_digest: request.state_space_digest.clone(),
        fallback_actions_digest: request.fallback_actions_digest.clone(),
        appraisal_at_ms: request.base.appraisal_at_ms,
        valid_until_ms: one_step.valid_until_ms,
        horizon: request.horizon,
        kernel_sizes_by_depth: Vec::new(),
        candidate_successors_evaluated: 0,
        viability_transition_evaluations: 0,
        findings,
        one_step_appraisal: one_step,
    }
}

fn defer(
    request: &ViabilityRequest,
    candidate: &CandidateAct,
    one_step: SafetyAppraisal,
    finding: ViabilityFinding,
) -> ViabilityAppraisal {
    base_result(
        request,
        candidate,
        one_step,
        ViabilityDisposition::DeferForEvidence,
        vec![finding],
    )
}

fn break_viability(
    request: &ViabilityRequest,
    candidate: &CandidateAct,
    one_step: SafetyAppraisal,
    finding: ViabilityFinding,
) -> ViabilityAppraisal {
    base_result(
        request,
        candidate,
        one_step,
        ViabilityDisposition::BreaksViability,
        vec![finding],
    )
}

fn bounded_transition(
    state: &OperationalState,
    operation: &CandidateOperation,
    models: &[CapacityUncertainty],
    disturbances: &[DemandDisturbance],
    envelope: &SafetyEnvelope,
    state_indexes: &BTreeMap<OperationalState, usize>,
    prior_kernel: &[bool],
    evaluations: &mut usize,
) -> Result<bool, ViabilityFinding> {
    for model in models {
        for disturbance in disturbances {
            *evaluations = evaluations
                .checked_add(1)
                .ok_or(ViabilityFinding::ArithmeticOverflow)?;
            if *evaluations > MAX_VIABILITY_TRANSITION_EVALUATIONS {
                return Err(ViabilityFinding::TransitionEvaluationLimitExceeded);
            }
            let successor = transition(state, model, disturbance, operation)
                .ok_or(ViabilityFinding::ArithmeticOverflow)?;
            if !successor_is_safe(&successor, envelope) {
                return Ok(false);
            }
            let next_state = successor_state(&successor);
            let next_index = state_indexes
                .get(&next_state)
                .ok_or(ViabilityFinding::SafeSuccessorOutsideStateSpace)?;
            if !prior_kernel[*next_index] {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

/// Compute K_0..K_H by exhaustive robust predecessor iteration, then require
/// every possible immediate candidate successor to belong to K_H.
///
/// A successful result means only "preserves viability in this declared finite
/// model"; it is not an authorization token or a production stability proof.
pub fn appraise_with_viability(
    request: &ViabilityRequest,
    candidate: &CandidateAct,
) -> ViabilityAppraisal {
    let one_step = appraise_one_step(&request.base, candidate);
    if one_step.disposition == FilterDisposition::DeferForEvidence {
        return base_result(
            request,
            candidate,
            one_step,
            ViabilityDisposition::DeferForEvidence,
            vec![ViabilityFinding::ImmediateSafetyIndeterminate],
        );
    }
    if one_step.disposition == FilterDisposition::RejectUnsafe {
        return base_result(
            request,
            candidate,
            one_step,
            ViabilityDisposition::BreaksViability,
            vec![ViabilityFinding::ImmediateSafetyRejected],
        );
    }

    if request.horizon == 0 || request.horizon > MAX_VIABILITY_HORIZON {
        return defer(
            request,
            candidate,
            one_step,
            ViabilityFinding::HorizonOutOfRange,
        );
    }
    if request.state_space.len() > MAX_VIABILITY_STATES {
        return defer(
            request,
            candidate,
            one_step,
            ViabilityFinding::StateSpaceLimitExceeded,
        );
    }
    if request.state_space.is_empty()
        || !nonzero_digest(&request.state_space_digest)
        || !nonzero_digest(&request.fallback_actions_digest)
    {
        return defer(
            request,
            candidate,
            one_step,
            ViabilityFinding::EmptyOrDuplicateStateSpace,
        );
    }
    if request.fallback_actions.is_empty()
        || request.fallback_actions.len() > MAX_FALLBACK_ACTIONS
    {
        return defer(
            request,
            candidate,
            one_step,
            ViabilityFinding::MissingFallbackActions,
        );
    }

    let unique_states: BTreeSet<OperationalState> =
        request.state_space.iter().cloned().collect();
    if unique_states.len() != request.state_space.len() {
        return defer(
            request,
            candidate,
            one_step,
            ViabilityFinding::EmptyOrDuplicateStateSpace,
        );
    }
    let unique_actions: BTreeSet<CandidateOperation> =
        request.fallback_actions.iter().cloned().collect();
    if unique_actions.len() != request.fallback_actions.len() {
        return defer(
            request,
            candidate,
            one_step,
            ViabilityFinding::DuplicateFallbackAction,
        );
    }
    if request
        .fallback_actions
        .iter()
        .any(|action| !valid_operation(action, &request.base.envelope))
    {
        return defer(
            request,
            candidate,
            one_step,
            ViabilityFinding::InvalidFallbackAction,
        );
    }

    // Capacity/model values above the envelope mean the finite universe is
    // inconsistent with its pinned model contract. Queue-over-limit states may
    // be included as explicit degraded states, but they are outside K_0.
    if request.state_space.iter().any(|state| {
        state.api_capacity > request.base.envelope.maximum_api_capacity
            || state.database_capacity > request.base.envelope.maximum_database_capacity
            || state.retry_multiplier_ppm > request.base.envelope.maximum_retry_multiplier_ppm
    }) {
        return defer(
            request,
            candidate,
            one_step,
            ViabilityFinding::InvalidStateSpace,
        );
    }

    let mut state_indexes = BTreeMap::new();
    for (index, state) in request.state_space.iter().cloned().enumerate() {
        state_indexes.insert(state, index);
    }

    let mut kernel: Vec<bool> = request
        .state_space
        .iter()
        .map(|state| state_is_safe(state, &request.base.envelope))
        .collect();
    let mut kernel_sizes_by_depth = vec![kernel.iter().filter(|member| **member).count()];
    if kernel_sizes_by_depth[0] == 0 {
        return defer(
            request,
            candidate,
            one_step,
            ViabilityFinding::NoViableStates,
        );
    }

    let mut evaluations = 0usize;
    for _depth in 1..=request.horizon {
        let mut predecessor = vec![false; request.state_space.len()];
        for (state_index, state) in request.state_space.iter().enumerate() {
            if !kernel[state_index] {
                continue;
            }
            for fallback in &request.fallback_actions {
                let robust = match bounded_transition(
                    state,
                    fallback,
                    &request.base.capacity_uncertainty,
                    &request.base.disturbances,
                    &request.base.envelope,
                    &state_indexes,
                    &kernel,
                    &mut evaluations,
                ) {
                    Ok(value) => value,
                    Err(finding) => return defer(request, candidate, one_step, finding),
                };
                if robust {
                    predecessor[state_index] = true;
                    break;
                }
            }
        }
        kernel = predecessor;
        kernel_sizes_by_depth.push(kernel.iter().filter(|member| **member).count());
        if kernel_sizes_by_depth.last() == Some(&0) {
            break;
        }
    }

    let mut candidate_successors_evaluated = 0usize;
    for state in &request.base.state_belief {
        for model in &request.base.capacity_uncertainty {
            for disturbance in &request.base.disturbances {
                let successor = match transition(state, model, disturbance, &candidate.operation) {
                    Some(next) => next,
                    None => {
                        return defer(
                            request,
                            candidate,
                            one_step,
                            ViabilityFinding::ArithmeticOverflow,
                        );
                    }
                };
                candidate_successors_evaluated += 1;
                if !successor_is_safe(&successor, &request.base.envelope) {
                    return break_viability(
                        request,
                        candidate,
                        one_step,
                        ViabilityFinding::ImmediateSafetyRejected,
                    );
                }
                let next_state = successor_state(&successor);
                let next_index = match state_indexes.get(&next_state) {
                    Some(index) => *index,
                    None => {
                        return defer(
                            request,
                            candidate,
                            one_step,
                            ViabilityFinding::SafeSuccessorOutsideStateSpace,
                        );
                    }
                };
                if !kernel[next_index] {
                    let mut result = break_viability(
                        request,
                        candidate,
                        one_step,
                        ViabilityFinding::CandidateSuccessorNotViable,
                    );
                    result.kernel_sizes_by_depth = kernel_sizes_by_depth;
                    result.candidate_successors_evaluated = candidate_successors_evaluated;
                    result.viability_transition_evaluations = evaluations;
                    return result;
                }
            }
        }
    }

    let mut result = base_result(
        request,
        candidate,
        one_step,
        ViabilityDisposition::PreservesViability,
        Vec::new(),
    );
    result.kernel_sizes_by_depth = kernel_sizes_by_depth;
    result.candidate_successors_evaluated = candidate_successors_evaluated;
    result.viability_transition_evaluations = evaluations;
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::safety_filter::{
        CandidateAct, CandidateOperation, CapacityUncertainty, DemandDisturbance,
        OperationalState, SafetyEnvelope, SafetyFilterInput,
    };

    fn d(byte: u8) -> Digest {
        Digest::from_byte(byte)
    }

    fn input() -> SafetyFilterInput {
        SafetyFilterInput {
            run_id: "run-viability".into(),
            scenario_id: "scenario-viability".into(),
            arm_id: "candidate".into(),
            observation_id: "observation-1".into(),
            observation_source_id: "independent-observer".into(),
            observation_digest: d(1),
            observed_at_ms: 100,
            appraisal_at_ms: 110,
            maximum_observation_age_ms: 100,
            observation_clock_uncertainty_ms: 0,
            receipt_validity_ms: 50,
            topology_epoch: 4,
            topology_digest: d(2),
            model_revision: "two-workload-v1".into(),
            model_digest: d(3),
            uncertainty_set_digest: d(4),
            disturbance_set_digest: d(5),
            envelope_digest: d(6),
            state_belief: vec![OperationalState {
                critical_queue: 0,
                bulk_queue: 0,
                api_capacity: 1,
                database_capacity: 1,
                retry_multiplier_ppm: 1_000_000,
            }],
            capacity_uncertainty: vec![CapacityUncertainty {
                api_capacity_loss: 0,
                database_capacity_loss: 0,
            }],
            disturbances: vec![DemandDisturbance {
                critical_demand: 0,
                bulk_demand: 1,
            }],
            envelope: SafetyEnvelope {
                maximum_critical_queue: 0,
                maximum_bulk_queue: 1,
                maximum_total_queue: 1,
                maximum_api_capacity: 1,
                maximum_database_capacity: 1,
                maximum_retry_multiplier_ppm: 2_000_000,
                minimum_spare_capacity: 0,
            },
        }
    }

    fn candidate(operation: CandidateOperation) -> CandidateAct {
        CandidateAct {
            action_id: "candidate-action".into(),
            tenant_id: "tenant-a".into(),
            participant_id: "participant-a".into(),
            target: "api-service".into(),
            action_digest: d(7),
            operation,
        }
    }

    fn state(queue: u32, retry: u32) -> OperationalState {
        OperationalState {
            critical_queue: 0,
            bulk_queue: queue,
            api_capacity: 1,
            database_capacity: 1,
            retry_multiplier_ppm: retry,
        }
    }

    fn request(fallback_actions: Vec<CandidateOperation>) -> ViabilityRequest {
        ViabilityRequest {
            base: input(),
            state_space_digest: d(8),
            fallback_actions_digest: d(9),
            state_space: vec![state(1, 2_000_000), state(1, 1_000_000)],
            fallback_actions,
            horizon: 1,
        }
    }

    #[test]
    fn immediate_safe_action_is_rejected_when_it_enters_a_non_viable_state() {
        let request = request(vec![CandidateOperation::Noop]);
        let result = appraise_with_viability(
            &request,
            &candidate(CandidateOperation::SetRetryMultiplierPpm(2_000_000)),
        );
        assert_eq!(result.one_step_appraisal.disposition, FilterDisposition::AdmitForAuthorization);
        assert_eq!(result.disposition, ViabilityDisposition::BreaksViability);
        assert!(result.findings.contains(&ViabilityFinding::CandidateSuccessorNotViable));
        assert_eq!(result.kernel_sizes_by_depth, vec![2, 1]);
    }

    #[test]
    fn robustly_reducing_retry_multiplier_preserves_viability() {
        let request = request(vec![CandidateOperation::SetRetryMultiplierPpm(1_000_000)]);
        let result = appraise_with_viability(
            &request,
            &candidate(CandidateOperation::SetRetryMultiplierPpm(2_000_000)),
        );
        assert_eq!(result.disposition, ViabilityDisposition::PreservesViability);
        assert_eq!(result.kernel_sizes_by_depth, vec![2, 2]);
        assert_eq!(result.candidate_successors_evaluated, 1);
    }

    #[test]
    fn fallback_safe_under_nominal_model_but_unsafe_under_uncertainty_is_not_robust() {
        let mut request = request(vec![CandidateOperation::Noop]);
        let state_at = |bulk_queue, retry_multiplier_ppm| OperationalState {
            critical_queue: 0,
            bulk_queue,
            api_capacity: 2,
            database_capacity: 2,
            retry_multiplier_ppm,
        };
        request.base.state_belief = vec![state_at(0, 1_000_000)];
        request.base.envelope.maximum_api_capacity = 2;
        request.base.envelope.maximum_database_capacity = 2;
        request.base.capacity_uncertainty.push(CapacityUncertainty {
            api_capacity_loss: 1,
            database_capacity_loss: 1,
        });
        request.state_space = vec![
            state_at(0, 2_000_000),
            state_at(1, 2_000_000),
            state_at(1, 1_000_000),
            state_at(0, 1_000_000),
        ];

        let result = appraise_with_viability(
            &request,
            &candidate(CandidateOperation::SetRetryMultiplierPpm(2_000_000)),
        );
        assert_eq!(
            result.one_step_appraisal.disposition,
            FilterDisposition::AdmitForAuthorization
        );
        assert_eq!(result.disposition, ViabilityDisposition::BreaksViability);
        assert!(result.findings.contains(&ViabilityFinding::CandidateSuccessorNotViable));
        // The nominal no-op path would remain in K_0; the capacity-loss
        // parameter creates a successor whose next no-op transition violates
        // the bulk queue envelope, so the fallback is not robust.
        assert_eq!(result.kernel_sizes_by_depth, vec![4, 3]);
    }

    #[test]
    fn safe_successor_missing_from_state_universe_defers() {
        let mut request = request(vec![CandidateOperation::SetRetryMultiplierPpm(1_000_000)]);
        request.state_space.remove(0);
        let result = appraise_with_viability(
            &request,
            &candidate(CandidateOperation::SetRetryMultiplierPpm(2_000_000)),
        );
        assert_eq!(result.disposition, ViabilityDisposition::DeferForEvidence);
        assert!(result.findings.contains(&ViabilityFinding::SafeSuccessorOutsideStateSpace));
    }

    #[test]
    fn duplicate_state_or_fallback_entries_defer() {
        let mut duplicate_state = request(vec![CandidateOperation::Noop]);
        duplicate_state.state_space.push(duplicate_state.state_space[0].clone());
        let state_result = appraise_with_viability(
            &duplicate_state,
            &candidate(CandidateOperation::SetRetryMultiplierPpm(2_000_000)),
        );
        assert_eq!(state_result.disposition, ViabilityDisposition::DeferForEvidence);

        let duplicate_action = request(vec![
            CandidateOperation::Noop,
            CandidateOperation::Noop,
        ]);
        let action_result = appraise_with_viability(
            &duplicate_action,
            &candidate(CandidateOperation::SetRetryMultiplierPpm(2_000_000)),
        );
        assert_eq!(action_result.disposition, ViabilityDisposition::DeferForEvidence);
        assert!(action_result.findings.contains(&ViabilityFinding::DuplicateFallbackAction));
    }

    #[test]
    fn clock_uncertainty_reduces_freshness_and_receipt_lifetime() {
        let mut base = input();
        base.observation_clock_uncertainty_ms = 10;
        base.maximum_observation_age_ms = 30;
        let mut request = request(vec![CandidateOperation::SetRetryMultiplierPpm(1_000_000)]);
        request.base = base;
        // Candidate demand leads to the zero-queue successor; include that
        // reachable state so this test measures freshness, not model coverage.
        request.state_space.push(state(0, 1_000_000));
        let result = appraise_with_viability(
            &request,
            &candidate(CandidateOperation::SetRetryMultiplierPpm(1_000_000)),
        );
        assert_eq!(result.disposition, ViabilityDisposition::PreservesViability);
        assert_eq!(result.valid_until_ms, Some(120));
    }

    #[test]
    fn exact_freshness_boundary_defers() {
        let mut base = input();
        base.maximum_observation_age_ms = 20;
        base.observation_clock_uncertainty_ms = 10;
        let request = ViabilityRequest { base, ..request(vec![CandidateOperation::SetRetryMultiplierPpm(1_000_000)]) };
        let result = appraise_with_viability(
            &request,
            &candidate(CandidateOperation::SetRetryMultiplierPpm(1_000_000)),
        );
        assert_eq!(result.disposition, ViabilityDisposition::DeferForEvidence);
    }

    #[test]
    fn horizon_and_missing_fallback_catalog_fail_closed() {
        let mut no_fallback = request(Vec::new());
        let result = appraise_with_viability(
            &no_fallback,
            &candidate(CandidateOperation::SetRetryMultiplierPpm(1_000_000)),
        );
        assert_eq!(result.disposition, ViabilityDisposition::DeferForEvidence);
        assert!(result.findings.contains(&ViabilityFinding::MissingFallbackActions));

        no_fallback.fallback_actions = vec![CandidateOperation::Noop];
        no_fallback.horizon = MAX_VIABILITY_HORIZON + 1;
        let result = appraise_with_viability(
            &no_fallback,
            &candidate(CandidateOperation::SetRetryMultiplierPpm(1_000_000)),
        );
        assert_eq!(result.disposition, ViabilityDisposition::DeferForEvidence);
        assert!(result.findings.contains(&ViabilityFinding::HorizonOutOfRange));
    }

    // Independent reference implementation uses direct arithmetic and its own
    // predecessor construction; it does not call appraise_with_viability,
    // bounded_transition, transition, or successor_is_safe.
    fn reference_viability(
        states: &[OperationalState],
        fallbacks: &[CandidateOperation],
        models: &[CapacityUncertainty],
        disturbances: &[DemandDisturbance],
        envelope: &SafetyEnvelope,
        horizon: u32,
    ) -> Vec<bool> {
        let safe = |s: &OperationalState| {
            let q = u64::from(s.critical_queue) + u64::from(s.bulk_queue);
            let cap = u64::from(s.api_capacity.min(s.database_capacity));
            s.api_capacity > 0
                && s.database_capacity > 0
                && s.critical_queue <= envelope.maximum_critical_queue
                && s.bulk_queue <= envelope.maximum_bulk_queue
                && q <= envelope.maximum_total_queue
                && s.api_capacity <= envelope.maximum_api_capacity
                && s.database_capacity <= envelope.maximum_database_capacity
                && s.retry_multiplier_ppm > 0
                && s.retry_multiplier_ppm <= envelope.maximum_retry_multiplier_ppm
                && cap.saturating_sub(q) >= u64::from(envelope.minimum_spare_capacity)
        };
        let mut kernel: Vec<bool> = states.iter().map(safe).collect();
        for _ in 0..horizon {
            let mut next = vec![false; states.len()];
            for (i, state) in states.iter().enumerate() {
                if !kernel[i] {
                    continue;
                }
                for fallback in fallbacks {
                    let mut all_inside = true;
                    for model in models {
                        for disturbance in disturbances {
                            let (api, retry) = match fallback {
                                CandidateOperation::Noop => (state.api_capacity, state.retry_multiplier_ppm),
                                CandidateOperation::SetApiCapacity(v) => (*v, state.retry_multiplier_ppm),
                                CandidateOperation::SetRetryMultiplierPpm(v) => (state.api_capacity, *v),
                            };
                            let eff_api = api.saturating_sub(model.api_capacity_loss);
                            let eff_db = state.database_capacity.saturating_sub(model.database_capacity_loss);
                            let capacity = eff_api.min(eff_db);
                            let bulk_numerator = u64::from(disturbance.bulk_demand) * u64::from(retry);
                            let bulk_arrivals = (bulk_numerator + (PPM - 1)) / PPM;
                            let critical_pending = u64::from(state.critical_queue) + u64::from(disturbance.critical_demand);
                            let bulk_pending = u64::from(state.bulk_queue) + bulk_arrivals;
                            let critical_served = critical_pending.min(u64::from(capacity));
                            let bulk_served = bulk_pending.min(u64::from(capacity) - critical_served);
                            let critical_left = critical_pending - critical_served;
                            let bulk_left = bulk_pending - bulk_served;
                            let spare = u64::from(capacity) - critical_served - bulk_served;
                            if critical_left > u64::from(envelope.maximum_critical_queue)
                                || bulk_left > u64::from(envelope.maximum_bulk_queue)
                                || critical_left + bulk_left > envelope.maximum_total_queue
                                || api == 0
                                || api > envelope.maximum_api_capacity
                                || state.database_capacity == 0
                                || state.database_capacity > envelope.maximum_database_capacity
                                || retry == 0
                                || retry > envelope.maximum_retry_multiplier_ppm
                                || spare < u64::from(envelope.minimum_spare_capacity)
                            {
                                all_inside = false;
                                break;
                            }
                            let successor = OperationalState {
                                critical_queue: critical_left as u32,
                                bulk_queue: bulk_left as u32,
                                api_capacity: api,
                                database_capacity: state.database_capacity,
                                retry_multiplier_ppm: retry,
                            };
                            let Some(next_index) = states.iter().position(|known| *known == successor) else {
                                all_inside = false;
                                break;
                            };
                            if !kernel[next_index] {
                                all_inside = false;
                                break;
                            }
                        }
                        if !all_inside { break; }
                    }
                    if all_inside {
                        next[i] = true;
                        break;
                    }
                }
            }
            kernel = next;
        }
        kernel
    }

    #[test]
    fn kernel_matches_independent_reference_over_small_action_state_matrix() {
        let request = request(vec![
            CandidateOperation::Noop,
            CandidateOperation::SetRetryMultiplierPpm(1_000_000),
        ]);
        let reference = reference_viability(
            &request.state_space,
            &request.fallback_actions,
            &request.base.capacity_uncertainty,
            &request.base.disturbances,
            &request.base.envelope,
            request.horizon,
        );
        let request_result = appraise_with_viability(
            &request,
            &candidate(CandidateOperation::SetRetryMultiplierPpm(2_000_000)),
        );
        let all_candidate_successors_viable = request_result.disposition
            == ViabilityDisposition::PreservesViability;
        let candidate_state = state(1, 2_000_000);
        let candidate_index = request.state_space.iter().position(|known| *known == candidate_state).unwrap();
        assert_eq!(all_candidate_successors_viable, reference[candidate_index]);
    }
}
