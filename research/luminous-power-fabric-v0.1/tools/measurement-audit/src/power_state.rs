use crate::AuditError;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalibrationSplit {
    Training,
    HeldOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleQuality {
    Good,
    Missing,
    Saturated,
    ClockUnsynchronized,
    CalibrationInvalid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PowerModelContract {
    pub model_id: String,
    pub model_revision: String,
    /// Canonical digest/reference over accelerator, board/server topology, CPU/memory,
    /// driver, firmware, runtime, model build, clocks, power cap and cooling envelope.
    /// This module checks equality; it does not authenticate the digest.
    pub hardware_configuration_fingerprint: String,
    pub measurement_boundary_id: String,
    pub sensor_id: String,
    pub sensor_calibration_evidence_id: String,
    pub sensor_uncertainty_mw: u64,
    pub dataset_id: String,
    pub dataset_version: String,
    pub dataset_evidence_id: String,
    pub use_authorization_reference: String,
    pub normalization_id: String,
    pub workload_contract_id: String,
    pub state_mapper_id: String,
    pub clock_alignment_method_id: String,
    pub sample_period_ns: u64,
    pub minimum_samples_per_state: usize,
    pub minimum_samples_per_transition: usize,
}

impl PowerModelContract {
    fn validate(&self) -> Result<(), AuditError> {
        for (value, field) in [
            (self.model_id.as_str(), "model_id"),
            (self.model_revision.as_str(), "model_revision"),
            (self.hardware_configuration_fingerprint.as_str(), "hardware_configuration_fingerprint"),
            (self.measurement_boundary_id.as_str(), "measurement_boundary_id"),
            (self.sensor_id.as_str(), "sensor_id"),
            (self.sensor_calibration_evidence_id.as_str(), "sensor_calibration_evidence_id"),
            (self.dataset_id.as_str(), "dataset_id"),
            (self.dataset_version.as_str(), "dataset_version"),
            (self.dataset_evidence_id.as_str(), "dataset_evidence_id"),
            (self.use_authorization_reference.as_str(), "use_authorization_reference"),
            (self.normalization_id.as_str(), "normalization_id"),
            (self.workload_contract_id.as_str(), "workload_contract_id"),
            (self.state_mapper_id.as_str(), "state_mapper_id"),
            (self.clock_alignment_method_id.as_str(), "clock_alignment_method_id"),
        ] {
            if value.trim().is_empty() {
                return Err(AuditError(format!("power model contract {field} must not be blank")));
            }
        }
        if self.sample_period_ns == 0
            || self.minimum_samples_per_state == 0
            || self.minimum_samples_per_transition == 0
        {
            return Err(AuditError(
                "sample period and state/transition support thresholds must be positive".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PowerTraceSample {
    pub timestamp_ns: u64,
    pub state_id: String,
    pub power_mw: u64,
    pub quality: SampleQuality,
    pub source_evidence_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PowerCalibrationRun {
    pub run_id: String,
    /// Split assignment is session-level: all runs in one session must use one split.
    pub session_id: String,
    pub split: CalibrationSplit,
    pub hardware_configuration_fingerprint: String,
    pub measurement_boundary_id: String,
    pub workload_contract_id: String,
    pub state_mapper_id: String,
    pub state_mapping_evidence_id: String,
    pub time_alignment_evidence_id: String,
    pub samples: Vec<PowerTraceSample>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PowerEnvelopeDistribution {
    pub sample_count: usize,
    pub minimum_power_mw: u64,
    pub p10_power_mw: u64,
    pub median_power_mw: u64,
    pub p90_power_mw: u64,
    pub maximum_power_mw: u64,
    pub mean_power_mw: u64,
    pub supported: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatePowerEnvelope {
    pub state_id: String,
    pub distribution: PowerEnvelopeDistribution,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateTransitionPowerEnvelope {
    pub from_state_id: String,
    pub to_state_id: String,
    /// Distribution of the destination sample conditioned on the observed
    /// previous-state -> current-state transition.
    pub distribution: PowerEnvelopeDistribution,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PowerStateModel {
    pub contract: PowerModelContract,
    pub state_envelopes: Vec<StatePowerEnvelope>,
    pub transition_envelopes: Vec<StateTransitionPowerEnvelope>,
    pub training_run_ids: Vec<String>,
    pub training_session_ids: Vec<String>,
    /// Recorded for split audit only. Held-out samples never enter fitted envelopes.
    pub held_out_run_ids: Vec<String>,
    pub held_out_session_ids: Vec<String>,
    pub claim_ceiling: &'static str,
}

fn nearest_rank(values: &[u64], numerator: u128, denominator: u128) -> u64 {
    let mut ordered = values.to_vec();
    ordered.sort_unstable();
    let n = ordered.len() as u128;
    let rank = ((n * numerator + denominator - 1) / denominator).max(1) as usize;
    ordered[rank - 1]
}

fn summarize(values: &[u64], minimum_samples: usize) -> Result<PowerEnvelopeDistribution, AuditError> {
    if values.is_empty() {
        return Err(AuditError("cannot summarize an empty power sample set".to_string()));
    }
    let sum = values.iter().try_fold(0_u128, |total, value| {
        total.checked_add(*value as u128)
            .ok_or_else(|| AuditError("power sample sum overflowed".to_string()))
    })?;
    let mean = u64::try_from(sum / values.len() as u128)
        .map_err(|_| AuditError("mean power does not fit the declared milliwatt type".to_string()))?;
    let minimum = *values.iter().min().expect("non-empty values checked");
    let maximum = *values.iter().max().expect("non-empty values checked");
    Ok(PowerEnvelopeDistribution {
        sample_count: values.len(),
        minimum_power_mw: minimum,
        p10_power_mw: nearest_rank(values, 1, 10),
        median_power_mw: nearest_rank(values, 1, 2),
        p90_power_mw: nearest_rank(values, 9, 10),
        maximum_power_mw: maximum,
        mean_power_mw: mean,
        supported: values.len() >= minimum_samples,
    })
}

fn validate_run(
    contract: &PowerModelContract,
    run: &PowerCalibrationRun,
) -> Result<(), AuditError> {
    for (value, field) in [
        (run.run_id.as_str(), "run_id"),
        (run.session_id.as_str(), "session_id"),
        (run.hardware_configuration_fingerprint.as_str(), "hardware_configuration_fingerprint"),
        (run.measurement_boundary_id.as_str(), "measurement_boundary_id"),
        (run.workload_contract_id.as_str(), "workload_contract_id"),
        (run.state_mapper_id.as_str(), "state_mapper_id"),
        (run.state_mapping_evidence_id.as_str(), "state_mapping_evidence_id"),
        (run.time_alignment_evidence_id.as_str(), "time_alignment_evidence_id"),
    ] {
        if value.trim().is_empty() {
            return Err(AuditError(format!("calibration run {field} must not be blank")));
        }
    }
    if run.hardware_configuration_fingerprint != contract.hardware_configuration_fingerprint
        || run.measurement_boundary_id != contract.measurement_boundary_id
        || run.workload_contract_id != contract.workload_contract_id
        || run.state_mapper_id != contract.state_mapper_id
    {
        return Err(AuditError(format!(
            "calibration run {:?} does not match the exact power-model configuration, boundary, workload or state mapper",
            run.run_id
        )));
    }
    if run.samples.len() < 2 {
        return Err(AuditError(format!(
            "calibration run {:?} needs at least two samples to establish an ordered transition",
            run.run_id
        )));
    }
    let mut previous_time = None;
    for sample in &run.samples {
        if sample.state_id.trim().is_empty() || sample.source_evidence_id.trim().is_empty() {
            return Err(AuditError(format!(
                "calibration run {:?} contains a sample without state or source evidence",
                run.run_id
            )));
        }
        if sample.quality != SampleQuality::Good {
            return Err(AuditError(format!(
                "calibration run {:?} contains a non-GOOD sample; missing, saturated, unsynchronized or uncalibrated samples are not silently omitted",
                run.run_id
            )));
        }
        if let Some(previous) = previous_time {
            let observed = sample.timestamp_ns.checked_sub(previous).ok_or_else(|| {
                AuditError(format!("calibration run {:?} timestamps are not increasing", run.run_id))
            })?;
            if observed != contract.sample_period_ns {
                return Err(AuditError(format!(
                    "calibration run {:?} has a missing, duplicate or irregular sample interval: expected {} ns, observed {} ns",
                    run.run_id, contract.sample_period_ns, observed
                )));
            }
        }
        previous_time = Some(sample.timestamp_ns);
    }
    Ok(())
}

/// Fit state-conditioned empirical power envelopes and ordered state-transition
/// envelopes. Entire sessions, never individual rows, are assigned to train/holdout.
/// Held-out runs are validated and recorded but are not read into either fitted map.
pub fn fit_power_state_model(
    contract: PowerModelContract,
    runs: &[PowerCalibrationRun],
) -> Result<PowerStateModel, AuditError> {
    contract.validate()?;
    if runs.is_empty() {
        return Err(AuditError("power calibration dataset has no runs".to_string()));
    }

    let mut run_ids = HashSet::new();
    let mut session_splits: HashMap<&str, CalibrationSplit> = HashMap::new();
    let mut state_values: BTreeMap<String, Vec<u64>> = BTreeMap::new();
    let mut transition_values: BTreeMap<(String, String), Vec<u64>> = BTreeMap::new();
    let mut training_run_ids = Vec::new();
    let mut training_session_ids = BTreeSet::new();
    let mut held_out_run_ids = Vec::new();
    let mut held_out_session_ids = BTreeSet::new();

    for run in runs {
        validate_run(&contract, run)?;
        if !run_ids.insert(run.run_id.as_str()) {
            return Err(AuditError(format!("duplicate calibration run ID {:?}", run.run_id)));
        }
        if let Some(previous_split) = session_splits.insert(run.session_id.as_str(), run.split) {
            if previous_split != run.split {
                return Err(AuditError(format!(
                    "session {:?} is split across training and held-out data",
                    run.session_id
                )));
            }
        }

        match run.split {
            CalibrationSplit::Training => {
                training_run_ids.push(run.run_id.clone());
                training_session_ids.insert(run.session_id.clone());
                for sample in &run.samples {
                    state_values
                        .entry(sample.state_id.clone())
                        .or_default()
                        .push(sample.power_mw);
                }
                for pair in run.samples.windows(2) {
                    transition_values
                        .entry((pair[0].state_id.clone(), pair[1].state_id.clone()))
                        .or_default()
                        .push(pair[1].power_mw);
                }
            }
            CalibrationSplit::HeldOut => {
                held_out_run_ids.push(run.run_id.clone());
                held_out_session_ids.insert(run.session_id.clone());
            }
        }
    }

    if training_run_ids.is_empty() || held_out_run_ids.is_empty() {
        return Err(AuditError(
            "calibration requires at least one training run and one whole-session held-out run".to_string(),
        ));
    }

    let state_envelopes = state_values
        .iter()
        .map(|(state_id, values)| {
            Ok(StatePowerEnvelope {
                state_id: state_id.clone(),
                distribution: summarize(values, contract.minimum_samples_per_state)?,
            })
        })
        .collect::<Result<Vec<_>, AuditError>>()?;

    let transition_envelopes = transition_values
        .iter()
        .map(|((from_state_id, to_state_id), values)| {
            Ok(StateTransitionPowerEnvelope {
                from_state_id: from_state_id.clone(),
                to_state_id: to_state_id.clone(),
                distribution: summarize(values, contract.minimum_samples_per_transition)?,
            })
        })
        .collect::<Result<Vec<_>, AuditError>>()?;

    training_run_ids.sort();
    held_out_run_ids.sort();
    Ok(PowerStateModel {
        contract,
        state_envelopes,
        transition_envelopes,
        training_run_ids,
        training_session_ids: training_session_ids.into_iter().collect(),
        held_out_run_ids,
        held_out_session_ids: held_out_session_ids.into_iter().collect(),
        claim_ceiling: "CALIBRATED_STATE_CONDITIONAL_POWER_MODEL_NOT_MEASURED_OUTPUT_OR_QUALIFICATION",
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappedStateSample {
    pub timestamp_ns: u64,
    pub state_id: String,
    pub source_event_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappedStateTrace {
    pub trace_id: String,
    pub run_id: String,
    pub hardware_configuration_fingerprint: String,
    pub measurement_boundary_id: String,
    pub workload_contract_id: String,
    pub state_mapper_id: String,
    pub state_mapping_evidence_id: String,
    pub time_alignment_evidence_id: String,
    pub source_timeline_digest_reference: String,
    pub samples: Vec<MappedStateSample>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PredictionBasis {
    InitialStateEnvelope,
    OrderedStateTransitionEnvelope,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredictedPowerSample {
    pub timestamp_ns: u64,
    pub state_id: String,
    pub source_event_id: String,
    pub lower_power_mw: u64,
    pub median_power_mw: u64,
    pub upper_power_mw: u64,
    pub basis: PredictionBasis,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredictedPowerTrace {
    pub trace_id: String,
    pub run_id: String,
    pub model_id: String,
    pub model_revision: String,
    pub hardware_configuration_fingerprint: String,
    pub measurement_boundary_id: String,
    pub workload_contract_id: String,
    pub state_mapper_id: String,
    pub time_alignment_evidence_id: String,
    pub source_timeline_digest_reference: String,
    pub sample_period_ns: u64,
    pub samples: Vec<PredictedPowerSample>,
    pub claim_ceiling: &'static str,
}

fn find_state_envelope<'a>(model: &'a PowerStateModel, state_id: &str) -> Option<&'a PowerEnvelopeDistribution> {
    model.state_envelopes
        .iter()
        .find(|entry| entry.state_id == state_id)
        .map(|entry| &entry.distribution)
}

fn find_transition_envelope<'a>(
    model: &'a PowerStateModel,
    from_state_id: &str,
    to_state_id: &str,
) -> Option<&'a PowerEnvelopeDistribution> {
    model.transition_envelopes
        .iter()
        .find(|entry| entry.from_state_id == from_state_id && entry.to_state_id == to_state_id)
        .map(|entry| &entry.distribution)
}

/// Map an independently evidenced state sequence to modelled power envelopes.
/// The input order and timestamps are preserved exactly; no sample shuffling,
/// interpolation across unsupported states, TDP scaling or linear GPU-count scaling occurs.
pub fn predict_power_trace(
    model: &PowerStateModel,
    trace: &MappedStateTrace,
) -> Result<PredictedPowerTrace, AuditError> {
    for (value, field) in [
        (trace.trace_id.as_str(), "trace_id"),
        (trace.run_id.as_str(), "run_id"),
        (trace.hardware_configuration_fingerprint.as_str(), "hardware_configuration_fingerprint"),
        (trace.measurement_boundary_id.as_str(), "measurement_boundary_id"),
        (trace.workload_contract_id.as_str(), "workload_contract_id"),
        (trace.state_mapper_id.as_str(), "state_mapper_id"),
        (trace.state_mapping_evidence_id.as_str(), "state_mapping_evidence_id"),
        (trace.time_alignment_evidence_id.as_str(), "time_alignment_evidence_id"),
        (trace.source_timeline_digest_reference.as_str(), "source_timeline_digest_reference"),
    ] {
        if value.trim().is_empty() {
            return Err(AuditError(format!("mapped state trace {field} must not be blank")));
        }
    }
    let contract = &model.contract;
    if trace.hardware_configuration_fingerprint != contract.hardware_configuration_fingerprint
        || trace.measurement_boundary_id != contract.measurement_boundary_id
        || trace.workload_contract_id != contract.workload_contract_id
        || trace.state_mapper_id != contract.state_mapper_id
    {
        return Err(AuditError(
            "prediction trace is outside the exact calibrated configuration, boundary, workload or state-mapper support".to_string(),
        ));
    }
    if trace.samples.is_empty() {
        return Err(AuditError("mapped state trace contains no samples".to_string()));
    }

    let mut predicted = Vec::with_capacity(trace.samples.len());
    let mut previous_time = None;
    let mut previous_state: Option<&str> = None;
    for sample in &trace.samples {
        if sample.state_id.trim().is_empty() || sample.source_event_id.trim().is_empty() {
            return Err(AuditError("mapped state sample is missing state or source-event identity".to_string()));
        }
        if let Some(previous) = previous_time {
            let observed = sample.timestamp_ns.checked_sub(previous).ok_or_else(|| {
                AuditError("mapped state timestamps are not strictly increasing".to_string())
            })?;
            if observed != contract.sample_period_ns {
                return Err(AuditError(format!(
                    "mapped state trace has a missing, duplicate or irregular sample interval: expected {} ns, observed {} ns",
                    contract.sample_period_ns, observed
                )));
            }
        }

        let (distribution, basis) = match previous_state {
            None => (
                find_state_envelope(model, &sample.state_id).ok_or_else(|| {
                    AuditError(format!("state {:?} has no calibration support", sample.state_id))
                })?,
                PredictionBasis::InitialStateEnvelope,
            ),
            Some(previous) => (
                find_transition_envelope(model, previous, &sample.state_id).ok_or_else(|| {
                    AuditError(format!(
                        "state transition {:?} -> {:?} has no calibration support",
                        previous, sample.state_id
                    ))
                })?,
                PredictionBasis::OrderedStateTransitionEnvelope,
            ),
        };
        if !distribution.supported {
            return Err(AuditError(format!(
                "state/transition support is insufficient: {} samples, minimum {}",
                distribution.sample_count,
                match basis {
                    PredictionBasis::InitialStateEnvelope => contract.minimum_samples_per_state,
                    PredictionBasis::OrderedStateTransitionEnvelope => contract.minimum_samples_per_transition,
                }
            )));
        }
        let upper_power_mw = distribution
            .p90_power_mw
            .checked_add(contract.sensor_uncertainty_mw)
            .ok_or_else(|| AuditError("upper prediction interval overflowed".to_string()))?;
        predicted.push(PredictedPowerSample {
            timestamp_ns: sample.timestamp_ns,
            state_id: sample.state_id.clone(),
            source_event_id: sample.source_event_id.clone(),
            lower_power_mw: distribution.p10_power_mw.saturating_sub(contract.sensor_uncertainty_mw),
            median_power_mw: distribution.median_power_mw,
            upper_power_mw,
            basis,
        });
        previous_time = Some(sample.timestamp_ns);
        previous_state = Some(sample.state_id.as_str());
    }

    Ok(PredictedPowerTrace {
        trace_id: trace.trace_id.clone(),
        run_id: trace.run_id.clone(),
        model_id: contract.model_id.clone(),
        model_revision: contract.model_revision.clone(),
        hardware_configuration_fingerprint: contract.hardware_configuration_fingerprint.clone(),
        measurement_boundary_id: contract.measurement_boundary_id.clone(),
        workload_contract_id: contract.workload_contract_id.clone(),
        state_mapper_id: contract.state_mapper_id.clone(),
        time_alignment_evidence_id: trace.time_alignment_evidence_id.clone(),
        source_timeline_digest_reference: trace.source_timeline_digest_reference.clone(),
        sample_period_ns: contract.sample_period_ns,
        samples: predicted,
        claim_ceiling: "MODELLED_CONDITIONAL_POWER_ENVELOPE_NOT_MEASUREMENT_OR_GRID_QUALIFICATION",
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct PowerValidationMetrics {
    pub sample_count: usize,
    /// Relative error of the integrated power-time sums; unit factors cancel.
    pub absolute_energy_error_percent: Option<f64>,
    pub absolute_mean_power_error_percent: Option<f64>,
    pub absolute_peak_power_error_percent: Option<f64>,
    pub mean_absolute_ramp_error_mw: Option<f64>,
    pub lag_one_autocorrelation_absolute_error: Option<f64>,
    pub prediction_interval_coverage_fraction: f64,
    pub mean_prediction_interval_width_mw: f64,
    pub claim_ceiling: &'static str,
}

fn lag_one_autocorrelation(values: &[u64]) -> Option<f64> {
    if values.len() < 3 {
        return None;
    }
    let mean = values.iter().map(|value| *value as f64).sum::<f64>() / values.len() as f64;
    let denominator = values
        .iter()
        .map(|value| {
            let centered = *value as f64 - mean;
            centered * centered
        })
        .sum::<f64>();
    if denominator <= f64::EPSILON {
        return None;
    }
    let numerator = values.windows(2).map(|pair| {
        (pair[0] as f64 - mean) * (pair[1] as f64 - mean)
    }).sum::<f64>();
    Some(numerator / denominator)
}

/// Compare a predicted trace against a separate held-out measured run.
/// Exact timestamps, state labels, sample count and sample cadence must match.
pub fn evaluate_held_out_power_trace(
    predicted: &PredictedPowerTrace,
    measured_run_id: &str,
    measured: &[PowerTraceSample],
) -> Result<PowerValidationMetrics, AuditError> {
    if measured_run_id != predicted.run_id {
        return Err(AuditError("held-out measurement run ID does not match the predicted trace".to_string()));
    }
    if measured.len() != predicted.samples.len() || measured.is_empty() {
        return Err(AuditError("held-out measured and predicted sample counts must match and be non-zero".to_string()));
    }

    let mut measured_energy = 0_u128;
    let mut predicted_energy = 0_u128;
    let mut measured_power_sum = 0_u128;
    let mut predicted_power_sum = 0_u128;
    let mut measured_peak = 0_u64;
    let mut predicted_peak = 0_u64;
    let mut covered = 0_usize;
    let mut interval_width_sum = 0_u128;
    let mut ramp_error_sum = 0_u128;

    for (index, (actual, estimate)) in measured.iter().zip(predicted.samples.iter()).enumerate() {
        if actual.quality != SampleQuality::Good {
            return Err(AuditError(format!(
                "held-out run contains non-GOOD measurement at sample {index}; missing or invalid samples cannot be silently dropped"
            )));
        }
        if actual.timestamp_ns != estimate.timestamp_ns || actual.state_id != estimate.state_id {
            return Err(AuditError(format!(
                "held-out sample {index} does not align exactly with the predicted timestamp/state sequence"
            )));
        }
        if actual.source_evidence_id.trim().is_empty() {
            return Err(AuditError(format!("held-out sample {index} has no source evidence ID")));
        }
        measured_energy = measured_energy
            .checked_add((actual.power_mw as u128).checked_mul(predicted.sample_period_ns as u128)
                .ok_or_else(|| AuditError("measured energy integral overflowed".to_string()))?)
            .ok_or_else(|| AuditError("measured energy sum overflowed".to_string()))?;
        predicted_energy = predicted_energy
            .checked_add((estimate.median_power_mw as u128).checked_mul(predicted.sample_period_ns as u128)
                .ok_or_else(|| AuditError("predicted energy integral overflowed".to_string()))?)
            .ok_or_else(|| AuditError("predicted energy sum overflowed".to_string()))?;
        measured_power_sum = measured_power_sum.checked_add(actual.power_mw as u128)
            .ok_or_else(|| AuditError("measured mean-power sum overflowed".to_string()))?;
        predicted_power_sum = predicted_power_sum.checked_add(estimate.median_power_mw as u128)
            .ok_or_else(|| AuditError("predicted mean-power sum overflowed".to_string()))?;
        measured_peak = measured_peak.max(actual.power_mw);
        predicted_peak = predicted_peak.max(estimate.median_power_mw);
        if estimate.lower_power_mw <= actual.power_mw && actual.power_mw <= estimate.upper_power_mw {
            covered += 1;
        }
        interval_width_sum = interval_width_sum.checked_add(
            (estimate.upper_power_mw - estimate.lower_power_mw) as u128
        ).ok_or_else(|| AuditError("prediction interval width sum overflowed".to_string()))?;

        if index > 0 {
            let actual_delta = actual.power_mw as i128 - measured[index - 1].power_mw as i128;
            let predicted_delta = estimate.median_power_mw as i128
                - predicted.samples[index - 1].median_power_mw as i128;
            ramp_error_sum = ramp_error_sum.checked_add((actual_delta - predicted_delta).unsigned_abs())
                .ok_or_else(|| AuditError("ramp error sum overflowed".to_string()))?;
        }
    }

    let relative_error = |estimate: u128, actual: u128| -> Option<f64> {
        if actual == 0 {
            None
        } else {
            Some((estimate as f64 - actual as f64).abs() / actual as f64 * 100.0)
        }
    };
    let measured_values: Vec<u64> = measured.iter().map(|sample| sample.power_mw).collect();
    let predicted_values: Vec<u64> = predicted.samples.iter().map(|sample| sample.median_power_mw).collect();
    let autocorrelation_error = lag_one_autocorrelation(&measured_values)
        .zip(lag_one_autocorrelation(&predicted_values))
        .map(|(actual, estimate)| (actual - estimate).abs());

    Ok(PowerValidationMetrics {
        sample_count: measured.len(),
        absolute_energy_error_percent: relative_error(predicted_energy, measured_energy),
        absolute_mean_power_error_percent: relative_error(
            predicted_power_sum / measured.len() as u128,
            measured_power_sum / measured.len() as u128,
        ),
        absolute_peak_power_error_percent: relative_error(predicted_peak as u128, measured_peak as u128),
        mean_absolute_ramp_error_mw: if measured.len() < 2 {
            None
        } else {
            Some(ramp_error_sum as f64 / (measured.len() - 1) as f64)
        },
        lag_one_autocorrelation_absolute_error: autocorrelation_error,
        prediction_interval_coverage_fraction: covered as f64 / measured.len() as f64,
        mean_prediction_interval_width_mw: interval_width_sum as f64 / measured.len() as f64,
        claim_ceiling: "HELD_OUT_TRACE_METRICS_NOT_INDEPENDENT_REPRODUCTION_OR_QUALIFICATION",
    })
}

#[cfg(test)]
mod tests {
    use super::{
        evaluate_held_out_power_trace, fit_power_state_model, predict_power_trace,
        CalibrationSplit, MappedStateSample, MappedStateTrace, PowerCalibrationRun,
        PowerModelContract, PowerTraceSample, SampleQuality,
    };

    fn contract() -> PowerModelContract {
        PowerModelContract {
            model_id: "state-envelope-v1".to_string(),
            model_revision: "git:fixture".to_string(),
            hardware_configuration_fingerprint: "sha256:exact-config".to_string(),
            measurement_boundary_id: "server-input-pdu-01".to_string(),
            sensor_id: "power-sensor-01".to_string(),
            sensor_calibration_evidence_id: "calibration:fixture".to_string(),
            sensor_uncertainty_mw: 5,
            dataset_id: "dataset-fixture".to_string(),
            dataset_version: "v1".to_string(),
            dataset_evidence_id: "dataset-evidence:fixture".to_string(),
            use_authorization_reference: "rights:fixture".to_string(),
            normalization_id: "normalization-v1".to_string(),
            workload_contract_id: "llm-serving-contract-v1".to_string(),
            state_mapper_id: "prefill-decode-idle-v1".to_string(),
            clock_alignment_method_id: "ptp-alignment-v1".to_string(),
            sample_period_ns: 10,
            minimum_samples_per_state: 1,
            minimum_samples_per_transition: 1,
        }
    }

    fn sample(time: u64, state: &str, power: u64) -> PowerTraceSample {
        PowerTraceSample {
            timestamp_ns: time,
            state_id: state.to_string(),
            power_mw: power,
            quality: SampleQuality::Good,
            source_evidence_id: format!("sensor-row-{time}"),
        }
    }

    fn run(run_id: &str, session_id: &str, split: CalibrationSplit, samples: Vec<PowerTraceSample>) -> PowerCalibrationRun {
        PowerCalibrationRun {
            run_id: run_id.to_string(),
            session_id: session_id.to_string(),
            split,
            hardware_configuration_fingerprint: "sha256:exact-config".to_string(),
            measurement_boundary_id: "server-input-pdu-01".to_string(),
            workload_contract_id: "llm-serving-contract-v1".to_string(),
            state_mapper_id: "prefill-decode-idle-v1".to_string(),
            state_mapping_evidence_id: format!("mapping:{run_id}"),
            time_alignment_evidence_id: format!("alignment:{run_id}"),
            samples,
        }
    }

    fn training_and_holdout() -> Vec<PowerCalibrationRun> {
        vec![
            run("train-a", "session-train", CalibrationSplit::Training, vec![
                sample(0, "idle", 100),
                sample(10, "idle", 110),
                sample(20, "prefill", 400),
                sample(30, "prefill", 420),
                sample(40, "decode", 250),
                sample(50, "decode", 240),
            ]),
            run("heldout-a", "session-heldout", CalibrationSplit::HeldOut, vec![
                sample(0, "idle", 900),
                sample(10, "idle", 910),
                sample(20, "prefill", 1_400),
                sample(30, "prefill", 1_420),
                sample(40, "decode", 1_250),
                sample(50, "decode", 1_240),
            ]),
        ]
    }

    fn mapped_trace(run_id: &str) -> MappedStateTrace {
        MappedStateTrace {
            trace_id: format!("trace-{run_id}"),
            run_id: run_id.to_string(),
            hardware_configuration_fingerprint: "sha256:exact-config".to_string(),
            measurement_boundary_id: "server-input-pdu-01".to_string(),
            workload_contract_id: "llm-serving-contract-v1".to_string(),
            state_mapper_id: "prefill-decode-idle-v1".to_string(),
            state_mapping_evidence_id: "mapping:heldout".to_string(),
            time_alignment_evidence_id: "alignment:heldout".to_string(),
            source_timeline_digest_reference: "sha256:source-timeline".to_string(),
            samples: vec![
                MappedStateSample { timestamp_ns: 0, state_id: "idle".to_string(), source_event_id: "event-0".to_string() },
                MappedStateSample { timestamp_ns: 10, state_id: "idle".to_string(), source_event_id: "event-1".to_string() },
                MappedStateSample { timestamp_ns: 20, state_id: "prefill".to_string(), source_event_id: "event-2".to_string() },
                MappedStateSample { timestamp_ns: 30, state_id: "prefill".to_string(), source_event_id: "event-3".to_string() },
                MappedStateSample { timestamp_ns: 40, state_id: "decode".to_string(), source_event_id: "event-4".to_string() },
                MappedStateSample { timestamp_ns: 50, state_id: "decode".to_string(), source_event_id: "event-5".to_string() },
            ],
        }
    }

    #[test]
    fn held_out_sessions_do_not_contribute_to_fitted_power_envelopes() {
        let model = fit_power_state_model(contract(), &training_and_holdout()).unwrap();
        let idle = model.state_envelopes.iter().find(|item| item.state_id == "idle").unwrap();
        assert_eq!(idle.distribution.maximum_power_mw, 110);
        assert_eq!(model.training_run_ids, vec!["train-a"]);
        assert_eq!(model.held_out_run_ids, vec!["heldout-a"]);
    }

    #[test]
    fn prediction_preserves_state_order_and_uses_ordered_transition_cells() {
        let model = fit_power_state_model(contract(), &training_and_holdout()).unwrap();
        let trace = predict_power_trace(&model, &mapped_trace("heldout-a")).unwrap();
        assert_eq!(trace.samples.len(), 6);
        assert_eq!(trace.samples[0].median_power_mw, 100);
        assert_eq!(trace.samples[2].basis, super::PredictionBasis::OrderedStateTransitionEnvelope);
        assert_eq!(trace.samples[2].state_id, "prefill");
        assert_eq!(trace.samples[4].state_id, "decode");
        assert_eq!(trace.claim_ceiling, "MODELLED_CONDITIONAL_POWER_ENVELOPE_NOT_MEASUREMENT_OR_GRID_QUALIFICATION");
    }

    #[test]
    fn exact_configuration_mismatch_fails_closed() {
        let model = fit_power_state_model(contract(), &training_and_holdout()).unwrap();
        let mut trace = mapped_trace("heldout-a");
        trace.hardware_configuration_fingerprint = "sha256:other-gpu-or-power-cap".to_string();
        assert!(predict_power_trace(&model, &trace).is_err());
    }

    #[test]
    fn uncalibrated_state_transition_fails_closed() {
        let model = fit_power_state_model(contract(), &training_and_holdout()).unwrap();
        let mut trace = mapped_trace("heldout-a");
        trace.samples[2].state_id = "unknown-state".to_string();
        assert!(predict_power_trace(&model, &trace).is_err());
    }

    #[test]
    fn non_good_calibration_samples_are_not_silently_dropped() {
        let mut runs = training_and_holdout();
        runs[0].samples[1].quality = SampleQuality::Saturated;
        assert!(fit_power_state_model(contract(), &runs).is_err());
    }

    #[test]
    fn irregular_sampling_fails_closed() {
        let mut runs = training_and_holdout();
        runs[0].samples[1].timestamp_ns = 11;
        assert!(fit_power_state_model(contract(), &runs).is_err());
    }

    #[test]
    fn one_session_cannot_be_split_between_training_and_holdout() {
        let mut runs = training_and_holdout();
        runs[1].session_id = "session-train".to_string();
        assert!(fit_power_state_model(contract(), &runs).is_err());
    }

    #[test]
    fn exact_holdout_alignment_and_quality_are_required_for_metrics() {
        let model = fit_power_state_model(contract(), &training_and_holdout()).unwrap();
        let prediction = predict_power_trace(&model, &mapped_trace("heldout-a")).unwrap();
        let heldout = vec![
            sample(0, "idle", 900),
            sample(10, "idle", 910),
            sample(20, "prefill", 1_400),
            sample(30, "prefill", 1_420),
            sample(40, "decode", 1_250),
            sample(50, "decode", 1_240),
        ];
        assert!(evaluate_held_out_power_trace(&prediction, "wrong-run", &heldout).is_err());
        let mut bad = heldout.clone();
        bad[2].timestamp_ns += 1;
        assert!(evaluate_held_out_power_trace(&prediction, "heldout-a", &bad).is_err());
        let mut missing = heldout;
        missing[3].quality = SampleQuality::Missing;
        assert!(evaluate_held_out_power_trace(&prediction, "heldout-a", &missing).is_err());
    }

    #[test]
    fn heldout_metrics_report_energy_peak_ramp_interval_and_autocorrelation() {
        let model = fit_power_state_model(contract(), &training_and_holdout()).unwrap();
        let prediction = predict_power_trace(&model, &mapped_trace("heldout-a")).unwrap();
        let heldout = vec![
            sample(0, "idle", 900),
            sample(10, "idle", 910),
            sample(20, "prefill", 1_400),
            sample(30, "prefill", 1_420),
            sample(40, "decode", 1_250),
            sample(50, "decode", 1_240),
        ];
        let metrics = evaluate_held_out_power_trace(&prediction, "heldout-a", &heldout).unwrap();
        assert_eq!(metrics.sample_count, 6);
        assert!(metrics.absolute_energy_error_percent.unwrap().is_finite());
        assert!(metrics.absolute_peak_power_error_percent.unwrap().is_finite());
        assert!(metrics.mean_absolute_ramp_error_mw.unwrap().is_finite());
        assert!(metrics.prediction_interval_coverage_fraction.is_finite());
        assert!(metrics.mean_prediction_interval_width_mw >= 0.0);
        assert_eq!(metrics.claim_ceiling, "HELD_OUT_TRACE_METRICS_NOT_INDEPENDENT_REPRODUCTION_OR_QUALIFICATION");
    }

    #[test]
    fn no_prediction_is_emitted_for_an_unsupported_transition_cell() {
        let model = fit_power_state_model(contract(), &training_and_holdout()).unwrap();
        let mut trace = mapped_trace("heldout-a");
        trace.samples[4].state_id = "idle".to_string();
        // Training has decode -> decode, but not decode -> idle.
        assert!(predict_power_trace(&model, &trace).is_err());
    }
}
