use crate::capability_envelope::EvidenceStatus;
use crate::lifecycle_service_cost::LifecycleScenarioReport;
use crate::AuditError;

const CLAIM: &str =
    "UNCERTAINTY_QUALIFIED_LIFECYCLE_PARETO_SCREEN_ONLY_NOT_LIFETIME_FORECAST_OR_SOTA_CLAIM";
const EPS: f64 = 1.0e-9;
const MAX_CANDIDATES: usize = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UncertaintyBasis {
    Metrology,
    Statistical,
    ModelParameter,
    Combined,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UncertaintyInterval {
    pub point: f64,
    pub lower: f64,
    pub upper: f64,
    pub confidence_level: f64,
    pub basis: UncertaintyBasis,
    pub evidence_status: EvidenceStatus,
    pub evidence_class: crate::lifecycle_service_cost::LifecycleEvidenceClass,
    pub evidence_reference: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LifecycleFrontierCandidate {
    pub candidate_id: String,
    pub candidate_configuration_sha256: String,
    pub service_boundary_id: String,
    pub site_profile_sha256: String,
    pub mission_set_sha256: String,
    pub scenario_manifest_sha256: String,
    pub initial_terminal_obligation_sha256: String,
    pub storage_in_scope: bool,
    pub accounting: LifecycleScenarioReport,
    pub cost_per_delivered_critical_kwh: UncertaintyInterval,
    pub unserved_critical_service_wh: UncertaintyInterval,
    pub system_unavailable_hours: UncertaintyInterval,
    /// Minimum qualified component capacity-retention fraction for this candidate.
    /// Required when storage is in scope; omitted for a non-storage comparator set.
    pub minimum_storage_capacity_retention: Option<UncertaintyInterval>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LifecycleFrontierInput {
    pub comparator_set_id: String,
    pub currency_code: String,
    pub minor_unit_scale: u8,
    pub confidence_level: f64,
    pub candidates: Vec<LifecycleFrontierCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleFrontierCandidateResult {
    pub candidate_id: String,
    pub service_feasible: bool,
    pub evidence_eligible: bool,
    pub included_in_frontier: bool,
    pub central_pareto_member: bool,
    pub robust_pareto_member: bool,
    pub exclusion_reasons: Vec<String>,
    pub centrally_dominated_by: Vec<String>,
    pub robustly_dominated_by: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LifecycleFrontierReport {
    pub comparator_set_id: String,
    pub service_boundary_id: String,
    pub site_profile_sha256: String,
    pub mission_set_sha256: String,
    pub scenario_manifest_sha256: String,
    pub initial_terminal_obligation_sha256: String,
    pub currency_code: String,
    pub minor_unit_scale: u8,
    pub confidence_level: f64,
    pub storage_retention_axis_included: bool,
    pub metric_axes: Vec<String>,
    pub candidates: Vec<LifecycleFrontierCandidateResult>,
    pub central_frontier_ids: Vec<String>,
    pub robust_frontier_ids: Vec<String>,
    pub claim_ceiling: &'static str,
}

impl LifecycleFrontierReport {
    pub const fn claim_ceiling(&self) -> &'static str { CLAIM }
}

fn nonblank(x: &str) -> bool { !x.trim().is_empty() }
fn valid_digest(x: &str) -> bool { x.len() == 64 && x.bytes().all(|b| b.is_ascii_hexdigit()) }
fn near(a: f64, b: f64) -> bool { (a - b).abs() <= EPS }
fn finite_nonnegative(x: f64) -> bool { x.is_finite() && x >= 0.0 }

fn validate_interval(
    name: &str,
    interval: &UncertaintyInterval,
    confidence_level: f64,
    max_value: Option<f64>,
) -> Result<(), AuditError> {
    if !finite_nonnegative(interval.point)
        || !finite_nonnegative(interval.lower)
        || !finite_nonnegative(interval.upper)
        || interval.lower > interval.point
        || interval.point > interval.upper
        || !interval.confidence_level.is_finite()
        || !near(interval.confidence_level, confidence_level)
        || interval.evidence_status != EvidenceStatus::Current
        || matches!(interval.evidence_class, crate::lifecycle_service_cost::LifecycleEvidenceClass::SyntheticFixture | crate::lifecycle_service_cost::LifecycleEvidenceClass::Assumption)
        || !nonblank(&interval.evidence_reference)
        || max_value.is_some_and(|max| interval.upper > max + EPS)
    {
        return Err(AuditError(format!(
            "{name} uncertainty interval is invalid, stale, unsupported or uses a different confidence level"
        )));
    }
    Ok(())
}

fn validate_matched_context(input: &LifecycleFrontierInput) -> Result<(), AuditError> {
    if !nonblank(&input.comparator_set_id)
        || input.currency_code.len() != 3
        || !input.currency_code.bytes().all(|b| b.is_ascii_uppercase())
        || input.minor_unit_scale > 3
        || !input.confidence_level.is_finite()
        || input.confidence_level <= 0.5
        || input.confidence_level >= 1.0
        || input.candidates.len() < 2
        || input.candidates.len() > MAX_CANDIDATES
    {
        return Err(AuditError("frontier metadata, confidence or candidate count is invalid".to_string()));
    }
    let first = &input.candidates[0];
    if !nonblank(&first.service_boundary_id)
        || !valid_digest(&first.site_profile_sha256)
        || !valid_digest(&first.mission_set_sha256)
        || !valid_digest(&first.scenario_manifest_sha256)
        || !valid_digest(&first.initial_terminal_obligation_sha256)
    {
        return Err(AuditError("frontier comparison context is incomplete or has invalid digests".to_string()));
    }
    let mut ids = Vec::new();
    for c in &input.candidates {
        if !nonblank(&c.candidate_id) || ids.iter().any(|id: &String| id == &c.candidate_id) {
            return Err(AuditError("candidate IDs must be non-empty and unique".to_string()));
        }
        ids.push(c.candidate_id.clone());
        if !valid_digest(&c.candidate_configuration_sha256)
            || c.accounting.comparator_set_id != input.comparator_set_id
            || c.accounting.currency_code != input.currency_code
            || c.accounting.minor_unit_scale != input.minor_unit_scale
            || c.service_boundary_id != first.service_boundary_id
            || c.site_profile_sha256 != first.site_profile_sha256
            || c.mission_set_sha256 != first.mission_set_sha256
            || c.scenario_manifest_sha256 != first.scenario_manifest_sha256
            || c.initial_terminal_obligation_sha256 != first.initial_terminal_obligation_sha256
            || c.storage_in_scope != first.storage_in_scope
        {
            return Err(AuditError(format!(
                "candidate {:?} is not matched on currency, service boundary, site, mission set, scenario manifest, initial/terminal obligations or storage scope",
                c.candidate_id
            )));
        }
        if c.accounting.scenario_id.trim().is_empty() {
            return Err(AuditError(format!("candidate {:?} has no lifecycle accounting report", c.candidate_id)));
        }
    }
    Ok(())
}

fn candidate_exclusions(c: &LifecycleFrontierCandidate) -> (bool, bool, Vec<String>) {
    let r = &c.accounting;
    let mut reasons = Vec::new();
    let service_feasible = r.mission_count > 0
        && r.completed_mission_count == r.mission_count
        && r.critical_service_pass_rate_percent >= 100.0 - EPS
        && r.configuration_eligible
        && r.lifecycle_accounting_eligible;
    if r.mission_count == 0 { reasons.push("no-mission-records".to_string()); }
    if r.completed_mission_count != r.mission_count
        || r.critical_service_pass_rate_percent < 100.0 - EPS {
        reasons.push("non-compensatory-critical-service-gate-failed".to_string());
    }
    if !r.configuration_eligible { reasons.push("configuration-ineligible".to_string()); }
    if !r.lifecycle_accounting_eligible { reasons.push("lifecycle-accounting-ineligible".to_string()); }

    let mut evidence_eligible = r.physical_lifecycle_claim_eligible
        && r.cost_evidence_current && r.cost_evidence_actual
        && r.mission_evidence_current && r.mission_evidence_physical
        && r.repair_evidence_current && r.repair_evidence_actual;
    if c.storage_in_scope {
        evidence_eligible &= r.storage_health_evidence_complete
            && !r.storage_health_trajectories.is_empty()
            && c.minimum_storage_capacity_retention.is_some();
    }
    if !r.physical_lifecycle_claim_eligible { reasons.push("physical-claim-evidence-ineligible".to_string()); }
    if !r.cost_evidence_current || !r.cost_evidence_actual { reasons.push("cost-evidence-incomplete-or-not-actual".to_string()); }
    if !r.mission_evidence_current || !r.mission_evidence_physical { reasons.push("mission-evidence-incomplete-or-not-physical".to_string()); }
    if !r.repair_evidence_current || !r.repair_evidence_actual { reasons.push("repair-evidence-incomplete-or-not-actual".to_string()); }
    if c.storage_in_scope && (!r.storage_health_evidence_complete
        || r.storage_health_trajectories.is_empty() || c.minimum_storage_capacity_retention.is_none()) {
        reasons.push("storage-health-evidence-incomplete".to_string());
    }
    (service_feasible, evidence_eligible, reasons)
}

fn point_cost(c: &LifecycleFrontierCandidate) -> Result<f64, AuditError> {
    c.accounting.cost_per_delivered_critical_kwh_minor_units
        .ok_or_else(|| AuditError(format!("candidate {:?} has no finite delivered-critical-service cost", c.candidate_id)))
}

fn validate_candidate_intervals(
    c: &LifecycleFrontierCandidate,
    confidence: f64,
    storage_in_scope: bool,
) -> Result<(), AuditError> {
    let cost = point_cost(c)?;
    if !cost.is_finite() || cost < 0.0
        || !near(c.cost_per_delivered_critical_kwh.point, cost)
        || !near(c.unserved_critical_service_wh.point, c.accounting.total_unserved_critical_service_wh)
        || !near(c.system_unavailable_hours.point, c.accounting.system_unavailable_hours)
    {
        return Err(AuditError(format!(
            "candidate {:?} uncertainty point estimates do not bind to the lifecycle accountant output",
            c.candidate_id
        )));
    }
    validate_interval("cost per delivered critical kWh", &c.cost_per_delivered_critical_kwh, confidence, None)?;
    validate_interval("unserved critical service", &c.unserved_critical_service_wh, confidence, None)?;
    validate_interval("system unavailable hours", &c.system_unavailable_hours, confidence, None)?;
    if storage_in_scope {
        let interval = c.minimum_storage_capacity_retention.as_ref()
            .ok_or_else(|| AuditError(format!("candidate {:?} is missing storage-retention uncertainty", c.candidate_id)))?;
        let observed_min = c.accounting.storage_health_trajectories.iter()
            .map(|x| x.usable_capacity_retention_fraction)
            .reduce(f64::min)
            .ok_or_else(|| AuditError(format!("candidate {:?} has no storage-health trajectories", c.candidate_id)))?;
        if !near(interval.point, observed_min) {
            return Err(AuditError(format!("candidate {:?} retention point does not bind to its lowest observed component retention", c.candidate_id)));
        }
        validate_interval("minimum storage retention", interval, confidence, Some(1.0))?;
    } else if c.minimum_storage_capacity_retention.is_some() {
        return Err(AuditError(format!("candidate {:?} supplies storage retention while storage is out of scope", c.candidate_id)));
    }
    Ok(())
}

fn metric_points(c: &LifecycleFrontierCandidate, storage: bool) -> Vec<(f64, bool)> {
    let mut m = vec![
        (c.cost_per_delivered_critical_kwh.point, false),
        (c.unserved_critical_service_wh.point, false),
        (c.system_unavailable_hours.point, false),
        (c.accounting.replacement_count as f64, false),
    ];
    if storage {
        m.push((c.minimum_storage_capacity_retention.as_ref().map(|x| x.point).unwrap_or(0.0), true));
    }
    m
}

fn central_dominates(a: &LifecycleFrontierCandidate, b: &LifecycleFrontierCandidate, storage: bool) -> bool {
    let av = metric_points(a, storage);
    let bv = metric_points(b, storage);
    let mut strictly_better = false;
    for ((x, maximize), (y, _)) in av.into_iter().zip(bv) {
        if maximize {
            if x < y - EPS { return false; }
            if x > y + EPS { strictly_better = true; }
        } else {
            if x > y + EPS { return false; }
            if x < y - EPS { strictly_better = true; }
        }
    }
    strictly_better
}

fn robust_dominates(a: &LifecycleFrontierCandidate, b: &LifecycleFrontierCandidate, storage: bool) -> bool {
    let mut all_no_worse = true;
    let mut one_strict = false;
    let pairs = [
        (&a.cost_per_delivered_critical_kwh, &b.cost_per_delivered_critical_kwh, false),
        (&a.unserved_critical_service_wh, &b.unserved_critical_service_wh, false),
        (&a.system_unavailable_hours, &b.system_unavailable_hours, false),
    ];
    for (x, y, maximize) in pairs {
        if maximize {
            if x.lower + EPS < y.upper { all_no_worse = false; }
            if x.lower > y.upper + EPS { one_strict = true; }
        } else {
            if x.upper > y.lower + EPS { all_no_worse = false; }
            if x.upper + EPS < y.lower { one_strict = true; }
        }
    }
    if a.accounting.replacement_count > b.accounting.replacement_count { all_no_worse = false; }
    if a.accounting.replacement_count < b.accounting.replacement_count { one_strict = true; }
    if storage {
        let x = a.minimum_storage_capacity_retention.as_ref();
        let y = b.minimum_storage_capacity_retention.as_ref();
        if let (Some(x), Some(y)) = (x, y) {
            if x.lower + EPS < y.upper { all_no_worse = false; }
            if x.lower > y.upper + EPS { one_strict = true; }
        } else {
            all_no_worse = false;
        }
    }
    all_no_worse && one_strict
}

/// Builds central and uncertainty-robust Pareto frontiers over the existing
/// lifecycle accountant. Critical service, reserve/terminal obligations and
/// evidence eligibility are hard gates, never weighted score dimensions.
pub fn evaluate_lifecycle_frontier(input: &LifecycleFrontierInput) -> Result<LifecycleFrontierReport, AuditError> {
    validate_matched_context(input)?;
    let storage = input.candidates[0].storage_in_scope;
    for c in &input.candidates {
        validate_candidate_intervals(c, input.confidence_level, storage)?;
    }

    let eligibility: Vec<(bool, bool, Vec<String>)> = input.candidates.iter().map(candidate_exclusions).collect();
    let included: Vec<usize> = eligibility.iter().enumerate()
        .filter(|(_, (service, evidence, _))| *service && *evidence)
        .map(|(i, _)| i).collect();

    let mut results = Vec::with_capacity(input.candidates.len());
    for (i, c) in input.candidates.iter().enumerate() {
        let (service_feasible, evidence_eligible, exclusion_reasons) = &eligibility[i];
        let mut central_dominated_by = Vec::new();
        let mut robustly_dominated_by = Vec::new();
        if *service_feasible && *evidence_eligible {
            for j in &included {
                if i == *j { continue; }
                let other = &input.candidates[*j];
                if central_dominates(other, c, storage) { central_dominated_by.push(other.candidate_id.clone()); }
                if robust_dominates(other, c, storage) { robustly_dominated_by.push(other.candidate_id.clone()); }
            }
        }
        central_dominated_by.sort();
        robustly_dominated_by.sort();
        results.push(LifecycleFrontierCandidateResult {
            candidate_id: c.candidate_id.clone(),
            service_feasible: *service_feasible,
            evidence_eligible: *evidence_eligible,
            included_in_frontier: *service_feasible && *evidence_eligible,
            central_pareto_member: *service_feasible && *evidence_eligible && central_dominated_by.is_empty(),
            robust_pareto_member: *service_feasible && *evidence_eligible && robustly_dominated_by.is_empty(),
            exclusion_reasons: exclusion_reasons.clone(),
            centrally_dominated_by,
            robustly_dominated_by,
        });
    }
    results.sort_by(|a, b| a.candidate_id.cmp(&b.candidate_id));
    let mut central_frontier_ids: Vec<String> = results.iter().filter(|x| x.central_pareto_member)
        .map(|x| x.candidate_id.clone()).collect();
    let mut robust_frontier_ids: Vec<String> = results.iter().filter(|x| x.robust_pareto_member)
        .map(|x| x.candidate_id.clone()).collect();
    central_frontier_ids.sort();
    robust_frontier_ids.sort();

    let mut metric_axes = vec![
        "lifecycle_cost_per_delivered_critical_kwh_minor_units".to_string(),
        "unserved_critical_service_wh".to_string(),
        "system_unavailable_hours".to_string(),
        "replacement_count".to_string(),
    ];
    if storage { metric_axes.push("minimum_observed_storage_capacity_retention_fraction".to_string()); }
    let matched_context = &input.candidates[0];
    Ok(LifecycleFrontierReport {
        comparator_set_id: input.comparator_set_id.clone(),
        service_boundary_id: matched_context.service_boundary_id.clone(),
        site_profile_sha256: matched_context.site_profile_sha256.clone(),
        mission_set_sha256: matched_context.mission_set_sha256.clone(),
        scenario_manifest_sha256: matched_context.scenario_manifest_sha256.clone(),
        initial_terminal_obligation_sha256: matched_context.initial_terminal_obligation_sha256.clone(),
        currency_code: input.currency_code.clone(),
        minor_unit_scale: input.minor_unit_scale,
        confidence_level: input.confidence_level,
        storage_retention_axis_included: storage,
        metric_axes,
        candidates: results,
        central_frontier_ids,
        robust_frontier_ids,
        claim_ceiling: CLAIM,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lifecycle_service_cost::{LifecycleCostCategoryTotal, StorageHealthTrajectory};

    fn h(c: char) -> String { std::iter::repeat(c).take(64).collect() }

    fn trajectory(retention: f64) -> StorageHealthTrajectory {
        StorageHealthTrajectory {
            component_id:"battery-1".into(), qualified_configuration_sha256:h('a'),
            first_observation_epoch_s:1,last_observation_epoch_s:2,
            first_usable_capacity_lower_bound_wh:900.0,last_usable_capacity_lower_bound_wh:900.0*retention,
            usable_capacity_retention_fraction:retention,
            first_discharge_power_lower_bound_w:1000.0,last_discharge_power_lower_bound_w:900.0,
            discharge_power_retention_fraction:0.9,energy_throughput_delta_wh:100.0,
            calendar_age_delta_hours:100.0,equivalent_full_cycle_delta:1.0,
            first_state_of_health_fraction:0.99,last_state_of_health_fraction:0.95,
            first_temperature_c:25.0,last_temperature_c:30.0,
            first_timebase_reference:"timebase:first".into(),last_timebase_reference:"timebase:last".into(),
            first_evidence_reference:"measurement:first".into(),last_evidence_reference:"measurement:last".into(),
        }
    }

    fn accounting(id: &str, cost: f64, unserved: f64, downtime: f64, replacements: usize, ret: f64) -> LifecycleScenarioReport {
        LifecycleScenarioReport {
            scenario_id:id.into(), comparator_set_id:"set-1".into(), currency_code:"USD".into(),minor_unit_scale:2,
            total_cost_minor_units:(cost*100.0) as u128,
            cost_by_category:vec![LifecycleCostCategoryTotal{category:crate::lifecycle_service_cost::LifecycleCostCategory::InitialCapital,amount_minor_units:(cost*100.0) as u128}],
            total_required_critical_service_wh:10000.0,total_delivered_critical_service_wh:10000.0-unserved,
            total_unserved_critical_service_wh:unserved,mission_count:100,completed_mission_count:if unserved<=EPS{100}else{98},
            critical_service_pass_rate_percent:if unserved<=EPS{100.0}else{98.0},
            cost_per_delivered_critical_kwh_minor_units:Some(cost),
            cost_per_completed_mission_minor_units:Some(cost/100.0),
            system_unavailable_hours:downtime,availability_percent:100.0-(downtime/100.0),
            repair_labor_hours:1.0,requalification_hours:1.0,replacement_count:replacements,
            cost_evidence_current:true,cost_evidence_actual:true,mission_evidence_current:true,mission_evidence_physical:true,
            repair_evidence_current:true,repair_evidence_actual:true,storage_health_evidence_complete:true,
            storage_health_trajectories:vec![trajectory(ret)],configuration_eligible:true,lifecycle_accounting_eligible:true,
            physical_lifecycle_claim_eligible:true,
        }
    }

    fn interval(point:f64,lower:f64,upper:f64)->UncertaintyInterval{
        UncertaintyInterval{point,lower,upper,confidence_level:0.95,basis:UncertaintyBasis::Combined,
            evidence_status:EvidenceStatus::Current,evidence_class:crate::lifecycle_service_cost::LifecycleEvidenceClass::AnalyticalModel,evidence_reference:"uncertainty:report-1".into()}
    }

    fn candidate(id:&str,cost:f64,cost_l:f64,cost_u:f64,down:f64,down_l:f64,down_u:f64,repl:usize,ret:f64,ret_l:f64,ret_u:f64,unserved:f64)->LifecycleFrontierCandidate{
        LifecycleFrontierCandidate{
            candidate_id:id.into(),candidate_configuration_sha256:h(if id=="A"{'b'}else if id=="B"{'c'}else{'d'}),
            service_boundary_id:"customer-poc".into(),site_profile_sha256:h('e'),mission_set_sha256:h('f'),
            scenario_manifest_sha256:h('1'),initial_terminal_obligation_sha256:h('2'),storage_in_scope:true,
            accounting:accounting(id,cost,unserved,down,repl,ret),
            cost_per_delivered_critical_kwh:interval(cost,cost_l,cost_u),
            unserved_critical_service_wh:interval(unserved,unserved,unserved),
            system_unavailable_hours:interval(down,down_l,down_u),
            minimum_storage_capacity_retention:Some(interval(ret,ret_l,ret_u)),
        }
    }

    fn input()->LifecycleFrontierInput{
        LifecycleFrontierInput{
            comparator_set_id:"set-1".into(),currency_code:"USD".into(),minor_unit_scale:2,confidence_level:0.95,
            candidates:vec![
                candidate("A",1000.0,980.0,1020.0,2.0,1.8,2.2,1,0.90,0.88,0.92,0.0),
                candidate("B",1100.0,1070.0,1130.0,1.0,0.8,1.2,0,0.93,0.91,0.95,0.0),
            ],
        }
    }

    #[test] fn matched_candidates_produce_central_and_robust_frontiers(){
        let r=evaluate_lifecycle_frontier(&input()).unwrap();
        assert_eq!(r.central_frontier_ids,vec!["A".to_string(),"B".to_string()]);
        assert_eq!(r.robust_frontier_ids,vec!["A".to_string(),"B".to_string()]);
        assert!(r.storage_retention_axis_included);
    }

    #[test] fn failed_critical_service_is_infeasible_even_when_cheaper(){
        let mut x=input();
        let mut failed=candidate("CHEAP-FAIL",500.0,490.0,510.0,0.5,0.4,0.6,0,0.95,0.94,0.96,100.0);
        failed.accounting.lifecycle_accounting_eligible=true;
        x.candidates.push(failed);
        let r=evaluate_lifecycle_frontier(&x).unwrap();
        let f=r.candidates.iter().find(|v|v.candidate_id=="CHEAP-FAIL").unwrap();
        assert!(!f.service_feasible);
        assert!(!f.included_in_frontier);
        assert!(f.exclusion_reasons.contains(&"non-compensatory-critical-service-gate-failed".to_string()));
    }

    #[test] fn stale_or_assumed_evidence_is_excluded_from_claim_frontier(){
        let mut x=input();x.candidates[0].accounting.physical_lifecycle_claim_eligible=false;
        let r=evaluate_lifecycle_frontier(&x).unwrap();
        let a=r.candidates.iter().find(|v|v.candidate_id=="A").unwrap();
        assert!(a.service_feasible);assert!(!a.evidence_eligible);assert!(!a.included_in_frontier);
    }

    #[test] fn robust_dominance_requires_non_overlapping_uncertainty(){
        let mut x=input();
        x.candidates.push(candidate("C",1300.0,1250.0,1350.0,3.0,2.8,3.2,2,0.85,0.82,0.87,0.0));
        let r=evaluate_lifecycle_frontier(&x).unwrap();
        let c=r.candidates.iter().find(|v|v.candidate_id=="C").unwrap();
        assert!(c.centrally_dominated_by.contains(&"A".to_string()));
        assert!(c.robustly_dominated_by.contains(&"A".to_string()));
        assert!(!c.central_pareto_member&&!c.robust_pareto_member);
    }

    #[test] fn overlapping_intervals_can_preserve_both_robust_frontier_members(){
        let mut x=input();
        x.candidates[1].cost_per_delivered_critical_kwh=interval(1010.0,1000.0,1025.0);
        x.candidates[1].accounting.cost_per_delivered_critical_kwh_minor_units=Some(1010.0);
        let r=evaluate_lifecycle_frontier(&x).unwrap();
        assert_eq!(r.robust_frontier_ids.len(),2);
    }

    #[test] fn ties_do_not_create_arbitrary_dominance(){
        let mut x=input();let mut b=x.candidates[0].clone();b.candidate_id="A2".into();
        b.candidate_configuration_sha256=h('7');x.candidates.push(b);
        let r=evaluate_lifecycle_frontier(&x).unwrap();
        assert!(r.central_frontier_ids.contains(&"A".to_string()));
        assert!(r.central_frontier_ids.contains(&"A2".to_string()));
    }

    #[test] fn candidate_context_mismatch_is_rejected(){
        let mut x=input();x.candidates[1].scenario_manifest_sha256=h('9');
        assert!(evaluate_lifecycle_frontier(&x).is_err());
    }

    #[test] fn initial_terminal_obligation_mismatch_is_rejected(){
        let mut x=input();x.candidates[1].initial_terminal_obligation_sha256=h('8');
        assert!(evaluate_lifecycle_frontier(&x).is_err());
    }

    #[test] fn metric_interval_point_must_match_accounting_report(){
        let mut x=input();x.candidates[0].cost_per_delivered_critical_kwh.point=900.0;
        assert!(evaluate_lifecycle_frontier(&x).is_err());
    }

    #[test] fn uncertainty_intervals_must_share_declared_confidence(){
        let mut x=input();x.candidates[0].cost_per_delivered_critical_kwh.confidence_level=0.90;
        assert!(evaluate_lifecycle_frontier(&x).is_err());
    }

    #[test] fn invalid_or_stale_uncertainty_is_rejected(){
        let mut x=input();x.candidates[0].system_unavailable_hours.lower=3.0;
        assert!(evaluate_lifecycle_frontier(&x).is_err());
        x=input();x.candidates[0].cost_per_delivered_critical_kwh.evidence_status=EvidenceStatus::Expired;
        assert!(evaluate_lifecycle_frontier(&x).is_err());
    }

    #[test] fn mixed_storage_scope_is_rejected(){
        let mut x=input();x.candidates[1].storage_in_scope=false;x.candidates[1].minimum_storage_capacity_retention=None;
        assert!(evaluate_lifecycle_frontier(&x).is_err());
    }

    #[test] fn missing_storage_uncertainty_is_rejected(){
        let mut x=input();x.candidates[1].minimum_storage_capacity_retention=None;
        assert!(evaluate_lifecycle_frontier(&x).is_err());
    }

    #[test] fn candidate_order_does_not_change_frontier(){
        let mut x=input();let a=evaluate_lifecycle_frontier(&x).unwrap();
        x.candidates.reverse();let b=evaluate_lifecycle_frontier(&x).unwrap();
        assert_eq!(a.central_frontier_ids,b.central_frontier_ids);
        assert_eq!(a.robust_frontier_ids,b.robust_frontier_ids);
    }

    #[test] fn confidence_must_be_strictly_between_half_and_one(){
        let mut x=input();x.confidence_level=0.5;
        assert!(evaluate_lifecycle_frontier(&x).is_err());
    }

    #[test] fn declared_mission_shortfall_within_owner_gate_remains_feasible_and_visible(){
        let mut x=input();
        x.candidates[0].accounting.total_unserved_critical_service_wh=100.0;
        x.candidates[0].accounting.total_delivered_critical_service_wh=9900.0;
        x.candidates[0].accounting.completed_mission_count=100;
        x.candidates[0].accounting.critical_service_pass_rate_percent=100.0;
        x.candidates[0].unserved_critical_service_wh=interval(100.0,90.0,110.0);
        let r=evaluate_lifecycle_frontier(&x).unwrap();
        let a=r.candidates.iter().find(|v|v.candidate_id=="A").unwrap();
        assert!(a.service_feasible);
    }

    #[test] fn assumption_only_uncertainty_is_not_claim_eligible(){
        let mut x=input();
        x.candidates[0].cost_per_delivered_critical_kwh.evidence_class=
            crate::lifecycle_service_cost::LifecycleEvidenceClass::Assumption;
        assert!(evaluate_lifecycle_frontier(&x).is_err());
    }

}
