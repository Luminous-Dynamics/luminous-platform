use crate::capability_envelope::EvidenceStatus;
use crate::lifecycle_frontier::LifecycleFrontierReport;
use crate::lifecycle_service_cost::LifecycleEvidenceClass;
use crate::paired_cluster_bootstrap::{BootstrapInterval, PairedClusterBootstrapReport};
use crate::AuditError;

const CLAIM: &str = "PREREGISTERED_SUPERIORITY_SCREEN_ONLY_NOT_FIELD_SUPERIORITY_OR_EQUIVALENCE";
const EPS: f64 = 1.0e-9;
const ALGORITHM: &str = "PAIRED_CLUSTER_PERCENTILE_BOOTSTRAP_SPLITMIX64_V0_1";
const RESAMPLING_UNIT: &str = "DECLARED_INDEPENDENT_MISSION_SITE_EVENT_CLUSTER";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimaryMetric { CostPerDeliveredCriticalKwh, UnservedCriticalServiceWh, SystemUnavailableHours }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricDirection { LowerIsBetter, HigherIsBetter }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuperiorityStatus { Established, NoPreregisteredSuperiority, Inconclusive, Ineligible }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasurementBoundKind { ConservativeAbsoluteBound, ExpandedCoverageInterval }

#[derive(Debug, Clone, PartialEq)]
pub struct SuperiorityPreregistration {
    pub plan_id: String,
    pub comparator_set_id: String,
    pub reference_candidate_id: String,
    pub candidate_ids: Vec<String>,
    pub service_boundary_id: String,
    pub site_profile_sha256: String,
    pub mission_set_sha256: String,
    pub scenario_manifest_sha256: String,
    pub initial_terminal_obligation_sha256: String,
    pub primary_metric: PrimaryMetric,
    pub direction: MetricDirection,
    pub family_wise_alpha: f64,
    pub minimum_meaningful_improvement: f64,
    pub bootstrap_algorithm: String,
    pub resampling_unit: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PairedMeasurementUncertainty {
    pub candidate_id: String,
    pub metric: PrimaryMetric,
    pub absolute_bound: f64,
    pub bound_kind: MeasurementBoundKind,
    pub evidence_status: EvidenceStatus,
    pub evidence_class: LifecycleEvidenceClass,
    pub method_reference: String,
    pub evidence_reference: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SuperiorityGateInput {
    pub preregistration: SuperiorityPreregistration,
    pub frontier: LifecycleFrontierReport,
    pub bootstrap: PairedClusterBootstrapReport,
    pub measurement_uncertainty: Vec<PairedMeasurementUncertainty>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CombinedDecisionInterval {
    pub bootstrap_mean_delta: f64,
    pub bootstrap_lower_bound: f64,
    pub bootstrap_upper_bound: f64,
    pub measurement_uncertainty_absolute_bound: f64,
    pub combined_lower_bound: f64,
    pub combined_upper_bound: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CandidateSuperiorityDecision {
    pub candidate_id: String,
    pub status: SuperiorityStatus,
    pub combined_interval: Option<CombinedDecisionInterval>,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SuperiorityGateReport {
    pub plan_id: String,
    pub comparator_set_id: String,
    pub reference_candidate_id: String,
    pub primary_metric: PrimaryMetric,
    pub family_wise_alpha: f64,
    pub comparison_count: usize,
    pub adjusted_confidence_level: f64,
    pub decisions: Vec<CandidateSuperiorityDecision>,
    pub claim_ceiling: &'static str,
}

impl SuperiorityGateReport {
    pub const fn claim_ceiling(&self) -> &'static str { CLAIM }
}

fn nonblank(value: &str) -> bool { !value.trim().is_empty() }
fn valid_digest(value: &str) -> bool { value.len()==64 && value.bytes().all(|b|b.is_ascii_hexdigit()) }
fn near(a:f64,b:f64)->bool {(a-b).abs()<=EPS}
fn metric_direction(_:PrimaryMetric)->MetricDirection { MetricDirection::LowerIsBetter }

fn validate_plan(input:&SuperiorityGateInput)->Result<f64,AuditError>{
    let p=&input.preregistration;
    if !nonblank(&p.plan_id)||!nonblank(&p.comparator_set_id)||!nonblank(&p.reference_candidate_id)
        ||p.candidate_ids.is_empty()||!nonblank(&p.service_boundary_id)
        ||!valid_digest(&p.site_profile_sha256)||!valid_digest(&p.mission_set_sha256)
        ||!valid_digest(&p.scenario_manifest_sha256)||!valid_digest(&p.initial_terminal_obligation_sha256)
        ||!p.family_wise_alpha.is_finite()||p.family_wise_alpha<=0.0||p.family_wise_alpha>=0.5
        ||!p.minimum_meaningful_improvement.is_finite()||p.minimum_meaningful_improvement<=0.0
        ||p.direction!=metric_direction(p.primary_metric)||p.bootstrap_algorithm!=ALGORITHM
        ||p.resampling_unit!=RESAMPLING_UNIT {
        return Err(AuditError("preregistration is incomplete, unsupported or inconsistent".to_string()));
    }
    let mut ids=std::collections::BTreeSet::new();
    for id in &p.candidate_ids {
        if !nonblank(id)||id==&p.reference_candidate_id||!ids.insert(id) {
            return Err(AuditError("candidate IDs must be unique and exclude the reference".to_string()));
        }
    }
    let m=p.candidate_ids.len();
    let adjusted=1.0-p.family_wise_alpha/m as f64;
    if input.frontier.comparator_set_id!=p.comparator_set_id
        ||input.frontier.service_boundary_id!=p.service_boundary_id
        ||input.frontier.site_profile_sha256!=p.site_profile_sha256
        ||input.frontier.mission_set_sha256!=p.mission_set_sha256
        ||input.frontier.scenario_manifest_sha256!=p.scenario_manifest_sha256
        ||input.frontier.initial_terminal_obligation_sha256!=p.initial_terminal_obligation_sha256 {
        return Err(AuditError("frontier context differs from preregistered context".to_string()));
    }
    let b=&input.bootstrap;
    if b.reference_candidate_id!=p.reference_candidate_id||b.scenario_manifest_sha256!=p.scenario_manifest_sha256
        ||b.bootstrap_algorithm!=ALGORITHM||b.resampling_unit!=RESAMPLING_UNIT
        ||!near(b.confidence_level,adjusted)||b.comparisons.len()!=m {
        return Err(AuditError("bootstrap does not match reference, manifest, algorithm, comparison family or adjusted confidence".to_string()));
    }
    Ok(adjusted)
}

fn find_bootstrap<'a>(
    report:&'a PairedClusterBootstrapReport,
    candidate_id:&str,
    metric:PrimaryMetric,
)->Result<&'a BootstrapInterval,AuditError>{
    let comparison=report.comparisons.iter().find(|x|x.candidate_id==candidate_id)
        .ok_or_else(||AuditError(format!("bootstrap comparison missing for {candidate_id:?}")))?;
    if comparison.reference_candidate_id!=report.reference_candidate_id||comparison.paired_cluster_count<30 {
        return Err(AuditError("bootstrap comparison is not adequately paired".to_string()));
    }
    Ok(match metric {
        PrimaryMetric::CostPerDeliveredCriticalKwh => &comparison.cost_per_delivered_critical_kwh_delta,
        PrimaryMetric::UnservedCriticalServiceWh => &comparison.unserved_critical_service_wh_delta,
        PrimaryMetric::SystemUnavailableHours => &comparison.system_unavailable_hours_delta,
    })
}

fn validate_bound(bound:&PairedMeasurementUncertainty,metric:PrimaryMetric)->Result<(),AuditError>{
    if bound.metric!=metric||!bound.absolute_bound.is_finite()||bound.absolute_bound<0.0
        ||bound.bound_kind!=MeasurementBoundKind::ConservativeAbsoluteBound
        ||bound.evidence_status!=EvidenceStatus::Current
        ||!matches!(bound.evidence_class,LifecycleEvidenceClass::IndependentPhysicalMeasurement|LifecycleEvidenceClass::AnalyticalModel)
        ||!nonblank(&bound.method_reference)||!nonblank(&bound.evidence_reference) {
        return Err(AuditError(format!("measurement bound for {:?} is stale, assumption-only or lacks conservative provenance",bound.candidate_id)));
    }
    Ok(())
}

/// Applies a preregistered family-wise Bonferroni confidence level and a
/// conservative paired metrology-error bound to the existing bootstrap interval.
/// The module does not calculate bootstrap samples or calibration uncertainty.
pub fn evaluate_preregistered_superiority(input:&SuperiorityGateInput)->Result<SuperiorityGateReport,AuditError>{
    let adjusted=validate_plan(input)?;
    let p=&input.preregistration;
    let mut frontier_ids:Vec<&str>=input.frontier.candidates.iter().map(|x|x.candidate_id.as_str()).collect();
    frontier_ids.sort_unstable();
    let mut planned_ids:Vec<&str>=std::iter::once(p.reference_candidate_id.as_str()).chain(p.candidate_ids.iter().map(String::as_str)).collect();
    planned_ids.sort_unstable();
    if frontier_ids!=planned_ids {
        return Err(AuditError("frontier candidate set differs from preregistered family".to_string()));
    }
    let reference=input.frontier.candidates.iter().find(|x|x.candidate_id==p.reference_candidate_id)
        .ok_or_else(||AuditError("reference is missing from frontier".to_string()))?;
    if !reference.service_feasible||!reference.evidence_eligible||!reference.included_in_frontier {
        return Err(AuditError("reference is not service-feasible and evidence-eligible".to_string()));
    }
    if input.measurement_uncertainty.len()!=p.candidate_ids.len() {
        return Err(AuditError("one paired measurement bound is required per candidate".to_string()));
    }
    let mut bound_ids=std::collections::BTreeSet::new();
    for bound in &input.measurement_uncertainty {
        if !bound_ids.insert(bound.candidate_id.as_str()) { return Err(AuditError("duplicate measurement bound".to_string())); }
        validate_bound(bound,p.primary_metric)?;
    }
    let mut decisions=Vec::new();
    for candidate_id in &p.candidate_ids {
        let f=input.frontier.candidates.iter().find(|x|x.candidate_id==*candidate_id)
            .ok_or_else(||AuditError(format!("frontier candidate missing: {candidate_id}")))?;
        if !f.service_feasible||!f.evidence_eligible||!f.included_in_frontier {
            decisions.push(CandidateSuperiorityDecision{candidate_id:candidate_id.clone(),status:SuperiorityStatus::Ineligible,
                combined_interval:None,reasons:f.exclusion_reasons.clone()});
            continue;
        }
        let b=find_bootstrap(&input.bootstrap,candidate_id,p.primary_metric)?;
        if !b.paired_mean_delta.is_finite()||!b.lower_bound.is_finite()||!b.upper_bound.is_finite()
            ||b.lower_bound>b.paired_mean_delta||b.paired_mean_delta>b.upper_bound {
            return Err(AuditError("bootstrap interval is invalid".to_string()));
        }
        let metrology=input.measurement_uncertainty.iter().find(|x|x.candidate_id==*candidate_id)
            .ok_or_else(||AuditError(format!("measurement bound missing for {candidate_id}")))?;
        let lower=b.lower_bound-metrology.absolute_bound;
        let upper=b.upper_bound+metrology.absolute_bound;
        if !lower.is_finite()||!upper.is_finite(){return Err(AuditError("combined interval overflowed".to_string()));}
        let threshold=p.minimum_meaningful_improvement;
        let status=if upper < -threshold {SuperiorityStatus::Established}
            else if lower >= -threshold {SuperiorityStatus::NoPreregisteredSuperiority}
            else {SuperiorityStatus::Inconclusive};
        let reason=match status {
            SuperiorityStatus::Established=>"combined adjusted interval exceeds the preregistered minimum improvement",
            SuperiorityStatus::NoPreregisteredSuperiority=>"combined interval does not support the preregistered minimum improvement",
            SuperiorityStatus::Inconclusive=>"combined interval crosses the preregistered improvement threshold",
            SuperiorityStatus::Ineligible=>"candidate failed service/evidence gate",
        };
        decisions.push(CandidateSuperiorityDecision{candidate_id:candidate_id.clone(),status,
            combined_interval:Some(CombinedDecisionInterval{
                bootstrap_mean_delta:b.paired_mean_delta,bootstrap_lower_bound:b.lower_bound,bootstrap_upper_bound:b.upper_bound,
                measurement_uncertainty_absolute_bound:metrology.absolute_bound,combined_lower_bound:lower,combined_upper_bound:upper,
            }),reasons:vec![reason.to_string()]});
    }
    decisions.sort_by(|a,b|a.candidate_id.cmp(&b.candidate_id));
    Ok(SuperiorityGateReport{plan_id:p.plan_id.clone(),comparator_set_id:p.comparator_set_id.clone(),
        reference_candidate_id:p.reference_candidate_id.clone(),primary_metric:p.primary_metric,family_wise_alpha:p.family_wise_alpha,
        comparison_count:p.candidate_ids.len(),adjusted_confidence_level:adjusted,decisions,claim_ceiling:CLAIM})
}

#[cfg(test)]
mod tests{
 use super::*;
 use crate::lifecycle_frontier::LifecycleFrontierCandidateResult;
 use crate::paired_cluster_bootstrap::PairedBootstrapComparison;
 fn h(c:char)->String{std::iter::repeat(c).take(64).collect()}
 fn plan(ids:Vec<String>)->SuperiorityPreregistration{SuperiorityPreregistration{plan_id:"plan-1".into(),comparator_set_id:"set-1".into(),reference_candidate_id:"ref".into(),candidate_ids:ids,
 service_boundary_id:"poc".into(),site_profile_sha256:h('e'),mission_set_sha256:h('f'),scenario_manifest_sha256:h('1'),initial_terminal_obligation_sha256:h('2'),
 primary_metric:PrimaryMetric::CostPerDeliveredCriticalKwh,direction:MetricDirection::LowerIsBetter,family_wise_alpha:0.05,minimum_meaningful_improvement:5.0,
 bootstrap_algorithm:ALGORITHM.into(),resampling_unit:RESAMPLING_UNIT.into()}}
 fn frontier(ids:&[&str],eligible:&[bool])->LifecycleFrontierReport{LifecycleFrontierReport{
 comparator_set_id:"set-1".into(),service_boundary_id:"poc".into(),site_profile_sha256:h('e'),mission_set_sha256:h('f'),
 scenario_manifest_sha256:h('1'),initial_terminal_obligation_sha256:h('2'),currency_code:"USD".into(),minor_unit_scale:2,confidence_level:0.95,
 storage_retention_axis_included:false,metric_axes:vec![],candidates:ids.iter().zip(eligible).map(|(id,ok)|LifecycleFrontierCandidateResult{
 candidate_id:(*id).into(),service_feasible:*ok,evidence_eligible:*ok,included_in_frontier:*ok,central_pareto_member:*ok,robust_pareto_member:*ok,
 exclusion_reasons:if *ok{vec![]}else{vec!["service-failure".into()]},centrally_dominated_by:vec![],robustly_dominated_by:vec![]}).collect(),
 central_frontier_ids:vec![],robust_frontier_ids:vec![],claim_ceiling:"synthetic-test"}}
 fn comparison(id:&str,l:f64,m:f64,u:f64)->PairedBootstrapComparison{let x=BootstrapInterval{paired_mean_delta:m,lower_bound:l,upper_bound:u};
 PairedBootstrapComparison{reference_candidate_id:"ref".into(),candidate_id:id.into(),paired_cluster_count:40,failed_mission_count_reference:0,failed_mission_count_candidate:0,
 cost_per_delivered_critical_kwh_delta:x.clone(),unserved_critical_service_wh_delta:x.clone(),system_unavailable_hours_delta:x}}
 fn input(l:f64,m:f64,u:f64,bound:f64)->SuperiorityGateInput{SuperiorityGateInput{preregistration:plan(vec!["cand".into()]),
 frontier:frontier(&["ref","cand"],&[true,true]),bootstrap:PairedClusterBootstrapReport{reference_candidate_id:"ref".into(),scenario_manifest_sha256:h('1'),
 confidence_level:0.95,bootstrap_iterations:2000,bootstrap_algorithm:ALGORITHM,resampling_unit:RESAMPLING_UNIT,comparisons:vec![comparison("cand",l,m,u)],claim_ceiling:"test"},
 measurement_uncertainty:vec![PairedMeasurementUncertainty{candidate_id:"cand".into(),metric:PrimaryMetric::CostPerDeliveredCriticalKwh,absolute_bound:bound,
 bound_kind:MeasurementBoundKind::ConservativeAbsoluteBound,evidence_status:EvidenceStatus::Current,evidence_class:LifecycleEvidenceClass::IndependentPhysicalMeasurement,
 method_reference:"paired-meter-method".into(),evidence_reference:"metrology-receipt".into()}]}}
 #[test]fn establishes_when_interval_clears_practical_margin(){let x=input(-10.0,-9.0,-8.0,1.0);let r=evaluate_preregistered_superiority(&x).unwrap();
 assert_eq!(r.decisions[0].status,SuperiorityStatus::Established);assert!((r.adjusted_confidence_level-0.95).abs()<EPS);}
 #[test]fn tiny_significant_change_is_not_practical_superiority(){let mut x=input(-0.10,-0.08,-0.05,0.0);x.preregistration.minimum_meaningful_improvement=1.0;
 assert_eq!(evaluate_preregistered_superiority(&x).unwrap().decisions[0].status,SuperiorityStatus::NoPreregisteredSuperiority);}
 #[test]fn metrology_bound_can_make_bootstrap_win_inconclusive(){let x=input(-10.0,-9.0,-8.0,4.0);let r=evaluate_preregistered_superiority(&x).unwrap();
 assert_eq!(r.decisions[0].status,SuperiorityStatus::Inconclusive);assert_eq!(r.decisions[0].combined_interval.as_ref().unwrap().combined_upper_bound,-4.0);}
 #[test]fn bonferroni_confidence_must_match_candidate_family(){let mut x=input(-10.0,-9.0,-8.0,1.0);x.preregistration.candidate_ids=vec!["cand".into(),"cand2".into()];
 x.frontier=frontier(&["ref","cand","cand2"],&[true,true,true]);x.bootstrap.comparisons.push(comparison("cand2",-10.0,-9.0,-8.0));
 assert!(evaluate_preregistered_superiority(&x).is_err());x.bootstrap.confidence_level=0.975;
 x.measurement_uncertainty.push(PairedMeasurementUncertainty{candidate_id:"cand2".into(),metric:PrimaryMetric::CostPerDeliveredCriticalKwh,absolute_bound:1.0,
 bound_kind:MeasurementBoundKind::ConservativeAbsoluteBound,evidence_status:EvidenceStatus::Current,evidence_class:LifecycleEvidenceClass::AnalyticalModel,
 method_reference:"model-method".into(),evidence_reference:"receipt-2".into()});assert!(evaluate_preregistered_superiority(&x).is_ok());}
 #[test]fn failed_service_candidate_is_ineligible_even_with_strong_interval(){let mut x=input(-20.0,-15.0,-10.0,0.1);x.frontier=frontier(&["ref","cand"],&[true,false]);
 assert_eq!(evaluate_preregistered_superiority(&x).unwrap().decisions[0].status,SuperiorityStatus::Ineligible);}
 #[test]fn stale_or_assumption_only_metrology_is_rejected(){let mut x=input(-10.0,-9.0,-8.0,1.0);x.measurement_uncertainty[0].evidence_status=EvidenceStatus::Expired;
 assert!(evaluate_preregistered_superiority(&x).is_err());x=input(-10.0,-9.0,-8.0,1.0);x.measurement_uncertainty[0].evidence_class=LifecycleEvidenceClass::Assumption;
 assert!(evaluate_preregistered_superiority(&x).is_err());}
 #[test]fn wrong_metric_direction_and_probabilistic_bound_are_rejected(){let mut x=input(-10.0,-9.0,-8.0,1.0);x.preregistration.direction=MetricDirection::HigherIsBetter;
 assert!(evaluate_preregistered_superiority(&x).is_err());x=input(-10.0,-9.0,-8.0,1.0);x.measurement_uncertainty[0].bound_kind=MeasurementBoundKind::ExpandedCoverageInterval;
 assert!(evaluate_preregistered_superiority(&x).is_err());}
 #[test]fn reference_and_manifest_mismatch_are_rejected(){let mut x=input(-10.0,-9.0,-8.0,1.0);x.bootstrap.reference_candidate_id="other".into();
 assert!(evaluate_preregistered_superiority(&x).is_err());x=input(-10.0,-9.0,-8.0,1.0);x.bootstrap.scenario_manifest_sha256=h('9');
 assert!(evaluate_preregistered_superiority(&x).is_err());}
 #[test]fn candidate_order_does_not_change_sorted_decisions(){let mut x=input(-10.0,-9.0,-8.0,1.0);x.preregistration.candidate_ids=vec!["cand".into(),"cand2".into()];
 x.frontier=frontier(&["ref","cand","cand2"],&[true,true,true]);x.bootstrap.comparisons.push(comparison("cand2",-10.0,-9.0,-8.0));x.bootstrap.confidence_level=0.975;
 x.measurement_uncertainty.push(PairedMeasurementUncertainty{candidate_id:"cand2".into(),metric:PrimaryMetric::CostPerDeliveredCriticalKwh,absolute_bound:1.0,
 bound_kind:MeasurementBoundKind::ConservativeAbsoluteBound,evidence_status:EvidenceStatus::Current,evidence_class:LifecycleEvidenceClass::AnalyticalModel,
 method_reference:"method".into(),evidence_reference:"receipt".into()});
 let a=evaluate_preregistered_superiority(&x).unwrap();x.preregistration.candidate_ids.reverse();let b=evaluate_preregistered_superiority(&x).unwrap();assert_eq!(a.decisions,b.decisions);}
 #[test]fn missing_duplicate_or_wrong_metric_bound_is_rejected(){let mut x=input(-10.0,-9.0,-8.0,1.0);x.measurement_uncertainty.clear();assert!(evaluate_preregistered_superiority(&x).is_err());
 x=input(-10.0,-9.0,-8.0,1.0);x.measurement_uncertainty.push(x.measurement_uncertainty[0].clone());assert!(evaluate_preregistered_superiority(&x).is_err());
 x=input(-10.0,-9.0,-8.0,1.0);x.measurement_uncertainty[0].metric=PrimaryMetric::UnservedCriticalServiceWh;assert!(evaluate_preregistered_superiority(&x).is_err());}
 #[test]fn selected_unserved_metric_uses_its_own_bootstrap_interval(){
  let mut x=input(100.0,110.0,120.0,0.1);
  x.preregistration.primary_metric=PrimaryMetric::UnservedCriticalServiceWh;
  x.measurement_uncertainty[0].metric=PrimaryMetric::UnservedCriticalServiceWh;
  x.bootstrap.comparisons[0].unserved_critical_service_wh_delta=BootstrapInterval{paired_mean_delta:-4.0,lower_bound:-5.0,upper_bound:-3.0};
  x.preregistration.minimum_meaningful_improvement=1.0;
  assert_eq!(evaluate_preregistered_superiority(&x).unwrap().decisions[0].status,SuperiorityStatus::Established);
 }
 #[test]fn selected_downtime_metric_uses_its_own_bootstrap_interval(){
  let mut x=input(100.0,110.0,120.0,0.1);
  x.preregistration.primary_metric=PrimaryMetric::SystemUnavailableHours;
  x.measurement_uncertainty[0].metric=PrimaryMetric::SystemUnavailableHours;
  x.bootstrap.comparisons[0].system_unavailable_hours_delta=BootstrapInterval{paired_mean_delta:-3.0,lower_bound:-4.0,upper_bound:-2.0};
  x.preregistration.minimum_meaningful_improvement=1.0;
  assert_eq!(evaluate_preregistered_superiority(&x).unwrap().decisions[0].status,SuperiorityStatus::Established);
 }

}
