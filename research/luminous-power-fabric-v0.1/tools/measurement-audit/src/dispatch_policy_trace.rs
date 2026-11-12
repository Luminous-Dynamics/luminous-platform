use crate::capability_envelope::EvidenceStatus;
use crate::AuditError;
use std::collections::HashSet;

const TRACE_CLAIM_CEILING: &str =
    "DISPATCH_TRACE_ACCOUNTING_ONLY_NOT_CONTROLLER_SAFETY_OR_PHYSICAL_QUALIFICATION";
const ENERGY_RELATIVE_TOLERANCE: f64 = 1.0e-9;
const MAX_TRACE_RECORDS: usize = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TraceEvidenceClass { Synthetic, Modelled, Measured, IndependentlyWitnessed, Accredited }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationQuality { Current, Stale, Missing, Invalid, Unknown }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionDisposition { Applied, Rejected, Unknown, Noop }

#[derive(Debug, Clone, PartialEq)]
pub struct DispatchObservation {
    pub id: String,
    pub source_sample_sha256: String,
    pub event_time_ns: u64,
    pub available_time_ns: u64,
    pub clock_epoch_id: String,
    pub timebase_sha256: String,
    pub quality: ObservationQuality,
    pub evidence_status: EvidenceStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationUse {
    pub observation_id: String,
    pub interpreted_as_current: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DispatchDecision {
    pub id: String,
    pub trigger_time_ns: u64,
    pub start_time_ns: u64,
    pub completion_time_ns: u64,
    pub deadline_time_ns: u64,
    pub compute_duration_ns: u64,
    pub action_sha256: String,
    pub disposition: ActionDisposition,
    pub effect_observed_time_ns: Option<u64>,
    pub observations_used: Vec<ObservationUse>,
    pub fallback_entered: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DispatchServiceInterval {
    pub id: String,
    pub start_ns: u64,
    pub end_ns: u64,
    pub outcome_evidence_sha256: String,
    pub critical_required_wh: f64,
    pub critical_delivered_wh: f64,
    pub critical_unserved_wh: f64,
    pub maximum_unserved_critical_wh: f64,
    pub primary_source_input_wh: f64,
    pub auxiliary_energy_wh: f64,
    pub storage_charge_input_wh: f64,
    pub storage_discharge_output_wh: f64,
    pub reserve_lower_bound_wh: f64,
    pub minimum_reserve_required_wh: f64,
    pub hard_limit_violation_references: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchedDispatchContext {
    pub scenario_manifest_sha256: String,
    pub mission_profile_sha256: String,
    pub environment_profile_sha256: String,
    pub plant_model_sha256: String,
    pub exogenous_event_trace_sha256: String,
    pub initial_state_sha256: String,
    pub terminal_obligation_sha256: String,
    pub auxiliary_boundary_sha256: String,
    pub information_budget_sha256: String,
    pub compute_budget_sha256: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DispatchPolicyTrace {
    pub trace_id: String,
    pub scenario_id: String,
    pub scenario_seed: u64,
    pub policy_id: String,
    pub policy_revision_sha256: String,
    pub matched: MatchedDispatchContext,
    pub clock_epoch_id: String,
    pub timebase_sha256: String,
    pub mission_horizon_ns: u64,
    pub decision_cadence_ns: u64,
    pub evidence_bundle_sha256: String,
    pub controller_evidence_class: TraceEvidenceClass,
    pub plant_evidence_class: TraceEvidenceClass,
    pub controller_evidence_status: EvidenceStatus,
    pub plant_evidence_status: EvidenceStatus,
    pub observations: Vec<DispatchObservation>,
    pub decisions: Vec<DispatchDecision>,
    pub intervals: Vec<DispatchServiceInterval>,
    pub independent_assessment_reference: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DispatchPolicyTraceReport {
    pub trace_id: String,
    pub scenario_id: String,
    pub effective_evidence_class: TraceEvidenceClass,
    pub critical_service_pass: bool,
    pub component_constraints_pass: bool,
    pub reserve_pass: bool,
    pub causal_integrity_pass: bool,
    pub evidence_current: bool,
    pub overall_hard_gates_pass: bool,
    pub critical_required_wh: f64,
    pub critical_delivered_wh: f64,
    pub critical_unserved_wh: f64,
    pub primary_source_input_wh: f64,
    pub auxiliary_energy_wh: f64,
    pub storage_charge_input_wh: f64,
    pub storage_discharge_output_wh: f64,
    pub minimum_reserve_margin_wh: f64,
    pub missed_decision_deadlines: usize,
    pub fallback_count: usize,
    pub rejected_action_count: usize,
    pub unknown_action_count: usize,
    pub noncurrent_observation_use_count: usize,
    pub blockers: Vec<String>,
    pub claim_ceiling: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DispatchPolicyComparison {
    pub reference: DispatchPolicyTraceReport,
    pub candidate: DispatchPolicyTraceReport,
    pub structural_match: bool,
    pub comparison_eligible: bool,
    pub physical_measurement_eligible: bool,
    pub independent_claim_evidence_eligible: bool,
    pub candidate_minus_reference_critical_unserved_wh: Option<f64>,
    pub candidate_minus_reference_primary_input_wh: Option<f64>,
    pub candidate_minus_reference_auxiliary_energy_wh: Option<f64>,
    pub blockers: Vec<String>,
    pub claim_ceiling: &'static str,
}

fn nonblank(v: &str) -> bool { !v.trim().is_empty() }
fn valid_sha256(v: &str) -> bool {
    v.len() == 64 && v.bytes().all(|b| b.is_ascii_hexdigit())
}
fn finite_nonnegative(v: f64) -> bool { v.is_finite() && v >= 0.0 }
fn energy_equal(a: f64, b: f64) -> bool {
    (a-b).abs() <= ENERGY_RELATIVE_TOLERANCE * a.abs().max(b.abs()).max(1.0)
}
fn effective_evidence(t: &DispatchPolicyTrace) -> TraceEvidenceClass {
    if t.controller_evidence_class <= t.plant_evidence_class {
        t.controller_evidence_class
    } else { t.plant_evidence_class }
}

fn validate_context(t: &DispatchPolicyTrace) -> Result<(), AuditError> {
    if !nonblank(&t.trace_id) || !nonblank(&t.scenario_id) || !nonblank(&t.policy_id) {
        return Err(AuditError("trace, scenario and policy identities must be non-empty".to_string()));
    }
    let d = [
        t.policy_revision_sha256.as_str(), t.evidence_bundle_sha256.as_str(),
        t.timebase_sha256.as_str(), t.matched.scenario_manifest_sha256.as_str(),
        t.matched.mission_profile_sha256.as_str(), t.matched.environment_profile_sha256.as_str(),
        t.matched.plant_model_sha256.as_str(), t.matched.exogenous_event_trace_sha256.as_str(),
        t.matched.initial_state_sha256.as_str(), t.matched.terminal_obligation_sha256.as_str(),
        t.matched.auxiliary_boundary_sha256.as_str(), t.matched.information_budget_sha256.as_str(),
        t.matched.compute_budget_sha256.as_str(),
    ];
    if d.iter().any(|v| !valid_sha256(v)) {
        return Err(AuditError("dispatch trace contains a malformed SHA-256 digest".to_string()));
    }
    if !nonblank(&t.clock_epoch_id) || t.mission_horizon_ns == 0 || t.decision_cadence_ns == 0 {
        return Err(AuditError("clock epoch, mission horizon and decision cadence must be valid".to_string()));
    }
    if t.observations.len() > MAX_TRACE_RECORDS || t.decisions.len() > MAX_TRACE_RECORDS
        || t.intervals.is_empty() || t.intervals.len() > MAX_TRACE_RECORDS {
        return Err(AuditError("trace record counts are empty or exceed supported bounds".to_string()));
    }
    if t.independent_assessment_reference.as_deref().is_some_and(|v| !nonblank(v)) {
        return Err(AuditError("independent assessment reference cannot be blank".to_string()));
    }
    Ok(())
}

fn validate_observations(t: &DispatchPolicyTrace) -> Result<(), AuditError> {
    let mut ids = HashSet::new();
    for o in &t.observations {
        if !nonblank(&o.id) || !ids.insert(o.id.as_str())
            || !valid_sha256(&o.source_sample_sha256) || !valid_sha256(&o.timebase_sha256)
            || !nonblank(&o.clock_epoch_id) {
            return Err(AuditError("observation identity, digest or clock metadata is invalid".to_string()));
        }
        if o.event_time_ns > o.available_time_ns || o.available_time_ns > t.mission_horizon_ns
            || o.clock_epoch_id != t.clock_epoch_id || o.timebase_sha256 != t.timebase_sha256 {
            return Err(AuditError("observation timing, clock epoch or timebase binding is inconsistent".to_string()));
        }
    }
    Ok(())
}

fn validate_decisions(t: &DispatchPolicyTrace) -> Result<(usize,usize,usize,usize,usize), AuditError> {
    let mut decision_ids=HashSet::new();
    let mut deadline_misses=0; let mut fallbacks=0; let mut rejected=0;
    let mut unknown=0; let mut noncurrent=0; let mut prior_completion=None;
    for d in &t.decisions {
        if !nonblank(&d.id) || !decision_ids.insert(d.id.as_str()) || !valid_sha256(&d.action_sha256) {
            return Err(AuditError("decision identity or action digest is invalid".to_string()));
        }
        if d.trigger_time_ns>d.start_time_ns || d.start_time_ns>d.completion_time_ns
            || d.completion_time_ns>t.mission_horizon_ns || d.deadline_time_ns<d.trigger_time_ns
            || d.deadline_time_ns>t.mission_horizon_ns
            || d.compute_duration_ns>d.completion_time_ns-d.start_time_ns {
            return Err(AuditError("decision trigger/start/complete/deadline timing is inconsistent".to_string()));
        }
        if prior_completion.is_some_and(|p| d.completion_time_ns<=p) {
            return Err(AuditError("decision completion times must be strictly increasing".to_string()));
        }
        prior_completion=Some(d.completion_time_ns);
        if d.completion_time_ns>d.deadline_time_ns { deadline_misses+=1; }
        if d.fallback_entered { fallbacks+=1; }
        let mut used=HashSet::new();
        for u in &d.observations_used {
            if !used.insert(u.observation_id.as_str()) {
                return Err(AuditError("decision contains a duplicate observation reference".to_string()));
            }
            let o=t.observations.iter().find(|o| o.id==u.observation_id)
                .ok_or_else(|| AuditError(format!("decision references unknown observation {:?}",u.observation_id)))?;
            if o.available_time_ns>d.start_time_ns {
                return Err(AuditError("future-information leakage: observation was unavailable when decision began".to_string()));
            }
            if u.interpreted_as_current && (o.quality!=ObservationQuality::Current
                || o.evidence_status!=EvidenceStatus::Current) {
                return Err(AuditError("non-current observation was interpreted as current".to_string()));
            }
            if !u.interpreted_as_current { noncurrent+=1; }
        }
        match d.disposition {
            ActionDisposition::Applied => {
                let at=d.effect_observed_time_ns.ok_or_else(|| AuditError("applied action requires observed effect time".to_string()))?;
                if at<d.completion_time_ns || at>t.mission_horizon_ns {
                    return Err(AuditError("applied action effect time is inconsistent".to_string()));
                }
            }
            ActionDisposition::Rejected|ActionDisposition::Unknown|ActionDisposition::Noop => {
                if d.effect_observed_time_ns.is_some() {
                    return Err(AuditError("non-applied action cannot claim an applied-effect timestamp".to_string()));
                }
            }
        }
        if d.disposition==ActionDisposition::Rejected { rejected+=1; }
        if d.disposition==ActionDisposition::Unknown { unknown+=1; }
    }
    Ok((deadline_misses,fallbacks,rejected,unknown,noncurrent))
}

/// Validate and summarize an already-produced policy/plant trace. No controller
/// runs and no physical plant model is included.
pub fn evaluate_dispatch_policy_trace(t: &DispatchPolicyTrace) -> Result<DispatchPolicyTraceReport, AuditError> {
    validate_context(t)?;
    validate_observations(t)?;
    let (misses,fallbacks,rejected,unknown,noncurrent)=validate_decisions(t)?;

    let mut ids=HashSet::new();
    let mut expected_start=0u64;
    let mut required=0.0; let mut delivered=0.0; let mut unserved=0.0;
    let mut primary=0.0; let mut auxiliary=0.0; let mut charge=0.0; let mut discharge=0.0;
    let mut min_reserve_margin=f64::INFINITY;
    let mut service_pass=true; let mut component_pass=true; let mut reserve_pass=true;
    let mut blockers=Vec::new();

    for i in &t.intervals {
        if !nonblank(&i.id) || !ids.insert(i.id.as_str()) || !valid_sha256(&i.outcome_evidence_sha256)
            || i.start_ns!=expected_start || i.end_ns<=i.start_ns || i.end_ns>t.mission_horizon_ns {
            return Err(AuditError("service intervals must have unique IDs and contiguous positive half-open windows".to_string()));
        }
        expected_start=i.end_ns;
        let values=[i.critical_required_wh,i.critical_delivered_wh,i.critical_unserved_wh,
            i.maximum_unserved_critical_wh,i.primary_source_input_wh,i.auxiliary_energy_wh,
            i.storage_charge_input_wh,i.storage_discharge_output_wh,i.reserve_lower_bound_wh,
            i.minimum_reserve_required_wh];
        if values.iter().any(|v| !finite_nonnegative(*v)) {
            return Err(AuditError("service interval contains a negative or non-finite quantity".to_string()));
        }
        if i.critical_delivered_wh>i.critical_required_wh
            || !energy_equal(i.critical_required_wh,i.critical_delivered_wh+i.critical_unserved_wh) {
            return Err(AuditError("critical energy must reconcile as delivered plus unserved equals required".to_string()));
        }
        if i.critical_unserved_wh>i.maximum_unserved_critical_wh {
            service_pass=false; blockers.push(format!("critical_service_shortfall:{}",i.id));
        }
        if !i.hard_limit_violation_references.is_empty() {
            component_pass=false;
            if i.hard_limit_violation_references.iter().any(|v| !nonblank(v)) {
                return Err(AuditError("hard-limit violation references must be non-empty".to_string()));
            }
            blockers.push(format!("component_or_protection_limit_violation:{}",i.id));
        }
        let margin=i.reserve_lower_bound_wh-i.minimum_reserve_required_wh;
        min_reserve_margin=min_reserve_margin.min(margin);
        if margin<0.0 { reserve_pass=false; blockers.push(format!("reserve_breach:{}",i.id)); }
        required+=i.critical_required_wh; delivered+=i.critical_delivered_wh; unserved+=i.critical_unserved_wh;
        primary+=i.primary_source_input_wh; auxiliary+=i.auxiliary_energy_wh;
        charge+=i.storage_charge_input_wh; discharge+=i.storage_discharge_output_wh;
    }
    if expected_start!=t.mission_horizon_ns {
        return Err(AuditError("service intervals do not cover the complete mission horizon".to_string()));
    }
    let totals=[required,delivered,unserved,primary,auxiliary,charge,discharge,min_reserve_margin];
    if totals.iter().any(|v| !v.is_finite()) {
        return Err(AuditError("dispatch trace total arithmetic overflowed".to_string()));
    }

    let evidence_current=t.controller_evidence_status==EvidenceStatus::Current
        && t.plant_evidence_status==EvidenceStatus::Current && unknown==0;
    if !evidence_current { blockers.push("controller_or_plant_evidence_not_current_or_action_unknown".to_string()); }
    let hard_gates=service_pass&&component_pass&&reserve_pass&&evidence_current;
    Ok(DispatchPolicyTraceReport {
        trace_id:t.trace_id.clone(),scenario_id:t.scenario_id.clone(),
        effective_evidence_class:if t.controller_evidence_class<=t.plant_evidence_class {t.controller_evidence_class} else {t.plant_evidence_class},
        critical_service_pass:service_pass,component_constraints_pass:component_pass,reserve_pass,
        causal_integrity_pass:true,evidence_current,overall_hard_gates_pass:hard_gates,
        critical_required_wh:required,critical_delivered_wh:delivered,critical_unserved_wh:unserved,
        primary_source_input_wh:primary,auxiliary_energy_wh:auxiliary,
        storage_charge_input_wh:charge,storage_discharge_output_wh:discharge,
        minimum_reserve_margin_wh:min_reserve_margin,missed_decision_deadlines:misses,
        fallback_count:fallbacks,rejected_action_count:rejected,unknown_action_count:unknown,
        noncurrent_observation_use_count:noncurrent,blockers,claim_ceiling:TRACE_CLAIM_CEILING,
    })
}

fn same_context(a:&DispatchPolicyTrace,b:&DispatchPolicyTrace)->bool {
    a.scenario_id==b.scenario_id && a.scenario_seed==b.scenario_seed
        && a.matched==b.matched && a.clock_epoch_id==b.clock_epoch_id
        && a.timebase_sha256==b.timebase_sha256
        && a.mission_horizon_ns==b.mission_horizon_ns
        && a.decision_cadence_ns==b.decision_cadence_ns
}

/// Pairwise comparison with no deltas when scenario or operating context differs.
pub fn compare_dispatch_policy_traces(
    reference:&DispatchPolicyTrace,candidate:&DispatchPolicyTrace
)->Result<DispatchPolicyComparison,AuditError>{
    if reference.trace_id==candidate.trace_id || reference.policy_revision_sha256==candidate.policy_revision_sha256 {
        return Err(AuditError("reference and candidate traces/policy revisions must be distinct".to_string()));
    }
    let r=evaluate_dispatch_policy_trace(reference)?;
    let c=evaluate_dispatch_policy_trace(candidate)?;
    let matched=same_context(reference,candidate);
    let blockers=if matched {Vec::new()} else {vec!["mismatched_scenario_or_operating_context".to_string()]};
    let physical=matched && r.evidence_current && c.evidence_current
        && r.effective_evidence_class>=TraceEvidenceClass::Measured
        && c.effective_evidence_class>=TraceEvidenceClass::Measured;
    let independent=physical
        && r.effective_evidence_class>=TraceEvidenceClass::IndependentlyWitnessed
        && c.effective_evidence_class>=TraceEvidenceClass::IndependentlyWitnessed
        && reference.independent_assessment_reference.as_deref().is_some_and(nonblank)
        && candidate.independent_assessment_reference.as_deref().is_some_and(nonblank);
    Ok(DispatchPolicyComparison {
        reference:r.clone(),candidate:c.clone(),structural_match:matched,comparison_eligible:matched,
        physical_measurement_eligible:physical,independent_claim_evidence_eligible:independent,
        candidate_minus_reference_critical_unserved_wh:if matched {Some(c.critical_unserved_wh-r.critical_unserved_wh)} else {None},
        candidate_minus_reference_primary_input_wh:if matched {Some(c.primary_source_input_wh-r.primary_source_input_wh)} else {None},
        candidate_minus_reference_auxiliary_energy_wh:if matched {Some(c.auxiliary_energy_wh-r.auxiliary_energy_wh)} else {None},
        blockers,claim_ceiling:"MATCHED_DISPATCH_TRACE_COMPARISON_ONLY_NOT_SUPERIORITY_OR_CONFORMITY",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn h(c:char)->String{std::iter::repeat(c).take(64).collect()}
    fn obs()->DispatchObservation{DispatchObservation{id:"o1".into(),source_sample_sha256:h('a'),
        event_time_ns:10,available_time_ns:20,clock_epoch_id:"epoch".into(),timebase_sha256:h('b'),
        quality:ObservationQuality::Current,evidence_status:EvidenceStatus::Current}}
    fn decision()->DispatchDecision{DispatchDecision{id:"d1".into(),trigger_time_ns:20,start_time_ns:25,
        completion_time_ns:30,deadline_time_ns:40,compute_duration_ns:5,action_sha256:h('c'),
        disposition:ActionDisposition::Applied,effect_observed_time_ns:Some(35),
        observations_used:vec![ObservationUse{observation_id:"o1".into(),interpreted_as_current:true}],
        fallback_entered:false}}
    fn interval()->DispatchServiceInterval{DispatchServiceInterval{id:"i1".into(),start_ns:0,end_ns:100,
        outcome_evidence_sha256:h('d'),critical_required_wh:100.0,critical_delivered_wh:100.0,
        critical_unserved_wh:0.0,maximum_unserved_critical_wh:0.0,primary_source_input_wh:120.0,
        auxiliary_energy_wh:5.0,storage_charge_input_wh:0.0,storage_discharge_output_wh:0.0,
        reserve_lower_bound_wh:50.0,minimum_reserve_required_wh:40.0,
        hard_limit_violation_references:Vec::new()}}
    fn trace()->DispatchPolicyTrace{DispatchPolicyTrace{trace_id:"t1".into(),scenario_id:"s1".into(),
        scenario_seed:7,policy_id:"p1".into(),policy_revision_sha256:h('e'),
        matched:MatchedDispatchContext{scenario_manifest_sha256:h('1'),mission_profile_sha256:h('2'),
            environment_profile_sha256:h('3'),plant_model_sha256:h('4'),exogenous_event_trace_sha256:h('5'),
            initial_state_sha256:h('6'),terminal_obligation_sha256:h('7'),auxiliary_boundary_sha256:h('8'),
            information_budget_sha256:h('9'),compute_budget_sha256:h('a')},
        clock_epoch_id:"epoch".into(),timebase_sha256:h('b'),mission_horizon_ns:100,decision_cadence_ns:20,
        evidence_bundle_sha256:h('c'),controller_evidence_class:TraceEvidenceClass::Synthetic,
        plant_evidence_class:TraceEvidenceClass::Synthetic,controller_evidence_status:EvidenceStatus::Current,
        plant_evidence_status:EvidenceStatus::Current,observations:vec![obs()],decisions:vec![decision()],
        intervals:vec![interval()],independent_assessment_reference:None}}

    #[test] fn accepts_matched_trace(){let r=evaluate_dispatch_policy_trace(&trace()).unwrap();assert!(r.overall_hard_gates_pass);assert_eq!(r.critical_delivered_wh,100.0);}
    #[test] fn rejects_future_information(){let mut t=trace();t.observations[0].available_time_ns=26;assert!(evaluate_dispatch_policy_trace(&t).is_err());}
    #[test] fn rejects_stale_marked_current(){let mut t=trace();t.observations[0].quality=ObservationQuality::Stale;assert!(evaluate_dispatch_policy_trace(&t).is_err());}
    #[test] fn records_stale_used_as_stale(){let mut t=trace();t.observations[0].quality=ObservationQuality::Stale;t.decisions[0].observations_used[0].interpreted_as_current=false;let r=evaluate_dispatch_policy_trace(&t).unwrap();assert_eq!(r.noncurrent_observation_use_count,1);}
    #[test] fn rejects_unknown_observation(){let mut t=trace();t.decisions[0].observations_used[0].observation_id="missing".into();assert!(evaluate_dispatch_policy_trace(&t).is_err());}
    #[test] fn rejects_duplicate_observation_ids(){let mut t=trace();let o=t.observations[0].clone();t.observations.push(o);assert!(evaluate_dispatch_policy_trace(&t).is_err());}
    #[test] fn rejects_duplicate_decision_ids(){let mut t=trace();let mut d=t.decisions[0].clone();d.trigger_time_ns=31;d.start_time_ns=32;d.completion_time_ns=33;d.deadline_time_ns=34;d.effect_observed_time_ns=Some(34);t.decisions.push(d);assert!(evaluate_dispatch_policy_trace(&t).is_err());}
    #[test] fn rejects_non_monotonic_decisions(){let mut t=trace();let mut d=t.decisions[0].clone();d.id="d2".into();d.trigger_time_ns=15;d.start_time_ns=16;d.completion_time_ns=17;d.deadline_time_ns=18;d.compute_duration_ns=1;d.effect_observed_time_ns=Some(19);t.decisions.push(d);assert!(evaluate_dispatch_policy_trace(&t).is_err());}
    #[test] fn applied_requires_effect_time(){let mut t=trace();t.decisions[0].effect_observed_time_ns=None;assert!(evaluate_dispatch_policy_trace(&t).is_err());}
    #[test] fn rejected_cannot_claim_effect_time(){let mut t=trace();t.decisions[0].disposition=ActionDisposition::Rejected;assert!(evaluate_dispatch_policy_trace(&t).is_err());}
    #[test] fn counts_deadline_miss(){let mut t=trace();t.decisions[0].completion_time_ns=45;t.decisions[0].effect_observed_time_ns=Some(50);let r=evaluate_dispatch_policy_trace(&t).unwrap();assert_eq!(r.missed_decision_deadlines,1);}
    #[test] fn rejects_interval_gap(){let mut t=trace();t.intervals[0].start_ns=1;assert!(evaluate_dispatch_policy_trace(&t).is_err());}
    #[test] fn rejects_overlap(){let mut t=trace();let mut i=interval();i.id="i2".into();i.start_ns=99;i.end_ns=100;t.intervals.push(i);assert!(evaluate_dispatch_policy_trace(&t).is_err());}
    #[test] fn requires_full_horizon(){let mut t=trace();t.intervals[0].end_ns=99;assert!(evaluate_dispatch_policy_trace(&t).is_err());}
    #[test] fn rejects_energy_reconciliation_error(){let mut t=trace();t.intervals[0].critical_unserved_wh=1.0;assert!(evaluate_dispatch_policy_trace(&t).is_err());}
    #[test] fn critical_shortfall_is_noncompensatory(){let mut t=trace();t.intervals[0].critical_delivered_wh=90.0;t.intervals[0].critical_unserved_wh=10.0;t.intervals[0].maximum_unserved_critical_wh=2.0;let r=evaluate_dispatch_policy_trace(&t).unwrap();assert!(!r.critical_service_pass);assert!(!r.overall_hard_gates_pass);}
    #[test] fn protection_violation_is_noncompensatory(){let mut t=trace();t.intervals[0].hard_limit_violation_references.push("limit-1".into());let r=evaluate_dispatch_policy_trace(&t).unwrap();assert!(!r.component_constraints_pass);assert!(!r.overall_hard_gates_pass);}
    #[test] fn reserve_excursion_cannot_be_averaged_away(){let mut t=trace();let mut a=interval();a.id="a".into();a.end_ns=50;a.reserve_lower_bound_wh=30.0;let mut b=interval();b.id="b".into();b.start_ns=50;b.reserve_lower_bound_wh=60.0;t.intervals=vec![a,b];let r=evaluate_dispatch_policy_trace(&t).unwrap();assert!(!r.reserve_pass);assert!(r.minimum_reserve_margin_wh<0.0);}
    #[test] fn pair_rejects_initial_state_mismatch(){let a=trace();let mut b=trace();b.trace_id="t2".into();b.policy_id="p2".into();b.policy_revision_sha256=h('f');b.matched.initial_state_sha256=h('f');let c=compare_dispatch_policy_traces(&a,&b).unwrap();assert!(!c.structural_match);assert!(c.candidate_minus_reference_primary_input_wh.is_none());}
    #[test] fn pair_requires_same_auxiliary_boundary(){let a=trace();let mut b=trace();b.trace_id="t2".into();b.policy_id="p2".into();b.policy_revision_sha256=h('f');b.matched.auxiliary_boundary_sha256=h('f');let c=compare_dispatch_policy_traces(&a,&b).unwrap();assert!(!c.comparison_eligible);}
    #[test] fn measured_controller_with_modelled_plant_remains_modelled(){let mut t=trace();t.controller_evidence_class=TraceEvidenceClass::Measured;t.plant_evidence_class=TraceEvidenceClass::Modelled;let r=evaluate_dispatch_policy_trace(&t).unwrap();assert_eq!(r.effective_evidence_class,TraceEvidenceClass::Modelled);}
    #[test] fn simulated_plant_cannot_claim_physical_comparison(){let mut a=trace();let mut b=trace();for x in [&mut a,&mut b]{x.controller_evidence_class=TraceEvidenceClass::Measured;x.plant_evidence_class=TraceEvidenceClass::Modelled;}b.trace_id="t2".into();b.policy_id="p2".into();b.policy_revision_sha256=h('f');let c=compare_dispatch_policy_traces(&a,&b).unwrap();assert!(c.comparison_eligible);assert!(!c.physical_measurement_eligible);}
    #[test] fn physical_requires_measured_controller_and_plant(){let mut a=trace();let mut b=trace();for x in [&mut a,&mut b]{x.controller_evidence_class=TraceEvidenceClass::Measured;x.plant_evidence_class=TraceEvidenceClass::Measured;}b.trace_id="t2".into();b.policy_id="p2".into();b.policy_revision_sha256=h('f');let c=compare_dispatch_policy_traces(&a,&b).unwrap();assert!(c.physical_measurement_eligible);assert!(!c.independent_claim_evidence_eligible);}
    #[test] fn unverified_plant_blocks_hard_gate(){let mut t=trace();t.plant_evidence_status=EvidenceStatus::Unverified;let r=evaluate_dispatch_policy_trace(&t).unwrap();assert!(!r.evidence_current);assert!(!r.overall_hard_gates_pass);}
    #[test] fn claim_ceiling_is_bounded(){let r=evaluate_dispatch_policy_trace(&trace()).unwrap();assert_eq!(r.claim_ceiling(),TRACE_CLAIM_CEILING);}
}
