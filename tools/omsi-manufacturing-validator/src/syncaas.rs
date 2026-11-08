use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const FIXTURE_CONTRACT: &str = "omsi.syncaas.annex-g-fixture-pack";
pub const REPORT_CONTRACT: &str = "omsi-syncaas-annex-g-report/0.3";
const PROFILE_BASE_EDITION: &str = "02/2026";
const PROFILE_AMENDMENT_STATUS: &str = "BASE_EDITION_ONLY_NOT_AMENDMENT_1_CONFORMANCE";
const DEFAULT_CLOCKCLASS_THRESHOLD: u8 = 6;
const MAX_CLOCK_CLASS: u64 = 255;
const WINDOW_SECONDS: usize = 10;
const MAX_CASES: usize = 100;
const MAX_SAMPLES_PER_CASE: usize = 100_000;
const MAX_PERIODS: usize = 10_000;

const EXPECTED_CASE_IDS: [&str; 14] = [
    "SVC-SUPPORTED-NOMINAL-001",
    "SVC-TRANSIENT-DEGRADATION-001",
    "SVC-OUTAGE-RECOVERY-001",
    "SVC-MAINTENANCE-PAUSE-001",
    "SVC-CROSS-PERIOD-001",
    "SVC-MISSING-OBSERVATION-001",
    "SVC-DISABLED-001",
    "SVC-UNSUPPORTED-001",
    "SVC-RECORD-MISMATCH-001",
    "SVC-PERIOD-INCOMPLETE-001",
    "SVC-PTS-SEPARATE-001",
    "SVC-PARTIAL-CLOCKCLASS-COVERAGE-001",
    "SVC-FTS-CLOCK-TYPE-NOT-APPLICABLE-001",
    "SVC-PTS-CLOCK-TYPE-NOT-APPLICABLE-001",
];

const CLAIM_CEILING: &str = "Synthetic base-edition (02/2026) G.8275.1 Annex M and G.8275.2 Annex G clockClass-to-N1S/D1S classification plus the common G.8275 Annex G AS/UAS state machine and record consistency. The August 2026 amendments are not integrated or claimed. This does not measure physical time error, qualify a PTP implementation, establish operational service quality, or certify equipment or a network.";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ServiceState {
    Available,
    Unavailable,
}

impl ServiceState {
    fn as_str(self) -> &'static str {
        match self {
            Self::Available => "AVAILABLE",
            Self::Unavailable => "UNAVAILABLE",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ClockClassCoverage {
    Complete,
    Partial,
    Unknown,
    MaintenancePaused,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CriterionState {
    Nominal,
    Degraded,
    Unobserved,
}

impl CriterionState {
    fn target(self) -> Option<ServiceState> {
        match self {
            Self::Nominal => Some(ServiceState::Available),
            Self::Degraded => Some(ServiceState::Unavailable),
            Self::Unobserved => None,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct Counters {
    available: u64,
    unavailable: u64,
    unclassified: u64,
    unobserved: u64,
    wall_seconds: u64,
    maintenance_paused_seconds: u64,
}

impl Counters {
    fn count_state(&mut self, state: ServiceState) {
        match state {
            ServiceState::Available => self.available += 1,
            ServiceState::Unavailable => self.unavailable += 1,
        }
    }
}

#[derive(Clone, Debug)]
struct Sample {
    period_id: String,
    clock_class_values: Vec<u64>,
    clockclass_coverage: ClockClassCoverage,
    maintenance: bool,
}

#[derive(Clone, Debug)]
struct Period {
    id: String,
    wall_seconds: u64,
    maintenance_paused_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Reconstruction {
    state: Option<ServiceState>,
    counters: BTreeMap<String, Counters>,
}

#[derive(Clone, Debug)]
struct CaseResult {
    status: &'static str,
    final_state: &'static str,
    reasons: Vec<&'static str>,
    primary: Reconstruction,
    reference: Reconstruction,
    availability_ratio: Option<(u64, u64)>,
}

impl Reconstruction {
    fn as_value(&self) -> Value {
        let mut output = serde_json::Map::new();
        for (id, counters) in &self.counters {
            output.insert(
                id.clone(),
                json!({
                    "AS": counters.available,
                    "UAS": counters.unavailable,
                    "unclassified_seconds": counters.unclassified,
                    "unobserved_seconds": counters.unobserved,
                    "wall_seconds": counters.wall_seconds,
                    "maintenance_paused_seconds": counters.maintenance_paused_seconds
                }),
            );
        }
        Value::Object(output)
    }
}

impl CaseResult {
    fn to_value(&self) -> Value {
        let ratio = self.availability_ratio.map(|(available, unavailable)| {
            json!({
                "available_seconds": available,
                "unavailable_seconds": unavailable,
                "measured_seconds": available + unavailable
            })
        });
        json!({
            "status": self.status,
            "final_state": self.final_state,
            "reason_codes": self.reasons,
            "primary": self.primary.as_value(),
            "reference": self.reference.as_value(),
            "independent_agreement": self.primary == self.reference,
            "availability_ratio": ratio,
            "claim_ceiling": CLAIM_CEILING
        })
    }
}

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn boolean(value: &Value, key: &str) -> Option<bool> {
    value.get(key).and_then(Value::as_bool)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

fn parse_periods(case: &Value) -> Result<(Vec<Period>, BTreeMap<String, usize>), &'static str> {
    let values = case
        .get("pm_periods")
        .and_then(Value::as_array)
        .ok_or("pm_periods_missing_or_not_array")?;
    if values.is_empty() || values.len() > MAX_PERIODS {
        return Err("pm_period_count_out_of_bounds");
    }

    let mut periods = Vec::with_capacity(values.len());
    let mut positions = BTreeMap::new();
    for value in values {
        let id = text(value, "id")
            .filter(|id| !id.trim().is_empty())
            .ok_or("period_id_missing")?;
        let wall = value
            .get("wall_seconds")
            .and_then(Value::as_u64)
            .ok_or("period_wall_seconds_invalid")?;
        let maintenance = value
            .get("maintenance_paused_seconds")
            .and_then(Value::as_u64)
            .ok_or("period_maintenance_seconds_invalid")?;
        if maintenance > wall {
            return Err("maintenance_seconds_exceed_wall_seconds");
        }
        if positions.contains_key(id) {
            return Err("duplicate_period_id");
        }
        positions.insert(id.to_owned(), periods.len());
        periods.push(Period {
            id: id.to_owned(),
            wall_seconds: wall,
            maintenance_paused_seconds: maintenance,
        });
    }
    Ok((periods, positions))
}

fn parse_samples(
    case: &Value,
    periods: &[Period],
    period_positions: &BTreeMap<String, usize>,
) -> Result<Vec<Sample>, &'static str> {
    let values = case
        .get("samples")
        .and_then(Value::as_array)
        .ok_or("samples_missing_or_not_array")?;
    if values.is_empty() || values.len() > MAX_SAMPLES_PER_CASE {
        return Err("sample_count_out_of_bounds");
    }

    let mut samples = Vec::with_capacity(values.len());
    let mut last_period_position = 0usize;
    for (expected_sequence, value) in values.iter().enumerate() {
        let sequence = value
            .get("sequence")
            .and_then(Value::as_u64)
            .and_then(|number| usize::try_from(number).ok())
            .ok_or("sample_sequence_invalid")?;
        if sequence != expected_sequence {
            return Err("sample_sequence_not_contiguous");
        }
        let period_id = text(value, "period_id").ok_or("sample_period_id_missing")?;
        let period_position = *period_positions
            .get(period_id)
            .ok_or("sample_references_unknown_period")?;
        if period_position < last_period_position {
            return Err("sample_period_order_regressed");
        }
        last_period_position = period_position;

        let values = value
            .get("clock_class_values")
            .and_then(Value::as_array)
            .ok_or("clock_class_values_missing_or_not_array")?;
        let mut clock_class_values = Vec::with_capacity(values.len());
        for item in values {
            let clock_class = item
                .as_u64()
                .ok_or("clock_class_value_not_unsigned_integer")?;
            if clock_class > MAX_CLOCK_CLASS {
                return Err("clock_class_value_out_of_uinteger8_range");
            }
            clock_class_values.push(clock_class);
        }
        let maintenance = boolean(value, "maintenance_window").ok_or("maintenance_flag_missing")?;
        let coverage = match text(value, "clockclass_coverage") {
            Some("COMPLETE") => ClockClassCoverage::Complete,
            Some("PARTIAL") => ClockClassCoverage::Partial,
            Some("UNKNOWN") => ClockClassCoverage::Unknown,
            Some("MAINTENANCE_PAUSED") => ClockClassCoverage::MaintenancePaused,
            _ => return Err("clockclass_coverage_missing_or_unsupported"),
        };
        if maintenance {
            if coverage != ClockClassCoverage::MaintenancePaused {
                return Err("maintenance_sample_requires_maintenance_paused_coverage");
            }
            if !clock_class_values.is_empty() {
                return Err("maintenance_sample_must_not_claim_clockclass_observation");
            }
        } else {
            if coverage == ClockClassCoverage::MaintenancePaused {
                return Err("non_maintenance_sample_cannot_claim_maintenance_paused_coverage");
            }
            if coverage == ClockClassCoverage::Complete && clock_class_values.is_empty() {
                return Err("complete_clockclass_coverage_requires_observation_values");
            }
        }
        samples.push(Sample {
            period_id: period_id.to_owned(),
            clock_class_values,
            clockclass_coverage: coverage,
            maintenance,
        });
    }

    let mut actual_wall = vec![0u64; periods.len()];
    let mut actual_maintenance = vec![0u64; periods.len()];
    for sample in &samples {
        let position = period_positions[&sample.period_id];
        actual_wall[position] += 1;
        if sample.maintenance {
            actual_maintenance[position] += 1;
        }
    }
    for (index, period) in periods.iter().enumerate() {
        if actual_wall[index] != period.wall_seconds {
            return Err("period_wall_seconds_do_not_match_sample_coverage");
        }
        if actual_maintenance[index] != period.maintenance_paused_seconds {
            return Err("period_maintenance_seconds_do_not_match_samples");
        }
    }
    Ok(samples)
}

fn empty_counters(periods: &[Period]) -> BTreeMap<String, Counters> {
    periods
        .iter()
        .map(|period| {
            (
                period.id.clone(),
                Counters {
                    wall_seconds: period.wall_seconds,
                    maintenance_paused_seconds: period.maintenance_paused_seconds,
                    ..Counters::default()
                },
            )
        })
        .collect()
}

fn count_for_period(
    counters: &mut BTreeMap<String, Counters>,
    period_id: &str,
    state: ServiceState,
) {
    if let Some(period) = counters.get_mut(period_id) {
        period.count_state(state);
    }
}

fn flush_pending(
    pending: &mut Vec<String>,
    state: Option<ServiceState>,
    counters: &mut BTreeMap<String, Counters>,
) {
    for period_id in pending.drain(..) {
        if let Some(stable_state) = state {
            count_for_period(counters, &period_id, stable_state);
        } else if let Some(period) = counters.get_mut(&period_id) {
            period.unclassified += 1;
        }
    }
}

/// Primary per-second classifier. N1S requires every observed clockClass value
/// within the one-second interval to be <= Cth; the IEEE 1588 hierarchy is
/// inverse numeric order (smaller is better). Empty observations are a local
/// UNKNOWN extension, never silently D1S.
fn classify_clockclass_primary(sample: &Sample, threshold: u8) -> CriterionState {
    if sample.maintenance
        || sample.clockclass_coverage != ClockClassCoverage::Complete
        || sample.clock_class_values.is_empty()
    {
        return CriterionState::Unobserved;
    }
    if sample
        .clock_class_values
        .iter()
        .all(|value| *value <= u64::from(threshold))
    {
        CriterionState::Nominal
    } else {
        CriterionState::Degraded
    }
}

/// Separate reference formulation: inspect the worst (maximum numeric)
/// clockClass observed within the one-second interval.
fn classify_clockclass_reference(sample: &Sample, threshold: u8) -> CriterionState {
    if sample.maintenance
        || sample.clockclass_coverage != ClockClassCoverage::Complete
        || sample.clock_class_values.is_empty()
    {
        return CriterionState::Unobserved;
    }
    let worst_clock_class = sample
        .clock_class_values
        .iter()
        .copied()
        .max()
        .unwrap_or(MAX_CLOCK_CLASS + 1);
    if worst_clock_class <= u64::from(threshold) {
        CriterionState::Nominal
    } else {
        CriterionState::Degraded
    }
}

/// Streaming implementation. Up to N-1 candidate seconds are held until the
/// transition is confirmed or broken. This permits a 10-second decision to
/// cross a PM-period boundary without prematurely finalizing prior-period AS/UAS.
fn streaming_reconstruction(
    samples: &[Sample],
    periods: &[Period],
    window: usize,
    threshold: u8,
) -> Reconstruction {
    let mut counters = empty_counters(periods);
    let mut state = None;
    let mut pending_target: Option<ServiceState> = None;
    let mut pending_periods: Vec<String> = Vec::new();

    for sample in samples {
        if sample.maintenance {
            continue;
        }

        let Some(target) = classify_clockclass_primary(sample, threshold).target() else {
            flush_pending(&mut pending_periods, state, &mut counters);
            pending_target = None;
            if let Some(period) = counters.get_mut(&sample.period_id) {
                period.unobserved += 1;
            }
            continue;
        };

        match state {
            Some(stable) if target == stable => {
                flush_pending(&mut pending_periods, state, &mut counters);
                pending_target = None;
                count_for_period(&mut counters, &sample.period_id, stable);
            }
            Some(_) => {
                pending_periods.push(sample.period_id.clone());
                pending_target = Some(target);
                if pending_periods.len() == window {
                    state = Some(target);
                    for period_id in pending_periods.drain(..) {
                        count_for_period(&mut counters, &period_id, target);
                    }
                    pending_target = None;
                }
            }
            None => {
                if pending_target != Some(target) {
                    flush_pending(&mut pending_periods, None, &mut counters);
                    pending_target = Some(target);
                }
                pending_periods.push(sample.period_id.clone());
                if pending_periods.len() == window {
                    state = Some(target);
                    for period_id in pending_periods.drain(..) {
                        count_for_period(&mut counters, &period_id, target);
                    }
                    pending_target = None;
                }
            }
        }
    }

    flush_pending(&mut pending_periods, state, &mut counters);
    Reconstruction { state, counters }
}

/// Reference implementation based on maximal runs, independent of the
/// streaming/pending-buffer implementation above. Maintenance samples are
/// removed from the evaluation sequence; unobserved samples split a run.
fn run_based_reference(
    samples: &[Sample],
    periods: &[Period],
    window: usize,
    threshold: u8,
) -> Reconstruction {
    let mut counters = empty_counters(periods);
    let mut state = None;
    let mut run_target: Option<ServiceState> = None;
    let mut run_periods: Vec<String> = Vec::new();

    let flush_run = |target: Option<ServiceState>,
                     run_periods: &mut Vec<String>,
                     state: &mut Option<ServiceState>,
                     counters: &mut BTreeMap<String, Counters>| {
        if run_periods.is_empty() {
            return;
        }
        if run_periods.len() >= window {
            if let Some(next_state) = target {
                *state = Some(next_state);
                for period_id in run_periods.drain(..) {
                    count_for_period(counters, &period_id, next_state);
                }
                return;
            }
        }
        for period_id in run_periods.drain(..) {
            if let Some(stable) = *state {
                count_for_period(counters, &period_id, stable);
            } else if let Some(period) = counters.get_mut(&period_id) {
                period.unclassified += 1;
            }
        }
    };

    for sample in samples {
        if sample.maintenance {
            continue;
        }
        let target = classify_clockclass_reference(sample, threshold).target();
        if target.is_none() {
            flush_run(run_target, &mut run_periods, &mut state, &mut counters);
            run_target = None;
            if let Some(period) = counters.get_mut(&sample.period_id) {
                period.unobserved += 1;
            }
            continue;
        }
        if run_target != target {
            flush_run(run_target, &mut run_periods, &mut state, &mut counters);
            run_target = target;
        }
        run_periods.push(sample.period_id.clone());
    }
    flush_run(run_target, &mut run_periods, &mut state, &mut counters);

    Reconstruction { state, counters }
}

fn parse_records(
    case: &Value,
    periods: &[Period],
) -> Result<BTreeMap<String, Value>, &'static str> {
    let values = case
        .get("records")
        .and_then(Value::as_array)
        .ok_or("records_missing_or_not_array")?;
    if values.len() != periods.len() {
        return Err("record_count_does_not_match_period_count");
    }
    let expected_ids: BTreeSet<&str> = periods.iter().map(|period| period.id.as_str()).collect();
    let mut records = BTreeMap::new();
    let mut indices = BTreeSet::new();

    for record in values {
        let period_id = text(record, "period_id").ok_or("record_period_id_missing")?;
        if !expected_ids.contains(period_id) {
            return Err("record_references_unknown_period");
        }
        let index = record
            .get("index")
            .and_then(Value::as_u64)
            .ok_or("record_index_invalid")?;
        if index > u16::MAX as u64 || !indices.insert(index) {
            return Err("record_index_out_of_u16_range_or_duplicate");
        }
        if text(record, "PMTime")
            .filter(|value| !value.trim().is_empty())
            .is_none()
        {
            return Err("record_pm_time_missing");
        }
        if records
            .insert(period_id.to_owned(), record.clone())
            .is_some()
        {
            return Err("duplicate_record_for_period");
        }
        for field in ["AS", "UAS"] {
            let value = record
                .get(field)
                .and_then(Value::as_u64)
                .ok_or("record_counter_invalid")?;
            if value > u32::MAX as u64 {
                return Err("record_counter_exceeds_uinteger32");
            }
        }
        if boolean(record, "measurementValid").is_none()
            || boolean(record, "periodComplete").is_none()
        {
            return Err("record_validity_flags_missing");
        }
    }
    Ok(records)
}

fn evaluate_case(case: &Value) -> CaseResult {
    let empty = Reconstruction {
        state: None,
        counters: BTreeMap::new(),
    };
    let Some(dataset) = case.get("dataset") else {
        return simple_case(
            "INVALID_INPUT",
            "UNKNOWN",
            "dataset_capability_missing",
            empty.clone(),
            empty,
        );
    };
    let supported = boolean(dataset, "supported");
    let enabled = boolean(dataset, "enable");
    match supported {
        Some(false) => {
            return simple_case(
                "NOT_SUPPORTED",
                "UNKNOWN",
                "annex_g_dataset_not_supported",
                empty.clone(),
                empty,
            )
        }
        None => {
            return simple_case(
                "INDETERMINATE",
                "UNKNOWN",
                "annex_g_dataset_support_unknown",
                empty.clone(),
                empty,
            )
        }
        Some(true) => {}
    }
    match enabled {
        Some(false) => {
            return simple_case(
                "DISABLED",
                "UNKNOWN",
                "annex_g_dataset_disabled",
                empty.clone(),
                empty,
            )
        }
        None => {
            return simple_case(
                "INDETERMINATE",
                "UNKNOWN",
                "annex_g_enable_state_unknown",
                empty.clone(),
                empty,
            )
        }
        Some(true) => {}
    }

    let profile = case.get("profile").unwrap_or(&Value::Null);
    let itu_profile = text(profile, "itu_t_profile");
    let expected_profile_annex = match itu_profile {
        Some("G.8275.1") => Some("M"),
        Some("G.8275.2") => Some("G"),
        _ => None,
    };
    let Some(expected_profile_annex) = expected_profile_annex else {
        return simple_case(
            "NOT_COMPARABLE",
            "UNKNOWN",
            "itu_t_profile_missing_or_unsupported",
            empty.clone(),
            empty,
        );
    };
    if text(profile, "edition") != Some(PROFILE_BASE_EDITION) {
        return simple_case(
            "NOT_COMPARABLE",
            "UNKNOWN",
            "profile_edition_not_supported_by_this_baseline",
            empty.clone(),
            empty,
        );
    }
    if text(profile, "profile_annex") != Some(expected_profile_annex) {
        return simple_case(
            "NOT_COMPARABLE",
            "UNKNOWN",
            "profile_specific_syncaas_annex_mismatch",
            empty.clone(),
            empty,
        );
    }
    let clock_type = text(profile, "clock_type");
    let clock_type_applicable = match itu_profile {
        Some("G.8275.1") => clock_type == Some("T-BC"),
        Some("G.8275.2") => matches!(clock_type, Some("T-BC-A" | "T-BC-P")),
        _ => false,
    };
    if !clock_type_applicable {
        return simple_case(
            "NOT_COMPARABLE",
            "UNKNOWN",
            "clock_type_not_applicable_for_profile",
            empty.clone(),
            empty,
        );
    }
    if text(profile, "amendment_2026_08") != Some("NOT_APPLIED") {
        return simple_case(
            "NOT_COMPARABLE",
            "UNKNOWN",
            "profile_amendment_not_integrated",
            empty.clone(),
            empty,
        );
    }
    if case.get("window_seconds").and_then(Value::as_u64) != Some(WINDOW_SECONDS as u64) {
        return simple_case(
            "NOT_COMPARABLE",
            "UNKNOWN",
            "annex_g_window_must_be_pinned_to_ten_seconds_for_this_profile",
            empty.clone(),
            empty,
        );
    }
    let criterion = case.get("criterion").unwrap_or(&Value::Null);
    if text(criterion, "source_dataset_member")
        != Some("parentDS.grandmasterClockQuality.clockClass")
        || text(criterion, "threshold_dataset_member")
            != Some("defaultDS.clockclassAvailabilityThreshold")
        || text(criterion, "classification_source_ref")
            .filter(|value| !value.trim().is_empty())
            .is_none()
    {
        return simple_case(
            "INVALID_INPUT",
            "UNKNOWN",
            "clockclass_source_threshold_member_or_provenance_invalid",
            empty.clone(),
            empty,
        );
    }
    let threshold_value = criterion
        .get("clockclass_availability_threshold")
        .and_then(Value::as_u64)
        .unwrap_or(u64::from(DEFAULT_CLOCKCLASS_THRESHOLD));
    if threshold_value > MAX_CLOCK_CLASS {
        return simple_case(
            "INVALID_INPUT",
            "UNKNOWN",
            "clockclass_availability_threshold_out_of_range",
            empty.clone(),
            empty,
        );
    }
    let threshold = threshold_value as u8;

    let (periods, period_positions) = match parse_periods(case) {
        Ok(parsed) => parsed,
        Err(reason) => {
            return simple_case("INVALID_INPUT", "UNKNOWN", reason, empty.clone(), empty)
        }
    };
    let samples = match parse_samples(case, &periods, &period_positions) {
        Ok(parsed) => parsed,
        Err(reason) => {
            return simple_case("INVALID_INPUT", "UNKNOWN", reason, empty.clone(), empty)
        }
    };
    let records = match parse_records(case, &periods) {
        Ok(parsed) => parsed,
        Err(reason) => {
            return simple_case("INVALID_INPUT", "UNKNOWN", reason, empty.clone(), empty)
        }
    };

    let primary = streaming_reconstruction(&samples, &periods, WINDOW_SECONDS, threshold);
    let reference = run_based_reference(&samples, &periods, WINDOW_SECONDS, threshold);
    if primary != reference {
        return simple_case(
            "REFERENCE_DISAGREEMENT",
            "UNKNOWN",
            "streaming_and_run_based_reconstruction_disagree",
            primary,
            reference,
        );
    }

    let mut reasons = Vec::new();
    let mut mismatch = false;
    let mut indeterminate = false;
    let mut total_as = 0u64;
    let mut total_uas = 0u64;

    for period in &periods {
        let reconstructed = &primary.counters[&period.id];
        let record = &records[&period.id];
        let measurement_valid = boolean(record, "measurementValid") == Some(true);
        let period_complete = boolean(record, "periodComplete") == Some(true);

        if !measurement_valid {
            indeterminate = true;
            reasons.push("measurement_valid_false");
        }
        if !period_complete {
            indeterminate = true;
            reasons.push("period_complete_false");
        }
        if reconstructed.unobserved > 0 || reconstructed.unclassified > 0 {
            indeterminate = true;
            reasons.push("unobserved_or_unclassified_seconds_present");
        }

        if measurement_valid
            && period_complete
            && reconstructed.unobserved == 0
            && reconstructed.unclassified == 0
        {
            let reported_as = record.get("AS").and_then(Value::as_u64).unwrap_or(u64::MAX);
            let reported_uas = record
                .get("UAS")
                .and_then(Value::as_u64)
                .unwrap_or(u64::MAX);
            if reported_as != reconstructed.available || reported_uas != reconstructed.unavailable {
                mismatch = true;
            } else {
                total_as = total_as.saturating_add(reported_as);
                total_uas = total_uas.saturating_add(reported_uas);
            }
        }
    }

    reasons.sort_unstable();
    reasons.dedup();
    if mismatch {
        reasons.push("reported_counters_do_not_match_annex_g_reconstruction");
        return CaseResult {
            status: "COUNTER_MISMATCH",
            final_state: primary.state.map_or("UNKNOWN", ServiceState::as_str),
            reasons,
            primary,
            reference,
            availability_ratio: None,
        };
    }
    if indeterminate {
        return CaseResult {
            status: "INDETERMINATE",
            final_state: primary.state.map_or("UNKNOWN", ServiceState::as_str),
            reasons,
            primary,
            reference,
            availability_ratio: None,
        };
    }

    CaseResult {
        status: "QUALIFIED",
        final_state: primary.state.map_or("UNKNOWN", ServiceState::as_str),
        reasons: vec!["reported_as_uas_match_reconstructed_annex_g_counts"],
        primary,
        reference,
        availability_ratio: Some((total_as, total_uas)),
    }
}

fn simple_case(
    status: &'static str,
    final_state: &'static str,
    reason: &'static str,
    primary: Reconstruction,
    reference: Reconstruction,
) -> CaseResult {
    CaseResult {
        status,
        final_state,
        reasons: vec![reason],
        primary,
        reference,
        availability_ratio: None,
    }
}

fn result_matches_expected(result: &CaseResult, expected: &Value) -> bool {
    let status = text(expected, "status");
    let final_state = text(expected, "final_state");
    let reasons = expected
        .get("reason_codes")
        .and_then(Value::as_array)
        .map(|items| {
            let mut codes: Vec<&str> = items.iter().filter_map(Value::as_str).collect();
            codes.sort_unstable();
            codes
        })
        .unwrap_or_default();
    let mut actual_reasons = result.reasons.clone();
    actual_reasons.sort_unstable();

    if status != Some(result.status)
        || final_state != Some(result.final_state)
        || reasons != actual_reasons
    {
        return false;
    }

    for (field, is_available) in [("AS_by_period", true), ("UAS_by_period", false)] {
        let Some(expected_map) = expected.get(field).and_then(Value::as_object) else {
            return false;
        };
        if expected_map.len() != result.primary.counters.len() {
            return false;
        }
        for (period_id, counters) in &result.primary.counters {
            let actual = if is_available {
                counters.available
            } else {
                counters.unavailable
            };
            if expected_map.get(period_id).and_then(Value::as_u64) != Some(actual) {
                return false;
            }
        }
    }
    true
}

pub fn validate_fixture_pack(document: &Value, source_bytes: &[u8]) -> Value {
    let cases = document.get("cases").and_then(Value::as_array);
    let mut discovered = Vec::new();
    let mut unique_ids = BTreeSet::new();
    let mut duplicate_ids = Vec::new();
    let mut case_reports = Vec::new();
    let mut failure_count = 0usize;
    let mut reference_disagreement_count = 0usize;
    let mut profile_groups: BTreeMap<String, usize> = BTreeMap::new();

    if let Some(items) = cases {
        for case in items {
            let id = text(case, "id").unwrap_or("<missing-id>");
            discovered.push(id.to_owned());
            if !unique_ids.insert(id.to_owned()) {
                duplicate_ids.push(id.to_owned());
            }
            if let Some(profile_id) =
                text(case.get("profile").unwrap_or(&Value::Null), "itu_t_profile")
            {
                *profile_groups.entry(profile_id.to_owned()).or_default() += 1;
            }

            let result = evaluate_case(case);
            let expected = case.get("expected").unwrap_or(&Value::Null);
            let expected_match = result_matches_expected(&result, expected);
            let independent_agreement = result.primary == result.reference;
            if !expected_match || !independent_agreement {
                failure_count += 1;
            }
            if !independent_agreement {
                reference_disagreement_count += 1;
            }
            case_reports.push(json!({
                "id": id,
                "profile": case.get("profile").cloned().unwrap_or(Value::Null),
                "result": result.to_value(),
                "expected": expected,
                "expected_match": expected_match,
                "passed": expected_match && independent_agreement
            }));
        }
    }

    let expected_set: BTreeSet<String> = EXPECTED_CASE_IDS
        .iter()
        .map(|id| (*id).to_owned())
        .collect();
    let discovered_set: BTreeSet<String> = discovered.iter().cloned().collect();
    let discovery_complete = discovered.len() == EXPECTED_CASE_IDS.len()
        && discovered_set == expected_set
        && duplicate_ids.is_empty();

    let standard = document.get("standard").unwrap_or(&Value::Null);
    let metadata_valid = text(document, "contract") == Some(FIXTURE_CONTRACT)
        && text(document, "version") == Some("0.2.0-draft")
        && text(document, "state") == Some("SYNTHETIC_BASE_EDITION_FIXTURES_EXECUTABLE")
        && text(standard, "recommendation") == Some("ITU-T G.8275")
        && text(standard, "edition") == Some("01/2026")
        && text(standard, "annex") == Some("G")
        && text(standard, "amendment_2026_08") == Some("NOT_APPLIED")
        && boolean(standard, "optional") == Some(true)
        && text(standard, "dataset_name") == Some("ptpServiceAvailabilityPerformanceMetricsDS")
        && text(standard, "record_name")
            == Some("ptpServiceAvailabilityPerformanceMetricsDataRecord")
        && document
            .get("default_window_seconds")
            .and_then(Value::as_u64)
            == Some(WINDOW_SECONDS as u64)
        && cases.is_some_and(|items| items.len() == EXPECTED_CASE_IDS.len())
        && cases.is_some_and(|items| items.len() <= MAX_CASES);

    let passed = metadata_valid
        && discovery_complete
        && failure_count == 0
        && reference_disagreement_count == 0;

    let mut profile_summary = serde_json::Map::new();
    for (profile, count) in profile_groups {
        profile_summary.insert(
            profile,
            json!({"case_count": count, "aggregated_with_other_profiles": false}),
        );
    }

    json!({
        "report_contract": REPORT_CONTRACT,
        "status": if passed { "passed" } else { "failed" },
        "authority": "synthetic_base_edition_clockclass_and_annex_g_state_machine_only",
        "input_sha256": sha256_hex(source_bytes),
        "recommendation": "ITU-T G.8275",
        "edition": "01/2026",
        "annex": "G",
        "profile_baseline": "02/2026",
        "profile_amendment_status": PROFILE_AMENDMENT_STATUS,
        "profile_specific_annexes": {
            "G.8275.1": "M",
            "G.8275.2": "G"
        },
        "criterion_source_dataset_member": "parentDS.grandmasterClockQuality.clockClass",
        "criterion_threshold_dataset_member": "defaultDS.clockclassAvailabilityThreshold",
        "default_clockclass_availability_threshold": DEFAULT_CLOCKCLASS_THRESHOLD,
        "clockclass_ordering": "lower_numeric_value_is_better",
        "frequency_layer_criterion_status": "FURTHER_STUDY_NOT_SCORED",
        "window_seconds": WINDOW_SECONDS,
        "case_count_expected": EXPECTED_CASE_IDS.len(),
        "case_count_discovered": cases.map_or(0, Vec::len),
        "case_ids": discovered,
        "discovery_complete": discovery_complete,
        "duplicate_case_ids": duplicate_ids,
        "metadata_valid": metadata_valid,
        "passed_count": case_reports.iter().filter(|report| report["passed"] == true).count(),
        "failure_count": failure_count,
        "reference_disagreement_count": reference_disagreement_count,
        "profile_groups": profile_summary,
        "per_case": case_reports,
        "claim_ceiling": CLAIM_CEILING,
        "non_claims": [
            "not_physical_time_error_measurement",
            "not_ptp_profile_or_device_certification",
            "not_independent_reference_instrument_qualification",
            "not_operational_service_availability_claim",
            "not_premature_recovery_or_authority_proof",
            "not_RF_or_GNSS_integrity_assessment",
            "not_G8275_2026_AMENDMENT_1_CONFORMANCE",
            "not_profile_specific_frequency_layer_availability",
            "not_announce_rate_below_one_message_per_second_assessment"
        ]
    })
}

#[cfg(test)]
mod tests {
    use super::validate_fixture_pack;
    use jsonschema::draft202012;
    use serde_json::Value;

    const FIXTURES: &str =
        include_str!("../../../docs/standards/omsi/syncaas-annex-g-fixtures-0.1.json");
    const FIXTURE_SCHEMA: &str =
        include_str!("../../../docs/standards/omsi/syncaas-annex-g-fixtures-0.1.schema.json");
    const REPORT_SCHEMA: &str = include_str!("../schema/syncaas-annex-g-report.schema.json");

    #[test]
    fn clockclass_threshold_is_inclusive_and_lower_values_are_better() {
        let nominal = super::Sample {
            period_id: "p1".to_owned(),
            clock_class_values: vec![5, 6],
            clockclass_coverage: ClockClassCoverage::Complete,
            maintenance: false,
        };
        let degraded = super::Sample {
            period_id: "p1".to_owned(),
            clock_class_values: vec![6, 7],
            clockclass_coverage: ClockClassCoverage::Complete,
            maintenance: false,
        };
        assert_eq!(
            super::classify_clockclass_primary(&nominal, 6),
            super::CriterionState::Nominal
        );
        assert_eq!(
            super::classify_clockclass_reference(&nominal, 6),
            super::CriterionState::Nominal
        );
        assert_eq!(
            super::classify_clockclass_primary(&degraded, 6),
            super::CriterionState::Degraded
        );
        assert_eq!(
            super::classify_clockclass_reference(&degraded, 6),
            super::CriterionState::Degraded
        );
    }

    #[test]
    fn configurable_clockclass_threshold_is_applied_without_reversing_order() {
        let sample = super::Sample {
            period_id: "p1".to_owned(),
            clock_class_values: vec![7],
            clockclass_coverage: ClockClassCoverage::Complete,
            maintenance: false,
        };
        assert_eq!(
            super::classify_clockclass_primary(&sample, 7),
            super::CriterionState::Nominal
        );
        assert_eq!(
            super::classify_clockclass_reference(&sample, 7),
            super::CriterionState::Nominal
        );
        assert_eq!(
            super::classify_clockclass_primary(&sample, 6),
            super::CriterionState::Degraded
        );
    }

    #[test]
    fn missing_clockclass_observation_is_not_silently_a_d1s() {
        let sample = super::Sample {
            period_id: "p1".to_owned(),
            clock_class_values: Vec::new(),
            clockclass_coverage: ClockClassCoverage::Unknown,
            maintenance: false,
        };
        assert_eq!(
            super::classify_clockclass_primary(&sample, 6),
            super::CriterionState::Unobserved
        );
        assert_eq!(
            super::classify_clockclass_reference(&sample, 6),
            super::CriterionState::Unobserved
        );
    }

    #[test]
    fn partial_clockclass_coverage_is_unobserved_even_with_good_observed_values() {
        let sample = super::Sample {
            period_id: "p1".to_owned(),
            clock_class_values: vec![5, 6],
            clockclass_coverage: super::ClockClassCoverage::Partial,
            maintenance: false,
        };
        assert_eq!(super::classify_clockclass_primary(&sample, 6), super::CriterionState::Unobserved);
        assert_eq!(super::classify_clockclass_reference(&sample, 6), super::CriterionState::Unobserved);
    }

    #[test]
    fn maintenance_pauses_the_ten_second_consecutive_measurement_window() {
        use super::{ClockClassCoverage, Period, Sample, ServiceState};
        let mut samples = Vec::new();
        for _ in 0..9 {
            samples.push(Sample {
                period_id: "p1".to_owned(),
                clock_class_values: vec![7],
                clockclass_coverage: ClockClassCoverage::Complete,
                maintenance: false,
            });
        }
        samples.push(Sample {
            period_id: "p1".to_owned(),
            clock_class_values: Vec::new(),
            clockclass_coverage: ClockClassCoverage::MaintenancePaused,
            maintenance: true,
        });
        samples.push(Sample {
            period_id: "p1".to_owned(),
            clock_class_values: vec![7],
            clockclass_coverage: ClockClassCoverage::Complete,
            maintenance: false,
        });
        let periods = vec![Period {
            id: "p1".to_owned(),
            wall_seconds: 11,
            maintenance_paused_seconds: 1,
        }];
        let primary = super::streaming_reconstruction(&samples, &periods, 10, 6);
        let reference = super::run_based_reference(&samples, &periods, 10, 6);
        assert_eq!(primary, reference);
        assert_eq!(primary.state, Some(ServiceState::Unavailable));
        assert_eq!(primary.counters["p1"].unavailable, 10);
    }

    #[test]
    fn primary_and_reference_match_for_all_three_state_sequences_through_ten_seconds() {
        use super::{ClockClassCoverage, Period, Sample};
        for length in 1..=10usize {
            let combinations = 3usize.pow(length as u32);
            for mut encoded in 0..combinations {
                let mut samples = Vec::with_capacity(length);
                for _ in 0..length {
                    let symbol = encoded % 3;
                    encoded /= 3;
                    let (values, coverage) = match symbol {
                        0 => (vec![5], ClockClassCoverage::Complete),
                        1 => (vec![7], ClockClassCoverage::Complete),
                        _ => (vec![5, 6], ClockClassCoverage::Partial),
                    };
                    samples.push(Sample {
                        period_id: "p1".to_owned(),
                        clock_class_values: values,
                        clockclass_coverage: coverage,
                        maintenance: false,
                    });
                }
                let periods = vec![Period {
                    id: "p1".to_owned(),
                    wall_seconds: length as u64,
                    maintenance_paused_seconds: 0,
                }];
                let primary = super::streaming_reconstruction(&samples, &periods, 10, 6);
                let reference = super::run_based_reference(&samples, &periods, 10, 6);
                assert_eq!(primary, reference, "state sequence length={length}");
            }
        }
    }

    #[test]
    fn frozen_annex_g_fixture_pack_executes_all_fourteen_cases() {
        let document: Value = serde_json::from_str(FIXTURES).expect("SyncaaS fixtures parse");
        let schema: Value = serde_json::from_str(FIXTURE_SCHEMA).expect("fixture schema parses");
        assert!(
            draft202012::meta::is_valid(&schema),
            "input schema is not Draft 2020-12"
        );
        let validator = draft202012::new(&schema).expect("input schema compiles");
        assert!(
            validator.is_valid(&document),
            "fixture pack does not match schema"
        );

        let report = validate_fixture_pack(&document, FIXTURES.as_bytes());
        assert_eq!(report["status"], "passed", "{report:#}");
        assert_eq!(report["case_count_expected"], 14);
        assert_eq!(report["case_count_discovered"], 14);
        assert_eq!(report["passed_count"], 14);
        assert_eq!(report["failure_count"], 0);
        assert_eq!(report["reference_disagreement_count"], 0);

        let report_schema: Value =
            serde_json::from_str(REPORT_SCHEMA).expect("report schema parses");
        assert!(
            draft202012::meta::is_valid(&report_schema),
            "report schema is not Draft 2020-12"
        );
        let report_validator = draft202012::new(&report_schema).expect("report schema compiles");
        assert!(
            report_validator.is_valid(&report),
            "report does not match schema: {report:#}"
        );
    }

    #[test]
    fn exact_case_discovery_fails_closed() {
        let mut document: Value = serde_json::from_str(FIXTURES).unwrap();
        document["cases"].as_array_mut().unwrap().pop();
        let report = validate_fixture_pack(&document, b"mutated");
        assert_eq!(report["status"], "failed");
        assert_eq!(report["discovery_complete"], false);
    }

    #[test]
    fn record_counter_tampering_is_not_smoothed_into_an_availability_ratio() {
        let mut document: Value = serde_json::from_str(FIXTURES).unwrap();
        let case = document["cases"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|case| case["id"] == "SVC-SUPPORTED-NOMINAL-001")
            .unwrap();
        case["records"][0]["AS"] = Value::from(9u64);
        let report = validate_fixture_pack(&document, b"mutated-counter");
        assert_eq!(report["status"], "failed");
        assert!(report["per_case"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["id"] == "SVC-SUPPORTED-NOMINAL-001"
                && entry["result"]["status"] == "COUNTER_MISMATCH"));
    }
}
