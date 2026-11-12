use crate::capability_envelope::{EvidenceStatus, ProtectionStatus};
use crate::AuditError;

const TIME_TOLERANCE_S: f64 = 1.0e-6;
const ENERGY_TOLERANCE_WH: f64 = 1.0e-9;

/// One contiguous mission interval at a single useful-service boundary.
///
/// The source power is an interval-average capability supplied by the owner
/// model (for example Symtropy). Any source ramp, start delay and time-varying
/// derating must already be reflected in that interval average. This evaluator
/// does not simulate electrical transients or generator/inverter dynamics.
#[derive(Debug, Clone, PartialEq)]
pub struct MissionEnergySegment {
    pub segment_id: String,
    pub start_s: f64,
    pub duration_s: f64,
    pub required_service_power_w: f64,
    pub qualified_source_average_power_w: f64,
    /// Conservative absolute power uncertainty bounds, subtracted from source
    /// capability before converting it to available interval energy.
    pub source_measurement_uncertainty_bound_w: f64,
    pub source_model_uncertainty_bound_w: f64,
    pub source_configuration_sha256: String,
    pub source_evidence_status: EvidenceStatus,
    pub source_evidence_reference: String,
}

/// A single aggregate storage bank at the same energy boundary as the mission.
///
/// Capacity, power and efficiencies are conservative qualified lower bounds;
/// capacity_uncertainty_bound_wh and initial_soc_uncertainty_fraction are
/// additional absolute uncertainty bounds. This is not a cell-level BMS model.
#[derive(Debug, Clone, PartialEq)]
pub struct StorageEnergyEnvelope {
    pub storage_id: String,
    pub qualified_configuration_sha256: String,
    pub observed_configuration_sha256: String,
    pub qualification_valid_until_epoch_s: u64,
    pub capability_status: EvidenceStatus,
    pub calibration_status: EvidenceStatus,
    pub sensor_status: EvidenceStatus,
    pub protection_status: ProtectionStatus,
    pub capacity_wh: f64,
    pub capacity_uncertainty_bound_wh: f64,
    pub initial_soc_fraction: f64,
    pub initial_soc_uncertainty_fraction: f64,
    pub minimum_soc_fraction: f64,
    pub maximum_soc_fraction: f64,
    pub end_of_mission_minimum_soc_fraction: f64,
    pub maximum_charge_power_w: f64,
    pub maximum_discharge_power_w: f64,
    /// Qualified lower-bound efficiencies at the declared common energy boundary.
    pub charge_efficiency: f64,
    pub discharge_efficiency: f64,
    pub evidence_reference: String,
}

/// Mission-level inputs. The top-level digest binds the assembled source,
/// converter, protection, storage and control configuration for this run.
#[derive(Debug, Clone, PartialEq)]
pub struct MissionEnergyInput {
    pub mission_id: String,
    pub qualified_configuration_sha256: String,
    pub observed_configuration_sha256: String,
    pub evaluation_time_epoch_s: u64,
    pub qualification_valid_until_epoch_s: u64,
    pub maximum_unserved_energy_wh: f64,
    pub segments: Vec<MissionEnergySegment>,
    pub storage: Option<StorageEnergyEnvelope>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MissionSegmentEnergyResult {
    pub segment_id: String,
    pub start_s: f64,
    pub duration_s: f64,
    pub required_energy_wh: f64,
    pub source_evidence_eligible: bool,
    pub qualified_source_average_power_w: f64,
    pub source_measurement_uncertainty_bound_w: f64,
    pub source_model_uncertainty_bound_w: f64,
    pub conservative_source_average_power_w: f64,
    pub source_evidence_status: EvidenceStatus,
    pub source_energy_available_wh: f64,
    pub source_energy_to_load_wh: f64,
    pub storage_energy_to_load_wh: f64,
    /// Energy accepted at the common bus/service boundary for charging.
    pub storage_charge_input_wh: f64,
    /// Battery-side energy added after charge efficiency.
    pub storage_energy_added_wh: f64,
    /// Source energy that could not serve load or charge storage.
    pub curtailed_source_energy_wh: f64,
    pub unserved_energy_wh: f64,
    pub storage_soc_start_fraction: Option<f64>,
    pub storage_soc_end_fraction: Option<f64>,
    pub source_configuration_sha256: String,
    pub source_evidence_reference: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MissionEnergyReport {
    pub mission_id: String,
    pub qualified_configuration_sha256: String,
    pub observed_configuration_sha256: String,
    pub global_qualification_eligible: bool,
    pub qualification_blockers: Vec<String>,
    pub excluded_source_segment_ids: Vec<String>,
    pub storage_exclusion_reason: Option<String>,
    /// Exact storage input snapshot, including evidence state, digests and limits.
    pub storage_constraint_snapshot: Option<StorageEnergyEnvelope>,
    pub storage_eligible: bool,
    pub total_required_energy_wh: f64,
    pub total_source_energy_available_wh: f64,
    pub total_source_energy_to_load_wh: f64,
    pub total_storage_energy_to_load_wh: f64,
    pub total_storage_charge_input_wh: f64,
    pub total_storage_energy_added_wh: f64,
    pub total_storage_energy_withdrawn_wh: f64,
    pub total_curtailed_source_energy_wh: f64,
    pub total_unserved_energy_wh: f64,
    pub initial_storage_soc_fraction: Option<f64>,
    pub final_storage_soc_fraction: Option<f64>,
    pub end_reserve_pass: Option<bool>,
    pub service_pass: bool,
    pub mission_pass: bool,
    pub segment_results: Vec<MissionSegmentEnergyResult>,
}

impl MissionEnergyReport {
    /// This is a deterministic interval-energy accounting screen only.
    pub const fn claim_ceiling(&self) -> &'static str {
        "MISSION_ENERGY_ACCOUNTING_SCREEN_ONLY_NOT_DISPATCH_CONTROL_OR_PHYSICAL_QUALIFICATION"
    }
}

fn nonblank(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn finite_nonnegative(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

fn validate_storage(storage: &StorageEnergyEnvelope) -> Result<(), AuditError> {
    if !nonblank(&storage.storage_id) || !nonblank(&storage.evidence_reference) {
        return Err(AuditError(
            "storage identity and evidence reference must be non-empty".to_string(),
        ));
    }
    if !valid_sha256(&storage.qualified_configuration_sha256)
        || !valid_sha256(&storage.observed_configuration_sha256)
    {
        return Err(AuditError(
            "storage configuration digests must be 64 hexadecimal SHA-256 characters"
                .to_string(),
        ));
    }
    if !finite_nonnegative(storage.capacity_wh) || storage.capacity_wh <= 0.0 {
        return Err(AuditError(
            "storage capacity must be positive and finite".to_string(),
        ));
    }
    if !finite_nonnegative(storage.capacity_uncertainty_bound_wh)
        || storage.capacity_uncertainty_bound_wh >= storage.capacity_wh
    {
        return Err(AuditError(
            "storage capacity uncertainty must be finite, non-negative and below capacity"
                .to_string(),
        ));
    }
    if !storage.initial_soc_fraction.is_finite()
        || !storage.initial_soc_uncertainty_fraction.is_finite()
        || !storage.minimum_soc_fraction.is_finite()
        || !storage.maximum_soc_fraction.is_finite()
        || !storage.end_of_mission_minimum_soc_fraction.is_finite()
    {
        return Err(AuditError("storage SOC values must be finite".to_string()));
    }
    if storage.minimum_soc_fraction < 0.0
        || storage.maximum_soc_fraction > 1.0
        || storage.minimum_soc_fraction >= storage.maximum_soc_fraction
        || storage.initial_soc_fraction < storage.minimum_soc_fraction
        || storage.initial_soc_fraction > storage.maximum_soc_fraction
        || storage.end_of_mission_minimum_soc_fraction < storage.minimum_soc_fraction
        || storage.end_of_mission_minimum_soc_fraction > storage.maximum_soc_fraction
        || storage.initial_soc_uncertainty_fraction < 0.0
        || storage.initial_soc_uncertainty_fraction > 1.0
    {
        return Err(AuditError(
            "storage SOC bounds or initial SOC are inconsistent".to_string(),
        ));
    }
    if !finite_nonnegative(storage.maximum_charge_power_w)
        || !finite_nonnegative(storage.maximum_discharge_power_w)
    {
        return Err(AuditError(
            "storage charge and discharge power limits must be finite and non-negative"
                .to_string(),
        ));
    }
    if !storage.charge_efficiency.is_finite()
        || storage.charge_efficiency <= 0.0
        || storage.charge_efficiency > 1.0
        || !storage.discharge_efficiency.is_finite()
        || storage.discharge_efficiency <= 0.0
        || storage.discharge_efficiency > 1.0
    {
        return Err(AuditError(
            "storage efficiencies must be in the interval (0, 1]".to_string(),
        ));
    }
    Ok(())
}

fn validate_input(input: &MissionEnergyInput) -> Result<(), AuditError> {
    if !nonblank(&input.mission_id) {
        return Err(AuditError("mission_id must be non-empty".to_string()));
    }
    if !valid_sha256(&input.qualified_configuration_sha256)
        || !valid_sha256(&input.observed_configuration_sha256)
    {
        return Err(AuditError(
            "system configuration digests must be 64 hexadecimal SHA-256 characters"
                .to_string(),
        ));
    }
    if !finite_nonnegative(input.maximum_unserved_energy_wh) {
        return Err(AuditError(
            "maximum_unserved_energy_wh must be finite and non-negative".to_string(),
        ));
    }
    if input.segments.is_empty() {
        return Err(AuditError(
            "mission must contain at least one segment".to_string(),
        ));
    }

    let mut expected_start_s = 0.0_f64;
    let mut seen_ids: Vec<&str> = Vec::with_capacity(input.segments.len());
    for segment in &input.segments {
        if !nonblank(&segment.segment_id) || !nonblank(&segment.source_evidence_reference) {
            return Err(AuditError(
                "segment identity and source evidence reference must be non-empty".to_string(),
            ));
        }
        if seen_ids.iter().any(|id| *id == segment.segment_id.as_str()) {
            return Err(AuditError(format!(
                "duplicate mission segment id {:?}",
                segment.segment_id
            )));
        }
        seen_ids.push(&segment.segment_id);
        if !valid_sha256(&segment.source_configuration_sha256) {
            return Err(AuditError(format!(
                "segment {:?} source configuration digest must be SHA-256",
                segment.segment_id
            )));
        }
        if !segment.start_s.is_finite()
            || !segment.duration_s.is_finite()
            || segment.start_s < 0.0
            || segment.duration_s <= 0.0
            || !segment.required_service_power_w.is_finite()
            || segment.required_service_power_w < 0.0
            || !finite_nonnegative(segment.qualified_source_average_power_w)
            || !finite_nonnegative(segment.source_measurement_uncertainty_bound_w)
            || !finite_nonnegative(segment.source_model_uncertainty_bound_w)
        {
            return Err(AuditError(format!(
                "segment {:?} contains invalid time, power or uncertainty values",
                segment.segment_id
            )));
        }
        if (segment.start_s - expected_start_s).abs() > TIME_TOLERANCE_S {
            return Err(AuditError(format!(
                "segment {:?} is not contiguous: expected start {:.9}s, found {:.9}s",
                segment.segment_id, expected_start_s, segment.start_s
            )));
        }
        expected_start_s = segment.start_s + segment.duration_s;
        if !expected_start_s.is_finite() {
            return Err(AuditError("mission timeline overflowed".to_string()));
        }
        let hours = segment.duration_s / 3600.0;
        let uncertainty_sum = segment.source_measurement_uncertainty_bound_w
            + segment.source_model_uncertainty_bound_w;
        let required_wh = segment.required_service_power_w * hours;
        let source_wh = segment.qualified_source_average_power_w * hours;
        if !hours.is_finite()
            || !uncertainty_sum.is_finite()
            || !required_wh.is_finite()
            || !source_wh.is_finite()
        {
            return Err(AuditError(format!(
                "segment {:?} energy conversion overflowed",
                segment.segment_id
            )));
        }
    }

    if let Some(storage) = &input.storage {
        validate_storage(storage)?;
    }
    Ok(())
}

/// Evaluate a piecewise-constant mission against qualified interval-average source
/// capability and one aggregate storage energy envelope.
///
/// Direct source-to-load energy is prioritised. Surplus source energy charges the
/// store subject to its charge-power and SOC ceilings. Source deficits are supplied
/// from storage subject to discharge-power and conservative usable-energy bounds.
/// No optimiser or controller is implied by this deterministic accounting order.
pub fn evaluate_mission_energy(
    input: &MissionEnergyInput,
) -> Result<MissionEnergyReport, AuditError> {
    validate_input(input)?;

    let mut qualification_blockers = Vec::new();
    if input.qualified_configuration_sha256 != input.observed_configuration_sha256 {
        qualification_blockers.push("system_configuration_digest_mismatch".to_string());
    }
    if input.qualification_valid_until_epoch_s <= input.evaluation_time_epoch_s {
        qualification_blockers.push("system_qualification_expired".to_string());
    }
    let global_qualification_eligible = qualification_blockers.is_empty();

    let storage_exclusion_reason = input.storage.as_ref().and_then(|storage| {
        if !global_qualification_eligible {
            Some("system_qualification_ineligible".to_string())
        } else if storage.qualified_configuration_sha256 != storage.observed_configuration_sha256 {
            Some("storage_configuration_digest_mismatch".to_string())
        } else if storage.qualification_valid_until_epoch_s <= input.evaluation_time_epoch_s {
            Some("storage_qualification_expired".to_string())
        } else if storage.capability_status != EvidenceStatus::Current {
            Some("storage_capability_evidence_not_current".to_string())
        } else if storage.calibration_status != EvidenceStatus::Current {
            Some("storage_calibration_evidence_not_current".to_string())
        } else if storage.sensor_status != EvidenceStatus::Current {
            Some("storage_sensor_evidence_not_current".to_string())
        } else if storage.protection_status != ProtectionStatus::Ready {
            Some("storage_protection_not_ready".to_string())
        } else {
            None
        }
    });

    let storage_eligible = input.storage.is_some() && storage_exclusion_reason.is_none();
    let effective_capacity_wh = if storage_eligible {
        input.storage.as_ref().map(|storage| {
            storage.capacity_wh - storage.capacity_uncertainty_bound_wh
        })
    } else {
        None
    };
    let mut stored_energy_wh = if storage_eligible {
        let storage = input.storage.as_ref().expect("eligible storage exists");
        let capacity = effective_capacity_wh.expect("eligible capacity exists");
        Some(
            ((storage.initial_soc_fraction - storage.initial_soc_uncertainty_fraction)
                .max(0.0)
                * capacity)
                .min(storage.maximum_soc_fraction * capacity),
        )
    } else {
        None
    };
    let initial_storage_soc_fraction = stored_energy_wh
        .zip(effective_capacity_wh)
        .map(|(energy, capacity)| energy / capacity);

    let mut total_required_energy_wh = 0.0_f64;
    let mut total_source_energy_available_wh = 0.0_f64;
    let mut total_source_energy_to_load_wh = 0.0_f64;
    let mut total_storage_energy_to_load_wh = 0.0_f64;
    let mut total_storage_charge_input_wh = 0.0_f64;
    let mut total_storage_energy_added_wh = 0.0_f64;
    let mut total_storage_energy_withdrawn_wh = 0.0_f64;
    let mut total_curtailed_source_energy_wh = 0.0_f64;
    let mut total_unserved_energy_wh = 0.0_f64;
    let mut excluded_source_segment_ids = Vec::new();
    let mut segment_results = Vec::with_capacity(input.segments.len());

    for segment in &input.segments {
        let source_evidence_eligible = global_qualification_eligible
            && segment.source_evidence_status == EvidenceStatus::Current;
        if !source_evidence_eligible && segment.qualified_source_average_power_w > 0.0 {
            excluded_source_segment_ids.push(segment.segment_id.clone());
        }

        let conservative_source_average_power_w = if source_evidence_eligible {
            (segment.qualified_source_average_power_w
                - segment.source_measurement_uncertainty_bound_w
                - segment.source_model_uncertainty_bound_w)
                .max(0.0)
        } else {
            0.0
        };
        let hours = segment.duration_s / 3600.0;
        let required_energy_wh = segment.required_service_power_w * hours;
        let source_energy_available_wh = conservative_source_average_power_w * hours;
        let source_energy_to_load_wh = required_energy_wh.min(source_energy_available_wh);
        let source_surplus_wh = (source_energy_available_wh - source_energy_to_load_wh).max(0.0);
        let source_deficit_wh = (required_energy_wh - source_energy_to_load_wh).max(0.0);

        let storage_soc_start_fraction = stored_energy_wh
            .zip(effective_capacity_wh)
            .map(|(energy, capacity)| energy / capacity);
        let mut storage_energy_to_load_wh = 0.0_f64;
        let mut storage_charge_input_wh = 0.0_f64;
        let mut storage_energy_added_wh = 0.0_f64;
        let mut storage_energy_withdrawn_wh = 0.0_f64;
        let mut curtailed_source_energy_wh = 0.0_f64;

        if let (Some(storage), Some(capacity), Some(stored)) = (
            input.storage.as_ref().filter(|_| storage_eligible),
            effective_capacity_wh,
            stored_energy_wh,
        ) {
            if source_deficit_wh > ENERGY_TOLERANCE_WH {
                let power_limited_output_wh =
                    storage.maximum_discharge_power_w * hours;
                let energy_limited_output_wh = (stored
                    - storage.minimum_soc_fraction * capacity)
                    .max(0.0)
                    * storage.discharge_efficiency;
                storage_energy_to_load_wh = source_deficit_wh
                    .min(power_limited_output_wh)
                    .min(energy_limited_output_wh);
                storage_energy_withdrawn_wh =
                    storage_energy_to_load_wh / storage.discharge_efficiency;
                stored_energy_wh = Some((stored - storage_energy_withdrawn_wh).max(0.0));
            } else if source_surplus_wh > ENERGY_TOLERANCE_WH {
                let power_limited_input_wh = storage.maximum_charge_power_w * hours;
                let room_limited_input_wh = (storage.maximum_soc_fraction * capacity
                    - stored)
                    .max(0.0)
                    / storage.charge_efficiency;
                storage_charge_input_wh = source_surplus_wh
                    .min(power_limited_input_wh)
                    .min(room_limited_input_wh);
                storage_energy_added_wh =
                    storage_charge_input_wh * storage.charge_efficiency;
                stored_energy_wh = Some(stored + storage_energy_added_wh);
                curtailed_source_energy_wh =
                    (source_surplus_wh - storage_charge_input_wh).max(0.0);
            } else {
                curtailed_source_energy_wh = source_surplus_wh;
            }
        } else {
            curtailed_source_energy_wh = source_surplus_wh;
        }

        let unserved_energy_wh =
            (source_deficit_wh - storage_energy_to_load_wh).max(0.0);
        let storage_soc_end_fraction = stored_energy_wh
            .zip(effective_capacity_wh)
            .map(|(energy, capacity)| energy / capacity);

        let calculated = [
            required_energy_wh,
            source_energy_available_wh,
            source_energy_to_load_wh,
            storage_energy_to_load_wh,
            storage_charge_input_wh,
            storage_energy_added_wh,
            storage_energy_withdrawn_wh,
            curtailed_source_energy_wh,
            unserved_energy_wh,
        ];
        if calculated.iter().any(|value| !value.is_finite()) {
            return Err(AuditError(format!(
                "segment {:?} energy accounting overflowed",
                segment.segment_id
            )));
        }

        total_required_energy_wh += required_energy_wh;
        total_source_energy_available_wh += source_energy_available_wh;
        total_source_energy_to_load_wh += source_energy_to_load_wh;
        total_storage_energy_to_load_wh += storage_energy_to_load_wh;
        total_storage_charge_input_wh += storage_charge_input_wh;
        total_storage_energy_added_wh += storage_energy_added_wh;
        total_storage_energy_withdrawn_wh += storage_energy_withdrawn_wh;
        total_curtailed_source_energy_wh += curtailed_source_energy_wh;
        total_unserved_energy_wh += unserved_energy_wh;

        segment_results.push(MissionSegmentEnergyResult {
            segment_id: segment.segment_id.clone(),
            start_s: segment.start_s,
            duration_s: segment.duration_s,
            required_energy_wh,
            source_evidence_eligible,
            qualified_source_average_power_w: segment.qualified_source_average_power_w,
            source_measurement_uncertainty_bound_w: segment.source_measurement_uncertainty_bound_w,
            source_model_uncertainty_bound_w: segment.source_model_uncertainty_bound_w,
            conservative_source_average_power_w,
            source_evidence_status: segment.source_evidence_status,
            source_energy_available_wh,
            source_energy_to_load_wh,
            storage_energy_to_load_wh,
            storage_charge_input_wh,
            storage_energy_added_wh,
            curtailed_source_energy_wh,
            unserved_energy_wh,
            storage_soc_start_fraction,
            storage_soc_end_fraction,
            source_configuration_sha256: segment.source_configuration_sha256.clone(),
            source_evidence_reference: segment.source_evidence_reference.clone(),
        });
    }

    let totals = [
        total_required_energy_wh,
        total_source_energy_available_wh,
        total_source_energy_to_load_wh,
        total_storage_energy_to_load_wh,
        total_storage_charge_input_wh,
        total_storage_energy_added_wh,
        total_storage_energy_withdrawn_wh,
        total_curtailed_source_energy_wh,
        total_unserved_energy_wh,
    ];
    if totals.iter().any(|value| !value.is_finite()) {
        return Err(AuditError("mission energy totals overflowed".to_string()));
    }

    let final_storage_soc_fraction = stored_energy_wh
        .zip(effective_capacity_wh)
        .map(|(energy, capacity)| energy / capacity);
    let end_reserve_pass = match (input.storage.as_ref(), stored_energy_wh, effective_capacity_wh) {
        (Some(storage), Some(energy), Some(capacity)) if storage_eligible => Some(
            energy + ENERGY_TOLERANCE_WH
                >= storage.end_of_mission_minimum_soc_fraction * capacity,
        ),
        _ => None,
    };
    let service_pass =
        total_unserved_energy_wh <= input.maximum_unserved_energy_wh + ENERGY_TOLERANCE_WH;
    let mission_pass = global_qualification_eligible
        && service_pass
        && end_reserve_pass.unwrap_or(true);

    Ok(MissionEnergyReport {
        mission_id: input.mission_id.clone(),
        qualified_configuration_sha256: input.qualified_configuration_sha256.clone(),
        observed_configuration_sha256: input.observed_configuration_sha256.clone(),
        global_qualification_eligible,
        qualification_blockers,
        excluded_source_segment_ids,
        storage_exclusion_reason,
        storage_constraint_snapshot: input.storage.clone(),
        storage_eligible,
        total_required_energy_wh,
        total_source_energy_available_wh,
        total_source_energy_to_load_wh,
        total_storage_energy_to_load_wh,
        total_storage_charge_input_wh,
        total_storage_energy_added_wh,
        total_storage_energy_withdrawn_wh,
        total_curtailed_source_energy_wh,
        total_unserved_energy_wh,
        initial_storage_soc_fraction,
        final_storage_soc_fraction,
        end_reserve_pass,
        service_pass,
        mission_pass,
        segment_results,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        evaluate_mission_energy, MissionEnergyInput, MissionEnergySegment,
        StorageEnergyEnvelope,
    };
    use crate::capability_envelope::{EvidenceStatus, ProtectionStatus};

    fn digest(byte: char) -> String {
        std::iter::repeat(byte).take(64).collect()
    }

    fn segment(
        id: &str,
        start_s: f64,
        duration_s: f64,
        demand_w: f64,
        source_w: f64,
    ) -> MissionEnergySegment {
        MissionEnergySegment {
            segment_id: id.to_string(),
            start_s,
            duration_s,
            required_service_power_w: demand_w,
            qualified_source_average_power_w: source_w,
            source_measurement_uncertainty_bound_w: 0.0,
            source_model_uncertainty_bound_w: 0.0,
            source_configuration_sha256: digest('b'),
            source_evidence_status: EvidenceStatus::Current,
            source_evidence_reference: format!("symtropy:source-run-{id}"),
        }
    }

    fn storage() -> StorageEnergyEnvelope {
        StorageEnergyEnvelope {
            storage_id: "aggregate-battery".to_string(),
            qualified_configuration_sha256: digest('a'),
            observed_configuration_sha256: digest('a'),
            qualification_valid_until_epoch_s: 100,
            capability_status: EvidenceStatus::Current,
            calibration_status: EvidenceStatus::Current,
            sensor_status: EvidenceStatus::Current,
            protection_status: ProtectionStatus::Ready,
            capacity_wh: 5_000.0,
            capacity_uncertainty_bound_wh: 0.0,
            initial_soc_fraction: 0.5,
            initial_soc_uncertainty_fraction: 0.0,
            minimum_soc_fraction: 0.1,
            maximum_soc_fraction: 0.9,
            end_of_mission_minimum_soc_fraction: 0.2,
            maximum_charge_power_w: 2_000.0,
            maximum_discharge_power_w: 1_000.0,
            charge_efficiency: 0.9,
            discharge_efficiency: 0.9,
            evidence_reference: "mycelix:storage-passport-001".to_string(),
        }
    }

    fn input(segments: Vec<MissionEnergySegment>) -> MissionEnergyInput {
        MissionEnergyInput {
            mission_id: "synthetic-mission-001".to_string(),
            qualified_configuration_sha256: digest('a'),
            observed_configuration_sha256: digest('a'),
            evaluation_time_epoch_s: 10,
            qualification_valid_until_epoch_s: 100,
            maximum_unserved_energy_wh: 0.0,
            segments,
            storage: None,
        }
    }

    #[test]
    fn generator_only_delivers_the_same_segment_service() {
        let report = evaluate_mission_energy(&input(vec![segment("shift", 0.0, 3600.0, 1_000.0, 1_000.0)]))
            .unwrap();
        assert!(report.mission_pass);
        assert_eq!(report.total_required_energy_wh, 1_000.0);
        assert_eq!(report.total_source_energy_to_load_wh, 1_000.0);
        assert_eq!(report.total_unserved_energy_wh, 0.0);
        assert_eq!(report.final_storage_soc_fraction, None);
    }

    #[test]
    fn storage_only_service_obeys_efficiency_and_energy_bounds() {
        let mut value = input(vec![segment("battery-run", 0.0, 7200.0, 500.0, 0.0)]);
        let mut bank = storage();
        bank.initial_soc_fraction = 0.5;
        value.storage = Some(bank);
        let report = evaluate_mission_energy(&value).unwrap();
        assert!(report.service_pass);
        assert!(report.end_reserve_pass.unwrap());
        assert!((report.total_storage_energy_to_load_wh - 1_000.0).abs() < 1e-9);
        assert!((report.total_storage_energy_withdrawn_wh - 1_111.111111111111).abs() < 1e-8);
        assert!((report.final_storage_soc_fraction.unwrap() - 0.2777777777777778).abs() < 1e-9);
    }

    #[test]
    fn hybrid_mission_charges_surplus_then_serves_later_deficit() {
        let mut value = input(vec![
            segment("charge-window", 0.0, 3600.0, 1_000.0, 2_000.0),
            segment("discharge-window", 3600.0, 3600.0, 800.0, 0.0),
        ]);
        let mut bank = storage();
        bank.initial_soc_fraction = 0.2;
        value.storage = Some(bank);
        let report = evaluate_mission_energy(&value).unwrap();
        assert!(report.mission_pass);
        assert_eq!(report.total_required_energy_wh, 1_800.0);
        assert_eq!(report.total_source_energy_available_wh, 2_000.0);
        assert_eq!(report.total_source_energy_to_load_wh, 1_000.0);
        assert_eq!(report.total_storage_charge_input_wh, 1_000.0);
        assert_eq!(report.total_storage_energy_added_wh, 900.0);
        assert_eq!(report.total_storage_energy_to_load_wh, 800.0);
        assert!((report.total_storage_energy_withdrawn_wh - 888.8888888888889).abs() < 1e-8);
        assert_eq!(report.total_curtailed_source_energy_wh, 0.0);
        assert_eq!(report.total_unserved_energy_wh, 0.0);
        assert!(report.final_storage_soc_fraction.unwrap() >= 0.2);
    }

    #[test]
    fn end_of_mission_reserve_prevents_borrowing_from_next_mission() {
        let mut value = input(vec![segment("reserve-test", 0.0, 3600.0, 600.0, 0.0)]);
        let mut bank = storage();
        bank.initial_soc_fraction = 0.5;
        bank.end_of_mission_minimum_soc_fraction = 0.4;
        value.storage = Some(bank);
        let report = evaluate_mission_energy(&value).unwrap();
        assert!(report.service_pass);
        assert_eq!(report.end_reserve_pass, Some(false));
        assert!(!report.mission_pass);
    }

    #[test]
    fn charge_power_limit_curtailed_energy_is_not_credited() {
        let mut value = input(vec![segment("charge-limited", 0.0, 3600.0, 0.0, 1_000.0)]);
        let mut bank = storage();
        bank.initial_soc_fraction = 0.2;
        bank.maximum_charge_power_w = 500.0;
        value.storage = Some(bank);
        let report = evaluate_mission_energy(&value).unwrap();
        assert_eq!(report.total_storage_charge_input_wh, 500.0);
        assert_eq!(report.total_curtailed_source_energy_wh, 500.0);
        assert_eq!(report.total_storage_energy_added_wh, 450.0);
    }

    #[test]
    fn discharge_power_limit_causes_unserved_energy_even_with_soc_available() {
        let mut value = input(vec![segment("discharge-limited", 0.0, 3600.0, 2_000.0, 0.0)]);
        let mut bank = storage();
        bank.maximum_discharge_power_w = 500.0;
        value.storage = Some(bank);
        let report = evaluate_mission_energy(&value).unwrap();
        assert_eq!(report.total_storage_energy_to_load_wh, 500.0);
        assert_eq!(report.total_unserved_energy_wh, 1_500.0);
        assert!(!report.service_pass);
    }

    #[test]
    fn source_uncertainty_is_subtracted_conservatively() {
        let mut item = segment("uncertain-source", 0.0, 3600.0, 1_000.0, 1_200.0);
        item.source_measurement_uncertainty_bound_w = 100.0;
        item.source_model_uncertainty_bound_w = 50.0;
        let report = evaluate_mission_energy(&input(vec![item])).unwrap();
        assert_eq!(report.segment_results[0].conservative_source_average_power_w, 1_050.0);
        assert_eq!(report.total_unserved_energy_wh, 0.0);
    }

    #[test]
    fn source_uncertainty_can_remove_nominally_available_power() {
        let mut item = segment("uncertain-source", 0.0, 3600.0, 1_000.0, 1_000.0);
        item.source_measurement_uncertainty_bound_w = 700.0;
        item.source_model_uncertainty_bound_w = 400.0;
        let report = evaluate_mission_energy(&input(vec![item])).unwrap();
        assert_eq!(report.segment_results[0].conservative_source_average_power_w, 0.0);
        assert_eq!(report.total_unserved_energy_wh, 1_000.0);
    }

    #[test]
    fn storage_capacity_and_initial_soc_uncertainty_reduce_available_energy() {
        let mut value = input(vec![segment("uncertain-storage", 0.0, 3600.0, 500.0, 0.0)]);
        let mut bank = storage();
        bank.capacity_uncertainty_bound_wh = 500.0;
        bank.initial_soc_fraction = 0.5;
        bank.initial_soc_uncertainty_fraction = 0.1;
        value.storage = Some(bank);
        let report = evaluate_mission_energy(&value).unwrap();
        assert_eq!(report.initial_storage_soc_fraction, Some(0.4));
        assert_eq!(report.total_storage_energy_to_load_wh, 500.0);
        assert!((report.final_storage_soc_fraction.unwrap() - 0.2765432098765432).abs() < 1e-9);
    }

    #[test]
    fn stale_source_evidence_excludes_that_source_segment() {
        let mut item = segment("stale-source", 0.0, 3600.0, 500.0, 1_000.0);
        item.source_evidence_status = EvidenceStatus::Unverified;
        let report = evaluate_mission_energy(&input(vec![item])).unwrap();
        assert_eq!(report.total_source_energy_available_wh, 0.0);
        assert_eq!(report.total_unserved_energy_wh, 500.0);
        assert_eq!(report.excluded_source_segment_ids, vec!["stale-source".to_string()]);
    }

    #[test]
    fn current_storage_can_cover_an_excluded_source_segment() {
        let mut item = segment("stale-source", 0.0, 3600.0, 500.0, 1_000.0);
        item.source_evidence_status = EvidenceStatus::Expired;
        let mut value = input(vec![item]);
        let mut bank = storage();
        bank.initial_soc_fraction = 0.5;
        value.storage = Some(bank);
        let report = evaluate_mission_energy(&value).unwrap();
        assert!(report.mission_pass);
        assert_eq!(report.total_source_energy_available_wh, 0.0);
        assert_eq!(report.total_storage_energy_to_load_wh, 500.0);
    }

    #[test]
    fn non_current_storage_evidence_excludes_the_bank() {
        let mut value = input(vec![segment("generator", 0.0, 3600.0, 1_000.0, 1_000.0)]);
        let mut bank = storage();
        bank.sensor_status = EvidenceStatus::Expired;
        value.storage = Some(bank);
        let report = evaluate_mission_energy(&value).unwrap();
        assert_eq!(
            report.storage_exclusion_reason,
            Some("storage_sensor_evidence_not_current".to_string())
        );
        assert_eq!(report.final_storage_soc_fraction, None);
        assert!(report.mission_pass);
    }

    #[test]
    fn system_configuration_mismatch_withdraws_all_capability() {
        let mut value = input(vec![segment("generator", 0.0, 3600.0, 500.0, 1_000.0)]);
        value.observed_configuration_sha256 = digest('c');
        value.storage = Some(storage());
        let report = evaluate_mission_energy(&value).unwrap();
        assert!(!report.global_qualification_eligible);
        assert_eq!(report.total_source_energy_available_wh, 0.0);
        assert_eq!(report.total_unserved_energy_wh, 500.0);
        assert_eq!(report.final_storage_soc_fraction, None);
        assert!(!report.mission_pass);
    }

    #[test]
    fn expired_storage_qualification_excludes_only_the_storage_bank() {
        let mut value = input(vec![segment("generator", 0.0, 3600.0, 1_000.0, 1_000.0)]);
        let mut bank = storage();
        bank.qualification_valid_until_epoch_s = 10;
        value.storage = Some(bank);
        let report = evaluate_mission_energy(&value).unwrap();
        assert_eq!(
            report.storage_exclusion_reason,
            Some("storage_qualification_expired".to_string())
        );
        assert!(report.mission_pass);
    }

    #[test]
    fn non_contiguous_segments_are_rejected() {
        let value = input(vec![
            segment("first", 0.0, 100.0, 0.0, 0.0),
            segment("gap", 101.0, 100.0, 0.0, 0.0),
        ]);
        assert!(evaluate_mission_energy(&value).is_err());
    }

    #[test]
    fn duplicate_segment_ids_are_rejected() {
        let value = input(vec![
            segment("duplicate", 0.0, 100.0, 0.0, 0.0),
            segment("duplicate", 100.0, 100.0, 0.0, 0.0),
        ]);
        assert!(evaluate_mission_energy(&value).is_err());
    }

    #[test]
    fn non_sha256_digest_is_rejected() {
        let mut value = input(vec![segment("bad-digest", 0.0, 100.0, 0.0, 0.0)]);
        value.qualified_configuration_sha256 = "not-a-digest".to_string();
        assert!(evaluate_mission_energy(&value).is_err());
    }

    #[test]
    fn non_finite_and_negative_uncertainty_are_rejected() {
        let mut item = segment("bad-uncertainty", 0.0, 100.0, 0.0, 0.0);
        item.source_model_uncertainty_bound_w = f64::NAN;
        assert!(evaluate_mission_energy(&input(vec![item])).is_err());
        let mut item = segment("negative-uncertainty", 0.0, 100.0, 0.0, 0.0);
        item.source_model_uncertainty_bound_w = -1.0;
        assert!(evaluate_mission_energy(&input(vec![item])).is_err());
    }

    #[test]
    fn invalid_storage_efficiency_is_rejected() {
        let mut value = input(vec![segment("storage", 0.0, 100.0, 0.0, 0.0)]);
        let mut bank = storage();
        bank.charge_efficiency = 1.01;
        value.storage = Some(bank);
        assert!(evaluate_mission_energy(&value).is_err());
    }

    #[test]
    fn system_qualification_expiry_is_a_global_blocker() {
        let mut value = input(vec![segment("expired", 0.0, 3600.0, 100.0, 1_000.0)]);
        value.qualification_valid_until_epoch_s = value.evaluation_time_epoch_s;
        let report = evaluate_mission_energy(&value).unwrap();
        assert!(!report.global_qualification_eligible);
        assert_eq!(report.qualification_blockers, vec!["system_qualification_expired".to_string()]);
        assert_eq!(report.total_source_energy_available_wh, 0.0);
        assert!(!report.mission_pass);
    }

    #[test]
    fn claim_ceiling_is_not_physical_qualification() {
        let report = evaluate_mission_energy(&input(vec![segment("claim", 0.0, 100.0, 0.0, 0.0)]))
            .unwrap();
        assert_eq!(
            report.claim_ceiling(),
            "MISSION_ENERGY_ACCOUNTING_SCREEN_ONLY_NOT_DISPATCH_CONTROL_OR_PHYSICAL_QUALIFICATION"
        );
    }
}
