use crate::power_state::{
    PowerModelContract, PowerTraceSample, PredictedPowerTrace, PredictionScope, SampleQuality,
};
use crate::AuditError;
use std::collections::{BTreeMap, HashSet};

/// Split-conformal calibration contract tied to one exact power-model revision.
///
/// The exchangeability unit is a complete acquisition session, not a sample
/// row. A session score is the maximum interval miss across every sample and
/// trace in that session, allowing arbitrary within-session temporal dependence.
#[derive(Debug, Clone, PartialEq)]
pub struct ForecastCalibrationContract {
    pub calibrator_id: String,
    pub calibrator_revision: String,
    pub model_id: String,
    pub model_revision: String,
    /// Complete upstream model contract; equality binds sensor, calibration,
    /// dataset, rights, normalization and support thresholds as well.
    pub power_model_contract: PowerModelContract,
    pub hardware_configuration_fingerprint: String,
    pub measurement_boundary_id: String,
    pub workload_contract_id: String,
    pub state_mapper_id: String,
    pub clock_alignment_method_id: String,
    pub sample_period_ns: u64,
    pub target_session_coverage: f64,
    pub minimum_calibration_sessions_per_regime: usize,
    pub required_regimes: Vec<String>,
    pub exchangeability_review_evidence_id: String,
    /// A declaration supported by the review reference, not machine proof.
    pub session_exchangeability_assumption: bool,
    /// Freeze exposure so session-maximum scores have comparable opportunities
    /// to encounter an extreme miss.
    pub session_window_duration_ns: u64,
    pub minimum_session_separation_ns: u64,
    pub traces_per_session: usize,
    pub samples_per_trace: usize,
}

impl ForecastCalibrationContract {
    fn validate(&self) -> Result<(), AuditError> {
        for (value, field) in [
            (self.calibrator_id.as_str(), "calibrator_id"),
            (self.calibrator_revision.as_str(), "calibrator_revision"),
            (self.model_id.as_str(), "model_id"),
            (self.model_revision.as_str(), "model_revision"),
            (
                self.hardware_configuration_fingerprint.as_str(),
                "hardware_configuration_fingerprint",
            ),
            (self.measurement_boundary_id.as_str(), "measurement_boundary_id"),
            (self.workload_contract_id.as_str(), "workload_contract_id"),
            (self.state_mapper_id.as_str(), "state_mapper_id"),
            (self.clock_alignment_method_id.as_str(), "clock_alignment_method_id"),
            (
                self.exchangeability_review_evidence_id.as_str(),
                "exchangeability_review_evidence_id",
            ),
        ] {
            if value.trim().is_empty() {
                return Err(AuditError(format!(
                    "forecast calibration contract {field} must not be blank"
                )));
            }
        }

        if !self.session_exchangeability_assumption {
            return Err(AuditError(
                "session-level conformal calibration requires an explicit exchangeability review"
                    .to_string(),
            ));
        }
        if !self.target_session_coverage.is_finite()
            || self.target_session_coverage <= 0.5
            || self.target_session_coverage >= 1.0
        {
            return Err(AuditError(
                "target session coverage must be finite and in (0.5, 1)".to_string(),
            ));
        }
        if self.minimum_calibration_sessions_per_regime == 0
            || self.sample_period_ns == 0
            || self.session_window_duration_ns == 0
            || self.minimum_session_separation_ns == 0
            || self.traces_per_session == 0
            || self.samples_per_trace == 0
        {
            return Err(AuditError(
                "calibration support, cadence, session window, separation and exposure counts must be positive"
                    .to_string(),
            ));
        }
        if self.required_regimes.is_empty() {
            return Err(AuditError(
                "at least one required operating regime must be declared".to_string(),
            ));
        }
        let mut regimes = HashSet::new();
        for regime in &self.required_regimes {
            if regime.trim().is_empty() || !regimes.insert(regime.as_str()) {
                return Err(AuditError(
                    "required operating regimes must be non-blank and unique".to_string(),
                ));
            }
        }
        Ok(())
    }
}

/// One prediction / measurement join. Predictions must be operational-scenario
/// predictions, never the model's held-out validation runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForecastTraceObservation {
    pub predicted: PredictedPowerTrace,
    pub measured: Vec<PowerTraceSample>,
    pub measurement_evidence_id: String,
}

/// A fixed-duration, predeclared calibration or monitoring unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForecastObservationSession {
    pub session_id: String,
    pub operating_regime_id: String,
    pub session_evidence_id: String,
    pub window_start_ns: u64,
    pub window_end_ns: u64,
    pub traces: Vec<ForecastTraceObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegimeConformalCalibration {
    pub operating_regime_id: String,
    pub calibration_session_count: usize,
    pub conformal_order_statistic_rank: usize,
    pub additive_interval_correction_mw: u64,
    pub calibration_session_ids: Vec<String>,
    pub calibration_session_evidence_ids: Vec<String>,
    pub calibration_measurement_evidence_ids: Vec<String>,
    pub calibration_run_ids: Vec<String>,
    pub calibration_prediction_trace_ids: Vec<String>,
    pub calibration_timeline_digest_references: Vec<String>,
    pub calibration_sample_evidence_ids: Vec<String>,
    pub calibration_source_event_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegimeForecastCalibrator {
    pub contract: ForecastCalibrationContract,
    pub regimes: Vec<RegimeConformalCalibration>,
    pub total_calibration_session_count: usize,
    pub claim_ceiling: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalibratedPowerSample {
    pub timestamp_ns: u64,
    pub state_id: String,
    pub source_event_id: String,
    pub lower_power_mw: u64,
    pub median_power_mw: u64,
    pub upper_power_mw: u64,
    pub additive_interval_correction_mw: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CalibratedPowerTrace {
    pub trace_id: String,
    pub run_id: String,
    pub operating_regime_id: String,
    pub calibrator_id: String,
    pub calibrator_revision: String,
    pub model_id: String,
    pub model_revision: String,
    pub hardware_configuration_fingerprint: String,
    pub measurement_boundary_id: String,
    pub workload_contract_id: String,
    pub state_mapper_id: String,
    pub state_mapping_evidence_id: String,
    pub time_alignment_evidence_id: String,
    pub source_timeline_digest_reference: String,
    pub calibration_session_count: usize,
    pub target_session_coverage: f64,
    pub sample_period_ns: u64,
    pub samples: Vec<CalibratedPowerSample>,
    pub claim_ceiling: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForecastHealthStatus {
    Nominal,
    Degraded,
    InsufficientSupport,
    OutOfDomain,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForecastSessionResult {
    pub session_id: String,
    pub operating_regime_id: String,
    pub calibrated: bool,
    pub all_samples_covered: Option<bool>,
    pub sample_count: usize,
    pub covered_sample_count: Option<usize>,
    pub additive_interval_correction_mw: Option<u64>,
    pub session_evidence_id: String,
    pub measurement_evidence_ids: Vec<String>,
    pub prediction_trace_ids: Vec<String>,
    pub source_timeline_digest_references: Vec<String>,
    pub sample_evidence_ids: Vec<String>,
    pub source_event_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegimeForecastHealth {
    pub operating_regime_id: String,
    pub observed_session_count: usize,
    pub covered_session_count: usize,
    pub empirical_session_coverage: Option<f64>,
    pub wilson_lower_bound_session_coverage: Option<f64>,
    pub sample_count: usize,
    pub covered_sample_count: usize,
    pub empirical_sample_coverage: Option<f64>,
    pub out_of_domain_session_count: usize,
    pub status: ForecastHealthStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForecastMonitoringContract {
    pub minimum_observed_sessions_per_regime: usize,
    pub minimum_wilson_lower_bound_session_coverage: f64,
    pub one_sided_confidence_z_score: f64,
}

impl ForecastMonitoringContract {
    fn validate(&self) -> Result<(), AuditError> {
        if self.minimum_observed_sessions_per_regime == 0
            || !self.minimum_wilson_lower_bound_session_coverage.is_finite()
            || self.minimum_wilson_lower_bound_session_coverage <= 0.0
            || self.minimum_wilson_lower_bound_session_coverage > 1.0
            || !self.one_sided_confidence_z_score.is_finite()
            || self.one_sided_confidence_z_score <= 0.0
            || !(self.one_sided_confidence_z_score * self.one_sided_confidence_z_score).is_finite()
        {
            return Err(AuditError(
                "forecast monitoring contract contains an invalid support, coverage or z-score threshold"
                    .to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForecastMonitoringReport {
    pub calibrator_id: String,
    pub calibrator_revision: String,
    pub target_session_coverage: f64,
    pub total_monitoring_session_count: usize,
    pub calibrated_session_count: usize,
    pub out_of_domain_session_count: usize,
    pub session_results: Vec<ForecastSessionResult>,
    pub regime_health: Vec<RegimeForecastHealth>,
    pub claim_ceiling: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SessionCalibrationScore {
    session_id: String,
    session_evidence_id: String,
    score_mw: u64,
    measurement_evidence_ids: Vec<String>,
    run_ids: Vec<String>,
    prediction_trace_ids: Vec<String>,
    timeline_digest_references: Vec<String>,
    sample_evidence_ids: Vec<String>,
    source_event_ids: Vec<String>,
}

fn conformal_order_statistic_rank(sample_count: usize, coverage: f64) -> Option<usize> {
    if sample_count == 0 || !coverage.is_finite() || coverage <= 0.0 || coverage >= 1.0 {
        return None;
    }
    let rank = (((sample_count as f64) + 1.0) * coverage).ceil();
    if !rank.is_finite() || rank < 1.0 || rank > usize::MAX as f64 {
        return None;
    }
    Some(rank as usize)
}

fn model_contract_matches(
    calibration: &ForecastCalibrationContract,
    predicted: &PredictedPowerTrace,
) -> bool {
    predicted.calibration_contract == calibration.power_model_contract
        && predicted.model_id == calibration.model_id
        && predicted.model_revision == calibration.model_revision
        && predicted.hardware_configuration_fingerprint
            == calibration.hardware_configuration_fingerprint
        && predicted.measurement_boundary_id == calibration.measurement_boundary_id
        && predicted.workload_contract_id == calibration.workload_contract_id
        && predicted.state_mapper_id == calibration.state_mapper_id
        && predicted.sample_period_ns == calibration.sample_period_ns
        && predicted.calibration_contract.model_id == calibration.model_id
        && predicted.calibration_contract.model_revision == calibration.model_revision
        && predicted.calibration_contract.hardware_configuration_fingerprint
            == calibration.hardware_configuration_fingerprint
        && predicted.calibration_contract.measurement_boundary_id
            == calibration.measurement_boundary_id
        && predicted.calibration_contract.workload_contract_id
            == calibration.workload_contract_id
        && predicted.calibration_contract.state_mapper_id == calibration.state_mapper_id
        && predicted.calibration_contract.clock_alignment_method_id
            == calibration.clock_alignment_method_id
        && predicted.sample_period_ns == predicted.calibration_contract.sample_period_ns
}

fn interval_violation_mw(lower: u64, upper: u64, actual: u64) -> u64 {
    if actual < lower {
        lower - actual
    } else if actual > upper {
        actual - upper
    } else {
        0
    }
}

fn validate_trace_observation(
    contract: &ForecastCalibrationContract,
    observation: &ForecastTraceObservation,
    session_start_ns: u64,
    session_end_ns: u64,
) -> Result<u64, AuditError> {
    if observation.measurement_evidence_id.trim().is_empty() {
        return Err(AuditError(
            "forecast observation requires measurement evidence".to_string(),
        ));
    }
    let predicted = &observation.predicted;
    if predicted.prediction_scope != PredictionScope::OperationalScenario {
        return Err(AuditError(
            "forecast calibrator accepts only operational-scenario predictions, never held-out validation"
                .to_string(),
        ));
    }
    if !model_contract_matches(contract, predicted) {
        return Err(AuditError(format!(
            "forecast run {:?} does not match the exact model revision, hardware, boundary, workload, mapper, clock or cadence contract",
            predicted.run_id
        )));
    }
    if predicted.run_id.trim().is_empty()
        || predicted.trace_id.trim().is_empty()
        || predicted.state_mapping_evidence_id.trim().is_empty()
        || predicted.time_alignment_evidence_id.trim().is_empty()
        || predicted.source_timeline_digest_reference.trim().is_empty()
        || predicted.samples.len() != contract.samples_per_trace
        || observation.measured.len() != contract.samples_per_trace
    {
        return Err(AuditError(
            "forecast prediction/measurement identity, evidence or exact sample exposure is invalid"
                .to_string(),
        ));
    }
    if predicted.training_evidence.iter().any(|entry| entry.run_id == predicted.run_id)
        || predicted.held_out_evidence.iter().any(|entry| entry.run_id == predicted.run_id)
    {
        return Err(AuditError(
            "operational forecast run ID overlaps training or held-out evidence".to_string(),
        ));
    }

    let mut previous_timestamp = None;
    let mut maximum_violation_mw = 0_u64;
    for (index, (estimate, actual)) in predicted
        .samples
        .iter()
        .zip(observation.measured.iter())
        .enumerate()
    {
        if estimate.lower_power_mw > estimate.median_power_mw
            || estimate.median_power_mw > estimate.upper_power_mw
            || estimate.source_event_id.trim().is_empty()
            || actual.quality != SampleQuality::Good
            || actual.source_evidence_id.trim().is_empty()
            || actual.timestamp_ns != estimate.timestamp_ns
            || actual.state_id != estimate.state_id
        {
            return Err(AuditError(format!(
                "forecast sample {index} has invalid interval order, quality, evidence or exact timestamp/state alignment"
            )));
        }
        if estimate.timestamp_ns < session_start_ns || estimate.timestamp_ns >= session_end_ns {
            return Err(AuditError(format!(
                "forecast sample {index} falls outside its declared session window"
            )));
        }
        if let Some(previous) = previous_timestamp {
            if estimate.timestamp_ns.checked_sub(previous) != Some(contract.sample_period_ns) {
                return Err(AuditError(format!(
                    "forecast session has irregular or non-increasing sample cadence at index {index}"
                )));
            }
        }
        previous_timestamp = Some(estimate.timestamp_ns);
        maximum_violation_mw = maximum_violation_mw.max(interval_violation_mw(
            estimate.lower_power_mw,
            estimate.upper_power_mw,
            actual.power_mw,
        ));
    }

    Ok(maximum_violation_mw)
}

fn validate_session_shape(
    contract: &ForecastCalibrationContract,
    session: &ForecastObservationSession,
) -> Result<(), AuditError> {
    if session.session_id.trim().is_empty()
        || session.operating_regime_id.trim().is_empty()
        || session.session_evidence_id.trim().is_empty()
    {
        return Err(AuditError(
            "forecast session identity, regime and evidence must be non-blank".to_string(),
        ));
    }
    if session.window_end_ns.checked_sub(session.window_start_ns)
        != Some(contract.session_window_duration_ns)
    {
        return Err(AuditError(format!(
            "forecast session {:?} duration differs from the frozen exposure window",
            session.session_id
        )));
    }
    if session.traces.len() != contract.traces_per_session {
        return Err(AuditError(format!(
            "forecast session {:?} has {} traces; exact frozen count is {}",
            session.session_id,
            session.traces.len(),
            contract.traces_per_session
        )));
    }
    Ok(())
}

fn validate_sessions_non_overlapping(
    contract: &ForecastCalibrationContract,
    sessions: &[ForecastObservationSession],
) -> Result<(), AuditError> {
    let mut ordered: Vec<&ForecastObservationSession> = sessions.iter().collect();
    ordered.sort_by_key(|session| session.window_start_ns);
    let mut previous_end = None;
    for session in ordered {
        if let Some(end) = previous_end {
            let earliest_start = end
                .checked_add(contract.minimum_session_separation_ns)
                .ok_or_else(|| AuditError("forecast session separation overflowed".to_string()))?;
            if session.window_start_ns < earliest_start {
                return Err(AuditError(
                    "forecast calibration/monitoring sessions overlap or violate declared separation"
                        .to_string(),
                ));
            }
        }
        previous_end = Some(session.window_end_ns);
    }
    Ok(())
}

/// Fit one additive correction per operating regime using session-level
/// maximum nonconformity scores. Every required regime must meet the exact
/// finite-sample conformal support requirement.
pub fn fit_regime_forecast_calibrator(
    contract: ForecastCalibrationContract,
    sessions: Vec<ForecastObservationSession>,
) -> Result<RegimeForecastCalibrator, AuditError> {
    contract.validate()?;
    if sessions.is_empty() {
        return Err(AuditError(
            "forecast calibration requires at least one session".to_string(),
        ));
    }
    validate_sessions_non_overlapping(&contract, &sessions)?;

    let mut session_ids = HashSet::new();
    let mut run_ids = HashSet::new();
    let mut trace_ids = HashSet::new();
    let mut scores_by_regime: BTreeMap<String, Vec<SessionCalibrationScore>> = BTreeMap::new();

    for session in &sessions {
        validate_session_shape(&contract, session)?;
        if !session_ids.insert(session.session_id.as_str()) {
            return Err(AuditError(format!(
                "duplicate forecast calibration session {:?}",
                session.session_id
            )));
        }
        let mut score_mw = 0_u64;
        let mut measurement_evidence_ids = Vec::with_capacity(session.traces.len());
        let mut session_run_ids = Vec::with_capacity(session.traces.len());
        let mut prediction_trace_ids = Vec::with_capacity(session.traces.len());
        let mut timeline_digest_references = Vec::with_capacity(session.traces.len());
        let mut sample_evidence_ids = Vec::new();
        let mut source_event_ids = Vec::new();
        for observation in &session.traces {
            if !run_ids.insert(observation.predicted.run_id.as_str()) {
                return Err(AuditError(format!(
                    "duplicate forecast calibration run {:?}",
                    observation.predicted.run_id
                )));
            }
            if !trace_ids.insert(observation.predicted.trace_id.as_str()) {
                return Err(AuditError(format!(
                    "duplicate forecast prediction trace {:?}",
                    observation.predicted.trace_id
                )));
            }
            score_mw = score_mw.max(validate_trace_observation(
                &contract,
                observation,
                session.window_start_ns,
                session.window_end_ns,
            )?);
            measurement_evidence_ids.push(observation.measurement_evidence_id.clone());
            session_run_ids.push(observation.predicted.run_id.clone());
            prediction_trace_ids.push(observation.predicted.trace_id.clone());
            timeline_digest_references.push(
                observation.predicted.source_timeline_digest_reference.clone(),
            );
            sample_evidence_ids.extend(
                observation.measured.iter().map(|sample| sample.source_evidence_id.clone()),
            );
            source_event_ids.extend(
                observation.predicted.samples.iter().map(|sample| sample.source_event_id.clone()),
            );
        }
        scores_by_regime
            .entry(session.operating_regime_id.clone())
            .or_default()
            .push(SessionCalibrationScore {
                session_id: session.session_id.clone(),
                session_evidence_id: session.session_evidence_id.clone(),
                score_mw,
                measurement_evidence_ids,
                run_ids: session_run_ids,
                prediction_trace_ids,
                timeline_digest_references,
                sample_evidence_ids,
                source_event_ids,
            });
    }

    let required_regimes: HashSet<&str> =
        contract.required_regimes.iter().map(String::as_str).collect();
    if scores_by_regime.keys().any(|regime| !required_regimes.contains(regime.as_str())) {
        return Err(AuditError(
            "calibration input contains an operating regime absent from the frozen contract"
                .to_string(),
        ));
    }

    let mut regimes = Vec::with_capacity(contract.required_regimes.len());
    for regime_id in &contract.required_regimes {
        let scores = scores_by_regime.get(regime_id).ok_or_else(|| {
            AuditError(format!("required forecast regime {regime_id:?} has no sessions"))
        })?;
        let session_count = scores.len();
        let rank = conformal_order_statistic_rank(
            session_count,
            contract.target_session_coverage,
        )
        .ok_or_else(|| AuditError("cannot derive conformal order-statistic rank".to_string()))?;

        if session_count < contract.minimum_calibration_sessions_per_regime || rank > session_count {
            return Err(AuditError(format!(
                "forecast regime {regime_id:?} has {session_count} calibration sessions; requires at least {} and conformal rank {rank} must be observable",
                contract.minimum_calibration_sessions_per_regime
            )));
        }

        let mut ordered_scores: Vec<u64> = scores.iter().map(|score| score.score_mw).collect();
        ordered_scores.sort_unstable();
        let correction = *ordered_scores.get(rank - 1).ok_or_else(|| {
            AuditError("conformal correction rank is outside the calibration sample".to_string())
        })?;

        let mut session_evidence = scores.clone();
        session_evidence.sort_by(|left, right| left.session_id.cmp(&right.session_id));
        regimes.push(RegimeConformalCalibration {
            operating_regime_id: regime_id.clone(),
            calibration_session_count: session_count,
            conformal_order_statistic_rank: rank,
            additive_interval_correction_mw: correction,
            calibration_session_ids: session_evidence
                .iter()
                .map(|score| score.session_id.clone())
                .collect(),
            calibration_session_evidence_ids: session_evidence
                .iter()
                .map(|score| score.session_evidence_id.clone())
                .collect(),
            calibration_measurement_evidence_ids: session_evidence
                .iter()
                .flat_map(|score| score.measurement_evidence_ids.clone())
                .collect(),
            calibration_run_ids: session_evidence
                .iter()
                .flat_map(|score| score.run_ids.clone())
                .collect(),
            calibration_prediction_trace_ids: session_evidence
                .iter()
                .flat_map(|score| score.prediction_trace_ids.clone())
                .collect(),
            calibration_timeline_digest_references: session_evidence
                .iter()
                .flat_map(|score| score.timeline_digest_references.clone())
                .collect(),
            calibration_sample_evidence_ids: session_evidence
                .iter()
                .flat_map(|score| score.sample_evidence_ids.clone())
                .collect(),
            calibration_source_event_ids: session_evidence
                .iter()
                .flat_map(|score| score.source_event_ids.clone())
                .collect(),
        });
    }

    Ok(RegimeForecastCalibrator {
        contract: contract.clone(),
        regimes,
        total_calibration_session_count: sessions.len(),
        claim_ceiling: "SESSION_LEVEL_SPLIT_CONFORMAL_MODELLED_INTERVAL_NOT_PHYSICAL_POWER_OR_GRID_QUALIFICATION",
    })
}

fn regime_calibration<'a>(
    calibrator: &'a RegimeForecastCalibrator,
    regime_id: &str,
) -> Option<&'a RegimeConformalCalibration> {
    calibrator
        .regimes
        .iter()
        .find(|regime| regime.operating_regime_id == regime_id)
}

/// Expand an operational-scenario interval using the regime-specific
/// session-level conformal correction. Unknown regimes fail closed.
pub fn apply_regime_forecast_calibration(
    calibrator: &RegimeForecastCalibrator,
    regime_id: &str,
    predicted: &PredictedPowerTrace,
) -> Result<CalibratedPowerTrace, AuditError> {
    calibrator.contract.validate()?;
    if regime_id.trim().is_empty() {
        return Err(AuditError(
            "forecast regime ID must not be blank".to_string(),
        ));
    }
    if predicted.prediction_scope != PredictionScope::OperationalScenario {
        return Err(AuditError(
            "regime calibration cannot be applied to a held-out validation prediction"
                .to_string(),
        ));
    }
    if !model_contract_matches(&calibrator.contract, predicted) {
        return Err(AuditError(
            "prediction does not match the calibrator's exact model/configuration contract"
                .to_string(),
        ));
    }
    if predicted.run_id.trim().is_empty()
        || predicted.trace_id.trim().is_empty()
        || predicted.state_mapping_evidence_id.trim().is_empty()
        || predicted.time_alignment_evidence_id.trim().is_empty()
        || predicted.source_timeline_digest_reference.trim().is_empty()
        || predicted.samples.len() != calibrator.contract.samples_per_trace
    {
        return Err(AuditError(
            "forecast prediction is missing identity, evidence or exact exposure".to_string(),
        ));
    }
    if predicted.training_evidence.iter().any(|entry| entry.run_id == predicted.run_id)
        || predicted.held_out_evidence.iter().any(|entry| entry.run_id == predicted.run_id)
    {
        return Err(AuditError(
            "operational forecast run ID overlaps training or held-out evidence".to_string(),
        ));
    }
    let regime = regime_calibration(calibrator, regime_id).ok_or_else(|| {
        AuditError(format!(
            "operating regime {regime_id:?} is OUT_OF_DOMAIN; no calibrated interval is available"
        ))
    })?;

    if regime.calibration_run_ids.iter().any(|run_id| run_id == &predicted.run_id)
        || regime.calibration_prediction_trace_ids.iter().any(|trace_id| trace_id == &predicted.trace_id)
    {
        return Err(AuditError(
            "prediction run or trace ID was already used to fit this regime calibrator".to_string(),
        ));
    }
    let mut samples = Vec::with_capacity(predicted.samples.len());
    let mut previous_timestamp = None;
    for (index, estimate) in predicted.samples.iter().enumerate() {
        if estimate.lower_power_mw > estimate.median_power_mw
            || estimate.median_power_mw > estimate.upper_power_mw
            || estimate.source_event_id.trim().is_empty()
        {
            return Err(AuditError(format!(
                "prediction sample {index} has an invalid interval or missing source event"
            )));
        }
        if let Some(previous) = previous_timestamp {
            if estimate.timestamp_ns.checked_sub(previous)
                != Some(calibrator.contract.sample_period_ns)
            {
                return Err(AuditError(format!(
                    "prediction has irregular cadence at sample {index}"
                )));
            }
        }
        previous_timestamp = Some(estimate.timestamp_ns);
        let lower = estimate
            .lower_power_mw
            .saturating_sub(regime.additive_interval_correction_mw);
        let upper = estimate
            .upper_power_mw
            .checked_add(regime.additive_interval_correction_mw)
            .ok_or_else(|| {
                AuditError(format!(
                    "calibrated upper interval overflowed at sample {index}"
                ))
            })?;
        samples.push(CalibratedPowerSample {
            timestamp_ns: estimate.timestamp_ns,
            state_id: estimate.state_id.clone(),
            source_event_id: estimate.source_event_id.clone(),
            lower_power_mw: lower,
            median_power_mw: estimate.median_power_mw,
            upper_power_mw: upper,
            additive_interval_correction_mw: regime.additive_interval_correction_mw,
        });
    }

    Ok(CalibratedPowerTrace {
        trace_id: predicted.trace_id.clone(),
        run_id: predicted.run_id.clone(),
        operating_regime_id: regime_id.to_string(),
        calibrator_id: calibrator.contract.calibrator_id.clone(),
        calibrator_revision: calibrator.contract.calibrator_revision.clone(),
        model_id: predicted.model_id.clone(),
        model_revision: predicted.model_revision.clone(),
        hardware_configuration_fingerprint: predicted.hardware_configuration_fingerprint.clone(),
        measurement_boundary_id: predicted.measurement_boundary_id.clone(),
        workload_contract_id: predicted.workload_contract_id.clone(),
        state_mapper_id: predicted.state_mapper_id.clone(),
        state_mapping_evidence_id: predicted.state_mapping_evidence_id.clone(),
        time_alignment_evidence_id: predicted.time_alignment_evidence_id.clone(),
        source_timeline_digest_reference: predicted.source_timeline_digest_reference.clone(),
        calibration_session_count: regime.calibration_session_count,
        target_session_coverage: calibrator.contract.target_session_coverage,
        sample_period_ns: predicted.sample_period_ns,
        samples,
        claim_ceiling: "REGIME_CONFORMAL_MODELLED_INTERVAL_NOT_MEASUREMENT_OR_GRID_QUALIFICATION",
    })
}

fn wilson_lower_bound(successes: usize, trials: usize, z_score: f64) -> Option<f64> {
    if trials == 0
        || successes > trials
        || !z_score.is_finite()
        || z_score <= 0.0
    {
        return None;
    }
    let n = trials as f64;
    let p = successes as f64 / n;
    let z2 = z_score * z_score;
    if !z2.is_finite() {
        return None;
    }
    let denominator = 1.0 + z2 / n;
    let center = p + z2 / (2.0 * n);
    let margin = z_score * (p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt();
    let lower = ((center - margin) / denominator).clamp(0.0, 1.0);
    lower.is_finite().then_some(lower)
}

/// Monitor already calibrated forecasts. This function does not update or
/// refit the calibrator from monitoring outcomes; a recalibration requires a
/// new revision and a separately frozen calibration session set.
pub fn evaluate_forecast_monitoring(
    calibrator: &RegimeForecastCalibrator,
    contract: ForecastMonitoringContract,
    sessions: Vec<ForecastObservationSession>,
) -> Result<ForecastMonitoringReport, AuditError> {
    calibrator.contract.validate()?;
    contract.validate()?;
    if sessions.is_empty() {
        return Err(AuditError(
            "forecast monitoring requires at least one observation session".to_string(),
        ));
    }
    validate_sessions_non_overlapping(&calibrator.contract, &sessions)?;

    let mut session_ids = HashSet::new();
    let mut run_ids = HashSet::new();
    let mut session_results = Vec::with_capacity(sessions.len());
    let mut grouped: BTreeMap<String, (usize, usize, usize, usize)> = BTreeMap::new();
    let mut out_of_domain_by_regime: BTreeMap<String, usize> = BTreeMap::new();
    let mut calibrated_session_count = 0_usize;
    let mut out_of_domain_session_count = 0_usize;

    for session in &sessions {
        validate_session_shape(&calibrator.contract, session)?;
        if !session_ids.insert(session.session_id.as_str()) {
            return Err(AuditError(format!(
                "duplicate forecast monitoring session {:?}",
                session.session_id
            )));
        }
        if calibrator.regimes.iter().any(|regime| {
            regime.calibration_session_ids.iter().any(|id| id == &session.session_id)
        }) {
            return Err(AuditError(
                "monitoring session ID overlaps the frozen conformal calibration set".to_string(),
            ));
        }

        let mut sample_count = 0_usize;
        let mut covered_sample_count = 0_usize;
        let mut session_measurement_evidence = Vec::with_capacity(session.traces.len());
        let mut session_trace_ids = Vec::with_capacity(session.traces.len());
        let mut session_timeline_digests = Vec::with_capacity(session.traces.len());
        let mut session_sample_evidence_ids = Vec::new();
        let mut session_source_event_ids = Vec::new();

        for observation in &session.traces {
            if !run_ids.insert(observation.predicted.run_id.as_str()) {
                return Err(AuditError(format!(
                    "duplicate forecast monitoring run {:?}",
                    observation.predicted.run_id
                )));
            }
            if calibrator.regimes.iter().any(|regime| {
                regime.calibration_run_ids.iter().any(|run_id| run_id == &observation.predicted.run_id)
                    || regime.calibration_prediction_trace_ids.iter().any(|trace_id| trace_id == &observation.predicted.trace_id)
            }) {
                return Err(AuditError(
                    "monitoring run or trace ID overlaps the frozen conformal calibration set".to_string(),
                ));
            }
            let _ = validate_trace_observation(
                &calibrator.contract,
                observation,
                session.window_start_ns,
                session.window_end_ns,
            )?;
            session_measurement_evidence.push(observation.measurement_evidence_id.clone());
            session_trace_ids.push(observation.predicted.trace_id.clone());
            session_timeline_digests.push(
                observation.predicted.source_timeline_digest_reference.clone(),
            );
            session_sample_evidence_ids.extend(
                observation.measured.iter().map(|sample| sample.source_evidence_id.clone()),
            );
            session_source_event_ids.extend(
                observation.predicted.samples.iter().map(|sample| sample.source_event_id.clone()),
            );
            sample_count = sample_count
                .checked_add(observation.measured.len())
                .ok_or_else(|| AuditError("monitoring sample count overflowed".to_string()))?;
        }

        let regime = regime_calibration(calibrator, &session.operating_regime_id);
        if let Some(regime) = regime {
            let mut all_samples_covered = true;
            for observation in &session.traces {
                for (estimate, actual) in observation
                    .predicted
                    .samples
                    .iter()
                    .zip(observation.measured.iter())
                {
                    let lower = estimate
                        .lower_power_mw
                        .saturating_sub(regime.additive_interval_correction_mw);
                    let upper = estimate
                        .upper_power_mw
                        .checked_add(regime.additive_interval_correction_mw)
                        .ok_or_else(|| {
                            AuditError(
                                "monitoring calibrated upper interval overflowed".to_string(),
                            )
                        })?;
                    let covered = actual.power_mw >= lower && actual.power_mw <= upper;
                    if covered {
                        covered_sample_count += 1;
                    } else {
                        all_samples_covered = false;
                    }
                }
            }
            calibrated_session_count += 1;
            let group = grouped
                .entry(session.operating_regime_id.clone())
                .or_insert((0, 0, 0, 0));
            group.0 += 1;
            group.1 += usize::from(all_samples_covered);
            group.2 += sample_count;
            group.3 += covered_sample_count;

            session_results.push(ForecastSessionResult {
                session_id: session.session_id.clone(),
                operating_regime_id: session.operating_regime_id.clone(),
                calibrated: true,
                all_samples_covered: Some(all_samples_covered),
                sample_count,
                covered_sample_count: Some(covered_sample_count),
                additive_interval_correction_mw: Some(regime.additive_interval_correction_mw),
                session_evidence_id: session.session_evidence_id.clone(),
                measurement_evidence_ids: session_measurement_evidence,
                prediction_trace_ids: session_trace_ids,
                source_timeline_digest_references: session_timeline_digests,
                sample_evidence_ids: session_sample_evidence_ids,
                source_event_ids: session_source_event_ids,
            });
        } else {
            out_of_domain_session_count += 1;
            *out_of_domain_by_regime
                .entry(session.operating_regime_id.clone())
                .or_insert(0) += 1;
            session_results.push(ForecastSessionResult {
                session_id: session.session_id.clone(),
                operating_regime_id: session.operating_regime_id.clone(),
                calibrated: false,
                all_samples_covered: None,
                sample_count,
                covered_sample_count: None,
                additive_interval_correction_mw: None,
                session_evidence_id: session.session_evidence_id.clone(),
                measurement_evidence_ids: session_measurement_evidence,
                prediction_trace_ids: session_trace_ids,
                source_timeline_digest_references: session_timeline_digests,
                sample_evidence_ids: session_sample_evidence_ids,
                source_event_ids: session_source_event_ids,
            });
        }
    }

    let mut regimes = Vec::new();
    for (regime_id, (observed, covered_sessions, sample_count, covered_samples)) in grouped {
        let empirical_session_coverage = covered_sessions as f64 / observed as f64;
        let wilson = wilson_lower_bound(
            covered_sessions,
            observed,
            contract.one_sided_confidence_z_score,
        );
        let empirical_sample_coverage = if sample_count > 0 {
            Some(covered_samples as f64 / sample_count as f64)
        } else {
            None
        };
        let status = if observed < contract.minimum_observed_sessions_per_regime {
            ForecastHealthStatus::InsufficientSupport
        } else if wilson.is_some_and(|lower_bound| {
            lower_bound >= contract.minimum_wilson_lower_bound_session_coverage
        }) {
            ForecastHealthStatus::Nominal
        } else {
            ForecastHealthStatus::Degraded
        };
        regimes.push(RegimeForecastHealth {
            operating_regime_id: regime_id.clone(),
            observed_session_count: observed,
            covered_session_count: covered_sessions,
            empirical_session_coverage: Some(empirical_session_coverage),
            wilson_lower_bound_session_coverage: wilson,
            sample_count,
            covered_sample_count: covered_samples,
            empirical_sample_coverage,
            out_of_domain_session_count: out_of_domain_by_regime
                .get(&regime_id)
                .copied()
                .unwrap_or(0),
            status,
        });
    }

    for (regime_id, out_of_domain_count) in out_of_domain_by_regime {
        regimes.push(RegimeForecastHealth {
            operating_regime_id: regime_id,
            observed_session_count: 0,
            covered_session_count: 0,
            empirical_session_coverage: None,
            wilson_lower_bound_session_coverage: None,
            sample_count: 0,
            covered_sample_count: 0,
            empirical_sample_coverage: None,
            out_of_domain_session_count: out_of_domain_count,
            status: ForecastHealthStatus::OutOfDomain,
        });
    }
    regimes.sort_by(|left, right| left.operating_regime_id.cmp(&right.operating_regime_id));

    Ok(ForecastMonitoringReport {
        calibrator_id: calibrator.contract.calibrator_id.clone(),
        calibrator_revision: calibrator.contract.calibrator_revision.clone(),
        target_session_coverage: calibrator.contract.target_session_coverage,
        total_monitoring_session_count: sessions.len(),
        calibrated_session_count,
        out_of_domain_session_count,
        session_results,
        regime_health: regimes,
        claim_ceiling: "PREQUENTIAL_FORECAST_HEALTH_MONITOR_NOT_FUTURE_COVERAGE_GUARANTEE_OR_PHYSICAL_QUALIFICATION",
    })
}

#[cfg(test)]
mod tests {
    use super::{
        apply_regime_forecast_calibration, conformal_order_statistic_rank,
        evaluate_forecast_monitoring, fit_regime_forecast_calibrator,
        ForecastCalibrationContract, ForecastHealthStatus, ForecastMonitoringContract,
        ForecastObservationSession, ForecastTraceObservation,
    };
    use crate::power_state::{
        PowerModelContract, PowerTraceSample, PredictedPowerSample, PredictedPowerTrace,
        PredictionBasis, PredictionScope, SampleQuality,
    };

    fn model_contract() -> PowerModelContract {
        PowerModelContract {
            model_id: "power-state-v1".to_string(),
            model_revision: "rev-a".to_string(),
            power_model_contract: model_contract(),
            hardware_configuration_fingerprint: "hardware-sha".to_string(),
            measurement_boundary_id: "gpu-board-pcc".to_string(),
            sensor_id: "sensor-1".to_string(),
            sensor_calibration_evidence_id: "sensor-cert-1".to_string(),
            sensor_uncertainty_mw: 10,
            dataset_id: "dataset-1".to_string(),
            dataset_version: "v1".to_string(),
            dataset_evidence_id: "dataset-evidence-1".to_string(),
            use_authorization_reference: "rights-1".to_string(),
            normalization_id: "norm-v1".to_string(),
            workload_contract_id: "workload-v1".to_string(),
            state_mapper_id: "mapper-v1".to_string(),
            clock_alignment_method_id: "ptp-profile-v1".to_string(),
            sample_period_ns: 10,
            minimum_samples_per_state: 1,
            minimum_samples_per_transition: 1,
        }
    }

    fn calibration_contract(regimes: &[&str]) -> ForecastCalibrationContract {
        ForecastCalibrationContract {
            calibrator_id: "regime-calibrator-v1".to_string(),
            calibrator_revision: "rev-1".to_string(),
            model_id: "power-state-v1".to_string(),
            model_revision: "rev-a".to_string(),
            hardware_configuration_fingerprint: "hardware-sha".to_string(),
            measurement_boundary_id: "gpu-board-pcc".to_string(),
            workload_contract_id: "workload-v1".to_string(),
            state_mapper_id: "mapper-v1".to_string(),
            clock_alignment_method_id: "ptp-profile-v1".to_string(),
            sample_period_ns: 10,
            target_session_coverage: 0.90,
            minimum_calibration_sessions_per_regime: 9,
            required_regimes: regimes.iter().map(|value| (*value).to_string()).collect(),
            exchangeability_review_evidence_id: "exchangeability-review-v1".to_string(),
            session_exchangeability_assumption: true,
            session_window_duration_ns: 100,
            minimum_session_separation_ns: 10,
            traces_per_session: 1,
            samples_per_trace: 4,
        }
    }

    fn predicted(run_index: usize, scope: PredictionScope) -> PredictedPowerTrace {
        let start = (run_index as u64) * 1_000;
        PredictedPowerTrace {
            trace_id: format!("trace-{run_index:03}"),
            run_id: format!("run-{run_index:03}"),
            prediction_scope: scope,
            calibration_contract: model_contract(),
            training_evidence: Vec::new(),
            held_out_evidence: Vec::new(),
            model_id: "power-state-v1".to_string(),
            model_revision: "rev-a".to_string(),
            hardware_configuration_fingerprint: "hardware-sha".to_string(),
            measurement_boundary_id: "gpu-board-pcc".to_string(),
            workload_contract_id: "workload-v1".to_string(),
            state_mapper_id: "mapper-v1".to_string(),
            state_mapping_evidence_id: format!("mapping-{run_index}"),
            time_alignment_evidence_id: format!("alignment-{run_index}"),
            source_timeline_digest_reference: format!("timeline-{run_index}"),
            sample_period_ns: 10,
            samples: (0..4)
                .map(|index| PredictedPowerSample {
                    timestamp_ns: start + index * 10,
                    state_id: if index < 2 { "idle" } else { "decode" }.to_string(),
                    source_event_id: format!("event-{run_index}-{index}"),
                    lower_power_mw: 90,
                    median_power_mw: 100,
                    upper_power_mw: 110,
                    basis: if index == 0 {
                        PredictionBasis::InitialStateEnvelope
                    } else {
                        PredictionBasis::OrderedStateTransitionEnvelope
                    },
                })
                .collect(),
            claim_ceiling: "MODELLED_OPERATIONAL_SCENARIO_NOT_HELD_OUT_VALIDATION_OR_MEASUREMENT",
        }
    }

    fn session(
        index: usize,
        regime: &str,
        power_mw: u64,
        scope: PredictionScope,
    ) -> ForecastObservationSession {
        let start = (index as u64) * 1_000;
        let prediction = predicted(index, scope);
        let measured = prediction.samples.iter().map(|sample| PowerTraceSample {
            timestamp_ns: sample.timestamp_ns,
            state_id: sample.state_id.clone(),
            power_mw,
            quality: SampleQuality::Good,
            source_evidence_id: format!("measurement-{index}-{}", sample.timestamp_ns),
        }).collect();
        ForecastObservationSession {
            session_id: format!("session-{index:03}"),
            operating_regime_id: regime.to_string(),
            session_evidence_id: format!("session-evidence-{index}"),
            window_start_ns: start,
            window_end_ns: start + 100,
            traces: vec![ForecastTraceObservation {
                predicted: prediction,
                measured,
                measurement_evidence_id: format!("measurement-set-{index}"),
            }],
        }
    }

    fn calibration_sessions(count: usize, regime: &str) -> Vec<ForecastObservationSession> {
        (0..count)
            .map(|index| session(index, regime, 100 + (index as u64 % 3) * 10, PredictionScope::OperationalScenario))
            .collect()
    }

    fn calibrator() -> super::RegimeForecastCalibrator {
        fit_regime_forecast_calibrator(
            calibration_contract(&["normal"]),
            calibration_sessions(9, "normal"),
        ).unwrap()
    }

    #[test]
    fn conformal_rank_requires_nine_sessions_for_ninety_percent_coverage() {
        assert_eq!(conformal_order_statistic_rank(9, 0.90), Some(9));
        assert_eq!(conformal_order_statistic_rank(8, 0.90), Some(9));
    }

    #[test]
    fn fit_uses_session_maximum_and_exact_nine_session_support() {
        let result = calibrator();
        assert_eq!(result.total_calibration_session_count, 9);
        assert_eq!(result.regimes[0].calibration_session_count, 9);
        assert_eq!(result.regimes[0].conformal_order_statistic_rank, 9);
        assert_eq!(result.regimes[0].additive_interval_correction_mw, 10);
    }

    #[test]
    fn rejects_insufficient_conformal_support() {
        assert!(fit_regime_forecast_calibrator(
            calibration_contract(&["normal"]),
            calibration_sessions(8, "normal"),
        ).is_err());
    }

    #[test]
    fn rejects_missing_required_regime() {
        assert!(fit_regime_forecast_calibrator(
            calibration_contract(&["normal", "hot"]),
            calibration_sessions(9, "normal"),
        ).is_err());
    }

    #[test]
    fn rejects_undeclared_regime() {
        assert!(fit_regime_forecast_calibrator(
            calibration_contract(&["normal"]),
            calibration_sessions(9, "normal")
                .into_iter()
                .chain(std::iter::once(session(9, "hot", 100, PredictionScope::OperationalScenario)))
                .collect(),
        ).is_err());
    }

    #[test]
    fn rejects_held_out_validation_as_conformal_calibration_data() {
        let mut sessions = calibration_sessions(9, "normal");
        sessions[0].traces[0].predicted.prediction_scope = PredictionScope::HeldOutValidation;
        assert!(fit_regime_forecast_calibrator(
            calibration_contract(&["normal"]),
            sessions,
        ).is_err());
    }

    #[test]
    fn rejects_model_revision_mismatch() {
        let mut sessions = calibration_sessions(9, "normal");
        sessions[0].traces[0].predicted.model_revision = "rev-b".to_string();
        assert!(fit_regime_forecast_calibrator(
            calibration_contract(&["normal"]),
            sessions,
        ).is_err());
    }

    #[test]
    fn rejects_measurement_timestamp_or_state_mismatch() {
        let mut sessions = calibration_sessions(9, "normal");
        sessions[0].traces[0].measured[2].timestamp_ns += 1;
        assert!(fit_regime_forecast_calibrator(
            calibration_contract(&["normal"]),
            sessions,
        ).is_err());
    }

    #[test]
    fn rejects_non_good_measurement_quality() {
        let mut sessions = calibration_sessions(9, "normal");
        sessions[0].traces[0].measured[1].quality = SampleQuality::Missing;
        assert!(fit_regime_forecast_calibrator(
            calibration_contract(&["normal"]),
            sessions,
        ).is_err());
    }

    #[test]
    fn rejects_duplicate_session_identity() {
        let mut sessions = calibration_sessions(9, "normal");
        sessions[1].session_id = sessions[0].session_id.clone();
        assert!(fit_regime_forecast_calibrator(
            calibration_contract(&["normal"]),
            sessions,
        ).is_err());
    }

    #[test]
    fn rejects_unequal_session_exposure() {
        let mut sessions = calibration_sessions(9, "normal");
        sessions[0].window_end_ns += 1;
        assert!(fit_regime_forecast_calibrator(
            calibration_contract(&["normal"]),
            sessions,
        ).is_err());
    }

    #[test]
    fn unknown_regime_is_out_of_domain_and_not_given_a_calibrated_interval() {
        let result = apply_regime_forecast_calibration(
            &calibrator(),
            "hot",
            &predicted(20, PredictionScope::OperationalScenario),
        );
        assert!(result.is_err());
    }

    #[test]
    fn calibrated_interval_expands_by_the_regime_specific_correction() {
        let result = apply_regime_forecast_calibration(
            &calibrator(),
            "normal",
            &predicted(20, PredictionScope::OperationalScenario),
        ).unwrap();
        assert_eq!(result.samples[0].lower_power_mw, 80);
        assert_eq!(result.samples[0].upper_power_mw, 120);
        assert_eq!(result.target_session_coverage, 0.90);
    }

    #[test]
    fn monitoring_reports_out_of_domain_without_crediting_coverage() {
        let report = evaluate_forecast_monitoring(
            &calibrator(),
            ForecastMonitoringContract {
                minimum_observed_sessions_per_regime: 5,
                minimum_wilson_lower_bound_session_coverage: 0.80,
                one_sided_confidence_z_score: 1.281_551_565_5,
            },
            vec![session(20, "hot", 100, PredictionScope::OperationalScenario)],
        ).unwrap();
        assert_eq!(report.out_of_domain_session_count, 1);
        assert_eq!(report.regime_health[0].status, ForecastHealthStatus::OutOfDomain);
        assert_eq!(report.regime_health[0].empirical_session_coverage, None);
    }

    #[test]
    fn monitoring_marks_low_coverage_as_degraded() {
        let report = evaluate_forecast_monitoring(
            &calibrator(),
            ForecastMonitoringContract {
                minimum_observed_sessions_per_regime: 9,
                minimum_wilson_lower_bound_session_coverage: 0.80,
                one_sided_confidence_z_score: 1.281_551_565_5,
            },
            (20..40)
                .map(|index| session(index, "normal", 10_000, PredictionScope::OperationalScenario))
                .collect(),
        ).unwrap();
        assert_eq!(report.regime_health[0].status, ForecastHealthStatus::Degraded);
    }

    #[test]
    fn monitoring_withholds_nominal_status_below_minimum_session_support() {
        let report = evaluate_forecast_monitoring(
            &calibrator(),
            ForecastMonitoringContract {
                minimum_observed_sessions_per_regime: 10,
                minimum_wilson_lower_bound_session_coverage: 0.50,
                one_sided_confidence_z_score: 1.281_551_565_5,
            },
            (20..29)
                .map(|index| session(index, "normal", 100, PredictionScope::OperationalScenario))
                .collect(),
        ).unwrap();
        assert_eq!(report.regime_health[0].status, ForecastHealthStatus::InsufficientSupport);
    }

    #[test]
    fn monitoring_does_not_mutate_the_frozen_calibrator() {
        let fitted = calibrator();
        let before = fitted.clone();
        let _ = evaluate_forecast_monitoring(
            &fitted,
            ForecastMonitoringContract {
                minimum_observed_sessions_per_regime: 5,
                minimum_wilson_lower_bound_session_coverage: 0.50,
                one_sided_confidence_z_score: 1.281_551_565_5,
            },
            vec![session(20, "normal", 100, PredictionScope::OperationalScenario)],
        ).unwrap();
        assert_eq!(fitted, before);
    }
}
