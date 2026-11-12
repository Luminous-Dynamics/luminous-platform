use crate::capability_envelope::EvidenceStatus;
use crate::AuditError;

const COST_CATEGORY_COUNT: usize = 10;
const ENERGY_TOLERANCE_WH: f64 = 1.0e-9;
const TIME_TOLERANCE_S: f64 = 1.0e-6;
const MAX_MISSION_RECORDS: usize = 100_000;
const MAX_COST_ENTRIES: usize = 100_000;
const MAX_REPAIR_EVENTS: usize = 100_000;
const MAX_STORAGE_OBSERVATIONS: usize = 100_000;

/// Monetary categories are additive, nominal costs in the scenario's declared
/// currency and minor-unit scale. No currency conversion or discounting occurs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleCostCategory {
    InitialCapital,
    Fuel,
    PurchasedEnergy,
    CoolingAndAuxiliaries,
    MaintenanceParts,
    MaintenanceLabor,
    Replacements,
    ServiceLogistics,
    Disposal,
    Other,
}

impl LifecycleCostCategory {
    const fn index(self) -> usize {
        match self {
            Self::InitialCapital => 0,
            Self::Fuel => 1,
            Self::PurchasedEnergy => 2,
            Self::CoolingAndAuxiliaries => 3,
            Self::MaintenanceParts => 4,
            Self::MaintenanceLabor => 5,
            Self::Replacements => 6,
            Self::ServiceLogistics => 7,
            Self::Disposal => 8,
            Self::Other => 9,
        }
    }
}

const COST_CATEGORIES: [LifecycleCostCategory; COST_CATEGORY_COUNT] = [
    LifecycleCostCategory::InitialCapital,
    LifecycleCostCategory::Fuel,
    LifecycleCostCategory::PurchasedEnergy,
    LifecycleCostCategory::CoolingAndAuxiliaries,
    LifecycleCostCategory::MaintenanceParts,
    LifecycleCostCategory::MaintenanceLabor,
    LifecycleCostCategory::Replacements,
    LifecycleCostCategory::ServiceLogistics,
    LifecycleCostCategory::Disposal,
    LifecycleCostCategory::Other,
];

/// Distinguishes actual observations from estimates and synthetic fixtures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleEvidenceClass {
    IndependentPhysicalMeasurement,
    InvoiceOrReceipt,
    UtilityBill,
    PayrollOrWorkOrder,
    VendorReport,
    SupplierQuote,
    AnalyticalModel,
    SyntheticFixture,
    Assumption,
}

impl LifecycleEvidenceClass {
    const fn is_actual_cost_record(self) -> bool {
        matches!(
            self,
            Self::InvoiceOrReceipt | Self::UtilityBill | Self::PayrollOrWorkOrder
        )
    }

    const fn is_independent_physical(self) -> bool {
        matches!(self, Self::IndependentPhysicalMeasurement)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LifecycleCostEntry {
    pub entry_id: String,
    /// Stable transaction/allocation identity; duplicate economic events are rejected.
    pub economic_event_id: String,
    pub category: LifecycleCostCategory,
    /// Integer minor currency units (e.g. cents). No floating money arithmetic.
    pub amount_minor_units: u64,
    pub incurred_at_epoch_s: u64,
    pub evidence_status: EvidenceStatus,
    pub evidence_class: LifecycleEvidenceClass,
    pub evidence_reference: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LifecycleMissionRecord {
    pub mission_id: String,
    pub mission_profile_sha256: String,
    pub terminal_state_obligation_sha256: String,
    pub qualified_configuration_sha256: String,
    pub observed_configuration_sha256: String,
    pub qualification_valid_until_epoch_s: u64,
    /// Study-window-relative mission start and duration.
    pub start_s: f64,
    pub duration_s: f64,
    pub required_critical_service_wh: f64,
    pub delivered_critical_service_wh: f64,
    pub maximum_unserved_critical_energy_wh: f64,
    /// Upstream mission-energy screen result from #422.
    pub energy_screen_pass: bool,
    /// Upstream power-duration event screen result from #425, if applicable.
    pub event_screen_pass: bool,
    /// Upstream end-state / reserve obligation result.
    pub terminal_state_pass: bool,
    pub evidence_status: EvidenceStatus,
    pub evidence_class: LifecycleEvidenceClass,
    pub evidence_reference: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LifecycleRepairEvent {
    pub repair_id: String,
    pub component_id: String,
    pub replacement_component_id: Option<String>,
    pub pre_repair_configuration_sha256: String,
    pub post_repair_configuration_sha256: String,
    /// Study-window-relative interval during which the system cannot deliver
    /// its declared service. Overlapping intervals are unioned, not summed.
    pub start_s: f64,
    pub system_unavailable_duration_s: f64,
    pub repair_labor_hours: f64,
    pub requalification_hours: f64,
    pub replacement_required: bool,
    pub evidence_status: EvidenceStatus,
    pub evidence_class: LifecycleEvidenceClass,
    pub evidence_reference: String,
}

/// Observed storage-health point. The accountant never fits or extrapolates an
/// ageing model. It only computes first-to-last retention for the same physical
/// component and exact unchanged configuration using independent measurements.
#[derive(Debug, Clone, PartialEq)]
pub struct StorageHealthObservation {
    pub observation_id: String,
    pub component_id: String,
    pub observed_at_epoch_s: u64,
    pub qualified_configuration_sha256: String,
    pub observed_configuration_sha256: String,
    pub usable_capacity_wh: f64,
    pub capacity_uncertainty_bound_wh: f64,
    pub maximum_discharge_power_w: f64,
    pub power_uncertainty_bound_w: f64,
    pub cumulative_energy_throughput_wh: f64,
    pub calendar_age_hours: f64,
    pub equivalent_full_cycles: f64,
    pub state_of_charge_fraction: f64,
    pub state_of_health_fraction: f64,
    pub temperature_c: f64,
    pub evidence_status: EvidenceStatus,
    pub calibration_status: EvidenceStatus,
    pub sensor_status: EvidenceStatus,
    pub evidence_class: LifecycleEvidenceClass,
    pub operating_condition_reference: String,
    pub timebase_reference: String,
    pub evidence_reference: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LifecycleScenarioInput {
    pub scenario_id: String,
    pub comparator_set_id: String,
    pub service_boundary_id: String,
    pub site_profile_sha256: String,
    pub qualified_configuration_sha256: String,
    pub observed_configuration_sha256: String,
    pub qualification_valid_until_epoch_s: u64,
    pub evaluation_time_epoch_s: u64,
    pub currency_code: String,
    /// Number of decimal minor-unit places (0..=3).
    pub minor_unit_scale: u8,
    pub study_start_epoch_s: u64,
    pub study_end_epoch_s: u64,
    /// Cumulative scheduled service time in seconds. Mission and outage
    /// intervals use this operating-time axis, not wall-clock elapsed time.
    pub operating_window_s: f64,
    pub storage_in_scope: bool,
    pub mission_records: Vec<LifecycleMissionRecord>,
    pub cost_entries: Vec<LifecycleCostEntry>,
    pub repair_history_complete: bool,
    pub repair_history_evidence_status: EvidenceStatus,
    pub repair_history_evidence_class: LifecycleEvidenceClass,
    pub repair_history_evidence_reference: String,
    pub repair_events: Vec<LifecycleRepairEvent>,
    pub storage_health_observations: Vec<StorageHealthObservation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LifecycleCostCategoryTotal {
    pub category: LifecycleCostCategory,
    pub amount_minor_units: u128,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StorageHealthTrajectory {
    pub component_id: String,
    pub qualified_configuration_sha256: String,
    pub first_observation_epoch_s: u64,
    pub last_observation_epoch_s: u64,
    pub first_usable_capacity_lower_bound_wh: f64,
    pub last_usable_capacity_lower_bound_wh: f64,
    pub usable_capacity_retention_fraction: f64,
    pub first_discharge_power_lower_bound_w: f64,
    pub last_discharge_power_lower_bound_w: f64,
    pub discharge_power_retention_fraction: f64,
    pub energy_throughput_delta_wh: f64,
    pub calendar_age_delta_hours: f64,
    pub equivalent_full_cycle_delta: f64,
    pub first_state_of_health_fraction: f64,
    pub last_state_of_health_fraction: f64,
    pub first_temperature_c: f64,
    pub last_temperature_c: f64,
    pub first_timebase_reference: String,
    pub last_timebase_reference: String,
    pub first_evidence_reference: String,
    pub last_evidence_reference: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LifecycleScenarioReport {
    pub scenario_id: String,
    pub comparator_set_id: String,
    pub currency_code: String,
    pub minor_unit_scale: u8,
    pub total_cost_minor_units: u128,
    pub cost_by_category: Vec<LifecycleCostCategoryTotal>,
    pub total_required_critical_service_wh: f64,
    pub total_delivered_critical_service_wh: f64,
    pub total_unserved_critical_service_wh: f64,
    pub mission_count: usize,
    pub completed_mission_count: usize,
    pub critical_service_pass_rate_percent: f64,
    /// Minor currency units per delivered critical kWh.
    pub cost_per_delivered_critical_kwh_minor_units: Option<f64>,
    /// Minor currency units per completed mission.
    pub cost_per_completed_mission_minor_units: Option<f64>,
    pub system_unavailable_hours: f64,
    pub availability_percent: f64,
    pub repair_labor_hours: f64,
    pub requalification_hours: f64,
    pub replacement_count: usize,
    pub cost_evidence_current: bool,
    pub cost_evidence_actual: bool,
    pub mission_evidence_current: bool,
    pub mission_evidence_physical: bool,
    pub repair_evidence_current: bool,
    pub repair_evidence_physical: bool,
    pub storage_health_evidence_complete: bool,
    pub storage_health_trajectories: Vec<StorageHealthTrajectory>,
    pub configuration_eligible: bool,
    pub lifecycle_accounting_eligible: bool,
    pub physical_lifecycle_claim_eligible: bool,
}

impl LifecycleScenarioReport {
    /// Nominal cost/service and observed retention arithmetic only; not an
    /// LCOE forecast, battery-life prediction, RUL estimate or product claim.
    pub const fn claim_ceiling(&self) -> &'static str {
        "LIFECYCLE_SERVICE_ACCOUNTING_ONLY_NOT_REMAINING_LIFE_OR_PHYSICAL_QUALIFICATION"
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LifecycleComparisonReport {
    pub reference: LifecycleScenarioReport,
    pub candidate: LifecycleScenarioReport,
    pub matched_service_contract: bool,
    pub comparison_claim_eligible: bool,
    /// Negative means lower candidate cost per delivered critical kWh.
    pub candidate_cost_change_percent: Option<f64>,
    /// Negative means fewer candidate system-unavailable hours.
    pub candidate_unavailability_change_hours: f64,
    /// Negative means fewer candidate replacements.
    pub candidate_replacement_count_change: i64,
}

impl LifecycleComparisonReport {
    pub const fn claim_ceiling(&self) -> &'static str {
        "MATCHED_LIFECYCLE_ACCOUNTING_ONLY_NOT_LIFETIME_FORECAST_OR_SOTA_CLAIM"
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

fn currency_code_valid(value: &str) -> bool {
    value.len() == 3 && value.bytes().all(|byte| byte.is_ascii_uppercase())
}

fn validate_input(input: &LifecycleScenarioInput) -> Result<(), AuditError> {
    if !nonblank(&input.scenario_id)
        || !nonblank(&input.comparator_set_id)
        || !nonblank(&input.service_boundary_id)
    {
        return Err(AuditError(
            "lifecycle scenario, comparator set and service boundary must be non-empty"
                .to_string(),
        ));
    }
    if !valid_sha256(&input.site_profile_sha256)
        || !valid_sha256(&input.qualified_configuration_sha256)
        || !valid_sha256(&input.observed_configuration_sha256)
    {
        return Err(AuditError(
            "lifecycle site and product configuration digests must be SHA-256".to_string(),
        ));
    }
    if !currency_code_valid(&input.currency_code) || input.minor_unit_scale > 3 {
        return Err(AuditError(
            "currency code must be three uppercase ASCII letters and scale must be 0..=3"
                .to_string(),
        ));
    }
    if input.study_end_epoch_s <= input.study_start_epoch_s {
        return Err(AuditError(
            "lifecycle study end must be later than study start".to_string(),
        ));
    }
    if input.mission_records.is_empty()
        || input.mission_records.len() > MAX_MISSION_RECORDS
        || input.cost_entries.is_empty()
        || input.cost_entries.len() > MAX_COST_ENTRIES
        || input.repair_events.len() > MAX_REPAIR_EVENTS
        || input.storage_health_observations.len() > MAX_STORAGE_OBSERVATIONS
    {
        return Err(AuditError(
            "lifecycle record counts are empty or exceed supported bounds".to_string(),
        ));
    }
    if !input.storage_in_scope && !input.storage_health_observations.is_empty() {
        return Err(AuditError(
            "storage health observations supplied while storage_in_scope is false".to_string(),
        ));
    }

    let study_window_s = (input.study_end_epoch_s - input.study_start_epoch_s) as f64;
    if !study_window_s.is_finite()
        || study_window_s <= 0.0
        || !input.operating_window_s.is_finite()
        || input.operating_window_s <= 0.0
        || input.operating_window_s > study_window_s + TIME_TOLERANCE_S
    {
        return Err(AuditError("lifecycle study/operating window is invalid".to_string()));
    }

    let mut mission_ids: Vec<&str> = Vec::with_capacity(input.mission_records.len());
    for mission in &input.mission_records {
        if !nonblank(&mission.mission_id) || !nonblank(&mission.evidence_reference) {
            return Err(AuditError(
                "lifecycle mission identity and evidence reference must be non-empty"
                    .to_string(),
            ));
        }
        if mission_ids.iter().any(|id| *id == mission.mission_id.as_str()) {
            return Err(AuditError(format!(
                "duplicate lifecycle mission id {:?}",
                mission.mission_id
            )));
        }
        mission_ids.push(&mission.mission_id);
        if !valid_sha256(&mission.mission_profile_sha256)
            || !valid_sha256(&mission.terminal_state_obligation_sha256)
            || !valid_sha256(&mission.qualified_configuration_sha256)
            || !valid_sha256(&mission.observed_configuration_sha256)
        {
            return Err(AuditError(format!(
                "mission {:?} contains an invalid digest",
                mission.mission_id
            )));
        }
        if !mission.start_s.is_finite()
            || !mission.duration_s.is_finite()
            || mission.start_s < 0.0
            || mission.duration_s <= 0.0
            || !finite_nonnegative(mission.required_critical_service_wh)
            || !finite_nonnegative(mission.delivered_critical_service_wh)
            || !finite_nonnegative(mission.maximum_unserved_critical_energy_wh)
            || mission.delivered_critical_service_wh
                > mission.required_critical_service_wh + ENERGY_TOLERANCE_WH
            || mission.maximum_unserved_critical_energy_wh
                > mission.required_critical_service_wh + ENERGY_TOLERANCE_WH
        {
            return Err(AuditError(format!(
                "mission {:?} contains invalid service or timing values",
                mission.mission_id
            )));
        }
        let end_s = mission.start_s + mission.duration_s;
        if !end_s.is_finite() || end_s > input.operating_window_s + TIME_TOLERANCE_S {
            return Err(AuditError(format!(
                "mission {:?} extends beyond the lifecycle study window",
                mission.mission_id
            )));
        }
    }

    let mut cost_ids: Vec<&str> = Vec::with_capacity(input.cost_entries.len());
    let mut economic_event_ids: Vec<&str> = Vec::with_capacity(input.cost_entries.len());
    for cost in &input.cost_entries {
        if !nonblank(&cost.entry_id)
            || !nonblank(&cost.economic_event_id)
            || !nonblank(&cost.evidence_reference)
        {
            return Err(AuditError(
                "cost entry identity and evidence reference must be non-empty".to_string(),
            ));
        }
        if cost_ids.iter().any(|id| *id == cost.entry_id.as_str()) {
            return Err(AuditError(format!(
                "duplicate lifecycle cost entry id {:?}",
                cost.entry_id
            )));
        }
        cost_ids.push(&cost.entry_id);
        if economic_event_ids.iter().any(|id| *id == cost.economic_event_id.as_str()) {
            return Err(AuditError(format!(
                "duplicate lifecycle economic event id {:?}",
                cost.economic_event_id
            )));
        }
        economic_event_ids.push(&cost.economic_event_id);
        if cost.incurred_at_epoch_s < input.study_start_epoch_s
            || cost.incurred_at_epoch_s > input.study_end_epoch_s
        {
            return Err(AuditError(format!(
                "cost entry {:?} falls outside the study window",
                cost.entry_id
            )));
        }
    }

    let mut repair_ids: Vec<&str> = Vec::with_capacity(input.repair_events.len());
    if !nonblank(&input.repair_history_evidence_reference) {
        return Err(AuditError("repair-history evidence reference must be non-empty".to_string()));
    }
    for repair in &input.repair_events {
        if !nonblank(&repair.repair_id)
            || !nonblank(&repair.component_id)
            || !nonblank(&repair.evidence_reference)
            || !valid_sha256(&repair.pre_repair_configuration_sha256)
            || !valid_sha256(&repair.post_repair_configuration_sha256)
        {
            return Err(AuditError(
                "repair event identity and evidence references must be non-empty".to_string(),
            ));
        }
        if repair_ids.iter().any(|id| *id == repair.repair_id.as_str()) {
            return Err(AuditError(format!(
                "duplicate lifecycle repair event id {:?}",
                repair.repair_id
            )));
        }
        repair_ids.push(&repair.repair_id);
        match (&repair.replacement_component_id, repair.replacement_required) {
            (Some(new_component_id), true)
                if nonblank(new_component_id) && new_component_id != &repair.component_id => {}
            (None, false) => {}
            _ => {
                return Err(AuditError(format!(
                    "repair event {:?} has inconsistent replacement identity",
                    repair.repair_id
                )));
            }
        }
        if !repair.start_s.is_finite()
            || !finite_nonnegative(repair.system_unavailable_duration_s)
            || !finite_nonnegative(repair.repair_labor_hours)
            || !finite_nonnegative(repair.requalification_hours)
            || repair.start_s < 0.0
        {
            return Err(AuditError(format!(
                "repair event {:?} contains invalid timing or labour values",
                repair.repair_id
            )));
        }
        let end_s = repair.start_s + repair.system_unavailable_duration_s;
        if !end_s.is_finite() || end_s > input.operating_window_s + TIME_TOLERANCE_S {
            return Err(AuditError(format!(
                "repair event {:?} extends beyond lifecycle operating window",
                repair.repair_id
            )));
        }
    }

    let mut observation_ids: Vec<&str> =
        Vec::with_capacity(input.storage_health_observations.len());
    for observation in &input.storage_health_observations {
        if !nonblank(&observation.observation_id)
            || !nonblank(&observation.component_id)
            || !nonblank(&observation.operating_condition_reference)
            || !nonblank(&observation.timebase_reference)
            || !nonblank(&observation.evidence_reference)
        {
            return Err(AuditError(
                "storage observation identity and evidence references must be non-empty"
                    .to_string(),
            ));
        }
        if observation_ids
            .iter()
            .any(|id| *id == observation.observation_id.as_str())
        {
            return Err(AuditError(format!(
                "duplicate storage observation id {:?}",
                observation.observation_id
            )));
        }
        observation_ids.push(&observation.observation_id);
        if !valid_sha256(&observation.qualified_configuration_sha256)
            || !valid_sha256(&observation.observed_configuration_sha256)
        {
            return Err(AuditError(format!(
                "storage observation {:?} has an invalid configuration digest",
                observation.observation_id
            )));
        }
        if observation.observed_at_epoch_s < input.study_start_epoch_s
            || observation.observed_at_epoch_s > input.study_end_epoch_s
        {
            return Err(AuditError(format!(
                "storage observation {:?} falls outside study window",
                observation.observation_id
            )));
        }
        if !finite_nonnegative(observation.usable_capacity_wh)
            || !finite_nonnegative(observation.capacity_uncertainty_bound_wh)
            || observation.capacity_uncertainty_bound_wh > observation.usable_capacity_wh
            || !finite_nonnegative(observation.maximum_discharge_power_w)
            || !finite_nonnegative(observation.power_uncertainty_bound_w)
            || observation.power_uncertainty_bound_w > observation.maximum_discharge_power_w
            || !finite_nonnegative(observation.cumulative_energy_throughput_wh)
            || !finite_nonnegative(observation.calendar_age_hours)
            || !finite_nonnegative(observation.equivalent_full_cycles)
            || !observation.state_of_charge_fraction.is_finite()
            || !(0.0..=1.0).contains(&observation.state_of_charge_fraction)
            || !observation.state_of_health_fraction.is_finite()
            || !(0.0..=1.0).contains(&observation.state_of_health_fraction)
            || !observation.temperature_c.is_finite()
        {
            return Err(AuditError(format!(
                "storage observation {:?} contains invalid health or uncertainty values",
                observation.observation_id
            )));
        }
    }
    Ok(())
}

fn storage_health_trajectories(
    observations: &[StorageHealthObservation],
    storage_in_scope: bool,
) -> Result<(bool, Vec<StorageHealthTrajectory>), AuditError> {
    if !storage_in_scope {
        return Ok((true, Vec::new()));
    }
    if observations.is_empty() {
        return Ok((false, Vec::new()));
    }

    let mut component_ids: Vec<&str> =
        observations.iter().map(|item| item.component_id.as_str()).collect();
    component_ids.sort_unstable();
    component_ids.dedup();

    let mut all_complete = true;
    let mut trajectories = Vec::new();

    for component_id in component_ids {
        let mut points: Vec<&StorageHealthObservation> = observations
            .iter()
            .filter(|item| item.component_id == component_id)
            .collect();
        points.sort_by(|left, right| left.observed_at_epoch_s.cmp(&right.observed_at_epoch_s));

        let mut group_valid = points.len() >= 2;
        let first = points[0];
        let mut previous = first;

        for point in points.iter().skip(1) {
            if point.observed_at_epoch_s <= previous.observed_at_epoch_s
                || point.cumulative_energy_throughput_wh
                    < previous.cumulative_energy_throughput_wh
                || point.calendar_age_hours < previous.calendar_age_hours
                || point.equivalent_full_cycles < previous.equivalent_full_cycles
            {
                return Err(AuditError(format!(
                    "storage health observations for component {:?} are not chronological/monotonic",
                    component_id
                )));
            }
            previous = *point;
        }

        let configuration = &first.qualified_configuration_sha256;
        for point in &points {
            if point.qualified_configuration_sha256 != point.observed_configuration_sha256
                || &point.qualified_configuration_sha256 != configuration
                || point.evidence_status != EvidenceStatus::Current
                || point.calibration_status != EvidenceStatus::Current
                || point.sensor_status != EvidenceStatus::Current
                || !point.evidence_class.is_independent_physical()
            {
                group_valid = false;
            }
        }

        let last = points[points.len() - 1];
        let first_capacity_lower =
            first.usable_capacity_wh - first.capacity_uncertainty_bound_wh;
        let last_capacity_lower =
            last.usable_capacity_wh - last.capacity_uncertainty_bound_wh;
        let first_power_lower =
            first.maximum_discharge_power_w - first.power_uncertainty_bound_w;
        let last_power_lower =
            last.maximum_discharge_power_w - last.power_uncertainty_bound_w;

        if first_capacity_lower <= 0.0 || first_power_lower <= 0.0 {
            group_valid = false;
        }

        if group_valid {
            trajectories.push(StorageHealthTrajectory {
                component_id: component_id.to_string(),
                qualified_configuration_sha256: configuration.clone(),
                first_observation_epoch_s: first.observed_at_epoch_s,
                last_observation_epoch_s: last.observed_at_epoch_s,
                first_usable_capacity_lower_bound_wh: first_capacity_lower,
                last_usable_capacity_lower_bound_wh: last_capacity_lower,
                usable_capacity_retention_fraction: last_capacity_lower / first_capacity_lower,
                first_discharge_power_lower_bound_w: first_power_lower,
                last_discharge_power_lower_bound_w: last_power_lower,
                discharge_power_retention_fraction: last_power_lower / first_power_lower,
                energy_throughput_delta_wh: last.cumulative_energy_throughput_wh
                    - first.cumulative_energy_throughput_wh,
                    calendar_age_delta_hours: last.calendar_age_hours - first.calendar_age_hours,
                equivalent_full_cycle_delta: last.equivalent_full_cycles
                    - first.equivalent_full_cycles,
                first_state_of_health_fraction: first.state_of_health_fraction,
                last_state_of_health_fraction: last.state_of_health_fraction,
                first_temperature_c: first.temperature_c,
                last_temperature_c: last.temperature_c,
                first_timebase_reference: first.timebase_reference.clone(),
                last_timebase_reference: last.timebase_reference.clone(),
                first_evidence_reference: first.evidence_reference.clone(),
                last_evidence_reference: last.evidence_reference.clone(),
            });
        } else {
            all_complete = false;
        }
    }

    Ok((all_complete, trajectories))
}

fn system_unavailable_union_seconds(
    repair_events: &[LifecycleRepairEvent],
) -> Result<f64, AuditError> {
    let mut intervals: Vec<(f64, f64)> = repair_events
        .iter()
        .filter(|event| event.system_unavailable_duration_s > 0.0)
        .map(|event| {
            (
                event.start_s,
                event.start_s + event.system_unavailable_duration_s,
            )
        })
        .collect();

    intervals.sort_by(|left, right| left.0.total_cmp(&right.0));
    if intervals.is_empty() {
        return Ok(0.0);
    }

    let mut total = 0.0_f64;
    let mut current_start = intervals[0].0;
    let mut current_end = intervals[0].1;

    for (start, end) in intervals.into_iter().skip(1) {
        if start <= current_end + TIME_TOLERANCE_S {
            current_end = current_end.max(end);
        } else {
            total += current_end - current_start;
            current_start = start;
            current_end = end;
        }
    }
    total += current_end - current_start;

    if !total.is_finite() || total < 0.0 {
        return Err(AuditError("repair downtime union is invalid".to_string()));
    }
    Ok(total)
}

/// Evaluate nominal lifecycle cost and repeated mission/service history.
///
/// This function does not predict battery ageing, remaining useful life, or
/// future replacement intervals. Storage retention is derived only from
/// repeated, current, calibrated, independent measurements of the same
/// component and exact unchanged configuration.
pub fn evaluate_lifecycle_accounting(
    input: &LifecycleScenarioInput,
) -> Result<LifecycleScenarioReport, AuditError> {
    validate_input(input)?;

    let mut cost_by_category = vec![
        LifecycleCostCategoryTotal {
            category: COST_CATEGORIES[0],
            amount_minor_units: 0,
        };
        COST_CATEGORY_COUNT
    ];
    let mut total_cost_minor_units = 0_u128;
    let mut cost_evidence_current = true;
    let mut cost_evidence_actual = true;

    for entry in &input.cost_entries {
        let amount = u128::from(entry.amount_minor_units);
        total_cost_minor_units = total_cost_minor_units
            .checked_add(amount)
            .ok_or_else(|| AuditError("lifecycle total cost overflowed".to_string()))?;
        let index = entry.category.index();
        cost_by_category[index].amount_minor_units = cost_by_category[index]
            .amount_minor_units
            .checked_add(amount)
            .ok_or_else(|| AuditError("lifecycle category cost overflowed".to_string()))?;
        if entry.evidence_status != EvidenceStatus::Current {
            cost_evidence_current = false;
        }
        if !entry.evidence_class.is_actual_cost_record() {
            cost_evidence_actual = false;
        }
    }

    let mut total_required_critical_service_wh = 0.0_f64;
    let mut total_delivered_critical_service_wh = 0.0_f64;
    let mut total_unserved_critical_service_wh = 0.0_f64;
    let mut completed_mission_count = 0_usize;
    let mut mission_evidence_current = true;
    let mut mission_evidence_physical = true;

    for mission in &input.mission_records {
        let unserved = (mission.required_critical_service_wh
            - mission.delivered_critical_service_wh)
            .max(0.0);
        let mission_pass = mission.energy_screen_pass
            && mission.event_screen_pass
            && mission.terminal_state_pass
            && unserved
                <= mission.maximum_unserved_critical_energy_wh + ENERGY_TOLERANCE_WH;
        if mission_pass {
            completed_mission_count += 1;
        }
        if mission.evidence_status != EvidenceStatus::Current
            || mission.qualified_configuration_sha256 != mission.observed_configuration_sha256
            || mission.qualification_valid_until_epoch_s <= input.evaluation_time_epoch_s
        {
            mission_evidence_current = false;
        }
        if !mission.evidence_class.is_independent_physical() {
            mission_evidence_physical = false;
        }

        total_required_critical_service_wh += mission.required_critical_service_wh;
        total_delivered_critical_service_wh += mission.delivered_critical_service_wh;
        total_unserved_critical_service_wh += unserved;
    }

    if !total_required_critical_service_wh.is_finite()
        || !total_delivered_critical_service_wh.is_finite()
        || !total_unserved_critical_service_wh.is_finite()
    {
        return Err(AuditError(
            "lifecycle critical-service totals overflowed".to_string(),
        ));
    }

    let mut repair_evidence_current = input.repair_history_complete
        && input.repair_history_evidence_status == EvidenceStatus::Current;
    let mut repair_evidence_physical = input.repair_history_complete
        && input.repair_history_evidence_status == EvidenceStatus::Current
        && input.repair_history_evidence_class.is_independent_physical();
    let mut repair_labor_hours = 0.0_f64;
    let mut requalification_hours = 0.0_f64;
    let mut replacement_count = 0_usize;

    for repair in &input.repair_events {
        repair_labor_hours += repair.repair_labor_hours;
        requalification_hours += repair.requalification_hours;
        if repair.replacement_required {
            replacement_count += 1;
        }
        if repair.evidence_status != EvidenceStatus::Current {
            repair_evidence_current = false;
        }
        if !repair.evidence_class.is_independent_physical() {
            repair_evidence_physical = false;
        }
    }

    if !repair_labor_hours.is_finite() || !requalification_hours.is_finite() {
        return Err(AuditError("repair labour totals overflowed".to_string()));
    }

    let operating_window_s = input.operating_window_s;
    let unavailable_s = system_unavailable_union_seconds(&input.repair_events)?;
    if unavailable_s > operating_window_s + TIME_TOLERANCE_S {
        return Err(AuditError(
            "system-unavailable intervals exceed the study window".to_string(),
        ));
    }
    let system_unavailable_hours = unavailable_s / 3600.0;
    let availability_percent =
        100.0 * (operating_window_s - unavailable_s).max(0.0) / operating_window_s;

    let (storage_health_evidence_complete, storage_health_trajectories) =
        storage_health_trajectories(
            &input.storage_health_observations,
            input.storage_in_scope,
        )?;

    let mission_count = input.mission_records.len();
    let critical_service_pass_rate_percent =
        100.0 * completed_mission_count as f64 / mission_count as f64;
    let cost_per_delivered_critical_kwh_minor_units =
        if total_delivered_critical_service_wh > ENERGY_TOLERANCE_WH {
            Some(
                total_cost_minor_units as f64 * 1_000.0
                    / total_delivered_critical_service_wh,
            )
        } else {
            None
        };
    let cost_per_completed_mission_minor_units = if completed_mission_count > 0 {
        Some(total_cost_minor_units as f64 / completed_mission_count as f64)
    } else {
        None
    };

    let configuration_eligible =
        input.qualified_configuration_sha256 == input.observed_configuration_sha256
            && input.qualification_valid_until_epoch_s > input.evaluation_time_epoch_s;
    let lifecycle_accounting_eligible = configuration_eligible
        && cost_evidence_current
        && mission_evidence_current
        && repair_evidence_current;
    let all_missions_pass = completed_mission_count == mission_count;
    let physical_lifecycle_claim_eligible = lifecycle_accounting_eligible
        && all_missions_pass
        && cost_evidence_actual
        && mission_evidence_physical
        && repair_evidence_physical
        && storage_health_evidence_complete;

    Ok(LifecycleScenarioReport {
        scenario_id: input.scenario_id.clone(),
        comparator_set_id: input.comparator_set_id.clone(),
        currency_code: input.currency_code.clone(),
        minor_unit_scale: input.minor_unit_scale,
        total_cost_minor_units,
        cost_by_category,
        total_required_critical_service_wh,
        total_delivered_critical_service_wh,
        total_unserved_critical_service_wh,
        mission_count,
        completed_mission_count,
        critical_service_pass_rate_percent,
        cost_per_delivered_critical_kwh_minor_units,
        cost_per_completed_mission_minor_units,
        system_unavailable_hours,
        availability_percent,
        repair_labor_hours,
        requalification_hours,
        replacement_count,
        cost_evidence_current,
        cost_evidence_actual,
        mission_evidence_current,
        mission_evidence_physical,
        repair_evidence_current,
        repair_evidence_physical,
        storage_health_evidence_complete,
        storage_health_trajectories,
        configuration_eligible,
        lifecycle_accounting_eligible,
        physical_lifecycle_claim_eligible,
    })
}

fn validate_matched_service(
    reference: &LifecycleScenarioInput,
    candidate: &LifecycleScenarioInput,
) -> Result<(), AuditError> {
    if reference.comparator_set_id != candidate.comparator_set_id
        || reference.service_boundary_id != candidate.service_boundary_id
        || reference.site_profile_sha256 != candidate.site_profile_sha256
        || reference.currency_code != candidate.currency_code
        || reference.minor_unit_scale != candidate.minor_unit_scale
        || reference.study_start_epoch_s != candidate.study_start_epoch_s
        || reference.study_end_epoch_s != candidate.study_end_epoch_s
        || (reference.operating_window_s - candidate.operating_window_s).abs()
            > TIME_TOLERANCE_S
    {
        return Err(AuditError(
            "lifecycle comparator set, service boundary, site, currency or study horizon differs"
                .to_string(),
        ));
    }
    if reference.mission_records.len() != candidate.mission_records.len() {
        return Err(AuditError(
            "lifecycle scenarios have different mission record counts".to_string(),
        ));
    }

    for reference_mission in &reference.mission_records {
        let candidate_mission = candidate
            .mission_records
            .iter()
            .find(|item| item.mission_id == reference_mission.mission_id)
            .ok_or_else(|| {
                AuditError(format!(
                    "candidate is missing matched mission {:?}",
                    reference_mission.mission_id
                ))
            })?;
        if reference_mission.mission_profile_sha256 != candidate_mission.mission_profile_sha256
            || reference_mission.terminal_state_obligation_sha256
                != candidate_mission.terminal_state_obligation_sha256
            || (reference_mission.start_s - candidate_mission.start_s).abs()
                > TIME_TOLERANCE_S
            || (reference_mission.duration_s - candidate_mission.duration_s).abs()
                > TIME_TOLERANCE_S
            || (reference_mission.required_critical_service_wh
                - candidate_mission.required_critical_service_wh)
                .abs()
                > ENERGY_TOLERANCE_WH
            || (reference_mission.maximum_unserved_critical_energy_wh
                - candidate_mission.maximum_unserved_critical_energy_wh)
                .abs()
                > ENERGY_TOLERANCE_WH
        {
            return Err(AuditError(format!(
                "mission {:?} does not share the same service/state obligation",
                reference_mission.mission_id
            )));
        }
    }

    Ok(())
}

/// Compare two architectures only after exact mission profiles and terminal-state
/// obligations have been matched. The output is accounting; it never asserts
/// superiority unless both scenarios have physically supported lifecycle evidence
/// and every critical mission has passed.
pub fn compare_lifecycle_scenarios(
    reference: &LifecycleScenarioInput,
    candidate: &LifecycleScenarioInput,
) -> Result<LifecycleComparisonReport, AuditError> {
    validate_input(reference)?;
    validate_input(candidate)?;
    validate_matched_service(reference, candidate)?;

    let reference_report = evaluate_lifecycle_accounting(reference)?;
    let candidate_report = evaluate_lifecycle_accounting(candidate)?;

    let candidate_cost_change_percent = match (
        reference_report.cost_per_delivered_critical_kwh_minor_units,
        candidate_report.cost_per_delivered_critical_kwh_minor_units,
    ) {
        (Some(reference_cost), Some(candidate_cost)) if reference_cost > 0.0 => {
            Some(100.0 * (candidate_cost / reference_cost - 1.0))
        }
        _ => None,
    };

    let candidate_replacement_count_change =
        candidate_report.replacement_count as i64 - reference_report.replacement_count as i64;
    let candidate_unavailability_change_hours =
        candidate_report.system_unavailable_hours - reference_report.system_unavailable_hours;

    let comparison_claim_eligible = reference_report.physical_lifecycle_claim_eligible
        && candidate_report.physical_lifecycle_claim_eligible;

    Ok(LifecycleComparisonReport {
        reference: reference_report,
        candidate: candidate_report,
        matched_service_contract: true,
        comparison_claim_eligible,
        candidate_cost_change_percent,
        candidate_unavailability_change_hours,
        candidate_replacement_count_change,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        compare_lifecycle_scenarios, evaluate_lifecycle_accounting, LifecycleCostCategory,
        LifecycleCostEntry, LifecycleEvidenceClass, LifecycleMissionRecord,
        LifecycleRepairEvent, LifecycleScenarioInput, StorageHealthObservation,
    };
    use crate::capability_envelope::EvidenceStatus;

    fn digest(byte: char) -> String {
        std::iter::repeat(byte).take(64).collect()
    }

    fn mission(id: &str) -> LifecycleMissionRecord {
        LifecycleMissionRecord {
            mission_id: id.to_string(),
            mission_profile_sha256: digest('b'),
            terminal_state_obligation_sha256: digest('c'),
            qualified_configuration_sha256: digest('a'),
            observed_configuration_sha256: digest('a'),
            qualification_valid_until_epoch_s: 10_000,
            start_s: 0.0,
            duration_s: 3_600.0,
            required_critical_service_wh: 1_000.0,
            delivered_critical_service_wh: 1_000.0,
            maximum_unserved_critical_energy_wh: 0.0,
            energy_screen_pass: true,
            event_screen_pass: true,
            terminal_state_pass: true,
            evidence_status: EvidenceStatus::Current,
            evidence_class: LifecycleEvidenceClass::IndependentPhysicalMeasurement,
            evidence_reference: format!("metrology:mission-{id}"),
        }
    }

    fn cost(id: &str, category: LifecycleCostCategory, amount: u64) -> LifecycleCostEntry {
        LifecycleCostEntry {
            entry_id: id.to_string(),
            economic_event_id: format!("economic-event-{id}"),
            category,
            amount_minor_units: amount,
            incurred_at_epoch_s: 1_500,
            evidence_status: EvidenceStatus::Current,
            evidence_class: LifecycleEvidenceClass::InvoiceOrReceipt,
            evidence_reference: format!("invoice:{id}"),
        }
    }

    fn repair(id: &str, start_s: f64, down_s: f64) -> LifecycleRepairEvent {
        LifecycleRepairEvent {
            repair_id: id.to_string(),
            component_id: format!("module-{id}"),
            replacement_component_id: None,
            pre_repair_configuration_sha256: digest('f'),
            post_repair_configuration_sha256: digest('f'),
            start_s,
            system_unavailable_duration_s: down_s,
            repair_labor_hours: 2.0,
            requalification_hours: 0.5,
            replacement_required: false,
            evidence_status: EvidenceStatus::Current,
            evidence_class: LifecycleEvidenceClass::IndependentPhysicalMeasurement,
            evidence_reference: format!("work-order:{id}"),
        }
    }

    fn observation(
        id: &str,
        component: &str,
        epoch: u64,
        capacity: f64,
        power: f64,
        throughput: f64,
        cycles: f64,
    ) -> StorageHealthObservation {
        StorageHealthObservation {
            observation_id: id.to_string(),
            component_id: component.to_string(),
            observed_at_epoch_s: epoch,
            qualified_configuration_sha256: digest('d'),
            observed_configuration_sha256: digest('d'),
            usable_capacity_wh: capacity,
            capacity_uncertainty_bound_wh: 10.0,
            maximum_discharge_power_w: power,
            power_uncertainty_bound_w: 5.0,
            cumulative_energy_throughput_wh: throughput,
            calendar_age_hours: (epoch - 1_000) as f64,
            equivalent_full_cycles: cycles,
            state_of_charge_fraction: 0.5,
            state_of_health_fraction: 0.9,
            temperature_c: 25.0,
            evidence_status: EvidenceStatus::Current,
            calibration_status: EvidenceStatus::Current,
            sensor_status: EvidenceStatus::Current,
            evidence_class: LifecycleEvidenceClass::IndependentPhysicalMeasurement,
            operating_condition_reference: format!("test-condition:{id}"),
            timebase_reference: format!("mycelix-timebase:{id}"),
            evidence_reference: format!("battery-test:{id}"),
        }
    }

    fn input(scenario: &str, storage_in_scope: bool) -> LifecycleScenarioInput {
        LifecycleScenarioInput {
            scenario_id: scenario.to_string(),
            comparator_set_id: "matched-site-shift-001".to_string(),
            service_boundary_id: "400v-ac-load-bus".to_string(),
            site_profile_sha256: digest('e'),
            qualified_configuration_sha256: digest('a'),
            observed_configuration_sha256: digest('a'),
            qualification_valid_until_epoch_s: 10_000,
            evaluation_time_epoch_s: 2_000,
            currency_code: "USD".to_string(),
            minor_unit_scale: 2,
            study_start_epoch_s: 1_000,
            study_end_epoch_s: 4_600,
            operating_window_s: 3_600.0,
            storage_in_scope,
            mission_records: vec![mission("shift-001")],
            cost_entries: vec![cost("fuel-1", LifecycleCostCategory::Fuel, 10_000)],
            repair_history_complete: true,
            repair_history_evidence_status: EvidenceStatus::Current,
            repair_history_evidence_class: LifecycleEvidenceClass::IndependentPhysicalMeasurement,
            repair_history_evidence_reference: "maintenance:complete-history-window-001".to_string(),
            repair_events: Vec::new(),
            storage_health_observations: Vec::new(),
        }
    }

    #[test]
    fn nominal_lifecycle_cost_per_delivered_critical_kwh_is_calculated() {
        let report = evaluate_lifecycle_accounting(&input("generator", false)).unwrap();
        assert_eq!(report.total_cost_minor_units, 10_000);
        assert_eq!(report.total_delivered_critical_service_wh, 1_000.0);
        assert_eq!(
            report.cost_per_delivered_critical_kwh_minor_units,
            Some(10_000.0)
        );
        assert_eq!(
            report.cost_per_completed_mission_minor_units,
            Some(10_000.0)
        );
        assert!(report.physical_lifecycle_claim_eligible);
    }

    #[test]
    fn failed_mission_is_not_removed_from_the_cost_denominator() {
        let mut value = input("failed", false);
        value.mission_records[0].delivered_critical_service_wh = 800.0;
        value.mission_records[0].energy_screen_pass = false;
        let report = evaluate_lifecycle_accounting(&value).unwrap();
        assert_eq!(report.total_unserved_critical_service_wh, 200.0);
        assert_eq!(report.completed_mission_count, 0);
        assert_eq!(report.mission_count, 1);
        assert_eq!(report.cost_per_completed_mission_minor_units, None);
        assert!(!report.physical_lifecycle_claim_eligible);
    }

    #[test]
    fn mission_local_critical_shortfall_gate_is_enforced() {
        let mut value = input("shortfall", false);
        value.mission_records[0].delivered_critical_service_wh = 999.0;
        value.mission_records[0].maximum_unserved_critical_energy_wh = 0.0;
        let report = evaluate_lifecycle_accounting(&value).unwrap();
        assert_eq!(report.completed_mission_count, 0);
    }

    #[test]
    fn overlapping_repair_downtime_is_unioned_not_summed() {
        let mut value = input("overlap", false);
        value.repair_events = vec![
            repair("a", 100.0, 100.0),
            repair("b", 150.0, 100.0),
        ];
        let report = evaluate_lifecycle_accounting(&value).unwrap();
        assert!((report.system_unavailable_hours - (150.0 / 3600.0)).abs() < 1e-12);
        assert_eq!(report.repair_labor_hours, 4.0);
        assert_eq!(report.requalification_hours, 1.0);
    }

    #[test]
    fn incomplete_repair_history_blocks_physical_lifecycle_claim() {
        let mut value = input("unknown-repairs", false);
        value.repair_history_complete = false;
        value.repair_history_evidence_status = EvidenceStatus::Unverified;
        let report = evaluate_lifecycle_accounting(&value).unwrap();
        assert!(!report.repair_evidence_current);
        assert!(!report.repair_evidence_physical);
        assert!(!report.physical_lifecycle_claim_eligible);
    }

    #[test]
    fn replacement_requires_a_new_component_identity() {
        let mut value = input("replacement-id", false);
        let mut item = repair("replacement", 100.0, 10.0);
        item.replacement_required = true;
        // Deliberately missing a distinct replacement identity.
        value.repair_events.push(item);
        assert!(evaluate_lifecycle_accounting(&value).is_err());
    }

    #[test]
    fn replacement_count_is_explicit() {
        let mut value = input("replace", false);
        let mut item = repair("replacement", 100.0, 10.0);
        item.replacement_required = true;
        item.replacement_component_id = Some("battery-new-serial-002".to_string());
        item.post_repair_configuration_sha256 = digest('g');
        value.repair_events.push(item);
        let report = evaluate_lifecycle_accounting(&value).unwrap();
        assert_eq!(report.replacement_count, 1);
    }

    #[test]
    fn unverified_cost_evidence_blocks_lifecycle_accounting_eligibility() {
        let mut value = input("unverified-cost", false);
        value.cost_entries[0].evidence_status = EvidenceStatus::Unverified;
        let report = evaluate_lifecycle_accounting(&value).unwrap();
        assert!(!report.cost_evidence_current);
        assert!(!report.lifecycle_accounting_eligible);
        assert!(!report.physical_lifecycle_claim_eligible);
    }

    #[test]
    fn estimate_cost_can_be_totalled_but_is_not_physical_cost_evidence() {
        let mut value = input("estimate", false);
        value.cost_entries[0].evidence_class = LifecycleEvidenceClass::EngineeringEstimate;
        let report = evaluate_lifecycle_accounting(&value).unwrap();
        assert_eq!(report.total_cost_minor_units, 10_000);
        assert!(!report.cost_evidence_actual);
        assert!(!report.physical_lifecycle_claim_eligible);
    }

    #[test]
    fn storage_retention_uses_lower_bounds_from_same_component() {
        let mut value = input("battery", true);
        value.storage_health_observations = vec![
            observation("initial", "battery-a", 1_000, 5_000.0, 2_000.0, 0.0, 0.0),
            observation("later", "battery-a", 4_000, 4_500.0, 1_800.0, 10_000.0, 2.0),
        ];
        let report = evaluate_lifecycle_accounting(&value).unwrap();
        assert!(report.storage_health_evidence_complete);
        assert_eq!(report.storage_health_trajectories.len(), 1);
        let trajectory = &report.storage_health_trajectories[0];
        assert!((trajectory.usable_capacity_retention_fraction - 4_490.0 / 4_990.0).abs() < 1e-12);
        assert!((trajectory.discharge_power_retention_fraction - 1_795.0 / 1_995.0).abs() < 1e-12);
        assert_eq!(trajectory.energy_throughput_delta_wh, 10_000.0);
        assert_eq!(trajectory.first_timebase_reference, "mycelix-timebase:initial");
        assert!((trajectory.last_state_of_health_fraction - 0.9).abs() < 1e-12);
        assert!(report.physical_lifecycle_claim_eligible);
    }

    #[test]
    fn storage_in_scope_without_repeated_observations_is_incomplete() {
        let report = evaluate_lifecycle_accounting(&input("missing-health", true)).unwrap();
        assert!(!report.storage_health_evidence_complete);
        assert!(!report.physical_lifecycle_claim_eligible);
    }

    #[test]
    fn single_storage_observation_is_not_a_degradation_trajectory() {
        let mut value = input("one-point", true);
        value.storage_health_observations = vec![
            observation("one", "battery-a", 1_000, 5_000.0, 2_000.0, 0.0, 0.0),
        ];
        let report = evaluate_lifecycle_accounting(&value).unwrap();
        assert!(!report.storage_health_evidence_complete);
        assert!(report.storage_health_trajectories.is_empty());
    }

    #[test]
    fn non_independent_storage_reports_do_not_establish_physical_retention() {
        let mut value = input("bms-report", true);
        let mut first = observation("initial", "battery-a", 1_000, 5_000.0, 2_000.0, 0.0, 0.0);
        let mut second = observation("later", "battery-a", 4_000, 4_500.0, 1_800.0, 10_000.0, 2.0);
        first.evidence_class = LifecycleEvidenceClass::VendorReport;
        second.evidence_class = LifecycleEvidenceClass::VendorReport;
        value.storage_health_observations = vec![first, second];
        let report = evaluate_lifecycle_accounting(&value).unwrap();
        assert!(!report.storage_health_evidence_complete);
        assert!(report.storage_health_trajectories.is_empty());
    }

    #[test]
    fn storage_configuration_change_does_not_splice_a_health_history() {
        let mut value = input("config-change", true);
        let first = observation("initial", "battery-a", 1_000, 5_000.0, 2_000.0, 0.0, 0.0);
        let mut second = observation("later", "battery-a", 4_000, 4_500.0, 1_800.0, 10_000.0, 2.0);
        second.qualified_configuration_sha256 = digest('f');
        second.observed_configuration_sha256 = digest('f');
        value.storage_health_observations = vec![first, second];
        let report = evaluate_lifecycle_accounting(&value).unwrap();
        assert!(!report.storage_health_evidence_complete);
        assert!(report.storage_health_trajectories.is_empty());
    }

    #[test]
    fn non_monotonic_storage_throughput_is_rejected() {
        let mut value = input("throughput-reset", true);
        value.storage_health_observations = vec![
            observation("later", "battery-a", 4_000, 4_500.0, 1_800.0, 10_000.0, 2.0),
            observation("earlier", "battery-a", 1_000, 5_000.0, 2_000.0, 0.0, 0.0),
        ];
        // Input order is not material: sorted observations remain monotonic.
        assert!(evaluate_lifecycle_accounting(&value).is_ok());
        value.storage_health_observations[1].cumulative_energy_throughput_wh = 15_000.0;
        assert!(evaluate_lifecycle_accounting(&value).is_err());
    }

    #[test]
    fn expired_storage_calibration_blocks_health_trajectory() {
        let mut value = input("expired-cal", true);
        let first = observation("initial", "battery-a", 1_000, 5_000.0, 2_000.0, 0.0, 0.0);
        let mut second = observation("later", "battery-a", 4_000, 4_500.0, 1_800.0, 10_000.0, 2.0);
        second.calibration_status = EvidenceStatus::Expired;
        value.storage_health_observations = vec![first, second];
        let report = evaluate_lifecycle_accounting(&value).unwrap();
        assert!(!report.storage_health_evidence_complete);
    }

    #[test]
    fn duplicate_cost_ids_are_rejected() {
        let mut value = input("duplicate-cost", false);
        let duplicate = value.cost_entries[0].clone();
        value.cost_entries.push(duplicate);
        assert!(evaluate_lifecycle_accounting(&value).is_err());
    }

    #[test]
    fn duplicate_economic_event_id_is_rejected_even_with_distinct_entry_ids() {
        let mut value = input("duplicate-economic-event", false);
        let mut duplicate = cost("second-entry", LifecycleCostCategory::Fuel, 10_000);
        duplicate.economic_event_id = value.cost_entries[0].economic_event_id.clone();
        value.cost_entries.push(duplicate);
        assert!(evaluate_lifecycle_accounting(&value).is_err());
    }

    #[test]
    fn invalid_currency_is_rejected() {
        let mut value = input("currency", false);
        value.currency_code = "US".to_string();
        assert!(evaluate_lifecycle_accounting(&value).is_err());
    }

    #[test]
    fn repair_interval_outside_study_is_rejected() {
        let mut value = input("repair-outside", false);
        value.repair_events.push(repair("outside", 3_500.0, 200.0));
        assert!(evaluate_lifecycle_accounting(&value).is_err());
    }

    #[test]
    fn comparison_rejects_mismatched_service_boundary() {
        let reference = input("reference", false);
        let mut candidate = input("candidate", false);
        candidate.service_boundary_id = "dc-bus".to_string();
        assert!(compare_lifecycle_scenarios(&reference, &candidate).is_err());
    }

    #[test]
    fn comparison_rejects_mismatched_terminal_state_obligation() {
        let reference = input("reference", false);
        let mut candidate = input("candidate", false);
        candidate.mission_records[0].terminal_state_obligation_sha256 = digest('f');
        assert!(compare_lifecycle_scenarios(&reference, &candidate).is_err());
    }

    #[test]
    fn matched_lifecycle_comparison_reports_cost_change() {
        let reference = input("reference", false);
        let mut candidate = input("candidate", false);
        candidate.cost_entries[0].amount_minor_units = 8_000;
        let result = compare_lifecycle_scenarios(&reference, &candidate).unwrap();
        assert!(result.matched_service_contract);
        assert!(result.comparison_claim_eligible);
        assert!(result.reference.storage_health_evidence_complete);
        assert!((result.candidate_cost_change_percent.unwrap() + 20.0).abs() < 1e-12);
    }

    #[test]
    fn failed_candidate_mission_prevents_lifecycle_claim() {
        let reference = input("reference", false);
        let mut candidate = input("candidate", false);
        candidate.mission_records[0].event_screen_pass = false;
        let result = compare_lifecycle_scenarios(&reference, &candidate).unwrap();
        assert!(result.matched_service_contract);
        assert!(!result.comparison_claim_eligible);
    }

    #[test]
    fn replaced_module_is_a_separate_health_trajectory() {
        let mut value = input("replacement", true);
        value.storage_health_observations = vec![
            observation("old-1", "battery-old", 1_000, 5_000.0, 2_000.0, 0.0, 0.0),
            observation("old-2", "battery-old", 2_000, 4_800.0, 1_900.0, 5_000.0, 1.0),
            observation("new-1", "battery-new", 3_000, 5_000.0, 2_000.0, 0.0, 0.0),
        ];
        let report = evaluate_lifecycle_accounting(&value).unwrap();
        assert!(!report.storage_health_evidence_complete);
        assert_eq!(report.storage_health_trajectories.len(), 1);
        assert_eq!(report.storage_health_trajectories[0].component_id, "battery-old");
    }

    #[test]
    fn cost_category_ledger_is_deterministic() {
        let mut a = input("ledger-a", false);
        a.cost_entries.push(cost("parts", LifecycleCostCategory::MaintenanceParts, 250));
        let mut b = a.clone();
        b.cost_entries.reverse();
        let ra = evaluate_lifecycle_accounting(&a).unwrap();
        let rb = evaluate_lifecycle_accounting(&b).unwrap();
        assert_eq!(ra.total_cost_minor_units, 10_250);
        assert_eq!(ra.cost_by_category, rb.cost_by_category);
        assert_eq!(ra.total_cost_minor_units, rb.total_cost_minor_units);
    }

    #[test]
    fn mission_configuration_mismatch_blocks_physical_claim() {
        let mut value = input("mission-config", false);
        value.mission_records[0].observed_configuration_sha256 = digest('f');
        let report = evaluate_lifecycle_accounting(&value).unwrap();
        assert!(!report.mission_evidence_current);
        assert!(!report.physical_lifecycle_claim_eligible);
    }

    #[test]
    fn scenario_claim_ceiling_excludes_remaining_life_forecast() {
        let report = evaluate_lifecycle_accounting(&input("ceiling", false)).unwrap();
        assert_eq!(
            report.claim_ceiling(),
            "LIFECYCLE_SERVICE_ACCOUNTING_ONLY_NOT_REMAINING_LIFE_OR_PHYSICAL_QUALIFICATION"
        );
    }
}
