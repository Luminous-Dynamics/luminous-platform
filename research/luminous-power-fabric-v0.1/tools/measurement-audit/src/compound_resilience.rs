use crate::gfm_capability_envelope::{GfmCapabilityProfile,GfmExecutionMode,GfmTestFamily};
use crate::model_validity_envelope::{ModelValidityProfile,PhysicalReferenceMode,ValidationScope};
use crate::mode_transition_corpus::ModeTransitionCorpus;
use crate::AuditError;

const CLAIM:&str="SYNTHETIC_OR_STRUCTURAL_COMPOUND_RESILIENCE_QUALIFICATION_ONLY_NOT_HIL_OR_FIELD_CLAIM";
const EPS:f64=1e-9;

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum FactorialArm{Baseline,PhysicalOnly,CyberOnly,Compound}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum PhysicalDisturbance{WeakGrid,SymmetricalFault,NegativeSequenceFault,PhaseAngleJump,LoadStepOrMotorStart,SourceTrip,IslandingOrPccTransition,BlackStart}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum CyberManipulation{FrequencyReferenceFdi,VoltageReferenceFdi,PowerFeedbackFdi,PccVoltageFdi,BreakerFeedbackSpoof,CommandReplay,MessageDelayOrReordering,TimebaseManipulation,SplitBrainLeadership,AuthorityScopeEscalation,DenialOfService}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum ResilienceExecutionMode{EmtSimulation,ControllerHil,PowerHil,PhysicalLaboratory,SiteCommissioning,FieldOperation}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum ResilienceEvidenceStatus{Current,Expired,Revoked,Unknown}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum ResilienceEvidenceClass{Synthetic,Modelled,Measured,Accredited}

#[derive(Debug,Clone,PartialEq)]
pub struct ResilienceEvidence{
 pub status:ResilienceEvidenceStatus,pub class:ResilienceEvidenceClass,
 pub dataset_sha256:String,pub raw_data_sha256:String,pub calibration_receipt_reference:String,
 pub timebase_reference:String,pub independent_witness_reference:String,pub result_bundle_reference:String,
}
#[derive(Debug,Clone,PartialEq)]
pub struct ResilienceOutcome{
 pub critical_service_unserved_wh:f64,pub terminal_reserve_wh:f64,
 pub maximum_abs_voltage_deviation_pu:f64,pub maximum_abs_frequency_deviation_hz:f64,
 pub maximum_rocof_hz_per_s:f64,pub recovery_duration_ms:f64,
 pub attack_detected:bool,pub mitigation_succeeded:bool,
 pub attack_detection_latency_ms:Option<f64>,pub mitigation_latency_ms:Option<f64>,
 pub false_positive_alert_count:u32,pub control_deadline_miss_count:u32,
 pub protection_actions_correct:bool,pub forbidden_transitions_observed:u32,
 pub unknown_state_mutations:u32,pub post_state_id:String,pub post_state_evidence_sha256:String,
}
#[derive(Debug,Clone,PartialEq)]
pub struct FactorialRun{
 pub run_id:String,pub arm:FactorialArm,
 pub physical_disturbance:Option<PhysicalDisturbance>,pub cyber_manipulation:Option<CyberManipulation>,
 pub gfm_profile_id:String,pub gfm_cell_id:String,pub gfm_test_id:String,
 pub model_profile_id:String,pub model_validation_case_id:String,
 pub product_configuration_sha256:String,pub firmware_sha256:String,
 pub controller_configuration_sha256:String,pub protection_configuration_sha256:String,
 pub topology_sha256:String,pub model_sha256:String,pub analysis_build_sha256:String,
 pub site_profile_sha256:String,pub scenario_seed_sha256:String,
 pub initial_state_id:String,pub initial_state_epoch:u64,pub initial_soc_fraction:f64,
 pub timebase_reference:String,pub observation_window_sha256:String,pub acceptance_criteria_sha256:String,
 pub physical_profile_sha256:String,pub cyber_profile_sha256:String,
 pub physical_event_offset_ms:Option<f64>,pub cyber_event_offset_ms:Option<f64>,
 pub transition_ids:Vec<String>,pub execution_mode:ResilienceExecutionMode,
 pub evidence:ResilienceEvidence,pub outcome:ResilienceOutcome,
}
#[derive(Debug,Clone,PartialEq)]
pub struct ResilienceAcceptanceCriteria{
 pub criteria_reference:String,pub criteria_sha256:String,
 pub maximum_unserved_critical_wh:f64,pub minimum_terminal_reserve_wh:f64,
 pub maximum_voltage_deviation_pu:f64,pub maximum_frequency_deviation_hz:f64,
 pub maximum_rocof_hz_per_s:f64,pub maximum_recovery_duration_ms:f64,
 pub maximum_control_deadline_misses:u32,pub maximum_attack_detection_latency_ms:f64,
 pub maximum_mitigation_latency_ms:f64,pub maximum_false_positive_alerts_per_no_attack_run:u32,
 pub maximum_positive_interaction_unserved_wh:f64,pub maximum_positive_interaction_voltage_pu:f64,
 pub maximum_positive_interaction_frequency_hz:f64,pub maximum_positive_interaction_rocof_hz_per_s:f64,
 pub maximum_positive_interaction_recovery_ms:f64,pub maximum_positive_interaction_reserve_deficit_wh:f64,
}
#[derive(Debug,Clone,PartialEq)]
pub struct CompoundResilienceBlock{
 pub block_id:String,pub expected_physical_disturbance:PhysicalDisturbance,
 pub expected_cyber_manipulation:CyberManipulation,
 pub physical_profile_reference:String,pub physical_profile_sha256:String,
 pub cyber_profile_reference:String,pub cyber_profile_sha256:String,
 pub physical_event_offset_ms:f64,pub cyber_event_offset_ms:f64,
 pub acceptance:ResilienceAcceptanceCriteria,pub runs:Vec<FactorialRun>,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum InteractionMetric{UnservedCriticalWh,VoltageDeviationPu,FrequencyDeviationHz,RoCoFHzPerS,RecoveryDurationMs,TerminalReserveDeficitWh}
#[derive(Debug,Clone,PartialEq)]
pub struct InteractionContrast{
 pub metric:InteractionMetric,pub baseline:f64,pub physical_only:f64,pub cyber_only:f64,pub compound:f64,
 pub interaction_delta:f64,pub maximum_permitted_positive_delta:f64,pub within_interaction_limit:bool,
}
#[derive(Debug,Clone,PartialEq)]
pub struct CompoundResilienceReport{
 pub block_id:String,pub structural_complete:bool,pub model_validation_eligible:bool,
 pub all_four_arms_passed:bool,pub compound_resilience_eligible:bool,pub physical_evidence_eligible:bool,
 pub interaction_contrasts:Vec<InteractionContrast>,pub failed_run_ids:Vec<String>,
 pub exclusion_reasons:Vec<String>,pub claim_ceiling:&'static str,
}
impl CompoundResilienceReport{pub const fn claim_ceiling(&self)->&'static str{CLAIM}}

fn nonblank(x:&str)->bool{!x.trim().is_empty()}
fn digest(x:&str)->bool{x.len()==64&&x.bytes().all(|b|b.is_ascii_hexdigit())}
fn finite(x:f64)->bool{x.is_finite()&&x>=0.0}
fn close(a:f64,b:f64)->bool{(a-b).abs()<=EPS}
fn physical_mode(m:ResilienceExecutionMode)->bool{matches!(m,ResilienceExecutionMode::PowerHil|ResilienceExecutionMode::PhysicalLaboratory|ResilienceExecutionMode::SiteCommissioning|ResilienceExecutionMode::FieldOperation)}
fn physical_evidence(e:&ResilienceEvidence,m:ResilienceExecutionMode)->bool{
 physical_mode(m)&&e.status==ResilienceEvidenceStatus::Current
 &&matches!(e.class,ResilienceEvidenceClass::Measured|ResilienceEvidenceClass::Accredited)
 &&digest(&e.dataset_sha256)&&digest(&e.raw_data_sha256)
 &&nonblank(&e.calibration_receipt_reference)&&nonblank(&e.timebase_reference)
 &&nonblank(&e.independent_witness_reference)&&nonblank(&e.result_bundle_reference)
}
fn outcome_valid(o:&ResilienceOutcome)->bool{
 finite(o.critical_service_unserved_wh)&&finite(o.terminal_reserve_wh)
 &&finite(o.maximum_abs_voltage_deviation_pu)&&finite(o.maximum_abs_frequency_deviation_hz)
 &&finite(o.maximum_rocof_hz_per_s)&&finite(o.recovery_duration_ms)
 &&o.attack_detection_latency_ms.is_none_or(finite)&&o.mitigation_latency_ms.is_none_or(finite)
 &&nonblank(&o.post_state_id)&&digest(&o.post_state_evidence_sha256)
}
fn run_passes(r:&FactorialRun,c:&ResilienceAcceptanceCriteria)->bool{
 let attack=if r.cyber_manipulation.is_some(){
  r.outcome.attack_detected&&r.outcome.mitigation_succeeded
   &&r.outcome.attack_detection_latency_ms.is_some_and(|x|x<=c.maximum_attack_detection_latency_ms)
   &&r.outcome.mitigation_latency_ms.is_some_and(|x|x<=c.maximum_mitigation_latency_ms)
 }else{r.outcome.false_positive_alert_count<=c.maximum_false_positive_alerts_per_no_attack_run};
 r.outcome.critical_service_unserved_wh<=c.maximum_unserved_critical_wh
 &&r.outcome.terminal_reserve_wh+EPS>=c.minimum_terminal_reserve_wh
 &&r.outcome.maximum_abs_voltage_deviation_pu<=c.maximum_voltage_deviation_pu
 &&r.outcome.maximum_abs_frequency_deviation_hz<=c.maximum_frequency_deviation_hz
 &&r.outcome.maximum_rocof_hz_per_s<=c.maximum_rocof_hz_per_s
 &&r.outcome.recovery_duration_ms<=c.maximum_recovery_duration_ms
 &&r.outcome.control_deadline_miss_count<=c.maximum_control_deadline_misses
 &&r.outcome.protection_actions_correct&&r.outcome.forbidden_transitions_observed==0
 &&r.outcome.unknown_state_mutations==0&&attack
}
fn reserve_deficit(r:&FactorialRun,c:&ResilienceAcceptanceCriteria)->f64{(c.minimum_terminal_reserve_wh-r.outcome.terminal_reserve_wh).max(0.0)}
fn make_contrast(m:InteractionMetric,v:[f64;4],limit:f64)->InteractionContrast{
 let delta=v[3]-v[1]-v[2]+v[0];
 InteractionContrast{metric:m,baseline:v[0],physical_only:v[1],cyber_only:v[2],compound:v[3],
  interaction_delta:delta,maximum_permitted_positive_delta:limit,within_interaction_limit:delta<=limit+EPS}
}
fn validate_criteria(c:&ResilienceAcceptanceCriteria)->Result<(),AuditError>{
 if !nonblank(&c.criteria_reference)||!digest(&c.criteria_sha256)
 ||[c.maximum_unserved_critical_wh,c.minimum_terminal_reserve_wh,c.maximum_voltage_deviation_pu,
 c.maximum_frequency_deviation_hz,c.maximum_rocof_hz_per_s,c.maximum_recovery_duration_ms,
 c.maximum_attack_detection_latency_ms,c.maximum_mitigation_latency_ms,
 c.maximum_positive_interaction_unserved_wh,c.maximum_positive_interaction_voltage_pu,
 c.maximum_positive_interaction_frequency_hz,c.maximum_positive_interaction_rocof_hz_per_s,
 c.maximum_positive_interaction_recovery_ms,c.maximum_positive_interaction_reserve_deficit_wh].iter().any(|x|!finite(*x)){
  return Err(AuditError("compound acceptance criteria are invalid".into()));
 }Ok(())
}
fn planned_mode(run:&FactorialRun)->GfmExecutionMode{match run.execution_mode{
 ResilienceExecutionMode::EmtSimulation=>GfmExecutionMode::EmtSimulation,
 ResilienceExecutionMode::ControllerHil=>GfmExecutionMode::ControllerHil,
 ResilienceExecutionMode::PowerHil=>GfmExecutionMode::PowerHil,
 ResilienceExecutionMode::PhysicalLaboratory=>GfmExecutionMode::PhysicalLaboratory,
 ResilienceExecutionMode::SiteCommissioning=>GfmExecutionMode::SiteCommissioning,
 ResilienceExecutionMode::FieldOperation=>GfmExecutionMode::FieldOperation,
}}
fn context_matches(a:&FactorialRun,b:&FactorialRun)->bool{
 a.gfm_profile_id==b.gfm_profile_id&&a.gfm_cell_id==b.gfm_cell_id&&a.gfm_test_id==b.gfm_test_id
 &&a.model_profile_id==b.model_profile_id&&a.model_validation_case_id==b.model_validation_case_id
 &&a.product_configuration_sha256==b.product_configuration_sha256&&a.firmware_sha256==b.firmware_sha256
 &&a.controller_configuration_sha256==b.controller_configuration_sha256&&a.protection_configuration_sha256==b.protection_configuration_sha256
 &&a.topology_sha256==b.topology_sha256&&a.model_sha256==b.model_sha256&&a.analysis_build_sha256==b.analysis_build_sha256
 &&a.site_profile_sha256==b.site_profile_sha256&&a.scenario_seed_sha256==b.scenario_seed_sha256
 &&a.initial_state_id==b.initial_state_id&&a.initial_state_epoch==b.initial_state_epoch
 &&close(a.initial_soc_fraction,b.initial_soc_fraction)&&a.timebase_reference==b.timebase_reference
 &&a.observation_window_sha256==b.observation_window_sha256&&a.acceptance_criteria_sha256==b.acceptance_criteria_sha256
 &&a.physical_profile_sha256==b.physical_profile_sha256&&a.cyber_profile_sha256==b.cyber_profile_sha256
 &&a.transition_ids==b.transition_ids&&a.execution_mode==b.execution_mode
 &&a.outcome.post_state_id==b.outcome.post_state_id
}
fn get_arm(b:&CompoundResilienceBlock,arm:FactorialArm)->Result<&FactorialRun,AuditError>{
 b.runs.iter().find(|r|r.arm==arm).ok_or_else(||AuditError(format!("missing {arm:?} arm")))
}
fn validate_bindings(b:&CompoundResilienceBlock,g:&GfmCapabilityProfile,m:&ModelValidityProfile,t:&ModeTransitionCorpus)->Result<(),AuditError>{
 let first=b.runs.first().ok_or_else(||AuditError("empty factorial block".into()))?;
 if first.gfm_profile_id!=g.profile_id||first.model_profile_id!=m.profile_id||m.gfm_profile_reference!=g.profile_id
 ||m.out_of_domain_policy!=g.out_of_domain_policy{return Err(AuditError("GFM/model profile identity mismatch".into()));}
 let cell=g.operating_cells.iter().find(|x|x.cell_id==first.gfm_cell_id).ok_or_else(||AuditError("unknown GFM cell".into()))?;
 let test=g.tests.iter().find(|x|x.test_id==first.gfm_test_id).ok_or_else(||AuditError("unknown GFM test".into()))?;
 if !test.operating_cell_ids.contains(&first.gfm_cell_id)||cell.product_configuration_sha256!=first.product_configuration_sha256
 ||cell.topology_sha256!=first.topology_sha256||cell.controller_configuration_sha256!=first.controller_configuration_sha256
 ||cell.protection_configuration_sha256!=first.protection_configuration_sha256{
  return Err(AuditError("tested run does not match its GFM cell/configuration".into()));
 }
 let model_case=m.validation_cases.iter().find(|x|x.case_id==first.model_validation_case_id)
  .ok_or_else(||AuditError("model validation case missing".into()))?;
 let model_cell=m.operating_cells.iter().find(|x|x.cell_id==model_case.cell_id)
  .ok_or_else(||AuditError("model operating cell missing".into()))?;
 let family_matches=match b.expected_physical_disturbance{
  PhysicalDisturbance::WeakGrid=>test.family==GfmTestFamily::WeakGridScrXr,
  PhysicalDisturbance::SymmetricalFault=>test.family==GfmTestFamily::CurrentLimitFaultRideThrough,
  PhysicalDisturbance::NegativeSequenceFault=>test.family==GfmTestFamily::NegativeSequenceUnbalancedFault,
  PhysicalDisturbance::PhaseAngleJump=>test.family==GfmTestFamily::PhaseAngleJump,
  PhysicalDisturbance::LoadStepOrMotorStart|PhysicalDisturbance::SourceTrip|PhysicalDisturbance::IslandingOrPccTransition|PhysicalDisturbance::BlackStart=>test.family==GfmTestFamily::OperatingPointHeadroom,
 };
 if !family_matches||test.disturbance_profile_reference!=b.physical_profile_reference{
  return Err(AuditError("physical disturbance profile is not bound to the GFM test family/reference".into()));
 }
 let scope=if test.family==GfmTestFamily::FrequencyDomainImpedanceScan{ValidationScope::FrequencyDomain}else{ValidationScope::TimeDomain};
 if model_cell.source_gfm_profile_id!=g.profile_id||model_cell.source_gfm_cell_id!=cell.cell_id
 ||m.identity.controller_configuration_sha256!=first.controller_configuration_sha256
 ||m.identity.protection_configuration_sha256!=first.protection_configuration_sha256
 ||m.identity.topology_sha256!=first.topology_sha256||m.identity.firmware_sha256!=first.firmware_sha256
 ||m.identity.model_sha256!=first.model_sha256||m.identity.analysis_build_sha256!=first.analysis_build_sha256
 ||!model_case.held_out_validation_data||model_case.scope!=scope
 ||model_case.dataset_sha256!=m.validation_dataset_sha256
 ||!digest(&model_case.raw_data_sha256)||!digest(&model_case.processing_build_sha256)
 ||!nonblank(&model_case.calibration_receipt_reference)||!nonblank(&model_case.timebase_reference)
 ||!nonblank(&model_case.independent_witness_reference)||!nonblank(&model_case.acceptance_criteria_reference)
 ||!matches!((first.execution_mode,model_case.reference_mode),
  (ResilienceExecutionMode::PowerHil,PhysicalReferenceMode::PowerHil)
  |(ResilienceExecutionMode::PhysicalLaboratory,PhysicalReferenceMode::PhysicalLaboratory)
  |(ResilienceExecutionMode::SiteCommissioning,PhysicalReferenceMode::SiteCommissioning)
  |(ResilienceExecutionMode::FieldOperation,PhysicalReferenceMode::FieldEvent)){
  return Err(AuditError("held-out model validation does not bind to the exact GFM cell and physical reference".into()));
 }
 if !t.states.iter().any(|x|x.state_id==first.initial_state_id){return Err(AuditError("unknown initial transition-graph state".into()));}
 for r in &b.runs{
  if !test.planned_execution_modes.contains(&planned_mode(r)){return Err(AuditError(format!("run {:?} uses an unplanned execution mode",r.run_id)));}
  if r.transition_ids.is_empty(){return Err(AuditError(format!("run {:?} has no transition references",r.run_id)));}
  let mut path_state=r.initial_state_id.as_str();
  for id in &r.transition_ids{
   let edge=t.transitions.iter().find(|x|x.transition_id==*id).ok_or_else(||AuditError(format!("unknown transition {id:?}")))?;
   if !edge.from_state_ids.iter().any(|state|state==path_state){return Err(AuditError(format!("transition path is discontinuous in run {:?}",r.run_id)));}
   path_state=edge.to_state_id.as_str();
  }
  if path_state!=r.outcome.post_state_id{return Err(AuditError(format!("transition path does not reach observed post-state in run {:?}",r.run_id)));}
 }
 Ok(())
}

/// Matched baseline/physical-only/cyber-only/compound evaluation. This is a
/// read-only scenario-level factorial contrast; it does not infer population
/// causality, authenticate evidence bytes, run attacks or command equipment.
pub fn evaluate_compound_resilience(b:&CompoundResilienceBlock,g:&GfmCapabilityProfile,m:&ModelValidityProfile,t:&ModeTransitionCorpus)->Result<CompoundResilienceReport,AuditError>{
 if !nonblank(&b.block_id)||b.runs.len()!=4{return Err(AuditError("factorial block requires exactly four runs".into()));}
 if !nonblank(&b.physical_profile_reference)||!digest(&b.physical_profile_sha256)
  ||!nonblank(&b.cyber_profile_reference)||!digest(&b.cyber_profile_sha256)
  ||!b.physical_event_offset_ms.is_finite()||!b.cyber_event_offset_ms.is_finite(){
  return Err(AuditError("physical/cyber profile identity or event timing is invalid".into()));
 }
 validate_criteria(&b.acceptance)?;
 let mut slots=[false;4];
 for r in &b.runs{
  let i=match r.arm{FactorialArm::Baseline=>0,FactorialArm::PhysicalOnly=>1,FactorialArm::CyberOnly=>2,FactorialArm::Compound=>3};
  if slots[i]{return Err(AuditError("duplicate factorial arm".into()));}slots[i]=true;
  if !nonblank(&r.run_id)||![&r.product_configuration_sha256,&r.firmware_sha256,&r.controller_configuration_sha256,
   &r.protection_configuration_sha256,&r.topology_sha256,&r.model_sha256,&r.analysis_build_sha256,
   &r.site_profile_sha256,&r.scenario_seed_sha256,&r.observation_window_sha256,&r.acceptance_criteria_sha256].iter().all(|x|digest(x))
   ||!nonblank(&r.timebase_reference)||!nonblank(&r.initial_state_id)||!finite(r.initial_soc_fraction)||r.initial_soc_fraction>1.0
   ||r.physical_profile_sha256!=b.physical_profile_sha256||r.cyber_profile_sha256!=b.cyber_profile_sha256
   ||r.acceptance_criteria_sha256!=b.acceptance.criteria_sha256||!outcome_valid(&r.outcome){
    return Err(AuditError(format!("run {:?} has invalid context or outcome",r.run_id)));
  }
  match r.arm{
   FactorialArm::Baseline if r.physical_disturbance.is_some()||r.cyber_manipulation.is_some()=>return Err(AuditError("baseline arm contains an active factor".into())),
   FactorialArm::PhysicalOnly if r.physical_disturbance!=Some(b.expected_physical_disturbance)||r.cyber_manipulation.is_some()=>return Err(AuditError("physical-only factor assignment mismatch".into())),
   FactorialArm::CyberOnly if r.physical_disturbance.is_some()||r.cyber_manipulation!=Some(b.expected_cyber_manipulation)=>return Err(AuditError("cyber-only factor assignment mismatch".into())),
   FactorialArm::Compound if r.physical_disturbance!=Some(b.expected_physical_disturbance)||r.cyber_manipulation!=Some(b.expected_cyber_manipulation)=>return Err(AuditError("compound factor assignment mismatch".into())),
   _=>(),
  }
  let e=&r.evidence;
  if e.status!=ResilienceEvidenceStatus::Current||!digest(&e.dataset_sha256)||!digest(&e.raw_data_sha256)
   ||!nonblank(&e.calibration_receipt_reference)||!nonblank(&e.timebase_reference)
   ||!nonblank(&e.result_bundle_reference)||e.timebase_reference!=r.timebase_reference{
   return Err(AuditError(format!("run {:?} has stale or incomplete evidence references",r.run_id)));
  }
 }
 if slots.iter().any(|x|!x){return Err(AuditError("one or more factorial arms missing".into()));}
 let b0=get_arm(b,FactorialArm::Baseline)?;for r in &b.runs{if !context_matches(b0,r){return Err(AuditError(format!("run {:?} is not context-matched",r.run_id)));}}
 validate_bindings(b,g,m,t)?;
 let b0=get_arm(b,FactorialArm::Baseline)?;let p=get_arm(b,FactorialArm::PhysicalOnly)?;
 let c=get_arm(b,FactorialArm::CyberOnly)?;let pc=get_arm(b,FactorialArm::Compound)?;
 let reserve=[reserve_deficit(b0,&b.acceptance),reserve_deficit(p,&b.acceptance),reserve_deficit(c,&b.acceptance),reserve_deficit(pc,&b.acceptance)];
 let contrasts=vec![
  make_contrast(InteractionMetric::UnservedCriticalWh,[b0.outcome.critical_service_unserved_wh,p.outcome.critical_service_unserved_wh,c.outcome.critical_service_unserved_wh,pc.outcome.critical_service_unserved_wh],b.acceptance.maximum_positive_interaction_unserved_wh),
  make_contrast(InteractionMetric::VoltageDeviationPu,[b0.outcome.maximum_abs_voltage_deviation_pu,p.outcome.maximum_abs_voltage_deviation_pu,c.outcome.maximum_abs_voltage_deviation_pu,pc.outcome.maximum_abs_voltage_deviation_pu],b.acceptance.maximum_positive_interaction_voltage_pu),
  make_contrast(InteractionMetric::FrequencyDeviationHz,[b0.outcome.maximum_abs_frequency_deviation_hz,p.outcome.maximum_abs_frequency_deviation_hz,c.outcome.maximum_abs_frequency_deviation_hz,pc.outcome.maximum_abs_frequency_deviation_hz],b.acceptance.maximum_positive_interaction_frequency_hz),
  make_contrast(InteractionMetric::RoCoFHzPerS,[b0.outcome.maximum_rocof_hz_per_s,p.outcome.maximum_rocof_hz_per_s,c.outcome.maximum_rocof_hz_per_s,pc.outcome.maximum_rocof_hz_per_s],b.acceptance.maximum_positive_interaction_rocof_hz_per_s),
  make_contrast(InteractionMetric::RecoveryDurationMs,[b0.outcome.recovery_duration_ms,p.outcome.recovery_duration_ms,c.outcome.recovery_duration_ms,pc.outcome.recovery_duration_ms],b.acceptance.maximum_positive_interaction_recovery_ms),
  make_contrast(InteractionMetric::TerminalReserveDeficitWh,reserve,b.acceptance.maximum_positive_interaction_reserve_deficit_wh),
 ];
 let failed:Vec<String>=b.runs.iter().filter(|r|!run_passes(r,&b.acceptance)).map(|r|r.run_id.clone()).collect();
 let arms_pass=failed.is_empty();let interaction_pass=contrasts.iter().all(|x|x.within_interaction_limit);
 let physical_ok=b.runs.iter().all(|r|physical_evidence(&r.evidence,r.execution_mode));
 let eligible=arms_pass&&interaction_pass;
 let mut reasons=Vec::new();
 if !arms_pass{reasons.push("one-or-more-arms-failed-a-hard-gate".into());}
 if !interaction_pass{reasons.push("compound-interaction-exceeded-preregistered-harm-limit".into());}
 if !physical_ok{reasons.push("evidence-does-not-support-independent-physical-claim".into());}
 Ok(CompoundResilienceReport{block_id:b.block_id.clone(),structural_complete:true,model_validation_eligible:true,
  all_four_arms_passed:arms_pass,compound_resilience_eligible:eligible,physical_evidence_eligible:physical_ok,
  interaction_contrasts:contrasts,failed_run_ids:failed,exclusion_reasons:reasons,claim_ceiling:CLAIM})
}

#[cfg(test)]
mod tests{
 use super::*;
 use crate::gfm_capability_envelope::{GfmElectricalBoundary,GfmOperatingCell,GfmTestCase,GfmTestControls};
 use crate::model_validity_envelope::{ModelFidelity,ModelIdentity,ModelOperatingCell,ModelValidationCase};
 use crate::mode_transition_corpus::{ModeState,ModeTransition};
 fn h(c:char)->String{std::iter::repeat(c).take(64).collect()}
 fn gfm()->GfmCapabilityProfile{
  GfmCapabilityProfile{profile_id:"GFM-01".into(),product_configuration_sha256:h('a'),criterion_bundle_reference:"criteria:gfm".into(),
   operating_cells:vec![GfmOperatingCell{cell_id:"CELL-01".into(),product_configuration_sha256:h('a'),topology_sha256:h('b'),
    controller_configuration_sha256:h('c'),protection_configuration_sha256:h('d'),active_power_pu:.4,reactive_power_pu:.1,
    state_of_charge_fraction:.8,current_headroom_pu:.3,dc_energy_headroom_kwh:20.0,temperature_c:25.0,
    short_circuit_ratio:3.0,grid_x_over_r:10.0,pre_disturbance_voltage_pu:1.0,pre_disturbance_frequency_hz:50.0}],
   tests:vec![GfmTestCase{test_id:"GFM-TEST".into(),family:GfmTestFamily::WeakGridScrXr,
    electrical_boundary:GfmElectricalBoundary::MicrogridPointOfConnection,operating_cell_ids:vec!["CELL-01".into()],
    disturbance_profile_reference:"disturbance:weak-grid".into(),acceptance_criteria_reference:"criteria:gfm".into(),
    planned_execution_modes:vec![GfmExecutionMode::PowerHil],required_observable_ids:vec!["voltage".into(),"frequency".into()],
    participant_configuration_sha256:vec![h('a')],controls:GfmTestControls{positive_control:"p".into(),negative_control:"n".into(),recovery_control:"r".into()}}],
   capability_requests:vec![],out_of_domain_policy:"NO_INTERPOLATION_OR_EXTRAPOLATION_OUTSIDE_EXPLICIT_OPERATING_CELLS".into()}
 }
 fn model()->ModelValidityProfile{
  ModelValidityProfile{profile_id:"MODEL-01".into(),model_fidelity:ModelFidelity::EmtSwitching,
   identity:ModelIdentity{model_sha256:h('e'),product_hardware_sha256:h('f'),firmware_sha256:h('1'),
    controller_configuration_sha256:h('c'),protection_configuration_sha256:h('d'),topology_sha256:h('b'),
    model_parameter_set_sha256:h('2'),analysis_build_sha256:h('3')},
   calibration_campaign_id:"cal".into(),calibration_dataset_sha256:h('4'),validation_campaign_id:"heldout".into(),
   validation_dataset_sha256:h('5'),gfm_profile_reference:"GFM-01".into(),
   out_of_domain_policy:"NO_INTERPOLATION_OR_EXTRAPOLATION_OUTSIDE_EXPLICIT_OPERATING_CELLS".into(),
   operating_cells:vec![ModelOperatingCell{cell_id:"MODEL-CELL".into(),source_gfm_profile_id:"GFM-01".into(),
    source_gfm_cell_id:"CELL-01".into(),operating_cell_sha256:h('6'),active_power_pu:.4,reactive_power_pu:.1,
    state_of_charge_fraction:.8,temperature_c:25.0,short_circuit_ratio:3.0,grid_x_over_r:10.0}],
   validation_cases:vec![ModelValidationCase{case_id:"HELDOUT".into(),cell_id:"MODEL-CELL".into(),scope:ValidationScope::TimeDomain,
    reference_mode:PhysicalReferenceMode::PowerHil,held_out_validation_data:true,dataset_sha256:h('5'),raw_data_sha256:h('7'),
    processing_build_sha256:h('3'),calibration_receipt_reference:"cal:receipt".into(),timebase_reference:"timebase:locked".into(),
    independent_witness_reference:"witness:lab".into(),acceptance_criteria_reference:"criteria:gfm".into(),
    required_observable_ids:vec!["voltage".into()],residuals:vec![]}],claim_requests:vec![]}
 }
 fn graph()->ModeTransitionCorpus{
  ModeTransitionCorpus{initial_state_ids:vec!["grid".into()],states:vec![
   ModeState{state_id:"grid".into(),topology_sha256:h('b'),configuration_sha256:h('c'),pcc_breaker_state:"closed".into(),
    grid_state:"connected".into(),synchronization_state:"locked".into(),grid_forming_state:"standby".into(),
    protection_state:"ready".into(),communications_state:"healthy".into(),critical_service_state:"served".into(),reserve_state:"available".into()},
   ModeState{state_id:"island".into(),topology_sha256:h('b'),configuration_sha256:h('c'),pcc_breaker_state:"open".into(),
    grid_state:"islanded".into(),synchronization_state:"n/a".into(),grid_forming_state:"active".into(),
    protection_state:"ready".into(),communications_state:"healthy".into(),critical_service_state:"served".into(),reserve_state:"available".into()}],
   guards:vec![],transitions:vec![ModeTransition{transition_id:"T-ISLAND".into(),from_state_ids:vec!["grid".into()],
    to_state_id:"island".into(),permitted:true,actor:"local_controller".into(),authorization_owner:"local_controller".into(),
    protection_interlock_required:true,guard_ids:vec!["pcc-open".into()],postcondition_ids:vec!["island-observed".into()],
    failure_disposition:"hold".into(),recovery_transition_id:None,forbidden_reason:None}],test_cases:vec![]}
 }
 fn evidence()->ResilienceEvidence{ResilienceEvidence{status:ResilienceEvidenceStatus::Current,class:ResilienceEvidenceClass::Synthetic,
  dataset_sha256:h('8'),raw_data_sha256:h('9'),calibration_receipt_reference:"synthetic:cal".into(),
  timebase_reference:"timebase:locked".into(),independent_witness_reference:"synthetic:witness".into(),result_bundle_reference:"synthetic:result".into()}}
 fn outcome(u:f64,v:f64,f:f64,r:f64,recovery:f64,reserve:f64,attack:bool)->ResilienceOutcome{
  ResilienceOutcome{critical_service_unserved_wh:u,terminal_reserve_wh:reserve,maximum_abs_voltage_deviation_pu:v,
   maximum_abs_frequency_deviation_hz:f,maximum_rocof_hz_per_s:r,recovery_duration_ms:recovery,
   attack_detected:attack,mitigation_succeeded:attack,attack_detection_latency_ms:if attack{Some(20.0)}else{None},
   mitigation_latency_ms:if attack{Some(80.0)}else{None},false_positive_alert_count:0,control_deadline_miss_count:0,
   protection_actions_correct:true,forbidden_transitions_observed:0,unknown_state_mutations:0,post_state_id:"island".into(),
   post_state_evidence_sha256:h('a')}}
 fn run(id:&str,arm:FactorialArm,p:Option<PhysicalDisturbance>,c:Option<CyberManipulation>,
  u:f64,v:f64,f:f64,r:f64,rec:f64,res:f64)->FactorialRun{
  FactorialRun{run_id:id.into(),arm,physical_disturbance:p,cyber_manipulation:c,gfm_profile_id:"GFM-01".into(),
   gfm_cell_id:"CELL-01".into(),gfm_test_id:"GFM-TEST".into(),model_profile_id:"MODEL-01".into(),model_validation_case_id:"HELDOUT".into(),
   product_configuration_sha256:h('a'),firmware_sha256:h('1'),controller_configuration_sha256:h('c'),
   protection_configuration_sha256:h('d'),topology_sha256:h('b'),model_sha256:h('e'),analysis_build_sha256:h('3'),
   site_profile_sha256:h('f'),scenario_seed_sha256:h('2'),initial_state_id:"grid".into(),initial_state_epoch:7,
   initial_soc_fraction:.8,timebase_reference:"timebase:locked".into(),observation_window_sha256:h('4'),
   acceptance_criteria_sha256:h('5'),physical_profile_sha256:h('6'),cyber_profile_sha256:h('7'),
   physical_event_offset_ms:match arm{FactorialArm::PhysicalOnly|FactorialArm::Compound=>Some(0.0),_=>None},
   cyber_event_offset_ms:match arm{FactorialArm::CyberOnly|FactorialArm::Compound=>Some(25.0),_=>None},
   transition_ids:vec!["T-ISLAND".into()],execution_mode:ResilienceExecutionMode::PowerHil,
   evidence:evidence(),outcome:outcome(u,v,f,r,rec,res,c.is_some())}
 }
 fn criteria()->ResilienceAcceptanceCriteria{ResilienceAcceptanceCriteria{criteria_reference:"criteria:factorial".into(),criteria_sha256:h('5'),
  maximum_unserved_critical_wh:50.0,minimum_terminal_reserve_wh:100.0,maximum_voltage_deviation_pu:.4,
  maximum_frequency_deviation_hz:.2,maximum_rocof_hz_per_s:2.0,maximum_recovery_duration_ms:1000.0,
  maximum_control_deadline_misses:0,maximum_attack_detection_latency_ms:100.0,maximum_mitigation_latency_ms:300.0,
  maximum_false_positive_alerts_per_no_attack_run:0,maximum_positive_interaction_unserved_wh:20.0,
  maximum_positive_interaction_voltage_pu:.2,maximum_positive_interaction_frequency_hz:.1,
  maximum_positive_interaction_rocof_hz_per_s:1.0,maximum_positive_interaction_recovery_ms:500.0,
  maximum_positive_interaction_reserve_deficit_wh:100.0}}
 fn block()->CompoundResilienceBlock{CompoundResilienceBlock{block_id:"BLOCK-01".into(),
  expected_physical_disturbance:PhysicalDisturbance::WeakGrid,expected_cyber_manipulation:CyberManipulation::PccVoltageFdi,
  physical_profile_reference:"disturbance:weak-grid".into(),physical_profile_sha256:h('6'),
  cyber_profile_reference:"attack:pcc-voltage-fdi".into(),cyber_profile_sha256:h('7'),
  physical_event_offset_ms:0.0,cyber_event_offset_ms:25.0,acceptance:criteria(),runs:vec![
   run("B",FactorialArm::Baseline,None,None,0.0,.01,.001,.02,100.0,200.0),
   run("P",FactorialArm::PhysicalOnly,Some(PhysicalDisturbance::WeakGrid),None,10.0,.10,.05,.50,300.0,150.0),
   run("C",FactorialArm::CyberOnly,None,Some(CyberManipulation::PccVoltageFdi),5.0,.05,.02,.20,200.0,180.0),
   run("PC",FactorialArm::Compound,Some(PhysicalDisturbance::WeakGrid),Some(CyberManipulation::PccVoltageFdi),30.0,.25,.12,1.20,700.0,120.0)]}}
 fn eval(b:&CompoundResilienceBlock)->Result<CompoundResilienceReport,AuditError>{evaluate_compound_resilience(b,&gfm(),&model(),&graph())}

 #[test]fn complete_four_arm_block_reports_interaction(){let r=eval(&block()).unwrap();assert!(r.structural_complete&&r.model_validation_eligible&&r.all_four_arms_passed&&r.compound_resilience_eligible);assert!(!r.physical_evidence_eligible);let x=r.interaction_contrasts.iter().find(|x|x.metric==InteractionMetric::UnservedCriticalWh).unwrap();assert!((x.interaction_delta-15.0).abs()<EPS);}
 #[test]fn missing_or_duplicate_arm_rejected(){let mut b=block();b.runs.pop();assert!(eval(&b).is_err());b=block();b.runs[3].arm=FactorialArm::CyberOnly;assert!(eval(&b).is_err());}
 #[test]fn wrong_factor_assignment_rejected(){let mut b=block();b.runs[1].cyber_manipulation=Some(CyberManipulation::CommandReplay);assert!(eval(&b).is_err());}
 #[test]fn scenario_seed_or_configuration_mismatch_rejected(){let mut b=block();b.runs[2].scenario_seed_sha256=h('9');assert!(eval(&b).is_err());b=block();b.runs[3].protection_configuration_sha256=h('8');assert!(eval(&b).is_err());}
 #[test]fn unknown_gfm_test_or_cell_rejected(){let mut b=block();b.runs[0].gfm_test_id="bad".into();assert!(eval(&b).is_err());}
 #[test]fn model_must_be_held_out_and_match_firmware(){let mut m=model();m.validation_cases[0].held_out_validation_data=false;assert!(evaluate_compound_resilience(&block(),&gfm(),&m,&graph()).is_err());m=model();m.identity.firmware_sha256=h('9');assert!(evaluate_compound_resilience(&block(),&gfm(),&m,&graph()).is_err());}
 #[test]fn transition_and_poststate_references_must_exist(){let mut b=block();b.runs[0].transition_ids=vec!["unknown".into()];assert!(eval(&b).is_err());b=block();b.runs[0].outcome.post_state_id="unknown".into();assert!(eval(&b).is_err());}
 #[test]fn critical_service_and_terminal_reserve_are_hard_gates(){let mut b=block();b.runs[3].outcome.critical_service_unserved_wh=100.0;assert!(!eval(&b).unwrap().compound_resilience_eligible);b=block();b.runs[3].outcome.terminal_reserve_wh=50.0;assert!(!eval(&b).unwrap().compound_resilience_eligible);}
 #[test]fn undetected_unmitigated_or_late_attack_fails(){let mut b=block();b.runs[3].outcome.attack_detected=false;assert!(!eval(&b).unwrap().compound_resilience_eligible);b=block();b.runs[2].outcome.mitigation_succeeded=false;assert!(!eval(&b).unwrap().compound_resilience_eligible);b=block();b.runs[3].outcome.attack_detection_latency_ms=Some(101.0);assert!(!eval(&b).unwrap().compound_resilience_eligible);}
 #[test]fn false_positives_in_no_attack_arms_fail(){let mut b=block();b.runs[0].outcome.false_positive_alert_count=1;assert!(!eval(&b).unwrap().compound_resilience_eligible);}
 #[test]fn forbidden_transition_or_unknown_state_mutation_fails(){let mut b=block();b.runs[3].outcome.forbidden_transitions_observed=1;assert!(!eval(&b).unwrap().compound_resilience_eligible);b=block();b.runs[2].outcome.unknown_state_mutations=1;assert!(!eval(&b).unwrap().compound_resilience_eligible);}
 #[test]fn excess_interaction_harm_fails_even_when_each_arm_passes(){let mut b=block();b.runs[3].outcome.critical_service_unserved_wh=50.0;let r=eval(&b).unwrap();assert!(r.all_four_arms_passed);assert!(!r.compound_resilience_eligible);}
 #[test]fn expired_evidence_rejected(){let mut b=block();b.runs[0].evidence.status=ResilienceEvidenceStatus::Expired;assert!(eval(&b).is_err());}
 #[test]fn nan_outcome_rejected(){let mut b=block();b.runs[0].outcome.maximum_abs_voltage_deviation_pu=f64::NAN;assert!(eval(&b).is_err());}
 #[test]fn physical_claim_requires_measured_power_hil_in_all_arms(){let mut b=block();for r in &mut b.runs{r.evidence.class=ResilienceEvidenceClass::Measured;r.evidence.independent_witness_reference="independent:witness".into();}assert!(eval(&b).unwrap().physical_evidence_eligible);b.runs[0].execution_mode=ResilienceExecutionMode::ControllerHil;assert!(!eval(&b).unwrap().physical_evidence_eligible);}
 #[test]fn current_claim_ceiling_never_self_certifies(){assert!(CLAIM.contains("NOT_HIL_OR_FIELD_CLAIM"));assert_eq!(eval(&block()).unwrap().claim_ceiling(),CLAIM);}
 #[test]fn event_offsets_and_profile_digests_are_matched_across_factor_arms(){
  let mut b=block();b.runs[3].cyber_event_offset_ms=Some(30.0);assert!(eval(&b).is_err());
  b=block();b.runs[2].cyber_profile_sha256=h('8');assert!(eval(&b).is_err());
 }
 #[test]fn held_out_campaign_and_dataset_must_differ_from_calibration(){
  let mut m=model();m.validation_campaign_id=m.calibration_campaign_id.clone();
  assert!(evaluate_compound_resilience(&block(),&gfm(),&m,&graph()).is_err());
  m=model();m.validation_dataset_sha256=m.calibration_dataset_sha256.clone();m.validation_cases[0].dataset_sha256=m.validation_dataset_sha256.clone();
  assert!(evaluate_compound_resilience(&block(),&gfm(),&m,&graph()).is_err());
 }

}
