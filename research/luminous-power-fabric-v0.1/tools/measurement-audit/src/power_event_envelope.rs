use crate::capability_envelope::{EvidenceStatus, ProtectionStatus};
use crate::AuditError;

const EVENT_TIME_TOLERANCE_S: f64 = 1.0e-6;
const EVENT_ENERGY_TOLERANCE_WH: f64 = 1.0e-9;
const MAX_POWER_EVENTS: usize = 4096;

/// Storage capability snapshot evaluated at this exact event's time, SOC,
/// SOH and temperature. Power is a qualified lower bound at the common
/// useful-service boundary; usable energy is battery-side energy remaining
/// after previously committed mission/event obligations.
#[derive(Debug, Clone, PartialEq)]
pub struct EventStorageSupport {
    pub storage_id: String,
    pub qualified_configuration_sha256: String,
    pub observed_configuration_sha256: String,
    pub qualification_valid_until_epoch_s: u64,
    pub capability_status: EvidenceStatus,
    pub calibration_status: EvidenceStatus,
    pub sensor_status: EvidenceStatus,
    pub protection_status: ProtectionStatus,
    /// Qualified lower-bound output power at the service boundary for
    /// max_supported_duration_s at the declared SOC/SOH/temperature.
    pub maximum_event_discharge_power_w: f64,
    pub max_supported_duration_s: f64,
    /// Battery-side energy available for this event after prior commitments.
    pub usable_energy_wh_at_event: f64,
    pub usable_energy_uncertainty_bound_wh: f64,
    pub discharge_efficiency: f64,
    pub state_of_charge_fraction: f64,
    pub state_of_health_fraction: f64,
    pub temperature_c: f64,
    pub operating_condition_reference: String,
    pub evidence_reference: String,
}

/// An event is a time-windowed peak-service obligation. Source power is the
/// guaranteed minimum over the entire event window, not a static nameplate
/// or instantaneous peak. Source ramp/start/transfer behaviour must already
/// be reflected in the owner-qualified event value.
#[derive(Debug, Clone, PartialEq)]
pub struct MissionPowerEvent {
    pub event_id: String,
    pub start_s: f64,
    pub duration_s: f64,
    pub required_peak_power_w: f64,
    pub maximum_unserved_energy_wh: f64,
    pub qualified_source_minimum_power_w: f64,
    pub source_measurement_uncertainty_bound_w: f64,
    pub source_model_uncertainty_bound_w: f64,
    /// Qualified lower-bound output-path power after the source/storage join:
    /// shared bus, inverter, converter, cabling and connector bottleneck.
    pub qualified_common_path_power_w: f64,
    pub common_path_uncertainty_bound_w: f64,
    pub qualified_common_path_configuration_sha256: String,
    pub observed_common_path_configuration_sha256: String,
    pub common_path_qualification_valid_until_epoch_s: u64,
    pub common_path_evidence_status: EvidenceStatus,
    pub common_path_evidence_reference: String,
    pub qualified_source_configuration_sha256: String,
    pub observed_source_configuration_sha256: String,
    pub source_qualification_valid_until_epoch_s: u64,
    pub source_evidence_status: EvidenceStatus,
    pub source_evidence_reference: String,
    pub storage_support: Option<EventStorageSupport>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PowerEventResult {
    pub event_id: String,
    pub start_s: f64,
    pub duration_s: f64,
    pub required_peak_power_w: f64,
    pub maximum_unserved_energy_wh: f64,
    pub source_eligible: bool,
    pub qualified_source_minimum_power_w: f64,
    pub source_measurement_uncertainty_bound_w: f64,
    pub source_model_uncertainty_bound_w: f64,
    pub conservative_source_power_w: f64,
    pub qualified_source_configuration_sha256: String,
    pub observed_source_configuration_sha256: String,
    pub source_qualification_valid_until_epoch_s: u64,
    pub source_evidence_status: EvidenceStatus,
    pub source_evidence_reference: String,
    pub common_path_eligible: bool,
    pub conservative_common_path_power_w: f64,
    pub qualified_common_path_power_w: f64,
    pub common_path_uncertainty_bound_w: f64,
    pub qualified_common_path_configuration_sha256: String,
    pub observed_common_path_configuration_sha256: String,
    pub common_path_qualification_valid_until_epoch_s: u64,
    pub common_path_evidence_status: EvidenceStatus,
    pub common_path_evidence_reference: String,
    pub storage_eligible: bool,
    pub storage_exclusion_reason: Option<String>,
    pub storage_constraint_snapshot: Option<EventStorageSupport>,
    pub conservative_storage_power_w: f64,
    pub conservative_storage_usable_energy_wh: f64,
    pub combined_support_power_w: f64,
    pub peak_power_shortfall_w: f64,
    /// Power shortfall integrated over this event window. This is a diagnostic
    /// for the event gate, not an additional energy term to add to the mission
    /// energy ledger a second time.
    pub event_unserved_energy_wh: f64,
    pub event_pass: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PowerEventEnvelopeInput {
    pub mission_id: String,
    pub qualified_configuration_sha256: String,
    pub observed_configuration_sha256: String,
    pub evaluation_time_epoch_s: u64,
    pub qualification_valid_until_epoch_s: u64,
    pub mission_duration_s: f64,
    pub events: Vec<MissionPowerEvent>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PowerEventEnvelopeReport {
    pub mission_id: String,
    pub qualified_configuration_sha256: String,
    pub observed_configuration_sha256: String,
    pub global_qualification_eligible: bool,
    pub qualification_blockers: Vec<String>,
    pub event_results: Vec<PowerEventResult>,
    pub failed_event_ids: Vec<String>,
    pub mission_event_pass: bool,
}

impl PowerEventEnvelopeReport {
    /// Event-window power/energy screening only; this is not waveform,
    /// power-quality, motor-start, protection or physical qualification.
    pub const fn claim_ceiling(&self) -> &'static str {
        "POWER_DURATION_EVENT_SCREEN_ONLY_NOT_WAVEFORM_OR_PROTECTION_QUALIFICATION"
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

fn validate_storage(storage: &EventStorageSupport) -> Result<(), AuditError> {
    if !nonblank(&storage.storage_id)
        || !nonblank(&storage.operating_condition_reference)
        || !nonblank(&storage.evidence_reference)
    {
        return Err(AuditError(
            "event storage identity and evidence references must be non-empty".to_string(),
        ));
    }
    if !valid_sha256(&storage.qualified_configuration_sha256)
        || !valid_sha256(&storage.observed_configuration_sha256)
    {
        return Err(AuditError(
            "event storage configuration digests must be 64 hexadecimal SHA-256 characters"
                .to_string(),
        ));
    }
    if !finite_nonnegative(storage.maximum_event_discharge_power_w)
        || !storage.max_supported_duration_s.is_finite()
        || storage.max_supported_duration_s <= 0.0
        || !finite_nonnegative(storage.usable_energy_wh_at_event)
        || !finite_nonnegative(storage.usable_energy_uncertainty_bound_wh)
        || storage.usable_energy_uncertainty_bound_wh
            > storage.usable_energy_wh_at_event
    {
        return Err(AuditError(
            "event storage power, duration or usable-energy bounds are invalid".to_string(),
        ));
    }
    if !storage.discharge_efficiency.is_finite()
        || storage.discharge_efficiency <= 0.0
        || storage.discharge_efficiency > 1.0
        || !storage.state_of_charge_fraction.is_finite()
        || !(0.0..=1.0).contains(&storage.state_of_charge_fraction)
        || !storage.state_of_health_fraction.is_finite()
        || !(0.0..=1.0).contains(&storage.state_of_health_fraction)
        || !storage.temperature_c.is_finite()
    {
        return Err(AuditError(
            "event storage efficiency, SOC, SOH or temperature is invalid".to_string(),
        ));
    }
    Ok(())
}

fn validate_input(input: &PowerEventEnvelopeInput) -> Result<(), AuditError> {
    if !nonblank(&input.mission_id) {
        return Err(AuditError("event mission_id must be non-empty".to_string()));
    }
    if !valid_sha256(&input.qualified_configuration_sha256)
        || !valid_sha256(&input.observed_configuration_sha256)
    {
        return Err(AuditError(
            "event system configuration digests must be SHA-256".to_string(),
        ));
    }
    if !input.mission_duration_s.is_finite() || input.mission_duration_s <= 0.0 {
        return Err(AuditError(
            "event mission duration must be positive and finite".to_string(),
        ));
    }
    if input.events.is_empty() {
        return Err(AuditError("at least one power event is required".to_string()));
    }
    if input.events.len() > MAX_POWER_EVENTS {
        return Err(AuditError(format!(
            "power event count exceeds v0.1 limit of {MAX_POWER_EVENTS}"
        )));
    }

    let mut ids: Vec<&str> = Vec::with_capacity(input.events.len());
    let mut ordered: Vec<&MissionPowerEvent> = input.events.iter().collect();
    ordered.sort_by(|left, right| {
        left.start_s
            .total_cmp(&right.start_s)
            .then_with(|| left.event_id.cmp(&right.event_id))
    });

    let mut previous_end_s = 0.0_f64;
    let mut first = true;
    for event in &ordered {
        if !nonblank(&event.event_id) || !nonblank(&event.source_evidence_reference) {
            return Err(AuditError(
                "power event identity and source evidence reference must be non-empty".to_string(),
            ));
        }
        if ids.iter().any(|id| *id == event.event_id.as_str()) {
            return Err(AuditError(format!(
                "duplicate power event id {:?}",
                event.event_id
            )));
        }
        ids.push(&event.event_id);
        if !valid_sha256(&event.qualified_source_configuration_sha256)
            || !valid_sha256(&event.observed_source_configuration_sha256)
        {
            return Err(AuditError(format!(
                "event {:?} source configuration digests must be SHA-256",
                event.event_id
            )));
        }
        if !event.start_s.is_finite()
            || !event.duration_s.is_finite()
            || event.start_s < 0.0
            || event.duration_s <= 0.0
            || !finite_nonnegative(event.required_peak_power_w)
            || !finite_nonnegative(event.maximum_unserved_energy_wh)
            || !finite_nonnegative(event.qualified_source_minimum_power_w)
            || !finite_nonnegative(event.source_measurement_uncertainty_bound_w)
            || !finite_nonnegative(event.source_model_uncertainty_bound_w)
            || !finite_nonnegative(event.qualified_common_path_power_w)
            || !finite_nonnegative(event.common_path_uncertainty_bound_w)
            || !(event.qualified_common_path_configuration_sha256.len() == 64
                && event.qualified_common_path_configuration_sha256.bytes().all(|b| b.is_ascii_hexdigit()))
            || !(event.observed_common_path_configuration_sha256.len() == 64
                && event.observed_common_path_configuration_sha256.bytes().all(|b| b.is_ascii_hexdigit()))
            || !nonblank(&event.common_path_evidence_reference)
        {
            return Err(AuditError(format!(
                "event {:?} contains invalid time, power or uncertainty values",
                event.event_id
            )));
        }
        let end_s = event.start_s + event.duration_s;
        let uncertainty_sum = event.source_measurement_uncertainty_bound_w
            + event.source_model_uncertainty_bound_w;
        let common_uncertainty_sum = event.common_path_uncertainty_bound_w;
        let duration_h = event.duration_s / 3600.0;
        let required_energy_wh = event.required_peak_power_w * duration_h;
        if !end_s.is_finite()
            || !uncertainty_sum.is_finite()
            || !common_uncertainty_sum.is_finite()
            || !duration_h.is_finite()
            || duration_h <= 0.0
            || !required_energy_wh.is_finite()
        {
            return Err(AuditError(format!(
                "event {:?} numeric conversion overflowed",
                event.event_id
            )));
        }
        if end_s > input.mission_duration_s + EVENT_TIME_TOLERANCE_S {
            return Err(AuditError(format!(
                "event {:?} extends beyond mission duration",
                event.event_id
            )));
        }
        if !first && event.start_s < previous_end_s - EVENT_TIME_TOLERANCE_S {
            return Err(AuditError(format!(
                "event {:?} overlaps the preceding v0.1 event window",
                event.event_id
            )));
        }
        previous_end_s = end_s;
        first = false;
        if let Some(storage) = &event.storage_support {
            validate_storage(storage)?;
        }
    }
    Ok(())
}

fn storage_exclusion_reason(
    storage: &EventStorageSupport,
    event_duration_s: f64,
    evaluation_time_epoch_s: u64,
    global_qualification_eligible: bool,
) -> Option<String> {
    if !global_qualification_eligible {
        Some("mission_configuration_or_qualification_ineligible".to_string())
    } else if storage.qualified_configuration_sha256 != storage.observed_configuration_sha256 {
        Some("storage_configuration_digest_mismatch".to_string())
    } else if storage.qualification_valid_until_epoch_s <= evaluation_time_epoch_s {
        Some("storage_qualification_expired".to_string())
    } else if storage.capability_status != EvidenceStatus::Current {
        Some("storage_capability_evidence_not_current".to_string())
    } else if storage.calibration_status != EvidenceStatus::Current {
        Some("storage_calibration_evidence_not_current".to_string())
    } else if storage.sensor_status != EvidenceStatus::Current {
        Some("storage_sensor_evidence_not_current".to_string())
    } else if storage.protection_status != ProtectionStatus::Ready {
        Some("storage_protection_not_ready".to_string())
    } else if storage.max_supported_duration_s + EVENT_TIME_TOLERANCE_S < event_duration_s {
        Some("storage_event_duration_exceeded".to_string())
    } else {
        None
    }
}

/// Evaluate non-overlapping transient power events against a source guaranteed
/// over the complete event window plus optional SOC/temperature/SOH-specific
/// storage support.
///
/// The supplied storage usable energy must already account for the mission's
/// state immediately before this event and any prior commitments. The returned
/// event-unserved energy is a diagnostic only; the event's energy remains part
/// of the aggregate mission profile exactly once.
pub fn evaluate_power_event_envelope(
    input: &PowerEventEnvelopeInput,
) -> Result<PowerEventEnvelopeReport, AuditError> {
    validate_input(input)?;

    let mut qualification_blockers = Vec::new();
    if input.qualified_configuration_sha256 != input.observed_configuration_sha256 {
        qualification_blockers.push("mission_configuration_digest_mismatch".to_string());
    }
    if input.qualification_valid_until_epoch_s <= input.evaluation_time_epoch_s {
        qualification_blockers.push("mission_qualification_expired".to_string());
    }
    let global_qualification_eligible = qualification_blockers.is_empty();

    let mut ordered: Vec<&MissionPowerEvent> = input.events.iter().collect();
    ordered.sort_by(|left, right| {
        left.start_s
            .total_cmp(&right.start_s)
            .then_with(|| left.event_id.cmp(&right.event_id))
    });

    let mut event_results = Vec::with_capacity(ordered.len());
    let mut failed_event_ids = Vec::new();

    for event in ordered {
        let source_eligible = global_qualification_eligible
            && event.qualified_source_configuration_sha256
                == event.observed_source_configuration_sha256
            && event.source_qualification_valid_until_epoch_s > input.evaluation_time_epoch_s
            && event.source_evidence_status == EvidenceStatus::Current;
        let common_path_eligible = global_qualification_eligible
            && event.qualified_common_path_configuration_sha256
                == event.observed_common_path_configuration_sha256
            && event.common_path_qualification_valid_until_epoch_s > input.evaluation_time_epoch_s
            && event.common_path_evidence_status == EvidenceStatus::Current;
        let conservative_common_path_power_w = if common_path_eligible {
            (event.qualified_common_path_power_w - event.common_path_uncertainty_bound_w)
                .max(0.0)
        } else {
            0.0
        };

        let conservative_source_power_w = if source_eligible {
            (event.qualified_source_minimum_power_w
                - event.source_measurement_uncertainty_bound_w
                - event.source_model_uncertainty_bound_w)
                .max(0.0)
        } else {
            0.0
        };

        let mut storage_eligible = false;
        let mut storage_exclusion_reason_value = None;
        let mut conservative_storage_power_w = 0.0_f64;
        let mut conservative_storage_usable_energy_wh = 0.0_f64;

        if let Some(storage) = &event.storage_support {
            storage_exclusion_reason_value = storage_exclusion_reason(
                storage,
                event.duration_s,
                input.evaluation_time_epoch_s,
                global_qualification_eligible,
            );
            storage_eligible = storage_exclusion_reason_value.is_none();
            if storage_eligible {
                conservative_storage_usable_energy_wh =
                    storage.usable_energy_wh_at_event - storage.usable_energy_uncertainty_bound_wh;
                let duration_h = event.duration_s / 3600.0;
                let energy_limited_power_w = conservative_storage_usable_energy_wh
                    * storage.discharge_efficiency
                    / duration_h;
                conservative_storage_power_w = storage
                    .maximum_event_discharge_power_w
                    .min(energy_limited_power_w);
            }
        }

        let unconstrained_support_power_w =
            conservative_source_power_w + conservative_storage_power_w;
        let combined_support_power_w = if common_path_eligible {
            unconstrained_support_power_w.min(conservative_common_path_power_w)
        } else {
            0.0
        };
        let peak_power_shortfall_w =
            (event.required_peak_power_w - combined_support_power_w).max(0.0);
        let event_unserved_energy_wh =
            peak_power_shortfall_w * event.duration_s / 3600.0;
        if !combined_support_power_w.is_finite()
            || !peak_power_shortfall_w.is_finite()
            || !event_unserved_energy_wh.is_finite()
        {
            return Err(AuditError(format!(
                "event {:?} support calculation overflowed",
                event.event_id
            )));
        }
        let event_pass = event_unserved_energy_wh
            <= event.maximum_unserved_energy_wh + EVENT_ENERGY_TOLERANCE_WH;
        if !event_pass {
            failed_event_ids.push(event.event_id.clone());
        }

        event_results.push(PowerEventResult {
            event_id: event.event_id.clone(),
            start_s: event.start_s,
            duration_s: event.duration_s,
            required_peak_power_w: event.required_peak_power_w,
            maximum_unserved_energy_wh: event.maximum_unserved_energy_wh,
            source_eligible,
            qualified_source_minimum_power_w: event.qualified_source_minimum_power_w,
            source_measurement_uncertainty_bound_w: event.source_measurement_uncertainty_bound_w,
            source_model_uncertainty_bound_w: event.source_model_uncertainty_bound_w,
            conservative_source_power_w,
            qualified_source_configuration_sha256: event
                .qualified_source_configuration_sha256
                .clone(),
            observed_source_configuration_sha256: event
                .observed_source_configuration_sha256
                .clone(),
            source_qualification_valid_until_epoch_s: event
                .source_qualification_valid_until_epoch_s,
            source_evidence_status: event.source_evidence_status,
            source_evidence_reference: event.source_evidence_reference.clone(),
            common_path_eligible,
            conservative_common_path_power_w,
            qualified_common_path_power_w: event.qualified_common_path_power_w,
            common_path_uncertainty_bound_w: event.common_path_uncertainty_bound_w,
            qualified_common_path_configuration_sha256: event
                .qualified_common_path_configuration_sha256
                .clone(),
            observed_common_path_configuration_sha256: event
                .observed_common_path_configuration_sha256
                .clone(),
            common_path_qualification_valid_until_epoch_s: event
                .common_path_qualification_valid_until_epoch_s,
            common_path_evidence_status: event.common_path_evidence_status,
            common_path_evidence_reference: event.common_path_evidence_reference.clone(),
            storage_eligible,
            storage_exclusion_reason: storage_exclusion_reason_value,
            storage_constraint_snapshot: event.storage_support.clone(),
            conservative_storage_power_w,
            conservative_storage_usable_energy_wh,
            combined_support_power_w,
            peak_power_shortfall_w,
            event_unserved_energy_wh,
            event_pass,
        });
    }

    let mission_event_pass = global_qualification_eligible && failed_event_ids.is_empty();

    Ok(PowerEventEnvelopeReport {
        mission_id: input.mission_id.clone(),
        qualified_configuration_sha256: input.qualified_configuration_sha256.clone(),
        observed_configuration_sha256: input.observed_configuration_sha256.clone(),
        global_qualification_eligible,
        qualification_blockers,
        event_results,
        failed_event_ids,
        mission_event_pass,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        evaluate_power_event_envelope, EventStorageSupport, MissionPowerEvent,
        PowerEventEnvelopeInput,
    };
    use crate::capability_envelope::{EvidenceStatus, ProtectionStatus};

    fn digest(byte: char) -> String {
        std::iter::repeat(byte).take(64).collect()
    }

    fn storage() -> EventStorageSupport {
        EventStorageSupport {
            storage_id: "bess-1".to_string(),
            qualified_configuration_sha256: digest('c'),
            observed_configuration_sha256: digest('c'),
            qualification_valid_until_epoch_s: 1_000,
            capability_status: EvidenceStatus::Current,
            calibration_status: EvidenceStatus::Current,
            sensor_status: EvidenceStatus::Current,
            protection_status: ProtectionStatus::Ready,
            maximum_event_discharge_power_w: 3_000.0,
            max_supported_duration_s: 30.0,
            usable_energy_wh_at_event: 500.0,
            usable_energy_uncertainty_bound_wh: 0.0,
            discharge_efficiency: 0.9,
            state_of_charge_fraction: 0.6,
            state_of_health_fraction: 0.95,
            temperature_c: 25.0,
            operating_condition_reference: "synthetic:soc-soh-thermal-point-1".to_string(),
            evidence_reference: "mycelix:storage-event-passport-1".to_string(),
        }
    }

    fn event(id: &str, start_s: f64, duration_s: f64, demand_w: f64, source_w: f64) -> MissionPowerEvent {
        MissionPowerEvent {
            event_id: id.to_string(),
            start_s,
            duration_s,
            required_peak_power_w: demand_w,
            maximum_unserved_energy_wh: 0.0,
            qualified_source_minimum_power_w: source_w,
            source_measurement_uncertainty_bound_w: 0.0,
            source_model_uncertainty_bound_w: 0.0,
            qualified_common_path_power_w: 7_500.0,
            common_path_uncertainty_bound_w: 0.0,
            qualified_common_path_configuration_sha256: digest('d'),
            observed_common_path_configuration_sha256: digest('d'),
            common_path_qualification_valid_until_epoch_s: 1_000,
            common_path_evidence_status: EvidenceStatus::Current,
            common_path_evidence_reference: format!("platform:common-path-{id}"),
            qualified_source_configuration_sha256: digest('b'),
            observed_source_configuration_sha256: digest('b'),
            source_qualification_valid_until_epoch_s: 1_000,
            source_evidence_status: EvidenceStatus::Current,
            source_evidence_reference: format!("symtropy:event-source-{id}"),
            storage_support: None,
        }
    }

    fn input(events: Vec<MissionPowerEvent>) -> PowerEventEnvelopeInput {
        PowerEventEnvelopeInput {
            mission_id: "synthetic-event-mission-1".to_string(),
            qualified_configuration_sha256: digest('a'),
            observed_configuration_sha256: digest('a'),
            evaluation_time_epoch_s: 100,
            qualification_valid_until_epoch_s: 1_000,
            mission_duration_s: 3_600.0,
            events,
        }
    }

    #[test]
    fn qualified_source_alone_passes_event() {
        let report = evaluate_power_event_envelope(&input(vec![
            event("load-step", 10.0, 0.1, 5_000.0, 5_000.0),
        ]))
        .unwrap();
        assert!(report.mission_event_pass);
        assert_eq!(report.event_results[0].combined_support_power_w, 5_000.0);
        assert_eq!(report.event_results[0].event_unserved_energy_wh, 0.0);
    }

    #[test]
    fn storage_closes_peak_power_gap_when_power_and_energy_are_sufficient() {
        let mut item = event("motor-start", 10.0, 1.0, 6_000.0, 4_000.0);
        item.storage_support = Some(storage());
        let report = evaluate_power_event_envelope(&input(vec![item])).unwrap();
        assert!(report.mission_event_pass);
        assert_eq!(report.event_results[0].conservative_storage_power_w, 3_000.0);
        assert_eq!(report.event_results[0].combined_support_power_w, 7_000.0);
        assert!(report.event_results[0].storage_eligible);
    }

    #[test]
    fn storage_power_duration_energy_limit_caps_support() {
        let mut item = event("long-pulse", 10.0, 1_800.0, 1_000.0, 0.0);
        let mut bank = storage();
        bank.max_supported_duration_s = 2_000.0;
        bank.maximum_event_discharge_power_w = 3_000.0;
        bank.usable_energy_wh_at_event = 10.0;
        item.storage_support = Some(bank);
        let report = evaluate_power_event_envelope(&input(vec![item])).unwrap();
        assert!((report.event_results[0].conservative_storage_power_w - 18.0).abs() < 1e-9);
        assert!(!report.mission_event_pass);
    }

    #[test]
    fn insufficient_storage_power_fails_event() {
        let mut item = event("motor-start", 10.0, 1.0, 6_000.0, 4_000.0);
        let mut bank = storage();
        bank.maximum_event_discharge_power_w = 1_000.0;
        item.storage_support = Some(bank);
        let report = evaluate_power_event_envelope(&input(vec![item])).unwrap();
        assert!(!report.mission_event_pass);
        assert_eq!(report.event_results[0].peak_power_shortfall_w, 1_000.0);
    }

    #[test]
    fn storage_duration_rating_shorter_than_event_is_excluded() {
        let mut item = event("long-motor-start", 10.0, 60.0, 6_000.0, 4_000.0);
        item.storage_support = Some(storage());
        let report = evaluate_power_event_envelope(&input(vec![item])).unwrap();
        assert_eq!(
            report.event_results[0].storage_exclusion_reason,
            Some("storage_event_duration_exceeded".to_string())
        );
        assert!(!report.mission_event_pass);
    }

    #[test]
    fn common_output_path_caps_source_plus_storage_sum() {
        let mut item = event("shared-output-limit", 10.0, 1.0, 8_000.0, 5_000.0);
        item.storage_support = Some(storage());
        item.qualified_common_path_power_w = 6_500.0;
        let report = evaluate_power_event_envelope(&input(vec![item])).unwrap();
        assert_eq!(report.event_results[0].conservative_source_power_w, 5_000.0);
        assert_eq!(report.event_results[0].conservative_storage_power_w, 3_000.0);
        assert_eq!(report.event_results[0].combined_support_power_w, 6_500.0);
        assert_eq!(report.event_results[0].peak_power_shortfall_w, 1_500.0);
        assert!(!report.mission_event_pass);
    }

    #[test]
    fn stale_common_path_evidence_withdraws_all_event_support() {
        let mut item = event("stale-common-path", 10.0, 1.0, 1_000.0, 5_000.0);
        item.common_path_evidence_status = EvidenceStatus::Unverified;
        let report = evaluate_power_event_envelope(&input(vec![item])).unwrap();
        assert!(!report.event_results[0].common_path_eligible);
        assert_eq!(report.event_results[0].combined_support_power_w, 0.0);
        assert!(!report.mission_event_pass);
    }

    #[test]
    fn common_path_configuration_mismatch_withdraws_all_event_support() {
        let mut item = event("changed-common-path", 10.0, 1.0, 1_000.0, 5_000.0);
        item.observed_common_path_configuration_sha256 = digest('e');
        let report = evaluate_power_event_envelope(&input(vec![item])).unwrap();
        assert!(!report.event_results[0].common_path_eligible);
        assert!(!report.mission_event_pass);
    }

    #[test]
    fn common_path_expiry_withdraws_all_event_support() {
        let mut item = event("expired-common-path", 10.0, 1.0, 1_000.0, 5_000.0);
        item.common_path_qualification_valid_until_epoch_s = 100;
        let report = evaluate_power_event_envelope(&input(vec![item])).unwrap();
        assert!(!report.event_results[0].common_path_eligible);
        assert!(!report.mission_event_pass);
    }

    #[test]
    fn source_uncertainty_reduces_event_support() {
        let mut item = event("load-step", 10.0, 1.0, 5_000.0, 5_000.0);
        item.source_measurement_uncertainty_bound_w = 300.0;
        item.source_model_uncertainty_bound_w = 200.0;
        let report = evaluate_power_event_envelope(&input(vec![item])).unwrap();
        assert_eq!(report.event_results[0].conservative_source_power_w, 4_500.0);
        assert!(!report.mission_event_pass);
    }

    #[test]
    fn stale_source_evidence_excludes_source() {
        let mut item = event("stale-source", 10.0, 1.0, 1_000.0, 5_000.0);
        item.source_evidence_status = EvidenceStatus::Unverified;
        let report = evaluate_power_event_envelope(&input(vec![item])).unwrap();
        assert!(!report.event_results[0].source_eligible);
        assert_eq!(report.event_results[0].conservative_source_power_w, 0.0);
        assert!(!report.mission_event_pass);
    }

    #[test]
    fn source_configuration_mismatch_excludes_source() {
        let mut item = event("changed-source", 10.0, 1.0, 1_000.0, 5_000.0);
        item.observed_source_configuration_sha256 = digest('d');
        let report = evaluate_power_event_envelope(&input(vec![item])).unwrap();
        assert!(!report.event_results[0].source_eligible);
        assert!(!report.mission_event_pass);
    }

    #[test]
    fn source_qualification_expiry_excludes_source() {
        let mut item = event("expired-source", 10.0, 1.0, 1_000.0, 5_000.0);
        item.source_qualification_valid_until_epoch_s = 100;
        let report = evaluate_power_event_envelope(&input(vec![item])).unwrap();
        assert!(!report.event_results[0].source_eligible);
        assert!(!report.mission_event_pass);
    }

    #[test]
    fn stale_storage_evidence_excludes_storage_and_fails_if_source_is_insufficient() {
        let mut item = event("stale-storage", 10.0, 1.0, 6_000.0, 4_000.0);
        let mut bank = storage();
        bank.sensor_status = EvidenceStatus::Expired;
        item.storage_support = Some(bank);
        let report = evaluate_power_event_envelope(&input(vec![item])).unwrap();
        assert!(!report.event_results[0].storage_eligible);
        assert_eq!(
            report.event_results[0].storage_exclusion_reason,
            Some("storage_sensor_evidence_not_current".to_string())
        );
        assert!(!report.mission_event_pass);
    }

    #[test]
    fn ineligible_storage_does_not_fail_event_if_source_alone_meets_it() {
        let mut item = event("source-alone", 10.0, 1.0, 4_000.0, 4_000.0);
        let mut bank = storage();
        bank.protection_status = ProtectionStatus::Unknown;
        item.storage_support = Some(bank);
        let report = evaluate_power_event_envelope(&input(vec![item])).unwrap();
        assert!(report.mission_event_pass);
        assert!(!report.event_results[0].storage_eligible);
    }

    #[test]
    fn storage_configuration_mismatch_excludes_storage() {
        let mut item = event("changed-storage", 10.0, 1.0, 6_000.0, 4_000.0);
        let mut bank = storage();
        bank.observed_configuration_sha256 = digest('d');
        item.storage_support = Some(bank);
        let report = evaluate_power_event_envelope(&input(vec![item])).unwrap();
        assert_eq!(
            report.event_results[0].storage_exclusion_reason,
            Some("storage_configuration_digest_mismatch".to_string())
        );
        assert!(!report.mission_event_pass);
    }

    #[test]
    fn storage_qualification_expiry_excludes_storage() {
        let mut item = event("expired-storage", 10.0, 1.0, 6_000.0, 4_000.0);
        let mut bank = storage();
        bank.qualification_valid_until_epoch_s = 100;
        item.storage_support = Some(bank);
        let report = evaluate_power_event_envelope(&input(vec![item])).unwrap();
        assert_eq!(
            report.event_results[0].storage_exclusion_reason,
            Some("storage_qualification_expired".to_string())
        );
        assert!(!report.mission_event_pass);
    }

    #[test]
    fn event_local_unserved_energy_limit_is_enforced() {
        let mut first = event("critical", 10.0, 3600.0, 1_000.0, 900.0);
        first.maximum_unserved_energy_wh = 0.0;
        let second = event("later", 3_700.0, 1.0, 1_000.0, 1_000.0);
        let mut value = input(vec![second, first]);
        value.mission_duration_s = 3_700.0;
        let report = evaluate_power_event_envelope(&value).unwrap();
        assert_eq!(report.failed_event_ids, vec!["critical".to_string()]);
        assert!(!report.mission_event_pass);
    }

    #[test]
    fn overlapping_event_windows_are_rejected() {
        let value = input(vec![
            event("first", 10.0, 5.0, 1_000.0, 1_000.0),
            event("overlap", 14.0, 5.0, 1_000.0, 1_000.0),
        ]);
        assert!(evaluate_power_event_envelope(&value).is_err());
    }

    #[test]
    fn adjacent_event_windows_are_allowed() {
        let report = evaluate_power_event_envelope(&input(vec![
            event("first", 10.0, 5.0, 1_000.0, 1_000.0),
            event("next", 15.0, 5.0, 1_000.0, 1_000.0),
        ]))
        .unwrap();
        assert!(report.mission_event_pass);
    }

    #[test]
    fn duplicate_event_ids_are_rejected() {
        let value = input(vec![
            event("duplicate", 10.0, 2.0, 1_000.0, 1_000.0),
            event("duplicate", 20.0, 2.0, 1_000.0, 1_000.0),
        ]);
        assert!(evaluate_power_event_envelope(&value).is_err());
    }

    #[test]
    fn event_beyond_mission_duration_is_rejected() {
        let mut value = input(vec![event("outside", 3_599.0, 2.0, 1_000.0, 1_000.0)]);
        value.mission_duration_s = 3_600.0;
        assert!(evaluate_power_event_envelope(&value).is_err());
    }

    #[test]
    fn global_configuration_mismatch_withdraws_all_event_support() {
        let mut value = input(vec![event("global-mismatch", 10.0, 1.0, 1_000.0, 5_000.0)]);
        value.observed_configuration_sha256 = digest('e');
        let report = evaluate_power_event_envelope(&value).unwrap();
        assert!(!report.global_qualification_eligible);
        assert_eq!(report.event_results[0].conservative_source_power_w, 0.0);
        assert!(!report.mission_event_pass);
    }

    #[test]
    fn invalid_storage_efficiency_is_rejected() {
        let mut item = event("bad-efficiency", 10.0, 1.0, 1_000.0, 0.0);
        let mut bank = storage();
        bank.discharge_efficiency = 1.1;
        item.storage_support = Some(bank);
        assert!(evaluate_power_event_envelope(&input(vec![item])).is_err());
    }

    #[test]
    fn non_finite_source_uncertainty_is_rejected() {
        let mut item = event("bad-uncertainty", 10.0, 1.0, 1_000.0, 5_000.0);
        item.source_model_uncertainty_bound_w = f64::NAN;
        assert!(evaluate_power_event_envelope(&input(vec![item])).is_err());
    }

    #[test]
    fn invalid_soc_is_rejected() {
        let mut item = event("bad-soc", 10.0, 1.0, 1_000.0, 0.0);
        let mut bank = storage();
        bank.state_of_charge_fraction = 1.1;
        item.storage_support = Some(bank);
        assert!(evaluate_power_event_envelope(&input(vec![item])).is_err());
    }

    #[test]
    fn event_input_order_does_not_change_result() {
        let mut a = event("a", 10.0, 1.0, 5_000.0, 4_000.0);
        a.storage_support = Some(storage());
        let b = event("b", 20.0, 2.0, 2_000.0, 2_000.0);
        let first = evaluate_power_event_envelope(&input(vec![a.clone(), b.clone()])).unwrap();
        let second = evaluate_power_event_envelope(&input(vec![b, a])).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn result_retains_exact_source_and_storage_lineage() {
        let mut item = event("lineage", 10.0, 1.0, 6_000.0, 4_000.0);
        item.storage_support = Some(storage());
        let report = evaluate_power_event_envelope(&input(vec![item])).unwrap();
        let result = &report.event_results[0];
        let bank = result.storage_constraint_snapshot.as_ref().unwrap();
        assert_eq!(result.source_evidence_reference, "symtropy:event-source-lineage");
        assert_eq!(result.qualified_source_configuration_sha256, digest('b'));
        assert_eq!(result.observed_source_configuration_sha256, digest('b'));
        assert_eq!(bank.evidence_reference, "mycelix:storage-event-passport-1");
        assert_eq!(bank.operating_condition_reference, "synthetic:soc-soh-thermal-point-1");
        assert!(result.storage_eligible);
    }

    #[test]
    fn claim_ceiling_excludes_waveform_and_protection_qualification() {
        let report = evaluate_power_event_envelope(&input(vec![
            event("claim", 10.0, 1.0, 1_000.0, 1_000.0),
        ]))
        .unwrap();
        assert_eq!(
            report.claim_ceiling(),
            "POWER_DURATION_EVENT_SCREEN_ONLY_NOT_WAVEFORM_OR_PROTECTION_QUALIFICATION"
        );
    }
}
