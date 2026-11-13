use crate::capability_envelope::EvidenceStatus;
use crate::lifecycle_service_cost::LifecycleEvidenceClass;
use crate::temporal_evidence::TaiTimeInterval;
use crate::AuditError;
use std::collections::{BTreeMap, BTreeSet};

const CLAIM: &str = "EXPOSURE_NORMALIZED_RELIABILITY_ACCOUNTING_ONLY_NOT_FAILURE_PROBABILITY_OR_SAFETY_CLAIM";
const NS_PER_HOUR: f64 = 3_600_000_000_000.0;
const HOURS_PER_YEAR: f64 = 8_760.0;
const MAX_EVENTS: usize = 100_000;
const MAX_EXPOSURE_WINDOWS: usize = 100_000;
const MAX_TEXT: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExposureAssetScope { SystemAsset, ComponentAsset }

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventClass {
    ConfirmedFailure,
    NearMiss,
    Precursor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutageClass {
    None,
    Planned,
    Unplanned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReliabilitySubsystem {
    BatteryCell,
    BatteryModule,
    BatteryManagement,
    PowerConversion,
    Inverter,
    Transformer,
    Switchgear,
    Protection,
    Controller,
    Communications,
    ThermalManagement,
    Generator,
    RenewableSource,
    AuxiliarySystem,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DetectionSource {
    ProtectionRelay,
    BatteryManagement,
    LocalController,
    SupervisoryController,
    InstrumentAlarm,
    Operator,
    Inspection,
    ExternalReport,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    NotAssessed,
    None,
    Minor,
    Moderate,
    Major,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SeverityDimension {
    PersonnelSafety,
    FireThermal,
    Electrical,
    Environmental,
    ServiceInterruption,
    EquipmentDamage,
    RegulatoryReportability,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootCauseConfidence {
    Confirmed,
    Probable,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SeverityProfile {
    pub personnel_safety: Severity,
    pub fire_thermal: Severity,
    pub electrical: Severity,
    pub environmental: Severity,
    pub service_interruption: Severity,
    pub equipment_damage: Severity,
    pub regulatory_reportability: Severity,
}

impl SeverityProfile {
    fn value(&self, dimension: SeverityDimension) -> Severity {
        match dimension {
            SeverityDimension::PersonnelSafety => self.personnel_safety,
            SeverityDimension::FireThermal => self.fire_thermal,
            SeverityDimension::Electrical => self.electrical,
            SeverityDimension::Environmental => self.environmental,
            SeverityDimension::ServiceInterruption => self.service_interruption,
            SeverityDimension::EquipmentDamage => self.equipment_damage,
            SeverityDimension::RegulatoryReportability => self.regulatory_reportability,
        }
    }

    fn core_assessment_complete(&self) -> bool {
        [
            self.personnel_safety,
            self.fire_thermal,
            self.electrical,
            self.environmental,
            self.service_interruption,
            self.equipment_damage,
            self.regulatory_reportability,
        ]
        .iter()
        .all(|value| *value != Severity::NotAssessed)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExposureWindow {
    pub exposure_window_id: String,
    pub exposure_population_id: String,
    /// The denominator asset must represent one system or component population,
    /// not a collection of overlapping subcomponents counted as independent systems.
    pub asset_id: String,
    pub asset_scope: ExposureAssetScope,
    pub topology_sha256: String,
    pub configuration_sha256: String,
    pub site_profile_sha256: String,
    pub start_tai: TaiTimeInterval,
    pub end_tai: TaiTimeInterval,
    pub operating_mode: String,
    pub installed_power_mw: f64,
    pub energized_hours: f64,
    pub charged_throughput_mwh: f64,
    pub discharged_throughput_mwh: f64,
    pub equivalent_full_cycles: f64,
    pub dispatch_attempts: u64,
    pub source_transfer_attempts: u64,
    pub telemetry_coverage_fraction: f64,
    pub evidence_status: EvidenceStatus,
    pub evidence_class: LifecycleEvidenceClass,
    pub timebase_evidence_status: EvidenceStatus,
    pub timebase_evidence_class: LifecycleEvidenceClass,
    pub timebase_evidence_reference: String,
    pub evidence_reference: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReliabilityEvent {
    pub event_id: String,
    pub incident_group_id: String,
    pub exposure_window_id: String,
    pub exposure_population_id: String,
    pub asset_id: String,
    pub component_id: String,
    pub subsystem: ReliabilitySubsystem,
    pub event_class: EventClass,
    pub outage_class: OutageClass,
    pub failure_mode: String,
    pub detection_source: DetectionSource,
    /// Both timestamps are uncertainty-bounded TAI intervals supplied by the
    /// temporal-evidence owner; this module does not reconstruct clock error.
    pub onset_tai: TaiTimeInterval,
    pub detected_tai: Option<TaiTimeInterval>,
    pub contained_tai: Option<TaiTimeInterval>,
    pub restored_tai: Option<TaiTimeInterval>,
    pub operating_mode: String,
    pub power_kw_at_onset: f64,
    pub voltage_v_at_onset: Option<f64>,
    pub current_a_at_onset: Option<f64>,
    pub state_of_charge_fraction: Option<f64>,
    pub temperature_c: Option<f64>,
    pub unserved_critical_service_wh: f64,
    pub critical_service_interruption_seconds: f64,
    pub reserve_shortfall_wh: f64,
    pub severity: SeverityProfile,
    pub root_cause_confidence: RootCauseConfidence,
    pub common_cause_group_id: Option<String>,
    pub replacement_component_id: Option<String>,
    pub repair_labor_hours: f64,
    pub requalification_hours: f64,
    pub topology_sha256: String,
    pub configuration_sha256: String,
    pub evidence_status: EvidenceStatus,
    pub evidence_class: LifecycleEvidenceClass,
    pub timebase_evidence_status: EvidenceStatus,
    pub timebase_evidence_class: LifecycleEvidenceClass,
    pub timebase_evidence_reference: String,
    pub root_cause_evidence_reference: String,
    pub evidence_reference: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReliabilityAccountingInput {
    pub candidate_id: String,
    pub comparator_set_id: String,
    pub evaluation_time_tai: TaiTimeInterval,
    pub minimum_telemetry_coverage_fraction: f64,
    pub event_history_complete: bool,
    pub event_history_evidence_status: EvidenceStatus,
    pub event_history_evidence_class: LifecycleEvidenceClass,
    pub event_history_evidence_reference: String,
    pub exposure_windows: Vec<ExposureWindow>,
    pub events: Vec<ReliabilityEvent>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReliabilityRate {
    pub metric_id: &'static str,
    pub numerator_incidents: usize,
    pub denominator_value: f64,
    pub denominator_unit: &'static str,
    pub normalization_factor: f64,
    pub rate: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SeverityCount {
    pub dimension: SeverityDimension,
    pub severity: Severity,
    pub incident_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DurationQuantileBounds {
    pub sample_count: usize,
    pub p50_lower_hours: Option<f64>,
    pub p50_upper_hours: Option<f64>,
    pub p90_lower_hours: Option<f64>,
    pub p90_upper_hours: Option<f64>,
    pub p95_lower_hours: Option<f64>,
    pub p95_upper_hours: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReliabilityReport {
    pub candidate_id: String,
    pub comparator_set_id: String,
    pub exposure_window_count: usize,
    pub event_record_count: usize,
    pub confirmed_failure_incident_count: usize,
    pub near_miss_incident_count: usize,
    pub precursor_incident_count: usize,
    pub planned_outage_incident_count: usize,
    pub unplanned_outage_incident_count: usize,
    pub open_outage_incident_count: usize,
    pub common_cause_group_count: usize,
    pub confirmed_root_cause_incident_count: usize,
    pub probable_root_cause_incident_count: usize,
    pub unknown_root_cause_incident_count: usize,
    pub asset_scope: ExposureAssetScope,
    pub total_energized_asset_hours: f64,
    pub total_installed_mw_years_lower_bound: f64,
    pub total_charged_throughput_mwh: f64,
    pub total_discharged_throughput_mwh: f64,
    pub total_equivalent_full_cycles: f64,
    pub total_dispatch_attempts: u128,
    pub total_source_transfer_attempts: u128,
    pub minimum_telemetry_coverage_fraction: f64,
    pub failure_rates: Vec<ReliabilityRate>,
    pub severity_counts: Vec<SeverityCount>,
    pub total_unserved_critical_service_wh: f64,
    pub total_reserve_shortfall_wh: f64,
    pub total_repair_labor_hours: f64,
    pub total_requalification_hours: f64,
    pub planned_asset_unavailable_hours_lower: f64,
    pub planned_asset_unavailable_hours_upper: f64,
    pub unplanned_asset_unavailable_hours_lower: f64,
    pub unplanned_asset_unavailable_hours_upper: f64,
    pub all_outage_asset_unavailable_hours_lower: f64,
    pub all_outage_asset_unavailable_hours_upper: f64,
    pub unplanned_repair_duration_quantiles: DurationQuantileBounds,
    pub event_history_complete: bool,
    pub evidence_complete: bool,
    pub severity_assessment_complete: bool,
    pub physical_reliability_claim_eligible: bool,
    pub claim_ceiling: &'static str,
}

impl ReliabilityReport {
    pub const fn claim_ceiling(&self) -> &'static str { CLAIM }
}

#[derive(Debug, Clone, Copy)]
struct Span {
    start_ns: i128,
    end_ns: i128,
}

#[derive(Debug, Clone, Copy)]
struct DurationBounds {
    lower_ns: u128,
    upper_ns: u128,
}

fn nonblank(value: &str) -> bool { !value.trim().is_empty() }

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn finite_nonnegative(value: f64) -> bool { value.is_finite() && value >= 0.0 }

fn valid_interval(value: &TaiTimeInterval) -> bool {
    value.lower_bound_ns <= value.upper_bound_ns
}

fn within_interval(value: &TaiTimeInterval, window_start: &TaiTimeInterval, window_end: &TaiTimeInterval) -> bool {
    value.lower_bound_ns >= window_start.upper_bound_ns
        && value.upper_bound_ns <= window_end.lower_bound_ns
}

fn duration_bounds(start: &TaiTimeInterval, end: &TaiTimeInterval) -> Result<DurationBounds, AuditError> {
    if !valid_interval(start) || !valid_interval(end) || end.upper_bound_ns < start.lower_bound_ns {
        return Err(AuditError("event duration intervals are invalid or end is before onset".to_string()));
    }
    let minimum = (end.lower_bound_ns - start.upper_bound_ns).max(0) as u128;
    let maximum = (end.upper_bound_ns - start.lower_bound_ns).max(0) as u128;
    Ok(DurationBounds { lower_ns: minimum, upper_ns: maximum })
}

fn duration_hours(value: u128) -> f64 { value as f64 / NS_PER_HOUR }

fn union_span_ns(mut spans: Vec<Span>) -> Result<u128, AuditError> {
    if spans.is_empty() { return Ok(0); }
    spans.sort_by_key(|span| (span.start_ns, span.end_ns));
    let mut total = 0_u128;
    let mut current_start = spans[0].start_ns;
    let mut current_end = spans[0].end_ns;

    for span in spans.into_iter().skip(1) {
        if span.end_ns < span.start_ns {
            return Err(AuditError("outage span end precedes start".to_string()));
        }
        if span.start_ns <= current_end {
            current_end = current_end.max(span.end_ns);
        } else {
            total = total.checked_add((current_end - current_start) as u128)
                .ok_or_else(|| AuditError("outage union duration overflowed".to_string()))?;
            current_start = span.start_ns;
            current_end = span.end_ns;
        }
    }
    total = total.checked_add((current_end - current_start) as u128)
        .ok_or_else(|| AuditError("outage union duration overflowed".to_string()))?;
    Ok(total)
}

fn nearest_rank(values: &[u128], numerator: usize, denominator: usize) -> Option<u128> {
    if values.is_empty() || denominator == 0 { return None; }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let rank = (numerator * sorted.len() + denominator - 1) / denominator;
    let index = rank.saturating_sub(1).min(sorted.len() - 1);
    sorted.get(index).copied()
}

fn rate(metric_id: &'static str, count: usize, denominator: f64, unit: &'static str, factor: f64) -> ReliabilityRate {
    let value = if denominator.is_finite() && denominator > 0.0 {
        Some(count as f64 / denominator * factor)
    } else {
        None
    };
    ReliabilityRate {
        metric_id,
        numerator_incidents: count,
        denominator_value: denominator,
        denominator_unit: unit,
        normalization_factor: factor,
        rate: value,
    }
}

fn evidence_is_physical(status: EvidenceStatus, class: LifecycleEvidenceClass) -> bool {
    status == EvidenceStatus::Current && class == LifecycleEvidenceClass::IndependentPhysicalMeasurement
}

fn validate_exposure(window: &ExposureWindow) -> Result<(), AuditError> {
    if !nonblank(&window.exposure_window_id)
        || !nonblank(&window.exposure_population_id)
        || !nonblank(&window.asset_id)
        || !nonblank(&window.operating_mode)
        || !nonblank(&window.evidence_reference)
        || !nonblank(&window.timebase_evidence_reference)
        || !valid_sha256(&window.topology_sha256)
        || !valid_sha256(&window.configuration_sha256)
        || !valid_sha256(&window.site_profile_sha256)
        || !valid_interval(&window.start_tai)
        || !valid_interval(&window.end_tai)
        || window.end_tai.lower_bound_ns <= window.start_tai.upper_bound_ns
        || !finite_nonnegative(window.installed_power_mw)
        || !finite_nonnegative(window.energized_hours)
        || !finite_nonnegative(window.charged_throughput_mwh)
        || !finite_nonnegative(window.discharged_throughput_mwh)
        || !finite_nonnegative(window.equivalent_full_cycles)
        || !window.telemetry_coverage_fraction.is_finite()
        || !(0.0..=1.0).contains(&window.telemetry_coverage_fraction)
    {
        return Err(AuditError(format!("exposure window {:?} is invalid", window.exposure_window_id)));
    }
    let maximum_duration_hours =
        (window.end_tai.upper_bound_ns - window.start_tai.lower_bound_ns) as f64 / NS_PER_HOUR;
    if window.energized_hours > maximum_duration_hours + 1.0e-6 {
        return Err(AuditError(format!(
            "exposure window {:?} reports more energized hours than its elapsed interval",
            window.exposure_window_id
        )));
    }
    Ok(())
}

fn validate_event<'a>(
    event: &ReliabilityEvent,
    windows: &BTreeMap<&str, &'a ExposureWindow>,
) -> Result<&'a ExposureWindow, AuditError> {
    if !nonblank(&event.event_id)
        || !nonblank(&event.incident_group_id)
        || !nonblank(&event.exposure_window_id)
        || !nonblank(&event.exposure_population_id)
        || !nonblank(&event.asset_id)
        || !nonblank(&event.component_id)
        || !nonblank(&event.failure_mode)
        || !nonblank(&event.operating_mode)
        || !nonblank(&event.timebase_evidence_reference)
        || !nonblank(&event.root_cause_evidence_reference)
        || !nonblank(&event.evidence_reference)
        || !valid_sha256(&event.topology_sha256)
        || !valid_sha256(&event.configuration_sha256)
        || !valid_interval(&event.onset_tai)
        || !finite_nonnegative(event.power_kw_at_onset)
        || !finite_nonnegative(event.unserved_critical_service_wh)
        || !finite_nonnegative(event.critical_service_interruption_seconds)
        || !finite_nonnegative(event.reserve_shortfall_wh)
        || !finite_nonnegative(event.repair_labor_hours)
        || !finite_nonnegative(event.requalification_hours)
        || event.voltage_v_at_onset.is_some_and(|value| !finite_nonnegative(value))
        || event.current_a_at_onset.is_some_and(|value| !value.is_finite())
        || event.state_of_charge_fraction.is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value))
        || event.temperature_c.is_some_and(|value| !value.is_finite())
        || event.common_cause_group_id.as_ref().is_some_and(|value| !nonblank(value))
        || event.replacement_component_id.as_ref().is_some_and(|value| !nonblank(value))
    {
        return Err(AuditError(format!("reliability event {:?} is invalid", event.event_id)));
    }

    let window = *windows.get(event.exposure_window_id.as_str())
        .ok_or_else(|| AuditError(format!("event {:?} references an unknown exposure window", event.event_id)))?;
    if event.exposure_population_id != window.exposure_population_id
        || event.asset_id != window.asset_id
        || event.topology_sha256 != window.topology_sha256
        || event.configuration_sha256 != window.configuration_sha256
        || !within_interval(&event.onset_tai, &window.start_tai, &window.end_tai)
    {
        return Err(AuditError(format!(
            "event {:?} is not matched to its exposure population, asset, configuration or TAI interval",
            event.event_id
        )));
    }
    for timestamp in [event.detected_tai.as_ref(), event.contained_tai.as_ref(), event.restored_tai.as_ref()].into_iter().flatten() {
        if !valid_interval(timestamp) || !within_interval(timestamp, &window.start_tai, &window.end_tai) {
            return Err(AuditError(format!("event {:?} has an invalid or out-of-window lifecycle timestamp", event.event_id)));
        }
    }
    if let Some(detected) = &event.detected_tai {
        let _ = duration_bounds(&event.onset_tai, detected)?;
    }
    if let Some(contained) = &event.contained_tai {
        let previous = event.detected_tai.as_ref().unwrap_or(&event.onset_tai);
        let _ = duration_bounds(previous, contained)?;
    }
    if let Some(restored) = &event.restored_tai {
        let previous = event.contained_tai.as_ref()
            .or(event.detected_tai.as_ref())
            .unwrap_or(&event.onset_tai);
        let _ = duration_bounds(previous, restored)?;
    }
    if event.outage_class != OutageClass::None && event.restored_tai.is_none() {
        // Open outages are allowed and reported; they contribute only through the
        // explicitly observed interruption/service-impact fields in this version.
    }
    Ok(window)
}

fn validate_exposure_non_overlap(windows: &[ExposureWindow]) -> Result<(), AuditError> {
    let mut grouped: BTreeMap<(&str, &str), Vec<&ExposureWindow>> = BTreeMap::new();
    for window in windows {
        grouped.entry((window.exposure_population_id.as_str(), window.asset_id.as_str()))
            .or_default().push(window);
    }
    for (_, values) in grouped.iter_mut() {
        values.sort_by_key(|window| window.start_tai.lower_bound_ns);
        for pair in values.windows(2) {
            if pair[1].start_tai.lower_bound_ns < pair[0].end_tai.upper_bound_ns {
                return Err(AuditError(format!(
                    "overlapping exposure windows {:?} and {:?} would double-count denominator exposure",
                    pair[0].exposure_window_id, pair[1].exposure_window_id
                )));
            }
        }
    }
    Ok(())
}

fn severity_dimensions() -> [SeverityDimension; 7] {
    [
        SeverityDimension::PersonnelSafety,
        SeverityDimension::FireThermal,
        SeverityDimension::Electrical,
        SeverityDimension::Environmental,
        SeverityDimension::ServiceInterruption,
        SeverityDimension::EquipmentDamage,
        SeverityDimension::RegulatoryReportability,
    ]
}

fn severity_levels() -> [Severity; 6] {
    [
        Severity::NotAssessed,
        Severity::None,
        Severity::Minor,
        Severity::Moderate,
        Severity::Major,
        Severity::Critical,
    ]
}

/// Read-only multi-denominator reliability accountant. It does not estimate
/// failure probability, infer root cause or certify safety. It consumes TAI
/// uncertainty intervals from the separate temporal-evidence owner.
pub fn evaluate_reliability_accounting(input: &ReliabilityAccountingInput) -> Result<ReliabilityReport, AuditError> {
    if !nonblank(&input.candidate_id)
        || !nonblank(&input.comparator_set_id)
        || !nonblank(&input.event_history_evidence_reference)
        || !valid_interval(&input.evaluation_time_tai)
        || !input.minimum_telemetry_coverage_fraction.is_finite()
        || !(0.0..=1.0).contains(&input.minimum_telemetry_coverage_fraction)
        || input.exposure_windows.is_empty()
        || input.exposure_windows.len() > MAX_EXPOSURE_WINDOWS
        || input.events.len() > MAX_EVENTS
    {
        return Err(AuditError("reliability accounting metadata or record counts are invalid".to_string()));
    }

    let mut window_ids = BTreeSet::new();
    let mut windows_by_id: BTreeMap<&str, &ExposureWindow> = BTreeMap::new();
    let cohort = &input.exposure_windows[0];
    for window in &input.exposure_windows {
        validate_exposure(window)?;
        if window.exposure_population_id != cohort.exposure_population_id
            || window.topology_sha256 != cohort.topology_sha256
            || window.configuration_sha256 != cohort.configuration_sha256
            || window.site_profile_sha256 != cohort.site_profile_sha256
            || window.asset_scope != cohort.asset_scope
        {
            return Err(AuditError("one reliability report cannot pool different populations, topologies, configurations or site profiles".to_string()));
        }
        if !window_ids.insert(window.exposure_window_id.as_str()) {
            return Err(AuditError(format!("duplicate exposure window ID {:?}", window.exposure_window_id)));
        }
        windows_by_id.insert(window.exposure_window_id.as_str(), window);
    }
    validate_exposure_non_overlap(&input.exposure_windows)?;

    let mut event_ids = BTreeSet::new();
    let mut incident_classes: BTreeMap<&str, EventClass> = BTreeMap::new();
    let mut incident_outage_classes: BTreeMap<&str, OutageClass> = BTreeMap::new();
    let mut incident_root_confidence: BTreeMap<&str, RootCauseConfidence> = BTreeMap::new();
    let mut failure_incidents = BTreeSet::new();
    let mut near_miss_incidents = BTreeSet::new();
    let mut precursor_incidents = BTreeSet::new();
    let mut planned_outage_incidents = BTreeSet::new();
    let mut unplanned_outage_incidents = BTreeSet::new();
    let mut common_cause_groups = BTreeSet::new();
    let mut confirmed_root_cause = BTreeSet::new();
    let mut probable_root_cause = BTreeSet::new();
    let mut unknown_root_cause = BTreeSet::new();
    let mut severity_by_incident: BTreeMap<(&str, SeverityDimension), Severity> = BTreeMap::new();

    let mut total_unserved = 0.0_f64;
    let mut total_reserve_shortfall = 0.0_f64;
    let mut total_repair_labor = 0.0_f64;
    let mut total_requalification = 0.0_f64;
    let mut total_unplanned_lower_durations = Vec::new();
    let mut total_unplanned_upper_durations = Vec::new();
    let mut open_outage_incidents = BTreeSet::new();

    let mut planned_lower: BTreeMap<&str, Vec<Span>> = BTreeMap::new();
    let mut planned_upper: BTreeMap<&str, Vec<Span>> = BTreeMap::new();
    let mut unplanned_lower: BTreeMap<&str, Vec<Span>> = BTreeMap::new();
    let mut unplanned_upper: BTreeMap<&str, Vec<Span>> = BTreeMap::new();
    let mut all_lower: BTreeMap<&str, Vec<Span>> = BTreeMap::new();
    let mut all_upper: BTreeMap<&str, Vec<Span>> = BTreeMap::new();

    let mut evidence_complete = input.event_history_complete
        && input.event_history_evidence_status == EvidenceStatus::Current
        && input.event_history_evidence_class == LifecycleEvidenceClass::IndependentPhysicalMeasurement;

    for event in &input.events {
        if !event_ids.insert(event.event_id.as_str()) {
            return Err(AuditError(format!("duplicate reliability event ID {:?}", event.event_id)));
        }
        let window = validate_event(event, &windows_by_id)?;
        if let Some(previous) = incident_classes.insert(&event.incident_group_id, event.event_class) {
            if previous != event.event_class {
                return Err(AuditError(format!(
                    "incident group {:?} contains contradictory event classes",
                    event.incident_group_id
                )));
            }
        }
        match event.event_class {
            EventClass::ConfirmedFailure => { failure_incidents.insert(event.incident_group_id.as_str()); }
            EventClass::NearMiss => { near_miss_incidents.insert(event.incident_group_id.as_str()); }
            EventClass::Precursor => { precursor_incidents.insert(event.incident_group_id.as_str()); }
        }
        if event.outage_class != OutageClass::None {
            if let Some(previous) = incident_outage_classes.insert(event.incident_group_id.as_str(), event.outage_class) {
                if previous != event.outage_class {
                    return Err(AuditError(format!(
                        "incident group {:?} contains contradictory outage classifications",
                        event.incident_group_id
                    )));
                }
            }
        }
        match event.outage_class {
            OutageClass::None => {}
            OutageClass::Planned => { planned_outage_incidents.insert(event.incident_group_id.as_str()); }
            OutageClass::Unplanned => { unplanned_outage_incidents.insert(event.incident_group_id.as_str()); }
        }
        if let Some(previous) = incident_root_confidence.insert(event.incident_group_id.as_str(), event.root_cause_confidence) {
            if previous != event.root_cause_confidence {
                return Err(AuditError(format!(
                    "incident group {:?} contains contradictory root-cause confidence",
                    event.incident_group_id
                )));
            }
        }
        match event.root_cause_confidence {
            RootCauseConfidence::Confirmed => { confirmed_root_cause.insert(event.incident_group_id.as_str()); }
            RootCauseConfidence::Probable => { probable_root_cause.insert(event.incident_group_id.as_str()); }
            RootCauseConfidence::Unknown => { unknown_root_cause.insert(event.incident_group_id.as_str()); }
        }
        if let Some(common_cause) = &event.common_cause_group_id {
            common_cause_groups.insert(common_cause.as_str());
        }
        for dimension in severity_dimensions() {
            let value = event.severity.value(dimension);
            severity_by_incident.entry((event.incident_group_id.as_str(), dimension))
                .and_modify(|existing| {
                    *existing = if *existing == Severity::NotAssessed || value == Severity::NotAssessed {
                        Severity::NotAssessed
                    } else {
                        (*existing).max(value)
                    };
                })
                .or_insert(value);
        }

        total_unserved += event.unserved_critical_service_wh;
        total_reserve_shortfall += event.reserve_shortfall_wh;
        total_repair_labor += event.repair_labor_hours;
        total_requalification += event.requalification_hours;
        if !total_unserved.is_finite() || !total_reserve_shortfall.is_finite()
            || !total_repair_labor.is_finite() || !total_requalification.is_finite() {
            return Err(AuditError("reliability event totals overflowed".to_string()));
        }

        evidence_complete &= evidence_is_physical(event.evidence_status, event.evidence_class)
            && evidence_is_physical(event.timebase_evidence_status, event.timebase_evidence_class);
        if !event.severity.core_assessment_complete() {
            // Retain partial severity evidence but prevent a full physical claim.
        }

        if event.outage_class != OutageClass::None {
            if let Some(restored) = &event.restored_tai {
                let bounds = duration_bounds(&event.onset_tai, restored)?;
                if event.outage_class == OutageClass::Unplanned {
                    total_unplanned_lower_durations.push(bounds.lower_ns);
                    total_unplanned_upper_durations.push(bounds.upper_ns);
                }

                let inner = if restored.lower_bound_ns > event.onset_tai.upper_bound_ns {
                    Some(Span { start_ns: event.onset_tai.upper_bound_ns, end_ns: restored.lower_bound_ns })
                } else { None };
                let outer = if restored.upper_bound_ns > event.onset_tai.lower_bound_ns {
                    Some(Span { start_ns: event.onset_tai.lower_bound_ns, end_ns: restored.upper_bound_ns })
                } else { None };

                if let Some(span) = inner {
                    all_lower.entry(&event.asset_id).or_default().push(span);
                    match event.outage_class {
                        OutageClass::Planned => planned_lower.entry(&event.asset_id).or_default().push(span),
                        OutageClass::Unplanned => unplanned_lower.entry(&event.asset_id).or_default().push(span),
                        OutageClass::None => {}
                    }
                }
                if let Some(span) = outer {
                    all_upper.entry(&event.asset_id).or_default().push(span);
                    match event.outage_class {
                        OutageClass::Planned => planned_upper.entry(&event.asset_id).or_default().push(span),
                        OutageClass::Unplanned => unplanned_upper.entry(&event.asset_id).or_default().push(span),
                        OutageClass::None => {}
                    }
                }
            } else {
                if !within_interval(&input.evaluation_time_tai, &window.start_tai, &window.end_tai) {
                    return Err(AuditError("open outage evaluation time is outside its exposure window".to_string()));
                }
                let bounds = duration_bounds(&event.onset_tai, &input.evaluation_time_tai)?;
                let inner = if input.evaluation_time_tai.lower_bound_ns > event.onset_tai.upper_bound_ns {
                    Some(Span { start_ns: event.onset_tai.upper_bound_ns, end_ns: input.evaluation_time_tai.lower_bound_ns })
                } else { None };
                let outer = if input.evaluation_time_tai.upper_bound_ns > event.onset_tai.lower_bound_ns {
                    Some(Span { start_ns: event.onset_tai.lower_bound_ns, end_ns: input.evaluation_time_tai.upper_bound_ns })
                } else { None };
                if let Some(span) = inner {
                    all_lower.entry(event.asset_id.as_str()).or_default().push(span);
                    match event.outage_class {
                        OutageClass::Planned => planned_lower.entry(event.asset_id.as_str()).or_default().push(span),
                        OutageClass::Unplanned => unplanned_lower.entry(event.asset_id.as_str()).or_default().push(span),
                        OutageClass::None => {}
                    }
                }
                if let Some(span) = outer {
                    all_upper.entry(event.asset_id.as_str()).or_default().push(span);
                    match event.outage_class {
                        OutageClass::Planned => planned_upper.entry(event.asset_id.as_str()).or_default().push(span),
                        OutageClass::Unplanned => unplanned_upper.entry(event.asset_id.as_str()).or_default().push(span),
                        OutageClass::None => {}
                    }
                }
                open_outage_incidents.insert(event.incident_group_id.as_str());
                if bounds.upper_ns == 0 {
                    return Err(AuditError("open outage has no positive duration bound".to_string()));
                }
            }
        }
    }

    let mut total_energized_hours = 0.0_f64;
    let mut total_mw_years_lower_bound = 0.0_f64;
    let mut total_charged_mwh = 0.0_f64;
    let mut total_discharged_mwh = 0.0_f64;
    let mut total_efc = 0.0_f64;
    let mut total_dispatches = 0_u128;
    let mut total_transfers = 0_u128;
    let mut minimum_coverage = 1.0_f64;

    for window in &input.exposure_windows {
        total_energized_hours += window.energized_hours;
        let minimum_exposure_hours = (window.end_tai.lower_bound_ns - window.start_tai.upper_bound_ns) as f64 / NS_PER_HOUR;
        total_mw_years_lower_bound += window.installed_power_mw * minimum_exposure_hours / HOURS_PER_YEAR;
        total_charged_mwh += window.charged_throughput_mwh;
        total_discharged_mwh += window.discharged_throughput_mwh;
        total_efc += window.equivalent_full_cycles;
        total_dispatches = total_dispatches.checked_add(u128::from(window.dispatch_attempts))
            .ok_or_else(|| AuditError("dispatch exposure total overflowed".to_string()))?;
        total_transfers = total_transfers.checked_add(u128::from(window.source_transfer_attempts))
            .ok_or_else(|| AuditError("source-transfer exposure total overflowed".to_string()))?;
        minimum_coverage = minimum_coverage.min(window.telemetry_coverage_fraction);
        evidence_complete &= evidence_is_physical(window.evidence_status, window.evidence_class)
            && evidence_is_physical(window.timebase_evidence_status, window.timebase_evidence_class);
    }

    if [
        total_energized_hours, total_mw_years_lower_bound, total_charged_mwh, total_discharged_mwh,
        total_efc, total_unserved, total_reserve_shortfall, total_repair_labor, total_requalification,
    ].iter().any(|value| !value.is_finite() || *value < 0.0) {
        return Err(AuditError("reliability aggregate contains a non-finite or negative total".to_string()));
    }

    evidence_complete &= minimum_coverage >= input.minimum_telemetry_coverage_fraction;
    let severity_assessment_complete = severity_dimensions().iter().all(|dimension| {
        severity_by_incident.iter()
            .filter(|((_, d), _)| d == dimension)
            .all(|(_, severity)| *severity != Severity::NotAssessed)
    });

    let mut severity_counts = Vec::new();
    for dimension in severity_dimensions() {
        for level in severity_levels() {
            let count = severity_by_incident.iter()
                .filter(|((_, d), severity)| *d == dimension && **severity == level)
                .count();
            if count > 0 {
                severity_counts.push(SeverityCount { dimension, severity: level, incident_count: count });
            }
        }
    }

    let confirmed_count = failure_incidents.len();
    let failure_rates = vec![
        rate("confirmed_failures_per_1000_energized_asset_hours", confirmed_count, total_energized_hours, "asset-hour", 1_000.0),
        rate("confirmed_failures_per_installed_mw_year_lower_bound", confirmed_count, total_mw_years_lower_bound, "installed-MW-year-lower-bound", 1.0),
        rate("confirmed_failures_per_1000_charged_mwh", confirmed_count, total_charged_mwh, "charged-MWh", 1_000.0),
        rate("confirmed_failures_per_1000_discharged_mwh", confirmed_count, total_discharged_mwh, "discharged-MWh", 1_000.0),
        rate("confirmed_failures_per_1000_equivalent_full_cycles", confirmed_count, total_efc, "equivalent-full-cycle", 1_000.0),
        rate("confirmed_failures_per_10000_dispatch_attempts", confirmed_count, total_dispatches as f64, "dispatch-attempt", 10_000.0),
        rate("confirmed_failures_per_1000_source_transfer_attempts", confirmed_count, total_transfers as f64, "source-transfer-attempt", 1_000.0),
    ];

    fn union_by_asset(map: &BTreeMap<&str, Vec<Span>>) -> Result<u128, AuditError> {
        let mut total = 0_u128;
        for spans in map.values() {
            total = total.checked_add(union_span_ns(spans.clone())?)
                .ok_or_else(|| AuditError("per-asset outage total overflowed".to_string()))?;
        }
        Ok(total)
    }

    let planned_lower_ns = union_by_asset(&planned_lower)?;
    let planned_upper_ns = union_by_asset(&planned_upper)?;
    let unplanned_lower_ns = union_by_asset(&unplanned_lower)?;
    let unplanned_upper_ns = union_by_asset(&unplanned_upper)?;
    let all_lower_ns = union_by_asset(&all_lower)?;
    let all_upper_ns = union_by_asset(&all_upper)?;

    let quantile = |values: &[u128], n: usize, d: usize| -> Option<f64> {
        nearest_rank(values, n, d).map(duration_hours)
    };
    let mttr = DurationQuantileBounds {
        sample_count: total_unplanned_lower_durations.len(),
        p50_lower_hours: quantile(&total_unplanned_lower_durations, 1, 2),
        p50_upper_hours: quantile(&total_unplanned_upper_durations, 1, 2),
        p90_lower_hours: quantile(&total_unplanned_lower_durations, 9, 10),
        p90_upper_hours: quantile(&total_unplanned_upper_durations, 9, 10),
        p95_lower_hours: quantile(&total_unplanned_lower_durations, 95, 100),
        p95_upper_hours: quantile(&total_unplanned_upper_durations, 95, 100),
    };

    Ok(ReliabilityReport {
        candidate_id: input.candidate_id.clone(),
        comparator_set_id: input.comparator_set_id.clone(),
        exposure_window_count: input.exposure_windows.len(),
        event_record_count: input.events.len(),
        confirmed_failure_incident_count: failure_incidents.len(),
        near_miss_incident_count: near_miss_incidents.len(),
        precursor_incident_count: precursor_incidents.len(),
        planned_outage_incident_count: planned_outage_incidents.len(),
        unplanned_outage_incident_count: unplanned_outage_incidents.len(),
        open_outage_incident_count: open_outage_incidents.len(),
        common_cause_group_count: common_cause_groups.len(),
        confirmed_root_cause_incident_count: confirmed_root_cause.len(),
        probable_root_cause_incident_count: probable_root_cause.len(),
        unknown_root_cause_incident_count: unknown_root_cause.len(),
        asset_scope: cohort.asset_scope,
        total_energized_asset_hours: total_energized_hours,
        total_installed_mw_years_lower_bound: total_mw_years_lower_bound,
        total_charged_throughput_mwh: total_charged_mwh,
        total_discharged_throughput_mwh: total_discharged_mwh,
        total_equivalent_full_cycles: total_efc,
        total_dispatch_attempts: total_dispatches,
        total_source_transfer_attempts: total_transfers,
        minimum_telemetry_coverage_fraction: minimum_coverage,
        failure_rates,
        severity_counts,
        total_unserved_critical_service_wh: total_unserved,
        total_reserve_shortfall_wh: total_reserve_shortfall,
        total_repair_labor_hours: total_repair_labor,
        total_requalification_hours: total_requalification,
        planned_asset_unavailable_hours_lower: duration_hours(planned_lower_ns),
        planned_asset_unavailable_hours_upper: duration_hours(planned_upper_ns),
        unplanned_asset_unavailable_hours_lower: duration_hours(unplanned_lower_ns),
        unplanned_asset_unavailable_hours_upper: duration_hours(unplanned_upper_ns),
        all_outage_asset_unavailable_hours_lower: duration_hours(all_lower_ns),
        all_outage_asset_unavailable_hours_upper: duration_hours(all_upper_ns),
        unplanned_repair_duration_quantiles: mttr,
        event_history_complete: input.event_history_complete,
        evidence_complete,
        severity_assessment_complete,
        physical_reliability_claim_eligible: evidence_complete
            && input.event_history_complete
            && severity_assessment_complete,
        claim_ceiling: CLAIM,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(ch: char) -> String { std::iter::repeat(ch).take(64).collect() }
    fn interval(start: i128, end: i128) -> TaiTimeInterval {
        TaiTimeInterval { lower_bound_ns: start, upper_bound_ns: end }
    }

    fn window() -> ExposureWindow {
        let day = 86_400_000_000_000_i128;
        ExposureWindow {
            exposure_window_id: "window-1".into(),
            exposure_population_id: "microgrid-fleet-a".into(),
            asset_id: "system-01".into(),
            asset_scope: ExposureAssetScope::SystemAsset,
            topology_sha256: h('a'),
            configuration_sha256: h('b'),
            site_profile_sha256: h('c'),
            start_tai: interval(1_000_000_000, 1_000_000_000),
            end_tai: interval(1_000_000_000 + 30 * day, 1_000_000_000 + 30 * day),
            operating_mode: "mixed-connected-islanded".into(),
            installed_power_mw: 1.0,
            energized_hours: 720.0,
            charged_throughput_mwh: 1_000.0,
            discharged_throughput_mwh: 1_500.0,
            equivalent_full_cycles: 300.0,
            dispatch_attempts: 500,
            source_transfer_attempts: 20,
            telemetry_coverage_fraction: 0.99,
            evidence_status: EvidenceStatus::Current,
            evidence_class: LifecycleEvidenceClass::SyntheticFixture,
            timebase_evidence_status: EvidenceStatus::Current,
            timebase_evidence_class: LifecycleEvidenceClass::SyntheticFixture,
            timebase_evidence_reference: "synthetic://timebase-window".into(),
            evidence_reference: "synthetic://exposure-window".into(),
        }
    }

    fn event(id: &str, incident: &str, class: EventClass, outage: OutageClass, onset_ns: i128) -> ReliabilityEvent {
        let second = 1_000_000_000_i128;
        let day = 86_400 * second;
        ReliabilityEvent {
            event_id: id.into(),
            incident_group_id: incident.into(),
            exposure_window_id: "window-1".into(),
            exposure_population_id: "microgrid-fleet-a".into(),
            asset_id: "system-01".into(),
            component_id: "pcs-01".into(),
            subsystem: ReliabilitySubsystem::PowerConversion,
            event_class: class,
            outage_class: outage,
            failure_mode: "synthetic-control-fault".into(),
            detection_source: DetectionSource::LocalController,
            onset_tai: interval(onset_ns, onset_ns + second),
            detected_tai: Some(interval(onset_ns + second, onset_ns + 2 * second)),
            contained_tai: None,
            restored_tai: if outage == OutageClass::Unplanned {
                Some(interval(onset_ns + 7_200 * second, onset_ns + 7_201 * second))
            } else { None },
            operating_mode: "islanded".into(),
            power_kw_at_onset: 420.0,
            voltage_v_at_onset: Some(400.0),
            current_a_at_onset: Some(1050.0),
            state_of_charge_fraction: Some(0.65),
            temperature_c: Some(31.0),
            unserved_critical_service_wh: if outage == OutageClass::Unplanned { 5_000.0 } else { 0.0 },
            critical_service_interruption_seconds: if outage == OutageClass::Unplanned { 7_200.0 } else { 0.0 },
            reserve_shortfall_wh: 250.0,
            severity: SeverityProfile {
                personnel_safety: Severity::None,
                fire_thermal: Severity::None,
                electrical: Severity::Moderate,
                environmental: Severity::None,
                service_interruption: if outage == OutageClass::Unplanned { Severity::Moderate } else { Severity::None },
                equipment_damage: Severity::Minor,
                regulatory_reportability: Severity::None,
            },
            root_cause_confidence: RootCauseConfidence::Probable,
            common_cause_group_id: Some("common-control-power-01".into()),
            replacement_component_id: None,
            repair_labor_hours: if outage == OutageClass::Unplanned { 4.0 } else { 0.0 },
            requalification_hours: if outage == OutageClass::Unplanned { 1.0 } else { 0.0 },
            topology_sha256: h('a'),
            configuration_sha256: h('b'),
            evidence_status: EvidenceStatus::Current,
            evidence_class: LifecycleEvidenceClass::SyntheticFixture,
            timebase_evidence_status: EvidenceStatus::Current,
            timebase_evidence_class: LifecycleEvidenceClass::SyntheticFixture,
            timebase_evidence_reference: "synthetic://timebase-event".into(),
            root_cause_evidence_reference: "synthetic://cause".into(),
            evidence_reference: format!("synthetic://event/{id}"),
        }
    }

    fn input() -> ReliabilityAccountingInput {
        let day = 86_400_000_000_000_i128;
        ReliabilityAccountingInput {
            candidate_id: "synthetic-candidate".into(),
            comparator_set_id: "synthetic-set".into(),
            evaluation_time_tai: interval(1_000_000_000 + 29 * day, 1_000_000_000 + 29 * day),
            minimum_telemetry_coverage_fraction: 0.95,
            event_history_complete: true,
            event_history_evidence_status: EvidenceStatus::Current,
            event_history_evidence_class: LifecycleEvidenceClass::SyntheticFixture,
            event_history_evidence_reference: "synthetic://event-history".into(),
            exposure_windows: vec![window()],
            events: vec![
                event("event-1", "incident-1", EventClass::ConfirmedFailure, OutageClass::Unplanned, 1_000_000_000 + 10 * day),
                event("event-2", "incident-1", EventClass::ConfirmedFailure, OutageClass::None, 1_000_000_000 + 10 * day + 3_600_000_000_000),
                event("event-3", "incident-2", EventClass::NearMiss, OutageClass::None, 1_000_000_000 + 20 * day),
                event("event-4", "incident-3", EventClass::Precursor, OutageClass::None, 1_000_000_000 + 25 * day),
            ],
        }
    }

    #[test]
    fn aggregates_multiple_exposure_denominators_without_collapsing_them() {
        let report = evaluate_reliability_accounting(&input()).unwrap();
        assert_eq!(report.confirmed_failure_incident_count, 1);
        assert_eq!(report.near_miss_incident_count, 1);
        assert_eq!(report.precursor_incident_count, 1);
        assert_eq!(report.total_dispatch_attempts, 500);
        assert_eq!(report.total_source_transfer_attempts, 20);
        assert_eq!(report.total_energized_asset_hours, 720.0);
        assert_eq!(report.total_installed_mw_years_lower_bound, 720.0 / HOURS_PER_YEAR);
        assert_eq!(report.failure_rates.len(), 7);
    }

    #[test]
    fn counts_multi_record_incident_once_but_preserves_event_records() {
        let report = evaluate_reliability_accounting(&input()).unwrap();
        assert_eq!(report.event_record_count, 4);
        assert_eq!(report.confirmed_failure_incident_count, 1);
    }

    #[test]
    fn common_cause_groups_are_reported_separately() {
        let report = evaluate_reliability_accounting(&input()).unwrap();
        assert_eq!(report.common_cause_group_count, 1);
    }

    #[test]
    fn outage_union_bounds_avoid_double_counting_overlapping_repairs() {
        let mut value = input();
        let mut second = event("event-5", "incident-4", EventClass::ConfirmedFailure, OutageClass::Unplanned,
            1_000_000_000 + 10 * 86_400_000_000_000 + 3_600_000_000_000);
        second.restored_tai = Some(interval(
            1_000_000_000 + 10 * 86_400_000_000_000 + 9_000_000_000_000,
            1_000_000_000 + 10 * 86_400_000_000_000 + 9_001_000_000_000,
        ));
        value.events.push(second);
        let report = evaluate_reliability_accounting(&value).unwrap();
        assert!(report.unplanned_asset_unavailable_hours_upper < 4.0);
    }

    #[test]
    fn overlapping_exposure_windows_for_one_population_are_rejected() {
        let mut value = input();
        let mut second = window();
        second.exposure_window_id = "window-2".into();
        second.start_tai = interval(1_000_000_000 + 20 * 86_400_000_000_000, 1_000_000_000 + 20 * 86_400_000_000_000);
        value.exposure_windows.push(second);
        assert!(evaluate_reliability_accounting(&value).is_err());
    }

    #[test]
    fn event_outside_exposure_or_on_wrong_configuration_is_rejected() {
        let mut value = input();
        value.events[0].configuration_sha256 = h('d');
        assert!(evaluate_reliability_accounting(&value).is_err());
        value = input();
        value.events[0].onset_tai = interval(1_000_000_000 + 40 * 86_400_000_000_000, 1_000_000_000 + 40 * 86_400_000_000_000);
        assert!(evaluate_reliability_accounting(&value).is_err());
    }

    #[test]
    fn same_incident_cannot_be_classified_as_failure_and_near_miss() {
        let mut value = input();
        value.events[2].incident_group_id = "incident-1".into();
        assert!(evaluate_reliability_accounting(&value).is_err());
    }

    #[test]
    fn incomplete_history_or_assumed_evidence_blocks_physical_claim() {
        let mut value = input();
        value.event_history_complete = false;
        let report = evaluate_reliability_accounting(&value).unwrap();
        assert!(!report.physical_reliability_claim_eligible);
        value = input();
        value.events[0].evidence_class = LifecycleEvidenceClass::Assumption;
        let report = evaluate_reliability_accounting(&value).unwrap();
        assert!(!report.physical_reliability_claim_eligible);
    }

    #[test]
    fn insufficient_telemetry_coverage_blocks_physical_claim() {
        let mut value = input();
        value.exposure_windows[0].telemetry_coverage_fraction = 0.50;
        let report = evaluate_reliability_accounting(&value).unwrap();
        assert!(!report.physical_reliability_claim_eligible);
    }

    #[test]
    fn unknown_severity_is_preserved_and_blocks_full_claim() {
        let mut value = input();
        value.events[0].severity.fire_thermal = Severity::NotAssessed;
        let report = evaluate_reliability_accounting(&value).unwrap();
        assert!(!report.severity_assessment_complete);
        assert!(!report.physical_reliability_claim_eligible);
        assert!(report.severity_counts.iter().any(|entry| entry.severity == Severity::NotAssessed));
    }

    #[test]
    fn unplanned_repair_duration_quantiles_are_reported_as_bounds() {
        let report = evaluate_reliability_accounting(&input()).unwrap();
        assert_eq!(report.unplanned_repair_duration_quantiles.sample_count, 1);
        assert!(report.unplanned_repair_duration_quantiles.p50_upper_hours.unwrap()
            >= report.unplanned_repair_duration_quantiles.p50_lower_hours.unwrap());
    }

    #[test]
    fn zero_denominator_yields_null_rate_not_infinite_value() {
        let mut value = input();
        value.exposure_windows[0].dispatch_attempts = 0;
        let report = evaluate_reliability_accounting(&value).unwrap();
        let dispatch_rate = report.failure_rates.iter().find(|x| x.metric_id == "confirmed_failures_per_10000_dispatch_attempts").unwrap();
        assert_eq!(dispatch_rate.rate, None);
    }

    #[test]
    fn synthetic_fixture_never_becomes_physical_reliability_claim() {
        let report = evaluate_reliability_accounting(&input()).unwrap();
        assert!(!report.physical_reliability_claim_eligible);
        assert!(report.claim_ceiling().contains("NOT_FAILURE_PROBABILITY"));
    }

    #[test]
    fn open_outage_contributes_elapsed_unavailability_to_evaluation_time() {
        let mut value = input();
        value.events[0].restored_tai = None;
        let report = evaluate_reliability_accounting(&value).unwrap();
        assert_eq!(report.open_outage_incident_count, 1);
        assert!(report.unplanned_asset_unavailable_hours_upper > 400.0);
    }

    #[test]
    fn rejects_configuration_or_site_profile_mixing_in_one_cohort() {
        let mut value = input();
        let mut next = window();
        next.exposure_window_id = "window-2".into();
        next.asset_id = "system-02".into();
        next.configuration_sha256 = h('d');
        value.exposure_windows.push(next);
        assert!(evaluate_reliability_accounting(&value).is_err());
    }

    #[test]
    fn overlapping_outage_records_are_unioned_per_asset() {
        let mut value = input();
        let second = event("event-5", "incident-4", EventClass::ConfirmedFailure, OutageClass::Unplanned,
            1_000_000_000 + 10 * 86_400_000_000_000 + 3_600_000_000_000);
        value.events.push(second);
        let report = evaluate_reliability_accounting(&value).unwrap();
        assert!(report.unplanned_asset_unavailable_hours_upper < 4.0);
        assert!(report.unplanned_asset_unavailable_hours_lower < report.unplanned_asset_unavailable_hours_upper);
    }

    #[test]
    fn rejects_lifecycle_timestamps_provably_out_of_order() {
        let mut value = input();
        value.events[0].detected_tai = Some(interval(
            value.events[0].onset_tai.lower_bound_ns - 10_000_000_000,
            value.events[0].onset_tai.lower_bound_ns - 9_000_000_000,
        ));
        assert!(evaluate_reliability_accounting(&value).is_err());
    }

    #[test]
    fn rejects_mixed_system_and_component_exposure_scopes() {
        let mut value = input();
        let mut next = window();
        next.exposure_window_id = "window-2".into();
        next.asset_id = "pcs-01".into();
        next.asset_scope = ExposureAssetScope::ComponentAsset;
        value.exposure_windows.push(next);
        assert!(evaluate_reliability_accounting(&value).is_err());
    }

    #[test]
    fn installed_mw_years_use_calendar_exposure_not_only_energized_hours() {
        let mut value = input();
        value.exposure_windows[0].energized_hours = 360.0;
        let report = evaluate_reliability_accounting(&value).unwrap();
        let expected = 1.0 * 720.0 / HOURS_PER_YEAR;
        assert!((report.total_installed_mw_years_lower_bound - expected).abs() < 1.0e-12);
        assert_eq!(report.total_energized_asset_hours, 360.0);
    }

    #[test]
    fn unknown_severity_is_not_overwritten_by_a_later_assessed_event_in_same_incident() {
        let mut value = input();
        value.events[0].severity.fire_thermal = Severity::NotAssessed;
        value.events[1].severity.fire_thermal = Severity::None;
        let report = evaluate_reliability_accounting(&value).unwrap();
        assert!(!report.severity_assessment_complete);
        assert!(report.severity_counts.iter().any(|entry|
            entry.dimension == SeverityDimension::FireThermal
                && entry.severity == Severity::NotAssessed
                && entry.incident_count == 1
        ));
    }

    #[test]
    fn contradictory_root_cause_or_outage_class_in_one_incident_is_rejected() {
        let mut value = input();
        value.events[1].root_cause_confidence = RootCauseConfidence::Confirmed;
        assert!(evaluate_reliability_accounting(&value).is_err());
        value = input();
        value.events[1].outage_class = OutageClass::Planned;
        assert!(evaluate_reliability_accounting(&value).is_err());
    }

}
