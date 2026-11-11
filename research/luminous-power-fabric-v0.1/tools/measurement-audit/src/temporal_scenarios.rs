use crate::power_state::{PowerModelContract, SampleQuality};
use crate::AuditError;
use std::collections::HashSet;

/// Scenario generation arms used in the matched forecast-to-reserve benchmark.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenarioGeneratorKind {
    /// Independently permutes each horizon step's marginal quantile samples.
    IndependentQuantileSampling,
    /// Adds a complete historical forecast-error trajectory to the median path.
    EmpiricalResidualTrajectoryBootstrap,
    /// Transfers historical cross-horizon residual ranks onto calibrated
    /// forecast marginal quantile samples (Schaake-shuffle style).
    SchaakeShuffle,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForecastQuantile {
    pub probability: f64,
    pub power_mw: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForecastHorizonStep {
    /// End timestamp of the interval-average power value for
    /// [timestamp_ns - step_period_ns, timestamp_ns).
    pub timestamp_ns: u64,
    pub quantiles: Vec<ForecastQuantile>,
}

/// A full-horizon probabilistic forecast tied to one exact source model contract.
#[derive(Debug, Clone, PartialEq)]
pub struct ProbabilisticPowerForecast {
    pub forecast_id: String,
    pub origin_id: String,
    pub origin_timestamp_ns: u64,
    pub clock_id: String,
    pub time_origin_id: String,
    pub operating_regime_id: String,
    pub model_contract: PowerModelContract,
    pub forecast_evidence_id: String,
    pub state_mapping_evidence_id: String,
    pub time_alignment_evidence_id: String,
    pub source_timeline_digest_reference: String,
    pub step_period_ns: u64,
    pub steps: Vec<ForecastHorizonStep>,
}

/// One complete historical forecast-error trajectory.
///
/// Residual sign convention is measured power minus the historical median
/// forecast. The complete target horizon must have been observed before the
/// current forecast origin. The whole vector is selected as one scenario block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForecastResidualTrajectory {
    pub residual_id: String,
    pub origin_id: String,
    pub operating_regime_id: String,
    pub origin_timestamp_ns: u64,
    pub target_window_end_ns: u64,
    pub clock_id: String,
    pub time_origin_id: String,
    pub model_contract: PowerModelContract,
    pub forecast_evidence_id: String,
    pub measurement_evidence_id: String,
    pub residual_generation_evidence_id: String,
    pub state_mapping_evidence_id: String,
    pub time_alignment_evidence_id: String,
    pub source_timeline_digest_reference: String,
    pub source_sample_timestamps_ns: Vec<u64>,
    pub source_sample_evidence_ids: Vec<String>,
    pub residual_power_mw: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioGenerationContract {
    pub generator_id: String,
    pub generator_revision: String,
    pub generator_kind: ScenarioGeneratorKind,
    pub model_contract: PowerModelContract,
    pub operating_regime_id: String,
    pub clock_id: String,
    pub time_origin_id: String,
    pub step_period_ns: u64,
    pub horizon_steps: usize,
    pub scenario_count: usize,
    pub deterministic_seed: u64,
    /// Required for residual bootstrap and Schaake shuffle; absent for the
    /// independent-quantile negative-control arm.
    pub residual_pool_evidence_id: Option<String>,
    pub minimum_residual_trajectory_count: usize,
    /// Minimum separation between historical forecast origins. For the
    /// dependence-preserving arms this must be at least one full target horizon
    /// so sampled historical target windows do not overlap.
    pub minimum_residual_origin_separation_ns: u64,
}

impl ScenarioGenerationContract {
    fn validate(&self) -> Result<(), AuditError> {
        for (value, field) in [
            (self.generator_id.as_str(), "generator_id"),
            (self.generator_revision.as_str(), "generator_revision"),
            (self.operating_regime_id.as_str(), "operating_regime_id"),
            (self.clock_id.as_str(), "clock_id"),
            (self.time_origin_id.as_str(), "time_origin_id"),
            (
                self.model_contract.model_id.as_str(),
                "model_contract.model_id",
            ),
            (
                self.model_contract.model_revision.as_str(),
                "model_contract.model_revision",
            ),
            (
                self.model_contract.hardware_configuration_fingerprint.as_str(),
                "model_contract.hardware_configuration_fingerprint",
            ),
            (self.model_contract.sensor_id.as_str(), "model_contract.sensor_id"),
            (
                self.model_contract.sensor_calibration_evidence_id.as_str(),
                "model_contract.sensor_calibration_evidence_id",
            ),
            (self.model_contract.dataset_id.as_str(), "model_contract.dataset_id"),
            (self.model_contract.dataset_version.as_str(), "model_contract.dataset_version"),
            (self.model_contract.normalization_id.as_str(), "model_contract.normalization_id"),
            (
                self.model_contract.measurement_boundary_id.as_str(),
                "model_contract.measurement_boundary_id",
            ),
            (
                self.model_contract.workload_contract_id.as_str(),
                "model_contract.workload_contract_id",
            ),
            (
                self.model_contract.state_mapper_id.as_str(),
                "model_contract.state_mapper_id",
            ),
            (
                self.model_contract.clock_alignment_method_id.as_str(),
                "model_contract.clock_alignment_method_id",
            ),
            (
                self.model_contract.dataset_evidence_id.as_str(),
                "model_contract.dataset_evidence_id",
            ),
            (
                self.model_contract.use_authorization_reference.as_str(),
                "model_contract.use_authorization_reference",
            ),
        ] {
            if value.trim().is_empty() {
                return Err(AuditError(format!(
                    "scenario-generation contract {field} must not be blank"
                )));
            }
        }
        if self.step_period_ns == 0
            || self.horizon_steps < 2
            || self.horizon_steps > 288
            || !(2..=512).contains(&self.scenario_count)
        {
            return Err(AuditError(
                "scenario generation requires positive cadence, 2..=288 horizon steps and 2..=512 scenarios"
                    .to_string(),
            ));
        }
        match self.generator_kind {
            ScenarioGeneratorKind::IndependentQuantileSampling => {
                if self.minimum_residual_trajectory_count != 0
                    || self.minimum_residual_origin_separation_ns != 0
                    || self.residual_pool_evidence_id.is_some()
                {
                    return Err(AuditError(
                        "independent quantile sampling must not claim a residual pool".to_string(),
                    ));
                }
            }
            ScenarioGeneratorKind::EmpiricalResidualTrajectoryBootstrap => {
                let horizon_duration_ns = (self.horizon_steps as u64)
                    .checked_mul(self.step_period_ns)
                    .ok_or_else(|| AuditError("scenario horizon duration overflowed".to_string()))?;
                if self.minimum_residual_trajectory_count < 2
                    || self.minimum_residual_origin_separation_ns < horizon_duration_ns
                    || self
                        .residual_pool_evidence_id
                        .as_deref()
                        .is_none_or(|value| value.trim().is_empty())
                {
                    return Err(AuditError(
                        "residual bootstrap requires a residual-pool evidence reference and at least two historical trajectories"
                            .to_string(),
                    ));
                }
            }
            ScenarioGeneratorKind::SchaakeShuffle => {
                let horizon_duration_ns = (self.horizon_steps as u64)
                    .checked_mul(self.step_period_ns)
                    .ok_or_else(|| AuditError("scenario horizon duration overflowed".to_string()))?;
                if self.minimum_residual_trajectory_count < self.scenario_count
                    || self.minimum_residual_origin_separation_ns < horizon_duration_ns
                    || self
                        .residual_pool_evidence_id
                        .as_deref()
                        .is_none_or(|value| value.trim().is_empty())
                {
                    return Err(AuditError(
                        "Schaake shuffle requires a residual-pool evidence reference and at least one historical rank row per scenario"
                            .to_string(),
                    ));
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForecastScenarioTrajectory {
    pub scenario_id: String,
    pub source_residual_id: Option<String>,
    pub source_residual_evidence_id: Option<String>,
    pub source_residual_forecast_evidence_id: Option<String>,
    pub source_residual_generation_evidence_id: Option<String>,
    pub source_residual_state_mapping_evidence_id: Option<String>,
    pub source_residual_time_alignment_evidence_id: Option<String>,
    pub source_residual_timeline_digest_reference: Option<String>,
    /// Exact measured-sample evidence references for the selected residual block.
    /// Empty only for the independent-quantile negative-control arm.
    pub source_sample_evidence_ids: Vec<String>,
    pub source_sample_timestamps_ns: Vec<u64>,
    pub power_mw: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForecastScenarioSet {
    pub contract: ScenarioGenerationContract,
    pub forecast_id: String,
    pub forecast_origin_timestamp_ns: u64,
    pub clock_id: String,
    pub time_origin_id: String,
    pub forecast_evidence_id: String,
    pub state_mapping_evidence_id: String,
    pub time_alignment_evidence_id: String,
    pub source_timeline_digest_reference: String,
    pub step_timestamps_ns: Vec<u64>,
    pub scheduled_median_power_mw: Vec<u64>,
    pub scenarios: Vec<ForecastScenarioTrajectory>,
    pub selected_residual_ids: Vec<String>,
    pub selected_residual_evidence_ids: Vec<String>,
    pub selected_residual_forecast_evidence_ids: Vec<String>,
    pub selected_residual_generation_evidence_ids: Vec<String>,
    pub selected_residual_state_mapping_evidence_ids: Vec<String>,
    pub selected_residual_time_alignment_evidence_ids: Vec<String>,
    pub selected_residual_timeline_digest_references: Vec<String>,
    pub marginal_tail_clipped_sample_count: usize,
    pub residual_floor_clamped_sample_count: usize,
    pub claim_ceiling: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasuredForecastHorizonSample {
    pub timestamp_ns: u64,
    pub power_mw: u64,
    pub quality: SampleQuality,
    pub source_evidence_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasuredForecastHorizon {
    pub forecast_id: String,
    pub measurement_run_id: String,
    pub clock_id: String,
    pub time_origin_id: String,
    pub measurement_evidence_id: String,
    pub time_alignment_evidence_id: String,
    pub source_timeline_digest_reference: String,
    pub model_contract: PowerModelContract,
    pub samples: Vec<MeasuredForecastHorizonSample>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioEvaluationReport {
    pub forecast_id: String,
    pub model_contract: PowerModelContract,
    pub generator_id: String,
    pub generator_revision: String,
    pub generator_kind: ScenarioGeneratorKind,
    pub forecast_evidence_id: String,
    pub clock_id: String,
    pub time_origin_id: String,
    pub state_mapping_evidence_id: String,
    pub forecast_time_alignment_evidence_id: String,
    pub forecast_source_timeline_digest_reference: String,
    pub residual_pool_evidence_id: Option<String>,
    pub selected_residual_ids: Vec<String>,
    pub selected_residual_evidence_ids: Vec<String>,
    pub selected_residual_forecast_evidence_ids: Vec<String>,
    pub selected_residual_generation_evidence_ids: Vec<String>,
    pub selected_residual_state_mapping_evidence_ids: Vec<String>,
    pub selected_residual_time_alignment_evidence_ids: Vec<String>,
    pub selected_residual_timeline_digest_references: Vec<String>,
    pub selected_residual_sample_evidence_ids: Vec<Vec<String>>,
    pub selected_residual_sample_timestamps_ns: Vec<Vec<u64>>,
    pub scenario_count: usize,
    pub horizon_steps: usize,
    pub energy_score_mw: f64,
    pub variogram_score: f64,
    /// Integrated measured horizon energy in mW·ns, using interval-average
    /// power and the frozen forecast step duration.
    pub actual_energy_mw_ns: u128,
    pub scenario_energy_p10_mw_ns: u128,
    pub scenario_energy_p50_mw_ns: u128,
    pub scenario_energy_p90_mw_ns: u128,
    pub actual_horizon_energy_pit: f64,
    pub actual_peak_power_mw: u64,
    pub actual_max_ramp_up_mw_per_step: u64,
    pub actual_max_ramp_down_mw_per_step: u64,
    pub actual_horizon_max_surplus_mw: u64,
    pub scenario_peak_p10_mw: u64,
    pub scenario_peak_p50_mw: u64,
    pub scenario_peak_p90_mw: u64,
    pub scenario_max_ramp_up_p50_mw_per_step: u64,
    pub scenario_max_ramp_down_p50_mw_per_step: u64,
    pub actual_peak_pit: f64,
    pub actual_max_ramp_up_pit: f64,
    pub actual_max_ramp_down_pit: f64,
    pub reserve_cost_ratio: f64,
    pub reserve_critical_fractile: f64,
    pub reserve_power_mw: u64,
    pub actual_shortfall_mw: u64,
    pub reserve_shortfall_occurred: bool,
    /// Cost normalized to c_shortfall = 1 and c_reserve = reserve_cost_ratio.
    pub realized_reserve_cost_ratio_units: f64,
    pub measurement_run_id: String,
    pub measurement_evidence_id: String,
    pub measurement_time_alignment_evidence_id: String,
    pub measurement_source_timeline_digest_reference: String,
    pub measured_sample_evidence_ids: Vec<String>,
    pub claim_ceiling: &'static str,
}

fn validate_forecast(
    contract: &ScenarioGenerationContract,
    forecast: &ProbabilisticPowerForecast,
) -> Result<(), AuditError> {
    for (value, field) in [
        (forecast.forecast_id.as_str(), "forecast_id"),
        (forecast.origin_id.as_str(), "origin_id"),
        (forecast.clock_id.as_str(), "clock_id"),
        (forecast.time_origin_id.as_str(), "time_origin_id"),
        (forecast.operating_regime_id.as_str(), "operating_regime_id"),
        (forecast.forecast_evidence_id.as_str(), "forecast_evidence_id"),
        (
            forecast.state_mapping_evidence_id.as_str(),
            "state_mapping_evidence_id",
        ),
        (
            forecast.time_alignment_evidence_id.as_str(),
            "time_alignment_evidence_id",
        ),
        (
            forecast.source_timeline_digest_reference.as_str(),
            "source_timeline_digest_reference",
        ),
    ] {
        if value.trim().is_empty() {
            return Err(AuditError(format!(
                "probabilistic forecast {field} must not be blank"
            )));
        }
    }
    if forecast.model_contract != contract.model_contract
        || forecast.clock_id != contract.clock_id
        || forecast.time_origin_id != contract.time_origin_id
        || forecast.operating_regime_id != contract.operating_regime_id
        || forecast.step_period_ns != contract.step_period_ns
        || forecast.steps.len() != contract.horizon_steps
    {
        return Err(AuditError(
            "forecast does not match exact model, regime, cadence or horizon contract"
                .to_string(),
        ));
    }

    for (index, step) in forecast.steps.iter().enumerate() {
        let offset = (index as u64 + 1)
            .checked_mul(contract.step_period_ns)
            .ok_or_else(|| AuditError("forecast horizon timestamp offset overflowed".to_string()))?;
        let expected_timestamp = forecast
            .origin_timestamp_ns
            .checked_add(offset)
            .ok_or_else(|| AuditError("forecast horizon timestamp overflowed".to_string()))?;
        if step.timestamp_ns != expected_timestamp {
            return Err(AuditError(format!(
                "forecast horizon step {index} does not match its exact timestamp contract"
            )));
        }
        if step.quantiles.len() < 3 {
            return Err(AuditError(format!(
                "forecast horizon step {index} requires at least three quantiles"
            )));
        }
        let mut found_median = false;
        let mut previous_probability = None;
        let mut previous_power = None;
        for quantile in &step.quantiles {
            if !quantile.probability.is_finite()
                || quantile.probability <= 0.0
                || quantile.probability >= 1.0
            {
                return Err(AuditError(format!(
                    "forecast horizon step {index} contains an invalid quantile probability"
                )));
            }
            if let Some(previous) = previous_probability {
                if quantile.probability <= previous {
                    return Err(AuditError(format!(
                        "forecast horizon step {index} quantile probabilities are not strictly increasing"
                    )));
                }
            }
            if let Some(previous) = previous_power {
                if quantile.power_mw < previous {
                    return Err(AuditError(format!(
                        "forecast horizon step {index} has crossing quantiles"
                    )));
                }
            }
            if (quantile.probability - 0.5).abs() <= f64::EPSILON {
                found_median = true;
            }
            previous_probability = Some(quantile.probability);
            previous_power = Some(quantile.power_mw);
        }
        if !found_median {
            return Err(AuditError(format!(
                "forecast horizon step {index} must include an exact median quantile at 0.5"
            )));
        }
    }
    Ok(())
}

fn validate_residual_trajectory(
    contract: &ScenarioGenerationContract,
    forecast: &ProbabilisticPowerForecast,
    trajectory: &ForecastResidualTrajectory,
) -> Result<(), AuditError> {
    for (value, field) in [
        (trajectory.residual_id.as_str(), "residual_id"),
        (trajectory.origin_id.as_str(), "origin_id"),
        (trajectory.clock_id.as_str(), "clock_id"),
        (trajectory.time_origin_id.as_str(), "time_origin_id"),
        (
            trajectory.operating_regime_id.as_str(),
            "operating_regime_id",
        ),
        (
            trajectory.forecast_evidence_id.as_str(),
            "forecast_evidence_id",
        ),
        (
            trajectory.measurement_evidence_id.as_str(),
            "measurement_evidence_id",
        ),
        (
            trajectory.residual_generation_evidence_id.as_str(),
            "residual_generation_evidence_id",
        ),
        (
            trajectory.state_mapping_evidence_id.as_str(),
            "state_mapping_evidence_id",
        ),
        (
            trajectory.time_alignment_evidence_id.as_str(),
            "time_alignment_evidence_id",
        ),
        (
            trajectory.source_timeline_digest_reference.as_str(),
            "source_timeline_digest_reference",
        ),
    ] {
        if value.trim().is_empty() {
            return Err(AuditError(format!(
                "residual trajectory {field} must not be blank"
            )));
        }
    }
    if trajectory.model_contract != contract.model_contract
        || trajectory.clock_id != contract.clock_id
        || trajectory.time_origin_id != contract.time_origin_id
        || trajectory.operating_regime_id != contract.operating_regime_id
        || trajectory.residual_power_mw.len() != contract.horizon_steps
        || trajectory.source_sample_timestamps_ns.len() != contract.horizon_steps
        || trajectory.source_sample_evidence_ids.len() != contract.horizon_steps
    {
        return Err(AuditError(format!(
            "residual trajectory {:?} does not match exact model, regime or horizon contract",
            trajectory.residual_id
        )));
    }
    if trajectory.origin_timestamp_ns >= trajectory.target_window_end_ns {
        return Err(AuditError(format!(
            "residual trajectory {:?} has an invalid target window",
            trajectory.residual_id
        )));
    }
    let expected_end_offset = (contract.horizon_steps as u64)
        .checked_mul(contract.step_period_ns)
        .ok_or_else(|| AuditError("residual target duration overflowed".to_string()))?;
    let expected_end = trajectory
        .origin_timestamp_ns
        .checked_add(expected_end_offset)
        .ok_or_else(|| AuditError("residual target end timestamp overflowed".to_string()))?;
    if trajectory.target_window_end_ns != expected_end
        || trajectory.target_window_end_ns > forecast.origin_timestamp_ns
    {
        return Err(AuditError(format!(
            "residual trajectory {:?} is incomplete or uses future information",
            trajectory.residual_id
        )));
    }
    if trajectory.source_sample_evidence_ids.iter().any(|value| value.trim().is_empty()) {
        return Err(AuditError(format!(
            "residual trajectory {:?} has a sample without source evidence",
            trajectory.residual_id
        )));
    }
    let mut evidence_ids = HashSet::new();
    let mut previous_timestamp = None;
    for (index, timestamp) in trajectory.source_sample_timestamps_ns.iter().enumerate() {
        let offset = (index as u64 + 1)
            .checked_mul(contract.step_period_ns)
            .ok_or_else(|| AuditError("residual sample timestamp offset overflowed".to_string()))?;
        let expected = trajectory.origin_timestamp_ns
            .checked_add(offset)
            .ok_or_else(|| AuditError("residual sample timestamp overflowed".to_string()))?;
        if *timestamp != expected
            || previous_timestamp.is_some_and(|previous| *timestamp <= previous)
            || !evidence_ids.insert(trajectory.source_sample_evidence_ids[index].as_str())
        {
            return Err(AuditError(format!(
                "residual trajectory {:?} has duplicate evidence or a sample timestamp/cadence mismatch at index {index}",
                trajectory.residual_id
            )));
        }
        previous_timestamp = Some(*timestamp);
    }
    Ok(())
}

fn next_random(state: &mut u64) -> u64 {
    *state = (*state).wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut value = *state;
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn deterministic_permutation(length: usize, seed: u64) -> Vec<usize> {
    let mut result: Vec<usize> = (0..length).collect();
    let mut state = seed;
    for index in (1..length).rev() {
        let swap_index = (next_random(&mut state) % (index as u64 + 1)) as usize;
        result.swap(index, swap_index);
    }
    result
}

fn interpolate_quantile(
    points: &[ForecastQuantile],
    probability: f64,
) -> Result<(u64, bool), AuditError> {
    if probability < points[0].probability {
        return Ok((points[0].power_mw, true));
    }
    let last = points.len() - 1;
    if probability > points[last].probability {
        return Ok((points[last].power_mw, true));
    }
    for pair in points.windows(2) {
        if probability >= pair[0].probability && probability <= pair[1].probability {
            let denominator = pair[1].probability - pair[0].probability;
            let fraction = (probability - pair[0].probability) / denominator;
            let low = pair[0].power_mw as f64;
            let delta = (pair[1].power_mw - pair[0].power_mw) as f64;
            let value = low + fraction * delta;
            if !value.is_finite() || value < 0.0 || value > u64::MAX as f64 {
                return Err(AuditError(
                    "quantile interpolation produced a value outside the power representation"
                        .to_string(),
                ));
            }
            return Ok((value.round() as u64, false));
        }
    }
    Err(AuditError(
        "forecast quantile interpolation could not bracket a probability".to_string(),
    ))
}

fn marginal_quantile_samples(
    forecast: &ProbabilisticPowerForecast,
    scenario_count: usize,
) -> Result<(Vec<Vec<u64>>, usize), AuditError> {
    let mut all = Vec::with_capacity(forecast.steps.len());
    let mut clipped = 0_usize;
    for step in &forecast.steps {
        let mut values = Vec::with_capacity(scenario_count);
        for scenario in 0..scenario_count {
            let probability = (scenario as f64 + 0.5) / scenario_count as f64;
            let (value, was_clipped) = interpolate_quantile(&step.quantiles, probability)?;
            values.push(value);
            if was_clipped {
                clipped = clipped.checked_add(1).ok_or_else(|| {
                    AuditError("tail-clipped sample count overflowed".to_string())
                })?;
            }
        }
        all.push(values);
    }
    Ok((all, clipped))
}

fn median_path(forecast: &ProbabilisticPowerForecast) -> Result<Vec<u64>, AuditError> {
    forecast
        .steps
        .iter()
        .map(|step| {
            step.quantiles
                .iter()
                .find(|point| (point.probability - 0.5).abs() <= f64::EPSILON)
                .map(|point| point.power_mw)
                .ok_or_else(|| AuditError("forecast median quantile disappeared".to_string()))
        })
        .collect()
}

fn add_signed_residual(base: u64, residual: i64) -> (u64, bool) {
    let value = base as i128 + residual as i128;
    if value < 0 {
        (0, true)
    } else if value > u64::MAX as i128 {
        (u64::MAX, true)
    } else {
        (value as u64, false)
    }
}

fn generate_independent_scenarios(
    contract: &ScenarioGenerationContract,
    forecast: &ProbabilisticPowerForecast,
    marginal_samples: &[Vec<u64>],
) -> Result<(Vec<ForecastScenarioTrajectory>, Vec<String>, Vec<String>, usize), AuditError> {
    let mut output = Vec::with_capacity(contract.scenario_count);
    for scenario_index in 0..contract.scenario_count {
        output.push(ForecastScenarioTrajectory {
            scenario_id: format!("{}-scenario-{scenario_index:04}", contract.generator_id),
            source_residual_id: None,
            source_residual_evidence_id: None,
            source_residual_forecast_evidence_id: None,
            source_residual_generation_evidence_id: None,
            source_residual_state_mapping_evidence_id: None,
            source_residual_time_alignment_evidence_id: None,
            source_residual_timeline_digest_reference: None,
            source_sample_evidence_ids: Vec::new(),
            source_sample_timestamps_ns: Vec::new(),
            power_mw: vec![0; contract.horizon_steps],
        });
    }
    for (horizon_index, marginal) in marginal_samples.iter().enumerate() {
        let seed = contract.deterministic_seed
            ^ (horizon_index as u64).wrapping_mul(0xd6e8_feb8_6659_fd93);
        let permutation = deterministic_permutation(contract.scenario_count, seed);
        for scenario_index in 0..contract.scenario_count {
            output[scenario_index].power_mw[horizon_index] = marginal[permutation[scenario_index]];
        }
    }
    let _ = forecast;
    Ok((output, Vec::new(), Vec::new(), 0))
}

fn generate_residual_bootstrap(
    contract: &ScenarioGenerationContract,
    forecast: &ProbabilisticPowerForecast,
    residuals: &[ForecastResidualTrajectory],
    scheduled_median: &[u64],
) -> Result<(Vec<ForecastScenarioTrajectory>, Vec<String>, Vec<String>, usize), AuditError> {
    let mut random_state = contract.deterministic_seed;
    let mut output = Vec::with_capacity(contract.scenario_count);
    let mut selected_ids = Vec::with_capacity(contract.scenario_count);
    let mut selected_evidence = Vec::with_capacity(contract.scenario_count);
    let mut clamped = 0_usize;

    // A bootstrap samples whole residual trajectories with replacement.
    // Repeated source IDs are expected and preserve cross-horizon error shape.
    for scenario_index in 0..contract.scenario_count {
        let residual_index = (next_random(&mut random_state) % residuals.len() as u64) as usize;
        let residual = &residuals[residual_index];
        let mut power = Vec::with_capacity(contract.horizon_steps);
        for horizon_index in 0..contract.horizon_steps {
            let (value, was_clamped) = add_signed_residual(
                scheduled_median[horizon_index],
                residual.residual_power_mw[horizon_index],
            );
            power.push(value);
            if was_clamped {
                clamped = clamped.checked_add(1).ok_or_else(|| {
                    AuditError("residual floor-clamp count overflowed".to_string())
                })?;
            }
        }
        output.push(ForecastScenarioTrajectory {
            scenario_id: format!("{}-scenario-{scenario_index:04}", contract.generator_id),
            source_residual_id: Some(residual.residual_id.clone()),
            source_residual_evidence_id: Some(residual.measurement_evidence_id.clone()),
            source_residual_forecast_evidence_id: Some(residual.forecast_evidence_id.clone()),
            source_residual_generation_evidence_id: Some(residual.residual_generation_evidence_id.clone()),
            source_residual_state_mapping_evidence_id: Some(residual.state_mapping_evidence_id.clone()),
            source_residual_time_alignment_evidence_id: Some(residual.time_alignment_evidence_id.clone()),
            source_residual_timeline_digest_reference: Some(residual.source_timeline_digest_reference.clone()),
            source_sample_evidence_ids: residual.source_sample_evidence_ids.clone(),
            source_sample_timestamps_ns: residual.source_sample_timestamps_ns.clone(),
            power_mw: power,
        });
        selected_ids.push(residual.residual_id.clone());
        selected_evidence.push(residual.measurement_evidence_id.clone());
    }
    let _ = forecast;
    Ok((output, selected_ids, selected_evidence, clamped))
}

fn generate_schaake_shuffle(
    contract: &ScenarioGenerationContract,
    residuals: &[ForecastResidualTrajectory],
    marginal_samples: &[Vec<u64>],
) -> Result<(Vec<ForecastScenarioTrajectory>, Vec<String>, Vec<String>, usize), AuditError> {
    let permutation = deterministic_permutation(residuals.len(), contract.deterministic_seed);
    let selected: Vec<usize> = permutation.into_iter().take(contract.scenario_count).collect();
    let mut rank_by_horizon = Vec::with_capacity(contract.horizon_steps);

    for horizon_index in 0..contract.horizon_steps {
        let mut ordered: Vec<usize> = (0..residuals.len()).collect();
        ordered.sort_by(|left, right| {
            residuals[*left].residual_power_mw[horizon_index]
                .cmp(&residuals[*right].residual_power_mw[horizon_index])
                .then_with(|| residuals[*left].residual_id.cmp(&residuals[*right].residual_id))
        });
        let mut rank = vec![0_usize; residuals.len()];
        for (position, residual_index) in ordered.iter().enumerate() {
            rank[*residual_index] = position;
        }
        rank_by_horizon.push(rank);
    }

    let mut output = Vec::with_capacity(contract.scenario_count);
    let mut selected_ids = Vec::with_capacity(contract.scenario_count);
    let mut selected_evidence = Vec::with_capacity(contract.scenario_count);
    for (scenario_index, residual_index) in selected.iter().copied().enumerate() {
        let mut power = Vec::with_capacity(contract.horizon_steps);
        for horizon_index in 0..contract.horizon_steps {
            let rank = rank_by_horizon[horizon_index][residual_index];
            let marginal_index = if residuals.len() <= 1 {
                0
            } else {
                rank.saturating_mul(contract.scenario_count - 1) / (residuals.len() - 1)
            };
            power.push(marginal_samples[horizon_index][marginal_index]);
        }
        output.push(ForecastScenarioTrajectory {
            scenario_id: format!("{}-scenario-{scenario_index:04}", contract.generator_id),
            source_residual_id: Some(residuals[residual_index].residual_id.clone()),
            source_residual_evidence_id: Some(
                residuals[residual_index].measurement_evidence_id.clone(),
            ),
            source_residual_forecast_evidence_id: Some(
                residuals[residual_index].forecast_evidence_id.clone(),
            ),
            source_residual_generation_evidence_id: Some(
                residuals[residual_index].residual_generation_evidence_id.clone(),
            ),
            source_residual_state_mapping_evidence_id: Some(
                residuals[residual_index].state_mapping_evidence_id.clone(),
            ),
            source_residual_time_alignment_evidence_id: Some(
                residuals[residual_index].time_alignment_evidence_id.clone(),
            ),
            source_residual_timeline_digest_reference: Some(
                residuals[residual_index].source_timeline_digest_reference.clone(),
            ),
            source_sample_evidence_ids: residuals[residual_index]
                .source_sample_evidence_ids
                .clone(),
            source_sample_timestamps_ns: residuals[residual_index]
                .source_sample_timestamps_ns
                .clone(),
            power_mw: power,
        });
        selected_ids.push(residuals[residual_index].residual_id.clone());
        selected_evidence.push(residuals[residual_index].measurement_evidence_id.clone());
    }
    Ok((output, selected_ids, selected_evidence, 0))
}

/// Generate deterministic marginal or dependence-preserving horizon scenarios.
///
/// All selected historical residual trajectories must have complete target
/// windows ending no later than the forecast origin. Scenario outputs are
/// MODELLED and are not a physical reserve or grid-service qualification.
pub fn generate_forecast_scenarios(
    contract: ScenarioGenerationContract,
    forecast: &ProbabilisticPowerForecast,
    residual_trajectories: &[ForecastResidualTrajectory],
) -> Result<ForecastScenarioSet, AuditError> {
    contract.validate()?;
    validate_forecast(&contract, forecast)?;
    let mut residual_ids = HashSet::new();
    let mut residual_origins = HashSet::new();

    match contract.generator_kind {
        ScenarioGeneratorKind::IndependentQuantileSampling => {
            if !residual_trajectories.is_empty() {
                return Err(AuditError(
                    "independent sampling must not consume residual trajectories".to_string(),
                ));
            }
        }
        ScenarioGeneratorKind::EmpiricalResidualTrajectoryBootstrap
        | ScenarioGeneratorKind::SchaakeShuffle => {
            let required_pool_size = match contract.generator_kind {
                ScenarioGeneratorKind::EmpiricalResidualTrajectoryBootstrap => {
                    contract.minimum_residual_trajectory_count
                }
                ScenarioGeneratorKind::SchaakeShuffle => {
                    contract.minimum_residual_trajectory_count.max(contract.scenario_count)
                }
                ScenarioGeneratorKind::IndependentQuantileSampling => 0,
            };
            if residual_trajectories.len() < required_pool_size {
                return Err(AuditError(format!(
                    "residual pool has {} trajectories; frozen method requires at least {}",
                    residual_trajectories.len(),
                    required_pool_size
                )));
            }
            let mut ordered_origins: Vec<u64> = Vec::with_capacity(residual_trajectories.len());
            let mut pool_sample_evidence_ids = HashSet::new();
            for residual in residual_trajectories {
                validate_residual_trajectory(&contract, forecast, residual)?;
                if residual.origin_id == forecast.origin_id
                    || !residual_ids.insert(residual.residual_id.as_str())
                    || !residual_origins.insert(residual.origin_id.as_str())
                {
                    return Err(AuditError(
                        "residual pool contains duplicate trajectory or origin identities"
                            .to_string(),
                    ));
                }
                for evidence_id in &residual.source_sample_evidence_ids {
                    if !pool_sample_evidence_ids.insert(evidence_id.as_str()) {
                        return Err(AuditError(
                            "residual pool reuses a measured-sample evidence ID across trajectories"
                                .to_string(),
                        ));
                    }
                }
                ordered_origins.push(residual.origin_timestamp_ns);
            }
            ordered_origins.sort_unstable();
            for pair in ordered_origins.windows(2) {
                if pair[1].checked_sub(pair[0]).is_none_or(|separation| {
                    separation < contract.minimum_residual_origin_separation_ns
                }) {
                    return Err(AuditError(
                        "residual trajectories overlap the frozen historical-origin separation"
                            .to_string(),
                    ));
                }
            }
        }
    }

    let (marginal_samples, tail_clipped) =
        marginal_quantile_samples(forecast, contract.scenario_count)?;
    let scheduled_median = median_path(forecast)?;
    let (scenarios, selected_ids, selected_evidence, clamped) = match contract.generator_kind {
        ScenarioGeneratorKind::IndependentQuantileSampling => generate_independent_scenarios(
            &contract,
            forecast,
            &marginal_samples,
        )?,
        ScenarioGeneratorKind::EmpiricalResidualTrajectoryBootstrap => generate_residual_bootstrap(
            &contract,
            forecast,
            residual_trajectories,
            &scheduled_median,
        )?,
        ScenarioGeneratorKind::SchaakeShuffle => generate_schaake_shuffle(
            &contract,
            residual_trajectories,
            &marginal_samples,
        )?,
    };

    let step_timestamps_ns = forecast.steps.iter().map(|step| step.timestamp_ns).collect();
    let selected_residual_forecast_evidence_ids = scenarios.iter()
        .filter_map(|scenario| scenario.source_residual_forecast_evidence_id.clone())
        .collect();
    let selected_residual_generation_evidence_ids = scenarios.iter()
        .filter_map(|scenario| scenario.source_residual_generation_evidence_id.clone())
        .collect();
    let selected_residual_state_mapping_evidence_ids = scenarios.iter()
        .filter_map(|scenario| scenario.source_residual_state_mapping_evidence_id.clone())
        .collect();
    let selected_residual_time_alignment_evidence_ids = scenarios.iter()
        .filter_map(|scenario| scenario.source_residual_time_alignment_evidence_id.clone())
        .collect();
    let selected_residual_timeline_digest_references = scenarios.iter()
        .filter_map(|scenario| scenario.source_residual_timeline_digest_reference.clone())
        .collect();
    Ok(ForecastScenarioSet {
        contract: contract.clone(),
        forecast_id: forecast.forecast_id.clone(),
        forecast_origin_timestamp_ns: forecast.origin_timestamp_ns,
        clock_id: forecast.clock_id.clone(),
        time_origin_id: forecast.time_origin_id.clone(),
        forecast_evidence_id: forecast.forecast_evidence_id.clone(),
        state_mapping_evidence_id: forecast.state_mapping_evidence_id.clone(),
        time_alignment_evidence_id: forecast.time_alignment_evidence_id.clone(),
        source_timeline_digest_reference: forecast.source_timeline_digest_reference.clone(),
        step_timestamps_ns,
        scheduled_median_power_mw: scheduled_median,
        scenarios,
        selected_residual_ids: selected_ids,
        selected_residual_evidence_ids: selected_evidence,
        selected_residual_forecast_evidence_ids,
        selected_residual_generation_evidence_ids,
        selected_residual_state_mapping_evidence_ids,
        selected_residual_time_alignment_evidence_ids,
        selected_residual_timeline_digest_references,
        marginal_tail_clipped_sample_count: tail_clipped,
        residual_floor_clamped_sample_count: clamped,
        claim_ceiling: "MODELLED_TEMPORAL_POWER_SCENARIOS_NOT_MEASUREMENT_OR_GRID_RESERVE_QUALIFICATION",
    })
}

fn nearest_rank_u64(values: &[u64], quantile: f64) -> Result<u64, AuditError> {
    if values.is_empty() || !quantile.is_finite() || quantile <= 0.0 || quantile > 1.0 {
        return Err(AuditError(
            "nearest-rank quantile requires data and a quantile in (0, 1]".to_string(),
        ));
    }
    let mut ordered = values.to_vec();
    ordered.sort_unstable();
    let rank = (quantile * ordered.len() as f64).ceil() as usize;
    ordered.get(rank.saturating_sub(1)).copied().ok_or_else(|| {
        AuditError("nearest-rank quantile fell outside its sample".to_string())
    })
}

fn nearest_rank_u128(values: &[u128], quantile: f64) -> Result<u128, AuditError> {
    if values.is_empty() || !quantile.is_finite() || quantile <= 0.0 || quantile > 1.0 {
        return Err(AuditError(
            "nearest-rank quantile requires data and a quantile in (0, 1]".to_string(),
        ));
    }
    let mut ordered = values.to_vec();
    ordered.sort_unstable();
    let rank = (quantile * ordered.len() as f64).ceil() as usize;
    ordered.get(rank.saturating_sub(1)).copied().ok_or_else(|| {
        AuditError("nearest-rank quantile fell outside its sample".to_string())
    })
}

fn vector_distance(left: &[u64], right: &[u64]) -> f64 {
    left.iter()
        .zip(right.iter())
        .map(|(a, b)| {
            let difference = *a as f64 - *b as f64;
            difference * difference
        })
        .sum::<f64>()
        .sqrt()
}

fn max_ramp_up(values: &[u64]) -> u64 {
    values
        .windows(2)
        .map(|pair| pair[1].saturating_sub(pair[0]))
        .max()
        .unwrap_or(0)
}

fn max_ramp_down(values: &[u64]) -> u64 {
    values
        .windows(2)
        .map(|pair| pair[0].saturating_sub(pair[1]))
        .max()
        .unwrap_or(0)
}

fn maximum_positive_deviation(actual: &[u64], baseline: &[u64]) -> Result<u64, AuditError> {
    if actual.len() != baseline.len() || actual.is_empty() {
        return Err(AuditError(
            "actual and baseline horizons must have the same non-zero length".to_string(),
        ));
    }
    Ok(actual
        .iter()
        .zip(baseline.iter())
        .map(|(value, median)| value.saturating_sub(*median))
        .max()
        .unwrap_or(0))
}

fn empirical_pit(values: &[u64], actual: u64) -> f64 {
    if values.is_empty() {
        return f64::NAN;
    }
    let less = values.iter().filter(|value| **value < actual).count() as f64;
    let equal = values.iter().filter(|value| **value == actual).count() as f64;
    (less + 0.5 * equal) / values.len() as f64
}

fn empirical_pit_u128(values: &[u128], actual: u128) -> f64 {
    if values.is_empty() {
        return f64::NAN;
    }
    let less = values.iter().filter(|value| **value < actual).count() as f64;
    let equal = values.iter().filter(|value| **value == actual).count() as f64;
    (less + 0.5 * equal) / values.len() as f64
}

fn validate_scenario_set(scenarios: &ForecastScenarioSet) -> Result<(), AuditError> {
    scenarios.contract.validate()?;
    if scenarios.claim_ceiling
        != "MODELLED_TEMPORAL_POWER_SCENARIOS_NOT_MEASUREMENT_OR_GRID_RESERVE_QUALIFICATION"
        || scenarios.forecast_id.trim().is_empty()
        || scenarios.forecast_evidence_id.trim().is_empty()
        || scenarios.state_mapping_evidence_id.trim().is_empty()
        || scenarios.time_alignment_evidence_id.trim().is_empty()
        || scenarios.source_timeline_digest_reference.trim().is_empty()
        || scenarios.scenarios.len() != scenarios.contract.scenario_count
        || scenarios.step_timestamps_ns.len() != scenarios.contract.horizon_steps
        || scenarios.scheduled_median_power_mw.len() != scenarios.contract.horizon_steps
    {
        return Err(AuditError(
            "scenario set has missing identity or inconsistent scenario/horizon dimensions"
                .to_string(),
        ));
    }

    for (index, timestamp) in scenarios.step_timestamps_ns.iter().enumerate() {
        let offset = (index as u64 + 1)
            .checked_mul(scenarios.contract.step_period_ns)
            .ok_or_else(|| AuditError("scenario timestamp offset overflowed".to_string()))?;
        let expected = scenarios.forecast_origin_timestamp_ns
            .checked_add(offset)
            .ok_or_else(|| AuditError("scenario timestamp overflowed".to_string()))?;
        if *timestamp != expected {
            return Err(AuditError(format!(
                "scenario set timestamp {index} does not match its frozen forecast origin"
            )));
        }
    }

    let mut scenario_ids = HashSet::new();
    for scenario in &scenarios.scenarios {
        if scenario.scenario_id.trim().is_empty()
            || !scenario_ids.insert(scenario.scenario_id.as_str())
            || scenario.power_mw.len() != scenarios.contract.horizon_steps
        {
            return Err(AuditError(
                "scenario set contains a blank/duplicate ID or invalid trajectory length"
                    .to_string(),
            ));
        }
    }

    match scenarios.contract.generator_kind {
        ScenarioGeneratorKind::IndependentQuantileSampling => {
            if !scenarios.selected_residual_ids.is_empty()
                || !scenarios.selected_residual_evidence_ids.is_empty()
                || !scenarios.selected_residual_forecast_evidence_ids.is_empty()
                || !scenarios.selected_residual_generation_evidence_ids.is_empty()
                || !scenarios.selected_residual_state_mapping_evidence_ids.is_empty()
                || !scenarios.selected_residual_time_alignment_evidence_ids.is_empty()
                || !scenarios.selected_residual_timeline_digest_references.is_empty()
                || scenarios.scenarios.iter().any(|scenario| {
                    scenario.source_residual_id.is_some()
                        || scenario.source_residual_evidence_id.is_some()
                        || scenario.source_residual_forecast_evidence_id.is_some()
                        || scenario.source_residual_generation_evidence_id.is_some()
                        || scenario.source_residual_state_mapping_evidence_id.is_some()
                        || scenario.source_residual_time_alignment_evidence_id.is_some()
                        || scenario.source_residual_timeline_digest_reference.is_some()
                        || !scenario.source_sample_evidence_ids.is_empty()
                        || !scenario.source_sample_timestamps_ns.is_empty()
                })
            {
                return Err(AuditError(
                    "independent-sampling scenario set unexpectedly carries residual lineage"
                        .to_string(),
                ));
            }
        }
        ScenarioGeneratorKind::EmpiricalResidualTrajectoryBootstrap
        | ScenarioGeneratorKind::SchaakeShuffle => {
            if scenarios.selected_residual_ids.len() != scenarios.contract.scenario_count
                || scenarios.selected_residual_evidence_ids.len()
                    != scenarios.contract.scenario_count
                || scenarios.selected_residual_forecast_evidence_ids.len()
                    != scenarios.contract.scenario_count
                || scenarios.selected_residual_generation_evidence_ids.len()
                    != scenarios.contract.scenario_count
                || scenarios.selected_residual_state_mapping_evidence_ids.len()
                    != scenarios.contract.scenario_count
                || scenarios.selected_residual_time_alignment_evidence_ids.len()
                    != scenarios.contract.scenario_count
                || scenarios.selected_residual_timeline_digest_references.len()
                    != scenarios.contract.scenario_count
                || scenarios.selected_residual_ids.iter().any(|id| id.trim().is_empty())
                || scenarios.selected_residual_evidence_ids.iter().any(|id| id.trim().is_empty())
                || scenarios.selected_residual_forecast_evidence_ids.iter().any(|id| id.trim().is_empty())
                || scenarios.selected_residual_generation_evidence_ids.iter().any(|id| id.trim().is_empty())
                || scenarios.selected_residual_state_mapping_evidence_ids.iter().any(|id| id.trim().is_empty())
                || scenarios.selected_residual_time_alignment_evidence_ids.iter().any(|id| id.trim().is_empty())
                || scenarios.selected_residual_timeline_digest_references.iter().any(|id| id.trim().is_empty())
                || scenarios.scenarios.iter().any(|scenario| {
                    scenario.source_residual_id.as_deref().is_none_or(|id| id.trim().is_empty())
                        || scenario.source_residual_evidence_id.as_deref().is_none_or(|id| id.trim().is_empty())
                        || scenario.source_residual_forecast_evidence_id.as_deref().is_none_or(|id| id.trim().is_empty())
                        || scenario.source_residual_generation_evidence_id.as_deref().is_none_or(|id| id.trim().is_empty())
                        || scenario.source_residual_state_mapping_evidence_id.as_deref().is_none_or(|id| id.trim().is_empty())
                        || scenario.source_residual_time_alignment_evidence_id.as_deref().is_none_or(|id| id.trim().is_empty())
                        || scenario.source_residual_timeline_digest_reference.as_deref().is_none_or(|id| id.trim().is_empty())
                        || scenario.source_sample_evidence_ids.len()
                            != scenarios.contract.horizon_steps
                        || scenario.source_sample_timestamps_ns.len()
                            != scenarios.contract.horizon_steps
                        || scenario.source_sample_evidence_ids.iter().any(|id| id.trim().is_empty())
                })
            {
                return Err(AuditError(
                    "dependence-preserving scenario set is missing residual and source-sample lineage"
                        .to_string(),
                ));
            }
            for (index, scenario) in scenarios.scenarios.iter().enumerate() {
                if scenario.source_residual_id.as_deref()
                    != Some(scenarios.selected_residual_ids[index].as_str())
                    || scenario.source_residual_evidence_id.as_deref()
                        != Some(scenarios.selected_residual_evidence_ids[index].as_str())
                    || scenario.source_residual_forecast_evidence_id.as_deref()
                        != Some(scenarios.selected_residual_forecast_evidence_ids[index].as_str())
                    || scenario.source_residual_generation_evidence_id.as_deref()
                        != Some(scenarios.selected_residual_generation_evidence_ids[index].as_str())
                    || scenario.source_residual_state_mapping_evidence_id.as_deref()
                        != Some(scenarios.selected_residual_state_mapping_evidence_ids[index].as_str())
                    || scenario.source_residual_time_alignment_evidence_id.as_deref()
                        != Some(scenarios.selected_residual_time_alignment_evidence_ids[index].as_str())
                    || scenario.source_residual_timeline_digest_reference.as_deref()
                        != Some(scenarios.selected_residual_timeline_digest_references[index].as_str())
                {
                    return Err(AuditError(
                        "scenario residual lineage differs from selected residual manifest"
                            .to_string(),
                    ));
                }
                let mut sample_ids = HashSet::new();
                let mut previous_timestamp = None;
                for (sample_index, timestamp) in
                    scenario.source_sample_timestamps_ns.iter().enumerate()
                {
                    if !sample_ids.insert(scenario.source_sample_evidence_ids[sample_index].as_str())
                        || previous_timestamp.is_some_and(|previous| {
                            timestamp.checked_sub(previous)
                                != Some(scenarios.contract.step_period_ns)
                        })
                    {
                        return Err(AuditError(
                            "scenario residual sample evidence/timestamps are not unique and cadence-aligned"
                                .to_string(),
                        ));
                    }
                    previous_timestamp = Some(*timestamp);
                }
            }
            if scenarios.contract.generator_kind == ScenarioGeneratorKind::SchaakeShuffle {
                let unique: HashSet<&str> = scenarios
                    .selected_residual_ids
                    .iter()
                    .map(String::as_str)
                    .collect();
                if unique.len() != scenarios.contract.scenario_count {
                    return Err(AuditError(
                        "Schaake-shuffle scenarios require unique historical rank trajectories"
                            .to_string(),
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Score a scenario set against one exact measured horizon and size reserve
/// using the horizon maximum-deviation critical fractile.
///
/// The result is a research measurement of scenario quality and modelled
/// reserve economics. It does not qualify a grid product, firm capacity or
/// physical delivery.
pub fn evaluate_forecast_scenarios(
    scenarios: &ForecastScenarioSet,
    actual: &MeasuredForecastHorizon,
    reserve_cost_ratio: f64,
) -> Result<ScenarioEvaluationReport, AuditError> {
    validate_scenario_set(scenarios)?;
    if actual.forecast_id != scenarios.forecast_id
        || actual.clock_id != scenarios.contract.clock_id
        || actual.time_origin_id != scenarios.contract.time_origin_id
        || actual.model_contract != scenarios.contract.model_contract
        || actual.measurement_run_id.trim().is_empty()
        || actual.measurement_evidence_id.trim().is_empty()
        || actual.time_alignment_evidence_id.trim().is_empty()
        || actual.source_timeline_digest_reference.trim().is_empty()
        || actual.samples.len() != scenarios.contract.horizon_steps
    {
        return Err(AuditError(
            "measured horizon does not match the exact forecast, model or scenario contract"
                .to_string(),
        ));
    }
    if !reserve_cost_ratio.is_finite() || reserve_cost_ratio <= 0.0 || reserve_cost_ratio >= 1.0 {
        return Err(AuditError(
            "reserve cost ratio c_R/c_S must be finite and in (0, 1)".to_string(),
        ));
    }
    if scenarios.scenarios.len() != scenarios.contract.scenario_count
        || scenarios.scheduled_median_power_mw.len() != scenarios.contract.horizon_steps
        || scenarios.step_timestamps_ns.len() != scenarios.contract.horizon_steps
    {
        return Err(AuditError(
            "scenario set has inconsistent scenario or horizon dimensions".to_string(),
        ));
    }

    let mut actual_power = Vec::with_capacity(actual.samples.len());
    let mut actual_energy = 0_u128;
    let mut actual_sample_ids = HashSet::new();
    let mut actual_evidence_ids = Vec::with_capacity(actual.samples.len());
    for (index, sample) in actual.samples.iter().enumerate() {
        if sample.timestamp_ns != scenarios.step_timestamps_ns[index]
            || sample.quality != SampleQuality::Good
            || sample.source_evidence_id.trim().is_empty()
            || !actual_sample_ids.insert(sample.source_evidence_id.as_str())
        {
            return Err(AuditError(format!(
                "measured horizon sample {index} is invalid or not exactly aligned"
            )));
        }
        let interval_energy = (sample.power_mw as u128)
            .checked_mul(scenarios.contract.step_period_ns as u128)
            .ok_or_else(|| AuditError("actual horizon energy interval overflowed".to_string()))?;
        actual_energy = actual_energy
            .checked_add(interval_energy)
            .ok_or_else(|| AuditError("actual horizon energy sum overflowed".to_string()))?;
        actual_power.push(sample.power_mw);
        actual_evidence_ids.push(sample.source_evidence_id.clone());
    }

    let mut scenario_peaks = Vec::with_capacity(scenarios.scenarios.len());
    let mut scenario_ramp_ups = Vec::with_capacity(scenarios.scenarios.len());
    let mut scenario_ramp_downs = Vec::with_capacity(scenarios.scenarios.len());
    let mut scenario_energies = Vec::with_capacity(scenarios.scenarios.len());
    let mut scenario_surpluses = Vec::with_capacity(scenarios.scenarios.len());
    let mut first_distance_sum = 0.0_f64;

    for scenario in &scenarios.scenarios {
        if scenario.power_mw.len() != scenarios.contract.horizon_steps {
            return Err(AuditError(
                "scenario trajectory has an invalid horizon length".to_string(),
            ));
        }
        first_distance_sum += vector_distance(&scenario.power_mw, &actual_power);
        scenario_peaks.push(*scenario.power_mw.iter().max().unwrap_or(&0));
        scenario_ramp_ups.push(max_ramp_up(&scenario.power_mw));
        scenario_ramp_downs.push(max_ramp_down(&scenario.power_mw));
        scenario_energies.push(
            scenario.power_mw.iter().try_fold(0_u128, |sum, value| {
                let interval_energy = (*value as u128)
                    .checked_mul(scenarios.contract.step_period_ns as u128)
                    .ok_or_else(|| AuditError("scenario energy interval overflowed".to_string()))?;
                sum.checked_add(interval_energy)
                    .ok_or_else(|| AuditError("scenario energy sum overflowed".to_string()))
            })?,
        );
        scenario_surpluses.push(maximum_positive_deviation(
            &scenario.power_mw,
            &scenarios.scheduled_median_power_mw,
        )?);
    }

    let scenario_count = scenarios.scenarios.len();
    let first_term = first_distance_sum / scenario_count as f64;
    let mut pairwise_distance_sum = 0.0_f64;
    for first in 0..scenario_count {
        for second in (first + 1)..scenario_count {
            pairwise_distance_sum += vector_distance(
                &scenarios.scenarios[first].power_mw,
                &scenarios.scenarios[second].power_mw,
            );
        }
    }
    // The full ordered-pair energy-score term is twice the i<j sum.
    let energy_score_mw = first_term
        - pairwise_distance_sum / (scenario_count as f64 * scenario_count as f64);

    let mut variogram_score = 0.0_f64;
    for first_step in 0..scenarios.contract.horizon_steps {
        for second_step in (first_step + 1)..scenarios.contract.horizon_steps {
            let actual_increment = (actual_power[first_step] as f64
                - actual_power[second_step] as f64)
                .abs()
                .sqrt();
            let forecast_increment = scenarios
                .scenarios
                .iter()
                .map(|scenario| {
                    (scenario.power_mw[first_step] as f64
                        - scenario.power_mw[second_step] as f64)
                        .abs()
                        .sqrt()
                })
                .sum::<f64>()
                / scenario_count as f64;
            let lag_weight = 1.0 / (second_step - first_step) as f64;
            variogram_score += lag_weight * (actual_increment - forecast_increment).powi(2);
        }
    }

    let reserve_critical_fractile = 1.0 - reserve_cost_ratio;
    let reserve_power_mw = nearest_rank_u64(&scenario_surpluses, reserve_critical_fractile)?;
    let actual_horizon_max_surplus_mw = maximum_positive_deviation(
        &actual_power,
        &scenarios.scheduled_median_power_mw,
    )?;
    let actual_shortfall_mw = actual_horizon_max_surplus_mw.saturating_sub(reserve_power_mw);
    let realized_reserve_cost_ratio_units =
        reserve_cost_ratio * reserve_power_mw as f64 + actual_shortfall_mw as f64;

    if !energy_score_mw.is_finite()
        || !variogram_score.is_finite()
        || !realized_reserve_cost_ratio_units.is_finite()
    {
        return Err(AuditError(
            "scenario evaluation produced a non-finite score or reserve cost".to_string(),
        ));
    }

    Ok(ScenarioEvaluationReport {
        forecast_id: scenarios.forecast_id.clone(),
        model_contract: scenarios.contract.model_contract.clone(),
        generator_id: scenarios.contract.generator_id.clone(),
        generator_revision: scenarios.contract.generator_revision.clone(),
        generator_kind: scenarios.contract.generator_kind,
        forecast_evidence_id: scenarios.forecast_evidence_id.clone(),
        state_mapping_evidence_id: scenarios.state_mapping_evidence_id.clone(),
        clock_id: scenarios.clock_id.clone(),
        time_origin_id: scenarios.time_origin_id.clone(),
        forecast_time_alignment_evidence_id: scenarios.time_alignment_evidence_id.clone(),
        forecast_source_timeline_digest_reference: scenarios.source_timeline_digest_reference.clone(),
        residual_pool_evidence_id: scenarios.contract.residual_pool_evidence_id.clone(),
        selected_residual_ids: scenarios.selected_residual_ids.clone(),
        selected_residual_evidence_ids: scenarios.selected_residual_evidence_ids.clone(),
        selected_residual_forecast_evidence_ids: scenarios.selected_residual_forecast_evidence_ids.clone(),
        selected_residual_generation_evidence_ids: scenarios.selected_residual_generation_evidence_ids.clone(),
        selected_residual_state_mapping_evidence_ids: scenarios.selected_residual_state_mapping_evidence_ids.clone(),
        selected_residual_time_alignment_evidence_ids: scenarios.selected_residual_time_alignment_evidence_ids.clone(),
        selected_residual_timeline_digest_references: scenarios.selected_residual_timeline_digest_references.clone(),
        selected_residual_sample_evidence_ids: scenarios.scenarios.iter()
            .map(|scenario| scenario.source_sample_evidence_ids.clone())
            .collect(),
        selected_residual_sample_timestamps_ns: scenarios.scenarios.iter()
            .map(|scenario| scenario.source_sample_timestamps_ns.clone())
            .collect(),
        scenario_count,
        horizon_steps: scenarios.contract.horizon_steps,
        energy_score_mw,
        variogram_score,
        actual_energy_mw_ns: actual_energy,
        scenario_energy_p10_mw_ns: nearest_rank_u128(&scenario_energies, 0.10)?,
        scenario_energy_p50_mw_ns: nearest_rank_u128(&scenario_energies, 0.50)?,
        scenario_energy_p90_mw_ns: nearest_rank_u128(&scenario_energies, 0.90)?,
        actual_horizon_energy_pit: empirical_pit_u128(&scenario_energies, actual_energy),
        actual_peak_power_mw: *actual_power.iter().max().unwrap_or(&0),
        actual_max_ramp_up_mw_per_step: max_ramp_up(&actual_power),
        actual_max_ramp_down_mw_per_step: max_ramp_down(&actual_power),
        actual_horizon_max_surplus_mw,
        scenario_peak_p10_mw: nearest_rank_u64(&scenario_peaks, 0.10)?,
        scenario_peak_p50_mw: nearest_rank_u64(&scenario_peaks, 0.50)?,
        scenario_peak_p90_mw: nearest_rank_u64(&scenario_peaks, 0.90)?,
        scenario_max_ramp_up_p50_mw_per_step: nearest_rank_u64(&scenario_ramp_ups, 0.50)?,
        scenario_max_ramp_down_p50_mw_per_step: nearest_rank_u64(&scenario_ramp_downs, 0.50)?,
        actual_peak_pit: empirical_pit(&scenario_peaks, *actual_power.iter().max().unwrap_or(&0)),
        actual_max_ramp_up_pit: empirical_pit(
            &scenario_ramp_ups,
            max_ramp_up(&actual_power),
        ),
        actual_max_ramp_down_pit: empirical_pit(
            &scenario_ramp_downs,
            max_ramp_down(&actual_power),
        ),
        reserve_cost_ratio,
        reserve_critical_fractile,
        reserve_power_mw,
        actual_shortfall_mw,
        reserve_shortfall_occurred: actual_shortfall_mw > 0,
        realized_reserve_cost_ratio_units,
        measurement_run_id: actual.measurement_run_id.clone(),
        measurement_evidence_id: actual.measurement_evidence_id.clone(),
        measurement_time_alignment_evidence_id: actual.time_alignment_evidence_id.clone(),
        measurement_source_timeline_digest_reference: actual.source_timeline_digest_reference.clone(),
        measured_sample_evidence_ids: actual_evidence_ids,
        claim_ceiling: "MODELLED_SCENARIO_AND_MEASURED_HORIZON_SCORE_NOT_GRID_RESERVE_QUALIFICATION",
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use super::{
        evaluate_forecast_scenarios, generate_forecast_scenarios, ForecastHorizonStep,
        ForecastQuantile, ForecastResidualTrajectory, MeasuredForecastHorizon,
        MeasuredForecastHorizonSample, ProbabilisticPowerForecast, ScenarioGenerationContract,
        ScenarioGeneratorKind,
    };
    use crate::power_state::{PowerModelContract, SampleQuality};

    fn model_contract() -> PowerModelContract {
        PowerModelContract {
            model_id: "power-state-v1".to_string(),
            model_revision: "rev-a".to_string(),
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
            clock_alignment_method_id: "ptp-v1".to_string(),
            sample_period_ns: 10,
            minimum_samples_per_state: 1,
            minimum_samples_per_transition: 1,
        }
    }

    fn forecast() -> ProbabilisticPowerForecast {
        let quantiles = vec![
            ForecastQuantile { probability: 0.10, power_mw: 80 },
            ForecastQuantile { probability: 0.50, power_mw: 100 },
            ForecastQuantile { probability: 0.90, power_mw: 120 },
        ];
        ProbabilisticPowerForecast {
            forecast_id: "forecast-current".to_string(),
            origin_id: "origin-current".to_string(),
            origin_timestamp_ns: 10_000,
            clock_id: "clock-common".to_string(),
            time_origin_id: "time-origin-common".to_string(),
            operating_regime_id: "active".to_string(),
            model_contract: model_contract(),
            forecast_evidence_id: "forecast-evidence-current".to_string(),
            state_mapping_evidence_id: "state-map-current".to_string(),
            time_alignment_evidence_id: "clock-current".to_string(),
            source_timeline_digest_reference: "timeline-current".to_string(),
            step_period_ns: 10,
            steps: (1..=3)
                .map(|step| ForecastHorizonStep {
                    timestamp_ns: 10_000 + step * 10,
                    quantiles: vec![
                        ForecastQuantile { probability: 0.10, power_mw: 80 + step * 5 },
                        ForecastQuantile { probability: 0.50, power_mw: 100 + step * 5 },
                        ForecastQuantile { probability: 0.90, power_mw: 120 + step * 5 },
                    ],
                })
                .collect(),
        }
    }

    fn contract(kind: ScenarioGeneratorKind) -> ScenarioGenerationContract {
        let residual_method = matches!(
            kind,
            ScenarioGeneratorKind::EmpiricalResidualTrajectoryBootstrap
                | ScenarioGeneratorKind::SchaakeShuffle
        );
        ScenarioGenerationContract {
            generator_id: "scenario-generator-v1".to_string(),
            generator_revision: "rev-1".to_string(),
            generator_kind: kind,
            model_contract: model_contract(),
            operating_regime_id: "active".to_string(),
            clock_id: "clock-common".to_string(),
            time_origin_id: "time-origin-common".to_string(),
            step_period_ns: 10,
            horizon_steps: 3,
            scenario_count: 3,
            deterministic_seed: 42,
            residual_pool_evidence_id: residual_method.then(|| "residual-pool-evidence".to_string()),
            minimum_residual_trajectory_count: if residual_method { 3 } else { 0 },
            minimum_residual_origin_separation_ns: if residual_method { 30 } else { 0 },
        }
    }

    fn residuals() -> Vec<ForecastResidualTrajectory> {
        vec![
            ForecastResidualTrajectory {
                residual_id: "residual-a".to_string(),
                origin_id: "origin-a".to_string(),
                operating_regime_id: "active".to_string(),
                origin_timestamp_ns: 1_000,
                target_window_end_ns: 1_030,
                clock_id: "clock-common".to_string(),
                time_origin_id: "time-origin-common".to_string(),
                model_contract: model_contract(),
                forecast_evidence_id: "forecast-a".to_string(),
                measurement_evidence_id: "measurement-a".to_string(),
                residual_generation_evidence_id: "residual-generation-a".to_string(),
                state_mapping_evidence_id: "state-map-a".to_string(),
                time_alignment_evidence_id: "clock-a".to_string(),
                source_timeline_digest_reference: "timeline-a".to_string(),
                source_sample_timestamps_ns: vec![1_010, 1_020, 1_030],
                source_sample_evidence_ids: vec!["a-1".to_string(), "a-2".to_string(), "a-3".to_string()],
                residual_power_mw: vec![0, 10, 0],
            },
            ForecastResidualTrajectory {
                residual_id: "residual-b".to_string(),
                origin_id: "origin-b".to_string(),
                operating_regime_id: "active".to_string(),
                origin_timestamp_ns: 2_000,
                target_window_end_ns: 2_030,
                clock_id: "clock-common".to_string(),
                time_origin_id: "time-origin-common".to_string(),
                model_contract: model_contract(),
                forecast_evidence_id: "forecast-b".to_string(),
                measurement_evidence_id: "measurement-b".to_string(),
                residual_generation_evidence_id: "residual-generation-b".to_string(),
                state_mapping_evidence_id: "state-map-b".to_string(),
                time_alignment_evidence_id: "clock-b".to_string(),
                source_timeline_digest_reference: "timeline-b".to_string(),
                source_sample_timestamps_ns: vec![2_010, 2_020, 2_030],
                source_sample_evidence_ids: vec!["b-1".to_string(), "b-2".to_string(), "b-3".to_string()],
                residual_power_mw: vec![10, 0, -10],
            },
            ForecastResidualTrajectory {
                residual_id: "residual-c".to_string(),
                origin_id: "origin-c".to_string(),
                operating_regime_id: "active".to_string(),
                origin_timestamp_ns: 3_000,
                target_window_end_ns: 3_030,
                clock_id: "clock-common".to_string(),
                time_origin_id: "time-origin-common".to_string(),
                model_contract: model_contract(),
                forecast_evidence_id: "forecast-c".to_string(),
                measurement_evidence_id: "measurement-c".to_string(),
                residual_generation_evidence_id: "residual-generation-c".to_string(),
                state_mapping_evidence_id: "state-map-c".to_string(),
                time_alignment_evidence_id: "clock-c".to_string(),
                source_timeline_digest_reference: "timeline-c".to_string(),
                source_sample_timestamps_ns: vec![3_010, 3_020, 3_030],
                source_sample_evidence_ids: vec!["c-1".to_string(), "c-2".to_string(), "c-3".to_string()],
                residual_power_mw: vec![-10, -10, 10],
            },
        ]
    }

    fn actual_horizon() -> MeasuredForecastHorizon {
        MeasuredForecastHorizon {
            forecast_id: "forecast-current".to_string(),
            measurement_run_id: "measurement-run-current".to_string(),
            clock_id: "clock-common".to_string(),
            time_origin_id: "time-origin-common".to_string(),
            measurement_evidence_id: "measurement-evidence-current".to_string(),
            time_alignment_evidence_id: "actual-clock-evidence-current".to_string(),
            source_timeline_digest_reference: "actual-timeline-current".to_string(),
            model_contract: model_contract(),
            samples: vec![
                MeasuredForecastHorizonSample {
                    timestamp_ns: 10_010,
                    power_mw: 150,
                    quality: SampleQuality::Good,
                    source_evidence_id: "actual-1".to_string(),
                },
                MeasuredForecastHorizonSample {
                    timestamp_ns: 10_020,
                    power_mw: 160,
                    quality: SampleQuality::Good,
                    source_evidence_id: "actual-2".to_string(),
                },
                MeasuredForecastHorizonSample {
                    timestamp_ns: 10_030,
                    power_mw: 160,
                    quality: SampleQuality::Good,
                    source_evidence_id: "actual-3".to_string(),
                },
            ],
        }
    }

    #[test]
    fn independent_quantile_sampling_is_deterministic_for_frozen_seed() {
        let contract = contract(ScenarioGeneratorKind::IndependentQuantileSampling);
        let first = generate_forecast_scenarios(contract.clone(), &forecast(), &[]).unwrap();
        let second = generate_forecast_scenarios(contract, &forecast(), &[]).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.scenarios.len(), 3);
        assert_eq!(first.selected_residual_ids.len(), 0);
    }

    #[test]
    fn independent_sampling_reports_quantile_tail_clipping() {
        let mut input = forecast();
        for step in &mut input.steps {
            step.quantiles[0].probability = 0.25;
            step.quantiles[2].probability = 0.75;
        }
        let result = generate_forecast_scenarios(
            contract(ScenarioGeneratorKind::IndependentQuantileSampling),
            &input,
            &[],
        ).unwrap();
        assert!(result.marginal_tail_clipped_sample_count > 0);
    }

    #[test]
    fn residual_bootstrap_keeps_whole_error_trajectories_together() {
        let result = generate_forecast_scenarios(
            contract(ScenarioGeneratorKind::EmpiricalResidualTrajectoryBootstrap),
            &forecast(),
            &residuals(),
        ).unwrap();
        for scenario in &result.scenarios {
            let source_id = scenario.source_residual_id.as_ref().unwrap();
            let source = residuals().into_iter().find(|item| &item.residual_id == source_id).unwrap();
            for (index, value) in scenario.power_mw.iter().enumerate() {
                assert_eq!(
                    *value as i128,
                    result.scheduled_median_power_mw[index] as i128
                        + source.residual_power_mw[index] as i128
                );
            }
        }
    }

    #[test]
    fn residual_bootstrap_samples_complete_trajectories_with_replacement() {
        let mut cfg = contract(ScenarioGeneratorKind::EmpiricalResidualTrajectoryBootstrap);
        cfg.scenario_count = 12;
        cfg.minimum_residual_trajectory_count = 3;
        let result = generate_forecast_scenarios(cfg, &forecast(), &residuals()).unwrap();
        assert_eq!(result.scenarios.len(), 12);
        assert!(result.selected_residual_ids.iter().collect::<HashSet<_>>().len() < 12);
        for scenario in &result.scenarios {
            assert_eq!(scenario.source_sample_evidence_ids.len(), 3);
            let residual_id = scenario.source_residual_id.as_ref().unwrap();
            let source = residuals()
                .into_iter()
                .find(|item| &item.residual_id == residual_id)
                .unwrap();
            assert_eq!(
                scenario.source_sample_evidence_ids,
                source.source_sample_evidence_ids
            );
        }
    }

    #[test]
    fn rejects_overlapping_historical_residual_target_windows() {
        let mut pool = residuals();
        pool[1].origin_timestamp_ns = 1_010;
        pool[1].target_window_end_ns = 1_040;
        pool[1].source_sample_timestamps_ns = vec![1_020, 1_030, 1_040];
        assert!(generate_forecast_scenarios(
            contract(ScenarioGeneratorKind::EmpiricalResidualTrajectoryBootstrap),
            &forecast(),
            &pool,
        ).is_err());
    }

    #[test]
    fn schaake_shuffle_preserves_each_forecast_marginal_sample_set() {
        let result = generate_forecast_scenarios(
            contract(ScenarioGeneratorKind::SchaakeShuffle),
            &forecast(),
            &residuals(),
        ).unwrap();
        for horizon_index in 0..3 {
            let generated: Vec<u64> = result.scenarios.iter()
                .map(|scenario| scenario.power_mw[horizon_index])
                .collect();
            let independent = generate_forecast_scenarios(
                contract(ScenarioGeneratorKind::IndependentQuantileSampling),
                &forecast(),
                &[],
            ).unwrap();
            let mut generated_sorted = generated;
            let mut expected_sorted: Vec<u64> = independent.scenarios.iter()
                .map(|scenario| scenario.power_mw[horizon_index])
                .collect();
            generated_sorted.sort_unstable();
            expected_sorted.sort_unstable();
            assert_eq!(generated_sorted, expected_sorted);
        }
    }

    #[test]
    fn rejects_residual_trajectory_with_misaligned_source_sample_timestamps() {
        let mut pool = residuals();
        pool[0].source_sample_timestamps_ns[1] += 1;
        assert!(generate_forecast_scenarios(
            contract(ScenarioGeneratorKind::EmpiricalResidualTrajectoryBootstrap),
            &forecast(),
            &pool,
        ).is_err());
    }

    #[test]
    fn rejects_residual_trajectory_that_uses_future_information() {
        let mut pool = residuals();
        pool[0].target_window_end_ns = 10_001;
        assert!(generate_forecast_scenarios(
            contract(ScenarioGeneratorKind::EmpiricalResidualTrajectoryBootstrap),
            &forecast(),
            &pool,
        ).is_err());
    }

    #[test]
    fn rejects_residual_clock_identity_mismatch() {
        let mut pool = residuals();
        pool[0].clock_id = "other-clock".to_string();
        assert!(generate_forecast_scenarios(
            contract(ScenarioGeneratorKind::EmpiricalResidualTrajectoryBootstrap),
            &forecast(),
            &pool,
        ).is_err());
    }

    #[test]
    fn rejects_measurement_time_origin_mismatch() {
        let scenarios = generate_forecast_scenarios(
            contract(ScenarioGeneratorKind::IndependentQuantileSampling),
            &forecast(),
            &[],
        ).unwrap();
        let mut actual = actual_horizon();
        actual.time_origin_id = "other-time-origin".to_string();
        assert!(evaluate_forecast_scenarios(&scenarios, &actual, 0.10).is_err());
    }

    #[test]
    fn rejects_residual_model_contract_mismatch() {
        let mut pool = residuals();
        pool[0].model_contract.model_revision = "rev-b".to_string();
        assert!(generate_forecast_scenarios(
            contract(ScenarioGeneratorKind::SchaakeShuffle),
            &forecast(),
            &pool,
        ).is_err());
    }

    #[test]
    fn rejects_residual_pool_reusing_current_forecast_origin_id() {
        let mut pool = residuals();
        pool[0].origin_id = forecast().origin_id;
        assert!(generate_forecast_scenarios(
            contract(ScenarioGeneratorKind::EmpiricalResidualTrajectoryBootstrap),
            &forecast(),
            &pool,
        ).is_err());
    }

    #[test]
    fn rejects_residual_pool_reusing_sample_evidence_across_blocks() {
        let mut pool = residuals();
        pool[1].source_sample_evidence_ids[0] =
            pool[0].source_sample_evidence_ids[0].clone();
        assert!(generate_forecast_scenarios(
            contract(ScenarioGeneratorKind::EmpiricalResidualTrajectoryBootstrap),
            &forecast(),
            &pool,
        ).is_err());
    }

    #[test]
    fn rejects_crossing_forecast_quantiles() {
        let mut input = forecast();
        input.steps[1].quantiles[1].power_mw = 70;
        assert!(generate_forecast_scenarios(
            contract(ScenarioGeneratorKind::IndependentQuantileSampling),
            &input,
            &[],
        ).is_err());
    }

    #[test]
    fn rejects_misaligned_forecast_horizon_timestamps() {
        let mut input = forecast();
        input.steps[2].timestamp_ns += 1;
        assert!(generate_forecast_scenarios(
            contract(ScenarioGeneratorKind::IndependentQuantileSampling),
            &input,
            &[],
        ).is_err());
    }

    #[test]
    fn rejects_insufficient_residual_pool_support() {
        let mut pool = residuals();
        pool.pop();
        assert!(generate_forecast_scenarios(
            contract(ScenarioGeneratorKind::SchaakeShuffle),
            &forecast(),
            &pool,
        ).is_err());
    }

    #[test]
    fn reserve_sizing_uses_horizon_maximum_and_declared_critical_fractile() {
        let scenarios = generate_forecast_scenarios(
            contract(ScenarioGeneratorKind::SchaakeShuffle),
            &forecast(),
            &residuals(),
        ).unwrap();
        let report = evaluate_forecast_scenarios(&scenarios, &actual_horizon(), 0.10).unwrap();
        assert_eq!(report.reserve_critical_fractile, 0.90);
        assert!(report.reserve_power_mw > 0);
        assert!(report.reserve_shortfall_occurred);
        assert!(report.realized_reserve_cost_ratio_units.is_finite());
    }

    #[test]
    fn scenario_evaluation_reports_energy_and_variogram_scores() {
        let scenarios = generate_forecast_scenarios(
            contract(ScenarioGeneratorKind::EmpiricalResidualTrajectoryBootstrap),
            &forecast(),
            &residuals(),
        ).unwrap();
        let report = evaluate_forecast_scenarios(&scenarios, &actual_horizon(), 0.10).unwrap();
        assert!(report.energy_score_mw.is_finite());
        assert!(report.variogram_score.is_finite());
        assert!(report.actual_energy_mw_ns > 0);
        assert!(report.scenario_energy_p90_mw_ns >= report.scenario_energy_p10_mw_ns);
        assert!(report.actual_peak_pit.is_finite());
        assert!(report.actual_horizon_energy_pit.is_finite());
        assert_eq!(report.model_contract, model_contract());
        assert_eq!(report.forecast_evidence_id, "forecast-evidence-current");
        assert_eq!(report.forecast_time_alignment_evidence_id, "clock-current");
        assert_eq!(
            report.measurement_time_alignment_evidence_id,
            "actual-clock-evidence-current"
        );
        assert_eq!(report.selected_residual_ids.len(), 3);
        assert_eq!(report.selected_residual_generation_evidence_ids.len(), 3);
        assert_eq!(report.selected_residual_state_mapping_evidence_ids.len(), 3);
        assert_eq!(report.selected_residual_time_alignment_evidence_ids.len(), 3);
        assert!(report.selected_residual_sample_evidence_ids.iter().all(|ids| ids.len() == 3));
        assert!(report.selected_residual_sample_timestamps_ns.iter().all(|times| times.len() == 3));
    }

    #[test]
    fn evaluation_rejects_tampered_claim_ceiling() {
        let mut scenarios = generate_forecast_scenarios(
            contract(ScenarioGeneratorKind::IndependentQuantileSampling),
            &forecast(),
            &[],
        ).unwrap();
        scenarios.claim_ceiling = "QUALIFIED";
        assert!(evaluate_forecast_scenarios(&scenarios, &actual_horizon(), 0.10).is_err());
    }

    #[test]
    fn evaluation_rejects_tampered_residual_sample_timestamps() {
        let mut scenarios = generate_forecast_scenarios(
            contract(ScenarioGeneratorKind::SchaakeShuffle),
            &forecast(),
            &residuals(),
        ).unwrap();
        scenarios.scenarios[0].source_sample_timestamps_ns[1] += 1;
        assert!(evaluate_forecast_scenarios(&scenarios, &actual_horizon(), 0.10).is_err());
    }

    #[test]
    fn evaluation_rejects_bad_measurement_quality() {
        let scenarios = generate_forecast_scenarios(
            contract(ScenarioGeneratorKind::IndependentQuantileSampling),
            &forecast(),
            &[],
        ).unwrap();
        let mut actual = actual_horizon();
        actual.samples[1].quality = SampleQuality::Missing;
        assert!(evaluate_forecast_scenarios(&scenarios, &actual, 0.10).is_err());
    }

    #[test]
    fn evaluation_requires_exact_actual_timestamps() {
        let scenarios = generate_forecast_scenarios(
            contract(ScenarioGeneratorKind::IndependentQuantileSampling),
            &forecast(),
            &[],
        ).unwrap();
        let mut actual = actual_horizon();
        actual.samples[1].timestamp_ns += 1;
        assert!(evaluate_forecast_scenarios(&scenarios, &actual, 0.10).is_err());
    }

    #[test]
    fn residual_pool_requires_unique_trajectory_and_origin_ids() {
        let mut pool = residuals();
        pool[1].residual_id = pool[0].residual_id.clone();
        assert!(generate_forecast_scenarios(
            contract(ScenarioGeneratorKind::EmpiricalResidualTrajectoryBootstrap),
            &forecast(),
            &pool,
        ).is_err());
    }

    #[test]
    fn independent_generator_rejects_residual_pool_data() {
        assert!(generate_forecast_scenarios(
            contract(ScenarioGeneratorKind::IndependentQuantileSampling),
            &forecast(),
            &residuals(),
        ).is_err());
    }

    #[test]
    fn residual_underflow_is_clamped_and_reported() {
        let mut pool = residuals();
        pool[0].residual_power_mw[0] = -1_000;
        let result = generate_forecast_scenarios(
            contract(ScenarioGeneratorKind::EmpiricalResidualTrajectoryBootstrap),
            &forecast(),
            &pool,
        ).unwrap();
        assert!(result.residual_floor_clamped_sample_count > 0);
    }

    #[test]
    fn scenario_set_retains_modelled_claim_ceiling() {
        let result = generate_forecast_scenarios(
            contract(ScenarioGeneratorKind::IndependentQuantileSampling),
            &forecast(),
            &[],
        ).unwrap();
        assert_eq!(
            result.claim_ceiling,
            "MODELLED_TEMPORAL_POWER_SCENARIOS_NOT_MEASUREMENT_OR_GRID_RESERVE_QUALIFICATION"
        );
    }
}
