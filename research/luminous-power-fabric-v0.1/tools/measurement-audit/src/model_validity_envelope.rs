use crate::AuditError;
use std::collections::BTreeSet;

const CLAIM: &str = "SYNTHETIC_MODEL_VALIDITY_PROFILE_ONLY_NOT_MODEL_VALIDATION_OR_CONFORMITY";
const DOMAIN_POLICY: &str = "NO_MODEL_CLAIM_OUTSIDE_EXPLICIT_HELD_OUT_VALIDATION_CELLS";

#[derive(Debug,Clone,Copy,PartialEq,Eq,PartialOrd,Ord)]
pub enum ModelFidelity { EmtSwitching, EmtAverage, DetailedRms, PositiveSequenceRms }
#[derive(Debug,Clone,Copy,PartialEq,Eq,PartialOrd,Ord)]
pub enum ValidationScope { TimeDomain, FrequencyDomain }
#[derive(Debug,Clone,Copy,PartialEq,Eq,PartialOrd,Ord)]
pub enum PhysicalReferenceMode { PowerHil, PhysicalLaboratory, SiteCommissioning, FieldEvent }
#[derive(Debug,Clone,Copy,PartialEq,Eq,PartialOrd,Ord)]
pub enum ResidualMetricKind {
    NormalizedRmse, PeakAbsoluteError, TimingLagMs,
    ResonanceFrequencyShiftHz, GainMarginDifferenceDb, PhaseMarginDifferenceDeg,
}

#[derive(Debug,Clone,PartialEq)]
pub struct ModelIdentity {
    pub model_sha256:String,
    pub product_hardware_sha256:String,
    pub firmware_sha256:String,
    pub controller_configuration_sha256:String,
    pub protection_configuration_sha256:String,
    pub topology_sha256:String,
    pub model_parameter_set_sha256:String,
    pub analysis_build_sha256:String,
}
#[derive(Debug,Clone,PartialEq)]
pub struct ModelOperatingCell {
    pub cell_id:String,
    pub source_gfm_profile_id:String,
    pub source_gfm_cell_id:String,
    pub operating_cell_sha256:String,
    pub active_power_pu:f64,
    pub reactive_power_pu:f64,
    pub state_of_charge_fraction:f64,
    pub temperature_c:f64,
    pub short_circuit_ratio:f64,
    pub grid_x_over_r:f64,
}
#[derive(Debug,Clone,PartialEq)]
pub struct ResidualRecord {
    pub observable_id:String,
    pub metric_kind:ResidualMetricKind,
    pub unit:String,
    pub normalized_rmse:f64,
    pub peak_absolute_error:f64,
    pub measurement_uncertainty:f64,
    pub timing_lag_ms:Option<f64>,
    pub model_summary_reference:String,
    pub measurement_summary_reference:String,
    pub residual_output_reference:String,
    pub uncertainty_reference:String,
}
#[derive(Debug,Clone,PartialEq)]
pub struct ModelValidationCase {
    pub case_id:String,
    pub cell_id:String,
    pub scope:ValidationScope,
    pub reference_mode:PhysicalReferenceMode,
    pub held_out_validation_data:bool,
    pub dataset_sha256:String,
    pub raw_data_sha256:String,
    pub processing_build_sha256:String,
    pub calibration_receipt_reference:String,
    pub timebase_reference:String,
    pub independent_witness_reference:String,
    pub acceptance_criteria_reference:String,
    pub required_observable_ids:Vec<String>,
    pub residuals:Vec<ResidualRecord>,
}
#[derive(Debug,Clone,PartialEq)]
pub struct ModelClaimRequest {
    pub claim_id:String,
    pub description:String,
    pub requested_cell_ids:Vec<String>,
    pub required_scopes:Vec<ValidationScope>,
}
#[derive(Debug,Clone,PartialEq)]
pub struct ModelValidityProfile {
    pub profile_id:String,
    pub model_fidelity:ModelFidelity,
    pub identity:ModelIdentity,
    pub calibration_campaign_id:String,
    pub calibration_dataset_sha256:String,
    pub validation_campaign_id:String,
    pub validation_dataset_sha256:String,
    pub gfm_profile_reference:String,
    pub out_of_domain_policy:String,
    pub operating_cells:Vec<ModelOperatingCell>,
    pub validation_cases:Vec<ModelValidationCase>,
    pub claim_requests:Vec<ModelClaimRequest>,
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct ModelValidityCoverageReport {
    pub profile_id:String,
    pub model_fidelity:ModelFidelity,
    pub operating_cell_count:usize,
    pub validation_case_count:usize,
    pub residual_record_count:usize,
    pub claim_request_count:usize,
    pub held_out_cell_scope_pairs_covered:usize,
    pub profile_status:&'static str,
    pub model_validation_claim_eligible:bool,
    pub claim_ceiling:&'static str,
}
impl ModelValidityCoverageReport {
    pub const fn claim_ceiling(&self)->&'static str { CLAIM }
}

fn nonblank(x:&str)->bool{!x.trim().is_empty()}
fn sha256(x:&str)->bool{x.len()==64&&x.bytes().all(|b|b.is_ascii_hexdigit())}
fn unique<'a>(values:impl Iterator<Item=&'a str>,label:&str)->Result<(),AuditError>{
    let mut set=BTreeSet::new();
    for x in values {if !nonblank(x)||!set.insert(x){return Err(AuditError(format!("{label} must be non-empty and unique")));}}
    Ok(())
}
fn mandatory_observables(scope:ValidationScope)->&'static[&'static str]{
    match scope {
        ValidationScope::TimeDomain=>&["voltage_response","current_response","active_power_response","reactive_power_response","frequency_response","phase_angle_response"],
        ValidationScope::FrequencyDomain=>&["impedance_magnitude_response","impedance_phase_response","resonance_frequency","gain_margin","phase_margin"],
    }
}
fn expected_metric_kind(scope:ValidationScope,observable:&str)->ResidualMetricKind{
    match (scope,observable) {
        (ValidationScope::FrequencyDomain,"resonance_frequency") =>
            ResidualMetricKind::ResonanceFrequencyShiftHz,
        (ValidationScope::FrequencyDomain,"gain_margin") =>
            ResidualMetricKind::GainMarginDifferenceDb,
        (ValidationScope::FrequencyDomain,"phase_margin") =>
            ResidualMetricKind::PhaseMarginDifferenceDeg,
        _ => ResidualMetricKind::NormalizedRmse,
    }
}
fn valid_residual(r:&ResidualRecord)->bool{
    nonblank(&r.observable_id)&&nonblank(&r.unit)&&
    r.normalized_rmse.is_finite()&&r.normalized_rmse>=0.0&&
    r.peak_absolute_error.is_finite()&&r.peak_absolute_error>=0.0&&
    r.measurement_uncertainty.is_finite()&&r.measurement_uncertainty>=0.0&&
    r.timing_lag_ms.is_none_or(f64::is_finite)&&
    nonblank(&r.model_summary_reference)&&nonblank(&r.measurement_summary_reference)&&
    nonblank(&r.residual_output_reference)&&nonblank(&r.uncertainty_reference)
}

/// Validates a held-out model-to-measurement validation plan and residual-record
/// structure. It does not evaluate acceptance thresholds, authenticate evidence,
/// prove independence, or declare the model physically validated.
pub fn validate_model_validity_profile(p:&ModelValidityProfile)->Result<ModelValidityCoverageReport,AuditError>{
    if !nonblank(&p.profile_id)||!nonblank(&p.gfm_profile_reference)||
        !nonblank(&p.calibration_campaign_id)||!nonblank(&p.validation_campaign_id)||
        p.calibration_campaign_id==p.validation_campaign_id||
        !sha256(&p.calibration_dataset_sha256)||!sha256(&p.validation_dataset_sha256)||
        p.calibration_dataset_sha256==p.validation_dataset_sha256||
        p.out_of_domain_policy!=DOMAIN_POLICY||p.operating_cells.is_empty()||
        p.validation_cases.is_empty()||p.claim_requests.is_empty(){
        return Err(AuditError("model-validity metadata, hold-out split or domain policy is invalid".into()));
    }
    let i=&p.identity;
    for digest in [&i.model_sha256,&i.product_hardware_sha256,&i.firmware_sha256,
        &i.controller_configuration_sha256,&i.protection_configuration_sha256,
        &i.topology_sha256,&i.model_parameter_set_sha256,&i.analysis_build_sha256]{
        if !sha256(digest){return Err(AuditError("model identity contains an invalid configuration digest".into()));}
    }
    unique(p.operating_cells.iter().map(|x|x.cell_id.as_str()),"model operating-cell IDs")?;
    unique(p.validation_cases.iter().map(|x|x.case_id.as_str()),"model validation case IDs")?;
    unique(p.claim_requests.iter().map(|x|x.claim_id.as_str()),"model claim IDs")?;

    let mut cell_ids=BTreeSet::new();
    for c in &p.operating_cells{
        if !nonblank(&c.source_gfm_profile_id)||!nonblank(&c.source_gfm_cell_id)||
            !sha256(&c.operating_cell_sha256)||!c.active_power_pu.is_finite()||
            !c.reactive_power_pu.is_finite()||!c.state_of_charge_fraction.is_finite()||
            !(0.0..=1.0).contains(&c.state_of_charge_fraction)||!c.temperature_c.is_finite()||
            !c.short_circuit_ratio.is_finite()||c.short_circuit_ratio<=0.0||
            !c.grid_x_over_r.is_finite()||c.grid_x_over_r<=0.0{
            return Err(AuditError(format!("model operating cell {:?} is invalid",c.cell_id)));
        }
        cell_ids.insert(c.cell_id.as_str());
    }

    let mut covered=BTreeSet::<(String,ValidationScope)>::new();
    let mut residual_count=0usize;
    for case in &p.validation_cases{
        if !nonblank(&case.cell_id)||!cell_ids.contains(case.cell_id.as_str())||
            !case.held_out_validation_data||case.dataset_sha256!=p.validation_dataset_sha256||
            !sha256(&case.dataset_sha256)||!sha256(&case.raw_data_sha256)||
            !sha256(&case.processing_build_sha256)||!nonblank(&case.calibration_receipt_reference)||
            !nonblank(&case.timebase_reference)||!nonblank(&case.independent_witness_reference)||
            !nonblank(&case.acceptance_criteria_reference){
            return Err(AuditError(format!("validation case {:?} is not bound to held-out, witnessed, calibrated evidence",case.case_id)));
        }
        if case.scope==ValidationScope::FrequencyDomain&&
            !matches!(p.model_fidelity,ModelFidelity::EmtSwitching|ModelFidelity::EmtAverage){
            return Err(AuditError("frequency-domain GFM model claims require EMT fidelity".into()));
        }
        if case.scope==ValidationScope::FrequencyDomain&&
            !matches!(case.reference_mode,PhysicalReferenceMode::PowerHil|PhysicalReferenceMode::PhysicalLaboratory){
            return Err(AuditError("frequency-domain validation requires power-HIL or physical-laboratory reference mode".into()));
        }
        unique(case.required_observable_ids.iter().map(String::as_str),"case observable IDs")?;
        let required:BTreeSet<&str>=mandatory_observables(case.scope).iter().copied().collect();
        let declared:BTreeSet<&str>=case.required_observable_ids.iter().map(String::as_str).collect();
        if !required.is_subset(&declared){
            return Err(AuditError(format!("validation case {:?} omits scope-required observables",case.case_id)));
        }
        unique(case.residuals.iter().map(|x|x.observable_id.as_str()),"residual observable IDs")?;
        let residual_ids:BTreeSet<&str>=case.residuals.iter().map(|x|x.observable_id.as_str()).collect();
        if !declared.is_subset(&residual_ids)||case.residuals.iter().any(|r|
            !valid_residual(r)||expected_metric_kind(case.scope,&r.observable_id)!=r.metric_kind
        ){
            return Err(AuditError(format!("validation case {:?} lacks complete, well-formed residual records",case.case_id)));
        }
        residual_count+=case.residuals.len();
        covered.insert((case.cell_id.clone(),case.scope));
    }

    let mut requested=BTreeSet::<(String,ValidationScope)>::new();
    for claim in &p.claim_requests{
        if !nonblank(&claim.description)||claim.requested_cell_ids.is_empty()||claim.required_scopes.is_empty(){
            return Err(AuditError(format!("model claim {:?} has empty requested domain or scopes",claim.claim_id)));
        }
        unique(claim.requested_cell_ids.iter().map(String::as_str),"claim cell IDs")?;
        let scopes:BTreeSet<ValidationScope>=claim.required_scopes.iter().copied().collect();
        if scopes.len()!=claim.required_scopes.len(){return Err(AuditError(format!("claim {:?} duplicates validation scopes",claim.claim_id)));}
        for cell in &claim.requested_cell_ids{
            if !cell_ids.contains(cell.as_str()){return Err(AuditError(format!("claim {:?} references unknown cell {:?}",claim.claim_id,cell)));}
            for scope in &claim.required_scopes{
                requested.insert((cell.clone(),*scope));
                if !covered.contains(&(cell.clone(),*scope)){
                    return Err(AuditError(format!("claim {:?} lacks held-out {:?} validation at cell {:?}",claim.claim_id,scope,cell)));
                }
            }
        }
    }
    Ok(ModelValidityCoverageReport{
        profile_id:p.profile_id.clone(),model_fidelity:p.model_fidelity,operating_cell_count:p.operating_cells.len(),
        validation_case_count:p.validation_cases.len(),residual_record_count:residual_count,
        claim_request_count:p.claim_requests.len(),held_out_cell_scope_pairs_covered:requested.len(),
        profile_status:"STRUCTURALLY_COMPLETE_TEST_PLAN_ONLY",model_validation_claim_eligible:false,claim_ceiling:CLAIM,
    })
}

#[cfg(test)]
mod tests{
 use super::*;
 fn h(c:char)->String{std::iter::repeat(c).take(64).collect()}
 fn residual(id:&str)->ResidualRecord{ResidualRecord{observable_id:id.into(),metric_kind:ResidualMetricKind::NormalizedRmse,
  unit:"pu".into(),normalized_rmse:0.03,peak_absolute_error:0.05,measurement_uncertainty:0.01,timing_lag_ms:Some(0.2),
  model_summary_reference:"synthetic://model-summary".into(),measurement_summary_reference:"synthetic://measurement-summary".into(),
  residual_output_reference:"synthetic://residual".into(),uncertainty_reference:"synthetic://uncertainty".into()}}
 fn case(id:&str,cell:&str,scope:ValidationScope,mode:PhysicalReferenceMode)->ModelValidationCase{
  let obs=mandatory_observables(scope).iter().map(|x|(*x).to_string()).collect::<Vec<_>>();
  ModelValidationCase{case_id:id.into(),cell_id:cell.into(),scope,reference_mode:mode,held_out_validation_data:true,
   dataset_sha256:h('b'),raw_data_sha256:h('c'),processing_build_sha256:h('d'),
   calibration_receipt_reference:"synthetic://calibration".into(),timebase_reference:"synthetic://timebase".into(),
   independent_witness_reference:"synthetic://witness".into(),acceptance_criteria_reference:"owner-supplied:criterion-revision".into(),
   required_observable_ids:obs.clone(),residuals:obs.iter().map(|x|residual(x)).collect()}
 }
 fn profile()->ModelValidityProfile{
  let cell=ModelOperatingCell{cell_id:"CELL-01".into(),source_gfm_profile_id:"PF-GFM-ENVELOPE-SYNTH-0.1".into(),
   source_gfm_cell_id:"CELL-BASE".into(),operating_cell_sha256:h('e'),active_power_pu:0.4,reactive_power_pu:0.1,
   state_of_charge_fraction:0.8,temperature_c:25.0,short_circuit_ratio:3.0,grid_x_over_r:10.0};
  ModelValidityProfile{profile_id:"MODEL-VALIDITY-SYNTH-01".into(),model_fidelity:ModelFidelity::EmtSwitching,
   identity:ModelIdentity{model_sha256:h('1'),product_hardware_sha256:h('2'),firmware_sha256:h('3'),
    controller_configuration_sha256:h('4'),protection_configuration_sha256:h('5'),topology_sha256:h('6'),
    model_parameter_set_sha256:h('7'),analysis_build_sha256:h('8')},
   calibration_campaign_id:"campaign-calibration".into(),calibration_dataset_sha256:h('9'),
   validation_campaign_id:"campaign-heldout".into(),validation_dataset_sha256:h('b'),
   gfm_profile_reference:"PF-GFM-ENVELOPE-SYNTH-0.1".into(),out_of_domain_policy:DOMAIN_POLICY.into(),
   operating_cells:vec![cell],validation_cases:vec![
    case("CASE-TIME","CELL-01",ValidationScope::TimeDomain,PhysicalReferenceMode::PhysicalLaboratory),
    case("CASE-FREQ","CELL-01",ValidationScope::FrequencyDomain,PhysicalReferenceMode::PowerHil),
   ],
   claim_requests:vec![ModelClaimRequest{claim_id:"CLAIM-01".into(),description:"time/frequency model claim within one tested cell".into(),
    requested_cell_ids:vec!["CELL-01".into()],required_scopes:vec![ValidationScope::TimeDomain,ValidationScope::FrequencyDomain]}]}
 }
 #[test]fn accepts_complete_heldout_profile_as_plan_only(){let r=validate_model_validity_profile(&profile()).unwrap();assert_eq!(r.validation_case_count,2);assert_eq!(r.residual_record_count,11);assert!(!r.model_validation_claim_eligible);}
 #[test]fn rejects_calibration_validation_dataset_reuse(){let mut x=profile();x.validation_dataset_sha256=x.calibration_dataset_sha256.clone();assert!(validate_model_validity_profile(&x).is_err());}
 #[test]fn rejects_calibration_validation_campaign_reuse(){let mut x=profile();x.validation_campaign_id=x.calibration_campaign_id.clone();assert!(validate_model_validity_profile(&x).is_err());}
 #[test]fn rejects_case_not_marked_heldout(){let mut x=profile();x.validation_cases[0].held_out_validation_data=false;assert!(validate_model_validity_profile(&x).is_err());}
 #[test]fn rejects_case_on_nonvalidation_dataset(){let mut x=profile();x.validation_cases[0].dataset_sha256=h('9');assert!(validate_model_validity_profile(&x).is_err());}
 #[test]fn rejects_frequency_scope_with_rms_model(){let mut x=profile();x.model_fidelity=ModelFidelity::DetailedRms;assert!(validate_model_validity_profile(&x).is_err());}
 #[test]fn rejects_frequency_scope_without_powerhil_or_lab_reference(){let mut x=profile();x.validation_cases[1].reference_mode=PhysicalReferenceMode::FieldEvent;assert!(validate_model_validity_profile(&x).is_err());}
 #[test]fn rejects_missing_cell_scope_validation(){let mut x=profile();x.validation_cases.retain(|c|c.scope!=ValidationScope::FrequencyDomain);assert!(validate_model_validity_profile(&x).is_err());}
 #[test]fn rejects_missing_residual_record(){let mut x=profile();x.validation_cases[0].residuals.pop();assert!(validate_model_validity_profile(&x).is_err());}
 #[test]fn rejects_invalid_uncertainty_or_nonfinite_residual(){let mut x=profile();x.validation_cases[0].residuals[0].measurement_uncertainty=-0.1;assert!(validate_model_validity_profile(&x).is_err());}
 #[test]fn rejects_missing_calibration_or_witness_reference(){let mut x=profile();x.validation_cases[0].calibration_receipt_reference.clear();assert!(validate_model_validity_profile(&x).is_err());}
 #[test]fn rejects_claim_outside_declared_cell(){let mut x=profile();x.claim_requests[0].requested_cell_ids.push("UNKNOWN".into());assert!(validate_model_validity_profile(&x).is_err());}
 #[test]fn rejects_invalid_model_identity_digest(){let mut x=profile();x.identity.firmware_sha256="bad".into();assert!(validate_model_validity_profile(&x).is_err());}
 #[test]fn rejects_outofdomain_interpolation_policy(){let mut x=profile();x.out_of_domain_policy="INTERPOLATE".into();assert!(validate_model_validity_profile(&x).is_err());}
 #[test]fn rejects_residual_metric_kind_mismatch(){let mut x=profile();x.validation_cases[1].residuals.iter_mut().find(|r|r.observable_id=="resonance_frequency").unwrap().metric_kind=ResidualMetricKind::NormalizedRmse;assert!(validate_model_validity_profile(&x).is_err());}

}
