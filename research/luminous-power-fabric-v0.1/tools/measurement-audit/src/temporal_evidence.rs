use crate::capability_envelope::EvidenceStatus;
use crate::lifecycle_service_cost::LifecycleEvidenceClass;
use crate::AuditError;

const CLAIM: &str = "TEMPORAL_EVIDENCE_QUALIFICATION_ONLY_NOT_CLOCK_SYNC_OR_PHYSICAL_RESPONSE_CLAIM";
const NS_PER_SECOND: u128 = 1_000_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeScale { Tai, Utc }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SynchronizationState { Locked, Holdover, FreeRunning, Lost, Unknown }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapturePoint {
    PhysicalSample,
    HardwareTrigger,
    MessageReceipt,
    ProcessingComplete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemporalOrder { ABeforeB, BBeforeA, Indeterminate }

#[derive(Debug, Clone, PartialEq)]
pub struct TimebaseEvidence {
    pub clock_domain_id: String,
    pub clock_epoch_id: String,
    pub time_scale: TimeScale,
    pub synchronization_state: SynchronizationState,
    pub profile_id: String,
    pub profile_revision: String,
    pub configuration_sha256: String,
    pub last_sync_timestamp_ns: u64,
    pub max_locked_age_ns: u64,
    pub max_holdover_age_ns: u64,
    pub clock_error_at_sync_bound_ns: u64,
    pub oscillator_drift_bound_ppb: u64,
    pub synchronization_evidence_status: EvidenceStatus,
    pub synchronization_evidence_class: LifecycleEvidenceClass,
    pub synchronization_method_reference: String,
    pub synchronization_evidence_reference: String,
    /// TAI - UTC offset for UTC timebases, valid only over the declared UTC interval.
    pub utc_tai_offset_ns: Option<i64>,
    pub utc_offset_valid_from_ns: Option<u64>,
    pub utc_offset_valid_until_ns: Option<u64>,
    pub utc_offset_evidence_status: EvidenceStatus,
    pub utc_offset_evidence_class: LifecycleEvidenceClass,
    pub utc_offset_evidence_reference: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventObservation {
    pub event_id: String,
    pub event_type: String,
    pub clock_domain_id: String,
    pub clock_epoch_id: String,
    pub timebase_configuration_sha256: String,
    pub sequence: u64,
    pub timestamp_ns: u64,
    pub timestamp_quantization_bound_ns: u64,
    /// Conservative maximum delay between physical event and the declared capture point.
    pub capture_delay_bound_ns: u64,
    pub capture_point: CapturePoint,
    pub evidence_status: EvidenceStatus,
    pub evidence_class: LifecycleEvidenceClass,
    pub evidence_reference: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClockDomainRelation {
    pub from_clock_domain_id: String,
    pub from_clock_epoch_id: String,
    pub to_clock_domain_id: String,
    pub to_clock_epoch_id: String,
    /// Maps normalized from-domain TAI values into normalized to-domain TAI values.
    /// Reference and validity timestamps are normalized TAI nanoseconds.
    pub reference_timestamp_ns: u64,
    pub valid_from_timestamp_ns: u64,
    pub valid_until_timestamp_ns: u64,
    pub offset_ns: i64,
    pub offset_error_bound_ns: u64,
    pub relative_drift_bound_ppb: u64,
    pub evidence_status: EvidenceStatus,
    pub evidence_class: LifecycleEvidenceClass,
    pub method_reference: String,
    pub evidence_reference: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaiTimeInterval {
    pub lower_bound_ns: i128,
    pub upper_bound_ns: i128,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventOrderReport {
    pub event_a_id: String,
    pub event_b_id: String,
    pub interval_a_tai: TaiTimeInterval,
    pub interval_b_tai: TaiTimeInterval,
    pub order: TemporalOrder,
    pub same_clock_sequence_consistent: bool,
    pub claim_ceiling: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventDurationReport {
    pub start_event_id: String,
    pub end_event_id: String,
    pub minimum_duration_ns: u128,
    pub maximum_duration_ns: u128,
    pub positive_duration_established: bool,
    pub order: TemporalOrder,
    pub claim_ceiling: &'static str,
}

impl EventOrderReport {
    pub const fn claim_ceiling(&self) -> &'static str { CLAIM }
}
impl EventDurationReport {
    pub const fn claim_ceiling(&self) -> &'static str { CLAIM }
}

fn nonblank(value: &str) -> bool { !value.trim().is_empty() }
fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
fn qualified_evidence(status: EvidenceStatus, class: LifecycleEvidenceClass) -> bool {
    status == EvidenceStatus::Current
        && matches!(class, LifecycleEvidenceClass::IndependentPhysicalMeasurement | LifecycleEvidenceClass::AnalyticalModel)
}

fn drift_growth_ns(drift_bound_ppb: u64, elapsed_ns: u64) -> Result<u64, AuditError> {
    let product = u128::from(drift_bound_ppb) * u128::from(elapsed_ns);
    let rounded = product.checked_add(NS_PER_SECOND - 1)
        .ok_or_else(|| AuditError("clock drift uncertainty overflowed".to_string()))? / NS_PER_SECOND;
    u64::try_from(rounded).map_err(|_| AuditError("clock drift uncertainty exceeds supported range".to_string()))
}

fn shift_interval(
    interval: TaiTimeInterval,
    offset_ns: i64,
    uncertainty_ns: u64,
) -> Result<TaiTimeInterval, AuditError> {
    let offset = i128::from(offset_ns);
    let uncertainty = i128::from(uncertainty_ns);
    let lower = interval.lower_bound_ns.checked_add(offset)
        .and_then(|value| value.checked_sub(uncertainty))
        .ok_or_else(|| AuditError("time-domain lower bound overflowed".to_string()))?;
    let upper = interval.upper_bound_ns.checked_add(offset)
        .and_then(|value| value.checked_add(uncertainty))
        .ok_or_else(|| AuditError("time-domain upper bound overflowed".to_string()))?;
    if lower < 0 || upper < lower {
        return Err(AuditError("mapped TAI interval is invalid".to_string()));
    }
    Ok(TaiTimeInterval { lower_bound_ns: lower, upper_bound_ns: upper })
}

fn event_interval_tai(
    event: &EventObservation,
    timebase: &TimebaseEvidence,
) -> Result<TaiTimeInterval, AuditError> {
    if !nonblank(&event.event_id) || !nonblank(&event.event_type)
        || !nonblank(&event.evidence_reference)
        || !valid_sha256(&event.timebase_configuration_sha256)
        || !valid_sha256(&timebase.configuration_sha256)
        || event.clock_domain_id != timebase.clock_domain_id
        || event.clock_epoch_id != timebase.clock_epoch_id
        || event.timebase_configuration_sha256 != timebase.configuration_sha256
        || !qualified_evidence(event.evidence_status, event.evidence_class)
        || !qualified_evidence(timebase.synchronization_evidence_status, timebase.synchronization_evidence_class)
        || !nonblank(&timebase.clock_domain_id)
        || !nonblank(&timebase.clock_epoch_id)
        || !nonblank(&timebase.profile_id)
        || !nonblank(&timebase.profile_revision)
        || !nonblank(&timebase.synchronization_method_reference)
        || !nonblank(&timebase.synchronization_evidence_reference)
        || event.timestamp_ns < timebase.last_sync_timestamp_ns
    {
        return Err(AuditError(format!("event {:?} or its timebase evidence is incomplete or mismatched", event.event_id)));
    }

    let elapsed = event.timestamp_ns - timebase.last_sync_timestamp_ns;
    match timebase.synchronization_state {
        SynchronizationState::Locked if elapsed <= timebase.max_locked_age_ns => {}
        SynchronizationState::Holdover if elapsed <= timebase.max_holdover_age_ns => {}
        SynchronizationState::Locked | SynchronizationState::Holdover
        | SynchronizationState::FreeRunning | SynchronizationState::Lost | SynchronizationState::Unknown => {
            return Err(AuditError(format!("clock domain {:?} is not within a qualified sync/holdover window", timebase.clock_domain_id)));
        }
    }

    let drift = drift_growth_ns(timebase.oscillator_drift_bound_ppb, elapsed)?;
    let symmetric_error = timebase.clock_error_at_sync_bound_ns
        .checked_add(drift)
        .and_then(|v| v.checked_add(event.timestamp_quantization_bound_ns))
        .ok_or_else(|| AuditError("event time uncertainty overflowed".to_string()))?;

    let local_lower = i128::from(event.timestamp_ns)
        .checked_sub(i128::from(event.capture_delay_bound_ns))
        .and_then(|v| v.checked_sub(i128::from(symmetric_error)))
        .ok_or_else(|| AuditError("event lower timestamp bound overflowed".to_string()))?;
    let local_upper = i128::from(event.timestamp_ns)
        .checked_add(i128::from(symmetric_error))
        .ok_or_else(|| AuditError("event upper timestamp bound overflowed".to_string()))?;
    if local_lower < 0 || local_upper < local_lower {
        return Err(AuditError("event local-time interval is invalid".to_string()));
    }

    let interval = TaiTimeInterval { lower_bound_ns: local_lower, upper_bound_ns: local_upper };
    if timebase.time_scale == TimeScale::Tai {
        if timebase.utc_tai_offset_ns.is_some() {
            return Err(AuditError("TAI clock must not declare a UTC-to-TAI offset".to_string()));
        }
        return Ok(interval);
    }

    let offset = timebase.utc_tai_offset_ns
        .ok_or_else(|| AuditError("UTC timebase has no UTC-to-TAI offset evidence".to_string()))?;
    let valid_from = timebase.utc_offset_valid_from_ns
        .ok_or_else(|| AuditError("UTC-to-TAI validity start is missing".to_string()))?;
    let valid_until = timebase.utc_offset_valid_until_ns
        .ok_or_else(|| AuditError("UTC-to-TAI validity end is missing".to_string()))?;
    if valid_until < valid_from || local_lower < i128::from(valid_from) || local_upper > i128::from(valid_until)
        || !qualified_evidence(timebase.utc_offset_evidence_status, timebase.utc_offset_evidence_class)
        || !nonblank(&timebase.utc_offset_evidence_reference)
    {
        return Err(AuditError("UTC-to-TAI mapping is missing, expired or unqualified".to_string()));
    }
    shift_interval(interval, offset, 0)
}

fn apply_clock_relation(
    interval: TaiTimeInterval,
    relation: &ClockDomainRelation,
    from_timebase: &TimebaseEvidence,
    to_timebase: &TimebaseEvidence,
) -> Result<TaiTimeInterval, AuditError> {
    let event_tai_center = interval.lower_bound_ns
        .checked_add((interval.upper_bound_ns - interval.lower_bound_ns) / 2)
        .ok_or_else(|| AuditError("event TAI midpoint overflowed".to_string()))?;
    let valid_from = i128::from(relation.valid_from_timestamp_ns);
    let valid_until = i128::from(relation.valid_until_timestamp_ns);
    if relation.from_clock_domain_id != from_timebase.clock_domain_id
        || relation.from_clock_epoch_id != from_timebase.clock_epoch_id
        || relation.to_clock_domain_id != to_timebase.clock_domain_id
        || relation.to_clock_epoch_id != to_timebase.clock_epoch_id
        || (relation.to_clock_domain_id == relation.from_clock_domain_id && relation.to_clock_epoch_id == relation.from_clock_epoch_id)
        || interval.lower_bound_ns < valid_from
        || interval.upper_bound_ns > valid_until
        || i128::from(relation.reference_timestamp_ns) < valid_from
        || i128::from(relation.reference_timestamp_ns) > valid_until
        || relation.valid_until_timestamp_ns < relation.valid_from_timestamp_ns
        || !qualified_evidence(relation.evidence_status, relation.evidence_class)
        || !nonblank(&relation.method_reference)
        || !nonblank(&relation.evidence_reference)
    {
        return Err(AuditError("cross-domain clock relation is mismatched, expired or unqualified".to_string()));
    }
    let age_u128 = (event_tai_center - i128::from(relation.reference_timestamp_ns)).unsigned_abs();
    let age = u64::try_from(age_u128)
        .map_err(|_| AuditError("cross-domain relation age exceeds supported range".to_string()))?;
    let drift = drift_growth_ns(relation.relative_drift_bound_ppb, age)?;
    let total_error = relation.offset_error_bound_ns.checked_add(drift)
        .ok_or_else(|| AuditError("cross-domain relation uncertainty overflowed".to_string()))?;
    shift_interval(interval, relation.offset_ns, total_error)
}

/// Builds an uncertainty-bounded TAI interval for two physical events and
/// establishes order only when the intervals are strictly disjoint.
pub fn evaluate_event_order(
    event_a: &EventObservation,
    timebase_a: &TimebaseEvidence,
    event_b: &EventObservation,
    timebase_b: &TimebaseEvidence,
    relation_a_to_b: Option<&ClockDomainRelation>,
) -> Result<EventOrderReport, AuditError> {
    if event_a.event_id == event_b.event_id {
        return Err(AuditError("event IDs must be distinct".to_string()));
    }
    let mut interval_a = event_interval_tai(event_a, timebase_a)?;
    let interval_b = event_interval_tai(event_b, timebase_b)?;
    let same_epoch = event_a.clock_domain_id == event_b.clock_domain_id
        && event_a.clock_epoch_id == event_b.clock_epoch_id;
    if same_epoch {
        if event_a.timebase_configuration_sha256 != event_b.timebase_configuration_sha256 || relation_a_to_b.is_some() {
            return Err(AuditError("same clock epoch must use one configuration and no cross-domain relation".to_string()));
        }
    } else {
        let relation = relation_a_to_b
            .ok_or_else(|| AuditError("cross-domain or cross-epoch ordering requires a current clock relation".to_string()))?;
        interval_a = apply_clock_relation(interval_a, relation, timebase_a, timebase_b)?;
    }

    let order = if interval_a.upper_bound_ns < interval_b.lower_bound_ns {
        TemporalOrder::ABeforeB
    } else if interval_b.upper_bound_ns < interval_a.lower_bound_ns {
        TemporalOrder::BBeforeA
    } else {
        TemporalOrder::Indeterminate
    };

    let sequence_consistent = if same_epoch && event_a.sequence != event_b.sequence {
        !((event_a.sequence < event_b.sequence && order == TemporalOrder::BBeforeA)
            || (event_a.sequence > event_b.sequence && order == TemporalOrder::ABeforeB))
    } else {
        true
    };
    if !sequence_consistent {
        return Err(AuditError("clock timestamp order contradicts monotonic device sequence".to_string()));
    }

    Ok(EventOrderReport {
        event_a_id: event_a.event_id.clone(),
        event_b_id: event_b.event_id.clone(),
        interval_a_tai: interval_a,
        interval_b_tai: interval_b,
        order,
        same_clock_sequence_consistent: sequence_consistent,
        claim_ceiling: CLAIM,
    })
}

/// Returns a non-negative duration interval for a declared start/end pair.
/// If timestamp intervals overlap, the minimum duration is zero and a positive
/// response-time claim is not established.
pub fn evaluate_event_duration(
    start: &EventObservation,
    start_timebase: &TimebaseEvidence,
    end: &EventObservation,
    end_timebase: &TimebaseEvidence,
    relation_start_to_end: Option<&ClockDomainRelation>,
) -> Result<EventDurationReport, AuditError> {
    let order = evaluate_event_order(start, start_timebase, end, end_timebase, relation_start_to_end)?;
    if order.order == TemporalOrder::BBeforeA {
        return Err(AuditError("declared end event is provably before start event".to_string()));
    }
    let raw_min = order.interval_b_tai.lower_bound_ns - order.interval_a_tai.upper_bound_ns;
    let raw_max = order.interval_b_tai.upper_bound_ns - order.interval_a_tai.lower_bound_ns;
    if raw_max < 0 {
        return Err(AuditError("duration upper bound is negative".to_string()));
    }
    let minimum = raw_min.max(0) as u128;
    let maximum = raw_max as u128;
    Ok(EventDurationReport {
        start_event_id: start.event_id.clone(),
        end_event_id: end.event_id.clone(),
        minimum_duration_ns: minimum,
        maximum_duration_ns: maximum,
        positive_duration_established: minimum > 0,
        order: order.order,
        claim_ceiling: CLAIM,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(c: char) -> String { std::iter::repeat(c).take(64).collect() }

    fn timebase(domain: &str) -> TimebaseEvidence {
        TimebaseEvidence {
            clock_domain_id:domain.into(),clock_epoch_id:"epoch-1".into(),time_scale:TimeScale::Tai,
            synchronization_state:SynchronizationState::Locked,profile_id:"ieee1588-profile".into(),
            profile_revision:"2019+amendments-recorded".into(),configuration_sha256:h('a'),
            last_sync_timestamp_ns:0,max_locked_age_ns:10_000_000_000,max_holdover_age_ns:60_000_000_000,
            clock_error_at_sync_bound_ns:10,oscillator_drift_bound_ppb:10,
            synchronization_evidence_status:EvidenceStatus::Current,
            synchronization_evidence_class:LifecycleEvidenceClass::IndependentPhysicalMeasurement,
            synchronization_method_reference:"clock-method".into(),synchronization_evidence_reference:"clock-receipt".into(),
            utc_tai_offset_ns:None,utc_offset_valid_from_ns:None,utc_offset_valid_until_ns:None,
            utc_offset_evidence_status:EvidenceStatus::Missing,utc_offset_evidence_class:LifecycleEvidenceClass::Assumption,
            utc_offset_evidence_reference:String::new(),
        }
    }

    fn event(id:&str,domain:&str,ts:u64,seq:u64)->EventObservation {
        EventObservation {
            event_id:id.into(),event_type:"breaker_feedback".into(),clock_domain_id:domain.into(),clock_epoch_id:"epoch-1".into(),
            timebase_configuration_sha256:h('a'),sequence:seq,timestamp_ns:ts,
            timestamp_quantization_bound_ns:2,capture_delay_bound_ns:2,capture_point:CapturePoint::HardwareTrigger,
            evidence_status:EvidenceStatus::Current,evidence_class:LifecycleEvidenceClass::IndependentPhysicalMeasurement,
            evidence_reference:"event-receipt".into(),
        }
    }

    #[test]
    fn disjoint_time_intervals_establish_order() {
        let a=event("a","clock-a",1_000_000,1);
        let b=event("b","clock-a",2_000_000,2);
        let report=evaluate_event_order(&a,&timebase("clock-a"),&b,&timebase("clock-a"),None).unwrap();
        assert_eq!(report.order,TemporalOrder::ABeforeB);
    }

    #[test]
    fn overlapping_time_intervals_are_order_unknown() {
        let mut a=event("a","clock-a",1_000_000,1);
        let mut b=event("b","clock-a",1_000_005,2);
        a.capture_delay_bound_ns=5;b.capture_delay_bound_ns=5;
        let report=evaluate_event_order(&a,&timebase("clock-a"),&b,&timebase("clock-a"),None).unwrap();
        assert_eq!(report.order,TemporalOrder::Indeterminate);
    }

    #[test]
    fn holdover_uncertainty_grows_with_elapsed_time() {
        let mut tb=timebase("clock-a");tb.synchronization_state=SynchronizationState::Holdover;
        tb.max_holdover_age_ns=5_000_000_000;tb.oscillator_drift_bound_ppb=100;
        let a=event("a","clock-a",1_000_000,1);let b=event("b","clock-a",2_000_000,2);
        let near_sync=evaluate_event_order(&a,&tb,&b,&tb,None).unwrap();
        tb.last_sync_timestamp_ns=0;
        let late=event("late","clock-a",4_000_000_000,3);
        let interval=event_interval_tai(&late,&tb).unwrap();
        assert!(interval.upper_bound_ns-interval.lower_bound_ns
            > near_sync.interval_a_tai.upper_bound_ns-near_sync.interval_a_tai.lower_bound_ns);
    }

    #[test]
    fn lost_or_overdue_clock_is_unqualified() {
        let mut tb=timebase("clock-a");tb.synchronization_state=SynchronizationState::Lost;
        let a=event("a","clock-a",1_000_000,1);let b=event("b","clock-a",2_000_000,2);
        assert!(evaluate_event_order(&a,&tb,&b,&tb,None).is_err());
        tb=timebase("clock-a");tb.max_locked_age_ns=10;
        assert!(evaluate_event_order(&a,&tb,&b,&tb,None).is_err());
    }

    #[test]
    fn cross_domain_order_requires_a_current_relation() {
        let a=event("a","clock-a",1_000_000,1);let b=event("b","clock-b",2_000_000,1);
        let ta=timebase("clock-a");let tb=timebase("clock-b");
        assert!(evaluate_event_order(&a,&ta,&b,&tb,None).is_err());
        let relation=ClockDomainRelation{
            from_clock_domain_id:"clock-a".into(),from_clock_epoch_id:"epoch-1".into(),
            to_clock_domain_id:"clock-b".into(),to_clock_epoch_id:"epoch-1".into(),
            reference_timestamp_ns:1_000_000,valid_from_timestamp_ns:0,valid_until_timestamp_ns:10_000_000,
            offset_ns:0,offset_error_bound_ns:10,relative_drift_bound_ppb:1,
            evidence_status:EvidenceStatus::Current,evidence_class:LifecycleEvidenceClass::IndependentPhysicalMeasurement,
            method_reference:"mapping-method".into(),evidence_reference:"mapping-receipt".into(),
        };
        let report=evaluate_event_order(&a,&ta,&b,&tb,Some(&relation)).unwrap();
        assert_eq!(report.order,TemporalOrder::ABeforeB);
    }

    #[test]
    fn expired_or_unverified_cross_domain_relation_is_rejected() {
        let a=event("a","clock-a",1_000_000,1);let b=event("b","clock-b",2_000_000,1);
        let ta=timebase("clock-a");let tb=timebase("clock-b");
        let mut relation=ClockDomainRelation{
            from_clock_domain_id:"clock-a".into(),from_clock_epoch_id:"epoch-1".into(),
            to_clock_domain_id:"clock-b".into(),to_clock_epoch_id:"epoch-1".into(),
            reference_timestamp_ns:1_000_000,valid_from_timestamp_ns:0,valid_until_timestamp_ns:1_000_001,
            offset_ns:0,offset_error_bound_ns:1,relative_drift_bound_ppb:1,
            evidence_status:EvidenceStatus::Current,evidence_class:LifecycleEvidenceClass::IndependentPhysicalMeasurement,
            method_reference:"mapping".into(),evidence_reference:"receipt".into(),
        };
        assert!(evaluate_event_order(&a,&ta,&b,&tb,Some(&relation)).is_err());
        relation.valid_until_timestamp_ns=3_000_000;relation.evidence_status=EvidenceStatus::Expired;
        assert!(evaluate_event_order(&a,&ta,&b,&tb,Some(&relation)).is_err());
    }

    #[test]
    fn utc_requires_current_valid_tai_conversion() {
        let mut tb=timebase("clock-a");tb.time_scale=TimeScale::Utc;
        let a=event("a","clock-a",1_000_000,1);let b=event("b","clock-a",2_000_000,2);
        assert!(evaluate_event_order(&a,&tb,&b,&tb,None).is_err());
        tb.utc_tai_offset_ns=Some(37_000_000_000);tb.utc_offset_valid_from_ns=Some(0);
        tb.utc_offset_valid_until_ns=Some(10_000_000);tb.utc_offset_evidence_status=EvidenceStatus::Current;
        tb.utc_offset_evidence_class=LifecycleEvidenceClass::IndependentPhysicalMeasurement;
        tb.utc_offset_evidence_reference="utc-offset-receipt".into();
        let report=evaluate_event_order(&a,&tb,&b,&tb,None).unwrap();
        assert_eq!(report.order,TemporalOrder::ABeforeB);
    }

    #[test]
    fn timestamp_configuration_or_sequence_contradiction_is_rejected() {
        let mut a=event("a","clock-a",2_000_000,1);let b=event("b","clock-a",1_000_000,2);
        assert!(evaluate_event_order(&a,&timebase("clock-a"),&b,&timebase("clock-a"),None).is_err());
        a=event("a","clock-a",1_000_000,1);let mut tb=timebase("clock-a");tb.configuration_sha256=h('b');
        assert!(evaluate_event_order(&a,&tb,&b,&tb,None).is_err());
    }

    #[test]
    fn response_time_positive_claim_requires_non_overlapping_events() {
        let start=event("command","clock-a",1_000_000,1);
        let end=event("effect","clock-a",2_000_000,2);
        let report=evaluate_event_duration(&start,&timebase("clock-a"),&end,&timebase("clock-a"),None).unwrap();
        assert!(report.positive_duration_established);assert!(report.minimum_duration_ns>0);
        let mut close=event("effect-close","clock-a",1_000_005,2);close.capture_delay_bound_ns=10;
        let uncertain=evaluate_event_duration(&start,&timebase("clock-a"),&close,&timebase("clock-a"),None).unwrap();
        assert!(!uncertain.positive_duration_established);assert_eq!(uncertain.minimum_duration_ns,0);
    }

    #[test]
    fn synthetic_or_assumption_evidence_cannot_qualify_time() {
        let mut a=event("a","clock-a",1_000_000,1);let b=event("b","clock-a",2_000_000,2);
        a.evidence_class=LifecycleEvidenceClass::SyntheticFixture;
        assert!(evaluate_event_order(&a,&timebase("clock-a"),&b,&timebase("clock-a"),None).is_err());
    }
    #[test]
    fn utc_offset_validity_must_cover_the_entire_uncertainty_interval() {
        let mut tb=timebase("clock-a");
        tb.time_scale=TimeScale::Utc;
        tb.utc_tai_offset_ns=Some(37_000_000_000);
        tb.utc_offset_valid_from_ns=Some(0);
        tb.utc_offset_valid_until_ns=Some(1_000_001);
        tb.utc_offset_evidence_status=EvidenceStatus::Current;
        tb.utc_offset_evidence_class=LifecycleEvidenceClass::IndependentPhysicalMeasurement;
        tb.utc_offset_evidence_reference="utc-offset-receipt".into();
        let a=event("a","clock-a",1_000_000,1);
        let b=event("b","clock-a",2_000_000,2);
        assert!(evaluate_event_order(&a,&tb,&b,&tb,None).is_err());
    }

    #[test]
    fn cross_domain_relation_validity_must_cover_the_entire_source_interval() {
        let a=event("a","clock-a",1_000_000,1);
        let b=event("b","clock-b",2_000_000,1);
        let ta=timebase("clock-a");
        let tb=timebase("clock-b");
        let relation=ClockDomainRelation{
            from_clock_domain_id:"clock-a".into(),from_clock_epoch_id:"epoch-1".into(),
            to_clock_domain_id:"clock-b".into(),to_clock_epoch_id:"epoch-1".into(),
            reference_timestamp_ns:1_000_000,valid_from_timestamp_ns:1_000_000,valid_until_timestamp_ns:1_000_001,
            offset_ns:0,offset_error_bound_ns:1,relative_drift_bound_ppb:1,
            evidence_status:EvidenceStatus::Current,evidence_class:LifecycleEvidenceClass::IndependentPhysicalMeasurement,
            method_reference:"mapping-method".into(),evidence_reference:"mapping-receipt".into(),
        };
        assert!(evaluate_event_order(&a,&ta,&b,&tb,Some(&relation)).is_err());
    }

}
