//! Paired evaluator for diagnostic probe interference.
//! This sidecar measures observability externalities; it does not schedule,
//! authorize, or execute probes and does not compute value of information.

use crate::Digest;
use std::collections::{BTreeMap, BTreeSet};

pub const PROBE_IMPACT_SCHEMA: &str = "continuous-opsbench-probe-impact-v0.1";
pub const PPM: u128 = 1_000_000;
pub const MAX_PROBES: usize = 10_000;
pub const MAX_WINDOWS: usize = 100_000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProbeResourceUsage {
    pub cpu_millicore_ms: u64,
    pub memory_byte_ms: u64,
    pub disk_read_bytes: u64,
    pub disk_write_bytes: u64,
    pub network_bytes: u64,
    pub query_rows: u64,
    pub duration_ms: u64,
}
impl ProbeResourceUsage {
    fn add(self, other: Self) -> Option<Self> {
        Some(Self {
            cpu_millicore_ms: self.cpu_millicore_ms.checked_add(other.cpu_millicore_ms)?,
            memory_byte_ms: self.memory_byte_ms.checked_add(other.memory_byte_ms)?,
            disk_read_bytes: self.disk_read_bytes.checked_add(other.disk_read_bytes)?,
            disk_write_bytes: self.disk_write_bytes.checked_add(other.disk_write_bytes)?,
            network_bytes: self.network_bytes.checked_add(other.network_bytes)?,
            query_rows: self.query_rows.checked_add(other.query_rows)?,
            duration_ms: self.duration_ms.checked_add(other.duration_ms)?,
        })
    }
    fn within(self, max: Self) -> bool {
        self.cpu_millicore_ms <= max.cpu_millicore_ms
            && self.memory_byte_ms <= max.memory_byte_ms
            && self.disk_read_bytes <= max.disk_read_bytes
            && self.disk_write_bytes <= max.disk_write_bytes
            && self.network_bytes <= max.network_bytes
            && self.query_rows <= max.query_rows
            && self.duration_ms <= max.duration_ms
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProbeCapability { ReadOnly, Mutating }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProbeExecution {
    pub execution_id: String,
    pub probe_id: String,
    pub tenant_id: String,
    pub target: String,
    pub operation_digest: Digest,
    pub capability: ProbeCapability,
    pub executor_id: String,
    pub executor_control_domain: String,
    pub run_id: String,
    pub scenario_id: String,
    pub arm_id: String,
    pub paired_block_id: String,
    pub started_at_ms: u64,
    pub ended_at_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProbeImpactReceipt {
    pub receipt_id: String,
    pub execution_id: String,
    pub probe_id: String,
    pub tenant_id: String,
    pub target: String,
    pub operation_digest: Digest,
    pub run_id: String,
    pub scenario_id: String,
    pub arm_id: String,
    pub paired_block_id: String,
    pub impact_observer_id: String,
    pub impact_observer_control_domain: String,
    pub impact_observer_adapter_digest: Digest,
    pub source_sequence: u64,
    pub measured_start_ms: u64,
    pub measured_end_ms: u64,
    pub evidence_digest: Digest,
    pub complete: bool,
    pub resource_usage: ProbeResourceUsage,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceWindow {
    pub window_id: String,
    pub run_id: String,
    pub scenario_id: String,
    pub arm_id: String,
    pub paired_block_id: String,
    pub service: String,
    pub workload_class: String,
    pub starts_at_ms: u64,
    pub ends_at_ms: u64,
    pub offered_requests: u64,
    pub failed_requests: u64,
    pub slow_requests: u64,
    pub incorrect_business_results: u64,
    pub workload_profile_digest: Digest,
    pub environment_digest: Digest,
    pub fault_schedule_digest: Digest,
    pub service_observer_id: String,
    pub service_observer_control_domain: String,
    pub service_observer_adapter_digest: Digest,
    pub complete: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceImpactLimit {
    pub service: String,
    pub workload_class: String,
    pub maximum_failure_delta_ppm: u32,
    pub maximum_slow_delta_ppm: u32,
    pub maximum_incorrect_delta_ppm: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProbeImpactContract {
    pub probe_run_id: String,
    pub no_probe_run_id: String,
    pub scenario_id: String,
    pub paired_block_id: String,
    pub probe_arm_id: String,
    pub no_probe_arm_id: String,
    pub tenant_id: String,
    pub executor_control_domain: String,
    pub impact_observer_control_domain: String,
    pub impact_observer_adapter_digest: Digest,
    pub service_observer_control_domain: String,
    pub service_observer_adapter_digest: Digest,
    pub workload_profile_digest: Digest,
    pub environment_digest: Digest,
    pub fault_schedule_digest: Digest,
    pub maximum_probe_duration_ms: u64,
    pub maximum_total_resource_usage: ProbeResourceUsage,
    pub service_limits: Vec<ServiceImpactLimit>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProbeImpactDisposition { Pass, Fail, Indeterminate }

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProbeImpactFinding {
    InvalidIdentity,
    InvalidContract,
    TooManyRecords,
    DuplicateExecution,
    DuplicateReceipt,
    MissingImpactReceipt,
    DuplicateEvidence,
    IncompleteImpactReceipt,
    ImpactIdentityMismatch,
    ObserverNotIndependent,
    InvalidExecutionInterval,
    ProbeNotReadOnly,
    ProbeDurationExceeded,
    ResourceAccountingOverflow,
    ResourceEnvelopeExceeded,
    MissingServiceWindow,
    DuplicateServiceWindow,
    IncompleteServiceWindow,
    ServiceWindowIdentityMismatch,
    InvalidRequestCounts,
    ZeroOfferedDemand,
    MissingServiceLimit,
    ServiceFailureImpactExceeded,
    ServiceLatencyImpactExceeded,
    BusinessCorrectnessImpactExceeded,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceImpactResult {
    pub service: String,
    pub workload_class: String,
    pub offered_requests: u64,
    pub failure_delta_ppm: i64,
    pub slow_delta_ppm: i64,
    pub incorrect_delta_ppm: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProbeImpactReport {
    pub schema: String,
    pub disposition: ProbeImpactDisposition,
    pub probe_run_id: String,
    pub no_probe_run_id: String,
    pub scenario_id: String,
    pub paired_block_id: String,
    pub execution_count: usize,
    pub receipt_count: usize,
    pub total_resource_usage: ProbeResourceUsage,
    pub service_impacts: Vec<ServiceImpactResult>,
    pub findings: Vec<ProbeImpactFinding>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProbeImpactError { ArithmeticOverflow }

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct WindowKey(String, String, u64, u64);

fn nonzero(d: &Digest) -> bool { d.0.iter().any(|b| *b != 0) }
fn ppm(count: u64, offered: u64) -> Option<u64> {
    u64::try_from(u128::from(count).checked_mul(PPM)?.checked_div(u128::from(offered))?).ok()
}
fn delta(a: u64, b: u64) -> Option<i64> {
    i64::try_from(i128::from(a).checked_sub(i128::from(b))?).ok()
}
fn key(w: &ServiceWindow) -> WindowKey {
    WindowKey(w.service.clone(), w.workload_class.clone(), w.starts_at_ms, w.ends_at_ms)
}
fn make_report(
    c: &ProbeImpactContract, disposition: ProbeImpactDisposition,
    findings: BTreeSet<ProbeImpactFinding>, execution_count: usize, receipt_count: usize,
    usage: ProbeResourceUsage, impacts: Vec<ServiceImpactResult>,
) -> ProbeImpactReport {
    ProbeImpactReport {
        schema: PROBE_IMPACT_SCHEMA.into(), disposition,
        probe_run_id: c.probe_run_id.clone(), no_probe_run_id: c.no_probe_run_id.clone(),
        scenario_id: c.scenario_id.clone(), paired_block_id: c.paired_block_id.clone(),
        execution_count, receipt_count, total_resource_usage: usage,
        service_impacts: impacts, findings: findings.into_iter().collect(),
    }
}
fn valid_window(w: &ServiceWindow, run: &str, arm: &str, c: &ProbeImpactContract) -> bool {
    w.run_id == run && w.scenario_id == c.scenario_id && w.arm_id == arm
        && w.paired_block_id == c.paired_block_id
        && w.workload_profile_digest == c.workload_profile_digest
        && w.environment_digest == c.environment_digest
        && w.fault_schedule_digest == c.fault_schedule_digest
        && w.service_observer_control_domain == c.service_observer_control_domain
        && w.service_observer_adapter_digest == c.service_observer_adapter_digest
        && !w.service_observer_id.is_empty() && !w.service.is_empty() && !w.workload_class.is_empty()
        && w.starts_at_ms < w.ends_at_ms
}

/// Compare service outcomes in a probe-enabled arm with an exact matched no-probe twin.
/// Counts for failure, slowness and business incorrectness are intentionally separate.
/// The resulting delta is a paired measurement; causal interpretation still requires
/// the scenario/seed randomization and independence plan declared by the experiment.
pub fn evaluate_probe_interference(
    c: &ProbeImpactContract,
    executions: &[ProbeExecution],
    receipts: &[ProbeImpactReceipt],
    probe_windows: &[ServiceWindow],
    no_probe_windows: &[ServiceWindow],
) -> Result<ProbeImpactReport, ProbeImpactError> {
    let mut findings = BTreeSet::new();
    let mut usage = ProbeResourceUsage::default();

    if [
        c.probe_run_id.as_str(), c.no_probe_run_id.as_str(), c.scenario_id.as_str(),
        c.paired_block_id.as_str(), c.probe_arm_id.as_str(), c.no_probe_arm_id.as_str(),
        c.tenant_id.as_str(), c.executor_control_domain.as_str(), c.observer_control_domain.as_str(),
    ].iter().any(|v| v.is_empty())
        || c.probe_run_id == c.no_probe_run_id || c.probe_arm_id == c.no_probe_arm_id
        || c.executor_control_domain == c.impact_observer_control_domain
        || c.executor_control_domain == c.service_observer_control_domain
        || c.impact_observer_control_domain == c.service_observer_control_domain
        || !nonzero(&c.impact_observer_adapter_digest)
        || !nonzero(&c.service_observer_adapter_digest) || !nonzero(&c.workload_profile_digest)
        || !nonzero(&c.environment_digest) || !nonzero(&c.fault_schedule_digest)
    { findings.insert(ProbeImpactFinding::InvalidIdentity); }
    if c.maximum_probe_duration_ms == 0 || c.service_limits.is_empty() {
        findings.insert(ProbeImpactFinding::InvalidContract);
    }
    if executions.is_empty() || executions.len() > MAX_PROBES || receipts.len() > MAX_PROBES
        || probe_windows.len() > MAX_WINDOWS || no_probe_windows.len() > MAX_WINDOWS
    { findings.insert(ProbeImpactFinding::TooManyRecords); }
    if !findings.is_empty() {
        return Ok(make_report(c, ProbeImpactDisposition::Indeterminate, findings,
            executions.len(), receipts.len(), usage, Vec::new()));
    }

    let mut execution_map = BTreeMap::new();
    for e in executions {
        if e.execution_id.is_empty() || e.probe_id.is_empty() || e.executor_id.is_empty()
            || e.target.is_empty() || e.tenant_id != c.tenant_id || e.run_id != c.probe_run_id
            || e.scenario_id != c.scenario_id || e.arm_id != c.probe_arm_id
            || e.paired_block_id != c.paired_block_id
            || e.executor_control_domain != c.executor_control_domain
            || !nonzero(&e.operation_digest)
        { findings.insert(ProbeImpactFinding::InvalidIdentity); continue; }
        if e.started_at_ms >= e.ended_at_ms {
            findings.insert(ProbeImpactFinding::InvalidExecutionInterval); continue;
        }
        if e.ended_at_ms - e.started_at_ms > c.maximum_probe_duration_ms {
            findings.insert(ProbeImpactFinding::ProbeDurationExceeded);
        }
        if e.capability != ProbeCapability::ReadOnly {
            findings.insert(ProbeImpactFinding::ProbeNotReadOnly);
        }
        if execution_map.insert(e.execution_id.as_str(), e).is_some() {
            findings.insert(ProbeImpactFinding::DuplicateExecution);
        }
    }

    let mut receipt_map = BTreeMap::new();
    let mut receipt_ids = BTreeSet::new();
    let mut evidence_digests = BTreeSet::new();
    for r in receipts {
        if r.receipt_id.is_empty() || !receipt_ids.insert(r.receipt_id.as_str()) {
            findings.insert(ProbeImpactFinding::DuplicateReceipt); continue;
        }
        if !nonzero(&r.evidence_digest) || !evidence_digests.insert(r.evidence_digest.clone()) {
            findings.insert(ProbeImpactFinding::DuplicateEvidence); continue;
        }
        if !r.complete {
            findings.insert(ProbeImpactFinding::IncompleteImpactReceipt); continue;
        }
        let e = match execution_map.get(r.execution_id.as_str()) {
            Some(e) => *e,
            None => { findings.insert(ProbeImpactFinding::ImpactIdentityMismatch); continue; }
        };
        if r.probe_id != e.probe_id || r.tenant_id != e.tenant_id || r.target != e.target
            || r.operation_digest != e.operation_digest || r.run_id != e.run_id
            || r.scenario_id != e.scenario_id || r.arm_id != e.arm_id
            || r.paired_block_id != e.paired_block_id
            || r.impact_observer_id.is_empty()
            || r.impact_observer_control_domain != c.impact_observer_control_domain
            || r.impact_observer_control_domain == e.executor_control_domain
            || r.impact_observer_adapter_digest != c.impact_observer_adapter_digest
            || r.measured_start_ms > e.started_at_ms || r.measured_end_ms < e.ended_at_ms
            || r.measured_start_ms >= r.measured_end_ms
        { findings.insert(ProbeImpactFinding::ImpactIdentityMismatch); continue; }
        if receipt_map.insert(r.execution_id.as_str(), r).is_some() {
            findings.insert(ProbeImpactFinding::DuplicateReceipt); continue;
        }
        usage = usage.add(r.resource_usage).ok_or(ProbeImpactError::ArithmeticOverflow)?;
    }
    if receipt_map.len() != execution_map.len() {
        findings.insert(ProbeImpactFinding::MissingImpactReceipt);
    }
    if !usage.within(c.maximum_total_resource_usage) {
        findings.insert(ProbeImpactFinding::ResourceEnvelopeExceeded);
    }

    let mut probe_map = BTreeMap::new();
    let mut control_map = BTreeMap::new();
    for (windows, run, arm, map) in [
        (probe_windows, c.probe_run_id.as_str(), c.probe_arm_id.as_str(), &mut probe_map),
        (no_probe_windows, c.no_probe_run_id.as_str(), c.no_probe_arm_id.as_str(), &mut control_map),
    ] {
        for w in windows {
            if !w.complete { findings.insert(ProbeImpactFinding::IncompleteServiceWindow); continue; }
            if w.offered_requests == 0 { findings.insert(ProbeImpactFinding::ZeroOfferedDemand); continue; }
            if w.failed_requests > w.offered_requests || w.slow_requests > w.offered_requests
                || w.incorrect_business_results > w.offered_requests
            { findings.insert(ProbeImpactFinding::InvalidRequestCounts); continue; }
            if !valid_window(w, run, arm, c) {
                findings.insert(ProbeImpactFinding::ServiceWindowIdentityMismatch); continue;
            }
            if map.insert(key(w), w).is_some() {
                findings.insert(ProbeImpactFinding::DuplicateServiceWindow);
            }
        }
    }
    if probe_map.len() != control_map.len()
        || probe_map.keys().any(|k| !control_map.contains_key(k))
    { findings.insert(ProbeImpactFinding::MissingServiceWindow); }

    let limits: BTreeMap<(&str, &str), &ServiceImpactLimit> = c.service_limits.iter()
        .map(|l| ((l.service.as_str(), l.workload_class.as_str()), l)).collect();
    if limits.len() != c.service_limits.len() || c.service_limits.iter().any(|l| {
        l.service.is_empty() || l.workload_class.is_empty()
            || l.maximum_failure_delta_ppm > PPM as u32
            || l.maximum_slow_delta_ppm > PPM as u32
            || l.maximum_incorrect_delta_ppm > PPM as u32
    }) { findings.insert(ProbeImpactFinding::InvalidContract); }

    let mut impacts = Vec::new();
    for (k, pw) in &probe_map {
        let cw = match control_map.get(k) {
            Some(w) => *w,
            None => continue,
        };
        let limit = match limits.get(&(k.0.as_str(), k.1.as_str())) {
            Some(l) => *l,
            None => { findings.insert(ProbeImpactFinding::MissingServiceLimit); continue; }
        };
        if pw.offered_requests != cw.offered_requests
            || pw.starts_at_ms != cw.starts_at_ms || pw.ends_at_ms != cw.ends_at_ms
        { findings.insert(ProbeImpactFinding::ServiceWindowIdentityMismatch); continue; }

        let pf = ppm(pw.failed_requests, pw.offered_requests).ok_or(ProbeImpactError::ArithmeticOverflow)?;
        let cf = ppm(cw.failed_requests, cw.offered_requests).ok_or(ProbeImpactError::ArithmeticOverflow)?;
        let ps = ppm(pw.slow_requests, pw.offered_requests).ok_or(ProbeImpactError::ArithmeticOverflow)?;
        let cs = ppm(cw.slow_requests, cw.offered_requests).ok_or(ProbeImpactError::ArithmeticOverflow)?;
        let pi = ppm(pw.incorrect_business_results, pw.offered_requests).ok_or(ProbeImpactError::ArithmeticOverflow)?;
        let ci = ppm(cw.incorrect_business_results, cw.offered_requests).ok_or(ProbeImpactError::ArithmeticOverflow)?;
        let fd = delta(pf, cf).ok_or(ProbeImpactError::ArithmeticOverflow)?;
        let sd = delta(ps, cs).ok_or(ProbeImpactError::ArithmeticOverflow)?;
        let id = delta(pi, ci).ok_or(ProbeImpactError::ArithmeticOverflow)?;
        if fd > i64::from(limit.maximum_failure_delta_ppm) {
            findings.insert(ProbeImpactFinding::ServiceFailureImpactExceeded);
        }
        if sd > i64::from(limit.maximum_slow_delta_ppm) {
            findings.insert(ProbeImpactFinding::ServiceLatencyImpactExceeded);
        }
        if id > i64::from(limit.maximum_incorrect_delta_ppm) {
            findings.insert(ProbeImpactFinding::BusinessCorrectnessImpactExceeded);
        }
        impacts.push(ServiceImpactResult {
            service: k.0.clone(), workload_class: k.1.clone(),
            offered_requests: pw.offered_requests, failure_delta_ppm: fd,
            slow_delta_ppm: sd, incorrect_delta_ppm: id,
        });
    }

    let indeterminate_findings = [
        ProbeImpactFinding::InvalidIdentity, ProbeImpactFinding::InvalidContract,
        ProbeImpactFinding::TooManyRecords, ProbeImpactFinding::DuplicateExecution,
        ProbeImpactFinding::DuplicateReceipt, ProbeImpactFinding::MissingImpactReceipt,
        ProbeImpactFinding::DuplicateEvidence, ProbeImpactFinding::IncompleteImpactReceipt,
        ProbeImpactFinding::ImpactIdentityMismatch, ProbeImpactFinding::InvalidExecutionInterval,
        ProbeImpactFinding::MissingServiceWindow, ProbeImpactFinding::DuplicateServiceWindow,
        ProbeImpactFinding::IncompleteServiceWindow, ProbeImpactFinding::ServiceWindowIdentityMismatch,
        ProbeImpactFinding::InvalidRequestCounts, ProbeImpactFinding::ZeroOfferedDemand,
        ProbeImpactFinding::MissingServiceLimit,
    ];
    let failed_findings = [
        ProbeImpactFinding::ProbeNotReadOnly, ProbeImpactFinding::ProbeDurationExceeded,
        ProbeImpactFinding::ResourceEnvelopeExceeded, ProbeImpactFinding::ServiceFailureImpactExceeded,
        ProbeImpactFinding::ServiceLatencyImpactExceeded, ProbeImpactFinding::BusinessCorrectnessImpactExceeded,
    ];
    let indeterminate = indeterminate_findings.iter().any(|f| findings.contains(f));
    let failed = failed_findings.iter().any(|f| findings.contains(f));
    let disposition = if indeterminate { ProbeImpactDisposition::Indeterminate }
        else if failed { ProbeImpactDisposition::Fail }
        else { ProbeImpactDisposition::Pass };
    Ok(make_report(c, disposition, findings, executions.len(), receipt_map.len(), usage, impacts))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn d(v: u8) -> Digest { Digest::from_byte(v) }
    fn contract() -> ProbeImpactContract {
        ProbeImpactContract {
            probe_run_id: "probe-run".into(), no_probe_run_id: "control-run".into(),
            scenario_id: "scenario-1".into(), paired_block_id: "pair-1".into(),
            probe_arm_id: "probe".into(), no_probe_arm_id: "no-probe".into(),
            tenant_id: "tenant-1".into(), executor_control_domain: "executor".into(),
            impact_observer_control_domain: "impact-observer".into(), impact_observer_adapter_digest: d(1),
            service_observer_control_domain: "service-observer-domain".into(), service_observer_adapter_digest: d(7),
            workload_profile_digest: d(2), environment_digest: d(3), fault_schedule_digest: d(4),
            maximum_probe_duration_ms: 100,
            maximum_total_resource_usage: ProbeResourceUsage {
                cpu_millicore_ms: 10_000, memory_byte_ms: 1_000_000,
                disk_read_bytes: 100_000, disk_write_bytes: 10_000, network_bytes: 100_000,
                query_rows: 10_000, duration_ms: 100,
            },
            service_limits: vec![ServiceImpactLimit {
                service: "api".into(), workload_class: "critical".into(),
                maximum_failure_delta_ppm: 10_000, maximum_slow_delta_ppm: 10_000,
                maximum_incorrect_delta_ppm: 0,
            }],
        }
    }
    fn execution(capability: ProbeCapability) -> ProbeExecution {
        ProbeExecution {
            execution_id: "exec-1".into(), probe_id: "probe-1".into(),
            tenant_id: "tenant-1".into(), target: "api".into(), operation_digest: d(5),
            capability, executor_id: "agent-adapter".into(), executor_control_domain: "executor".into(),
            run_id: "probe-run".into(), scenario_id: "scenario-1".into(), arm_id: "probe".into(),
            paired_block_id: "pair-1".into(), started_at_ms: 100, ended_at_ms: 150,
        }
    }
    fn receipt(usage: ProbeResourceUsage) -> ProbeImpactReceipt {
        ProbeImpactReceipt {
            receipt_id: "receipt-1".into(), execution_id: "exec-1".into(), probe_id: "probe-1".into(),
            tenant_id: "tenant-1".into(), target: "api".into(), operation_digest: d(5),
            run_id: "probe-run".into(), scenario_id: "scenario-1".into(), arm_id: "probe".into(),
            paired_block_id: "pair-1".into(), impact_observer_id: "observer-agent".into(),
            impact_observer_control_domain: "impact-observer".into(), impact_observer_adapter_digest: d(1), source_sequence: 1,
            measured_start_ms: 90, measured_end_ms: 160, evidence_digest: d(6), complete: true,
            resource_usage: usage,
        }
    }
    fn window(arm: &str, run: &str, failed: u64, slow: u64, incorrect: u64) -> ServiceWindow {
        ServiceWindow {
            window_id: format!("{arm}-window"), run_id: run.into(), scenario_id: "scenario-1".into(),
            arm_id: arm.into(), paired_block_id: "pair-1".into(), service: "api".into(),
            workload_class: "critical".into(), starts_at_ms: 100, ends_at_ms: 200,
            offered_requests: 1_000, failed_requests: failed, slow_requests: slow,
            incorrect_business_results: incorrect, workload_profile_digest: d(2),
            environment_digest: d(3), fault_schedule_digest: d(4), observer_id: "service-observer".into(),
            observer_control_domain: "observer".into(), complete: true,
        }
    }
    fn run(cap: ProbeCapability, usage: ProbeResourceUsage, p_fail: u64, p_slow: u64, p_bad: u64, have_receipt: bool) -> ProbeImpactReport {
        evaluate_probe_interference(
            &contract(), &[execution(cap)],
            if have_receipt { &[receipt(usage)] } else { &[] },
            &[window("probe", "probe-run", p_fail, p_slow, p_bad)],
            &[window("no-probe", "control-run", 1, 10, 0)],
        ).unwrap()
    }
    #[test]
    fn matched_probe_inside_impact_and_resource_envelopes_passes() {
        let r = run(ProbeCapability::ReadOnly, ProbeResourceUsage::default(), 1, 10, 0, true);
        assert_eq!(r.disposition, ProbeImpactDisposition::Pass);
        assert_eq!(r.service_impacts[0].failure_delta_ppm, 0);
    }
    #[test]
    fn critical_service_harm_exceeding_delta_limit_fails() {
        let r = run(ProbeCapability::ReadOnly, ProbeResourceUsage::default(), 20, 10, 0, true);
        assert_eq!(r.disposition, ProbeImpactDisposition::Fail);
        assert!(r.findings.contains(&ProbeImpactFinding::ServiceFailureImpactExceeded));
    }
    #[test]
    fn business_correctness_is_a_separate_hard_limit() {
        let r = run(ProbeCapability::ReadOnly, ProbeResourceUsage::default(), 1, 0, 1, true);
        assert_eq!(r.disposition, ProbeImpactDisposition::Fail);
        assert!(r.findings.contains(&ProbeImpactFinding::BusinessCorrectnessImpactExceeded));
    }
    #[test]
    fn mutating_probe_fails_even_when_service_metrics_do_not_change() {
        let r = run(ProbeCapability::Mutating, ProbeResourceUsage::default(), 1, 10, 0, true);
        assert_eq!(r.disposition, ProbeImpactDisposition::Fail);
        assert!(r.findings.contains(&ProbeImpactFinding::ProbeNotReadOnly));
    }
    #[test]
    fn missing_independent_receipt_is_indeterminate_not_zero() {
        let r = run(ProbeCapability::ReadOnly, ProbeResourceUsage::default(), 1, 10, 0, false);
        assert_eq!(r.disposition, ProbeImpactDisposition::Indeterminate);
        assert!(r.findings.contains(&ProbeImpactFinding::MissingImpactReceipt));
    }
    #[test]
    fn resource_usage_over_budget_fails() {
        let r = run(ProbeCapability::ReadOnly, ProbeResourceUsage {
            cpu_millicore_ms: 20_000, ..ProbeResourceUsage::default()
        }, 1, 10, 0, true);
        assert_eq!(r.disposition, ProbeImpactDisposition::Fail);
        assert!(r.findings.contains(&ProbeImpactFinding::ResourceEnvelopeExceeded));
    }
    #[test]
    fn observer_must_be_independent_from_executor() {
        let mut c = contract();
        c.impact_observer_control_domain = c.executor_control_domain.clone();
        let r = evaluate_probe_interference(
            &c, &[execution(ProbeCapability::ReadOnly)], &[receipt(ProbeResourceUsage::default())],
            &[window("probe", "probe-run", 1, 10, 0)], &[window("no-probe", "control-run", 1, 10, 0)]
        ).unwrap();
        assert_eq!(r.disposition, ProbeImpactDisposition::Indeterminate);
    }
    #[test]
    fn changed_fault_schedule_invalidates_pair() {
        let mut control = window("no-probe", "control-run", 1, 10, 0);
        control.fault_schedule_digest = d(99);
        let r = evaluate_probe_interference(
            &contract(), &[execution(ProbeCapability::ReadOnly)], &[receipt(ProbeResourceUsage::default())],
            &[window("probe", "probe-run", 1, 10, 0)], &[control]
        ).unwrap();
        assert_eq!(r.disposition, ProbeImpactDisposition::Indeterminate);
    }
    #[test]
    fn demand_mismatch_invalidates_pair() {
        let mut control = window("no-probe", "control-run", 1, 10, 0);
        control.offered_requests = 900;
        let r = evaluate_probe_interference(
            &contract(), &[execution(ProbeCapability::ReadOnly)], &[receipt(ProbeResourceUsage::default())],
            &[window("probe", "probe-run", 1, 10, 0)], &[control]
        ).unwrap();
        assert_eq!(r.disposition, ProbeImpactDisposition::Indeterminate);
    }
}
