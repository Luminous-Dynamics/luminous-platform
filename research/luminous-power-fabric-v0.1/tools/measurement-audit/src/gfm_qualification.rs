use crate::AuditError;
use std::collections::BTreeSet;

const CLAIM: &str =
    "GFM_CAPABILITY_EVIDENCE_COVERAGE_ONLY_NOT_STANDARD_CONFORMITY_OR_FIELD_SUPERIORITY";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GfmTestId {
    VoltageFrequencyAdaptability, PowerQuality, ActivePowerControl, ReactivePowerControl,
    PrimaryFrequencyRegulation, VoltageReactivePowerRegulation, InertiaResponse,
    PowerOscillationDamping, VoltageTolerance, PhaseAngleJumpTolerance, GfmGflModeSwitching,
    EquipmentBlackStart, BlackStartFromZeroVoltage, IslandedLoadPickup, GridLossResynchronization,
    LoadFollowingDroop, WeakGridScrVariation, UnbalancedFaultNegativeSequence,
    MultiConverterMultiVendor, SystemDisturbanceStudy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GfmCapability {
    BasicPcsFunctionality, ActivePowerControl, ReactivePowerControl, PrimaryFrequencySupport,
    VoltageSupport, InertiaResponse, PowerOscillationDamping, VoltageRideThrough,
    PhaseAngleJumpResponse, GfmGflModeSwitching, EquipmentBlackStart, AutonomousBlackStart,
    IslandedLoadPickup, GridLossResynchronization, LoadFollowingDroop, WeakGridStability,
    NegativeSequenceFaultSupport, MultiVendorInteroperability, SystemDisturbanceResilience,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationMethod {
    RmsSimulation, EmtSimulation, HybridRmsEmtCoSimulation, ControllerHardwareInLoop,
    PowerHardwareInLoop, PcsLabGridEmulator, SiteCommissioning, FieldOperation,
    LargeScaleSystemSimulation,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceClass { SyntheticFixture, VendorAssertion, Modelled, Measured, Witnessed, Accredited }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceStatus { Current, Expired, Revoked }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestOutcome { Passed, Failed, NotRun }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GfmClaimLevel {
    SyntheticCoverageOnly, ModelValidated, ControllerHIL, PowerHardwareTested,
    SiteDemonstrated, IndependentSiteClaim,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotApplicableTest {
    pub test_id: GfmTestId,
    pub rationale: String,
    pub approved_by_reference: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GfmTestConditions {
    pub initial_active_power_milli_kw: i64,
    pub initial_reactive_power_milli_kvar: i64,
    pub initial_voltage_pu_micros: u32,
    pub initial_frequency_millihz: u32,
    pub initial_soc_ppm: Option<u32>,
    pub energy_headroom_wh: Option<u64>,
    pub short_circuit_ratio_milli: Option<u32>,
    pub x_over_r_milli: Option<u32>,
    pub disturbance_kind: String,
    pub disturbance_magnitude_milli: u64,
    pub disturbance_unit: String,
    pub disturbance_duration_ms: u64,
    pub disturbance_location: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GfmMeasurementConditions {
    pub bandwidth_hz_milli: u64,
    pub sample_rate_hz_milli: u64,
    pub expanded_uncertainty_ppm: u32,
    pub uncertainty_reference: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GfmTestEvidence {
    pub evidence_record_id: String,
    pub test_id: GfmTestId,
    pub method: ValidationMethod,
    pub outcome: TestOutcome,
    pub evidence_class: EvidenceClass,
    pub evidence_status: EvidenceStatus,
    pub product_configuration_sha256: String,
    pub scenario_manifest_sha256: String,
    pub acceptance_criteria_sha256: String,
    pub evidence_bundle_sha256: String,
    pub measurement_boundary: String,
    pub test_conditions: GfmTestConditions,
    pub measurement_conditions: GfmMeasurementConditions,
    pub raw_data_reference: String,
    pub calibration_reference: String,
    pub timebase_reference: String,
    pub witness_reference: String,
    pub measured_metric_ids: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GfmQualificationInput {
    pub claim_id: String,
    pub claim_profile_sha256: String,
    pub claim_level: GfmClaimLevel,
    pub requested_capabilities: Vec<GfmCapability>,
    pub not_applicable_tests: Vec<NotApplicableTest>,
    pub evidence_records: Vec<GfmTestEvidence>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GfmQualificationReport {
    pub claim_id: String,
    pub claim_level: GfmClaimLevel,
    pub required_test_ids: Vec<GfmTestId>,
    pub passed_test_ids: Vec<GfmTestId>,
    pub missing_test_ids: Vec<GfmTestId>,
    pub invalid_test_ids: Vec<GfmTestId>,
    pub not_applicable_test_ids: Vec<GfmTestId>,
    pub coverage_complete: bool,
    pub claim_eligible: bool,
    pub physical_evidence_eligible: bool,
    pub claim_ceiling: &'static str,
}
impl GfmQualificationReport {
    pub const fn claim_ceiling(&self) -> &'static str { CLAIM }
}

fn nonblank(value: &str) -> bool { !value.trim().is_empty() }
fn valid_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn tests_for_capability(capability: GfmCapability) -> &'static [GfmTestId] {
    match capability {
        GfmCapability::BasicPcsFunctionality => &[GfmTestId::VoltageFrequencyAdaptability, GfmTestId::PowerQuality],
        GfmCapability::ActivePowerControl => &[GfmTestId::ActivePowerControl],
        GfmCapability::ReactivePowerControl => &[GfmTestId::ReactivePowerControl],
        GfmCapability::PrimaryFrequencySupport => &[GfmTestId::PrimaryFrequencyRegulation],
        GfmCapability::VoltageSupport => &[GfmTestId::VoltageReactivePowerRegulation, GfmTestId::VoltageTolerance],
        GfmCapability::InertiaResponse => &[GfmTestId::InertiaResponse],
        GfmCapability::PowerOscillationDamping => &[GfmTestId::PowerOscillationDamping],
        GfmCapability::VoltageRideThrough => &[GfmTestId::VoltageTolerance],
        GfmCapability::PhaseAngleJumpResponse => &[GfmTestId::PhaseAngleJumpTolerance],
        GfmCapability::GfmGflModeSwitching => &[GfmTestId::GfmGflModeSwitching],
        GfmCapability::EquipmentBlackStart => &[GfmTestId::EquipmentBlackStart],
        GfmCapability::AutonomousBlackStart => &[GfmTestId::BlackStartFromZeroVoltage],
        GfmCapability::IslandedLoadPickup => &[GfmTestId::IslandedLoadPickup],
        GfmCapability::GridLossResynchronization => &[GfmTestId::GridLossResynchronization],
        GfmCapability::LoadFollowingDroop => &[GfmTestId::LoadFollowingDroop],
        GfmCapability::WeakGridStability => &[GfmTestId::WeakGridScrVariation],
        GfmCapability::NegativeSequenceFaultSupport => &[GfmTestId::UnbalancedFaultNegativeSequence],
        GfmCapability::MultiVendorInteroperability => &[GfmTestId::MultiConverterMultiVendor],
        GfmCapability::SystemDisturbanceResilience => &[GfmTestId::SystemDisturbanceStudy],
    }
}

fn allowed_methods(test: GfmTestId) -> &'static [ValidationMethod] {
    use ValidationMethod::*;
    match test {
        GfmTestId::VoltageFrequencyAdaptability | GfmTestId::PowerQuality
        | GfmTestId::ActivePowerControl | GfmTestId::ReactivePowerControl
        | GfmTestId::PrimaryFrequencyRegulation | GfmTestId::VoltageReactivePowerRegulation
        | GfmTestId::InertiaResponse | GfmTestId::PowerOscillationDamping
        | GfmTestId::VoltageTolerance | GfmTestId::PhaseAngleJumpTolerance
        | GfmTestId::GfmGflModeSwitching =>
            &[EmtSimulation, PcsLabGridEmulator, PowerHardwareInLoop, SiteCommissioning, FieldOperation],
        GfmTestId::EquipmentBlackStart => &[SiteCommissioning, FieldOperation],
        GfmTestId::BlackStartFromZeroVoltage | GfmTestId::IslandedLoadPickup
        | GfmTestId::GridLossResynchronization | GfmTestId::LoadFollowingDroop =>
            &[ControllerHardwareInLoop, SiteCommissioning, FieldOperation],
        GfmTestId::WeakGridScrVariation =>
            &[EmtSimulation, ControllerHardwareInLoop, PowerHardwareInLoop, SiteCommissioning, LargeScaleSystemSimulation],
        GfmTestId::UnbalancedFaultNegativeSequence =>
            &[PcsLabGridEmulator, PowerHardwareInLoop, SiteCommissioning, FieldOperation],
        GfmTestId::MultiConverterMultiVendor =>
            &[ControllerHardwareInLoop, PowerHardwareInLoop, SiteCommissioning, FieldOperation],
        GfmTestId::SystemDisturbanceStudy =>
            &[RmsSimulation, EmtSimulation, HybridRmsEmtCoSimulation, LargeScaleSystemSimulation],
    }
}

fn required_metrics(test: GfmTestId) -> &'static [&'static str] {
    match test {
        GfmTestId::VoltageFrequencyAdaptability => &["terminal_voltage", "frequency", "trip_state"],
        GfmTestId::PowerQuality => &["voltage_thd", "current_thd", "harmonic_spectrum"],
        GfmTestId::ActivePowerControl => &["active_power_command", "active_power_measured", "response_time", "settling_time"],
        GfmTestId::ReactivePowerControl => &["reactive_power_command", "reactive_power_measured", "response_time", "settling_time"],
        GfmTestId::PrimaryFrequencyRegulation => &["frequency_deviation", "active_power_response", "droop_slope", "response_time"],
        GfmTestId::VoltageReactivePowerRegulation => &["voltage_deviation", "reactive_power_response", "steady_state_error"],
        GfmTestId::InertiaResponse => &["rocof", "inertia_power_response", "response_onset", "peak_current", "sustain_duration"],
        GfmTestId::PowerOscillationDamping => &["oscillation_frequency", "damping_ratio", "decay_time", "power_modulation"],
        GfmTestId::VoltageTolerance => &["voltage_time_envelope", "peak_current", "current_limit_duration", "voltage_recovery"],
        GfmTestId::PhaseAngleJumpTolerance => &["phase_angle_step", "active_power_excursion", "peak_current", "synchronism", "settling_time"],
        GfmTestId::GfmGflModeSwitching => &["mode_transition", "voltage_excursion", "current_excursion", "power_transient", "post_switch_stability"],
        GfmTestId::EquipmentBlackStart => &["voltage_build_up_time", "terminal_voltage", "frequency", "energization_current", "dc_energy_margin"],
        GfmTestId::BlackStartFromZeroVoltage => &["zero_voltage_start", "voltage_trajectory", "energization_inrush", "energy_margin", "load_restoration"],
        GfmTestId::IslandedLoadPickup => &["frequency_nadir", "frequency_zenith", "rocof", "voltage_deviation", "recovery_time", "dc_energy_use"],
        GfmTestId::GridLossResynchronization => &["grid_loss_excursion", "island_stabilization_time", "resynchronization_mismatch", "reconnection_transient", "protection_coordination"],
        GfmTestId::LoadFollowingDroop => &["active_power_tracking", "reactive_power_sharing", "frequency_droop", "voltage_droop", "overshoot", "settling_time"],
        GfmTestId::WeakGridScrVariation => &["short_circuit_ratio", "x_over_r", "oscillation_damping", "resonance_location", "stability_envelope"],
        GfmTestId::UnbalancedFaultNegativeSequence => &["negative_sequence_current", "phase_current_imbalance", "current_limit_use", "dc_link_ripple", "fault_recovery"],
        GfmTestId::MultiConverterMultiVendor => &["active_power_sharing", "reactive_power_sharing", "circulating_power", "interaction_damping", "stability_margin"],
        GfmTestId::SystemDisturbanceStudy => &["system_rocof", "frequency_nadir", "post_fault_recovery", "voltage_trajectory", "inter_area_damping", "power_redistribution"],
    }
}

fn is_simulation(method: ValidationMethod) -> bool {
    matches!(method, ValidationMethod::RmsSimulation | ValidationMethod::EmtSimulation
        | ValidationMethod::HybridRmsEmtCoSimulation | ValidationMethod::LargeScaleSystemSimulation)
}
fn is_physical(method: ValidationMethod) -> bool {
    matches!(method, ValidationMethod::ControllerHardwareInLoop | ValidationMethod::PowerHardwareInLoop
        | ValidationMethod::PcsLabGridEmulator | ValidationMethod::SiteCommissioning | ValidationMethod::FieldOperation)
}

fn evidence_meets_claim_level(record: &GfmTestEvidence, level: GfmClaimLevel) -> bool {
    match level {
        GfmClaimLevel::SyntheticCoverageOnly => record.evidence_class == EvidenceClass::SyntheticFixture,
        GfmClaimLevel::ModelValidated => record.evidence_class == EvidenceClass::Modelled && is_simulation(record.method),
        GfmClaimLevel::ControllerHIL => matches!(record.evidence_class, EvidenceClass::Measured | EvidenceClass::Witnessed | EvidenceClass::Accredited)
            && record.method == ValidationMethod::ControllerHardwareInLoop,
        GfmClaimLevel::PowerHardwareTested => matches!(record.evidence_class, EvidenceClass::Measured | EvidenceClass::Witnessed | EvidenceClass::Accredited)
            && matches!(record.method, ValidationMethod::PowerHardwareInLoop | ValidationMethod::PcsLabGridEmulator),
        GfmClaimLevel::SiteDemonstrated => matches!(record.evidence_class, EvidenceClass::Measured | EvidenceClass::Witnessed | EvidenceClass::Accredited)
            && matches!(record.method, ValidationMethod::SiteCommissioning | ValidationMethod::FieldOperation),
        GfmClaimLevel::IndependentSiteClaim => matches!(record.evidence_class, EvidenceClass::Witnessed | EvidenceClass::Accredited)
            && matches!(record.method, ValidationMethod::SiteCommissioning | ValidationMethod::FieldOperation)
            && nonblank(&record.witness_reference),
    }
}

fn evidence_is_complete(record: &GfmTestEvidence) -> bool {
    let c=&record.test_conditions;
    let m=&record.measurement_conditions;
    let energy_required=matches!(record.test_id,
        GfmTestId::InertiaResponse|GfmTestId::EquipmentBlackStart|GfmTestId::BlackStartFromZeroVoltage
        |GfmTestId::IslandedLoadPickup|GfmTestId::GridLossResynchronization);
    let weak_grid=record.test_id==GfmTestId::WeakGridScrVariation;
    if c.initial_voltage_pu_micros==0 || c.initial_frequency_millihz==0
        || c.initial_soc_ppm.is_some_and(|v|v>1_000_000)
        || (energy_required&&!c.energy_headroom_wh.is_some_and(|v|v>0))
        || (weak_grid&&(!c.short_circuit_ratio_milli.is_some_and(|v|v>0)||!c.x_over_r_milli.is_some_and(|v|v>0)))
        || c.short_circuit_ratio_milli.is_some_and(|v|v==0)
        || c.x_over_r_milli.is_some_and(|v|v==0)
        || !nonblank(&c.disturbance_kind)||c.disturbance_magnitude_milli==0
        || !nonblank(&c.disturbance_unit)||c.disturbance_duration_ms==0
        || !nonblank(&c.disturbance_location)
        || m.bandwidth_hz_milli==0
        || m.sample_rate_hz_milli<m.bandwidth_hz_milli.saturating_mul(2)
        || m.expanded_uncertainty_ppm==0||!nonblank(&m.uncertainty_reference)
        || !valid_digest(&record.product_configuration_sha256)
        || !valid_digest(&record.scenario_manifest_sha256)
        || !valid_digest(&record.acceptance_criteria_sha256)
        || !valid_digest(&record.evidence_bundle_sha256)
        || !nonblank(&record.measurement_boundary)||!nonblank(&record.raw_data_reference)
        || record.measured_metric_ids.iter().any(|x|!nonblank(x)) {
        return false;
    }
    if is_physical(record.method)&&(!nonblank(&record.calibration_reference)||!nonblank(&record.timebase_reference)){return false;}
    if matches!(record.evidence_class,EvidenceClass::Witnessed|EvidenceClass::Accredited)&&!nonblank(&record.witness_reference){return false;}
    true
}

fn record_satisfies_test(record: &GfmTestEvidence, test: GfmTestId, level: GfmClaimLevel) -> bool {
    record.test_id == test && record.outcome == TestOutcome::Passed
        && record.evidence_status == EvidenceStatus::Current
        && allowed_methods(test).contains(&record.method)
        && evidence_meets_claim_level(record, level) && evidence_is_complete(record)
        && required_metrics(test).iter().all(|required| record.measured_metric_ids.iter().any(|actual| actual == required))
}

/// Read-only claim-to-test coverage evaluator. It checks method/evidence tier,
/// not raw waveforms, acceptance-threshold adequacy, or standards conformity.
pub fn evaluate_gfm_qualification(input: &GfmQualificationInput) -> Result<GfmQualificationReport, AuditError> {
    if !nonblank(&input.claim_id) || !valid_digest(&input.claim_profile_sha256) || input.requested_capabilities.is_empty() {
        return Err(AuditError("GFM claim metadata or requested capability set is invalid".to_string()));
    }
    let mut capabilities = BTreeSet::new();
    let mut required = BTreeSet::new();
    for capability in &input.requested_capabilities {
        if !capabilities.insert(*capability) {
            return Err(AuditError("requested GFM capabilities must be unique".to_string()));
        }
        for test in tests_for_capability(*capability) { required.insert(*test); }
    }

    let mut not_applicable = BTreeSet::new();
    for exclusion in &input.not_applicable_tests {
        if !not_applicable.insert(exclusion.test_id) || !nonblank(&exclusion.rationale)
            || !nonblank(&exclusion.approved_by_reference) || required.contains(&exclusion.test_id) {
            return Err(AuditError(format!("test {:?} has duplicate, unapproved, or claim-conflicting non-applicability", exclusion.test_id)));
        }
    }
    let mut record_ids = BTreeSet::new();
    for record in &input.evidence_records {
        if !nonblank(&record.evidence_record_id) || !record_ids.insert(record.evidence_record_id.as_str()) {
            return Err(AuditError("GFM evidence record IDs must be non-empty and unique".to_string()));
        }
    }

    let mut passed = Vec::new();
    let mut missing = Vec::new();
    let mut invalid = Vec::new();
    for test in &required {
        let records: Vec<&GfmTestEvidence> = input.evidence_records.iter().filter(|record| record.test_id == *test).collect();
        if records.is_empty() { missing.push(*test); continue; }
        let failed_current = records.iter().any(|record| record.evidence_status == EvidenceStatus::Current && record.outcome == TestOutcome::Failed);
        let valid_pass = records.iter().any(|record| record_satisfies_test(record, *test, input.claim_level));
        if failed_current || !valid_pass { invalid.push(*test); } else { passed.push(*test); }
    }
    let complete = missing.is_empty() && invalid.is_empty();
    Ok(GfmQualificationReport {
        claim_id: input.claim_id.clone(), claim_level: input.claim_level,
        required_test_ids: required.into_iter().collect(), passed_test_ids: passed,
        missing_test_ids: missing, invalid_test_ids: invalid, not_applicable_test_ids: not_applicable.into_iter().collect(),
        coverage_complete: complete, claim_eligible: complete,
        physical_evidence_eligible: complete && input.claim_level != GfmClaimLevel::SyntheticCoverageOnly,
        claim_ceiling: CLAIM,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn h(c: char) -> String { std::iter::repeat(c).take(64).collect() }

    fn capabilities() -> Vec<GfmCapability> {
        vec![
            GfmCapability::BasicPcsFunctionality,GfmCapability::ActivePowerControl,GfmCapability::ReactivePowerControl,
            GfmCapability::PrimaryFrequencySupport,GfmCapability::VoltageSupport,GfmCapability::InertiaResponse,
            GfmCapability::PowerOscillationDamping,GfmCapability::VoltageRideThrough,GfmCapability::PhaseAngleJumpResponse,
            GfmCapability::GfmGflModeSwitching,GfmCapability::EquipmentBlackStart,GfmCapability::AutonomousBlackStart,
            GfmCapability::IslandedLoadPickup,GfmCapability::GridLossResynchronization,GfmCapability::LoadFollowingDroop,
            GfmCapability::WeakGridStability,GfmCapability::NegativeSequenceFaultSupport,GfmCapability::MultiVendorInteroperability,
        ]
    }
    fn method(test: GfmTestId, level: GfmClaimLevel) -> ValidationMethod {
        match level {
            GfmClaimLevel::ModelValidated => ValidationMethod::EmtSimulation,
            GfmClaimLevel::ControllerHIL if matches!(test,GfmTestId::BlackStartFromZeroVoltage|GfmTestId::IslandedLoadPickup|GfmTestId::GridLossResynchronization|GfmTestId::LoadFollowingDroop|GfmTestId::MultiConverterMultiVendor|GfmTestId::WeakGridScrVariation) => ValidationMethod::ControllerHardwareInLoop,
            GfmClaimLevel::SiteDemonstrated|GfmClaimLevel::IndependentSiteClaim => ValidationMethod::SiteCommissioning,
            _ if test==GfmTestId::EquipmentBlackStart||test==GfmTestId::WeakGridScrVariation => ValidationMethod::SiteCommissioning,
            _ if matches!(test,GfmTestId::BlackStartFromZeroVoltage|GfmTestId::IslandedLoadPickup|GfmTestId::GridLossResynchronization|GfmTestId::LoadFollowingDroop|GfmTestId::MultiConverterMultiVendor) => ValidationMethod::ControllerHardwareInLoop,
            _ => ValidationMethod::PcsLabGridEmulator,
        }
    }
    fn record(test:GfmTestId,level:GfmClaimLevel,id:&str)->GfmTestEvidence {
        let (class,method) = match level {
            GfmClaimLevel::SyntheticCoverageOnly => (EvidenceClass::SyntheticFixture,method(test,level)),
            GfmClaimLevel::ModelValidated => (EvidenceClass::Modelled,method(test,level)),
            GfmClaimLevel::IndependentSiteClaim => (EvidenceClass::Accredited,method(test,level)),
            _ => (EvidenceClass::Measured,method(test,level)),
        };
        GfmTestEvidence{
            evidence_record_id:id.into(),test_id:test,method,outcome:TestOutcome::Passed,evidence_class:class,
            evidence_status:EvidenceStatus::Current,product_configuration_sha256:h('a'),scenario_manifest_sha256:h('b'),
            acceptance_criteria_sha256:h('c'),evidence_bundle_sha256:h('d'),measurement_boundary:"PCS-terminal-or-declared-POC".into(),
            test_conditions:GfmTestConditions {
                initial_active_power_milli_kw:500,initial_reactive_power_milli_kvar:100,
                initial_voltage_pu_micros:1_000_000,initial_frequency_millihz:50_000,initial_soc_ppm:Some(800_000),
                energy_headroom_wh:if matches!(test,GfmTestId::InertiaResponse|GfmTestId::EquipmentBlackStart|GfmTestId::BlackStartFromZeroVoltage|GfmTestId::IslandedLoadPickup|GfmTestId::GridLossResynchronization){Some(100_000)}else{None},
                short_circuit_ratio_milli:if test==GfmTestId::WeakGridScrVariation{Some(3_000)}else{None},
                x_over_r_milli:if test==GfmTestId::WeakGridScrVariation{Some(10_000)}else{None},
                disturbance_kind:"load_step".into(),disturbance_magnitude_milli:100,disturbance_unit:"percent_rated_power".into(),
                disturbance_duration_ms:500,disturbance_location:"pcc-01".into(),
            },
            measurement_conditions:GfmMeasurementConditions{bandwidth_hz_milli:5_000_000,sample_rate_hz_milli:20_000_000,
                expanded_uncertainty_ppm:1_000,uncertainty_reference:"synthetic://uncertainty".into()},
            raw_data_reference:"evidence://raw".into(),
            calibration_reference:if is_physical(method){"calibration://current"}else{""}.into(),
            timebase_reference:if is_physical(method){"timebase://traceable"}else{""}.into(),
            witness_reference:if matches!(class,EvidenceClass::Witnessed|EvidenceClass::Accredited){"witness://qualified"}else{""}.into(),
            measured_metric_ids:required_metrics(test).iter().map(|x|x.to_string()).collect(),
        }
    }
    fn input(level:GfmClaimLevel)->GfmQualificationInput{
        let requested_capabilities=capabilities();
        let mut tests=BTreeSet::new();
        for capability in &requested_capabilities{for test in tests_for_capability(*capability){tests.insert(*test);}}
        let evidence_records=tests.into_iter().enumerate().map(|(n,test)|record(test,level,&format!("record-{n:02}"))).collect();
        GfmQualificationInput{claim_id:"test-gfm".into(),claim_profile_sha256:h('e'),claim_level:level,requested_capabilities,not_applicable_tests:vec![],evidence_records}
    }

    #[test] fn synthetic_profile_covers_nineteen_test_ids_only(){
        let r=evaluate_gfm_qualification(&input(GfmClaimLevel::SyntheticCoverageOnly)).unwrap();
        assert_eq!(r.required_test_ids.len(),19);assert!(r.coverage_complete);assert!(!r.physical_evidence_eligible);
    }
    #[test] fn generic_ibr_tests_do_not_establish_inertia_phase_jump_or_black_start(){
        let x=GfmQualificationInput{claim_id:"claim".into(),claim_profile_sha256:h('e'),claim_level:GfmClaimLevel::PowerHardwareTested,
            requested_capabilities:vec![GfmCapability::InertiaResponse,GfmCapability::PhaseAngleJumpResponse,GfmCapability::AutonomousBlackStart],
            not_applicable_tests:vec![],evidence_records:vec![
                record(GfmTestId::VoltageFrequencyAdaptability,GfmClaimLevel::PowerHardwareTested,"b1"),
                record(GfmTestId::PowerQuality,GfmClaimLevel::PowerHardwareTested,"b2"),
                record(GfmTestId::ActivePowerControl,GfmClaimLevel::PowerHardwareTested,"b3"),
                record(GfmTestId::ReactivePowerControl,GfmClaimLevel::PowerHardwareTested,"b4")]};
        let r=evaluate_gfm_qualification(&x).unwrap();assert!(!r.claim_eligible);
        assert!(r.missing_test_ids.contains(&GfmTestId::InertiaResponse));
        assert!(r.missing_test_ids.contains(&GfmTestId::PhaseAngleJumpTolerance));
        assert!(r.missing_test_ids.contains(&GfmTestId::BlackStartFromZeroVoltage));
    }
    #[test] fn modelled_emt_cannot_satisfy_power_hardware_claim(){
        let mut x=input(GfmClaimLevel::PowerHardwareTested);x.evidence_records[0].evidence_class=EvidenceClass::Modelled;x.evidence_records[0].method=ValidationMethod::EmtSimulation;
        let r=evaluate_gfm_qualification(&x).unwrap();assert!(r.invalid_test_ids.contains(&x.evidence_records[0].test_id));
    }
    #[test] fn physical_record_requires_calibration_and_timebase(){
        let mut x=input(GfmClaimLevel::PowerHardwareTested);x.evidence_records[0].calibration_reference.clear();
        let r=evaluate_gfm_qualification(&x).unwrap();assert!(r.invalid_test_ids.contains(&x.evidence_records[0].test_id));
    }
    #[test] fn required_metrics_cannot_be_omitted(){
        let mut x=input(GfmClaimLevel::SyntheticCoverageOnly);x.evidence_records[0].measured_metric_ids.pop();
        let r=evaluate_gfm_qualification(&x).unwrap();assert!(r.invalid_test_ids.contains(&x.evidence_records[0].test_id));
    }
    #[test] fn current_failed_repeat_cannot_be_hidden(){
        let mut x=input(GfmClaimLevel::SyntheticCoverageOnly);let mut f=x.evidence_records[0].clone();f.evidence_record_id="failed".into();f.outcome=TestOutcome::Failed;x.evidence_records.push(f);
        let r=evaluate_gfm_qualification(&x).unwrap();assert!(r.invalid_test_ids.contains(&x.evidence_records[0].test_id));
    }
    #[test] fn expired_record_does_not_qualify(){
        let mut x=input(GfmClaimLevel::SyntheticCoverageOnly);x.evidence_records[0].evidence_status=EvidenceStatus::Expired;
        let r=evaluate_gfm_qualification(&x).unwrap();assert!(r.invalid_test_ids.contains(&x.evidence_records[0].test_id));
    }
    #[test] fn not_applicable_requires_approval_and_cannot_overlap_claim(){
        let mut x=input(GfmClaimLevel::SyntheticCoverageOnly);x.not_applicable_tests.push(NotApplicableTest{test_id:GfmTestId::GfmGflModeSwitching,rationale:"not claimed".into(),approved_by_reference:"approval://scope".into()});
        assert!(evaluate_gfm_qualification(&x).is_err());x.requested_capabilities.retain(|c|*c!=GfmCapability::GfmGflModeSwitching);
        x.not_applicable_tests[0].approved_by_reference.clear();assert!(evaluate_gfm_qualification(&x).is_err());
        x.not_applicable_tests[0].approved_by_reference="approval://scope".into();assert!(evaluate_gfm_qualification(&x).is_ok());
    }
    #[test] fn independent_site_claim_requires_accredited_witness(){
        let mut x=input(GfmClaimLevel::IndependentSiteClaim);x.evidence_records[0].evidence_class=EvidenceClass::Measured;
        let r=evaluate_gfm_qualification(&x).unwrap();assert!(r.invalid_test_ids.contains(&x.evidence_records[0].test_id));
    }
    #[test] fn duplicate_evidence_ids_are_rejected(){
        let mut x=input(GfmClaimLevel::SyntheticCoverageOnly);x.evidence_records[1].evidence_record_id=x.evidence_records[0].evidence_record_id.clone();
        assert!(evaluate_gfm_qualification(&x).is_err());
    }
    #[test] fn duplicate_capability_claims_are_rejected(){
        let mut x=input(GfmClaimLevel::SyntheticCoverageOnly);x.requested_capabilities.push(x.requested_capabilities[0]);
        assert!(evaluate_gfm_qualification(&x).is_err());
    }
    #[test] fn invalid_claim_profile_digest_is_rejected(){
        let mut x=input(GfmClaimLevel::SyntheticCoverageOnly);x.claim_profile_sha256="bad".into();
        assert!(evaluate_gfm_qualification(&x).is_err());
    }
    #[test] fn site_claim_requires_site_method_and_has_physical_eligibility(){
        let r=evaluate_gfm_qualification(&input(GfmClaimLevel::SiteDemonstrated)).unwrap();assert!(r.claim_eligible);assert!(r.physical_evidence_eligible);
    }
    #[test] fn synthetic_evidence_cannot_satisfy_physical_claim(){
        let mut x=input(GfmClaimLevel::PowerHardwareTested);for e in &mut x.evidence_records{e.evidence_class=EvidenceClass::SyntheticFixture;}
        let r=evaluate_gfm_qualification(&x).unwrap();assert!(!r.claim_eligible);assert!(!r.physical_evidence_eligible);
    }
    #[test] fn malformed_bundle_digest_invalidates_test(){
        let mut x=input(GfmClaimLevel::SyntheticCoverageOnly);x.evidence_records[0].evidence_bundle_sha256="bad".into();
        let r=evaluate_gfm_qualification(&x).unwrap();assert!(r.invalid_test_ids.contains(&x.evidence_records[0].test_id));
    }
    #[test] fn requested_scope_determines_exact_test_set(){
        let mut x=input(GfmClaimLevel::SyntheticCoverageOnly);x.requested_capabilities=vec![GfmCapability::PrimaryFrequencySupport];
        x.evidence_records.retain(|e|e.test_id==GfmTestId::PrimaryFrequencyRegulation);
        let r=evaluate_gfm_qualification(&x).unwrap();assert_eq!(r.required_test_ids,vec![GfmTestId::PrimaryFrequencyRegulation]);assert!(r.claim_eligible);
    }

    #[test]
    fn weak_grid_claim_requires_scr_and_x_over_r() {
        let mut x=input(GfmClaimLevel::SyntheticCoverageOnly);
        let i=x.evidence_records.iter().position(|r|r.test_id==GfmTestId::WeakGridScrVariation).unwrap();
        x.evidence_records[i].test_conditions.short_circuit_ratio_milli=None;
        assert!(evaluate_gfm_qualification(&x).unwrap().invalid_test_ids.contains(&GfmTestId::WeakGridScrVariation));
    }
    #[test]
    fn inertia_claim_requires_energy_headroom() {
        let mut x=input(GfmClaimLevel::SyntheticCoverageOnly);
        let i=x.evidence_records.iter().position(|r|r.test_id==GfmTestId::InertiaResponse).unwrap();
        x.evidence_records[i].test_conditions.energy_headroom_wh=None;
        assert!(evaluate_gfm_qualification(&x).unwrap().invalid_test_ids.contains(&GfmTestId::InertiaResponse));
    }
    #[test]
    fn sample_rate_must_be_at_least_twice_measurement_bandwidth() {
        let mut x=input(GfmClaimLevel::SyntheticCoverageOnly);
        x.evidence_records[0].measurement_conditions.sample_rate_hz_milli=x.evidence_records[0].measurement_conditions.bandwidth_hz_milli;
        assert!(evaluate_gfm_qualification(&x).unwrap().invalid_test_ids.contains(&x.evidence_records[0].test_id));
    }
    #[test]
    fn disturbance_and_uncertainty_provenance_are_required() {
        let mut x=input(GfmClaimLevel::SyntheticCoverageOnly);
        x.evidence_records[0].test_conditions.disturbance_location.clear();
        assert!(evaluate_gfm_qualification(&x).unwrap().invalid_test_ids.contains(&x.evidence_records[0].test_id));
        x=input(GfmClaimLevel::SyntheticCoverageOnly);
        x.evidence_records[0].measurement_conditions.uncertainty_reference.clear();
        assert!(evaluate_gfm_qualification(&x).unwrap().invalid_test_ids.contains(&x.evidence_records[0].test_id));
    }

}
