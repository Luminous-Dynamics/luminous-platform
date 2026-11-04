//! Conservative one-step operational safety filter.
//!
//! This is a pure appraisal function over a finite, explicitly supplied belief,
//! model-uncertainty and disturbance set. It has no effect port and cannot
//! authorize or execute an action. A positive result means only that every
//! enumerated successor passes this versioned one-step envelope.

use crate::Digest;
use std::collections::BTreeSet;

pub const SAFETY_FILTER_SCHEMA: &str = "continuous-opsbench-predictive-safety-filter-v0.1";
pub const PPM: u64 = 1_000_000;
pub const MAX_ENUMERATED_COMBINATIONS: usize = 1_000_000;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct OperationalState {
    pub critical_queue: u32,
    pub bulk_queue: u32,
    pub api_capacity: u32,
    pub database_capacity: u32,
    pub retry_multiplier_ppm: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CapacityUncertainty {
    pub api_capacity_loss: u32,
    pub database_capacity_loss: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DemandDisturbance {
    pub critical_demand: u32,
    pub bulk_demand: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SafetyEnvelope {
    pub maximum_critical_queue: u32,
    pub maximum_bulk_queue: u32,
    pub maximum_total_queue: u64,
    pub maximum_api_capacity: u32,
    pub maximum_database_capacity: u32,
    pub maximum_retry_multiplier_ppm: u32,
    pub minimum_spare_capacity: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CandidateOperation {
    Noop,
    SetApiCapacity(u32),
    SetRetryMultiplierPpm(u32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CandidateAct {
    pub action_id: String,
    pub tenant_id: String,
    pub participant_id: String,
    pub target: String,
    pub action_digest: Digest,
    pub operation: CandidateOperation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SafetyFilterInput {
    pub run_id: String,
    pub scenario_id: String,
    pub arm_id: String,
    pub observation_id: String,
    pub observation_source_id: String,
    pub observation_digest: Digest,
    pub observed_at_ms: u64,
    pub appraisal_at_ms: u64,
    pub maximum_observation_age_ms: u64,
    pub receipt_validity_ms: u64,
    pub topology_epoch: u64,
    pub topology_digest: Digest,
    pub model_revision: String,
    pub model_digest: Digest,
    pub uncertainty_set_digest: Digest,
    pub disturbance_set_digest: Digest,
    pub envelope_digest: Digest,
    /// All states consistent with the permitted observation model.
    pub state_belief: Vec<OperationalState>,
    /// Bounded losses from nominal API/database capacities.
    pub capacity_uncertainty: Vec<CapacityUncertainty>,
    /// Bounded exogenous demand arriving during the next plant transition.
    pub disturbances: Vec<DemandDisturbance>,
    pub envelope: SafetyEnvelope,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterDisposition {
    AdmitForAuthorization,
    RejectUnsafe,
    DeferForEvidence,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SafetyFinding {
    CriticalQueueLimit,
    BulkQueueLimit,
    TotalQueueLimit,
    ApiCapacityLimit,
    DatabaseCapacityLimit,
    RetryMultiplierLimit,
    SpareCapacityFloor,
    InvalidCandidateAction,
    IncompleteEvidence,
    StaleObservation,
    InvalidInputSet,
    EnumerationLimitExceeded,
    ArithmeticOverflow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SafetyAppraisal {
    pub schema: String,
    pub disposition: FilterDisposition,
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
    pub appraisal_at_ms: u64,
    pub valid_until_ms: Option<u64>,
    pub evaluated_combinations: usize,
    /// Minimum signed distance to any declared invariant over enumerated successors.
    /// Negative means at least one declared invariant was violated.
    pub minimum_invariant_margin: Option<i64>,
    pub findings: Vec<SafetyFinding>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Successor {
    critical_queue: u32,
    bulk_queue: u32,
    api_capacity: u32,
    database_capacity: u32,
    retry_multiplier_ppm: u32,
    spare_capacity: u32,
}

fn nonzero_digest(digest: &Digest) -> bool {
    digest.0.iter().any(|byte| *byte != 0)
}

fn unique<T: Ord + Clone>(items: &[T]) -> bool {
    let set: BTreeSet<T> = items.iter().cloned().collect();
    set.len() == items.len()
}

fn base_appraisal(
    input: &SafetyFilterInput,
    action: &CandidateAct,
    disposition: FilterDisposition,
    valid_until_ms: Option<u64>,
    findings: Vec<SafetyFinding>,
) -> SafetyAppraisal {
    SafetyAppraisal {
        schema: SAFETY_FILTER_SCHEMA.to_owned(),
        disposition,
        run_id: input.run_id.clone(),
        scenario_id: input.scenario_id.clone(),
        arm_id: input.arm_id.clone(),
        action_id: action.action_id.clone(),
        action_digest: action.action_digest.clone(),
        tenant_id: action.tenant_id.clone(),
        participant_id: action.participant_id.clone(),
        target: action.target.clone(),
        topology_epoch: input.topology_epoch,
        topology_digest: input.topology_digest.clone(),
        observation_id: input.observation_id.clone(),
        observation_digest: input.observation_digest.clone(),
        model_revision: input.model_revision.clone(),
        model_digest: input.model_digest.clone(),
        uncertainty_set_digest: input.uncertainty_set_digest.clone(),
        disturbance_set_digest: input.disturbance_set_digest.clone(),
        envelope_digest: input.envelope_digest.clone(),
        appraisal_at_ms: input.appraisal_at_ms,
        valid_until_ms,
        evaluated_combinations: 0,
        minimum_invariant_margin: None,
        findings,
    }
}

fn defer(
    input: &SafetyFilterInput,
    action: &CandidateAct,
    finding: SafetyFinding,
) -> SafetyAppraisal {
    base_appraisal(
        input,
        action,
        FilterDisposition::DeferForEvidence,
        None,
        vec![finding],
    )
}

fn invalid_identity(input: &SafetyFilterInput, action: &CandidateAct) -> bool {
    [
        input.run_id.as_str(),
        input.scenario_id.as_str(),
        input.arm_id.as_str(),
        input.observation_id.as_str(),
        input.observation_source_id.as_str(),
        input.model_revision.as_str(),
        action.action_id.as_str(),
        action.tenant_id.as_str(),
        action.participant_id.as_str(),
        action.target.as_str(),
    ]
    .iter()
    .any(|value| value.is_empty())
}

fn transition(
    state: &OperationalState,
    model: &CapacityUncertainty,
    disturbance: &DemandDisturbance,
    action: &CandidateOperation,
) -> Option<Successor> {
    let (api_capacity, retry_multiplier_ppm) = match action {
        CandidateOperation::Noop => (state.api_capacity, state.retry_multiplier_ppm),
        CandidateOperation::SetApiCapacity(value) => (*value, state.retry_multiplier_ppm),
        CandidateOperation::SetRetryMultiplierPpm(value) => (state.api_capacity, *value),
    };

    let effective_api = api_capacity.saturating_sub(model.api_capacity_loss);
    let effective_database =
        state.database_capacity.saturating_sub(model.database_capacity_loss);
    let effective_capacity = effective_api.min(effective_database);

    // Round retry-amplified bulk arrivals upward. Rounding down could admit an
    // action by discarding a fractional request from the conservative envelope.
    let amplified_bulk = u64::from(disturbance.bulk_demand)
        .checked_mul(u64::from(retry_multiplier_ppm))?
        .checked_add(PPM - 1)?
        .checked_div(PPM)?;
    let amplified_bulk = u32::try_from(amplified_bulk).ok()?;

    let critical_work = state.critical_queue.checked_add(disturbance.critical_demand)?;
    let bulk_work = state.bulk_queue.checked_add(amplified_bulk)?;

    // Critical workload receives strict priority. Bulk service uses only the
    // capacity left after critical work; unsatisfied work remains queued.
    let critical_served = critical_work.min(effective_capacity);
    let capacity_after_critical = effective_capacity - critical_served;
    let bulk_served = bulk_work.min(capacity_after_critical);

    let critical_queue = critical_work - critical_served;
    let bulk_queue = bulk_work - bulk_served;
    let total_served = critical_served.checked_add(bulk_served)?;
    let spare_capacity = effective_capacity - total_served;

    Some(Successor {
        critical_queue,
        bulk_queue,
        api_capacity,
        database_capacity: state.database_capacity,
        retry_multiplier_ppm,
        spare_capacity,
    })
}

fn margin(limit: u64, observed: u64) -> i64 {
    // Both inputs originate as u32/u64 bounded contract fields. Saturating
    // conversion preserves a conservative negative margin on extreme values.
    if limit >= observed {
        i64::try_from(limit - observed).unwrap_or(i64::MAX)
    } else {
        -i64::try_from(observed - limit).unwrap_or(i64::MAX)
    }
}

/// Appraise one exact candidate action against every state/model/disturbance
/// tuple in the supplied finite sets. This function is pure: it cannot dispatch,
/// mutate the plant, or create an authorization/effect receipt.
///
/// Horizon is exactly one transition. A future multi-step viability kernel must
/// be separately implemented and must not relabel this result as a stability proof.
pub fn appraise_one_step(
    input: &SafetyFilterInput,
    action: &CandidateAct,
) -> SafetyAppraisal {
    if invalid_identity(input, action)
        || !nonzero_digest(&action.action_digest)
        || !nonzero_digest(&input.observation_digest)
        || !nonzero_digest(&input.topology_digest)
        || !nonzero_digest(&input.model_digest)
        || !nonzero_digest(&input.uncertainty_set_digest)
        || !nonzero_digest(&input.disturbance_set_digest)
        || !nonzero_digest(&input.envelope_digest)
        || input.state_belief.is_empty()
        || input.capacity_uncertainty.is_empty()
        || input.disturbances.is_empty()
        || input.receipt_validity_ms == 0
        || input.envelope.maximum_total_queue == 0
        || input.envelope.maximum_api_capacity == 0
        || input.envelope.maximum_database_capacity == 0
        || input.envelope.maximum_retry_multiplier_ppm == 0
        || !unique(&input.state_belief)
        || !unique(&input.capacity_uncertainty)
        || !unique(&input.disturbances)
    {
        return defer(input, action, SafetyFinding::IncompleteEvidence);
    }

    let age = match input.appraisal_at_ms.checked_sub(input.observed_at_ms) {
        Some(age) => age,
        None => return defer(input, action, SafetyFinding::StaleObservation),
    };
    if age > input.maximum_observation_age_ms {
        return defer(input, action, SafetyFinding::StaleObservation);
    }

    let valid_until_ms = match input.appraisal_at_ms.checked_add(input.receipt_validity_ms) {
        Some(expiry) => Some(expiry),
        None => return defer(input, action, SafetyFinding::IncompleteEvidence),
    };

    if let CandidateOperation::SetApiCapacity(value) = action.operation {
        if value == 0 || value > input.envelope.maximum_api_capacity {
            return base_appraisal(
                input,
                action,
                FilterDisposition::RejectUnsafe,
                valid_until_ms,
                vec![SafetyFinding::InvalidCandidateAction],
            );
        }
    }
    if let CandidateOperation::SetRetryMultiplierPpm(value) = action.operation {
        if value == 0 || value > input.envelope.maximum_retry_multiplier_ppm {
            return base_appraisal(
                input,
                action,
                FilterDisposition::RejectUnsafe,
                valid_until_ms,
                vec![SafetyFinding::InvalidCandidateAction],
            );
        }
    }

    let combinations = match input.state_belief.len()
        .checked_mul(input.capacity_uncertainty.len())
        .and_then(|n| n.checked_mul(input.disturbances.len()))
    {
        Some(count) if count <= MAX_ENUMERATED_COMBINATIONS => count,
        _ => return defer(input, action, SafetyFinding::EnumerationLimitExceeded),
    };

    if input.state_belief.iter().any(|state| {
        state.api_capacity > input.envelope.maximum_api_capacity
            || state.database_capacity > input.envelope.maximum_database_capacity
            || state.retry_multiplier_ppm == 0
            || state.retry_multiplier_ppm > input.envelope.maximum_retry_multiplier_ppm
    }) {
        return defer(input, action, SafetyFinding::InvalidInputSet);
    }

    let mut findings = BTreeSet::new();
    let mut minimum_margin: Option<i64> = None;
    let mut evaluated = 0usize;

    for state in &input.state_belief {
        for model in &input.capacity_uncertainty {
            for disturbance in &input.disturbances {
                let successor = match transition(state, model, disturbance, &action.operation) {
                    Some(next) => next,
                    None => {
                        return defer(input, action, SafetyFinding::ArithmeticOverflow);
                    }
                };
                evaluated += 1;

                let total_queue = match u64::from(successor.critical_queue)
                    .checked_add(u64::from(successor.bulk_queue))
                {
                    Some(total) => total,
                    None => return defer(input, action, SafetyFinding::ArithmeticOverflow),
                };

                let margins = [
                    margin(
                        u64::from(input.envelope.maximum_critical_queue),
                        u64::from(successor.critical_queue),
                    ),
                    margin(
                        u64::from(input.envelope.maximum_bulk_queue),
                        u64::from(successor.bulk_queue),
                    ),
                    margin(input.envelope.maximum_total_queue, total_queue),
                    margin(
                        u64::from(input.envelope.maximum_api_capacity),
                        u64::from(successor.api_capacity),
                    ),
                    margin(
                        u64::from(input.envelope.maximum_database_capacity),
                        u64::from(successor.database_capacity),
                    ),
                    margin(
                        u64::from(input.envelope.maximum_retry_multiplier_ppm),
                        u64::from(successor.retry_multiplier_ppm),
                    ),
                    margin(
                        u64::from(successor.spare_capacity),
                        u64::from(input.envelope.minimum_spare_capacity),
                    ),
                ];
                for value in margins {
                    minimum_margin = Some(minimum_margin.map_or(value, |current| current.min(value)));
                }

                if successor.critical_queue > input.envelope.maximum_critical_queue {
                    findings.insert(SafetyFinding::CriticalQueueLimit);
                }
                if successor.bulk_queue > input.envelope.maximum_bulk_queue {
                    findings.insert(SafetyFinding::BulkQueueLimit);
                }
                if total_queue > input.envelope.maximum_total_queue {
                    findings.insert(SafetyFinding::TotalQueueLimit);
                }
                if successor.api_capacity > input.envelope.maximum_api_capacity {
                    findings.insert(SafetyFinding::ApiCapacityLimit);
                }
                if successor.database_capacity > input.envelope.maximum_database_capacity {
                    findings.insert(SafetyFinding::DatabaseCapacityLimit);
                }
                if successor.retry_multiplier_ppm > input.envelope.maximum_retry_multiplier_ppm {
                    findings.insert(SafetyFinding::RetryMultiplierLimit);
                }
                if successor.spare_capacity < input.envelope.minimum_spare_capacity {
                    findings.insert(SafetyFinding::SpareCapacityFloor);
                }
            }
        }
    }

    let disposition = if findings.is_empty() {
        FilterDisposition::AdmitForAuthorization
    } else {
        FilterDisposition::RejectUnsafe
    };

    let mut result = base_appraisal(
        input,
        action,
        disposition,
        valid_until_ms,
        findings.into_iter().collect(),
    );
    result.evaluated_combinations = evaluated;
    result.minimum_invariant_margin = minimum_margin;
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(value: u8) -> Digest {
        Digest::from_byte(value)
    }

    fn state() -> OperationalState {
        OperationalState {
            critical_queue: 0,
            bulk_queue: 0,
            api_capacity: 10,
            database_capacity: 10,
            retry_multiplier_ppm: 1_000_000,
        }
    }

    fn input() -> SafetyFilterInput {
        SafetyFilterInput {
            run_id: "run-1".into(),
            scenario_id: "scenario-1".into(),
            arm_id: "candidate".into(),
            observation_id: "obs-1".into(),
            observation_source_id: "observer-a".into(),
            observation_digest: digest(1),
            observed_at_ms: 100,
            appraisal_at_ms: 110,
            maximum_observation_age_ms: 20,
            receipt_validity_ms: 50,
            topology_epoch: 7,
            topology_digest: digest(2),
            model_revision: "plant-v1".into(),
            model_digest: digest(3),
            uncertainty_set_digest: digest(4),
            disturbance_set_digest: digest(5),
            envelope_digest: digest(6),
            state_belief: vec![state()],
            capacity_uncertainty: vec![CapacityUncertainty {
                api_capacity_loss: 0,
                database_capacity_loss: 0,
            }],
            disturbances: vec![DemandDisturbance {
                critical_demand: 2,
                bulk_demand: 3,
            }],
            envelope: SafetyEnvelope {
                maximum_critical_queue: 0,
                maximum_bulk_queue: 0,
                maximum_total_queue: 0,
                maximum_api_capacity: 10,
                maximum_database_capacity: 10,
                maximum_retry_multiplier_ppm: 1_000_000,
                minimum_spare_capacity: 5,
            },
        }
    }

    fn act(operation: CandidateOperation) -> CandidateAct {
        CandidateAct {
            action_id: "action-1".into(),
            tenant_id: "tenant-1".into(),
            participant_id: "participant-1".into(),
            target: "api-service".into(),
            action_digest: digest(7),
            operation,
        }
    }

    #[test]
    fn safe_action_is_admitted_for_authorization_only() {
        let result = appraise_one_step(&input(), &act(CandidateOperation::Noop));
        assert_eq!(result.disposition, FilterDisposition::AdmitForAuthorization);
        assert_eq!(result.evaluated_combinations, 1);
        assert_eq!(result.minimum_invariant_margin, Some(0));
        assert_eq!(result.valid_until_ms, Some(160));
    }

    #[test]
    fn capacity_reduction_that_starves_bulk_work_is_rejected() {
        let result = appraise_one_step(
            &input(),
            &act(CandidateOperation::SetApiCapacity(2)),
        );
        assert_eq!(result.disposition, FilterDisposition::RejectUnsafe);
        assert!(result.findings.contains(&SafetyFinding::BulkQueueLimit));
        assert!(result.findings.contains(&SafetyFinding::SpareCapacityFloor));
    }

    #[test]
    fn every_model_and_disturbance_member_must_be_safe() {
        let mut evidence = input();
        evidence.capacity_uncertainty.push(CapacityUncertainty {
            api_capacity_loss: 8,
            database_capacity_loss: 8,
        });
        let result = appraise_one_step(&evidence, &act(CandidateOperation::Noop));
        assert_eq!(result.disposition, FilterDisposition::RejectUnsafe);
        assert!(result.findings.contains(&SafetyFinding::BulkQueueLimit));
    }

    #[test]
    fn stale_or_future_observation_defers_instead_of_admitting() {
        let mut evidence = input();
        evidence.appraisal_at_ms = 121;
        let stale = appraise_one_step(&evidence, &act(CandidateOperation::Noop));
        assert_eq!(stale.disposition, FilterDisposition::DeferForEvidence);
        assert!(stale.findings.contains(&SafetyFinding::StaleObservation));

        evidence.appraisal_at_ms = 99;
        let future = appraise_one_step(&evidence, &act(CandidateOperation::Noop));
        assert_eq!(future.disposition, FilterDisposition::DeferForEvidence);
    }

    #[test]
    fn missing_uncertainty_or_duplicate_set_members_defer() {
        let mut evidence = input();
        evidence.capacity_uncertainty.clear();
        let missing = appraise_one_step(&evidence, &act(CandidateOperation::Noop));
        assert_eq!(missing.disposition, FilterDisposition::DeferForEvidence);

        let mut evidence = input();
        evidence.state_belief.push(state());
        let duplicate = appraise_one_step(&evidence, &act(CandidateOperation::Noop));
        assert_eq!(duplicate.disposition, FilterDisposition::DeferForEvidence);
    }

    #[test]
    fn arithmetic_overflow_is_fail_closed() {
        let mut evidence = input();
        evidence.state_belief[0].critical_queue = u32::MAX;
        evidence.disturbances[0].critical_demand = 1;
        let result = appraise_one_step(&evidence, &act(CandidateOperation::Noop));
        assert_eq!(result.disposition, FilterDisposition::DeferForEvidence);
        assert!(result.findings.contains(&SafetyFinding::ArithmeticOverflow));
    }

    #[test]
    fn excessive_retry_multiplier_is_rejected_before_transition() {
        let result = appraise_one_step(
            &input(),
            &act(CandidateOperation::SetRetryMultiplierPpm(1_500_000)),
        );
        assert_eq!(result.disposition, FilterDisposition::RejectUnsafe);
        assert!(result.findings.contains(&SafetyFinding::InvalidCandidateAction));
    }

    // Deliberately separate test oracle. It does not call the filter's transition
    // helper or margin code; it directly enumerates the small integer model.
    fn exhaustive_oracle_safe(
        evidence: &SafetyFilterInput,
        action: &CandidateOperation,
    ) -> bool {
        for state in &evidence.state_belief {
            for model in &evidence.capacity_uncertainty {
                for disturbance in &evidence.disturbances {
                    let (api, retry) = match action {
                        CandidateOperation::Noop => (state.api_capacity, state.retry_multiplier_ppm),
                        CandidateOperation::SetApiCapacity(v) => (*v, state.retry_multiplier_ppm),
                        CandidateOperation::SetRetryMultiplierPpm(v) => (state.api_capacity, *v),
                    };
                    let api_after_loss = api.saturating_sub(model.api_capacity_loss);
                    let db_after_loss = state.database_capacity
                        .saturating_sub(model.database_capacity_loss);
                    let capacity = api_after_loss.min(db_after_loss);
                    let bulk_product = u64::from(disturbance.bulk_demand) * u64::from(retry);
                    let bulk_arrivals = (bulk_product + (PPM - 1)) / PPM;
                    if bulk_arrivals > u64::from(u32::MAX) {
                        return false;
                    }
                    let critical_pending = match state.critical_queue.checked_add(disturbance.critical_demand) {
                        Some(v) => v,
                        None => return false,
                    };
                    let bulk_pending = match state.bulk_queue.checked_add(bulk_arrivals as u32) {
                        Some(v) => v,
                        None => return false,
                    };
                    let critical_done = critical_pending.min(capacity);
                    let bulk_done = bulk_pending.min(capacity - critical_done);
                    let critical_left = critical_pending - critical_done;
                    let bulk_left = bulk_pending - bulk_done;
                    let total_left = u64::from(critical_left) + u64::from(bulk_left);
                    let spare = capacity - critical_done - bulk_done;

                    if critical_left > evidence.envelope.maximum_critical_queue
                        || bulk_left > evidence.envelope.maximum_bulk_queue
                        || total_left > evidence.envelope.maximum_total_queue
                        || api > evidence.envelope.maximum_api_capacity
                        || state.database_capacity > evidence.envelope.maximum_database_capacity
                        || retry > evidence.envelope.maximum_retry_multiplier_ppm
                        || spare < evidence.envelope.minimum_spare_capacity
                    {
                        return false;
                    }
                }
            }
        }
        true
    }

    #[test]
    fn filter_matches_independent_exhaustive_oracle_over_small_action_matrix() {
        let mut evidence = input();
        evidence.state_belief = vec![
            state(),
            OperationalState {
                critical_queue: 1,
                bulk_queue: 0,
                api_capacity: 11,
                database_capacity: 11,
                retry_multiplier_ppm: 1_000_000,
            },
        ];
        evidence.envelope.maximum_api_capacity = 12;
        evidence.envelope.maximum_database_capacity = 12;
        evidence.envelope.maximum_critical_queue = 2;
        evidence.envelope.maximum_bulk_queue = 3;
        evidence.envelope.maximum_total_queue = 5;
        evidence.envelope.minimum_spare_capacity = 0;
        evidence.capacity_uncertainty = vec![
            CapacityUncertainty { api_capacity_loss: 0, database_capacity_loss: 0 },
            CapacityUncertainty { api_capacity_loss: 2, database_capacity_loss: 1 },
        ];
        evidence.disturbances = vec![
            DemandDisturbance { critical_demand: 0, bulk_demand: 1 },
            DemandDisturbance { critical_demand: 1, bulk_demand: 2 },
        ];
        let actions = [
            CandidateOperation::Noop,
            CandidateOperation::SetApiCapacity(8),
            CandidateOperation::SetApiCapacity(12),
            CandidateOperation::SetRetryMultiplierPpm(500_000),
            CandidateOperation::SetRetryMultiplierPpm(1_000_000),
        ];
        for action in actions {
            let expected = exhaustive_oracle_safe(&evidence, &action);
            let actual = appraise_one_step(&evidence, &act(action)).disposition
                == FilterDisposition::AdmitForAuthorization;
            assert_eq!(actual, expected, "oracle mismatch for action: {action:?}");
        }
    }
}
