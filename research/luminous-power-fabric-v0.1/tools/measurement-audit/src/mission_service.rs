use std::collections::HashSet;

const EPSILON: f64 = 1.0e-12;

/// Evidence state supplied by the metrology/evidence layer. This evaluator does
/// not authenticate the evidence; it fails closed unless each state is Current.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceState {
    Current,
    Missing,
    Unverified,
    Invalid,
    Expired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MissionServiceBlocker {
    ConfigurationDigestMalformed,
    ConfigurationMismatch,
    QualificationExpired,
    MeasurementEvidenceNotCurrent,
    CalibrationEvidenceNotCurrent,
    TimebaseEvidenceNotCurrent,
    ChannelEvidenceNotCurrent,
    CaptureCompletenessNotCurrent,
    EnergyBalanceOutsideTolerance,
    ServiceBalanceOutsideTolerance,
    RequiredDisturbanceObservationsMissing,
    RecoveryMeasurementMissing,
    VoltageMeasurementMissing,
    FrequencyMeasurementMissing,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MissionServiceError(pub String);

impl std::fmt::Display for MissionServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for MissionServiceError {}

#[derive(Debug, Clone, PartialEq)]
pub struct MissionMeasurementLineage {
    pub measurement_run_id: String,
    pub raw_capture_sha256: String,
    pub channel_manifest_sha256: String,
    pub qualified_configuration_sha256: String,
    pub observed_configuration_sha256: String,
    pub timebase_reference: String,
    pub calibration_evidence_reference: String,
    pub uncertainty_evidence_reference: String,
    pub measurement_status: EvidenceState,
    pub calibration_status: EvidenceState,
    pub timebase_status: EvidenceState,
    pub channel_status: EvidenceState,
    pub completeness_status: EvidenceState,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeasuredMissionInterval {
    /// Half-open interval [start_epoch_s, start_epoch_s + duration_s).
    pub start_epoch_s: u64,
    pub duration_s: u64,
    /// Energy demanded by the declared useful-service boundary.
    pub demanded_energy_kwh: f64,
    /// Energy actually delivered to that same useful-service boundary.
    pub delivered_energy_kwh: f64,
    /// Demand not delivered. It is service deficit, not supplied energy.
    pub unserved_energy_kwh: f64,
    /// Source energy entering the common service bus.
    pub source_energy_kwh: f64,
    /// Storage discharge energy delivered to the common service bus.
    pub storage_discharge_energy_kwh: f64,
    /// Energy drawn from the common service bus into storage.
    pub storage_charge_energy_kwh: f64,
    /// Declared conversion losses and system auxiliaries at the same boundary.
    pub auxiliary_and_loss_energy_kwh: f64,
    /// Independently observed stored energy at the storage-internal boundary.
    pub stored_energy_start_kwh: f64,
    pub stored_energy_end_kwh: f64,
    /// Fuel consumed during the interval; zero for non-fuel sources.
    pub fuel_liters: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DisturbanceObservation {
    pub event_id: String,
    pub event_time_epoch_s: u64,
    /// Elapsed time from event to the declared recovered state.
    pub recovery_time_s: Option<f64>,
    /// Peak absolute deviation from the declared nominal voltage.
    pub peak_voltage_deviation_percent: Option<f64>,
    /// Peak absolute deviation from the declared nominal frequency.
    pub peak_frequency_deviation_hz: Option<f64>,
    pub quality_and_recovery_evidence_reference: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MissionServiceRequirements {
    pub minimum_demanded_energy_kwh: f64,
    pub max_unserved_energy_kwh: f64,
    pub max_unserved_fraction: f64,
    pub energy_balance_tolerance_kwh: f64,
    pub service_balance_tolerance_kwh: f64,
    pub storage_state_tolerance_kwh: f64,
    pub min_terminal_storage_energy_kwh: Option<f64>,
    pub max_fuel_l_per_delivered_kwh: Option<f64>,
    pub max_recovery_time_s: Option<f64>,
    pub max_voltage_deviation_percent: Option<f64>,
    pub max_frequency_deviation_hz: Option<f64>,
    /// Must be non-zero whenever a transient/recovery threshold is requested.
    pub minimum_disturbance_observations: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MissionServiceInput {
    pub mission_id: String,
    pub evaluated_at_epoch_s: u64,
    pub qualification_valid_until_epoch_s: u64,
    pub mission_start_epoch_s: u64,
    pub mission_end_epoch_s: u64,
    pub qualified_configuration_sha256: String,
    pub observed_configuration_sha256: String,
    pub storage_usable_capacity_kwh: f64,
    pub intervals: Vec<MeasuredMissionInterval>,
    pub disturbances: Vec<DisturbanceObservation>,
    pub requirements: MissionServiceRequirements,
    pub lineage: MissionMeasurementLineage,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MissionServiceReport {
    pub mission_id: String,
    pub evidence_eligible: bool,
    /// None means the data/evidence gates did not permit a service verdict.
    pub service_requirements_met: Option<bool>,
    pub hard_blockers: Vec<MissionServiceBlocker>,
    pub mission_duration_s: u64,
    pub interval_count: usize,
    pub disturbance_count: usize,
    pub demanded_energy_kwh: f64,
    pub delivered_energy_kwh: f64,
    pub unserved_energy_kwh: f64,
    pub unserved_fraction: f64,
    pub source_energy_kwh: f64,
    pub storage_discharge_energy_kwh: f64,
    pub storage_charge_energy_kwh: f64,
    pub auxiliary_and_loss_energy_kwh: f64,
    pub initial_stored_energy_kwh: f64,
    pub terminal_stored_energy_kwh: f64,
    pub net_stored_energy_change_kwh: f64,
    pub fuel_liters: f64,
    pub fuel_l_per_delivered_kwh: Option<f64>,
    pub worst_interval_energy_balance_residual_kwh: f64,
    pub total_energy_balance_residual_kwh: f64,
    pub worst_interval_service_balance_residual_kwh: f64,
    pub max_recovery_time_s: Option<f64>,
    pub max_voltage_deviation_percent: Option<f64>,
    pub max_frequency_deviation_hz: Option<f64>,
    pub lineage: MissionMeasurementLineage,
    /// Original event-level records and their evidence references are retained.
    pub disturbances: Vec<DisturbanceObservation>,
}

impl MissionServiceReport {
    /// This is an offline evidence and arithmetic audit, not a product qualification.
    pub const fn claim_ceiling(&self) -> &'static str {
        "MEASURED_MISSION_SERVICE_AUDIT_ONLY_NOT_DISPATCH_SAFETY_OR_PHYSICAL_QUALIFICATION"
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn require_non_empty(value: &str, field: &str) -> Result<(), MissionServiceError> {
    if value.trim().is_empty() {
        return Err(MissionServiceError(format!("{field} must be non-empty")));
    }
    Ok(())
}

fn require_non_negative_finite(value: f64, field: &str) -> Result<(), MissionServiceError> {
    if !value.is_finite() || value < 0.0 {
        return Err(MissionServiceError(format!(
            "{field} must be finite and non-negative"
        )));
    }
    Ok(())
}

fn require_optional_non_negative_finite(
    value: Option<f64>,
    field: &str,
) -> Result<(), MissionServiceError> {
    if let Some(value) = value {
        require_non_negative_finite(value, field)?;
    }
    Ok(())
}

fn add_finite(total: &mut f64, value: f64, field: &str) -> Result<(), MissionServiceError> {
    *total += value;
    if !total.is_finite() {
        return Err(MissionServiceError(format!(
            "accumulated {field} is non-finite"
        )));
    }
    Ok(())
}

fn max_optional(current: &mut Option<f64>, value: Option<f64>) {
    if let Some(value) = value {
        *current = Some((*current).map_or(value, |previous| previous.max(value)));
    }
}

/// Evaluate a completed, measured mission against a frozen acceptance contract.
///
/// All energy terms must refer to the same common service-bus boundary, except
/// stored_energy_* which refer to the storage-internal state boundary. This
/// function verifies only declared arithmetic and state bounds. It does not
/// authenticate meter evidence, estimate battery state-of-health, model a plant,
/// control equipment, authorize dispatch, or establish physical qualification.
pub fn evaluate_mission_service(
    input: &MissionServiceInput,
) -> Result<MissionServiceReport, MissionServiceError> {
    require_non_empty(&input.mission_id, "mission_id")?;
    if input.mission_start_epoch_s >= input.mission_end_epoch_s {
        return Err(MissionServiceError(
            "mission end must be after mission start".to_string(),
        ));
    }
    if input.evaluated_at_epoch_s < input.mission_end_epoch_s {
        return Err(MissionServiceError(
            "evaluation time must be at or after the completed mission end".to_string(),
        ));
    }
    if input.intervals.is_empty() {
        return Err(MissionServiceError(
            "at least one measured mission interval is required".to_string(),
        ));
    }

    require_non_negative_finite(
        input.storage_usable_capacity_kwh,
        "storage_usable_capacity_kwh",
    )?;
    let requirements = &input.requirements;
    require_non_negative_finite(
        requirements.minimum_demanded_energy_kwh,
        "minimum_demanded_energy_kwh",
    )?;
    if requirements.minimum_demanded_energy_kwh <= 0.0 {
        return Err(MissionServiceError(
            "minimum_demanded_energy_kwh must be positive to prevent empty-service passes".to_string(),
        ));
    }
    require_non_negative_finite(
        requirements.max_unserved_energy_kwh,
        "max_unserved_energy_kwh",
    )?;
    require_non_negative_finite(
        requirements.max_unserved_fraction,
        "max_unserved_fraction",
    )?;
    if requirements.max_unserved_fraction > 1.0 {
        return Err(MissionServiceError(
            "max_unserved_fraction must not exceed 1".to_string(),
        ));
    }
    require_non_negative_finite(
        requirements.energy_balance_tolerance_kwh,
        "energy_balance_tolerance_kwh",
    )?;
    require_non_negative_finite(
        requirements.service_balance_tolerance_kwh,
        "service_balance_tolerance_kwh",
    )?;
    require_non_negative_finite(
        requirements.storage_state_tolerance_kwh,
        "storage_state_tolerance_kwh",
    )?;
    require_optional_non_negative_finite(
        requirements.min_terminal_storage_energy_kwh,
        "min_terminal_storage_energy_kwh",
    )?;
    require_optional_non_negative_finite(
        requirements.max_fuel_l_per_delivered_kwh,
        "max_fuel_l_per_delivered_kwh",
    )?;
    require_optional_non_negative_finite(
        requirements.max_recovery_time_s,
        "max_recovery_time_s",
    )?;
    require_optional_non_negative_finite(
        requirements.max_voltage_deviation_percent,
        "max_voltage_deviation_percent",
    )?;
    require_optional_non_negative_finite(
        requirements.max_frequency_deviation_hz,
        "max_frequency_deviation_hz",
    )?;

    if requirements.max_unserved_fraction > 0.0 && requirements.max_unserved_energy_kwh == 0.0 {
        // This is valid but intentionally strict: the absolute limit dominates.
    }
    let dynamic_threshold_requested = requirements.max_recovery_time_s.is_some()
        || requirements.max_voltage_deviation_percent.is_some()
        || requirements.max_frequency_deviation_hz.is_some();
    if dynamic_threshold_requested && requirements.minimum_disturbance_observations == 0 {
        return Err(MissionServiceError(
            "minimum_disturbance_observations must be positive when dynamic thresholds are requested"
                .to_string(),
        ));
    }

    let lineage = &input.lineage;
    require_non_empty(&lineage.measurement_run_id, "measurement_run_id")?;
    require_non_empty(&lineage.timebase_reference, "timebase_reference")?;
    require_non_empty(
        &lineage.calibration_evidence_reference,
        "calibration_evidence_reference",
    )?;
    require_non_empty(
        &lineage.uncertainty_evidence_reference,
        "uncertainty_evidence_reference",
    )?;
    if !is_sha256(&input.qualified_configuration_sha256)
        || !is_sha256(&input.observed_configuration_sha256)
        || !is_sha256(&lineage.raw_capture_sha256)
        || !is_sha256(&lineage.channel_manifest_sha256)
        || !is_sha256(&lineage.qualified_configuration_sha256)
        || !is_sha256(&lineage.observed_configuration_sha256)
    {
        // Malformed digests are represented as blockers so the report can retain
        // the submitted values, but an entirely missing lineage reference is invalid input.
    }

    if input.storage_usable_capacity_kwh == 0.0
        && requirements
            .min_terminal_storage_energy_kwh
            .is_some_and(|minimum| minimum > requirements.storage_state_tolerance_kwh)
    {
        return Err(MissionServiceError(
            "terminal storage minimum exceeds a zero-capacity storage boundary".to_string(),
        ));
    }
    if input.storage_usable_capacity_kwh > 0.0
        && requirements.min_terminal_storage_energy_kwh.is_none()
    {
        return Err(MissionServiceError(
            "storage-equipped missions must declare a terminal energy floor".to_string(),
        ));
    }
    if requirements
        .min_terminal_storage_energy_kwh
        .is_some_and(|minimum| minimum > input.storage_usable_capacity_kwh + requirements.storage_state_tolerance_kwh)
    {
        return Err(MissionServiceError(
            "terminal storage energy floor exceeds usable storage capacity".to_string(),
        ));
    }

    let mut blockers = Vec::new();
    let add_blocker = |items: &mut Vec<MissionServiceBlocker>, blocker| {
        if !items.contains(&blocker) {
            items.push(blocker);
        }
    };

    let digests_well_formed = is_sha256(&input.qualified_configuration_sha256)
        && is_sha256(&input.observed_configuration_sha256)
        && is_sha256(&lineage.raw_capture_sha256)
        && is_sha256(&lineage.channel_manifest_sha256)
        && is_sha256(&lineage.qualified_configuration_sha256)
        && is_sha256(&lineage.observed_configuration_sha256);
    if !digests_well_formed {
        add_blocker(&mut blockers, MissionServiceBlocker::ConfigurationDigestMalformed);
    }
    if input.qualified_configuration_sha256.to_ascii_lowercase()
        != input.observed_configuration_sha256.to_ascii_lowercase()
        || lineage.qualified_configuration_sha256.to_ascii_lowercase()
            != input.qualified_configuration_sha256.to_ascii_lowercase()
        || lineage.observed_configuration_sha256.to_ascii_lowercase()
            != input.observed_configuration_sha256.to_ascii_lowercase()
    {
        add_blocker(&mut blockers, MissionServiceBlocker::ConfigurationMismatch);
    }
    if input.evaluated_at_epoch_s > input.qualification_valid_until_epoch_s {
        add_blocker(&mut blockers, MissionServiceBlocker::QualificationExpired);
    }
    if lineage.measurement_status != EvidenceState::Current {
        add_blocker(&mut blockers, MissionServiceBlocker::MeasurementEvidenceNotCurrent);
    }
    if lineage.calibration_status != EvidenceState::Current {
        add_blocker(&mut blockers, MissionServiceBlocker::CalibrationEvidenceNotCurrent);
    }
    if lineage.timebase_status != EvidenceState::Current {
        add_blocker(&mut blockers, MissionServiceBlocker::TimebaseEvidenceNotCurrent);
    }
    if lineage.channel_status != EvidenceState::Current {
        add_blocker(&mut blockers, MissionServiceBlocker::ChannelEvidenceNotCurrent);
    }
    if lineage.completeness_status != EvidenceState::Current {
        add_blocker(&mut blockers, MissionServiceBlocker::CaptureCompletenessNotCurrent);
    }

    let mut demanded = 0.0_f64;
    let mut delivered = 0.0_f64;
    let mut unserved = 0.0_f64;
    let mut source = 0.0_f64;
    let mut storage_discharge = 0.0_f64;
    let mut storage_charge = 0.0_f64;
    let mut losses = 0.0_f64;
    let mut fuel = 0.0_f64;
    let mut worst_energy_residual = 0.0_f64;
    let mut worst_service_residual = 0.0_f64;
    let mut total_energy_residual = 0.0_f64;
    let mut expected_start = input.mission_start_epoch_s;
    let mut initial_stored_energy = 0.0_f64;
    let mut terminal_stored_energy = 0.0_f64;
    let mut previous_storage_end: Option<f64> = None;

    for (index, interval) in input.intervals.iter().enumerate() {
        if interval.duration_s == 0 {
            return Err(MissionServiceError(format!(
                "interval {index} duration must be positive"
            )));
        }
        if interval.start_epoch_s != expected_start {
            return Err(MissionServiceError(format!(
                "interval {index} is not contiguous with the preceding interval"
            )));
        }
        let interval_end = interval
            .start_epoch_s
            .checked_add(interval.duration_s)
            .ok_or_else(|| {
                MissionServiceError(format!("interval {index} end timestamp overflows"))
            })?;
        if interval_end > input.mission_end_epoch_s {
            return Err(MissionServiceError(format!(
                "interval {index} extends beyond mission_end_epoch_s"
            )));
        }
        expected_start = interval_end;

        let values = [
            ("demanded_energy_kwh", interval.demanded_energy_kwh),
            ("delivered_energy_kwh", interval.delivered_energy_kwh),
            ("unserved_energy_kwh", interval.unserved_energy_kwh),
            ("source_energy_kwh", interval.source_energy_kwh),
            (
                "storage_discharge_energy_kwh",
                interval.storage_discharge_energy_kwh,
            ),
            (
                "storage_charge_energy_kwh",
                interval.storage_charge_energy_kwh,
            ),
            (
                "auxiliary_and_loss_energy_kwh",
                interval.auxiliary_and_loss_energy_kwh,
            ),
            ("stored_energy_start_kwh", interval.stored_energy_start_kwh),
            ("stored_energy_end_kwh", interval.stored_energy_end_kwh),
            ("fuel_liters", interval.fuel_liters),
        ];
        for (field, value) in values {
            require_non_negative_finite(value, &format!("interval {index} {field}"))?;
        }

        if interval.delivered_energy_kwh > interval.demanded_energy_kwh
            + requirements.service_balance_tolerance_kwh
        {
            return Err(MissionServiceError(format!(
                "interval {index} delivered energy exceeds demanded energy"
            )));
        }
        if interval.unserved_energy_kwh > interval.demanded_energy_kwh
            + requirements.service_balance_tolerance_kwh
        {
            return Err(MissionServiceError(format!(
                "interval {index} unserved energy exceeds demanded energy"
            )));
        }
        if interval.storage_energy_start_kwh > input.storage_usable_capacity_kwh
            + requirements.storage_state_tolerance_kwh
            || interval.stored_energy_end_kwh > input.storage_usable_capacity_kwh
                + requirements.storage_state_tolerance_kwh
        {
            return Err(MissionServiceError(format!(
                "interval {index} storage state exceeds usable capacity"
            )));
        }
        if let Some(previous_end) = previous_storage_end {
            if (interval.stored_energy_start_kwh - previous_end).abs()
                > requirements.storage_state_tolerance_kwh
            {
                return Err(MissionServiceError(format!(
                    "interval {index} storage state does not reconcile with previous interval"
                )));
            }
        } else {
            initial_stored_energy = interval.stored_energy_start_kwh;
        }

        let is_charging = interval.storage_charge_energy_kwh > EPSILON;
        let is_discharging = interval.storage_discharge_energy_kwh > EPSILON;
        if is_charging && is_discharging {
            return Err(MissionServiceError(format!(
                "interval {index} mixes storage charge and discharge; split the interval at the direction change"
            )));
        }
        let stored_delta =
            interval.stored_energy_end_kwh - interval.stored_energy_start_kwh;
        let state_tolerance = requirements.storage_state_tolerance_kwh;
        if is_charging {
            if stored_delta < -state_tolerance
                || stored_delta > interval.storage_charge_energy_kwh + state_tolerance
            {
                return Err(MissionServiceError(format!(
                    "interval {index} stored-energy increase is inconsistent with bus-side charge energy"
                )));
            }
        } else if is_discharging {
            if stored_delta > state_tolerance
                || -stored_delta > interval.storage_discharge_energy_kwh + state_tolerance
            {
                return Err(MissionServiceError(format!(
                    "interval {index} stored-energy decrease is inconsistent with bus-side discharge energy"
                )));
            }
        } else if stored_delta.abs() > state_tolerance {
            return Err(MissionServiceError(format!(
                "interval {index} storage state changes without a charge or discharge flow"
            )));
        }
        terminal_stored_energy = interval.stored_energy_end_kwh;
        previous_storage_end = Some(interval.stored_energy_end_kwh);

        let service_residual = (interval.demanded_energy_kwh
            - interval.delivered_energy_kwh
            - interval.unserved_energy_kwh)
            .abs();
        worst_service_residual = worst_service_residual.max(service_residual);
        if service_residual > requirements.service_balance_tolerance_kwh {
            add_blocker(
                &mut blockers,
                MissionServiceBlocker::ServiceBalanceOutsideTolerance,
            );
        }

        let supplied = interval.source_energy_kwh + interval.storage_discharge_energy_kwh;
        let accounted = interval.delivered_energy_kwh
            + interval.storage_charge_energy_kwh
            + interval.auxiliary_and_loss_energy_kwh;
        let energy_residual = (supplied - accounted).abs();
        worst_energy_residual = worst_energy_residual.max(energy_residual);
        add_finite(
            &mut total_energy_residual,
            energy_residual,
            "total energy residual",
        )?;
        if energy_residual > requirements.energy_balance_tolerance_kwh {
            add_blocker(
                &mut blockers,
                MissionServiceBlocker::EnergyBalanceOutsideTolerance,
            );
        }

        add_finite(&mut demanded, interval.demanded_energy_kwh, "demanded energy")?;
        add_finite(&mut delivered, interval.delivered_energy_kwh, "delivered energy")?;
        add_finite(&mut unserved, interval.unserved_energy_kwh, "unserved energy")?;
        add_finite(&mut source, interval.source_energy_kwh, "source energy")?;
        add_finite(
            &mut storage_discharge,
            interval.storage_discharge_energy_kwh,
            "storage discharge energy",
        )?;
        add_finite(
            &mut storage_charge,
            interval.storage_charge_energy_kwh,
            "storage charge energy",
        )?;
        add_finite(
            &mut losses,
            interval.auxiliary_and_loss_energy_kwh,
            "auxiliary/loss energy",
        )?;
        add_finite(&mut fuel, interval.fuel_liters, "fuel consumption")?;
    }

    if expected_start != input.mission_end_epoch_s {
        return Err(MissionServiceError(
            "measured intervals do not cover the complete declared mission".to_string(),
        ));
    }
    if initial_stored_energy > input.storage_usable_capacity_kwh
        + requirements.storage_state_tolerance_kwh
        || terminal_stored_energy > input.storage_usable_capacity_kwh
            + requirements.storage_state_tolerance_kwh
    {
        return Err(MissionServiceError(
            "initial or terminal storage state exceeds usable capacity".to_string(),
        ));
    }
    if let Some(minimum_terminal_energy) = requirements.min_terminal_storage_energy_kwh {
        if minimum_terminal_energy + requirements.storage_state_tolerance_kwh
            < initial_stored_energy
        {
            return Err(MissionServiceError(
                "terminal storage floor must be at least the measured initial stored energy; explicit depletion accounting is a separate benchmark".to_string(),
            ));
        }
    }

    let mut seen_event_ids = HashSet::new();
    let mut max_recovery = None;
    let mut max_voltage_deviation = None;
    let mut max_frequency_deviation = None;
    for event in &input.disturbances {
        require_non_empty(&event.event_id, "disturbance event_id")?;
        require_non_empty(
            &event.quality_and_recovery_evidence_reference,
            "quality_and_recovery_evidence_reference",
        )?;
        if !seen_event_ids.insert(event.event_id.as_str()) {
            return Err(MissionServiceError(format!(
                "duplicate disturbance event_id {:?}",
                event.event_id
            )));
        }
        if event.event_time_epoch_s < input.mission_start_epoch_s
            || event.event_time_epoch_s >= input.mission_end_epoch_s
        {
            return Err(MissionServiceError(format!(
                "disturbance event {:?} lies outside the mission window",
                event.event_id
            )));
        }
        require_optional_non_negative_finite(event.recovery_time_s, "recovery_time_s")?;
        require_optional_non_negative_finite(
            event.peak_voltage_deviation_percent,
            "peak_voltage_deviation_percent",
        )?;
        require_optional_non_negative_finite(
            event.peak_frequency_deviation_hz,
            "peak_frequency_deviation_hz",
        )?;
        max_optional(&mut max_recovery, event.recovery_time_s);
        max_optional(
            &mut max_voltage_deviation,
            event.peak_voltage_deviation_percent,
        );
        max_optional(
            &mut max_frequency_deviation,
            event.peak_frequency_deviation_hz,
        );
    }

    if input.disturbances.len() < requirements.minimum_disturbance_observations {
        add_blocker(
            &mut blockers,
            MissionServiceBlocker::RequiredDisturbanceObservationsMissing,
        );
    }
    if requirements.max_recovery_time_s.is_some()
        && input.disturbances.iter().any(|event| event.recovery_time_s.is_none())
    {
        add_blocker(
            &mut blockers,
            MissionServiceBlocker::RecoveryMeasurementMissing,
        );
    }
    if requirements.max_voltage_deviation_percent.is_some()
        && input
            .disturbances
            .iter()
            .any(|event| event.peak_voltage_deviation_percent.is_none())
    {
        add_blocker(
            &mut blockers,
            MissionServiceBlocker::VoltageMeasurementMissing,
        );
    }
    if requirements.max_frequency_deviation_hz.is_some()
        && input
            .disturbances
            .iter()
            .any(|event| event.peak_frequency_deviation_hz.is_none())
    {
        add_blocker(
            &mut blockers,
            MissionServiceBlocker::FrequencyMeasurementMissing,
        );
    }

    let demand_denominator = demanded.max(EPSILON);
    let unserved_fraction = if demanded > 0.0 {
        unserved / demand_denominator
    } else {
        0.0
    };
    let fuel_intensity = if delivered > 0.0 {
        Some(fuel / delivered)
    } else {
        None
    };
    let evidence_eligible = blockers.is_empty();

    let service_gates_met = demanded + EPSILON >= requirements.minimum_demanded_energy_kwh
        && unserved <= requirements.max_unserved_energy_kwh + EPSILON
        && unserved_fraction <= requirements.max_unserved_fraction + EPSILON
        && requirements
            .min_terminal_storage_energy_kwh
            .is_none_or(|minimum| terminal_stored_energy + EPSILON >= minimum)
        && requirements.max_fuel_l_per_delivered_kwh.is_none_or(|maximum| {
            fuel_intensity.is_some_and(|actual| actual <= maximum + EPSILON)
        })
        && requirements
            .max_recovery_time_s
            .is_none_or(|maximum| max_recovery.is_some_and(|actual| actual <= maximum + EPSILON))
        && requirements.max_voltage_deviation_percent.is_none_or(|maximum| {
            max_voltage_deviation.is_some_and(|actual| actual <= maximum + EPSILON)
        })
        && requirements.max_frequency_deviation_hz.is_none_or(|maximum| {
            max_frequency_deviation.is_some_and(|actual| actual <= maximum + EPSILON)
        });

    Ok(MissionServiceReport {
        mission_id: input.mission_id.clone(),
        evidence_eligible,
        service_requirements_met: if evidence_eligible {
            Some(service_gates_met)
        } else {
            None
        },
        hard_blockers: blockers,
        mission_duration_s: input.mission_end_epoch_s - input.mission_start_epoch_s,
        interval_count: input.intervals.len(),
        disturbance_count: input.disturbances.len(),
        demanded_energy_kwh: demanded,
        delivered_energy_kwh: delivered,
        unserved_energy_kwh: unserved,
        unserved_fraction,
        source_energy_kwh: source,
        storage_discharge_energy_kwh: storage_discharge,
        storage_charge_energy_kwh: storage_charge,
        auxiliary_and_loss_energy_kwh: losses,
        initial_stored_energy_kwh: initial_stored_energy,
        terminal_stored_energy_kwh: terminal_stored_energy,
        net_stored_energy_change_kwh: terminal_stored_energy - initial_stored_energy,
        fuel_liters: fuel,
        fuel_l_per_delivered_kwh: fuel_intensity,
        worst_interval_energy_balance_residual_kwh: worst_energy_residual,
        total_energy_balance_residual_kwh: total_energy_residual,
        worst_interval_service_balance_residual_kwh: worst_service_residual,
        max_recovery_time_s: max_recovery,
        max_voltage_deviation_percent: max_voltage_deviation,
        max_frequency_deviation_hz: max_frequency_deviation,
        lineage: input.lineage.clone(),
        disturbances: input.disturbances.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::{
        evaluate_mission_service, DisturbanceObservation, EvidenceState, MissionMeasurementLineage,
        MissionServiceBlocker, MissionServiceInput, MissionServiceRequirements,
        MeasuredMissionInterval,
    };

    fn interval(
        start_epoch_s: u64,
        stored_start: f64,
        stored_end: f64,
        demanded: f64,
        delivered: f64,
        unserved: f64,
        source: f64,
        discharge: f64,
        charge: f64,
        losses: f64,
        fuel: f64,
    ) -> MeasuredMissionInterval {
        MeasuredMissionInterval {
            start_epoch_s,
            duration_s: 3_600,
            demanded_energy_kwh: demanded,
            delivered_energy_kwh: delivered,
            unserved_energy_kwh: unserved,
            source_energy_kwh: source,
            storage_discharge_energy_kwh: discharge,
            storage_charge_energy_kwh: charge,
            auxiliary_and_loss_energy_kwh: losses,
            stored_energy_start_kwh: stored_start,
            stored_energy_end_kwh: stored_end,
            fuel_liters: fuel,
        }
    }

    fn valid_input() -> MissionServiceInput {
        let digest = "a".repeat(64);
        MissionServiceInput {
            mission_id: "PF-FIELD-SYNTHETIC-SHIFT-001".to_string(),
            evaluated_at_epoch_s: 8_300,
            qualification_valid_until_epoch_s: 9_000,
            mission_start_epoch_s: 1_000,
            mission_end_epoch_s: 8_200,
            qualified_configuration_sha256: digest.clone(),
            observed_configuration_sha256: digest.clone(),
            storage_usable_capacity_kwh: 10.0,
            intervals: vec![
                // 4.0 source + 1.0 storage = 4.5 delivered + 0.5 loss.
                interval(1_000, 8.0, 7.2, 5.0, 4.5, 0.5, 4.0, 1.0, 0.0, 0.5, 0.35),
                // 5.2 source = 4.0 delivered + 0.5 charge + 0.7 loss.
                interval(4_600, 7.2, 8.0, 4.0, 4.0, 0.0, 5.7, 0.0, 1.0, 0.7, 0.40),
            ],
            disturbances: vec![
                DisturbanceObservation {
                    event_id: "source_step_001".to_string(),
                    event_time_epoch_s: 1_200,
                    recovery_time_s: Some(3.0),
                    peak_voltage_deviation_percent: Some(7.0),
                    peak_frequency_deviation_hz: Some(0.6),
                    quality_and_recovery_evidence_reference: "synthetic-quality-event-001".to_string(),
                },
                DisturbanceObservation {
                    event_id: "motor_start_001".to_string(),
                    event_time_epoch_s: 6_000,
                    recovery_time_s: Some(2.0),
                    peak_voltage_deviation_percent: Some(6.0),
                    peak_frequency_deviation_hz: Some(0.5),
                    quality_and_recovery_evidence_reference: "synthetic-quality-event-002".to_string(),
                },
            ],
            requirements: MissionServiceRequirements {
                minimum_demanded_energy_kwh: 8.0,
                max_unserved_energy_kwh: 0.6,
                max_unserved_fraction: 0.07,
                energy_balance_tolerance_kwh: 0.02,
                service_balance_tolerance_kwh: 0.02,
                storage_state_tolerance_kwh: 0.02,
                min_terminal_storage_energy_kwh: Some(8.0),
                max_fuel_l_per_delivered_kwh: Some(0.10),
                max_recovery_time_s: Some(5.0),
                max_voltage_deviation_percent: Some(10.0),
                max_frequency_deviation_hz: Some(1.0),
                minimum_disturbance_observations: 2,
            },
            lineage: MissionMeasurementLineage {
                measurement_run_id: "synthetic-run-001".to_string(),
                raw_capture_sha256: "b".repeat(64),
                channel_manifest_sha256: "c".repeat(64),
                qualified_configuration_sha256: digest.clone(),
                observed_configuration_sha256: digest,
                timebase_reference: "synthetic-timebase-001".to_string(),
                calibration_evidence_reference: "synthetic-calibration-001".to_string(),
                uncertainty_evidence_reference: "synthetic-uncertainty-001".to_string(),
                measurement_status: EvidenceState::Current,
                calibration_status: EvidenceState::Current,
                timebase_status: EvidenceState::Current,
                channel_status: EvidenceState::Current,
                completeness_status: EvidenceState::Current,
            },
        }
    }

    fn has_blocker(
        input: &MissionServiceInput,
        expected: MissionServiceBlocker,
    ) -> bool {
        evaluate_mission_service(input)
            .unwrap()
            .hard_blockers
            .contains(&expected)
    }

    #[test]
    fn nominal_mission_reconciles_and_meets_frozen_service_requirements() {
        let report = evaluate_mission_service(&valid_input()).unwrap();
        assert!(report.evidence_eligible);
        assert_eq!(report.service_requirements_met, Some(true));
        assert_eq!(report.demanded_energy_kwh, 9.0);
        assert_eq!(report.delivered_energy_kwh, 8.5);
        assert_eq!(report.unserved_energy_kwh, 0.5);
        assert_eq!(report.source_energy_kwh, 9.2);
        assert_eq!(report.storage_discharge_energy_kwh, 1.0);
        assert_eq!(report.storage_charge_energy_kwh, 0.5);
        assert_eq!(report.terminal_stored_energy_kwh, 7.6);
        assert!((report.fuel_l_per_delivered_kwh.unwrap() - (0.75 / 8.5)).abs() < 1e-12);
    }

    #[test]
    fn report_retains_measurement_and_configuration_lineage() {
        let input = valid_input();
        let report = evaluate_mission_service(&input).unwrap();
        assert_eq!(report.lineage, input.lineage);
        assert_eq!(report.disturbances, input.disturbances);
        assert_eq!(
            report.claim_ceiling(),
            "MEASURED_MISSION_SERVICE_AUDIT_ONLY_NOT_DISPATCH_SAFETY_OR_PHYSICAL_QUALIFICATION"
        );
    }

    #[test]
    fn configuration_mismatch_withholds_evidence_eligibility() {
        let mut input = valid_input();
        input.observed_configuration_sha256 = "d".repeat(64);
        input.lineage.observed_configuration_sha256 = "d".repeat(64);
        assert!(has_blocker(
            &input,
            MissionServiceBlocker::ConfigurationMismatch
        ));
        let report = evaluate_mission_service(&input).unwrap();
        assert!(!report.evidence_eligible);
        assert_eq!(report.service_requirements_met, None);
    }

    #[test]
    fn malformed_capture_digest_withholds_evidence_eligibility() {
        let mut input = valid_input();
        input.lineage.raw_capture_sha256 = "not-a-digest".to_string();
        assert!(has_blocker(
            &input,
            MissionServiceBlocker::ConfigurationDigestMalformed
        ));
    }

    #[test]
    fn stale_calibration_withholds_evidence_eligibility() {
        let mut input = valid_input();
        input.lineage.calibration_status = EvidenceState::Expired;
        assert!(has_blocker(
            &input,
            MissionServiceBlocker::CalibrationEvidenceNotCurrent
        ));
    }

    #[test]
    fn stale_timebase_withholds_evidence_eligibility() {
        let mut input = valid_input();
        input.lineage.timebase_status = EvidenceState::Unverified;
        assert!(has_blocker(
            &input,
            MissionServiceBlocker::TimebaseEvidenceNotCurrent
        ));
    }

    #[test]
    fn incomplete_capture_withholds_evidence_eligibility() {
        let mut input = valid_input();
        input.lineage.completeness_status = EvidenceState::Missing;
        assert!(has_blocker(
            &input,
            MissionServiceBlocker::CaptureCompletenessNotCurrent
        ));
    }

    #[test]
    fn expired_qualification_withholds_evidence_eligibility() {
        let mut input = valid_input();
        input.evaluated_at_epoch_s = 9_001;
        assert!(has_blocker(
            &input,
            MissionServiceBlocker::QualificationExpired
        ));
    }

    #[test]
    fn non_contiguous_intervals_are_rejected() {
        let mut input = valid_input();
        input.intervals[1].start_epoch_s += 1;
        assert!(evaluate_mission_service(&input).is_err());
    }

    #[test]
    fn mission_must_be_covered_to_declared_end() {
        let mut input = valid_input();
        input.intervals[1].duration_s -= 1;
        assert!(evaluate_mission_service(&input).is_err());
    }

    #[test]
    fn service_ledger_residual_is_a_hard_evidence_blocker() {
        let mut input = valid_input();
        input.intervals[0].unserved_energy_kwh = 0.8;
        assert!(has_blocker(
            &input,
            MissionServiceBlocker::ServiceBalanceOutsideTolerance
        ));
    }

    #[test]
    fn bus_energy_creation_or_unaccounted_flow_withholds_eligibility() {
        let mut input = valid_input();
        input.intervals[1].source_energy_kwh = 4.0;
        assert!(has_blocker(
            &input,
            MissionServiceBlocker::EnergyBalanceOutsideTolerance
        ));
    }

    #[test]
    fn storage_state_handoff_must_be_contiguous() {
        let mut input = valid_input();
        input.intervals[1].stored_energy_start_kwh = 7.0;
        assert!(evaluate_mission_service(&input).is_err());
    }

    #[test]
    fn storage_cannot_gain_more_internal_energy_than_bus_charge() {
        let mut input = valid_input();
        input.intervals[1].stored_energy_end_kwh = 8.0;
        assert!(evaluate_mission_service(&input).is_err());
    }

    #[test]
    fn storage_cannot_lose_more_internal_energy_than_bus_discharge() {
        let mut input = valid_input();
        input.intervals[0].stored_energy_end_kwh = 6.0;
        input.intervals[1].stored_energy_start_kwh = 6.0;
        input.intervals[1].stored_energy_end_kwh = 6.4;
        assert!(evaluate_mission_service(&input).is_err());
    }

    #[test]
    fn simultaneous_storage_charge_and_discharge_requires_interval_split() {
        let mut input = valid_input();
        input.intervals[0].storage_charge_energy_kwh = 0.1;
        assert!(evaluate_mission_service(&input).is_err());
    }

    #[test]
    fn terminal_storage_obligation_is_not_hidden_by_fuel_savings() {
        let mut input = valid_input();
        input.intervals[1].stored_energy_end_kwh = 7.6;
        let report = evaluate_mission_service(&input).unwrap();
        assert!(report.evidence_eligible);
        assert_eq!(report.service_requirements_met, Some(false));
        assert_eq!(report.net_stored_energy_change_kwh, -0.4);
    }

    #[test]
    fn excessive_unserved_energy_fails_service_without_invalidating_evidence() {
        let mut input = valid_input();
        input.requirements.max_unserved_energy_kwh = 0.4;
        let report = evaluate_mission_service(&input).unwrap();
        assert!(report.evidence_eligible);
        assert_eq!(report.service_requirements_met, Some(false));
    }

    #[test]
    fn zero_service_cannot_pass_a_mission_minimum() {
        let mut input = valid_input();
        input.intervals[0].demanded_energy_kwh = 0.0;
        input.intervals[0].delivered_energy_kwh = 0.0;
        input.intervals[0].unserved_energy_kwh = 0.0;
        input.intervals[0].source_energy_kwh = 0.0;
        input.intervals[0].storage_discharge_energy_kwh = 0.0;
        input.intervals[0].auxiliary_and_loss_energy_kwh = 0.0;
        input.intervals[0].stored_energy_end_kwh = 8.0;
        input.intervals[0].fuel_liters = 0.0;
        input.intervals[1].demanded_energy_kwh = 0.0;
        input.intervals[1].delivered_energy_kwh = 0.0;
        input.intervals[1].source_energy_kwh = 0.0;
        input.intervals[1].storage_charge_energy_kwh = 0.0;
        input.intervals[1].auxiliary_and_loss_energy_kwh = 0.0;
        input.intervals[1].stored_energy_start_kwh = 8.0;
        input.intervals[1].stored_energy_end_kwh = 8.0;
        input.intervals[1].fuel_liters = 0.0;
        let report = evaluate_mission_service(&input).unwrap();
        assert!(report.evidence_eligible);
        assert_eq!(report.service_requirements_met, Some(false));
    }

    #[test]
    fn excessive_fuel_intensity_fails_service() {
        let mut input = valid_input();
        input.requirements.max_fuel_l_per_delivered_kwh = Some(0.05);
        let report = evaluate_mission_service(&input).unwrap();
        assert_eq!(report.service_requirements_met, Some(false));
    }

    #[test]
    fn missing_required_recovery_data_withholds_evidence_eligibility() {
        let mut input = valid_input();
        input.disturbances[0].recovery_time_s = None;
        assert!(has_blocker(
            &input,
            MissionServiceBlocker::RecoveryMeasurementMissing
        ));
    }

    #[test]
    fn missing_required_voltage_data_withholds_evidence_eligibility() {
        let mut input = valid_input();
        input.disturbances[0].peak_voltage_deviation_percent = None;
        assert!(has_blocker(
            &input,
            MissionServiceBlocker::VoltageMeasurementMissing
        ));
    }

    #[test]
    fn missing_required_disturbance_observations_withholds_evidence_eligibility() {
        let mut input = valid_input();
        input.disturbances.pop();
        assert!(has_blocker(
            &input,
            MissionServiceBlocker::RequiredDisturbanceObservationsMissing
        ));
    }

    #[test]
    fn excessive_recovery_time_fails_service() {
        let mut input = valid_input();
        input.disturbances[0].recovery_time_s = Some(7.0);
        let report = evaluate_mission_service(&input).unwrap();
        assert!(report.evidence_eligible);
        assert_eq!(report.service_requirements_met, Some(false));
    }

    #[test]
    fn duplicate_disturbance_ids_are_rejected() {
        let mut input = valid_input();
        input.disturbances[1].event_id = input.disturbances[0].event_id.clone();
        assert!(evaluate_mission_service(&input).is_err());
    }

    #[test]
    fn zero_storage_boundary_is_supported_for_non_storage_systems() {
        let mut input = valid_input();
        input.storage_usable_capacity_kwh = 0.0;
        input.requirements.min_terminal_storage_energy_kwh = None;
        input.intervals[0].stored_energy_start_kwh = 0.0;
        input.intervals[0].stored_energy_end_kwh = 0.0;
        input.intervals[0].storage_discharge_energy_kwh = 0.0;
        input.intervals[1].stored_energy_start_kwh = 0.0;
        input.intervals[1].stored_energy_end_kwh = 0.0;
        input.intervals[1].storage_charge_energy_kwh = 0.0;
        input.intervals[0].source_energy_kwh = 5.0;
        input.intervals[1].source_energy_kwh = 4.7;
        input.intervals[1].auxiliary_and_loss_energy_kwh = 0.7;
        let report = evaluate_mission_service(&input).unwrap();
        assert!(report.evidence_eligible);
    }

    #[test]
    fn invalid_numeric_measurements_are_rejected() {
        let mut input = valid_input();
        input.intervals[0].demanded_energy_kwh = f64::NAN;
        assert!(evaluate_mission_service(&input).is_err());
    }
}
