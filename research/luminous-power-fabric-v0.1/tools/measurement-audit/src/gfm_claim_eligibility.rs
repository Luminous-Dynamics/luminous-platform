use crate::AuditError;
use std::collections::BTreeSet;

const CLAIM_CEILING: &str =
    "STRUCTURAL_CLAIM_SUPPORT_ONLY_NOT_RELEASE_AUTHORIZATION_OR_CONFORMITY";

#[derive(Debug,Clone,Copy,PartialEq,Eq,PartialOrd,Ord)]
pub enum GfmFamily {
    OperatingPointHeadroom, WeakGridScrXr, PhaseAngleJump, CurrentLimitFaultRideThrough,
    NegativeSequenceUnbalancedFault, FrequencyDomainImpedanceScan, MultiVendorInteraction,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum ElectricalBoundary { ConverterTerminals, PlantPointOfConnection, MicrogridPointOfConnection }
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum ExecutionMode {
    ModelOnly, ControllerHil, PowerHil, PhysicalLaboratory, SiteCommissioning, PeriodicFieldOperation,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum ClaimScope {
    ModelConditionedResponse, ControllerImplementation, PhysicalPowerInterface,
    LaboratoryDevicePerformance, InstalledCommissioning, BoundedFieldOperation,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum RunState { Planned, Executed }
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum RunResult { Pass, Fail, Inconclusive, Aborted }
#[derive(Debug,Clone,Copy,PartialEq,Eq,PartialOrd,Ord)]
pub enum ComponentRole {
    MicrogridController, PowerConversionSystem, ProtectionRelay, Meter,
    PowerInterface, BatteryCells, Generator, Photovoltaic, DistributionNetwork, Load,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum Realization { Physical, Simulated, Emulated, NotInTest }
#[derive(Debug,Clone,Copy,PartialEq,Eq,PartialOrd,Ord)]
pub enum ModelScope { TimeDomain, FrequencyDomain }

#[derive(Debug,Clone,PartialEq)]
pub struct ComponentEvidence {
    pub component_id:String,
    pub role:ComponentRole,
    pub realization:Realization,
    pub configuration_sha256:String,
}
#[derive(Debug,Clone,PartialEq)]
pub struct ModelBinding {
    pub model_validity_profile_sha256:String,
    pub model_sha256:String,
    pub product_hardware_sha256:String,
    pub firmware_sha256:String,
    pub controller_configuration_sha256:String,
    pub protection_configuration_sha256:String,
    pub topology_sha256:String,
    pub validated_cell_id:String,
    pub validated_scopes:Vec<ModelScope>,
}
#[derive(Debug,Clone,PartialEq)]
pub struct FieldSamplingFrame {
    pub site_profile_sha256:String,
    pub installed_configuration_sha256:String,
    pub observation_start_epoch_s:u64,
    pub observation_end_epoch_s:u64,
    pub planned_mission_count:u64,
    pub observed_mission_count:u64,
    pub data_coverage_fraction:f64,
    pub observed_regime_ids:Vec<String>,
    pub sampling_evidence_reference:String,
}
#[derive(Debug,Clone,PartialEq)]
pub struct ExecutedGfmTestEvidence {
    pub evidence_id:String,
    pub gfm_profile_sha256:String,
    pub gfm_profile_id:String,
    pub gfm_test_id:String,
    pub family:GfmFamily,
    pub operating_cell_id:String,
    pub electrical_boundary:ElectricalBoundary,
    pub execution_mode:ExecutionMode,
    pub run_state:RunState,
    pub run_result:RunResult,
    pub product_configuration_sha256:String,
    pub hardware_configuration_sha256:String,
    pub firmware_sha256:String,
    pub controller_configuration_sha256:String,
    pub protection_configuration_sha256:String,
    pub topology_sha256:String,
    pub environment_passport_sha256:String,
    pub analysis_build_sha256:String,
    pub criterion_reference:String,
    pub test_procedure_reference:String,
    pub run_record_sha256:String,
    pub raw_data_sha256:String,
    pub observed_observable_ids:Vec<String>,
    pub components:Vec<ComponentEvidence>,
    pub model_binding:Option<ModelBinding>,
    pub calibration_receipt_reference:Option<String>,
    pub timebase_reference:String,
    pub independent_witness_reference:Option<String>,
    /// Reference only. This module does not authenticate or verify Mycelix receipts.
    pub mycelix_receipt_reference:String,
    pub mycelix_policy_revision_reference:String,
    pub site_sampling_frame:Option<FieldSamplingFrame>,
}
#[derive(Debug,Clone,PartialEq)]
pub struct GfmClaimRequest {
    pub claim_id:String,
    pub description:String,
    pub evidence_id:String,
    pub requested_scope:ClaimScope,
    pub family:GfmFamily,
    pub operating_cell_id:String,
    pub electrical_boundary:ElectricalBoundary,
    pub product_configuration_sha256:String,
    pub hardware_configuration_sha256:String,
    pub firmware_sha256:String,
    pub controller_configuration_sha256:String,
    pub protection_configuration_sha256:String,
    pub topology_sha256:String,
    pub required_observable_ids:Vec<String>,
    pub required_model_scopes:Vec<ModelScope>,
    pub required_field_regime_ids:Vec<String>,
    pub minimum_field_data_coverage_fraction:f64,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum ClaimSupportDisposition {
    NotSupported,
    StructurallySupportedPendingMycelixVerification,
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct GfmClaimAssessment {
    pub claim_id:String,
    pub evidence_id:String,
    pub disposition:ClaimSupportDisposition,
    pub exclusion_reasons:Vec<String>,
    pub mycelix_verification_required:bool,
    pub independent_review_required:bool,
    /// Always false in v0.1: structural support is not release authority.
    pub claim_release_permitted:bool,
    pub claim_ceiling:&'static str,
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct GfmClaimEligibilityReport {
    pub assessments:Vec<GfmClaimAssessment>,
    pub structurally_supported_count:usize,
    pub excluded_count:usize,
    pub claim_release_permitted:bool,
    pub claim_ceiling:&'static str,
}
impl GfmClaimEligibilityReport {
    pub const fn claim_ceiling(&self)->&'static str { CLAIM_CEILING }
}

fn nonblank(x:&str)->bool{!x.trim().is_empty()}
fn sha(x:&str)->bool{x.len()==64&&x.bytes().all(|b|b.is_ascii_hexdigit())}
fn unique<'a>(values:impl Iterator<Item=&'a str>,label:&str)->Result<(),AuditError>{
    let mut set=BTreeSet::new();
    for x in values{if !nonblank(x)||!set.insert(x){return Err(AuditError(format!("{label} must be non-empty and unique")));}}
    Ok(())
}
fn component_is_physical(e:&ExecutedGfmTestEvidence,role:ComponentRole)->bool{
    e.components.iter().any(|c|c.role==role&&c.realization==Realization::Physical&&sha(&c.configuration_sha256))
}
fn stage_supports(scope:ClaimScope,mode:ExecutionMode)->bool{
    match scope{
        ClaimScope::ModelConditionedResponse=>matches!(mode,ExecutionMode::ModelOnly|ExecutionMode::ControllerHil|ExecutionMode::PowerHil),
        ClaimScope::ControllerImplementation=>matches!(mode,ExecutionMode::ControllerHil|ExecutionMode::PowerHil|ExecutionMode::PhysicalLaboratory|ExecutionMode::SiteCommissioning|ExecutionMode::PeriodicFieldOperation),
        ClaimScope::PhysicalPowerInterface=>matches!(mode,ExecutionMode::PowerHil|ExecutionMode::PhysicalLaboratory|ExecutionMode::SiteCommissioning|ExecutionMode::PeriodicFieldOperation),
        ClaimScope::LaboratoryDevicePerformance=>mode==ExecutionMode::PhysicalLaboratory,
        ClaimScope::InstalledCommissioning=>mode==ExecutionMode::SiteCommissioning,
        ClaimScope::BoundedFieldOperation=>mode==ExecutionMode::PeriodicFieldOperation,
    }
}
fn validate_request(r:&GfmClaimRequest)->Result<(),AuditError>{
    if !nonblank(&r.claim_id)||!nonblank(&r.description)||!nonblank(&r.evidence_id)||
       !sha(&r.product_configuration_sha256)||!sha(&r.hardware_configuration_sha256)||
       !sha(&r.firmware_sha256)||!sha(&r.controller_configuration_sha256)||
       !sha(&r.protection_configuration_sha256)||!sha(&r.topology_sha256)||
       r.required_observable_ids.is_empty()||!r.minimum_field_data_coverage_fraction.is_finite()||
       !(0.0..=1.0).contains(&r.minimum_field_data_coverage_fraction){
        return Err(AuditError(format!("claim request {:?} is incomplete",r.claim_id)));
    }
    unique(r.required_observable_ids.iter().map(String::as_str),"claim observable IDs")?;
    unique(r.required_field_regime_ids.iter().map(String::as_str),"claim regime IDs")?;
    let scopes:BTreeSet<ModelScope>=r.required_model_scopes.iter().copied().collect();
    if scopes.len()!=r.required_model_scopes.len(){
        return Err(AuditError(format!("claim {:?} duplicates model validation scopes",r.claim_id)));
    }
    Ok(())
}
fn assess_one(r:&GfmClaimRequest,e:Option<&ExecutedGfmTestEvidence>)->GfmClaimAssessment{
    let mut reasons=Vec::new();
    let Some(e)=e else{
        return GfmClaimAssessment{claim_id:r.claim_id.clone(),evidence_id:r.evidence_id.clone(),
            disposition:ClaimSupportDisposition::NotSupported,exclusion_reasons:vec!["executed-test-evidence-not-found".into()],
            mycelix_verification_required:true,independent_review_required:true,claim_release_permitted:false,claim_ceiling:CLAIM_CEILING};
    };
    if e.evidence_id!=r.evidence_id{reasons.push("evidence-identity-mismatch".into());}
    if e.run_state!=RunState::Executed{reasons.push("test-not-executed".into());}
    if e.run_result!=RunResult::Pass{reasons.push("test-did-not-pass".into());}
    if e.family!=r.family{reasons.push("test-family-mismatch".into());}
    if e.operating_cell_id!=r.operating_cell_id{reasons.push("operating-cell-mismatch".into());}
    if e.electrical_boundary!=r.electrical_boundary{reasons.push("electrical-boundary-mismatch".into());}
    if !stage_supports(r.requested_scope,e.execution_mode){reasons.push("execution-stage-cannot-support-requested-claim".into());}
    if e.product_configuration_sha256!=r.product_configuration_sha256||
       e.hardware_configuration_sha256!=r.hardware_configuration_sha256||
       e.firmware_sha256!=r.firmware_sha256||
       e.controller_configuration_sha256!=r.controller_configuration_sha256||
       e.protection_configuration_sha256!=r.protection_configuration_sha256||
       e.topology_sha256!=r.topology_sha256{
        reasons.push("configuration-identity-mismatch".into());
    }
    if !sha(&e.gfm_profile_sha256)||!nonblank(&e.gfm_profile_id)||!nonblank(&e.gfm_test_id)||
       !sha(&e.environment_passport_sha256)||!sha(&e.analysis_build_sha256)||
       !sha(&e.run_record_sha256)||!sha(&e.raw_data_sha256)||
       !nonblank(&e.criterion_reference)||!nonblank(&e.test_procedure_reference)||
       !nonblank(&e.timebase_reference)||!nonblank(&e.mycelix_receipt_reference)||
       !nonblank(&e.mycelix_policy_revision_reference){
        reasons.push("evidence-lineage-incomplete".into());
    }
    let component_ids:BTreeSet<&str>=e.components.iter().map(|c|c.component_id.as_str()).collect();
    if component_ids.len()!=e.components.len()||e.components.iter().any(|c|!nonblank(&c.component_id)||!sha(&c.configuration_sha256)){
        reasons.push("component-boundary-map-invalid".into());
    }
    if e.components.iter().any(|c|
        (c.role==ComponentRole::MicrogridController && c.configuration_sha256!=r.controller_configuration_sha256) ||
        (c.role==ComponentRole::PowerConversionSystem && c.configuration_sha256!=r.hardware_configuration_sha256) ||
        (c.role==ComponentRole::ProtectionRelay && c.configuration_sha256!=r.protection_configuration_sha256)
    ){
        reasons.push("component-configuration-digest-mismatch".into());
    }
    if e.execution_mode!=ExecutionMode::ModelOnly &&
        (e.calibration_receipt_reference.as_deref().is_none_or(|x|!nonblank(x)) ||
         e.independent_witness_reference.as_deref().is_none_or(|x|!nonblank(x))){
        reasons.push("physical-execution-calibration-or-witness-reference-missing".into());
    }
    let observed:BTreeSet<&str>=e.observed_observable_ids.iter().map(String::as_str).collect();
    if r.required_observable_ids.iter().any(|x|!observed.contains(x.as_str())){
        reasons.push("required-observable-missing".into());
    }
    match r.requested_scope{
        ClaimScope::ModelConditionedResponse=>{
            match &e.model_binding{
                Some(m)=>{
                    if !sha(&m.model_validity_profile_sha256)||!sha(&m.model_sha256)||
                       m.product_hardware_sha256!=r.hardware_configuration_sha256||
                       m.firmware_sha256!=r.firmware_sha256||
                       m.controller_configuration_sha256!=r.controller_configuration_sha256||
                       m.protection_configuration_sha256!=r.protection_configuration_sha256||
                       m.topology_sha256!=r.topology_sha256||m.validated_cell_id!=r.operating_cell_id||
                       r.required_model_scopes.iter().any(|s|!m.validated_scopes.contains(s)){
                        reasons.push("model-validity-domain-or-identity-mismatch".into());
                    }
                }
                None=>reasons.push("model-validity-profile-reference-missing".into()),
            }
        }
        ClaimScope::ControllerImplementation=>{
            if !component_is_physical(e,ComponentRole::MicrogridController){
                reasons.push("physical-controller-not-present".into());
            }
        }
        ClaimScope::PhysicalPowerInterface=>{
            if !component_is_physical(e,ComponentRole::MicrogridController)||
               !component_is_physical(e,ComponentRole::PowerConversionSystem)||
               !component_is_physical(e,ComponentRole::PowerInterface){
                reasons.push("physical-controller-pcs-or-power-interface-not-present".into());
            }
        }
        ClaimScope::LaboratoryDevicePerformance=>{
            for role in [ComponentRole::MicrogridController,ComponentRole::PowerConversionSystem,ComponentRole::ProtectionRelay,ComponentRole::Meter]{
                if !component_is_physical(e,role){reasons.push(format!("required-physical-component-missing-{role:?}"));}
            }
        }
        ClaimScope::InstalledCommissioning=>{
            for role in [ComponentRole::MicrogridController,ComponentRole::PowerConversionSystem,ComponentRole::ProtectionRelay,ComponentRole::Meter]{
                if !component_is_physical(e,role){reasons.push(format!("required-installed-physical-component-missing-{role:?}"));}
            }
            match &e.site_sampling_frame{
                Some(s)=>{
                    if !sha(&s.site_profile_sha256)||s.installed_configuration_sha256!=r.product_configuration_sha256||
                       !nonblank(&s.sampling_evidence_reference){
                        reasons.push("site-installation-identity-mismatch".into());
                    }
                }
                None=>reasons.push("site-context-missing".into()),
            }
        }
        ClaimScope::BoundedFieldOperation=>{
            for role in [ComponentRole::MicrogridController,ComponentRole::PowerConversionSystem,ComponentRole::ProtectionRelay,ComponentRole::Meter]{
                if !component_is_physical(e,role){reasons.push(format!("required-field-physical-component-missing-{role:?}"));}
            }
            match &e.site_sampling_frame{
                Some(s)=>{
                    if !sha(&s.site_profile_sha256)||s.installed_configuration_sha256!=r.product_configuration_sha256||
                       s.observation_end_epoch_s<=s.observation_start_epoch_s||
                       s.planned_mission_count==0||s.observed_mission_count>s.planned_mission_count||
                       !s.data_coverage_fraction.is_finite()||
                       s.data_coverage_fraction<r.minimum_field_data_coverage_fraction||
                       r.required_field_regime_ids.iter().any(|x|!s.observed_regime_ids.contains(x))||
                       !nonblank(&s.sampling_evidence_reference){
                        reasons.push("field-sampling-frame-incomplete-or-out-of-scope".into());
                    }
                }
                None=>reasons.push("field-sampling-frame-missing".into()),
            }
        }
    }
    let disposition=if reasons.is_empty(){ClaimSupportDisposition::StructurallySupportedPendingMycelixVerification}else{ClaimSupportDisposition::NotSupported};
    GfmClaimAssessment{claim_id:r.claim_id.clone(),evidence_id:r.evidence_id.clone(),disposition,
        exclusion_reasons:reasons,mycelix_verification_required:true,independent_review_required:true,
        claim_release_permitted:false,claim_ceiling:CLAIM_CEILING}
}

/// Assesses whether executed test evidence is structurally bounded enough to
/// proceed to external trust verification and independent technical review.
/// Mycelix receipt references are required but are not authenticated here.
/// This function never issues, releases, or certifies a capability claim.
pub fn evaluate_gfm_claim_eligibility(
    requests:&[GfmClaimRequest],
    evidence:&[ExecutedGfmTestEvidence],
)->Result<GfmClaimEligibilityReport,AuditError>{
    if requests.is_empty(){return Err(AuditError("claim request set is empty".into()));}
    unique(requests.iter().map(|x|x.claim_id.as_str()),"claim IDs")?;
    unique(evidence.iter().map(|x|x.evidence_id.as_str()),"evidence IDs")?;
    for r in requests{validate_request(r)?;}
    let by_id:std::collections::BTreeMap<&str,&ExecutedGfmTestEvidence>=evidence.iter().map(|x|(x.evidence_id.as_str(),x)).collect();
    let mut assessments=Vec::new();
    for r in requests{assessments.push(assess_one(r,by_id.get(r.evidence_id.as_str()).copied()));}
    assessments.sort_by(|a,b|a.claim_id.cmp(&b.claim_id));
    let structurally_supported_count=assessments.iter().filter(|x|x.disposition==ClaimSupportDisposition::StructurallySupportedPendingMycelixVerification).count();
    let excluded_count=assessments.len()-structurally_supported_count;
    Ok(GfmClaimEligibilityReport{assessments,structurally_supported_count,excluded_count,
        claim_release_permitted:false,claim_ceiling:CLAIM_CEILING})
}

#[cfg(test)]
mod tests{
 use super::*;
 fn h(c:char)->String{std::iter::repeat(c).take(64).collect()}
 fn component(id:&str,role:ComponentRole,realization:Realization)->ComponentEvidence{
  let digest=match role{
   ComponentRole::MicrogridController=>h('e'),
   ComponentRole::PowerConversionSystem|ComponentRole::PowerInterface|ComponentRole::Meter=>h('c'),
   ComponentRole::ProtectionRelay=>h('f'),
   _=>h('a'),
  };
  ComponentEvidence{component_id:id.into(),role,realization,configuration_sha256:digest}
 }
 fn evidence(mode:ExecutionMode)->ExecutedGfmTestEvidence{
  let mut components=vec![component("controller",ComponentRole::MicrogridController,Realization::Physical)];
  match mode{
   ExecutionMode::ModelOnly=>{components[0].realization=Realization::Simulated;}
   ExecutionMode::ControllerHil=>{components.push(component("pcs-model",ComponentRole::PowerConversionSystem,Realization::Simulated));}
   ExecutionMode::PowerHil=>{components.push(component("pcs",ComponentRole::PowerConversionSystem,Realization::Physical));components.push(component("pif",ComponentRole::PowerInterface,Realization::Physical));}
   ExecutionMode::PhysicalLaboratory|ExecutionMode::SiteCommissioning|ExecutionMode::PeriodicFieldOperation=>{
    components.push(component("pcs",ComponentRole::PowerConversionSystem,Realization::Physical));
    components.push(component("protection",ComponentRole::ProtectionRelay,Realization::Physical));
    components.push(component("meter",ComponentRole::Meter,Realization::Physical));
    if mode==ExecutionMode::PowerHil{components.push(component("pif",ComponentRole::PowerInterface,Realization::Physical));}
   }
  }
  ExecutedGfmTestEvidence{
   evidence_id:"EVIDENCE-1".into(),gfm_profile_sha256:h('b'),gfm_profile_id:"GFM-PROFILE".into(),gfm_test_id:"GFM-01".into(),
   family:GfmFamily::OperatingPointHeadroom,operating_cell_id:"CELL-1".into(),electrical_boundary:ElectricalBoundary::ConverterTerminals,
   execution_mode:mode,run_state:RunState::Executed,run_result:RunResult::Pass,
   product_configuration_sha256:h('a'),hardware_configuration_sha256:h('c'),firmware_sha256:h('d'),
   controller_configuration_sha256:h('e'),protection_configuration_sha256:h('f'),topology_sha256:h('1'),
   environment_passport_sha256:h('2'),analysis_build_sha256:h('3'),criterion_reference:"criterion:rev-1".into(),
   test_procedure_reference:"procedure:rev-1".into(),run_record_sha256:h('4'),raw_data_sha256:h('5'),
   observed_observable_ids:vec!["active_power_pu".into(),"reactive_power_pu".into()],
   components,model_binding:None,calibration_receipt_reference:Some("calibration:receipt".into()),
   timebase_reference:"timebase:trace".into(),independent_witness_reference:Some("witness:receipt".into()),
   mycelix_receipt_reference:"mycelix:receipt-pending".into(),mycelix_policy_revision_reference:"mycelix:policy-v1".into(),
   site_sampling_frame:if matches!(mode,ExecutionMode::SiteCommissioning|ExecutionMode::PeriodicFieldOperation){
    Some(FieldSamplingFrame{site_profile_sha256:h('6'),installed_configuration_sha256:h('a'),
     observation_start_epoch_s:100,observation_end_epoch_s:200,planned_mission_count:10,observed_mission_count:10,
     data_coverage_fraction:1.0,observed_regime_ids:vec!["nominal".into(),"peak".into()],sampling_evidence_reference:"sampling:ref".into()})
   }else{None}
  }
 }
 fn request(scope:ClaimScope)->GfmClaimRequest{
  GfmClaimRequest{claim_id:"CLAIM-1".into(),description:"test claim".into(),evidence_id:"EVIDENCE-1".into(),requested_scope:scope,
   family:GfmFamily::OperatingPointHeadroom,operating_cell_id:"CELL-1".into(),electrical_boundary:ElectricalBoundary::ConverterTerminals,
   product_configuration_sha256:h('a'),hardware_configuration_sha256:h('c'),firmware_sha256:h('d'),
   controller_configuration_sha256:h('e'),protection_configuration_sha256:h('f'),topology_sha256:h('1'),
   required_observable_ids:vec!["active_power_pu".into(),"reactive_power_pu".into()],
   required_model_scopes:vec![],required_field_regime_ids:vec![],minimum_field_data_coverage_fraction:0.9}
 }
 fn result(r:GfmClaimAssessment)->GfmClaimAssessment{r}

 #[test]fn controller_hil_supports_only_bounded_controller_claim(){
  let report=evaluate_gfm_claim_eligibility(&[request(ClaimScope::ControllerImplementation)],&[evidence(ExecutionMode::ControllerHil)]).unwrap();
  assert_eq!(report.structurally_supported_count,1);assert!(!report.claim_release_permitted);
 }
 #[test]fn controller_hil_cannot_support_physical_power_interface_claim(){
  let report=evaluate_gfm_claim_eligibility(&[request(ClaimScope::PhysicalPowerInterface)],&[evidence(ExecutionMode::ControllerHil)]).unwrap();
  assert_eq!(report.assessments[0].disposition,ClaimSupportDisposition::NotSupported);
 }
 #[test]fn model_only_cannot_support_physical_controller_claim(){
  let report=evaluate_gfm_claim_eligibility(&[request(ClaimScope::ControllerImplementation)],&[evidence(ExecutionMode::ModelOnly)]).unwrap();
  assert_eq!(report.assessments[0].disposition,ClaimSupportDisposition::NotSupported);
 }
 #[test]fn failed_or_inconclusive_run_never_supports_claim(){
  let mut e=evidence(ExecutionMode::PhysicalLaboratory);e.run_result=RunResult::Fail;
  assert_eq!(evaluate_gfm_claim_eligibility(&[request(ClaimScope::LaboratoryDevicePerformance)],&[e]).unwrap().excluded_count,1);
 }
 #[test]fn planned_run_never_supports_claim(){
  let mut e=evidence(ExecutionMode::ControllerHil);e.run_state=RunState::Planned;
  assert_eq!(evaluate_gfm_claim_eligibility(&[request(ClaimScope::ControllerImplementation)],&[e]).unwrap().excluded_count,1);
 }
 #[test]fn exact_electrical_boundary_is_required(){
  let mut r=request(ClaimScope::ControllerImplementation);r.electrical_boundary=ElectricalBoundary::PlantPointOfConnection;
  assert_eq!(evaluate_gfm_claim_eligibility(&[r],&[evidence(ExecutionMode::ControllerHil)]).unwrap().excluded_count,1);
 }
 #[test]fn product_or_firmware_configuration_mismatch_rejects(){
  let mut r=request(ClaimScope::ControllerImplementation);r.firmware_sha256=h('9');
  assert_eq!(evaluate_gfm_claim_eligibility(&[r],&[evidence(ExecutionMode::ControllerHil)]).unwrap().excluded_count,1);
 }
 #[test]fn missing_observable_rejects(){
  let mut e=evidence(ExecutionMode::ControllerHil);e.observed_observable_ids.clear();
  assert_eq!(evaluate_gfm_claim_eligibility(&[request(ClaimScope::ControllerImplementation)],&[e]).unwrap().excluded_count,1);
 }
 #[test]fn physical_power_interface_requires_physical_pcs_and_interface(){
  let report=evaluate_gfm_claim_eligibility(&[request(ClaimScope::PhysicalPowerInterface)],&[evidence(ExecutionMode::PowerHil)]).unwrap();
  assert_eq!(report.structurally_supported_count,1);
 }
 #[test]fn lab_performance_requires_physical_device_protection_and_meter(){
  let report=evaluate_gfm_claim_eligibility(&[request(ClaimScope::LaboratoryDevicePerformance)],&[evidence(ExecutionMode::PhysicalLaboratory)]).unwrap();
  assert_eq!(report.structurally_supported_count,1);
 }
 #[test]fn commissioning_requires_site_and_installed_config_match(){
  let report=evaluate_gfm_claim_eligibility(&[request(ClaimScope::InstalledCommissioning)],&[evidence(ExecutionMode::SiteCommissioning)]).unwrap();
  assert_eq!(report.structurally_supported_count,1);
 }
 #[test]fn field_claim_requires_sampling_frame_coverage_and_regime(){
  let mut r=request(ClaimScope::BoundedFieldOperation);r.required_field_regime_ids=vec!["peak".into()];
  assert_eq!(evaluate_gfm_claim_eligibility(&[r],&[evidence(ExecutionMode::PeriodicFieldOperation)]).unwrap().structurally_supported_count,1);
  let mut e=evidence(ExecutionMode::PeriodicFieldOperation);e.site_sampling_frame.as_mut().unwrap().data_coverage_fraction=0.5;
  let mut r=request(ClaimScope::BoundedFieldOperation);r.minimum_field_data_coverage_fraction=0.9;
  assert_eq!(evaluate_gfm_claim_eligibility(&[r],&[e]).unwrap().excluded_count,1);
 }
 #[test]fn mycelix_receipt_and_policy_references_are_mandatory(){
  let mut e=evidence(ExecutionMode::ControllerHil);e.mycelix_receipt_reference.clear();
  let report=evaluate_gfm_claim_eligibility(&[request(ClaimScope::ControllerImplementation)],&[e]).unwrap();
  assert_eq!(report.excluded_count,1);
 }
 #[test]fn model_conditioned_claim_requires_matching_model_validation_binding(){
  let mut e=evidence(ExecutionMode::ModelOnly);e.model_binding=Some(ModelBinding{
   model_validity_profile_sha256:h('7'),model_sha256:h('8'),product_hardware_sha256:h('c'),firmware_sha256:h('d'),
   controller_configuration_sha256:h('e'),protection_configuration_sha256:h('f'),topology_sha256:h('1'),
   validated_cell_id:"CELL-1".into(),validated_scopes:vec![ModelScope::TimeDomain]
  });
  let mut r=request(ClaimScope::ModelConditionedResponse);r.required_model_scopes=vec![ModelScope::TimeDomain];
  assert_eq!(evaluate_gfm_claim_eligibility(&[r],&[e]).unwrap().structurally_supported_count,1);
 }
 #[test]fn model_conditioned_claim_outside_validation_scope_rejects(){
  let mut e=evidence(ExecutionMode::ModelOnly);e.model_binding=Some(ModelBinding{
   model_validity_profile_sha256:h('7'),model_sha256:h('8'),product_hardware_sha256:h('c'),firmware_sha256:h('d'),
   controller_configuration_sha256:h('e'),protection_configuration_sha256:h('f'),topology_sha256:h('1'),
   validated_cell_id:"OTHER".into(),validated_scopes:vec![ModelScope::TimeDomain]
  });
  let mut r=request(ClaimScope::ModelConditionedResponse);r.required_model_scopes=vec![ModelScope::FrequencyDomain];
  assert_eq!(evaluate_gfm_claim_eligibility(&[r],&[e]).unwrap().excluded_count,1);
 }
 #[test]fn claim_release_is_never_authorized_by_this_structural_evaluator(){
  let report=evaluate_gfm_claim_eligibility(&[request(ClaimScope::ControllerImplementation)],&[evidence(ExecutionMode::ControllerHil)]).unwrap();
  assert!(!report.claim_release_permitted);assert!(report.assessments[0].mycelix_verification_required);
 }
 #[test]fn component_configuration_digests_must_match_claim_identity(){
  let mut e=evidence(ExecutionMode::ControllerHil);
  e.components.iter_mut().find(|c|c.role==ComponentRole::MicrogridController).unwrap().configuration_sha256=h('9');
  let report=evaluate_gfm_claim_eligibility(&[request(ClaimScope::ControllerImplementation)],&[e]).unwrap();
  assert_eq!(report.excluded_count,1);
 }
 #[test]fn physical_execution_requires_calibration_and_independent_witness(){
  let mut e=evidence(ExecutionMode::PhysicalLaboratory);
  e.independent_witness_reference=None;
  let report=evaluate_gfm_claim_eligibility(&[request(ClaimScope::LaboratoryDevicePerformance)],&[e]).unwrap();
  assert_eq!(report.excluded_count,1);
 }

}
