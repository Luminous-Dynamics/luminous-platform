use crate::AuditError;
use std::collections::BTreeSet;

const CLAIM: &str = "SYNTHETIC_GFM_TEST_PLAN_AND_DOMAIN_COVERAGE_ONLY_NOT_CAPABILITY_OR_CONFORMITY";
const DOMAIN_POLICY: &str = "NO_INTERPOLATION_OR_EXTRAPOLATION_OUTSIDE_EXPLICIT_OPERATING_CELLS";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GfmTestFamily {
    OperatingPointHeadroom,
    WeakGridScrXr,
    PhaseAngleJump,
    CurrentLimitFaultRideThrough,
    NegativeSequenceUnbalancedFault,
    FrequencyDomainImpedanceScan,
    MultiVendorInteraction,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GfmExecutionMode {
    EmtSimulation, ControllerHil, PowerHil, PhysicalLaboratory, SiteCommissioning, FieldOperation,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GfmElectricalBoundary { ConverterTerminals, PlantPointOfConnection, MicrogridPointOfConnection }

#[derive(Debug, Clone, PartialEq)]
pub struct GfmOperatingCell {
    pub cell_id: String,
    pub product_configuration_sha256: String,
    pub topology_sha256: String,
    pub controller_configuration_sha256: String,
    pub protection_configuration_sha256: String,
    pub active_power_pu: f64,
    pub reactive_power_pu: f64,
    pub state_of_charge_fraction: f64,
    pub current_headroom_pu: f64,
    pub dc_energy_headroom_kwh: f64,
    pub temperature_c: f64,
    pub short_circuit_ratio: f64,
    pub grid_x_over_r: f64,
    pub pre_disturbance_voltage_pu: f64,
    pub pre_disturbance_frequency_hz: f64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GfmTestControls {
    pub positive_control: String,
    pub negative_control: String,
    pub recovery_control: String,
}
#[derive(Debug, Clone, PartialEq)]
pub struct GfmTestCase {
    pub test_id: String,
    pub family: GfmTestFamily,
    pub electrical_boundary: GfmElectricalBoundary,
    pub operating_cell_ids: Vec<String>,
    pub disturbance_profile_reference: String,
    pub acceptance_criteria_reference: String,
    pub planned_execution_modes: Vec<GfmExecutionMode>,
    pub required_observable_ids: Vec<String>,
    pub participant_configuration_sha256: Vec<String>,
    pub controls: GfmTestControls,
}
#[derive(Debug, Clone, PartialEq)]
pub struct GfmCapabilityRequest {
    pub capability_id: String,
    pub description: String,
    pub electrical_boundary: GfmElectricalBoundary,
    pub requested_operating_cell_ids: Vec<String>,
    pub required_test_families: Vec<GfmTestFamily>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct GfmCapabilityProfile {
    pub profile_id: String,
    pub product_configuration_sha256: String,
    pub criterion_bundle_reference: String,
    pub operating_cells: Vec<GfmOperatingCell>,
    pub tests: Vec<GfmTestCase>,
    pub capability_requests: Vec<GfmCapabilityRequest>,
    pub out_of_domain_policy: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GfmCapabilityCoverageReport {
    pub profile_id: String,
    pub operating_cell_count: usize,
    pub test_count: usize,
    pub capability_request_count: usize,
    pub covered_cell_family_pairs: usize,
    pub claim_ceiling: &'static str,
    pub out_of_domain_policy: &'static str,
}
impl GfmCapabilityCoverageReport {
    pub const fn claim_ceiling(&self) -> &'static str { CLAIM }
}

fn nonblank(value: &str) -> bool { !value.trim().is_empty() }
fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
fn unique_nonblank<'a>(values: impl Iterator<Item = &'a str>, label: &str) -> Result<(), AuditError> {
    let mut seen = BTreeSet::new();
    for value in values {
        if !nonblank(value) || !seen.insert(value) {
            return Err(AuditError(format!("{label} values must be non-empty and unique")));
        }
    }
    Ok(())
}

fn required_metrics(family: GfmTestFamily) -> &'static [&'static str] {
    match family {
        GfmTestFamily::OperatingPointHeadroom => &[
            "active_power_pu","reactive_power_pu","state_of_charge_fraction",
            "dc_energy_headroom_kwh","current_headroom_pu","thermal_margin_c",
        ],
        GfmTestFamily::WeakGridScrXr => &[
            "scr_at_poc","x_over_r_at_poc","voltage_response_pu","frequency_response_hz",
            "oscillation_damping_ratio","minimum_stable_scr",
        ],
        GfmTestFamily::PhaseAngleJump => &[
            "phase_angle_jump_deg","active_power_transient_pu","peak_current_pu",
            "loss_of_synchronism","recovery_time_s",
        ],
        GfmTestFamily::CurrentLimitFaultRideThrough => &[
            "fault_voltage_time_profile","peak_current_pu","sustained_current_pu",
            "active_reactive_current_priority","dc_energy_consumed_kwh",
            "post_fault_recovery_time_s","synchronism_retained",
        ],
        GfmTestFamily::NegativeSequenceUnbalancedFault => &[
            "negative_sequence_current_pu","phase_current_unbalance_percent",
            "dc_link_ripple_v","post_fault_recovery_status",
        ],
        GfmTestFamily::FrequencyDomainImpedanceScan => &[
            "impedance_scan_evidence_reference","resonance_frequency_hz",
            "gain_margin_db","phase_margin_deg","model_measurement_deviation",
        ],
        GfmTestFamily::MultiVendorInteraction => &[
            "active_power_sharing_error_percent","reactive_power_sharing_error_percent",
            "circulating_power_pu","oscillation_frequency_hz","damping_ratio",
            "small_signal_stability_margin","grid_strength_sweep",
        ],
    }
}
fn has_required_execution_mode(test: &GfmTestCase) -> bool {
    let has = |mode| test.planned_execution_modes.contains(&mode);
    match test.family {
        GfmTestFamily::OperatingPointHeadroom | GfmTestFamily::WeakGridScrXr =>
            has(GfmExecutionMode::EmtSimulation) || has(GfmExecutionMode::ControllerHil) || has(GfmExecutionMode::PhysicalLaboratory),
        GfmTestFamily::PhaseAngleJump =>
            has(GfmExecutionMode::EmtSimulation) || has(GfmExecutionMode::ControllerHil)
                || has(GfmExecutionMode::PowerHil) || has(GfmExecutionMode::PhysicalLaboratory),
        GfmTestFamily::CurrentLimitFaultRideThrough | GfmTestFamily::NegativeSequenceUnbalancedFault =>
            has(GfmExecutionMode::PowerHil) || has(GfmExecutionMode::PhysicalLaboratory),
        GfmTestFamily::FrequencyDomainImpedanceScan =>
            has(GfmExecutionMode::EmtSimulation) || has(GfmExecutionMode::PhysicalLaboratory),
        GfmTestFamily::MultiVendorInteraction =>
            has(GfmExecutionMode::EmtSimulation) || has(GfmExecutionMode::ControllerHil) || has(GfmExecutionMode::PhysicalLaboratory),
    }
}

/// Read-only test-plan coverage validation. This does not execute tests, evaluate
/// measurements, validate model fidelity or certify the requested capability.
pub fn validate_gfm_capability_profile(
    profile: &GfmCapabilityProfile,
) -> Result<GfmCapabilityCoverageReport, AuditError> {
    if !nonblank(&profile.profile_id)
        || !valid_sha256(&profile.product_configuration_sha256)
        || !nonblank(&profile.criterion_bundle_reference)
        || profile.out_of_domain_policy != DOMAIN_POLICY
        || profile.operating_cells.is_empty() || profile.tests.is_empty()
        || profile.capability_requests.is_empty()
    {
        return Err(AuditError("GFM profile metadata or domain policy is invalid".to_string()));
    }
    unique_nonblank(profile.operating_cells.iter().map(|x| x.cell_id.as_str()), "operating cell ID")?;
    unique_nonblank(profile.tests.iter().map(|x| x.test_id.as_str()), "GFM test ID")?;
    unique_nonblank(profile.capability_requests.iter().map(|x| x.capability_id.as_str()), "capability request ID")?;

    let mut cell_ids = BTreeSet::new();
    for cell in &profile.operating_cells {
        if !valid_sha256(&cell.product_configuration_sha256)
            || !valid_sha256(&cell.topology_sha256)
            || !valid_sha256(&cell.controller_configuration_sha256)
            || !valid_sha256(&cell.protection_configuration_sha256)
            || cell.product_configuration_sha256 != profile.product_configuration_sha256
            || !cell.active_power_pu.is_finite() || !cell.reactive_power_pu.is_finite()
            || !cell.state_of_charge_fraction.is_finite() || !(0.0..=1.0).contains(&cell.state_of_charge_fraction)
            || !cell.current_headroom_pu.is_finite() || cell.current_headroom_pu < 0.0
            || !cell.dc_energy_headroom_kwh.is_finite() || cell.dc_energy_headroom_kwh < 0.0
            || !cell.temperature_c.is_finite()
            || !cell.short_circuit_ratio.is_finite() || cell.short_circuit_ratio <= 0.0
            || !cell.grid_x_over_r.is_finite() || cell.grid_x_over_r <= 0.0
            || !cell.pre_disturbance_voltage_pu.is_finite() || cell.pre_disturbance_voltage_pu <= 0.0
            || !cell.pre_disturbance_frequency_hz.is_finite() || cell.pre_disturbance_frequency_hz <= 0.0
        {
            return Err(AuditError(format!("operating cell {:?} has invalid, unmatched or incomplete conditions", cell.cell_id)));
        }
        cell_ids.insert(cell.cell_id.as_str());
    }

    for test in &profile.tests {
        if test.operating_cell_ids.is_empty() || !nonblank(&test.disturbance_profile_reference)
            || !nonblank(&test.acceptance_criteria_reference) || test.planned_execution_modes.is_empty()
        {
            return Err(AuditError(format!("test {:?} lacks cells, disturbance, criteria or execution mode", test.test_id)));
        }
        unique_nonblank(test.operating_cell_ids.iter().map(String::as_str), "test operating-cell reference")?;
        unique_nonblank(test.required_observable_ids.iter().map(String::as_str), "required observable ID")?;
        if test.required_observable_ids.is_empty() {
            return Err(AuditError(format!("test {:?} has no required observables", test.test_id)));
        }
        for cell_id in &test.operating_cell_ids {
            if !cell_ids.contains(cell_id.as_str()) {
                return Err(AuditError(format!("test {:?} references unknown operating cell {:?}", test.test_id, cell_id)));
            }
        }
        let modes: BTreeSet<GfmExecutionMode> = test.planned_execution_modes.iter().copied().collect();
        if modes.len() != test.planned_execution_modes.len() || !has_required_execution_mode(test) {
            return Err(AuditError(format!("test {:?} duplicates modes or lacks a suitable planned execution mode", test.test_id)));
        }
        let required: BTreeSet<&str> = required_metrics(test.family).iter().copied().collect();
        let observed: BTreeSet<&str> = test.required_observable_ids.iter().map(String::as_str).collect();
        if !required.is_subset(&observed) {
            let missing: Vec<&str> = required.difference(&observed).copied().collect();
            return Err(AuditError(format!("test {:?} is missing required observables: {}", test.test_id, missing.join(", "))));
        }
        if test.family == GfmTestFamily::MultiVendorInteraction {
            unique_nonblank(test.participant_configuration_sha256.iter().map(String::as_str), "multi-vendor participant digest")?;
            let participants: BTreeSet<&str> = test.participant_configuration_sha256.iter().map(String::as_str).collect();
            if participants.len() < 2 || participants.iter().any(|x| !valid_sha256(x)) {
                return Err(AuditError("multi-vendor test requires two distinct valid participant configurations".to_string()));
            }
        } else if !test.participant_configuration_sha256.is_empty() {
            unique_nonblank(test.participant_configuration_sha256.iter().map(String::as_str), "participant digest")?;
            if test.participant_configuration_sha256.iter().any(|x| !valid_sha256(x)) {
                return Err(AuditError("participant configuration digest is invalid".to_string()));
            }
        }
        if !nonblank(&test.controls.positive_control) || !nonblank(&test.controls.negative_control)
            || !nonblank(&test.controls.recovery_control)
        {
            return Err(AuditError(format!("test {:?} requires positive, negative and recovery controls", test.test_id)));
        }
    }

    let mut covered = BTreeSet::<(String, GfmTestFamily)>::new();
    for test in &profile.tests {
        for cell in &test.operating_cell_ids { covered.insert((cell.clone(), test.family)); }
    }
    let mut requested = BTreeSet::<(String, GfmTestFamily)>::new();
    for request in &profile.capability_requests {
        if !nonblank(&request.description) || request.requested_operating_cell_ids.is_empty()
            || request.required_test_families.is_empty()
        {
            return Err(AuditError(format!("capability request {:?} has an empty scope", request.capability_id)));
        }
        unique_nonblank(request.requested_operating_cell_ids.iter().map(String::as_str), "capability operating-cell ID")?;
        let families: BTreeSet<GfmTestFamily> = request.required_test_families.iter().copied().collect();
        if families.len() != request.required_test_families.len() {
            return Err(AuditError(format!("capability request {:?} duplicates required test families", request.capability_id)));
        }
        for cell in &request.requested_operating_cell_ids {
            if !cell_ids.contains(cell.as_str()) {
                return Err(AuditError(format!("capability request {:?} references unknown cell {:?}", request.capability_id, cell)));
            }
            for family in &request.required_test_families {
                requested.insert((cell.clone(), *family));
                let boundary_covered = profile.tests.iter().any(|test|
                    test.family == *family
                        && test.electrical_boundary == request.electrical_boundary
                        && test.operating_cell_ids.iter().any(|test_cell| test_cell == cell)
                );
                if !covered.contains(&(cell.clone(), *family)) || !boundary_covered {
                    return Err(AuditError(format!(
                        "capability {:?} lacks {:?} coverage at cell {:?} and requested boundary {:?}",
                        request.capability_id, family, cell, request.electrical_boundary
                    )));
                }
            }
        }
    }

    Ok(GfmCapabilityCoverageReport {
        profile_id: profile.profile_id.clone(),
        operating_cell_count: profile.operating_cells.len(),
        test_count: profile.tests.len(),
        capability_request_count: profile.capability_requests.len(),
        covered_cell_family_pairs: requested.len(),
        claim_ceiling: CLAIM,
        out_of_domain_policy: DOMAIN_POLICY,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn h(c: char) -> String { std::iter::repeat(c).take(64).collect() }
    fn cell(id: &str, scr: f64) -> GfmOperatingCell {
        GfmOperatingCell { cell_id:id.into(),product_configuration_sha256:h('a'),topology_sha256:h('b'),
            controller_configuration_sha256:h('c'),protection_configuration_sha256:h('d'),
            active_power_pu:0.4,reactive_power_pu:0.1,state_of_charge_fraction:0.8,current_headroom_pu:0.3,
            dc_energy_headroom_kwh:20.0,temperature_c:25.0,short_circuit_ratio:scr,grid_x_over_r:10.0,
            pre_disturbance_voltage_pu:1.0,pre_disturbance_frequency_hz:50.0 }
    }
    fn metrics(f: GfmTestFamily) -> Vec<String> { required_metrics(f).iter().map(|x|x.to_string()).collect() }
    fn test(id:&str,f:GfmTestFamily,cells:Vec<&str>)->GfmTestCase {
        let modes=match f {
            GfmTestFamily::CurrentLimitFaultRideThrough|GfmTestFamily::NegativeSequenceUnbalancedFault=>vec![GfmExecutionMode::PowerHil,GfmExecutionMode::PhysicalLaboratory],
            GfmTestFamily::FrequencyDomainImpedanceScan=>vec![GfmExecutionMode::EmtSimulation,GfmExecutionMode::PhysicalLaboratory],
            _=>vec![GfmExecutionMode::EmtSimulation,GfmExecutionMode::ControllerHil],
        };
        let participants=if f==GfmTestFamily::MultiVendorInteraction{vec![h('e'),h('f')]}else{vec![]};
        let boundary = if f == GfmTestFamily::MultiVendorInteraction {
            GfmElectricalBoundary::PlantPointOfConnection
        } else {
            GfmElectricalBoundary::ConverterTerminals
        };
        GfmTestCase{test_id:id.into(),family:f,electrical_boundary:boundary,
            operating_cell_ids:cells.into_iter().map(str::to_string).collect(),disturbance_profile_reference:format!("synthetic://disturbance/{id}"),
            acceptance_criteria_reference:"owner-supplied:UNIFI-v3-or-product-profile:criterion-revision-required".into(),
            planned_execution_modes:modes,required_observable_ids:metrics(f),participant_configuration_sha256:participants,
            controls:GfmTestControls{positive_control:"declared response stays within owner criteria".into(),
                negative_control:"one invalid condition is detected without a false pass".into(),
                recovery_control:"return to a declared cell and independently observe recovery".into()}}
    }
    fn profile()->GfmCapabilityProfile {
        let cells=vec![cell("BASE",3.0),cell("WEAK",1.5),cell("LIMIT",2.0)];
        let tests=vec![
            test("GFM-01",GfmTestFamily::OperatingPointHeadroom,vec!["BASE","WEAK","LIMIT"]),
            test("GFM-02",GfmTestFamily::WeakGridScrXr,vec!["BASE","WEAK","LIMIT"]),
            test("GFM-03",GfmTestFamily::PhaseAngleJump,vec!["BASE","WEAK"]),
            test("GFM-04",GfmTestFamily::CurrentLimitFaultRideThrough,vec!["LIMIT"]),
            test("GFM-05",GfmTestFamily::NegativeSequenceUnbalancedFault,vec!["LIMIT"]),
            test("GFM-06",GfmTestFamily::FrequencyDomainImpedanceScan,vec!["BASE","WEAK","LIMIT"]),
            test("GFM-07",GfmTestFamily::MultiVendorInteraction,vec!["BASE","WEAK"]),
        ];
        let requests=vec![
            GfmCapabilityRequest{capability_id:"voltage_magnitude_support".into(),description:"requested voltage-support domain".into(),
                electrical_boundary:GfmElectricalBoundary::ConverterTerminals,requested_operating_cell_ids:vec!["BASE".into(),"WEAK".into(),"LIMIT".into()],
                required_test_families:vec![GfmTestFamily::OperatingPointHeadroom,GfmTestFamily::WeakGridScrXr]},
            GfmCapabilityRequest{capability_id:"voltage_angle_frequency_support".into(),description:"phase-angle/frequency response domain".into(),
                electrical_boundary:GfmElectricalBoundary::ConverterTerminals,requested_operating_cell_ids:vec!["BASE".into(),"WEAK".into()],
                required_test_families:vec![GfmTestFamily::PhaseAngleJump,GfmTestFamily::FrequencyDomainImpedanceScan]},
            GfmCapabilityRequest{capability_id:"fault_current_limit_recovery".into(),description:"current-limited fault response".into(),
                electrical_boundary:GfmElectricalBoundary::ConverterTerminals,requested_operating_cell_ids:vec!["LIMIT".into()],
                required_test_families:vec![GfmTestFamily::CurrentLimitFaultRideThrough]},
            GfmCapabilityRequest{capability_id:"unbalanced_fault_response".into(),description:"negative-sequence domain".into(),
                electrical_boundary:GfmElectricalBoundary::ConverterTerminals,requested_operating_cell_ids:vec!["LIMIT".into()],
                required_test_families:vec![GfmTestFamily::NegativeSequenceUnbalancedFault]},
            GfmCapabilityRequest{capability_id:"multi_vendor_interoperability".into(),description:"parallel multi-vendor interaction".into(),
                electrical_boundary:GfmElectricalBoundary::PlantPointOfConnection,requested_operating_cell_ids:vec!["BASE".into(),"WEAK".into()],
                required_test_families:vec![GfmTestFamily::MultiVendorInteraction]},
        ];
        GfmCapabilityProfile{profile_id:"PF-GFM-ENVELOPE-SYNTH-0.1".into(),product_configuration_sha256:h('a'),
            criterion_bundle_reference:"owner-supplied:UNIFI-v3-or-grid-code-profile:revision-required".into(),
            operating_cells:cells,tests,capability_requests:requests,out_of_domain_policy:DOMAIN_POLICY.into()}
    }

    #[test] fn accepts_complete_synthetic_profile_as_plan_only(){
        let r=validate_gfm_capability_profile(&profile()).unwrap();
        assert_eq!(r.operating_cell_count,3);assert_eq!(r.test_count,7);assert_eq!(r.capability_request_count,5);
        assert_eq!(r.claim_ceiling,CLAIM);
    }
    #[test] fn rejects_cell_product_configuration_mismatch(){let mut x=profile();x.operating_cells[0].product_configuration_sha256=h('9');assert!(validate_gfm_capability_profile(&x).is_err());}
    #[test] fn rejects_duplicate_cell_ids(){let mut x=profile();x.operating_cells[1].cell_id=x.operating_cells[0].cell_id.clone();assert!(validate_gfm_capability_profile(&x).is_err());}
    #[test] fn rejects_soc_outside_zero_to_one(){let mut x=profile();x.operating_cells[0].state_of_charge_fraction=1.1;assert!(validate_gfm_capability_profile(&x).is_err());}
    #[test] fn rejects_nonpositive_scr_and_xr(){let mut x=profile();x.operating_cells[0].short_circuit_ratio=0.0;assert!(validate_gfm_capability_profile(&x).is_err());x=profile();x.operating_cells[0].grid_x_over_r=-1.0;assert!(validate_gfm_capability_profile(&x).is_err());}
    #[test] fn rejects_missing_family_observable(){let mut x=profile();x.tests[1].required_observable_ids.retain(|v|v!="minimum_stable_scr");assert!(validate_gfm_capability_profile(&x).is_err());}
    #[test] fn rejects_missing_recovery_control(){let mut x=profile();x.tests[0].controls.recovery_control.clear();assert!(validate_gfm_capability_profile(&x).is_err());}
    #[test] fn rejects_missing_acceptance_reference(){let mut x=profile();x.tests[0].acceptance_criteria_reference.clear();assert!(validate_gfm_capability_profile(&x).is_err());}
    #[test] fn rejects_unknown_test_cell_reference(){let mut x=profile();x.tests[0].operating_cell_ids.push("UNKNOWN".into());assert!(validate_gfm_capability_profile(&x).is_err());}
    #[test] fn rejects_capability_family_gap(){let mut x=profile();x.tests.retain(|v|v.family!=GfmTestFamily::WeakGridScrXr);assert!(validate_gfm_capability_profile(&x).is_err());}
    #[test] fn current_limit_requires_power_hil_or_physical_lab(){let mut x=profile();x.tests[3].planned_execution_modes=vec![GfmExecutionMode::EmtSimulation];assert!(validate_gfm_capability_profile(&x).is_err());}
    #[test] fn multivendor_requires_distinct_participant_configs(){let mut x=profile();x.tests[6].participant_configuration_sha256=vec![h('e'),h('e')];assert!(validate_gfm_capability_profile(&x).is_err());}
    #[test] fn rejects_duplicate_execution_modes(){let mut x=profile();x.tests[0].planned_execution_modes.push(GfmExecutionMode::EmtSimulation);assert!(validate_gfm_capability_profile(&x).is_err());}
    #[test] fn rejects_extrapolation_policy(){let mut x=profile();x.out_of_domain_policy="INTERPOLATE".into();assert!(validate_gfm_capability_profile(&x).is_err());}
    #[test] fn rejects_duplicate_capability_ids(){let mut x=profile();x.capability_requests[1].capability_id=x.capability_requests[0].capability_id.clone();assert!(validate_gfm_capability_profile(&x).is_err());}
}
    #[test]
    fn rejects_test_boundary_mismatch_for_requested_capability() {
        let mut value = profile();
        value.tests[6].electrical_boundary = GfmElectricalBoundary::ConverterTerminals;
        assert!(validate_gfm_capability_profile(&value).is_err());
    }

